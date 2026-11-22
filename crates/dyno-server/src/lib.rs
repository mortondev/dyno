//! Dyno game server.
//!
//! One thread accepts TCP connections. Each connection gets its own thread that
//! speaks WebSocket and passes messages to the shared game state. A tick thread
//! advances the world 20 times per second, sends player poses and saves.
//! Bad input closes that connection only.

pub mod abilities;
pub mod creatures;
pub mod game;
pub mod gear;
pub mod professions;
pub mod rpg;
pub mod store;

use dyno_protocol::{self as proto, ClientMsg, Mode, PlayerId, Pose, ServerMsg};
use dyno_sim::rng::Rng;
use dyno_world::World;
use dyno_world::content::Content;
use game::{State, log};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

pub use game::DAY_SECONDS;

/// Post events to a Discord webhook from a background thread, at most one a second.
fn start_webhook(url: String) -> std::sync::mpsc::Sender<String> {
    let (tx, rx) = channel::<String>();
    std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        for text in rx {
            let body = serde_json::json!({ "content": text, "allowed_mentions": { "parse": [] } });
            if let Err(e) = agent.post(&url).send_json(&body) {
                log(format!("Webhook failed: {e}"));
            }
            std::thread::sleep(Duration::from_millis(1100));
        }
    });
    tx
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub world_path: PathBuf,
    pub world_size: [u32; 3],
    pub seed: u32,
    pub motd: String,
    pub max_players: usize,
    /// mode for players who join for the first time
    pub default_mode: Mode,
    /// content JSON; None uses the built-in Hearthblock pack
    pub content_path: Option<PathBuf>,
    /// names allowed to use /gamemode, /time, /reload and /give. Empty: the
    /// world's owner (its first player) only.
    pub ops: Vec<String>,
    /// Discord webhook URL for milestones, deaths and days (optional)
    pub webhook: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: SocketAddr::from(([0, 0, 0, 0], proto::DEFAULT_PORT)),
            world_path: PathBuf::from("world/world.dynw"),
            world_size: [256, 96, 256],
            seed: 20261122,
            motd: "Welcome to Dyno! Build together.".into(),
            max_players: 32,
            default_mode: Mode::Survival,
            content_path: None,
            ops: Vec::new(),
            webhook: None,
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
        let _ = TcpStream::connect_timeout(&self.local_addr, Duration::from_millis(200));
        self.server.save();
    }
}

fn load_content(path: &Option<PathBuf>) -> Result<Content, String> {
    match path {
        Some(p) => {
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            Content::from_json(&text)
        }
        None => Ok(Content::default_pack()),
    }
}

