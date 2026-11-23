//! Player movement: walking, running, jumping, swimming, stepping up and
//! colliding with the block world. Shared by client and server, so the
//! server can simulate players from their inputs and the client can predict
//! the same result (docs/design/data-and-streaming.md §6).

use dyno_world::{self as w, World};
use glam::{Mat4, Vec3};

pub const EYE: f32 = 1.62;
const HALF_W: f32 = 0.3;
const HEIGHT: f32 = 1.8;
/// 1.6 times Earth's gravity: real gravity looks floaty from a game camera,
/// and most games use 1.5 to 2.5 times it (scale review).
pub const GRAVITY: f32 = 15.7;
/// A jump that clears about 0.6 m, like a real running jump onto a ledge.
pub const JUMP: f32 = 4.35;

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
    /// walk up one-block steps without jumping (RPG worlds)
    pub auto_step: bool,
    /// visual lag after stepping up, eased back to 0 so the camera glides
    pub step_offset: f32,
    /// base running speed (no sprint in RPG worlds)
    pub run_speed: f32,
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
            auto_step: true,
            step_offset: 0.0,
            run_speed: 4.4,
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
            self.run_speed
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
        let was_on_ground = self.on_ground;
        self.step_offset = (self.step_offset - dt * 6.0).max(0.0);
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
                    // step up onto a one-block ledge instead of stopping
                    // (or, swimming, climb out over a bank a block above the water)
                    let wet = in_water
                        || world.get(
                            self.pos.x.floor() as i32,
                            (self.pos.y - 0.2).floor() as i32,
                            self.pos.z.floor() as i32,
                        ) == w::WATER;
                    if axis != 1 && self.auto_step && (was_on_ground || wet) && !self.flying {
                        let mut stepped = false;
                        for k in [1.0, 2.0] {
                            let up = Vec3::new(next.x, self.pos.y.floor() + k + 1e-3, next.z);
                            let rise = up.y - self.pos.y;
                            if rise > 0.0 && rise <= 1.1 && !self.collides(world, up) {
                                self.pos = up;
                                self.step_offset = (self.step_offset + rise).min(1.0);
                                self.on_ground = true;
                                stepped = true;
                                break;
                            }
                        }
                        if stepped {
                            continue;
                        }
                    }
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
