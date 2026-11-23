//! Creatures: Grazers by day, Shades in the dark.
//!
//! Rules (docs/design/survival.md, "Night rules"):
//! - Shades spawn 18 to 32 blocks from an active player, where block light is
//!   below 4, at night on the surface or at any time underground. Never inside
//!   a hearth's reach. At most one every 20 seconds per player.
//! - Hearths are sanctuary: Shades do not enter their radius.
//! - Block light 10 or more halves a Shade's speed and burns it.
//! - Daylight destroys Shades that stand under the open sky.
//! - Away players are not chosen as new targets.

use crate::game::{Entity, State};
use dyno_protocol::PlayerId;
use dyno_protocol::{EntityKind, Mode, ServerMsg};
use dyno_sim::inventory::Stack;
use dyno_world::content::{Content, Spawn};
use dyno_world::{self as w, World};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Mob {
    pub creature: u16,
    pub health: f32,
    pub target: Option<PlayerId>,
    /// wandering direction (x, z), zero when idle
    pub wander: [f32; 2],
    pub think: f32,
    pub attack_cd: f32,
    /// seconds left running away from `flee_from`
    pub flee: f32,
    pub flee_from: [f32; 3],
    /// seconds left during which knockback owns the velocity
    pub stun: f32,
    pub on_ground: bool,
    /// time since the last "burning" flash
    pub burn_flash: f32,
    /// last player who hit it, for drops and milestones
    pub last_hit: Option<(PlayerId, Instant)>,
    /// who hit it first: it is theirs (and their party's) to kill and loot
    pub tag: Option<(PlayerId, Instant)>,
    pub level: u8,
    pub max_health: f32,
    pub stunned_until: Instant,
    pub rooted_until: Instant,
    /// slowed until then, by this share of its speed
    pub slowed_until: Instant,
    pub slow: f32,
    /// someone has hit it since it last stood at peace
    pub engaged: bool,
    /// caught unawares: until then, openers still count as a first blow
    pub surprised_until: Instant,
    pub dots: Vec<crate::abilities::Dot>,
    /// where it was spawned; pulled too far away it returns here
    pub home: [f32; 3],
    pub evading: bool,
    /// damage taken from each player: the highest is its target
    pub threat: std::collections::HashMap<PlayerId, f32>,
    /// health and damage multiplier (rare elites)
    pub power: f32,
    /// a rare elite's own name
    pub name: Option<String>,
    /// belongs to a camp: never despawns, the camp respawns it
    pub camp: bool,
    /// special move timers: charging until, ready again at; has it fled yet
    pub charge_until: Instant,
    pub ability_ready: Instant,
    pub fled: bool,
    /// part of a world event (marches on its home, never leashes)
    pub event: bool,
    /// seconds it has wanted to move and could not (a wall, a ledge, a tree)
    pub stuck: f32,
}

/// An evading creature that cannot walk home gives up and appears there.
pub const EVADE_STUCK: f32 = 3.0;
/// A creature that cannot get at its target this long gives up the fight.
pub const CHASE_STUCK: f32 = 6.0;

/// A creature hit while at peace is off guard this long: an opener that
/// lands just after your first swing still counts.
pub const SURPRISE: Duration = Duration::from_millis(1500);

/// How long a tag lasts after its owner's last blow.
pub const TAG_LIFE: Duration = Duration::from_secs(15);

impl Mob {
    /// A creature at `pos` with full `health`.
    pub fn new(creature: u16, health: f32, level: u8, pos: [f32; 3]) -> Mob {
        let now = Instant::now();
        Mob {
            creature,
            health,
            target: None,
            wander: [0.0; 2],
            think: 0.0,
            attack_cd: 1.0,
            flee: 0.0,
            flee_from: pos,
            stun: 0.0,
            on_ground: false,
            burn_flash: 0.0,
            last_hit: None,
            tag: None,
            level,
            max_health: health,
            stunned_until: now,
            rooted_until: now,
            slowed_until: now,
            slow: 0.0,
            engaged: false,
            surprised_until: now,
            dots: Vec::new(),
            home: pos,
            evading: false,
            threat: std::collections::HashMap::new(),
            power: 1.0,
            name: None,
            camp: false,
            charge_until: now,
            ability_ready: now,
            fled: false,
            event: false,
            stuck: 0.0,
        }
    }

    /// Give up the fight: heal, forget everyone, walk home (and take no
    /// damage on the way).
    pub fn evade(&mut self) {
        self.evading = true;
        self.tag = None;
        self.health = self.max_health;
        self.threat.clear();
        self.engaged = false;
        self.target = None;
        self.dots.clear();
        self.stuck = 0.0;
    }

    /// Home again (walked or put back): whole, and free for anyone.
    pub fn arrive_home(&mut self) {
        self.evading = false;
        self.health = self.max_health;
        self.tag = None;
        self.stuck = 0.0;
    }

    /// Count the time it wanted to move but did not get anywhere.
    pub fn note_motion(&mut self, wanted: bool, moved: f32, dt: f32) {
        if wanted && moved < 0.01 {
            self.stuck += dt;
        } else if moved > 0.05 || !wanted {
            self.stuck = 0.0;
        }
    }

