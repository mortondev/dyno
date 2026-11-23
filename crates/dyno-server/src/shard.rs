//! Shards: whole copies of the world that players move between without
//! noticing. A shard holds its own creatures, NPCs and players; characters,
//! chests and parties are shared above it. A player moves at a quiet moment:
//! the old shard lets go of them, the new one takes them, and their client is
//! told who and what is around them now.

use crate::game::{Player, State};
use dyno_protocol::{PlayerId, ServerMsg};
use std::time::Instant;

/// What the balancer needs to know about one player.
#[derive(Clone, Debug)]
pub struct Snap {
    pub id: PlayerId,
    pub name: String,
    pub shard: u32,
    pub area: Option<usize>,
    /// may be moved now: alive, not fighting or casting, nothing hunting them
    pub movable: bool,
    pub level: u8,
    pub class: Option<u16>,
    pub health: f32,
    pub max: f32,
}

impl State {
    /// Whether a player can be moved to another shard without them noticing.
    pub fn movable(&self, id: PlayerId) -> bool {
        let Some(p) = self.players.get(&id) else {
            return false;
        };
        let now = Instant::now();
        if p.dead || p.casting.is_some() || p.combat_until > now || p.sleeping.is_some() {
            return false;
        }
        // nothing is after them
        !self.entities.values().any(|e| {
            e.mob
                .as_ref()
                .is_some_and(|m| m.target == Some(id) || m.threat.contains_key(&id))
        })
    }

    pub fn snapshot(&self) -> Vec<Snap> {
        self.players
            .iter()
            .map(|(id, p)| Snap {
                id: *id,
                name: p.name.clone(),
                shard: self.shard,
                area: p.area,
                movable: self.movable(*id),
                level: p.level,
                class: p.class,
                health: p.health,
                max: self.max_health(*id),
            })
            .collect()
    }

    /// Let go of a player who is moving to another shard. Everything that
    /// pointed at this shard's creatures is cleared.
    pub fn detach(&mut self, id: PlayerId) -> Option<Player> {
        let mut p = self.players.remove(&id)?;
        self.broadcast(&ServerMsg::PlayerLeft { id }, None);
        for e in self.entities.values_mut() {
            if let Some(m) = e.mob.as_mut() {
                m.threat.remove(&id);
                if m.target == Some(id) {
                    m.target = None;
                }
            }
        }
        p.auto_target = None;
        p.casting = None;
        p.container = None;
        p.combo_on = None;
        Some(p)
    }

    /// Take in a player from another shard. `blocks` are the blocks that
    /// differ from the world they left.
    pub fn attach(&mut self, id: PlayerId, mut p: Player, blocks: Vec<([i32; 3], u8)>) {
        p.phases = self.phases_of(&p);
        let players = self
            .players
            .iter()
            .map(|(i, q)| self.player_info(*i, q))
            .collect();
        p.send(&ServerMsg::Shard {
            shard: self.shard,
            players,
            blocks,
        });
        for (eid, e) in &self.entities {
            if self.sees(p.phases, *eid) {
                for m in self.entity_intro(*eid, e) {
                    p.send(&m);
                }
            }
        }
        p.send(&self.time_msg());
        p.send(&ServerMsg::Weather {
            kind: self.weather.0,
            strength: self.weather.1,
        });
        // the zone will be re-announced as they move
        p.area = None;
        self.broadcast(&ServerMsg::PlayerJoined(self.player_info(id, &p)), None);
        self.players.insert(id, p);
    }
}

/// What a new shard copies from the main one: shared stores, rules and time.
pub struct ForkSeed {
    content: dyno_world::content::Content,
    next_id: std::sync::Arc<std::sync::atomic::AtomicU32>,
    saved: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, crate::store::PlayerData>>,
    >,
    containers: crate::game::Chests,
    clock: f32,
    day: u32,
    default_mode: dyno_protocol::Mode,
    ops: Vec<String>,
    owner: Option<String>,
    webhook: Option<std::sync::mpsc::Sender<String>>,
    spawn_hearth: [i32; 3],
    weather: (u8, f32, f32),
    seed: u64,
}

impl State {
    pub fn fork_seed(&self) -> ForkSeed {
        ForkSeed {
            content: self.content.clone(),
            next_id: self.next_id.clone(),
            saved: self.saved.clone(),
            containers: self.containers.clone(),
            clock: self.clock,
            day: self.day,
            default_mode: self.default_mode,
            ops: self.ops.clone(),
            owner: self.owner.clone(),
            webhook: self.webhook.clone(),
            spawn_hearth: self.spawn_hearth,
            weather: self.weather,
            seed: self.next_entity as u64 ^ 0x5eed_0000,
        }
    }

    /// A new shard: a fresh copy of `world`, with its own creatures and NPCs.
    pub fn fork(seed: ForkSeed, shard: u32, world: dyno_world::World) -> State {
        let light = dyno_world::light::Light::new(&world);
        let hearths = crate::creatures::find_hearths(&world);
        let mut st = State {
            world,
            content: seed.content,
            players: Default::default(),
            observers: Default::default(),
            next_id: seed.next_id,
            dirty: false,
            saved: seed.saved,
            containers: seed.containers,
            entities: Default::default(),
            next_entity: 1,
            rng: dyno_sim::rng::Rng::new(seed.seed ^ (shard as u64) << 32),
            clock: seed.clock,
            day: seed.day,
            default_mode: seed.default_mode,
            ops: seed.ops,
            owner: seed.owner,
            light,
            hearths,
            events: Default::default(),
            webhook: seed.webhook,
            meta_dirty: false,
            spawn_hearth: seed.spawn_hearth,
            regrow: Vec::new(),
            spawn_points: Vec::new(),
            zone_clock: 0.0,
            depleted: Default::default(),
            incursion: None,
            next_phase: None,
            shard,
            corpses: Default::default(),
            rested_clock: 0.0,
            weather: seed.weather,
        };
        st.spawn_points = crate::camps::plan_spawn_points(&st.world, &st.content, st.spawn_hearth);
        st.spawn_npcs();
        st
    }
}
