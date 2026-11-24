//! Classic MMO screens: Character (C), Bags (B), Spellbook (P) and Skills (K),
//! with item and ability tooltips. One carved window with tabs; the words
//! come from the pack (`uitext`). See docs/design/ui.md.

use crate::survival;
use crate::theme;
use crate::uitext::{f as tf, t};
use dyno_protocol::Stack;
use dyno_sim::character::{self, Sheet};
use dyno_sim::progression::money_text;
#[cfg(test)]
use dyno_world::content::Slot;
use dyno_world::content::{Ability, Class, Content, EQUIP_PLACES, Gear, Resource};
use egui::{Align2, Color32, CursorIcon, Pos2, Rect, RichText, Sense, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Character,
    Bags,
    Spellbook,
    Skills,
}

/// What is being dragged to the action bar: one of its own slots, or an
/// ability from the spellbook (egui's drag-and-drop payload).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarDrag {
    Slot(usize),
    Ability(u16),
}

/// What a screen asks the game to do.
pub enum ScreenCmd {
    Equip(u8),
    Unequip(u8),
    Eat(u8),
    Sell(u8),
    /// put a bag slot on the trade table
    Offer(u8),
    /// put an ability on action-bar key `slot` (0-based)
    Bind(u16, u8),
    /// tidy the bags: stacks joined, best things first
    Sort,
    /// sell every poor (grey) item to the merchant
    SellJunk,
    Close,
    Switch(Tab),
}

/// One profession skill as the server reports it: name, skill, cap.
pub struct SkillLine<'a> {
    pub name: &'a str,
    pub value: u16,
    pub max: u16,
}

pub struct View<'a> {
    pub content: &'a Content,
    pub name: &'a str,
    pub class: Option<&'a Class>,
    pub level: u8,
    pub money: u32,
    pub pack: &'a [Option<Stack>],
    pub equip: &'a [Option<Stack>],
    pub health: f32,
    pub power: f32,
    /// talking to someone (selling allowed)
    pub at_vendor: bool,
    /// a trade window is open (right-click offers)
    pub trading: bool,
    /// ability IDs on the 12 action-bar keys
    pub bar: &'a [Option<u16>],
    /// the names of those keys
    pub keys: &'a [&'a str],
    pub skills: &'a [SkillLine<'a>],
    /// stat points from buffs
    pub extra: dyno_world::content::Stats,
}

pub fn quality_color(q: u8) -> Color32 {
    theme::quality_color(q)
}

const BAD: Color32 = Color32::from_rgb(255, 84, 72);
const BETTER: Color32 = Color32::from_rgb(64, 224, 80);

/// The pack's word for an enum value: `item.slot.mainhand`, `item.weapon.sword`.
fn word<T: std::fmt::Debug>(group: &str, v: T) -> String {
    t(&format!("{group}.{}", format!("{v:?}").to_lowercase()))
}

fn resource_name(r: Resource) -> String {
    word("ability.resource", r)
}

/// A line with something at each end.
fn two(ui: &mut egui::Ui, left: String, right: String) {
    ui.horizontal(|ui| {
        ui.label(left);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(right);
        });
    });
}

/// A small arrow, up and green for better or down and red for worse, then
/// the line.
fn delta_line(ui: &mut egui::Ui, better: bool, text: String) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (r, _) = ui.allocate_exact_size(Vec2::new(11.0, 14.0), Sense::hover());
        let c = r.center();
        let (col, s) = if better { (BETTER, -1.0) } else { (BAD, 1.0) };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                c + Vec2::new(0.0, 4.5 * s),
                c + Vec2::new(-4.5, -3.5 * s),
                c + Vec2::new(4.5, -3.5 * s),
            ],
            col,
            Stroke::NONE,
        ));
        ui.label(RichText::new(text).color(col));
    });
}

