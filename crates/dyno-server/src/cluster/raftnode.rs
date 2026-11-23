//! Drives one Raft node (the `raft` crate): ticks, steps messages from
//! peers, persists entries and hard state before acknowledging them, applies
//! committed commands to the state machine, and compacts the log into
//! snapshots that carry the whole (small) machine.

use super::machine::{Cmd, Machine, NodeId, Outcome};
use protobuf::Message as _;
use raft::prelude::*;
use raft::storage::MemStorage;
use raft::{GetEntriesContext, RaftState, StateRole, Storage};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(100);
/// Compact the log once this many entries have been applied since the last snapshot.
const COMPACT_EVERY: u64 = 400;

/// A command as it travels through the log: who proposed it and the
/// request it answers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prop {
    pub origin: NodeId,
    pub req: u64,
    pub cmd: Cmd,
}

/// Work for the Raft thread.
pub enum In {
    Raft(Box<Message>),
    /// propose (here if leader, else forwarded)
    Propose(Prop),
    /// add a voter (leader only); answers through `req` like a proposal
    AddNode {
        id: NodeId,
        origin: NodeId,
        req: u64,
    },
    /// remove a voter
    RemoveNode {
        id: NodeId,
        origin: NodeId,
        req: u64,
    },
    Stop,
}

/// What the Raft thread tells the rest of the instance.
pub trait Outbox: Send + Sync + 'static {
    /// send a Raft message to node `to`
    fn raft(&self, to: NodeId, msg: Vec<u8>);
    /// a proposal must go to the leader
    fn forward(&self, leader: NodeId, prop: Prop);
    /// a command this node proposed was applied
    fn applied(&self, req: u64, outcome: Outcome);
}

/// Shared view of the node, read by everyone else.
pub struct Shared {
    pub machine: RwLock<Machine>,
    pub leader: AtomicU64,
    pub is_leader: AtomicBool,
    /// last time this node knew it was in contact with a quorum
    pub contact: Mutex<Instant>,
    pub applied: AtomicU64,
}

impl Shared {
    pub fn new() -> Arc<Shared> {
        Arc::new(Shared {
            machine: RwLock::new(Machine::default()),
            leader: AtomicU64::new(0),
            is_leader: AtomicBool::new(false),
            contact: Mutex::new(Instant::now()),
            applied: AtomicU64::new(0),
        })
    }

    pub fn in_contact(&self, within: Duration) -> bool {
        self.contact.lock().unwrap().elapsed() < within
    }
}

/// The log store: entries in memory, a snapshot built from the machine.
struct Store {
    mem: MemStorage,
    snap: Arc<Mutex<Snapshot>>,
}

impl Storage for Store {
    fn initial_state(&self) -> raft::Result<RaftState> {
        self.mem.initial_state()
    }
    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        context: GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        self.mem.entries(low, high, max_size, context)
    }
    fn term(&self, idx: u64) -> raft::Result<u64> {
        self.mem.term(idx)
    }
    fn first_index(&self) -> raft::Result<u64> {
        self.mem.first_index()
    }
    fn last_index(&self) -> raft::Result<u64> {
        self.mem.last_index()
    }
    fn snapshot(&self, request_index: u64, _to: u64) -> raft::Result<Snapshot> {
        let s = self.snap.lock().unwrap().clone();
        if s.get_metadata().index < request_index {
            return Err(raft::Error::Store(
                raft::StorageError::SnapshotTemporarilyUnavailable,
            ));
        }
        Ok(s)
    }
}

