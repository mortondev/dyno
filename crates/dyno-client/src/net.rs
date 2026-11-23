//! Client connection to a game server over WebSocket (ws:// or wss://).
//!
//! Joining downloads the content pack's files the cache lacks
//! (docs/design/data-and-streaming.md §1): the `Welcome` carries the pack's
//! manifest, the client asks for missing hashes with `Want`, and the server
//! streams `Asset` chunks on the same connection with bandwidth play leaves.

use crate::cache::{self, Cache};
use dyno_protocol::{
    self as proto, ClientMsg, Content, Manifest, Mode, PlayerId, PlayerInfo, ServerMsg,
};
use dyno_world::World;
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

pub enum NetEvent {
    Msg(ServerMsg),
    Closed(String),
    /// The rules changed (after /reload) and have arrived.
    Content(Box<Content>),
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
    /// the content pack this world runs
    pub manifest: Manifest,
    /// bytes of pack files downloaded to join (0 when the cache had them all)
    pub downloaded: u64,
}

/// How far the pack download of a join has come, for the loading screen.
#[derive(Debug, Default)]
pub struct Progress {
    /// bytes of pack files present (cached or arrived)
    pub done: AtomicU64,
    /// bytes of pack files needed before play
    pub total: AtomicU64,
    /// bytes actually downloaded
    pub downloaded: AtomicU64,
}

impl Progress {
    /// 0..1, or None before the manifest arrives.
    pub fn fraction(&self) -> Option<f32> {
        let t = self.total.load(Ordering::Relaxed);
        (t > 0).then(|| (self.done.load(Ordering::Relaxed) as f32 / t as f32).min(1.0))
    }
}

/// Join options beyond the address and name.
#[derive(Clone, Default)]
pub struct JoinOptions {
    pub token: u64,
    /// observers (tools such as `dyno shot`) are invisible and cannot act
    pub observer: bool,
    /// create this character now (class and looks)
    pub create: Option<proto::NewCharacter>,
    /// where pack files are kept (default: the platform cache directory)
    pub cache: Option<Cache>,
    pub progress: Option<Arc<Progress>>,
}

/// Pack files on their way from the server: assembles chunks, checks
/// hashes, keeps them in the cache and the asset store.
struct Fetch {
    manifest: Manifest,
    cache: Cache,
    progress: Arc<Progress>,
    /// hashes asked for and not complete yet
    waiting: HashSet<[u8; 32]>,
    partial: HashMap<[u8; 32], Vec<u8>>,
    /// this connection's rules (the asset store is shared by every connection)
    rules: Option<Vec<u8>>,
    /// the rules arrived anew (after a reload)
    rules_changed: bool,
}

fn want_msgs(hashes: Vec<[u8; 32]>) -> Vec<ClientMsg> {
    hashes
        .chunks(proto::MAX_WANT)
        .map(|c| ClientMsg::Want { hashes: c.to_vec() })
        .collect()
}

impl Fetch {
    fn new(cache: Cache, progress: Arc<Progress>) -> Fetch {
        Fetch {
            manifest: Manifest::default(),
            cache,
            progress,
            waiting: HashSet::new(),
            partial: HashMap::new(),
            rules: None,
            rules_changed: false,
        }
    }

    fn store(&mut self, hash: &[u8; 32], bytes: Vec<u8>) {
        for f in self.manifest.files.iter().filter(|f| f.hash == *hash) {
            crate::assets::insert(&f.path, bytes.clone());
        }
        if self.manifest.rules().is_some_and(|r| r.hash == *hash) {
            self.rules = Some(bytes);
            self.rules_changed = true;
        }
    }

    fn add_done(&self, n: u64) {
        self.progress.done.fetch_add(n, Ordering::Relaxed);
    }

    /// A (new) manifest: take what the cache has; returns the messages that ask for the rest.
    fn manifest(&mut self, m: Manifest) -> Vec<ClientMsg> {
        let old_rules = self.manifest.rules().map(|r| r.hash);
        self.manifest = m;
        let files: Vec<proto::AssetRef> = self
            .manifest
            .files
            .iter()
            .filter(|f| self.manifest.preload(&f.path))
            .cloned()
            .collect();
        self.progress
            .total
            .store(files.iter().map(|f| f.size as u64).sum(), Ordering::Relaxed);
        self.progress.done.store(0, Ordering::Relaxed);
        let mut seen = HashSet::new();
        let mut want = Vec::new();
        for f in files {
            // the same bytes under two paths, or the rules we already hold
            let held = !seen.insert(f.hash) || (old_rules == Some(f.hash) && self.rules.is_some());
            if held {
                self.add_done(f.size as u64);
                continue;
            }
            match self.cache.read(&f) {
                Some(b) => {
                    self.add_done(f.size as u64);
                    self.store(&f.hash, b);
                }
                None => {
                    if self.waiting.insert(f.hash) {
                        want.push(f.hash);
                    }
                }
            }
        }
        want_msgs(want)
    }

