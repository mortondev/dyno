//! The RPG heads-up display: unit frames, the action bar, the moments (a
//! level gained, a quest step, loot), the tutorial tips, the death screen
//! and the game menu. Drawn with the theme's carved pieces; every word
//! comes from the pack (`uitext`). See docs/design/ui.md.

use super::*;
use crate::theme;
use crate::tips::Tips;
use crate::uitext::{f as tf, t};
use dyno_world::content::{ItemId, Resource};

/// The page of the game menu (Esc).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuPage {
    #[default]
    Main,
    Options,
    Controls,
}

/// A level just gained: the banner and what came with it.
pub struct LevelUp {
    pub at: Instant,
    pub level: u8,
    /// the abilities learned, and the bar key each was put on
    pub learned: Vec<(u16, Option<usize>)>,
    /// health before the level (the banner shows the gain)
    pub health_before: f32,
}

/// What the HUD remembers between frames.
pub struct HudState {
    pub tips: Tips,
    /// lines at the top centre (quest steps): when, text, colour
    pub notices: Vec<(Instant, String, Color32)>,
    /// things received, at the lower right: when, item, count
    pub loots: Vec<(Instant, ItemId, u32)>,
    pub level_up: Option<LevelUp>,
    /// abilities just learned: their bar slots glow for a while
    pub fresh: HashMap<u16, Instant>,
    /// what I carried and wore at the last count, and when that changed
    totals: Option<HashMap<ItemId, u32>>,
    totals_dirty: Option<Instant>,
    pub menu_page: MenuPage,
    /// where I stood when the HUD first drew (the "moved" event)
    start: Option<Vec3>,
    /// the named HUD parts, for the tips' arrows
    anchors: HashMap<&'static str, Rect>,
    /// events already fired this session (cheap to fire only once)
    fired: HashSet<&'static str>,
    /// the bar was loaded from the settings (or made) for this character
    pub bar_ready: bool,
    /// the quest log has arrived once (what it held then is not news)
    quests_known: bool,
    /// the world map (M) is open
    pub map_open: bool,
    /// tooltips held open for a screenshot: an item, an ability
    pinned: (Option<Stack>, Option<u16>),
}

impl HudState {
    pub fn new() -> HudState {
        let o = crate::options::interface();
        HudState {
            tips: Tips::new(&o.tips_seen, o.tips),
            notices: Vec::new(),
            loots: Vec::new(),
            level_up: None,
            fresh: HashMap::new(),
            totals: None,
            totals_dirty: None,
            menu_page: MenuPage::Main,
            start: None,
            anchors: HashMap::new(),
            fired: HashSet::new(),
            bar_ready: false,
            quests_known: false,
            map_open: false,
            pinned: (None, None),
        }
    }

    /// Fire a tip event at most once a session.
    pub fn once(&mut self, event: &'static str) {
        if self.fired.insert(event) {
            self.tips.fire(event);
        }
    }

    pub fn notice(&mut self, text: String, colour: Color32) {
        self.notices.push((Instant::now(), text, colour));
        if self.notices.len() > 5 {
            self.notices.remove(0);
        }
    }
}

const NOTICE_YELLOW: Color32 = Color32::from_rgb(255, 216, 64);

