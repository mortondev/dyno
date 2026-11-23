//! The animation contract: one set of animations ([`Anim`]) that every
//! character and creature performs, a handful of skeletons ([`Skeleton`]) with
//! named bones, a pose function per skeleton that matches every animation
//! (no wildcard arms: adding an animation is a compile error until every
//! skeleton says how it does it), and an [`Animator`] that crossfades between
//! looping states and layers one-shot actions on top.
//!
//! Everything here is pure and time-based (seconds), so tests and the
//! contact sheet can freeze any pose. See `docs/design/animation.md`.

use dyno_world::content::Model;
use glam::{Quat, Vec3};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

macro_rules! anims {
    ($($(#[$m:meta])* $v:ident),* $(,)?) => {
        /// Every animation a figure can play. Creatures play the same set:
        /// what an animation looks like is up to the skeleton.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Anim {
            $($(#[$m])* $v),*
        }
        impl Anim {
            /// Every animation, in order.
            pub const ALL: &'static [Anim] = &[$(Anim::$v),*];
            pub fn name(self) -> &'static str {
                match self {
                    $(Anim::$v => stringify!($v)),*
                }
            }
        }
    };
}

anims! {
    /// standing: breathing, a slow weight shift, now and then a fidget
    /// (floaters hover)
    Idle,
    /// unhurried movement (floaters drift)
    Walk,
    /// full-speed movement (quadrupeds gallop, floaters fly)
    Run,
    /// moving backwards while facing ahead
    Backpedal,
    StrafeLeft,
    StrafeRight,
    /// the crouch and spring of a jump (one-shot)
    JumpStart,
    /// rising or at the top of a jump
    JumpAir,
    /// falling for a while
    Fall,
    /// the knees soak up a landing (one-shot)
    JumpLand,
    /// in deep water
    Swim,
    /// combat idle: weapon up, knees bent
    Ready,
    /// a one-handed swing (also tools)
    Attack1H,
    /// a two-handed overhead chop
    Attack2H,
    /// a punch; for beasts, their natural attack (bite, gore, claw snap)
    AttackUnarmed,
    /// draw and loose a bow
    Shoot,
    /// a creature's special move (rear up, web, scream, bolt); for people a
    /// war cry
    Special,
    /// precast aimed at a target: hands gather power in front
    CastDirected,
    /// precast with no target (heals, recall): arms raised
    CastOmni,
    /// a held channel: arms thrust out and steady
    Channel,
    /// the spell leaves the hands (one-shot)
    CastRelease,
    /// flinch from a blow (one-shot)
    HitReact,
    /// dazed, swaying
    Stun,
    /// falling dead (holds its last frame)
    Death,
    /// lying dead
    Dead,
    Sit,
    /// down on one knee: looting, gathering
    Kneel,
    /// in conversation: gestures and nods
    Talk,
    Wave,
    Bow,
    Cheer,
    Point,
}

impl Anim {
    /// How long a one-shot lasts, in seconds; `None` for states that loop.
    /// `Death` is a one-shot that holds its last frame.
    pub fn duration(self) -> Option<f32> {
        use Anim::*;
        match self {
            JumpStart => Some(0.18),
            JumpLand => Some(0.32),
            Attack1H => Some(0.7),
            Attack2H => Some(1.0),
            AttackUnarmed => Some(0.5),
            Shoot => Some(1.0),
            Special => Some(1.2),
            CastRelease => Some(0.5),
            HitReact => Some(0.4),
            Death => Some(1.2),
            Wave => Some(2.0),
            Bow => Some(2.0),
            Cheer => Some(1.8),
            Point => Some(1.6),
            Idle | Walk | Run | Backpedal | StrafeLeft | StrafeRight | JumpAir | Fall | Swim
            | Ready | CastDirected | CastOmni | Channel | Stun | Dead | Sit | Kneel | Talk => None,
        }
    }

    /// Whether the legs are busy: an action layered over this keeps the legs
    /// of the base and takes the upper body only.
    pub fn moving(self) -> bool {
        use Anim::*;
        matches!(
            self,
            Walk | Run | Backpedal | StrafeLeft | StrafeRight | JumpAir | Fall | Swim
        )
    }

    /// The emote for a chat command, if it is one: `/wave`, `/bow`...
    pub fn emote(cmd: &str) -> Option<Anim> {
        Some(
            match cmd.trim().trim_start_matches('/').to_lowercase().as_str() {
                "wave" | "hello" | "bye" => Anim::Wave,
                "bow" => Anim::Bow,
                "cheer" => Anim::Cheer,
                "point" => Anim::Point,
                "sit" => Anim::Sit,
                "kneel" => Anim::Kneel,
                "roar" | "charge" => Anim::Special,
                "talk" => Anim::Talk,
                "dead" | "playdead" => Anim::Dead,
                _ => return None,
            },
        )
    }
}

// ------------------------------------------------------------- skeletons

/// The rigs figures are built on. Each has named bones; a [`Pose`] holds a
/// rotation and an offset (voxels) per bone, plus a few shape keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Skeleton {
    /// people, bandits, candlekin: pelvis, spine, head, two-part arms and
    /// legs, a weapon grip and a cape
    Biped,
    /// hooded wraiths with no legs (shade, barrow warden): body, hood, arms
    Wraith,
    /// four legs, a body and a head (grazer, thornboar)
    Quadruped,
    /// a body, up to eight legs and two claws (crab, spider)
    Multileg,
    /// a floating light: core, veil, motes and a tail (gloomwisp)
    Wisp,
}

impl Skeleton {
    pub const ALL: [Skeleton; 5] = [
        Skeleton::Biped,
        Skeleton::Wraith,
        Skeleton::Quadruped,
        Skeleton::Multileg,
        Skeleton::Wisp,
    ];

    /// The skeleton a creature model is built on.
    pub fn of(model: Model, variant: u8) -> Skeleton {
        match (model, variant) {
            (Model::Quadruped, 2 | 3) => Skeleton::Multileg,
            (Model::Quadruped, _) => Skeleton::Quadruped,
            (Model::Humanoid, 1..=3) => Skeleton::Biped,
            (Model::Humanoid, _) => Skeleton::Wraith,
            (Model::Orb, _) => Skeleton::Wisp,
        }
    }

    /// Bone names, by index.
    pub fn bones(self) -> &'static [&'static str] {
        match self {
            Skeleton::Biped => &biped::NAMES,
            Skeleton::Wraith => &wraith::NAMES,
            Skeleton::Quadruped => &quad::NAMES,
            Skeleton::Multileg => &multi::NAMES,
            Skeleton::Wisp => &wisp::NAMES,
        }
    }

    /// The bones an action takes over while the legs keep moving.
    pub fn upper(self) -> u32 {
        let bits = |list: &[usize]| list.iter().fold(0u32, |m, b| m | 1 << b);
        match self {
            Skeleton::Biped => bits(&[
                biped::SPINE,
                biped::HEAD,
                biped::ARM_U[0],
                biped::ARM_U[1],
                biped::ARM_L[0],
                biped::ARM_L[1],
                biped::GRIP,
            ]),
            Skeleton::Wraith => bits(&[
                wraith::HOOD,
                wraith::ARM_U[0],
                wraith::ARM_U[1],
                wraith::ARM_L[0],
                wraith::ARM_L[1],
            ]),
            Skeleton::Quadruped => bits(&[quad::HEAD]),
            Skeleton::Multileg => bits(&[multi::CLAW[0], multi::CLAW[1]]),
            Skeleton::Wisp => bits(&[wisp::CORE, wisp::VEIL]),
        }
    }

    /// How far the figure travels per step (half a stride cycle), in blocks,
    /// at `speed` blocks per second, for a model of size `scale`.
    pub fn step(self, speed: f32, scale: f32) -> f32 {
        let s = match self {
            Skeleton::Biped => (0.55 + 0.2 * speed).clamp(0.8, 1.45),
            Skeleton::Quadruped => (0.4 + 0.1 * speed).clamp(0.5, 0.95),
            Skeleton::Multileg => (0.3 + 0.05 * speed).clamp(0.35, 0.55),
            Skeleton::Wraith | Skeleton::Wisp => 1.2,
        };
        s * scale.max(0.3)
    }
}

/// Bones of a biped. Arms and legs: right then left.
pub mod biped {
    pub const PELVIS: usize = 0;
    pub const SPINE: usize = 1;
    pub const HEAD: usize = 2;
    pub const ARM_U: [usize; 2] = [3, 4];
    pub const ARM_L: [usize; 2] = [5, 6];
    pub const THIGH: [usize; 2] = [7, 8];
    pub const SHIN: [usize; 2] = [9, 10];
    /// the weapon in the right fist
    pub const GRIP: usize = 11;
    /// the top of the cape
    pub const CAPE: usize = 12;
    pub const NAMES: [&str; 13] = [
        "pelvis",
        "spine",
        "head",
        "upper_arm.r",
        "upper_arm.l",
        "forearm.r",
        "forearm.l",
        "thigh.r",
        "thigh.l",
        "shin.r",
        "shin.l",
        "grip",
        "cape",
    ];
}

/// Bones of a legless wraith.
pub mod wraith {
    pub const BODY: usize = 0;
    pub const HOOD: usize = 1;
    pub const ARM_U: [usize; 2] = [2, 3];
    pub const ARM_L: [usize; 2] = [4, 5];
    pub const NAMES: [&str; 6] = [
        "body",
        "hood",
        "upper_arm.r",
        "upper_arm.l",
        "forearm.r",
        "forearm.l",
    ];
}

/// Bones of a four-legged beast.
pub mod quad {
    pub const BODY: usize = 0;
    pub const HEAD: usize = 1;
    /// front right, front left, back right, back left
    pub const LEGS: [usize; 4] = [2, 3, 4, 5];
    pub const NAMES: [&str; 6] = ["body", "head", "leg.fr", "leg.fl", "leg.br", "leg.bl"];
}

/// Bones of a many-legged crawler.
pub mod multi {
    pub const BODY: usize = 0;
    pub const CLAW: [usize; 2] = [1, 2];
    /// legs, in the model's order
    pub const LEG0: usize = 3;
    pub const MAX_LEGS: usize = 8;
    pub const NAMES: [&str; 11] = [
        "body", "claw.r", "claw.l", "leg.0", "leg.1", "leg.2", "leg.3", "leg.4", "leg.5", "leg.6",
        "leg.7",
    ];
}

/// Bones of a wisp.
pub mod wisp {
    pub const ROOT: usize = 0;
    pub const CORE: usize = 1;
    pub const VEIL: usize = 2;
    pub const NAMES: [&str; 3] = ["root", "core", "veil"];
}

/// Shape keys in [`Pose::k`].
pub mod key {
    /// 0..1: capes, rags and tails stream behind
    pub const FLOW: usize = 0;
    /// biped: how far the tunic skirts lift; wraith: eyes open (0..1);
    /// wisp: size of the core
    pub const SHAPE: usize = 1;
    /// wisp: how far the motes circle (1 normal)
    pub const SPREAD: usize = 2;
    /// the animator's clock, for secondary motion (set, not blended)
    pub const CLOCK: usize = 3;
}

// ----------------------------------------------------------------- poses

pub const MAX_BONES: usize = 16;

/// A pose: for each bone a rotation about its pivot and an offset of the
/// pivot (in model voxels), plus shape keys ([`key`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub rot: [Quat; MAX_BONES],
    pub off: [Vec3; MAX_BONES],
    pub k: [f32; 4],
}

impl Default for Pose {
    fn default() -> Self {
        Pose::REST
    }
}

impl Pose {
    pub const REST: Pose = Pose {
        rot: [Quat::IDENTITY; MAX_BONES],
        off: [Vec3::ZERO; MAX_BONES],
        k: [0.0, 0.0, 1.0, 0.0],
    };

    /// `self` moved towards `o` by `w` (0..1), for the bones in `mask`
    /// (shape keys always).
    pub fn blend(&self, o: &Pose, w: f32, mask: u32) -> Pose {
        let w = w.clamp(0.0, 1.0);
        let mut p = *self;
        for i in 0..MAX_BONES {
            if mask & (1 << i) != 0 {
                p.rot[i] = self.rot[i].lerp(o.rot[i], w);
                p.off[i] = self.off[i].lerp(o.off[i], w);
            }
        }
        for i in 0..3 {
            p.k[i] = self.k[i] + (o.k[i] - self.k[i]) * w;
        }
        p.k[key::CLOCK] = o.k[key::CLOCK];
        p
    }