/// What wearing `g` instead of `worn` would change: (amount, word), better
/// when the amount is positive.
fn compare(g: &Gear, worn: Option<&Gear>) -> Vec<(f32, String)> {
    let zero = dyno_world::content::Stats::default();
    let (ws, wa) = worn.map(|w| (w.stats, w.armor)).unwrap_or((zero, 0));
    let dps = |g: Option<&Gear>| {
        g.and_then(|g| g.weapon)
            .map(|w| (w.min + w.max) * 0.5 / w.speed.max(0.1))
    };
    let mut out = Vec::new();
    if let Some(mine) = dps(Some(g)) {
        let d = mine - dps(worn).unwrap_or(0.0);
        if d.abs() >= 0.05 {
            out.push((d, tf("item.delta_dps", &[("n", &format!("{d:+.1}"))])));
        }
    }
    let d = g.armor as f32 - wa as f32;
    if d != 0.0 {
        out.push((d, tf("item.delta_armor", &[("n", &format!("{d:+.0}"))])));
    }
    for (new, old, key) in [
        (g.stats.str, ws.str, "str"),
        (g.stats.agi, ws.agi, "agi"),
        (g.stats.sta, ws.sta, "sta"),
        (g.stats.int, ws.int, "int"),
        (g.stats.spi, ws.spi, "spi"),
    ] {
        let d = new - old;
        if d != 0 {
            out.push((
                d as f32,
                format!("{d:+} {}", t(&format!("sheet.{key}.name"))),
            ));
        }
    }
    out
}

/// The classic item tooltip.
pub fn item_tooltip(ui: &mut egui::Ui, c: &Content, s: &Stack, class: Option<&Class>, level: u8) {
    item_tooltip_vs(ui, c, s, class, level, None);
}

