//! RPG interface: quest givers, the quest log and tracker, unit frames and
//! the XP bar (docs/design/rpg.md).

use crate::theme;
use dyno_protocol::QuestProgress;
use dyno_sim::progression as prog;
use dyno_world::content::{Content, Objective, Quest};
use egui::{Align2, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use std::collections::HashSet;

/// An open RPG window.
#[derive(Clone, Debug, PartialEq)]
pub enum Panel {
    /// talking to an NPC; `quest` is the quest being read, `choice` the picked reward
    Dialog {
        entity: u32,
        npc: u16,
        quest: Option<u16>,
        choice: u8,
    },
    /// the quest log; `quest` is the selected entry
    Log { quest: Option<u16> },
}

pub enum RpgCmd {
    Accept(u16),
    Complete(u16, u8),
    Abandon(u16),
    Close,
    Show(Panel),
}

/// What a player knows about their own character.
pub struct Me<'a> {
    pub content: &'a Content,
    pub level: u8,
    pub log: &'a [QuestProgress],
    pub done: &'a HashSet<u16>,
    /// where I stand, and the spawn hearth (explore places are relative to it)
    pub pos: [f32; 3],
    pub hearth: Option<[i32; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marker {
    /// a quest you can take
    Available,
    /// a quest you can hand in here
    Ready,
    /// a quest for this NPC, not finished yet
    Waiting,
}

impl Me<'_> {
    pub fn progress(&self, q: u16) -> Option<&QuestProgress> {
        self.log.iter().find(|x| x.quest == q)
    }
    pub fn finished(&self, q: &Quest) -> bool {
        self.progress(q.id).is_some_and(|p| {
            q.objectives
                .iter()
                .zip(&p.counts)
                .all(|(o, n)| *n >= o.need())
        })
    }
    pub fn available(&self, q: &Quest) -> bool {
        !self.done.contains(&q.id)
            && self.progress(q.id).is_none()
            && self.level + 1 >= q.level
            && q.requires.iter().all(|r| self.done.contains(r))
    }
    /// The marker above an NPC's head.
    pub fn marker(&self, npc: u16) -> Option<Marker> {
        let qs = &self.content.quests;
        if qs.iter().any(|q| q.ender == npc && self.finished(q)) {
            return Some(Marker::Ready);
        }
        if qs.iter().any(|q| q.giver == npc && self.available(q)) {
            return Some(Marker::Available);
        }
        if qs
            .iter()
            .any(|q| q.ender == npc && self.progress(q.id).is_some())
        {
            return Some(Marker::Waiting);
        }
        None
    }
    pub fn objective_text(&self, o: &Objective, n: u8) -> String {
        let c = self.content;
        match o {
            Objective::Kill { creature, count } => {
                let name = c
                    .creature(*creature)
                    .map(|c| c.name.as_str())
                    .unwrap_or("?");
                format!("{name} slain: {}/{count}", n.min(*count))
            }
            Objective::Collect { item, count } => {
                format!("{}: {}/{count}", c.item(*item).name, n.min(*count))
            }
            Objective::Talk { npc } => {
                let name = c.npc(*npc).map(|n| n.name.as_str()).unwrap_or("?");
                format!("Speak with {name}{}", if n > 0 { " (done)" } else { "" })
            }
            Objective::Explore { name, at, .. } => {
                if n > 0 {
                    return format!("Find {name} (done)");
                }
                match self.hearth {
                    Some(h) => {
                        let dx = (h[0] + at[0]) as f32 + 0.5 - self.pos[0];
                        let dz = (h[2] + at[2]) as f32 + 0.5 - self.pos[2];
                        let d = (dx * dx + dz * dz).sqrt();
                        // north is -z
                        let ang = dx.atan2(-dz).to_degrees().rem_euclid(360.0);
                        let dirs = [
                            "north",
                            "north-east",
                            "east",
                            "south-east",
                            "south",
                            "south-west",
                            "west",
                            "north-west",
                        ];
                        let dir = dirs[((ang + 22.5) / 45.0) as usize % 8];
                        format!("Find {name}: {d:.0} m {dir}")
                    }
                    None => format!("Find {name}"),
                }
            }
        }
    }
}

/// Colour of a level for this player: grey, green, yellow, orange or red.
pub fn level_color(me: u8, other: u8) -> Color32 {
    match prog::difficulty(me, other) {
        0 => Color32::from_rgb(150, 150, 150),
        1 => Color32::from_rgb(90, 200, 90),
        2 => Color32::from_rgb(240, 220, 90),
        3 => Color32::from_rgb(240, 150, 60),
        _ => Color32::from_rgb(230, 70, 60),
    }
}

const GOLD: Color32 = theme::GOLD_BRIGHT;

/// An item with its icon in a small slot; `selected` makes it a choice.
fn reward_item(
    ui: &mut egui::Ui,
    c: &Content,
    item: u16,
    n: u8,
    selected: Option<bool>,
) -> egui::Response {
    let name = c.item(item).name;
    let label = if n > 1 {
        format!("{name} × {n}")
    } else {
        name.to_string()
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label, theme::body_font(15.0), theme::INK);
    let size = Vec2::new(34.0 + 8.0 + galley.size().x + 8.0, 34.0);
    let sense = if selected.is_some() {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(size, sense);
    let p = ui.painter();
    let on = selected == Some(true);
    if on || resp.hovered() && selected.is_some() {
        p.rect_filled(
            r.expand(2.0),
            5.0,
            Color32::from_rgba_unmultiplied(150, 104, 30, if on { 70 } else { 36 }),
        );
    }
    if on {
        p.rect_stroke(
            r.expand(2.0),
            5.0,
            Stroke::new(1.0_f32, theme::INK_GOLD),
            egui::StrokeKind::Inside,
        );
    }
    let slot = Rect::from_min_size(r.min, Vec2::splat(34.0));
    theme::slot_well(p, slot);
    crate::survival::item_icon(p, slot.shrink(4.0), c, item);
    theme::slot_rim(p, slot, on);
    p.galley(
        Pos2::new(slot.right() + 8.0, r.center().y - galley.size().y * 0.5),
        galley,
        theme::INK,
    );
    resp
}

fn rewards(ui: &mut egui::Ui, me: &Me, q: &Quest, choice: &mut u8) {
    let c = me.content;
    ui.add_space(8.0);
    theme::heading(ui, "Rewards", theme::INK_GOLD);
    let xp = prog::quest_xp(q.xp, me.level, q.level);
    let mut line = format!("{xp} experience");
    if q.money > 0 {
        line += &format!("  ·  {}", prog::money_text(q.money));
    }
    ui.label(RichText::new(line).color(theme::INK));
    if !q.items.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for (i, n) in &q.items {
                reward_item(ui, c, *i, *n, None);
            }
        });
    }
    if !q.choice.is_empty() {
        ui.add_space(2.0);
        ui.label(
            RichText::new("Choose one of these:")
                .italics()
                .color(theme::INK_FADED),
        );
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
            for (k, (i, n)) in q.choice.iter().enumerate() {
                if reward_item(ui, c, *i, *n, Some(*choice == k as u8)).clicked() {
                    *choice = k as u8;
                }
            }
        });
    }
}