    pub fn is_finite(&self) -> bool {
        self.rot.iter().all(|q| q.is_finite())
            && self.off.iter().all(|v| v.is_finite())
            && self.k.iter().all(|k| k.is_finite())
    }

    /// The biggest difference from another pose: radians or voxels.
    pub fn distance(&self, o: &Pose) -> f32 {
        let mut d = 0.0f32;
        for i in 0..MAX_BONES {
            d = d.max(self.rot[i].angle_between(o.rot[i]));
            d = d.max(self.off[i].distance(o.off[i]) * 0.1);
        }
        for i in 0..3 {
            d = d.max((self.k[i] - o.k[i]).abs());
        }
        d
    }
}

/// Where a pose is evaluated: seconds into the animation, the stride phase
/// (radians, advanced by distance travelled) and a per-figure seed (0..1)
/// that staggers idle fidgets.
#[derive(Clone, Copy, Debug)]
pub struct At {
    pub t: f32,
    pub stride: f32,
    pub seed: f32,
}

/// Rotation: turn about y, then pitch about x, then roll about z (applied to
/// the part in that order from the outside in).
pub fn euler(x: f32, y: f32, z: f32) -> Quat {
    Quat::from_rotation_y(y) * Quat::from_rotation_x(x) * Quat::from_rotation_z(z)
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A keyframed value: eased between `(time, value)` keys, held at the ends.
pub fn track(u: f32, keys: &[(f32, f32)]) -> f32 {
    let Some(first) = keys.first() else {
        return 0.0;
    };
    if u <= first.0 {
        return first.1;
    }
    for w in keys.windows(2) {
        let ((t0, v0), (t1, v1)) = (w[0], w[1]);
        if u <= t1 {
            let f = if t1 > t0 { (u - t0) / (t1 - t0) } else { 1.0 };
            return v0 + (v1 - v0) * smooth(f);
        }
    }
    keys[keys.len() - 1].1
}

/// Normalised time through a one-shot (0..1, held at 1).
fn progress(a: Anim, t: f32) -> f32 {
    (t / a.duration().unwrap_or(1.0)).clamp(0.0, 1.0)
}

/// Rises then falls over a one-shot: 0 at the ends, 1 in the middle.
fn hump(u: f32, rise: f32, fall: f32) -> f32 {
    smooth(u / rise.max(1e-3)).min(smooth((1.0 - u) / fall.max(1e-3)))
}

/// An idle fidget: every ~11 s a 2.5 s envelope, and which of three it is.
fn fidget(at: &At) -> (f32, u32) {
    let period = 11.0;
    let tt = at.t + at.seed * 37.0 + 4.0;
    let f = tt.rem_euclid(period);
    let env = if f < 2.5 { (PI * f / 2.5).sin() } else { 0.0 };
    (
        env,
        ((tt / period) as u32).wrapping_add((at.seed * 7.0) as u32) % 3,
    )
}

// ------------------------------------------------------------------ biped

/// What a biped holds, which shapes its stances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip {
    Empty,
    OneHand,
    TwoHand,
    Bow,
}

/// The parts of a biped that change how it moves.
#[derive(Clone, Copy, Debug)]
pub struct BipedRig {
    pub grip: Grip,
    /// a long robe: short strides
    pub robe: bool,
}

/// The attack a biped makes with what it holds.
pub fn attack_for(grip: Grip) -> Anim {
    match grip {
        Grip::Empty => Anim::AttackUnarmed,
        Grip::OneHand => Anim::Attack1H,
        Grip::TwoHand => Anim::Attack2H,
        Grip::Bow => Anim::Shoot,
    }
}

/// A biped pose under construction: Euler angles per bone.
#[derive(Clone, Copy, Default)]
struct B {
    /// pelvis offset (voxels) and turn
    hip: Vec3,
    pelvis: [f32; 3],
    spine: [f32; 3],
    head: [f32; 3],
    arm: [[f32; 3]; 2],
    fore: [[f32; 3]; 2],
    thigh: [[f32; 3]; 2],
    shin: [f32; 2],
    grip: [f32; 3],
    cape: f32,
    flow: f32,
    flap: f32,
}

const SIDES: [f32; 2] = [1.0, -1.0];
/// thigh and shin length in voxels
const LEG: f32 = 13.0;

impl B {
    fn finish(self) -> Pose {
        use biped::*;
        let mut p = Pose::REST;
        let e = |a: [f32; 3]| euler(a[0], a[1], a[2]);
        p.off[PELVIS] = self.hip;
        p.rot[PELVIS] = e(self.pelvis);
        p.rot[SPINE] = e(self.spine);
        p.rot[HEAD] = e(self.head);
        for i in 0..2 {
            p.rot[ARM_U[i]] = e(self.arm[i]);
            p.rot[ARM_L[i]] = e(self.fore[i]);
            p.rot[THIGH[i]] = e(self.thigh[i]);
            p.rot[SHIN[i]] = Quat::from_rotation_x(self.shin[i]);
        }
        p.rot[GRIP] = e(self.grip);
        p.rot[CAPE] = Quat::from_rotation_x(self.cape);
        p.k[key::FLOW] = self.flow;
        p.k[key::SHAPE] = self.flap;
        p
    }

    /// Bend the knees by `a` with the feet kept under the hips.
    fn crouch(&mut self, a: f32) {
        for i in 0..2 {
            self.thigh[i][0] -= a;
            self.shin[i] += 2.0 * a;
        }
        // the heels lift a little, so the toes do not dig in
        self.hip.y -= 2.0 * LEG * (1.0 - a.cos()) - 4.0 * a.max(0.0).sin();
        self.spine[0] += a * 0.35;
    }

    /// Turn the pelvis about x by `a` with the legs and body kept as they
    /// were: only the skirts follow (a robe swept forward to sit in).
    fn tilt(&mut self, a: f32) {
        self.pelvis[0] += a;
        self.spine[0] -= a;
        for i in 0..2 {
            self.thigh[i][0] -= a;
        }
    }

    /// Feet apart, left foot forward: a fighter's stance.
    fn stance(&mut self, a: f32) {
        self.crouch(a);
        self.thigh[0][2] += 0.1;
        self.thigh[1][2] -= 0.1;
        self.thigh[0][0] += 0.15;
        self.thigh[1][0] -= 0.25;
        self.shin[1] += 0.1;
        self.pelvis[1] -= 0.18;
        self.spine[1] += 0.12;
    }

    /// Arms as they rest by what is held.
    fn arms_rest(&mut self, rig: &BipedRig, breath: f32) {
        for (i, s) in SIDES.into_iter().enumerate() {
            self.arm[i] = [breath * 0.025 * s, 0.0, s * (0.06 + breath * 0.015)];
            self.fore[i] = [-0.14, 0.0, 0.0];
        }
        match rig.grip {
            Grip::Empty => {}
            Grip::OneHand | Grip::Bow => {
                self.arm[0] = [-0.45 + breath * 0.02, 0.0, 0.05];
                self.fore[0] = [-0.95, 0.0, 0.0];
            }
            Grip::TwoHand => {
                self.arm[0] = [-0.4 + breath * 0.02, 0.0, 0.05];
                self.fore[0] = [-1.0, 0.0, 0.0];
                self.grip = [0.0, 0.0, -0.1];
            }
        }
    }

    /// Arms held up for a fight.
    fn arms_ready(&mut self, rig: &BipedRig) {
        match rig.grip {
            Grip::Empty => {
                for (i, s) in SIDES.into_iter().enumerate() {
                    self.arm[i] = [-0.55, 0.0, s * 0.18];
                    self.fore[i] = [-1.9, 0.0, 0.0];
                }
                self.arm[1][0] = -0.7;
            }
            Grip::OneHand => {
                self.arm[0] = [-0.75, 0.0, 0.12];
                self.fore[0] = [-1.0, 0.0, 0.0];
                self.grip = [-0.3, 0.0, 0.0];
                self.arm[1] = [-0.5, 0.0, -0.2];
                self.fore[1] = [-1.5, 0.0, 0.0];
            }
            Grip::TwoHand => {
                self.arm[0] = [-0.65, 0.0, -0.2];
                self.fore[0] = [-1.2, 0.0, 0.0];
                self.arm[1] = [-0.7, 0.0, 0.45];
                self.fore[1] = [-1.25, 0.0, 0.0];
                self.grip = [-0.25, 0.0, 0.0];
            }
            Grip::Bow => {
                self.arm[0] = [-0.55, 0.0, 0.1];
                self.fore[0] = [-0.55, 0.0, 0.0];
                self.arm[1] = [-0.35, 0.0, 0.1];
                self.fore[1] = [-0.9, 0.0, 0.0];
            }
        }
    }

    /// Legs and arms in a stride: `swing` and `knee` set how big.
    fn stride(&mut self, ph: f32, swing: f32, knee: f32, rig: &BipedRig) {
        let (sn, cs) = ph.sin_cos();
        let reach = if rig.robe { 0.35 } else { 1.0 };
        for (i, s) in SIDES.into_iter().enumerate() {
            self.thigh[i][0] = sn * swing * s * reach;
            // the knee folds as the foot comes through, more on a trailing leg
            self.shin[i] = ((-cs * s).max(0.0) * knee + (sn * s).max(0.0) * knee * 0.6) * reach;
        }
        self.pelvis[1] = sn * 0.08 * swing;
        self.spine[1] = -sn * 0.08 * swing;
        self.flap = (sn * swing).abs() * 0.6;
    }
}