/// The item tooltip, and under it how the item compares with what is worn
/// in its place (`equip`: the paper doll; `None` for an item already worn).
pub fn item_tooltip_vs(
    ui: &mut egui::Ui,
    c: &Content,
    s: &Stack,
    class: Option<&Class>,
    level: u8,
    equip: Option<&[Option<Stack>]>,
) {
    let it = c.item(s.item);
    ui.set_max_width(270.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    theme::tooltip_title(ui, it.name, quality_color(it.quality));
    let mut usable = true;
    if let Some(g) = it.gear {
        let kind = match (g.weapon, g.armor_type) {
            (Some(w), _) => word("item.weapon", w.kind),
            (None, Some(a)) => word("item.armor", a),
            _ => String::new(),
        };
        two(ui, word("item.slot", g.slot), kind);
        if let Some(w) = g.weapon {
            two(
                ui,
                tf(
                    "item.damage",
                    &[
                        ("min", &format!("{:.0}", w.min)),
                        ("max", &format!("{:.0}", w.max)),
                    ],
                ),
                tf("item.speed", &[("n", &format!("{:.2}", w.speed))]),
            );
            ui.label(tf(
                "item.dps",
                &[("n", &format!("{:.1}", (w.min + w.max) * 0.5 / w.speed))],
            ));
        }
        if g.armor > 0 {
            ui.label(tf("item.armor_points", &[("n", &g.armor.to_string())]));
        }
        let st = g.stats;
        for (v, key) in [
            (st.str, "str"),
            (st.agi, "agi"),
            (st.sta, "sta"),
            (st.int, "int"),
            (st.spi, "spi"),
        ] {
            if v != 0 {
                ui.label(
                    RichText::new(format!("{v:+} {}", t(&format!("sheet.{key}.name"))))
                        .color(Color32::WHITE),
                );
            }
        }
        if it.level > 1 {
            let col = if it.level > level { BAD } else { theme::TEXT };
            ui.label(
                RichText::new(tf("item.requires_level", &[("n", &it.level.to_string())]))
                    .color(col),
            );
            usable &= it.level <= level;
        }
        if let Some(cl) = class
            && let Some(why) = character::cannot_equip(cl, level, &it)
        {
            usable = false;
            if it.level <= level {
                ui.label(RichText::new(why).color(BAD));
            }
        }
    } else if let Some(tool) = it.tool {
        ui.label(tf("item.tool", &[("n", &tool.tier.to_string())]));
    }
    if it.food > 0 {
        ui.label(
            RichText::new(tf("item.food", &[("n", &(it.food as u32 * 5).to_string())]))
                .color(BETTER),
        );
    }
    if it.block.is_some() && it.gear.is_none() {
        ui.label(RichText::new(t("item.material")).color(theme::TEXT_DIM));
    }
    if it.quality == 0 && it.gear.is_none() && it.food == 0 && it.tool.is_none() && it.value > 0 {
        ui.label(RichText::new(t("item.junk")).color(theme::TEXT_DIM));
    }
    if it.value > 0 {
        let each = money_text(it.value * s.count as u32);
        ui.label(RichText::new(tf("item.sell", &[("price", &each)])).color(theme::TEXT_DIM));
    }
    // against what is worn in its place
    if let (Some(g), Some(equip), true) = (it.gear, equip, usable) {
        let place = character::places_for(g.slot).into_iter().next();
        let worn = place
            .and_then(|i| equip.get(i))
            .and_then(|s| s.as_ref())
            .map(|s| c.item(s.item));
        let lines = compare(g, worn.as_ref().and_then(|w| w.gear));
        ui.add_space(4.0);
        theme::ui_divider(ui, theme::GOLD_DARK);
        match &worn {
            Some(w) => {
                ui.label(
                    RichText::new(tf("item.compare", &[("item", w.name)])).color(theme::TEXT_DIM),
                );
            }
            None => {
                ui.label(RichText::new(t("item.compare_empty")).color(theme::TEXT_DIM));
            }
        }
        if lines.is_empty() {
            ui.label(RichText::new(t("item.compare_same")).color(theme::TEXT_DIM));
        }
        for (d, text) in lines {
            delta_line(ui, d > 0.0, text);
        }
    }
}

/// An ability's tooltip: name, cost and range, cast time and cooldown, what
/// it does; all from the ability's data.
pub fn ability_tooltip(ui: &mut egui::Ui, c: &Content, a: &Ability, known: bool) {
    ui.set_max_width(290.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    theme::tooltip_title(ui, &a.name, Color32::WHITE);
    let res = c
        .class(a.class)
        .map(|k| resource_name(k.resource))
        .unwrap_or_default();
    let cost = if a.cost > 0.0 {
        tf(
            "ability.cost",
            &[("n", &format!("{:.0}", a.cost)), ("resource", &res)],
        )
    } else {
        String::new()
    };
    let reach = if a.range > 0.0 {
        tf("ability.range", &[("n", &format!("{:.0}", a.range))])
    } else if a.radius > 0.0 {
        tf("ability.radius", &[("n", &format!("{:.0}", a.radius))])
    } else {
        String::new()
    };
    if !cost.is_empty() || !reach.is_empty() {
        two(ui, cost, reach);
    }
    let cast = if a.cast > 0.0 {
        tf("ability.cast", &[("n", &format!("{:.1}", a.cast))])
    } else {
        t("ability.instant")
    };
    let cooldown = if a.cooldown >= 60.0 {
        tf(
            "ability.cooldown_min",
            &[("n", &format!("{:.0}", a.cooldown / 60.0))],
        )
    } else if a.cooldown > 0.0 {
        tf(
            "ability.cooldown_sec",
            &[("n", &format!("{:.0}", a.cooldown))],
        )
    } else {
        String::new()
    };
    two(ui, cast, cooldown);
    ui.add_space(2.0);
    ui.label(RichText::new(&a.text).color(theme::GOLD_BRIGHT));
    if !known {
        ui.label(RichText::new(tf("ability.learn", &[("n", &a.level.to_string())])).color(BAD));
    }
}

/// A square slot with an icon (or a ghost label), quality border and hover.
fn slot_button(
    ui: &mut egui::Ui,
    size: f32,
    c: &Content,
    s: Option<&Stack>,
    ghost: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let p = ui.painter();
    match s {
        Some(st) => {
            theme::item_slot(p, rect, c.item(st.item).quality, None);
            survival::stack_icon(p, rect, c, st);
        }
        None => theme::item_slot(p, rect, 0, (!ghost.is_empty()).then_some(ghost)),
    }
    if resp.hovered() {
        p.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.5_f32, theme::GOLD_BRIGHT),
            egui::StrokeKind::Inside,
        );
    }
    resp
}

