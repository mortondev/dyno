//! The game server's side of the cluster: where a joining player goes,
//! relaying a player between the edge that holds their connection and the
//! instance that hosts them, claiming and releasing characters through the
//! quorum, saving under the lock's epoch, fencing when the quorum is lost,
//! and moving players between instances without them noticing.

use crate::cluster::machine::{Cmd, NodeId, Outcome};
use crate::cluster::{App, Cluster};
use crate::game::log;
use crate::{Route, Server};
use dyno_protocol::{self as proto, ClientMsg, PlayerId, ServerMsg};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

const PROPOSE: Duration = Duration::from_secs(5);

/// An edge's client whose player lives on another instance.
pub struct EdgeRelay {
    pub host: NodeId,
    pub tx: Sender<Arc<Vec<u8>>>,
}

/// Hook the server into its cluster: incoming player messages, the base
/// world for newcomers, and the watchdog.
pub fn attach(server: &Arc<Server>) {
    let Some(c) = server.cluster.clone() else {
        return;
    };
    let w: Weak<Server> = Arc::downgrade(server);
    c.on_app(Arc::new(move |from, msg| {
        if let Some(s) = w.upgrade() {
            s.on_app(from, msg);
        }
    }));
    let bytes = Arc::new(server.base_bytes.clone());
    c.serve_world(Arc::new(move || Some((*bytes).clone())));
    let w = Arc::downgrade(server);
    let _ = std::thread::Builder::new()
        .name("cluster-watch".into())
        .spawn(move || watchdog(w));
}

fn watchdog(w: Weak<Server>) {
    let mut tick = 0u64;
    loop {
        std::thread::sleep(Duration::from_millis(250));
        tick += 1;
        let Some(s) = w.upgrade() else { return };
        if s.stop.load(Ordering::SeqCst) {
            return;
        }
        let Some(c) = s.cluster.clone() else { return };
        // fencing: out of contact with the quorum, serve no one
        let fenced = !c.in_contact();
        let was = s.fenced.swap(fenced, Ordering::SeqCst);
        if fenced && !was {
            log("Cluster: lost the quorum; dropping players until it returns");
            s.drop_everyone();
        } else if !fenced && was {
            log("Cluster: back in contact with the quorum");
        }
        // draining: a different protocol or content from the leader
        let draining = {
            let m = c.shared.machine.read().unwrap();
            let me = c.me.lock().unwrap();
            c.leader()
                .and_then(|l| m.members.get(&l))
                .is_some_and(|l| l.protocol != me.protocol || l.content != me.content)
        };
        if draining != s.draining.swap(draining, Ordering::SeqCst) {
            log(if draining {
                "Cluster: this instance runs a different version from the cluster; draining"
            } else {
                "Cluster: versions match the cluster again"
            });
        }
        // what we tell the others about our load
        if tick.is_multiple_of(2) {
            s.report_load(&c, draining);
        }
        if tick.is_multiple_of(20) && !fenced && !draining {
            s.consolidate(&c);
        }
        if tick.is_multiple_of(40) && !fenced {
            s.save_cluster();
        }
    }
}

impl Server {
    fn my_id(&self) -> NodeId {
        self.cluster.as_ref().map(|c| c.id()).unwrap_or(0)
    }

    /// Whether this instance can take players now.
    pub(crate) fn cluster_ready(&self) -> Result<(), String> {
        if self.fenced.load(Ordering::SeqCst) {
            return Err("The world is reconnecting. Try again in a moment.".into());
        }
        if self.draining.load(Ordering::SeqCst) {
            return Err("This server is updating. Try again in a moment.".into());
        }
        Ok(())
    }

    fn report_load(&self, c: &Cluster, draining: bool) {
        let routes = self.routes.lock().unwrap();
        let mut shards: std::collections::HashMap<u32, u32> =
            self.shard_list().iter().map(|s| (s.id, 0)).collect();
        for r in routes.values() {
            *shards.entry(r.shard).or_default() += 1;
        }
        let players = routes.values().map(|r| r.name.to_lowercase()).collect();
        drop(routes);
        let mut me = c.me.lock().unwrap();
        me.shards = shards.into_iter().collect();
        me.shards.sort();
        me.players = players;
        me.draining = draining;
    }

