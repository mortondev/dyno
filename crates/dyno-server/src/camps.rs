//! Areas and camps: named places with fixed creature spawn points that
//! respawn on a timer, rare named elites, and "you are entering" names.

use crate::game::{State, dist};
use dyno_protocol::{PlayerId, ServerMsg};
use dyno_world::{self as w, World};
use std::time::{Duration, Instant};

/// A camp's spawn points fill when a player is this close.
const WAKE: f32 = 90.0;

#[derive(Clone, Debug)]
pub struct SpawnPoint {
    pub area: usize,
    pub camp: usize,
    pub pos: [f32; 3],
    pub entity: Option<u32>,
    pub ready_at: Instant,
}

/// Where a camp's creatures stand: scattered around the camp centre on open ground.
pub fn plan_spawn_points(
    world: &World,
    c: &dyno_world::content::Content,
    hearth: [i32; 3],
) -> Vec<SpawnPoint> {
    let mut out = Vec::new();
    let now = Instant::now();
    for (ai, a) in c.areas.iter().enumerate() {
        for (ci, camp) in a.camps.iter().enumerate() {
            for k in 0..camp.count as u32 {
                // golden-angle spiral: even, deterministic scatter
                let ang = k as f32 * 2.399_963;
                let r = camp.spread * ((k as f32 + 0.5) / camp.count.max(1) as f32).sqrt();
                let x = hearth[0] + camp.at[0] + (ang.cos() * r).round() as i32;
                let z = hearth[2] + camp.at[1] + (ang.sin() * r).round() as i32;
                if !world.in_bounds([x, 1, z]) {
                    continue;
                }
                let free = |y: i32| {
                    w::is_solid(world.get(x, y - 1, z))
                        && world.get(x, y - 1, z) != w::WATER
                        && !w::is_solid(world.get(x, y, z))
                        && !w::is_solid(world.get(x, y + 1, z))
                };
                let y = match camp.y {
                    // underground: the nearest open floor to the wanted height
                    Some(dy) => {
                        let want = hearth[1] + dy;
                        match (0..12)
                            .flat_map(|d| [want - d, want + d])
                            .find(|y| free(*y))
                        {
                            Some(y) => y,
                            None => continue,
                        }
                    }
                    None => {
                        let y = world.ground(x, z) + 1;
                        if !free(y) {
                            continue;
                        }
                        y
                    }
                };
                out.push(SpawnPoint {
                    area: ai,
                    camp: ci,
                    pos: [x as f32 + 0.5, y as f32, z as f32 + 0.5],
                    entity: None,
                    ready_at: now,
                });
            }
        }
    }
    out
}

impl State {
    /// Fill empty spawn points near players; free points whose creature is gone.
    pub fn tick_camps(&mut self) {
        let now = Instant::now();
        let players: Vec<[f32; 3]> = self
            .players
            .values()
            .filter(|p| !p.dead && !p.away())
            .map(|p| p.pose.pos)
            .collect();
        for i in 0..self.spawn_points.len() {
            let sp = self.spawn_points[i].clone();
            if let Some(e) = sp.entity {
                if !self.entities.contains_key(&e) {
                    // killed (or despawned): wait for the camp's respawn
                    // time, less where players crowd (`world.crowd_respawn`)
                    let respawn = self.content.areas[sp.area].camps[sp.camp].respawn as f32;
                    let crowd = self.content.world.crowd_respawn.map_or(1.0, |c| {
                        c.factor(
                            players
                                .iter()
                                .filter(|p| dist(**p, sp.pos) < c.radius)
                                .count(),
                        )
                    });
                    self.spawn_points[i].entity = None;
                    self.spawn_points[i].ready_at = now + Duration::from_secs_f32(respawn * crowd);
                }
                continue;
            }
            // a tool looking at a camp (`dyno shot`) wakes it as a player
            // there would, so pictures show the world as it is played
            let watched = self.observers.values().any(|o| {
                o.view
                    .center
                    .is_some_and(|c| (c[0] - sp.pos[0]).hypot(c[2] - sp.pos[2]) < WAKE)
            });
            if sp.ready_at > now || !(watched || players.iter().any(|p| dist(*p, sp.pos) < WAKE)) {
                continue;
            }
            // not on top of someone (how near, and for how long a due point
            // waits for them to step away, are rules: at 12 m and for ever,
            // players waiting where an elite stands keep it from returning)
            let rule = self.content.world.crowd_respawn;
            let keep = rule.and_then(|c| c.near).unwrap_or(12.0);
            let waited_out = rule
                .and_then(|c| c.patience)
                .is_some_and(|secs| now.duration_since(sp.ready_at).as_secs_f32() >= secs);
            let area = &self.content.areas[sp.area];
            let camp = &area.camps[sp.camp];
            // (a training dummy is stood back up where it fell, whoever is by)
            let dummy = self
                .content
                .creature(camp.creature)
                .is_some_and(|c| c.anchored);
            if !dummy && !waited_out && players.iter().any(|p| dist(*p, sp.pos) < keep) {
                continue;
            }
            let (creature, levels, rare) = (
                camp.creature,
                camp.levels.unwrap_or(area.levels),
                camp.rare.clone(),
            );
            let level = levels[0]
                + self
                    .rng
                    .range(0, levels[1].saturating_sub(levels[0]) as u32) as u8;
            let rare = rare.filter(|r| self.rng.f32() < r.chance);
            self.next_phase = camp.phase;
            let id = self.spawn_creature(creature, sp.pos);
            if id == 0 {
                continue;
            }
            self.spawn_points[i].entity = Some(id);
            let Some(m) = self.entities.get_mut(&id).and_then(|e| e.mob.as_mut()) else {
                continue;
            };
            m.level = level + rare.as_ref().map_or(0, |r| r.levels);
            let (hs, _) = dyno_sim::progression::mob_scale(m.level);
            let power = rare.as_ref().map_or(1.0, |r| r.power);
            let base = self.content.creature(creature).map_or(10.0, |c| c.health);
            m.max_health = base * hs * power;
            m.health = m.max_health;
            m.power = power;
            m.extra_drops = rare.as_ref().map(|r| r.drops.clone()).unwrap_or_default();
            m.home = sp.pos;
            m.camp = true;
            let info = ServerMsg::EntityInfo {
                id,
                level: m.level,
                health: m.health,
                max: m.max_health,
            };
            self.broadcast(&info, None);
            if let Some(r) = rare {
                if let Some(m) = self.entities.get_mut(&id).and_then(|e| e.mob.as_mut()) {
                    m.name = Some(r.name.clone());
                }
                self.broadcast(
                    &ServerMsg::EntityName {
                        id,
                        name: r.name.clone(),
                        elite: true,
                    },
                    None,
                );
            }
        }
    }

