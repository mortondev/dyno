//! Game state and survival rules on the server: players, packs, items in the
//! world, mining, crafting, health and hunger. The network layer calls
//! `handle` for each message and `tick` 20 times per second.

use crate::store::{self, PlayerData};
use dyno_protocol::{
    self as proto, ClientMsg, EntityKind, EnvDamage, Mode, PlayerId, PlayerInfo, Pose, ServerMsg,
    SlotRef,
};
use dyno_sim::crafting::{self, Stations};
use dyno_sim::inventory::{Inventory, Stack, transfer, transfer_within};
use dyno_sim::mining;
use dyno_sim::rng::Rng;
use dyno_world::content::{Content, ItemId};
use dyno_world::{self as w, World};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

pub const CHEST_SLOTS: usize = 27;
const REACH: f32 = 7.5;
const ITEM_LIFE: Duration = Duration::from_secs(300);

pub fn log(msg: impl AsRef<str>) {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((t / 3600) % 24, (t / 60) % 60, t % 60);
    println!("[{h:02}:{m:02}:{s:02} UTC] {}", msg.as_ref());
}

pub struct Player {
    pub name: String,
    pub pose: Pose,
    pub tx: Sender<Arc<Vec<u8>>>,
    pub token: u64,
    pub mode: Mode,
    pub health: f32,
    pub food: f32,
    pub pack: Inventory,
    pub held: u8,
    pub fuel: u16,
    pub spawn: Option<[f32; 3]>,
    pub milestones: HashSet<String>,
    pub last_pose_at: Instant,
    pub dig: Option<([i32; 3], Instant)>,
    pub dead: bool,
    pub container: Option<[i32; 3]>,
    pub hunger_clock: f32,
    pub regen_clock: f32,
    pub saw_night: bool,
}

impl Player {
    pub fn away(&self) -> bool {
        self.last_pose_at.elapsed() > Duration::from_secs(3)
    }
    fn eye(&self) -> [f32; 3] {
        [self.pose.pos[0], self.pose.pos[1] + 1.62, self.pose.pos[2]]
    }
    fn send(&self, msg: &ServerMsg) {
        let _ = self.tx.send(Arc::new(proto::encode(msg)));
    }
    fn status(&self) -> ServerMsg {
        ServerMsg::Status {
            health: self.health,
            food: self.food,
            mode: self.mode,
            fuel: self.fuel,
        }
    }
    fn held_tool<'a>(&self, c: &'a Content) -> Option<&'a dyno_world::content::Tool> {
        self.pack
            .slots
            .get(self.held as usize)
            .copied()
            .flatten()
            .and_then(|s| c.item(s.item).tool)
    }
}

pub struct Entity {
    pub kind: EntityKind,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub yaw: f32,
    pub born: Instant,
    pub pickup_after: Instant,
    pub moved: bool,
}

pub struct State {
    pub world: World,
    pub content: Content,
    pub players: HashMap<PlayerId, Player>,
    /// invisible tool connections: they receive updates but cannot act
    pub observers: HashMap<PlayerId, Sender<Arc<Vec<u8>>>>,
    pub next_id: PlayerId,
    pub dirty: bool,
    pub saved: HashMap<String, PlayerData>,
    pub containers: HashMap<[i32; 3], Vec<Option<Stack>>>,
    pub entities: HashMap<u32, Entity>,
    pub next_entity: u32,
    pub rng: Rng,
    pub time_at_base: f32,
    pub time_base: Instant,
    pub default_mode: Mode,
    pub ops: Vec<String>,
}

/// Length of one in-game day.
pub const DAY_SECONDS: f32 = 20.0 * 60.0;

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn center(p: [i32; 3]) -> [f32; 3] {
    [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5]
}

impl State {
    pub fn time(&self) -> f32 {
        (self.time_at_base + self.time_base.elapsed().as_secs_f32() / DAY_SECONDS).rem_euclid(1.0)
    }
    pub fn is_night(&self) -> bool {
        let t = self.time();
        t > 0.52 && t < 0.98
    }
    pub fn time_msg(&self) -> ServerMsg {
        ServerMsg::Time {
            time: self.time(),
            day_seconds: DAY_SECONDS,
        }
    }

