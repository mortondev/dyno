//! Dyno game server.
//!
//! One thread accepts TCP connections. Each connection gets its own thread that
//! speaks WebSocket and passes messages to the shared game state. A tick thread
//! advances the world 20 times per second, sends player poses and saves.
//! Bad input closes that connection only.

pub mod abilities;
pub mod camps;
pub mod cluster;
mod clustered;
pub mod creatures;
pub mod events;
pub mod game;
pub mod gear;
pub mod loot;
pub mod migrate;
pub mod movement;
pub mod pack;
pub mod party;
pub mod professions;
pub mod routines;
pub mod rpg;
pub mod shard;
pub mod store;
pub mod stream;
pub mod trade;

use dyno_protocol::{self as proto, ClientMsg, Mode, PlayerId, ServerMsg};
use dyno_sim::rng::Rng;
use dyno_world::World;
use dyno_world::content::Content;
use game::{State, log};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, RwLock};
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
    /// the size of the world to make. Zeros: the size of the pack's layout
    /// (384 × 96 × 384 for a world without one). With a layout, a smaller
    /// size makes the window of that size round the spawn (tests), and the
    /// height is always the layout's.
    pub world_size: [u32; 3],
    pub seed: u32,
    pub motd: String,
    pub max_players: usize,
    /// mode for players who join for the first time
    pub default_mode: Mode,
    /// the content pack: a pack directory, or a rules JSON file (the
    /// built-in pack with those rules); None uses the built-in pack
    pub content_path: Option<PathBuf>,
    /// bytes per second of pack files each session may send (left over from play)
    pub asset_rate: usize,
    /// the same before the player starts playing (on the loading screen,
    /// nothing competes with the download)
    pub join_asset_rate: usize,
    /// names allowed to use /gamemode, /time, /reload and /give. Empty: the
    /// world's owner (its first player) only.
    pub ops: Vec<String>,
    /// Discord webhook URL for milestones, deaths and days (optional)
    pub webhook: Option<String>,
    /// players per shard before another copy of the world opens (RPG worlds)
    pub shard_cap: usize,
    /// players in one area of one shard before newcomers there are spread out
    pub area_cap: usize,
    /// most shards one server runs
    pub max_shards: usize,
    /// accept any movement from clients (tests only)
    pub trust_movement: bool,
    /// run as one instance of a cluster (absent: a stand-alone server)
    pub cluster: Option<cluster::ClusterConfig>,
    /// world streaming and area of interest: view radius, budgets
    pub stream: stream::StreamConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: SocketAddr::from(([0, 0, 0, 0], proto::DEFAULT_PORT)),
            world_path: PathBuf::from("world/world.dynw"),
            world_size: [0, 0, 0],
            seed: 20261122,
            motd: "Welcome to Ashfall Vale.".into(),
            max_players: 32,
            default_mode: Mode::Survival,
            content_path: None,
            asset_rate: 256 * 1024,
            join_asset_rate: 2 * 1024 * 1024,
            ops: Vec::new(),
            webhook: None,
            shard_cap: 40,
            area_cap: 16,
            max_shards: 16,
            trust_movement: false,
            cluster: None,
            stream: Default::default(),
        }
    }
}

/// One copy of the world and the players in it.
pub struct ShardHost {
    pub id: u32,
    pub state: Mutex<State>,
    empty_since: Mutex<Option<Instant>>,
}

/// Where a connected player is, and how to reach them.
#[derive(Clone)]
struct Route {
    shard: u32,
    name: String,
    tx: Sender<Arc<Vec<u8>>>,
    /// the client connection (on the edge that holds it)
    conn: u64,
    /// relayed players: the edge instance that holds the client
    edge: Option<cluster::machine::NodeId>,
}

/// A relayed client: the edge instance and its connection number.
type RelayKey = (cluster::machine::NodeId, u64);

pub struct Server {
    cfg: Config,
    stop: AtomicBool,
    shards: RwLock<Vec<Arc<ShardHost>>>,
    routes: Mutex<HashMap<PlayerId, Route>>,
    /// names being joined right now (one character, one place)
    joining: Mutex<std::collections::HashSet<String>>,
    parties: Mutex<party::Parties>,
    /// the world as built at start: every new shard is a copy of it (and
    /// of its light), sharing every section until it changes one
    base: World,
    base_light: dyno_world::light::Light,
    /// the base world's far tiles, encoded (a new shard starts with them)
    base_far: dyno_world::far::Encoded,
    next_shard: AtomicU32,
    /// the latest view of every player, for the balancer and party frames
    snaps: Mutex<Vec<shard::Snap>>,
    /// what every tick since the start has cost
    ticks: Mutex<TickCost>,
    /// when each player last spoke in a world channel (one line a second)
    channel_at: Mutex<HashMap<PlayerId, Instant>>,
    rpg: bool,
    /// cluster mode: the quorum and everything that spans instances
    cluster: Option<Arc<cluster::Cluster>>,
    /// cluster mode: epochs of the characters this instance hosts
    epochs: Mutex<HashMap<String, u64>>,
    /// edge side: client connections whose player lives on another instance
    relays_out: Mutex<HashMap<u64, clustered::EdgeRelay>>,
    /// host side: players relayed from another instance's edge
    hosted: Mutex<HashMap<(cluster::machine::NodeId, u64), PlayerId>>,
    /// host side: one ordered queue per relayed client (never block the link)
    relay_in: Mutex<HashMap<RelayKey, Sender<Vec<u8>>>>,
    next_conn: std::sync::atomic::AtomicU64,
    /// connections to end (fenced, signed in elsewhere)
    kicked: Mutex<std::collections::HashSet<u64>>,
    /// out of contact with the quorum: serve no one
    fenced: AtomicBool,
    /// running a different version from the cluster: take no one new
    draining: AtomicBool,
    adoptions: cluster::link::Waiters<Result<(), String>>,
    relay_opens: cluster::link::Waiters<Result<(), String>>,
    /// the base world, as served to instances that join
    base_bytes: Vec<u8>,
    /// the layout version the saved world was built from
    world_layout: Option<u32>,
    /// the content pack clients download (its rules are what the shards run)
    pack: RwLock<Arc<pack::Pack>>,
}

/// A running server. Dropping it does not stop it; call `stop`.
/// What the server's ticks have cost: a count per quarter millisecond (the
/// last bucket holds everything of 250 ms and over), the sum and the worst.
#[derive(Clone, Debug)]
pub struct TickCost {
    buckets: Vec<u32>,
    pub ticks: u32,
    pub sum: Duration,
    pub worst: Duration,
}

impl Default for TickCost {
    fn default() -> Self {
        TickCost {
            buckets: vec![0; 1001],
            ticks: 0,
            sum: Duration::ZERO,
            worst: Duration::ZERO,
        }
    }
}

impl TickCost {
    fn add(&mut self, d: Duration) {
        let i = ((d.as_secs_f64() * 4000.0) as usize).min(self.buckets.len() - 1);
        self.buckets[i] += 1;
        self.ticks += 1;
        self.sum += d;
        self.worst = self.worst.max(d);
    }

    /// The ticks since an earlier reading.
    pub fn since(&self, old: &TickCost) -> TickCost {
        TickCost {
            buckets: self
                .buckets
                .iter()
                .zip(&old.buckets)
                .map(|(a, b)| a - b)
                .collect(),
            ticks: self.ticks - old.ticks,
            sum: self.sum - old.sum,
            // the worst of a part is not known: the worst so far
            worst: self.worst,
        }
    }

