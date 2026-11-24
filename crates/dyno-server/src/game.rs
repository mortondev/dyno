//! Game state and rules on the server: players, packs, items in the
//! world, mining, crafting, health and hunger. The network layer calls
//! `handle` for each message and `tick` 20 times per second.

use crate::store::{self, PlayerData};
use dyno_protocol::{
    self as proto, ClientMsg, EntityKind, EnvDamage, Mode, PlayerId, PlayerInfo, Pose, ServerMsg,
    SlotRef,
};
use dyno_sim::crafting;
use dyno_sim::inventory::{Inventory, Stack, transfer_within};
use dyno_sim::rng::Rng;
use dyno_world::content::{Content, ItemId};
use dyno_world::{self as w, World};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const CHEST_SLOTS: usize = 27;
const REACH: f32 = 7.5;
/// Items on the ground, including death piles, last this long.
const ITEM_LIFE: Duration = Duration::from_secs(30 * 60);

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
    pub dead: bool,
    pub regen_clock: f32,
    pub shade_timer: f32,
    pub last_seen: u64,
    /// Shades ignore this player until then (after joining or respawning)
    pub safe_until: Instant,
    pub level: u8,
    pub xp: u32,
    /// copper
    pub money: u32,
    /// banked rested XP
    pub rested: u32,
    /// my party, by name (kept up to date by the server)
    pub party: Vec<String>,
    /// keeps movement honest
    pub moves: crate::movement::MoveCheck,
    /// the phases this player is in, one bit each
    pub phases: u64,
    /// rested XP not yet a whole point
    pub rested_carry: f32,
    pub quests: Vec<proto::QuestProgress>,
    pub done: HashSet<u16>,
    /// no health regeneration until then
    pub combat_until: Instant,
    pub hint_at: Instant,
    pub quest_clock: f32,
    pub class: Option<u16>,
    /// rage or mana
    pub power: f32,
    pub power_clock: f32,
    /// when each ability is ready again
    pub cooldowns: HashMap<u16, Instant>,
    pub gcd_until: Instant,
    /// an ability pressed just before the global cooldown or a cast ended:
    /// it goes off as soon as it can
    pub queued: Option<(u16, Option<u32>)>,
    pub casting: Option<crate::abilities::Casting>,
    /// worn gear, by paper-doll place
    pub equip: Vec<Option<Stack>>,
    pub auto_target: Option<u32>,
    pub swing_at: Instant,
    /// profession skills by key
    pub skills: HashMap<String, u16>,
    pub combo: u8,
    pub combo_on: Option<u32>,
    /// Rogue-style fast global cooldown
    pub combo_class: bool,
    pub auras: Vec<crate::abilities::Aura>,
    /// the named area you stand in
    pub area: Option<usize>,
    /// keys of the named areas you have found (discovery XP, rides)
    pub discovered: std::collections::HashSet<String>,
    /// how the player chose to look
    pub look: proto::Appearance,
    /// the sections of the world this player's client holds (stream.rs)
    pub view: crate::stream::View,
    /// entities this player's client knows (area of interest). A cell, so
    /// `broadcast` can introduce a new entity to those near it at once.
    pub known: std::cell::RefCell<HashSet<u32>>,
    /// players near enough that their poses are sent
    pub near: HashSet<PlayerId>,
    /// where a tool (`dyno shot --as`) asked to look from: the world and
    /// what is near stream round this point instead of the player
    pub look_at: Option<[f32; 3]>,
}

