//! Many server instances, one world. Each instance is an edge (players
//! connect to it), a shard host and a member of the cluster. Members agree
//! through Raft on a small replicated state (machine.rs): who is in the
//! cluster, which instance hosts each character (a lock with an epoch) and
//! the characters themselves. Everything else (players moving about, relayed
//! traffic, load) flows directly over the authenticated cluster link.
//!
//! An instance started without cluster options never creates any of this.

pub mod link;
pub mod machine;
pub mod raftnode;

use link::{Envelope, Link, Waiters};
use machine::{Cmd, Member, NodeId, Outcome};
use raftnode::{In, Outbox, Prop, Shared};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

/// How long an instance may go without hearing the quorum before it fences
/// itself (stops writing, drops its players).
pub const FENCE_AFTER: Duration = Duration::from_secs(5);
/// How long the leader waits before declaring a silent instance down and
/// releasing its characters. Always well above `FENCE_AFTER`.
pub const DOWN_AFTER: Duration = Duration::from_secs(12);

#[derive(Clone, Debug)]
pub struct ClusterConfig {
    /// where the cluster link listens
    pub listen: SocketAddr,
    /// how other instances reach it (defaults to `listen`)
    pub advertise: Option<String>,
    /// an instance already in the cluster (absent: start a new cluster)
    pub join: Option<String>,
    /// shared secret; never logged
    pub secret: Vec<u8>,
    /// where this instance keeps its Raft log, id and base world
    pub dir: PathBuf,
}

/// What an instance tells the others every second.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Load {
    pub node: NodeId,
    pub protocol: u32,
    pub content: String,
    pub version: String,
    pub capacity: u32,
    /// (shard, players) for shards it hosts
    pub shards: Vec<(u32, u32)>,
    /// lower-case names it hosts
    pub players: Vec<String>,
    pub draining: bool,
    pub fenced: bool,
}

impl Load {
    pub fn total(&self) -> u32 {
        self.shards.iter().map(|s| s.1).sum()
    }
}