    pub fn mean_ms(&self) -> f64 {
        self.sum.as_secs_f64() * 1e3 / self.ticks.max(1) as f64
    }

    /// The cost in milliseconds that share `p` of the ticks stayed under
    /// (to the quarter millisecond; 250 for anything over).
    pub fn percentile_ms(&self, p: f64) -> f64 {
        let want = (self.ticks as f64 * p).ceil().max(1.0) as u32;
        let mut seen = 0;
        for (i, n) in self.buckets.iter().enumerate() {
            seen += n;
            if seen >= want {
                return (i + 1) as f64 * 0.25;
            }
        }
        0.0
    }
}

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
        if let Some(c) = &self.server.cluster {
            c.stop();
        }
    }

    /// Stop at once, like a crash: no save, no goodbyes (tests).
    pub fn kill(&self) {
        self.server.stop.store(true, Ordering::SeqCst);
        if let Some(c) = &self.server.cluster {
            c.stop();
        }
        let _ = TcpStream::connect_timeout(&self.local_addr, Duration::from_millis(200));
    }

    /// What the ticks have cost since the start (for load tests).
    pub fn tick_cost(&self) -> TickCost {
        self.server.ticks.lock().unwrap().clone()
    }

    /// The cluster this instance belongs to, if any.
    pub fn cluster(&self) -> Option<Arc<cluster::Cluster>> {
        self.server.cluster.clone()
    }
}

/// What instances of a cluster must agree on: the rules and every pack file.
fn content_identity(p: &pack::Pack) -> String {
    format!("{} v{} pack {}", p.content.name, p.content.version, p.id())
}

/// The size of the world a server makes: the layout's own, or the window
/// of it that `world_size` asks for (tests, small stages).
fn layout_size(cfg: &Config, l: &dyno_world::layout::Layout) -> [u32; 3] {
    let want = |i: usize| match cfg.world_size[i] {
        0 => l.size[i],
        n => n.min(l.size[i]),
    };
    [want(0), l.size[1], want(2)]
}

/// The world of a pack with a layout: the saved one if it was built from
/// this version of the layout, else a new one (an older saved world is
/// rebuilt, and everything saved moves to its place in the new one).
fn load_or_build(
    cfg: &Config,
    meta: &store::Meta,
    l: &dyno_world::layout::Layout,
    stamps: &dyno_world::stamp::Stamps,
) -> std::io::Result<(World, [i32; 3])> {
    let size = layout_size(cfg, l);
    let old = World::load(&cfg.world_path).ok();
    if let (Some(w), Some(h)) = (&old, meta.spawn_hearth)
        && meta.world_layout == Some(l.version)
        && w.size == size
    {
        log(format!(
            "Loaded world {} ({:?}, layout v{})",
            cfg.world_path.display(),
            w.size,
            l.version
        ));
        return Ok((old.unwrap_or_else(|| World::empty(size)), h));
    }
    let t = Instant::now();
    let (mut w, built) =
        dyno_world::worldgen::build(l, stamps, size).map_err(std::io::Error::other)?;
    // every section in its smallest form (shards share them)
    w.compact();
    log(format!(
        "Built the world {:?} from layout v{} in {:.1} s: {} places, {} stamps, {} trees",
        w.size,
        l.version,
        t.elapsed().as_secs_f32(),
        built.places.len(),
        built.stamps.len(),
        built.trees.len()
    ));
    if let Some(old) = old {
        // a world from another layout (or from before layouts): move what
        // was saved in it to the new one
        migrate::rebuild(&cfg.world_path, &old, meta, &w, &built, l)?;
    }
    if let Err(e) = w.save(&cfg.world_path) {
        log(format!(
            "Warning: cannot save world to {}: {e}",
            cfg.world_path.display()
        ));
    }
    // the hearth and the layout version go on disk with the world, so a
    // stop before the first save cannot leave the two out of step
    let mut m = store::load_meta(&store::meta_path(&cfg.world_path));
    m.spawn_hearth = Some(built.hearth);
    m.world_layout = Some(l.version);
    store::save_meta(&store::meta_path(&cfg.world_path), &m)?;
    Ok((w, built.hearth))
}

/// A world without a layout (builder worlds, other rules): terrain from
/// the seed, with a spawn hearth and the quest beacons of its rules.
fn load_or_generate(cfg: &Config, meta: &store::Meta, content: &Content) -> (World, [i32; 3]) {
    let size = if cfg.world_size.contains(&0) {
        [384, 96, 384]
    } else {
        cfg.world_size
    };
    let mut world = match World::load(&cfg.world_path) {
        Ok(w) => {
            log(format!(
                "Loaded world {} ({:?})",
                cfg.world_path.display(),
                w.size
            ));
            // an older, smaller world grows to the current map size, keeping everything
            if w.size[0] < size[0] && w.size[2] < size[2] && w.size[1] == size[1] {
                let grown = migrate::expand(&cfg.world_path, w, size, cfg.seed);
                if let Err(e) = grown.save(&cfg.world_path) {
                    log(format!("Warning: cannot save the grown world: {e}"));
                }
                grown
            } else {
                w
            }
        }
        Err(_) => {
            log(format!(
                "Generating a new world {size:?} with seed {}",
                cfg.seed
            ));
            let w = World::generate(size, cfg.seed);
            if let Err(e) = w.save(&cfg.world_path) {
                log(format!(
                    "Warning: cannot save world to {}: {e}",
                    cfg.world_path.display()
                ));
            }
            w
        }
    };
    // growing moves the saved hearth: read it again
    let meta_now = store::load_meta(&store::meta_path(&cfg.world_path));
    let known = meta_now.spawn_hearth.or(meta.spawn_hearth);
    let before = world.clone();
    let spawn_hearth = world.ensure_spawn_hearth(known);
    if world.get(spawn_hearth[0], spawn_hearth[1], spawn_hearth[2])
        != before.get(spawn_hearth[0], spawn_hearth[1], spawn_hearth[2])
    {
        log(format!("Built the spawn hearth at {spawn_hearth:?}"));
        if let Err(e) = world.save(&cfg.world_path) {
            log(format!("Warning: cannot save world: {e}"));
        }
    }
    drop(before);
    if rpg::build_landmarks(&mut world, content, spawn_hearth) {
        log("Built quest beacons");
        if let Err(e) = world.save(&cfg.world_path) {
            log(format!("Warning: cannot save world: {e}"));
        }
    }
    // built: every section in its smallest form (shards share them)
    world.compact();
    (world, spawn_hearth)
}

