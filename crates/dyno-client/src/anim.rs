//! The animation contract: one set of animation names ([`Anim`]) that every
//! character and creature performs, poses on a skeleton's named bones
//! ([`Pose`]), and an [`Animator`] that crossfades between looping states
//! and layers one-shot actions on top.
//!
//! What each animation looks like is content: clips in the pack
//! (`anims/<skeleton>.json`, see [`crate::clip`]). A pack test checks that
//! every figure has a clip for every name here. See `docs/design/animation.md`.

use crate::clip::ClipSet;
use glam::{Quat, Vec3};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

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
            /// The animation with this name ("Idle", "Attack1H"...).
            pub fn by_name(name: &str) -> Option<Anim> {
                Anim::ALL.iter().copied().find(|a| a.name() == name)
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
    /// at work: swinging an axe or a pick at something in front (loops)
    Chop,
    /// at work: short blows with a hammer on something at waist height
    Hammer,
    /// at work: a rod held out over the water, now and then a tug
    Fish,
    /// at work: long strokes with a broom or a rake
    Sweep,
}

impl Anim {
    /// Whether this is a one-shot (an action or emote) rather than a state
    /// that loops. How long it lasts is up to the clip.
    pub fn one_shot(self) -> bool {
        use Anim::*;
        matches!(
            self,
            JumpStart
                | JumpLand
                | Attack1H
                | Attack2H
                | AttackUnarmed
                | Shoot
                | Special
                | CastRelease
                | HitReact
                | Death
                | Wave
                | Bow
                | Cheer
                | Point
        )
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

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// ------------------------------------------------------------------ grips

/// What a biped holds, which shapes its stances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip {
    Empty,
    OneHand,
    TwoHand,
    Bow,
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
    /// how long the action lasts, once its clip is known
    action_len: Option<f32>,
    /// the pose being faded out, and when the fade began
    from: Option<(Pose, f32)>,
    last: Option<Pose>,
    stride: f32,
    seed: f32,
    /// actions started so far, and whether this one plays mirrored (for
    /// clips that may: a flinch goes either way)
    plays: u32,
    mirrored: bool,
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
            action_len: None,
            from: None,
            last: None,
            stride: 0.0,
            plays: 0,
            mirrored: false,
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

    /// The action playing at `now`, if any (until it is posed past its
    /// end).
    pub fn action(&self, now: f32) -> Option<Anim> {
        self.action
            .filter(|(_, at)| self.action_len.is_none_or(|d| now - at < d))
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
        self.action_len = None;
        // the side, from the figure and the count of plays: the same for
        // the same history, so sheets and tests stay put
        self.plays = self.plays.wrapping_add(1);
        let h =
            (self.seed.to_bits() ^ self.plays.wrapping_mul(0x9E37_79B9)).wrapping_mul(0x85EB_CA6B);
        self.mirrored = (h >> 15) & 1 == 1;
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

    /// The pose at `now` from a figure's clips: the base state crossfading
    /// from the last, and the action layered on (upper body only while the
    /// legs move).
    pub fn pose(&mut self, now: f32, set: &ClipSet) -> Pose {
        let mut p = set.pose(
            self.base,
            (now - self.since).max(0.0),
            self.stride,
            self.seed,
        );
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
            let d = set.duration(a).unwrap_or(0.0);
            self.action_len = Some(d);
            if t >= d || t < 0.0 {
                self.action = None;
            } else {
                let w = smooth(t / ACTION_IN).min(smooth((d - t) / ACTION_OUT));
                let q = set.pose_side(a, t, self.stride, self.seed, self.mirrored);
                let mask = if self.base.moving() {
                    set.upper
                } else {
                    u32::MAX
                };
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
        let sk: crate::clip::SkeletonSpec =
            serde_json::from_value(serde_json::json!({"bones": ["body", "arm"], "upper": ["arm"]}))
                .unwrap();
        let sets = crate::clip::parse_sets(
            &sk,
            &serde_json::json!({"sets": {"default": {
                "Idle": {"loop": 1.0, "time": {"arm.rx": [[0, 0], [1, 10]]}},
                "Death": {"duration": 1.2, "hold": true, "time": {"body.rx": [[0, 0], [1.2, -90]]}},
                "HitReact": {"duration": 0.4, "time": {"arm.rz": [[0, 0], [0.2, 30], [0.4, 0]]}}
            }}}),
        )
        .unwrap();
        let set = &sets["default"];
        let mut an = Animator::new(1, 0.0);
        an.drive(0.0, Anim::Idle, 0.0, 1.0);
        let idle = an.pose(0.0, set);
        an.drive(1.0, Anim::Death, 0.0, 1.0);
        let mid = an.pose(1.0 + FADE * 0.5, set);
        assert!(mid.is_finite());
        assert!(mid.distance(&idle) > 0.0);
        an.play(2.0, Anim::HitReact);
        assert_eq!(an.action(2.1), Some(Anim::HitReact));
        let _ = an.pose(2.1, set);
        let _ = an.pose(3.0, set);
        assert_eq!(an.action(3.0), None);
    }

    #[test]
    fn emotes_parse() {
        assert_eq!(Anim::emote("/wave"), Some(Anim::Wave));
        assert_eq!(Anim::emote("/Cheer "), Some(Anim::Cheer));
        assert_eq!(Anim::emote("/who"), None);
    }
}