fn rgb(c: [f32; 3]) -> Color32 {
    Color32::from_rgb(
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn resource_colour(r: Resource) -> Color32 {
    match r {
        Resource::Rage => theme::RAGE,
        Resource::Mana => theme::MANA,
        Resource::Energy => theme::ENERGY,
    }
}

// ---- the action bar's contents: saved per character ----

const KEY_RECALL: &str = "@recall";
const KEY_EAT: &str = "@eat";

fn bar_keys(p: &Play) -> Vec<String> {
    p.bar
        .iter()
        .map(|s| match s {
            Some(dyno_protocol::RECALL) => KEY_RECALL.to_string(),
            Some(BAR_EAT) => KEY_EAT.to_string(),
            Some(a) => p
                .content
                .ability(*a)
                .map(|a| a.key.clone())
                .unwrap_or_default(),
            None => String::new(),
        })
        .collect()
}

/// The bar as this character left it, else the default; abilities learned
/// since (and not on it) go into its empty slots.
pub fn load_bar(p: &mut Play) {
    let saved = crate::options::interface().bars.get(&p.my_name).cloned();
    let class = p.class;
    let mut bar: Vec<Option<u16>> = match saved {
        Some(keys) if keys.len() == 12 => keys
            .iter()
            .map(|k| match k.as_str() {
                "" => None,
                KEY_RECALL => Some(dyno_protocol::RECALL),
                KEY_EAT => Some(BAR_EAT),
                k => p
                    .content
                    .abilities
                    .iter()
                    .find(|a| a.key == k && Some(a.class) == class)
                    .map(|a| a.id),
            })
            .collect(),
        _ => default_bar(p),
    };
    if let Some(c) = class {
        for a in p.content.known_abilities(c, p.level) {
            if !bar.contains(&Some(a.id))
                && let Some(i) = bar.iter().take(10).position(|s| s.is_none())
            {
                bar[i] = Some(a.id);
            }
        }
    }
    p.bar = bar;
    p.ui.bar_ready = true;
}

/// Remember the bar for this character (the app saves the settings).
pub fn save_bar(p: &Play, actions: &mut Vec<Action>) {
    let keys = bar_keys(p);
    let name = p.my_name.clone();
    if crate::options::change_interface(|o| {
        o.bars.insert(name, keys);
    }) {
        actions.push(Action::Interface);
    }
}

// ---- things that happen (called as messages arrive) ----

/// I reached `level`: the banner, and each new ability onto the first free
/// key of the bar.
pub fn level_up(p: &mut Play, level: u8) {
    let mut learned = Vec::new();
    if let Some(c) = p.class {
        if !p.ui.bar_ready {
            load_bar(p);
        }
        let new: Vec<u16> = p
            .content
            .abilities
            .iter()
            // (every level gained at once teaches its own)
            .filter(|a| {
                a.class == c && (a.level == level || (a.level > p.level && a.level < level))
            })
            .map(|a| a.id)
            .collect();
        for a in new {
            let at = p.bar.iter().position(|s| *s == Some(a)).or_else(|| {
                let i = p.bar.iter().take(10).position(|s| s.is_none())?;
                p.bar[i] = Some(a);
                Some(i)
            });
            p.ui.fresh.insert(a, Instant::now());
            learned.push((a, at));
        }
    }
    if !learned.is_empty() {
        p.ui.tips.fire("ability");
        // (after the level's own fanfare)
        crate::audio::emit_in("ui.ability_new", 2.4);
    }
    // the banner is the one "learned" line on screen; the record of it is
    // in the combat log
    for (a, _) in &learned {
        if let Some(ab) = p.content.ability(*a) {
            p.combat_log.push((
                Instant::now(),
                tf(
                    "hud.log_learned",
                    &[("ability", &ab.name), ("text", &ab.text)],
                ),
                theme::GOLD_BRIGHT,
            ));
        }
    }
    p.ui.tips.fire("level");
    p.ui.level_up = Some(LevelUp {
        at: Instant::now(),
        level,
        learned,
        health_before: p.max_health,
    });
}

/// The quest log changed: a line for each step made, quest taken, finished
/// or handed in. `old` and `old_done` are the log and the finished quests
/// before. True when a quest became ready to hand in (the caller chimes).
pub fn quests_changed(
    p: &mut Play,
    old: &[dyno_protocol::QuestProgress],
    old_done: &HashSet<u16>,
) -> bool {
    // what the log holds when it first arrives (at a join) is not news
    if !std::mem::replace(&mut p.ui.quests_known, true) {
        return false;
    }
    let me = rpg_ui::Me {
        content: &p.content,
        money: p.money,
        level: p.level,
        log: &p.quest_log,
        done: &p.quests_done,
        pos: p.player.pos.into(),
        hearth: p.hearth,
        equip: &p.equip,
        class: p.class.and_then(|c| p.content.class(c)),
    };
    let mut lines: Vec<(String, Color32)> = Vec::new();
    let mut events: Vec<&'static str> = Vec::new();
    let mut ready = false;
    for now in &p.quest_log {
        let Some(q) = p.content.quest(now.quest) else {
            continue;
        };
        let before = old.iter().find(|o| o.quest == now.quest);
        let Some(before) = before else {
            lines.push((
                tf("hud.quest_accepted", &[("quest", &q.title)]),
                NOTICE_YELLOW,
            ));
            events.push("quest_accepted");
            continue;
        };
        for (i, o) in q.objectives.iter().enumerate() {
            let n = now.counts.get(i).copied().unwrap_or(0);
            let was = before.counts.get(i).copied().unwrap_or(0);
            if n > was {
                let done = n >= o.need();
                lines.push((
                    me.objective_text(o, n),
                    if done { theme::GOOD } else { NOTICE_YELLOW },
                ));
                events.push("quest_step");
            }
        }
        if quest_finished(&p.content, now) && !quest_finished(&p.content, before) {
            let to = p
                .content
                .npc(q.ender)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            lines.push((
                tf("hud.quest_ready", &[("quest", &q.title), ("npc", &to)]),
                theme::GOOD,
            ));
            events.push("quest_ready");
            ready = true;
        }
    }
    for q in p.quests_done.iter().filter(|q| !old_done.contains(q)) {
        if let Some(q) = p.content.quest(*q) {
            lines.push((
                tf("hud.quest_done", &[("quest", &q.title)]),
                theme::GOLD_BRIGHT,
            ));
            events.push("quest_done");
        }
    }
    for (text, colour) in lines {
        p.ui.notice(text, colour);
    }
    // the weightiest change is the one heard
    let sound = ["quest_done", "quest_ready", "quest_accepted", "quest_step"]
        .into_iter()
        .find(|e| events.contains(e));
    match sound {
        Some("quest_done") => crate::audio::emit("ui.quest_complete"),
        Some("quest_ready") => crate::audio::emit("ui.quest_ready"),
        Some("quest_accepted") => crate::audio::emit("ui.quest_accept"),
        Some(_) => crate::audio::emit("ui.quest_step"),
        None => {}
    }
    for e in events {
        p.ui.tips.fire(e);
    }
    ready
}

/// My bags or my gear changed: count again a moment later (a swap arrives
/// as two messages), and show what I gained.
pub fn carried_changed(p: &mut Play) {
    p.ui.totals_dirty = Some(Instant::now());
}

fn carried(p: &Play) -> HashMap<ItemId, u32> {
    let mut m: HashMap<ItemId, u32> = HashMap::new();
    for s in p.pack.iter().chain(p.equip.iter()).flatten() {
        *m.entry(s.item).or_default() += s.count as u32;
    }
    m
}

fn count_loot(p: &mut Play) {
    let Some(at) = p.ui.totals_dirty else { return };
    if at.elapsed() < Duration::from_millis(180) {
        return;
    }
    p.ui.totals_dirty = None;
    let now = carried(p);
    if let Some(before) = &p.ui.totals {
        let mut gear = false;
        let mut any = false;
        for (item, n) in &now {
            let was = before.get(item).copied().unwrap_or(0);
            if *n > was {
                any = true;
                gear |= p.content.item(*item).gear.is_some();
                p.ui.loots.push((Instant::now(), *item, n - was));
            }
        }
        while p.ui.loots.len() > 6 {
            p.ui.loots.remove(0);
        }
        if any {
            crate::audio::emit("ui.loot");
        }
        if gear {
            p.ui.tips.fire("gear");
        } else if any {
            p.ui.tips.fire("item");
        }
    }
    p.ui.totals = Some(now);
}

/// I was healed (the server says by how much): one green number over my
/// head, one line in the combat log.
pub fn healed(p: &mut Play, amount: f32) {
    if amount < 0.5 {
        return;
    }
    crate::audio::emit("combat.heal");
    p.combat_text
        .push((Instant::now(), None, format!("+{amount:.0}"), theme::GOOD));
    p.combat_log.push((
        Instant::now(),
        tf("hud.log_heal", &[("n", &format!("{amount:.0}"))]),
        theme::GOOD,
    ));
}

/// The picture of what a bar slot holds (`id`: an ability, Recall or Eat).
fn action_icon(painter: &egui::Painter, r: Rect, p: &Play, id: Option<u16>) {
    match id {
        Some(dyno_protocol::RECALL) => {
            item_ui::item_icon(painter, r, &p.content, dyno_world::HEARTH as ItemId)
        }
        Some(BAR_EAT) => {
            let pack = dyno_sim::inventory::Inventory {
                slots: p.pack.clone(),
            };
            if let Some(food) =
                dyno_sim::inventory::best_food(&pack, &p.content).and_then(|s| p.pack[s])
            {
                item_ui::item_icon(painter, r, &p.content, food.item);
            }
        }
        Some(a) => {
            if let Some(a) = p.content.ability(a) {
                item_ui::ability_icon(painter, r, &p.content, a);
            }
        }
        None => {}
    }
}

// ---- drawing ----

/// A tooltip beside the pointer for a painted (not egui) part of the HUD.
fn hover_tip(ctx: &egui::Context, id: &str, rect: Rect, add: impl FnOnce(&mut egui::Ui)) {
    let Some(at) = ctx.pointer_hover_pos() else {
        return;
    };
    if !rect.contains(at) {
        return;
    }
    egui::Area::new(egui::Id::new(("hud-tip", id)))
        .order(egui::Order::Tooltip)
        .fixed_pos(at + Vec2::new(14.0, 18.0))
        .constrain(true)
        .interactable(false)
        .show(ctx, |ui| theme::tooltip(ui, add));
}

/// Player, target and party frames; combo points; buffs.
pub fn frames(ctx: &egui::Context, painter: &egui::Painter, p: &mut Play, map: Rect) {
    let screen = ctx.content_rect();
    let class = p.class.and_then(|c| p.content.class(c));
    let at = screen.left_top() + Vec2::new(14.0, 14.0);
    let resource = class
        .filter(|_| p.power_max > 0.0)
        .map(|c| (p.power, p.power_max, resource_colour(c.resource)));
    rpg_ui::unit_frame(
        painter,
        at,
        &p.my_name,
        p.level,
        theme::GOLD_BRIGHT,
        p.health,
        p.max_health,
        None,
        resource,
    );
    let mine = Rect::from_min_size(at, Vec2::new(230.0, 54.0));
    p.ui.anchors.insert("frame", mine);
    // an absorb shield: a pale band over the health bar for its share of
    // your health, and what it will still take beside your name
    if p.shield.0 > 0.5 {
        let (x0, x1) = (at.x + 59.0, at.x + 222.0);
        let y = at.y + if resource.is_some() { 22.0 } else { 26.0 };
        let share = (p.shield.0 / p.max_health.max(1.0)).clamp(0.04, 1.0);
        let band = Rect::from_min_max(
            Pos2::new(x1 - (x1 - x0) * share, y - 1.0),
            Pos2::new(x1, y + 4.0),
        );
        let pale = Color32::from_rgb(236, 226, 170);
        painter.rect_filled(band.expand(1.0), 2.0, theme::OUTLINE);
        theme::gradient_rect(painter, band, Color32::WHITE, pale);
        let text = tf("hud.shield", &[("n", &format!("{:.0}", p.shield.0))]);
        let r = theme::shadow_text(
            painter,
            Pos2::new(x1 - 2.0, at.y + if resource.is_some() { 6.0 } else { 8.0 }),
            Align2::RIGHT_TOP,
            &text,
            theme::bold_font(12.0),
            pale,
        );
        theme::diamond(
            painter,
            Pos2::new(r.left() - 7.0, r.center().y),
            4.0,
            pale,
            Stroke::new(1.0_f32, theme::OUTLINE),
        );
        hover_tip(ctx, "shield", mine, |ui| {
            ui.label(RichText::new(t("hud.shield_text")).color(theme::TEXT));
        });
    }
    // the target, beside it
    let target_at = at + Vec2::new(250.0, 0.0);
    let picked = p.focus.or(p.target_entity);
    let mut has_target = false;
    if let Some(e) = picked.and_then(|id| p.entities.get(&id)) {
        let found = match e.kind {
            EntityKind::Creature(c) => p.content.creature(c).map(|cr| {
                let (l, h, m) = e.info.unwrap_or((1, 1.0, 1.0));
                let (name, sub) = match &e.name {
                    Some(n) => (n.clone(), Some(tf("hud.elite", &[("kind", &cr.name)]))),
                    None => (cr.name.clone(), None),
                };
                let col = if cr.hostile {
                    rpg_ui::level_color(p.level, l)
                } else {
                    theme::GOLD_BRIGHT
                };
                (name, sub, l, col, h, m)
            }),
            EntityKind::Npc(n) => p.content.npc(n).map(|n| {
                let sub = (!n.title.is_empty()).then(|| n.title.clone());
                (n.name.clone(), sub, 0, theme::GOOD, 1.0, 1.0)
            }),
            _ => None,
        };
        if let Some((name, sub, level, col, h, m)) = found {
            has_target = true;
            rpg_ui::unit_frame(
                painter,
                target_at,
                &name,
                level,
                col,
                h,
                m,
                sub.as_deref(),
                None,
            );
            p.ui.anchors.insert(
                "target",
                Rect::from_min_size(target_at, Vec2::new(230.0, 54.0)),
            );
            // what players have put on it: each effect's picture and the
            // seconds it has left, under the frame
            let now = Instant::now();
            let row = target_at + Vec2::new(59.0, if sub.is_some() { 76.0 } else { 58.0 });
            let auras = picked.and_then(|id| p.entity_auras.get(&id));
            for (k, (a, until)) in auras
                .into_iter()
                .flatten()
                .filter(|(_, until)| *until > now)
                .take(6)
                .enumerate()
            {
                let Some(ab) = p.content.ability(*a) else {
                    continue;
                };
                let r =
                    Rect::from_min_size(row + Vec2::new(k as f32 * 30.0, 0.0), Vec2::splat(26.0));
                theme::slot_well(painter, r);
                item_ui::ability_icon(painter, r.shrink(3.0), &p.content, ab);
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(190, 60, 48)),
                    egui::StrokeKind::Inside,
                );
                let left = until.saturating_duration_since(now).as_secs_f32();
                theme::shadow_text(
                    painter,
                    r.center_bottom() + Vec2::new(0.0, 1.0),
                    Align2::CENTER_TOP,
                    &format!("{left:.0}"),
                    theme::bold_font(11.0),
                    theme::TEXT,
                );
                hover_tip(ctx, "debuff", r, |ui| {
                    theme::tooltip_title(ui, &ab.name, Color32::from_rgb(255, 150, 130));
                    ui.label(RichText::new(&ab.text).color(theme::TEXT));
                });
            }
        }
    }
    if !has_target {
        p.ui.anchors.remove("target");
    }
    // combo points along the bottom of the target frame
    if p.combo.0 > 0 && p.combo.1.is_some() && p.combo.1 == picked {
        for k in 0..5u8 {
            let c = target_at + Vec2::new(247.0, 5.0 + k as f32 * 11.0);
            let on = k < p.combo.0;
            theme::diamond(
                painter,
                c,
                6.0,
                if on {
                    theme::EMBER
                } else {
                    Color32::from_black_alpha(160)
                },
                Stroke::new(
                    1.0_f32,
                    if on {
                        theme::GOLD_BRIGHT
                    } else {
                        theme::GOLD_DARK
                    },
                ),
            );
        }
    }
    // party frames, under my own
    let my_name = p.my_name.clone();
    for (i, m) in p.party.iter().filter(|m| m.name != my_name).enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(at.x + 6.0, at.y + 70.0 + i as f32 * 44.0),
            Vec2::new(176.0, 38.0),
        );
        painter.rect_filled(
            r.translate(Vec2::new(0.0, 2.0)),
            5.0,
            Color32::from_black_alpha(110),
        );
        painter.rect_filled(r, 5.0, theme::WOOD_DARK);
        theme::gradient_rect(
            painter,
            r.shrink(2.0),
            Color32::from_rgb(54, 39, 26),
            Color32::from_rgb(26, 18, 13),
        );
        painter.rect_stroke(
            r,
            5.0,
            Stroke::new(1.5_f32, theme::GOLD_DARK),
            egui::StrokeKind::Inside,
        );
        let col = m
            .class
            .and_then(|c| p.content.class(c))
            .map(|c| rgb(c.color))
            .unwrap_or(theme::TEXT);
        let label = if m.online {
            format!("{}  {}", m.name, m.level)
        } else {
            tf("hud.offline", &[("name", &m.name)])
        };
        if m.leader {
            theme::diamond(
                painter,
                r.right_top() + Vec2::new(-10.0, 9.0),
                3.5,
                theme::GOLD_BRIGHT,
                Stroke::new(1.0_f32, theme::OUTLINE),
            );
        }
        theme::shadow_text(
            painter,
            r.left_top() + Vec2::new(8.0, 3.0),
            Align2::LEFT_TOP,
            &label,
            theme::title_font(12.0),
            if m.online { col } else { theme::TEXT_DIM },
        );
        theme::resource_bar(
            painter,
            Rect::from_min_size(r.left_top() + Vec2::new(8.0, 21.0), Vec2::new(160.0, 11.0)),
            m.health,
            m.max.max(1.0),
            theme::HEALTH,
            "",
        );
    }
    // buffs: a row to the left of the minimap, each with its time left
    let now = Instant::now();
    p.auras.retain(|(_, t)| *t > now);
    for (k, (a, until)) in p.auras.iter().enumerate() {
        let Some(ab) = p.content.ability(*a) else {
            continue;
        };
        let r = Rect::from_min_size(
            Pos2::new(map.left() - 50.0 - k as f32 * 44.0, screen.top() + 14.0),
            Vec2::splat(36.0),
        );
        theme::slot_well(painter, r);
        item_ui::ability_icon(painter, r.shrink(4.0), &p.content, ab);
        theme::slot_rim(painter, r, false);
        let left = until.saturating_duration_since(now).as_secs_f32();
        let txt = if left >= 60.0 {
            tf("hud.minutes", &[("n", &format!("{:.0}", left / 60.0))])
        } else {
            tf("hud.seconds", &[("n", &format!("{left:.0}"))])
        };
        theme::shadow_text(
            painter,
            r.center_bottom() + Vec2::new(0.0, 2.0),
            Align2::CENTER_TOP,
            &txt,
            theme::body_font(12.0),
            if left < 5.0 {
                Color32::from_rgb(255, 150, 120)
            } else {
                theme::TEXT
            },
        );
        hover_tip(ctx, "buff", r, |ui| {
            theme::tooltip_title(ui, &ab.name, theme::GOLD_BRIGHT);
            ui.label(RichText::new(&ab.text).color(theme::TEXT));
        });
    }
}