/// Start the server in background threads and return at once.
pub fn start(cfg: Config) -> std::io::Result<Handle> {
    let pack = pack::Pack::load(cfg.content_path.as_deref()).map_err(std::io::Error::other)?;
    let content: Content = pack.content.clone();
    // cluster mode: join (or found) the cluster first; a new instance takes
    // the base world from the cluster, checked against its hash
    let cluster = match &cfg.cluster {
        Some(cc) => {
            let me = cluster::Load {
                protocol: proto::PROTOCOL_VERSION,
                content: content_identity(&pack),
                version: env!("CARGO_PKG_VERSION").into(),
                capacity: (cfg.shard_cap.max(1) * cfg.max_shards.max(1)) as u32,
                ..Default::default()
            };
            let c = cluster::Cluster::start(cc.clone(), me).map_err(std::io::Error::other)?;
            log(format!(
                "Cluster: this is instance {} ({})",
                c.id(),
                if c.founder { "founder" } else { "member" }
            ));
            let base_file = cc.dir.join("base.dynw");
            if !c.founder && !base_file.exists() {
                let bytes = c.fetch_world().map_err(std::io::Error::other)?;
                std::fs::write(&base_file, &bytes)?;
                if !cfg.world_path.exists() {
                    if let Some(d) = cfg.world_path.parent() {
                        std::fs::create_dir_all(d)?;
                    }
                    World::decode(&bytes)
                        .map_err(std::io::Error::other)?
                        .save(&cfg.world_path)?;
                }
                log("Cluster: fetched the base world and checked its hash");
            }
            Some(c)
        }
        None => None,
    };
    let meta = store::load_meta(&store::meta_path(&cfg.world_path));
    // a protected world with a layout in its pack is built from that data
    let layout = pack.layout.clone().filter(|_| content.world.protected);
    let (world, spawn_hearth) = match &layout {
        Some(l) => {
            let (w, hearth) = load_or_build(&cfg, &meta, l, &pack.stamps)?;
            (w, hearth)
        }
        None => load_or_generate(&cfg, &meta, &content),
    };
    let world_layout = layout.as_ref().map(|l| l.version);
    // building or growing a world rewrites what is saved beside it
    let meta = store::load_meta(&store::meta_path(&cfg.world_path));
    // changes are noted, so the far view's tiles are sent again (stream.rs)
    let mut world = world;
    world.log_changes(true);
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
        "Content: {} v{} ({} items, {} recipes, {} creatures); pack {} v{}: {} files, {} KB, {}",
        content.name,
        content.version,
        content.items.len(),
        content.recipes.len(),
        content.creatures.len(),
        pack.manifest.pack,
        pack.manifest.version,
        pack.manifest.files.len(),
        pack.bytes() / 1024,
        pack.id()
    ));
    let mut state = State {
        world,
        content,
        players: HashMap::new(),
        observers: HashMap::new(),
        next_id: Arc::new(std::sync::atomic::AtomicU32::new(1)),
        dirty: false,
        saved: Arc::new(Mutex::new(saved)),
        containers: Arc::new(Mutex::new(containers)),
        entities: HashMap::new(),
        next_entity: 1,
        rng: Rng::new(cfg.seed as u64 ^ 0x5eed_1234),
        clock: if meta.day == 0 { 0.02 } else { meta.clock },
        day: meta.day.max(1),
        default_mode: cfg.default_mode,
        trust_movement: cfg.trust_movement,
        ops: cfg.ops.clone(),
        owner: meta.owner,
        light,
        hearths,
        events: meta.events.into_iter().collect(),
        webhook,
        meta_dirty: true,
        spawn_hearth,
        regrow: meta.regrow.clone(),
        spawn_points: Vec::new(),
        zone_clock: 0.0,
        depleted: HashMap::new(),
        incursion: None,
        next_phase: None,
        shard: 0,
        trades: Default::default(),
        trade_asks: Default::default(),
        corpses: Default::default(),
        rested_clock: 0.0,
        weather: (0, 0.0, 240.0),
        stream: cfg.stream,
        encoded: HashMap::new(),
        routines: HashMap::new(),
        routine_cost: Default::default(),
        far_encoded: HashMap::new(),
    };
    state.spawn_points = camps::plan_spawn_points(&state.world, &state.content, state.spawn_hearth);
    state.spawn_npcs();
    let rpg = state.content.world.protected;
    // the base world every shard copies; in a cluster it is fixed at founding
    let (base, base_light, base_bytes) = match (&cluster, &cfg.cluster) {
        (Some(c), Some(cc)) => {
            let file = cc.dir.join("base.dynw");
            let bytes = match std::fs::read(&file) {
                Ok(b) => b,
                Err(_) => {
                    let b = state.world.encode();
                    std::fs::write(&file, &b)?;
                    b
                }
            };
            let hash = cluster::sha256_hex(&bytes);
            if c.founder {
                let _ = c.propose(
                    cluster::machine::Cmd::WorldHash { hash: hash.clone() },
                    Duration::from_secs(5),
                );
                // bring this server's characters into the cluster (once)
                let players: Vec<(String, store::PlayerData)> = state
                    .saved
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let _ = c.propose(
                    cluster::machine::Cmd::Import { players },
                    Duration::from_secs(10),
                );
            }
            let want = c.shared.machine.read().unwrap().world_hash.clone();
            if want.as_ref().is_some_and(|w| *w != hash) {
                return Err(std::io::Error::other(
                    "this instance's base world does not match the cluster's",
                ));
            }
            let w = World::decode(&bytes).map_err(std::io::Error::other)?;
            let light = dyno_world::light::Light::new(&w);
            (w, light, bytes)
        }
        _ => (state.world.clone(), state.light.clone(), Vec::new()),
    };
    // every far tile built now, so no tick builds them (a join's first far
    // view comes in a burst); a new shard starts with the base world's
    let t = Instant::now();
    state.far_encoded = dyno_world::far::encode_all(&state.world);
    let base_far = if base_bytes.is_empty() {
        state.far_encoded.clone()
    } else {
        dyno_world::far::encode_all(&base)
    };
    log(format!(
        "Far view: {} tiles, {} KB, made in {} ms",
        state.far_encoded.len(),
        state
            .far_encoded
            .values()
            .map(|(_, d)| d.len())
            .sum::<usize>()
            / 1024,
        t.elapsed().as_millis()
    ));
    let server = Arc::new(Server {
        cfg,
        stop: AtomicBool::new(false),
        shards: RwLock::new(vec![Arc::new(ShardHost {
            id: 0,
            state: Mutex::new(state),
            empty_since: Mutex::new(None),
        })]),
        routes: Mutex::new(HashMap::new()),
        joining: Mutex::new(Default::default()),
        parties: Mutex::new(Default::default()),
        base,
        base_light,
        base_far,
        next_shard: AtomicU32::new(1),
        snaps: Mutex::new(Vec::new()),
        ticks: Mutex::new(TickCost::default()),
        channel_at: Mutex::new(HashMap::new()),
        rpg,
        cluster: cluster.clone(),
        epochs: Mutex::new(HashMap::new()),
        relays_out: Mutex::new(HashMap::new()),
        hosted: Mutex::new(HashMap::new()),
        relay_in: Mutex::new(HashMap::new()),
        next_conn: std::sync::atomic::AtomicU64::new(1),
        kicked: Mutex::new(Default::default()),
        fenced: AtomicBool::new(false),
        draining: AtomicBool::new(false),
        adoptions: Default::default(),
        relay_opens: Default::default(),
        base_bytes,
        pack: RwLock::new(Arc::new(pack)),
        world_layout,
    });
    if server.cluster.is_some() {
        clustered::attach(&server);
    }
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
    /// The pack clients download now.
    pub fn pack(&self) -> Arc<pack::Pack> {
        self.pack.read().unwrap().clone()
    }

    fn manifest(&self) -> proto::Manifest {
        self.pack().manifest.clone()
    }

    pub fn player_count(&self) -> usize {
        self.shard_list()
            .iter()
            .map(|s| s.state.lock().unwrap().players.len())
            .sum()
    }

    /// How many shards are open.
    pub fn shard_count(&self) -> usize {
        self.shards.read().unwrap().len()
    }

    /// Send a system message to everyone.
    pub fn announce(&self, text: &str) {
        for sh in self.shard_list() {
            sh.state.lock().unwrap().broadcast(
                &ServerMsg::System {
                    text: text.to_string(),
                },
                None,
            );
        }
    }

    fn shard_list(&self) -> Vec<Arc<ShardHost>> {
        self.shards.read().unwrap().clone()
    }

    fn shard(&self, sid: u32) -> Option<Arc<ShardHost>> {
        self.shards
            .read()
            .unwrap()
            .iter()
            .find(|s| s.id == sid)
            .cloned()
    }

    fn main_shard(&self) -> Arc<ShardHost> {
        self.shards.read().unwrap()[0].clone()
    }

    fn shard_of(&self, id: PlayerId) -> Option<Arc<ShardHost>> {
        let sid = self.routes.lock().unwrap().get(&id)?.shard;
        self.shard(sid)
    }

    /// Players per shard before another opens.
    fn cap(&self) -> usize {
        if self.rpg {
            self.cfg.shard_cap.max(1)
        } else {
            usize::MAX
        }
    }

    fn population(&self) -> HashMap<u32, usize> {
        let mut pop: HashMap<u32, usize> = self.shard_list().iter().map(|s| (s.id, 0)).collect();
        for r in self.routes.lock().unwrap().values() {
            *pop.entry(r.shard).or_default() += 1;
        }
        pop
    }

    /// The shard a joining player goes to: their party's, else the fullest
    /// one with room (busy worlds are fun), else a new one.
    fn place(&self, name: &str) -> Arc<ShardHost> {
        let cap = self.cap();
        let pop = self.population();
        let members = self.parties.lock().unwrap().members(name);
        {
            let routes = self.routes.lock().unwrap();
            for m in &members {
                if let Some(r) = routes.values().find(|r| r.name.eq_ignore_ascii_case(m))
                    && pop.get(&r.shard).copied().unwrap_or(0) < cap + party::MAX_PARTY
                    && let Some(sh) = self.shard(r.shard)
                {
                    return sh;
                }
            }
        }
        let mut open: Vec<(u32, usize)> = pop.into_iter().filter(|(_, n)| *n < cap).collect();
        open.sort_by_key(|(id, n)| (std::cmp::Reverse(*n), *id));
        if let Some((id, _)) = open.first()
            && let Some(sh) = self.shard(*id)
        {
            return sh;
        }
        self.open_shard().unwrap_or_else(|| self.main_shard())
    }

    /// Open another copy of the world.
    fn open_shard(&self) -> Option<Arc<ShardHost>> {
        if self.shard_count() >= self.cfg.max_shards.max(1) {
            return None;
        }
        let id = self.next_shard.fetch_add(1, Ordering::SeqCst);
        let seed = self.main_shard().state.lock().unwrap().fork_seed();
        let st = State::fork(
            seed,
            id,
            self.base.clone(),
            self.base_light.clone(),
            self.base_far.clone(),
        );
        let sh = Arc::new(ShardHost {
            id,
            state: Mutex::new(st),
            empty_since: Mutex::new(None),
        });
        self.shards.write().unwrap().push(sh.clone());
        log(format!("Opened shard {id}"));
        Some(sh)
    }

    /// Move a player to another shard, if they can go now. They stay
    /// connected; their client is told what changed.
    /// The spawn hearth of the main shard (content positions are relative to it).
    pub fn main_hearth(&self) -> [i32; 3] {
        self.main_shard().state.lock().unwrap().spawn_hearth
    }

    pub fn transfer(&self, id: PlayerId, to: u32) -> bool {
        let Some(from) = self.routes.lock().unwrap().get(&id).map(|r| r.shard) else {
            return false;
        };
        let (Some(a), Some(b)) = (self.shard(from), self.shard(to)) else {
            return false;
        };
        if from == to {
            return false;
        }
        // always lock the lower shard first
        let (mut ga, mut gb) = if from < to {
            let ga = a.state.lock().unwrap();
            (ga, b.state.lock().unwrap())
        } else {
            let gb = b.state.lock().unwrap();
            (a.state.lock().unwrap(), gb)
        };
        if !ga.movable(id) {
            return false;
        }
        let Some(p) = ga.detach(id) else {
            return false;
        };
        // only what differs in the sections the client holds
        let blocks = ga
            .world
            .diff(&gb.world)
            .into_iter()
            .filter(|(at, _)| p.view.holds_cell(*at))
            .collect();
        gb.attach(id, p, blocks);
        drop((ga, gb));
        if let Some(r) = self.routes.lock().unwrap().get_mut(&id) {
            r.shard = to;
        }
        true
    }

    /// Keep shards healthy: parties together, no area too crowded, quiet
    /// shards folded into busier ones, empty ones closed.
    fn balance(&self) {
        let cap = self.cap();
        if cap == usize::MAX {
            return;
        }
        let snaps = self.snaps.lock().unwrap().clone();
        let mut pop = self.population();
        let mut moves: Vec<(PlayerId, u32)> = Vec::new();
        let moving = |moves: &Vec<(PlayerId, u32)>, id: PlayerId| moves.iter().any(|m| m.0 == id);
        // 1. parties play in one shard
        let parties: Vec<Vec<String>> = self
            .parties
            .lock()
            .unwrap()
            .all()
            .map(|p| p.members.clone())
            .collect();
        let in_party: std::collections::HashSet<String> =
            parties.iter().flatten().map(|n| n.to_lowercase()).collect();
        for members in &parties {
            let here: Vec<&shard::Snap> = snaps
                .iter()
                .filter(|s| members.iter().any(|m| m.eq_ignore_ascii_case(&s.name)))
                .collect();
            if here.len() < 2 {
                continue;
            }
            let leader = here
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(&members[0]))
                .map(|s| s.shard);
            let target = leader.unwrap_or_else(|| {
                let mut count: HashMap<u32, usize> = HashMap::new();
                for s in &here {
                    *count.entry(s.shard).or_default() += 1;
                }
                count
                    .into_iter()
                    .max_by_key(|(id, n)| (*n, std::cmp::Reverse(*id)))
                    .unwrap()
                    .0
            });
            for s in &here {
                if s.shard != target
                    && s.movable
                    && pop.get(&target).copied().unwrap_or(0) < cap + party::MAX_PARTY
                {
                    moves.push((s.id, target));
                    *pop.entry(target).or_default() += 1;
                    *pop.entry(s.shard).or_default() -= 1;
                }
            }
        }
        // 2. no area crowded: spread newcomers to the quietest copy of that area
        let area_cap = self.cfg.area_cap.max(1);
        let mut dens: HashMap<(u32, usize), usize> = HashMap::new();
        for s in &snaps {
            if let Some(a) = s.area {
                *dens.entry((s.shard, a)).or_default() += 1;
            }
        }
        let crowded: Vec<((u32, usize), usize)> = dens
            .iter()
            .filter(|(_, n)| **n > area_cap)
            .map(|(k, n)| (*k, *n))
            .collect();
        for ((sid, area), n) in crowded {
            let mut extra = n - area_cap;
            let mut who: Vec<&shard::Snap> = snaps
                .iter()
                .filter(|s| {
                    s.shard == sid
                        && s.area == Some(area)
                        && s.movable
                        && !in_party.contains(&s.name.to_lowercase())
                        && !moving(&moves, s.id)
                })
                .collect();
            // the most recent arrivals move first (highest id joined last)
            who.sort_by_key(|s| std::cmp::Reverse(s.id));
            for s in who {
                if extra == 0 {
                    break;
                }
                let target = pop
                    .iter()
                    .filter(|(t, n)| {
                        **t != sid
                            && **n < cap
                            && dens.get(&(**t, area)).copied().unwrap_or(0) < area_cap
                    })
                    .min_by_key(|(t, _)| (dens.get(&(**t, area)).copied().unwrap_or(0), **t))
                    .map(|(t, _)| *t)
                    .or_else(|| self.open_shard().map(|sh| sh.id));
                let Some(t) = target else { break };
                pop.entry(t).or_default();
                moves.push((s.id, t));
                *pop.entry(t).or_default() += 1;
                *pop.entry(sid).or_default() -= 1;
                *dens.entry((t, area)).or_default() += 1;
                *dens.entry((sid, area)).or_default() -= 1;
                extra -= 1;
            }
        }
        // 3. fold a quiet shard into a busier one, if no area gets crowded
        let mut by_pop: Vec<(u32, usize)> = pop.iter().map(|(k, v)| (*k, *v)).collect();
        by_pop.sort_by_key(|(id, n)| (*n, std::cmp::Reverse(*id)));
        if let Some(&(small, n)) = by_pop.iter().find(|(_, n)| *n > 0)
            && n <= cap / 4
        {
            let fits = |t: u32| {
                dens.iter()
                    .filter(|((sh, _), _)| *sh == small)
                    .all(|((_, a), k)| dens.get(&(t, *a)).copied().unwrap_or(0) + k <= area_cap)
            };
            let target = by_pop
                .iter()
                .rev()
                .find(|(t, m)| *t != small && *m > 0 && m + n <= cap * 3 / 4 && fits(*t))
                .map(|(t, _)| *t);
            if let Some(t) = target {
                for s in snaps.iter().filter(|s| s.shard == small && s.movable) {
                    if !moving(&moves, s.id) {
                        moves.push((s.id, t));
                    }
                }
            }
        }
        for (id, to) in moves {
            if self.transfer(id, to) {
                log(format!("Moved player {id} to shard {to}"));
            }
        }
        // 4. close shards that have stood empty a while (never the first)
        let pop = self.population();
        let mut close = Vec::new();
        for sh in self.shard_list() {
            if sh.id == 0 {
                continue;
            }
            let mut since = sh.empty_since.lock().unwrap();
            if pop.get(&sh.id).copied().unwrap_or(0) == 0 {
                let t = since.get_or_insert_with(Instant::now);
                if t.elapsed() > Duration::from_secs(30) {
                    close.push(sh.id);
                }
            } else {
                *since = None;
            }
        }
        if !close.is_empty() {
            self.shards
                .write()
                .unwrap()
                .retain(|s| !close.contains(&s.id));
            for id in close {
                log(format!("Closed shard {id}"));
            }
        }
    }

    /// Party frames, once a second; shards learn who is grouped with whom.
    fn send_parties(&self) {
        {
            let parties = self.parties.lock().unwrap();
            for sh in self.shard_list() {
                let mut st = sh.state.lock().unwrap();
                for p in st.players.values_mut() {
                    p.party = parties.members(&p.name);
                }
            }
        }
        let snaps = self.snaps.lock().unwrap().clone();
        let routes = self.routes.lock().unwrap().clone();
        let parties: Vec<Vec<String>> = self
            .parties
            .lock()
            .unwrap()
            .all()
            .map(|p| p.members.clone())
            .collect();
        for members in parties {
            let list: Vec<proto::PartyMember> = members
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let s = snaps.iter().find(|s| s.name.eq_ignore_ascii_case(m));
                    proto::PartyMember {
                        name: s.map(|s| s.name.clone()).unwrap_or_else(|| m.clone()),
                        level: s.map(|s| s.level).unwrap_or(0),
                        class: s.and_then(|s| s.class),
                        health: s.map(|s| s.health).unwrap_or(0.0),
                        max: s.map(|s| s.max).unwrap_or(1.0),
                        online: s.is_some(),
                        leader: i == 0,
                    }
                })
                .collect();
            let bytes = Arc::new(proto::encode(&ServerMsg::Party { members: list }));
            for r in routes.values() {
                if members.iter().any(|m| m.eq_ignore_ascii_case(&r.name)) {
                    let _ = r.tx.send(bytes.clone());
                }
            }
        }
    }

    /// Tell one connected player something, wherever they are.
    fn tell_name(&self, name: &str, msg: &ServerMsg) -> bool {
        let routes = self.routes.lock().unwrap();
        match routes.values().find(|r| r.name.eq_ignore_ascii_case(name)) {
            Some(r) => {
                let _ = r.tx.send(Arc::new(proto::encode(msg)));
                true
            }
            None => false,
        }
    }

    /// Commands that reach across shards. Returns false if `line` is not one.
    fn global_command(&self, id: PlayerId, line: &str) -> bool {
        let me = match self.routes.lock().unwrap().get(&id) {
            Some(r) => r.name.clone(),
            None => return true,
        };
        let mut parts = line.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
        let rest = parts.next().unwrap_or("").trim().to_string();
        let sys = |text: String| ServerMsg::System { text };
        let tell = |text: String| {
            self.tell_name(&me, &sys(text));
        };
        match cmd.as_str() {
            "/invite" => {
                let who = rest.split_whitespace().next().unwrap_or("").to_string();
                let online = self
                    .routes
                    .lock()
                    .unwrap()
                    .values()
                    .find(|r| r.name.eq_ignore_ascii_case(&who))
                    .map(|r| r.name.clone());
                match online {
                    None => tell(format!("No one called '{who}' is online.")),
                    Some(who) => match self.parties.lock().unwrap().invite(&me, &who) {
                        Ok(()) => {
                            self.tell_name(&who, &ServerMsg::Invite { from: me.clone() });
                            tell(format!("You invite {who} to your party."));
                        }
                        Err(e) => tell(e),
                    },
                }
            }
            "/accept" => {
                let r = self.parties.lock().unwrap().accept(&me);
                match r {
                    Ok(members) => {
                        for m in &members {
                            self.tell_name(m, &sys(format!("{me} joins the party.")));
                        }
                        self.send_parties();
                    }
                    Err(e) => tell(e),
                }
            }
            "/decline" => {
                if let Some(from) = self.parties.lock().unwrap().decline(&me) {
                    self.tell_name(&from, &sys(format!("{me} declines your invitation.")));
                }
            }
            "/leave" | "/leaveparty" => {
                let left = self.parties.lock().unwrap().leave(&me);
                self.tell_name(
                    &me,
                    &ServerMsg::Party {
                        members: Vec::new(),
                    },
                );
                tell("You leave the party.".into());
                for m in &left {
                    self.tell_name(m, &sys(format!("{me} leaves the party.")));
                    if left.len() <= 1 {
                        self.tell_name(
                            m,
                            &ServerMsg::Party {
                                members: Vec::new(),
                            },
                        );
                    }
                }
                self.send_parties();
            }
            "/p" | "/party" => {
                let members = self.parties.lock().unwrap().members(&me);
                if members.is_empty() {
                    tell("You are not in a party.".into());
                } else {
                    let text: String = rest.chars().take(proto::MAX_CHAT).collect();
                    for m in &members {
                        self.tell_name(
                            m,
                            &ServerMsg::Chat {
                                from: format!("[Party] {me}"),
                                text: text.clone(),
                            },
                        );
                    }
                }
            }
            // world channels reach everyone in every shard
            "/1" | "/g" | "/general" | "/2" | "/t" | "/trade" => {
                let channel = if matches!(cmd.as_str(), "/2" | "/t" | "/trade") {
                    "Trade"
                } else {
                    "General"
                };
                let text: String = rest
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(proto::MAX_CHAT)
                    .collect();
                if text.trim().is_empty() {
                    return true;
                }
                {
                    let mut at = self.channel_at.lock().unwrap();
                    if at
                        .get(&id)
                        .is_some_and(|t| t.elapsed() < Duration::from_millis(1000))
                    {
                        tell("You are speaking too quickly.".into());
                        return true;
                    }
                    at.insert(id, Instant::now());
                }
                log(format!("[{channel}] <{me}> {text}"));
                let bytes = Arc::new(proto::encode(&ServerMsg::Chat {
                    from: format!("[{channel}] {me}"),
                    text,
                }));
                for r in self.routes.lock().unwrap().values() {
                    let _ = r.tx.send(bytes.clone());
                }
            }
            "/w" | "/whisper" | "/tell" => {
                let mut it = rest.splitn(2, ' ');
                let who = it.next().unwrap_or("").to_string();
                let text: String = it
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(proto::MAX_CHAT)
                    .collect();
                let sent = self.tell_name(
                    &who,
                    &ServerMsg::Chat {
                        from: format!("[From] {me}"),
                        text: text.clone(),
                    },
                );
                if sent {
                    tell(format!("[To {who}] {text}"));
                } else {
                    tell(format!("No one called '{who}' is online."));
                }
            }
            "/who" => {
                let routes = self.routes.lock().unwrap();
                let mut names: Vec<String> = routes.values().map(|r| r.name.clone()).collect();
                names.sort();
                let here = routes.get(&id).map(|r| r.shard);
                let near = routes.values().filter(|r| Some(r.shard) == here).count();
                drop(routes);
                tell(format!(
                    "{} online ({} near you): {}",
                    names.len(),
                    near,
                    names.join(", ")
                ));
            }
            "/instance" => {
                let op = self.main_shard().state.lock().unwrap().may_op(&me);
                match (&self.cluster, rest.parse::<u64>()) {
                    (None, _) => tell("This server is not part of a cluster.".into()),
                    (Some(_), Ok(to)) if op => {
                        if let Err(e) = self.transfer_remote(id, to) {
                            log(format!(
                                "Cluster: could not move {me} to instance {to}: {e}"
                            ));
                            tell(format!("Cannot move: {e}"));
                        }
                    }
                    (Some(c), _) => tell(format!(
                        "You are on instance {} of {}.",
                        c.id(),
                        c.shared.machine.read().unwrap().alive().len()
                    )),
                }
            }
            "/shard" => {
                let here = self
                    .routes
                    .lock()
                    .unwrap()
                    .get(&id)
                    .map(|r| r.shard)
                    .unwrap_or(0);
                let op = self.main_shard().state.lock().unwrap().may_op(&me);
                match rest.parse::<u32>() {
                    Ok(to) if op => {
                        let to = if self.shard(to).is_some() {
                            Some(to)
                        } else {
                            self.open_shard().map(|s| s.id)
                        };
                        match to {
                            Some(to) if self.transfer(id, to) => {
                                tell(format!("You are now in shard {to}."))
                            }
                            _ => tell("You cannot move right now.".into()),
                        }
                    }
                    _ => tell(format!(
                        "You are in shard {here} of {}.",
                        self.shard_count()
                    )),
                }
            }
            _ => return false,
        }
        true
    }

    pub fn save(&self) {
        // everyone, in every shard, is written to the shared store first
        for sh in self.shard_list().iter().skip(1) {
            let mut st = sh.state.lock().unwrap();
            let ids: Vec<PlayerId> = st.players.keys().copied().collect();
            for id in ids {
                let p = st.players.remove(&id).unwrap();
                st.remember(&p);
                st.players.insert(id, p);
            }
        }
        let main = self.main_shard();
        let (world, players, chests, content, meta) = {
            let mut st = main.state.lock().unwrap();
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
                world_layout: self.world_layout,
            };
            (
                world,
                st.saved.lock().unwrap().clone(),
                st.containers.lock().unwrap().clone(),
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
        // DYNO_TICK_STATS: what a whole tick costs (sum, worst, ticks), once a minute
        let tick_stats = std::env::var_os("DYNO_TICK_STATS").is_some();
        let mut cost = (Duration::ZERO, Duration::ZERO, 0u32);
        let mut cost_at = TickCost::default();
        while !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));
            n += 1;
            let dt = last_tick.elapsed().as_secs_f32().min(0.25);
            last_tick = Instant::now();
            if tick_stats && cost.2 >= 1200 {
                let players: usize = self
                    .shard_list()
                    .iter()
                    .map(|sh| sh.state.lock().unwrap().players.len())
                    .sum();
                let all = self.ticks.lock().unwrap().clone();
                log(format!(
                    "tick: mean {:.2} ms, p95 {:.2} ms, worst {:.1} ms over {} ticks, {players} players",
                    cost.0.as_secs_f64() * 1e3 / cost.2 as f64,
                    all.since(&cost_at).percentile_ms(0.95),
                    cost.1.as_secs_f64() * 1e3,
                    cost.2
                ));
                cost_at = all;
                cost = (Duration::ZERO, Duration::ZERO, 0);
            }
            let mut snaps = Vec::new();
            let main_clock = {
                let st = self.main_shard();
                let st = st.state.lock().unwrap();
                (st.clock, st.day)
            };
            for sh in self.shard_list() {
                let mut st = sh.state.lock().unwrap();
                // every shard shares the main shard's day
                if sh.id != 0 {
                    st.clock = main_clock.0;
                    st.day = main_clock.1;
                }
                st.tick(dt);
                // DYNO_TICK_STATS: what the NPC routines cost, once a minute
                if n.is_multiple_of(1200) && std::env::var_os("DYNO_TICK_STATS").is_some() {
                    let (mean, walked) = st.routine_cost();
                    log(format!(
                        "routines: {walked} NPCs, {:.1} microseconds a tick",
                        mean.as_secs_f64() * 1e6
                    ));
                }
                if n.is_multiple_of(10) {
                    snaps.extend(st.snapshot());
                }
                // poses of the players near each player; everyone to observers
                st.send_poses();
                // the world and the entities around each player (stream.rs)
                st.update_interest();
                st.stream_sections(dt);
                if n.is_multiple_of(2)
                    && let Some(m) = st.entity_moves()
                {
                    st.broadcast(&m, None);
                }
            }
            if n.is_multiple_of(10) {
                *self.snaps.lock().unwrap() = snaps;
                self.send_parties();
            }
            if n.is_multiple_of(40) {
                self.balance();
            }
            if last_time.elapsed() > Duration::from_secs(10) {
                last_time = Instant::now();
                for sh in self.shard_list() {
                    let st = sh.state.lock().unwrap();
                    st.broadcast(&st.time_msg(), None);
                }
            }
            // the save is not part of the tick's cost: it runs every 10 s
            let spent = last_tick.elapsed();
            cost = (cost.0 + spent, cost.1.max(spent), cost.2 + 1);
            self.ticks.lock().unwrap().add(spent);
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
        let (token, observer, create) = match read_msg(&mut ws)? {
            ClientMsg::Session {
                token,
                observer,
                create,
            } => (token, observer, create),
            // the character select screen: list (or delete) my characters, then close
            ClientMsg::ListCharacters { token } => {
                let msg = self
                    .main_shard()
                    .state
                    .lock()
                    .unwrap()
                    .characters(token, &self.cfg.motd);
                send(&mut ws, &msg)?;
                let _ = ws.close(None);
                return Ok(());
            }
            ClientMsg::DeleteCharacter { token, name } => {
                let online = self
                    .routes
                    .lock()
                    .unwrap()
                    .values()
                    .any(|r| r.name.eq_ignore_ascii_case(&name));
                let main = self.main_shard();
                let msg = {
                    let mut st = main.state.lock().unwrap();
                    if !online {
                        st.delete_character(token, &name);
                    }
                    st.characters(token, &self.cfg.motd)
                };
                send(&mut ws, &msg)?;
                let _ = ws.close(None);
                return Ok(());
            }
            _ => return Err("second message was not session".into()),
        };
        let name = proto::clean_name(&name);
        let (tx, rx) = channel::<Arc<Vec<u8>>>();
        let conn = self.next_conn.fetch_add(1, Ordering::SeqCst);
        // cluster mode: the player may live on another instance
        if !observer && self.cluster.is_some() {
            if let Err(reason) = self.cluster_ready() {
                send(&mut ws, &ServerMsg::Reject { reason })?;
                let _ = ws.close(None);
                return Err("not serving".into());
            }
            let host = self.choose_host(&name);
            if Some(host) != self.cluster.as_ref().map(|c| c.id()) {
                if let Err(reason) = self.open_remote(host, conn, &name, token, create, tx) {
                    send(
                        &mut ws,
                        &ServerMsg::Reject {
                            reason: reason.clone(),
                        },
                    )?;
                    let _ = ws.close(None);
                    return Err(format!("refused {name}: {reason}"));
                }
                log(format!(
                    "{name} joined from {peer} (dyno {version}), relayed to instance {host}"
                ));
                let result = self.session(&mut ws, 0, conn, Vec::new(), false, rx);
                self.close_conn(conn, "left");
                log(format!(
                    "{name} left ({})",
                    result.as_ref().err().map(String::as_str).unwrap_or("quit")
                ));
                let _ = ws.close(None);
                let _ = ws.flush();
                return Ok(());
            }
        }

        let (id, first) = {
            if !observer {
                let playing = self
                    .routes
                    .lock()
                    .unwrap()
                    .values()
                    .any(|r| r.name.eq_ignore_ascii_case(&name));
                if playing || !self.joining.lock().unwrap().insert(name.to_lowercase()) {
                    send(
                        &mut ws,
                        &ServerMsg::Reject {
                            reason: format!("{name} is already playing. Choose another name."),
                        },
                    )?;
                    return Err("already playing".into());
                }
            }
            if !observer && let Err(reason) = self.claim_here(&name, token) {
                self.joining.lock().unwrap().remove(&name.to_lowercase());
                send(
                    &mut ws,
                    &ServerMsg::Reject {
                        reason: reason.clone(),
                    },
                )?;
                let _ = ws.close(None);
                return Err(format!("refused {name}: {reason}"));
            }
            let sh = if observer {
                self.main_shard()
            } else {
                self.place(&name)
            };
            let mut st = sh.state.lock().unwrap();
            if observer {
                let id = st.next_id.fetch_add(1, Ordering::SeqCst);
                st.observers.insert(
                    id,
                    game::Observer {
                        tx,
                        view: Default::default(),
                    },
                );
                let welcome = ServerMsg::Welcome {
                    your_id: id,
                    server_version: env!("CARGO_PKG_VERSION").into(),
                    motd: self.cfg.motd.clone(),
                    size: st.world.size,
                    spawn: st.spawn_point(),
                    players: st
                        .players
                        .iter()
                        .map(|(i, p)| st.player_info(*i, p))
                        .collect(),
                    mode: Mode::Creative,
                    manifest: self.manifest(),
                    hearth: st.spawn_hearth,
                    view: st.stream.view,
                };
                // a tool sees what stands in the world (in no story phase),
                // as it sees the world itself
                let mut first = vec![welcome];
                for (eid, e) in st.entities.iter().filter(|(_, e)| e.phase.is_none()) {
                    first.extend(st.entity_intro(*eid, e));
                }
                (id, first)
            } else {
                if self.routes.lock().unwrap().len() >= self.cfg.max_players.max(self.cap()) {
                    drop(st);
                    self.joining.lock().unwrap().remove(&name.to_lowercase());
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
                let joined = st.join(&name, token, tx.clone(), create);
                self.joining.lock().unwrap().remove(&name.to_lowercase());
                match joined {
                    Ok((id, mut msgs)) => {
                        if let Some(ServerMsg::Welcome { motd, manifest, .. }) = msgs.first_mut() {
                            *motd = self.cfg.motd.clone();
                            *manifest = self.manifest();
                        }
                        let display = st
                            .players
                            .get(&id)
                            .map(|p| p.name.clone())
                            .unwrap_or(name.clone());
                        self.routes.lock().unwrap().insert(
                            id,
                            Route {
                                shard: sh.id,
                                name: display,
                                tx,
                                conn,
                                edge: None,
                            },
                        );
                        (id, msgs)
                    }
                    Err(reason) => {
                        drop(st);
                        self.release_claim(&name);
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
        let result = self.session(&mut ws, id, conn, first, true, rx);
        if self.relays_out.lock().unwrap().contains_key(&conn) {
            // the player moved to another instance during the session
            self.close_conn(conn, "left");
        } else {
            let id = self.local_id(conn).unwrap_or(id);
            self.leave_here(id);
        }
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

    #[allow(clippy::too_many_arguments)]
    fn session(
        &self,
        ws: &mut WebSocket<TcpStream>,
        id: PlayerId,
        conn: u64,
        first: Vec<ServerMsg>,
        // the player lives here (false: relayed to another instance)
        local: bool,
        rx: Receiver<Arc<Vec<u8>>>,
    ) -> Result<(), String> {
        // a relayed session's welcome comes from the host: it names this
        // edge's pack, since this edge serves the files
        let mut patch_welcome = !local;
        let mut outbox = pack::Outbox::new(self.cfg.join_asset_rate.max(self.cfg.asset_rate));
        let mut playing = false;
        let mut first = first.into_iter();
        if let Some(welcome) = first.next() {
            send(ws, &welcome)?;
        }
        if local {
            // the world follows section by section (stream.rs)
            let time = self
                .shard_of(id)
                .unwrap_or_else(|| self.main_shard())
                .state
                .lock()
                .unwrap()
                .time_msg();
            send(ws, &time)?;
        }
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
            if self.kicked.lock().unwrap().remove(&conn) {
                // tell the client why, then end the session
                let _ = send(
                    ws,
                    &ServerMsg::Reject {
                        reason: "Lost contact with the world. Reconnect in a moment.".into(),
                    },
                );
                return Err("dropped".into());
            }
            match ws.read() {
                Ok(Message::Binary(b)) => {
                    last_msg = Instant::now();
                    msgs += 1;
                    let msg = proto::decode::<ClientMsg>(&b);
                    // pack files come from this instance, wherever the player lives
                    let want = matches!(msg, Ok(ClientMsg::Want { .. }));
                    if !want && !playing {
                        // the player is in the world now: play comes first
                        playing = true;
                        outbox.set_rate(self.cfg.asset_rate);
                    }
                    if let Ok(ClientMsg::Want { hashes }) = &msg {
                        outbox.want(hashes, &self.pack());
                    } else if self.relay_host(conn).is_some() {
                        // the player lives on another instance: pass it on untouched
                        self.relay_up(conn, b.to_vec());
                    } else {
                        let msg = msg.map_err(|e| format!("bad message: {e}"))?;
                        if matches!(msg, ClientMsg::Hello { .. } | ClientMsg::Session { .. }) {
                            return Err("second hello".into());
                        }
                        if let ClientMsg::Chat { .. } = msg {
                            chats += 1;
                        }
                        // the player id changes if they moved away and came back
                        let id = self.local_id(conn).unwrap_or(id);
                        self.client_msg(id, msg, chats)?;
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
                let bytes = if patch_welcome {
                    patch_welcome = false;
                    self.edge_welcome(bytes)
                } else {
                    bytes
                };
                ws.write(Message::Binary((*bytes).clone().into()))
                    .map_err(|e| format!("write: {e}"))?;
                wrote = true;
            }
            // then pack files, with what the budget leaves
            if !outbox.is_empty() {
                for m in outbox.chunks(&self.pack()) {
                    ws.write(Message::Binary(proto::encode(&m).into()))
                        .map_err(|e| format!("write: {e}"))?;
                    wrote = true;
                }
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

    /// A relayed welcome, naming this edge's pack.
    fn edge_welcome(&self, bytes: Arc<Vec<u8>>) -> Arc<Vec<u8>> {
        match proto::decode::<ServerMsg>(&bytes) {
            Ok(ServerMsg::Welcome {
                your_id,
                server_version,
                motd,
                size,
                spawn,
                players,
                mode,
                hearth,
                view,
                ..
            }) => Arc::new(proto::encode(&ServerMsg::Welcome {
                your_id,
                server_version,
                motd,
                size,
                spawn,
                players,
                mode,
                manifest: self.manifest(),
                hearth,
                view,
            })),
            _ => bytes,
        }
    }

    /// One message from a player (from a local session or a relay).
    /// `chats` counts chat lines in the current second.
    fn client_msg(&self, id: PlayerId, msg: ClientMsg, chats: u32) -> Result<(), String> {
        match msg {
            ClientMsg::Chat { text } => {
                let text: String = text
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(proto::MAX_CHAT)
                    .collect();
                if chats <= 20 && text.trim_start().starts_with('/') {
                    if !self.global_command(id, text.trim()) {
                        self.command(id, text.trim());
                    }
                } else if chats <= 5 && !text.trim().is_empty() {
                    let Some(sh) = self.shard_of(id) else {
                        return Ok(());
                    };
                    let st = sh.state.lock().unwrap();
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
            other => {
                // observers (tools) live on the main shard, outside the routes
                let sh = self.shard_of(id).or_else(|| {
                    matches!(other, ClientMsg::ViewAt { .. }).then(|| self.main_shard())
                });
                if let Some(sh) = sh {
                    sh.state.lock().unwrap().handle(id, other)?;
                }
            }
        }
        Ok(())
    }

    fn command(&self, id: PlayerId, line: &str) {
        let Some(sh) = self.shard_of(id) else { return };
        let mut st = sh.state.lock().unwrap();
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
            "/help" => st.tell(id, "Chat: /1 General, /2 Trade, /p party, /w name whisper. Party: /invite name, /accept, /decline, /leave. Trade: /trade name. Commands: /who, /tp <player>, /spawn, /time <day|noon|sunset|night|midnight|0-1>, /who, /kill, /gamemode <survival|creative>, /give <item> [count], /summon <creature>, /slay, /money <copper>, /xp <amount>, /reload"),
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
            "/event" if op => {
                if arg.trim() == "incursion" {
                    st.maybe_start_incursion();
                    if let Some(inc) = st.incursion.as_mut() {
                        inc.next_wave = Instant::now() + Duration::from_secs(3);
                    }
                } else {
                    st.tell(id, "Usage: /event incursion");
                }
            }
            "/weather" if op => {
                let kind = match arg.trim() {
                    "clear" => Some(0),
                    "cloudy" | "clouds" => Some(1),
                    "rain" => Some(2),
                    "fog" | "mist" => Some(3),
                    _ => None,
                };
                match kind {
                    Some(k) => {
                        st.weather = (k, 1.0, 600.0);
                        st.broadcast(&ServerMsg::Weather { kind: k, strength: 1.0 }, None);
                    }
                    None => st.tell(id, "Usage: /weather clear|cloudy|rain|fog"),
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
            "/slay" => {
                if !op {
                    return st.tell(id, "Only the world's owner can slay creatures.");
                }
                // the nearest creature within 20 blocks falls, as if you had killed it
                let me_at = st.players[&id].pose.pos;
                let near = st
                    .entities
                    .iter()
                    .filter(|(_, e)| e.mob.is_some())
                    .map(|(eid, e)| (*eid, game::dist(e.pos, me_at)))
                    .filter(|(_, d)| *d < 20.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                match near {
                    Some((eid, _)) => {
                        if let Some(m) = st.entities.get_mut(&eid).and_then(|e| e.mob.as_mut()) {
                            m.last_hit = Some((id, std::time::Instant::now()));
                            m.tag = Some((id, std::time::Instant::now()));
                        }
                        st.kill_mob_by(eid);
                    }
                    None => st.tell(id, "No creature near enough."),
                }
            }
            "/finish" => {
                if !op {
                    return st.tell(id, "Only the world's owner can finish quests by decree.");
                }
                let key = arg.trim();
                match st.content.quests.iter().find(|q| q.key == key).map(|q| q.id) {
                    Some(q) => {
                        if let Some(p) = st.players.get_mut(&id) {
                            p.quests.retain(|x| x.quest != q);
                            p.done.insert(q);
                        }
                        st.send_quests(id);
                        st.refresh_phases(id);
                        st.tell(id, format!("Finished {key}."));
                    }
                    None => st.tell(id, "Usage: /finish <quest key>"),
                }
            }
            "/money" => {
                if !op {
                    return st.tell(id, "Only the world's owner can mint coins.");
                }
                match arg.trim().parse::<u32>() {
                    Ok(n) => {
                        if let Some(p) = st.players.get_mut(&id) {
                            p.money = p.money.saturating_add(n);
                        }
                        st.send_character(id);
                    }
                    Err(_) => st.tell(id, "Usage: /money <copper>"),
                }
            }
            "/xp" => {
                if !op {
                    return st.tell(id, "Only the world's owner can grant experience.");
                }
                match arg.trim().parse::<u32>() {
                    Ok(n) => st.reward(id, n, 0),
                    Err(_) => st.tell(id, "Usage: /xp <amount>"),
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
                match pack::Pack::load(self.cfg.content_path.as_deref()) {
                    Ok(pk) => {
                        let c = pk.content.clone();
                        let summary = format!("{} v{}: {} items, {} recipes", c.name, c.version, c.items.len(), c.recipes.len());
                        let manifest = pk.manifest.clone();
                        *self.pack.write().unwrap() = Arc::new(pk);
                        st.replace_content(c, manifest);
                        st.broadcast(&ServerMsg::System { text: format!("{me} reloaded content ({summary})") }, None);
                    }
                    Err(e) => st.tell(id, format!("Reload failed: {e}")),
                }
            }
            _ => st.tell(id, "Unknown command. Try /help"),
        }
    }
}