/// The pose of a biped.
pub fn biped(a: Anim, at: &At, rig: &BipedRig) -> Pose {
    let t = at.t;
    let ph = at.stride;
    let u = progress(a, t);
    let breath = (t * 2.1 + at.seed * TAU).sin();
    let mut b = B::default();
    b.arms_rest(rig, breath);
    b.cape = 0.0;
    b.flow = 0.05;
    let armed = rig.grip != Grip::Empty;
    match a {
        Anim::Idle => {
            b.spine[0] = 0.015 * breath;
            b.head[0] = -0.02 * breath;
            b.hip.y = 0.15 * breath;
            // shift the weight from foot to foot
            let w = (t * 0.45 + at.seed * 9.0).sin();
            b.hip.x = w * 0.7;
            b.pelvis[2] = w * 0.035;
            for i in 0..2 {
                b.thigh[i][2] = -w * 0.035;
            }
            b.spine[2] = -w * 0.03;
            let (env, kind) = fidget(at);
            match kind {
                0 => b.head[1] = (t * 2.3).sin() * 0.65 * env,
                1 => {
                    b.spine[0] -= 0.12 * env;
                    b.head[0] -= 0.3 * env;
                    for (i, s) in SIDES.into_iter().enumerate() {
                        b.arm[i][2] += s * 0.25 * env;
                    }
                }
                _ => {
                    b.head[0] += 0.35 * env;
                    b.head[1] -= 0.25 * env;
                    if armed {
                        b.fore[0][0] -= 0.35 * env;
                        b.grip[1] += 0.6 * env;
                    } else {
                        b.arm[1][0] -= 0.3 * env;
                        b.fore[1][0] -= 0.9 * env;
                    }
                }
            }
        }
        Anim::Walk | Anim::Backpedal => {
            let back = a == Anim::Backpedal;
            let ph = if back { -ph } else { ph };
            let swing = if back { 0.45 } else { 0.6 };
            b.stride(ph, swing, 0.75, rig);
            b.hip.y = ph.cos().abs() * 0.7;
            b.spine[0] = if back { -0.06 } else { 0.04 };
            let sn = ph.sin();
            for (i, s) in SIDES.into_iter().enumerate() {
                if i == 0 && armed {
                    b.arm[0][0] += -sn * 0.18;
                } else {
                    b.arm[i][0] = -sn * 0.55 * s;
                    b.fore[i][0] = -0.2 - (sn * s).max(0.0) * 0.35;
                }
            }
            b.flow = if back { 0.15 } else { 0.5 };
        }
        Anim::Run => {
            b.stride(ph, 0.95, 1.35, rig);
            for i in 0..2 {
                b.shin[i] += 0.25;
            }
            b.hip.y = ph.cos().abs() * 1.8 - 1.2;
            b.spine[0] = 0.24;
            b.head[0] = -0.16;
            b.spine[1] *= 1.6;
            let sn = ph.sin();
            for (i, s) in SIDES.into_iter().enumerate() {
                if i == 0 && armed {
                    b.arm[0] = [-0.3 - sn * 0.3, 0.0, 0.12];
                    b.fore[0] = [-1.15, 0.0, 0.0];
                    b.grip[0] = 0.35;
                } else {
                    b.arm[i] = [-sn * 0.9 * s, 0.0, s * 0.12];
                    b.fore[i] = [-1.25 - (sn * s).max(0.0) * 0.2, 0.0, 0.0];
                }
            }
            b.flow = 1.0;
        }
        Anim::StrafeLeft | Anim::StrafeRight => {
            let dir = if a == Anim::StrafeRight { 1.0 } else { -1.0 };
            b.stride(ph, 0.55, 0.8, rig);
            b.hip.y = ph.cos().abs() * 0.8;
            // hips turn towards the way it goes, shoulders stay ahead
            b.pelvis[1] += dir * 0.75;
            b.spine[1] -= dir * 0.75;
            b.pelvis[2] = dir * 0.05;
            b.spine[2] = -dir * 0.05;
            b.flow = 0.35;
        }
        Anim::JumpStart => {
            b.crouch(track(u, &[(0.0, 0.1), (0.5, 0.45), (1.0, 0.15)]));
            let sw = track(u, &[(0.0, 0.0), (0.5, 0.5), (1.0, -1.0)]);
            for i in 0..2 {
                if !(i == 0 && armed) {
                    b.arm[i][0] = sw;
                }
            }
            b.flow = 0.4;
        }
        Anim::JumpAir => {
            b.thigh[0][0] = -0.75;
            b.shin[0] = 1.25;
            b.thigh[1][0] = -0.2;
            b.shin[1] = 0.55;
            b.spine[0] = 0.1;
            for (i, s) in SIDES.into_iter().enumerate() {
                if !(i == 0 && armed) {
                    b.arm[i] = [-0.9, 0.0, s * 0.55];
                    b.fore[i] = [-0.5, 0.0, 0.0];
                }
            }
            b.flow = 0.7;
            b.flap = 0.4;
        }
        Anim::Fall => {
            for (i, s) in SIDES.into_iter().enumerate() {
                b.thigh[i] = [(t * 7.0 + s).sin() * 0.3 - 0.15, 0.0, s * 0.14];
                b.shin[i] = 0.45 + (t * 7.0 + s).cos() * 0.2;
                b.arm[i] = [-1.9 + (t * 9.0 + s * 1.5).sin() * 0.35, 0.0, s * 0.9];
                b.fore[i] = [-0.4, 0.0, 0.0];
            }
            b.spine[0] = -0.12;
            b.head[0] = -0.2;
            b.cape = -0.6;
            b.flow = 0.2;
            b.flap = 0.5;
        }
        Anim::JumpLand => {
            b.crouch(track(u, &[(0.0, 0.55), (0.3, 0.65), (1.0, 0.0)]));
            for i in 0..2 {
                if !(i == 0 && armed) {
                    b.arm[i][0] = -0.35 * (1.0 - u);
                }
            }
        }
        Anim::Swim => {
            // body laid forward in the water, a breaststroke and a kick
            b.pelvis[0] = 1.15;
            b.hip.y = 2.0;
            b.head[0] = -0.95;
            let s5 = t * 4.2;
            for (i, s) in SIDES.into_iter().enumerate() {
                b.thigh[i] = [(t * 5.5 + s * FRAC_PI_2).sin() * 0.35, 0.0, s * 0.08];
                b.shin[i] = 0.35 + (t * 5.5 + s).cos().max(0.0) * 0.4;
                let pull = s5.sin();
                b.arm[i] = [-2.5 + pull * 0.55, 0.0, s * (0.35 + pull.max(0.0) * 0.6)];
                b.fore[i] = [-0.3 - (-pull).max(0.0) * 0.8, 0.0, 0.0];
            }
            b.flow = 0.3;
        }
        Anim::Ready => {
            b.stance(0.24);
            b.arms_ready(rig);
            b.hip.y += (t * 4.4 + at.seed).sin() * 0.35;
            b.head[0] = -0.12;
            b.flow = 0.1;
        }
        Anim::Attack1H => {
            b.stance(track(
                u,
                &[(0.0, 0.22), (0.32, 0.15), (0.5, 0.36), (1.0, 0.22)],
            ));
            b.arms_ready(rig);
            b.arm[0] = [
                track(
                    u,
                    &[
                        (0.0, -0.75),
                        (0.32, -2.75),
                        (0.5, -0.35),
                        (0.65, -0.3),
                        (1.0, -0.75),
                    ],
                ),
                0.0,
                track(u, &[(0.0, 0.1), (0.32, 0.5), (0.5, -0.4), (1.0, 0.1)]),
            ];
            b.fore[0][0] = track(u, &[(0.0, -1.0), (0.32, -1.45), (0.5, -0.12), (1.0, -1.0)]);
            b.grip[0] = track(u, &[(0.0, -0.3), (0.32, 0.25), (0.5, -0.5), (1.0, -0.3)]);
            b.spine[1] += track(u, &[(0.0, 0.0), (0.32, 0.5), (0.5, -0.45), (1.0, 0.0)]);
            b.spine[0] += track(u, &[(0.0, 0.0), (0.32, -0.12), (0.5, 0.18), (1.0, 0.0)]);
            b.arm[1][0] = track(u, &[(0.0, -0.5), (0.32, -0.8), (0.5, 0.25), (1.0, -0.5)]);
            b.thigh[0][0] += track(u, &[(0.0, 0.0), (0.5, -0.4), (1.0, 0.0)]);
            b.thigh[1][0] += track(u, &[(0.0, 0.0), (0.5, 0.3), (1.0, 0.0)]);
            b.hip.z = track(u, &[(0.0, 0.0), (0.5, 2.5), (1.0, 0.0)]);
            b.head[0] = -0.1;
        }
        Anim::Attack2H => {
            b.stance(track(
                u,
                &[(0.0, 0.22), (0.4, 0.1), (0.58, 0.34), (1.0, 0.22)],
            ));
            b.arms_ready(rig);
            let lift = track(
                u,
                &[
                    (0.0, -0.65),
                    (0.4, -2.95),
                    (0.58, -0.55),
                    (0.75, -0.5),
                    (1.0, -0.65),
                ],
            );
            let fore = track(u, &[(0.0, -1.2), (0.4, -0.95), (0.58, -0.1), (1.0, -1.2)]);
            b.arm[0] = [
                lift,
                0.0,
                track(u, &[(0.0, -0.2), (0.4, -0.05), (0.58, -0.3), (1.0, -0.2)]),
            ];
            b.arm[1] = [
                lift - 0.05,
                0.0,
                track(u, &[(0.0, 0.45), (0.4, 0.2), (0.58, 0.45), (1.0, 0.45)]),
            ];
            b.fore = [[fore, 0.0, 0.0], [fore - 0.05, 0.0, 0.0]];
            b.grip[0] = track(u, &[(0.0, -0.25), (0.4, 0.3), (0.58, -0.35), (1.0, -0.25)]);
            let lean = track(
                u,
                &[
                    (0.0, 0.0),
                    (0.4, -0.3),
                    (0.58, 0.32),
                    (0.75, 0.28),
                    (1.0, 0.0),
                ],
            );
            b.spine[0] += lean;
            b.head[0] = -lean * 0.5 - 0.1;
            b.hip.z = track(u, &[(0.0, 0.0), (0.4, -1.0), (0.58, 2.0), (1.0, 0.0)]);
        }
        Anim::AttackUnarmed => {
            b.stance(0.22);
            b.arms_ready(&BipedRig {
                grip: Grip::Empty,
                ..*rig
            });
            b.arm[0] = [
                track(
                    u,
                    &[
                        (0.0, -0.55),
                        (0.25, -0.2),
                        (0.45, -1.5),
                        (0.6, -1.45),
                        (1.0, -0.55),
                    ],
                ),
                0.0,
                track(u, &[(0.0, 0.18), (0.45, -0.18), (1.0, 0.18)]),
            ];
            b.fore[0][0] = track(
                u,
                &[
                    (0.0, -1.9),
                    (0.25, -2.1),
                    (0.45, -0.1),
                    (0.6, -0.15),
                    (1.0, -1.9),
                ],
            );
            b.spine[1] += track(u, &[(0.0, 0.0), (0.25, 0.35), (0.45, -0.45), (1.0, 0.0)]);
            b.hip.z = track(u, &[(0.0, 0.0), (0.45, 2.0), (1.0, 0.0)]);
            b.thigh[0][0] += track(u, &[(0.0, 0.0), (0.45, -0.3), (1.0, 0.0)]);
        }
        Anim::Shoot => {
            // the bow arm goes out straight, the other draws to the chin
            b.stance(0.18);
            b.spine[1] = track(u, &[(0.0, 0.0), (0.25, -0.45), (0.85, -0.45), (1.0, 0.0)]);
            let turn = -b.spine[1] - b.pelvis[1];
            b.arm[0] = [
                track(u, &[(0.0, -0.5), (0.25, -1.55), (0.85, -1.5), (1.0, -0.5)]),
                turn,
                0.0,
            ];
            b.fore[0][0] = track(u, &[(0.0, -0.6), (0.25, -0.05), (1.0, -0.2)]);
            // the bow stands upright in the outstretched fist
            b.grip = [track(u, &[(0.0, 0.0), (0.25, 0.3)]), 0.0, 0.0];
            b.arm[1] = [
                track(
                    u,
                    &[
                        (0.0, -0.3),
                        (0.25, -1.45),
                        (0.7, -1.5),
                        (0.8, -1.3),
                        (1.0, -0.3),
                    ],
                ),
                turn,
                track(
                    u,
                    &[
                        (0.0, 0.1),
                        (0.25, 0.75),
                        (0.7, 0.1),
                        (0.8, -0.35),
                        (1.0, 0.1),
                    ],
                ),
            ];
            b.fore[1][0] = track(
                u,
                &[
                    (0.0, -0.9),
                    (0.25, -0.2),
                    (0.7, -2.2),
                    (0.76, -2.2),
                    (0.85, -0.5),
                    (1.0, -0.9),
                ],
            );
            b.head[1] = -b.spine[1] * 0.8;
        }
        Anim::Special => {
            // a war cry: chest out, arms flung wide, head back
            let env = track(u, &[(0.0, 0.0), (0.25, 1.0), (0.8, 1.0), (1.0, 0.0)]);
            let shake = (t * 31.0).sin() * 0.03 * env;
            b.stance(0.3 * env);
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i] = [
                    b.arm[i][0] * (1.0 - env) + 0.3 * env,
                    0.0,
                    s * (0.06 + 1.05 * env) + shake,
                ];
                b.fore[i][0] = b.fore[i][0] * (1.0 - env) - 0.45 * env;
            }
            b.spine[0] = -0.25 * env;
            b.head[0] = -0.4 * env + shake;
            b.flow = 0.4 * env;
        }
        Anim::CastDirected => {
            b.crouch(0.1);
            let pulse = (t * 5.5).sin();
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i] = [-1.0 + pulse * 0.06, 0.0, -s * 0.4];
                b.fore[i] = [-0.95 - pulse * 0.08, 0.0, 0.0];
            }
            b.spine[0] = 0.1;
            b.head[0] = -0.08;
            b.grip[0] = -0.6;
            b.flow = 0.15 + pulse * 0.05;
        }
        Anim::CastOmni => {
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i] = [-2.55 + (t * 3.0 + s).sin() * 0.08, 0.0, s * 0.45];
                b.fore[i] = [-0.35, 0.0, 0.0];
            }
            b.head[0] = -0.35;
            b.spine[0] = -0.12;
            b.hip.y = (t * 3.0).sin() * 0.3;
            b.flow = 0.2 + (t * 2.0).sin() * 0.1;
        }
        Anim::Channel => {
            b.stance(0.18);
            let tremble = (t * 17.0).sin() * 0.025;
            b.arm[0] = [-1.45 + tremble, 0.0, -0.05];
            b.fore[0] = [-0.08, 0.0, 0.0];
            b.arm[1] = [-1.2 - tremble, 0.0, 0.3];
            b.fore[1] = [-0.35, 0.0, 0.0];
            b.grip[0] = -0.9;
            b.spine[0] += 0.06;
            b.flow = 0.25;
        }
        Anim::CastRelease => {
            let reach = track(u, &[(0.0, -1.0), (0.2, -1.55), (0.5, -1.5), (1.0, -0.45)]);
            let fore = track(u, &[(0.0, -1.0), (0.2, 0.0), (0.5, -0.05), (1.0, -0.3)]);
            let inward = track(u, &[(0.0, 0.4), (0.2, 0.1), (1.0, 0.1)]);
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i] = [reach, 0.0, -s * inward];
                b.fore[i] = [fore, 0.0, 0.0];
            }
            b.grip[0] = -0.9;
            b.spine[0] = track(u, &[(0.0, 0.1), (0.2, 0.25), (1.0, 0.05)]);
            b.hip.z = track(u, &[(0.0, 0.0), (0.2, 1.5), (1.0, 0.0)]);
            b.flow = 0.3;
        }
        Anim::HitReact => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (1.0, 0.0)]);
            let side = if at.seed > 0.5 { 1.0 } else { -1.0 };
            b.crouch(0.2 * env);
            b.spine[0] -= 0.4 * env;
            b.spine[1] += 0.2 * env * side;
            b.head[0] = -0.35 * env;
            b.hip.z = -1.8 * env;
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i][0] += 0.25 * env;
                b.arm[i][2] += s * 0.45 * env;
                b.fore[i][0] -= 0.5 * env;
            }
            b.flow = 0.3 * env;
        }
        Anim::Stun => {
            b.crouch(0.25);
            b.spine[0] += 0.25 + (t * 1.7).sin() * 0.05;
            b.head = [0.35 + (t * 2.3).sin() * 0.15, 0.0, (t * 2.3).cos() * 0.25];
            b.hip.x = (t * 1.7).sin() * 1.0;
            b.pelvis[2] = (t * 1.7).sin() * 0.04;
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i] = [0.05 + (t * 1.7 + s).sin() * 0.06, 0.0, s * 0.03];
                b.fore[i] = [-0.1, 0.0, 0.0];
            }
        }
        Anim::Death | Anim::Dead => {
            // recoil, the knees buckle, then down onto the back
            let u = if a == Anim::Dead { 1.0 } else { u };
            b.pelvis[0] = track(u, &[(0.0, 0.0), (0.6, 0.0), (0.88, -FRAC_PI_2)]);
            // the knees give: the hips drop as far as the bend takes them
            let knees = |c: f32| -(2.0 * LEG * (1.0 - c.cos()) - 4.0 * c.sin());
            let buckle = knees(track(u, &[(0.25, 0.0), (0.6, 0.9)]));
            b.hip = Vec3::new(
                0.0,
                if u <= 0.6 {
                    buckle
                } else {
                    track(
                        u,
                        &[
                            (0.6, knees(0.9)),
                            (0.88, -16.0),
                            (0.94, -15.0),
                            (1.0, -16.0),
                        ],
                    )
                },
                track(u, &[(0.0, 0.0), (0.25, -1.0), (0.6, -3.0), (0.88, -13.0)]),
            );
            for (i, s) in SIDES.into_iter().enumerate() {
                b.thigh[i] = [
                    track(
                        u,
                        &[
                            (0.0, 0.0),
                            (0.25, 0.0),
                            (0.6, -0.9),
                            (0.88, -0.1),
                            (1.0, -0.06),
                        ],
                    ),
                    0.0,
                    s * track(u, &[(0.6, 0.0), (1.0, 0.12)]),
                ];
                b.shin[i] = track(u, &[(0.0, 0.0), (0.6, 1.8), (0.88, 0.2), (1.0, 0.12)]);
                b.arm[i] = [
                    track(
                        u,
                        &[
                            (0.0, 0.0),
                            (0.25, 0.2),
                            (0.6, -0.4),
                            (0.88, -0.25),
                            (1.0, -0.1),
                        ],
                    ),
                    0.0,
                    s * track(
                        u,
                        &[
                            (0.0, 0.06),
                            (0.25, 0.6),
                            (0.6, 0.3),
                            (0.88, 1.15),
                            (1.0, 1.25),
                        ],
                    ),
                ];
                b.fore[i] = [
                    track(u, &[(0.0, -0.12), (0.25, -0.5), (0.88, -0.3), (1.0, -0.2)]),
                    0.0,
                    0.0,
                ];
            }
            b.thigh[1][0] -= track(u, &[(0.6, 0.0), (1.0, 0.25)]);
            b.shin[1] += track(u, &[(0.6, 0.0), (1.0, 0.5)]);
            b.spine[0] = track(
                u,
                &[
                    (0.0, 0.0),
                    (0.25, -0.35),
                    (0.6, 0.15),
                    (0.88, -0.05),
                    (1.0, 0.0),
                ],
            );
            b.head = [
                track(
                    u,
                    &[
                        (0.0, 0.0),
                        (0.25, -0.4),
                        (0.6, 0.2),
                        (0.88, -0.15),
                        (1.0, -0.05),
                    ],
                ),
                track(u, &[(0.6, 0.0), (1.0, 0.45)]),
                0.0,
            ];
            b.grip = [track(u, &[(0.6, 0.0), (1.0, -1.1)]), 0.0, 0.0];
            b.cape = track(u, &[(0.0, 0.0), (0.25, 0.3), (0.6, 0.7), (0.88, -0.2)]);
            b.flow = 0.0;
        }
        Anim::Sit => {
            b.hip = Vec3::new(0.0, -17.0 + breath * 0.15, -4.0);
            for (i, s) in SIDES.into_iter().enumerate() {
                b.thigh[i] = [-1.9, 0.0, s * 0.2];
                b.shin[i] = 2.35;
                b.arm[i] = [-0.75 + breath * 0.02, 0.0, -s * 0.05];
                b.fore[i] = [-0.55, 0.0, 0.0];
            }
            b.spine[0] = 0.18 + breath * 0.015;
            b.head[0] = -0.12;
            b.grip[0] = -1.1;
            b.cape = 0.85;
            if rig.robe {
                b.tilt(-1.45);
            }
        }
        Anim::Kneel => {
            b.hip = Vec3::new(0.0, -7.5, -1.0);
            b.thigh[0] = [-1.2, 0.0, 0.08];
            b.shin[0] = 1.2;
            b.thigh[1] = [0.12, 0.0, -0.06];
            b.shin[1] = 1.45;
            b.spine[0] = 0.45 + breath * 0.015;
            b.head[0] = 0.2;
            let rummage = (t * 3.0).sin();
            b.arm[0] = [-0.85 + rummage * 0.15, 0.0, -0.05];
            b.fore[0] = [-0.35 - rummage * 0.15, 0.0, 0.0];
            b.arm[1] = [-0.75, 0.0, 0.25];
            b.fore[1] = [-0.9, 0.0, 0.0];
            b.grip[0] = -1.1;
            // the cape hangs straight down from the stooped shoulders
            b.cape = -0.25;
            if rig.robe {
                b.tilt(-0.8);
            }
        }
        Anim::Talk => {
            let g = if armed { 1 } else { 0 };
            let s = SIDES[g];
            b.arm[g] = [-0.45 + (t * 2.1).sin() * 0.2, 0.0, s * 0.12];
            b.fore[g] = [-1.3 + (t * 3.3).sin() * 0.25, 0.0, 0.0];
            if !armed {
                b.arm[1][0] = -((t * 1.3).sin().max(0.0)) * 0.4;
                b.fore[1][0] = -0.2 - (t * 1.3).sin().max(0.0) * 0.8;
            }
            b.head = [(t * 3.7).sin() * 0.06, (t * 0.9).sin() * 0.15, 0.0];
            b.spine[1] = (t * 1.1).sin() * 0.06;
            b.hip.y = 0.15 * breath;
        }
        Anim::Wave => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (0.85, 1.0), (1.0, 0.0)]);
            let g = if armed { 1 } else { 0 };
            let s = SIDES[g];
            b.arm[g] = [-0.2 * env, 0.0, s * (0.06 + 2.45 * env)];
            b.fore[g] = [-0.3 * env, 0.0, s * (t * 10.0).sin() * 0.45 * env];
            b.head[2] = -s * 0.1 * env;
            b.spine[2] = -s * 0.05 * env;
        }
        Anim::Bow => {
            let env = track(u, &[(0.0, 0.0), (0.3, 1.0), (0.7, 1.0), (1.0, 0.0)]);
            b.spine[0] = 0.85 * env;
            b.head[0] = 0.25 * env;
            b.hip.z = -1.5 * env;
            b.arm[0] = [b.arm[0][0] * (1.0 - env) - 0.7 * env, 0.0, -0.55 * env];
            b.fore[0][0] = b.fore[0][0] * (1.0 - env) - 1.7 * env;
            b.arm[1] = [0.45 * env, 0.0, -0.06 + 0.35 * env];
            b.fore[1][0] = -0.14 - 1.2 * env;
            b.cape = -0.5 * env;
        }
        Anim::Cheer => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (0.85, 1.0), (1.0, 0.0)]);
            let pump = (t * 9.0).sin();
            for (i, s) in SIDES.into_iter().enumerate() {
                b.arm[i] = [(-2.85 + pump * 0.2) * env, 0.0, s * (0.06 + 0.25 * env)];
                b.fore[i][0] = (-0.5 + pump * 0.4) * env - 0.14;
            }
            b.hip.y = (t * TAU / 0.6).sin().max(0.0) * 2.5 * env;
            b.head[0] = -0.3 * env;
            b.flow = 0.3 * env;
        }
        Anim::Point => {
            let env = track(u, &[(0.0, 0.0), (0.2, 1.0), (0.85, 1.0), (1.0, 0.0)]);
            b.arm[0] = [b.arm[0][0] * (1.0 - env) - 1.55 * env, 0.0, 0.05];
            b.fore[0][0] *= 1.0 - env;
            b.grip[0] = 0.4 * env;
            b.arm[1] = [0.1 * env, 0.0, -0.06 - 0.45 * env];
            b.fore[1][0] = -0.14 - 1.4 * env;
            b.spine[1] = -0.15 * env;
            b.head[0] = -0.05 * env;
        }
    }
    b.finish()
}

