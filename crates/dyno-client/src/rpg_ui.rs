//! RPG interface: quest givers, the quest log and tracker, unit frames and
//! the XP bar (docs/design/rpg.md).

use dyno_protocol::QuestProgress;
use dyno_sim::progression as prog;
use dyno_world::content::{Content, Objective, Quest};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
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

const GOLD: Color32 = Color32::from_rgb(255, 210, 80);
const PARCHMENT: Color32 = Color32::from_rgb(236, 226, 200);

fn rewards(ui: &mut egui::Ui, me: &Me, q: &Quest, choice: &mut u8) {
    let c = me.content;
    ui.add_space(6.0);
    ui.label(RichText::new("Rewards").strong().color(GOLD));
    let xp = prog::quest_xp(q.xp, me.level, q.level);
    let mut line = format!("{xp} experience");
    if q.money > 0 {
        line += &format!(" · {}", prog::money_text(q.money));
    }
    ui.label(line);
    for (i, n) in &q.items {
        ui.label(format!("{} × {n}", c.item(*i).name));
    }
    if !q.choice.is_empty() {
        ui.label(RichText::new("Choose one:").weak());
        ui.horizontal_wrapped(|ui| {
            for (k, (i, n)) in q.choice.iter().enumerate() {
                let label = if *n > 1 {
                    format!("{} × {n}", c.item(*i).name)
                } else {
                    c.item(*i).name.to_string()
                };
                ui.selectable_value(choice, k as u8, label);
            }
        });
    }
}

fn objectives(ui: &mut egui::Ui, me: &Me, q: &Quest) {
    ui.add_space(6.0);
    ui.label(RichText::new("Objectives").strong().color(GOLD));
    let p = me.progress(q.id);
    for (i, o) in q.objectives.iter().enumerate() {
        let n = p
            .map(|p| p.counts.get(i).copied().unwrap_or(0))
            .unwrap_or(0);
        let done = n >= o.need();
        let t = me.objective_text(o, n);
        ui.label(if done {
            RichText::new(t).color(Color32::from_rgb(140, 220, 140))
        } else {
            RichText::new(t)
        });
    }
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
    egui::Window::new(RichText::new(&n.name).color(GOLD).size(18.0))
        .id(egui::Id::new("npc-dialog"))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::LEFT_CENTER, Vec2::new(40.0, 0.0))
        .default_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            if !n.title.is_empty() {
                ui.label(RichText::new(&n.title).italics().weak());
            }
            match quest.and_then(|q| c.quest(q)) {
                None => {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&n.greeting).color(PARCHMENT));
                    ui.add_space(8.0);
                    let mut any = false;
                    for q in c.quests.iter() {
                        let (mark, show) = if q.ender == *npc && me.finished(q) {
                            ("?", true)
                        } else if q.giver == *npc && me.available(q) {
                            ("!", true)
                        } else if q.ender == *npc && me.progress(q.id).is_some() {
                            ("?", true)
                        } else {
                            ("", false)
                        };
                        if !show {
                            continue;
                        }
                        any = true;
                        let colour = if mark == "!" || me.finished(q) {
                            GOLD
                        } else {
                            Color32::GRAY
                        };
                        let text =
                            RichText::new(format!("{mark}  {}  (level {})", q.title, q.level))
                                .color(colour);
                        if ui.button(text).clicked() {
                            *quest = Some(q.id);
                            *choice = 0;
                        }
                    }
                    if !any {
                        ui.label(RichText::new("They have nothing for you right now.").weak());
                    }
                }
                Some(q) => {
                    ui.label(RichText::new(&q.title).strong().size(16.0));
                    ui.add_space(4.0);
                    let in_log = me.progress(q.id).is_some();
                    let finished = me.finished(q);
                    let text = if finished && q.ender == *npc {
                        &q.done
                    } else if in_log {
                        &q.progress
                    } else {
                        &q.text
                    };
                    ui.label(RichText::new(text).color(PARCHMENT));
                    if !(finished && q.ender == *npc) {
                        objectives(ui, me, q);
                    }
                    rewards(ui, me, q, choice);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if !in_log
                            && me.available(q)
                            && ui.button(RichText::new("Accept").strong()).clicked()
                        {
                            out.push(RpgCmd::Accept(q.id));
                            *quest = None;
                        }
                        if finished
                            && q.ender == *npc
                            && ui
                                .button(RichText::new("Complete quest").strong())
                                .clicked()
                        {
                            out.push(RpgCmd::Complete(q.id, *choice));
                            *quest = None;
                        }
                        if ui.button("Back").clicked() {
                            *quest = None;
                        }
                    });
                }
            }
        });
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
    egui::Window::new(RichText::new("Quest log").color(GOLD).size(18.0))
        .id(egui::Id::new("quest-log"))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.columns(2, |cols| {
                if me.log.is_empty() {
                    cols[0]
                        .label(RichText::new("No quests. Look for ! above people's heads.").weak());
                }
                for p in me.log {
                    let Some(q) = c.quest(p.quest) else { continue };
                    let done = me.finished(q);
                    let colour = level_color(me.level, q.level);
                    let label = format!(
                        "[{}] {}{}",
                        q.level,
                        q.title,
                        if done { " (complete)" } else { "" }
                    );
                    if cols[0]
                        .selectable_label(*quest == Some(q.id), RichText::new(label).color(colour))
                        .clicked()
                    {
                        *quest = Some(q.id);
                    }
                }
                if let Some(q) = quest
                    .and_then(|q| c.quest(q))
                    .filter(|q| me.progress(q.id).is_some())
                {
                    let ui = &mut cols[1];
                    ui.label(RichText::new(&q.title).strong().size(16.0));
                    ui.label(RichText::new(&q.text).color(PARCHMENT));
                    objectives(ui, me, q);
                    let mut ch = 0;
                    rewards(ui, me, q, &mut ch);
                    if let Some(n) = c.npc(q.ender) {
                        ui.label(RichText::new(format!("Return to {}", n.name)).weak());
                    }
                    ui.add_space(6.0);
                    if ui.button("Abandon").clicked() {
                        out.push(RpgCmd::Abandon(q.id));
                        *quest = None;
                    }
                }
            });
        });
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::L)) {
        out.push(RpgCmd::Close);
    }
    out
}

