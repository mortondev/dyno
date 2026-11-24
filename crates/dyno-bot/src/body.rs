//! A headless player: a copy of the world, real player physics, everything
//! the server tells us (entities, health, bags, quests), and a path follower.
//! Brains and the text interface drive a body; it does no thinking itself.

use crate::nav::{self, Cell, Goal};
use dyno_client::game::{Input, Player};
use dyno_client::net::{self, Joined, NetEvent};
use dyno_client::predict::Predictor;
use dyno_protocol::{
    self as proto, Appearance, ClientMsg, EntityKind, EnvDamage, NewCharacter, PlayerId,
    QuestProgress, ServerMsg, Stack,
};
use dyno_world::content::{Content, Objective};
use dyno_world::{self as w, World};
use glam::Vec3;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// Physics step and pose rate: the real client sends a pose every 50 ms.
pub const TICK: f32 = 0.05;

/// Something we know about in the world.
#[derive(Clone, Debug)]
pub struct Ent {
    pub id: u32,
    pub kind: EntityKind,
    pub pos: Vec3,
    pub level: u8,
    pub health: f32,
    pub max: f32,
    pub name: Option<String>,
    pub elite: bool,
    /// when we last hit it
    pub my_hit: Option<Instant>,
    /// when anyone last hurt it
    pub hurt_at: Option<Instant>,
    pub seen: Instant,
}

impl Ent {
    pub fn creature(&self) -> Option<u16> {
        match self.kind {
            EntityKind::Creature(c) => Some(c),
            _ => None,
        }
    }
    pub fn npc(&self) -> Option<u16> {
        match self.kind {
            EntityKind::Npc(n) => Some(n),
            _ => None,
        }
    }
}

/// Things that happened, for brains, telemetry and the text interface.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Gain {
        xp: u32,
        money: u32,
    },
    LevelUp(u8),
    /// a creature we fought died and we got the credit (XP or quest count)
    Kill {
        creature: u16,
        level: u8,
        xp: u32,
    },
    /// a creature we hit died, but the credit went to someone else (or it was grey)
    KillNoCredit {
        creature: u16,
        level: u8,
    },
    QuestAccepted(u16),
    QuestDone(u16),
    QuestProgress(u16),
    /// items that arrived in the bags (count)
    Item {
        item: u16,
        count: u32,
        first: bool,
    },
    ItemLost {
        item: u16,
        count: u32,
    },
    Money(i64),
    Died(String),
    Respawned,
    Hurt(f32),
    Dealt {
        entity: u32,
        amount: f32,
    },
    Zone {
        name: String,
        safe: bool,
    },
    System(String),
    Chat {
        from: String,
        text: String,
    },
    WorldEvent(String),
    /// the server moved us: (from, to)
    Teleport(Vec3, Vec3),
    Invite(String),
    Dialog(u16),
    Loot {
        id: u32,
        money: u32,
        items: usize,
    },
    Closed(String),
}

/// Our path following, with stuck detection.
#[derive(Clone, Debug, Default)]
pub struct Nav {
    pub goal: Option<Goal>,
    pub path: Vec<Cell>,
    pub idx: usize,
    pub partial: bool,
    pub repaths: u32,
    /// stuck checks without progress
    pub stalls: u32,
    pub check_at: Option<Instant>,
    pub check_pos: Vec3,
    pub best: f32,
    pub state: NavState,
    pub jump_for: f32,
    pub strafe_for: f32,
    pub strafe_dir: f32,
    pub path_at: Option<Instant>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum NavState {
    #[default]
    Idle,
    Moving,
    Arrived,
    Failed(String),
}

/// A place where a bot could not get on.
#[derive(Clone, Debug)]
pub struct Stuck {
    pub at: Vec3,
    pub goal: [f32; 3],
    pub why: String,
}

pub struct Body {
    pub tx: Sender<ClientMsg>,
    rx: std::sync::mpsc::Receiver<NetEvent>,
    pub name: String,
    pub id: PlayerId,
    pub world: World,
    pub content: Content,
    pub hearth: [i32; 3],
    /// the body: predicted here, run by the server (dyno_client::predict)
    pub player: Player,
    pub pred: Predictor,
    pub started: Instant,

    pub ents: HashMap<u32, Ent>,
    pub players: HashMap<PlayerId, (String, Vec3)>,

    pub health: f32,
    pub max_health: f32,
    pub power: f32,
    pub max_power: f32,
    pub level: u8,
    pub xp: u32,
    pub xp_next: u32,
    pub money: u32,
    pub rested: u32,
    pub class: Option<u16>,
    pub pack: Vec<Option<Stack>>,
    pub equip: Vec<Option<Stack>>,
    pub quests: Vec<QuestProgress>,
    pub done: HashSet<u16>,
    pub skills: HashMap<String, u16>,
    pub cooldowns: HashMap<u16, Instant>,
    pub gcd_until: Instant,
    pub casting: Option<(u16, Instant)>,
    pub auras: HashMap<u16, Instant>,
    pub combo: (u8, Option<u32>),
    pub attacking: Option<u32>,
    pub zone: (String, [u8; 2], bool),
    pub dead: Option<String>,
    pub died_at: Option<Instant>,
    time: f32,
    time_at: Instant,
    day_seconds: f32,
    pub world_event: Option<(String, String)>,
    pub dialog: Option<(u32, u16)>,
    pub loot_open: Option<(u32, u32, Vec<Stack>)>,
    pub in_combat_until: Instant,
    pub last_hurt: Option<Instant>,
    pub closed: Option<String>,
    pub party: Vec<String>,
    pub invite: Option<String>,

