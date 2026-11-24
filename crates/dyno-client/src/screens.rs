//! Classic MMO screens: Character (C), Bags (B), Spellbook (P) and Skills (K),
//! with item and ability tooltips.

use crate::item_ui;
use crate::theme;
use dyno_protocol::Stack;
use dyno_sim::character::{self, Sheet};
use dyno_sim::progression::money_text;
use dyno_world::content::{Ability, ArmorType, Class, Content, EQUIP_PLACES, Slot, WeaponKind};
use egui::{Align2, Color32, CursorIcon, RichText, Sense, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Character,
    Bags,
    Spellbook,
    Skills,
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
    /// ability IDs on keys 1 to 7
    pub bar: &'a [Option<u16>],
    pub skills: &'a [SkillLine<'a>],
    /// stat points from buffs
    pub extra: dyno_world::content::Stats,
}

pub fn quality_color(q: u8) -> Color32 {
    match q {
        0 => Color32::from_rgb(157, 157, 157),
        1 => Color32::from_rgb(240, 240, 240),
        2 => Color32::from_rgb(30, 220, 40),
        3 => Color32::from_rgb(40, 130, 255),
        _ => Color32::from_rgb(180, 70, 240),
    }
}

fn slot_name(s: Slot) -> &'static str {
    match s {
        Slot::Head => "Head",
        Slot::Neck => "Neck",
        Slot::Shoulders => "Shoulder",
        Slot::Back => "Back",
        Slot::Chest => "Chest",
        Slot::Wrist => "Wrist",
        Slot::Hands => "Hands",
        Slot::Waist => "Waist",
        Slot::Legs => "Legs",
        Slot::Feet => "Feet",
        Slot::Ring => "Finger",
        Slot::Trinket => "Trinket",
        Slot::MainHand => "Main Hand",
        Slot::OffHand => "Off Hand",
        Slot::TwoHand => "Two-Hand",
        Slot::Ranged => "Ranged",
    }
}

fn weapon_name(k: WeaponKind) -> &'static str {
    match k {
        WeaponKind::Sword => "Sword",
        WeaponKind::Axe => "Axe",
        WeaponKind::Mace => "Mace",
        WeaponKind::Dagger => "Dagger",
        WeaponKind::Staff => "Staff",
        WeaponKind::Polearm => "Polearm",
        WeaponKind::Bow => "Bow",
        WeaponKind::Wand => "Wand",
        WeaponKind::Shield => "Shield",
        WeaponKind::Offhand => "Held in Off-hand",
    }
}

fn armor_name(a: ArmorType) -> &'static str {
    match a {
        ArmorType::Cloth => "Cloth",
        ArmorType::Leather => "Leather",
        ArmorType::Mail => "Mail",
        ArmorType::Plate => "Plate",
    }
}

/// The classic item tooltip.
pub fn item_tooltip(ui: &mut egui::Ui, c: &Content, s: &Stack, class: Option<&Class>, level: u8) {
    let it = c.item(s.item);
    ui.set_max_width(260.0);
    ui.label(
        RichText::new(it.name)
            .strong()
            .size(15.0)
            .color(quality_color(it.quality)),
    );
    if let Some(g) = it.gear {
        ui.horizontal(|ui| {
            ui.label(slot_name(g.slot));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(w) = g.weapon {
                    ui.label(weapon_name(w.kind));
                } else if let Some(a) = g.armor_type {
                    ui.label(armor_name(a));
                }
            });
        });
        if let Some(w) = g.weapon {
            ui.horizontal(|ui| {
                ui.label(format!("{:.0} - {:.0} Damage", w.min, w.max));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Speed {:.2}", w.speed));
                });
            });
            ui.label(format!(
                "({:.1} damage per second)",
                (w.min + w.max) * 0.5 / w.speed
            ));
        }
        if g.armor > 0 {
            ui.label(format!("{} Armor", g.armor));
        }
        let st = g.stats;
        for (v, n) in [
            (st.str, "Strength"),
            (st.agi, "Agility"),
            (st.sta, "Stamina"),
            (st.int, "Intellect"),
            (st.spi, "Spirit"),
        ] {
            if v != 0 {
                ui.label(format!("{v:+} {n}"));
            }
        }
        if it.level > 1 {
            let col = if it.level > level {
                Color32::from_rgb(255, 60, 60)
            } else {
                Color32::WHITE
            };
            ui.label(RichText::new(format!("Requires Level {}", it.level)).color(col));
        }
        if let Some(cl) = class
            && let Some(why) = character::cannot_equip(cl, level, &it)
            && it.level <= level
        {
            ui.label(RichText::new(why).color(Color32::from_rgb(255, 60, 60)));
        }
    } else if let Some(t) = it.tool {
        let kind = format!("{:?}", t.kind);
        ui.label(format!("{kind} (gathering tool, tier {})", t.tier));
    }
    if it.food > 0 {
        ui.label(
            RichText::new(format!(
                "Use: Restores health while out of combat ({} food).",
                it.food
            ))
            .color(Color32::from_rgb(30, 220, 40)),
        );
    }
    if it.block.is_some() && it.gear.is_none() {
        ui.label(RichText::new("Building material").weak());
    }
    if it.value > 0 {
        let each = money_text(it.value * s.count as u32);
        ui.label(RichText::new(format!("Sell price: {each}")).color(Color32::from_gray(200)));
    }
}