fn tabs(ui: &mut egui::Ui, tab: Tab, out: &mut Vec<ScreenCmd>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (k, key) in [
            (Tab::Character, "panels.character"),
            (Tab::Bags, "panels.bags"),
            (Tab::Spellbook, "panels.spellbook"),
            (Tab::Skills, "panels.skills"),
        ] {
            if theme::tab(ui, &t(key), k == tab).clicked() && k != tab {
                out.push(ScreenCmd::Switch(k));
            }
        }
    });
    theme::tab_rule(ui);
    ui.add_space(4.0);
}

/// A dark button with gold words.
fn button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(text)
                .font(theme::title_font(12.5))
                .color(theme::GOLD),
        )
        .fill(theme::WOOD_DARK)
        .stroke(Stroke::new(1.0_f32, theme::GOLD_DARK))
        .min_size(Vec2::new(0.0, 26.0)),
    )
}

/// The character, bags, spellbook and skills window.
pub fn window(ctx: &egui::Context, v: &View, tab: Tab) -> Vec<ScreenCmd> {
    let mut out = Vec::new();
    let mut open = true;
    let (title, sub) = match (tab, v.class) {
        (Tab::Character, Some(class)) => (
            v.name.to_string(),
            tf(
                "panels.level_class",
                &[("level", &v.level.to_string()), ("class", &class.name)],
            ),
        ),
        (Tab::Character, None) => (v.name.to_string(), String::new()),
        (Tab::Bags, _) => (t("panels.bags_title"), String::new()),
        (Tab::Spellbook, _) => (t("panels.spellbook_title"), String::new()),
        (Tab::Skills, _) => (t("panels.skills_title"), String::new()),
    };
    theme::window(
        ctx,
        "screens",
        &title,
        &sub,
        470.0,
        (Align2::RIGHT_CENTER, Vec2::new(-300.0, 20.0)),
        &mut open,
        |ui| {
            tabs(ui, tab, &mut out);
            match tab {
                Tab::Character => character_tab(ui, v, &mut out),
                Tab::Bags => bags_tab(ui, v, &mut out),
                Tab::Spellbook => spellbook_tab(ui, v),
                Tab::Skills => skills_tab(ui, v),
            }
        },
    );
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        out.push(ScreenCmd::Close);
    }
    out
}

fn dim(ui: &mut egui::Ui, text: String) {
    ui.label(
        RichText::new(text)
            .font(theme::body_font(13.0))
            .color(theme::TEXT_DIM),
    );
}

/// A stat with its value; pointing at it says what the stat does.
fn stat(ui: &mut egui::Ui, key: &str, value: String) {
    let r = ui
        .scope(|ui| theme::stat_row(ui, &t(&format!("sheet.{key}.name")), &value, Color32::WHITE))
        .response;
    let text = crate::uitext::opt(&format!("sheet.{key}.text"));
    if !text.is_empty() {
        r.on_hover_ui(|ui| {
            ui.set_max_width(260.0);
            theme::tooltip_title(ui, &t(&format!("sheet.{key}.name")), theme::GOLD_BRIGHT);
            ui.label(RichText::new(text).color(theme::TEXT));
        });
    }
}