    /// events not yet taken by the brain or interface
    pub events: Vec<(Instant, Ev)>,
    /// items ever held, for "new item" rewards
    pub seen_items: HashSet<u16>,
    /// a creature we fought died: (kind, level, when, quest credit seen)
    pending_kill: Option<(u16, u8, Instant, bool)>,
    /// where creatures we fought died (for finding our corpses)
    kill_spots: Vec<(Vec3, Instant)>,
    /// corpses of creatures we fought, newest last
    pub my_corpses: Vec<(u32, Instant)>,
    expect_quest_xp: bool,
    got_quests: bool,
    got_pack: bool,
    got_character: bool,

    /// until then a teleport is ours (respawn, recall, charge, /spawn)
    pub teleport_ok_until: Instant,
    pub nav: Nav,
    pub stuck_log: Vec<Stuck>,
    pub distance: f32,
    pub input: Input,
    fall_from: Option<f32>,
    air: f32,
    drown_clock: f32,
    last_tick: Instant,
    /// time from an action to the server's answer (ms), for health checks
    pub latency: Vec<u32>,
    latency_probe: Option<(Instant, Probe)>,
    pub messages: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Probe {
    Talk,
    Attack,
}

/// A stable token per bot name (so a bot keeps its character).
pub fn token_for(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("dyno-bot/{name}").bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

impl Body {
    /// Join a server as `name`, creating the character with `class` if new.
    pub fn join(
        addr: &str,
        name: &str,
        class: Option<&str>,
        look: Appearance,
    ) -> Result<Body, String> {
        let create = class.map(|c| NewCharacter {
            class: c.to_string(),
            look,
        });
        let j = net::connect_as(
            addr,
            name,
            token_for(name),
            false,
            create,
            Duration::from_secs(20),
        )?;
        Ok(Body::new(j))
    }

    pub fn new(j: Joined) -> Body {
        // the server names it: the world round it may not have streamed in
        let hearth = j.hearth;
        let mut player = Player::new(j.spawn);
        dyno_sim::movement::apply_rules(&mut player, j.content.world.protected, false);
        let now = Instant::now();
        Body {
            tx: j.conn.tx,
            rx: j.conn.rx,
            name: j.name,
            id: j.your_id,
            world: j.world,
            content: j.content,
            hearth,
            player,
            pred: Predictor::new(),
            started: now,
            ents: HashMap::new(),
            players: j
                .players
                .iter()
                .map(|p| (p.id, (p.name.clone(), Vec3::from(p.pose.pos))))
                .collect(),
            health: 50.0,
            max_health: 50.0,
            power: 0.0,
            max_power: 0.0,
            level: 1,
            xp: 0,
            xp_next: 400,
            money: 0,
            rested: 0,
            class: None,
            pack: vec![None; 36],
            equip: vec![None; 17],
            quests: Vec::new(),
            done: HashSet::new(),
            skills: HashMap::new(),
            cooldowns: HashMap::new(),
            gcd_until: now,
            casting: None,
            auras: HashMap::new(),
            combo: (0, None),
            attacking: None,
            zone: (String::new(), [1, 1], false),
            dead: None,
            died_at: None,
            time: 0.1,
            time_at: now,
            day_seconds: 1200.0,
            world_event: None,
            dialog: None,
            loot_open: None,
            in_combat_until: now,
            last_hurt: None,
            closed: None,
            party: Vec::new(),
            invite: None,
            events: Vec::new(),
            seen_items: HashSet::new(),
            pending_kill: None,
            kill_spots: Vec::new(),
            my_corpses: Vec::new(),
            expect_quest_xp: false,
            got_quests: false,
            got_pack: false,
            got_character: false,
            teleport_ok_until: now + Duration::from_secs(5),
            nav: Nav::default(),
            stuck_log: Vec::new(),
            distance: 0.0,
            input: Input::default(),
            fall_from: None,
            air: 10.0,
            drown_clock: 0.0,
            last_tick: now,
            latency: Vec::new(),
            latency_probe: None,
            messages: 0,
        }
    }

    pub fn send(&mut self, m: ClientMsg) {
        match &m {
            ClientMsg::Talk { .. } if self.latency_probe.is_none() => {
                self.latency_probe = Some((Instant::now(), Probe::Talk))
            }
            ClientMsg::AutoAttack { target: Some(t) }
                if self.latency_probe.is_none() && self.attacking != Some(*t) =>
            {
                self.latency_probe = Some((Instant::now(), Probe::Attack))
            }
            _ => {}
        }
        if self.tx.send(m).is_err() && self.closed.is_none() {
            self.closed = Some("connection closed".into());
        }
    }

    /// Expect the server to move us within `secs` (not a rubber-band).
    pub fn expect_teleport(&mut self, secs: f32) {
        self.teleport_ok_until = Instant::now() + Duration::from_secs_f32(secs);
    }

    pub fn say(&mut self, text: &str) {
        self.send(ClientMsg::Chat {
            text: text.chars().take(proto::MAX_CHAT).collect(),
        });
    }

    fn ev(&mut self, e: Ev) {
        self.events.push((Instant::now(), e));
    }

    /// Seconds since this body joined.
    pub fn secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    // ---------------- where things are ----------------

    /// A hearth-relative content position as a world position (feet level
    /// on the ground, under any tree canopy).
    /// Land that has not streamed in yet is taken to be at our own height
    /// until it arrives.
    pub fn rel(&self, at: [i32; 3]) -> Vec3 {
        let (x, z) = (self.hearth[0] + at[0], self.hearth[2] + at[2]);
        let g = ground(&self.world, x, z);
        let y = if g < 0 && !self.world.known(x, self.world.size[1] as i32 - 1, z) {
            self.player.pos.y
        } else {
            (g + 1) as f32
        };
        Vec3::new(x as f32 + 0.5, y, z as f32 + 0.5)
    }

    pub fn rel2(&self, at: [i32; 2]) -> Vec3 {
        self.rel([at[0], 0, at[1]])
    }

    pub fn pos(&self) -> Vec3 {
        self.player.pos
    }

    pub fn dist(&self, p: Vec3) -> f32 {
        let d = p - self.player.pos;
        (d.x * d.x + d.z * d.z).sqrt()
    }

    pub fn dist3(&self, p: Vec3) -> f32 {
        p.distance(self.player.pos)
    }

    pub fn npc_ent(&self, npc: u16) -> Option<&Ent> {
        self.ents.values().find(|e| e.npc() == Some(npc))
    }

    /// Where an NPC stands: the live entity, else the content position.
    pub fn npc_pos(&self, npc: u16) -> Option<Vec3> {
        self.npc_ent(npc)
            .map(|e| e.pos)
            .or_else(|| self.content.npc(npc).map(|n| self.rel(n.at)))
    }

    pub fn creatures(&self) -> impl Iterator<Item = &Ent> {
        self.ents
            .values()
            .filter(|e| matches!(e.kind, EntityKind::Creature(_)) && e.health > 0.0)
    }

    pub fn in_combat(&self) -> bool {
        self.in_combat_until > Instant::now()
    }

    /// Time of day: 0 sunrise, 0.25 noon, 0.5 sunset, 0.75 midnight.
    pub fn clock(&self) -> f32 {
        (self.time + self.time_at.elapsed().as_secs_f32() / self.day_seconds).fract()
    }

    pub fn is_night(&self) -> bool {
        let t = self.clock();
        t > 0.52 && t < 0.98
    }

    // ---------------- bags ----------------

    pub fn count(&self, item: u16) -> u32 {
        self.pack
            .iter()
            .flatten()
            .filter(|s| s.item == item)
            .map(|s| s.count as u32)
            .sum()
    }

    pub fn free_slots(&self) -> usize {
        self.pack.iter().filter(|s| s.is_none()).count()
    }

    pub fn food_count(&self) -> u32 {
        self.pack
            .iter()
            .flatten()
            .filter(|s| self.content.item(s.item).food > 0)
            .map(|s| s.count as u32)
            .sum()
    }

    /// Items some unfinished quest asks us to collect.
    pub fn wanted_items(&self) -> HashSet<u16> {
        let mut out = HashSet::new();
        for q in &self.quests {
            if let Some(quest) = self.content.quest(q.quest) {
                for o in &quest.objectives {
                    if let Objective::Collect { item, .. } = o {
                        out.insert(*item);
                    }
                }
            }
        }
        // and items of quests we may take later
        for quest in &self.content.quests {
            if self.done.contains(&quest.id) {
                continue;
            }
            for o in &quest.objectives {
                if let Objective::Collect { item, .. } = o {
                    out.insert(*item);
                }
            }
        }
        out
    }

    /// Bag slots holding things a merchant buys that we have no use for.
    pub fn junk_slots(&self) -> Vec<usize> {
        let wanted = self.wanted_items();
        self.pack
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i, s)))
            .filter(|(_, s)| {
                let it = self.content.item(s.item);
                it.value > 0
                    && it.tool.is_none()
                    && it.food == 0
                    && !wanted.contains(&s.item)
                    && (it.gear.is_none() || it.quality == 0 || !self.is_upgrade(s.item))
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ---------------- gear ----------------

    /// How good gear makes this character, for comparing items.
    pub fn gear_score(&self, equip: &[Option<Stack>]) -> f32 {
        let Some(class) = self.class.and_then(|c| self.content.class(c)) else {
            return 0.0;
        };
        let s = dyno_sim::character::sheet(
            &self.content,
            class,
            self.level,
            equip,
            &Default::default(),
        );
        let caster = class.resource == dyno_world::content::Resource::Mana
            && !class
                .weapons
                .contains(&dyno_world::content::WeaponKind::Bow);
        let offence = if caster {
            s.spell_power * 3.0 + s.max_power * 0.05 + s.dps
        } else {
            s.dps * 4.0 + s.attack_power * 0.3
        };
        offence + s.max_health * 0.15 + s.armor * 0.05 + s.crit * 0.5
    }

    /// The place an item would go, if wearing it makes us better.
    pub fn upgrade_place(&self, item: u16) -> Option<usize> {
        let class = self.class.and_then(|c| self.content.class(c))?;
        let view = self.content.item(item);
        let g = view.gear?;
        if dyno_sim::character::cannot_equip(class, self.level, &view).is_some() {
            return None;
        }
        let now = self.gear_score(&self.equip);
        let mut best = None;
        for place in dyno_sim::character::places_for(g.slot) {
            let mut e = self.equip.clone();
            e[place] = Some(Stack::new(item, 1));
            let s = self.gear_score(&e);
            if s > now + 0.01 && best.is_none_or(|(_, b)| s > b) {
                best = Some((place, s));
            }
        }
        best.map(|(p, _)| p)
    }

    pub fn is_upgrade(&self, item: u16) -> bool {
        self.upgrade_place(item).is_some()
    }

    // ---------------- abilities ----------------

    pub fn ready(&self, ability: u16) -> bool {
        let now = Instant::now();
        self.casting.is_none_or(|(_, t)| t <= now)
            && self.gcd_until <= now
            && self.cooldowns.get(&ability).is_none_or(|t| *t <= now)
    }

    pub fn is_casting(&self) -> bool {
        self.casting.is_some_and(|(_, t)| t > Instant::now())
    }

    // ---------------- the network ----------------

    /// Take in everything the server sent. Returns false once disconnected.
    pub fn pump(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(NetEvent::Msg(m)) => {
                    self.messages += 1;
                    self.handle(m);
                }
                // new rules from the pack (after /reload)
                Ok(NetEvent::Content(c)) => self.content = *c,
                // (only when asked for: bots take sections as messages)
                Ok(NetEvent::Sections(list)) => {
                    for (s, d) in list {
                        let _ = self.world.put_prepared(s, d);
                    }
                }
                Ok(NetEvent::Closed(r)) => {
                    self.closed = Some(r.clone());
                    self.ev(Ev::Closed(r));
                    return false;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if self.closed.is_none() {
                        self.closed = Some("disconnected".into());
                        self.ev(Ev::Closed("disconnected".into()));
                    }
                    return false;
                }
            }
        }
        // a kill nobody credited us for
        if let Some((c, l, at, credited)) = self.pending_kill
            && at.elapsed() > Duration::from_millis(400)
        {
            self.pending_kill = None;
            self.ev(if credited {
                Ev::Kill {
                    creature: c,
                    level: l,
                    xp: 0,
                }
            } else {
                Ev::KillNoCredit {
                    creature: c,
                    level: l,
                }
            });
        }
        self.closed.is_none()
    }