/// What a bar slot holds, to draw: icon item, tint, cooldown left (0..1),
/// usable, and whether it is the cast in progress.
struct SlotLook {
    /// nothing is on the key
    empty: bool,
    tint: Option<Color32>,
    cooldown: f32,
    /// seconds of its own cooldown left (shown as a number when long)
    seconds: f32,
    usable: bool,
    active: bool,
    fresh: f32,
}

fn slot_look(p: &Play, i: usize) -> SlotLook {
    let now = Instant::now();
    let own = |id: u16| {
        p.cooldowns
            .get(&id)
            .map(|(t, total)| {
                let left = t.saturating_duration_since(now).as_secs_f32();
                (left / total.max(0.01), left)
            })
            .unwrap_or((0.0, 0.0))
    };
    let casting = p.my_cast.as_ref().map(|c| c.0);
    match action_at(p, i) {
        ActionSlot::Ability(a) => {
            let (cd, secs) = own(a.id);
            let g = p.gcd.0.saturating_duration_since(now).as_secs_f32() / p.gcd.1.max(0.01);
            // too far from the target to use it
            let far = a.range > 0.0
                && p.focus
                    .or(p.target_entity)
                    .and_then(|id| p.entities.get(&id))
                    .filter(|e| matches!(e.kind, EntityKind::Creature(_)))
                    .is_some_and(|e| e.pos().distance(p.player.pos) > a.range + 1.0);
            let fresh =
                p.ui.fresh
                    .get(&a.id)
                    .map(|t| (1.0 - t.elapsed().as_secs_f32() / 12.0).max(0.0))
                    .unwrap_or(0.0);
            SlotLook {
                empty: false,
                tint: Some(rgb([
                    a.color[0] * 0.47,
                    a.color[1] * 0.47,
                    a.color[2] * 0.47,
                ])),
                cooldown: cd.max(g),
                seconds: secs,
                usable: p.power + 0.01 >= a.cost && !far,
                active: casting == Some(a.id),
                fresh,
            }
        }
        ActionSlot::Recall => {
            let (cd, secs) = own(dyno_protocol::RECALL);
            SlotLook {
                empty: false,
                tint: Some(Color32::from_rgb(63, 35, 14)),
                cooldown: cd,
                seconds: secs,
                usable: true,
                active: casting == Some(dyno_protocol::RECALL),
                fresh: 0.0,
            }
        }
        ActionSlot::Eat => {
            let pack = dyno_sim::inventory::Inventory {
                slots: p.pack.clone(),
            };
            let food = dyno_sim::inventory::best_food(&pack, &p.content)
                .and_then(|s| p.pack[s])
                .map(|s| s.item);
            SlotLook {
                empty: false,
                tint: None,
                cooldown: 0.0,
                seconds: 0.0,
                usable: food.is_some(),
                active: false,
                fresh: 0.0,
            }
        }
        ActionSlot::Empty => SlotLook {
            empty: true,
            tint: None,
            cooldown: 0.0,
            seconds: 0.0,
            usable: true,
            active: false,
            fresh: 0.0,
        },
    }
}

const SLOT: f32 = 46.0;
const SLOT_GAP: f32 = 4.0;

/// The 12-key action bar: carved frame, icons, cooldown sweeps, key hints,
/// tooltips; click to use, drag to rearrange, drag off to clear, and a drop
/// from the spellbook binds. Returns the rect of the bar.
pub fn action_bar(
    ctx: &egui::Context,
    painter: &egui::Painter,
    p: &mut Play,
    actions: &mut Vec<Action>,
) -> Rect {
    let screen = ctx.content_rect();
    if !p.ui.bar_ready && p.class.is_some() {
        load_bar(p);
    }
    let width = 12.0 * (SLOT + SLOT_GAP) + 8.0;
    let bar = Rect::from_min_size(
        Pos2::new(
            screen.center().x - width * 0.5,
            screen.bottom() - SLOT - 12.0 - 10.0,
        ),
        Vec2::new(width, SLOT + 12.0),
    );
    theme::action_bar_frame(painter, bar);
    p.ui.anchors.insert("bar", bar);
    p.ui.anchors.insert(
        "xp",
        Rect::from_min_max(
            Pos2::new(bar.left(), bar.top() - 16.0),
            Pos2::new(bar.right(), bar.top() - 2.0),
        ),
    );
    let slot_rect = |i: usize| {
        Rect::from_min_size(
            bar.min + Vec2::new(6.0 + i as f32 * (SLOT + SLOT_GAP), 6.0),
            Vec2::splat(SLOT),
        )
    };
    let dragging = egui::DragAndDrop::payload::<screens::BarDrag>(ctx).map(|d| *d);
    for (i, key) in BAR_KEYS.iter().enumerate() {
        let r = slot_rect(i);
        let look = slot_look(p, i);
        theme::slot_well(painter, r);
        if let Some(t) = look.tint {
            painter.rect_filled(r.shrink(3.0), 2.0, t.gamma_multiply(0.75));
        }
        // the slot being dragged shows empty while its icon is in the air
        let lifted = dragging == Some(screens::BarDrag::Slot(i));
        if !lifted {
            // an ability draws its own picture (from the pack); the rest their item
            let held = if look.empty {
                None
            } else {
                p.bar.get(i).copied().flatten()
            };
            action_icon(painter, r.shrink(5.0), p, held);
        }
        theme::action_slot(
            painter,
            r,
            key,
            look.cooldown,
            look.active,
            look.usable || look.empty,
        );
        if look.seconds >= 1.5 {
            theme::shadow_text(
                painter,
                r.center(),
                Align2::CENTER_CENTER,
                &format!("{:.0}", look.seconds.ceil()),
                theme::bold_font(17.0),
                theme::GOLD_BRIGHT,
            );
        }
        if look.fresh > 0.0 {
            // a new ability: a slow gold pulse round its slot
            let pulse = 0.55 + 0.45 * (ctx.input(|i| i.time) as f32 * 4.0).sin();
            painter.rect_stroke(
                r.expand(2.0),
                5.0,
                Stroke::new(
                    2.5_f32,
                    theme::GOLD_BRIGHT.gamma_multiply(look.fresh.min(1.0) * pulse),
                ),
                egui::StrokeKind::Outside,
            );
        }
    }
    // clicks, drags and tooltips (the cursor is free in the RPG's controls)
    let mut used = None;
    let mut dropped: Option<(screens::BarDrag, usize)> = None;
    let mut over_bar = false;
    egui::Area::new(egui::Id::new("action-bar"))
        .order(egui::Order::Middle)
        .fixed_pos(bar.min)
        .show(ctx, |ui| {
            let (_, whole) = ui.allocate_exact_size(bar.size(), egui::Sense::hover());
            over_bar = whole.contains_pointer();
            for i in 0..12 {
                let r = slot_rect(i);
                let resp = ui.interact(
                    r,
                    egui::Id::new(("action-slot", i)),
                    egui::Sense::click_and_drag(),
                );
                crate::probe::mark(format!("bar.{i}"), r);
                let slot = action_at(p, i);
                let empty = matches!(slot, ActionSlot::Empty);
                if !empty {
                    resp.dnd_set_drag_payload(screens::BarDrag::Slot(i));
                }
                if let Some(d) = resp.dnd_release_payload::<screens::BarDrag>() {
                    dropped = Some((*d, i));
                } else if resp.clicked() && !empty {
                    used = Some(i);
                }
                if resp.dnd_hover_payload::<screens::BarDrag>().is_some() {
                    ui.painter().rect_stroke(
                        r,
                        4.0,
                        Stroke::new(2.0_f32, theme::GOLD_BRIGHT),
                        egui::StrokeKind::Inside,
                    );
                }
                if dragging.is_none() {
                    match slot {
                        ActionSlot::Ability(a) => {
                            let c = &p.content;
                            resp.on_hover_ui(|ui| {
                                screens::ability_tooltip(ui, c, a, true);
                                key_line(ui, i);
                            });
                        }
                        ActionSlot::Recall => {
                            resp.on_hover_ui(|ui| {
                                plain_tooltip(ui, &t("hud.recall"), &t("hud.recall_text"));
                                key_line(ui, i);
                            });
                        }
                        ActionSlot::Eat => {
                            resp.on_hover_ui(|ui| {
                                plain_tooltip(ui, &t("hud.eat"), &t("hud.eat_text"));
                                key_line(ui, i);
                            });
                        }
                        ActionSlot::Empty => {}
                    }
                }
            }
        });
    // the icon in the air
    if let (Some(d), Some(at)) = (dragging, ctx.pointer_interact_pos()) {
        let id = match d {
            screens::BarDrag::Slot(i) => p.bar.get(i).copied().flatten(),
            screens::BarDrag::Ability(a) => Some(a),
        };
        if id.is_some() {
            let top = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("bar-drag"),
            ));
            let r = Rect::from_center_size(at, Vec2::splat(SLOT - 6.0));
            top.rect_filled(r, 4.0, Color32::from_black_alpha(150));
            action_icon(&top, r.shrink(3.0), p, id);
            top.rect_stroke(
                r,
                4.0,
                Stroke::new(1.5_f32, theme::GOLD_BRIGHT),
                egui::StrokeKind::Inside,
            );
        }
        // let go away from the bar: the slot is cleared
        if ctx.input(|i| i.pointer.any_released())
            && dropped.is_none()
            && !over_bar
            && let screens::BarDrag::Slot(i) = d
        {
            p.bar[i] = None;
            save_bar(p, actions);
        }
    }
    if let Some((d, to)) = dropped {
        if !p.ui.bar_ready {
            load_bar(p);
        }
        match d {
            screens::BarDrag::Slot(from) if from != to => p.bar.swap(from, to),
            screens::BarDrag::Ability(a) => {
                for b in p.bar.iter_mut() {
                    if *b == Some(a) {
                        *b = None;
                    }
                }
                p.bar[to] = Some(a);
                p.ui.fresh.remove(&a);
            }
            _ => {}
        }
        save_bar(p, actions);
    } else if let Some(i) = used {
        use_action(p, i);
    }
    bar
}