    /// Tell players when they enter or leave a named place.
    pub fn tick_zones(&mut self) {
        let h = self.spawn_hearth;
        let ids: Vec<PlayerId> = self.players.keys().copied().collect();
        for id in ids {
            let pos = self.players[&id].pose.pos;
            // the smallest area that contains you
            let here = self
                .content
                .areas
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    let dx = pos[0] - (h[0] + a.at[0]) as f32;
                    let dz = pos[2] - (h[2] + a.at[1]) as f32;
                    (dx * dx + dz * dz).sqrt() <= a.radius
                })
                .min_by(|x, y| x.1.radius.total_cmp(&y.1.radius))
                .map(|(i, _)| i);
            let p = self.players.get_mut(&id).unwrap();
            if p.area != here {
                p.area = here;
                // the first time here: remember it, and pay for the walk
                // (not for where a new character wakes up)
                let found = here.map(|i| &self.content.areas[i]).and_then(|a| {
                    let first = p.discovered.is_empty();
                    (p.discovered.insert(a.key.clone()) && !first)
                        .then(|| (a.name.clone(), self.content.world.discovery(a.levels[0])))
                });
                let msg = match here.map(|i| &self.content.areas[i]) {
                    Some(a) => ServerMsg::Zone {
                        name: a.name.clone(),
                        levels: a.levels,
                        safe: a.safe,
                    },
                    None => ServerMsg::Zone {
                        name: String::new(),
                        levels: [0, 0],
                        safe: false,
                    },
                };
                p.send(&msg);
                let mut places: Vec<String> = p.discovered.iter().cloned().collect();
                places.sort();
                p.send(&ServerMsg::Discovered(places));
                if let Some((name, xp)) = found.filter(|f| f.1 > 0 && self.rpg()) {
                    self.tell(id, format!("Discovered: {name} (+{xp} XP)"));
                    self.reward(id, xp, 0);
                }
            }
        }
    }

    /// The level band of the smallest area containing this point.
    pub fn area_levels(&self, pos: [f32; 3]) -> Option<[u8; 2]> {
        let h = self.spawn_hearth;
        self.content
            .areas
            .iter()
            .filter(|a| {
                let dx = pos[0] - (h[0] + a.at[0]) as f32;
                let dz = pos[2] - (h[2] + a.at[1]) as f32;
                (dx * dx + dz * dz).sqrt() <= a.radius
            })
            .min_by(|a, b| a.radius.total_cmp(&b.radius))
            .map(|a| a.levels)
    }

    /// Is this point inside an area marked safe?
    /// Inside an area where the dark rises (`Area::dark`)?
    pub fn in_dark_area(&self, pos: [f32; 3]) -> bool {
        let h = self.spawn_hearth;
        self.content.areas.iter().any(|a| {
            let dx = pos[0] - (h[0] + a.at[0]) as f32;
            let dz = pos[2] - (h[2] + a.at[1]) as f32;
            a.dark && (dx * dx + dz * dz).sqrt() <= a.radius
        })
    }

    pub fn in_safe_area(&self, pos: [f32; 3]) -> bool {
        let h = self.spawn_hearth;
        self.content.areas.iter().any(|a| {
            let dx = pos[0] - (h[0] + a.at[0]) as f32;
            let dz = pos[2] - (h[2] + a.at[1]) as f32;
            a.safe && (dx * dx + dz * dz).sqrt() <= a.radius
        })
    }
}