fn objectives(ui: &mut egui::Ui, me: &Me, q: &Quest) {
    ui.add_space(8.0);
    theme::heading(ui, "Objectives", theme::INK_GOLD);
    let p = me.progress(q.id);
    for (i, o) in q.objectives.iter().enumerate() {
        let n = p
            .map(|p| p.counts.get(i).copied().unwrap_or(0))
            .unwrap_or(0);
        let done = n >= o.need();
        let t = me.objective_text(o, n);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 18.0), egui::Sense::hover());
            theme::check_mark(ui.painter(), r, done, theme::INK_FADED, theme::INK_GREEN);
            ui.label(RichText::new(t).color(if done { theme::INK_GREEN } else { theme::INK }));
        });
    }
}

/// A clickable line for a quest: a medallion, the title and a note at the right.
#[allow(clippy::too_many_arguments)]
fn quest_row(
    ui: &mut egui::Ui,
    mark: &str,
    mark_col: Color32,
    title: &str,
    title_col: Color32,
    note: &str,
    note_col: Color32,
    selected: bool,
    hover: Color32,
) -> egui::Response {
    let (r, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::click());
    let p = ui.painter();
    if selected || resp.hovered() {
        p.rect_filled(
            r,
            4.0,
            hover.gamma_multiply(if selected { 1.0 } else { 0.55 }),
        );
    }
    if selected {
        p.rect_stroke(
            r,
            4.0,
            Stroke::new(1.0_f32, theme::GOLD_DARK),
            egui::StrokeKind::Inside,
        );
    }
    theme::medallion(
        p,
        Pos2::new(r.left() + 15.0, r.center().y),
        9.5,
        Color32::from_rgb(40, 28, 18),
        mark,
        mark_col,
    );
    let right = if note.is_empty() {
        r.right() - 6.0
    } else {
        p.text(
            Pos2::new(r.right() - 8.0, r.center().y),
            Align2::RIGHT_CENTER,
            note,
            theme::body_font(13.0),
            note_col,
        )
        .left()
            - 6.0
    };
    let clip = Rect::from_min_max(
        Pos2::new(r.left() + 30.0, r.top()),
        Pos2::new(right, r.bottom()),
    );
    p.with_clip_rect(clip).text(
        Pos2::new(r.left() + 32.0, r.center().y - 1.0),
        Align2::LEFT_CENTER,
        title,
        theme::bold_font(15.0),
        title_col,
    );
    resp
}

