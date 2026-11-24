//! The play camera (F5 in docs/design/feel.md; the rules are in
//! docs/design/camera.md): third person behind the character, held short of
//! walls without popping, with a light follow lag and a slight look-ahead.
//!
//! It is plain state stepped once a frame by [`Camera::update`], with the
//! world given as a solidity query, so the tests below drive it without a
//! window. `dyno shot` and the gauntlet do not use it: they place their
//! camera at once with `app::pull_back`, so their framing is unchanged.
//!
//! The two rules, kept apart:
//! - **hard, every frame:** the eye (a sphere of [`MARGIN`]) is never in a
//!   solid cell. When it would be, the boom is cut to the clear length at
//!   once.
//! - **eased:** the boom's wanted length shortens smoothly as something
//!   nears it (see [`reach`]), so a lintel or a trunk coming between the
//!   camera and the character draws it in over a few frames instead of all
//!   at once; it follows that length in fast but at a bounded speed
//!   ([`IN_SPEED`]), and when the way is clear again it eases back out over
//!   about [`OUT_TIME`] seconds.
//!
//! Screen shake (F2) goes in [`Camera::shake`]: an offset added to the eye
//! after the collision, dropped for a frame if it would put the eye in a wall.

use glam::Vec3;

/// The default distance behind the character, metres (scale.md).
pub const DEFAULT_ZOOM: f32 = 9.0;
/// The wheel's range, metres. One notch in from the nearest is first person.
pub const MIN_ZOOM: f32 = 2.0;
pub const MAX_ZOOM: f32 = 25.0;
/// How much the eye rises for each metre back (as `pull_back` does, so a
/// play camera at rest frames like a `dyno shot` one).
pub const RISE: f32 = 0.08;
/// The eye keeps this far from any solid cell (the near plane is 0.05 m).
pub const MARGIN: f32 = 0.25;
/// Closest the eye may pull in towards the head while drawn as third person;
/// nearer than this the character is hidden.
const HEAD: f32 = 0.5;
/// Nearer than this to the head the character starts to fade (dithered).
const FADE_FROM: f32 = 1.3;
/// Pull-in: a critically damped spring (rad/s), its speed capped (zoom units/s).
const IN_RATE: f32 = 18.0;
const IN_RATE_HIDDEN: f32 = 45.0;
pub const IN_SPEED: f32 = 50.0;
/// Ease back out: about this long to settle (a critically damped spring).
pub const OUT_TIME: f32 = 0.3;
/// The follow point's springs (critically damped), rad/s: across and up.
const FOLLOW_ACROSS: f32 = 22.0;
const FOLLOW_UP: f32 = 12.0;
/// The follow point never trails the head by more than this across, metres...
const MAX_LAG: f32 = 1.0;
/// ...nor above or below it by more than this (two steps of a stair).
const MAX_LAG_UP: f32 = 2.0;
/// Further than this from the head (a teleport, a respawn): snap.
const SNAP: f32 = 4.0;
/// Look-ahead: seconds of travel, the most it leads, and how fast it eases.
const LEAD_TIME: f32 = 0.09;
const MAX_LEAD: f32 = 0.7;
const LEAD_RATE: f32 = 3.0;
/// The wheel's zoom eases to where it was set at this rate (1/s).
const ZOOM_RATE: f32 = 12.0;
/// Auto-follow (an option): after this long without the player turning
/// the camera, moving swings it back behind the character at this rate.
pub const FOLLOW_AFTER: f32 = 2.5;
const FOLLOW_RATE: f32 = 2.0;
/// The step along the boom when looking for its clear length, metres.
const PROBE_STEP: f32 = 0.05;
/// The boom starts to shorten when it passes this near something, metres...
const SOFT: f32 = 1.5;
/// ...but within this far of the head only in part (a doorway's sides).
const SOFT_RAMP: f32 = 2.5;
/// How far (zoom units) the eye comes in ahead of something closing on it.
const END: f32 = 1.5;