// ----------------------------------------------------------------- wraith

/// The pose of a legless wraith. Its body hovers; `off` of the body is the
/// height above its hover.
pub fn wraith(a: Anim, at: &At) -> Pose {
    use wraith::*;
    let t = at.t;
    let u = progress(a, t);
    let mut p = Pose::REST;
    let bob = (t * 2.1 + at.seed * TAU).sin() * 2.2;
    // body lean, hood nod, arm reach and bend, flow, eyes, lift
    let mut lean = (t * 1.3).sin() * 0.03;
    let mut nod = (t * 1.7).sin() * 0.04;
    let mut arm = [
        [-0.75 - (t * 2.4).sin() * 0.12, 0.0, 0.12],
        [-0.75 - (t * 2.4 - 1.0).sin() * 0.12, 0.0, 0.12],
    ];
    let mut fore = [
        -0.55 - (t * 2.0 + 2.0).sin() * 0.15,
        -0.55 - (t * 2.0 - 2.0).sin() * 0.15,
    ];
    let mut flow = 0.12;
    let mut eyes = if (t * 0.37 + at.seed).fract() < 0.03 {
        0.15
    } else {
        1.0
    };
    let mut lift = Vec3::new(0.0, bob, 0.0);
    let mut turn = 0.0;
    match a {
        Anim::Idle => {
            let (env, kind) = fidget(at);
            if kind == 0 {
                nod += 0.2 * env;
                turn = (t * 1.5).sin() * 0.5 * env;
            } else {
                arm[0][0] -= 0.5 * env;
                fore[0] -= 0.5 * env;
            }
        }
        Anim::Walk | Anim::Backpedal | Anim::StrafeLeft | Anim::StrafeRight | Anim::Swim => {
            let k = match a {
                Anim::Backpedal => -0.4,
                _ => 1.0,
            };
            lean = 0.22 * k;
            flow = 0.55;
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i][0] = -1.15 - (at.stride + s).sin() * 0.15;
            }
            if a == Anim::StrafeLeft || a == Anim::StrafeRight {
                let d = if a == Anim::StrafeRight { 1.0 } else { -1.0 };
                p.rot[BODY] = Quat::from_rotation_z(-d * 0.18);
            }
        }
        Anim::Run | Anim::JumpAir | Anim::Fall | Anim::JumpStart | Anim::JumpLand => {
            lean = 0.38;
            flow = 1.0;
            nod = -0.2;
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i] = [0.55 + (t * 3.0 + s).sin() * 0.1, 0.0, 0.2];
                fore[i] = -0.3;
            }
            if a == Anim::Fall {
                lift.y -= 3.0;
            }
            if a == Anim::JumpLand {
                lift.y -= 2.0 * (1.0 - u);
            }
        }
        Anim::Ready => {
            lean = 0.15;
            nod = 0.1;
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i] = [-1.3 + (t * 3.0 + s).sin() * 0.08, 0.0, 0.45];
                fore[i] = -1.0;
            }
            flow = 0.2;
        }
        Anim::Attack1H | Anim::Attack2H | Anim::AttackUnarmed | Anim::Shoot => {
            // a double rake: claws up, then down through the target
            let raise = track(u, &[(0.0, -1.0), (0.4, -2.8), (0.6, -0.3), (1.0, -1.0)]);
            for (i, _) in SIDES.into_iter().enumerate() {
                arm[i] = [
                    raise,
                    0.0,
                    track(u, &[(0.0, 0.3), (0.4, 0.55), (0.6, 0.0), (1.0, 0.3)]),
                ];
                fore[i] = track(u, &[(0.0, -0.8), (0.4, -1.2), (0.6, -0.1), (1.0, -0.8)]);
            }
            lean = track(u, &[(0.0, 0.1), (0.4, -0.15), (0.6, 0.4), (1.0, 0.1)]);
            nod = track(u, &[(0.0, 0.0), (0.4, -0.2), (0.6, 0.25), (1.0, 0.0)]);
            lift.z = track(u, &[(0.0, 0.0), (0.4, -2.0), (0.6, 5.0), (1.0, 0.0)]);
            flow = 0.3 + 0.4 * hump(u, 0.4, 0.5);
        }
        Anim::Special => {
            // a scream: arms flung wide, hood thrown back, rising
            let env = track(u, &[(0.0, 0.0), (0.3, 1.0), (0.8, 1.0), (1.0, 0.0)]);
            for (i, _) in SIDES.into_iter().enumerate() {
                arm[i] = [-0.4 * env + arm[i][0] * (1.0 - env), 0.0, 0.12 + 1.15 * env];
                fore[i] = -0.2 * env + fore[i] * (1.0 - env);
            }
            nod = -0.45 * env;
            lean = -0.2 * env;
            lift.y += 4.0 * env;
            eyes = 1.0;
            flow = 0.12 + 0.5 * env;
        }
        Anim::CastDirected | Anim::CastOmni | Anim::Channel => {
            let up = a == Anim::CastOmni;
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i] = if up {
                    [-2.4 + (t * 3.0 + s).sin() * 0.1, 0.0, 0.45]
                } else {
                    [-1.45 + (t * 6.0 + s).sin() * 0.05, 0.0, -0.25]
                };
                fore[i] = if up { -0.3 } else { -0.4 };
            }
            nod = if up { -0.3 } else { 0.1 };
            flow = 0.35 + (t * 4.0).sin() * 0.1;
        }
        Anim::CastRelease => {
            let reach = track(u, &[(0.0, -1.2), (0.25, -1.6), (1.0, -0.9)]);
            for (i, _) in SIDES.into_iter().enumerate() {
                arm[i] = [reach, 0.0, -0.1];
                fore[i] = track(u, &[(0.0, -1.0), (0.25, 0.0), (1.0, -0.5)]);
            }
            lift.z = track(u, &[(0.0, 0.0), (0.25, 2.5), (1.0, 0.0)]);
            lean = 0.2;
        }
        Anim::HitReact => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (1.0, 0.0)]);
            lean -= 0.35 * env;
            nod -= 0.3 * env;
            lift.z -= 3.0 * env;
            for a in arm.iter_mut() {
                a[2] += 0.5 * env;
            }
            eyes = 1.0 - 0.7 * env;
        }
        Anim::Stun => {
            lean = 0.3 + (t * 1.2).sin() * 0.06;
            nod = 0.45;
            turn = (t * 1.9).sin() * 0.25;
            lift.y = -2.5 + bob * 0.3;
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i] = [0.1 + (t * 1.4 + s).sin() * 0.08, 0.0, 0.08];
                fore[i] = -0.15;
            }
            eyes = 0.4;
        }
        Anim::Death | Anim::Dead => {
            // flung up, then the empty robe crumples to the ground
            let u = if a == Anim::Dead { 1.0 } else { u };
            lift = Vec3::new(
                0.0,
                track(
                    u,
                    &[
                        (0.0, bob),
                        (0.25, 4.0),
                        (0.85, -13.0),
                        (0.93, -12.0),
                        (1.0, -13.0),
                    ],
                ),
                track(u, &[(0.25, 0.0), (0.9, -8.0)]),
            );
            lean = track(u, &[(0.0, 0.0), (0.25, -0.35), (0.9, -1.45)]);
            nod = track(u, &[(0.0, 0.0), (0.25, -0.5), (0.9, 0.3)]);
            turn = track(u, &[(0.5, 0.0), (1.0, 0.45)]);
            for (i, _) in SIDES.into_iter().enumerate() {
                arm[i] = [
                    track(u, &[(0.0, arm[i][0]), (0.25, -2.7), (0.9, -0.2)]),
                    0.0,
                    track(u, &[(0.0, 0.12), (0.25, 0.4), (0.9, 1.3)]),
                ];
                fore[i] = track(u, &[(0.0, fore[i]), (0.25, -0.2), (0.9, -0.5)]);
            }
            flow = track(u, &[(0.0, 0.2), (0.25, 0.9), (0.9, 0.0)]);
            eyes = track(u, &[(0.0, 1.0), (0.3, 1.0), (0.9, 0.12)]);
        }
        Anim::Sit | Anim::Kneel => {
            lift.y = -4.0 + bob * 0.3;
            nod = 0.25;
            lean = 0.15;
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i] = [-1.0 + (t * 1.3 + s).sin() * 0.05, 0.0, -0.35];
                fore[i] = -1.4;
            }
            flow = 0.0;
        }
        Anim::Talk => {
            nod = (t * 3.7).sin() * 0.1;
            arm[0] = [-1.1 + (t * 2.1).sin() * 0.25, 0.0, 0.2];
            fore[0] = -1.0 + (t * 3.1).sin() * 0.3;
        }
        Anim::Wave | Anim::Point => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (0.85, 1.0), (1.0, 0.0)]);
            if a == Anim::Wave {
                arm[0] = [-0.3 * env + arm[0][0] * (1.0 - env), 0.0, 0.12 + 2.3 * env];
                fore[0] = -0.3 * env + (t * 10.0).sin() * 0.4 * env;
            } else {
                arm[0] = [-1.6 * env + arm[0][0] * (1.0 - env), 0.0, 0.05];
                fore[0] *= 1.0 - env;
            }
        }
        Anim::Bow => {
            let env = track(u, &[(0.0, 0.0), (0.3, 1.0), (0.7, 1.0), (1.0, 0.0)]);
            lean = 0.7 * env;
            nod = 0.3 * env;
            lift.y -= 2.0 * env;
        }
        Anim::Cheer => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (0.85, 1.0), (1.0, 0.0)]);
            for (i, s) in SIDES.into_iter().enumerate() {
                arm[i] = [
                    (-2.7 + (t * 9.0 + s).sin() * 0.2) * env + arm[i][0] * (1.0 - env),
                    0.0,
                    0.3,
                ];
            }
            lift.y += (t * TAU / 0.6).sin().max(0.0) * 3.0 * env;
            nod = -0.3 * env;
        }
    }
    p.off[BODY] = lift;
    p.rot[BODY] = Quat::from_rotation_y(turn * 0.3) * Quat::from_rotation_x(lean) * p.rot[BODY];
    p.rot[HOOD] = euler(nod + lean * 0.4, turn, 0.0);
    for (i, s) in SIDES.into_iter().enumerate() {
        p.rot[ARM_U[i]] = euler(arm[i][0], arm[i][1] * s, arm[i][2] * s);
        p.rot[ARM_L[i]] = Quat::from_rotation_x(fore[i]);
    }
    p.k[key::FLOW] = flow;
    p.k[key::SHAPE] = eyes;
    p
}