pub fn ability_tooltip(ui: &mut egui::Ui, c: &Content, a: &Ability, known: bool) {
    ui.set_max_width(280.0);
    ui.label(
        RichText::new(&a.name)
            .strong()
            .size(15.0)
            .color(Color32::WHITE),
    );
    let res = c.class(a.class).map(|k| match k.resource {
        dyno_world::content::Resource::Rage => "Rage",
        dyno_world::content::Resource::Mana => "Mana",
        dyno_world::content::Resource::Energy => "Energy",
    });
    ui.horizontal(|ui| {
        if a.cost > 0.0 {
            ui.label(format!("{:.0} {}", a.cost, res.unwrap_or("")));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if a.range > 0.0 {
                ui.label(format!("{:.0} yd range", a.range));
            } else if a.radius > 0.0 {
                ui.label(format!("{:.0} yd radius", a.radius));
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label(if a.cast > 0.0 {
            format!("{:.1} sec cast", a.cast)
        } else {
            "Instant".into()
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if a.cooldown > 0.0 {
                let cd = if a.cooldown >= 60.0 {
                    format!("{:.0} min cooldown", a.cooldown / 60.0)
                } else {
                    format!("{:.0} sec cooldown", a.cooldown)
                };
                ui.label(cd);
            }
        });
    });
    ui.label(RichText::new(&a.text).color(Color32::from_rgb(255, 210, 80)));
    if !known {
        ui.label(
            RichText::new(format!("Learned at level {}", a.level))
                .color(Color32::from_rgb(255, 60, 60)),
        );
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
    p.rect_filled(rect, 4.0, Color32::from_rgb(24, 18, 14));
    match s {
        Some(st) => {
            item_ui::stack_icon(p, rect, c, st);
            let q = c.item(st.item).quality;
            p.rect_stroke(
                rect,
                4.0,
                Stroke::new(1.5_f32, quality_color(q).gamma_multiply(0.9)),
                egui::StrokeKind::Inside,
            );
        }
        None => {
            p.rect_stroke(
                rect,
                4.0,
                Stroke::new(1.0_f32, Color32::from_rgb(90, 70, 40)),
                egui::StrokeKind::Inside,
            );
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                ghost,
                egui::FontId::proportional(9.0),
                Color32::from_gray(110),
            );
        }
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
        for (t, n, key) in [
            (Tab::Character, "Character", "C"),
            (Tab::Bags, "Bags", "B"),
            (Tab::Spellbook, "Spellbook", "P"),
            (Tab::Skills, "Skills", "K"),
        ] {
            let label = RichText::new(format!("{n} ({key})")).color(if t == tab {
                theme::GOLD_BRIGHT
            } else {
                theme::TEXT_DIM
            });
            if ui.selectable_label(t == tab, label).clicked() && t != tab {
                out.push(ScreenCmd::Switch(t));
            }
        }
    });
    ui.separator();
}

/// The character, bags, spellbook and skills window.
pub fn window(ctx: &egui::Context, v: &View, tab: Tab) -> Vec<ScreenCmd> {
    let mut out = Vec::new();
    let mut open = true;
    let title = match tab {
        Tab::Character => v.name.to_string(),
        Tab::Bags => "Bags".into(),
        Tab::Spellbook => "Spellbook".into(),
        Tab::Skills => "Skills".into(),
    };
    egui::Window::new(RichText::new(title).size(20.0).color(theme::GOLD))
        .id(egui::Id::new("screens"))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::RIGHT_CENTER, Vec2::new(-40.0, 0.0))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_width(470.0);
            tabs(ui, tab, &mut out);
            match tab {
                Tab::Character => character_tab(ui, v, &mut out),
                Tab::Bags => bags_tab(ui, v, &mut out),
                Tab::Spellbook => spellbook_tab(ui, v, &mut out),
                Tab::Skills => skills_tab(ui, v),
            }
        });
    if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        out.push(ScreenCmd::Close);
    }
    out
}

