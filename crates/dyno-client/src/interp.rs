//! Remote smoothing (docs/design/netcode.md): other players and creatures
//! are drawn from a buffer of timestamped positions, about [`DELAY`] behind
//! the newest, so the jitter of their arrival never shows.
//!
//! Each position comes stamped with the sender's own clock: the server's
//! for creatures (`EntityMoves`), the moving player's movement clock for
//! players (`Poses`: the milliseconds of steps the server has run for
//! them, so their steps' timing survives the trip).
//!
//! A [`Clock`] keeps the time things are drawn at, on their clock. It aims
//! [`DELAY`] behind the newest position as positions usually arrive (the
//! median of the last two seconds), and is steered there by running a
//! little fast or slow, never by a jump. When positions stop coming (a lost
//! packet holds a TCP line for a quarter of a second) it eases to a stop
//! before it is [`EXTRAPOLATE`] past the newest, and catches up when they
//! come; nothing moves faster or slower than [`WARP`] off its speed for it.
//! A line that keeps stalling earns more delay (up to [`MORE`] on top),
//! given back slowly once it is clean: 100 ms on a good line.
//!
//! A [`Track`] keeps one mover's positions and gives the one at a time:
//! between two positions, a straight line; past the newest, carried on at
//! the last speed for at most [`EXTRAPOLATE`], then held.
//!
//! Times are seconds; [`now`] is this process's clock, and the rest reads
//! none (the tests pass their own).

use glam::Vec3;
use std::collections::VecDeque;

/// Seconds since this process first asked.
pub fn now() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
}

/// How far behind the newest position things are drawn (s), as positions
/// usually arrive.
pub const DELAY: f64 = 0.1;
/// How long past the newest position a mover is carried on (s).
pub const EXTRAPOLATE: f64 = 0.15;
/// A jump between two positions longer than this (m) is a teleport: drawn
/// at once, not slid along.
pub const TELEPORT: f32 = 8.0;
/// The most the drawn clock runs fast or slow to reach its aim: 3%.
pub const WARP: f64 = 0.03;
/// The most delay a stalling line earns on top of [`DELAY`] (s).
pub const MORE: f64 = 0.1;
/// Delay earned while drawn past the newest position (s a second), and
/// given back while not (s a second).
const EARN: f64 = 0.5;
const GIVE_BACK: f64 = 0.01;
/// Arrivals over this long (s) decide where the clock aims.
const WINDOW: f64 = 2.0;
/// How hard the clock steers to its aim: this much faster a second it is
/// behind (3% for 30 ms; never more than `WARP`).
const STEER: f64 = 1.0;
/// A clock this far off its aim (s) is put right at once (a new shard, a
/// long stall).
const RESET: f64 = 0.6;
/// Positions kept, at the most.
const KEEP: usize = 64;

/// The time things are drawn at, on the sender's clock.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    /// the time drawn (theirs)
    t: Option<f64>,
    /// ours when last drawn
    last: Option<f64>,
    /// the newest stamp that has come
    newest: Option<f64>,
    /// recent arrivals: when (ours) and ours less theirs
    arrivals: VecDeque<(f64, f64)>,
    /// delay earned by stalls, on top of `DELAY`
    more: f64,
}

impl Clock {
    /// Something stamped `stamp` (theirs, s) arrived `now` (ours, s).
    pub fn arrived(&mut self, now: f64, stamp: f64) {
        while self.arrivals.front().is_some_and(|(t, _)| now - t > WINDOW) {
            self.arrivals.pop_front();
        }
        self.arrivals.push_back((now, now - stamp));
        if self.newest.is_none_or(|n| stamp > n || n - stamp > 10.0) {
            self.newest = Some(stamp);
        }
    }

    /// Ours less theirs, as things usually arrive: the median of the window.
    fn offset(&self) -> Option<f64> {
        if self.arrivals.is_empty() {
            return None;
        }
        let mut o: Vec<f64> = self.arrivals.iter().map(|a| a.1).collect();
        o.sort_by(f64::total_cmp);
        Some(o[o.len() / 2])
    }

