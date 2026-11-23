//! NPC routines: townsfolk go round their spots and work at them. A
//! lumberjack chops at a log, walks to the wood pile, wipes his brow; a
//! smith hammers at the anvil; a fisher stands by the water with a rod.
//!
//! All of it is data in the rules (`Npc::routine`): spots relative to where
//! the NPC stands, or "beside the nearest anvil", so a round survives the
//! world being rebuilt; the animation played at each (a name from the
//! animation contract); how long they stay; emotes thrown in. The server
//! walks them and tells clients what they are doing with the messages
//! creatures and players already use (`EntityMoves`, `Act::Emote`).

use crate::game::{Entity, State};
use dyno_protocol::{self as proto, EntityKind, ServerMsg};
use dyno_world::content::RoutineStep;
use dyno_world::{self as w, World};
use std::time::{Duration, Instant};

/// Walking pace between spots, metres a second.
const PACE: f32 = 1.35;

struct Spot {
    pos: [f32; 3],
    /// the way they face there (None: the way they came)
    yaw: Option<f32>,
    /// the animation held there
    anim: Option<u8>,
    emotes: Vec<u8>,
    secs: [f32; 2],
}

enum Doing {
    Walking,
    Staying { left: f32 },
}

/// One NPC's round.
pub struct Routine {
    spots: Vec<Spot>,
    at: usize,
    doing: Doing,
    /// the animation held now, for those who come into view later
    pub hold: Option<u8>,
    emote_in: f32,
    /// seconds without getting nearer the next spot
    stuck: f32,
    /// this NPC's own run of random numbers
    dice: u32,
}

/// Whether someone can stand with their feet in this cell.
fn free(world: &World, x: i32, y: i32, z: i32) -> bool {
    w::is_solid(world.get(x, y - 1, z))
        && !w::is_solid(world.get(x, y, z))
        && world.get(x, y, z) != w::WATER
        && !w::is_solid(world.get(x, y + 1, z))
}

/// The standing height in a column nearest to `y`, within `reach` blocks.
fn stand(world: &World, x: i32, y: i32, z: i32, reach: i32) -> Option<i32> {
    (0..=reach)
        .flat_map(|d| [y + d, y - d])
        .find(|y| free(world, x, *y, z))
}

fn yaw_to(from: [f32; 3], to: [f32; 3]) -> f32 {
    (-(to[0] - from[0])).atan2(-(to[2] - from[2]))
}

/// Where a step of a routine is in this world, for an NPC standing at
/// `home`. None when the place cannot be stood on (the step is skipped).
fn resolve(world: &World, home: [f32; 3], st: &RoutineStep) -> Option<Spot> {
    let (hx, hy, hz) = (
        home[0].floor() as i32,
        home[1].round() as i32,
        home[2].floor() as i32,
    );
    let (bx, bz) = (hx + st.at[0], hz + st.at[1]);
    let mut yaw = st.face.map(|d| d.to_radians());
    let cell = match st.near.as_deref().and_then(w::block_by_key) {
        Some(block) => {
            // the nearest such block, then the nearest place beside it
            let r = st.within.clamp(1, 16) as i32;
            let mut best: Option<(i32, [i32; 3])> = None;
            for dz in -r..=r {
                for dx in -r..=r {
                    for dy in -3..=4 {
                        let p = [bx + dx, hy + dy, bz + dz];
                        if world.get(p[0], p[1], p[2]) != block {
                            continue;
                        }
                        let d = dx * dx + dz * dz + dy * dy;
                        if best.is_none_or(|(b, _)| d < b) {
                            best = Some((d, p));
                        }
                    }
                }
            }
            let (_, b) = best?;
            let beside = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .filter_map(|(dx, dz)| {
                    let (x, z) = (b[0] + dx, b[2] + dz);
                    stand(world, x, b[1], z, 2).map(|y| [x, y, z])
                })
                .min_by_key(|c| (c[0] - hx).pow(2) + (c[2] - hz).pow(2))?;
            let pos = [
                beside[0] as f32 + 0.5,
                beside[1] as f32,
                beside[2] as f32 + 0.5,
            ];
            yaw = yaw.or(Some(yaw_to(
                pos,
                [b[0] as f32 + 0.5, pos[1], b[2] as f32 + 0.5],
            )));
            beside
        }
        None => [bx, stand(world, bx, hy, bz, 6)?, bz],
    };
    let index = |n: &String| proto::anim_index(n);
    Some(Spot {
        pos: [cell[0] as f32 + 0.5, cell[1] as f32, cell[2] as f32 + 0.5],
        yaw,
        anim: st.anim.as_ref().and_then(index),
        emotes: st.emotes.iter().filter_map(index).collect(),
        secs: st.secs,
    })
}

