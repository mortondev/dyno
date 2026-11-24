//! Player movement: walking, running, jumping, swimming, stepping up and
//! colliding with the block world. Shared by client and server, so the
//! server can simulate players from their inputs and the client can predict
//! the same result (docs/design/data-and-streaming.md §6).

use dyno_world::{self as w, World};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

pub const EYE: f32 = 1.62;
const HALF_W: f32 = 0.3;
const HEIGHT: f32 = 1.8;
/// 1.6 times Earth's gravity: real gravity looks floaty from a game camera,
/// and most games use 1.5 to 2.5 times it (scale review).
pub const GRAVITY: f32 = 15.7;
/// A jump that clears about 0.6 m, like a real running jump onto a ledge.
pub const JUMP: f32 = 4.35;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub forward: f32,
    pub right: f32,
    pub jump: bool,
    pub down: bool,
    pub sprint: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

/// The longest step a client may send: longer frames are split.
pub const MAX_STEP_MS: u8 = 50;

/// One step of a player's movement as it travels from client to server
/// (docs/design/netcode.md): how long it lasts, the movement keys, and the
/// facing. Both sides run [`Player::step`] on exactly these numbers, so the
/// client's prediction and the server's simulation agree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// milliseconds, 1 to [`MAX_STEP_MS`]
    pub ms: u8,
    /// -127..=127 for -1..=1
    pub forward: i8,
    pub right: i8,
    /// [`Step::JUMP`], [`Step::DOWN`], [`Step::SPRINT`]
    pub keys: u8,
    /// facing, a whole turn in 65536 parts
    pub yaw: u16,
}

impl Step {
    pub const JUMP: u8 = 1;
    pub const DOWN: u8 = 2;
    pub const SPRINT: u8 = 4;

    /// The step for `ms` milliseconds of `input` facing `yaw` (radians),
    /// rounded to what the wire carries.
    pub fn new(ms: u8, input: Input, yaw: f32) -> Step {
        let axis = |v: f32| (v.clamp(-1.0, 1.0) * 127.0).round() as i8;
        let turn = yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        Step {
            ms: ms.clamp(1, MAX_STEP_MS),
            forward: axis(input.forward),
            right: axis(input.right),
            keys: [
                (input.jump, Step::JUMP),
                (input.down, Step::DOWN),
                (input.sprint, Step::SPRINT),
            ]
            .iter()
            .filter(|(on, _)| *on)
            .fold(0, |k, (_, bit)| k | bit),
            yaw: ((turn * 65536.0).round() as u32 & 0xffff) as u16,
        }
    }

    pub fn input(&self) -> Input {
        Input {
            forward: (self.forward.max(-127) as f32) / 127.0,
            right: (self.right.max(-127) as f32) / 127.0,
            jump: self.keys & Step::JUMP != 0,
            down: self.keys & Step::DOWN != 0,
            sprint: self.keys & Step::SPRINT != 0,
        }
    }

    pub fn yaw(&self) -> f32 {
        self.yaw as f32 * (std::f32::consts::TAU / 65536.0)
    }

    pub fn dt(&self) -> f32 {
        self.ms.clamp(1, MAX_STEP_MS) as f32 / 1000.0
    }

    /// Standing still (no keys), facing as before.
    pub fn idle(ms: u8, yaw: u16) -> Step {
        Step {
            ms: ms.clamp(1, MAX_STEP_MS),
            yaw,
            ..Default::default()
        }
    }
}

/// How a world's rules move a player: RPG worlds (`protected`) run at 6.2
/// and step up ledges; only build mode flies. Client, bots and server all
/// set the player up with this, so they simulate the same body.
pub fn apply_rules(p: &mut Player, protected: bool, build: bool) {
    if protected && !build {
        p.auto_step = true;
        p.run_speed = 6.2;
    }
    if !build {
        p.flying = false;
    }
}

impl Player {
    /// Run one [`Step`]: face its way, then move for its time.
    pub fn step(&mut self, world: &World, step: Step) {
        self.yaw = step.yaw();
        self.update(world, step.input(), step.dt());
    }

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
        // where the world has not streamed in yet, wait for it in place
        if !world.known(
            self.pos.x.floor() as i32,
            (self.pos.y - 0.5).floor() as i32,
            self.pos.z.floor() as i32,
        ) {
            self.vel = Vec3::ZERO;
            return;
        }
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
            if world.solid(
                (p.x + dx * 0.98).floor() as i32,
                y,
                (p.z + dz * 0.98).floor() as i32,
            ) {
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
                    // ground that has not streamed in yet holds you up
                    if world.solid(x, y, z) {
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
