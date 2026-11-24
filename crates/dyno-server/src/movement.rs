//! Server-authoritative movement (docs/design/netcode.md). A client sends
//! its movement as steps (keys, facing, milliseconds: `ClientMsg::Steps`);
//! the server runs every player's steps with the same `dyno_sim::movement`
//! code the client predicts with, and is the authority on where everyone is.
//!
//! - **No faster than real time.** Each player has a budget of real
//!   milliseconds. A step spends its time from it, and a step that finds it
//!   short waits for a later tick. Sending steps faster (a speed hack) only
//!   queues them.
//! - **Positions never come from the client,** so a teleport or a hover
//!   can't be asked for: the body falls and collides on the server.
//! - **A silent client is stood still.** When steps stop coming (a stall, a
//!   minimised window) the budget banks up to [`BANK_MS`], so a burst of late
//!   steps runs at once; when none have come for longer than that, the
//!   server runs idle steps itself, so the body falls and takes knockback.
//!
//! The answer is `ServerMsg::Moved` (the body after the last step run),
//! sent at the end of each tick in which the body changed.
//!
//! `DYNO_MOVE_LOG` logs what departs from plain running: steps dropped,
//! idle steps run for a silent client, steps answered unrun.

use crate::game::State;
use dyno_protocol::{Mode, PlayerId, Pose, ServerMsg};
use dyno_sim::movement::{MAX_STEP_MS, Player as Body, Step, apply_rules};
use std::collections::VecDeque;
use std::time::Instant;

/// The most real time a player may bank while no steps come: a burst of
/// late steps after a stall of up to this long runs at once.
pub const BANK_MS: f32 = 1000.0;
/// The most step time that may wait in a player's queue; older steps are
/// dropped beyond it (a client that sends faster than real time).
pub const QUEUE_MS: u32 = 2000;
/// A new body's budget, so its first steps run at once.
const START_MS: f32 = 200.0;

fn log_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("DYNO_MOVE_LOG").is_some())
}

/// The server's clock (milliseconds since it started): what creatures'
/// positions are stamped with (`EntityMoves`).
pub fn clock_ms() -> u32 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u32
}

/// One player's movement on the server: the body, and the steps on their way.
#[derive(Clone, Debug)]
pub struct Moves {
    pub body: Body,
    queue: VecDeque<(u32, Step)>,
    queued_ms: u32,
    /// real milliseconds the player may still spend
    budget: f32,
    /// milliseconds the server stood a silent player still: if the steps
    /// of that time come late after all (a burst), as many are answered,
    /// not run, so the queue never lags real time for good
    owed: u32,
    /// when steps last came, and how far idle steps have run since
    heard: Instant,
    idle_to: Instant,
    at: Instant,
    /// the last step run (acknowledged in `Moved`)
    pub seq: u32,
    /// the last step taken in (later ones only)
    last_in: Option<u32>,
    /// `Teleport`s sent to this player
    pub teleports: u8,
    /// milliseconds of movement run, idle steps too: the time the poses
    /// others see of this player are stamped with
    pub clock: u32,
    /// the body changed since the last `Moved`
    pub changed: bool,
    pub pitch: f32,
    facing: u16,
    /// steps refused (sent faster than real time), for the log
    pub dropped: u32,
    /// placed by trusted poses (build mode, tests) since the last steps:
    /// the server leaves the body where the pose put it
    posed: bool,
}

impl Moves {
    pub fn new(pos: [f32; 3], protected: bool, mode: Mode) -> Moves {
        let mut body = Body::new(pos);
        apply_rules(&mut body, protected, mode == Mode::Build);
        Moves {
            body,
            queue: VecDeque::new(),
            queued_ms: 0,
            budget: START_MS,
            owed: 0,
            heard: Instant::now(),
            idle_to: Instant::now(),
            at: Instant::now(),
            seq: 0,
            last_in: None,
            teleports: 0,
            clock: 0,
            changed: true,
            pitch: 0.0,
            facing: 0,
            dropped: 0,
            posed: false,
        }
    }

    /// Put the body somewhere (a teleport, a respawn): stopped, and the
    /// steps on their way forgotten (they were sent from somewhere else).
    pub fn put(&mut self, pos: [f32; 3]) {
        self.body.pos = pos.into();
        self.body.vel = glam::Vec3::ZERO;
        self.body.step_offset = 0.0;
        self.queue.clear();
        self.queued_ms = 0;
        self.owed = 0;
        self.changed = true;
    }