/// Talking to an NPC: the greeting, their quests, then one quest's text.
pub fn dialog_window(ctx: &egui::Context, me: &Me, panel: &mut Panel) -> Vec<RpgCmd> {
    let mut out = Vec::new();
    let Panel::Dialog {
        npc, quest, choice, ..
    } = panel
    else {
        return out;
    };
    let c = me.content;
    let Some(n) = c.npc(*npc) else {
        out.push(RpgCmd::Close);
        return out;
    };
    let mut open = true;
    theme::window(
        ctx,
        "npc-dialog",
        &n.name,
        &n.title,
        390.0,
        (Align2::LEFT_CENTER, Vec2::new(40.0, 0.0)),
        &mut open,
        |ui| {
            let shown = quest.and_then(|q| c.quest(q));
            theme::parchment(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(150.0);
                match shown {
                    None => {
                        ui.label(RichText::new(&n.greeting).color(theme::INK));
                        ui.add_space(6.0);
                        theme::ui_divider(ui, theme::INK_GOLD);
                        ui.add_space(2.0);
                        let mut any = false;
                        for q in c.quests.iter() {
                            let ready = q.ender == *npc && me.finished(q);
                            let (mark, bright) = if ready {
                                ("?", true)
                            } else if q.giver == *npc && me.available(q) {
                                ("!", true)
                            } else if q.ender == *npc && me.progress(q.id).is_some() {
                                ("?", false)
                            } else {
                                continue;
                            };
                            any = true;
                            let resp = quest_row(
                                ui,
                                mark,
                                if bright {
                                    GOLD
                                } else {
                                    Color32::from_gray(150)
                                },
                                &q.title,
                                if bright {
                                    theme::INK_GOLD
                                } else {
                                    theme::INK_FADED
                                },
                                &format!("Level {}", q.level),
                                theme::INK_FADED,
                                false,
                                Color32::from_rgba_unmultiplied(150, 104, 30, 60),
                            );
                            if resp.clicked() {
                                *quest = Some(q.id);
                                *choice = 0;
                            }
                        }
                        if !any {
                            ui.label(
                                RichText::new("They have nothing for you right now.")
                                    .italics()
                                    .color(theme::INK_FADED),
                            );
                        }
                    }
                    Some(q) => {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(&q.title)
                                    .font(theme::title_font(18.0))
                                    .color(theme::INK_GOLD),
                            );
                        });
                        theme::ui_divider(ui, theme::INK_GOLD);
                        ui.add_space(2.0);
                        let in_log = me.progress(q.id).is_some();
                        let finished = me.finished(q);
                        let text = if finished && q.ender == *npc {
                            &q.done
                        } else if in_log {
                            &q.progress
                        } else {
                            &q.text
                        };
                        ui.label(RichText::new(text).color(theme::INK));
                        if !(finished && q.ender == *npc) {
                            objectives(ui, me, q);
                        }
                        rewards(ui, me, q, choice);
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let Some(q) = shown else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Farewell").clicked() {
                            out.push(RpgCmd::Close);
                        }
                    });
                    return;
                };
                let in_log = me.progress(q.id).is_some();
                let big = |t: &str| {
                    egui::Button::new(RichText::new(t).font(theme::title_font(14.0)).color(GOLD))
                        .min_size(Vec2::new(120.0, 28.0))
                };
                if !in_log && me.available(q) && ui.add(big("Accept")).clicked() {
                    out.push(RpgCmd::Accept(q.id));
                    *quest = None;
                }
                if me.finished(q) && q.ender == *npc && ui.add(big("Complete")).clicked() {
                    out.push(RpgCmd::Complete(q.id, *choice));
                    *quest = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new("Back").min_size(Vec2::new(70.0, 28.0)))
                        .clicked()
                    {
                        *quest = None;
                    }
                });
            });
        },
    );
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        out.push(RpgCmd::Close);
    }
    out
}

