//! Dyno network protocol (bootstrap).
//!
//! Every message is one binary WebSocket message, encoded with postcard.
//! WebSocket lets the same server work over a direct `ip:port` connection and
//! through an HTTPS tunnel such as a Cloudflare quick tunnel.

use serde::{Deserialize, Serialize};

/// Network protocol version. Increase it on every incompatible change.
/// Clients and servers with different versions refuse to connect.
pub const PROTOCOL_VERSION: u32 = 1;

/// Default TCP port of the game server.
pub const DEFAULT_PORT: u16 = 7400;

/// Largest message a client may send to the server, in bytes.
pub const MAX_CLIENT_MESSAGE: usize = 64 * 1024;
/// Largest message the server may send to a client, in bytes.
pub const MAX_SERVER_MESSAGE: usize = 32 * 1024 * 1024;
/// Longest player name, in characters.
pub const MAX_NAME: usize = 24;
/// Longest chat message, in characters.
pub const MAX_CHAT: usize = 256;

pub type PlayerId = u32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    /// First message. The server answers with `Welcome` or `Reject`.
    Hello {
        protocol: u32,
        version: String,
        name: String,
    },
    /// The player's position and view direction.
    Pose(Pose),
    /// Request to change one cell.
    SetBlock { pos: [i32; 3], block: u8 },
    /// Chat text.
    Chat { text: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct Pose {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub pose: Pose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMsg {
    Welcome {
        your_id: PlayerId,
        server_version: String,
        motd: String,
        size: [u32; 3],
        spawn: [f32; 3],
        players: Vec<PlayerInfo>,
    },
    /// The connection is refused. The server closes it after this message.
    Reject {
        reason: String,
    },
    /// The whole world, run-length encoded (see `dyno_world::World::encode`).
    World {
        data: Vec<u8>,
    },
    PlayerJoined(PlayerInfo),
    PlayerLeft {
        id: PlayerId,
    },
    /// Poses of all other players, sent 20 times per second.
    Poses(Vec<(PlayerId, Pose)>),
    /// One cell changed.
    Block {
        pos: [i32; 3],
        block: u8,
    },
    Chat {
        from: String,
        text: String,
    },
    System {
        text: String,
    },
}

pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    postcard::to_stdvec(msg).expect("protocol messages always encode")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, postcard::Error> {
    postcard::from_bytes(bytes)
}

/// Explain a version mismatch to a person, naming the side that is older.
pub fn mismatch_reason(
    server_protocol: u32,
    server_version: &str,
    client_protocol: u32,
    client_version: &str,
) -> String {
    let older = if client_protocol < server_protocol {
        "your game"
    } else {
        "the server"
    };
    format!(
        "Version mismatch: the server runs dyno {server_version} (protocol {server_protocol}), \
         you have dyno {client_version} (protocol {client_protocol}). {older} is older. \
         Restart dyno to update, or ask the host to update the server."
    )
}

/// Normalise a player name: trim, limit length, keep printable characters.
pub fn clean_name(name: &str) -> String {
    let n: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME)
        .collect();
    if n.is_empty() {
        "player".to_string()
    } else {
        n
    }
}

/// Turn what a person typed into a WebSocket URL.
///
/// Accepts `host`, `host:port`, `1.2.3.4`, `[::1]:7400`, `ws://...`, `wss://...`
/// and `https://...` (for tunnels). A bare host uses `DEFAULT_PORT`.
pub fn server_url(input: &str) -> Result<String, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(
            "Enter a server address, for example 192.168.1.20 or https://name.trycloudflare.com"
                .into(),
        );
    }
    if s.chars().any(char::is_whitespace) {
        return Err("The address must not contain spaces.".into());
    }
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("https://") {
        return Ok(format!("wss://{}", &s[s.len() - rest.len()..]));
    }
    if let Some(rest) = lower.strip_prefix("http://") {
        return Ok(format!("ws://{}", &s[s.len() - rest.len()..]));
    }
    if lower.starts_with("ws://") || lower.starts_with("wss://") {
        return Ok(s.to_string());
    }
    // host part, without any path
    let (hostport, path) = match s.find('/') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    let has_port = if hostport.starts_with('[') {
        hostport.contains("]:")
    } else {
        hostport.matches(':').count() == 1
    };
    let hostport = if hostport.starts_with('[') || hostport.matches(':').count() <= 1 {
        hostport.to_string()
    } else {
        // bare IPv6 without brackets
        format!("[{hostport}]")
    };
    if has_port {
        Ok(format!("ws://{hostport}{path}"))
    } else {
        Ok(format!("ws://{hostport}:{DEFAULT_PORT}{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(server_url("1.2.3.4").unwrap(), "ws://1.2.3.4:7400");
        assert_eq!(server_url(" 1.2.3.4:9000 ").unwrap(), "ws://1.2.3.4:9000");
        assert_eq!(server_url("localhost").unwrap(), "ws://localhost:7400");
        assert_eq!(server_url("[::1]:7400").unwrap(), "ws://[::1]:7400");
        assert_eq!(server_url("[::1]").unwrap(), "ws://[::1]:7400");
        assert_eq!(server_url("::1").unwrap(), "ws://[::1]:7400");
        assert_eq!(
            server_url("https://abc.trycloudflare.com").unwrap(),
            "wss://abc.trycloudflare.com"
        );
        assert_eq!(
            server_url("HTTPS://Abc.trycloudflare.com/").unwrap(),
            "wss://Abc.trycloudflare.com/"
        );
        assert_eq!(server_url("wss://x.y").unwrap(), "wss://x.y");
        assert!(server_url("").is_err());
        assert!(server_url("a b").is_err());
    }

    #[test]
    fn roundtrip() {
        let m = ClientMsg::Hello {
            protocol: 1,
            version: "0.0.1".into(),
            name: "wren".into(),
        };
        assert_eq!(decode::<ClientMsg>(&encode(&m)).unwrap(), m);
    }

    #[test]
    fn names() {
        assert_eq!(clean_name("  "), "player");
        assert_eq!(clean_name("a\nb"), "ab");
        assert_eq!(clean_name(&"x".repeat(100)).len(), MAX_NAME);
    }
}
