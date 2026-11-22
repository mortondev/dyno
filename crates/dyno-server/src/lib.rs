//! Dyno game server (bootstrap).
//!
//! One thread accepts TCP connections. Each connection gets its own thread that
//! speaks WebSocket. A tick thread sends player poses 20 times per second and
//! saves the world when it changed. Bad input closes that connection only.

use dyno_protocol::{self as proto, ClientMsg, PlayerId, PlayerInfo, Pose, ServerMsg};
use dyno_world::{BLOCK_COUNT, World};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub world_path: PathBuf,
    pub world_size: [u32; 3],
    pub seed: u32,
    pub motd: String,
    pub max_players: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: SocketAddr::from(([0, 0, 0, 0], proto::DEFAULT_PORT)),
            world_path: PathBuf::from("world/world.dynw"),
            world_size: [192, 64, 192],
            seed: 20261122,
            motd: "Welcome to Dyno! Build together.".into(),
            max_players: 32,
        }
    }
}

struct Player {
    name: String,
    pose: Pose,
    tx: Sender<Arc<Vec<u8>>>,
}

struct State {
    world: World,
    players: HashMap<PlayerId, Player>,
    next_id: PlayerId,
    dirty: bool,
    /// Time of day at `time_base`.
    time_at_base: f32,
    time_base: Instant,
}

/// Length of one in-game day.
pub const DAY_SECONDS: f32 = 20.0 * 60.0;

impl State {
    fn time(&self) -> f32 {
        (self.time_at_base + self.time_base.elapsed().as_secs_f32() / DAY_SECONDS).rem_euclid(1.0)
    }
    fn time_msg(&self) -> ServerMsg {
        ServerMsg::Time {
            time: self.time(),
            day_seconds: DAY_SECONDS,
        }
    }
    fn tell(&self, id: PlayerId, text: impl Into<String>) {
        if let Some(p) = self.players.get(&id) {
            let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::System {
                text: text.into(),
            })));
        }
    }
}

pub struct Server {
    cfg: Config,
    state: Mutex<State>,
    stop: AtomicBool,
}

/// A running server. Dropping it does not stop it; call `stop`.
pub struct Handle {
    pub server: Arc<Server>,
    pub local_addr: SocketAddr,
}

impl Handle {
    /// Stop accepting players, save the world and return.
    pub fn stop(&self) {
        self.server.stop.store(true, Ordering::SeqCst);
        // wake the accept loop
        let _ = TcpStream::connect_timeout(&self.local_addr, Duration::from_millis(200));
        self.server.save();
    }
}

fn log(msg: impl AsRef<str>) {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((t / 3600) % 24, (t / 60) % 60, t % 60);
    println!("[{h:02}:{m:02}:{s:02} UTC] {}", msg.as_ref());
}

/// Start the server in background threads and return at once.
pub fn start(cfg: Config) -> std::io::Result<Handle> {
    let world = match World::load(&cfg.world_path) {
        Ok(w) => {
            log(format!(
                "Loaded world {} ({:?})",
                cfg.world_path.display(),
                w.size
            ));
            w
        }
        Err(_) => {
            log(format!(
                "Generating a new world {:?} with seed {}",
                cfg.world_size, cfg.seed
            ));
            let w = World::generate(cfg.world_size, cfg.seed);
            if let Err(e) = w.save(&cfg.world_path) {
                log(format!(
                    "Warning: cannot save world to {}: {e}",
                    cfg.world_path.display()
                ));
            }
            w
        }
    };
    let listener = TcpListener::bind(cfg.bind)?;
    let local_addr = listener.local_addr()?;
    let server = Arc::new(Server {
        cfg,
        state: Mutex::new(State {
            world,
            players: HashMap::new(),
            next_id: 1,
            dirty: false,
            time_at_base: 0.05,
            time_base: Instant::now(),
        }),
        stop: AtomicBool::new(false),
    });
    log(format!(
        "Dyno server {} listening on {local_addr} (protocol {})",
        env!("CARGO_PKG_VERSION"),
        proto::PROTOCOL_VERSION
    ));

    let s = server.clone();
    std::thread::Builder::new()
        .name("accept".into())
        .spawn(move || {
            for stream in listener.incoming() {
                if s.stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let s2 = s.clone();
                let _ = std::thread::Builder::new()
                    .name("conn".into())
                    .spawn(move || {
                        let peer = stream.peer_addr().ok();
                        if let Err(e) = s2.handle(stream) {
                            log(format!("Connection {peer:?} closed: {e}"));
                        }
                    });
            }
        })?;

    let s = server.clone();
    std::thread::Builder::new()
        .name("tick".into())
        .spawn(move || s.tick_loop())?;
    Ok(Handle { server, local_addr })
}

