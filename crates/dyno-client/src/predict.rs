//! Client-side prediction and reconciliation (docs/design/netcode.md).
//!
//! The server runs everyone's movement; the client must still move the
//! instant a key goes down. So the client runs its own steps at once with
//! the same `dyno_sim::movement` code, sends them (`ClientMsg::Steps`), and
//! keeps those not yet answered. When the server's body comes back
//! (`ServerMsg::Moved`, after step `seq`) the client puts its body there and
//! runs its later steps again. Where that lands differs from what was on
//! screen only when the two disagreed (a door that opened, a blow, a lost
//! race for a ledge): a small difference is eased away over about 120 ms
//! ([`Predictor::offset`]); a large one (over [`SNAP`]) is a snap.
//!
//! Used by the game, the bots and the tests alike; it reads no clock (time
//! comes in as `dt`). `DYNO_PREDICT_LOG` prints every correction over 5 cm.

use dyno_protocol::ClientMsg;
use dyno_sim::movement::{Input, MAX_STEP_MS, Player, Step};
use dyno_world::World;
use glam::Vec3;
use std::collections::VecDeque;

/// A correction larger than this (metres) is a snap: shown at once.
pub const SNAP: f32 = 2.0;
/// How fast a small correction is eased away: its time constant (s). After
/// three of them (120 ms) 95% of it is gone.
pub const EASE: f32 = 0.04;
/// How often steps are sent (s): 30 messages a second.
pub const SEND_EVERY: f32 = 1.0 / 30.0;
/// The most unanswered steps kept (about 10 s at 60 a second).
const MAX_PENDING: usize = 600;
/// The most time one call may turn into steps: the rest of a long stall
/// is dropped (the server stands a silent player still meanwhile).
const MAX_CATCH_UP_MS: f32 = 2000.0;

fn log_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("DYNO_PREDICT_LOG").is_some())
}

/// What the reconciliation has done, for the debug overlay and the tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// answers from the server
    pub acks: u32,
    /// corrections over [`SNAP`]: shown as a jump
    pub snaps: u32,
    /// corrections over 5 cm, eased away
    pub eased: u32,
    /// the largest correction (metres)
    pub worst: f32,
    /// the last correction (metres)
    pub last: f32,
    /// `Teleport`s taken (deliberate moves: not corrections)
    pub teleports: u32,
}

pub struct Predictor {
    next_seq: u32,
    /// steps run and not yet answered, oldest first
    pending: VecDeque<(u32, Step)>,
    /// steps run and not yet sent, the first numbered `out_seq`
    outbox: Vec<Step>,
    out_seq: u32,
    since_send: f32,
    /// a fraction of a millisecond not yet turned into a step
    carry: f32,
    teleports: u8,
    /// what the screen shows less where the body is: eased to nothing
    pub offset: Vec3,
    pub stats: Stats,
}

impl Default for Predictor {
    fn default() -> Self {
        Predictor::new()
    }
}

impl Predictor {
    pub fn new() -> Predictor {
        Predictor {
            next_seq: 1,
            pending: VecDeque::new(),
            outbox: Vec::new(),
            out_seq: 1,
            since_send: 0.0,
            carry: 0.0,
            teleports: 0,
            offset: Vec3::ZERO,
            stats: Stats::default(),
        }
    }

    /// Run `dt` seconds of `input` facing `yaw` (radians) as steps: the body
    /// moves now, and the steps wait to be sent. Where the world under the
    /// body has not streamed in yet the steps are idle (the server would
    /// walk you where you cannot see).
    pub fn advance(&mut self, body: &mut Player, world: &World, input: Input, yaw: f32, dt: f32) {
        let feet = [
            body.pos.x.floor() as i32,
            (body.pos.y - 0.5).floor() as i32,
            body.pos.z.floor() as i32,
        ];
        let input = if world.known(feet[0], feet[1], feet[2]) {
            input
        } else {
            Input::default()
        };
        let total = (dt.max(0.0) * 1000.0 + self.carry).min(MAX_CATCH_UP_MS);
        let mut whole = total.floor() as u32;
        self.carry = total - whole as f32;
        while whole > 0 {
            let ms = whole.min(MAX_STEP_MS as u32) as u8;
            whole -= ms as u32;
            let step = Step::new(ms, input, yaw);
            body.step(world, step);
            self.pending.push_back((self.next_seq, step));
            if self.outbox.is_empty() {
                self.out_seq = self.next_seq;
            }
            self.outbox.push(step);
            self.next_seq = self.next_seq.wrapping_add(1);
        }
        while self.pending.len() > MAX_PENDING {
            self.pending.pop_front();
        }
        self.offset *= (-dt.max(0.0) / EASE).exp();
        if self.offset.length_squared() < 1e-8 {
            self.offset = Vec3::ZERO;
        }
        self.since_send += dt.max(0.0);
    }

    /// The steps to send, when it is time (every [`SEND_EVERY`]).
    pub fn flush(&mut self, pitch: f32) -> Option<ClientMsg> {
        if self.outbox.is_empty() || self.since_send < SEND_EVERY {
            return None;
        }
        self.since_send = 0.0;
        Some(ClientMsg::Steps {
            seq: self.out_seq,
            teleports: self.teleports,
            pitch,
            steps: std::mem::take(&mut self.outbox),
        })
    }