    /// Send to every player and observer, except one player.
    pub fn broadcast(&self, msg: &ServerMsg, except: Option<PlayerId>) {
        let bytes = Arc::new(proto::encode(msg));
        for (id, p) in &self.players {
            if Some(*id) != except {
                let _ = p.tx.send(bytes.clone());
            }
        }
        for (id, tx) in &self.observers {
            if Some(*id) != except {
                let _ = tx.send(bytes.clone());
            }
        }
    }

    pub fn tell(&self, id: PlayerId, text: impl Into<String>) {
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::System { text: text.into() });
        }
    }

    pub fn may_op(&self, name: &str) -> bool {
        self.ops.is_empty() || self.ops.iter().any(|o| o.eq_ignore_ascii_case(name))
    }

    // ---------------- joining and leaving ----------------

    /// Create a player from saved data, or a new one. Errors are shown to the player.
    pub fn join(
        &mut self,
        name: &str,
        token: u64,
        tx: Sender<Arc<Vec<u8>>>,
    ) -> Result<(PlayerId, Vec<ServerMsg>), String> {
        let key = name.to_lowercase();
        if self
            .players
            .values()
            .any(|p| p.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("{name} is already playing. Choose another name."));
        }
        let data = match self.saved.get(&key) {
            Some(d) if d.token != token && d.token != 0 => {
                return Err(format!(
                    "The name {name} belongs to another player on this server. Choose another name."
                ));
            }
            Some(d) => d.clone(),
            None => PlayerData::new(self.default_mode, self.world.spawn()),
        };
        let id = self.next_id;
        self.next_id += 1;
        let pack = store::pack_from_keys(&self.content, &data.pack);
        let pose = Pose {
            pos: data.pos,
            yaw: 0.0,
            pitch: 0.0,
        };
        let p = Player {
            name: name.to_string(),
            pose,
            tx,
            token,
            mode: data.mode,
            health: data.health,
            food: data.food,
            pack,
            held: 0,
            fuel: data.fuel,
            spawn: data.spawn,
            milestones: data.milestones.iter().cloned().collect(),
            last_pose_at: Instant::now(),
            dig: None,
            dead: data.health <= 0.0,
            container: None,
            hunger_clock: 0.0,
            regen_clock: 0.0,
            saw_night: false,
        };
        let others: Vec<PlayerInfo> = self
            .players
            .iter()
            .map(|(i, q)| PlayerInfo {
                id: *i,
                name: q.name.clone(),
                pose: q.pose,
            })
            .collect();
        let welcome = ServerMsg::Welcome {
            your_id: id,
            server_version: env!("CARGO_PKG_VERSION").into(),
            motd: String::new(),
            size: self.world.size,
            spawn: data.pos,
            players: others,
            mode: p.mode,
            content: Box::new(self.content.clone()),
        };
        let mut after = vec![ServerMsg::Pack(p.pack.slots.clone()), p.status()];
        for (eid, e) in &self.entities {
            after.push(ServerMsg::EntitySpawn {
                id: *eid,
                kind: e.kind,
                pos: e.pos,
                yaw: e.yaw,
            });
        }
        if p.dead {
            after.push(ServerMsg::Died {
                message: "You died.".into(),
            });
        }
        self.broadcast(
            &ServerMsg::PlayerJoined(PlayerInfo {
                id,
                name: name.to_string(),
                pose,
            }),
            None,
        );
        self.broadcast(
            &ServerMsg::System {
                text: format!("{name} joined"),
            },
            None,
        );
        self.players.insert(id, p);
        Ok((id, [vec![welcome], after].concat()))
    }

    pub fn leave(&mut self, id: PlayerId) {
        if let Some(p) = self.players.remove(&id) {
            self.remember(&p);
            self.broadcast(&ServerMsg::PlayerLeft { id }, None);
            self.broadcast(
                &ServerMsg::System {
                    text: format!("{} left", p.name),
                },
                None,
            );
        }
        self.observers.remove(&id);
    }

    pub fn remember(&mut self, p: &Player) {
        let data = PlayerData {
            token: p.token,
            mode: p.mode,
            health: p.health,
            food: p.food,
            pos: p.pose.pos,
            spawn: p.spawn,
            pack: store::pack_to_keys(&self.content, &p.pack),
            fuel: p.fuel,
            milestones: p.milestones.iter().cloned().collect(),
        };
        self.saved.insert(p.name.to_lowercase(), data);
    }

    // ---------------- messages ----------------

    /// Handle one message. Err disconnects the player (only for bad input).
    pub fn handle(&mut self, id: PlayerId, msg: ClientMsg) -> Result<(), String> {
        if self.observers.contains_key(&id) {
            return Ok(());
        }
        if !self.players.contains_key(&id) {
            return Ok(());
        }
        match msg {
            ClientMsg::Pose(pose) => {
                let p = self.players.get_mut(&id).unwrap();
                if pose
                    .pos
                    .iter()
                    .chain([pose.yaw, pose.pitch].iter())
                    .all(|v| v.is_finite())
                    && !p.dead
                {
                    p.pose = pose;
                    p.last_pose_at = Instant::now();
                }
            }
            ClientMsg::Held { slot } => {
                if slot < 9 {
                    self.players.get_mut(&id).unwrap().held = slot;
                }
            }
            ClientMsg::DigStart { pos } => {
                let p = self.players.get_mut(&id).unwrap();
                p.dig = Some((pos, Instant::now()));
                self.broadcast(
                    &ServerMsg::Digging {
                        player: id,
                        pos: Some(pos),
                    },
                    Some(id),
                );
            }
            ClientMsg::Dig { pos } => self.dig(id, pos),
            ClientMsg::Place { pos, slot } => self.place(id, pos, slot),
            ClientMsg::SetBlock { pos, block } => {
                if block >= w::BLOCK_COUNT {
                    return Err(format!("unknown block {block}"));
                }
                if self.players[&id].mode == Mode::Creative
                    && self.within(id, pos, REACH + 4.0)
                    && self.world.in_bounds(pos)
                {
                    self.set_block(pos, block);
                } else {
                    self.resync(id, pos);
                }
            }
            ClientMsg::Fill { a, b, block } => self.fill(id, a, b, block)?,
            ClientMsg::CreativeSet { slot, item } => {
                let c = &self.content;
                let p = self.players.get_mut(&id).unwrap();
                if p.mode == Mode::Creative && (slot as usize) < p.pack.slots.len() {
                    p.pack.slots[slot as usize] = if item == 0 {
                        None
                    } else {
                        Some(Stack::new(item, c.item(item).stack))
                    };
                    p.send(&ServerMsg::Pack(p.pack.slots.clone()));
                }
            }
            ClientMsg::Move { from, to, count } => self.move_items(id, from, to, count),
            ClientMsg::Throw { from, count } => self.throw(id, from, count),
            ClientMsg::Craft { recipe, times } => self.craft(id, recipe, times.clamp(1, 64)),
            ClientMsg::UseHeld => self.use_held(id),
            ClientMsg::Attack { entity: _ } => {}
            ClientMsg::Interact { pos } => self.interact(id, pos),
            ClientMsg::CloseContainer => {
                self.players.get_mut(&id).unwrap().container = None;
            }
            ClientMsg::Hurt { kind, amount } => {
                let what = match kind {
                    EnvDamage::Fall => "fell from a high place",
                    EnvDamage::Drown => "drowned",
                };
                self.damage(id, amount.min(40) as f32, [0.0; 3], what);
            }
            ClientMsg::Respawn => self.respawn(id),
            ClientMsg::Chat { .. } | ClientMsg::Hello { .. } | ClientMsg::Session { .. } => {}
        }
        Ok(())
    }

    fn within(&self, id: PlayerId, pos: [i32; 3], reach: f32) -> bool {
        self.players
            .get(&id)
            .is_some_and(|p| dist(p.eye(), center(pos)) <= reach)
    }

    fn resync(&self, id: PlayerId, pos: [i32; 3]) {
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Block {
                pos,
                block: self.world.get(pos[0], pos[1], pos[2]),
            });
        }
    }

    pub fn set_block(&mut self, pos: [i32; 3], block: u8) {
        self.world.set(pos[0], pos[1], pos[2], block);
        self.dirty = true;
        self.broadcast(&ServerMsg::Block { pos, block }, None);
    }

    fn dig(&mut self, id: PlayerId, pos: [i32; 3]) {
        let b = self.world.get(pos[0], pos[1], pos[2]);
        let p = &self.players[&id];
        if b == w::AIR || b == w::WATER || p.dead || !self.within(id, pos, REACH) {
            return self.resync(id, pos);
        }
        let creative = p.mode == Mode::Creative;
        let tool = p.held_tool(&self.content).cloned();
        if !creative {
            let Some(need) = mining::break_time(&self.content, b, tool.as_ref()) else {
                return self.resync(id, pos);
            };
            let ok = match p.dig {
                Some((start, at)) => {
                    start == pos && at.elapsed().as_secs_f32() >= need * 0.7 - 0.25
                }
                None => need < 0.05,
            };
            if !ok {
                return self.resync(id, pos);
            }
        }
        let drops = if creative {
            Vec::new()
        } else {
            mining::drops(&self.content, b, tool.as_ref(), &mut self.rng)
        };
        if b == w::CHEST
            && let Some(slots) = self.containers.remove(&pos)
        {
            for s in slots.into_iter().flatten() {
                self.spawn_item(center(pos), s, 0.3);
            }
        }
        self.set_block(pos, w::AIR);
        for s in drops {
            self.spawn_item(center(pos), s, 0.25);
        }
        let held = self.players[&id].held as usize;
        let p = self.players.get_mut(&id).unwrap();
        p.dig = None;
        if !creative && tool.is_some() {
            // tools wear out
            if let Some(s) = p.pack.slots[held].as_mut() {
                s.wear += 1;
                let max = tool.as_ref().map(|t| t.durability).unwrap_or(u16::MAX);
                if s.wear >= max {
                    p.pack.slots[held] = None;
                    p.send(&ServerMsg::System {
                        text: "Your tool broke.".into(),
                    });
                }
            }
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
        self.broadcast(
            &ServerMsg::Digging {
                player: id,
                pos: None,
            },
            Some(id),
        );
    }

    fn place(&mut self, id: PlayerId, pos: [i32; 3], slot: u8) {
        let p = &self.players[&id];
        let Some(stack) = p.pack.slots.get(slot as usize).copied().flatten() else {
            return self.resync(id, pos);
        };
        let Some(block) = self.content.item(stack.item).block else {
            return self.resync(id, pos);
        };
        let here = self.world.get(pos[0], pos[1], pos[2]);
        if p.dead
            || !(here == w::AIR || here == w::WATER)
            || !self.world.in_bounds(pos)
            || !self.within(id, pos, REACH + 1.0)
        {
            return self.resync(id, pos);
        }
        // no placing solid blocks inside a player
        if w::is_solid(block) {
            for q in self.players.values() {
                let f = q.pose.pos;
                let inside = (pos[0] as f32) < f[0] + 0.3
                    && (pos[0] + 1) as f32 > f[0] - 0.3
                    && (pos[2] as f32) < f[2] + 0.3
                    && (pos[2] + 1) as f32 > f[2] - 0.3
                    && (pos[1] as f32) < f[1] + 1.8
                    && (pos[1] + 1) as f32 > f[1];
                if inside {
                    return self.resync(id, pos);
                }
            }
        }
        let creative = p.mode == Mode::Creative;
        if !creative {
            let p = self.players.get_mut(&id).unwrap();
            p.pack.take(slot as usize, 1);
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
        self.set_block(pos, block);
        if block == w::CHEST {
            self.containers.insert(pos, vec![None; CHEST_SLOTS]);
        }
        let key = format!("place:{}", w::block_key(block));
        self.milestone(id, &key);
    }

    fn fill(&mut self, id: PlayerId, a: [i32; 3], b: [i32; 3], block: u8) -> Result<(), String> {
        if block >= w::BLOCK_COUNT {
            return Err(format!("unknown block {block}"));
        }
        if self.players[&id].mode != Mode::Creative {
            self.tell(id, "Fill works in creative mode only.");
            return Ok(());
        }
        let [sx, sy, sz] = self.world.size;
        let lo = [
            a[0].min(b[0]).max(0),
            a[1].min(b[1]).max(0),
            a[2].min(b[2]).max(0),
        ];
        let hi = [
            a[0].max(b[0]).min(sx as i32 - 1),
            a[1].max(b[1]).min(sy as i32 - 1),
            a[2].max(b[2]).min(sz as i32 - 1),
        ];
        if (0..3).any(|i| lo[i] > hi[i]) {
            return Ok(());
        }
        let vol = (0..3).map(|i| (hi[i] - lo[i] + 1) as i64).product::<i64>();
        let me = self.players[&id].pose.pos;
        let c: Vec<f32> = (0..3).map(|i| (lo[i] + hi[i]) as f32 * 0.5).collect();
        let d2: f32 = (0..3).map(|i| (c[i] - me[i]).powi(2)).sum();
        if vol > proto::MAX_FILL || d2 > 96.0 * 96.0 {
            self.tell(
                id,
                format!(
                    "Fill refused: at most {} cells, within 96 blocks of you.",
                    proto::MAX_FILL
                ),
            );
            return Ok(());
        }
        for y in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                for x in lo[0]..=hi[0] {
                    if self.world.get(x, y, z) == w::CHEST {
                        self.containers.remove(&[x, y, z]);
                    }
                    self.world.set(x, y, z, block);
                }
            }
        }
        self.dirty = true;
        log(format!(
            "{} filled {vol} cells with {}",
            self.players[&id].name,
            w::block(block).name
        ));
        self.broadcast(
            &ServerMsg::Fill {
                a: lo,
                b: hi,
                block,
            },
            None,
        );
        Ok(())
    }

    /// Look up a slot for reading and writing. Chest slots need an open chest in reach.
    fn slot_ok(&self, id: PlayerId, s: SlotRef) -> bool {
        match s {
            SlotRef::Pack(i) => (i as usize) < dyno_sim::SLOTS,
            SlotRef::Chest(i) => {
                (i as usize) < CHEST_SLOTS
                    && self.players[&id].container.is_some_and(|c| {
                        self.containers.contains_key(&c) && self.within(id, c, REACH + 1.0)
                    })
            }
        }
    }

    fn move_items(&mut self, id: PlayerId, from: SlotRef, to: SlotRef, count: u8) {
        if !self.slot_ok(id, from) || !self.slot_ok(id, to) || self.players[&id].dead {
            return self.send_pack(id);
        }
        let chest = self.players[&id].container;
        let c = &self.content;
        let p = self.players.get_mut(&id).unwrap();
        match (from, to) {
            (SlotRef::Pack(a), SlotRef::Pack(b)) => {
                transfer_within(c, &mut p.pack.slots, a as usize, b as usize, count)
            }
            (SlotRef::Chest(a), SlotRef::Chest(b)) => {
                let slots = self.containers.get_mut(&chest.unwrap()).unwrap();
                transfer_within(c, slots, a as usize, b as usize, count);
            }
            (SlotRef::Pack(a), SlotRef::Chest(b)) => {
                let slots = self.containers.get_mut(&chest.unwrap()).unwrap();
                transfer(
                    c,
                    &mut p.pack.slots[a as usize],
                    &mut slots[b as usize],
                    count,
                );
            }
            (SlotRef::Chest(a), SlotRef::Pack(b)) => {
                let slots = self.containers.get_mut(&chest.unwrap()).unwrap();
                transfer(
                    c,
                    &mut slots[a as usize],
                    &mut p.pack.slots[b as usize],
                    count,
                );
            }
        }
        self.send_pack(id);
        if let Some(pos) = chest
            && (matches!(from, SlotRef::Chest(_)) || matches!(to, SlotRef::Chest(_)))
        {
            self.dirty = true;
            self.send_container(pos);
        }
    }

    fn send_pack(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
    }

    /// Send a chest's contents to everyone who has it open.
    fn send_container(&self, pos: [i32; 3]) {
        let Some(slots) = self.containers.get(&pos) else {
            return;
        };
        let msg = ServerMsg::Container(Some((pos, slots.clone())));
        for p in self.players.values().filter(|p| p.container == Some(pos)) {
            p.send(&msg);
        }
    }

    fn throw(&mut self, id: PlayerId, from: SlotRef, count: u8) {
        if !self.slot_ok(id, from) {
            return;
        }
        let chest = self.players[&id].container;
        let p = self.players.get_mut(&id).unwrap();
        let taken = match from {
            SlotRef::Pack(i) => p.pack.take(i as usize, count),
            SlotRef::Chest(i) => {
                let slots = self.containers.get_mut(&chest.unwrap()).unwrap();
                let s = slots[i as usize].as_mut();
                match s {
                    Some(st) => {
                        let n = if count == 0 {
                            st.count
                        } else {
                            count.min(st.count)
                        };
                        let out = Stack { count: n, ..*st };
                        st.count -= n;
                        if st.count == 0 {
                            slots[i as usize] = None;
                        }
                        Some(out)
                    }
                    None => None,
                }
            }
        };
        let Some(s) = taken else { return };
        let p = &self.players[&id];
        let (sy, cy) = p.pose.yaw.sin_cos();
        let (sp, cp) = p.pose.pitch.sin_cos();
        let dir = [-sy * cp, sp, -cy * cp];
        let eye = p.eye();
        let start = [eye[0] + dir[0] * 0.4, eye[1] - 0.3, eye[2] + dir[2] * 0.4];
        let eid = self.spawn_item(start, s, 0.0);
        if let Some(e) = self.entities.get_mut(&eid) {
            e.vel = [dir[0] * 5.0, dir[1] * 5.0 + 2.0, dir[2] * 5.0];
            e.pickup_after = Instant::now() + Duration::from_millis(1500);
        }
        self.send_pack(id);
        if let Some(pos) = chest {
            self.send_container(pos);
        }
    }

    fn stations_near(&self, id: PlayerId) -> Stations {
        let p = self.players[&id].pose.pos;
        let mut st = Stations::default();
        let (bx, by, bz) = (
            p[0].floor() as i32,
            p[1].floor() as i32,
            p[2].floor() as i32,
        );
        for y in by - 3..=by + 4 {
            for z in bz - 5..=bz + 5 {
                for x in bx - 5..=bx + 5 {
                    match self.world.get(x, y, z) {
                        w::CRAFTING_TABLE => st.table = true,
                        w::FURNACE => st.furnace = true,
                        _ => {}
                    }
                }
            }
        }
        st
    }

    fn craft(&mut self, id: PlayerId, recipe: u16, times: u8) {
        let Some(r) = self.content.recipes.get(recipe as usize).cloned() else {
            return;
        };
        let st = self.stations_near(id);
        let mut made = 0;
        let mut spill = Vec::new();
        let mut why = None;
        {
            let c = &self.content;
            let p = self.players.get_mut(&id).unwrap();
            if p.dead {
                return;
            }
            for _ in 0..times {
                match crafting::craft(c, &r, &mut p.pack, st, &mut p.fuel) {
                    Ok(left) => {
                        made += 1;
                        if let Some(l) = left {
                            spill.push(l);
                        }
                    }
                    Err(e) => {
                        why = Some(e);
                        break;
                    }
                }
            }
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
            p.send(&p.status());
        }
        let pos = self.players[&id].pose.pos;
        for s in spill {
            self.spawn_item([pos[0], pos[1] + 1.0, pos[2]], s, 0.2);
        }
        if made == 0 {
            if let Some(e) = why {
                self.tell(id, e);
            }
        } else {
            let key = format!("craft:{}", self.content.item_key(r.output));
            self.milestone(id, &key);
        }
    }

    fn use_held(&mut self, id: PlayerId) {
        let c = &self.content;
        let p = self.players.get_mut(&id).unwrap();
        let slot = p.held as usize;
        let Some(s) = p.pack.slots[slot] else { return };
        let food = c.item(s.item).food;
        if food == 0 || p.dead || p.mode == Mode::Creative || p.food >= 20.0 {
            return;
        }
        p.pack.take(slot, 1);
        p.food = (p.food + food as f32).min(20.0);
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        p.send(&p.status());
    }

    fn interact(&mut self, id: PlayerId, pos: [i32; 3]) {
        if !self.within(id, pos, REACH + 1.0) {
            return;
        }
        match self.world.get(pos[0], pos[1], pos[2]) {
            w::CHEST => {
                self.containers
                    .entry(pos)
                    .or_insert_with(|| vec![None; CHEST_SLOTS]);
                self.players.get_mut(&id).unwrap().container = Some(pos);
                self.send_container(pos);
            }
            w::BED => {
                let p = self.players.get_mut(&id).unwrap();
                p.spawn = Some([
                    pos[0] as f32 + 0.5,
                    pos[1] as f32 + 1.01,
                    pos[2] as f32 + 0.5,
                ]);
                p.send(&ServerMsg::System {
                    text: "Spawn point set at your bed.".into(),
                });
            }
            _ => {}
        }
    }

    pub fn damage(&mut self, id: PlayerId, amount: f32, push: [f32; 3], cause: &str) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if p.mode == Mode::Creative || p.dead || amount <= 0.0 {
            return;
        }
        p.health = (p.health - amount).max(0.0);
        p.send(&ServerMsg::Hurt { amount, push });
        p.send(&p.status());
        if p.health <= 0.0 {
            p.dead = true;
            p.container = None;
            let items = p.pack.drain();
            let pos = p.pose.pos;
            let name = p.name.clone();
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
            p.send(&ServerMsg::Died {
                message: format!("You {cause}."),
            });
            for s in items {
                self.spawn_item([pos[0], pos[1] + 0.5, pos[2]], s, 0.35);
            }
            self.broadcast(
                &ServerMsg::System {
                    text: format!("{name} {cause}."),
                },
                None,
            );
        }
    }

    fn respawn(&mut self, id: PlayerId) {
        let spawn = self.world.spawn();
        let p = self.players.get_mut(&id).unwrap();
        if !p.dead {
            return;
        }
        p.dead = false;
        p.health = 20.0;
        p.food = 20.0;
        let to = p.spawn.unwrap_or(spawn);
        p.pose.pos = to;
        p.send(&ServerMsg::Teleport { pos: to });
        p.send(&p.status());
    }

    pub fn milestone(&mut self, id: PlayerId, trigger: &str) {
        let Some(m) = self
            .content
            .milestones
            .iter()
            .find(|m| m.trigger == trigger)
            .cloned()
        else {
            return;
        };
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if !p.milestones.insert(m.key.clone()) {
            return;
        }
        p.send(&ServerMsg::Milestone {
            title: m.title.clone(),
        });
        let name = p.name.clone();
        self.broadcast(
            &ServerMsg::System {
                text: format!("{name} reached {}", m.title),
            },
            None,
        );
    }

    // ---------------- items in the world ----------------

    /// Add an item entity. `spread` is the random sideways speed.
    pub fn spawn_item(&mut self, pos: [f32; 3], s: Stack, spread: f32) -> u32 {
        let id = self.next_entity;
        self.next_entity += 1;
        let r = &mut self.rng;
        let vel = [
            (r.f32() - 0.5) * spread * 10.0,
            3.0 + r.f32() * 2.0 * spread.max(0.1),
            (r.f32() - 0.5) * spread * 10.0,
        ];
        let e = Entity {
            kind: EntityKind::Item(s),
            pos,
            vel,
            yaw: r.f32() * std::f32::consts::TAU,
            born: Instant::now(),
            pickup_after: Instant::now() + Duration::from_millis(500),
            moved: true,
        };
        self.broadcast(
            &ServerMsg::EntitySpawn {
                id,
                kind: e.kind,
                pos: e.pos,
                yaw: e.yaw,
            },
            None,
        );
        self.entities.insert(id, e);
        id
    }

    fn remove_entity(&mut self, id: u32, picked_by: Option<PlayerId>) {
        if self.entities.remove(&id).is_some() {
            self.broadcast(&ServerMsg::EntityGone { id, picked_by }, None);
        }
    }

    /// Advance the world by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.tick_items(dt);
        self.tick_players(dt);
    }

    fn tick_items(&mut self, dt: f32) {
        let world = &self.world;
        for e in self.entities.values_mut() {
            if !matches!(e.kind, EntityKind::Item(_)) {
                continue;
            }
            let before = e.pos;
            e.vel[1] -= 20.0 * dt;
            for axis in 0..3 {
                let mut next = e.pos;
                next[axis] += e.vel[axis] * dt;
                let cell = |p: [f32; 3]| {
                    world.get(
                        p[0].floor() as i32,
                        (p[1] - 0.12).floor() as i32,
                        p[2].floor() as i32,
                    )
                };
                if w::is_solid(cell(next)) {
                    if axis == 1 && e.vel[1] < 0.0 {
                        e.pos[1] = (next[1] - 0.12).floor() + 1.12;
                    }
                    e.vel[axis] = 0.0;
                    if axis == 1 {
                        e.vel[0] *= 0.6;
                        e.vel[2] *= 0.6;
                    }
                } else {
                    e.pos[axis] = next[axis];
                }
            }
            if e.pos[1] < -10.0 {
                e.pos[1] = -10.0;
            }
            if dist(before, e.pos) > 0.001 {
                e.moved = true;
            }
        }
        // pick up
        let mut picks: Vec<(u32, PlayerId)> = Vec::new();
        for (eid, e) in &self.entities {
            if !matches!(e.kind, EntityKind::Item(_)) || e.pickup_after > Instant::now() {
                continue;
            }
            for (pid, p) in &self.players {
                if p.dead {
                    continue;
                }
                let f = p.pose.pos;
                let d = ((e.pos[0] - f[0]).powi(2) + (e.pos[2] - f[2]).powi(2)).sqrt();
                if d < 1.6 && e.pos[1] > f[1] - 1.0 && e.pos[1] < f[1] + 2.4 {
                    picks.push((*eid, *pid));
                    break;
                }
            }
        }
        for (eid, pid) in picks {
            let Some(EntityKind::Item(s)) = self.entities.get(&eid).map(|e| e.kind) else {
                continue;
            };
            let c = &self.content;
            let p = self.players.get_mut(&pid).unwrap();
            let left = p.pack.add(c, s);
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
            let key = format!("collect:{}", c.item_key(s.item));
            match left {
                None => self.remove_entity(eid, Some(pid)),
                Some(rest) if rest.count < s.count => {
                    // part fit: replace the entity with the rest
                    let pos = self.entities[&eid].pos;
                    self.remove_entity(eid, Some(pid));
                    let nid = self.spawn_item(pos, rest, 0.0);
                    if let Some(e) = self.entities.get_mut(&nid) {
                        e.vel = [0.0; 3];
                        e.pickup_after = Instant::now() + Duration::from_secs(2);
                    }
                }
                Some(_) => {
                    if let Some(e) = self.entities.get_mut(&eid) {
                        e.pickup_after = Instant::now() + Duration::from_secs(2);
                    }
                }
            }
            self.milestone(pid, &key);
        }
        // expire
        let old: Vec<u32> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(e.kind, EntityKind::Item(_)) && e.born.elapsed() > ITEM_LIFE)
            .map(|(i, _)| *i)
            .collect();
        for i in old {
            self.remove_entity(i, None);
        }
    }

    /// Positions of entities that moved, then clear the flags.
    pub fn entity_moves(&mut self) -> Option<ServerMsg> {
        let moves: Vec<(u32, [f32; 3], f32)> = self
            .entities
            .iter_mut()
            .filter(|(_, e)| e.moved)
            .map(|(i, e)| {
                e.moved = false;
                (*i, e.pos, e.yaw)
            })
            .collect();
        if moves.is_empty() {
            None
        } else {
            Some(ServerMsg::EntityMoves(moves))
        }
    }

    fn tick_players(&mut self, dt: f32) {
        let night = self.is_night();
        let mut starving = Vec::new();
        let mut milestones = Vec::new();
        for (id, p) in self.players.iter_mut() {
            if p.mode == Mode::Creative || p.dead {
                continue;
            }
            if night && !p.away() {
                p.saw_night = true;
            }
            if !night && p.saw_night {
                p.saw_night = false;
                milestones.push(*id);
            }
            if p.away() {
                continue;
            }
            let mut changed = false;
            // hunger drains slowly
            p.hunger_clock += dt;
            if p.hunger_clock >= 45.0 {
                p.hunger_clock = 0.0;
                p.food = (p.food - 1.0).max(0.0);
                changed = true;
            }
            p.regen_clock += dt;
            if p.regen_clock >= 4.0 {
                p.regen_clock = 0.0;
                if p.food >= 18.0 && p.health < 20.0 {
                    p.health = (p.health + 1.0).min(20.0);
                    p.food = (p.food - 0.25).max(0.0);
                    changed = true;
                } else if p.food <= 0.0 && p.health > 1.0 {
                    starving.push(*id);
                }
            }
            if changed {
                p.send(&p.status());
            }
        }
        for id in starving {
            self.damage(id, 1.0, [0.0; 3], "starved");
        }
        for id in milestones {
            self.milestone(id, "night");
        }
    }

    /// Swap content (after /reload). Items keep their keys; IDs may change.
    pub fn replace_content(&mut self, new: Content) {
        let old = std::mem::replace(&mut self.content, new);
        let remap = |s: &mut Option<Stack>, old: &Content, new: &Content| {
            if let Some(st) = s {
                match new.item_by_key(&old.item_key(st.item)) {
                    Some(id) => st.item = id,
                    None => *s = None,
                }
            }
        };
        let c = self.content.clone();
        for p in self.players.values_mut() {
            for s in p.pack.slots.iter_mut() {
                remap(s, &old, &c);
            }
        }
        for slots in self.containers.values_mut() {
            for s in slots.iter_mut() {
                remap(s, &old, &c);
            }
        }
        let ids: Vec<u32> = self.entities.keys().copied().collect();
        for i in ids {
            if let Some(EntityKind::Item(_)) = self.entities.get(&i).map(|e| e.kind) {
                self.remove_entity(i, None);
            }
        }
        self.broadcast(&ServerMsg::Content(Box::new(self.content.clone())), None);
        for p in self.players.values() {
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
    }

    pub fn item_id(&self, key: &str) -> Option<ItemId> {
        self.content.item_by_key(key)
    }
}