/// What the camera shows this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct View {
    pub eye: Vec3,
    /// where it looks (the character's view direction, led a little)
    pub dir: Vec3,
    /// how much of the character is drawn: 1 all, 0 none (the camera is in
    /// or at the head)
    pub fade: f32,
    /// the boom's length now (in zoom units: metres back along the view)
    pub arm: f32,
    /// the clear length of the boom this frame (zoom units)
    pub limit: f32,
    /// the frame time it was stepped by, seconds
    pub dt: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Camera {
    ready: bool,
    want: f32,
    arm: f32,
    arm_vel: f32,
    pivot: Vec3,
    pivot_vel: Vec3,
    lead: Vec3,
    /// seconds since the player last turned the camera (auto-follow waits)
    since_turn: f32,
    /// added to the eye after collision (screen shake, F2)
    pub shake: Vec3,
    /// the last frame's view
    pub view: View,
}

/// One step of a critically damped spring towards a still target (exact for
/// any `dt`, so a long frame cannot make it overshoot).
fn spring(x: f32, v: f32, target: f32, omega: f32, dt: f32) -> (f32, f32) {
    let y = x - target;
    let e = (-omega * dt).exp();
    let k = (v + omega * y) * dt;
    (target + (y + k) * e, (v - omega * k) * e)
}

/// Is a ball of radius `r` at `q` clear of every solid cell?
pub fn clear(solid: &impl Fn(i32, i32, i32) -> bool, q: Vec3, r: f32) -> bool {
    let lo = (q - Vec3::splat(r)).floor();
    let hi = (q + Vec3::splat(r)).floor();
    for x in lo.x as i32..=hi.x as i32 {
        for y in lo.y as i32..=hi.y as i32 {
            for z in lo.z as i32..=hi.z as i32 {
                if !solid(x, y, z) {
                    continue;
                }
                let c = Vec3::new(x as f32, y as f32, z as f32);
                let near = q.clamp(c, c + Vec3::ONE);
                if near.distance_squared(q) < r * r {
                    return false;
                }
            }
        }
    }
    true
}

/// How far the nearest solid cell is from `q`, up to `most`.
pub fn clearance(solid: &impl Fn(i32, i32, i32) -> bool, q: Vec3, most: f32) -> f32 {
    let lo = (q - Vec3::splat(most)).floor();
    let hi = (q + Vec3::splat(most)).floor();
    let mut best = most * most;
    for x in lo.x as i32..=hi.x as i32 {
        for y in lo.y as i32..=hi.y as i32 {
            for z in lo.z as i32..=hi.z as i32 {
                if !solid(x, y, z) {
                    continue;
                }
                let c = Vec3::new(x as f32, y as f32, z as f32);
                best = best.min(q.clamp(c, c + Vec3::ONE).distance_squared(q));
            }
        }
    }
    best.sqrt()
}