fn key_line(ui: &mut egui::Ui, i: usize) {
    ui.label(
        RichText::new(tf("hud.key", &[("key", BAR_KEYS[i])]))
            .font(theme::body_font(12.0))
            .color(theme::TEXT_DIM),
    );
}

fn plain_tooltip(ui: &mut egui::Ui, title: &str, text: &str) {
    ui.set_max_width(280.0);
    theme::tooltip_title(ui, title, Color32::WHITE);
    ui.label(RichText::new(text).color(theme::GOLD_BRIGHT));
}

/// My cast in progress, above the action bar.
pub fn cast_bar(painter: &egui::Painter, p: &Play, bar: Rect) {
    let Some((a, start, secs)) = p.my_cast else {
        return;
    };
    let name = if a == dyno_protocol::RECALL {
        t("hud.recall")
    } else if a == dyno_protocol::GATHER {
        t("hud.gathering")
    } else {
        p.content
            .ability(a)
            .map(|a| a.name.clone())
            .unwrap_or_default()
    };
    let f = (start.elapsed().as_secs_f32() / secs.max(0.01)).min(1.0);
    theme::cast_bar(
        painter,
        Pos2::new(bar.center().x, bar.top() - 92.0),
        280.0,
        f,
        &name,
    );
}

/// The banners and passing lines: the big title, the level gained, toasts,
/// quest steps, things received, floating XP.
pub fn moments(ctx: &egui::Context, painter: &egui::Painter, p: &mut Play, bar: Rect) {
    let screen = ctx.content_rect();
    let c = screen.center();
    count_loot(p);
    // big title: a place discovered, dusk, a new day
    if let Some((at, text)) = &p.title {
        let s = at.elapsed().as_secs_f32();
        let fade = (s / 0.6).min(1.0) * (1.0 - (s - 3.2).max(0.0) / 1.3).clamp(0.0, 1.0);
        if p.ui.level_up.is_none() {
            let at = Pos2::new(c.x, screen.top() + screen.height() * 0.24);
            theme::title_text(
                painter,
                at,
                text,
                46.0,
                theme::GOLD_BRIGHT.gamma_multiply(fade),
            );
            theme::divider(
                painter,
                at + Vec2::new(0.0, 36.0),
                260.0,
                theme::GOLD.gamma_multiply(fade),
            );
        }
    }
    // a level gained
    if p.ui
        .level_up
        .as_ref()
        .is_some_and(|l| l.at.elapsed() > Duration::from_secs(7))
    {
        p.ui.level_up = None;
    }
    if let Some(l) = &p.ui.level_up {
        let s = l.at.elapsed().as_secs_f32();
        let fade = (s / 0.35).min(1.0) * (1.0 - (s - 5.6).max(0.0) / 1.4).clamp(0.0, 1.0);
        let y = screen.top() + screen.height() * 0.22;
        // a warm flash over the whole screen, then a band of light behind the words
        let flash = (1.0 - s / 0.9).clamp(0.0, 1.0);
        if flash > 0.0 {
            painter.rect_filled(
                screen,
                0.0,
                Color32::from_rgba_unmultiplied(255, 214, 120, (70.0 * flash) as u8),
            );
        }
        let half = (screen.width() * 0.34) * (s / 0.5).min(1.0);
        // (dark, so the words read against a bright sky; gold rules above and below)
        let glow = |a: u8| Color32::from_rgba_unmultiplied(14, 9, 6, (a as f32 * fade) as u8);
        let band = Rect::from_center_size(Pos2::new(c.x, y + 22.0), Vec2::new(half * 2.0, 136.0));
        for edge in [band.top(), band.bottom()] {
            let rule = |from: f32, to: f32| {
                theme::gradient_rect_h(
                    painter,
                    Rect::from_min_max(
                        Pos2::new(from.min(to), edge - 0.75),
                        Pos2::new(from.max(to), edge + 0.75),
                    ),
                    theme::GOLD.gamma_multiply(if from < to { 0.0 } else { fade }),
                    theme::GOLD.gamma_multiply(if from < to { fade } else { 0.0 }),
                );
            };
            rule(c.x - half, c.x);
            rule(c.x + half, c.x);
        }
        let (l_half, r_half) = band.split_left_right_at_fraction(0.5);
        for part in [l_half, r_half] {
            let (top, bottom) = part.split_top_bottom_at_fraction(0.5);
            let left = part.left() < c.x - 1.0;
            for (rect, upper) in [(top, true), (bottom, false)] {
                // brightest at the middle of the band, fading to its ends and edges
                let mut mesh = egui::epaint::Mesh::default();
                let (inner, outer) = if left {
                    (rect.right(), rect.left())
                } else {
                    (rect.left(), rect.right())
                };
                let (mid, edge) = if upper {
                    (rect.bottom(), rect.top())
                } else {
                    (rect.top(), rect.bottom())
                };
                mesh.colored_vertex(Pos2::new(inner, mid), glow(190));
                mesh.colored_vertex(Pos2::new(outer, mid), glow(0));
                mesh.colored_vertex(Pos2::new(outer, edge), glow(0));
                mesh.colored_vertex(Pos2::new(inner, edge), glow(150));
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(egui::Shape::mesh(mesh));
            }
        }
        let num = l.level.to_string();
        theme::title_text(
            painter,
            Pos2::new(c.x, y),
            &tf("hud.level_up", &[("level", &num)]),
            60.0,
            theme::GOLD_BRIGHT.gamma_multiply(fade),
        );
        theme::divider(
            painter,
            Pos2::new(c.x, y + 44.0),
            half.min(300.0),
            theme::GOLD.gamma_multiply(fade),
        );
        let gain = p.max_health - l.health_before;
        let mut line = tf("hud.level_reached", &[("level", &num)]);
        if gain >= 1.0 {
            line.push_str("   ");
            line.push_str(&tf("hud.level_health", &[("n", &format!("{gain:.0}"))]));
        }
        theme::subtitle_text(
            painter,
            Pos2::new(c.x, y + 66.0),
            &line,
            18.0,
            theme::TEXT.gamma_multiply(fade),
        );
        // what was learned: icon, name, and the key it went on
        let mut row = y + 96.0;
        for (a, key) in &l.learned {
            let Some(ab) = p.content.ability(*a) else {
                continue;
            };
            let name = tf("hud.new_ability", &[("ability", &ab.name)]);
            let hint = match key {
                Some(k) => tf("hud.new_ability_key", &[("key", BAR_KEYS[*k])]),
                None => t("hud.new_ability_book"),
            };
            let name_w = painter
                .layout_no_wrap(name.clone(), theme::title_font(17.0), Color32::WHITE)
                .size()
                .x;
            let hint_w = painter
                .layout_no_wrap(hint.clone(), theme::body_font(14.0), Color32::WHITE)
                .size()
                .x;
            let w = 44.0 + 10.0 + name_w.max(hint_w);
            let left = c.x - w * 0.5;
            let plate =
                Rect::from_min_size(Pos2::new(left - 10.0, row - 4.0), Vec2::new(w + 20.0, 52.0));
            painter.rect_filled(
                plate,
                6.0,
                Color32::from_rgba_unmultiplied(16, 11, 8, (200.0 * fade) as u8),
            );
            painter.rect_stroke(
                plate,
                6.0,
                Stroke::new(1.0_f32, theme::GOLD_DARK.gamma_multiply(fade)),
                egui::StrokeKind::Inside,
            );
            let icon = Rect::from_min_size(Pos2::new(left, row), Vec2::splat(44.0));
            if fade > 0.5 {
                theme::slot_well(painter, icon);
                item_ui::ability_icon(painter, icon.shrink(5.0), &p.content, ab);
                theme::slot_rim(painter, icon, true);
            }
            theme::shadow_text(
                painter,
                Pos2::new(left + 54.0, row + 2.0),
                Align2::LEFT_TOP,
                &name,
                theme::title_font(17.0),
                theme::GOLD_BRIGHT.gamma_multiply(fade),
            );
            theme::shadow_text(
                painter,
                Pos2::new(left + 54.0, row + 25.0),
                Align2::LEFT_TOP,
                &hint,
                theme::body_font(14.0),
                theme::TEXT.gamma_multiply(fade),
            );
            row += 58.0;
        }
    }
    // toasts: a dark plate with gilded ends, under the top edge
    for (k, (at, text)) in p.toasts.iter().enumerate() {
        let fade = (1.0 - (at.elapsed().as_secs_f32() - 3.5).max(0.0) / 1.5).clamp(0.0, 1.0);
        let pos = Pos2::new(c.x, screen.top() + 84.0 + k as f32 * 38.0);
        let galley = painter.layout_no_wrap(
            text.clone(),
            theme::body_font(17.0),
            theme::GOLD_BRIGHT.gamma_multiply(fade),
        );
        let r = Rect::from_center_size(pos, galley.size() + Vec2::new(44.0, 14.0));
        painter.rect_filled(
            r,
            4.0,
            Color32::from_rgba_unmultiplied(22, 16, 12, (215.0 * fade) as u8),
        );
        painter.rect_stroke(
            r,
            4.0,
            Stroke::new(1.5_f32, theme::GOLD_DARK.gamma_multiply(fade)),
            egui::StrokeKind::Inside,
        );
        for x in [r.left(), r.right()] {
            theme::diamond(
                painter,
                Pos2::new(x, r.center().y),
                5.0,
                theme::GOLD.gamma_multiply(fade),
                Stroke::new(1.0_f32, theme::OUTLINE.gamma_multiply(fade)),
            );
        }
        painter.galley(r.center() - galley.size() * 0.5, galley, theme::GOLD_BRIGHT);
    }
    // quest steps and the like: plain bright lines, upper middle
    p.ui.notices
        .retain(|(at, _, _)| at.elapsed() < Duration::from_millis(5200));
    // (under the level banner and what it lists, while that shows)
    let top = match &p.ui.level_up {
        Some(l) => screen.top() + screen.height() * 0.22 + 128.0 + l.learned.len() as f32 * 58.0,
        None => screen.top() + screen.height() * 0.15 + p.toasts.len() as f32 * 38.0,
    };
    for (k, (at, text, colour)) in p.ui.notices.iter().enumerate() {
        let s = at.elapsed().as_secs_f32();
        let fade = (s / 0.2).min(1.0) * (1.0 - (s - 4.0).max(0.0) / 1.2).clamp(0.0, 1.0);
        // a small swell as the line arrives
        let size = 19.0 + 4.0 * (1.0 - s / 0.25).clamp(0.0, 1.0);
        theme::shadow_text(
            painter,
            Pos2::new(c.x, top + k as f32 * 26.0),
            Align2::CENTER_CENTER,
            text,
            theme::bold_font(size),
            colour.gamma_multiply(fade),
        );
    }
    // things received: icon and name, lower right, newest at the bottom
    p.ui.loots
        .retain(|(at, _, _)| at.elapsed() < Duration::from_millis(5600));
    let n = p.ui.loots.len();
    for (k, (at, item, count)) in p.ui.loots.iter().enumerate() {
        let s = at.elapsed().as_secs_f32();
        let fade = (s / 0.25).min(1.0) * (1.0 - (s - 4.4).max(0.0) / 1.2).clamp(0.0, 1.0);
        let it = p.content.item(*item);
        let name = if *count > 1 {
            format!("{} \u{d7}{}", it.name, count)
        } else {
            it.name.to_string()
        };
        let col = theme::quality_color(it.quality);
        let galley = painter.layout_no_wrap(name, theme::bold_font(15.0), col.gamma_multiply(fade));
        let w = galley.size().x + 34.0 + 22.0;
        let slide = (1.0 - s / 0.25).clamp(0.0, 1.0) * 30.0;
        let r = Rect::from_min_size(
            Pos2::new(
                screen.right() - 16.0 - w + slide,
                bar.top() - 70.0 - (n - 1 - k) as f32 * 38.0,
            ),
            Vec2::new(w, 32.0),
        );
        painter.rect_filled(
            r,
            4.0,
            Color32::from_rgba_unmultiplied(16, 11, 8, (205.0 * fade) as u8),
        );
        painter.rect_stroke(
            r,
            4.0,
            Stroke::new(
                1.0_f32,
                if it.quality >= 2 {
                    col.gamma_multiply(0.8 * fade)
                } else {
                    theme::GOLD_DARK.gamma_multiply(fade)
                },
            ),
            egui::StrokeKind::Inside,
        );
        if fade > 0.4 {
            item_ui::item_icon(
                painter,
                Rect::from_min_size(r.min + Vec2::new(4.0, 3.0), Vec2::splat(26.0)),
                &p.content,
                *item,
            );
        }
        painter.galley(
            Pos2::new(r.left() + 36.0, r.center().y - galley.size().y * 0.5),
            galley,
            col,
        );
    }
    // experience and coins, rising beside you
    p.floaters
        .retain(|(t, _, _)| t.elapsed() < Duration::from_millis(2200));
    for (k, (at, text, colour)) in p.floaters.iter().enumerate() {
        let a = at.elapsed().as_secs_f32();
        let pos = Pos2::new(c.x + 70.0, c.y - 20.0 - a * 34.0 - k as f32 * 22.0);
        let fade = (1.0 - (a - 1.2).max(0.0) / 1.0).clamp(0.0, 1.0);
        theme::shadow_text(
            painter,
            pos,
            Align2::LEFT_CENTER,
            text,
            theme::bold_font(18.0),
            colour.gamma_multiply(fade),
        );
    }
}