impl Player {
    pub fn away(&self) -> bool {
        self.last_pose_at.elapsed() > Duration::from_secs(3)
    }
    /// Where the world streams round, and what is near is heard from.
    pub fn centre(&self) -> [f32; 3] {
        self.look_at.unwrap_or(self.pose.pos)
    }
    fn eye(&self) -> [f32; 3] {
        [self.pose.pos[0], self.pose.pos[1] + 1.62, self.pose.pos[2]]
    }
    pub(crate) fn send(&self, msg: &ServerMsg) {
        let _ = self.tx.send(Arc::new(proto::encode(msg)));
    }
    pub(crate) fn status(&self) -> ServerMsg {
        ServerMsg::Status {
            health: self.health,
            food: self.food,
            mode: self.mode,
            fuel: self.fuel,
        }
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
    /// set for creatures
    pub mob: Option<crate::creatures::Mob>,
    /// seen only by players in (true) or out of (false) a phase
    pub phase: Option<(u8, bool)>,
}

/// An invisible tool connection (`dyno shot`): it hears everything and
/// streams the world around the point it asks for.
pub struct Observer {
    pub tx: Sender<Arc<Vec<u8>>>,
    pub view: crate::stream::View,
}

/// Every chest's slots by position, shared by all shards.
pub type Chests = Arc<Mutex<HashMap<[i32; 3], Vec<Option<Stack>>>>>;

pub struct State {
    pub world: World,
    pub content: Content,
    pub players: HashMap<PlayerId, Player>,
    /// invisible tool connections: they receive updates but cannot act
    pub observers: HashMap<PlayerId, Observer>,
    /// player IDs are unique across every shard
    pub next_id: Arc<std::sync::atomic::AtomicU32>,
    pub dirty: bool,
    /// every character, shared by all shards
    pub saved: Arc<Mutex<HashMap<String, PlayerData>>>,
    /// chests, shared by all shards so nothing is copied
    pub containers: Chests,
    pub entities: HashMap<u32, Entity>,
    pub next_entity: u32,
    pub rng: Rng,
    /// time of day: 0 sunrise, 0.25 noon, 0.5 sunset, 0.75 midnight
    pub clock: f32,
    /// world ticks run so far (the answer to a ping carries it)
    pub ticks: u32,
    /// day number, starting at 1
    pub day: u32,
    pub ops: Vec<String>,
    /// accept any movement (tests that teleport players about)
    pub trust_movement: bool,
    /// the first player on a new world; may use operator commands
    pub owner: Option<String>,
    pub light: dyno_world::light::Light,
    pub hearths: Vec<[i32; 3]>,
    /// recent notable events, for players who were away: (unix seconds, text)
    pub events: std::collections::VecDeque<(u64, String)>,
    pub webhook: Option<Sender<String>>,
    pub meta_dirty: bool,
    /// the ruined hearth at the world spawn; players cannot mine it
    pub spawn_hearth: [i32; 3],
    /// harvested resource nodes: where, which block, when it grows back (unix seconds)
    pub regrow: Vec<([i32; 3], u8, u64)>,
    /// camp spawn points of every area
    pub spawn_points: Vec<crate::camps::SpawnPoint>,
    pub zone_clock: f32,
    /// nodes resting after gathering (trees, bushes): until (unix seconds)
    /// nodes a player has gathered, and when each is theirs again: a node
    /// rests for the one who took from it, not for everyone (a second
    /// player at the same tree was told "gathered recently" with nothing
    /// to show why)
    pub depleted: HashMap<([i32; 3], PlayerId), u64>,
    /// the Nightfall Incursion, when one is under way
    pub incursion: Option<crate::events::Incursion>,
    /// the phase the next spawned creature belongs to (set by camps)
    pub next_phase: Option<(u8, bool)>,
    /// this shard's number
    pub shard: u32,
    /// open trade windows, both sides
    pub trades: HashMap<PlayerId, crate::trade::Trade>,
    /// who has been asked to trade, by whom, when
    pub trade_asks: HashMap<PlayerId, (PlayerId, Instant)>,
    /// fallen creatures still holding loot
    pub corpses: HashMap<u32, crate::loot::Corpse>,
    pub rested_clock: f32,
    /// weather kind (0 clear, 1 cloudy, 2 rain, 3 fog), strength, seconds left
    pub weather: (u8, f32, f32),
    /// view radius, area of interest and section budgets
    pub stream: crate::stream::StreamConfig,
    /// sections encoded for streaming, with the world's count of changes
    /// to each when it was encoded (stream.rs)
    pub encoded: HashMap<dyno_world::SectionPos, (u32, Vec<u8>)>,
    /// the rounds NPCs walk, by entity (routines.rs)
    pub routines: HashMap<u32, crate::routines::Routine>,
    /// time spent walking them, and over how many ticks
    pub routine_cost: (std::time::Duration, u32),
    /// far tiles encoded for streaming, with the tile's revision then
    pub far_encoded: dyno_world::far::Encoded,
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Day takes 10.8 minutes and night 5: the clock runs faster at night.
const DAY_RATE: f32 = 0.54 / (10.8 * 60.0);
const NIGHT_RATE: f32 = 0.46 / (5.0 * 60.0);

pub fn is_night_at(t: f32) -> bool {
    t > 0.52 && t < 0.98
}

/// Length of one in-game day.
pub const DAY_SECONDS: f32 = 20.0 * 60.0;

/// A player's quest log and finished quests by key, kept across a content reload.
type SavedQuests = (PlayerId, Vec<(String, Vec<u8>)>, Vec<String>);

pub(crate) fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn center(p: [i32; 3]) -> [f32; 3] {
    [p[0] as f32 + 0.5, p[1] as f32 + 0.5, p[2] as f32 + 0.5]
}

impl State {
    pub fn time(&self) -> f32 {
        self.clock
    }
    /// Where new and bedless players appear: beside the spawn hearth.
    pub fn spawn_point(&self) -> [f32; 3] {
        self.world.spawn_near(self.spawn_hearth)
    }
    pub fn is_night(&self) -> bool {
        is_night_at(self.clock)
    }
    fn rate(&self) -> f32 {
        if self.is_night() {
            NIGHT_RATE
        } else {
            DAY_RATE
        }
    }
    /// The time and how fast it moves now (as a day length), for clients.
    pub fn time_msg(&self) -> ServerMsg {
        ServerMsg::Time {
            time: self.clock,
            day_seconds: 1.0 / self.rate(),
        }
    }
    pub fn set_time(&mut self, t: f32) {
        self.clock = t.rem_euclid(1.0);
        let m = self.time_msg();
        self.broadcast(&m, None);
    }

    /// Advance the clock. It stops while nobody is playing.
    fn tick_clock(&mut self, dt: f32) {
        if !self.players.values().any(|p| !p.away()) {
            return;
        }
        let before = self.clock;
        let was_night = self.is_night();
        self.clock = (self.clock + dt * self.rate()).rem_euclid(1.0);
        if was_night != self.is_night() {
            let m = self.time_msg();
            self.broadcast(&m, None);
        }
        if before < 0.47 && self.clock >= 0.47 {
            self.broadcast(
                &ServerMsg::DayEvent {
                    dusk: true,
                    day: self.day,
                },
                None,
            );
            self.broadcast(
                &ServerMsg::System {
                    text: "Dusk is coming. Find light.".into(),
                },
                None,
            );
            self.maybe_start_incursion();
        }
        if self.clock < before {
            self.day += 1;
            self.meta_dirty = true;
            self.broadcast(
                &ServerMsg::DayEvent {
                    dusk: false,
                    day: self.day,
                },
                None,
            );
            self.event(format!("Day {} dawns in the valley.", self.day));
        }
    }

    /// What other players see of someone: name, place, looks and class colour.
    pub fn player_info(&self, id: PlayerId, p: &Player) -> PlayerInfo {
        PlayerInfo {
            id,
            name: p.name.clone(),
            pose: p.pose,
            look: p.look,
            color: p
                .class
                .and_then(|c| self.content.class(c))
                .map(|c| c.color)
                .unwrap_or([0.5, 0.5, 0.6]),
        }
    }

    /// This realm, and the characters a token owns here (character select).
    pub fn characters(&self, token: u64, motd: &str) -> ServerMsg {
        let c = &self.content;
        let classes = c
            .classes
            .iter()
            .map(|k| proto::ClassInfo {
                key: k.key.clone(),
                name: k.name.clone(),
                color: k.color,
                blurb: k.blurb.clone(),
                resource: format!("{:?}", k.resource),
                armor: format!("{:?}", k.armor),
                abilities: c
                    .known_abilities(k.id, 6)
                    .iter()
                    .take(4)
                    .map(|a| a.name.clone())
                    .collect(),
                starter: k.starter.iter().map(|(key, _)| key.clone()).collect(),
            })
            .collect();
        let online: Vec<&Player> = self.players.values().filter(|p| p.token == token).collect();
        let saved = self.saved.lock().unwrap();
        let mut list: Vec<proto::CharacterSummary> = saved
            .iter()
            .filter(|(_, d)| d.token == token && token != 0)
            .map(|(name, d)| {
                let class = d
                    .class
                    .as_ref()
                    .and_then(|k| c.classes.iter().find(|x| &x.key == k));
                let live = online.iter().find(|p| p.name.eq_ignore_ascii_case(name));
                proto::CharacterSummary {
                    name: live.map(|p| p.name.clone()).unwrap_or_else(|| {
                        if d.name.is_empty() {
                            name.to_string()
                        } else {
                            d.name.clone()
                        }
                    }),
                    level: live.map(|p| p.level).unwrap_or(d.level),
                    class: class
                        .map(|k| k.name.clone())
                        .unwrap_or_else(|| "Adventurer".into()),
                    color: class.map(|k| k.color).unwrap_or([0.6, 0.6, 0.6]),
                    zone: if d.zone.is_empty() {
                        "Hearthstead".into()
                    } else {
                        d.zone.clone()
                    },
                    look: d.look,
                }
            })
            .collect();
        list.sort_by(|a, b| b.level.cmp(&a.level).then(a.name.cmp(&b.name)));
        ServerMsg::Characters {
            realm: proto::RealmInfo {
                name: c.name.clone(),
                motd: motd.to_string(),
                version: env!("CARGO_PKG_VERSION").into(),
                online: self.players.len() as u16,
                classes,
            },
            list,
        }
    }