fn character_tab(ui: &mut egui::Ui, v: &View, out: &mut Vec<ScreenCmd>) {
    let c = v.content;
    let Some(class) = v.class else {
        ui.label("Choose a class first.");
        return;
    };
    let sheet: Sheet = character::sheet(c, class, v.level, v.equip, &v.extra);
    let class_col = Color32::from_rgb(
        (class.color[0] * 255.0) as u8,
        (class.color[1] * 255.0) as u8,
        (class.color[2] * 255.0) as u8,
    );
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(format!("Level {} {}", v.level, class.name))
                .size(15.0)
                .color(class_col),
        );
    });
    ui.add_space(4.0);
    let place = |ui: &mut egui::Ui, i: usize, out: &mut Vec<ScreenCmd>| {
        let s = v.equip.get(i).and_then(|s| s.as_ref());
        let r = slot_button(ui, 40.0, c, s, EQUIP_PLACES[i].1);
        if let Some(st) = s {
            let r = r.on_hover_ui(|ui| {
                item_tooltip(ui, c, st, Some(class), v.level);
                ui.label(RichText::new("Right click to take off").weak().small());
            });
            if r.secondary_clicked() || r.double_clicked() {
                out.push(ScreenCmd::Unequip(i as u8));
            }
        } else {
            r.on_hover_text(EQUIP_PLACES[i].1);
        }
    };
    ui.horizontal(|ui| {
        // left column of the paper doll
        ui.vertical(|ui| {
            for i in 0..8 {
                place(ui, i, out);
            }
        });
        // stats in the middle
        ui.vertical(|ui| {
            ui.set_width(330.0);
            ui.add_space(4.0);
            let st = sheet.stats;
            egui::Grid::new("stats")
                .num_columns(2)
                .spacing([24.0, 3.0])
                .show(ui, |ui| {
                    let row = |ui: &mut egui::Ui, n: &str, val: String| {
                        ui.label(RichText::new(n).color(theme::TEXT_DIM));
                        ui.label(RichText::new(val).color(Color32::WHITE));
                        ui.end_row();
                    };
                    row(
                        ui,
                        "Health",
                        format!("{:.0} / {:.0}", v.health, sheet.max_health),
                    );
                    let res = match class.resource {
                        dyno_world::content::Resource::Rage => "Rage",
                        dyno_world::content::Resource::Mana => "Mana",
                        dyno_world::content::Resource::Energy => "Energy",
                    };
                    row(ui, res, format!("{:.0} / {:.0}", v.power, sheet.max_power));
                    row(ui, "Strength", st.str.to_string());
                    row(ui, "Agility", st.agi.to_string());
                    row(ui, "Stamina", st.sta.to_string());
                    row(ui, "Intellect", st.int.to_string());
                    row(ui, "Spirit", st.spi.to_string());
                    row(
                        ui,
                        "Armor",
                        format!(
                            "{:.0}  ({:.0}% less damage)",
                            sheet.armor,
                            character::armor_reduction(sheet.armor, v.level) * 100.0
                        ),
                    );
                    row(
                        ui,
                        "Damage",
                        format!(
                            "{:.0} - {:.0}  (speed {:.1})",
                            sheet.weapon.0, sheet.weapon.1, sheet.weapon.2
                        ),
                    );
                    row(ui, "Damage per second", format!("{:.1}", sheet.dps));
                    row(ui, "Attack power", format!("{:.0}", sheet.attack_power));
                    row(ui, "Spell power", format!("{:.0}", sheet.spell_power));
                    row(
                        ui,
                        "Critical strike",
                        format!("{:.1}%  (spells {:.1}%)", sheet.crit, sheet.spell_crit),
                    );
                });
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Wears {} and lighter", armor_name(class.armor)))
                    .small()
                    .color(theme::TEXT_DIM),
            );
            let weapons: Vec<&str> = class.weapons.iter().map(|w| weapon_name(*w)).collect();
            ui.label(
                RichText::new(format!("Weapons: {}", weapons.join(", ")))
                    .small()
                    .color(theme::TEXT_DIM),
            );
        });
        // right column
        ui.vertical(|ui| {
            for i in 8..14 {
                place(ui, i, out);
            }
        });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(150.0);
        for i in 14..17 {
            place(ui, i, out);
        }
    });
}