impl Server {
    pub fn player_count(&self) -> usize {
        self.state.lock().unwrap().players.len()
    }

    fn broadcast(&self, st: &State, msg: &ServerMsg, except: Option<PlayerId>) {
        let bytes = Arc::new(proto::encode(msg));
        for (id, p) in &st.players {
            if Some(*id) != except {
                let _ = p.tx.send(bytes.clone());
            }
        }
    }

    /// Send a system message to every player.
    pub fn announce(&self, text: &str) {
        let st = self.state.lock().unwrap();
        self.broadcast(
            &st,
            &ServerMsg::System {
                text: text.to_string(),
            },
            None,
        );
    }

    pub fn save(&self) {
        let (data, path) = {
            let mut st = self.state.lock().unwrap();
            if !st.dirty {
                return;
            }
            st.dirty = false;
            (st.world.clone(), self.cfg.world_path.clone())
        };
        match data.save(&path) {
            Ok(()) => log(format!("Saved world to {}", path.display())),
            Err(e) => log(format!("Warning: save failed: {e}")),
        }
    }

    fn tick_loop(&self) {
        let mut last_save = Instant::now();
        let mut last_time = Instant::now();
        while !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));
            {
                let st = self.state.lock().unwrap();
                if st.players.len() > 1 {
                    for (id, p) in &st.players {
                        let list: Vec<(PlayerId, Pose)> = st
                            .players
                            .iter()
                            .filter(|(o, _)| *o != id)
                            .map(|(o, q)| (*o, q.pose))
                            .collect();
                        let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Poses(list))));
                    }
                }
            }
            if last_time.elapsed() > Duration::from_secs(10) {
                last_time = Instant::now();
                let st = self.state.lock().unwrap();
                self.broadcast(&st, &st.time_msg(), None);
            }
            if last_save.elapsed() > Duration::from_secs(10) {
                last_save = Instant::now();
                self.save();
            }
        }
    }

    fn handle(&self, stream: TcpStream) -> Result<(), String> {
        let peer = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default();
        stream.set_nodelay(true).ok();
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(10))).ok();
        let cfg = WebSocketConfig::default()
            .max_message_size(Some(proto::MAX_CLIENT_MESSAGE))
            .max_frame_size(Some(proto::MAX_CLIENT_MESSAGE));
        let mut ws = tungstenite::accept_with_config(stream, Some(cfg))
            .map_err(|e| format!("handshake: {e}"))?;

        // Hello
        let hello = match ws.read().map_err(|e| format!("no hello: {e}"))? {
            Message::Binary(b) => {
                proto::decode::<ClientMsg>(&b).map_err(|e| format!("bad hello: {e}"))?
            }
            _ => return Err("expected a binary hello".into()),
        };
        let ClientMsg::Hello {
            protocol,
            version,
            name,
        } = hello
        else {
            return Err("first message was not hello".into());
        };
        let version: String = version.chars().take(32).collect();
        if protocol != proto::PROTOCOL_VERSION {
            let reason = proto::mismatch_reason(
                proto::PROTOCOL_VERSION,
                env!("CARGO_PKG_VERSION"),
                protocol,
                &version,
            );
            send(
                &mut ws,
                &ServerMsg::Reject {
                    reason: reason.clone(),
                },
            )?;
            let _ = ws.close(None);
            return Err(format!("rejected {peer}: {reason}"));
        }
        let name = proto::clean_name(&name);

        let (tx, rx) = channel::<Arc<Vec<u8>>>();
        let (id, welcome, world_data) = {
            let mut st = self.state.lock().unwrap();
            if st.players.len() >= self.cfg.max_players {
                drop(st);
                send(
                    &mut ws,
                    &ServerMsg::Reject {
                        reason: format!("The server is full ({} players).", self.cfg.max_players),
                    },
                )?;
                return Err("server full".into());
            }
            let mut name = name;
            if st
                .players
                .values()
                .any(|p| p.name.eq_ignore_ascii_case(&name))
            {
                name = format!(
                    "{}{}",
                    name.chars().take(proto::MAX_NAME - 3).collect::<String>(),
                    st.next_id % 1000
                );
            }
            let id = st.next_id;
            st.next_id += 1;
            let spawn = st.world.spawn();
            let players: Vec<PlayerInfo> = st
                .players
                .iter()
                .map(|(i, p)| PlayerInfo {
                    id: *i,
                    name: p.name.clone(),
                    pose: p.pose,
                })
                .collect();
            let welcome = ServerMsg::Welcome {
                your_id: id,
                server_version: env!("CARGO_PKG_VERSION").into(),
                motd: self.cfg.motd.clone(),
                size: st.world.size,
                spawn,
                players,
            };
            let pose = Pose {
                pos: spawn,
                yaw: 0.0,
                pitch: 0.0,
            };
            let info = PlayerInfo {
                id,
                name: name.clone(),
                pose,
            };
            self.broadcast(&st, &ServerMsg::PlayerJoined(info), None);
            self.broadcast(
                &st,
                &ServerMsg::System {
                    text: format!("{name} joined"),
                },
                None,
            );
            st.players.insert(
                id,
                Player {
                    name: name.clone(),
                    pose,
                    tx,
                },
            );
            (id, welcome, st.world.encode())
        };
        let name = self.state.lock().unwrap().players[&id].name.clone();
        log(format!(
            "{name} joined from {peer} (dyno {version}), {} online",
            self.player_count()
        ));

        let result = self.session(&mut ws, id, welcome, world_data, rx);

        {
            let mut st = self.state.lock().unwrap();
            st.players.remove(&id);
            self.broadcast(&st, &ServerMsg::PlayerLeft { id }, None);
            self.broadcast(
                &st,
                &ServerMsg::System {
                    text: format!("{name} left"),
                },
                None,
            );
        }
        log(format!(
            "{name} left ({}), {} online",
            result.as_ref().err().map(String::as_str).unwrap_or("quit"),
            self.player_count()
        ));
        let _ = ws.close(None);
        let _ = ws.flush();
        Ok(())
    }

    fn session(
        &self,
        ws: &mut WebSocket<TcpStream>,
        id: PlayerId,
        welcome: ServerMsg,
        world_data: Vec<u8>,
        rx: Receiver<Arc<Vec<u8>>>,
    ) -> Result<(), String> {
        send(ws, &welcome)?;
        send(ws, &ServerMsg::World { data: world_data })?;
        let time = self.state.lock().unwrap().time_msg();
        send(ws, &time)?;
        ws.get_mut()
            .set_read_timeout(Some(Duration::from_millis(15)))
            .ok();

        let mut last_msg = Instant::now();
        let mut window = Instant::now();
        let (mut msgs, mut chats) = (0u32, 0u32);
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return Err("server stopping".into());
            }
            match ws.read() {
                Ok(Message::Binary(b)) => {
                    last_msg = Instant::now();
                    msgs += 1;
                    let msg =
                        proto::decode::<ClientMsg>(&b).map_err(|e| format!("bad message: {e}"))?;
                    match msg {
                        ClientMsg::Pose(p) => {
                            if p.pos
                                .iter()
                                .chain([p.yaw, p.pitch].iter())
                                .all(|v| v.is_finite())
                                && let Some(pl) = self.state.lock().unwrap().players.get_mut(&id)
                            {
                                pl.pose = p;
                            }
                        }
                        ClientMsg::SetBlock { pos, block } => {
                            if block >= BLOCK_COUNT {
                                return Err(format!("unknown block {block}"));
                            }
                            let mut st = self.state.lock().unwrap();
                            let me = st.players.get(&id).map(|p| p.pose.pos).unwrap_or_default();
                            let d2: f32 =
                                (0..3).map(|i| (pos[i] as f32 + 0.5 - me[i]).powi(2)).sum();
                            if st.world.in_bounds(pos) && d2 <= 12.0 * 12.0 {
                                st.world.set(pos[0], pos[1], pos[2], block);
                                st.dirty = true;
                                self.broadcast(&st, &ServerMsg::Block { pos, block }, None);
                            } else {
                                // tell the client the real value so it can undo its prediction
                                let real = st.world.get(pos[0], pos[1], pos[2]);
                                if let Some(p) = st.players.get(&id) {
                                    let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Block {
                                        pos,
                                        block: real,
                                    })));
                                }
                            }
                        }
                        ClientMsg::Chat { text } => {
                            chats += 1;
                            let text: String = text
                                .chars()
                                .filter(|c| !c.is_control())
                                .take(proto::MAX_CHAT)
                                .collect();
                            if chats <= 5 && text.trim_start().starts_with('/') {
                                self.command(id, text.trim());
                            } else if chats <= 5 && !text.trim().is_empty() {
                                let st = self.state.lock().unwrap();
                                let from = st
                                    .players
                                    .get(&id)
                                    .map(|p| p.name.clone())
                                    .unwrap_or_default();
                                log(format!("<{from}> {text}"));
                                self.broadcast(&st, &ServerMsg::Chat { from, text }, None);
                            }
                        }
                        ClientMsg::Hello { .. } => return Err("second hello".into()),
                    }
                }
                Ok(Message::Close(_)) => return Ok(()),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(tungstenite::Error::ConnectionClosed)
                | Err(tungstenite::Error::AlreadyClosed) => return Ok(()),
                Err(e) => return Err(format!("read: {e}")),
            }
            if window.elapsed() >= Duration::from_secs(1) {
                if msgs > 200 {
                    return Err(format!("too many messages ({msgs}/s)"));
                }
                window = Instant::now();
                msgs = 0;
                chats = 0;
            }
            if last_msg.elapsed() > Duration::from_secs(60) {
                return Err("timed out".into());
            }
            let mut wrote = false;
            while let Ok(bytes) = rx.try_recv() {
                ws.write(Message::Binary((*bytes).clone().into()))
                    .map_err(|e| format!("write: {e}"))?;
                wrote = true;
            }
            if wrote {
                match ws.flush() {
                    Ok(()) => {}
                    Err(tungstenite::Error::Io(e))
                        if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(e) => return Err(format!("flush: {e}")),
                }
            }
        }
    }
}

