//! Local player movement, collision, block targeting and remote players.

use dyno_protocol::{PlayerId, Pose};
use dyno_world::{self as w, World};
use glam::Vec3;
use std::collections::HashMap;
use std::time::Instant;

pub use dyno_sim::movement::{EYE, Input, Player};
pub const REACH: f32 = 7.0;

/// The pose a player reports (until movement is fully server-side).
pub trait PlayerPose {
    fn pose(&self) -> Pose;
}

impl PlayerPose for Player {
    fn pose(&self) -> Pose {
        Pose {
            pos: self.pos.into(),
            yaw: self.yaw,
            pitch: self.pitch,
        }
    }
}

/// The block the player looks at, and the empty cell in front of it.
pub struct Target {
    pub block: [i32; 3],
    pub place: [i32; 3],
}

/// The first block a ray meets (anything but air and water).
pub fn raycast(world: &World, origin: Vec3, dir: Vec3, max: f32) -> Option<Target> {
    raycast_where(world, origin, dir, max, |b| b != w::AIR && b != w::WATER)
}

/// The first block a ray meets that stops a line of sight: a torch (the
/// one block that is not a full cube) does not hide what stands behind it.
pub fn raycast_solid(world: &World, origin: Vec3, dir: Vec3, max: f32) -> Option<Target> {
    raycast_where(world, origin, dir, max, w::is_solid)
}

/// Voxel ray cast (Amanatides and Woo): the first block `stops` is true of.
fn raycast_where(
    world: &World,
    origin: Vec3,
    dir: Vec3,
    max: f32,
    stops: impl Fn(w::BlockId) -> bool,
) -> Option<Target> {
    let mut cell = [
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    ];
    let step = [
        dir.x.signum() as i32,
        dir.y.signum() as i32,
        dir.z.signum() as i32,
    ];
    let inv = |d: f32| {
        if d.abs() < 1e-9 {
            f32::INFINITY
        } else {
            1.0 / d.abs()
        }
    };
    let delta = [inv(dir.x), inv(dir.y), inv(dir.z)];
    let mut tmax = [0f32; 3];
    for i in 0..3 {
        let o = origin[i];
        let frac = if dir[i] > 0.0 {
            o.floor() + 1.0 - o
        } else {
            o - o.floor()
        };
        tmax[i] = if delta[i].is_infinite() {
            f32::INFINITY
        } else {
            frac * delta[i]
        };
    }
    let mut prev = cell;
    let mut t = 0.0;
    while t <= max {
        let b = world.get(cell[0], cell[1], cell[2]);
        if stops(b) {
            return Some(Target {
                block: cell,
                place: prev,
            });
        }
        prev = cell;
        let a = if tmax[0] < tmax[1] && tmax[0] < tmax[2] {
            0
        } else if tmax[1] < tmax[2] {
            1
        } else {
            2
        };
        cell[a] += step[a];
        t = tmax[a];
        tmax[a] += delta[a];
    }
    None
}

/// Another player, drawn between the last two poses received.
pub struct Remote {
    pub name: String,
    /// how they chose to look, and their class colour
    pub look: dyno_protocol::Appearance,
    pub color: [f32; 3],
    /// out of your area of interest: no poses come, so they are not drawn
    /// (they start so, until their first pose arrives)
    pub far: bool,
    from: Pose,
    to: Pose,
    at: Instant,
}

impl Remote {
    pub fn new(name: String, pose: Pose) -> Self {
        Remote {
            name,
            look: Default::default(),
            color: [0.5, 0.5, 0.6],
            far: true,
            from: pose,
            to: pose,
            at: Instant::now(),
        }
    }

    pub fn set(&mut self, pose: Pose) {
        if self.far {
            // back in view: appear where they are, not slide in from where they were
            self.far = false;
            self.from = pose;
            self.to = pose;
            self.at = Instant::now();
            return;
        }
        self.from = self.current();
        self.to = pose;
        self.at = Instant::now();
    }

    /// Velocity in blocks per second, from the last two poses (zero once
    /// they go stale).
    pub fn velocity(&self) -> glam::Vec3 {
        if self.at.elapsed().as_secs_f32() > 0.2 {
            return glam::Vec3::ZERO;
        }
        (glam::Vec3::from(self.to.pos) - glam::Vec3::from(self.from.pos)) / 0.05
    }

    /// Horizontal speed in blocks per second.
    pub fn speed(&self) -> f32 {
        let d = [
            self.to.pos[0] - self.from.pos[0],
            self.to.pos[2] - self.from.pos[2],
        ];
        (d[0] * d[0] + d[1] * d[1]).sqrt() / 0.05
    }