/// The quest log (L).
pub fn log_window(ctx: &egui::Context, me: &Me, panel: &mut Panel) -> Vec<RpgCmd> {
    let mut out = Vec::new();
    let Panel::Log { quest } = panel else {
        return out;
    };
    let c = me.content;
    let mut open = true;
    let count = match me.log.len() {
        0 => String::new(),
        1 => "1 quest".to_string(),
        n => format!("{n} quests"),
    };
    theme::window(
        ctx,
        "quest-log",
        "Quest Log",
        &count,
        660.0,
        (Align2::CENTER_CENTER, Vec2::ZERO),
        &mut open,
        |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                // the list, on dark wood
                egui::Frame::new()
                    .fill(theme::WOOD_DARK)
                    .stroke(Stroke::new(1.0_f32, theme::BRONZE))
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(226.0);
                            ui.set_min_height(330.0);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            if me.log.is_empty() {
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "No quests yet. Look for a gold ! above people's heads.",
                                    )
                                    .italics()
                                    .color(theme::TEXT_DIM),
                                );
                            }
                            for p in me.log {
                                let Some(q) = c.quest(p.quest) else { continue };
                                let done = me.finished(q);
                                let resp = quest_row(
                                    ui,
                                    &format!("{}", q.level),
                                    level_color(me.level, q.level),
                                    &q.title,
                                    if done { theme::GOOD } else { GOLD },
                                    if done { "done" } else { "" },
                                    theme::GOOD,
                                    *quest == Some(q.id),
                                    Color32::from_rgb(70, 49, 31),
                                );
                                if resp.clicked() {
                                    *quest = Some(q.id);
                                }
                            }
                        });
                    });
                // the selected quest, on parchment
                ui.vertical(|ui| {
                    let picked = quest
                        .and_then(|q| c.quest(q))
                        .filter(|q| me.progress(q.id).is_some());
                    theme::parchment(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_min_height(292.0);
                        let Some(q) = picked else {
                            ui.add_space(110.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(if me.log.is_empty() {
                                        "Your journal is empty."
                                    } else {
                                        "Choose a quest to read it."
                                    })
                                    .italics()
                                    .color(theme::INK_FADED),
                                );
                            });
                            return;
                        };
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(&q.title)
                                    .font(theme::title_font(18.0))
                                    .color(theme::INK_GOLD),
                            );
                        });
                        theme::ui_divider(ui, theme::INK_GOLD);
                        ui.label(RichText::new(&q.text).color(theme::INK));
                        objectives(ui, me, q);
                        let mut ch = u8::MAX;
                        rewards(ui, me, q, &mut ch);
                        if let Some(n) = c.npc(q.ender) {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!("Return to {}.", n.name))
                                    .italics()
                                    .color(theme::INK_FADED),
                            );
                        }
                    });
                    if let Some(q) = picked {
                        ui.add_space(6.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new("Abandon quest")
                                        .min_size(Vec2::new(110.0, 26.0)),
                                )
                                .clicked()
                            {
                                out.push(RpgCmd::Abandon(q.id));
                                *quest = None;
                            }
                        });
                    }
                });
            });
        },
    );
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::L)) {
        out.push(RpgCmd::Close);
    }
    out
}