// -------------------------------------------------------------- quadruped

/// The parts of a beast that change how it moves.
#[derive(Clone, Copy, Debug)]
pub struct QuadRig {
    /// dips its head to graze when idle (else roots and tosses it)
    pub grazes: bool,
    /// how high its hips are, in voxels
    pub hip_y: f32,
    /// half its body's width, in voxels: how high it lies on its side
    pub half_width: f32,
    /// half the distance from front to back legs, in voxels
    pub half_length: f32,
}

/// The pose of a four-legged beast. The body turns about the middle of its
/// back at hip height.
pub fn quadruped(a: Anim, at: &At, rig: &QuadRig) -> Pose {
    use quad::*;
    let t = at.t;
    let ph = at.stride;
    let u = progress(a, t);
    let mut p = Pose::REST;
    // body pitch (+ head down), roll, yaw and offset; head pitch and turn;
    // leg swing (+ back) and splay (+ out)
    let mut pitch = 0.0f32;
    let mut roll = 0.0f32;
    let mut yaw = 0.0f32;
    let mut off = Vec3::new(0.0, (t * 1.8 + at.seed * TAU).sin() * 0.25, 0.0);
    let mut head = [0.0f32; 2];
    let mut legs = [[0.0f32; 2]; 4];
    let mut flow = 0.0;
    let diag = |i: usize| if i == 0 || i == 3 { 0.0 } else { PI };
    let front = |i: usize| i < 2;
    let side = |i: usize| if i.is_multiple_of(2) { 1.0 } else { -1.0 };
    // the body pitched by `a` about its middle: keep the hind feet down
    let rear_up = |a: f32| rig.half_length * a.sin();
    match a {
        Anim::Idle => {
            let (env, kind) = fidget(at);
            if rig.grazes {
                let graze = ((t * 0.7 + at.seed * 5.0).sin() - 0.55).max(0.0) * 1.6;
                let chew = if graze > 0.3 {
                    (t * 9.0).sin() * 0.04
                } else {
                    0.0
                };
                head[0] = graze * 1.25 + chew;
            } else {
                head[0] = 0.12 + (t * 1.9 + at.seed).sin() * 0.07;
            }
            match kind {
                0 => head[1] = (t * 1.8).sin() * 0.55 * env,
                1 => legs[0][0] = -((t * 5.0).sin().max(0.0)) * 0.35 * env,
                _ => {
                    head[0] -= 0.35 * env;
                    roll = (t * 14.0).sin() * 0.05 * env;
                }
            }
        }
        Anim::Walk | Anim::Backpedal | Anim::StrafeLeft | Anim::StrafeRight => {
            let ph = if a == Anim::Backpedal { -ph } else { ph };
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = (ph + diag(i)).sin() * 0.5;
            }
            off.y = (ph * 2.0).sin().abs() * 0.6;
            pitch = (ph * 2.0).cos() * 0.02;
            head[0] = ph.sin() * 0.06;
            yaw = match a {
                Anim::StrafeLeft => -0.35,
                Anim::StrafeRight => 0.35,
                _ => 0.0,
            };
            flow = 0.4;
        }
        Anim::Run => {
            // a gallop: the fore pair, then the hind pair, the back rocking
            for (i, l) in legs.iter_mut().enumerate() {
                let lag = if front(i) { 0.0 } else { 2.3 } + if side(i) > 0.0 { 0.0 } else { 0.35 };
                l[0] = (ph + lag).sin() * 0.85;
            }
            off.y = (ph + 1.1).sin().max(0.0) * 2.4;
            pitch = ph.cos() * 0.1;
            head[0] = 0.18 + ph.cos() * 0.12;
            flow = 1.0;
        }
        Anim::JumpStart | Anim::JumpLand => {
            off.y = -2.5 * hump(u, 0.4, 0.6);
            pitch = -0.1 * hump(u, 0.4, 0.6);
        }
        Anim::JumpAir | Anim::Fall => {
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) { -0.75 } else { 0.7 };
                if a == Anim::Fall {
                    l[1] = 0.35 + (t * 9.0 + i as f32).sin() * 0.15;
                }
            }
            head[0] = -0.3;
            flow = 0.8;
        }
        Anim::Swim => {
            off.y = -rig.hip_y * 0.3;
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = (t * 8.0 + diag(i)).sin() * 0.6;
            }
            head[0] = -0.45;
        }
        Anim::Ready => {
            pitch = 0.08;
            off.y = -1.2;
            head[0] = 0.35 + (t * 3.0).sin() * 0.05;
            // pawing the ground, and a snort now and then
            let paw = (t * 5.0).sin().max(0.0) * (t * 0.9 + at.seed).sin().max(0.0);
            legs[0][0] = -paw * 0.55;
            legs[0][1] = 0.1;
            legs[1][1] = 0.1;
            roll = (t * 2.0).sin() * 0.02;
        }
        Anim::Attack1H | Anim::Attack2H | Anim::AttackUnarmed | Anim::Shoot => {
            // rear back, lunge, gore and toss the head
            pitch = track(u, &[(0.0, 0.0), (0.3, -0.22), (0.5, 0.15), (1.0, 0.0)]);
            off = Vec3::new(
                0.0,
                track(u, &[(0.0, 0.0), (0.3, 1.0), (0.5, -0.5), (1.0, 0.0)])
                    + rear_up(-pitch).max(0.0),
                track(u, &[(0.0, 0.0), (0.3, -2.5), (0.5, 4.5), (1.0, 0.0)]),
            );
            head[0] = track(
                u,
                &[
                    (0.0, 0.0),
                    (0.3, -0.3),
                    (0.5, 0.6),
                    (0.65, -0.4),
                    (1.0, 0.0),
                ],
            );
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) {
                    track(u, &[(0.0, 0.0), (0.3, -0.45), (0.5, 0.25), (1.0, 0.0)])
                } else {
                    track(u, &[(0.0, 0.0), (0.3, 0.15), (0.5, -0.25), (1.0, 0.0)])
                } - pitch;
            }
        }
        Anim::Special | Anim::CastDirected | Anim::CastOmni | Anim::Channel => {
            // rears up on its hind legs, forelegs kicking, head high
            let env = if a == Anim::Special {
                track(u, &[(0.0, 0.0), (0.35, 1.0), (0.75, 1.0), (1.0, 0.0)])
            } else {
                0.75 + (t * 2.0).sin() * 0.05
            };
            pitch = -0.75 * env;
            off.y = rear_up(0.75 * env);
            off.z = -rig.half_length * (1.0 - (0.75 * env).cos());
            head[0] = -0.45 * env;
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) {
                    (-0.6 + (t * 12.0 + i as f32 * 1.7).sin() * 0.4) * env
                } else {
                    0.75 * env
                };
            }
        }
        Anim::CastRelease => {
            head[0] = track(u, &[(0.0, -0.4), (0.3, 0.3), (1.0, 0.0)]);
            off.z = track(u, &[(0.0, 0.0), (0.3, 3.0), (1.0, 0.0)]);
        }
        Anim::HitReact => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (1.0, 0.0)]);
            off.z = -2.5 * env;
            off.y = -env;
            pitch = -0.12 * env;
            head[0] = -0.4 * env;
            roll = 0.08 * env * if at.seed > 0.5 { 1.0 } else { -1.0 };
            for l in legs.iter_mut() {
                l[1] = 0.2 * env;
            }
        }
        Anim::Stun => {
            head[0] = 0.5 + (t * 1.9).sin() * 0.12;
            head[1] = (t * 1.3).sin() * 0.25;
            roll = (t * 1.6).sin() * 0.07;
            off.y = -1.5;
            for (i, l) in legs.iter_mut().enumerate() {
                l[1] = 0.18 + (t * 1.6 + i as f32).sin() * 0.05;
            }
        }
        Anim::Death | Anim::Dead => {
            // the forelegs buckle, then it rolls onto its side, legs stiff
            let u = if a == Anim::Dead { 1.0 } else { u };
            let down = rig.half_width - rig.hip_y + 1.5;
            pitch = track(u, &[(0.0, 0.0), (0.3, 0.15), (0.85, 0.05)]);
            roll = track(u, &[(0.0, 0.0), (0.3, 0.1), (0.85, FRAC_PI_2)]);
            off.y = track(
                u,
                &[
                    (0.0, 0.0),
                    (0.3, -2.0),
                    (0.6, 1.5),
                    (0.85, down),
                    (0.92, down + 1.2),
                    (1.0, down),
                ],
            );
            head[0] = track(u, &[(0.0, 0.0), (0.3, 0.2), (0.85, -0.3)]);
            head[1] = track(u, &[(0.3, 0.0), (1.0, -0.3)]);
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) {
                    track(u, &[(0.0, 0.0), (0.3, 0.9), (0.85, -0.35)])
                } else {
                    track(u, &[(0.0, 0.0), (0.3, -0.2), (0.85, 0.35)])
                };
                l[1] = track(u, &[(0.3, 0.0), (0.85, -0.15 * side(i))]);
            }
        }
        Anim::Sit => {
            // haunches down, forelegs straight
            pitch = -0.45;
            off.y = rear_up(0.45) - 6.0;
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) { 0.45 } else { -1.1 };
            }
            head[0] = -0.2 + (t * 1.1).sin() * 0.05;
        }
        Anim::Kneel => {
            // lying down, legs folded beneath
            off.y = 7.0 - rig.hip_y + (t * 1.8).sin() * 0.2;
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) { 1.45 } else { -1.45 };
            }
            head[0] = 0.15 + (t * 0.6).sin() * 0.05;
            head[1] = (t * 0.4 + at.seed).sin() * 0.3;
        }
        Anim::Talk => {
            head[0] = -0.2 + (t * 6.0).sin() * 0.12;
            head[1] = (t * 1.1).sin() * 0.2;
        }
        Anim::Wave => {
            let env = hump(u, 0.15, 0.15);
            head[0] = (t * 8.0).sin() * 0.4 * env - 0.2 * env;
            head[1] = (t * 5.0).sin() * 0.3 * env;
        }
        Anim::Bow => {
            // the play bow: forequarters down, tail end up
            let env = hump(u, 0.3, 0.3);
            pitch = 0.2 * env;
            off.y = -env;
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) { -0.8 * env } else { 0.2 * env };
            }
        }
        Anim::Cheer => {
            let env = hump(u, 0.15, 0.15);
            off.y = (t * TAU / 0.6).sin().max(0.0) * 4.0 * env;
            for (i, l) in legs.iter_mut().enumerate() {
                l[0] = if front(i) { -0.4 } else { 0.4 } * env;
            }
            head[0] = -0.3 * env;
        }
        Anim::Point => {
            // alert: head up and still, a foreleg raised
            let env = hump(u, 0.2, 0.15);
            head[0] = -0.4 * env;
            legs[0][0] = -0.6 * env;
        }
    }
    p.off[BODY] = off;
    p.rot[BODY] = euler(pitch, yaw, roll);
    p.rot[HEAD] = euler(head[0], head[1], 0.0);
    for (i, l) in legs.iter().enumerate() {
        p.rot[LEGS[i]] = Quat::from_rotation_x(l[0]) * Quat::from_rotation_z(l[1] * side(i));
    }
    p.k[key::FLOW] = flow;
    p
}

