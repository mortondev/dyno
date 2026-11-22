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
}

const HALF_W: f32 = 0.3;
const GRAVITY: f32 = 25.0;
const AGGRO: f32 = 24.0;
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
    let mut out = Vec::new();
    let [sx, sy, sz] = world.size;
    for x in 0..sx as i32 {
        for z in 0..sz as i32 {
            for y in 0..sy as i32 {
                if world.get(x, y, z) == w::HEARTH {
                    out.push([x, y, z]);
                }
            }
        }
    }
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
        let Some(c) = self.content.creature(creature) else {
            return 0;
        };
        let health = c.health;
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
                mob: Some(Mob {
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
                }),
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
            let y = self.world.surface(x, z) + 1;
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
                            if dark && !daylit && !crowded && !in_sanctuary(&self.hearths, f) {
                                self.spawn_creature(cid, f);
                                break;
                            }
                        }
                    }
                    Spawn::Meadow if grazer_due && !night => {
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
        let mut gone: Vec<(u32, bool)> = Vec::new();
        let mut hurt: Vec<u32> = Vec::new();
        for id in ids {
            let e = self.entities.get_mut(&id).unwrap();
            let mob = e.mob.as_mut().unwrap();
            let Some(c) = self.content.creature(mob.creature) else {
                gone.push((id, false));
                continue;
            };
            // far from everyone: despawn
            if active.iter().all(|(_, p)| hdist(*p, e.pos) > DESPAWN) {
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
            mob.attack_cd -= dt;
            mob.stun -= dt;
            mob.think -= dt;
            mob.burn_flash += dt;
            if c.burns_in_daylight {
                if !night && sky >= 12 {
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
            if c.hostile {
                // keep a target that went away only if close; never pick a new away player
                let keep = mob
                    .target
                    .and_then(|t| prey.iter().find(|(p, _, _)| *p == t))
                    .filter(|(_, p, away)| {
                        let d = hdist(*p, e.pos);
                        d < AGGRO * 1.5 && (!away || d < 16.0)
                    });
                let target = keep.copied().or_else(|| {
                    prey.iter()
                        .filter(|(_, p, away)| {
                            !away && hdist(*p, e.pos) < AGGRO && (p[1] - e.pos[1]).abs() < 12.0
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
                    if d < 1.7 && (p[1] - e.pos[1]).abs() < 2.0 && mob.attack_cd <= 0.0 {
                        mob.attack_cd = 1.5;
                        attacks.push((pid, c.damage, [dx / d * 6.0, 4.5, dz / d * 6.0], c.id));
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
            if hdist(before, e.pos) > 0.001 || (before[1] - e.pos[1]).abs() > 0.001 {
                e.moved = true;
            }
        }
        for id in hurt {
            self.broadcast(&ServerMsg::EntityHurt { id }, None);
        }
        for (id, drops) in gone {
            self.kill_mob(id, drops);
        }
        for (pid, dmg, push, cid) in attacks {
            let name = self
                .content
                .creature(cid)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            self.damage(pid, dmg, push, &format!("fell to a {name}"));
        }
    }

    /// Remove a creature; with `drops`, it leaves its drops and counts as a kill.
    fn kill_mob(&mut self, id: u32, drops: bool) {
        let Some(e) = self.entities.get(&id) else {
            return;
        };
        let (pos, mob) = (e.pos, e.mob.clone());
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
        for d in list {
            if self.rng.f32() < d.chance {
                let n = self.rng.range(d.min as u32, d.max as u32) as u8;
                if n > 0 {
                    self.spawn_item([pos[0], pos[1] + 0.6, pos[2]], Stack::new(d.item, n), 0.2);
                }
            }
        }
        if let Some((pid, at)) = mob.last_hit
            && at.elapsed() < Duration::from_secs(10)
        {
            self.milestone(pid, &format!("kill:{key}"));
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
        let tool = p.held_tool(&self.content).cloned();
        let held = p.held as usize;
        let creative = p.mode == Mode::Creative;
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
            tool.as_ref().map(|t| t.damage).unwrap_or(1.0)
        };
        mob.health -= damage;
        mob.last_hit = Some((pid, Instant::now()));
        mob.flee = 5.0;
        mob.flee_from = eye;
        mob.target = Some(pid);
        let dx = e.pos[0] - eye[0];
        let dz = e.pos[2] - eye[2];
        let hd = (dx * dx + dz * dz).sqrt().max(0.001);
        e.vel = [dx / hd * 7.0, 5.0, dz / hd * 7.0];
        mob.stun = 0.35;
        e.moved = true;
        let dead = mob.health <= 0.0;
        self.broadcast(&ServerMsg::EntityHurt { id }, None);
        let p = self.players.get_mut(&pid).unwrap();
        p.last_attack = Instant::now();
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
        if dead {
            self.kill_mob(id, true);
        }
    }
}

/// Does this content define any creature that spawns in the dark?
pub fn has_shades(c: &Content) -> bool {
    c.creatures.iter().any(|c| c.spawn == Spawn::Dark)
}