    /// Damage from a player. An evading creature takes none (it would heal
    /// anyway); returns false then.
    pub fn take_hit(&mut self, pid: PlayerId, amount: f32) -> bool {
        if self.evading {
            return false;
        }
        if !self.engaged {
            self.engaged = true;
            if self.target.is_none() {
                self.surprised_until = Instant::now() + SURPRISE;
            }
        }
        self.health -= amount;
        self.last_hit = Some((pid, Instant::now()));
        self.claim(pid);
        self.target = Some(pid);
        true
    }

    /// A player hits it: the first to hit owns it, and keeps it while they
    /// keep fighting. A tag nobody renews runs out, so walking away frees it.
    pub fn claim(&mut self, pid: PlayerId) {
        let now = Instant::now();
        match self.tag {
            Some((t, at)) if t != pid && at.elapsed() < TAG_LIFE => {}
            _ => self.tag = Some((pid, now)),
        }
    }
}

/// A ranged attack in flight: target, damage, from, to, colour, creature.
type Bolt = (PlayerId, f32, [f32; 3], [f32; 3], [f32; 3], u16);

const HALF_W: f32 = 0.3;
const GRAVITY: f32 = 25.0;
const AGGRO: f32 = 24.0;
/// RPG: a creature pulled this far from where it spawned gives up.
const LEASH: f32 = 30.0;
const DESPAWN: f32 = 80.0;
const SHADE_INTERVAL: f32 = 20.0;
const GRAZER_INTERVAL: f32 = 12.0;

fn hdist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn cell(p: [f32; 3]) -> [i32; 3] {
    [
        p[0].floor() as i32,
        p[1].floor() as i32,
        p[2].floor() as i32,
    ]
}

/// Every hearth in the world.
pub fn find_hearths(world: &World) -> Vec<[i32; 3]> {
    // sections without a hearth are skipped (a big world is mostly sky and rock)
    let mut out = world.positions_of(w::HEARTH);
    out.sort();
    out
}

/// Inside a hearth's reach: no Shade may spawn or walk here.
pub fn in_sanctuary(hearths: &[[i32; 3]], p: [f32; 3]) -> bool {
    hearths.iter().any(|h| {
        let c = [h[0] as f32 + 0.5, h[1] as f32, h[2] as f32 + 0.5];
        hdist(c, p) < w::HEARTH_RADIUS && (p[1] - c[1]).abs() < 24.0
    })
}