/// Floating combat text, rising from the creature (or from you). A
/// critical hit (its text ends with `!`) arrives large and settles.
pub fn combat_text(painter: &egui::Painter, p: &mut Play, screen: Rect, vp: glam::Mat4) {
    p.combat_text
        .retain(|(t, _, _, _)| t.elapsed() < Duration::from_millis(1500));
    for (k, (t, ent, text, col)) in p.combat_text.iter().enumerate() {
        let a = t.elapsed().as_secs_f32();
        let world = match ent {
            Some(id) => p.entities.get(id).map(|e| {
                e.pos() + Vec3::Y * (p.tops.borrow().get(id).copied().unwrap_or(1.6) + 0.5)
            }),
            None => Some(p.player.pos + Vec3::Y * 1.9),
        };
        let Some(wp) = world else { continue };
        let clip = vp * (wp + Vec3::Y * a * 0.9).extend(1.0);
        if clip.w <= 0.1 {
            continue;
        }
        let ndc = clip.truncate() / clip.w;
        // each number drifts to its own side, so a flurry stays readable
        let side = [(-1.0_f32), 1.0, 0.0][k % 3] * (14.0 + a * 16.0);
        let at = Pos2::new(
            screen.left() + (ndc.x * 0.5 + 0.5) * screen.width() + side,
            screen.top() + (0.5 - ndc.y * 0.5) * screen.height(),
        );
        let fade = (1.0 - (a - 0.9).max(0.0) / 0.6).clamp(0.0, 1.0);
        let crit = text.ends_with('!');
        let size = if crit {
            // lands big, settles
            30.0 + 16.0 * (1.0 - a / 0.22).clamp(0.0, 1.0)
        } else {
            21.0
        };
        theme::shadow_text(
            painter,
            at,
            Align2::CENTER_CENTER,
            text,
            theme::bold_font(size),
            col.gamma_multiply(fade),
        );
    }
}

/// The air left while under water: a thin meter over the right end of the
/// bar, shown only while it is not full. (Its own function, to be themed.)
pub fn air_meter(painter: &egui::Painter, p: &Play, bar: Rect) {
    if p.creative() || p.air >= 10.0 {
        return;
    }
    let k = (p.air / 10.0).clamp(0.0, 1.0);
    let at = Pos2::new(bar.right() - 122.0, bar.top() - 30.0);
    painter.rect_filled(
        Rect::from_min_size(at, Vec2::new(110.0, 5.0)).expand(1.0),
        2.0,
        Color32::from_black_alpha(150),
    );
    painter.rect_filled(
        Rect::from_min_size(at, Vec2::new(110.0 * k, 5.0)),
        2.0,
        Color32::from_rgb(110, 170, 230),
    );
}

/// The death screen: what happened, what it costs, and the way back.
pub fn death(ctx: &egui::Context, painter: &egui::Painter, p: &Play, actions: &mut Vec<Action>) {
    let Some(msg) = &p.dead else { return };
    let screen = ctx.content_rect();
    painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(40, 6, 8, 150));
    let resp = egui::Window::new("dead")
        .id(egui::Id::new("dead"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -40.0))
        .frame(theme::window_frame())
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("death.title"))
                        .font(theme::title_font(30.0))
                        .color(Color32::from_rgb(236, 112, 96)),
                );
                theme::ui_divider(ui, theme::GOLD_DARK);
                ui.add_space(4.0);
                ui.label(
                    RichText::new(msg)
                        .font(theme::body_font(16.0))
                        .color(theme::TEXT),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t("death.hint"))
                        .font(theme::body_font(14.0))
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(12.0);
                let b = egui::Button::new(
                    RichText::new(t("death.release"))
                        .font(theme::title_font(15.0))
                        .color(theme::INK),
                )
                .fill(theme::GOLD)
                .stroke(Stroke::new(1.0_f32, theme::GOLD_BRIGHT))
                .min_size(Vec2::new(220.0, 36.0));
                if ui.add(b).clicked() {
                    actions.push(Action::Respawn);
                }
                ui.add_space(4.0);
            });
        });
    if let Some(r) = resp {
        theme::window_trim(&ctx.layer_painter(r.response.layer_id), r.response.rect);
    }
}