    /// Files asked for by path (streamed ones), unless here or on the way.
    fn request(&mut self, paths: &[String]) -> Vec<ClientMsg> {
        let mut want = Vec::new();
        for p in paths {
            let Some(f) = self.manifest.file(p).cloned() else {
                continue;
            };
            if self.waiting.contains(&f.hash) {
                continue;
            }
            match self.cache.read(&f) {
                Some(b) => self.store(&f.hash, b),
                None => {
                    self.waiting.insert(f.hash);
                    want.push(f.hash);
                }
            }
        }
        want_msgs(want)
    }

    /// A chunk arrived.
    fn chunk(&mut self, hash: [u8; 32], offset: u32, total: u32, data: Vec<u8>) {
        if !self.waiting.contains(&hash) {
            return;
        }
        self.progress
            .downloaded
            .fetch_add(data.len() as u64, Ordering::Relaxed);
        let buf = self.partial.entry(hash).or_default();
        if offset as usize != buf.len() {
            // never happens on one ordered connection; drop the file
            self.partial.remove(&hash);
            self.waiting.remove(&hash);
            return;
        }
        buf.extend_from_slice(&data);
        let have = buf.len();
        self.add_done(data.len() as u64);
        if have < total as usize {
            return;
        }
        let bytes = self.partial.remove(&hash).unwrap_or_default();
        self.waiting.remove(&hash);
        if cache::sha256(&bytes) != hash {
            eprintln!("pack file {} arrived damaged", proto::hex(&hash));
            return;
        }
        self.cache.write(&hash, &bytes);
        self.store(&hash, bytes);
    }

    fn done(&self) -> bool {
        self.waiting.is_empty()
    }

    /// New rules, once, after they arrive in play.
    fn new_rules(&mut self) -> Option<Content> {
        if !std::mem::take(&mut self.rules_changed) {
            return None;
        }
        let text = std::str::from_utf8(self.rules.as_ref()?).ok()?;
        match Content::from_json(text) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("the server's new rules: {e}");
                None
            }
        }
    }
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
    connect_as(input, name, token, observer, None, timeout)
}

/// Open a WebSocket and say hello (shared by joining and the realm query).
fn open(input: &str, name: &str, timeout: Duration) -> Result<(Ws, String), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let url = proto::server_url(input)?;
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
                "Could not reach {url} within {} s.",
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
    Ok((ws, url))
}

/// Ask a realm about itself and the characters `token` has there; with
/// `delete`, delete that character first. Returns the realm, the list, and
/// how long the answer took (ms).
pub fn realm(
    input: &str,
    token: u64,
    delete: Option<&str>,
    timeout: Duration,
) -> Result<(proto::RealmInfo, Vec<proto::CharacterSummary>, u32), String> {
    let t0 = Instant::now();
    let (mut ws, _) = open(input, "", timeout)?;
    let ask = match delete {
        Some(name) => ClientMsg::DeleteCharacter {
            token,
            name: name.to_string(),
        },
        None => ClientMsg::ListCharacters { token },
    };
    ws.send(Message::Binary(proto::encode(&ask).into()))
        .map_err(|e| format!("send: {e}"))?;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match ws.read() {
            Ok(Message::Binary(b)) => {
                match proto::decode::<ServerMsg>(&b).map_err(|e| format!("bad message: {e}"))? {
                    ServerMsg::Characters { realm, list } => {
                        let _ = ws.close(None);
                        return Ok((realm, list, t0.elapsed().as_millis() as u32));
                    }
                    ServerMsg::Reject { reason } => return Err(reason),
                    _ => {}
                }
            }
            Ok(Message::Close(_)) => return Err("The realm closed the connection.".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return Err(format!("Connection lost: {e}")),
        }
    }
    Err("The realm did not answer in time.".into())
}

/// Join, creating a new character with `create` if this name is new here.
pub fn connect_as(
    input: &str,
    name: &str,
    token: u64,
    observer: bool,
    create: Option<proto::NewCharacter>,
    timeout: Duration,
) -> Result<Joined, String> {
    connect_with(
        input,
        name,
        JoinOptions {
            token,
            observer,
            create,
            ..Default::default()
        },
        timeout,
    )
}