    pub fn current(&self) -> Pose {
        let t = (self.at.elapsed().as_secs_f32() / 0.05).clamp(0.0, 1.0);
        let l = |a: f32, b: f32| a + (b - a) * t;
        let mut dyaw = self.to.yaw - self.from.yaw;
        while dyaw > std::f32::consts::PI {
            dyaw -= std::f32::consts::TAU;
        }
        while dyaw < -std::f32::consts::PI {
            dyaw += std::f32::consts::TAU;
        }
        Pose {
            pos: [
                l(self.from.pos[0], self.to.pos[0]),
                l(self.from.pos[1], self.to.pos[1]),
                l(self.from.pos[2], self.to.pos[2]),
            ],
            yaw: self.from.yaw + dyaw * t,
            pitch: l(self.from.pitch, self.to.pitch),
        }
    }
}

pub fn player_color(id: PlayerId) -> [f32; 3] {
    const C: [[f32; 3]; 8] = [
        [0.23, 0.43, 0.65],
        [0.54, 0.25, 0.42],
        [0.20, 0.55, 0.45],
        [0.75, 0.45, 0.15],
        [0.45, 0.35, 0.70],
        [0.70, 0.25, 0.25],
        [0.25, 0.60, 0.75],
        [0.55, 0.55, 0.20],
    ];
    C[id as usize % C.len()]
}

pub type Remotes = HashMap<PlayerId, Remote>;

#[cfg(test)]
mod tests {
    use super::*;

    fn platform() -> World {
        let mut w = World::empty([16, 16, 16]);
        for x in 4..8 {
            for z in 4..8 {
                w.set(x, 5, z, w::STONE);
            }
        }
        w
    }

    #[test]
    fn sneaking_stops_at_the_edge() {
        let w = platform();
        let mut p = Player::new([6.0, 6.0, 6.0]);
        p.yaw = std::f32::consts::FRAC_PI_2; // facing -x
        for _ in 0..120 {
            p.update(
                &w,
                Input {
                    forward: 1.0,
                    down: true,
                    ..Default::default()
                },
                1.0 / 60.0,
            );
        }
        assert!(p.pos.y > 5.9, "fell off: {:?}", p.pos);
        assert!(p.pos.x > 3.5, "walked off the edge: {:?}", p.pos);
    }

    /// With a life-size jump (0.6 m) you still climb a 1 m step and get out
    /// of water over a 1 m bank: auto-step does it.
    #[test]
    fn climbs_a_step_and_leaves_the_water() {
        let mut w = World::empty([24, 16, 24]);
        for x in 0..24 {
            for z in 0..24 {
                for y in 0..=4 {
                    w.set(x, y, z, dyno_world::STONE);
                }
                // a step up at x = 12, and a pool before it
                if x >= 12 {
                    w.set(x, 5, z, dyno_world::STONE);
                } else if x >= 6 {
                    w.set(x, 4, z, dyno_world::WATER);
                    w.set(x, 3, z, dyno_world::WATER);
                }
            }
        }
        let mut p = Player::new([3.5, 5.0, 12.5]);
        p.yaw = -std::f32::consts::FRAC_PI_2;
        let mut was = p.pos;
        for _ in 0..600 {
            p.update(
                &w,
                Input {
                    forward: 1.0,
                    jump: true,
                    ..Default::default()
                },
                1.0 / 60.0,
            );
            was = was.max(p.pos);
        }
        assert!(
            p.pos.x > 13.0 && p.pos.y >= 5.9,
            "out of the pool and up the step: {:?}",
            p.pos
        );
        use dyno_sim::movement::{GRAVITY, JUMP};
        let peak = 0.5 * JUMP * JUMP / GRAVITY;
        assert!((0.5..0.7).contains(&peak), "a life-size jump: {peak}");
    }

    #[test]
    fn walking_falls_off_the_edge() {
        let w = platform();
        let mut p = Player::new([6.0, 6.0, 6.0]);
        p.yaw = std::f32::consts::FRAC_PI_2;
        for _ in 0..120 {
            p.update(
                &w,
                Input {
                    forward: 1.0,
                    ..Default::default()
                },
                1.0 / 60.0,
            );
        }
        assert!(p.pos.y < 5.9, "should have fallen: {:?}", p.pos);
    }

    #[test]
    fn raycast_hits_the_block_in_front() {
        let w = platform();
        let t = raycast(&w, Vec3::new(6.5, 8.0, 6.5), Vec3::new(0.0, -1.0, 0.0), 6.0).expect("hit");
        assert_eq!(t.block, [6, 5, 6]);
        assert_eq!(t.place, [6, 6, 6]);
    }

    #[test]
    fn a_torch_does_not_block_a_line_of_sight_but_stone_does() {
        let mut w = platform();
        w.set(6, 6, 6, w::TORCH);
        assert!(!w::is_solid(w::TORCH));
        let (o, down) = (Vec3::new(6.5, 9.0, 6.5), Vec3::new(0.0, -1.0, 0.0));
        // a click on the block finds the torch; the line of sight goes on to the stone
        assert_eq!(raycast(&w, o, down, 8.0).unwrap().block, [6, 6, 6]);
        assert_eq!(raycast_solid(&w, o, down, 8.0).unwrap().block, [6, 5, 6]);
    }
}