fn character_tab(ui: &mut egui::Ui, v: &View, out: &mut Vec<ScreenCmd>) {
    let c = v.content;
    let Some(class) = v.class else {
        dim(ui, t("panels.no_class"));
        return;
    };
    let sheet: Sheet = character::sheet(c, class, v.level, v.equip, &v.extra);
    let place = |ui: &mut egui::Ui, i: usize, out: &mut Vec<ScreenCmd>| {
        let s = v.equip.get(i).and_then(|s| s.as_ref());
        let name = word("item.slot", EQUIP_PLACES[i].0);
        let r = slot_button(ui, 42.0, c, s, &name);
        if let Some(st) = s {
            let r = r.on_hover_ui(|ui| {
                item_tooltip(ui, c, st, Some(class), v.level);
                ui.add_space(3.0);
                dim(ui, t("panels.take_off"));
            });
            if r.secondary_clicked() || r.double_clicked() {
                out.push(ScreenCmd::Unequip(i as u8));
            }
        } else {
            r.on_hover_text(name);
        }
    };
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
        // the paper doll's left side
        ui.vertical(|ui| {
            for i in 0..8 {
                place(ui, i, out);
            }
        });
        // what it all adds up to
        ui.vertical(|ui| {
            ui.set_width(346.0);
            ui.spacing_mut().item_spacing.y = 1.0;
            theme::heading(ui, &t("sheet.attributes"), theme::GOLD);
            let st = sheet.stats;
            stat(
                ui,
                "health",
                format!("{:.0} / {:.0}", v.health, sheet.max_health),
            );
            let res = format!("{:?}", class.resource).to_lowercase();
            stat(ui, &res, format!("{:.0} / {:.0}", v.power, sheet.max_power));
            stat(ui, "str", st.str.to_string());
            stat(ui, "agi", st.agi.to_string());
            stat(ui, "sta", st.sta.to_string());
            stat(ui, "int", st.int.to_string());
            stat(ui, "spi", st.spi.to_string());
            ui.add_space(6.0);
            theme::heading(ui, &t("sheet.combat"), theme::GOLD);
            stat(
                ui,
                "damage",
                tf(
                    "sheet.damage.value",
                    &[
                        ("min", &format!("{:.0}", sheet.weapon.0)),
                        ("max", &format!("{:.0}", sheet.weapon.1)),
                        ("speed", &format!("{:.1}", sheet.weapon.2)),
                    ],
                ),
            );
            stat(ui, "dps", format!("{:.1}", sheet.dps));
            stat(ui, "attack_power", format!("{:.0}", sheet.attack_power));
            stat(ui, "spell_power", format!("{:.0}", sheet.spell_power));
            stat(ui, "crit", format!("{:.1}%", sheet.crit));
            stat(ui, "spell_crit", format!("{:.1}%", sheet.spell_crit));
            stat(
                ui,
                "armor",
                tf(
                    "sheet.armor.value",
                    &[
                        ("n", &format!("{:.0}", sheet.armor)),
                        (
                            "percent",
                            &format!(
                                "{:.0}",
                                character::armor_reduction(sheet.armor, v.level) * 100.0
                            ),
                        ),
                    ],
                ),
            );
            ui.add_space(8.0);
            let weapons: Vec<String> = class
                .weapons
                .iter()
                .map(|w| word("item.weapon", *w))
                .collect();
            dim(
                ui,
                tf(
                    "sheet.wears",
                    &[("armor", &word("item.armor", class.armor))],
                ),
            );
            dim(ui, tf("sheet.wields", &[("weapons", &weapons.join(", "))]));
        });
        // and its right side
        ui.vertical(|ui| {
            for i in 8..14 {
                place(ui, i, out);
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space((ui.available_width() - 3.0 * 42.0 - 16.0) * 0.5);
        for i in 14..17 {
            place(ui, i, out);
        }
    });
}

fn bags_tab(ui: &mut egui::Ui, v: &View, out: &mut Vec<ScreenCmd>) {
    let c = v.content;
    dim(
        ui,
        t(if v.trading {
            "panels.bags_hint_trade"
        } else if v.at_vendor {
            "panels.bags_hint_vendor"
        } else {
            "panels.bags_hint"
        }),
    );
    ui.add_space(2.0);
    theme::leather(ui, |ui| {
        egui::Grid::new("bag-grid")
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for (i, s) in v.pack.iter().enumerate() {
                    let r = slot_button(ui, 44.0, c, s.as_ref(), "");
                    if let Some(st) = s {
                        let r = r
                            .on_hover_ui(|ui| {
                                item_tooltip_vs(ui, c, st, v.class, v.level, Some(v.equip))
                            })
                            .on_hover_cursor(CursorIcon::PointingHand);
                        if r.secondary_clicked() {
                            let shift = ui.input(|i| i.modifiers.shift);
                            let it = c.item(st.item);
                            if v.trading && !shift {
                                out.push(ScreenCmd::Offer(i as u8));
                            } else if v.at_vendor && !shift {
                                out.push(ScreenCmd::Sell(i as u8));
                            } else if it.gear.is_some() {
                                out.push(ScreenCmd::Equip(i as u8));
                            } else if it.food > 0 {
                                out.push(ScreenCmd::Eat(i as u8));
                            }
                        }
                    }
                    if i % 9 == 8 {
                        ui.end_row();
                    }
                }
            });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        coins(ui, v.money);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button(ui, &t("panels.sort")).clicked() {
                out.push(ScreenCmd::Sort);
            }
            let junk: u32 = v
                .pack
                .iter()
                .flatten()
                .map(|s| (c.item(s.item), s.count as u32))
                .filter(|(it, _)| is_junk(it))
                .map(|(it, n)| it.value * n)
                .sum();
            if v.at_vendor && junk > 0 {
                let label = tf("panels.sell_junk", &[("price", &money_text(junk))]);
                if button(ui, &label).clicked() {
                    out.push(ScreenCmd::SellJunk);
                }
            }
            let free = v.pack.iter().filter(|s| s.is_none()).count();
            dim(ui, tf("panels.free", &[("n", &free.to_string())]));
        });
    });
}