    /// The server's body after step `seq`: put ours there and run the later
    /// steps again. Returns the correction (metres).
    pub fn ack(
        &mut self,
        body: &mut Player,
        world: &World,
        seq: u32,
        pos: [f32; 3],
        vel: [f32; 3],
        ground: bool,
    ) -> f32 {
        self.stats.acks += 1;
        while self
            .pending
            .front()
            .is_some_and(|(n, _)| n.wrapping_sub(seq) as i32 <= 0)
        {
            self.pending.pop_front();
        }
        let was = body.pos;
        let step_offset = body.step_offset;
        body.pos = pos.into();
        body.vel = vel.into();
        body.on_ground = ground;
        for (_, s) in &self.pending {
            body.step(world, *s);
        }
        // (the camera's glide up a step is the screen's, not the body's)
        body.step_offset = step_offset;
        let err = was - body.pos;
        let size = err.length();
        if size > 0.05 && log_on() {
            eprintln!(
                "prediction: corrected {size:.2} m after step {seq} ({} unanswered): \
                 server {pos:?} vel {vel:?} ground {ground}; predicted {was:?}",
                self.pending.len()
            );
        }
        self.stats.last = size;
        self.stats.worst = self.stats.worst.max(size);
        if size > SNAP || (self.offset + err).length() > SNAP {
            self.stats.snaps += 1;
            self.offset = Vec3::ZERO;
        } else {
            if size > 0.05 {
                self.stats.eased += 1;
            }
            self.offset += err;
        }
        size
    }

    /// The server moved us (`Teleport`): the body goes there, and steps
    /// run from where we were are forgotten (the server drops them too).
    pub fn teleport(&mut self, body: &mut Player, pos: [f32; 3]) {
        body.pos = pos.into();
        body.vel = Vec3::ZERO;
        body.step_offset = 0.0;
        self.teleports = self.teleports.wrapping_add(1);
        self.pending.clear();
        self.outbox.clear();
        self.offset = Vec3::ZERO;
        self.stats.teleports += 1;
    }

    /// Where to draw the body: where it is, plus what is still being eased.
    pub fn drawn(&self, body: &Player) -> Vec3 {
        body.pos + self.offset
    }

    /// Steps run and not yet answered.
    pub fn unanswered(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> World {
        let mut w = World::empty([64, 16, 64]);
        for x in 0..64 {
            for z in 0..64 {
                w.set(x, 4, z, dyno_world::STONE);
            }
        }
        w
    }

    fn forward() -> Input {
        Input {
            forward: 1.0,
            ..Default::default()
        }
    }

    /// A server that runs the same steps answers with the same body: no
    /// correction at all.
    #[test]
    fn the_same_steps_agree() {
        let w = flat();
        let mut me = Player::new([32.5, 5.0, 50.5]);
        let mut server = me;
        let mut pr = Predictor::new();
        let mut sent = Vec::new();
        for i in 0..120 {
            pr.advance(
                &mut me,
                &w,
                forward(),
                0.3,
                1.0 / 60.0 + (i % 3) as f32 * 0.001,
            );
            if let Some(ClientMsg::Steps { seq, steps, .. }) = pr.flush(0.0) {
                sent.push((seq, steps));
            }
            // the server answers every tenth frame, for what it has had
            if i % 10 == 9 {
                let mut last = 0;
                for (seq, steps) in sent.drain(..) {
                    for (k, s) in steps.iter().enumerate() {
                        server.step(&w, *s);
                        last = seq + k as u32;
                    }
                }
                if last > 0 {
                    let e = pr.ack(
                        &mut me,
                        &w,
                        last,
                        server.pos.into(),
                        server.vel.into(),
                        server.on_ground,
                    );
                    assert!(e < 1e-4, "no correction: {e}");
                }
            }
        }
        assert_eq!(pr.stats.snaps, 0);
        assert!(me.pos.z < 45.0, "it ran: {:?}", me.pos);
    }

    /// A small disagreement is eased away, not shown as a jump; a large one
    /// is a snap.
    #[test]
    fn small_errors_ease_and_large_ones_snap() {
        let w = flat();
        let mut me = Player::new([32.5, 5.0, 32.5]);
        let mut pr = Predictor::new();
        pr.advance(&mut me, &w, Input::default(), 0.0, 0.1);
        let shown = pr.drawn(&me);
        let e = pr.ack(&mut me, &w, 1, [32.5, 5.0, 33.0], [0.0; 3], true);
        assert!(e > 0.4 && e < 0.6);
        assert_eq!(pr.stats.snaps, 0);
        assert!(
            (pr.drawn(&me) - shown).length() < 1e-3,
            "the screen does not jump"
        );
        for _ in 0..9 {
            pr.advance(&mut me, &w, Input::default(), 0.0, 1.0 / 60.0);
        }
        assert!(pr.offset.length() < 0.5 * 0.05, "eased away in 150 ms");
        pr.ack(&mut me, &w, 100, [10.5, 5.0, 10.5], [0.0; 3], true);
        assert_eq!(pr.stats.snaps, 1);
        assert_eq!(pr.offset, Vec3::ZERO);
    }

    /// Frames of any length become steps of whole milliseconds that add up
    /// to the time that passed.
    #[test]
    fn steps_add_up_to_the_time() {
        let w = flat();
        let mut me = Player::new([32.5, 5.0, 32.5]);
        let mut pr = Predictor::new();
        let mut ms = 0u32;
        for i in 0..500 {
            let dt = [0.0069, 0.0167, 0.004, 0.12][i % 4];
            pr.advance(&mut me, &w, Input::default(), 0.0, dt);
            pr.since_send = 1.0;
            if let Some(ClientMsg::Steps { steps, .. }) = pr.flush(0.0) {
                assert!(steps.iter().all(|s| (1..=MAX_STEP_MS).contains(&s.ms)));
                ms += steps.iter().map(|s| s.ms as u32).sum::<u32>();
            }
        }
        let want = (0.0069 + 0.0167 + 0.004 + 0.12) * 125.0 * 1000.0;
        assert!((ms as f32 - want).abs() <= 1.0, "{ms} ms for {want}");
    }
}