    fn probe_done(&mut self, p: Probe) {
        if let Some((t, want)) = self.latency_probe
            && want == p
        {
            self.latency.push(t.elapsed().as_millis() as u32);
            self.latency_probe = None;
        }
    }

    fn handle(&mut self, m: ServerMsg) {
        let now = Instant::now();
        if let Some((t, _)) = self.latency_probe
            && t.elapsed() > Duration::from_secs(5)
        {
            self.latency_probe = None;
        }
        match m {
            ServerMsg::Teleport { pos } => {
                let from = self.player.pos;
                self.pred.teleport(&mut self.player, pos);
                self.fall_from = None;
                self.nav.path.clear();
                self.ev(Ev::Teleport(from, pos.into()));
            }
            // the world streams in around the bot and is let go behind it
            m @ (ServerMsg::Sections(_)
            | ServerMsg::SectionsGone(_)
            | ServerMsg::Block { .. }
            | ServerMsg::Fill { .. }) => {
                net::apply_world(&mut self.world, &m);
            }
            ServerMsg::Shard {
                players, blocks, ..
            } => {
                self.ents.clear();
                self.players = players
                    .iter()
                    .filter(|p| p.id != self.id)
                    .map(|p| (p.id, (p.name.clone(), Vec3::from(p.pose.pos))))
                    .collect();
                for (p, b) in blocks {
                    self.world.set(p[0], p[1], p[2], b);
                }
            }
            ServerMsg::PlayerJoined(p) => {
                if p.id != self.id {
                    self.players.insert(p.id, (p.name, p.pose.pos.into()));
                }
            }
            ServerMsg::PlayerLeft { id } => {
                self.players.remove(&id);
            }
            ServerMsg::Poses(list) => {
                for (id, _, pose) in list {
                    if let Some(p) = self.players.get_mut(&id) {
                        p.1 = pose.pos.into();
                    }
                }
            }
            ServerMsg::Moved {
                seq,
                pos,
                vel,
                ground,
            } => {
                self.pred
                    .ack(&mut self.player, &self.world, seq, pos, vel, ground);
            }
            ServerMsg::Chat { from, text } => self.ev(Ev::Chat { from, text }),
            ServerMsg::System { text } => {
                if text.starts_with("Quest complete:") {
                    self.expect_quest_xp = true;
                }
                self.ev(Ev::System(text));
            }
            ServerMsg::Time { time, day_seconds } => {
                self.time = time;
                self.time_at = now;
                self.day_seconds = day_seconds.max(1.0);
            }
            ServerMsg::Pack(p) => {
                let before: HashMap<u16, u32> = count_items(&self.pack);
                let after: HashMap<u16, u32> = count_items(&p);
                self.pack = p;
                if !self.got_pack {
                    // what we joined with is not news
                    self.got_pack = true;
                    self.seen_items.extend(after.keys());
                    return;
                }
                for (item, n) in &after {
                    let b = before.get(item).copied().unwrap_or(0);
                    if *n > b {
                        let first = self.seen_items.insert(*item);
                        self.ev(Ev::Item {
                            item: *item,
                            count: n - b,
                            first,
                        });
                    }
                }
                for (item, b) in &before {
                    let n = after.get(item).copied().unwrap_or(0);
                    if n < *b {
                        self.ev(Ev::ItemLost {
                            item: *item,
                            count: b - n,
                        });
                    }
                }
            }
            ServerMsg::Status { health, .. } => {
                self.health = health;
                if health > 0.0 && self.dead.is_some() {
                    self.dead = None;
                    self.ev(Ev::Respawned);
                }
            }
            ServerMsg::EntitySpawn { id, kind, pos, .. } => {
                // the body of something we just fought: probably ours to loot
                if matches!(kind, EntityKind::Corpse { loot: true, .. })
                    && !self.ents.contains_key(&id)
                    && self.kill_spots.iter().any(|(p, t)| {
                        p.distance(Vec3::from(pos)) < 2.5 && t.elapsed() < Duration::from_secs(3)
                    })
                {
                    self.my_corpses.push((id, now));
                    self.my_corpses
                        .retain(|(_, t)| t.elapsed() < Duration::from_secs(120));
                }
                let e = self.ents.entry(id).or_insert(Ent {
                    id,
                    kind,
                    pos: pos.into(),
                    level: 0,
                    health: 1.0,
                    max: 1.0,
                    name: None,
                    elite: false,
                    my_hit: None,
                    hurt_at: None,
                    seen: now,
                });
                e.kind = kind;
                e.pos = pos.into();
                e.seen = now;
            }
            ServerMsg::EntityMoves { list, .. } => {
                for (id, pos, _) in list {
                    if let Some(e) = self.ents.get_mut(&id) {
                        e.pos = pos.into();
                        e.seen = now;
                    }
                }
            }
            ServerMsg::EntityGone { id, .. } => {
                if let Some(e) = self.ents.remove(&id)
                    && let Some(c) = e.creature()
                    && e.my_hit
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(12))
                {
                    self.pending_kill = Some((c, e.level, now, false));
                    self.kill_spots.push((e.pos, now));
                    self.kill_spots
                        .retain(|(_, t)| t.elapsed() < Duration::from_secs(5));
                }
                if self.attacking == Some(id) {
                    self.attacking = None;
                }
            }
            ServerMsg::EntityHurt { id } => {
                if let Some(e) = self.ents.get_mut(&id) {
                    e.hurt_at = Some(now);
                }
            }
            ServerMsg::EntityInfo {
                id,
                level,
                health,
                max,
            } => {
                if let Some(e) = self.ents.get_mut(&id) {
                    e.level = level;
                    e.health = health;
                    e.max = max;
                }
            }
            ServerMsg::EntityName { id, name, elite } => {
                if let Some(e) = self.ents.get_mut(&id) {
                    e.name = Some(name);
                    e.elite = elite;
                }
            }
            // (the knockback comes in the server's `Moved`)
            ServerMsg::Hurt { amount, .. } => {
                self.last_hurt = Some(now);
                self.in_combat_until = now + Duration::from_secs(6);
                self.ev(Ev::Hurt(amount));
            }
            ServerMsg::Died { message } => {
                if self.dead.is_none() {
                    self.died_at = Some(now);
                    self.dead = Some(message.clone());
                    self.attacking = None;
                    self.casting = None;
                    self.ev(Ev::Died(message));
                }
            }
            ServerMsg::Character {
                class,
                level,
                xp,
                xp_next,
                money,
                max_health,
                ..
            } => {
                let dm = money as i64 - self.money as i64;
                if dm != 0 && self.got_character {
                    self.ev(Ev::Money(dm));
                }
                self.got_character = true;
                self.class = class;
                self.level = level;
                self.xp = xp;
                self.xp_next = xp_next;
                self.money = money;
                self.max_health = max_health;
            }
            ServerMsg::Gain { xp, money } => {
                if xp > 0 {
                    if self.expect_quest_xp {
                        self.expect_quest_xp = false;
                    } else if let Some((c, l, _, _)) = self.pending_kill.take() {
                        self.ev(Ev::Kill {
                            creature: c,
                            level: l,
                            xp,
                        });
                    }
                }
                self.ev(Ev::Gain { xp, money });
            }
            ServerMsg::LevelUp { player, level } => {
                if player == self.id {
                    self.ev(Ev::LevelUp(level));
                }
            }
            ServerMsg::Quests { log, done } => {
                let done: HashSet<u16> = done.into_iter().collect();
                if !self.got_quests {
                    self.got_quests = true;
                    self.quests = log;
                    self.done = done;
                    return;
                }
                for q in &log {
                    match self.quests.iter().find(|o| o.quest == q.quest) {
                        None => self.ev(Ev::QuestAccepted(q.quest)),
                        Some(o) if o.counts != q.counts => {
                            // a kill objective that moved counts as credit for a kill
                            if let Some(p) = &mut self.pending_kill {
                                p.3 = true;
                            }
                            self.ev(Ev::QuestProgress(q.quest))
                        }
                        _ => {}
                    }
                }
                let fresh: Vec<u16> = done.difference(&self.done).copied().collect();
                for q in fresh {
                    self.ev(Ev::QuestDone(q));
                }
                self.quests = log;
                self.done = done;
            }
            ServerMsg::Dialog { entity, npc } => {
                self.dialog = Some((entity, npc));
                self.probe_done(Probe::Talk);
                self.ev(Ev::Dialog(npc));
            }
            ServerMsg::Power { value, max } => {
                self.power = value;
                self.max_power = max;
            }
            ServerMsg::Casting {
                player,
                ability,
                secs,
            } => {
                if player == self.id {
                    self.casting = ability.map(|a| (a, now + Duration::from_secs_f32(secs)));
                }
            }
            ServerMsg::Cooldowns { list, global } => {
                self.cooldowns = list
                    .into_iter()
                    .map(|(a, s)| (a, now + Duration::from_secs_f32(s)))
                    .collect();
                self.gcd_until = now + Duration::from_secs_f32(global);
            }
            ServerMsg::Equipment(e) => self.equip = e,
            ServerMsg::Attacking(t) => {
                self.attacking = t;
                if t.is_some() {
                    self.probe_done(Probe::Attack);
                }
            }
            ServerMsg::Auras(list) => {
                self.auras = list
                    .into_iter()
                    .map(|(a, s)| (a, now + Duration::from_secs_f32(s)))
                    .collect();
            }
            ServerMsg::Combo { points, target } => self.combo = (points, target),
            ServerMsg::WorldEvent { title, text, .. } => {
                if title.is_empty() {
                    self.world_event = None;
                } else {
                    if self.world_event.as_ref().is_none_or(|(t, _)| *t != title) {
                        self.ev(Ev::WorldEvent(title.clone()));
                    }
                    self.world_event = Some((title, text));
                }
            }
            ServerMsg::Zone { name, levels, safe } => {
                if name != self.zone.0 {
                    self.ev(Ev::Zone {
                        name: name.clone(),
                        safe,
                    });
                }
                self.zone = (name, levels, safe);
            }
            ServerMsg::Skills(list) => {
                self.skills = list.into_iter().map(|(k, v, _)| (k, v)).collect();
            }
            ServerMsg::Damage {
                entity: Some(id),
                amount,
                ..
            } => {
                if let Some(e) = self.ents.get_mut(&id) {
                    e.my_hit = Some(now);
                }
                self.in_combat_until = now + Duration::from_secs(6);
                self.ev(Ev::Dealt { entity: id, amount });
            }
            ServerMsg::Loot { id, money, items } => {
                let n = items.len();
                self.loot_open = if money == 0 && items.is_empty() {
                    None
                } else {
                    Some((id, money, items))
                };
                self.ev(Ev::Loot {
                    id,
                    money,
                    items: n,
                });
            }
            ServerMsg::Rested { pool } => self.rested = pool,
            ServerMsg::Party { members } => {
                self.party = members.into_iter().map(|m| m.name).collect();
            }
            ServerMsg::Invite { from } => {
                self.invite = Some(from.clone());
                self.ev(Ev::Invite(from));
            }
            ServerMsg::Reject { reason } => {
                self.closed = Some(reason.clone());
                self.ev(Ev::Closed(reason));
            }
            _ => {}
        }
    }