    /// Delete one of a token's characters (not while it is online).
    pub fn delete_character(&mut self, token: u64, name: &str) {
        let key = name.to_lowercase();
        let online = self
            .players
            .values()
            .any(|p| p.name.eq_ignore_ascii_case(name));
        let mut saved = self.saved.lock().unwrap();
        if !online
            && saved
                .get(&key)
                .is_some_and(|d| d.token == token && token != 0)
        {
            saved.remove(&key);
            log(format!("Deleted character {name}"));
        }
    }

    /// Weather drifts: mostly fair, sometimes cloud, rain, or morning fog.
    fn tick_weather(&mut self, dt: f32) {
        self.weather.2 -= dt;
        if self.weather.2 > 0.0 || self.players.is_empty() {
            return;
        }
        let dawn = self.clock > 0.9 || self.clock < 0.12;
        let r = self.rng.f32();
        let kind = if dawn && r < 0.35 {
            3
        } else if r < 0.5 {
            0
        } else if r < 0.75 {
            1
        } else if r < 0.93 {
            2
        } else {
            3
        };
        let strength = 0.5 + self.rng.f32() * 0.5;
        self.weather = (kind, strength, 180.0 + self.rng.f32() * 300.0);
        self.broadcast(&ServerMsg::Weather { kind, strength }, None);
    }

    /// Record a notable event: chat line, Discord, and the away replay.
    pub fn event(&mut self, text: String) {
        self.broadcast(&ServerMsg::System { text: text.clone() }, None);
        if let Some(w) = &self.webhook {
            let _ = w.send(text.clone());
        }
        self.events.push_back((now_secs(), text));
        while self.events.len() > 40 {
            self.events.pop_front();
        }
    }

    /// Send to every player and observer, except one player.
    ///
    /// This is where area of interest and phasing are enforced
    /// (docs/architecture.md §4, docs/design/data-and-streaming.md §4):
    /// a message about an entity goes only to players who know it (near
    /// enough, and in its phase); an action or a spell effect only to
    /// players near it; a block edit only to clients that hold its section.
    /// Observers (tools) hear about every entity and action.
    pub fn broadcast(&self, msg: &ServerMsg, except: Option<PlayerId>) {
        enum To {
            All,
            Knows(u32),
            Meets(u32, [f32; 3]),
            Near(Vec<[f32; 3]>),
            Holds([i32; 3], [i32; 3]),
        }
        let pose_of = |pid: &PlayerId| self.players.get(pid).map(|p| p.pose.pos);
        let to = match msg {
            // states, not moments: a body must not stay down (or a cast bar
            // up) for someone who was near when it began
            ServerMsg::Act {
                actor: proto::Actor::Player(_),
                act: proto::Act::Died | proto::Act::Revived,
            }
            | ServerMsg::Casting { ability: None, .. }
            | ServerMsg::Digging { pos: None, .. } => To::All,
            ServerMsg::EntitySpawn { id, pos, .. } => To::Meets(*id, *pos),
            ServerMsg::EntityHurt { id }
            | ServerMsg::EntityInfo { id, .. }
            | ServerMsg::EntityName { id, .. }
            | ServerMsg::EntityGone { id, .. }
            | ServerMsg::Damage {
                entity: Some(id), ..
            }
            | ServerMsg::Act {
                actor: proto::Actor::Entity(id),
                ..
            } => To::Knows(*id),
            ServerMsg::Act {
                actor: proto::Actor::Player(pid),
                ..
            }
            | ServerMsg::Casting { player: pid, .. }
            | ServerMsg::Digging { player: pid, .. } => match pose_of(pid) {
                Some(at) => To::Near(vec![at]),
                None => To::All,
            },
            ServerMsg::Effect { from, to, .. } => To::Near(vec![*from, *to]),
            ServerMsg::Block { pos, .. } => To::Holds(*pos, *pos),
            ServerMsg::Fill { a, b, .. } => To::Holds(*a, *b),
            ServerMsg::EntityMoves(list) => {
                for (id, p) in &self.players {
                    if Some(*id) == except {
                        continue;
                    }
                    let mine: Vec<(u32, [f32; 3], f32)> = list
                        .iter()
                        .filter(|(eid, ..)| p.known.borrow().contains(eid))
                        .copied()
                        .collect();
                    if !mine.is_empty() {
                        p.send(&ServerMsg::EntityMoves(mine));
                    }
                }
                let bytes = Arc::new(proto::encode(msg));
                for (id, o) in &self.observers {
                    if Some(*id) != except {
                        let _ = o.tx.send(bytes.clone());
                    }
                }
                return;
            }
            _ => To::All,
        };
        let reach = self.stream.aoi + self.stream.aoi_keep;
        let bytes = Arc::new(proto::encode(msg));
        for (id, p) in &self.players {
            if Some(*id) == except {
                continue;
            }
            let send = match &to {
                To::All => true,
                To::Knows(e) => p.known.borrow().contains(e),
                // something new (or changed: a corpse stops glinting): those
                // who know it hear, and those near who may see it meet it now
                To::Meets(e, at) => {
                    p.known.borrow().contains(e)
                        || ((at[0] - p.centre()[0]).hypot(at[2] - p.centre()[2]) <= self.stream.aoi
                            && self.sees(p.phases, *e)
                            && p.known.borrow_mut().insert(*e))
                }
                To::Near(points) => points
                    .iter()
                    .any(|at| (at[0] - p.centre()[0]).hypot(at[2] - p.centre()[2]) <= reach),
                To::Holds(a, b) => p.view.holds_box(*a, *b),
            };
            if send {
                let _ = p.tx.send(bytes.clone());
            }
        }
        for (id, o) in &self.observers {
            if Some(*id) == except {
                continue;
            }
            if let To::Holds(a, b) = &to
                && !o.view.holds_box(*a, *b)
            {
                continue;
            }
            let _ = o.tx.send(bytes.clone());
        }
    }