    /// The time (theirs) to draw at `now` (ours), steered toward `DELAY`
    /// behind the newest as positions usually arrive.
    pub fn render(&mut self, now: f64) -> Option<f64> {
        let dt = (now - self.last.unwrap_or(now)).clamp(0.0, 0.25);
        self.last = Some(now);
        // (while nothing new comes the aim runs away: no reset for that)
        let starved = |t: f64| self.newest.is_some_and(|n| t > n);
        if self.t.is_some_and(starved) {
            self.more = (self.more + dt * EARN).min(MORE);
        } else {
            self.more = (self.more - dt * GIVE_BACK).max(0.0);
        }
        let aim = now - self.offset()? - DELAY - self.more;
        let t = match self.t {
            Some(t) if (aim - t).abs() <= RESET || starved(t) => {
                // (how far off the aim it would be, run at its plain speed)
                let mut rate = 1.0 + ((aim - (t + dt)) * STEER).clamp(-WARP, WARP);
                // past the newest: ease to a stop before the cap (the
                // second half of it), rather than stop dead at it
                if let Some(n) = self.newest {
                    let past = (t - n) / EXTRAPOLATE;
                    if past > 0.5 {
                        rate *= (2.0 * (1.0 - past)).clamp(0.0, 1.0);
                    }
                }
                t + dt * rate
            }
            _ => aim,
        };
        self.t = Some(t);
        Some(t)
    }

    /// Forget everything (a new shard or instance: another clock).
    pub fn reset(&mut self) {
        *self = Clock::default();
    }
}

/// One mover's positions, and where it is drawn.
#[derive(Clone, Debug)]
pub struct Track {
    /// stamped positions (their clock, s), oldest first
    samples: VecDeque<(f64, Vec3, f32)>,
    drawn: Vec3,
    yaw: f32,
    vel: Vec3,
    /// the time (theirs) of the newest batch that came without this mover
    /// in it: it stood still since its last position
    still_since: Option<f64>,
    /// drawn past the newest position (carried on, or held): its
    /// positions are late
    beyond: bool,
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut d = b - a;
    while d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    while d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    a + d * t
}

impl Track {
    /// Standing at `pos` (no stamp yet).
    pub fn new(pos: Vec3, yaw: f32) -> Track {
        Track {
            samples: VecDeque::new(),
            drawn: pos,
            yaw,
            vel: Vec3::ZERO,
            still_since: None,
            beyond: false,
        }
    }

    /// A position stamped `stamp` (their clock, s).
    pub fn push(&mut self, stamp: f64, pos: Vec3, yaw: f32) {
        if let Some(&(t, p, _)) = self.samples.back() {
            if stamp < t - 10.0 {
                // a clock started again (another server): start again too
                self.snap(pos, yaw);
            } else if stamp < t {
                return;
            }
            if p.distance(pos) > TELEPORT {
                self.snap(pos, yaw);
            } else if stamp == t {
                self.samples.pop_back();
            } else if let Some(s) = self.still_since.filter(|s| *s > t && *s < stamp) {
                // it stood there until the last batch without it: it set
                // off from there then, not from where it stopped long ago
                let y = self.samples.back().map_or(yaw, |b| b.2);
                self.samples.push_back((s, p, y));
            }
        }
        self.samples.push_back((stamp, pos, yaw));
        self.still_since = None;
        while self.samples.len() > KEEP {
            self.samples.pop_front();
        }
    }

    /// A batch stamped `stamp` came and this mover was not in it: it has
    /// not moved since its last position (entities are sent only when they
    /// move).
    pub fn still_at(&mut self, stamp: f64) {
        if self.samples.back().is_some_and(|(t, ..)| *t < stamp) {
            self.still_since = Some(stamp);
        }
    }

    /// Drawn at `pos` from now on, with nothing to slide from.
    pub fn snap(&mut self, pos: Vec3, yaw: f32) {
        self.samples.clear();
        self.drawn = pos;
        self.yaw = yaw;
        self.vel = Vec3::ZERO;
        self.still_since = None;
    }