fn menu_button(ui: &mut egui::Ui, text: &str, gold: bool) -> bool {
    let (ink, fill, edge) = if gold {
        (theme::INK, theme::GOLD, theme::GOLD_BRIGHT)
    } else {
        (theme::GOLD, theme::WOOD_DARK, theme::GOLD_DARK)
    };
    ui.add_sized(
        [ui.available_width(), 36.0],
        egui::Button::new(RichText::new(text).font(theme::title_font(15.0)).color(ink))
            .fill(fill)
            .stroke(Stroke::new(1.0_f32, edge)),
    )
    .clicked()
}

/// The game menu (Esc): back to the game, options, controls, leave.
pub fn menu(ctx: &egui::Context, p: &mut Play, host: &mut Option<Host>, actions: &mut Vec<Action>) {
    let screen = ctx.content_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("menu-dim"),
    ))
    .rect_filled(screen, 0.0, Color32::from_black_alpha(110));
    let page = p.ui.menu_page;
    let (title, width) = match page {
        MenuPage::Main => (t("menu.title"), 300.0),
        MenuPage::Options => (t("menu.options"), 520.0),
        MenuPage::Controls => (t("menu.controls"), 520.0),
    };
    let mut open = true;
    let mut go = None;
    theme::window(
        ctx,
        match page {
            MenuPage::Main => "game-menu",
            MenuPage::Options => "game-menu-options",
            MenuPage::Controls => "game-menu-controls",
        },
        &title,
        "",
        width,
        (Align2::CENTER_CENTER, Vec2::ZERO),
        &mut open,
        |ui| match page {
            MenuPage::Main => {
                ui.spacing_mut().item_spacing.y = 8.0;
                if menu_button(ui, &t("menu.resume"), true) {
                    actions.push(Action::Resume);
                }
                if menu_button(ui, &t("menu.options"), false) {
                    go = Some(MenuPage::Options);
                }
                if menu_button(ui, &t("menu.controls"), false) {
                    go = Some(MenuPage::Controls);
                }
                host_panel(ui, host, actions);
                ui.add_space(6.0);
                if menu_button(ui, &t("menu.leave"), false) {
                    actions.push(Action::Disconnect);
                }
                if menu_button(ui, &t("menu.quit"), false) {
                    actions.push(Action::Quit);
                }
            }
            MenuPage::Options => {
                if crate::options::panel(ui) {
                    actions.push(Action::Graphics);
                }
                ui.add_space(10.0);
                if menu_button(ui, &t("menu.back"), false) {
                    go = Some(MenuPage::Main);
                }
            }
            MenuPage::Controls => {
                // the keys, from the pack: [keys, what they do] pairs under headings
                let key = if p.wow() { "keys.rpg" } else { "keys.builder" };
                let groups = crate::uitext::value(key)
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                egui::ScrollArea::vertical()
                    .max_height((screen.height() - 260.0).max(200.0))
                    .min_scrolled_height((screen.height() - 260.0).max(200.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for g in &groups {
                            let name = g.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            theme::heading(ui, name, theme::GOLD);
                            let rows = g
                                .get("rows")
                                .and_then(|r| r.as_array().cloned())
                                .unwrap_or_default();
                            egui::Grid::new(("keys", name))
                                .num_columns(2)
                                .spacing([18.0, 5.0])
                                .min_col_width(150.0)
                                .show(ui, |ui| {
                                    for r in &rows {
                                        let cell = |i: usize| {
                                            r.get(i).and_then(|v| v.as_str()).unwrap_or("")
                                        };
                                        key_caps(ui, cell(0));
                                        ui.label(
                                            RichText::new(cell(1))
                                                .font(theme::body_font(15.0))
                                                .color(theme::TEXT),
                                        );
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(6.0);
                        }
                    });
                ui.add_space(6.0);
                if menu_button(ui, &t("menu.back"), false) {
                    go = Some(MenuPage::Main);
                }
            }
        },
    );
    if let Some(g) = go {
        p.ui.menu_page = g;
    }
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if page == MenuPage::Main {
            actions.push(Action::Resume);
        } else {
            p.ui.menu_page = MenuPage::Main;
        }
    }
}

/// Keys as small caps: "W  S" draws two key tops.
fn key_caps(ui: &mut egui::Ui, keys: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for k in keys.split("  ") {
            let galley = ui.painter().layout_no_wrap(
                k.to_string(),
                theme::bold_font(13.0),
                theme::GOLD_BRIGHT,
            );
            let (r, _) = ui.allocate_exact_size(
                Vec2::new(galley.size().x + 14.0, 24.0),
                egui::Sense::hover(),
            );
            let p = ui.painter();
            p.rect_filled(r, 4.0, theme::WOOD_DARK);
            p.rect_stroke(
                r,
                4.0,
                Stroke::new(1.0_f32, theme::GOLD_DARK),
                egui::StrokeKind::Inside,
            );
            p.galley(r.center() - galley.size() * 0.5, galley, theme::GOLD_BRIGHT);
        }
    });
}

/// The world map (M).
pub fn map(ctx: &egui::Context, p: &mut Play) {
    if !p.ui.map_open {
        return;
    }
    p.ui.once("map");
    let mut map = std::mem::take(&mut p.world_map);
    let me = rpg_ui::Me {
        content: &p.content,
        money: p.money,
        level: p.level,
        log: &p.quest_log,
        done: &p.quests_done,
        pos: p.player.pos.into(),
        hearth: p.hearth,
        equip: &p.equip,
        class: p.class.and_then(|c| p.content.class(c)),
    };
    let party = p
        .remotes
        .values()
        .filter(|r| p.party.iter().any(|m| m.name == r.name))
        .map(|r| (r.name.clone(), Vec3::from(r.current().pos)))
        .collect();
    let open = map.window(
        ctx,
        &crate::worldmap::View {
            far: &p.far,
            me: &me,
            pos: p.player.pos,
            yaw: p.player.yaw + p.cam_orbit,
            discovered: &p.discovered,
            zone: &p.zone,
            party,
        },
    );
    p.world_map = map;
    if !open {
        p.ui.map_open = false;
    }
}

/// The tutorial tips: fire what the frame's state shows, then draw.
pub fn tips(
    ctx: &egui::Context,
    p: &mut Play,
    map: Rect,
    tracker: Rect,
    actions: &mut Vec<Action>,
) {
    let o = crate::options::interface();
    if p.ui.tips.enabled != o.tips {
        p.ui.tips.enabled = o.tips;
    }
    // "show the tips again" in the options empties the saved list
    if o.tips && o.tips_seen.is_empty() && p.ui.tips.seen_count() > 0 {
        p.ui.tips.reset();
        p.ui.fired.clear();
    }
    p.ui.anchors.insert("minimap", map);
    p.ui.anchors.insert("tracker", tracker);
    let rpg = p.content.world.protected && !p.creative();
    if rpg && p.class.is_some() && p.intro.is_none() && !p.land_wait {
        p.ui.once("spawn");
        let start = *p.ui.start.get_or_insert(p.player.pos);
        if start.distance(p.player.pos) > 4.0 {
            p.ui.once("moved");
        }
        if matches!(p.panel, Some(rpg_ui::Panel::Dialog { .. })) {
            p.ui.once("dialog");
        }
        if matches!(p.panel, Some(rpg_ui::Panel::Vendor { .. })) {
            p.ui.once("vendor");
        }
        match p.screen_tab {
            Some(screens::Tab::Bags) => p.ui.once("bags"),
            Some(screens::Tab::Spellbook) => p.ui.once("spellbook"),
            Some(screens::Tab::Character) => p.ui.once("character"),
            Some(screens::Tab::Skills) => p.ui.once("skills"),
            None => {}
        }
        let target = p
            .focus
            .or(p.target_entity)
            .and_then(|id| p.entities.get(&id));
        if target.is_some_and(|e| matches!(e.kind, EntityKind::Creature(_))) {
            p.ui.once("target");
        }
        if p.attacking.is_some() {
            p.ui.once("combat");
            match p.class.and_then(|c| p.content.class(c)).map(|c| c.resource) {
                Some(Resource::Rage) => p.ui.once("combat_rage"),
                Some(Resource::Energy) => p.ui.once("combat_energy"),
                Some(Resource::Mana) => p.ui.once("combat_mana"),
                None => {}
            }
        }
        if p.health > 0.0 && p.health < p.max_health * 0.35 && p.dead.is_none() {
            p.ui.once("low_health");
        }
        if p.loot.is_some() {
            p.ui.once("loot");
        }
        if p.dead.is_some() {
            p.ui.once("death");
        }
        if p.rested > 0 {
            p.ui.once("rested");
        }
        if !p.party.is_empty() {
            p.ui.once("party");
        }
        if p.combo.0 > 0 {
            p.ui.once("combo");
        }
    }
    pinned(ctx, p);
    let quiet = p.menu_open
        || p.intro.is_some()
        || p.land_wait
        || p.class_pick
        || p.ui.level_up.is_some()
        || matches!(p.panel, Some(rpg_ui::Panel::Dialog { quest: Some(_), .. }));
    let anchors = &p.ui.anchors;
    let hint = if p.captured {
        String::new()
    } else {
        crate::uitext::opt("tips.dismiss")
    };
    // a tip does not lie over an open window: where its place beside a HUD
    // part is taken, it goes to the middle (above the action bar) instead
    let windows: Vec<Rect> = [
        "screens",
        "npc-dialog",
        "npc-vendor",
        "quest-log",
        "trade",
        "world-map",
    ]
    .iter()
    .filter_map(|id| ctx.memory(|m| m.area_rect(egui::Id::new(*id))))
    .collect();
    let open = p.screen_tab.is_some() || p.panel.is_some() || p.trade.is_some() || p.ui.map_open;
    let windows = if open { windows } else { Vec::new() };
    p.ui.tips.show(
        ctx,
        |name| {
            let r = anchors.get(name).copied()?;
            let zone = r.expand2(Vec2::new(380.0, 60.0));
            (!windows.iter().any(|w| w.intersects(zone))).then_some(r)
        },
        &windows,
        quiet,
        &hint,
    );
    if let Some(seen) = p.ui.tips.take_seen()
        && crate::options::change_interface(|o| o.tips_seen = seen)
    {
        actions.push(Action::Interface);
    }
}

