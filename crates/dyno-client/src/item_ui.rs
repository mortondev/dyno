//! Item UI: item and block icons, and the crafting window (the bags beside the
//! recipes of the stations in reach; in build mode, every item).

use crate::theme;
use dyno_protocol::{Mode, SlotRef, Stack};
use dyno_sim::crafting::{self, Stations};
use dyno_sim::inventory::Inventory;
use dyno_world as w;
use dyno_world::content::{Content, Icon, ItemId};
use egui::{Align2, Color32, Pos2, Rect, RichText, Shape, Stroke, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(240, 138, 60);

fn col(rgb: [f32; 3], k: f32) -> Color32 {
    Color32::from_rgb(
        (rgb[0] * k * 255.0).min(255.0) as u8,
        (rgb[1] * k * 255.0).min(255.0) as u8,
        (rgb[2] * k * 255.0).min(255.0) as u8,
    )
}

/// A small isometric cube for a block.
/// Every block texture in one egui image, for icons.
#[derive(Clone)]
struct Atlas {
    tex: egui::TextureHandle,
    cols: usize,
    rows: usize,
}

const ATLAS_COLS: usize = 16;

fn atlas(ctx: &egui::Context) -> Atlas {
    // built again when the block textures change (more of the pack arrived)
    let id = egui::Id::new(("dyno-block-atlas", crate::textures::generation()));
    if let Some(a) = ctx.data(|d| d.get_temp::<Atlas>(id)) {
        return a;
    }
    let t = crate::render::textures();
    let tile = crate::textures::TILE;
    let n = t.layers.len().max(1);
    let (cols, rows) = (ATLAS_COLS, n.div_ceil(ATLAS_COLS));
    let (wd, ht) = (cols * tile, rows * tile);
    let mut buf = vec![0u8; wd * ht * 4];
    for (i, layer) in t.layers.iter().enumerate() {
        let (ox, oy) = ((i % cols) * tile, (i / cols) * tile);
        for y in 0..tile {
            let src = &layer[y * tile * 4..(y + 1) * tile * 4];
            let o = ((oy + y) * wd + ox) * 4;
            buf[o..o + tile * 4].copy_from_slice(src);
        }
    }
    let img = egui::ColorImage::from_rgba_unmultiplied([wd, ht], &buf);
    let tex = ctx.load_texture("dyno-blocks", img, egui::TextureOptions::NEAREST);
    let a = Atlas { tex, cols, rows };
    ctx.data_mut(|d| d.insert_temp(id, a.clone()));
    a
}

impl Atlas {
    fn uv(&self, layer: u16, x: f32, y: f32) -> Pos2 {
        let (l, c) = (layer as usize, self.cols);
        Pos2::new(
            ((l % c) as f32 + x) / c as f32,
            ((l / c) as f32 + y) / self.rows as f32,
        )
    }
}

/// An item's picture from the pack, loaded once it has arrived. None while
/// it is missing (the caller draws a plain swatch).
fn item_texture(ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
    let bytes = crate::assets::get(path)?;
    // keyed by the bytes too, so a replaced file shows
    let id = egui::Id::new(("dyno-item", path, bytes.as_ptr() as usize));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let (w, h, px) = crate::textures::decode_png(&bytes)?;
    let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &px);
    let t = ctx.load_texture(path, img, egui::TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

pub fn block_icon(painter: &egui::Painter, r: Rect, b: w::BlockId) {
    let a = atlas(painter.ctx());
    let t = crate::render::textures();
    if w::shape(b) == w::Shape::Post {
        // a torch: the side texture as an upright stick
        let layer = t.face(b, 1);
        let c = r.center();
        let (hw, hh) = (r.width() * 0.11, r.height() * 0.46);
        let rect = Rect::from_center_size(c, Vec2::new(hw * 2.0, hh * 2.0));
        painter.image(
            a.tex.id(),
            rect,
            Rect::from_min_max(a.uv(layer, 6.5 / 16.0, 0.0), a.uv(layer, 9.5 / 16.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    let c = r.center();
    let s = r.width() * 0.5;
    let top = Pos2::new(c.x, c.y - s);
    let left = Pos2::new(c.x - s * 0.87, c.y - s * 0.5);
    let right = Pos2::new(c.x + s * 0.87, c.y - s * 0.5);
    let mid = Pos2::new(c.x, c.y);
    let bl = Pos2::new(c.x - s * 0.87, c.y + s * 0.5);
    let br = Pos2::new(c.x + s * 0.87, c.y + s * 0.5);
    let bot = Pos2::new(c.x, c.y + s);
    let glow = w::emission(b) > 0;
    let (top_l, side_l) = (t.face(b, 0), t.face(b, 1));
    let mut mesh = egui::Mesh::with_texture(a.tex.id());
    let mut quad = |pts: [Pos2; 4], layer: u16, shade: u8| {
        let i = mesh.vertices.len() as u32;
        let tint = Color32::from_gray(if glow { 255 } else { shade });
        for (p, (x, y)) in pts
            .into_iter()
            .zip([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
        {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p,
                uv: a.uv(layer, x, y),
                color: tint,
            });
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    };
    quad([left, top, right, mid], top_l, 255);
    quad([left, mid, bot, bl], side_l, 205);
    quad([mid, right, br, bot], side_l, 160);
    painter.add(Shape::mesh(mesh));
}

pub fn item_icon(painter: &egui::Painter, r: Rect, c: &Content, item: ItemId) {
    let it = c.item(item);
    if let (Icon::Block, Some(b)) = (it.icon, it.block) {
        return block_icon(painter, r.shrink(r.width() * 0.08), b);
    }
    if let Some(t) = it.image.and_then(|p| item_texture(painter.ctx(), p)) {
        painter.image(
            t.id(),
            r.shrink(r.width() * 0.02),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    // no picture (or not arrived yet): the block it places, or a plain
    // swatch of its colour
    match it.block {
        Some(b) => block_icon(painter, r.shrink(r.width() * 0.08), b),
        None => {
            painter.rect_filled(r.shrink(r.width() * 0.25), 3.0, col(it.color, 1.0));
        }
    }
}

/// A slot with its item, count and wear bar.
pub fn stack_icon(painter: &egui::Painter, r: Rect, c: &Content, s: &Stack) {
    item_icon(painter, r.shrink(r.width() * 0.12), c, s.item);
    if s.count > 1 {
        let t = format!("{}", s.count);
        painter.text(
            r.right_bottom() + Vec2::new(-3.0, -1.0),
            Align2::RIGHT_BOTTOM,
            &t,
            theme::bold_font(13.0),
            Color32::BLACK,
        );
        painter.text(
            r.right_bottom() + Vec2::new(-4.0, -2.0),
            Align2::RIGHT_BOTTOM,
            t,
            theme::bold_font(13.0),
            Color32::from_rgb(255, 246, 220),
        );
    }
    if let Some(tool) = c.item(s.item).tool
        && s.wear > 0
    {
        let left = 1.0 - s.wear as f32 / tool.durability.max(1) as f32;
        let bar = Rect::from_min_size(
            Pos2::new(r.left() + 4.0, r.bottom() - 6.0),
            Vec2::new(r.width() - 8.0, 3.0),
        );
        painter.rect_filled(bar, 1.0, Color32::from_black_alpha(200));
        let g = Color32::from_rgb((255.0 * (1.0 - left)) as u8, (255.0 * left) as u8, 40);
        painter.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(bar.width() * left, 3.0)),
            1.0,
            g,
        );
    }
}

/// What the player did in the pack screen.
pub enum PackCmd {
    Move(SlotRef, SlotRef, u8),
    Craft(u16, u8),
    Close,
    /// build mode: put an item in the selected hotbar slot
    Creative(ItemId),
}

pub struct PackView<'a> {
    pub content: &'a Content,
    pub pack: &'a [Option<Stack>],
    pub stations: Stations,
    pub fuel: u16,
    pub mode: Mode,
    pub selected: &'a mut Option<SlotRef>,
    pub hotbar_slot: usize,
}

fn slot_at(v: &PackView, s: SlotRef) -> Option<Stack> {
    match s {
        SlotRef::Pack(i) => v.pack.get(i as usize).copied().flatten(),
        SlotRef::Chest(_) => None,
    }
}

/// First slot in the other area where a stack can go (for shift-click).
fn quick_target(v: &PackView, from: SlotRef) -> Option<SlotRef> {
    let s = slot_at(v, from)?;
    let max = v.content.item(s.item).stack;
    let candidates: Vec<SlotRef> = match from {
        SlotRef::Pack(i) if i < 9 => (9..36).map(SlotRef::Pack).collect(),
        _ => (0..9).map(SlotRef::Pack).collect(),
    };
    let merge = candidates.iter().copied().find(|c| matches!(slot_at(v, *c), Some(t) if t.item == s.item && t.wear == 0 && s.wear == 0 && t.count < max && max > 1));
    merge.or_else(|| candidates.into_iter().find(|c| slot_at(v, *c).is_none()))
}

fn slot_ui(ui: &mut egui::Ui, v: &mut PackView, s: SlotRef, out: &mut Vec<PackCmd>) {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(48.0), egui::Sense::click());
    let sel = *v.selected == Some(s);
    let hot = matches!(s, SlotRef::Pack(i) if i as usize == v.hotbar_slot);
    let p = ui.painter();
    theme::slot_well(p, r);
    if resp.hovered() {
        p.rect_filled(r.shrink(2.0), 3.0, Color32::from_white_alpha(22));
    }
    let stack = slot_at(v, s);
    if let Some(st) = &stack {
        stack_icon(p, r, v.content, st);
    }
    theme::slot_rim(p, r, sel);
    if hot && !sel {
        p.rect_stroke(
            r.shrink(2.0),
            3.0,
            Stroke::new(1.0_f32, theme::GOLD.gamma_multiply(0.7)),
            egui::StrokeKind::Inside,
        );
    }
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.clicked() {
        if shift {
            if let Some(t) = quick_target(v, s) {
                out.push(PackCmd::Move(s, t, 0));
            }
        } else {
            match *v.selected {
                Some(from) if from != s => {
                    out.push(PackCmd::Move(from, s, 0));
                    *v.selected = None;
                }
                Some(_) => *v.selected = None,
                None if stack.is_some() => *v.selected = Some(s),
                None => {}
            }
        }
    }
    if resp.secondary_clicked() {
        if let Some(from) = *v.selected {
            if from != s {
                out.push(PackCmd::Move(from, s, 1));
            }
        } else if let Some(st) = stack {
            // right click: pick up half
            if st.count > 1 {
                *v.selected = Some(s);
            }
        }
    }
    if let Some(st) = stack {
        let it = v.content.item(st.item);
        let mut tip = it.name.to_string();
        if let Some(t) = it.tool {
            tip += &format!(
                "\nTier {} · {} uses left",
                t.tier,
                t.durability.saturating_sub(st.wear)
            );
        }
        if it.food > 0 {
            tip += &format!(
                "\nFood: heals {}% of your health, out of combat",
                it.food as u32 * 5
            );
        }
        resp.on_hover_text(
            tip + "\nClick to pick up, click another slot to put down. Shift-click to move.",
        );
    }
}

/// A small tag saying whether a crafting station is in reach.
fn station_tag(ui: &mut egui::Ui, near: bool, name: &str) {
    let text = if near {
        format!("{name} nearby")
    } else {
        format!("no {name}")
    };
    let col = if near { theme::GOOD } else { theme::TEXT_DIM };
    let galley = ui
        .painter()
        .layout_no_wrap(text, theme::body_font(12.5), col);
    let (r, _) = ui.allocate_exact_size(galley.size() + Vec2::new(22.0, 4.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 9.0, theme::WOOD_DARK);
    p.rect_stroke(
        r,
        9.0,
        Stroke::new(1.0_f32, col.gamma_multiply(0.45)),
        egui::StrokeKind::Inside,
    );
    let dot = Pos2::new(r.left() + 9.0, r.center().y);
    if near {
        p.circle_filled(dot, 5.0, theme::EMBER.gamma_multiply(0.25));
        p.circle_filled(dot, 3.0, theme::EMBER);
    } else {
        p.circle_stroke(dot, 3.0, Stroke::new(1.0_f32, col));
    }
    p.galley(
        Pos2::new(r.left() + 16.0, r.center().y - galley.size().y * 0.5),
        galley,
        col,
    );
}

const SLOT_GAP: f32 = 4.0;
const BAG_W: f32 = 9.0 * 48.0 + 8.0 * SLOT_GAP;
const CRAFT_W: f32 = 380.0;

/// The pack screen: the bags and the recipe list.
pub fn craft_window(
    ctx: &egui::Context,
    mut v: PackView,
    recipes_filter: &mut String,
) -> Vec<PackCmd> {
    let mut out = Vec::new();
    let (id, title) = ("bags-window", "Bags");
    let mut open = true;
    theme::window(
        ctx,
        id,
        title,
        "",
        BAG_W + 24.0 + 12.0 + CRAFT_W + 20.0,
        (Align2::CENTER_CENTER, Vec2::ZERO),
        &mut open,
        |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ui.vertical(|ui| {
                    theme::leather(ui, |ui| {
                        ui.set_width(BAG_W);
                        theme::heading(ui, "Backpack", theme::GOLD);
                        egui::Grid::new("pack")
                            .spacing([SLOT_GAP, SLOT_GAP])
                            .show(ui, |ui| {
                                for i in 9..36 {
                                    slot_ui(ui, &mut v, SlotRef::Pack(i as u8), &mut out);
                                    if (i - 8) % 9 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                        ui.add_space(6.0);
                        theme::heading(ui, "Belt", theme::GOLD);
                        egui::Grid::new("hot")
                            .spacing([SLOT_GAP, SLOT_GAP])
                            .show(ui, |ui| {
                                for i in 0..9 {
                                    slot_ui(ui, &mut v, SlotRef::Pack(i as u8), &mut out);
                                }
                            });
                    });
                    ui.add_space(6.0);
                    let hints: &[&str] = &[
                        "Click a slot to pick up, click another to put down.",
                        "Right-click places one. Shift-click moves between bag and belt.",
                        "E or Esc closes.",
                    ];
                    for hint in hints.iter().copied() {
                        ui.label(
                            RichText::new(hint)
                                .italics()
                                .size(13.0)
                                .color(theme::TEXT_DIM),
                        );
                    }
                });
                egui::Frame::new()
                    .fill(theme::WOOD_DARK)
                    .stroke(Stroke::new(1.0_f32, theme::BRONZE))
                    .corner_radius(5.0)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.vertical(|ui| side_panel(ui, &mut v, recipes_filter, &mut out))
                    });
            });
            if ui.input(|i| i.key_pressed(egui::Key::E) || i.key_pressed(egui::Key::Escape)) {
                out.push(PackCmd::Close);
            }
        },
    );
    if !open {
        out.push(PackCmd::Close);
    }
    out
}

/// The right side of the bags: the creative palette, or the recipes.
fn side_panel(
    ui: &mut egui::Ui,
    v: &mut PackView,
    recipes_filter: &mut String,
    out: &mut Vec<PackCmd>,
) {
    ui.set_width(CRAFT_W);
    // the list has a fixed height, so the window is the right size from its first frame
    let list_height = 300.0;
    ui.set_min_height(list_height + 92.0);
    if v.mode == Mode::Creative {
        theme::heading(ui, "Build", theme::GOLD);
        ui.label(
            RichText::new("Click any item to put it in the selected belt slot.")
                .italics()
                .color(theme::TEXT_DIM),
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(list_height + 50.0)
            .show(ui, |ui| {
                egui::Grid::new("creative")
                    .spacing([4.0, 4.0])
                    .show(ui, |ui| {
                        let mut items: Vec<ItemId> = w::placeable().map(|b| b as ItemId).collect();
                        items.extend(v.content.items.iter().map(|i| i.id));
                        for (k, item) in items.into_iter().enumerate() {
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::click());
                            theme::slot_well(ui.painter(), r);
                            if resp.hovered() {
                                ui.painter().rect_filled(
                                    r.shrink(2.0),
                                    3.0,
                                    Color32::from_white_alpha(26),
                                );
                            }
                            item_icon(ui.painter(), r.shrink(5.0), v.content, item);
                            theme::slot_rim(ui.painter(), r, resp.hovered());
                            if resp.on_hover_text(v.content.item(item).name).clicked() {
                                out.push(PackCmd::Creative(item));
                            }
                            if (k + 1) % 8 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        return;
    }
    theme::heading(ui, "Crafting", theme::GOLD);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (i, s) in v.content.stations.iter().enumerate() {
            station_tag(ui, v.stations.has(Some(i as u8)), &s.name);
        }
        if v.fuel > 0 {
            ui.label(
                RichText::new(format!("fuel {}", v.fuel))
                    .size(13.0)
                    .color(theme::EMBER),
            );
        }
    });
    ui.add_space(2.0);
    ui.add(
        egui::TextEdit::singleline(recipes_filter)
            .hint_text("Search recipes")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(2.0);
    let inv = Inventory {
        slots: v.pack.to_vec(),
    };
    let list = crafting::list(v.content, &inv, v.stations, v.fuel);
    let filter = recipes_filter.to_lowercase();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(list_height)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            // craftable first, then the rest, grouped
            let mut rows: Vec<_> = list
                .into_iter()
                .filter(|(r, _)| {
                    filter.is_empty()
                        || v.content
                            .item(r.output)
                            .name
                            .to_lowercase()
                            .contains(&filter)
                })
                .collect();
            rows.sort_by_key(|(r, ok)| (ok.is_err(), r.group.clone(), r.id));
            for (r, ok) in rows {
                let can = ok.is_ok();
                let frame = egui::Frame::new()
                    .fill(if can {
                        Color32::from_rgb(54, 38, 25)
                    } else {
                        Color32::from_rgb(30, 22, 16)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if can {
                            theme::GOLD_DARK
                        } else {
                            Color32::from_rgb(56, 42, 28)
                        },
                    ))
                    .corner_radius(5.0)
                    .inner_margin(egui::Margin::symmetric(6, 5));
                frame.show(ui, |ui| {
                    ui.set_width((ui.available_width() - 10.0).max(40.0));
                    ui.horizontal(|ui| {
                        let (ir, _) =
                            ui.allocate_exact_size(Vec2::splat(38.0), egui::Sense::hover());
                        theme::slot_well(ui.painter(), ir);
                        item_icon(ui.painter(), ir.shrink(4.0), v.content, r.output);
                        if !can {
                            ui.painter().rect_filled(
                                ir.shrink(2.0),
                                2.0,
                                Color32::from_black_alpha(110),
                            );
                        }
                        theme::slot_rim(ui.painter(), ir, false);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let name = v.content.item(r.output).name;
                            let label = if r.count > 1 {
                                format!("{name} ×{}", r.count)
                            } else {
                                name.to_string()
                            };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(label).font(theme::bold_font(15.0)).color(
                                    if can {
                                        theme::GOLD_BRIGHT
                                    } else {
                                        theme::TEXT_DIM
                                    },
                                ));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if can {
                                            if ui
                                                .small_button("×5")
                                                .on_hover_text("Craft five times")
                                                .clicked()
                                            {
                                                out.push(PackCmd::Craft(r.id, 5));
                                            }
                                            if ui.small_button("Craft").clicked() {
                                                out.push(PackCmd::Craft(r.id, 1));
                                            }
                                        } else if let Err(why) = &ok {
                                            ui.label(RichText::new("?").color(theme::TEXT_DIM))
                                                .on_hover_text(why.as_str());
                                        }
                                    },
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for ing in &r.inputs {
                                    let have: u32 = ing.any.iter().map(|i| inv.count(*i)).sum();
                                    let (ir, _) = ui.allocate_exact_size(
                                        Vec2::splat(18.0),
                                        egui::Sense::hover(),
                                    );
                                    item_icon(ui.painter(), ir, v.content, ing.any[0]);
                                    let c = if have >= ing.count as u32 {
                                        Color32::from_rgb(170, 220, 170)
                                    } else {
                                        Color32::from_rgb(230, 130, 120)
                                    };
                                    ui.label(
                                        RichText::new(format!("{}", ing.count))
                                            .font(theme::bold_font(13.0))
                                            .color(c),
                                    );
                                    ui.add_space(4.0);
                                }
                                let note =
                                    |t: String| RichText::new(t).size(12.5).color(theme::TEXT_DIM);
                                let station = r
                                    .station
                                    .and_then(|i| v.content.stations.get(i as usize))
                                    .map(|s| s.name.as_str());
                                match (station, r.fuel) {
                                    (Some(s), 0) => ui.label(note(format!("· {s}"))),
                                    (Some(s), n) => ui.label(note(format!("· {s}, {n} fuel"))),
                                    (None, 0) => ui.label(""),
                                    (None, n) => ui.label(note(format!("· {n} fuel"))),
                                };
                            });
                        });
                    });
                });
            }
        });
}