    /// Where it is drawn at `t` (their clock, s; None: no mapping yet).
    pub fn update(&mut self, t: Option<f64>) {
        let Some(t) = t else {
            if let Some(&(_, p, y)) = self.samples.back() {
                self.drawn = p;
                self.yaw = y;
            }
            return;
        };
        // drop what is behind us, keeping one position before `t`
        while self.samples.len() > 2 && self.samples[1].0 <= t {
            self.samples.pop_front();
        }
        let n = self.samples.len();
        if n == 0 {
            self.vel = Vec3::ZERO;
            return;
        }
        let (t0, p0, y0) = self.samples[0];
        if n == 1 || t <= t0 {
            self.drawn = p0;
            self.yaw = y0;
            self.vel = Vec3::ZERO;
            return;
        }
        let (t1, p1, y1) = self.samples[1];
        let span = (t1 - t0).max(1e-4);
        let vel = (p1 - p0) / span as f32;
        self.beyond = t > t1 && self.still_since.is_none_or(|s| s < t1);
        if t <= t1 {
            let k = ((t - t0) / span) as f32;
            self.drawn = p0.lerp(p1, k);
            self.yaw = lerp_angle(y0, y1, k);
            self.vel = vel;
            return;
        }
        // past the newest position: carried on a little, unless it is
        // known to have stopped there
        let still = self.still_since.is_some_and(|s| s >= t1);
        let ahead = if still {
            0.0
        } else {
            (t - t1).min(EXTRAPOLATE)
        };
        self.drawn = p1 + vel * ahead as f32;
        self.yaw = y1;
        self.vel = if still || t - t1 > EXTRAPOLATE {
            Vec3::ZERO
        } else {
            vel
        };
    }

