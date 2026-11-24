//! The debug overlay (F3): what a frame costs and how the connection is
//! doing, for hand play and for whoever tunes the feel of the game
//! (docs/design/feel.md, F1). Off until F3 is pressed.
//!
//! - frames a second, and a frame's time over the last second: the mean
//!   and the worst;
//! - ping: the round trip of a `Ping` to the server and its `Pong` back,
//!   measured once a second while the overlay is open;
//! - the server tick the last `Pong` carried;
//! - movement corrections: the times the server has moved this player
//!   (`Teleport`) since joining. A blink, a charge and a recall count too:
//!   they are the same message;
//! - entities drawn: the creatures, people and bodies in the picture.
//!
//! Its words are the pack's (`debug` in `ui/strings.json`).

use super::*;
use crate::theme;
use std::cell::Cell;
use std::collections::VecDeque;

pub struct DebugState {
    pub open: bool,
    /// the frames of the last second: when each began and how long the one
    /// before it took, in milliseconds
    frames: VecDeque<(Instant, f32)>,
    /// the last round trip, in milliseconds, and the tick it brought
    ping: Option<f32>,
    tick: Option<u32>,
    /// the ping in the air: its number and when it left
    sent: Option<(u32, Instant)>,
    last_ping: Instant,
    nonce: u32,
    /// times the server has moved this player
    pub corrections: u32,
    /// figures put in the picture this frame (counted while drawing), and
    /// the count of the frame before (shown)
    pub drawing: Cell<u32>,
    drawn: u32,
}

impl Default for DebugState {
    fn default() -> Self {
        DebugState {
            open: false,
            frames: VecDeque::new(),
            ping: None,
            tick: None,
            sent: None,
            last_ping: Instant::now(),
            nonce: 0,
            corrections: 0,
            drawing: Cell::new(0),
            drawn: 0,
        }
    }
}

/// How often a ping leaves while the overlay is open, and when one counts
/// as lost.
const PING_EVERY: Duration = Duration::from_secs(1);
const PING_LOST: Duration = Duration::from_secs(5);

impl DebugState {
    /// A frame begins, `dt` seconds after the last. Returns a ping to send,
    /// when one is due.
    pub fn frame(&mut self, now: Instant, dt: f32) -> Option<ClientMsg> {
        self.frames.push_back((now, dt * 1e3));
        while self
            .frames
            .front()
            .is_some_and(|(t, _)| now.duration_since(*t) > Duration::from_secs(1))
        {
            self.frames.pop_front();
        }
        self.drawn = self.drawing.replace(0);
        if !self.open {
            return None;
        }
        if self.sent.is_some_and(|(_, at)| at.elapsed() > PING_LOST) {
            self.sent = None;
            self.ping = None;
        }
        if self.sent.is_none() && self.last_ping.elapsed() >= PING_EVERY {
            self.nonce = self.nonce.wrapping_add(1);
            self.sent = Some((self.nonce, now));
            self.last_ping = now;
            return Some(ClientMsg::Ping { nonce: self.nonce });
        }
        None
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        // (the first ping leaves on the next frame)
        self.last_ping = Instant::now() - PING_EVERY;
    }

    pub fn pong(&mut self, nonce: u32, tick: u32) {
        if let Some((n, at)) = self.sent
            && n == nonce
        {
            self.ping = Some(at.elapsed().as_secs_f32() * 1e3);
            self.sent = None;
        }
        self.tick = Some(tick);
    }

    /// Frames in the last second, and a frame's mean and worst time in
    /// milliseconds.
    pub fn frame_times(&self) -> (usize, f32, f32) {
        let n = self.frames.len();
        let sum: f32 = self.frames.iter().map(|f| f.1).sum();
        let worst = self.frames.iter().map(|f| f.1).fold(0.0, f32::max);
        (n, sum / n.max(1) as f32, worst)
    }