    /// Take in steps from the client (arrived `now`): those numbered after
    /// the last taken.
    pub fn take(&mut self, seq: u32, steps: &[Step], now: Instant) {
        if steps.is_empty() {
            return;
        }
        self.heard = now;
        if self.last_in.is_none() {
            // the first steps: no time banked before them (a join, a load)
            self.budget = self.budget.min(START_MS);
        }
        for (i, s) in steps.iter().enumerate() {
            let n = seq.wrapping_add(i as u32);
            if self.last_in.is_some_and(|l| n.wrapping_sub(l) as i32 <= 0) {
                continue;
            }
            self.last_in = Some(n);
            let s = Step {
                ms: s.ms.clamp(1, MAX_STEP_MS),
                ..*s
            };
            self.queued_ms += s.ms as u32;
            self.queue.push_back((n, s));
        }
        if self.queued_ms > QUEUE_MS && log_on() {
            crate::game::log(format!(
                "movement: {} ms of steps waiting, over {QUEUE_MS}: the oldest dropped",
                self.queued_ms
            ));
        }
        while self.queued_ms > QUEUE_MS {
            let Some((n, s)) = self.queue.pop_front() else {
                break;
            };
            self.queued_ms -= s.ms as u32;
            self.dropped += 1;
            // never run, but answered: the client's prediction moves on
            self.seq = n;
            self.changed = true;
        }
    }

    /// Steps the client sent before it took our latest teleport: they were
    /// sent from somewhere else. Answered, never run.
    pub fn skip(&mut self, seq: u32, count: usize) {
        if count == 0 {
            return;
        }
        let last = seq.wrapping_add(count as u32 - 1);
        if self.last_in.is_none_or(|l| last.wrapping_sub(l) as i32 > 0) {
            self.last_in = Some(last);
        }
        self.seq = last;
        self.changed = true;
    }

    /// Run what the budget allows (one tick). True if the body moved.
    pub fn run(&mut self, world: &dyno_world::World, now: Instant) -> bool {
        let real = now.duration_since(self.at).as_secs_f32() * 1000.0;
        self.at = now;
        self.budget = (self.budget + real).min(BANK_MS);
        let before = (self.body.pos, self.body.vel);
        // steps owed to a stand-still: skipped if they came as a burst (the
        // client caught up on the stall), forgotten if they did not
        if self.owed > 0 && !self.queue.is_empty() {
            if self.queued_ms <= self.owed {
                self.owed = 0;
            }
            while self.owed > 0 {
                let Some(&(n, s)) = self.queue.front() else {
                    break;
                };
                if (s.ms as u32) > self.owed {
                    self.owed = 0;
                    break;
                }
                self.queue.pop_front();
                self.queued_ms -= s.ms as u32;
                self.owed -= s.ms as u32;
                self.seq = n;
                self.changed = true;
                if log_on() {
                    crate::game::log(format!(
                        "movement: step {n} answered unrun ({} ms owed)",
                        self.owed
                    ));
                }
            }
        }
        while let Some(&(n, s)) = self.queue.front() {
            if self.budget < s.ms as f32 {
                break;
            }
            self.queue.pop_front();
            self.queued_ms -= s.ms as u32;
            self.budget -= s.ms as f32;
            self.body.step(world, s);
            self.facing = s.yaw;
            self.seq = n;
            self.clock = self.clock.wrapping_add(s.ms as u32);
            self.changed = true;
        }
        // nothing has come for longer than the bank: stand still meanwhile
        let since = self
            .idle_to
            .max(self.heard + std::time::Duration::from_secs_f32(BANK_MS / 1000.0));
        if self.queue.is_empty() && now > since {
            let mut idle = (now.duration_since(since).as_secs_f32() * 1000.0).round() as u32;
            self.idle_to = now;
            if log_on() && self.last_in.is_some() {
                crate::game::log(format!(
                    "movement: silent past the bank: {idle} ms stood still"
                ));
            }
            // (a client that has sent steps may send those of a stall when
            // it comes back, up to 2 s of them: predict.rs)
            if self.last_in.is_some() {
                self.owed = (self.owed + idle).min(QUEUE_MS);
            }
            while idle > 0 {
                let ms = idle.min(MAX_STEP_MS as u32) as u8;
                idle -= ms as u32;
                self.body.step(world, Step::idle(ms, self.facing));
                self.clock = self.clock.wrapping_add(ms as u32);
            }
        }
        let moved = (self.body.pos, self.body.vel) != before;
        self.changed |= moved;
        moved
    }