fn bags_tab(ui: &mut egui::Ui, v: &View, out: &mut Vec<ScreenCmd>) {
    let c = v.content;
    ui.label(
        RichText::new(if v.at_vendor {
            "Right click: sell (you are at a merchant). Shift + right click: use or equip."
        } else {
            "Right click: equip or use."
        })
        .small()
        .color(theme::TEXT_DIM),
    );
    ui.add_space(4.0);
    egui::Grid::new("bag-grid")
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (i, s) in v.pack.iter().enumerate() {
                let r = slot_button(ui, 44.0, c, s.as_ref(), "");
                if let Some(st) = s {
                    let r = r
                        .on_hover_ui(|ui| item_tooltip(ui, c, st, v.class, v.level))
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
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(money_text(v.money)).color(theme::GOLD));
        let free = v.pack.iter().filter(|s| s.is_none()).count();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{free} free")).color(theme::TEXT_DIM));
        });
    });
}

fn spellbook_tab(ui: &mut egui::Ui, v: &View, out: &mut Vec<ScreenCmd>) {
    let c = v.content;
    let Some(class) = v.class else {
        ui.label("Choose a class first.");
        return;
    };
    ui.label(
        RichText::new("Click a number to put the ability on that key.")
            .small()
            .color(theme::TEXT_DIM),
    );
    let mut list: Vec<&Ability> = c.abilities.iter().filter(|a| a.class == class.id).collect();
    list.sort_by_key(|a| (a.level, a.id));
    egui::ScrollArea::vertical()
        .max_height(420.0)
        .show(ui, |ui| {
            for a in list {
                let known = a.level <= v.level;
                ui.horizontal(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(rect, 4.0, Color32::from_black_alpha(160));
                    if let Some(item) = c.item_by_key(&a.icon) {
                        item_ui::item_icon(p, rect.shrink(4.0), c, item);
                    }
                    if !known {
                        p.rect_filled(rect, 4.0, Color32::from_black_alpha(170));
                    }
                    p.rect_stroke(
                        rect,
                        4.0,
                        Stroke::new(1.0_f32, theme::GOLD_DARK),
                        egui::StrokeKind::Inside,
                    );
                    resp.on_hover_ui(|ui| ability_tooltip(ui, c, a, known));
                    ui.vertical(|ui| {
                        let col = if known {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(120)
                        };
                        ui.label(RichText::new(&a.name).strong().color(col));
                        ui.label(
                            RichText::new(if known {
                                format!("Level {}", a.level)
                            } else {
                                format!("Learn at level {}", a.level)
                            })
                            .small()
                            .color(theme::TEXT_DIM),
                        );
                    });
                    if known {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            for k in (0..12u8).rev() {
                                let on = v.bar.get(k as usize).copied().flatten() == Some(a.id);
                                let key =
                                    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="]
                                        [k as usize];
                                let b = egui::Button::new(RichText::new(key).small()).selected(on);
                                if ui.add(b).clicked() {
                                    out.push(ScreenCmd::Bind(a.id, k));
                                }
                            }
                        });
                    }
                });
                ui.add_space(2.0);
            }
        });
}

fn skills_tab(ui: &mut egui::Ui, v: &View) {
    ui.label(RichText::new("Professions").strong().color(theme::GOLD));
    if v.skills.is_empty() {
        ui.label(
            RichText::new("Gather from trees, ore and bushes to learn a profession.")
                .color(theme::TEXT_DIM),
        );
    }
    for s in v.skills {
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.name).color(Color32::WHITE));
            let f = s.value as f32 / s.max.max(1) as f32;
            ui.add(
                egui::ProgressBar::new(f)
                    .desired_width(260.0)
                    .text(format!("{} / {}", s.value, s.max)),
            );
        });
    }
}