/// Join, fetching the pack files the cache lacks before returning.
pub fn connect_with(
    input: &str,
    name: &str,
    opts: JoinOptions,
    timeout: Duration,
) -> Result<Joined, String> {
    let JoinOptions {
        token,
        observer,
        create,
        cache,
        progress,
    } = opts;
    let mut fetch = Fetch::new(cache.unwrap_or_default(), progress.unwrap_or_default());
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
        proto::encode(&ClientMsg::Session {
            token,
            observer,
            create,
        })
        .into(),
    ))
    .map_err(|e| format!("send: {e}"))?;

    let mut welcome = None;
    let mut world = None;
    // what arrives after the welcome while pack files download, for the game
    let mut backlog = Vec::new();
    let mut last_progress = Instant::now();
    let mut last_ping = Instant::now();
    loop {
        if world.is_some() && welcome.is_some() && fetch.done() {
            break;
        }
        if world.is_none() && Instant::now() > deadline + Duration::from_secs(60) {
            return Err("The server did not send the world in time.".into());
        }
        if world.is_some() && last_progress.elapsed() > Duration::from_secs(30) {
            return Err("The server stopped sending the game's files.".into());
        }
        // a long download: keep the connection alive
        if last_ping.elapsed() > Duration::from_secs(5) {
            last_ping = Instant::now();
            let _ = ws.send(Message::Ping(Vec::new().into()));
        }
        match ws.read() {
            Ok(Message::Binary(b)) => match proto::decode::<ServerMsg>(&b)
                .map_err(|e| format!("bad message from server: {e}"))?
            {
                ServerMsg::Reject { reason } => return Err(reason),
                ServerMsg::Welcome {
                    your_id,
                    server_version,
                    motd,
                    size,
                    spawn,
                    players,
                    mode,
                    manifest,
                } => {
                    last_progress = Instant::now();
                    for m in fetch.manifest(manifest.clone()) {
                        ws.send(Message::Binary(proto::encode(&m).into()))
                            .map_err(|e| format!("send: {e}"))?;
                    }
                    welcome = Some((your_id, server_version, motd, size, spawn, players, mode));
                }
                ServerMsg::World { data } => {
                    last_progress = Instant::now();
                    world = Some(World::decode(&data)?);
                }
                ServerMsg::Asset {
                    hash,
                    offset,
                    total,
                    data,
                } => {
                    last_progress = Instant::now();
                    fetch.chunk(hash, offset, total, data);
                }
                other if welcome.is_some() => backlog.push(other),
                _ => {}
            },
            Ok(Message::Close(_)) => return Err("The server closed the connection.".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return Err(format!("Connection lost while joining: {e}")),
        }
    }
    let Some((your_id, server_version, motd, _size, spawn, players, mode)) = welcome else {
        return Err("The server sent no welcome.".into());
    };
    let world = world.ok_or("The server sent no world.")?;
    let manifest = fetch.manifest.clone();
    // this connection's rules: other connections in this process may run others
    let rules = fetch
        .rules
        .as_deref()
        .ok_or("The server's pack has no rules.")?;
    let content = Content::from_json(
        std::str::from_utf8(rules).map_err(|e| format!("the server's rules: {e}"))?,
    )
    .map_err(|e| format!("the server's rules: {e}"))?;
    fetch.rules_changed = false;
    fetch.cache.save_manifest(&manifest);
    let downloaded = fetch.progress.downloaded.load(Ordering::Relaxed);
    set_timeout(&mut ws, Duration::from_millis(10));
    let (tx, out_rx) = channel::<ClientMsg>();
    let (in_tx, rx) = channel::<NetEvent>();
    for m in backlog {
        let _ = in_tx.send(NetEvent::Msg(m));
    }
    std::thread::Builder::new()
        .name("net".into())
        .spawn(move || {
            let reason = pump(&mut ws, out_rx, &in_tx, &mut fetch);
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
        content,
        manifest,
        downloaded,
    })
}

fn pump(
    ws: &mut Ws,
    out_rx: Receiver<ClientMsg>,
    in_tx: &Sender<NetEvent>,
    fetch: &mut Fetch,
) -> String {
    // Keepalive: a minimised window stops drawing and sends nothing, so ping the server.
    let mut last_ping = Instant::now();
    loop {
        if last_ping.elapsed() >= Duration::from_secs(5) {
            last_ping = Instant::now();
            if let Err(e) = ws.write(Message::Ping(Vec::new().into())) {
                return format!("send failed: {e}");
            }
        }
        // streamed files the game asked for
        let asked = crate::assets::take_requests(|p| fetch.manifest.file(p).is_some());
        let mut out = if asked.is_empty() {
            Vec::new()
        } else {
            fetch.request(&asked)
        };
        loop {
            match out_rx.try_recv() {
                Ok(m) => out.push(m),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return "left the game".into(),
            }
        }
        for m in out {
            if let Err(e) = ws.write(Message::Binary(proto::encode(&m).into())) {
                return format!("send failed: {e}");
            }
        }
        match ws.flush() {
            Ok(()) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return format!("send failed: {e}"),
        }
        let event = match ws.read() {
            Ok(Message::Binary(b)) => match proto::decode::<ServerMsg>(&b) {
                Ok(ServerMsg::Manifest(m)) => {
                    for w in fetch.manifest(m) {
                        if let Err(e) = ws.write(Message::Binary(proto::encode(&w).into())) {
                            return format!("send failed: {e}");
                        }
                    }
                    fetch.new_rules().map(|c| NetEvent::Content(Box::new(c)))
                }
                Ok(ServerMsg::Asset {
                    hash,
                    offset,
                    total,
                    data,
                }) => {
                    fetch.chunk(hash, offset, total, data);
                    fetch.new_rules().map(|c| NetEvent::Content(Box::new(c)))
                }
                Ok(m) => Some(NetEvent::Msg(m)),
                Err(e) => return format!("bad message from server: {e}"),
            },
            Ok(Message::Close(_)) => return "The server closed the connection.".into(),
            Ok(_) => None,
            Err(tungstenite::Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                None
            }
            Err(e) => return format!("Connection lost: {e}"),
        };
        if let Some(ev) = event
            && in_tx.send(ev).is_err()
        {
            return "left the game".into();
        }
    }
}