    /// The instance a joining player should live on: where their character
    /// is already held, else the fullest shard with room anywhere, else the
    /// least loaded instance.
    pub(crate) fn choose_host(&self, name: &str) -> NodeId {
        let Some(c) = &self.cluster else { return 0 };
        let me = c.id();
        let machine = c.shared.machine.read().unwrap();
        let alive: BTreeSet<NodeId> = machine.alive();
        if let Some(l) = machine.locks.get(&name.to_lowercase())
            && alive.contains(&l.node)
        {
            return l.node;
        }
        drop(machine);
        let mine = c.me.lock().unwrap().clone();
        let loads: Vec<crate::cluster::Load> = c
            .loads
            .lock()
            .unwrap()
            .values()
            .filter(|l| {
                alive.contains(&l.node)
                    && !l.fenced
                    && !l.draining
                    && l.protocol == mine.protocol
                    && l.content == mine.content
            })
            .cloned()
            .collect();
        let cap = self.cap() as u32;
        // fullest shard with room (busy worlds are more fun); prefer here on ties
        let best = loads
            .iter()
            .flat_map(|l| l.shards.iter().map(move |(_, n)| (*n, l.node)))
            .filter(|(n, _)| *n < cap)
            .max_by_key(|(n, node)| (*n, *node == me, std::cmp::Reverse(*node)));
        if let Some((n, node)) = best
            && n > 0
        {
            return node;
        }
        loads
            .iter()
            .filter(|l| l.total() < l.capacity.max(1))
            .min_by_key(|l| (l.total(), l.node != me, l.node))
            .map(|l| l.node)
            .unwrap_or(me)
    }

    /// Claim a character for this instance through the quorum. Its stored
    /// data goes into the local store so `join` finds it.
    pub(crate) fn claim_here(&self, name: &str, token: u64) -> Result<(), String> {
        let Some(c) = &self.cluster else {
            return Ok(());
        };
        self.cluster_ready()?;
        let (mode, spawn) = {
            let st = self.main_shard();
            let st = st.state.lock().unwrap();
            (st.default_mode, st.spawn_point())
        };
        let mut fresh = crate::store::PlayerData::new(mode, spawn);
        fresh.token = token;
        let key = name.to_lowercase();
        match c.propose(
            Cmd::Claim {
                name: name.to_string(),
                token,
                node: c.id(),
                fresh: Box::new(fresh),
            },
            PROPOSE,
        ) {
            Ok(Outcome::Claimed { epoch, data, fresh }) => {
                self.epochs.lock().unwrap().insert(key.clone(), epoch);
                let main = self.main_shard();
                let st = main.state.lock().unwrap();
                let mut saved = st.saved.lock().unwrap();
                if fresh {
                    // a new character: let join give them the class and kit
                    saved.remove(&key);
                } else {
                    saved.insert(key, *data);
                }
                Ok(())
            }
            Ok(Outcome::Refused(why)) => Err(why),
            Ok(o) => Err(format!("unexpected answer: {o:?}")),
            Err(e) => Err(format!("The world did not answer ({e}). Try again.")),
        }
    }

    /// Let go of a character this instance holds, writing it one last time.
    pub(crate) fn release_claim(&self, name: &str) {
        let Some(c) = self.cluster.clone() else {
            return;
        };
        let key = name.to_lowercase();
        let Some(epoch) = self.epochs.lock().unwrap().remove(&key) else {
            return;
        };
        let data = {
            let main = self.main_shard();
            let st = main.state.lock().unwrap();
            st.saved.lock().unwrap().get(&key).cloned()
        };
        let Some(data) = data else { return };
        let node = c.id();
        std::thread::spawn(move || {
            let r = c.propose(
                Cmd::Release {
                    name: key.clone(),
                    node,
                    epoch,
                    data: Box::new(data),
                },
                PROPOSE,
            );
            if !matches!(r, Ok(Outcome::Ok)) {
                log(format!("Cluster: could not release {key}: {r:?}"));
            }
        });
    }

    /// A local player leaves: out of the shard, out of the routes, released.
    pub(crate) fn leave_here(&self, id: PlayerId) {
        let name = self.routes.lock().unwrap().get(&id).map(|r| r.name.clone());
        let sh = self.shard_of(id).unwrap_or_else(|| self.main_shard());
        sh.state.lock().unwrap().leave(id);
        self.routes.lock().unwrap().remove(&id);
        if let Some(name) = name {
            self.release_claim(&name);
        }
    }