// --------------------------------------------------------------- multileg

/// A crawler's leg: which side it is on (+1 right), where in the gait it
/// steps, and whether it is a front leg (raised to strike).
#[derive(Clone, Copy, Debug)]
pub struct LegRig {
    pub side: f32,
    pub phase: f32,
    pub front: bool,
}

/// The parts of a crawler that change how it moves.
#[derive(Clone, Debug)]
pub struct MultiRig {
    pub legs: Vec<LegRig>,
    pub claws: usize,
    /// legs reach fore and aft, its face is to its right and it travels
    /// sideways (a crab); it turns to face its foe when it stops
    pub sideways: bool,
    /// height of its shell when upside down, in voxels
    pub height: f32,
}

/// The pose of a crawler. Each leg gets a lift (+ tip up) and a swing (+ tip
/// forward); claws a raise and a pinch.
pub fn multileg(a: Anim, at: &At, rig: &MultiRig) -> Pose {
    use multi::*;
    let t = at.t;
    let ph = at.stride;
    let u = progress(a, t);
    let mut p = Pose::REST;
    let mut off = Vec3::new(0.0, (t * 1.6 + at.seed * TAU).sin() * 0.35, 0.0);
    let (mut pitch, mut roll) = (0.0f32, 0.0f32);
    // a crab turns its face (+x) to the front when it is not scuttling
    let mut face = if rig.sideways { 1.0f32 } else { 0.0 };
    let n = rig.legs.len().min(MAX_LEGS);
    let mut lift = vec![0.0f32; n];
    let mut swing = vec![0.0f32; n];
    let mut claw = [
        0.12f32 + (t * 1.7).sin() * 0.14,
        0.12 + (t * 1.7 + 2.0).sin() * 0.14,
    ];
    let mut pinch = [0.0f32; 2];
    let twitch = |i: usize, l: &LegRig| (t * 1.3 + l.phase + i as f32 * 1.7).sin() * 0.03;
    let gait = |lift: &mut [f32], swing: &mut [f32], ph: f32, amount: f32| {
        for (i, l) in rig.legs.iter().take(n).enumerate() {
            let (s, c) = (ph + l.phase).sin_cos();
            lift[i] = c.max(0.0) * amount;
            swing[i] = s * amount;
        }
    };
    match a {
        Anim::Idle => {
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                lift[i] = twitch(i, l);
            }
            let (env, kind) = fidget(at);
            match kind {
                0 => face *= 1.0 - 0.3 * env,
                1 => {
                    claw[0] += 0.6 * env;
                    pinch[0] = (t * 12.0).sin() * 0.2 * env;
                    if rig.claws == 0 {
                        for (i, l) in rig.legs.iter().take(n).enumerate() {
                            if l.front {
                                lift[i] += 0.5 * env * (t * 6.0).sin().max(0.0);
                            }
                        }
                    }
                }
                _ => off.y -= 1.0 * env,
            }
        }
        Anim::Walk | Anim::Backpedal | Anim::StrafeLeft | Anim::StrafeRight => {
            let ph = if a == Anim::Backpedal { -ph } else { ph };
            gait(&mut lift, &mut swing, ph, 1.0);
            off.y += (ph * 2.0).sin() * 0.5;
            face = 0.0;
            claw = [claw[0] + 0.2, claw[1] + 0.2];
        }
        Anim::Run => {
            gait(&mut lift, &mut swing, ph, 1.35);
            off.y += (ph * 2.0).sin() * 0.6 - 1.0;
            face = 0.0;
            claw = [0.5, 0.5];
            pitch = 0.05;
        }
        Anim::JumpStart | Anim::JumpLand => {
            off.y -= 2.0 * hump(u, 0.4, 0.6);
            for l in lift.iter_mut() {
                *l = -0.3 * hump(u, 0.4, 0.6);
            }
        }
        Anim::JumpAir | Anim::Fall | Anim::Swim => {
            for (i, l) in lift.iter_mut().enumerate() {
                *l = 0.5 + (t * 10.0 + i as f32).sin() * 0.25;
            }
            claw = [0.7, 0.7];
            face = 0.0;
        }
        Anim::Ready => {
            off.y -= 1.0;
            claw = [
                0.75 + (t * 3.0).sin() * 0.1,
                0.75 + (t * 3.0 + 1.0).sin() * 0.1,
            ];
            pinch = [
                (t * 7.0).sin().max(0.0) * 0.25,
                (t * 7.0 + 2.0).sin().max(0.0) * 0.25,
            ];
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                lift[i] = twitch(i, l) - 0.15;
                if l.front && rig.claws == 0 {
                    lift[i] = 0.45 + (t * 4.0 + l.side).sin() * 0.12;
                }
            }
            pitch = if rig.claws == 0 { -0.12 } else { 0.0 };
        }
        Anim::Attack1H | Anim::Attack2H | Anim::AttackUnarmed | Anim::Shoot => {
            // claws up and snapping down; a spider rears and bites
            let raise = track(u, &[(0.0, 0.3), (0.35, 1.0), (0.55, -0.35), (1.0, 0.3)]);
            claw = [raise, raise];
            pinch = [track(u, &[(0.35, 0.3), (0.55, -0.15), (1.0, 0.0)]); 2];
            pitch = track(u, &[(0.0, 0.0), (0.35, -0.4), (0.55, 0.18), (1.0, 0.0)]);
            off.z = track(u, &[(0.0, 0.0), (0.35, -2.0), (0.55, 4.0), (1.0, 0.0)]);
            off.y += track(u, &[(0.0, 0.0), (0.35, 2.5), (0.55, -0.5), (1.0, 0.0)]);
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                lift[i] = if l.front {
                    track(u, &[(0.0, 0.0), (0.35, 1.9), (0.55, -0.2), (1.0, 0.0)])
                } else {
                    -pitch * 0.5
                };
            }
        }
        Anim::Special | Anim::CastDirected | Anim::CastOmni | Anim::Channel => {
            // a threat display: claws high and clacking; a spider lifts its
            // spinnerets and spits silk
            let env = if a == Anim::Special {
                track(u, &[(0.0, 0.0), (0.3, 1.0), (0.8, 1.0), (1.0, 0.0)])
            } else {
                1.0
            };
            claw = [1.15 * env + 0.1, 1.15 * env + 0.1];
            pinch = [
                (t * 14.0).sin() * 0.2 * env,
                (t * 14.0 + 1.0).sin() * 0.2 * env,
            ];
            pitch = if rig.claws == 0 {
                0.4 * env
            } else {
                -0.1 * env
            };
            off.y += 2.0 * env;
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                lift[i] = if l.front { 0.6 * env } else { -0.25 * env };
            }
        }
        Anim::CastRelease => {
            off.z = track(u, &[(0.0, 0.0), (0.3, 3.0), (1.0, 0.0)]);
            claw = [track(u, &[(0.0, 1.0), (0.3, 0.0), (1.0, 0.3)]); 2];
        }
        Anim::HitReact => {
            let env = track(u, &[(0.0, 0.0), (0.15, 1.0), (1.0, 0.0)]);
            off.y -= 1.8 * env;
            off.z -= 2.0 * env;
            pitch = -0.12 * env;
            for l in lift.iter_mut() {
                *l = -0.25 * env;
            }
            claw = [claw[0] + 0.6 * env, claw[1] + 0.6 * env];
        }
        Anim::Stun => {
            roll = (t * 2.5).sin() * 0.1;
            pitch = (t * 1.7).sin() * 0.06;
            off.y -= 1.5;
            for (i, l) in lift.iter_mut().enumerate() {
                *l = -0.2 + (t * 6.0 + i as f32 * 2.0).sin() * 0.12;
            }
            claw = [-0.1, -0.1];
        }
        Anim::Death | Anim::Dead => {
            // flipped onto its back, legs curled up into the air
            let u = if a == Anim::Dead { 1.0 } else { u };
            roll = track(u, &[(0.0, 0.0), (0.25, -0.2), (0.75, PI)]);
            let over = rig.height.max(16.0);
            off.y = track(
                u,
                &[
                    (0.0, 0.0),
                    (0.25, 6.0),
                    (0.5, over),
                    (0.75, rig.height),
                    (0.85, rig.height + 1.5),
                    (1.0, rig.height),
                ],
            );
            let k = if rig.sideways { 1.0 } else { 2.8 };
            let curl = k * track(u, &[(0.0, 0.0), (0.25, 0.3), (0.45, -0.9), (1.0, -1.05)]);
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                lift[i] = curl + (i as f32 * 1.3).sin() * 0.08;
                swing[i] = track(u, &[(0.5, 0.0), (1.0, if l.front { 0.25 } else { -0.15 })]);
            }
            claw = [track(u, &[(0.0, 0.3), (0.75, -0.6)]); 2];
            face *= 1.0 - u * 0.5;
        }
        Anim::Sit | Anim::Kneel => {
            // hunkered down, legs splayed flat
            off.y -= 1.5;
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                lift[i] = 0.6 + twitch(i, l);
            }
            claw = [-0.2, -0.2];
        }
        Anim::Talk => {
            claw = [
                0.4 + (t * 4.0).sin() * 0.25,
                0.4 + (t * 4.0 + 2.0).sin() * 0.25,
            ];
            pinch = [(t * 9.0).sin() * 0.15; 2];
            off.y += (t * 4.0).sin() * 0.3;
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                if l.front && rig.claws == 0 {
                    lift[i] = 0.3 + (t * 4.0 + l.side).sin() * 0.2;
                }
            }
        }
        Anim::Wave | Anim::Point => {
            // one claw (or front leg) up, waving or held out
            let env = hump(u, 0.15, 0.15);
            let wave = if a == Anim::Wave {
                (t * 9.0).sin() * 0.35
            } else {
                0.0
            };
            claw[0] = claw[0] * (1.0 - env) + (1.3 + wave) * env;
            for (i, l) in rig.legs.iter().take(n).enumerate() {
                if l.front && l.side > 0.0 {
                    lift[i] = (1.6 + wave) * env;
                }
            }
        }
        Anim::Bow => {
            let env = hump(u, 0.3, 0.3);
            pitch = 0.15 * env;
            off.y -= 1.0 * env;
            claw = [-0.3 * env, -0.3 * env];
        }
        Anim::Cheer => {
            let env = hump(u, 0.15, 0.15);
            off.y += (t * TAU / 0.5).sin().max(0.0) * 3.0 * env;
            claw = [1.2 * env, 1.2 * env];
        }
    }
    // as the body sinks the legs splay to keep their feet on the ground
    // (not when it lies on its back)
    if !matches!(a, Anim::Death | Anim::Dead) {
        let sink = (-off.y).max(0.0) * 0.25 + pitch.abs() * 1.5;
        for l in lift.iter_mut() {
            *l += sink;
        }
    }
    // a crab that stops turns to face its foe after a moment (not on every
    // pause in a chase); for a strike it turns at once
    if a.duration().is_none() && a != Anim::Dead {
        face *= smooth((t - 0.35) / 0.4);
    }
    p.off[BODY] = off;
    p.rot[BODY] = Quat::from_rotation_y(-FRAC_PI_2 * face) * euler(pitch, 0.0, roll);
    for (i, l) in rig.legs.iter().take(n).enumerate() {
        let s = l.side;
        p.rot[LEG0 + i] = if rig.sideways {
            Quat::from_rotation_x(-s * (lift[i] * 0.4 + swing[i] * 0.22))
        } else {
            Quat::from_rotation_y(-s * swing[i] * 0.38) * Quat::from_rotation_z(s * lift[i] * 0.3)
        };
    }
    for i in 0..rig.claws.min(2) {
        let s = SIDES[i];
        p.rot[CLAW[i]] =
            Quat::from_rotation_z(claw[i]) * Quat::from_rotation_y(s * (-0.2 + pinch[i]));
    }
    p
}