    // ---------------- moving ----------------

    /// Walk to within `range` of `to`. Repaths only when the goal moved.
    pub fn go_to(&mut self, to: Vec3, range: f32) {
        self.go_to_h(to, range, 3.0)
    }

    /// Like `go_to`, with any height difference allowed within `height`.
    pub fn go_to_h(&mut self, to: Vec3, range: f32, height: f32) {
        let mut goal = Goal::new(to.into(), range.max(0.4));
        goal.height = height;
        if let Some(g) = &self.nav.goal {
            let moved = Vec3::from(g.to).distance(to);
            if moved < 1.5 && (g.range - goal.range).abs() < 0.1 {
                if self.nav.state == NavState::Arrived && self.dist(to) > range + 1.0 {
                    self.nav.state = NavState::Moving;
                    self.nav.path.clear();
                }
                return;
            }
            // a moving target: keep the path, and only refresh it now and then
            if moved < 4.0
                && self.nav.state == NavState::Moving
                && self
                    .nav
                    .path_at
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
            {
                self.nav.goal = Some(goal);
                return;
            }
        }
        self.nav = Nav {
            goal: Some(goal),
            state: NavState::Moving,
            check_pos: self.player.pos,
            best: f32::MAX,
            ..Default::default()
        };
    }

    pub fn stop(&mut self) {
        self.nav = Nav::default();
        self.input = Input::default();
    }