    /// The `Moved` answer, if the body changed since the last.
    pub fn answer(&mut self) -> Option<ServerMsg> {
        if !std::mem::take(&mut self.changed) {
            return None;
        }
        Some(ServerMsg::Moved {
            seq: self.seq,
            pos: self.body.pos.into(),
            vel: self.body.vel.into(),
            ground: self.body.on_ground,
        })
    }

    pub fn pose(&self) -> Pose {
        Pose {
            pos: self.body.pos.into(),
            yaw: self.body.yaw,
            pitch: self.pitch,
        }
    }
}

impl State {
    /// `ClientMsg::Steps` from a player.
    pub fn take_steps(
        &mut self,
        id: PlayerId,
        seq: u32,
        teleports: u8,
        pitch: f32,
        steps: &[Step],
    ) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        p.last_pose_at = Instant::now();
        if p.dead || p.mode == Mode::Build {
            return;
        }
        let m = &mut p.moves;
        if pitch.is_finite() {
            m.pitch = pitch.clamp(-1.6, 1.6);
        }
        let behind = m.teleports.wrapping_sub(teleports) as i8;
        if behind > 0 {
            // sent from before our latest teleport
            m.skip(seq, steps.len());
            return;
        }
        if behind < 0 {
            // a client that has counted more than we sent (we adopted it
            // from another instance): count from where it is
            m.teleports = teleports;
        }
        m.posed = false;
        m.take(seq, steps, Instant::now());
    }

    /// A trusted pose (build mode, or a server that trusts movement: tests).
    pub fn take_pose(&mut self, id: PlayerId, pose: Pose) {
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        if p.dead || !(p.mode == Mode::Build || self.trust_movement) {
            return;
        }
        let m = &mut p.moves;
        let now = Instant::now();
        m.clock = m
            .clock
            .wrapping_add((now.duration_since(p.last_pose_at).as_secs_f32() * 1000.0) as u32);
        m.body.pos = pose.pos.into();
        m.body.vel = glam::Vec3::ZERO;
        m.body.yaw = pose.yaw;
        m.pitch = pose.pitch;
        m.queue.clear();
        m.queued_ms = 0;
        m.owed = 0;
        m.posed = true;
        p.pose = pose;
        p.last_pose_at = now;
    }

    /// Run every player's steps (each tick, before the creatures move).
    pub fn tick_moves(&mut self) {
        let now = Instant::now();
        let protected = self.content.world.protected;
        let world = &self.world;
        for p in self.players.values_mut() {
            let m = &mut p.moves;
            if p.dead || p.mode == Mode::Build || m.posed {
                m.at = now;
                m.budget = START_MS;
                m.queue.clear();
                m.queued_ms = 0;
                m.owed = 0;
                continue;
            }
            apply_rules(&mut m.body, protected, false);
            m.run(world, now);
            p.pose = m.pose();
        }
    }

    /// `Moved` to each player whose body changed this tick (end of tick, so
    /// knockback dealt during it is in).
    pub fn send_moved(&mut self) {
        for p in self.players.values_mut() {
            if p.mode == Mode::Build {
                p.moves.changed = false;
                continue;
            }
            if let Some(m) = p.moves.answer() {
                p.send(&m);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dyno_sim::movement::Input;
    use dyno_world::World;
    use std::time::Duration;

    fn flat() -> World {
        let mut w = World::empty([64, 16, 64]);
        for x in 0..64 {
            for z in 0..64 {
                w.set(x, 4, z, dyno_world::STONE);
            }
        }
        w
    }

    fn run(ms: u8) -> Step {
        Step::new(
            ms,
            Input {
                forward: 1.0,
                ..Default::default()
            },
            0.0,
        )
    }

    /// Steps sent ten times faster than real time do not move the body
    /// faster: they wait in the queue, and the oldest are dropped.
    #[test]
    fn steps_run_no_faster_than_real_time() {
        let w = flat();
        let mut m = Moves::new([32.5, 5.0, 60.5], true, Mode::Play);
        let t0 = Instant::now();
        m.at = t0;
        m.budget = 0.0;
        // 5 s of running sent in the first half second
        let mut seq = 1;
        for i in 0..10 {
            let steps: Vec<Step> = (0..20).map(|_| run(25)).collect();
            m.take(seq, &steps, t0 + Duration::from_millis(50 * i));
            seq += steps.len() as u32;
            m.run(&w, t0 + Duration::from_millis(50 * (i + 1)));
        }
        let ran = 60.5 - m.body.pos.z;
        // half a second of real time: at most 0.5 s of running (6.2 m/s)
        assert!(ran < 6.4 * 0.5 + 0.05, "ran {ran} m in 0.5 s");
        assert!(m.dropped > 0, "steps beyond the queue are dropped");
        assert!(m.queued_ms <= QUEUE_MS);
    }

    /// Holding jump never lifts you more than a jump: the body is the
    /// server's, and it falls.
    #[test]
    fn jumping_never_flies() {
        let w = flat();
        let mut m = Moves::new([32.5, 5.0, 32.5], true, Mode::Play);
        let t0 = Instant::now();
        m.at = t0;
        let jump = Step::new(
            20,
            Input {
                jump: true,
                ..Default::default()
            },
            0.0,
        );
        let mut top: f32 = 0.0;
        for i in 0..150u32 {
            m.take(i + 1, &[jump], t0 + Duration::from_millis(20 * i as u64));
            m.run(&w, t0 + Duration::from_millis(20 * (i as u64 + 1)));
            top = top.max(m.body.pos.y - 5.0);
        }
        assert!(top < 0.7, "a jump clears about 0.6 m, not {top}");
    }

    /// A client that falls silent in the air still comes down, once the
    /// bank is spent.
    #[test]
    fn a_silent_body_falls() {
        let w = flat();
        let mut m = Moves::new([32.5, 12.0, 32.5], true, Mode::Play);
        let t0 = Instant::now();
        m.at = t0;
        for i in 1..=60u64 {
            m.run(&w, t0 + Duration::from_millis(50 * i));
        }
        assert!(m.body.pos.y < 5.1, "fell to the floor: {:?}", m.body.pos);
    }

    /// A stall longer than the bank: the server stands the body still, and
    /// as many of the late steps are answered without running, so the
    /// queue does not lag real time from then on.
    #[test]
    fn a_long_stall_is_not_run_twice() {
        let w = flat();
        let mut m = Moves::new([32.5, 5.0, 60.5], true, Mode::Play);
        let t0 = Instant::now();
        m.at = t0;
        m.budget = 0.0;
        m.take(0, &[Step::idle(1, 0)], t0);
        // silent for 1.5 s: half a second more than the bank is stood still
        m.run(&w, t0 + Duration::from_millis(1500));
        assert_eq!(m.owed, 500);
        // the late steps come: 1.5 s of running
        let steps: Vec<Step> = (0..75).map(|_| run(20)).collect();
        m.take(1, &steps, t0 + Duration::from_millis(1540));
        m.run(&w, t0 + Duration::from_millis(1550));
        assert_eq!(m.owed, 0);
        assert!(m.queue.is_empty(), "all answered at once");
        assert_eq!(m.seq, 75);
        let ran = 60.5 - m.body.pos.z;
        assert!(ran < 6.4 * 1.05 && ran > 5.0, "a second's running: {ran}");
    }

    /// A client that was silent for a while (joining, loading) and then
    /// sends steps in real time, in uneven bursts, has every one of them
    /// run: none stood still for, none skipped.
    #[test]
    fn steady_steps_after_a_silence_all_run() {
        let w = flat();
        let mut m = Moves::new([32.5, 5.0, 62.5], true, Mode::Play);
        let t0 = Instant::now();
        m.at = t0;
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        for i in 1..=60u64 {
            m.run(&w, at(50 * i));
        }
        let start = m.body.pos;
        // then 6 s of running, 100 ms of steps every other tick
        let mut seq = 1;
        for i in 61..=180u64 {
            if i % 2 == 0 {
                let steps: Vec<Step> = (0..5).map(|_| run(20)).collect();
                m.take(seq, &steps, at(50 * i - 10));
                seq += 5;
            }
            m.run(&w, at(50 * i));
        }
        let ran = start.z - m.body.pos.z;
        assert_eq!(m.owed, 0);
        assert_eq!(m.seq, seq - 1);
        assert!(ran > 6.2 * 5.9 && ran < 6.2 * 6.1, "{ran} m in 6 s");
    }

    /// Steps from before a teleport are answered but never run.
    #[test]
    fn steps_from_before_a_teleport_are_skipped() {
        let w = flat();
        let mut m = Moves::new([32.5, 5.0, 32.5], true, Mode::Play);
        m.put([10.5, 5.0, 10.5]);
        m.skip(1, 10);
        assert_eq!(m.seq, 10);
        m.take(5, &[run(20)], Instant::now());
        assert!(m.queue.is_empty(), "an old step is not taken");
        m.take(11, &[run(20)], Instant::now());
        m.run(&w, Instant::now());
        assert_eq!(m.seq, 11);
        assert!((m.body.pos.x - 10.5).abs() < 0.01);
    }
}