/// Objectives of the quests in the log, at the right of the screen.
pub fn tracker(painter: &egui::Painter, screen: Rect, me: &Me) {
    let mut y = screen.top() + 170.0;
    let x = screen.right() - 16.0;
    for p in me.log.iter().take(6) {
        let Some(q) = me.content.quest(p.quest) else {
            continue;
        };
        let done = me.finished(q);
        let title = if done {
            format!("{} (complete)", q.title)
        } else {
            q.title.clone()
        };
        painter.text(
            Pos2::new(x + 1.0, y + 1.0),
            Align2::RIGHT_TOP,
            &title,
            FontId::proportional(15.0),
            Color32::from_black_alpha(200),
        );
        painter.text(
            Pos2::new(x, y),
            Align2::RIGHT_TOP,
            &title,
            FontId::proportional(15.0),
            GOLD,
        );
        y += 19.0;
        if !done {
            for (i, o) in q.objectives.iter().enumerate() {
                let n = p.counts.get(i).copied().unwrap_or(0);
                let t = format!("- {}", me.objective_text(o, n));
                let colour = if n >= o.need() {
                    Color32::from_rgb(140, 220, 140)
                } else {
                    Color32::from_gray(230)
                };
                painter.text(
                    Pos2::new(x + 1.0, y + 1.0),
                    Align2::RIGHT_TOP,
                    &t,
                    FontId::proportional(13.0),
                    Color32::from_black_alpha(200),
                );
                painter.text(
                    Pos2::new(x, y),
                    Align2::RIGHT_TOP,
                    t,
                    FontId::proportional(13.0),
                    colour,
                );
                y += 16.0;
            }
        } else if let Some(n) = me.content.npc(q.ender) {
            let t = format!("Return to {}", n.name);
            painter.text(
                Pos2::new(x, y),
                Align2::RIGHT_TOP,
                t,
                FontId::proportional(13.0),
                Color32::from_gray(210),
            );
            y += 16.0;
        }
        y += 8.0;
    }
}

/// A unit frame: name, level and a health bar.
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
) {
    let r = Rect::from_min_size(at, Vec2::new(230.0, 54.0));
    painter.rect_filled(r, 6.0, Color32::from_black_alpha(160));
    painter.rect_stroke(
        r,
        6.0,
        Stroke::new(1.0_f32, Color32::from_white_alpha(40)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        r.min + Vec2::new(10.0, 6.0),
        Align2::LEFT_TOP,
        name,
        FontId::proportional(15.0),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(r.right() - 10.0, r.top() + 6.0),
        Align2::RIGHT_TOP,
        format!("{level}"),
        FontId::proportional(15.0),
        level_col,
    );
    let bar = Rect::from_min_size(r.min + Vec2::new(10.0, 28.0), Vec2::new(210.0, 16.0));
    painter.rect_filled(bar, 3.0, Color32::from_rgb(40, 16, 16));
    let f = if max > 0.0 {
        (health / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    painter.rect_filled(
        Rect::from_min_size(bar.min, Vec2::new(bar.width() * f, bar.height())),
        3.0,
        Color32::from_rgb(60, 170, 60),
    );
    painter.text(
        bar.center(),
        Align2::CENTER_CENTER,
        format!("{} / {}", health.ceil() as i32, max.round() as i32),
        FontId::proportional(12.0),
        Color32::WHITE,
    );
    if let Some(s) = sub {
        painter.text(
            Pos2::new(r.left() + 10.0, r.bottom() + 4.0),
            Align2::LEFT_TOP,
            s,
            FontId::proportional(12.0),
            GOLD,
        );
    }
}

/// The XP bar, centred at `at`, `width` wide (just above the hotbar).
pub fn xp_bar(painter: &egui::Painter, at: Pos2, width: f32, xp: u32, next: u32, level: u8) {
    let r = Rect::from_center_size(at, Vec2::new(width, 7.0));
    painter.rect_filled(r, 2.0, Color32::from_black_alpha(170));
    let f = if next > 0 {
        xp as f32 / next as f32
    } else {
        1.0
    };
    painter.rect_filled(
        Rect::from_min_size(r.min, Vec2::new(r.width() * f.clamp(0.0, 1.0), r.height())),
        2.0,
        Color32::from_rgb(150, 90, 220),
    );
    for k in 1..10 {
        let x = r.left() + r.width() * k as f32 / 10.0;
        painter.line_segment(
            [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
            Stroke::new(1.0_f32, Color32::from_black_alpha(120)),
        );
    }
    let text = if next > 0 {
        format!("Level {level}   {xp} / {next} XP")
    } else {
        format!("Level {level}")
    };
    painter.text(
        Pos2::new(r.right(), r.top() - 2.0),
        Align2::RIGHT_BOTTOM,
        text,
        FontId::proportional(11.0),
        Color32::from_gray(210),
    );
}