    /// Whether a player with these phases sees an entity.
    pub fn sees(&self, phases: u64, entity: u32) -> bool {
        self.entities
            .get(&entity)
            .is_none_or(|e| e.phase.is_none_or(|ph| in_phase(phases, ph)))
    }

    /// Whether a player sees an entity (and so may talk to, hit or loot it).
    pub fn player_sees(&self, id: PlayerId, entity: u32) -> bool {
        self.players
            .get(&id)
            .is_some_and(|p| self.sees(p.phases, entity))
    }

    /// The phases a player's story has put them in.
    pub fn phases_of(&self, p: &Player) -> u64 {
        let mut m = 0u64;
        for (i, ph) in self.content.phases.iter().enumerate().take(64) {
            if p.done.contains(&ph.done) {
                m |= 1 << i;
            }
        }
        m
    }

    /// Recompute a player's phases; show and hide what changed for them.
    pub fn refresh_phases(&mut self, id: PlayerId) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
        let new = self.phases_of(p);
        let old = p.phases;
        if new == old {
            return;
        }
        // what left the phase is forgotten now; what entered it is
        // introduced by the interest pass if it is near (stream.rs)
        let gone: Vec<u32> = p
            .known
            .borrow()
            .iter()
            .filter(|eid| {
                self.entities
                    .get(eid)
                    .and_then(|e| e.phase)
                    .is_some_and(|ph| !in_phase(new, ph))
            })
            .copied()
            .collect();
        let p = self.players.get_mut(&id).unwrap();
        p.phases = new;
        for eid in gone {
            p.known.borrow_mut().remove(&eid);
            p.send(&ServerMsg::EntityGone {
                id: eid,
                picked_by: None,
            });
        }
    }

    /// The messages that introduce an entity to a player who has not seen it.
    pub fn entity_intro(&self, id: u32, e: &Entity) -> Vec<ServerMsg> {
        let mut out = vec![ServerMsg::EntitySpawn {
            id,
            kind: e.kind,
            pos: e.pos,
            yaw: e.yaw,
        }];
        if let Some(m) = &e.mob {
            if let Some(n) = &m.name {
                out.push(ServerMsg::EntityName {
                    id,
                    name: n.clone(),
                    elite: true,
                });
            }
            out.push(ServerMsg::EntityInfo {
                id,
                level: m.level,
                health: m.health,
                max: m.max_health,
            });
        }
        // what an NPC is busy with
        if let Some(a) = self.routines.get(&id).and_then(|r| r.hold) {
            out.push(ServerMsg::Act {
                actor: proto::Actor::Entity(id),
                act: proto::Act::Emote(a),
            });
        }
        out
    }