/// Tooltips held open for a screenshot.
fn pinned(ctx: &egui::Context, p: &Play) {
    let screen = ctx.content_rect();
    let class = p.class.and_then(|c| p.content.class(c));
    let show = |id: &str, at: Pos2, add: &dyn Fn(&mut egui::Ui)| {
        egui::Area::new(egui::Id::new(id))
            .order(egui::Order::Tooltip)
            .fixed_pos(at)
            .show(ctx, |ui| theme::tooltip(ui, add));
    };
    if let Some(st) = p.ui.pinned.0 {
        show(
            "pinned-item",
            screen.center() + Vec2::new(-560.0, -220.0),
            &|ui| screens::item_tooltip_vs(ui, &p.content, &st, class, p.level, Some(&p.equip)),
        );
    }
    if let Some(a) = p.ui.pinned.1.and_then(|a| p.content.ability(a)) {
        show(
            "pinned-ability",
            screen.center() + Vec2::new(-560.0, 60.0),
            &|ui| screens::ability_tooltip(ui, &p.content, a, a.level <= p.level),
        );
    }
}

/// Put the HUD in a named state for a screenshot (`dyno shot --ui NAME`):
/// the moments that pass too quickly to catch, made from what the
/// character really has. `tip:EVENT` shows the tip for an event.
pub fn shot_state(p: &mut Play, ui: &str) {
    let past = |ms: u64| Instant::now() - Duration::from_millis(ms);
    // (called again before the frame that is kept: start clean)
    p.ui.notices.clear();
    p.ui.loots.clear();
    p.ui.level_up = None;
    p.combat_text.clear();
    p.floaters.clear();
    p.auras.clear();
    // tips only when asked for: a shot shows one thing
    let tip = ui.strip_prefix("tip:");
    crate::options::change_interface(|o| {
        o.tips = tip.is_some();
        o.tips_seen.clear();
    });
    p.ui.tips = Tips::new(&[], tip.is_some());
    if p.class.is_some() {
        load_bar(p);
    }
    // the nearest creature within a fight's reach; where there is none (a
    // town), a stand-in a few steps ahead, so the target frame has a subject
    let nearest = |p: &mut Play| {
        let me = p.player.pos;
        let near = p
            .entities
            .iter()
            .filter(|(_, e)| matches!(e.kind, EntityKind::Creature(_)))
            .map(|(id, e)| (*id, e.pos().distance(me)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|(_, d)| *d < 20.0)
            .map(|(id, _)| id);
        near.or_else(|| {
            let kind = p
                .content
                .creatures
                .iter()
                .find(|c| c.hostile)
                .or(p.content.creatures.first())?
                .id;
            let d = p.player.look_dir();
            let d = Vec3::new(d.x, 0.0, d.z).normalize_or_zero();
            let at = me + d * 6.0 + Vec3::new(-d.z, 0.0, d.x) * 1.5;
            let at = [
                at.x,
                p.world.surface(at.x as i32, at.z as i32) as f32 + 1.0,
                at.z,
            ];
            let id = 9_000_001;
            p.entities.insert(
                id,
                ClientEntity {
                    kind: EntityKind::Creature(kind),
                    from: at,
                    to: at,
                    at: Instant::now(),
                    yaw: p.player.yaw + std::f32::consts::PI,
                    born: Instant::now() - Duration::from_secs(5),
                    hurt_at: None,
                    info: Some((p.level, 40.0, 40.0)),
                    name: None,
                },
            );
            Some(id)
        })
    };
    let known: Vec<u16> = p
        .class
        .map(|c| {
            p.content
                .known_abilities(c, p.level)
                .iter()
                .map(|a| a.id)
                .collect()
        })
        .unwrap_or_default();
    match ui {
        "combat" => {
            let target = nearest(p);
            p.focus = target;
            p.attacking = target;
            if let Some(e) = target.and_then(|t| p.entities.get_mut(&t)) {
                let (l, _, m) = e.info.unwrap_or((1, 30.0, 30.0));
                e.info = Some((l, m * 0.55, m));
            }
            p.health = p.max_health * 0.7;
            p.power = p.power_max * 0.45;
            // a shield on me; what I have put on it
            p.shield = (p.max_health * 0.3, p.max_health * 0.4);
            if let Some(t) = target {
                let now = Instant::now();
                p.entity_auras.insert(
                    t,
                    known
                        .iter()
                        .take(2)
                        .enumerate()
                        .map(|(k, a)| (*a, now + Duration::from_secs(5 + 7 * k as u64)))
                        .collect(),
                );
            }
            p.combo = (3, target);
            if let Some(a) = known.first() {
                p.my_cast = Some((*a, past(700), 1.5));
            }
            if let Some(a) = known.get(1) {
                p.cooldowns
                    .insert(*a, (Instant::now() + Duration::from_secs(6), 10.0));
                p.auras.push((*a, Instant::now() + Duration::from_secs(42)));
            }
            let white = Color32::WHITE;
            p.combat_text.push((past(350), target, "14".into(), white));
            p.combat_text.push((
                past(120),
                target,
                "31!".into(),
                Color32::from_rgb(255, 230, 80),
            ));
            p.combat_text
                .push((past(500), None, "-6".into(), Color32::from_rgb(255, 70, 60)));
            // (a heal as the server reports one: a single number)
            healed(p, 12.0);
            p.floaters
                .push((past(300), "+55 XP".into(), Color32::from_rgb(190, 140, 255)));
            let line = p.quest_log.first().and_then(|q| {
                let quest = p.content.quest(q.quest)?;
                let o = quest.objectives.first()?;
                let me = rpg_ui::Me {
                    content: &p.content,
                    money: p.money,
                    level: p.level,
                    log: &p.quest_log,
                    done: &p.quests_done,
                    pos: p.player.pos.into(),
                    hearth: p.hearth,
                    equip: &p.equip,
                    class: p.class.and_then(|c| p.content.class(c)),
                };
                Some(me.objective_text(o, q.counts.first().copied().unwrap_or(0)))
            });
            if let Some(line) = line {
                p.ui.notices.push((past(600), line, NOTICE_YELLOW));
            }
            let got: Vec<ItemId> = p.pack.iter().flatten().map(|s| s.item).take(2).collect();
            for (k, item) in got.into_iter().enumerate() {
                p.ui.loots.push((past(900), item, 1 + k as u32));
            }
        }
        "levelup" => {
            // the first level that teaches this class something
            let level = p
                .class
                .and_then(|c| {
                    p.content
                        .abilities
                        .iter()
                        .filter(|a| a.class == c && a.level > 1)
                        .map(|a| a.level)
                        .min()
                })
                .unwrap_or(2);
            p.level = level;
            let content = &p.content;
            p.bar.iter_mut().for_each(|s| {
                if s.is_some_and(|a| content.ability(a).is_some_and(|a| a.level == level)) {
                    *s = None;
                }
            });
            level_up(p, level);
            if let Some(l) = &mut p.ui.level_up {
                l.at = past(600);
                l.health_before = p.max_health - 5.0;
            }
            // most levels come with a quest handed in: its line shows too
            if let Some(q) = p.content.quests.first() {
                let line = tf("hud.quest_done", &[("quest", &q.title)]);
                p.ui.notices.push((past(700), line, theme::GOLD_BRIGHT));
            }
        }
        "map" => {
            p.ui.map_open = true;
            p.captured = false;
        }
        "nopower" => {
            // nothing to spend: every ability that costs is tinted
            p.power = 0.0;
        }
        "classpick" => {
            p.class_pick = true;
            p.captured = false;
        }
        "social" => {
            // an invitation, a request, an open trade and the chat box
            p.invite = Some("Maelis".into());
            p.trade_ask = Some("Torvald".into());
            let stacks: Vec<Stack> = p.pack.iter().flatten().copied().collect();
            p.trade = Some(dyno_protocol::TradeView {
                with: "Torvald".into(),
                mine: stacks.iter().take(2).copied().collect(),
                theirs: stacks.iter().skip(2).take(1).copied().collect(),
                my_money: 35,
                their_money: 120,
                i_accept: false,
                they_accept: true,
            });
            p.chat_open = true;
            p.captured = false;
            if p.chat.len() < 3 {
                for (from, text) in [
                    ("[General] Maelis", "Anyone for the boar hunt?"),
                    ("[Party] Torvald", "On my way."),
                    ("Torvald", "Well met."),
                ] {
                    p.chat.push(Chat {
                        at: Instant::now(),
                        from: Some(from.into()),
                        text: text.into(),
                    });
                }
            }
        }
        "menu" | "menu-options" | "menu-controls" => {
            p.menu_open = true;
            p.captured = false;
            p.ui.menu_page = match ui {
                "menu-options" => MenuPage::Options,
                "menu-controls" => MenuPage::Controls,
                _ => MenuPage::Main,
            };
        }
        "tooltip" => {
            // gear this class can wear in a place where it wears something
            // else, and its newest ability
            let class = p.class.and_then(|c| p.content.class(c));
            let item = p
                .content
                .items
                .iter()
                .map(|i| p.content.item(i.id))
                .find(|it| {
                    let Some(g) = it.gear else { return false };
                    let worn = dyno_sim::character::places_for(g.slot)
                        .first()
                        .and_then(|i| p.equip.get(*i).copied().flatten());
                    worn.is_some_and(|w| w.item != it.id)
                        && it.quality >= 2
                        && class
                            .is_some_and(|c| dyno_sim::character::cannot_equip(c, 60, it).is_none())
                });
            p.ui.pinned = (item.map(|it| Stack::new(it.id, 1)), known.last().copied());
            p.screen_tab = Some(screens::Tab::Bags);
            p.captured = false;
        }
        "party" => {
            let class = p.class;
            p.party = [("Maelis", 3u8, 0.8f32, true), ("Torvald", 2, 0.35, false)]
                .iter()
                .map(|(n, l, h, leader)| dyno_protocol::PartyMember {
                    name: n.to_string(),
                    level: *l,
                    class,
                    health: 60.0 * h,
                    max: 60.0,
                    online: true,
                    leader: *leader,
                })
                .collect();
            p.focus = nearest(p);
        }
        _ => {}
    }
    if let Some(event) = tip {
        p.intro = None;
        if matches!(event, "target" | "combo") {
            p.focus = nearest(p);
            p.combo = (2, p.focus);
        }
        p.ui.tips.fire(event);
        p.ui.tips.backdate(Duration::from_millis(900));
    }
}

// ---- the smaller windows: class picker, invitations, trade, chat ----

/// A button in the theme (`theme::button`).
pub fn button(ui: &mut egui::Ui, text: &str, gold: bool) -> egui::Response {
    theme::button(ui, text, gold)
}

/// A one-line text field in the theme (`theme::field`).
pub fn field(ui: &mut egui::Ui, text: &mut String, width: f32, hint: &str) -> egui::Response {
    theme::field(ui, text, width, hint)
}

/// A character without a class chooses one (the creation screen normally
/// has: this is for characters made before it, or by a command).
pub fn class_picker(ctx: &egui::Context, p: &Play, actions: &mut Vec<Action>) {
    if !p.class_pick {
        return;
    }
    let n = p.content.classes.len().max(1);
    let mut open = true;
    let mut chosen = None;
    theme::window(
        ctx,
        "class-pick",
        &t("classes.title"),
        "",
        (n as f32 * 190.0).clamp(380.0, 1140.0),
        (Align2::CENTER_CENTER, Vec2::ZERO),
        &mut open,
        |ui| {
            ui.label(
                RichText::new(t("classes.prompt"))
                    .font(theme::body_font(16.0))
                    .color(theme::TEXT),
            );
            ui.add_space(8.0);
            ui.columns(n, |cols| {
                for (k, c) in p.content.classes.iter().enumerate() {
                    let ui = &mut cols[k];
                    egui::Frame::new()
                        .fill(theme::WOOD_DARK)
                        .stroke(Stroke::new(1.0_f32, theme::BRONZE))
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_min_height(250.0);
                            ui.label(
                                RichText::new(&c.name)
                                    .font(theme::title_font(18.0))
                                    .color(rgb(c.color)),
                            );
                            ui.label(
                                RichText::new(&c.blurb)
                                    .font(theme::body_font(13.5))
                                    .color(theme::TEXT),
                            );
                            ui.add_space(6.0);
                            for a in p.content.known_abilities(c.id, 6) {
                                ui.label(
                                    RichText::new(tf(
                                        "classes.ability",
                                        &[("ability", &a.name), ("level", &a.level.to_string())],
                                    ))
                                    .font(theme::body_font(12.5))
                                    .color(theme::TEXT_DIM),
                                )
                                .on_hover_ui(|ui| {
                                    screens::ability_tooltip(ui, &p.content, a, true)
                                });
                            }
                            ui.add_space(8.0);
                            let label = tf("classes.become", &[("class", &c.name)]);
                            let b = button(ui, &label, true);
                            crate::probe::mark(format!("class.{}", c.key), b.rect);
                            if b.clicked() {
                                chosen = Some(c.id);
                            }
                        });
                }
            });
        },
    );
    if let Some(c) = chosen {
        actions.push(Action::ChooseClass(c));
    }
}