/// Start the server in background threads and return at once.
pub fn start(cfg: Config) -> std::io::Result<Handle> {
    let content = load_content(&cfg.content_path).map_err(std::io::Error::other)?;
    let mut world = match World::load(&cfg.world_path) {
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
    let meta = store::load_meta(&store::meta_path(&cfg.world_path));
    let before = world.clone();
    let spawn_hearth = world.ensure_spawn_hearth(meta.spawn_hearth);
    if world.get(spawn_hearth[0], spawn_hearth[1], spawn_hearth[2])
        != before.get(spawn_hearth[0], spawn_hearth[1], spawn_hearth[2])
    {
        log(format!("Built the spawn hearth at {spawn_hearth:?}"));
        if let Err(e) = world.save(&cfg.world_path) {
            log(format!("Warning: cannot save world: {e}"));
        }
    }
    drop(before);
    if rpg::build_landmarks(&mut world, &content, spawn_hearth) {
        log("Built quest beacons");
        if let Err(e) = world.save(&cfg.world_path) {
            log(format!("Warning: cannot save world: {e}"));
        }
    }
    if content.world.protected && dyno_world::village::build_hearthstead(&mut world, spawn_hearth) {
        log("Built Hearthstead");
        if let Err(e) = world.save(&cfg.world_path) {
            log(format!("Warning: cannot save world: {e}"));
        }
    }
    let hearths = creatures::find_hearths(&world);
    let light = dyno_world::light::Light::new(&world);
    let webhook = cfg
        .webhook
        .clone()
        .filter(|u| !u.is_empty())
        .map(start_webhook);
    let listener = TcpListener::bind(cfg.bind)?;
    let local_addr = listener.local_addr()?;
    let saved = store::load_players(&store::players_path(&cfg.world_path));
    let containers = store::load_chests(&content, &store::chests_path(&cfg.world_path));
    log(format!(
        "Content: {} v{} ({} items, {} recipes, {} creatures)",
        content.name,
        content.version,
        content.items.len(),
        content.recipes.len(),
        content.creatures.len()
    ));
    let mut state = State {
        world,
        content,
        players: HashMap::new(),
        observers: HashMap::new(),
        next_id: 1,
        dirty: false,
        saved,
        containers,
        entities: HashMap::new(),
        next_entity: 1,
        rng: Rng::new(cfg.seed as u64 ^ 0x5eed_1234),
        clock: if meta.day == 0 { 0.02 } else { meta.clock },
        day: meta.day.max(1),
        default_mode: cfg.default_mode,
        ops: cfg.ops.clone(),
        owner: meta.owner,
        light,
        hearths,
        events: meta.events.into_iter().collect(),
        webhook,
        meta_dirty: true,
        spawn_hearth,
        regrow: meta.regrow.clone(),
    };
    state.spawn_npcs();
    let server = Arc::new(Server {
        cfg,
        state: Mutex::new(state),
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
                        if let Err(e) = s2.connection(stream) {
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

fn read_msg(ws: &mut WebSocket<TcpStream>) -> Result<ClientMsg, String> {
    match ws.read().map_err(|e| format!("read: {e}"))? {
        Message::Binary(b) => {
            proto::decode::<ClientMsg>(&b).map_err(|e| format!("bad message: {e}"))
        }
        _ => Err("expected a binary message".into()),
    }
}

fn send(ws: &mut WebSocket<TcpStream>, msg: &ServerMsg) -> Result<(), String> {
    ws.send(Message::Binary(proto::encode(msg).into()))
        .map_err(|e| format!("send: {e}"))
}

impl Server {
    pub fn player_count(&self) -> usize {
        self.state.lock().unwrap().players.len()
    }

    /// Send a system message to everyone.
    pub fn announce(&self, text: &str) {
        self.state.lock().unwrap().broadcast(
            &ServerMsg::System {
                text: text.to_string(),
            },
            None,
        );
    }

    pub fn save(&self) {
        let (world, players, chests, content, meta) = {
            let mut st = self.state.lock().unwrap();
            let ids: Vec<PlayerId> = st.players.keys().copied().collect();
            for id in ids {
                let p = st.players.remove(&id).unwrap();
                st.remember(&p);
                st.players.insert(id, p);
            }
            let world = if st.dirty {
                st.dirty = false;
                Some(st.world.clone())
            } else {
                None
            };
            st.meta_dirty = false;
            let meta = store::Meta {
                owner: st.owner.clone(),
                day: st.day,
                clock: st.clock,
                events: st.events.iter().cloned().collect(),
                spawn_hearth: Some(st.spawn_hearth),
                regrow: st.regrow.clone(),
            };
            (
                world,
                st.saved.clone(),
                st.containers.clone(),
                st.content.clone(),
                meta,
            )
        };
        let path = &self.cfg.world_path;
        if let Err(e) = store::save_meta(&store::meta_path(path), &meta) {
            log(format!("Warning: cannot save server state: {e}"));
        }
        if let Err(e) = store::save_players(&store::players_path(path), &players) {
            log(format!("Warning: cannot save players: {e}"));
        }
        if let Err(e) = store::save_chests(&content, &store::chests_path(path), &chests) {
            log(format!("Warning: cannot save chests: {e}"));
        }
        if let Some(w) = world {
            match w.save(path) {
                Ok(()) => log(format!("Saved world to {}", path.display())),
                Err(e) => log(format!("Warning: save failed: {e}")),
            }
        }
    }

    fn tick_loop(&self) {
        let mut last_save = Instant::now();
        let mut last_time = Instant::now();
        let mut last_tick = Instant::now();
        let mut n: u64 = 0;
        while !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));
            n += 1;
            let dt = last_tick.elapsed().as_secs_f32().min(0.25);
            last_tick = Instant::now();
            {
                let mut st = self.state.lock().unwrap();
                st.tick(dt);
                // poses of everyone else, to each player; everyone to observers
                if !st.players.is_empty() {
                    let all: Vec<(PlayerId, Pose)> = st
                        .players
                        .iter()
                        .filter(|(_, p)| !p.dead)
                        .map(|(i, p)| (*i, p.pose))
                        .collect();
                    for (id, p) in &st.players {
                        let list: Vec<(PlayerId, Pose)> =
                            all.iter().filter(|(o, _)| o != id).copied().collect();
                        if !list.is_empty() {
                            let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Poses(list))));
                        }
                    }
                    let bytes = Arc::new(proto::encode(&ServerMsg::Poses(all)));
                    for tx in st.observers.values() {
                        let _ = tx.send(bytes.clone());
                    }
                }
                if n.is_multiple_of(2)
                    && let Some(m) = st.entity_moves()
                {
                    st.broadcast(&m, None);
                }
            }
            if last_time.elapsed() > Duration::from_secs(10) {
                last_time = Instant::now();
                let st = self.state.lock().unwrap();
                st.broadcast(&st.time_msg(), None);
            }
            if last_save.elapsed() > Duration::from_secs(10) {
                last_save = Instant::now();
                self.save();
            }
        }
    }

    fn connection(&self, stream: TcpStream) -> Result<(), String> {
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

        let ClientMsg::Hello {
            protocol,
            version,
            name,
        } = read_msg(&mut ws)?
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
        let ClientMsg::Session { token, observer } = read_msg(&mut ws)? else {
            return Err("second message was not session".into());
        };
        let name = proto::clean_name(&name);
        let (tx, rx) = channel::<Arc<Vec<u8>>>();

        let (id, first, world_data) = {
            let mut st = self.state.lock().unwrap();
            if observer {
                let id = st.next_id;
                st.next_id += 1;
                st.observers.insert(id, tx);
                let welcome = ServerMsg::Welcome {
                    your_id: id,
                    server_version: env!("CARGO_PKG_VERSION").into(),
                    motd: self.cfg.motd.clone(),
                    size: st.world.size,
                    spawn: st.spawn_point(),
                    players: st
                        .players
                        .iter()
                        .map(|(i, p)| proto::PlayerInfo {
                            id: *i,
                            name: p.name.clone(),
                            pose: p.pose,
                        })
                        .collect(),
                    mode: Mode::Creative,
                    content: Box::new(st.content.clone()),
                };
                (id, vec![welcome], st.world.encode())
            } else {
                if st.players.len() >= self.cfg.max_players {
                    drop(st);
                    send(
                        &mut ws,
                        &ServerMsg::Reject {
                            reason: format!(
                                "The server is full ({} players).",
                                self.cfg.max_players
                            ),
                        },
                    )?;
                    return Err("server full".into());
                }
                match st.join(&name, token, tx) {
                    Ok((id, mut msgs)) => {
                        if let Some(ServerMsg::Welcome { motd, .. }) = msgs.first_mut() {
                            *motd = self.cfg.motd.clone();
                        }
                        (id, msgs, st.world.encode())
                    }
                    Err(reason) => {
                        drop(st);
                        send(
                            &mut ws,
                            &ServerMsg::Reject {
                                reason: reason.clone(),
                            },
                        )?;
                        let _ = ws.close(None);
                        return Err(format!("refused {name}: {reason}"));
                    }
                }
            }
        };
        if !observer {
            log(format!(
                "{name} joined from {peer} (dyno {version}), {} online",
                self.player_count()
            ));
        }
        let result = self.session(&mut ws, id, first, world_data, rx);
        self.state.lock().unwrap().leave(id);
        if !observer {
            log(format!(
                "{name} left ({}), {} online",
                result.as_ref().err().map(String::as_str).unwrap_or("quit"),
                self.player_count()
            ));
        }
        let _ = ws.close(None);
        let _ = ws.flush();
        Ok(())
    }

    fn session(
        &self,
        ws: &mut WebSocket<TcpStream>,
        id: PlayerId,
        first: Vec<ServerMsg>,
        world_data: Vec<u8>,
        rx: Receiver<Arc<Vec<u8>>>,
    ) -> Result<(), String> {
        let mut first = first.into_iter();
        if let Some(welcome) = first.next() {
            send(ws, &welcome)?;
        }
        send(ws, &ServerMsg::World { data: world_data })?;
        let time = self.state.lock().unwrap().time_msg();
        send(ws, &time)?;
        for m in first {
            send(ws, &m)?;
        }
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
                        ClientMsg::Chat { text } => {
                            chats += 1;
                            let text: String = text
                                .chars()
                                .filter(|c| !c.is_control())
                                .take(proto::MAX_CHAT)
                                .collect();
                            if chats <= 20 && text.trim_start().starts_with('/') {
                                self.command(id, text.trim());
                            } else if chats <= 5 && !text.trim().is_empty() {
                                let st = self.state.lock().unwrap();
                                if let Some(p) = st.players.get(&id) {
                                    let from = p.name.clone();
                                    log(format!("<{from}> {text}"));
                                    st.broadcast(&ServerMsg::Chat { from, text }, None);
                                }
                            }
                        }
                        ClientMsg::Hello { .. } | ClientMsg::Session { .. } => {
                            return Err("second hello".into());
                        }
                        other => self.state.lock().unwrap().handle(id, other)?,
                    }
                }
                Ok(Message::Close(_)) => return Ok(()),
                Ok(_) => last_msg = Instant::now(),
                Err(tungstenite::Error::Io(e))
                    if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(tungstenite::Error::ConnectionClosed)
                | Err(tungstenite::Error::AlreadyClosed) => return Ok(()),
                Err(e) => return Err(format!("read: {e}")),
            }
            if window.elapsed() >= Duration::from_secs(1) {
                if msgs > 300 {
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

    fn command(&self, id: PlayerId, line: &str) {
        let mut st = self.state.lock().unwrap();
        let Some(me) = st.players.get(&id).map(|p| p.name.clone()) else {
            return;
        };
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
        let args: Vec<&str> = parts.collect();
        let arg = args.join(" ");
        log(format!("{me} ran {line}"));
        let op = st.may_op(&me);
        match cmd.as_str() {
            "/help" => st.tell(id, "Commands: /tp <player>, /spawn, /time <day|noon|sunset|night|midnight|0-1>, /who, /kill, /gamemode <survival|creative>, /give <item> [count], /summon <creature>, /reload"),
            "/who" => {
                let names: Vec<String> = st.players.values().map(|p| format!("{}{}", p.name, if p.away() { " (away)" } else { "" })).collect();
                st.tell(id, format!("{} online: {}", names.len(), names.join(", ")));
            }
            "/spawn" => {
                let pos = st.players[&id].spawn.unwrap_or_else(|| st.spawn_point());
                if let Some(p) = st.players.get_mut(&id) {
                    p.pose.pos = pos;
                    let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Teleport { pos })));
                }
            }
            "/kill" => st.damage(id, 1000.0, [0.0; 3], "gave up"),
            "/tp" => {
                let lower = arg.to_lowercase();
                let target = st
                    .players
                    .iter()
                    .filter(|(pid, _)| **pid != id)
                    .map(|(_, p)| p)
                    .find(|p| p.name.to_lowercase() == lower || (!lower.is_empty() && p.name.to_lowercase().starts_with(&lower)))
                    .map(|p| (p.name.clone(), p.pose.pos));
                match target {
                    Some((name, pos)) => {
                        if let Some(p) = st.players.get_mut(&id) {
                            p.pose.pos = pos;
                            let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Teleport { pos })));
                        }
                        st.tell(id, format!("Teleported to {name}"));
                    }
                    None => st.tell(id, format!("No player called '{arg}'. Try /who")),
                }
            }
            "/class" => {
                let key = arg.trim().to_lowercase();
                if key == "reset" && op {
                    if let Some(p) = st.players.get_mut(&id) {
                        p.class = None;
                        p.power = 0.0;
                        p.auras.clear();
                    }
                    st.send_character(id);
                    st.tell(id, "Class cleared. Choose again with /class <name>.");
                } else {
                    match st.content.classes.iter().find(|c| c.key == key || c.name.eq_ignore_ascii_case(&key)).map(|c| c.id) {
                        Some(c) => st.choose_class(id, c),
                        None => {
                            let names: Vec<String> = st.content.classes.iter().map(|c| c.key.clone()).collect();
                            st.tell(id, format!("Usage: /class <{}>", names.join("|")));
                        }
                    }
                }
            }
            "/summon" => {
                let key = arg.trim().to_lowercase();
                let found = st.content.creatures.iter().find(|c| c.key == key || c.name.eq_ignore_ascii_case(&key)).map(|c| c.id);
                match found {
                    _ if !op => st.tell(id, "Only the world's owner can summon creatures."),
                    Some(cid) => {
                        let pose = st.players[&id].pose;
                        let (s, c) = pose.yaw.sin_cos();
                        let at = [pose.pos[0] - s * 4.0, pose.pos[1] + 0.5, pose.pos[2] - c * 4.0];
                        st.spawn_creature(cid, at);
                    }
                    None => {
                        let keys: Vec<String> = st.content.creatures.iter().map(|c| c.key.clone()).collect();
                        st.tell(id, format!("Usage: /summon <{}>", keys.join("|")));
                    }
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
                    other => other.parse::<f32>().ok().filter(|v| (0.0..=1.0).contains(v)),
                };
                match t {
                    _ if !op => st.tell(id, "Only the world's owner can change the time."),
                    Some(t) => {
                        st.set_time(t);
                        st.broadcast(&ServerMsg::System { text: format!("{me} set the time to {arg}") }, None);
                    }
                    None => st.tell(id, "Usage: /time day|noon|sunset|night|midnight|sunrise, or a number from 0 to 1"),
                }
            }
            "/gamemode" | "/gm" => {
                if !op {
                    return st.tell(id, "Only server operators can change game mode.");
                }
                let mode = match args.first().copied() {
                    Some("creative") | Some("c") | Some("1") => Mode::Creative,
                    Some("survival") | Some("s") | Some("0") => Mode::Survival,
                    _ => return st.tell(id, "Usage: /gamemode survival|creative"),
                };
                let p = st.players.get_mut(&id).unwrap();
                p.mode = mode;
                let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Status { health: p.health, food: p.food, mode, fuel: p.fuel })));
                st.broadcast(&ServerMsg::System { text: format!("{me} is now in {} mode", if mode == Mode::Creative { "creative" } else { "survival" }) }, None);
            }
            "/give" => {
                if !op {
                    return st.tell(id, "Only server operators can use /give.");
                }
                let Some(key) = args.first() else { return st.tell(id, "Usage: /give <item> [count]") };
                let n: u8 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(1);
                match st.item_id(key) {
                    Some(item) => {
                        let c = st.content.clone();
                        let p = st.players.get_mut(&id).unwrap();
                        p.pack.add(&c, dyno_sim::Stack::new(item, n));
                        let _ = p.tx.send(Arc::new(proto::encode(&ServerMsg::Pack(p.pack.slots.clone()))));
                        st.broadcast(&ServerMsg::System { text: format!("{me} gave themselves {n} {}", c.item(item).name) }, None);
                    }
                    None => st.tell(id, format!("No item '{key}'.")),
                }
            }
            "/reload" => {
                if !op {
                    return st.tell(id, "Only server operators can reload content.");
                }
                match load_content(&self.cfg.content_path) {
                    Ok(c) => {
                        let summary = format!("{} v{}: {} items, {} recipes", c.name, c.version, c.items.len(), c.recipes.len());
                        st.replace_content(c);
                        st.broadcast(&ServerMsg::System { text: format!("{me} reloaded content ({summary})") }, None);
                    }
                    Err(e) => st.tell(id, format!("Reload failed: {e}")),
                }
            }
            _ => st.tell(id, "Unknown command. Try /help"),
        }
    }
}