/// The nearest point of a solid cell to `q` within `most`, if any.
fn nearest(solid: &impl Fn(i32, i32, i32) -> bool, q: Vec3, most: f32) -> Option<Vec3> {
    let lo = (q - Vec3::splat(most)).floor();
    let hi = (q + Vec3::splat(most)).floor();
    let mut best: Option<(f32, Vec3)> = None;
    for x in lo.x as i32..=hi.x as i32 {
        for y in lo.y as i32..=hi.y as i32 {
            for z in lo.z as i32..=hi.z as i32 {
                if !solid(x, y, z) {
                    continue;
                }
                let c = Vec3::new(x as f32, y as f32, z as f32);
                let p = q.clamp(c, c + Vec3::ONE);
                let d = p.distance_squared(q);
                if d < most * most && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// The most the eye is pushed off the boom to slide along a surface, metres.
const PUSH: f32 = 1.0;

/// `q` moved out of the solid cells near it until a ball of `r` is clear,
/// by no more than [`PUSH`]; `None` when it cannot be (it is inside one).
fn push_out(solid: &impl Fn(i32, i32, i32) -> bool, q: Vec3, r: f32) -> Option<Vec3> {
    let mut e = q;
    for _ in 0..4 {
        let Some(p) = nearest(solid, e, r) else {
            return (e.distance(q) <= PUSH).then_some(e);
        };
        let d = e.distance(p);
        if d < 1e-4 {
            return None;
        }
        e = p + (e - p) / d * (r * 1.001);
    }
    (clear(solid, e, r) && e.distance(q) <= PUSH).then_some(e)
}

/// Can the eye at `to` see `from` (no solid cell on the line between)?
fn sight(solid: &impl Fn(i32, i32, i32) -> bool, from: Vec3, to: Vec3) -> bool {
    let n = (from.distance(to) / 0.1).ceil().max(1.0) as usize;
    (0..=n).all(|i| {
        let p = from.lerp(to, i as f32 / n as f32).floor();
        !solid(p.x as i32, p.y as i32, p.z as i32)
    })
}

/// How long the boom may be: from `from` along `boom` (the eye's offset
/// per zoom unit), up to `want` units. Two lengths:
/// - `hard`: the eye ball ([`MARGIN`]) is clear the whole way to it;
/// - `soft` (no longer than `hard`): also shortened, in proportion, where
///   the boom passes within [`SOFT`] of something, so the camera starts to
///   come in as an edge nears the boom instead of all at once when the edge
///   crosses it. It changes smoothly as the character and the view move.
pub fn reach(
    solid: &impl Fn(i32, i32, i32) -> bool,
    from: Vec3,
    boom: Vec3,
    want: f32,
) -> (f32, f32) {
    let len = boom.length().max(1e-3);
    if want <= 0.0 || clearance(solid, from, MARGIN) < MARGIN {
        return (0.0, 0.0);
    }
    let step = PROBE_STEP / len;
    let (mut soft, mut d) = (want, 0.0f32);
    while d < want {
        let next = (d + step).min(want);
        let s = clearance(solid, from + boom * next, SOFT);
        if s < MARGIN {
            return (soft.min(d), d);
        }
        // near the head the boom is let pass closer (a doorway's sides)
        let w = smoothstep(MARGIN, SOFT, s);
        let ramp = (next * len / SOFT_RAMP).min(1.0);
        soft = soft.min(next + (want - next) * (1.0 - (1.0 - w) * ramp));
        // near the eye's end, something closing in from the side draws it
        // in before it gets there (the above alone would wait for the touch)
        if next > want - END {
            soft = soft.min(want - (1.0 - w) * END * ramp);
        }
        d = next;
    }
    (soft, want)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The eye's offset per zoom unit behind a view direction.
pub fn boom(dir: Vec3) -> Vec3 {
    -dir + Vec3::Y * RISE
}

fn instant() -> bool {
    static INSTANT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *INSTANT.get_or_init(|| std::env::var("DYNO_CAMERA").is_ok_and(|v| v == "instant"))
}

/// Shortest signed angle, -pi..pi.
fn wrap(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

impl Camera {
    /// Forget the springs: the next frame starts where the character is.
    pub fn snap(&mut self) {
        self.ready = false;
    }

    /// The player turned the camera (orbit drag): auto-follow waits.
    pub fn turned(&mut self) {
        self.since_turn = 0.0;
    }

    /// Auto-follow: while moving, after [`FOLLOW_AFTER`] seconds without the
    /// player turning the camera, ease `orbit` (the camera's yaw about the
    /// character) back to behind it.
    pub fn follow(&mut self, orbit: &mut f32, on: bool, moving: bool, dt: f32) {
        self.since_turn += dt;
        if on && moving && self.since_turn > FOLLOW_AFTER {
            *orbit = wrap(*orbit) * (-FOLLOW_RATE * dt.min(0.1)).exp();
        }
    }

    /// Step the camera one frame. `head` is the character's eye point,
    /// `dir` the view direction (yaw and pitch, not smoothed), `vel` the
    /// character's velocity, `zoom` the distance the wheel asks for (below
    /// 0.5 first person).
    pub fn update(
        &mut self,
        solid: impl Fn(i32, i32, i32) -> bool,
        head: Vec3,
        dir: Vec3,
        vel: Vec3,
        zoom: f32,
        dt: f32,
    ) -> View {
        let dt = dt.clamp(0.0, 0.1);
        if zoom < 0.5 {
            // first person: the boom grows from nothing when it comes back
            self.ready = true;
            self.want = 0.0;
            self.arm = 0.0;
            self.arm_vel = 0.0;
            self.pivot = head;
            self.pivot_vel = Vec3::ZERO;
            self.lead = Vec3::ZERO;
            self.view = View {
                eye: head,
                dir,
                fade: 0.0,
                arm: 0.0,
                limit: 0.0,
                dt,
            };
            return self.view;
        }
        // DYNO_CAMERA=instant: every frame placed at once, no smoothing (the
        // camera before F5, for a before-and-after with `dyno drive`)
        let snap = !self.ready || self.pivot.distance(head) > SNAP || instant();
        if snap {
            self.pivot = head;
            self.pivot_vel = Vec3::ZERO;
            self.lead = Vec3::ZERO;
            self.want = zoom;
        }
        // the wheel's distance, eased
        self.want += (zoom - self.want) * (1.0 - (-ZOOM_RATE * dt).exp());
        // the follow point: critically damped behind the head, never far
        let (x, vx) = spring(self.pivot.x, self.pivot_vel.x, head.x, FOLLOW_ACROSS, dt);
        let (y, vy) = spring(self.pivot.y, self.pivot_vel.y, head.y, FOLLOW_UP, dt);
        let (z, vz) = spring(self.pivot.z, self.pivot_vel.z, head.z, FOLLOW_ACROSS, dt);
        self.pivot = Vec3::new(x, y, z);
        self.pivot_vel = Vec3::new(vx, vy, vz);
        let lag = self.pivot - head;
        let across = Vec3::new(lag.x, 0.0, lag.z).clamp_length_max(MAX_LAG);
        let up = lag.y.clamp(-MAX_LAG_UP, MAX_LAG_UP);
        self.pivot = head + Vec3::new(across.x, up, across.z);
        // (it trails along the way the head came, so it is in the open; if
        // not, the nearest point towards the head that is)
        if !clear(&solid, self.pivot, MARGIN) {
            let from = self.pivot;
            self.pivot = (1..=8)
                .map(|i| from.lerp(head, i as f32 / 8.0))
                .find(|q| clear(&solid, *q, MARGIN))
                .unwrap_or(head);
            self.pivot_vel = Vec3::ZERO;
        }
        // look-ahead: a little of the way the character is going
        let flat = Vec3::new(vel.x, 0.0, vel.z) * LEAD_TIME;
        let lead_to = flat.clamp_length_max(MAX_LEAD);
        self.lead += (lead_to - self.lead) * (1.0 - (-LEAD_RATE * dt).exp());
        // the boom: how long it may be, then the eased length
        let b = boom(dir);
        let (soft, hard) = reach(&solid, self.pivot, b, self.want);
        let target = self.want.min(soft);
        if snap {
            self.arm = if instant() { hard } else { target };
            self.arm_vel = 0.0;
        } else {
            // one critically damped spring, so the boom's speed never jumps:
            // in fast when something nears or comes between, back out over
            // about OUT_TIME when it is clear
            // (faster while the eye is cut off from the character: what it
            // then has to cross to come in front is still thin)
            let omega = if self.arm > hard + 0.05 {
                IN_RATE_HIDDEN
            } else if target < self.arm {
                IN_RATE
            } else {
                4.74 / OUT_TIME
            };
            let (a, v) = spring(self.arm, self.arm_vel, target, omega, dt);
            let v = v.clamp(-IN_SPEED, IN_SPEED);
            let most = IN_SPEED * dt;
            self.arm = a.clamp(self.arm - most, self.arm + most).max(0.0);
            self.arm_vel = v;
            // (never further out than it may be)
            if self.arm > target && v > 0.0 {
                self.arm = target;
                self.arm_vel = 0.0;
            }
        }
        // the hard rule: the eye itself is never in a wall. Grazing a
        // surface (a ceiling or the ground pressing on the eye) it slides
        // along it, pushed out as far as it went in; only when that cannot
        // be done is the boom cut to its clear length at once.
        let mut eye = self.pivot + b * self.arm;
        if self.arm > hard && !clear(&solid, eye, MARGIN) {
            match push_out(&solid, eye, MARGIN) {
                Some(e) if sight(&solid, self.pivot, e) => eye = e,
                _ => {
                    self.arm = hard;
                    self.arm_vel = 0.0;
                    eye = self.pivot + b * self.arm;
                }
            }
        }
        let limit = hard;
        let shaken = eye + self.shake;
        if self.shake != Vec3::ZERO && clear(&solid, shaken, MARGIN) {
            eye = shaken;
        }
        // look-ahead turns the view a little towards where it is going (less
        // when the camera is close in, where a little is a lot)
        let reach = (self.arm * b.length()).max(1.0);
        let look = (dir * reach + self.lead * (self.arm / 4.0).min(1.0)).normalize_or_zero();
        let near = eye.distance(head);
        let fade = ((near - HEAD) / (FADE_FROM - HEAD)).clamp(0.0, 1.0);
        self.ready = true;
        self.view = View {
            eye,
            dir: if look == Vec3::ZERO { dir } else { look },
            fade,
            arm: self.arm,
            limit,
            dt,
        };
        self.view
    }
}

/// One notch of the wheel (`notches` > 0 in). From the nearest distance one
/// more notch in is first person (0), and one out of first person is the
/// nearest distance.
pub fn wheel(zoom: f32, notches: f32) -> f32 {
    if zoom < 0.5 {
        return if notches < 0.0 { MIN_ZOOM } else { 0.0 };
    }
    if notches > 0.0 && zoom <= MIN_ZOOM + 0.01 {
        return 0.0;
    }
    (zoom * 0.87f32.powf(notches)).clamp(MIN_ZOOM, MAX_ZOOM)
}

/// A saved zoom, made sane (a file from an older client has none: 9 m).
pub fn saved(zoom: f32) -> f32 {
    if !zoom.is_finite() {
        DEFAULT_ZOOM
    } else if zoom < 0.5 {
        0.0
    } else {
        zoom.clamp(MIN_ZOOM, MAX_ZOOM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn dir_north(pitch: f32) -> Vec3 {
        let (sp, cp) = pitch.sin_cos();
        Vec3::new(0.0, sp, -cp)
    }

    /// A floor at y < 0 and a wall across z = `wall`..`wall`+1 (when given).
    fn world(wall: Option<i32>) -> impl Fn(i32, i32, i32) -> bool {
        move |_x, y, z| y < 0 || wall.is_some_and(|w| z == w && y < 6)
    }

    #[test]
    fn at_rest_it_frames_like_pull_back() {
        let solid = world(None);
        let mut c = Camera::default();
        let head = Vec3::new(0.5, 1.62, 0.5);
        let d = dir_north(-0.26);
        let mut v = View::default();
        for _ in 0..120 {
            v = c.update(&solid, head, d, Vec3::ZERO, DEFAULT_ZOOM, DT);
        }
        let want = head + boom(d) * DEFAULT_ZOOM;
        assert!(v.eye.distance(want) < 1e-3, "{:?} vs {want:?}", v.eye);
        assert!((v.dir - d).length() < 1e-4);
        assert_eq!(v.fade, 1.0);
    }

    #[test]
    fn a_wall_coming_between_pulls_in_fast_and_eases_out() {
        let d = dir_north(-0.2);
        let head = Vec3::new(0.5, 1.62, 0.5);
        let mut c = Camera::default();
        for _ in 0..60 {
            c.update(world(None), head, d, Vec3::ZERO, 9.0, DT);
        }
        // a wall appears from nowhere 3 m behind: the one case with no
        // smooth way in (the eye must cross it), so one step may be the
        // wall's thickness and the margins
        let solid = world(Some(3));
        let mut last = c.view.eye;
        let mut jumps = Vec::new();
        let mut frames = 0;
        let mut crossings = 0;
        loop {
            let v = c.update(&solid, head, d, Vec3::ZERO, 9.0, DT);
            let jump = v.eye.distance(last);
            jumps.push(jump);
            last = v.eye;
            frames += 1;
            assert!(clear(&solid, v.eye, MARGIN - 1e-3), "inside: {:?}", v.eye);
            if jump > IN_SPEED * DT * 1.1 + 1e-3 {
                crossings += 1;
                assert!(jump < 1.0 + 2.0 * MARGIN + IN_SPEED * DT, "jump {jump}");
            }
            if v.arm <= v.limit + 1e-3 {
                break;
            }
            assert!(frames < 30, "still not in after {frames} frames");
        }
        assert!(crossings <= 1, "{jumps:?}");
        assert!(frames <= 20, "in after {frames} frames");
        // gone again: back out over about OUT_TIME, smoothly
        let solid = world(None);
        let mut t = 0.0;
        let mut prev = c.view.arm;
        while c.view.arm < 9.0 * 0.95 {
            c.update(&solid, head, d, Vec3::ZERO, 9.0, DT);
            assert!(c.view.arm >= prev - 1e-4, "it went back in");
            assert!(c.view.arm - prev < 1.0, "a jump out");
            prev = c.view.arm;
            t += DT;
            assert!(t < 1.0);
        }
        assert!(
            (OUT_TIME * 0.6..=OUT_TIME * 1.5).contains(&t),
            "out in {t} s"
        );
    }

    #[test]
    fn walking_through_a_door_has_no_jump() {
        // a wall across z = -4 with a door two wide and three tall
        let solid = |x: i32, y: i32, z: i32| {
            y < 0 || (z == -4 && y < 6 && !((0..=1).contains(&x) && y <= 2))
        };
        let d = dir_north(-0.26);
        let vel = Vec3::new(0.0, 0.0, -6.2);
        let mut c = Camera::default();
        let mut head = Vec3::new(1.0, 1.62, 6.0);
        let mut last = None;
        let mut worst: f32 = 0.0;
        for _ in 0..150 {
            head += vel * DT;
            let v = c.update(solid, head, d, vel, 9.0, DT);
            assert!(clear(&solid, v.eye, MARGIN - 1e-3), "inside at {head:?}");
            if let Some(l) = last {
                worst = worst.max(v.eye.distance(l));
            }
            last = Some(v.eye);
        }
        assert!(
            c.view.arm < 6.0,
            "it came in under the lintel: {}",
            c.view.arm
        );
        assert!(worst < 0.6, "a jump of {worst} m");
    }

    #[test]
    fn backing_into_a_wall_never_puts_the_eye_inside() {
        // walk backwards towards a wall: the boom shortens as it nears
        let solid = world(Some(8));
        let d = dir_north(-0.2);
        let mut c = Camera::default();
        let mut head = Vec3::new(0.5, 1.62, -6.0);
        for _ in 0..300 {
            head.z += 3.0 * DT;
            if head.z > 7.4 {
                break;
            }
            let v = c.update(&solid, head, d, Vec3::new(0.0, 0.0, 3.0), 9.0, DT);
            assert!(clear(&solid, v.eye, MARGIN - 1e-3), "inside at {head:?}");
        }
        // right up against it: the camera is at the head, the figure hidden
        assert!(c.view.fade < 0.5, "fade {}", c.view.fade);
    }

    #[test]
    fn the_follow_lags_a_little_and_snaps_on_a_teleport() {
        let solid = world(None);
        let d = dir_north(-0.2);
        let mut c = Camera::default();
        let mut head = Vec3::new(0.5, 1.62, 0.5);
        let vel = Vec3::new(0.0, 0.0, -6.2);
        for _ in 0..120 {
            head += vel * DT;
            c.update(&solid, head, d, vel, 9.0, DT);
        }
        let lag = c.pivot.distance(head);
        assert!(lag > 0.2 && lag <= MAX_LAG + 1e-4, "lag {lag}");
        // a step up of one block: the follow point glides up
        head.y += 1.0;
        let before = c.view.eye;
        let v = c.update(&solid, head, d, vel, 9.0, DT);
        assert!(
            (v.eye.y - before.y) < 0.3,
            "{} up in a frame",
            v.eye.y - before.y
        );
        // a teleport: at once
        head += Vec3::new(50.0, 0.0, 0.0);
        let v = c.update(&solid, head, d, Vec3::ZERO, 9.0, DT);
        assert!(v.eye.distance(head + boom(d) * 9.0) < 1e-3);
    }

    #[test]
    fn running_leads_the_view() {
        let solid = world(None);
        let d = dir_north(-0.2);
        let mut c = Camera::default();
        let mut head = Vec3::new(0.5, 1.62, 0.5);
        let vel = Vec3::new(6.2, 0.0, 0.0); // strafing right
        for _ in 0..180 {
            head += vel * DT;
            c.update(&solid, head, d, vel, 9.0, DT);
        }
        let side = c.view.dir.x;
        assert!(side > 0.02 && side < 0.12, "lead {side}");
    }

    #[test]
    fn the_shake_is_added_and_kept_out_of_walls() {
        let solid = world(None);
        let d = dir_north(-0.2);
        let mut c = Camera::default();
        let head = Vec3::new(0.5, 1.62, 0.5);
        let a = c.update(&solid, head, d, Vec3::ZERO, 9.0, DT).eye;
        c.shake = Vec3::new(0.1, 0.0, 0.0);
        let b = c.update(&solid, head, d, Vec3::ZERO, 9.0, DT).eye;
        assert!((b - a - c.shake).length() < 1e-3);
        c.shake = Vec3::new(0.0, -50.0, 0.0);
        let e = c.update(&solid, head, d, Vec3::ZERO, 9.0, DT).eye;
        assert!(clear(&solid, e, MARGIN - 1e-3));
    }

    #[test]
    fn auto_follow_waits_and_is_off_unless_asked() {
        let mut c = Camera::default();
        let mut orbit = 1.0;
        for _ in 0..600 {
            c.follow(&mut orbit, false, true, DT);
        }
        assert_eq!(orbit, 1.0, "off: the camera stays where it was turned");
        c.turned();
        for _ in 0..120 {
            c.follow(&mut orbit, true, true, DT);
        }
        assert_eq!(orbit, 1.0, "waits after a turn");
        for _ in 0..240 {
            c.follow(&mut orbit, true, true, DT);
        }
        assert!(orbit.abs() < 0.1, "swung back: {orbit}");
        // the short way round
        let mut o = 6.0;
        c.follow(&mut o, true, true, DT);
        assert!(o < 0.0 && o > -0.3, "{o}");
    }

    #[test]
    fn the_wheel_covers_two_to_twenty_five_metres() {
        let mut z = DEFAULT_ZOOM;
        for _ in 0..40 {
            z = wheel(z, -1.0);
        }
        assert_eq!(z, MAX_ZOOM);
        for _ in 0..40 {
            z = wheel(z, 1.0);
            if z < 0.5 {
                break;
            }
            assert!(z >= MIN_ZOOM);
        }
        assert_eq!(z, 0.0, "past the nearest: first person");
        assert_eq!(wheel(0.0, -1.0), MIN_ZOOM);
        assert_eq!(saved(f32::NAN), DEFAULT_ZOOM);
        assert_eq!(saved(80.0), MAX_ZOOM);
    }

    #[test]
    fn spring_is_exact_and_does_not_overshoot() {
        let (mut x, mut v) = (0.0, 0.0);
        for _ in 0..100 {
            (x, v) = spring(x, v, 1.0, 16.0, 0.1);
            assert!(x <= 1.0 + 1e-5);
        }
        assert!((x - 1.0).abs() < 1e-4);
        let _ = v;
    }
}