/// A question from another player, with its two answers. The answer
/// clicked: true for the first.
fn prompt(ctx: &egui::Context, id: &str, y: f32, text: &str, yes: &str, no: &str) -> Option<bool> {
    let mut answer = None;
    let resp = egui::Window::new(id)
        .id(egui::Id::new(id))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, y))
        .frame(theme::window_frame())
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(text)
                        .font(theme::body_font(16.0))
                        .color(theme::TEXT),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 200.0).max(0.0) * 0.5);
                    if button(ui, yes, true).clicked() {
                        answer = Some(true);
                    }
                    if button(ui, no, false).clicked() {
                        answer = Some(false);
                    }
                });
            });
        });
    if let Some(r) = resp {
        theme::window_trim(&ctx.layer_painter(r.response.layer_id), r.response.rect);
    }
    answer
}

/// Invitations: to a party, to a trade.
pub fn prompts(ctx: &egui::Context, p: &mut Play) {
    if let Some(from) = p.invite.clone() {
        let text = tf("social.invite", &[("name", &from)]);
        if let Some(yes) = prompt(
            ctx,
            "party-invite",
            130.0,
            &text,
            &t("social.accept"),
            &t("social.decline"),
        ) {
            let reply = if yes { "/accept" } else { "/decline" };
            let _ = p.conn.tx.send(ClientMsg::Chat { text: reply.into() });
            p.invite = None;
        }
    }
    if let Some(from) = p.trade_ask.clone() {
        let text = tf("social.trade_ask", &[("name", &from)]);
        if let Some(yes) = prompt(
            ctx,
            "trade-ask",
            220.0,
            &text,
            &t("social.trade"),
            &t("social.decline"),
        ) {
            let op = if yes {
                dyno_protocol::TradeOp::Agree
            } else {
                dyno_protocol::TradeOp::Cancel
            };
            let _ = p.conn.tx.send(ClientMsg::Trade(op));
            p.trade_ask = None;
        }
    }
}

/// The trade window: what each side offers, coins, and both acceptances.
pub fn trade(ctx: &egui::Context, p: &mut Play) {
    let Some(view) = p.trade.clone() else { return };
    let mut ops = Vec::new();
    let mut open = true;
    let mut coins_typed = std::mem::take(&mut p.trade_money);
    theme::window(
        ctx,
        "trade",
        &t("social.trade_title"),
        &tf("social.trade_with", &[("name", &view.with)]),
        440.0,
        (Align2::LEFT_CENTER, Vec2::new(40.0, 0.0)),
        &mut open,
        |ui| {
            ui.label(
                RichText::new(t("social.trade_hint"))
                    .font(theme::body_font(13.0))
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.columns(2, |cols| {
                let you = t("social.you_offer");
                for (k, (title, items, money, accept)) in [
                    (you.as_str(), &view.mine, view.my_money, view.i_accept),
                    (
                        view.with.as_str(),
                        &view.theirs,
                        view.their_money,
                        view.they_accept,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let ui = &mut cols[k];
                    egui::Frame::new()
                        .fill(theme::WOOD_DARK)
                        .stroke(Stroke::new(
                            1.5_f32,
                            if accept { theme::GOOD } else { theme::BRONZE },
                        ))
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_min_height(210.0);
                            ui.set_width(ui.available_width());
                            theme::heading(ui, title, theme::GOLD);
                            for (i, st) in items.iter().enumerate() {
                                let it = p.content.item(st.item);
                                let (row, resp) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 36.0),
                                    if k == 0 {
                                        egui::Sense::click()
                                    } else {
                                        egui::Sense::hover()
                                    },
                                );
                                let pt = ui.painter();
                                if resp.hovered() && k == 0 {
                                    pt.rect_filled(
                                        row,
                                        4.0,
                                        Color32::from_rgba_unmultiplied(150, 104, 30, 70),
                                    );
                                }
                                let slot = Rect::from_min_size(
                                    row.min + Vec2::splat(2.0),
                                    Vec2::splat(32.0),
                                );
                                theme::item_slot(pt, slot, it.quality, None);
                                item_ui::stack_icon(pt, slot, &p.content, st);
                                pt.text(
                                    Pos2::new(slot.right() + 8.0, row.center().y),
                                    Align2::LEFT_CENTER,
                                    it.name,
                                    theme::body_font(14.5),
                                    theme::quality_color(it.quality),
                                );
                                let st = *st;
                                let resp = resp.on_hover_ui(|ui| {
                                    screens::item_tooltip(ui, &p.content, &st, None, p.level)
                                });
                                if k == 0 && resp.clicked() {
                                    ops.push(dyno_protocol::TradeOp::Withdraw { line: i as u8 });
                                }
                            }
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                                ui.label(
                                    RichText::new(t(if accept {
                                        "social.accepted"
                                    } else {
                                        "social.not_accepted"
                                    }))
                                    .font(theme::body_font(13.0))
                                    .color(if accept {
                                        theme::GOOD
                                    } else {
                                        theme::TEXT_DIM
                                    }),
                                );
                                screens::coins(ui, money);
                            });
                        });
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t("social.coins"))
                        .font(theme::body_font(14.0))
                        .color(theme::TEXT),
                );
                let r = field(ui, &mut coins_typed, 90.0, "0");
                if r.lost_focus()
                    && let Ok(n) = coins_typed.trim().parse::<u32>()
                {
                    ops.push(dyno_protocol::TradeOp::Money(n));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, &t("social.cancel"), false).clicked() {
                        ops.push(dyno_protocol::TradeOp::Cancel);
                    }
                    if button(ui, &t("social.accept_trade"), true).clicked() {
                        ops.push(dyno_protocol::TradeOp::Accept);
                    }
                });
            });
        },
    );
    p.trade_money = coins_typed;
    if !open {
        ops.push(dyno_protocol::TradeOp::Cancel);
    }
    for op in ops {
        let _ = p.conn.tx.send(ClientMsg::Trade(op));
    }
}

/// The two tabs over the chat lines: Chat and Combat.
pub fn chat_tabs(ctx: &egui::Context, p: &mut Play, at: Pos2) {
    egui::Area::new(egui::Id::new("chat_tabs"))
        .fixed_pos(at)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for (key, combat) in [("chat.tab_chat", false), ("chat.tab_combat", true)] {
                    let on = p.chat_tab_combat == combat;
                    let b = egui::Button::new(
                        RichText::new(t(key))
                            .font(theme::title_font(11.5))
                            .color(if on {
                                theme::GOLD_BRIGHT
                            } else {
                                theme::TEXT_DIM
                            }),
                    )
                    .fill(if on {
                        Color32::from_rgba_unmultiplied(40, 29, 20, 235)
                    } else {
                        Color32::from_rgba_unmultiplied(22, 16, 12, 150)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if on { theme::GOLD_DARK } else { theme::BRONZE },
                    ))
                    .min_size(Vec2::new(58.0, 22.0));
                    let r = ui.add(b);
                    crate::probe::mark(key, r.rect);
                    if r.clicked() {
                        p.chat_tab_combat = combat;
                    }
                }
            });
        });
}