/// Poor things with no use but their price.
pub fn is_junk(it: &dyno_world::content::ItemView) -> bool {
    it.quality == 0 && it.value > 0 && it.gear.is_none() && it.food == 0 && it.tool.is_none()
}

/// Money as coins: a gold, silver and copper disc with each amount.
pub fn coins(ui: &mut egui::Ui, copper: u32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let parts = [
            (copper / 10_000, Color32::from_rgb(255, 205, 70)),
            (copper / 100 % 100, Color32::from_rgb(200, 204, 212)),
            (copper % 100, Color32::from_rgb(196, 122, 70)),
        ];
        for (k, (n, col)) in parts.iter().enumerate() {
            if *n == 0 && k < 2 {
                continue;
            }
            ui.label(
                RichText::new(n.to_string())
                    .font(theme::bold_font(15.0))
                    .color(theme::TEXT),
            );
            let (r, _) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
            let p = ui.painter();
            p.circle_filled(r.center(), 6.0, theme::OUTLINE);
            p.circle_filled(r.center(), 5.0, *col);
            p.circle_filled(
                r.center() - Vec2::new(1.3, 1.3),
                2.0,
                Color32::from_white_alpha(90),
            );
            ui.add_space(4.0);
        }
    });
}

fn spellbook_tab(ui: &mut egui::Ui, v: &View) {
    let c = v.content;
    let Some(class) = v.class else {
        dim(ui, t("panels.no_class"));
        return;
    };
    dim(ui, t("panels.spellbook_hint"));
    let mut list: Vec<&Ability> = c.abilities.iter().filter(|a| a.class == class.id).collect();
    list.sort_by_key(|a| (a.level, a.id));
    if let Some(next) = list.iter().find(|a| a.level > v.level) {
        ui.label(
            RichText::new(tf(
                "panels.next_ability",
                &[("ability", &next.name), ("level", &next.level.to_string())],
            ))
            .font(theme::body_font(14.0))
            .color(theme::GOLD),
        );
    }
    ui.add_space(4.0);
    let height = (ui.ctx().content_rect().height() - 330.0).clamp(240.0, 520.0);
    egui::ScrollArea::vertical()
        .max_height(height)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for a in list {
                let known = a.level <= v.level;
                let (row, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(
                    row,
                    4.0,
                    if known {
                        Color32::from_rgb(44, 32, 22)
                    } else {
                        Color32::from_rgb(28, 21, 16)
                    },
                );
                p.rect_stroke(
                    row,
                    4.0,
                    Stroke::new(1.0_f32, theme::BRONZE),
                    egui::StrokeKind::Inside,
                );
                let icon = Rect::from_min_size(row.min + Vec2::splat(6.0), Vec2::splat(44.0));
                theme::slot_well(p, icon);
                survival::ability_icon(p, icon.shrink(5.0), c, a);
                if !known {
                    p.rect_filled(icon.shrink(2.0), 3.0, Color32::from_black_alpha(165));
                }
                theme::slot_rim(p, icon, false);
                let ink = if known { theme::TEXT } else { theme::TEXT_DIM };
                let x = icon.right() + 10.0;
                p.text(
                    Pos2::new(x, row.top() + 6.0),
                    Align2::LEFT_TOP,
                    &a.name,
                    theme::title_font(14.5),
                    if known { theme::GOLD_BRIGHT } else { ink },
                );
                // where it is: the key it is on, or the level it comes at
                let on = v.bar.iter().position(|s| *s == Some(a.id));
                let (tag, tag_col) = if !known {
                    (tf("panels.learn_at", &[("n", &a.level.to_string())]), BAD)
                } else if let Some(k) = on {
                    (
                        tf(
                            "panels.on_key",
                            &[("key", v.keys.get(k).copied().unwrap_or(""))],
                        ),
                        theme::GOOD,
                    )
                } else {
                    (t("panels.not_on_bar"), theme::EMBER)
                };
                p.text(
                    Pos2::new(row.right() - 8.0, row.top() + 7.0),
                    Align2::RIGHT_TOP,
                    tag,
                    theme::body_font(12.5),
                    tag_col,
                );
                // what it does, in a line or two
                let galley = p.layout(
                    a.text.clone(),
                    theme::body_font(13.0),
                    ink,
                    row.right() - 8.0 - x,
                );
                p.with_clip_rect(Rect::from_min_max(
                    Pos2::new(x, row.top() + 24.0),
                    row.max - Vec2::splat(3.0),
                ))
                .galley(Pos2::new(x, row.top() + 25.0), galley, ink);
                // the whole row explains itself; the icon of a known ability drags
                let resp = ui.interact(
                    row,
                    ui.id().with(("ability", a.id)),
                    if known {
                        Sense::click_and_drag()
                    } else {
                        Sense::hover()
                    },
                );
                if known {
                    resp.dnd_set_drag_payload(BarDrag::Ability(a.id));
                }
                if !egui::DragAndDrop::has_any_payload(ui.ctx()) {
                    resp.on_hover_ui(|ui| ability_tooltip(ui, c, a, known));
                }
                ui.add_space(1.0);
            }
        });
}