// ------------------------------------------------------------------- wisp

/// The pose of a wisp. Its root floats 0.85 blocks up; `off` moves it from
/// there. The core's size is [`key::SHAPE`], the motes' circle
/// [`key::SPREAD`], the tail's streaming [`key::FLOW`].
pub fn wisp(a: Anim, at: &At) -> Pose {
    use wisp::*;
    let t = at.t;
    let u = progress(a, t);
    let mut p = Pose::REST;
    let bob = ((t * 1.7 + at.seed * TAU).sin() * 0.15 + (t * 4.3).sin() * 0.03) * 32.0;
    let mut off = Vec3::new(0.0, bob, 0.0);
    let mut size = 1.0 + (t * 4.0).sin() * 0.07;
    let mut spread = 1.0;
    let mut flow = 0.25;
    let mut tilt = 0.0;
    let mut veil = 1.0 + (t * 2.3).sin() * 0.08;
    // the floor is 0.85 blocks below the root
    let floor = -0.85 * 32.0 + 3.5;
    match a {
        Anim::Idle => {
            let (env, _) = fidget(at);
            spread += 0.3 * env;
            off.x = (t * 0.6).sin() * 1.5;
        }
        Anim::Walk | Anim::Backpedal | Anim::StrafeLeft | Anim::StrafeRight | Anim::Swim => {
            flow = 0.25 + 0.65;
            tilt = 0.15;
            spread = 0.9;
        }
        Anim::Run | Anim::JumpAir | Anim::Fall | Anim::JumpStart | Anim::JumpLand => {
            flow = 1.15;
            tilt = 0.3;
            spread = 0.8;
            off.y *= 0.5;
        }
        Anim::Ready => {
            size = 1.12 + (t * 7.0).sin() * 0.06;
            spread = 0.8 + (t * 3.0).sin() * 0.1;
            flow = 0.35;
        }
        Anim::Attack1H | Anim::Attack2H | Anim::AttackUnarmed | Anim::Shoot | Anim::Special => {
            // gathers its motes in, swells, and spits
            size = track(u, &[(0.0, 1.0), (0.5, 1.5), (0.6, 0.8), (1.0, 1.0)]);
            spread = track(u, &[(0.0, 1.0), (0.5, 0.45), (0.6, 2.0), (1.0, 1.0)]);
            off.z = track(u, &[(0.0, 0.0), (0.5, -2.0), (0.6, 5.0), (1.0, 0.0)]);
            veil = track(u, &[(0.0, 1.0), (0.5, 1.3), (0.6, 0.9), (1.0, 1.0)]);
        }
        Anim::CastDirected | Anim::CastOmni | Anim::Channel => {
            size = 1.25 + (t * 9.0).sin() * 0.12;
            spread = 0.6 + (t * 2.0).sin() * 0.08;
            veil = 1.25;
            if a == Anim::CastOmni {
                off.y += 4.0;
            }
        }
        Anim::CastRelease => {
            size = track(u, &[(0.0, 1.4), (0.25, 0.8), (1.0, 1.0)]);
            spread = track(u, &[(0.0, 0.6), (0.25, 2.0), (1.0, 1.0)]);
            off.z = track(u, &[(0.0, 0.0), (0.25, 3.0), (1.0, 0.0)]);
        }
        Anim::HitReact => {
            let env = track(u, &[(0.0, 0.0), (0.12, 1.0), (1.0, 0.0)]);
            size = 1.0 - 0.35 * env;
            spread = 1.0 + 0.5 * env;
            off.z = -3.0 * env;
            off.x = (t * 40.0).sin() * 1.0 * env;
        }
        Anim::Stun => {
            off.y = -8.0 + bob * 0.3;
            size = 0.8 + (t * 13.0).sin().abs() * 0.1;
            spread = 0.7;
            flow = 0.0;
            tilt = (t * 1.5).sin() * 0.3;
        }
        Anim::Death | Anim::Dead => {
            // a last flare, the motes scatter, and it sinks to an ember
            let u = if a == Anim::Dead { 1.0 } else { u };
            off.y = track(u, &[(0.0, bob), (0.3, 4.0), (1.0, floor)]);
            size = track(u, &[(0.0, 1.0), (0.3, 1.55), (1.0, 0.3)]);
            spread = track(u, &[(0.0, 1.0), (0.3, 2.6), (1.0, 0.3)]);
            veil = track(u, &[(0.0, 1.0), (0.3, 1.4), (1.0, 0.3)]);
            flow = 0.0;
        }
        Anim::Sit | Anim::Kneel => {
            off.y = -10.0 + bob * 0.3;
            spread = 0.7;
            flow = 0.0;
        }
        Anim::Talk => {
            size = 1.0 + (t * 6.0).sin().abs() * 0.15;
            spread = 0.9;
        }
        Anim::Wave | Anim::Cheer => {
            // a little loop in the air
            let env = hump(u, 0.15, 0.15);
            let (s, c) = (t * 7.0).sin_cos();
            off += Vec3::new(
                s * 3.0,
                c * 3.0 + if a == Anim::Cheer { 5.0 } else { 0.0 },
                0.0,
            ) * env;
            spread += 0.4 * env;
        }
        Anim::Bow => {
            let env = hump(u, 0.3, 0.3);
            off.y -= 6.0 * env;
            tilt = 0.5 * env;
        }
        Anim::Point => {
            let env = hump(u, 0.2, 0.15);
            off.z = 4.0 * env;
            size = 1.0 + 0.2 * env;
        }
    }
    p.off[ROOT] = off;
    p.rot[ROOT] = Quat::from_rotation_x(tilt);
    p.off[VEIL] = Vec3::splat(veil);
    p.k[key::SHAPE] = size;
    p.k[key::SPREAD] = spread;
    p.k[key::FLOW] = flow;
    p
}