/// Messages between instances that are about players, not consensus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum App {
    /// an edge asks a host to take a player (the host claims and joins them)
    RelayOpen {
        conn: u64,
        name: String,
        token: u64,
        create: Option<dyno_protocol::NewCharacter>,
    },
    RelayOpened {
        conn: u64,
        result: Result<(), String>,
    },
    /// a client message, edge → host (encoded ClientMsg)
    RelayClient { conn: u64, bytes: Vec<u8> },
    /// a server message, host → edge (encoded ServerMsg)
    RelayServer { conn: u64, bytes: Vec<u8> },
    /// the relay is over (either side)
    RelayClose { conn: u64, reason: String },
    /// the player behind `conn` now lives on `host`
    RelayMoved { conn: u64, host: NodeId },
    /// take a player mid-session (a seamless move between instances)
    Adopt {
        req: u64,
        name: String,
        token: u64,
        epoch: u64,
        data: Box<crate::store::PlayerData>,
        edge: NodeId,
        conn: u64,
        /// blocks the old host's world changed from the base
        changed: Vec<[i32; 3]>,
    },
    AdoptResult {
        req: u64,
        result: Result<(), String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Wire {
    Raft(Vec<u8>),
    Forward(Prop),
    Join {
        req: u64,
        reply: String,
        member: Member,
    },
    JoinOk {
        req: u64,
        id: NodeId,
        members: Vec<Member>,
    },
    JoinErr {
        req: u64,
        why: String,
    },
    FetchWorld {
        req: u64,
        reply: String,
    },
    World {
        req: u64,
        bytes: Vec<u8>,
    },
    Load(Load),
    App(App),
}

type AppHandler = Arc<dyn Fn(NodeId, App) + Send + Sync>;
type WorldSource = Arc<dyn Fn() -> Option<Vec<u8>> + Send + Sync>;

pub struct Cluster {
    id: AtomicU64,
    pub shared: Arc<Shared>,
    pub link: Arc<Link<Wire>>,
    raft: Mutex<Option<Sender<In>>>,
    proposals: Waiters<Outcome>,
    joins: Waiters<Result<(NodeId, Vec<Member>), String>>,
    worlds: Waiters<Vec<u8>>,
    app: OnceLock<AppHandler>,
    world: OnceLock<WorldSource>,
    /// last time each instance was heard from
    seen: Mutex<HashMap<NodeId, Instant>>,
    /// latest load report from each instance
    pub loads: Mutex<HashMap<NodeId, Load>>,
    /// addresses learned before the replicated member list arrives
    known: Mutex<HashMap<NodeId, String>>,
    /// this instance's own report
    pub me: Mutex<Load>,
    stop: AtomicBool,
    /// whether this instance created the cluster
    pub founder: bool,
    /// (leader, since) for the down-timer grace period
    leading: Mutex<Option<(u64, Instant)>>,
}

struct Out(Weak<Cluster>);

impl Outbox for Out {
    fn raft(&self, to: NodeId, msg: Vec<u8>) {
        if let Some(c) = self.0.upgrade()
            && let Some(a) = c.addr_of(to)
        {
            c.link.send(&a, Wire::Raft(msg));
        }
    }
    fn forward(&self, leader: NodeId, prop: Prop) {
        if let Some(c) = self.0.upgrade()
            && let Some(a) = c.addr_of(leader)
        {
            c.link.send(&a, Wire::Forward(prop));
        }
    }
    fn applied(&self, req: u64, outcome: Outcome) {
        if let Some(c) = self.0.upgrade() {
            c.proposals.answer(req, outcome);
        }
    }
}

fn read_id(dir: &std::path::Path) -> Option<NodeId> {
    std::fs::read_to_string(dir.join("node.id"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

impl Cluster {
    /// Join or create the cluster. `me` describes this instance (versions,
    /// capacity).
    pub fn start(cfg: ClusterConfig, me: Load) -> Result<Arc<Cluster>, String> {
        std::fs::create_dir_all(&cfg.dir).map_err(|e| e.to_string())?;
        let advertise = cfg
            .advertise
            .clone()
            .unwrap_or_else(|| cfg.listen.to_string());
        let saved_id = read_id(&cfg.dir);
        let founder = saved_id.is_none() && cfg.join.is_none() || saved_id == Some(1);
        let mut err = None;
        let cluster = Arc::new_cyclic(|weak: &Weak<Cluster>| {
            let w = weak.clone();
            let link = match Link::start(
                cfg.listen,
                advertise.clone(),
                cfg.secret.clone(),
                Arc::new(move |env: Envelope<Wire>| {
                    if let Some(c) = w.upgrade() {
                        c.receive(env);
                    }
                }),
            ) {
                Ok(l) => l,
                Err(e) => {
                    err = Some(format!("cluster link on {}: {e}", cfg.listen));
                    Link::start(
                        "127.0.0.1:0".parse().unwrap(),
                        String::new(),
                        Vec::new(),
                        Arc::new(|_| {}),
                    )
                    .expect("loopback listener")
                }
            };
            Cluster {
                id: AtomicU64::new(0),
                shared: Shared::new(),
                link,
                raft: Mutex::new(None),
                proposals: Waiters::default(),
                joins: Waiters::default(),
                worlds: Waiters::default(),
                app: OnceLock::new(),
                world: OnceLock::new(),
                seen: Mutex::new(HashMap::new()),
                loads: Mutex::new(HashMap::new()),
                known: Mutex::new(HashMap::new()),
                me: Mutex::new(me.clone()),
                stop: AtomicBool::new(false),
                founder,
                leading: Mutex::new(None),
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        // who am I: saved, the founder (1), or assigned by the cluster
        let (id, bootstrap) = match (saved_id, &cfg.join) {
            (Some(id), _) => (id, false),
            (None, None) => (1, true),
            (None, Some(seed)) => {
                let member = Member {
                    id: 0,
                    addr: advertise.clone(),
                    protocol: me.protocol,
                    content: me.content.clone(),
                    version: me.version.clone(),
                    capacity: me.capacity,
                    down: false,
                };
                let (id, members) = cluster.ask_join(seed, member)?;
                for m in members {
                    cluster.known.lock().unwrap().insert(m.id, m.addr);
                }
                (id, false)
            }
        };
        std::fs::write(cfg.dir.join("node.id"), id.to_string()).map_err(|e| e.to_string())?;
        cluster.id.store(id, Ordering::SeqCst);
        cluster.link.id.store(id, Ordering::SeqCst);
        cluster.me.lock().unwrap().node = id;
        let tx = raftnode::start(
            raftnode::Boot {
                id,
                bootstrap,
                dir: cfg.dir.clone(),
            },
            cluster.shared.clone(),
            Arc::new(Out(Arc::downgrade(&cluster))),
        )
        .map_err(|e| format!("raft: {e}"))?;
        *cluster.raft.lock().unwrap() = Some(tx);
        // be a member on record before serving anyone
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            let present = cluster
                .shared
                .machine
                .read()
                .unwrap()
                .members
                .contains_key(&id);
            if present {
                break;
            }
            if Instant::now() > until {
                return Err("could not register with the cluster".into());
            }
            let mine = Member {
                id,
                addr: advertise.clone(),
                protocol: me.protocol,
                content: me.content.clone(),
                version: me.version.clone(),
                capacity: me.capacity,
                down: false,
            };
            if cluster.leader().is_some() {
                let _ = cluster.propose(Cmd::Member(mine), Duration::from_secs(2));
            } else {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        let c = cluster.clone();
        std::thread::Builder::new()
            .name("cluster".into())
            .spawn(move || c.background(advertise))
            .map_err(|e| e.to_string())?;
        Ok(cluster)
    }

    pub fn id(&self) -> NodeId {
        self.id.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(tx) = self.raft.lock().unwrap().take() {
            let _ = tx.send(In::Stop);
        }
        self.link.stop();
    }

    pub fn on_app(&self, f: AppHandler) {
        let _ = self.app.set(f);
    }

    pub fn serve_world(&self, f: WorldSource) {
        let _ = self.world.set(f);
    }

    pub fn addr_of(&self, id: NodeId) -> Option<String> {
        if let Some(m) = self.shared.machine.read().unwrap().members.get(&id) {
            return Some(m.addr.clone());
        }
        self.known.lock().unwrap().get(&id).cloned()
    }

    pub fn leader(&self) -> Option<NodeId> {
        match self.shared.leader.load(Ordering::SeqCst) {
            0 => None,
            l => Some(l),
        }
    }

    pub fn is_leader(&self) -> bool {
        self.shared.is_leader.load(Ordering::SeqCst)
    }

    /// In contact with a quorum recently: safe to keep serving players.
    pub fn in_contact(&self) -> bool {
        self.shared.in_contact(FENCE_AFTER) && !self.link.cut.load(Ordering::SeqCst)
    }

    /// Propose a command and wait for it to be applied.
    pub fn propose(&self, cmd: Cmd, timeout: Duration) -> Result<Outcome, String> {
        let (req, rx) = self.proposals.open();
        let prop = Prop {
            origin: self.id(),
            req,
            cmd,
        };
        match &*self.raft.lock().unwrap() {
            Some(tx) => {
                let _ = tx.send(In::Propose(prop));
            }
            None => return Err("cluster stopped".into()),
        }
        let r = rx.recv_timeout(timeout);
        self.proposals.forget(req);
        r.map_err(|_| "the cluster did not answer in time".to_string())
    }

    /// Send a player-level message to another instance.
    pub fn send_app(&self, to: NodeId, msg: App) {
        if let Some(a) = self.addr_of(to) {
            self.link.send(&a, Wire::App(msg));
        }
    }

    /// Ask a seed to let us in (it forwards to the leader).
    fn ask_join(&self, seed: &str, member: Member) -> Result<(NodeId, Vec<Member>), String> {
        let until = Instant::now() + Duration::from_secs(30);
        while Instant::now() < until {
            let (req, rx) = self.joins.open();
            self.link.send(
                seed,
                Wire::Join {
                    req,
                    reply: self.link.addr.clone(),
                    member: member.clone(),
                },
            );
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(r) => return r,
                Err(_) => self.joins.forget(req),
            }
        }
        Err(format!("no answer from the cluster at {seed}"))
    }

    /// Fetch the base world from any instance and check it against the
    /// cluster's hash.
    pub fn fetch_world(&self) -> Result<Vec<u8>, String> {
        let until = Instant::now() + Duration::from_secs(60);
        while Instant::now() < until {
            let hash = self.shared.machine.read().unwrap().world_hash.clone();
            let peers: Vec<(NodeId, String)> = self
                .shared
                .machine
                .read()
                .unwrap()
                .members
                .values()
                .filter(|m| m.id != self.id() && !m.down)
                .map(|m| (m.id, m.addr.clone()))
                .collect();
            if let Some(hash) = hash {
                for (_, addr) in peers {
                    let (req, rx) = self.worlds.open();
                    self.link.send(
                        &addr,
                        Wire::FetchWorld {
                            req,
                            reply: self.link.addr.clone(),
                        },
                    );
                    if let Ok(bytes) = rx.recv_timeout(Duration::from_secs(20)) {
                        if sha256_hex(&bytes) == hash {
                            return Ok(bytes);
                        }
                        crate::game::log(format!(
                            "Cluster: the world from {addr} does not match the cluster's hash"
                        ));
                    } else {
                        self.worlds.forget(req);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        Err("could not fetch the base world from the cluster".into())
    }

    fn receive(self: &Arc<Self>, env: Envelope<Wire>) {
        if env.from != 0 {
            self.seen.lock().unwrap().insert(env.from, Instant::now());
            self.known
                .lock()
                .unwrap()
                .entry(env.from)
                .or_insert(env.reply_to.clone());
        }
        match env.msg {
            Wire::Raft(b) => {
                if let Ok(m) = <raft::prelude::Message as protobuf::Message>::parse_from_bytes(&b)
                    && let Some(tx) = &*self.raft.lock().unwrap()
                {
                    let _ = tx.send(In::Raft(Box::new(m)));
                }
            }
            Wire::Forward(p) => {
                if let Some(tx) = &*self.raft.lock().unwrap() {
                    let _ = tx.send(In::Propose(p));
                }
            }
            Wire::Join { req, reply, member } => {
                let c = self.clone();
                std::thread::spawn(move || c.handle_join(req, reply, member));
            }
            Wire::JoinOk { req, id, members } => self.joins.answer(req, Ok((id, members))),
            Wire::JoinErr { req, why } => self.joins.answer(req, Err(why)),
            Wire::FetchWorld { req, reply } => {
                if let Some(f) = self.world.get()
                    && let Some(bytes) = f()
                {
                    self.link.send(&reply, Wire::World { req, bytes });
                }
            }
            Wire::World { req, bytes } => self.worlds.answer(req, bytes),
            Wire::Load(l) => {
                self.loads.lock().unwrap().insert(l.node, l);
            }
            Wire::App(a) => {
                if let Some(f) = self.app.get() {
                    f(env.from, a);
                }
            }
        }
    }

    /// A new instance asks to join: the leader assigns an id and adds it as
    /// a voter. Anyone else passes the request to the leader.
    fn handle_join(&self, req: u64, reply: String, member: Member) {
        if !self.is_leader() {
            if let Some(a) = self.leader().and_then(|l| self.addr_of(l)) {
                self.link.send(&a, Wire::Join { req, reply, member });
            }
            return;
        }
        let existing = self
            .shared
            .machine
            .read()
            .unwrap()
            .members
            .values()
            .find(|m| m.addr == member.addr)
            .map(|m| m.id);
        let id = match existing {
            Some(id) => id,
            None => match self.propose(Cmd::Member(member.clone()), Duration::from_secs(5)) {
                Ok(Outcome::Joined { id }) => id,
                Ok(o) => {
                    self.link.send(
                        &reply,
                        Wire::JoinErr {
                            req,
                            why: format!("{o:?}"),
                        },
                    );
                    return;
                }
                Err(e) => {
                    self.link.send(&reply, Wire::JoinErr { req, why: e });
                    return;
                }
            },
        };
        self.known.lock().unwrap().insert(id, member.addr.clone());
        let (creq, rx) = self.proposals.open();
        if let Some(tx) = &*self.raft.lock().unwrap() {
            let _ = tx.send(In::AddNode {
                id,
                origin: self.id(),
                req: creq,
            });
        }
        let ok = rx.recv_timeout(Duration::from_secs(5)).is_ok();
        self.proposals.forget(creq);
        if !ok {
            self.link.send(
                &reply,
                Wire::JoinErr {
                    req,
                    why: "the cluster could not add this instance".into(),
                },
            );
            return;
        }
        let members: Vec<Member> = self
            .shared
            .machine
            .read()
            .unwrap()
            .members
            .values()
            .cloned()
            .collect();
        crate::game::log(format!(
            "Cluster: instance {id} joined from {}",
            member.addr
        ));
        self.link.send(&reply, Wire::JoinOk { req, id, members });
    }

    /// Every second: tell the others our load; keep our member record
    /// current; as leader, mark silent instances down and quiet ones up.
    fn background(self: Arc<Self>, advertise: String) {
        let mut tick = 0u64;
        while !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(250));
            tick += 1;
            let me = {
                let mut l = self.me.lock().unwrap();
                l.node = self.id();
                l.fenced = !self.in_contact();
                l.clone()
            };
            self.loads.lock().unwrap().insert(self.id(), me.clone());
            if !tick.is_multiple_of(4) {
                continue;
            }
            let members: Vec<Member> = self
                .shared
                .machine
                .read()
                .unwrap()
                .members
                .values()
                .cloned()
                .collect();
            for m in &members {
                if m.id != self.id() {
                    self.link.send(&m.addr, Wire::Load(me.clone()));
                }
            }
            // our own record: present and current
            let mine = members.iter().find(|m| m.id == self.id()).cloned();
            let want = Member {
                id: self.id(),
                addr: advertise.clone(),
                protocol: me.protocol,
                content: me.content.clone(),
                version: me.version.clone(),
                capacity: me.capacity,
                down: mine.as_ref().is_some_and(|m| m.down),
            };
            if self.in_contact() && mine.as_ref() != Some(&want) {
                let _ = self.propose(Cmd::Member(want), Duration::from_secs(2));
            }
            if self.in_contact() && mine.is_some_and(|m| m.down) {
                let _ = self.propose(Cmd::Up { id: self.id() }, Duration::from_secs(2));
            }
            if self.is_leader() {
                let seen = self.seen.lock().unwrap().clone();
                for m in &members {
                    if m.id == self.id() || m.down {
                        continue;
                    }
                    let quiet = seen.get(&m.id).is_none_or(|t| t.elapsed() > DOWN_AFTER);
                    let grace = self.leading_since().elapsed() > DOWN_AFTER;
                    if quiet && grace {
                        crate::game::log(format!(
                            "Cluster: instance {} is silent; releasing its characters",
                            m.id
                        ));
                        let _ = self.propose(Cmd::Down { id: m.id }, Duration::from_secs(2));
                    }
                }
            }
        }
    }

    fn leading_since(&self) -> Instant {
        let leader = self.shared.leader.load(Ordering::SeqCst);
        let mut s = self.leading.lock().unwrap();
        match *s {
            Some((l, t)) if l == leader => t,
            _ => {
                let t = Instant::now();
                *s = Some((leader, t));
                t
            }
        }
    }
}

pub fn sha256_hex(b: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

#[cfg(test)]
mod tests;