    pub fn pos(&self) -> Vec3 {
        self.drawn
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Its speed as drawn (m/s).
    pub fn vel(&self) -> Vec3 {
        self.vel
    }

    /// Drawn past its newest position (its positions are late).
    pub fn beyond(&self) -> bool {
        self.beyond
    }

    /// The newest position, wherever it is drawn.
    pub fn newest(&self) -> Option<Vec3> {
        self.samples.back().map(|s| s.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny random number from a seed (no crate needed).
    fn noise(seed: &mut u64) -> f64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed % 10_000) as f64 / 10_000.0
    }

    /// A mover at a steady 6 m/s, sampled 20 times a second and arriving
    /// with up to 30 ms of jitter, is drawn at a steady 6 m/s.
    #[test]
    fn jitter_does_not_show_as_wobble() {
        let mut clock = Clock::default();
        let mut track = Track::new(Vec3::ZERO, 0.0);
        let mut seed = 7u64;
        let mut arrivals: Vec<(f64, f64)> = (0..200)
            .map(|i| {
                let stamp = i as f64 * 0.05;
                (stamp + 0.075 + noise(&mut seed) * 0.03, stamp)
            })
            .collect();
        arrivals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut next = 0;
        let mut last: Option<Vec3> = None;
        let mut worst: f32 = 0.0;
        let mut frame = 0;
        while frame < 580 {
            let now = frame as f64 / 60.0;
            while next < arrivals.len() && arrivals[next].0 <= now {
                let (at, stamp) = arrivals[next];
                clock.arrived(at, stamp);
                track.push(stamp, Vec3::new(stamp as f32 * 6.0, 0.0, 0.0), 0.0);
                next += 1;
            }
            track.update(clock.render(now));
            if frame > 60 {
                let v = (track.pos() - last.unwrap()).length() * 60.0;
                worst = worst.max((v - 6.0).abs());
            }
            last = Some(track.pos());
            frame += 1;
        }
        assert!(worst < 0.25, "drawn speed off by {worst} m/s");
    }

    /// A mover that stops is not carried past where it stopped.
    #[test]
    fn a_stop_is_not_overshot() {
        let mut track = Track::new(Vec3::ZERO, 0.0);
        for i in 0..10 {
            track.push(i as f64 * 0.05, Vec3::new(i as f32 * 0.3, 0.0, 0.0), 0.0);
        }
        // batches came later without it: it stood still
        track.still_at(0.6);
        track.update(Some(0.55));
        assert!((track.pos().x - 2.7).abs() < 1e-4, "{:?}", track.pos());
        // with nothing newer, it is carried on, but only so far
        let mut t2 = Track::new(Vec3::ZERO, 0.0);
        for i in 0..10 {
            t2.push(i as f64 * 0.05, Vec3::new(i as f32 * 0.3, 0.0, 0.0), 0.0);
        }
        t2.update(Some(5.0));
        assert!((t2.pos().x - (2.7 + 6.0 * 0.15)).abs() < 1e-3);
    }

    /// A mover that set off after a rest is drawn setting off when it did,
    /// not sliding from where it stopped long ago.
    #[test]
    fn a_start_after_a_rest_is_not_a_jump() {
        let mut track = Track::new(Vec3::ZERO, 0.0);
        track.push(0.0, Vec3::ZERO, 0.0);
        for i in 1..=40 {
            track.still_at(i as f64 * 0.05);
        }
        // it set off: 0.1 m in the next 50 ms
        track.push(2.05, Vec3::new(0.1, 0.0, 0.0), 0.0);
        track.update(Some(2.025));
        assert!(track.pos().x < 0.06, "{:?}", track.pos());
        assert!((track.vel().x - 2.0).abs() < 1e-3, "{:?}", track.vel());
    }

    /// A long jump is a teleport: drawn there at once.
    #[test]
    fn a_teleport_is_not_slid() {
        let mut track = Track::new(Vec3::ZERO, 0.0);
        track.push(0.0, Vec3::ZERO, 0.0);
        track.push(0.05, Vec3::new(0.3, 0.0, 0.0), 0.0);
        track.push(0.10, Vec3::new(100.0, 0.0, 0.0), 0.0);
        track.update(Some(0.06));
        assert_eq!(track.pos().x, 100.0);
    }

    /// The clock aims at the usual arrival and gets there by running a
    /// little fast or slow, never by a jump.
    #[test]
    fn the_clock_steers_and_never_jumps() {
        let mut c = Clock::default();
        c.arrived(1.0, 0.9);
        assert!((c.render(1.0).unwrap() - (0.9 - DELAY)).abs() < 1e-9);
        // arrivals turn 50 ms quicker: the drawn time catches up, by no
        // more than WARP of a frame at a time (over 0.6 s: it is put
        // right at once)
        let mut last = c.render(1.0).unwrap();
        for i in 1..=300 {
            let now = 1.0 + i as f64 / 60.0;
            c.arrived(now, now - 0.05);
            let t = c.render(now).unwrap();
            let step = t - last;
            assert!(
                (step - 1.0 / 60.0).abs() <= WARP / 60.0 + 1e-9,
                "a frame of {step}"
            );
            last = t;
        }
        // and it got there (the median moves once most arrivals are quicker)
        assert!((last - (6.0 - 0.05 - DELAY)).abs() < 0.01, "{last}");
    }

    /// A line that stalls earns more delay, up to `MORE`; a clean one gives
    /// it back slowly.
    #[test]
    fn stalls_earn_delay() {
        let mut c = Clock::default();
        let mut now = 0.0;
        let mut stamp = 0.0;
        for i in 0..1200 {
            now += 1.0 / 60.0;
            // a position every 50 ms, but nothing for 300 ms each second
            if i % 3 == 0 && (i % 60) > 18 {
                c.arrived(now + 0.08, stamp);
            }
            if i % 3 == 0 {
                stamp += 0.05;
            }
            c.render(now);
        }
        assert!(c.more > 0.05, "earned {}", c.more);
        let earned = c.more;
        for _ in 0..600 {
            now += 1.0 / 60.0;
            stamp += 1.0 / 60.0;
            c.arrived(now + 0.08, stamp);
            c.render(now);
        }
        assert!(c.more < earned && c.more > earned - 0.15, "{}", c.more);
    }

    /// With nothing new coming, the clock slows to a stop short of the
    /// cap, and runs on once positions come again.
    #[test]
    fn a_stall_eases_to_a_stop() {
        let mut c = Clock::default();
        for i in 0..40 {
            let t = i as f64 * 0.05;
            c.arrived(t + 0.08, t);
        }
        let newest = 39.0 * 0.05;
        let mut now = 39.0 * 0.05 + 0.08;
        let mut t = c.render(now).unwrap();
        for _ in 0..60 {
            now += 1.0 / 60.0;
            t = c.render(now).unwrap();
        }
        assert!(t <= newest + EXTRAPOLATE, "{t} past {newest}");
        assert!(t > newest + EXTRAPOLATE * 0.5);
        c.arrived(now, newest + 1.0);
        let before = t;
        now += 1.0 / 60.0;
        let after = c.render(now).unwrap();
        assert!(after > before);
    }
}