/// Objectives of the quests in the log, at the right of the screen.
pub fn tracker(painter: &egui::Painter, screen: Rect, me: &Me) {
    let shown = || {
        me.log
            .iter()
            .take(6)
            .filter_map(|p| me.content.quest(p.quest).map(|q| (p, q)))
    };
    // measure first, for the backdrop
    let mut height = 0.0;
    for (_, q) in shown() {
        height += 20.0 + 8.0;
        height += if me.finished(q) {
            17.0
        } else {
            17.0 * q.objectives.len() as f32
        };
    }
    if height == 0.0 {
        return;
    }
    let width = 250.0;
    let x = screen.right() - 22.0;
    let top = screen.top() + 150.0;
    let back = Rect::from_min_max(
        Pos2::new(x - width, top),
        Pos2::new(screen.right() - 8.0, top + height + 30.0),
    );
    // a soft backdrop that fades out towards the world
    let shade = |a: u8| Color32::from_rgba_unmultiplied(12, 8, 5, a);
    let (fade, solid) = back.split_left_right_at_fraction(0.4);
    theme::gradient_rect_h(painter, fade, shade(0), shade(130));
    theme::gradient_rect_h(painter, solid, shade(130), shade(175));
    theme::shadow_text(
        painter,
        Pos2::new(x, top + 4.0),
        Align2::RIGHT_TOP,
        "Quests",
        theme::title_font(14.0),
        theme::GOLD,
    );
    let rule = top + 24.0;
    theme::gradient_rect_h(
        painter,
        Rect::from_min_max(Pos2::new(x - width + 30.0, rule), Pos2::new(x, rule + 1.5)),
        theme::GOLD.gamma_multiply(0.0),
        theme::GOLD,
    );
    theme::diamond(
        painter,
        Pos2::new(x + 5.0, rule + 0.75),
        3.5,
        theme::GOLD,
        Stroke::NONE,
    );
    let mut y = top + 32.0;
    for (p, q) in shown() {
        let done = me.finished(q);
        theme::shadow_text(
            painter,
            Pos2::new(x, y),
            Align2::RIGHT_TOP,
            &q.title,
            theme::bold_font(15.0),
            if done { theme::GOOD } else { GOLD },
        );
        y += 20.0;
        if !done {
            for (i, o) in q.objectives.iter().enumerate() {
                let n = p.counts.get(i).copied().unwrap_or(0);
                let ok = n >= o.need();
                let r = theme::shadow_text(
                    painter,
                    Pos2::new(x - 16.0, y),
                    Align2::RIGHT_TOP,
                    &me.objective_text(o, n),
                    theme::body_font(13.5),
                    if ok {
                        theme::GOOD
                    } else {
                        Color32::from_rgb(236, 228, 210)
                    },
                );
                theme::check_mark(
                    painter,
                    Rect::from_center_size(
                        Pos2::new(x - 5.0, r.center().y + 1.0),
                        Vec2::splat(10.0),
                    ),
                    ok,
                    theme::TEXT_DIM,
                    theme::GOOD,
                );
                y += 17.0;
            }
        } else if let Some(n) = me.content.npc(q.ender) {
            let r = theme::shadow_text(
                painter,
                Pos2::new(x - 16.0, y),
                Align2::RIGHT_TOP,
                &format!("Return to {}", n.name),
                theme::body_font(13.5),
                Color32::from_rgb(236, 228, 210),
            );
            theme::medallion(
                painter,
                Pos2::new(x - 5.0, r.center().y + 1.0),
                6.0,
                Color32::from_rgb(40, 28, 18),
                "?",
                GOLD,
            );
            y += 17.0;
        }
        y += 8.0;
    }
}