    pub fn arrived(&self) -> bool {
        self.nav.state == NavState::Arrived
    }

    pub fn face(&mut self, p: Vec3) {
        let d = p - self.player.pos;
        if d.x.abs() + d.z.abs() > 0.01 {
            self.player.yaw = (-d.x).atan2(-d.z);
        }
    }

    fn plan(&mut self) {
        let Some(goal) = self.nav.goal else { return };
        let start = nav::node_at(&self.world, self.player.pos.into());
        let Some(start) = start else {
            self.nav.path.clear();
            return;
        };
        let far = self.dist(goal.to.into()) > 60.0;
        let budget = if far { 120_000 } else { 40_000 };
        self.nav.path_at = Some(Instant::now());
        match nav::find_path(&self.world, start, goal, budget) {
            Some(p) => {
                self.nav.partial = p.partial;
                self.nav.path = p.cells;
                self.nav.idx = 1.min(self.nav.path.len().saturating_sub(1));
            }
            None => {
                self.nav.path.clear();
                let canopy = on_canopy(&self.world, Vec3::from(goal.to));
                let why = if canopy {
                    "no path (target on a tree canopy)"
                } else {
                    "no path"
                };
                self.nav.state = NavState::Failed(why.into());
                self.log_stuck(goal.to, why);
            }
        }
    }