    // ----- edge side -----

    /// The local player behind one of this instance's own connections.
    pub(crate) fn local_id(&self, conn: u64) -> Option<PlayerId> {
        self.routes
            .lock()
            .unwrap()
            .iter()
            .find(|(_, r)| r.conn == conn && r.edge.is_none())
            .map(|(id, _)| *id)
    }

    pub(crate) fn relay_host(&self, conn: u64) -> Option<NodeId> {
        self.relays_out.lock().unwrap().get(&conn).map(|r| r.host)
    }

    pub(crate) fn relay_up(&self, conn: u64, bytes: Vec<u8>) {
        if let (Some(c), Some(host)) = (&self.cluster, self.relay_host(conn)) {
            c.send_app(host, App::RelayClient { conn, bytes });
        }
    }

    /// Ask `host` to take the player behind this connection.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open_remote(
        &self,
        host: NodeId,
        conn: u64,
        name: &str,
        token: u64,
        create: Option<proto::NewCharacter>,
        tx: Sender<Arc<Vec<u8>>>,
    ) -> Result<(), String> {
        let Some(c) = &self.cluster else {
            return Err("not in a cluster".into());
        };
        let rx = self.relay_opens.open_at(conn);
        self.relays_out
            .lock()
            .unwrap()
            .insert(conn, EdgeRelay { host, tx });
        c.send_app(
            host,
            App::RelayOpen {
                conn,
                name: name.to_string(),
                token,
                create,
            },
        );
        let r = rx
            .recv_timeout(Duration::from_secs(15))
            .unwrap_or_else(|_| Err("The world did not answer. Try again.".into()));
        self.relay_opens.forget(conn);
        if r.is_err() {
            self.relays_out.lock().unwrap().remove(&conn);
        }
        r
    }

    /// The client behind `conn` is gone (or the session ended).
    pub(crate) fn close_conn(&self, conn: u64, reason: &str) {
        let gone = self.relays_out.lock().unwrap().remove(&conn);
        if let (Some(c), Some(r)) = (&self.cluster, gone) {
            c.send_app(
                r.host,
                App::RelayClose {
                    conn,
                    reason: reason.into(),
                },
            );
        }
    }

    // ----- host side -----

    fn on_app(self: &Arc<Self>, from: NodeId, msg: App) {
        match msg {
            App::RelayOpen {
                conn,
                name,
                token,
                create,
            } => {
                let s = self.clone();
                std::thread::spawn(move || {
                    let result = s.host_open(from, conn, &name, token, create);
                    if let Some(c) = &s.cluster {
                        c.send_app(from, App::RelayOpened { conn, result });
                    }
                });
            }
            App::RelayOpened { conn, result } => self.relay_opens.answer(conn, result),
            App::RelayClient { conn, bytes } => {
                // handled in order on the client's own worker: a command may
                // take seconds (a move), and the link must keep flowing
                let mut queues = self.relay_in.lock().unwrap();
                let q = queues.entry((from, conn)).or_insert_with(|| {
                    let (tx, rx) = channel::<Vec<u8>>();
                    let s = self.clone();
                    std::thread::spawn(move || {
                        while let Ok(bytes) = rx.recv() {
                            let id = s.hosted.lock().unwrap().get(&(from, conn)).copied();
                            let Some(id) = id else { continue };
                            let ok = proto::decode::<ClientMsg>(&bytes)
                                .map_err(|e| e.to_string())
                                .and_then(|m| s.client_msg(id, m, 1));
                            if let Err(e) = ok {
                                s.host_close(from, conn, &e);
                            }
                        }
                    });
                    tx
                });
                let _ = q.send(bytes);
            }
            App::RelayServer { conn, bytes } => {
                if let Some(r) = self.relays_out.lock().unwrap().get(&conn) {
                    let _ = r.tx.send(Arc::new(bytes));
                }
            }
            App::RelayClose { conn, reason } => {
                self.relay_in.lock().unwrap().remove(&(from, conn));
                // the edge's client left
                if let Some(id) = self.hosted.lock().unwrap().remove(&(from, conn)) {
                    let _ = reason;
                    self.leave_here(id);
                }
                // or the host dropped our client
                let ours = self
                    .relays_out
                    .lock()
                    .unwrap()
                    .get(&conn)
                    .is_some_and(|r| r.host == from);
                if ours {
                    self.relays_out.lock().unwrap().remove(&conn);
                    self.kicked.lock().unwrap().insert(conn);
                }
            }
            App::RelayMoved { conn, host } => {
                if let Some(r) = self.relays_out.lock().unwrap().get_mut(&conn) {
                    r.host = host;
                }
            }
            App::Adopt {
                req,
                name,
                token,
                epoch,
                data,
                edge,
                conn,
                changed,
            } => {
                let s = self.clone();
                std::thread::spawn(move || {
                    let result = s.host_adopt(&name, token, epoch, *data, edge, conn, changed);
                    if let Err(e) = &result {
                        log(format!(
                            "Cluster: could not take {name} from instance {from}: {e}"
                        ));
                    }
                    if let Some(c) = &s.cluster {
                        c.send_app(from, App::AdoptResult { req, result });
                    }
                });
            }
            App::AdoptResult { req, result } => self.adoptions.answer(req, result),
        }
    }

    /// A channel whose messages go to `edge` as relayed server messages.
    fn pump(&self, edge: NodeId, conn: u64) -> Sender<Arc<Vec<u8>>> {
        let (tx, rx) = channel::<Arc<Vec<u8>>>();
        let c = self.cluster.clone();
        std::thread::spawn(move || {
            while let Ok(b) = rx.recv() {
                if let Some(c) = &c {
                    c.send_app(
                        edge,
                        App::RelayServer {
                            conn,
                            bytes: (*b).clone(),
                        },
                    );
                }
            }
        });
        tx
    }

    /// Take a player an edge relays to us: claim, join, stream the start.
    fn host_open(
        &self,
        edge: NodeId,
        conn: u64,
        name: &str,
        token: u64,
        create: Option<proto::NewCharacter>,
    ) -> Result<(), String> {
        let name = proto::clean_name(name);
        self.claim_here(&name, token)?;
        let tx = self.pump(edge, conn);
        let sh = self.place(&name);
        let mut st = sh.state.lock().unwrap();
        let (id, msgs) = match st.join(&name, token, tx.clone(), create) {
            Ok(x) => x,
            Err(e) => {
                drop(st);
                self.release_claim(&name);
                return Err(e);
            }
        };
        let mut msgs = msgs.into_iter();
        let send = |m: &ServerMsg| {
            let _ = tx.send(Arc::new(proto::encode(m)));
        };
        if let Some(mut w) = msgs.next() {
            if let ServerMsg::Welcome { motd, manifest, .. } = &mut w {
                *motd = self.cfg.motd.clone();
                // the edge serves the files and names its own pack
                *manifest = self.manifest();
            }
            send(&w);
        }
        send(&ServerMsg::World {
            data: st.world.encode(),
        });
        send(&st.time_msg());
        for m in msgs {
            send(&m);
        }
        let display = st
            .players
            .get(&id)
            .map(|p| p.name.clone())
            .unwrap_or(name.clone());
        drop(st);
        self.routes.lock().unwrap().insert(
            id,
            Route {
                shard: sh.id,
                name: display.clone(),
                tx,
                conn,
                edge: Some(edge),
            },
        );
        self.hosted.lock().unwrap().insert((edge, conn), id);
        log(format!(
            "{display} joined through instance {edge}, {} here",
            self.player_count()
        ));
        Ok(())
    }

    /// End a relayed player's stay here (and tell their edge).
    fn host_close(&self, edge: NodeId, conn: u64, reason: &str) {
        self.relay_in.lock().unwrap().remove(&(edge, conn));
        if let Some(id) = self.hosted.lock().unwrap().remove(&(edge, conn)) {
            self.leave_here(id);
        }
        if let Some(c) = &self.cluster {
            c.send_app(
                edge,
                App::RelayClose {
                    conn,
                    reason: reason.into(),
                },
            );
        }
    }

    /// Fenced: drop every player without writing (the quorum will release
    /// them once this instance is declared down).
    fn drop_everyone(&self) {
        let routes: Vec<(PlayerId, Route)> = self
            .routes
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        self.epochs.lock().unwrap().clear();
        for (id, r) in routes {
            match r.edge {
                None => {
                    self.kicked.lock().unwrap().insert(r.conn);
                }
                Some(edge) => {
                    self.hosted.lock().unwrap().remove(&(edge, r.conn));
                    if let Some(c) = &self.cluster {
                        c.send_app(
                            edge,
                            App::RelayClose {
                                conn: r.conn,
                                reason: "the host lost the quorum".into(),
                            },
                        );
                    }
                }
            }
            let sh = self.shard_of(id).unwrap_or_else(|| self.main_shard());
            sh.state.lock().unwrap().leave(id);
            self.routes.lock().unwrap().remove(&id);
        }
        // our own clients living elsewhere: drop them too (we cannot be sure
        // our relay still works)
        let conns: Vec<u64> = self.relays_out.lock().unwrap().keys().copied().collect();
        for conn in conns {
            self.close_conn(conn, "the edge lost the quorum");
            self.kicked.lock().unwrap().insert(conn);
        }
    }

    /// Write every character hosted here under its lock's epoch. A refused
    /// write means another instance owns them now: drop them here.
    pub(crate) fn save_cluster(self: &Arc<Self>) {
        let Some(c) = self.cluster.clone() else {
            return;
        };
        let routes: Vec<(PlayerId, Route)> = self
            .routes
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let mut writes = Vec::new();
        for (id, r) in &routes {
            let Some(sh) = self.shard_of(*id) else {
                continue;
            };
            let data = {
                let mut st = sh.state.lock().unwrap();
                let Some(p) = st.players.remove(id) else {
                    continue;
                };
                st.remember(&p);
                st.players.insert(*id, p);
                st.saved
                    .lock()
                    .unwrap()
                    .get(&r.name.to_lowercase())
                    .cloned()
            };
            let epoch = self
                .epochs
                .lock()
                .unwrap()
                .get(&r.name.to_lowercase())
                .copied();
            if let (Some(d), Some(e)) = (data, epoch) {
                writes.push((*id, r.clone(), d, e));
            }
        }
        let s = self.clone();
        std::thread::spawn(move || {
            for (id, r, data, epoch) in writes {
                let out = c.propose(
                    Cmd::Save {
                        name: r.name.clone(),
                        node: c.id(),
                        epoch,
                        data: Box::new(data),
                    },
                    PROPOSE,
                );
                if let Ok(Outcome::Refused(why)) = out {
                    log(format!("Cluster: dropping {} ({why})", r.name));
                    s.epochs.lock().unwrap().remove(&r.name.to_lowercase());
                    match r.edge {
                        None => {
                            s.kicked.lock().unwrap().insert(r.conn);
                        }
                        Some(edge) => s.host_close(edge, r.conn, "signed in elsewhere"),
                    }
                    let sh = s.shard_of(id).unwrap_or_else(|| s.main_shard());
                    sh.state.lock().unwrap().leave(id);
                    s.routes.lock().unwrap().remove(&id);
                }
            }
        });
    }

    // ----- moving between instances -----

    /// Move a player to another instance at a quiet moment. They stay
    /// connected; their edge's relay switches to the new host.
    pub fn transfer_remote(&self, id: PlayerId, to: NodeId) -> Result<(), String> {
        let c = self.cluster.clone().ok_or("not in a cluster")?;
        let me = c.id();
        if to == me {
            return Err("already here".into());
        }
        let route = self
            .routes
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or("no such player")?;
        let key = route.name.to_lowercase();
        let epoch = *self
            .epochs
            .lock()
            .unwrap()
            .get(&key)
            .ok_or("not held here")?;
        let sh = self.shard_of(id).ok_or("no shard")?;
        // freeze: out of the shard, remembered, nothing else touches them
        let (player, data, changed, token) = {
            let mut st = sh.state.lock().unwrap();
            if !st.movable(id) {
                return Err("not now (in combat or busy)".into());
            }
            let p = st.detach(id).ok_or("no such player")?;
            st.remember(&p);
            let data = st
                .saved
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .ok_or("no data")?;
            let changed: Vec<[i32; 3]> = self
                .base
                .diff(&st.world)
                .into_iter()
                .map(|(p, _)| p)
                .collect();
            let token = p.token;
            (p, data, changed, token)
        };
        self.routes.lock().unwrap().remove(&id);
        if let Some(edge) = route.edge {
            self.hosted.lock().unwrap().remove(&(edge, route.conn));
        }
        let unfreeze = |player: crate::game::Player, epoch: u64| {
            self.epochs.lock().unwrap().insert(key.clone(), epoch);
            let sh = self.shard_of_any(route.shard);
            sh.state.lock().unwrap().attach(id, player, Vec::new());
            self.routes.lock().unwrap().insert(id, route.clone());
            if let Some(edge) = route.edge {
                self.hosted.lock().unwrap().insert((edge, route.conn), id);
            }
        };
        // hand the character over in the quorum
        let moved = c.propose(
            Cmd::Move {
                name: key.clone(),
                from: me,
                epoch,
                to,
                data: Box::new(data.clone()),
            },
            PROPOSE,
        );
        let new_epoch = match moved {
            Ok(Outcome::Claimed { epoch: e, .. }) => e,
            Ok(Outcome::Refused(why)) => {
                unfreeze(player, epoch);
                return Err(why);
            }
            Ok(_) => {
                unfreeze(player, epoch);
                return Err("unexpected answer".into());
            }
            Err(_) => {
                // unknown: maybe it committed before a leader change. Ask the
                // quorum's state once a leader is back.
                match self.settle_lock(&c, &key, Duration::from_secs(10)) {
                    Some(l) if l.node == to && l.epoch > epoch => l.epoch,
                    Some(l) if l.node == me && l.epoch == epoch => {
                        unfreeze(player, epoch);
                        return Err("the move did not commit".into());
                    }
                    _ => {
                        // released (we were declared down): drop the player
                        self.epochs.lock().unwrap().remove(&key);
                        self.forget_client(&route);
                        return Err("lost the character's lock".into());
                    }
                }
            }
        };
        self.epochs.lock().unwrap().remove(&key);
        let edge = route.edge.unwrap_or(me);
        let (req, rx) = self.adoptions.open();
        if route.edge.is_none() {
            // our own client: point its relay at the new host before that host
            // speaks, so nothing it sends is dropped
            self.relays_out.lock().unwrap().insert(
                route.conn,
                EdgeRelay {
                    host: to,
                    tx: route.tx.clone(),
                },
            );
        }
        c.send_app(
            to,
            App::Adopt {
                req,
                name: route.name.clone(),
                token,
                epoch: new_epoch,
                data: Box::new(data.clone()),
                edge,
                conn: route.conn,
                changed,
            },
        );
        let adopted = rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| Err("no answer".into()));
        self.adoptions.forget(req);
        match adopted {
            Ok(()) => {
                if route.edge.is_some() {
                    c.send_app(
                        edge,
                        App::RelayMoved {
                            conn: route.conn,
                            host: to,
                        },
                    );
                }
                log(format!("Moved {} to instance {to}", route.name));
                Ok(())
            }
            Err(e) => {
                // take them back: move the lock home and unfreeze
                if route.edge.is_none() {
                    self.relays_out.lock().unwrap().remove(&route.conn);
                }
                let back = c.propose(
                    Cmd::Move {
                        name: key.clone(),
                        from: to,
                        epoch: new_epoch,
                        to: me,
                        data: Box::new(data),
                    },
                    PROPOSE,
                );
                match back {
                    Ok(Outcome::Claimed { epoch: e2, .. }) => {
                        unfreeze(player, e2);
                        Err(format!("the other instance could not take them: {e}"))
                    }
                    _ => {
                        self.forget_client(&route);
                        Err(format!("move failed and could not be undone: {e}"))
                    }
                }
            }
        }
    }

    fn shard_of_any(&self, sid: u32) -> Arc<crate::ShardHost> {
        self.shard(sid).unwrap_or_else(|| self.main_shard())
    }

    /// Wait for a leader, then read where a character's lock is.
    fn settle_lock(
        &self,
        c: &Cluster,
        key: &str,
        wait: Duration,
    ) -> Option<crate::cluster::machine::Lock> {
        let until = Instant::now() + wait;
        while Instant::now() < until && (c.leader().is_none() || !c.in_contact()) {
            std::thread::sleep(Duration::from_millis(100));
        }
        // let the new leader's first entries apply
        std::thread::sleep(Duration::from_millis(500));
        c.shared.machine.read().unwrap().locks.get(key).copied()
    }

    /// The player is gone from here for good: end their connection.
    fn forget_client(&self, route: &Route) {
        match route.edge {
            None => {
                self.kicked.lock().unwrap().insert(route.conn);
            }
            Some(edge) => {
                if let Some(c) = &self.cluster {
                    c.send_app(
                        edge,
                        App::RelayClose {
                            conn: route.conn,
                            reason: "lost".into(),
                        },
                    );
                }
            }
        }
    }

    /// Take a player another instance is handing over mid-session.
    #[allow(clippy::too_many_arguments)]
    fn host_adopt(
        &self,
        name: &str,
        token: u64,
        epoch: u64,
        data: crate::store::PlayerData,
        edge: NodeId,
        conn: u64,
        changed: Vec<[i32; 3]>,
    ) -> Result<(), String> {
        self.cluster_ready()?;
        let key = name.to_lowercase();
        {
            let main = self.main_shard();
            let st = main.state.lock().unwrap();
            st.saved.lock().unwrap().insert(key.clone(), data);
        }
        self.epochs.lock().unwrap().insert(key.clone(), epoch);
        let me = self.my_id();
        // our own client coming home: no relay any more
        let (tx, route_edge) = if edge == me {
            match self.relays_out.lock().unwrap().remove(&conn) {
                Some(r) => (r.tx, None),
                None => return Err("the client is gone".into()),
            }
        } else {
            (self.pump(edge, conn), Some(edge))
        };
        let sh = self.place(name);
        let mut st = sh.state.lock().unwrap();
        let (id, msgs) = st.join(name, token, tx.clone(), None).inspect_err(|_| {
            self.epochs.lock().unwrap().remove(&key);
        })?;
        // a seamless move: no Welcome or World, a Shard message instead
        let mut msgs = msgs.into_iter();
        let players = match msgs.next() {
            Some(ServerMsg::Welcome { players, .. }) => players,
            _ => Vec::new(),
        };
        let mut spots: BTreeSet<[i32; 3]> = changed.into_iter().collect();
        for (p, _) in self.base.diff(&st.world) {
            spots.insert(p);
        }
        let blocks = spots
            .into_iter()
            .map(|p| (p, st.world.get(p[0], p[1], p[2])))
            .collect();
        let display = st
            .players
            .get(&id)
            .map(|p| p.name.clone())
            .unwrap_or(name.to_string());
        // registered before the client hears anything, so whatever it sends
        // back at once finds them
        self.routes.lock().unwrap().insert(
            id,
            Route {
                shard: sh.id,
                name: display,
                tx: tx.clone(),
                conn,
                edge: route_edge,
            },
        );
        if let Some(e) = route_edge {
            self.hosted.lock().unwrap().insert((e, conn), id);
        }
        let send = |m: &ServerMsg| {
            let _ = tx.send(Arc::new(proto::encode(m)));
        };
        send(&ServerMsg::Shard {
            shard: sh.id,
            players,
            blocks,
        });
        send(&st.time_msg());
        for m in msgs {
            send(&m);
        }
        Ok(())
    }

    /// Fold a quiet instance into a busier one: if this instance hosts only
    /// a few players and another (same version) has room, move them there.
    fn consolidate(&self, c: &Cluster) {
        let cap = self.cap() as u32;
        if cap == u32::MAX {
            return;
        }
        let me = c.id();
        let mine = c.me.lock().unwrap().clone();
        let here = mine.total();
        if here == 0 || here > cap / 4 {
            return;
        }
        let alive = c.shared.machine.read().unwrap().alive();
        let target = c
            .loads
            .lock()
            .unwrap()
            .values()
            .filter(|l| {
                l.node != me
                    && alive.contains(&l.node)
                    && !l.fenced
                    && !l.draining
                    && l.protocol == mine.protocol
                    && l.content == mine.content
                    // the busier one takes the quieter (ties: the lower id stays)
                    && (l.total() > here || l.total() == here && l.node < me)
                    && l.total() + here <= cap * 3 / 4
            })
            .max_by_key(|l| (l.total(), std::cmp::Reverse(l.node)))
            .map(|l| l.node);
        let Some(to) = target else { return };
        let ids: Vec<PlayerId> = self.routes.lock().unwrap().keys().copied().collect();
        for id in ids {
            if let Err(e) = self.transfer_remote(id, to) {
                log(format!("Cluster: could not move player {id} to {to}: {e}"));
            }
        }
    }
}