/// A unit frame: a round portrait with the unit's initial, a level badge, a
/// name plate and a health bar; `resource` adds a mana or rage bar under it as
/// `(value, max, colour)`. It fills 230 x 54 from `at`; `sub` goes under it.
#[allow(clippy::too_many_arguments)]
pub fn unit_frame(
    painter: &egui::Painter,
    at: Pos2,
    name: &str,
    level: u8,
    level_col: Color32,
    health: f32,
    max: f32,
    sub: Option<&str>,
    resource: Option<(f32, f32, Color32)>,
) {
    let r = Rect::from_min_size(at, Vec2::new(230.0, 54.0));
    let pc = at + Vec2::new(27.0, 27.0);
    // the name plate
    let plate = Rect::from_min_max(at + Vec2::new(24.0, 3.0), r.max - Vec2::new(0.0, 3.0));
    painter.rect_filled(
        plate.expand(1.5).translate(Vec2::new(0.0, 2.0)),
        6.0,
        Color32::from_black_alpha(110),
    );
    painter.rect_filled(plate, 5.0, theme::WOOD_DARK);
    theme::gradient_rect(
        painter,
        plate.shrink(2.0),
        Color32::from_rgba_unmultiplied(54, 39, 26, 255),
        Color32::from_rgba_unmultiplied(26, 18, 13, 255),
    );
    painter.rect_stroke(
        plate,
        5.0,
        Stroke::new(1.5_f32, theme::OUTLINE),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        plate,
        5.0,
        Stroke::new(1.5_f32, theme::GOLD_DARK),
        egui::StrokeKind::Inside,
    );
    for y in [plate.top() + 1.0, plate.bottom() - 1.0] {
        theme::diamond(
            painter,
            Pos2::new(plate.right() - 9.0, y),
            3.0,
            theme::GOLD,
            Stroke::new(1.0_f32, theme::OUTLINE),
        );
    }
    let x0 = at.x + 59.0;
    let bw = r.right() - 8.0 - x0;
    let clip = Rect::from_min_max(Pos2::new(x0, plate.top()), plate.max - Vec2::new(6.0, 0.0));
    painter.with_clip_rect(clip).text(
        Pos2::new(x0 + 1.0, at.y + 7.0),
        Align2::LEFT_TOP,
        name,
        theme::title_font(13.0),
        theme::TEXT,
    );
    let hp = format!("{} / {}", health.ceil() as i32, max.round() as i32);
    match resource {
        Some((v, m, col)) => {
            theme::resource_bar(
                painter,
                Rect::from_min_size(Pos2::new(x0, at.y + 24.0), Vec2::new(bw, 13.0)),
                health,
                max,
                theme::HEALTH,
                &hp,
            );
            theme::resource_bar(
                painter,
                Rect::from_min_size(Pos2::new(x0, at.y + 39.0), Vec2::new(bw, 7.0)),
                v,
                m,
                col,
                "",
            );
        }
        None => theme::resource_bar(
            painter,
            Rect::from_min_size(Pos2::new(x0, at.y + 26.0), Vec2::new(bw, 16.0)),
            health,
            max,
            theme::HEALTH,
            &hp,
        ),
    }
    // the portrait: a gilded ring around a tinted disc with the initial
    let hash = name
        .bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    let tint = [
        Color32::from_rgb(44, 62, 84),
        Color32::from_rgb(70, 44, 40),
        Color32::from_rgb(42, 68, 52),
        Color32::from_rgb(64, 50, 78),
        Color32::from_rgb(78, 62, 36),
    ][(hash % 5) as usize];
    painter.circle_filled(
        pc + Vec2::new(0.0, 2.0),
        27.5,
        Color32::from_black_alpha(130),
    );
    painter.circle_filled(pc, 26.0, theme::OUTLINE);
    painter.circle_filled(pc, 24.5, theme::GOLD_DARK);
    painter.circle_stroke(pc, 23.6, Stroke::new(1.5_f32, theme::GOLD));
    painter.circle_filled(pc, 21.5, theme::OUTLINE);
    painter.circle_filled(pc, 20.5, tint);
    painter.circle_filled(
        pc - Vec2::new(0.0, 5.0),
        14.0,
        Color32::from_white_alpha(14),
    );
    let mut buf = [0u8; 4];
    let initial = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('?')
        .encode_utf8(&mut buf);
    theme::shadow_text(
        painter,
        pc - Vec2::new(0.0, 1.0),
        Align2::CENTER_CENTER,
        initial,
        theme::title_font(25.0),
        Color32::from_rgb(244, 230, 196),
    );
    // four studs on the ring
    for k in 0..4 {
        let a = std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::FRAC_PI_2;
        painter.circle_filled(
            pc + Vec2::new(a.cos(), a.sin()) * 24.0,
            1.8,
            theme::GOLD_BRIGHT,
        );
    }
    if level > 0 {
        let mut num = [0u8; 3];
        theme::medallion(
            painter,
            at + Vec2::new(45.0, 45.0),
            9.5,
            Color32::from_rgb(28, 20, 14),
            fmt_u8(level, &mut num),
            level_col,
        );
    }
    if let Some(s) = sub {
        theme::shadow_text(
            painter,
            Pos2::new(x0, r.bottom() + 2.0),
            Align2::LEFT_TOP,
            s,
            theme::body_font(13.0),
            GOLD,
        );
    }
}

