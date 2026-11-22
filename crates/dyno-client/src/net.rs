//! Client connection to a game server over WebSocket (ws:// or wss://).

use dyno_protocol::{self as proto, ClientMsg, Content, Mode, PlayerId, PlayerInfo, ServerMsg};
use dyno_world::World;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

pub enum NetEvent {
    Msg(ServerMsg),
    Closed(String),
}

/// A live connection. Send with `tx`; receive with `rx`. Dropping `tx` closes it.
pub struct Connection {
    pub tx: Sender<ClientMsg>,
    pub rx: Receiver<NetEvent>,
}

pub struct Joined {
    pub conn: Connection,
    pub name: String,
    pub url: String,
    pub your_id: PlayerId,
    pub server_version: String,
    pub motd: String,
    pub spawn: [f32; 3],
    pub players: Vec<PlayerInfo>,
    pub world: World,
    pub mode: Mode,
    pub content: Content,
}

fn set_timeout(ws: &mut Ws, t: Duration) {
    match ws.get_mut() {
        MaybeTlsStream::Plain(s) => {
            let _ = s.set_read_timeout(Some(t));
        }
        MaybeTlsStream::Rustls(s) => {
            let _ = s.sock.set_read_timeout(Some(t));
        }
        _ => {}
    }
}

/// Connect, say hello and download the world. Blocks for up to `timeout`.
/// `token` identifies this install; observers are invisible tools.
pub fn connect(
    input: &str,
    name: &str,
    token: u64,
    observer: bool,
    timeout: Duration,
) -> Result<Joined, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let url = proto::server_url(input)?;
    let deadline = Instant::now() + timeout;

    // tungstenite has no connect timeout, so connect on a helper thread.
    let (ctx, crx) = channel();
    let u = url.clone();
    std::thread::spawn(move || {
        let cfg = WebSocketConfig::default()
            .max_message_size(Some(proto::MAX_SERVER_MESSAGE))
            .max_frame_size(Some(proto::MAX_SERVER_MESSAGE));
        let _ = ctx.send(tungstenite::client::connect_with_config(
            u.as_str(),
            Some(cfg),
            3,
        ));
    });
    let mut ws = match crx.recv_timeout(timeout) {
        Ok(Ok((ws, _resp))) => ws,
        Ok(Err(e)) => return Err(format!("Could not connect to {url}: {e}")),
        Err(_) => {
            return Err(format!(
                "Could not connect to {url}: no answer within {} s. Check the address, and that the server port is open.",
                timeout.as_secs()
            ));
        }
    };
    set_timeout(&mut ws, Duration::from_secs(2));
    let hello = ClientMsg::Hello {
        protocol: proto::PROTOCOL_VERSION,
        version: env!("CARGO_PKG_VERSION").into(),
        name: proto::clean_name(name),
    };
    ws.send(Message::Binary(proto::encode(&hello).into()))
        .map_err(|e| format!("send: {e}"))?;
    ws.send(Message::Binary(
        proto::encode(&ClientMsg::Session { token, observer }).into(),
    ))
    .map_err(|e| format!("send: {e}"))?;

    let mut welcome = None;
    let world;
    loop {
        if Instant::now() > deadline + Duration::from_secs(60) {
            return Err("The server did not send the world in time.".into());
        }
        match ws.read() {
            Ok(Message::Binary(b)) => match proto::decode::<ServerMsg>(&b)
                .map_err(|e| format!("bad message from server: {e}"))?
            {
                ServerMsg::Reject { reason } => return Err(reason),
                w @ ServerMsg::Welcome { .. } => welcome = Some(w),
                ServerMsg::World { data } => {
                    world = World::decode(&data)?;
                    break;
                }
                _ => {}
            },
            Ok(Message::Close(_)) => return Err("The server closed the connection.".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return Err(format!("Connection lost while joining: {e}")),
        }
    }
    let Some(ServerMsg::Welcome {
        your_id,
        server_version,
        motd,
        spawn,
        players,
        mode,
        content,
        ..
    }) = welcome
    else {
        return Err("The server sent no welcome.".into());
    };
    set_timeout(&mut ws, Duration::from_millis(10));
    let (tx, out_rx) = channel::<ClientMsg>();
    let (in_tx, rx) = channel::<NetEvent>();
    std::thread::Builder::new()
        .name("net".into())
        .spawn(move || {
            let reason = pump(&mut ws, out_rx, &in_tx);
            let _ = ws.close(None);
            let _ = ws.flush();
            let _ = in_tx.send(NetEvent::Closed(reason));
        })
        .map_err(|e| e.to_string())?;
    Ok(Joined {
        conn: Connection { tx, rx },
        name: name.to_string(),
        url,
        your_id,
        server_version,
        motd,
        spawn,
        players,
        world,
        mode,
        content: *content,
    })
}

fn pump(ws: &mut Ws, out_rx: Receiver<ClientMsg>, in_tx: &Sender<NetEvent>) -> String {
    // Keepalive: a minimised window stops drawing and sends nothing, so ping the server.
    let mut last_ping = Instant::now();
    loop {
        if last_ping.elapsed() >= Duration::from_secs(5) {
            last_ping = Instant::now();
            if let Err(e) = ws.write(Message::Ping(Vec::new().into())) {
                return format!("send failed: {e}");
            }
        }
        loop {
            match out_rx.try_recv() {
                Ok(m) => {
                    if let Err(e) = ws.write(Message::Binary(proto::encode(&m).into())) {
                        return format!("send failed: {e}");
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return "left the game".into(),
            }
        }
        match ws.flush() {
            Ok(()) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return format!("send failed: {e}"),
        }
        match ws.read() {
            Ok(Message::Binary(b)) => match proto::decode::<ServerMsg>(&b) {
                Ok(m) => {
                    if in_tx.send(NetEvent::Msg(m)).is_err() {
                        return "left the game".into();
                    }
                }
                Err(e) => return format!("bad message from server: {e}"),
            },
            Ok(Message::Close(_)) => return "The server closed the connection.".into(),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return format!("Connection lost: {e}"),
        }
    }
}