impl Routine {
    /// The round of an NPC standing at `home`. None when it has none (or no
    /// spot of it exists in this world).
    pub fn new(world: &World, home: [f32; 3], steps: &[RoutineStep], seed: u32) -> Option<Routine> {
        let spots: Vec<Spot> = steps
            .iter()
            .filter_map(|s| resolve(world, home, s))
            .collect();
        if spots.is_empty() {
            return None;
        }
        Some(Routine {
            at: seed as usize % spots.len(),
            spots,
            doing: Doing::Walking,
            hold: None,
            emote_in: 3.0 + (seed % 5) as f32,
            stuck: 0.0,
            dice: seed.wrapping_mul(2_654_435_761) | 1,
        })
    }
}

impl State {
    /// Give every NPC with a routine its round (after NPCs are placed).
    pub fn start_routines(&mut self) {
        self.routines.clear();
        let mut made = Vec::new();
        for (id, e) in &self.entities {
            let EntityKind::Npc(n) = e.kind else {
                continue;
            };
            let Some(npc) = self.content.npc(n) else {
                continue;
            };
            if let Some(r) = Routine::new(&self.world, e.pos, &npc.routine, *id) {
                made.push((*id, r));
            }
        }
        self.routines.extend(made);
    }

    /// Walk the NPCs round their spots.
    pub fn tick_routines(&mut self, dt: f32) {
        if self.routines.is_empty() {
            return;
        }
        let started = Instant::now();
        let mut acts: Vec<(u32, u8)> = Vec::new();
        let mut gone: Vec<u32> = Vec::new();
        let State {
            routines,
            entities,
            world,
            ..
        } = self;
        for (id, r) in routines.iter_mut() {
            let Some(e) = entities.get_mut(id) else {
                gone.push(*id);
                continue;
            };
            // their own throws: the world's dice stay as they were, so
            // what the world rolls does not depend on who is sweeping
            r.dice = r.dice.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let roll = (r.dice >> 8) as f32 / 16_777_216.0;
            step(r, e, world, dt, roll, &mut |a| acts.push((*id, a)));
        }
        for id in gone {
            self.routines.remove(&id);
        }
        for (id, anim) in acts {
            self.broadcast(
                &ServerMsg::Act {
                    actor: proto::Actor::Entity(id),
                    act: proto::Act::Emote(anim),
                },
                None,
            );
        }
        self.routine_cost.0 += started.elapsed();
        self.routine_cost.1 += 1;
    }

    /// Mean time `tick_routines` has taken, and how many NPCs it walks.
    pub fn routine_cost(&self) -> (Duration, usize) {
        let (total, n) = self.routine_cost;
        (total / n.max(1), self.routines.len())
    }
}