/// On-disk records: a snapshot file and a write-ahead log after it.
#[derive(Serialize, Deserialize)]
struct SnapFile {
    index: u64,
    term: u64,
    conf: Vec<u8>,
    machine: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
enum Wal {
    Hard(Vec<u8>),
    Entries(Vec<Vec<u8>>),
}

struct Disk {
    dir: PathBuf,
    wal: std::fs::File,
}

fn frame(f: &mut std::fs::File, bytes: &[u8]) -> std::io::Result<()> {
    f.write_all(&(bytes.len() as u32).to_le_bytes())?;
    f.write_all(bytes)
}

impl Disk {
    fn snap_path(dir: &Path) -> PathBuf {
        dir.join("raft.snap")
    }
    fn wal_path(dir: &Path) -> PathBuf {
        dir.join("raft.wal")
    }

    fn open(dir: &Path) -> std::io::Result<Disk> {
        std::fs::create_dir_all(dir)?;
        let wal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Self::wal_path(dir))?;
        Ok(Disk {
            dir: dir.to_path_buf(),
            wal,
        })
    }

    fn append(&mut self, rec: &Wal) -> std::io::Result<()> {
        let b = postcard::to_stdvec(rec).map_err(std::io::Error::other)?;
        frame(&mut self.wal, &b)?;
        self.wal.sync_data()
    }

    /// Write a snapshot and restart the log with what comes after it.
    fn checkpoint(
        &mut self,
        s: &SnapFile,
        hard: &HardState,
        rest: &[Entry],
    ) -> std::io::Result<()> {
        let tmp = self.dir.join("raft.snap.tmp");
        std::fs::write(&tmp, postcard::to_stdvec(s).map_err(std::io::Error::other)?)?;
        std::fs::rename(&tmp, Self::snap_path(&self.dir))?;
        let tmp = self.dir.join("raft.wal.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            let h = Wal::Hard(hard.write_to_bytes().map_err(std::io::Error::other)?);
            frame(
                &mut f,
                &postcard::to_stdvec(&h).map_err(std::io::Error::other)?,
            )?;
            if !rest.is_empty() {
                let e = Wal::Entries(
                    rest.iter()
                        .map(|e| e.write_to_bytes().expect("entries encode"))
                        .collect(),
                );
                frame(
                    &mut f,
                    &postcard::to_stdvec(&e).map_err(std::io::Error::other)?,
                )?;
            }
            f.sync_all()?;
        }
        std::fs::rename(&tmp, Self::wal_path(&self.dir))?;
        self.wal = std::fs::OpenOptions::new()
            .append(true)
            .open(Self::wal_path(&self.dir))?;
        Ok(())
    }

    /// Read back what was persisted: snapshot (if any), hard state, entries.
    #[allow(clippy::type_complexity)]
    fn load(dir: &Path) -> std::io::Result<(Option<SnapFile>, Option<HardState>, Vec<Entry>)> {
        let snap = std::fs::read(Self::snap_path(dir))
            .ok()
            .and_then(|b| postcard::from_bytes::<SnapFile>(&b).ok());
        let mut hard = None;
        let mut ents: Vec<Entry> = Vec::new();
        if let Ok(mut f) = std::fs::File::open(Self::wal_path(dir)) {
            let mut all = Vec::new();
            f.read_to_end(&mut all)?;
            let mut i = 0;
            while i + 4 <= all.len() {
                let n = u32::from_le_bytes(all[i..i + 4].try_into().unwrap()) as usize;
                if i + 4 + n > all.len() {
                    break; // a torn last write
                }
                let Ok(rec) = postcard::from_bytes::<Wal>(&all[i + 4..i + 4 + n]) else {
                    break;
                };
                i += 4 + n;
                match rec {
                    Wal::Hard(b) => hard = HardState::parse_from_bytes(&b).ok(),
                    Wal::Entries(list) => {
                        for b in list {
                            let Ok(e) = Entry::parse_from_bytes(&b) else {
                                continue;
                            };
                            // a later write of an index replaces the old tail
                            while ents.last().is_some_and(|l| l.index >= e.index) {
                                ents.pop();
                            }
                            ents.push(e);
                        }
                    }
                }
            }
        }
        Ok((snap, hard, ents))
    }
}