    pub fn tell(&self, id: PlayerId, text: impl Into<String>) {
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::System { text: text.into() });
        }
    }

    pub fn may_op(&self, name: &str) -> bool {
        self.ops.iter().any(|o| o.eq_ignore_ascii_case(name))
            || self
                .owner
                .as_deref()
                .is_some_and(|o| o.eq_ignore_ascii_case(name))
            || (self.ops.is_empty() && self.owner.is_none())
    }

    // ---------------- joining and leaving ----------------

    /// Create a player from saved data, or a new one. Errors are shown to the player.
    pub fn join(
        &mut self,
        name: &str,
        token: u64,
        tx: Sender<Arc<Vec<u8>>>,
        create: Option<proto::NewCharacter>,
    ) -> Result<(PlayerId, Vec<ServerMsg>), String> {
        let key = name.to_lowercase();
        if self
            .players
            .values()
            .any(|p| p.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("{name} is already playing. Choose another name."));
        }
        let stored = self.saved.lock().unwrap().get(&key).cloned();
        let fresh = stored.is_none();
        let data = match stored {
            Some(d) if d.token != token && d.token != 0 => {
                return Err(format!(
                    "The name {name} belongs to another player on this server. Choose another name."
                ));
            }
            Some(d) => d,
            None => PlayerData::new(self.spawn_point()),
        };
        // a new character arrives with the class and looks chosen at creation
        let mut data = data;
        let mut new_class = None;
        if fresh && let Some(c) = &create {
            data.look = c.look;
            new_class = self
                .content
                .classes
                .iter()
                .find(|k| k.key == c.class)
                .map(|k| k.id);
        }
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut pack = store::pack_from_keys(&self.content, &data.pack);
        if fresh && self.content.world.protected {
            data.health = dyno_sim::progression::max_health(1);
            for (k, n) in &self.content.world.starter {
                if let Some(item) = self.content.item_by_key(k) {
                    let _ = pack.add(&self.content, Stack::new(item, *n));
                }
            }
        }
        // time away banks rested XP: in full in a town or camp, a little anywhere else
        let data_rested_before = data.rested;
        let rested_now = if self.rpg() && data.last_seen > 0 {
            let hours = now_secs().saturating_sub(data.last_seen) as f32 / 3600.0;
            let next = dyno_sim::progression::xp_to_next(data.level.max(1));
            let safe = self.in_safe_area(data.pos);
            (data.rested + crate::loot::rested_gain(next, hours, safe))
                .min(crate::loot::rested_cap(next))
        } else {
            data.rested
        };
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
            dead: data.health <= 0.0,
            regen_clock: 0.0,
            shade_timer: 10.0,
            last_seen: data.last_seen,
            safe_until: Instant::now() + Duration::from_secs(10),
            level: data.level.max(1),
            xp: data.xp,
            money: data.money,
            rested: rested_now,
            discovered: data.discovered.iter().cloned().collect(),
            moves: crate::movement::MoveCheck::new(data.pos),
            party: Vec::new(),
            phases: 0,
            rested_carry: 0.0,
            quests: data
                .quests
                .iter()
                .filter_map(|(k, counts)| {
                    let q = self.content.quests.iter().find(|q| &q.key == k)?;
                    let mut counts = counts.clone();
                    counts.resize(q.objectives.len(), 0);
                    Some(proto::QuestProgress {
                        quest: q.id,
                        counts,
                    })
                })
                .collect(),
            done: data
                .done
                .iter()
                .filter_map(|k| {
                    self.content
                        .quests
                        .iter()
                        .find(|q| &q.key == k)
                        .map(|q| q.id)
                })
                .collect(),
            combat_until: Instant::now(),
            hint_at: Instant::now() - Duration::from_secs(60),
            quest_clock: 0.0,
            class: data
                .class
                .as_ref()
                .and_then(|k| self.content.classes.iter().find(|c| &c.key == k))
                .map(|c| c.id),
            power: data.power,
            power_clock: 0.0,
            cooldowns: HashMap::new(),
            gcd_until: Instant::now(),
            queued: None,
            casting: None,
            equip: {
                let mut v = store::pack_from_keys(&self.content, &data.equip).slots;
                v.resize(dyno_world::content::EQUIP_PLACES.len(), None);
                v.truncate(dyno_world::content::EQUIP_PLACES.len());
                v
            },
            auto_target: None,
            swing_at: Instant::now(),
            skills: data.skills.iter().cloned().collect(),
            combo: 0,
            combo_on: None,
            combo_class: data
                .class
                .as_ref()
                .and_then(|k| self.content.classes.iter().find(|c| &c.key == k))
                .is_some_and(|c| c.resource == dyno_world::content::Resource::Energy),
            auras: Vec::new(),
            area: None,
            look: data.look,
            view: Default::default(),
            known: Default::default(),
            near: HashSet::new(),
            look_at: None,
        };
        let others: Vec<PlayerInfo> = self
            .players
            .iter()
            .map(|(i, q)| self.player_info(*i, q))
            .collect();
        let welcome = ServerMsg::Welcome {
            your_id: id,
            server_version: env!("CARGO_PKG_VERSION").into(),
            motd: String::new(),
            size: self.world.size,
            spawn: data.pos,
            players: others,
            mode: p.mode,
            // the server fills in its pack's manifest
            manifest: Default::default(),
            hearth: self.spawn_hearth,
            view: self.stream.view,
        };
        let mut p = p;
        p.phases = self.phases_of(&p);
        // the world and the entities near you follow at the end of the tick
        // (stream_sections, update_interest)
        let mut after = vec![ServerMsg::Pack(p.pack.slots.clone()), p.status()];
        {
            let max = match (self.rpg(), p.class.and_then(|c| self.content.class(c))) {
                (true, Some(class)) => {
                    dyno_sim::character::sheet(
                        &self.content,
                        class,
                        p.level,
                        &p.equip,
                        &Default::default(),
                    )
                    .max_health
                }
                (true, None) => dyno_sim::progression::max_health(p.level),
                _ => 20.0,
            };
            after.push(ServerMsg::Rested { pool: p.rested });
            if p.rested > 0 && rested_now > data_rested_before {
                after.push(ServerMsg::System {
                    text: "You feel rested: kills give double experience for a while.".into(),
                });
            }
            after.push(ServerMsg::Character {
                class: p.class,
                look: p.look,
                level: p.level,
                xp: p.xp,
                xp_next: dyno_sim::progression::xp_to_next(p.level),
                money: p.money,
                max_health: max,
            });
            let mut done: Vec<u16> = p.done.iter().copied().collect();
            done.sort();
            after.push(ServerMsg::Quests {
                log: p.quests.clone(),
                done,
            });
            let mut places: Vec<String> = p.discovered.iter().cloned().collect();
            places.sort();
            after.push(ServerMsg::Discovered(places));
            let max_power = match p
                .class
                .and_then(|c| self.content.class(c))
                .map(|c| c.resource)
            {
                Some(dyno_world::content::Resource::Rage)
                | Some(dyno_world::content::Resource::Energy) => 100.0,
                Some(dyno_world::content::Resource::Mana) => 100.0 + 15.0 * (p.level as f32 - 1.0),
                None => 0.0,
            };
            after.push(ServerMsg::Power {
                value: p.power.min(max_power),
                max: max_power,
            });
            after.push(ServerMsg::Equipment(p.equip.clone()));
            after.push(ServerMsg::Weather {
                kind: self.weather.0,
                strength: self.weather.1,
            });
            let cap = dyno_sim::progression::skill_cap(p.level);
            after.push(ServerMsg::Skills(
                p.skills.iter().map(|(k, v)| (k.clone(), *v, cap)).collect(),
            ));
            if self.rpg() && p.class.is_none() && !self.content.classes.is_empty() {
                after.push(ServerMsg::ChooseClass);
            }
        }
        if p.dead {
            after.push(ServerMsg::Died {
                message: "You died.".into(),
            });
        }
        self.broadcast(&ServerMsg::PlayerJoined(self.player_info(id, &p)), None);
        let empty = self.players.is_empty();
        let last = p.last_seen;
        let missed: Vec<String> = self
            .events
            .iter()
            .filter(|(t, _)| last > 0 && *t > last)
            .map(|(_, e)| e.clone())
            .collect();
        // joins go to chat and Discord, but not to the away replay
        let joined = format!("{name} joined");
        self.broadcast(
            &ServerMsg::System {
                text: joined.clone(),
            },
            None,
        );
        if let Some(w) = &self.webhook {
            let _ = w.send(joined);
        }
        for o in self.players.keys().copied().collect::<Vec<_>>() {
            after.push(ServerMsg::Wielding {
                player: o,
                item: self.wielding(o),
            });
            after.push(ServerMsg::Wearing {
                player: o,
                items: self.wearing(o),
            });
        }
        self.players.insert(id, p);
        self.send_wielding(id);
        if let Some(class) = new_class {
            self.choose_class(id, class);
        }
        if empty && self.is_night() {
            // every session starts at dawn
            self.set_time(0.0);
        }
        if self.owner.is_none() && self.ops.is_empty() {
            self.owner = Some(name.to_string());
            self.meta_dirty = true;
            after.push(ServerMsg::System { text: "You are the first player here, so you own this world: you can use /gamemode, /time, /give and /reload.".into() });
        }
        if !missed.is_empty() {
            after.push(ServerMsg::System {
                text: "While you were away:".into(),
            });
            for e in missed.iter().rev().take(10).rev() {
                after.push(ServerMsg::System {
                    text: format!("  {e}"),
                });
            }
        }
        Ok((id, [vec![welcome], after].concat()))
    }

    pub fn leave(&mut self, id: PlayerId) {
        self.cancel_trade(id, "Trade cancelled.");
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
            name: p.name.clone(),
            look: p.look,
            zone: p
                .area
                .and_then(|a| self.content.areas.get(a))
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            last_seen: now_secs(),
            level: p.level,
            xp: p.xp,
            money: p.money,
            rested: p.rested,
            discovered: {
                let mut d: Vec<String> = p.discovered.iter().cloned().collect();
                d.sort();
                d
            },
            class: p
                .class
                .and_then(|c| self.content.class(c))
                .map(|c| c.key.clone()),
            skills: p.skills.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            equip: store::pack_to_keys(
                &self.content,
                &dyno_sim::inventory::Inventory {
                    slots: p.equip.clone(),
                },
            ),
            power: p.power,
            quests: p
                .quests
                .iter()
                .filter_map(|q| {
                    self.content
                        .quest(q.quest)
                        .map(|x| (x.key.clone(), q.counts.clone()))
                })
                .collect(),
            done: p
                .done
                .iter()
                .filter_map(|q| self.content.quest(*q).map(|x| x.key.clone()))
                .collect(),
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
        self.saved
            .lock()
            .unwrap()
            .insert(p.name.to_lowercase(), data);
    }

    // ---------------- messages ----------------

    /// Handle one message. Err disconnects the player (only for bad input).
    pub fn handle(&mut self, id: PlayerId, msg: ClientMsg) -> Result<(), String> {
        if let Some(o) = self.observers.get_mut(&id) {
            // tools may only say where to look
            if let ClientMsg::ViewAt { pos } = msg
                && pos.iter().all(|v| v.is_finite())
            {
                o.view.center = Some(pos);
            }
            return Ok(());
        }
        if !self.players.contains_key(&id) {
            return Ok(());
        }
        match msg {
            ClientMsg::Pose(pose) => {
                let p = self.players.get_mut(&id).unwrap();
                let ok = pose
                    .pos
                    .iter()
                    .chain([pose.yaw, pose.pitch].iter())
                    .all(|v| v.is_finite())
                    && !p.dead;
                if ok && self.check_pose(id, pose) {
                    let p = self.players.get_mut(&id).unwrap();
                    p.pose = pose;
                    p.last_pose_at = Instant::now();
                }
            }
            ClientMsg::Held { slot } => {
                if slot < 9 {
                    self.players.get_mut(&id).unwrap().held = slot;
                }
            }
            ClientMsg::Dig { pos } => self.dig(id, pos),
            ClientMsg::Place { pos, slot } => self.place(id, pos, slot),
            ClientMsg::SetBlock { pos, block } => {
                if block >= w::BLOCK_COUNT {
                    return Err(format!("unknown block {block}"));
                }
                if self.players[&id].mode == Mode::Build
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
                if p.mode == Mode::Build && (slot as usize) < p.pack.slots.len() {
                    p.pack.slots[slot as usize] = if item == 0 {
                        None
                    } else {
                        Some(Stack::new(item, c.item(item).stack))
                    };
                    p.send(&ServerMsg::Pack(p.pack.slots.clone()));
                }
            }
            ClientMsg::Move { from, to, count } => self.move_items(id, from, to, count),
            ClientMsg::Craft { recipe, times } => self.craft(id, recipe, times.clamp(1, 64)),
            ClientMsg::UseHeld => self.use_held(id),
            ClientMsg::Attack { entity } => self.set_auto_attack(id, Some(entity)),
            ClientMsg::Hurt { kind, amount } => {
                let what = match kind {
                    EnvDamage::Fall => "fell from a high place",
                    EnvDamage::Drown => "drowned",
                };
                // falls and drowning were designed for 20 health; scale them
                let scale = self.max_health(id) / 20.0;
                self.damage(id, amount.min(40) as f32 * scale, [0.0; 3], what);
            }
            ClientMsg::Respawn => self.respawn(id),
            // phasing: what you cannot see, you cannot touch
            ClientMsg::Talk { entity } if !self.player_sees(id, entity) => {}
            ClientMsg::Cast {
                target: Some(t), ..
            } if !self.player_sees(id, t) => {}
            ClientMsg::AutoAttack { target: Some(t) } if !self.player_sees(id, t) => {}
            ClientMsg::Talk { entity } => self.talk(id, entity),
            ClientMsg::Cast { ability, target } => self.cast(id, ability, target),
            ClientMsg::ChooseClass { class } => self.choose_class(id, class),
            ClientMsg::Recall => self.cast(id, proto::RECALL, None),
            ClientMsg::EatBest => self.eat_best(id),
            ClientMsg::Equip { from } => self.equip(id, from),
            ClientMsg::Unequip { place } => self.unequip(id, place),
            ClientMsg::AutoAttack { target } => self.set_auto_attack(id, target),
            ClientMsg::Sell { from } => self.sell(id, from),
            ClientMsg::Loot { id: corpse, all } => self.open_loot(id, corpse, all),
            ClientMsg::LootTake { id: corpse, slot } => self.take_loot(id, corpse, slot),
            ClientMsg::Buy { npc, item, count } => self.buy(id, npc, item, count),
            ClientMsg::Trade(op) => self.trade(id, op),
            ClientMsg::Emote(n) => self.broadcast(
                &ServerMsg::Act {
                    actor: proto::Actor::Player(id),
                    act: proto::Act::Emote(n),
                },
                Some(id),
            ),
            ClientMsg::Gather { pos } => self.gather(id, pos),
            // a tool looking from somewhere else (the world data and what
            // stands in it are no secret: before streaming, every client
            // heard of everything)
            ClientMsg::ViewAt { pos } => {
                if pos.iter().all(|v| v.is_finite())
                    && let Some(p) = self.players.get_mut(&id)
                {
                    p.look_at = Some(pos);
                }
            }
            ClientMsg::Ping { nonce } => {
                if let Some(p) = self.players.get(&id) {
                    p.send(&ServerMsg::Pong {
                        nonce,
                        tick: self.ticks,
                    });
                }
            }
            ClientMsg::AcceptQuest { quest } => self.accept_quest(id, quest),
            ClientMsg::CompleteQuest { quest, choice } => self.complete_quest(id, quest, choice),
            ClientMsg::AbandonQuest { quest } => self.abandon_quest(id, quest),
            // kept in the protocol, no longer acted on: timed digging,
            // throwing, chests and beds
            ClientMsg::DigStart { .. }
            | ClientMsg::Throw { .. }
            | ClientMsg::Interact { .. }
            | ClientMsg::CloseContainer => {}
            ClientMsg::Chat { .. }
            | ClientMsg::Want { .. }
            | ClientMsg::Hello { .. }
            | ClientMsg::Session { .. }
            | ClientMsg::ListCharacters { .. }
            | ClientMsg::DeleteCharacter { .. } => {}
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
        self.set_block_as(pos, block, true);
    }

    /// Change a block; `persist` false for changes that need no world save
    /// (a worked-out ore node and its regrowth live in the meta's regrow list).
    pub fn set_block_as(&mut self, pos: [i32; 3], block: u8, persist: bool) {
        let old = self.world.get(pos[0], pos[1], pos[2]);
        if old == w::HEARTH {
            self.hearths.retain(|h| *h != pos);
        }
        if block == w::HEARTH {
            self.hearths.push(pos);
        }
        if old != block {
            self.world.set(pos[0], pos[1], pos[2], block);
            self.light.changed(&self.world, pos);
        }
        if persist {
            self.dirty = true;
        } else {
            self.meta_dirty = true;
        }
        self.broadcast(&ServerMsg::Block { pos, block }, None);
    }

    /// Break a block: build mode only.
    fn dig(&mut self, id: PlayerId, pos: [i32; 3]) {
        let b = self.world.get(pos[0], pos[1], pos[2]);
        if b == w::AIR
            || b == w::WATER
            || self.players[&id].dead
            || !self.within(id, pos, REACH)
            || !self.may_build(id)
        {
            return self.resync(id, pos);
        }
        if b == w::CHEST {
            self.containers.lock().unwrap().remove(&pos);
        }
        self.set_block(pos, w::AIR);
    }

    /// Place the block item from a pack slot: build mode only.
    fn place(&mut self, id: PlayerId, pos: [i32; 3], slot: u8) {
        if !self.may_build(id) {
            return self.resync(id, pos);
        }
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
        self.set_block(pos, block);
    }

    fn fill(&mut self, id: PlayerId, a: [i32; 3], b: [i32; 3], block: u8) -> Result<(), String> {
        let r = self.fill_inner(id, a, b, block);
        self.hearths = crate::creatures::find_hearths(&self.world);
        r
    }

    fn fill_inner(
        &mut self,
        id: PlayerId,
        a: [i32; 3],
        b: [i32; 3],
        block: u8,
    ) -> Result<(), String> {
        if block >= w::BLOCK_COUNT {
            return Err(format!("unknown block {block}"));
        }
        if self.players[&id].mode != Mode::Build {
            self.tell(id, "Fill works in build mode only.");
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
                        self.containers.lock().unwrap().remove(&[x, y, z]);
                    }
                    self.world.set(x, y, z, block);
                }
            }
        }
        self.dirty = true;
        self.light.changed_box(&self.world, lo, hi);
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

    /// Move items between two slots of the bags.
    fn move_items(&mut self, id: PlayerId, from: SlotRef, to: SlotRef, count: u8) {
        let (SlotRef::Pack(a), SlotRef::Pack(b)) = (from, to) else {
            return self.send_pack(id);
        };
        let (a, b) = (a as usize, b as usize);
        if a >= dyno_sim::SLOTS || b >= dyno_sim::SLOTS || self.players[&id].dead {
            return self.send_pack(id);
        }
        let c = &self.content;
        let p = self.players.get_mut(&id).unwrap();
        transfer_within(c, &mut p.pack.slots, a, b, count);
        self.send_pack(id);
    }

    fn send_pack(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
    }

    fn craft(&mut self, id: PlayerId, recipe: u16, times: u8) {
        let Some(r) = self.content.recipes.get(recipe as usize).cloned() else {
            return;
        };
        let st = crafting::stations_near(&self.content, &self.world, self.players[&id].pose.pos);
        if let Err(e) = self.check_skill(id, &r.skill) {
            return self.tell(id, e);
        }
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
            for _ in 0..made {
                self.skill_up(id, &r.skill);
            }
        }
    }

    pub(crate) fn use_food(&mut self, id: PlayerId) {
        self.use_held(id)
    }

    fn use_held(&mut self, id: PlayerId) {
        let sheet_max = self.max_health(id);
        let c = &self.content;
        let p = self.players.get_mut(&id).unwrap();
        let slot = p.held as usize;
        let Some(s) = p.pack.slots[slot] else { return };
        let food = c.item(s.item).food;
        // food heals, out of combat
        let max = sheet_max;
        if food == 0 || p.dead || p.health >= max || p.combat_until > Instant::now() {
            return;
        }
        p.pack.take(slot, 1);
        p.health = (p.health + max * 0.05 * food as f32).min(max);
        p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        p.send(&p.status());
    }

    pub fn damage(&mut self, id: PlayerId, amount: f32, push: [f32; 3], cause: &str) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if p.mode == Mode::Build || p.dead || amount <= 0.0 {
            return;
        }
        use dyno_world::content::BuffKind;
        let ward: f32 = p
            .auras
            .iter()
            .filter(|a| a.buff.is_some_and(|b| b.kind == BuffKind::Ward))
            .map(|a| a.amount)
            .sum();
        let mut amount = amount * (1.0 - (ward / 100.0).min(0.9));
        // a shield soaks up the blow first, and breaks when it is spent
        let mut auras_changed = crate::abilities::drop_stealth(&mut p.auras);
        let mut soaked = 0.0;
        for a in p.auras.iter_mut() {
            if a.buff.is_some_and(|b| b.kind == BuffKind::Absorb) && amount > 0.0 {
                let take = a.amount.min(amount);
                a.amount -= take;
                amount -= take;
                soaked += take;
            }
        }
        if soaked > 0.0 {
            let n = p.auras.len();
            p.auras
                .retain(|a| a.buff.is_none_or(|b| b.kind != BuffKind::Absorb) || a.amount > 0.01);
            auras_changed |= p.auras.len() != n;
            p.send(&ServerMsg::System {
                text: format!("Your shield absorbs {soaked:.0}."),
            });
            p.send(&ServerMsg::Shield {
                left: crate::abilities::shield_left(&p.auras),
            });
            p.combat_until = Instant::now() + Duration::from_secs(6);
        }
        if auras_changed {
            self.send_auras(id);
        }
        if amount <= 0.0 {
            return;
        }
        let act = |a| ServerMsg::Act {
            actor: proto::Actor::Player(id),
            act: a,
        };
        if amount > 0.0 {
            self.broadcast(&act(proto::Act::Hurt), Some(id));
            if self.players[&id].health - amount <= 0.0 {
                self.broadcast(&act(proto::Act::Died), Some(id));
            }
        }
        let p = self.players.get_mut(&id).unwrap();
        p.health = (p.health - amount).max(0.0);
        p.combat_until = Instant::now() + Duration::from_secs(6);
        p.send(&ServerMsg::Hurt { amount, push });
        p.send(&p.status());
        if p.health <= 0.0 {
            p.dead = true;
            // you keep everything you carry
            let name = p.name.clone();
            p.send(&ServerMsg::Died {
                message: format!("You {cause}."),
            });
            self.event(format!("{name} {cause}."));
            return;
        }
        self.gain_rage(id, amount * 0.6);
    }

    fn respawn(&mut self, id: PlayerId) {
        let spawn = self.spawn_point();
        let sheet_max = self.max_health(id);
        let p = self.players.get_mut(&id).unwrap();
        if !p.dead {
            return;
        }
        p.dead = false;
        p.health = (sheet_max * 0.6).round();
        p.safe_until = Instant::now() + Duration::from_secs(10);
        let to = p.spawn.unwrap_or(spawn);
        p.pose.pos = to;
        p.send(&ServerMsg::Teleport { pos: to });
        p.send(&p.status());
        self.broadcast(
            &ServerMsg::Act {
                actor: proto::Actor::Player(id),
                act: proto::Act::Revived,
            },
            Some(id),
        );
    }

    /// What a player holds in their weapon hand, for others to see.
    pub fn wielding(&self, id: PlayerId) -> Option<u16> {
        let p = self.players.get(&id)?;
        let slot = |i: usize| p.equip.get(i).copied().flatten().map(|s| s.item);
        slot(dyno_world::content::MAIN_HAND).or_else(|| slot(dyno_world::content::RANGED))
    }

    /// Everything a player has equipped, for others to see on their figure.
    pub fn wearing(&self, id: PlayerId) -> Vec<u16> {
        self.players
            .get(&id)
            .map(|p| p.equip.iter().flatten().map(|s| s.item).collect())
            .unwrap_or_default()
    }

    pub fn send_wielding(&self, id: PlayerId) {
        let item = self.wielding(id);
        self.broadcast(&ServerMsg::Wielding { player: id, item }, None);
        let items = self.wearing(id);
        self.broadcast(&ServerMsg::Wearing { player: id, items }, None);
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
            mob: None,
            phase: None,
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
        self.ticks = self.ticks.wrapping_add(1);
        self.tick_clock(dt);
        self.tick_casts();
        self.tick_auto_attacks();
        self.tick_auras();
        self.tick_weather(dt);
        self.zone_clock += dt;
        if self.zone_clock >= 1.0 {
            self.zone_clock = 0.0;
            self.tick_camps();
            self.tick_zones();
            self.tick_events();
            self.tick_seekers();
        }
        self.tick_power(dt);
        self.tick_items(dt);
        if self.zone_clock == 0.0 {
            self.tick_trades();
        }
        self.tick_loot(dt);
        self.tick_creatures(dt);
        self.tick_routines(dt);
        self.spawn_creatures(dt);
        self.tick_players(dt);
    }

    fn tick_items(&mut self, dt: f32) {
        // a gentle magnet: items drift to a living player within 2 blocks
        // whose pack has room for them
        let now = Instant::now();
        let c = &self.content;
        let bodies: Vec<([f32; 3], &Inventory)> = self
            .players
            .values()
            .filter(|p| !p.dead)
            .map(|p| ([p.pose.pos[0], p.pose.pos[1] + 0.7, p.pose.pos[2]], &p.pack))
            .collect();
        for e in self.entities.values_mut() {
            let EntityKind::Item(st) = e.kind else {
                continue;
            };
            if e.pickup_after > now {
                continue;
            }
            let one = Stack { count: 1, ..st };
            if let Some((b, _)) = bodies
                .iter()
                .filter(|(b, pack)| dist(*b, e.pos) < 2.0 && (*pack).clone().add(c, one).is_none())
                .min_by(|a, b| dist(a.0, e.pos).total_cmp(&dist(b.0, e.pos)))
            {
                let d = dist(*b, e.pos).max(0.1);
                for (v, (bk, pk)) in e.vel.iter_mut().zip(b.iter().zip(e.pos)) {
                    *v += (bk - pk) / d * 30.0 * dt;
                }
                e.vel[1] += 12.0 * dt; // partly cancel gravity so items rise to the player
            }
        }
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
        let rpg = self.rpg();
        let sheets: HashMap<PlayerId, (f32, f32)> = self
            .players
            .keys()
            .filter_map(|i| {
                self.sheet(*i)
                    .map(|s| (*i, (s.max_health, s.stats.spi as f32)))
            })
            .collect();
        let mut quest_checks = Vec::new();
        for (id, p) in self.players.iter_mut() {
            if p.mode == Mode::Build || p.dead || p.away() {
                continue;
            }
            // out of combat, health returns at 2% a second
            p.quest_clock += dt;
            if p.quest_clock >= 1.0 {
                p.quest_clock = 0.0;
                quest_checks.push(*id);
            }
            let (max, spirit) = sheets
                .get(id)
                .copied()
                .unwrap_or((dyno_sim::progression::max_health(p.level), 3.0));
            // (the field is still saved and sent: always full)
            p.food = 20.0;
            p.regen_clock += dt;
            if p.regen_clock >= 2.0 {
                p.regen_clock = 0.0;
                if p.health < max && p.combat_until <= Instant::now() {
                    p.health = (p.health + 1.0 + spirit / 10.0 + max * 0.01).min(max);
                    p.send(&p.status());
                }
            }
        }
        for id in quest_checks {
            if self.update_quests(id) {
                self.send_quests(id);
            }
        }
        if rpg {
            self.tick_regrow();
        }
    }

    /// Swap content (after /reload). Items keep their keys; IDs may change.
    pub fn replace_content(&mut self, new: Content, manifest: proto::Manifest) {
        self.clear_creatures();
        // quests are kept by key
        let saved: Vec<SavedQuests> = self
            .players
            .iter()
            .map(|(id, p)| {
                let log = p
                    .quests
                    .iter()
                    .filter_map(|q| {
                        self.content
                            .quest(q.quest)
                            .map(|x| (x.key.clone(), q.counts.clone()))
                    })
                    .collect();
                let done = p
                    .done
                    .iter()
                    .filter_map(|q| self.content.quest(*q).map(|x| x.key.clone()))
                    .collect();
                (*id, log, done)
            })
            .collect();
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
        for slots in self.containers.lock().unwrap().values_mut() {
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
        // clients fetch the changed files (the rules among them)
        self.broadcast(&ServerMsg::Manifest(manifest), None);
        for p in self.players.values() {
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
        for (id, log, done) in saved {
            let c = &self.content;
            if let Some(p) = self.players.get_mut(&id) {
                p.quests = log
                    .into_iter()
                    .filter_map(|(k, mut counts)| {
                        let q = c.quests.iter().find(|q| q.key == k)?;
                        counts.resize(q.objectives.len(), 0);
                        Some(proto::QuestProgress {
                            quest: q.id,
                            counts,
                        })
                    })
                    .collect();
                p.done = done
                    .iter()
                    .filter_map(|k| c.quests.iter().find(|q| &q.key == k).map(|q| q.id))
                    .collect();
            }
            self.send_quests(id);
            self.send_character(id);
        }
        self.spawn_npcs();
        self.spawn_points =
            crate::camps::plan_spawn_points(&self.world, &self.content, self.spawn_hearth);
    }

    pub fn item_id(&self, key: &str) -> Option<ItemId> {
        self.content.item_by_key(key)
    }
}

/// Whether a phase mask puts you on the visible side of an entity's phase.
pub fn in_phase(mask: u64, (phase, inside): (u8, bool)) -> bool {
    ((mask >> phase) & 1 == 1) == inside
}