// --------------------------------------------------------------- animator

/// How long a change of state crossfades, in seconds.
pub const FADE: f32 = 0.2;
const ACTION_IN: f32 = 0.08;
const ACTION_OUT: f32 = 0.18;

/// One figure's animation state: a looping base state that crossfades when
/// it changes, a one-shot action layered on top (upper body only while the
/// legs are moving), and the stride phase, advanced by distance travelled.
#[derive(Clone, Debug)]
pub struct Animator {
    base: Anim,
    since: f32,
    action: Option<(Anim, f32)>,
    /// the pose being faded out, and when the fade began
    from: Option<(Pose, f32)>,
    last: Option<Pose>,
    stride: f32,
    seed: f32,
    last_pos: Option<Vec3>,
    /// when this was last driven, for pruning
    pub touched: f32,
}

impl Animator {
    pub fn new(seed: u32, now: f32) -> Animator {
        let h = seed.wrapping_mul(0x9E37_79B9) ^ 0x5bd1_e995;
        Animator {
            base: Anim::Idle,
            since: now,
            action: None,
            from: None,
            last: None,
            stride: 0.0,
            last_pos: None,
            seed: (h >> 8) as f32 / (1u32 << 24) as f32,
            touched: now,
        }
    }

    /// A figure in `anim` from time 0 at stride `stride`, with no fades: for
    /// sheets and tests. Draw it at the time into the animation wanted.
    pub fn still(anim: Anim, stride: f32) -> Animator {
        let mut a = Animator::new(7, 0.0);
        a.base = anim;
        a.stride = stride;
        a.seed = 0.3;
        a
    }

    pub fn base(&self) -> Anim {
        self.base
    }

    /// The action playing at `now`, if any.
    pub fn action(&self, now: f32) -> Option<Anim> {
        self.action
            .filter(|(a, at)| now - at < a.duration().unwrap_or(0.0))
            .map(|(a, _)| a)
    }

    /// How long the base state has run.
    pub fn base_time(&self, now: f32) -> f32 {
        now - self.since
    }

    /// Set the base state (crossfading if it changes) and move the stride on
    /// by `moved` blocks at `step` blocks per step.
    pub fn drive(&mut self, now: f32, base: Anim, moved: f32, step: f32) {
        self.touched = now;
        if base != self.base {
            if let Some(last) = self.last {
                self.from = Some((last, now));
            }
            self.base = base;
            self.since = now;
        }
        if moved.is_finite() && step > 0.0 {
            self.stride = (self.stride + moved.min(4.0) / step * PI).rem_euclid(TAU * 64.0);
        }
    }

    /// Start a one-shot action, replacing any playing.
    pub fn play(&mut self, now: f32, action: Anim) {
        self.action = Some((action, now));
    }

    /// Start a one-shot unless another began in the last quarter second.
    pub fn play_fresh(&mut self, now: f32, action: Anim) {
        if !self.action.is_some_and(|(_, at)| now - at < 0.25) {
            self.play(now, action);
        }
    }

    /// Set the base state from where the figure now stands: the stride moves
    /// on by the ground covered since last time.
    pub fn drive_at(&mut self, now: f32, base: Anim, pos: Vec3, step: f32) {
        let moved = self
            .last_pos
            .map(|p| Vec3::new(pos.x - p.x, 0.0, pos.z - p.z).length())
            .unwrap_or(0.0);
        self.last_pos = Some(pos);
        self.drive(now, base, moved, step);
    }

    /// The pose at `now`, from `pose` (a skeleton's pose function); `upper`
    /// is the skeleton's upper-body mask.
    pub fn pose(&mut self, now: f32, upper: u32, pose: impl Fn(Anim, &At) -> Pose) -> Pose {
        let at = At {
            t: (now - self.since).max(0.0),
            stride: self.stride,
            seed: self.seed,
        };
        let mut p = pose(self.base, &at);
        if let Some((from, start)) = &self.from {
            let w = (now - start) / FADE;
            if w < 1.0 {
                p = from.blend(&p, smooth(w), u32::MAX);
            } else {
                self.from = None;
            }
        }
        self.last = Some(p);
        if let Some((a, start)) = self.action {
            let t = now - start;
            let d = a.duration().unwrap_or(0.0);
            if t >= d || t < 0.0 {
                self.action = None;
            } else {
                let w = smooth(t / ACTION_IN).min(smooth((d - t) / ACTION_OUT));
                let q = pose(a, &At { t, ..at });
                let mask = if self.base.moving() { upper } else { u32::MAX };
                p = p.blend(&q, w, mask);
            }
        }
        p.k[key::CLOCK] = now;
        p
    }
}

/// How a figure is moving, for choosing its locomotion.
pub fn locomotion(vel: Vec3, yaw: f32, run_speed: f32) -> Option<Anim> {
    let flat = Vec3::new(vel.x, 0.0, vel.z);
    let speed = flat.length();
    if speed < 0.3 {
        return None;
    }
    let fwd = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let (f, r) = (flat.dot(fwd) / speed, flat.dot(right) / speed);
    Some(if f < -0.4 {
        Anim::Backpedal
    } else if r.abs() > 0.8 {
        if r > 0.0 {
            Anim::StrafeRight
        } else {
            Anim::StrafeLeft
        }
    } else if speed >= run_speed * 0.62 {
        Anim::Run
    } else {
        Anim::Walk
    })
}

// ------------------------------------------------------------------- book

/// Animation state for every figure in view: you, other players and
/// entities (creatures, NPCs, corpses), on one clock.
pub struct Book {
    epoch: std::time::Instant,
    pub me: Option<Animator>,
    pub players: HashMap<u32, Animator>,
    pub entities: HashMap<u32, Animator>,
    /// what each player is casting (ability), since when, and for how long
    pub casts: HashMap<u32, (u16, f32, f32)>,
    /// when you last swung at your auto-attack target
    pub last_swing: f32,
    /// players breaking or gathering a block
    pub digging: HashMap<u32, f32>,
    /// where creatures vanished lately: a corpse there has just died
    gone: Vec<(Vec3, f32)>,
    /// you were on the ground last frame, and since when you have not been
    pub grounded: bool,
    pub air_since: f32,
    /// a held emote (Sit, Kneel, Dead), until you move
    pub hold: Option<Anim>,
    /// what you hold, for picking your attack
    pub grip: Grip,
}

impl Default for Book {
    fn default() -> Self {
        Book {
            epoch: std::time::Instant::now(),
            me: None,
            players: HashMap::new(),
            entities: HashMap::new(),
            casts: HashMap::new(),
            last_swing: 0.0,
            digging: HashMap::new(),
            gone: Vec::new(),
            grounded: true,
            air_since: 0.0,
            hold: None,
            grip: Grip::Empty,
        }
    }
}

impl Book {
    /// Seconds on the book's clock.
    pub fn now(&self) -> f32 {
        self.epoch.elapsed().as_secs_f32()
    }

    pub fn me(&mut self, now: f32) -> &mut Animator {
        self.me.get_or_insert_with(|| Animator::new(1, now))
    }

    pub fn player(&mut self, id: u32, now: f32) -> &mut Animator {
        self.players
            .entry(id)
            .or_insert_with(|| Animator::new(id.wrapping_add(0x1000), now))
    }

    /// An entity's animator, and whether it is new.
    pub fn entity(&mut self, id: u32, now: f32) -> (&mut Animator, bool) {
        let mut new = false;
        let a = self.entities.entry(id).or_insert_with(|| {
            new = true;
            Animator::new(id, now)
        });
        (a, new)
    }

    /// Start a one-shot on an entity, unless one began just now (so that a
    /// spell's damage does not cut off its release).
    pub fn play_entity(&mut self, id: u32, now: f32, action: Anim) {
        let (a, _) = self.entity(id, now);
        a.play_fresh(now, action);
    }

    /// The ability a player is casting, if the cast bar has not run out.
    pub fn casting(&self, player: u32, now: f32) -> Option<u16> {
        self.casts
            .get(&player)
            .filter(|(_, at, secs)| now - at < secs + 0.5)
            .map(|(a, _, _)| *a)
    }

    /// A creature vanished at `pos` (it died, or wandered off).
    pub fn creature_gone(&mut self, pos: Vec3, now: f32) {
        self.gone.retain(|(_, t)| now - t < 2.0);
        self.gone.push((pos, now));
    }

    /// Whether a corpse at `pos` is one that died a moment ago (rather than
    /// one that was already lying there when it came into view).
    pub fn fresh_corpse(&self, pos: Vec3, now: f32) -> bool {
        self.gone
            .iter()
            .any(|(p, t)| now - t < 1.0 && p.distance(pos) < 1.5)
    }

    /// Forget figures not drawn for a few seconds.
    pub fn prune(&mut self, now: f32) {
        let keep = |a: &Animator| now - a.touched < 5.0;
        self.players.retain(|_, a| keep(a));
        self.entities.retain(|_, a| keep(a));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_ease_and_hold() {
        let k = [(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)];
        assert_eq!(track(-1.0, &k), 0.0);
        assert_eq!(track(0.5, &k), 1.0);
        assert_eq!(track(2.0, &k), 0.0);
        assert!((track(0.25, &k) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn locomotion_reads_direction() {
        let yaw = 0.0; // facing -z
        assert_eq!(locomotion(Vec3::ZERO, yaw, 4.4), None);
        assert_eq!(
            locomotion(Vec3::new(0.0, 0.0, -4.4), yaw, 4.4),
            Some(Anim::Run)
        );
        assert_eq!(
            locomotion(Vec3::new(0.0, 0.0, -1.5), yaw, 4.4),
            Some(Anim::Walk)
        );
        assert_eq!(
            locomotion(Vec3::new(0.0, 0.0, 2.0), yaw, 4.4),
            Some(Anim::Backpedal)
        );
        assert_eq!(
            locomotion(Vec3::new(3.0, 0.0, 0.0), yaw, 4.4),
            Some(Anim::StrafeRight)
        );
        assert_eq!(
            locomotion(Vec3::new(-3.0, 0.0, 0.0), yaw, 4.4),
            Some(Anim::StrafeLeft)
        );
    }

    #[test]
    fn crossfades_and_actions_settle() {
        let rig = BipedRig {
            grip: Grip::OneHand,
            robe: false,
        };
        let f = |a: Anim, at: &At| biped(a, at, &rig);
        let mut an = Animator::new(1, 0.0);
        an.drive(0.0, Anim::Idle, 0.0, 1.0);
        let idle = an.pose(0.0, Skeleton::Biped.upper(), f);
        an.drive(1.0, Anim::Death, 0.0, 1.0);
        let mid = an.pose(1.0 + FADE * 0.5, Skeleton::Biped.upper(), f);
        assert!(mid.is_finite());
        assert!(mid.distance(&idle) > 0.0);
        an.play(2.0, Anim::HitReact);
        assert_eq!(an.action(2.1), Some(Anim::HitReact));
        let _ = an.pose(2.1, Skeleton::Biped.upper(), f);
        let _ = an.pose(3.0, Skeleton::Biped.upper(), f);
        assert_eq!(an.action(3.0), None);
    }

    #[test]
    fn emotes_parse() {
        assert_eq!(Anim::emote("/wave"), Some(Anim::Wave));
        assert_eq!(Anim::emote("/Cheer "), Some(Anim::Cheer));
        assert_eq!(Anim::emote("/who"), None);
    }
}