fn save_conf(dir: &Path, cs: &ConfState) {
    let tmp = dir.join("raft.conf.tmp");
    if std::fs::write(&tmp, cs.write_to_bytes().unwrap_or_default()).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("raft.conf"));
    }
}

fn encode_machine(m: &Machine) -> Vec<u8> {
    postcard::to_stdvec(m).expect("machine encodes")
}

pub struct Boot {
    pub id: NodeId,
    /// start a new one-node cluster (first instance only)
    pub bootstrap: bool,
    pub dir: PathBuf,
}

/// Start the Raft thread. Returns the queue that feeds it.
pub fn start(boot: Boot, shared: Arc<Shared>, out: Arc<dyn Outbox>) -> std::io::Result<Sender<In>> {
    std::fs::create_dir_all(&boot.dir)?;
    let (snap, hard, ents) = Disk::load(&boot.dir)?;
    let mem = MemStorage::new();
    let mut applied = 0;
    let snapshot = Arc::new(Mutex::new(Snapshot::default()));
    if let Some(s) = &snap {
        let mut sn = Snapshot::default();
        sn.mut_metadata().index = s.index;
        sn.mut_metadata().term = s.term;
        sn.mut_metadata()
            .set_conf_state(ConfState::parse_from_bytes(&s.conf).unwrap_or_default());
        sn.data = s.machine.clone().into();
        mem.wl()
            .apply_snapshot(sn.clone())
            .map_err(std::io::Error::other)?;
        *shared.machine.write().unwrap() = postcard::from_bytes(&s.machine).unwrap_or_default();
        applied = s.index;
        *snapshot.lock().unwrap() = sn;
    } else if boot.bootstrap && hard.is_none() && ents.is_empty() {
        let cs = ConfState::from((vec![boot.id], vec![]));
        save_conf(&boot.dir, &cs);
        mem.wl().set_conf_state(cs);
    }
    // the voter set as of the last change (the log replays the rest)
    if let Some(cs) = std::fs::read(boot.dir.join("raft.conf"))
        .ok()
        .and_then(|b| ConfState::parse_from_bytes(&b).ok())
    {
        mem.wl().set_conf_state(cs);
    }
    let ents: Vec<Entry> = ents.into_iter().filter(|e| e.index > applied).collect();
    if !ents.is_empty() {
        mem.wl().append(&ents).map_err(std::io::Error::other)?;
    }
    if let Some(h) = hard {
        mem.wl().set_hardstate(h);
    }
    // replay the conf changes in the log so the voter set is right
    let store = Store {
        mem,
        snap: snapshot.clone(),
    };
    let cfg = Config {
        id: boot.id,
        election_tick: 10,
        heartbeat_tick: 2,
        check_quorum: true,
        pre_vote: true,
        applied,
        max_size_per_msg: 1 << 20,
        max_inflight_msgs: 256,
        ..Default::default()
    };
    cfg.validate().map_err(std::io::Error::other)?;
    let logger = slog::Logger::root(slog::Discard, slog::o!());
    let mut node = RawNode::new(&cfg, store, &logger).map_err(std::io::Error::other)?;
    if boot.bootstrap && applied == 0 && node.raft.raft_log.last_index() == 0 {
        let _ = node.campaign();
    }
    let mut disk = Disk::open(&boot.dir)?;
    shared.applied.store(applied, Ordering::SeqCst);
    let (tx, rx) = std::sync::mpsc::channel::<In>();
    let me = boot.id;
    std::thread::Builder::new()
        .name("raft".into())
        .spawn(move || {
            let mut r = Runner {
                node,
                rx,
                shared,
                out,
                disk: &mut disk,
                me,
                snapshot,
                snap_index: applied,
                last_tick: Instant::now(),
            };
            r.run();
        })?;
    Ok(tx)
}