/// Advance one NPC. `roll` is a fresh number in 0..1; `act` announces an
/// animation (0, the first of the list, lets a held one go).
fn step(
    r: &mut Routine,
    e: &mut Entity,
    world: &World,
    dt: f32,
    roll: f32,
    act: &mut dyn FnMut(u8),
) {
    let spot = &r.spots[r.at];
    match &mut r.doing {
        Doing::Staying { left } => {
            *left -= dt;
            r.emote_in -= dt;
            if r.emote_in <= 0.0 && !spot.emotes.is_empty() {
                r.emote_in = 5.0 + roll * 9.0;
                act(spot.emotes[(roll * 97.0) as usize % spot.emotes.len()]);
            }
            if *left <= 0.0 && r.spots.len() > 1 {
                r.at = (r.at + 1) % r.spots.len();
                r.doing = Doing::Walking;
                r.stuck = 0.0;
                if r.hold.take().is_some() {
                    act(0);
                }
            } else if *left <= 0.0 {
                // a round of one spot: stay on
                *left = spot.secs[0] + (spot.secs[1] - spot.secs[0]) * roll;
            }
        }
        Doing::Walking => {
            let (dx, dz) = (spot.pos[0] - e.pos[0], spot.pos[2] - e.pos[2]);
            let dist = dx.hypot(dz);
            let pace = PACE * dt;
            if dist <= pace.max(0.05) {
                // there
                if dist > 0.0 || e.pos[1] != spot.pos[1] {
                    e.pos = spot.pos;
                    e.moved = true;
                }
                if let Some(y) = spot.yaw
                    && e.yaw != y
                {
                    e.yaw = y;
                    e.moved = true;
                }
                r.doing = Doing::Staying {
                    left: spot.secs[0] + (spot.secs[1] - spot.secs[0]) * roll,
                };
                r.hold = spot.anim;
                if let Some(a) = spot.anim {
                    act(a);
                }
                return;
            }
            let (nx, nz) = (e.pos[0] + dx / dist * pace, e.pos[2] + dz / dist * pace);
            let y = e.pos[1].round() as i32;
            match stand(world, nx.floor() as i32, y, nz.floor() as i32, 1) {
                Some(ny) => {
                    e.pos = [nx, ny as f32, nz];
                    e.yaw = yaw_to([0.0; 3], [dx, 0.0, dz]);
                    e.moved = true;
                    r.stuck = 0.0;
                }
                None => {
                    // something in the way: give this spot up for now
                    r.stuck += dt;
                    if r.stuck > 1.0 {
                        r.stuck = 0.0;
                        r.at = (r.at + 1) % r.spots.len();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> World {
        let mut world = World::empty([32, 32, 32]);
        for x in 0..32 {
            for z in 0..32 {
                for y in 0..=10 {
                    world.set(x, y, z, if y == 10 { w::GRASS } else { w::DIRT });
                }
            }
        }
        world
    }

    fn entity(pos: [f32; 3]) -> Entity {
        let now = Instant::now();
        Entity {
            kind: EntityKind::Npc(0),
            pos,
            vel: [0.0; 3],
            yaw: 0.0,
            born: now,
            pickup_after: now,
            moved: false,
            mob: None,
            phase: None,
        }
    }

    #[test]
    fn a_round_walks_works_and_moves_on() {
        let mut world = flat();
        world.set(20, 11, 16, w::ANVIL);
        let steps = vec![
            RoutineStep {
                near: Some("anvil".into()),
                within: 8,
                anim: Some("Hammer".into()),
                secs: [2.0, 2.0],
                ..Default::default()
            },
            RoutineStep {
                at: [-4, 0],
                secs: [1.0, 1.0],
                emotes: vec!["Wave".into()],
                ..Default::default()
            },
        ];
        let home = [16.5, 11.0, 16.5];
        let mut r = Routine::new(&world, home, &steps, 0).unwrap();
        // beside the anvil, on the near side, facing it
        assert_eq!(r.spots[0].pos, [19.5, 11.0, 16.5]);
        let face = r.spots[0].yaw.unwrap();
        assert!((face.sin() + 1.0).abs() < 1e-5, "faces +x: {face}");
        let mut e = entity(home);
        let mut acts = Vec::new();
        let mut seen_hammer_at = None;
        for i in 0..400 {
            step(&mut r, &mut e, &world, 0.05, 0.5, &mut |a| acts.push(a));
            if seen_hammer_at.is_none() && r.hold.is_some() {
                seen_hammer_at = Some(i);
                assert_eq!(e.pos, [19.5, 11.0, 16.5]);
            }
        }
        let hammer = proto::anim_index("Hammer").unwrap();
        assert!(seen_hammer_at.is_some());
        // hammered, let go, waved at the other spot, came back
        assert!(
            acts.iter().filter(|a| **a == hammer).count() >= 2,
            "{acts:?}"
        );
        assert!(acts.contains(&0));
        assert!(acts.contains(&proto::anim_index("Wave").unwrap()));
    }

    #[test]
    fn a_wall_does_not_trap_them() {
        let mut world = flat();
        for z in 0..32 {
            for y in 11..14 {
                world.set(19, y, z, w::STONE);
            }
        }
        let steps = vec![
            RoutineStep {
                at: [6, 0],
                secs: [1.0, 1.0],
                ..Default::default()
            },
            RoutineStep {
                at: [-3, 0],
                secs: [1.0, 1.0],
                ..Default::default()
            },
        ];
        let home = [16.5, 11.0, 16.5];
        let mut r = Routine::new(&world, home, &steps, 0).unwrap();
        let mut e = entity(home);
        let mut reached = false;
        for _ in 0..600 {
            step(&mut r, &mut e, &world, 0.05, 0.5, &mut |_| {});
            assert!(e.pos[0] < 19.0, "walked into the wall: {:?}", e.pos);
            reached |= e.pos[0] < 14.0;
        }
        assert!(reached, "never went to the other spot");
    }

    /// The cost of a tick with a town's worth of routines.
    #[test]
    fn tick_cost() {
        let world = flat();
        let steps: Vec<RoutineStep> = (0..4)
            .map(|i| RoutineStep {
                at: [[5, 0], [0, 5], [-5, 0], [0, -5]][i],
                secs: [0.5, 1.0],
                anim: Some("Chop".into()),
                emotes: vec!["Wave".into()],
                ..Default::default()
            })
            .collect();
        let home = [16.5, 11.0, 16.5];
        let mut all: Vec<(Routine, Entity)> = (0..40)
            .map(|i| (Routine::new(&world, home, &steps, i).unwrap(), entity(home)))
            .collect();
        let ticks = 4000;
        let t = Instant::now();
        for _ in 0..ticks {
            for (r, e) in &mut all {
                step(r, e, &world, 0.05, 0.37, &mut |_| {});
            }
        }
        let per = t.elapsed().as_secs_f64() * 1e6 / ticks as f64;
        eprintln!("routines: 40 NPCs, {per:.2} microseconds a tick");
        // the tick is 50 ms; this must stay far under a thousandth of it
        assert!(per < 500.0, "{per} microseconds a tick");
    }
}
