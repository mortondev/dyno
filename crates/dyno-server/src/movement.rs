//! Movement checks. The client moves its own player (it feels instant), but
//! the server keeps it honest: no running faster than you can, no flying, no
//! teleporting. A move that breaks the rules is refused and the player is put
//! back where they last stood legitimately ("rubber-banding").

use crate::game::State;
use dyno_protocol::{Mode, PlayerId, Pose, ServerMsg};
use dyno_world as w;
use std::time::Instant;

/// Sprint speed (6.4) with slack for latency and knockback.
pub const MAX_SPEED: f32 = 8.5;
/// Distance that can be banked for a burst (lag spikes, knockback).
pub const BURST: f32 = 14.0;
/// How long you may stay in the air without falling.
pub const MAX_HANG: f32 = 1.6;

#[derive(Clone, Debug)]
pub struct MoveCheck {
    pub budget: f32,
    pub at: Instant,
    /// when we left the ground, and our height then
    pub air: Option<(Instant, f32)>,
    /// the last place we stood on something
    pub ground: [f32; 3],
    pub strikes: u32,
}

impl MoveCheck {
    pub fn new(pos: [f32; 3]) -> Self {
        MoveCheck {
            budget: BURST,
            at: Instant::now(),
            air: None,
            ground: pos,
            strikes: 0,
        }
    }
}

impl State {
    /// Something to stand on, or water to swim in, at or just below `pos`.
    fn supported(&self, pos: [f32; 3]) -> bool {
        let (x, z) = (pos[0].floor() as i32, pos[2].floor() as i32);
        let y = pos[1].floor() as i32;
        // feet can be a little off the block edge: check the corners too
        let corners = [
            (0.0, 0.0),
            (0.3, 0.3),
            (-0.3, 0.3),
            (0.3, -0.3),
            (-0.3, -0.3),
        ];
        corners.iter().any(|(dx, dz)| {
            let (cx, cz) = ((pos[0] + dx).floor() as i32, (pos[2] + dz).floor() as i32);
            (y - 2..=y).any(|yy| {
                let b = self.world.get(cx, yy, cz);
                w::is_solid(b) || b == w::WATER
            })
        }) || self.world.get(x, y, z) == w::WATER
    }

    /// Check a pose from a client. Returns the pose to keep, or None (and
    /// puts the player back) when it breaks the rules.
    pub fn check_pose(&mut self, id: PlayerId, pose: Pose) -> bool {
        let Some(p) = self.players.get(&id) else {
            return false;
        };
        if p.mode == Mode::Creative || self.trust_movement {
            return true;
        }
        let now = Instant::now();
        let from = p.pose.pos;
        let mut mc = p.moves.clone();
        let dt = now.duration_since(mc.at).as_secs_f32().min(2.0);
        mc.at = now;
        mc.budget = (mc.budget + MAX_SPEED * dt).min(BURST);
        let dx = pose.pos[0] - from[0];
        let dz = pose.pos[2] - from[2];
        let horizontal = (dx * dx + dz * dz).sqrt();
        let rise = pose.pos[1] - from[1];
        let mut bad = None;
        if horizontal > mc.budget {
            bad = Some("too fast");
        } else if rise > 2.2 + MAX_SPEED * dt {
            bad = Some("rose too fast");
        } else {
            mc.budget -= horizontal;
            if self.supported(pose.pos) {
                mc.air = None;
                mc.ground = pose.pos;
            } else {
                match mc.air {
                    None => mc.air = Some((now, pose.pos[1])),
                    Some((since, y0)) => {
                        // in the air a while and not coming down: flying
                        let hang = now.duration_since(since).as_secs_f32();
                        let fallen = y0 - pose.pos[1];
                        let should = 0.5 * 28.0 * (hang - 0.8).max(0.0).powi(2);
                        if hang > MAX_HANG && fallen < should.min(3.0) {
                            bad = Some("flying");
                        }
                    }
                }
            }
        }
        let p = self.players.get_mut(&id).unwrap();
        match bad {
            None => {
                p.moves = mc;
                true
            }
            Some(why) => {
                mc.strikes += 1;
                let back = if why == "flying" { mc.ground } else { from };
                if mc.strikes.is_power_of_two() {
                    crate::game::log(format!(
                        "Movement check: {} ({why}, {} times)",
                        p.name, mc.strikes
                    ));
                }
                mc.air = None;
                mc.budget = 0.0;
                p.moves = mc;
                p.pose.pos = back;
                p.send(&ServerMsg::Teleport { pos: back });
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_allows_running_but_not_teleporting() {
        let mut m = MoveCheck::new([0.0; 3]);
        // a second of running at sprint speed fits
        m.budget = (m.budget + MAX_SPEED).min(BURST);
        assert!(6.4 <= m.budget);
        // but not 40 blocks at once
        assert!(40.0 > m.budget);
    }
}