struct Runner<'a> {
    node: RawNode<Store>,
    rx: Receiver<In>,
    shared: Arc<Shared>,
    out: Arc<dyn Outbox>,
    disk: &'a mut Disk,
    me: NodeId,
    snapshot: Arc<Mutex<Snapshot>>,
    snap_index: u64,
    last_tick: Instant,
}

impl Runner<'_> {
    fn run(&mut self) {
        loop {
            match self.rx.recv_timeout(TICK) {
                Ok(In::Stop) => return,
                Ok(m) => self.input(m),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            while let Ok(m) = self.rx.try_recv() {
                if matches!(m, In::Stop) {
                    return;
                }
                self.input(m);
            }
            if self.last_tick.elapsed() >= TICK {
                self.last_tick = Instant::now();
                self.node.tick();
            }
            self.ready();
            self.status();
        }
    }

    fn status(&mut self) {
        let leader = self.node.raft.leader_id;
        self.shared.leader.store(leader, Ordering::SeqCst);
        let is_leader = self.node.raft.state == StateRole::Leader;
        self.shared.is_leader.store(is_leader, Ordering::SeqCst);
        if is_leader {
            // check_quorum steps a cut-off leader down within an election timeout
            *self.shared.contact.lock().unwrap() = Instant::now();
        }
    }

    fn input(&mut self, m: In) {
        match m {
            In::Raft(msg) => {
                if msg.from == self.node.raft.leader_id && msg.from != 0 {
                    *self.shared.contact.lock().unwrap() = Instant::now();
                }
                let _ = self.node.step(*msg);
            }
            In::Propose(p) => {
                if self.node.raft.state == StateRole::Leader {
                    let ctx = p.req.to_le_bytes().to_vec();
                    let data = postcard::to_stdvec(&p).expect("props encode");
                    let _ = self.node.propose(ctx, data);
                } else {
                    let leader = self.node.raft.leader_id;
                    if leader != 0 {
                        self.out.forward(leader, p);
                    }
                }
            }
            In::AddNode { id, origin, req } | In::RemoveNode { id, origin, req } => {
                let add = matches!(m, In::AddNode { .. });
                if self.node.raft.state != StateRole::Leader {
                    return;
                }
                let mut cc = ConfChange::default();
                cc.set_change_type(if add {
                    ConfChangeType::AddNode
                } else {
                    ConfChangeType::RemoveNode
                });
                cc.node_id = id;
                let ctx = postcard::to_stdvec(&(origin, req)).expect("encodes");
                let _ = self.node.propose_conf_change(ctx, cc);
            }
            In::Stop => {}
        }
    }

    fn send(&self, msgs: Vec<Message>) {
        for m in msgs {
            let to = m.to;
            if let Ok(b) = m.write_to_bytes() {
                self.out.raft(to, b);
            }
        }
    }

    fn apply(&mut self, ents: Vec<Entry>) {
        for e in ents {
            if e.data.is_empty() {
                self.shared.applied.store(e.index, Ordering::SeqCst);
                continue;
            }
            match e.get_entry_type() {
                EntryType::EntryNormal => {
                    if let Ok(p) = postcard::from_bytes::<Prop>(&e.data) {
                        let outcome = self.shared.machine.write().unwrap().apply(&p.cmd);
                        if p.origin == self.me {
                            self.out.applied(p.req, outcome);
                        }
                    }
                }
                EntryType::EntryConfChange => {
                    if let Ok(cc) = ConfChange::parse_from_bytes(&e.data) {
                        if let Ok(cs) = self.node.apply_conf_change(&cc) {
                            save_conf(&self.disk.dir, &cs);
                            self.node.mut_store().mem.wl().set_conf_state(cs);
                        }
                        if let Ok((origin, req)) = postcard::from_bytes::<(NodeId, u64)>(&e.context)
                            && origin == self.me
                        {
                            self.out.applied(req, Outcome::Ok);
                        }
                    }
                }
                EntryType::EntryConfChangeV2 => {}
            }
            self.shared.applied.store(e.index, Ordering::SeqCst);
        }
    }

    fn ready(&mut self) {
        if !self.node.has_ready() {
            return;
        }
        let mut ready = self.node.ready();
        if !ready.messages().is_empty() {
            self.send(ready.take_messages());
        }
        if !ready.snapshot().is_empty() {
            let s = ready.snapshot().clone();
            let index = s.get_metadata().index;
            let _ = self.node.mut_store().mem.wl().apply_snapshot(s.clone());
            if let Ok(m) = postcard::from_bytes::<Machine>(&s.data) {
                *self.shared.machine.write().unwrap() = m;
            }
            self.shared.applied.store(index, Ordering::SeqCst);
            *self.snapshot.lock().unwrap() = s.clone();
            self.snap_index = index;
            let sf = SnapFile {
                index,
                term: s.get_metadata().term,
                conf: s
                    .get_metadata()
                    .get_conf_state()
                    .write_to_bytes()
                    .unwrap_or_default(),
                machine: s.data.to_vec(),
            };
            let hard = self.node.store().mem.rl().hard_state().clone();
            let _ = self.disk.checkpoint(&sf, &hard, &[]);
        }
        let committed = ready.take_committed_entries();
        self.apply(committed);
        if !ready.entries().is_empty() {
            let ents = ready.entries().clone();
            let _ = self.disk.append(&Wal::Entries(
                ents.iter()
                    .map(|e| e.write_to_bytes().expect("entries encode"))
                    .collect(),
            ));
            let _ = self.node.mut_store().mem.wl().append(&ents);
        }
        if let Some(hs) = ready.hs() {
            let _ = self
                .disk
                .append(&Wal::Hard(hs.write_to_bytes().expect("hard state encodes")));
            self.node.mut_store().mem.wl().set_hardstate(hs.clone());
        }
        if !ready.persisted_messages().is_empty() {
            self.send(ready.take_persisted_messages());
        }
        let mut light = self.node.advance(ready);
        if let Some(commit) = light.commit_index() {
            self.node
                .mut_store()
                .mem
                .wl()
                .mut_hard_state()
                .set_commit(commit);
            let hs = self.node.store().mem.rl().hard_state().clone();
            let _ = self
                .disk
                .append(&Wal::Hard(hs.write_to_bytes().expect("encodes")));
        }
        self.send(light.take_messages());
        let committed = light.take_committed_entries();
        self.apply(committed);
        self.node.advance_apply();
        self.compact();
    }

    /// Fold the applied log into a snapshot of the machine.
    fn compact(&mut self) {
        let applied = self.shared.applied.load(Ordering::SeqCst);
        if applied < self.snap_index + COMPACT_EVERY {
            return;
        }
        let Ok(term) = self.node.store().mem.term(applied) else {
            return;
        };
        let machine = encode_machine(&self.shared.machine.read().unwrap());
        let conf = self
            .node
            .store()
            .mem
            .initial_state()
            .map(|s| s.conf_state)
            .unwrap_or_default();
        let mut sn = Snapshot::default();
        sn.mut_metadata().index = applied;
        sn.mut_metadata().term = term;
        sn.mut_metadata().set_conf_state(conf.clone());
        sn.data = machine.clone().into();
        *self.snapshot.lock().unwrap() = sn;
        // keep the entry at `applied` so its term stays known
        let _ = self.node.mut_store().mem.wl().compact(applied);
        self.snap_index = applied;
        let last = self.node.store().mem.last_index().unwrap_or(applied);
        let rest = if last > applied {
            self.node
                .store()
                .mem
                .entries(applied + 1, last + 1, None, GetEntriesContext::empty(false))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let hard = self.node.store().mem.rl().hard_state().clone();
        let sf = SnapFile {
            index: applied,
            term,
            conf: conf.write_to_bytes().unwrap_or_default(),
            machine,
        };
        let _ = self.disk.checkpoint(&sf, &hard, &rest);
    }
}