    /// The rows: a name and a value each.
    fn rows(&self) -> Vec<(String, String)> {
        use crate::uitext::{fill, t};
        let (fps, mean, worst) = self.frame_times();
        let none = t("debug.none");
        vec![
            (t("debug.fps"), fps.to_string()),
            (
                t("debug.frame"),
                fill(
                    &t("debug.frame_value"),
                    &[
                        ("mean", &format!("{mean:.1}")),
                        ("worst", &format!("{worst:.1}")),
                    ],
                ),
            ),
            (
                t("debug.ping"),
                self.ping.map_or(none.clone(), |ms| {
                    fill(&t("debug.ping_value"), &[("ms", &format!("{ms:.0}"))])
                }),
            ),
            (
                t("debug.tick"),
                self.tick.map_or(none, |tick| tick.to_string()),
            ),
            (t("debug.corrections"), self.corrections.to_string()),
            (t("debug.drawn"), self.drawn.to_string()),
        ]
    }
}

/// Draw the overlay with its top-left corner at `at`. `place` is the line
/// under the rows: the version, the address and where the player stands.
pub fn draw(painter: &egui::Painter, d: &DebugState, at: Pos2, place: &str) {
    if !d.open {
        return;
    }
    let rows = d.rows();
    let (line, pad, split) = (17.0, 12.0, 158.0);
    let font = theme::body_font(13.0);
    let width = 330.0;
    let height = pad * 2.0 + 22.0 + rows.len() as f32 * line + 8.0 + 2.0 * 15.0;
    let r = Rect::from_min_size(at, Vec2::new(width, height));
    theme::panel_frame(painter, r);
    crate::probe::mark("debug", r);
    let mut y = r.top() + pad;
    theme::shadow_text(
        painter,
        Pos2::new(r.left() + pad, y),
        Align2::LEFT_TOP,
        &crate::uitext::t("debug.title"),
        theme::bold_font(14.0),
        theme::GOLD_BRIGHT,
    );
    y += 22.0;
    for (name, value) in rows {
        painter.text(
            Pos2::new(r.left() + pad, y),
            Align2::LEFT_TOP,
            name,
            font.clone(),
            theme::TEXT_DIM,
        );
        painter.text(
            Pos2::new(r.left() + pad + split, y),
            Align2::LEFT_TOP,
            value,
            font.clone(),
            theme::TEXT,
        );
        y += line;
    }
    y += 6.0;
    theme::divider(
        painter,
        Pos2::new(r.center().x, y),
        width - pad * 2.0,
        theme::GOLD_DARK,
    );
    painter.text(
        Pos2::new(r.left() + pad, y + 5.0),
        Align2::LEFT_TOP,
        place,
        theme::body_font(11.5),
        theme::TEXT_DIM,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_times_cover_the_last_second() {
        let mut d = DebugState::default();
        let t0 = Instant::now();
        for i in 0..90 {
            let dt = if i == 80 { 0.040 } else { 0.0167 };
            d.frame(t0 + Duration::from_secs_f32(i as f32 * 0.0167), dt);
        }
        let (n, mean, worst) = d.frame_times();
        assert!((58..=62).contains(&n), "{n} frames in a second");
        assert!((worst - 40.0).abs() < 0.1, "worst {worst}");
        assert!(mean > 16.0 && mean < 18.0, "mean {mean}");
    }

    #[test]
    fn a_ping_leaves_only_while_open_and_one_at_a_time() {
        let mut d = DebugState::default();
        let now = Instant::now();
        assert_eq!(d.frame(now, 0.016), None);
        d.toggle();
        let Some(ClientMsg::Ping { nonce }) = d.frame(now, 0.016) else {
            panic!("no ping left");
        };
        // none more until the answer is back
        assert_eq!(d.frame(now, 0.016), None);
        d.pong(nonce, 77);
        assert!(d.ping.is_some());
        assert_eq!(d.tick, Some(77));
    }
}