    /// Remember a place we could not get on from (once per spot and goal).
    fn log_stuck(&mut self, goal: [f32; 3], why: &str) {
        let at = self.player.pos;
        let dup = self.stuck_log.iter().rev().take(20).any(|s| {
            s.at.distance(at) < 4.0 && Vec3::from(s.goal).distance(Vec3::from(goal)) < 6.0
        });
        if !dup {
            self.stuck_log.push(Stuck {
                at,
                goal,
                why: why.into(),
            });
        }
    }

    /// Steer along the path; detect being stuck and try to get free.
    fn follow(&mut self) {
        self.input = Input::default();
        if self.nav.state != NavState::Moving {
            return;
        }
        let Some(goal) = self.nav.goal else {
            self.nav.state = NavState::Idle;
            return;
        };
        let here = self.player.pos;
        let to = Vec3::from(goal.to);
        let hd = self.dist(to);
        if hd <= goal.range && (here.y - to.y).abs() <= goal.height + 0.5 && self.player.on_ground
            || hd <= goal.range * 0.8 && (here.y - to.y).abs() <= goal.height
        {
            self.nav.state = NavState::Arrived;
            self.nav.path.clear();
            return;
        }
        let now = Instant::now();
        // stuck checks every second
        let check = *self.nav.check_at.get_or_insert(now);
        if now.duration_since(check) >= Duration::from_secs(1) {
            self.nav.check_at = Some(now);
            let moved = self.nav.check_pos.distance(here);
            let progress = self.nav.best - hd;
            self.nav.check_pos = here;
            if hd < self.nav.best {
                self.nav.best = hd;
            }
            if moved < 0.6 || (progress < 0.2 && self.nav.stalls > 0 && moved < 2.0) {
                self.nav.stalls += 1;
            } else if progress > 1.0 {
                self.nav.stalls = 0;
            }
            match self.nav.stalls {
                0 => {}
                1 | 2 => self.nav.jump_for = 0.4,
                3 => {
                    self.nav.strafe_for = 0.8;
                    self.nav.strafe_dir = if (self.secs() as i64) % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    };
                    self.nav.jump_for = 0.3;
                }
                4 | 6 | 8 => {
                    self.nav.repaths += 1;
                    self.nav.path.clear();
                }
                n if n >= 10 => {
                    self.log_stuck(goal.to, "stuck");
                    self.nav.state = NavState::Failed("stuck".into());
                    return;
                }
                _ => {}
            }
        }
        if self.nav.path.is_empty() || self.nav.idx >= self.nav.path.len() {
            if self.nav.repaths > 12 {
                self.nav.state = NavState::Failed("too many repaths".into());
                self.log_stuck(goal.to, "repaths");
                return;
            }
            self.nav.repaths += 1;
            self.plan();
            if self.nav.path.is_empty() {
                if self.nav.state == NavState::Moving {
                    // nothing to stand on here: walk straight at it
                    self.steer(to, false);
                }
                return;
            }
        }
        // advance along the path
        while self.nav.idx < self.nav.path.len() {
            let c = self.nav.path[self.nav.idx];
            let cp = Vec3::new(c[0] as f32 + 0.5, c[1] as f32, c[2] as f32 + 0.5);
            let d = Vec3::new(cp.x - here.x, 0.0, cp.z - here.z).length();
            let last = self.nav.idx + 1 == self.nav.path.len();
            if d < if last { 0.35 } else { 0.55 } && (here.y - cp.y).abs() < 1.4 {
                self.nav.idx += 1;
            } else {
                break;
            }
        }
        if self.nav.idx >= self.nav.path.len() {
            // end of a partial path (or overshoot): plan again from here
            self.nav.path.clear();
            return;
        }
        let c = self.nav.path[self.nav.idx];
        let cp = Vec3::new(c[0] as f32 + 0.5, c[1] as f32, c[2] as f32 + 0.5);
        let up = cp.y > here.y + 0.6;
        self.steer(cp, up);
    }

    fn steer(&mut self, to: Vec3, up: bool) {
        self.face(to);
        let d = self.dist(to);
        let in_water = self.world.get(
            self.player.pos.x.floor() as i32,
            (self.player.pos.y + 0.4).floor() as i32,
            self.player.pos.z.floor() as i32,
        ) == w::WATER;
        self.input.forward = if d < 0.6 { (d / 0.6).max(0.3) } else { 1.0 };
        self.input.jump = in_water || (up && d < 1.6);
        if self.nav.jump_for > 0.0 {
            self.input.jump = true;
            self.nav.jump_for -= TICK;
        }
        if self.nav.strafe_for > 0.0 {
            self.input.right = self.nav.strafe_dir;
            self.input.forward = -0.3;
            self.nav.strafe_for -= TICK;
        }
    }

    /// One step of the world: physics, the pose to the server, falls and drowning.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_secs_f32().min(0.25);
        self.last_tick = now;
        if self.dead.is_some() {
            self.input = Input::default();
            return;
        }
        self.follow();
        // in deep water, keep the head up (players hold jump), even standing still
        let eye = self.player.eye();
        if self.world.get(
            eye.x.floor() as i32,
            (eye.y - 0.3).floor() as i32,
            eye.z.floor() as i32,
        ) == w::WATER
        {
            self.input.jump = true;
        }
        let before = self.player.pos;
        // steps of the time that passed, predicted here and run by the server
        let protected = self.content.world.protected;
        dyno_sim::movement::apply_rules(&mut self.player, protected, false);
        let yaw = self.player.yaw;
        // in slices of 25 ms at most, like a client at 40 fps
        let mut t = elapsed;
        while t > 1e-4 {
            let dt = t.min(0.025);
            self.pred
                .advance(&mut self.player, &self.world, self.input, yaw, dt);
            t -= dt;
        }
        let moved = self.player.pos - before;
        self.distance += (moved.x * moved.x + moved.z * moved.z).sqrt().min(2.0);
        // falls and drowning are measured by the client and reported
        let p = self.player.pos;
        let in_water = self.world.get(
            p.x.floor() as i32,
            (p.y + 0.3).floor() as i32,
            p.z.floor() as i32,
        ) == w::WATER;
        if self.player.on_ground || in_water {
            if let Some(y0) = self.fall_from.take() {
                let fell = y0 - p.y;
                if fell > 3.5 && !in_water {
                    self.send(ClientMsg::Hurt {
                        kind: EnvDamage::Fall,
                        amount: (fell - 3.0).floor() as u8,
                    });
                }
            }
        } else {
            self.fall_from = Some(self.fall_from.map_or(p.y, |f| f.max(p.y)));
        }
        let eye = self.player.eye();
        let head_wet = self.world.get(
            eye.x.floor() as i32,
            eye.y.floor() as i32,
            eye.z.floor() as i32,
        ) == w::WATER;
        if head_wet {
            self.air -= elapsed;
            if self.air < 0.0 {
                self.drown_clock += elapsed;
                if self.drown_clock >= 1.0 {
                    self.drown_clock = 0.0;
                    self.send(ClientMsg::Hurt {
                        kind: EnvDamage::Drown,
                        amount: 2,
                    });
                }
            }
        } else {
            self.air = (self.air + elapsed * 4.0).min(10.0);
        }
        if let Some(m) = self.pred.flush(self.player.pitch) {
            self.send(m);
        }
    }
}

