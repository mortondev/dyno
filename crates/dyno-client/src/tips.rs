//! The tutorial layer of a player's first half hour: a short tip the first
//! time something happens (docs/design/ui.md).
//!
//! The tips are data: `tips.list` in the pack's `ui/strings.json`. Each has
//! an `id`, the event that shows it (`on`), the part of the HUD it points
//! at (`anchor`), a `title` and a `text`; optionally the event that puts it
//! away (`until`) and how long it stays (`secs`). The engine only knows
//! event and anchor names: it fires events as the game goes, and shows each
//! tip once per install (the ids seen are kept in the settings).

use crate::theme;
use egui::{Align2, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use std::collections::{HashSet, VecDeque};
use std::time::Instant;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct Tip {
    pub id: String,
    /// the event that shows it
    pub on: String,
    /// the event that puts it away early
    pub until: String,
    /// the HUD part it points at ("" or "center": none)
    pub anchor: String,
    pub title: String,
    pub text: String,
    /// seconds it stays up (0: the default)
    pub secs: f32,
}

/// How long a tip stays when the pack does not say.
const SECS: f32 = 14.0;
/// The quiet gap between two tips.
const GAP: f32 = 1.2;

#[derive(Default)]
pub struct Tips {
    list: Vec<Tip>,
    /// the asset generation `list` was read at
    read_at: Option<u64>,
    seen: HashSet<String>,
    queue: VecDeque<usize>,
    showing: Option<(usize, Instant)>,
    last_closed: Option<Instant>,
    /// the seen list changed since `take_seen`
    dirty: bool,
    pub enabled: bool,
}

impl Tips {
    pub fn new(seen: &[String], enabled: bool) -> Tips {
        Tips {
            seen: seen.iter().cloned().collect(),
            enabled,
            ..Tips::default()
        }
    }

    fn refresh(&mut self) {
        let generation = crate::assets::generation();
        // (u64::MAX: a list set by hand, in tests)
        if self.read_at == Some(generation) || self.read_at == Some(u64::MAX) {
            return;
        }
        self.read_at = Some(generation);
        let list: Vec<Tip> = crate::uitext::value("tips.list")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        // (the queue and the tip on show index the old list)
        if list.len() != self.list.len() {
            self.queue.clear();
            self.showing = None;
        }
        self.list = list;
    }

    /// Something happened: queue the tips for it that this player has not
    /// seen, and put away a tip that was waiting for it.
    pub fn fire(&mut self, event: &str) {
        self.refresh();
        if let Some((i, _)) = self.showing
            && self.list.get(i).is_some_and(|t| t.until == event)
        {
            self.close();
        }
        // a queued tip whose moment has passed is not worth showing
        let list = &self.list;
        let before = self.queue.len();
        let mut skipped = Vec::new();
        self.queue.retain(|i| {
            let gone = list[*i].until == event;
            if gone {
                skipped.push(list[*i].id.clone());
            }
            !gone
        });
        if self.queue.len() != before {
            self.seen.extend(skipped);
            self.dirty = true;
        }
        if !self.enabled {
            return;
        }
        for (i, t) in self.list.iter().enumerate() {
            let on_show = self.showing.is_some_and(|(k, _)| k == i);
            if t.on == event && !self.seen.contains(&t.id) && !self.queue.contains(&i) && !on_show {
                self.queue.push_back(i);
            }
        }
    }

    fn close(&mut self) {
        if self.showing.take().is_some() {
            self.last_closed = Some(Instant::now());
        }
    }

    /// Forget what has been seen (the options' "Show the tips again").
    pub fn reset(&mut self) {
        self.seen.clear();
        self.queue.clear();
        self.showing = None;
        self.dirty = true;
    }

    /// Show the next tip at once, as if it had been up for `age` (screenshots).
    pub fn backdate(&mut self, age: std::time::Duration) {
        self.refresh();
        if self.showing.is_none()
            && let Some(i) = self.queue.pop_front()
        {
            self.showing = Some((i, Instant::now() - age));
        }
    }

    /// How many tips this player has seen.
    pub fn seen_count(&self) -> usize {
        self.seen.len()
    }

    /// The seen list, when it has changed since the last call (to save).
    pub fn take_seen(&mut self) -> Option<Vec<String>> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        let mut v: Vec<String> = self.seen.iter().cloned().collect();
        v.sort();
        Some(v)
    }

    /// The tip on show, if any.
    pub fn current(&self) -> Option<&Tip> {
        self.showing.and_then(|(i, _)| self.list.get(i))
    }

    /// Draw the tip on show. `anchor` gives the rect of a named HUD part;
    /// `quiet` holds new tips back (a window is open, the intro plays).
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        anchor: impl Fn(&str) -> Option<Rect>,
        quiet: bool,
        dismiss_hint: &str,
    ) {
        self.refresh();
        if !self.enabled {
            self.showing = None;
            self.queue.clear();
            return;
        }
        if let Some((i, at)) = self.showing {
            let secs = self
                .list
                .get(i)
                .map_or(SECS, |t| if t.secs > 0.0 { t.secs } else { SECS });
            if at.elapsed().as_secs_f32() > secs {
                self.close();
            }
        }
        if self.showing.is_none()
            && !quiet
            && self
                .last_closed
                .is_none_or(|t| t.elapsed().as_secs_f32() > GAP)
            && let Some(i) = self.queue.pop_front()
        {
            self.showing = Some((i, Instant::now()));
            self.seen.insert(self.list[i].id.clone());
            self.dirty = true;
        }
        let Some((i, at)) = self.showing else { return };
        let Some(tip) = self.list.get(i).cloned() else {
            return;
        };
        let screen = ctx.content_rect();
        let target = anchor(&tip.anchor);
        let width = 330.0;
        // beside the part it points at, on the side with more room
        let (pos, pivot, arrow) = match target {
            Some(r) => {
                let c = r.center();
                if r.width() > r.height() * 2.5 && c.y > screen.center().y {
                    // a wide strip at the bottom (the action bar): above it
                    (
                        Pos2::new(c.x, r.top() - 18.0),
                        Align2::CENTER_BOTTOM,
                        Some((Pos2::new(c.x, r.top() - 4.0), Vec2::new(0.0, 1.0))),
                    )
                } else if c.x > screen.center().x {
                    (
                        Pos2::new(r.left() - 18.0, c.y.max(screen.top() + 60.0)),
                        Align2::RIGHT_CENTER,
                        Some((Pos2::new(r.left() - 4.0, c.y), Vec2::new(1.0, 0.0))),
                    )
                } else if c.y < screen.center().y && r.width() > 200.0 {
                    // a frame at the top left: under it
                    (
                        Pos2::new(r.left() + 30.0, r.bottom() + 18.0),
                        Align2::LEFT_TOP,
                        Some((
                            Pos2::new(r.left() + 60.0, r.bottom() + 4.0),
                            Vec2::new(0.0, -1.0),
                        )),
                    )
                } else {
                    (
                        Pos2::new(r.right() + 18.0, c.y),
                        Align2::LEFT_CENTER,
                        Some((Pos2::new(r.right() + 4.0, c.y), Vec2::new(-1.0, 0.0))),
                    )
                }
            }
            None => (
                Pos2::new(screen.center().x, screen.bottom() - 200.0),
                Align2::CENTER_BOTTOM,
                None,
            ),
        };
        let t = at.elapsed().as_secs_f32();
        let fade = (t / 0.35).clamp(0.0, 1.0);
        let resp = egui::Area::new(egui::Id::new("tip"))
            .order(egui::Order::Foreground)
            .pivot(pivot)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.set_opacity(fade);
                let inner = theme::tooltip_frame()
                    .stroke(Stroke::new(1.5_f32, theme::GOLD))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                            theme::medallion(
                                ui.painter(),
                                r.center(),
                                9.0,
                                Color32::from_rgb(40, 28, 18),
                                "!",
                                theme::GOLD_BRIGHT,
                            );
                            ui.label(
                                RichText::new(&tip.title)
                                    .font(theme::title_font(15.0))
                                    .color(theme::GOLD_BRIGHT),
                            );
                        });
                        ui.label(
                            RichText::new(&tip.text)
                                .font(theme::body_font(15.0))
                                .color(theme::TEXT),
                        );
                        if !dismiss_hint.is_empty() {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label(
                                    RichText::new(dismiss_hint)
                                        .font(theme::body_font(11.5))
                                        .color(theme::TEXT_DIM),
                                );
                            });
                        }
                    });
                ui.interact(
                    inner.response.rect,
                    egui::Id::new("tip-click"),
                    egui::Sense::click(),
                )
            });
        // an arrow from the panel to what it explains, bobbing a little
        if let Some((tip_at, dir)) = arrow {
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("tip-arrow"),
            ));
            let bob = (t * 3.2).sin() * 2.5;
            let at = tip_at - dir * (3.0 + bob);
            let side = Vec2::new(-dir.y, dir.x);
            let pts = vec![
                at + dir * 2.0,
                at - dir * 11.0 + side * 8.0,
                at - dir * 11.0 - side * 8.0,
            ];
            p.add(egui::Shape::convex_polygon(
                pts,
                theme::GOLD_BRIGHT.gamma_multiply(fade),
                Stroke::new(1.5_f32, theme::OUTLINE.gamma_multiply(fade)),
            ));
        }
        if resp.inner.clicked() {
            self.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tips() -> Tips {
        let mut t = Tips::new(&["old".to_string()], true);
        t.read_at = Some(u64::MAX);
        let tip = |id: &str, on: &str, until: &str| Tip {
            id: id.into(),
            on: on.into(),
            until: until.into(),
            ..Tip::default()
        };
        t.list = vec![
            tip("move", "spawn", "moved"),
            tip("talk", "spawn", ""),
            tip("old", "kill", ""),
        ];
        t
    }

    #[test]
    fn a_tip_is_queued_once_and_never_after_it_was_seen() {
        let mut t = tips();
        t.fire("spawn");
        t.fire("spawn");
        assert_eq!(t.queue.len(), 2);
        // seen in an earlier session
        t.fire("kill");
        assert_eq!(t.queue.len(), 2);
    }

    #[test]
    fn the_until_event_drops_a_waiting_tip_and_marks_it_seen() {
        let mut t = tips();
        t.fire("spawn");
        t.fire("moved");
        assert_eq!(t.queue.len(), 1);
        assert!(t.take_seen().unwrap().contains(&"move".to_string()));
        assert!(t.take_seen().is_none());
    }

    #[test]
    fn nothing_is_queued_when_tips_are_off() {
        let mut t = tips();
        t.enabled = false;
        t.fire("spawn");
        assert!(t.queue.is_empty());
    }
}