fn skills_tab(ui: &mut egui::Ui, v: &View) {
    theme::heading(ui, &t("panels.professions"), theme::GOLD);
    if v.skills.is_empty() {
        dim(ui, t("panels.no_skills"));
    }
    for s in v.skills {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [130.0, 20.0],
                egui::Label::new(
                    RichText::new(s.name)
                        .font(theme::title_font(13.5))
                        .color(theme::TEXT),
                ),
            );
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width() - 4.0, 18.0), Sense::hover());
            theme::resource_bar(
                ui.painter(),
                r,
                s.value as f32,
                s.max.max(1) as f32,
                theme::GOLD_DARK,
                &format!("{} / {}", s.value, s.max),
            );
        });
    }
    ui.add_space(8.0);
    dim(ui, t("panels.skills_hint"));
}

/// The moves that tidy a pack: equal stacks joined, then gear, food, other
/// things and junk, each best first. Pairs of bag slots to move wholly from
/// and to, in order (the server's own stacking rules applied to a copy).
pub fn sort_moves(c: &Content, pack: &[Option<Stack>]) -> Vec<(u8, u8)> {
    let mut slots = pack.to_vec();
    let mut moves = Vec::new();
    let rank = |s: &Option<Stack>| match s {
        None => (9u8, 0u8, 0u8, String::new()),
        Some(s) => {
            let it = c.item(s.item);
            let group = if it.gear.is_some() {
                0
            } else if it.food > 0 {
                1
            } else if it.tool.is_some() {
                2
            } else if is_junk(&it) {
                4
            } else {
                3
            };
            (group, 255 - it.quality, 255 - it.level, it.name.to_string())
        }
    };
    let n = slots.len();
    for i in 0..n {
        // the best of what is left comes here
        let best = (i..n).min_by_key(|j| rank(&slots[*j])).unwrap_or(i);
        if slots[best].is_none() {
            break;
        }
        if best != i {
            dyno_sim::inventory::transfer_within(c, &mut slots, best, i, 0);
            moves.push((best as u8, i as u8));
        }
        // and more of the same joins it, while it has room
        for j in i + 1..n {
            let (Some(a), Some(b)) = (slots[i], slots[j]) else {
                continue;
            };
            if a.item == b.item && a.count < c.item(a.item).stack {
                let before = slots[j];
                dyno_sim::inventory::transfer_within(c, &mut slots, j, i, 0);
                if slots[j] != before {
                    moves.push((j as u8, i as u8));
                }
            }
        }
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use dyno_world::content::{Stats, Weapon, WeaponKind};

    fn gear(armor: u16, str: i16, sta: i16, weapon: Option<(f32, f32, f32)>) -> Gear {
        Gear {
            slot: Slot::MainHand,
            armor_type: None,
            weapon: weapon.map(|(min, max, speed)| Weapon {
                kind: WeaponKind::Sword,
                min,
                max,
                speed,
            }),
            armor,
            stats: Stats {
                str,
                sta,
                ..Stats::default()
            },
        }
    }

    #[test]
    fn comparison_lists_only_what_changes_with_its_sign() {
        let new = gear(10, 3, 1, Some((4.0, 8.0, 2.0)));
        let worn = gear(10, 1, 2, Some((2.0, 4.0, 2.0)));
        let lines = compare(&new, Some(&worn));
        // damage per second up, strength up, stamina down; armour the same
        let signs: Vec<bool> = lines.iter().map(|(d, _)| *d > 0.0).collect();
        assert_eq!(signs, vec![true, true, false]);
        // against an empty place everything it has is a gain
        assert!(compare(&new, None).iter().all(|(d, _)| *d > 0.0));
        assert!(compare(&new, Some(&new)).is_empty());
    }

    #[test]
    fn sorting_joins_stacks_and_puts_gear_first_and_junk_last() {
        let c = Content::default_pack();
        let pick = |f: &dyn Fn(&dyno_world::content::ItemView) -> bool| {
            c.items
                .iter()
                .map(|i| c.item(i.id))
                .find(|it| f(it))
                .map(|it| it.id)
                .expect("the pack has such an item")
        };
        let food = pick(&|it| it.food > 0 && it.stack >= 10 && it.gear.is_none());
        let sword = pick(&|it| it.gear.is_some());
        let junk = pick(&is_junk);
        let mut pack = vec![None; 12];
        pack[1] = Some(Stack::new(junk, 1));
        pack[4] = Some(Stack::new(food, 2));
        pack[7] = Some(Stack::new(sword, 1));
        pack[9] = Some(Stack::new(food, 3));
        // play the moves as the server would
        for (from, to) in sort_moves(&c, &pack) {
            dyno_sim::inventory::transfer_within(&c, &mut pack, from as usize, to as usize, 0);
        }
        assert_eq!(pack[0].map(|s| s.item), Some(sword));
        assert_eq!(pack[1], Some(Stack::new(food, 5)));
        assert_eq!(pack[2].map(|s| s.item), Some(junk));
        assert!(pack[3..].iter().all(|s| s.is_none()));
        // a tidy pack needs no moves
        assert!(sort_moves(&c, &pack).is_empty());
    }
}