/// Is something at `pos` standing on a tree canopy (leaves under its feet)?
pub fn on_canopy(world: &World, pos: Vec3) -> bool {
    let b = world.get(
        pos.x.floor() as i32,
        (pos.y - 0.5).floor() as i32,
        pos.z.floor() as i32,
    );
    w::is_leaves(b)
}

/// The top of the ground in a column: the highest solid block that is not
/// part of a tree (leaves and trunks) or a landmark lantern. -1 if none.
pub fn ground(world: &World, x: i32, z: i32) -> i32 {
    let mut y = world.surface(x, z);
    while y > 0 {
        let b = world.get(x, y, z);
        let tree = w::is_leaves(b) || matches!(b, w::LOG | w::BIRCH_LOG | w::LANTERN);
        if !tree && w::is_solid(b) {
            return y;
        }
        y -= 1;
    }
    y
}

fn count_items(p: &[Option<Stack>]) -> HashMap<u16, u32> {
    let mut m = HashMap::new();
    for s in p.iter().flatten() {
        *m.entry(s.item).or_insert(0) += s.count as u32;
    }
    m
}

/// Compass direction from a to b, as N, NE, E... (north is -z).
pub fn compass(from: Vec3, to: Vec3) -> &'static str {
    let (dx, dz) = (to.x - from.x, to.z - from.z);
    if dx.abs() + dz.abs() < 0.5 {
        return "here";
    }
    let a = dx.atan2(-dz).to_degrees();
    let i = (((a + 360.0 + 22.5) % 360.0) / 45.0) as usize;
    ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][i % 8]
}