/// A number as text without allocating.
fn fmt_u8(n: u8, buf: &mut [u8; 3]) -> &str {
    let mut i = 3;
    let mut n = n;
    loop {
        i -= 1;
        buf[i] = b'0' + n % 10;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    std::str::from_utf8(&buf[i..]).unwrap_or("?")
}

/// The XP bar, centred at `at`, `width` wide (just above the action bar):
/// twenty segments in a gold trim.
pub fn xp_bar(painter: &egui::Painter, at: Pos2, width: f32, xp: u32, next: u32, level: u8) {
    let r = Rect::from_center_size(at, Vec2::new(width, 9.0));
    painter.rect_filled(r, 2.0, Color32::from_rgb(16, 10, 20));
    let f = if next > 0 {
        (xp as f32 / next as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let inner = r.shrink(1.5);
    let fill = Rect::from_min_size(inner.min, Vec2::new(inner.width() * f, inner.height()));
    theme::gradient_rect(
        painter,
        fill,
        Color32::from_rgb(190, 130, 250),
        Color32::from_rgb(104, 52, 176),
    );
    if f > 0.0 && f < 1.0 {
        painter.line_segment(
            [fill.right_top(), fill.right_bottom()],
            Stroke::new(1.5_f32, Color32::from_rgb(236, 210, 255)),
        );
    }
    for k in 1..20 {
        let x = r.left() + r.width() * k as f32 / 20.0;
        let major = k % 5 == 0;
        painter.line_segment(
            [Pos2::new(x, r.top() + 1.0), Pos2::new(x, r.bottom() - 1.0)],
            Stroke::new(
                1.0_f32,
                if major {
                    theme::GOLD_DARK
                } else {
                    Color32::from_black_alpha(130)
                },
            ),
        );
    }
    painter.rect_stroke(
        r,
        2.0,
        Stroke::new(1.0_f32, theme::OUTLINE),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        r,
        2.0,
        Stroke::new(1.0_f32, theme::GOLD_DARK),
        egui::StrokeKind::Inside,
    );
    for x in [r.left(), r.right()] {
        theme::diamond(
            painter,
            Pos2::new(x, at.y),
            5.0,
            theme::GOLD,
            Stroke::new(1.0_f32, theme::OUTLINE),
        );
    }
    let text = if next > 0 {
        format!("Level {level}   {xp} / {next} XP")
    } else {
        format!("Level {level}")
    };
    theme::shadow_text(
        painter,
        Pos2::new(r.right() - 8.0, r.top() - 1.0),
        Align2::RIGHT_BOTTOM,
        &text,
        theme::body_font(12.0),
        Color32::from_rgb(226, 214, 190),
    );
}