fn send(ws: &mut WebSocket<TcpStream>, msg: &ServerMsg) -> Result<(), String> {
    ws.send(Message::Binary(proto::encode(msg).into()))
        .map_err(|e| format!("send: {e}"))
}

impl Server {
    fn command(&self, id: PlayerId, line: &str) {
        let mut st = self.state.lock().unwrap();
        let me = st
            .players
            .get(&id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
        let arg = parts.collect::<Vec<_>>().join(" ");
        log(format!("{me} ran {line}"));
        match cmd.as_str() {
            "/help" => st.tell(
                id,
                "Commands: /tp <player>, /spawn, /time <day|noon|sunset|night|midnight|0-1>, /who",
            ),
            "/who" => {
                let names: Vec<String> = st.players.values().map(|p| p.name.clone()).collect();
                st.tell(id, format!("{} online: {}", names.len(), names.join(", ")));
            }
            "/spawn" => {
                let pos = st.world.spawn();
                if let Some(p) = st.players.get(&id) {
                    let _ =
                        p.tx.send(Arc::new(proto::encode(&ServerMsg::Teleport { pos })));
                }
            }
            "/tp" => {
                let lower = arg.to_lowercase();
                let target = st
                    .players
                    .iter()
                    .filter(|(pid, _)| **pid != id)
                    .map(|(_, p)| p)
                    .find(|p| p.name.to_lowercase() == lower)
                    .or_else(|| {
                        st.players.values().find(|p| {
                            !lower.is_empty() && p.name.to_lowercase().starts_with(&lower)
                        })
                    })
                    .map(|p| (p.name.clone(), p.pose.pos));
                match target {
                    Some((name, pos)) => {
                        if let Some(p) = st.players.get(&id) {
                            let _ =
                                p.tx.send(Arc::new(proto::encode(&ServerMsg::Teleport { pos })));
                        }
                        st.tell(id, format!("Teleported to {name}"));
                    }
                    None => st.tell(id, format!("No player called '{arg}'. Try /who")),
                }
            }
            "/time" => {
                let t = match arg.as_str() {
                    "day" | "morning" => Some(0.05),
                    "noon" => Some(0.25),
                    "sunset" | "dusk" | "evening" => Some(0.47),
                    "night" => Some(0.58),
                    "midnight" => Some(0.75),
                    "sunrise" | "dawn" => Some(0.97),
                    other => other
                        .parse::<f32>()
                        .ok()
                        .filter(|v| (0.0..=1.0).contains(v)),
                };
                match t {
                    Some(t) => {
                        st.time_at_base = t;
                        st.time_base = Instant::now();
                        let msg = st.time_msg();
                        self.broadcast(&st, &msg, None);
                        self.broadcast(&st, &ServerMsg::System { text: format!("{me} set the time to {arg}") }, None);
                    }
                    None => st.tell(id, "Usage: /time day|noon|sunset|night|midnight|sunrise, or a number from 0 to 1"),
                }
            }
            _ => st.tell(id, "Unknown command. Try /help"),
        }
    }
}