/// Does a creature box at `p` overlap a solid block?
fn blocked(world: &World, p: [f32; 3], height: f32) -> bool {
    let lo = [p[0] - HALF_W, p[1], p[2] - HALF_W];
    let hi = [p[0] + HALF_W, p[1] + height, p[2] + HALF_W];
    for x in lo[0].floor() as i32..=hi[0].floor() as i32 {
        for y in lo[1].floor() as i32..=(hi[1] - 0.01).floor() as i32 {
            for z in lo[2].floor() as i32..=hi[2].floor() as i32 {
                if w::is_solid(world.get(x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Move with gravity and block collisions. Returns (on ground, bumped a wall).
fn physics(
    world: &World,
    pos: &mut [f32; 3],
    vel: &mut [f32; 3],
    dt: f32,
    height: f32,
) -> (bool, bool) {
    vel[1] -= GRAVITY * dt;
    let mut on_ground = false;
    let mut bumped = false;
    for axis in [1, 0, 2] {
        let mut next = *pos;
        next[axis] += vel[axis] * dt;
        if blocked(world, next, height) {
            if axis == 1 {
                if vel[1] < 0.0 {
                    on_ground = true;
                    pos[1] = pos[1].floor().max(next[1].floor() + 1.0).min(pos[1]);
                }
                vel[1] = 0.0;
            } else {
                vel[axis] = 0.0;
                bumped = true;
            }
        } else {
            pos[axis] = next[axis];
        }
    }
    if pos[1] < -20.0 {
        pos[1] = -20.0;
        vel[1] = 0.0;
    }
    (on_ground, bumped)
}

/// A place a creature can stand: solid below, two cells of air (not water).
fn standable(world: &World, x: i32, y: i32, z: i32) -> bool {
    let below = world.get(x, y - 1, z);
    w::is_solid(below)
        && below != w::WATER
        && world.get(x, y, z) == w::AIR
        && world.get(x, y + 1, z) == w::AIR
}

impl State {
    fn active_players(&self) -> Vec<(PlayerId, [f32; 3])> {
        self.players
            .iter()
            .filter(|(_, p)| !p.dead && !p.away())
            .map(|(id, p)| (*id, p.pose.pos))
            .collect()
    }

    fn count_near(&self, creature: u16, at: [f32; 3], r: f32) -> usize {
        self.entities
            .values()
            .filter(|e| e.kind == EntityKind::Creature(creature) && hdist(e.pos, at) < r)
            .count()
    }

    pub fn spawn_creature(&mut self, creature: u16, pos: [f32; 3]) -> u32 {
        // the camp's phase, taken now so it never leaks onto a later spawn
        let phase = self.next_phase.take();
        let Some(c) = self.content.creature(creature) else {
            return 0;
        };
        let levels = c.levels;
        // deeper into the wild (further from the spawn hearth), stronger mobs
        let h = self.spawn_hearth;
        let far = ((pos[0] - h[0] as f32).powi(2) + (pos[2] - h[2] as f32).powi(2)).sqrt();
        let level = if self.rpg() {
            // the area's level band decides (a level 1-2 meadow gets level 1-2
            // Shades); outside any area, deeper into the wild is stronger
            match self.area_levels(pos) {
                Some([lo, hi]) => lo + self.rng.range(0, hi.saturating_sub(lo) as u32) as u8,
                // (a level every 28 m on the 384 m map; in proportion on others)
                None => (1
                    + (far / (28.0 * self.world.size[0] as f32 / 384.0)) as u8
                    + (self.rng.f32() < 0.3) as u8)
                    .clamp(levels[0], levels[1]),
            }
        } else {
            1
        };
        let health = c.health
            * if self.rpg() {
                dyno_sim::progression::mob_scale(level).0
            } else {
                1.0
            };
        let id = self.next_entity;
        self.next_entity += 1;
        let yaw = self.rng.f32() * std::f32::consts::TAU;
        let now = Instant::now();
        self.entities.insert(
            id,
            Entity {
                kind: EntityKind::Creature(creature),
                pos,
                vel: [0.0; 3],
                yaw,
                born: now,
                pickup_after: now,
                moved: false,
                phase,
                mob: Some(Mob::new(creature, health, level, pos)),
            },
        );
        self.broadcast(
            &ServerMsg::EntitySpawn {
                id,
                kind: EntityKind::Creature(creature),
                pos,
                yaw,
            },
            None,
        );
        self.broadcast(
            &ServerMsg::EntityInfo {
                id,
                level,
                health,
                max: health,
            },
            None,
        );
        id
    }

    /// Remove every creature (after a content reload changed their IDs).
    pub fn clear_creatures(&mut self) {
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|(_, e)| e.mob.is_some())
            .map(|(i, _)| *i)
            .collect();
        for id in ids {
            self.entities.remove(&id);
            self.broadcast(
                &ServerMsg::EntityGone {
                    id,
                    picked_by: None,
                },
                None,
            );
        }
    }

    /// Try a spot `dist` blocks from `around`, at the surface or near `around`'s height.
    fn spawn_spot(
        &mut self,
        around: [f32; 3],
        lo: f32,
        hi: f32,
        surface: bool,
    ) -> Option<[i32; 3]> {
        let a = self.rng.f32() * std::f32::consts::TAU;
        let d = lo + self.rng.f32() * (hi - lo);
        let x = (around[0] + a.cos() * d).floor() as i32;
        let z = (around[2] + a.sin() * d).floor() as i32;
        let [sx, _, sz] = self.world.size;
        if x < 1 || z < 1 || x >= sx as i32 - 1 || z >= sz as i32 - 1 {
            return None;
        }
        if surface {
            let y = self.world.ground(x, z) + 1;
            return standable(&self.world, x, y, z).then_some([x, y, z]);
        }
        let y0 = around[1].floor() as i32;
        (-8..=8)
            .map(|dy| y0 + dy)
            .find(|y| standable(&self.world, x, *y, z))
            .map(|y| [x, y, z])
    }

    pub fn spawn_creatures(&mut self, dt: f32) {
        let active = self.active_players();
        if active.is_empty() {
            return;
        }
        let night = self.is_night();
        let has_areas = !self.content.areas.is_empty();
        let creatures: Vec<(u16, Spawn, u8)> = self
            .content
            .creatures
            .iter()
            .map(|c| (c.id, c.spawn, c.cap))
            .collect();
        for (pid, at) in active {
            let survival = self
                .players
                .get(&pid)
                .is_some_and(|p| p.mode == Mode::Survival);
            let (shade_due, grazer_due) = {
                let p = self.players.get_mut(&pid).unwrap();
                p.shade_timer -= dt;
                p.grazer_timer -= dt;
                let s = p.shade_timer <= 0.0;
                let g = p.grazer_timer <= 0.0;
                if s {
                    p.shade_timer = SHADE_INTERVAL;
                }
                if g {
                    p.grazer_timer = GRAZER_INTERVAL;
                }
                (s, g)
            };
            for &(cid, kind, cap) in &creatures {
                match kind {
                    Spawn::Dark if shade_due && survival => {
                        let cap = if self.day <= 1 { cap.min(2) } else { cap } as usize;
                        if self.count_near(cid, at, 40.0) >= cap {
                            continue;
                        }
                        let underground =
                            self.light.sky(at[0] as i32, at[1] as i32 + 1, at[2] as i32) < 4;
                        if !night && !underground {
                            continue;
                        }
                        for _ in 0..8 {
                            let Some(s) = self.spawn_spot(at, 18.0, 32.0, !underground) else {
                                continue;
                            };
                            let f = [s[0] as f32 + 0.5, s[1] as f32, s[2] as f32 + 0.5];
                            let dark = self.light.get(s[0], s[1], s[2]) < 4;
                            let daylit = !night && self.light.sky(s[0], s[1], s[2]) >= 4;
                            let crowded =
                                self.players.values().any(|p| hdist(p.pose.pos, f) < 12.0);
                            if dark
                                && !daylit
                                && !crowded
                                && !in_sanctuary(&self.hearths, f)
                                && !self.in_safe_area(f)
                            {
                                self.spawn_creature(cid, f);
                                break;
                            }
                        }
                    }
                    Spawn::Meadow if grazer_due && !night && !has_areas => {
                        if self.count_near(cid, at, 64.0) >= cap as usize {
                            continue;
                        }
                        for _ in 0..6 {
                            let Some(s) = self.spawn_spot(at, 24.0, 44.0, true) else {
                                continue;
                            };
                            if self.world.get(s[0], s[1] - 1, s[2]) != w::GRASS {
                                continue;
                            }
                            let herd = 1 + self.rng.range(0, 2);
                            for i in 0..herd {
                                let f = [
                                    s[0] as f32 + 0.5 + i as f32 * 0.9,
                                    s[1] as f32,
                                    s[2] as f32 + 0.5,
                                ];
                                self.spawn_creature(cid, f);
                            }
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn tick_creatures(&mut self, dt: f32) {
        let rpg = self.rpg();
        let active = self.active_players();
        if active.is_empty() {
            return; // the world rests while nobody plays
        }
        let night = self.is_night();
        // who may be hunted: alive survival players, not protected
        let now = Instant::now();
        let prey: Vec<(PlayerId, [f32; 3], bool)> = self
            .players
            .iter()
            .filter(|(_, p)| !p.dead && p.mode == Mode::Survival && p.safe_until <= now)
            .map(|(id, p)| (*id, p.pose.pos, p.away()))
            .collect();
        let ids: Vec<u32> = self
            .entities
            .iter()
            .filter(|(_, e)| e.mob.is_some())
            .map(|(i, _)| *i)
            .collect();
        let mut attacks: Vec<(PlayerId, f32, [f32; 3], u16)> = Vec::new();
        let levels: std::collections::HashMap<PlayerId, u8> =
            self.players.iter().map(|(i, p)| (*i, p.level)).collect();
        // the hidden: creatures notice them from less far
        let hidden: std::collections::HashMap<PlayerId, f32> = self
            .players
            .keys()
            .map(|i| (*i, self.buff_amount(*i, dyno_world::content::BuffKind::Stealth)))
            .filter(|(_, s)| *s > 0.0)
            .collect();
        // phasing: a creature only hunts the players who can see it
        let phases: std::collections::HashMap<PlayerId, u64> =
            self.players.iter().map(|(i, p)| (*i, p.phases)).collect();
        let prey_all = prey;
        let mut heal_info: Vec<u32> = Vec::new();
        let mut bolts: Vec<Bolt> = Vec::new();
        let mut gone: Vec<(u32, bool)> = Vec::new();
        // creatures that struck this tick (everyone nearby sees the blow)
        let mut swings: Vec<u32> = Vec::new();
        let mut hurt: Vec<u32> = Vec::new();
        for id in ids {
            let e = self.entities.get_mut(&id).unwrap();
            let prey: Vec<(PlayerId, [f32; 3], bool)> = match e.phase {
                Some(ph) => prey_all
                    .iter()
                    .filter(|(pid, ..)| {
                        crate::game::in_phase(phases.get(pid).copied().unwrap_or(0), ph)
                    })
                    .copied()
                    .collect(),
                None => prey_all.clone(),
            };
            let mob = e.mob.as_mut().unwrap();
            let Some(c) = self.content.creature(mob.creature) else {
                gone.push((id, false));
                continue;
            };
            // far from everyone: despawn
            if !mob.camp && active.iter().all(|(_, p)| hdist(*p, e.pos) > DESPAWN) {
                gone.push((id, false));
                continue;
            }
            let height = if c.hostile { 1.8 } else { 1.1 };
            let feet = cell(e.pos);
            let head = [feet[0], feet[1] + 1, feet[2]];
            let block_light = self
                .light
                .get(head[0], head[1], head[2])
                .max(self.light.get(feet[0], feet[1], feet[2]));
            let sky = self.light.sky(head[0], head[1], head[2]);
            let mut speed = c.speed;
            if mob.slowed_until > now {
                speed *= 1.0 - mob.slow;
            }
            mob.attack_cd -= dt;
            mob.stun -= dt;
            mob.think -= dt;
            mob.burn_flash += dt;
            if c.burns_in_daylight {
                if !night && sky >= 4 {
                    // daylight unmakes it
                    mob.health -= 8.0 * dt;
                    if mob.burn_flash > 0.5 {
                        mob.burn_flash = 0.0;
                        hurt.push(id);
                    }
                }
                if block_light >= 10 {
                    speed *= 0.5;
                    mob.health -= dt;
                    if mob.burn_flash > 1.0 {
                        mob.burn_flash = 0.0;
                        hurt.push(id);
                    }
                }
                if mob.health <= 0.0 {
                    // killed by light: dust still falls if a player fought it or lit it
                    gone.push((id, block_light >= 10 || mob.last_hit.is_some()));
                    continue;
                }
            }
            // decide where to go
            let mut want = [0.0f32; 2];
            // leash: pulled too far from home, it gives up, heals and walks back
            if rpg && mob.camp && !mob.evading && !mob.event && hdist(e.pos, mob.home) > LEASH {
                mob.evade();
                heal_info.push(id);
            }
            // it cannot get at its target (a ledge, a wall, a tree between):
            // it gives up rather than stand there being hit
            if rpg && !mob.evading && !mob.event && mob.target.is_some() && mob.stuck >= CHASE_STUCK
            {
                mob.evade();
                heal_info.push(id);
            }
            // it cannot walk home either: it is put back there
            if mob.evading && mob.stuck >= EVADE_STUCK {
                e.pos = mob.home;
                e.vel = [0.0; 3];
                e.moved = true;
                mob.arrive_home();
                heal_info.push(id);
            }
            if mob.evading {
                let dx = mob.home[0] - e.pos[0];
                let dz = mob.home[2] - e.pos[2];
                let d = (dx * dx + dz * dz).sqrt();
                if d < 2.0 {
                    mob.arrive_home();
                    heal_info.push(id);
                } else {
                    want = [dx / d, dz / d];
                    speed *= 1.6;
                }
            }
            // hostile creatures attack on sight; others only when attacked
            let provoked = !mob.threat.is_empty();
            if !mob.evading && (c.hostile || provoked) {
                // the player who has done the most damage, if still around
                let by_threat = mob
                    .threat
                    .iter()
                    .filter_map(|(pid, t)| prey.iter().find(|(p, _, _)| p == pid).map(|x| (*x, *t)))
                    .filter(|((_, p, _), _)| hdist(*p, e.pos) < LEASH * 1.5)
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(x, _)| x);
                let keep = by_threat.or_else(|| {
                    mob.target
                        .and_then(|t| prey.iter().find(|(p, _, _)| *p == t))
                        .filter(|(_, p, away)| {
                            let d = hdist(*p, e.pos);
                            d < AGGRO * 1.5 && (!away || d < 16.0)
                        })
                        .copied()
                });
                // aggro radius: about 10 blocks, larger for higher-level mobs
                let target = keep.or_else(|| {
                    if !c.hostile {
                        return None;
                    }
                    prey.iter()
                        .filter(|(pid, p, away)| {
                            let lvl = levels.get(pid).copied().unwrap_or(1) as f32;
                            let r = if rpg {
                                ((if c.aggro > 0.0 { c.aggro } else { 10.0 }) + mob.level as f32
                                    - lvl)
                                    .clamp(4.0, 20.0)
                            } else {
                                AGGRO
                            };
                            let r = match hidden.get(pid) {
                                Some(s) => (r * (1.0 - s / 100.0)).max(1.5),
                                None => r,
                            };
                            !away && hdist(*p, e.pos) < r && (p[1] - e.pos[1]).abs() < 12.0
                        })
                        .min_by(|a, b| hdist(a.1, e.pos).total_cmp(&hdist(b.1, e.pos)))
                        .copied()
                });
                mob.target = target.map(|t| t.0);
                if let Some((pid, p, _)) = target {
                    let dx = p[0] - e.pos[0];
                    let dz = p[2] - e.pos[2];
                    let d = (dx * dx + dz * dz).sqrt().max(0.001);
                    if d > 1.1 {
                        want = [dx / d, dz / d];
                    }
                    use dyno_world::content::CreatureAbility as Ab;
                    match c.ability {
                        // rush in from a distance
                        Some(Ab::Charge { range })
                            if d > 4.0 && d < range && mob.ability_ready <= now =>
                        {
                            mob.charge_until = now + Duration::from_millis(1200);
                            mob.ability_ready = now + Duration::from_secs(10);
                        }
                        // stand off and shoot
                        Some(Ab::Bolt { range }) if d < range && d > 2.5 => {
                            want = [0.0, 0.0];
                            e.yaw = (-dx).atan2(-dz);
                            if mob.attack_cd <= 0.0 && mob.stunned_until <= now {
                                mob.attack_cd = c.attack_speed;
                                let dmg = mob.power
                                    * c.damage
                                    * if rpg {
                                        dyno_sim::progression::mob_scale(mob.level).1
                                    } else {
                                        1.0
                                    };
                                bolts.push((
                                    pid,
                                    dmg,
                                    [e.pos[0], e.pos[1] + 1.0, e.pos[2]],
                                    [p[0], p[1] + 1.2, p[2]],
                                    c.color,
                                    c.id,
                                ));
                            }
                        }
                        // run at low health
                        Some(Ab::Flee { below })
                            if !mob.fled && mob.health < mob.max_health * below =>
                        {
                            mob.fled = true;
                            mob.flee = 4.0;
                            mob.flee_from = p;
                        }
                        _ => {}
                    }
                    if mob.flee > 0.0 {
                        mob.flee -= dt;
                        want = [-dx / d, -dz / d];
                    }
                    if mob.charge_until > now {
                        speed *= 3.0;
                    }
                    if d < 1.7
                        && (p[1] - e.pos[1]).abs() < 2.0
                        && mob.attack_cd <= 0.0
                        && mob.stunned_until <= now
                    {
                        mob.attack_cd = c.attack_speed;
                        let dmg = mob.power
                            * c.damage
                            * if rpg {
                                dyno_sim::progression::mob_scale(mob.level).1
                            } else {
                                1.0
                            };
                        let push = if rpg {
                            [0.0; 3]
                        } else {
                            [dx / d * 6.0, 4.5, dz / d * 6.0]
                        };
                        attacks.push((pid, dmg, push, c.id));
                        swings.push(id);
                    }
                }
            } else if mob.flee > 0.0 {
                mob.flee -= dt;
                let dx = e.pos[0] - mob.flee_from[0];
                let dz = e.pos[2] - mob.flee_from[2];
                let d = (dx * dx + dz * dz).sqrt().max(0.001);
                want = [dx / d, dz / d];
                speed *= 2.2;
            }
            if want == [0.0, 0.0] && mob.event && mob.target.is_none() {
                let dx = mob.home[0] - e.pos[0];
                let dz = mob.home[2] - e.pos[2];
                let d = (dx * dx + dz * dz).sqrt();
                if d > 18.0 {
                    want = [dx / d, dz / d];
                    speed *= 0.7;
                }
            }
            if want == [0.0, 0.0] {
                if mob.think <= 0.0 {
                    mob.think = 2.0 + self.rng.f32() * 5.0;
                    mob.wander = if self.rng.f32() < 0.45 {
                        [0.0, 0.0]
                    } else {
                        let a = self.rng.f32() * std::f32::consts::TAU;
                        [a.cos(), a.sin()]
                    };
                }
                want = mob.wander;
                speed *= 0.45;
            }
            // hearths are sanctuary: a Shade stops at the edge
            if c.hostile {
                let ahead = [e.pos[0] + want[0] * 0.8, e.pos[1], e.pos[2] + want[1] * 0.8];
                if in_sanctuary(&self.hearths, ahead) {
                    if in_sanctuary(&self.hearths, e.pos) {
                        // inside (a hearth was built around it): leave
                        let h = self
                            .hearths
                            .iter()
                            .min_by(|a, b| {
                                let da = hdist([a[0] as f32, 0.0, a[2] as f32], e.pos);
                                let db = hdist([b[0] as f32, 0.0, b[2] as f32], e.pos);
                                da.total_cmp(&db)
                            })
                            .copied()
                            .unwrap_or(feet);
                        let dx = e.pos[0] - h[0] as f32 - 0.5;
                        let dz = e.pos[2] - h[2] as f32 - 0.5;
                        let d = (dx * dx + dz * dz).sqrt().max(0.001);
                        want = [dx / d, dz / d];
                        speed = c.speed;
                    } else {
                        want = [0.0, 0.0];
                    }
                }
            } else if want != [0.0, 0.0] {
                // grazers avoid water and cliffs
                let ax = (e.pos[0] + want[0] * 0.9).floor() as i32;
                let az = (e.pos[2] + want[1] * 0.9).floor() as i32;
                let y = feet[1];
                let ground = (1..=3).any(|dy| w::is_solid(self.world.get(ax, y - dy, az)));
                let wet = self.world.get(ax, y, az) == w::WATER
                    || self.world.get(ax, y - 1, az) == w::WATER;
                if !ground || wet {
                    want = [0.0, 0.0];
                    mob.think = 0.0;
                }
            }
            if mob.stunned_until > now || mob.rooted_until > now {
                want = [0.0, 0.0];
            }
            if mob.stun <= 0.0 {
                e.vel[0] = want[0] * speed;
                e.vel[2] = want[1] * speed;
            } else {
                e.vel[0] *= 1.0 - (4.0 * dt).min(1.0);
                e.vel[2] *= 1.0 - (4.0 * dt).min(1.0);
            }
            if want != [0.0, 0.0] {
                e.yaw = (-want[0]).atan2(-want[1]);
            }
            let before = e.pos;
            let (ground, bumped) = physics(&self.world, &mut e.pos, &mut e.vel, dt, height);
            if ground && bumped && want != [0.0, 0.0] {
                e.vel[1] = 7.8; // hop up one block
            }
            // swim up in water
            if self.world.get(feet[0], feet[1], feet[2]) == w::WATER {
                e.vel[1] = e.vel[1].max(2.5);
            }
            mob.on_ground = ground;
            // standing still to shoot, or in reach of its target, is not stuck
            let in_reach = mob
                .target
                .and_then(|t| prey_all.iter().find(|(p, ..)| *p == t))
                .is_some_and(|(_, p, _)| hdist(*p, e.pos) < 1.7 && (p[1] - e.pos[1]).abs() < 2.0);
            mob.note_motion(
                want != [0.0, 0.0] && (mob.target.is_some() || mob.evading) && !in_reach,
                hdist(before, e.pos),
                dt,
            );
            if hdist(before, e.pos) > 0.001 || (before[1] - e.pos[1]).abs() > 0.001 {
                e.moved = true;
            }
        }
        for id in heal_info {
            hurt.push(id);
        }
        for id in hurt {
            self.broadcast(&ServerMsg::EntityHurt { id }, None);
            if let Some(m) = self.entities.get(&id).and_then(|e| e.mob.as_ref()) {
                let info = ServerMsg::EntityInfo {
                    id,
                    level: m.level,
                    health: m.health.max(0.0),
                    max: m.max_health,
                };
                self.broadcast(&info, None);
            }
        }
        for id in swings {
            self.broadcast(
                &ServerMsg::Act {
                    actor: dyno_protocol::Actor::Entity(id),
                    act: dyno_protocol::Act::Swing,
                },
                None,
            );
        }
        for (id, drops) in gone {
            self.kill_mob(id, drops);
        }
        for (pid, dmg, from, to, color, cid) in bolts {
            self.broadcast(
                &ServerMsg::Effect {
                    ability: None,
                    color,
                    from,
                    to,
                    radius: 0.0,
                },
                None,
            );
            attacks.push((pid, dmg, [0.0; 3], cid));
        }
        for (pid, dmg, push, cid) in attacks {
            let name = self
                .content
                .creature(cid)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            // armour turns some of the blow
            let (armor, level) = (
                self.sheet(pid).map(|s| s.armor).unwrap_or(0.0),
                self.players.get(&pid).map(|p| p.level).unwrap_or(1),
            );
            let dmg = dmg * (1.0 - dyno_sim::character::armor_reduction(armor, level));
            if let Some(p) = self.players.get(&pid) {
                p.send(&ServerMsg::Damage {
                    entity: None,
                    amount: dmg,
                    crit: false,
                });
            }
            self.damage(pid, dmg, push, &format!("fell to a {name}"));
        }
    }

    /// A creature killed by a player's ability.
    pub(crate) fn kill_mob_by(&mut self, id: u32) {
        self.kill_mob(id, true);
    }

    /// Remove a creature; with `drops`, it leaves its drops and counts as a kill.
    fn kill_mob(&mut self, id: u32, drops: bool) {
        let Some(e) = self.entities.get(&id) else {
            return;
        };
        let (pos, yaw, mob, phase) = (e.pos, e.yaw, e.mob.clone(), e.phase);
        self.entities.remove(&id);
        self.broadcast(
            &ServerMsg::EntityGone {
                id,
                picked_by: None,
            },
            None,
        );
        let Some(mob) = mob else {
            return;
        };
        if mob.event {
            self.event_kill(id, mob.last_hit.map(|(p, _)| p));
        }
        if !drops {
            return;
        }
        let list = self
            .content
            .creature(mob.creature)
            .map(|c| (c.key.clone(), c.drops.clone()));
        let Some((key, list)) = list else {
            return;
        };
        // the creature belongs to whoever hit it first, and their party
        // a tag counts while its owner keeps fighting and is still nearby
        let killer = mob
            .tag
            .filter(|(t, at)| {
                at.elapsed() < TAG_LIFE
                    && self
                        .players
                        .get(t)
                        .is_some_and(|p| !p.dead && crate::game::dist(p.pose.pos, pos) < 80.0)
            })
            .map(|(t, _)| t)
            .or(mob
                .last_hit
                .filter(|(_, at)| at.elapsed() < Duration::from_secs(10))
                .map(|(p, _)| p));
        let credited = killer.map(|k| self.credited(k, pos)).unwrap_or_default();
        // in a party, quest items drop for each member who needs one (and go
        // straight into their bags); the shared body holds the rest
        let personal: Vec<(PlayerId, Vec<u16>)> = if credited.len() > 1 {
            credited
                .iter()
                .map(|pid| (*pid, self.quest_items_needed(*pid)))
                .collect()
        } else {
            Vec::new()
        };
        let mut items = Vec::new();
        for d in list {
            if personal.iter().any(|(_, need)| need.contains(&d.item)) {
                for (pid, need) in &personal {
                    if need.contains(&d.item) && self.rng.f32() < d.chance {
                        let n = self.rng.range(d.min as u32, d.max as u32).max(1) as u8;
                        self.give_loot(*pid, Stack::new(d.item, n));
                    }
                }
                continue;
            }
            if self.rng.f32() < d.chance {
                let n = self.rng.range(d.min as u32, d.max as u32) as u8;
                if n > 0 {
                    items.push(Stack::new(d.item, n));
                }
            }
        }
        if self.rpg() {
            // RPG: the body stays, holding its loot for whoever killed it
            let money = self.roll_money(mob.creature, mob.level);
            self.spawn_corpse(
                pos,
                yaw,
                mob.creature,
                credited.clone(),
                money,
                items,
                phase,
            );
        } else {
            for st in items {
                self.spawn_item([pos[0], pos[1] + 0.6, pos[2]], st, 0.2);
            }
        }
        // everyone who shares the kill gets quest credit; XP is split, with a group bonus
        let share = self.content.world.kill_share(credited.len());
        for pid in credited {
            self.milestone(pid, &format!("kill:{key}"));
            self.credit_kill(pid, mob.creature, mob.level, share);
        }
    }

    /// A player hits a creature with what they hold.
    pub fn attack(&mut self, pid: PlayerId, id: u32) {
        let Some(p) = self.players.get(&pid) else {
            return;
        };
        if p.dead || p.last_attack.elapsed() < Duration::from_millis(350) {
            return;
        }
        let eye = [p.pose.pos[0], p.pose.pos[1] + 1.62, p.pose.pos[2]];
        let (tool, held) = if self.rpg() {
            match dyno_sim::inventory::best_weapon(&p.pack, &self.content) {
                Some((slot, t)) => (Some(t.clone()), slot),
                None => (None, p.held as usize),
            }
        } else {
            (p.held_tool(&self.content).cloned(), p.held as usize)
        };
        let creative = p.mode == Mode::Creative;
        let power = if self.rpg() {
            2.0 * dyno_sim::progression::power(p.level)
        } else {
            1.0
        };
        let Some(e) = self.entities.get_mut(&id) else {
            return;
        };
        let Some(mob) = e.mob.as_mut() else {
            return;
        };
        let d = ((e.pos[0] - eye[0]).powi(2)
            + (e.pos[1] + 0.9 - eye[1]).powi(2)
            + (e.pos[2] - eye[2]).powi(2))
        .sqrt();
        if d > 4.5 {
            return;
        }
        let damage = if creative {
            1000.0
        } else {
            tool.as_ref().map(|t| t.damage).unwrap_or(1.0) * power
        };
        if !mob.take_hit(pid, damage) {
            return;
        }
        *mob.threat.entry(pid).or_default() += damage;
        let dx = e.pos[0] - eye[0];
        let dz = e.pos[2] - eye[2];
        let hd = (dx * dx + dz * dz).sqrt().max(0.001);
        if !self.content.world.protected {
            e.vel = [dx / hd * 7.0, 5.0, dz / hd * 7.0];
            mob.stun = 0.35;
        }
        e.moved = true;
        let dead = mob.health <= 0.0;
        let (level, health, max) = (mob.level, mob.health.max(0.0), mob.max_health);
        self.broadcast(&ServerMsg::EntityHurt { id }, None);
        self.broadcast(
            &ServerMsg::EntityInfo {
                id,
                level,
                health,
                max,
            },
            None,
        );
        let p = self.players.get_mut(&pid).unwrap();
        p.last_attack = Instant::now();
        p.combat_until = Instant::now() + Duration::from_secs(6);
        if !creative
            && let Some(t) = tool
            && let Some(s) = p.pack.slots[held].as_mut()
        {
            s.wear += if t.kind == dyno_world::content::ToolKind::Sword {
                1
            } else {
                2
            };
            if s.wear >= t.durability {
                p.pack.slots[held] = None;
                p.send(&ServerMsg::System {
                    text: "Your tool broke.".into(),
                });
            }
            p.send(&ServerMsg::Pack(p.pack.slots.clone()));
        }
        self.gain_rage(pid, 8.0);
        if dead {
            self.kill_mob(id, true);
        }
    }
}

/// Does this content define any creature that spawns in the dark?
pub fn has_shades(c: &Content) -> bool {
    c.creatures.iter().any(|c| c.spawn == Spawn::Dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Thornboar stall from the playtest: a creature pulled past its
    /// leash walked home, got stuck on a ledge, and stayed evading forever:
    /// it took hits without dying and never fought back.
    #[test]
    fn an_evading_creature_takes_no_damage_and_is_not_stuck_forever() {
        let mut m = Mob::new(0, 60.0, 4, [10.0, 5.0, 10.0]);
        m.take_hit(1, 20.0);
        assert_eq!(m.health, 40.0);
        m.evade();
        assert_eq!(m.health, 60.0, "evading heals");
        assert!(!m.take_hit(1, 20.0), "no damage while evading");
        assert_eq!(m.health, 60.0);
        assert!(m.tag.is_none() && m.target.is_none());
        // it wants to walk home but a ledge stops it
        for _ in 0..40 {
            m.note_motion(true, 0.0, 0.05);
        }
        assert!(m.stuck >= 2.0 - 1e-3 && m.stuck < EVADE_STUCK);
        for _ in 0..30 {
            m.note_motion(true, 0.0, 0.05);
        }
        assert!(m.stuck >= EVADE_STUCK, "the tick puts it home now");
        m.arrive_home();
        assert!(!m.evading && m.stuck == 0.0 && m.health == m.max_health);
        assert!(m.take_hit(2, 5.0), "and it can be fought again");
    }

    /// Openers (Ambush, Marked Shot) count against a creature at peace, and
    /// for a moment after the first blow, so a swing that lands just before
    /// the opener does not waste it.
    #[test]
    fn a_creature_at_peace_is_caught_off_guard_once() {
        let mut m = Mob::new(0, 60.0, 2, [0.0; 3]);
        assert!(!m.engaged);
        let before = Instant::now();
        m.take_hit(1, 5.0);
        assert!(m.engaged && m.surprised_until > before, "off guard at first");
        let until = m.surprised_until;
        m.take_hit(1, 5.0);
        assert_eq!(m.surprised_until, until, "a second blow does not renew it");
        // one that was already coming for you is not surprised
        let mut m = Mob::new(0, 60.0, 2, [0.0; 3]);
        m.target = Some(7);
        let now = Instant::now();
        m.take_hit(7, 5.0);
        assert!(m.surprised_until <= now);
        // back home and at peace, it can be surprised again
        m.evade();
        m.arrive_home();
        assert!(!m.engaged);
    }

    #[test]
    fn moving_or_standing_still_on_purpose_is_not_stuck() {
        let mut m = Mob::new(0, 40.0, 3, [0.0; 3]);
        for _ in 0..200 {
            m.note_motion(true, 0.2, 0.05); // running along
        }
        assert_eq!(m.stuck, 0.0);
        for _ in 0..200 {
            m.note_motion(false, 0.0, 0.05); // idle, or in reach of its target
        }
        assert_eq!(m.stuck, 0.0);
        for _ in 0..(CHASE_STUCK / 0.05) as usize + 1 {
            m.note_motion(true, 0.0, 0.05); // blocked by a wall
        }
        assert!(m.stuck >= CHASE_STUCK);
    }
}
