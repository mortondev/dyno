//! Local player movement, collision, block targeting and remote players.

use dyno_protocol::{PlayerId, Pose};
use dyno_world::{self as w, World};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::time::Instant;

pub const EYE: f32 = 1.62;
const HALF_W: f32 = 0.3;
const HEIGHT: f32 = 1.8;
const GRAVITY: f32 = 28.0;
const JUMP: f32 = 8.6;
pub const REACH: f32 = 7.0;

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub forward: f32,
    pub right: f32,
    pub jump: bool,
    pub down: bool,
    pub sprint: bool,
}

pub struct Player {
    /// Feet position.
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub flying: bool,
}

impl Player {
    pub fn new(spawn: [f32; 3]) -> Self {
        Player {
            pos: spawn.into(),
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            flying: false,
        }
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::Y * EYE
    }

    pub fn look_dir(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    pub fn view_proj(&self, aspect: f32, fov_deg: f32, far: f32) -> Mat4 {
        let proj = Mat4::perspective_rh(fov_deg.to_radians(), aspect.max(0.01), 0.05, far);
        let view = Mat4::look_to_rh(self.eye(), self.look_dir(), Vec3::Y);
        proj * view
    }

    pub fn pose(&self) -> Pose {
        Pose {
            pos: self.pos.into(),
            yaw: self.yaw,
            pitch: self.pitch,
        }
    }

    pub fn update(&mut self, world: &World, input: Input, dt: f32) {
        let dt = dt.min(0.05);
        let (sy, cy) = self.yaw.sin_cos();
        let fwd = Vec3::new(-sy, 0.0, -cy);
        let right = Vec3::new(cy, 0.0, -sy);
        let mut wish = fwd * input.forward + right * input.right;
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }
        let in_water = world.get(
            self.pos.x.floor() as i32,
            (self.pos.y + 0.4).floor() as i32,
            self.pos.z.floor() as i32,
        ) == w::WATER;
        let sneaking = input.down && !self.flying && self.on_ground;
        let speed = if self.flying {
            if input.sprint { 18.0 } else { 9.0 }
        } else if input.sprint {
            6.4
        } else {
            4.4
        } * if in_water && !self.flying { 0.55 } else { 1.0 }
            * if sneaking { 0.35 } else { 1.0 };
        let target = wish * speed;
        let accel = if self.on_ground || self.flying {
            14.0
        } else {
            3.5
        };
        let k = (accel * dt).min(1.0);
        self.vel.x += (target.x - self.vel.x) * k;
        self.vel.z += (target.z - self.vel.z) * k;
        if self.flying {
            let vy = (input.jump as i32 - input.down as i32) as f32 * speed;
            self.vel.y += (vy - self.vel.y) * k;
        } else if in_water {
            self.vel.y -= GRAVITY * 0.25 * dt;
            if input.jump {
                self.vel.y = self.vel.y.max(3.2);
            }
            self.vel.y = self.vel.y.max(-4.0);
        } else {
            self.vel.y -= GRAVITY * dt;
            if input.jump && self.on_ground {
                self.vel.y = JUMP;
            }
            self.vel.y = self.vel.y.max(-60.0);
        }
        self.on_ground = false;
        for axis in [1usize, 0, 2] {
            let mut d = self.vel[axis] * dt;
            // move in small steps so fast falls do not tunnel
            while d.abs() > 1e-6 {
                let step = d.clamp(-0.45, 0.45);
                d -= step;
                let mut next = self.pos;
                next[axis] += step;
                // sneaking: do not step off an edge
                if sneaking
                    && axis != 1
                    && !self.supported(world, next)
                    && self.supported(world, self.pos)
                {
                    self.vel[axis] = 0.0;
                    break;
                }
                if self.collides(world, next) {
                    if axis == 1 && step < 0.0 {
                        self.on_ground = true;
                        // snap to the top of the block
                        self.pos.y = (next.y).floor() + 1.0;
                        if self.collides(world, self.pos) {
                            self.pos.y = next.y - step;
                        }
                    }
                    self.vel[axis] = 0.0;
                    break;
                }
                self.pos = next;
            }
        }
        // keep inside the world horizontally, and above the void
        let [sx, _, sz] = world.size;
        self.pos.x = self.pos.x.clamp(0.4, sx as f32 - 0.4);
        self.pos.z = self.pos.z.clamp(0.4, sz as f32 - 0.4);
        if self.pos.y < -20.0 {
            self.pos = world.spawn().into();
            self.vel = Vec3::ZERO;
        }
    }

    /// True if any solid block is just below the player's feet at `p`.
    fn supported(&self, world: &World, p: Vec3) -> bool {
        let y = (p.y - 0.05).floor() as i32;
        for (dx, dz) in [
            (-HALF_W, -HALF_W),
            (HALF_W, -HALF_W),
            (-HALF_W, HALF_W),
            (HALF_W, HALF_W),
        ] {
            if w::is_solid(world.get(
                (p.x + dx * 0.98).floor() as i32,
                y,
                (p.z + dz * 0.98).floor() as i32,
            )) {
                return true;
            }
        }
        false
    }

    fn collides(&self, world: &World, p: Vec3) -> bool {
        let min = Vec3::new(p.x - HALF_W, p.y, p.z - HALF_W);
        let max = Vec3::new(p.x + HALF_W, p.y + HEIGHT, p.z + HALF_W);
        for y in min.y.floor() as i32..=(max.y - 1e-4).floor() as i32 {
            for z in min.z.floor() as i32..=(max.z - 1e-4).floor() as i32 {
                for x in min.x.floor() as i32..=(max.x - 1e-4).floor() as i32 {
                    if w::is_solid(world.get(x, y, z)) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// True if a block at `p` would overlap the player.
    pub fn overlaps_block(&self, p: [i32; 3]) -> bool {
        let min = Vec3::new(self.pos.x - HALF_W, self.pos.y, self.pos.z - HALF_W);
        let max = Vec3::new(
            self.pos.x + HALF_W,
            self.pos.y + HEIGHT,
            self.pos.z + HALF_W,
        );
        let (bmin, bmax) = (
            Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32),
            Vec3::new(p[0] as f32 + 1.0, p[1] as f32 + 1.0, p[2] as f32 + 1.0),
        );
        min.x < bmax.x
            && max.x > bmin.x
            && min.y < bmax.y
            && max.y > bmin.y
            && min.z < bmax.z
            && max.z > bmin.z
    }
}

/// The block the player looks at, and the empty cell in front of it.
pub struct Target {
    pub block: [i32; 3],
    pub place: [i32; 3],
}

/// Voxel ray cast (Amanatides and Woo).
pub fn raycast(world: &World, origin: Vec3, dir: Vec3, max: f32) -> Option<Target> {
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
        if b != w::AIR && b != w::WATER {
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
    from: Pose,
    to: Pose,
    at: Instant,
}

impl Remote {
    pub fn new(name: String, pose: Pose) -> Self {
        Remote {
            name,
            from: pose,
            to: pose,
            at: Instant::now(),
        }
    }

    pub fn set(&mut self, pose: Pose) {
        self.from = self.current();
        self.to = pose;
        self.at = Instant::now();
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
}
