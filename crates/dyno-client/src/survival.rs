//! Survival UI: item icons, health and hunger, the pack screen with the recipe list.

use crate::theme;
use dyno_protocol::{Mode, SlotRef, Stack};
use dyno_sim::crafting::{self, Stations};
use dyno_sim::inventory::Inventory;
use dyno_world::content::{Content, Icon, ItemId, Station, ToolKind};
use dyno_world::{self as w, World};
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
    let id = egui::Id::new("dyno-block-atlas");
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

/// A pixel sprite for an item that is not a block, made once per item.
fn item_texture(ctx: &egui::Context, c: &Content, item: ItemId) -> egui::TextureHandle {
    let it = c.item(item);
    let id = egui::Id::new((
        "dyno-item",
        c.item_key(item).to_string(),
        it.name.to_string(),
    ));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let px = crate::textures::item_sprite(&it);
    let n = crate::textures::TILE;
    let img = egui::ColorImage::from_rgba_unmultiplied([n, n], &px);
    let t = ctx.load_texture(format!("item-{item}"), img, egui::TextureOptions::NEAREST);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
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
    if it.block.is_none() {
        let t = item_texture(painter.ctx(), c, item);
        painter.image(
            t.id(),
            r.shrink(r.width() * 0.02),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    let rgb = it.color;
    let center = r.center();
    let s = r.width() * 0.42;
    let wood = col([0.55, 0.40, 0.24], 1.0);
    match it.icon {
        Icon::Stick => {
            painter.line_segment(
                [center + Vec2::new(-s, s), center + Vec2::new(s, -s)],
                Stroke::new(s * 0.32, wood),
            );
        }
        Icon::Lump => {
            let pts: Vec<Pos2> = (0..7)
                .map(|i| {
                    let a = i as f32 / 7.0 * std::f32::consts::TAU;
                    let k = [0.9, 0.7, 1.0, 0.8, 0.95, 0.65, 0.85][i];
                    center + Vec2::new(a.cos(), a.sin()) * s * k
                })
                .collect();
            painter.add(Shape::convex_polygon(
                pts,
                col(rgb, 1.0),
                Stroke::new(1.0_f32, col(rgb, 0.6)),
            ));
        }
        Icon::Ingot => {
            let pts = vec![
                center + Vec2::new(-s * 0.6, -s * 0.35),
                center + Vec2::new(s * 0.6, -s * 0.35),
                center + Vec2::new(s * 0.95, s * 0.4),
                center + Vec2::new(-s * 0.95, s * 0.4),
            ];
            painter.add(Shape::convex_polygon(
                pts,
                col(rgb, 1.0),
                Stroke::new(1.0_f32, col(rgb, 0.6)),
            ));
        }
        Icon::Gem => {
            let pts = vec![
                center + Vec2::new(0.0, -s),
                center + Vec2::new(s * 0.7, 0.0),
                center + Vec2::new(0.0, s),
                center + Vec2::new(-s * 0.7, 0.0),
            ];
            painter.add(Shape::convex_polygon(
                pts,
                col(rgb, 1.0),
                Stroke::new(1.0_f32, col(rgb, 1.3)),
            ));
        }
        Icon::Dust => {
            for (dx, dy) in [
                (-0.4, 0.3),
                (0.2, 0.4),
                (0.4, -0.1),
                (-0.1, -0.3),
                (0.0, 0.05),
            ] {
                painter.circle_filled(
                    center + Vec2::new(dx * s * 1.6, dy * s * 1.6),
                    s * 0.22,
                    col(rgb, 1.0),
                );
            }
        }
        Icon::Berries => {
            for (dx, dy) in [(-0.35, 0.2), (0.35, 0.2), (0.0, -0.3)] {
                painter.circle_filled(
                    center + Vec2::new(dx * s * 1.4, dy * s * 1.4),
                    s * 0.42,
                    col(rgb, 1.0),
                );
            }
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -s * 0.5),
                    center + Vec2::new(s * 0.3, -s),
                ],
                Stroke::new(2.0_f32, col([0.25, 0.5, 0.2], 1.0)),
            );
        }
        Icon::Meat => {
            painter.circle_filled(center + Vec2::new(-s * 0.15, 0.0), s * 0.7, col(rgb, 1.0));
            painter.line_segment(
                [
                    center + Vec2::new(s * 0.3, s * 0.3),
                    center + Vec2::new(s, s),
                ],
                Stroke::new(s * 0.25, Color32::from_rgb(235, 225, 210)),
            );
        }
        Icon::Tool | Icon::Block => {
            let kind = it.tool.map(|t| t.kind).unwrap_or(ToolKind::Pickaxe);
            let head = col(rgb, 1.0);
            let a = center + Vec2::new(-s, s);
            let b = center + Vec2::new(s * 0.55, -s * 0.55);
            if kind != ToolKind::Sword {
                painter.line_segment([a, b], Stroke::new(s * 0.22, wood));
            }
            match kind {
                ToolKind::Pickaxe => {
                    let p = [
                        center + Vec2::new(-s * 0.2, -s * 0.95),
                        center + Vec2::new(s * 0.55, -s * 0.55),
                        center + Vec2::new(s * 0.95, s * 0.2),
                    ];
                    painter.add(Shape::line(p.to_vec(), Stroke::new(s * 0.3, head)));
                }
                ToolKind::Axe => {
                    let pts = vec![
                        b + Vec2::new(-s * 0.1, -s * 0.5),
                        b + Vec2::new(s * 0.45, -s * 0.2),
                        b + Vec2::new(s * 0.2, s * 0.45),
                        b + Vec2::new(-s * 0.15, 0.0),
                    ];
                    painter.add(Shape::convex_polygon(pts, head, Stroke::NONE));
                }
                ToolKind::Shovel | ToolKind::Hoe => {
                    painter.circle_filled(b + Vec2::new(s * 0.2, -s * 0.2), s * 0.38, head);
                }
                ToolKind::Sword => {
                    painter.line_segment(
                        [
                            center + Vec2::new(-s * 0.5, s * 0.5),
                            center + Vec2::new(s, -s),
                        ],
                        Stroke::new(s * 0.3, head),
                    );
                    painter.line_segment(
                        [
                            center + Vec2::new(-s * 0.75, s * 0.25),
                            center + Vec2::new(-s * 0.25, s * 0.75),
                        ],
                        Stroke::new(s * 0.2, wood),
                    );
                    painter.line_segment(
                        [
                            center + Vec2::new(-s * 0.5, s * 0.5),
                            center + Vec2::new(-s, s),
                        ],
                        Stroke::new(s * 0.22, wood),
                    );
                }
            }
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

/// Health and hunger as rows of pips above the hotbar.
pub fn bars(painter: &egui::Painter, left: Pos2, right: Pos2, health: f32, food: f32) {
    let pip = 9.0;
    let gap = 2.5;
    for i in 0..10 {
        let x = left.x + i as f32 * (pip + gap);
        let r = Rect::from_min_size(Pos2::new(x, left.y), Vec2::splat(pip));
        painter.rect_filled(r, 2.0, Color32::from_black_alpha(150));
        let v = (health - i as f32 * 2.0).clamp(0.0, 2.0) / 2.0;
        if v > 0.0 {
            painter.rect_filled(
                Rect::from_min_size(r.min, Vec2::new(pip * v, pip)).shrink(1.0),
                1.5,
                Color32::from_rgb(214, 52, 58),
            );
        }
        let x2 = right.x - (i as f32 + 1.0) * (pip + gap) + gap;
        let r2 = Rect::from_min_size(Pos2::new(x2, right.y), Vec2::splat(pip));
        painter.rect_filled(r2, 2.0, Color32::from_black_alpha(150));
        let f = (food - i as f32 * 2.0).clamp(0.0, 2.0) / 2.0;
        if f > 0.0 {
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(r2.right() - pip * f, r2.top()),
                    Vec2::new(pip * f, pip),
                )
                .shrink(1.0),
                1.5,
                Color32::from_rgb(222, 146, 58),
            );
        }
    }
}

/// Crafting stations within reach of a position (same rule as the server).
pub fn stations_near(world: &World, pos: [f32; 3]) -> Stations {
    let mut st = Stations::default();
    let (bx, by, bz) = (
        pos[0].floor() as i32,
        pos[1].floor() as i32,
        pos[2].floor() as i32,
    );
    for y in by - 3..=by + 4 {
        for z in bz - 5..=bz + 5 {
            for x in bx - 5..=bx + 5 {
                match world.get(x, y, z) {
                    w::CRAFTING_TABLE => st.table = true,
                    w::FURNACE => st.furnace = true,
                    _ => {}
                }
            }
        }
    }
    st
}

/// What the player did in the pack screen.
pub enum PackCmd {
    Move(SlotRef, SlotRef, u8),
    Throw(SlotRef, u8),
    Craft(u16, u8),
    Close,
    /// creative: put an item in the selected hotbar slot
    Creative(ItemId),
}

pub struct PackView<'a> {
    pub content: &'a Content,
    pub pack: &'a [Option<Stack>],
    pub chest: Option<&'a [Option<Stack>]>,
    pub stations: Stations,
    pub fuel: u16,
    pub mode: Mode,
    pub selected: &'a mut Option<SlotRef>,
    pub hotbar_slot: usize,
}

fn slot_at(v: &PackView, s: SlotRef) -> Option<Stack> {
    match s {
        SlotRef::Pack(i) => v.pack.get(i as usize).copied().flatten(),
        SlotRef::Chest(i) => v.chest.and_then(|c| c.get(i as usize).copied().flatten()),
    }
}

/// First slot in the other area where a stack can go (for shift-click).
fn quick_target(v: &PackView, from: SlotRef) -> Option<SlotRef> {
    let s = slot_at(v, from)?;
    let max = v.content.item(s.item).stack;
    let candidates: Vec<SlotRef> = match from {
        SlotRef::Pack(_i) if v.chest.is_some() => (0..v.chest.unwrap().len() as u8)
            .map(SlotRef::Chest)
            .collect(),
        SlotRef::Pack(i) if i < 9 => (9..36).map(SlotRef::Pack).collect(),
        SlotRef::Pack(_) => (0..9).map(SlotRef::Pack).collect(),
        SlotRef::Chest(_) => (0..36).map(SlotRef::Pack).collect(),
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
            tip += &format!("\nRestores {} hunger (right click to eat)", it.food);
        }
        resp.on_hover_text(tip + "\nClick to pick up, click another slot to put down. Shift-click to move. Q to throw.");
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

/// The pack screen: the bags, an open chest, and the recipe list.
pub fn pack_window(
    ctx: &egui::Context,
    mut v: PackView,
    recipes_filter: &mut String,
) -> Vec<PackCmd> {
    let mut out = Vec::new();
    let (id, title) = if v.chest.is_some() {
        ("chest-window", "Chest")
    } else {
        ("bags-window", "Bags")
    };
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
                    if let Some(chest) = v.chest {
                        theme::leather(ui, |ui| {
                            ui.set_width(BAG_W);
                            theme::heading(ui, "Chest", theme::GOLD);
                            egui::Grid::new("chest")
                                .spacing([SLOT_GAP, SLOT_GAP])
                                .show(ui, |ui| {
                                    for i in 0..chest.len() {
                                        slot_ui(ui, &mut v, SlotRef::Chest(i as u8), &mut out);
                                        if (i + 1) % 9 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                        ui.add_space(8.0);
                    }
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
                    ui.horizontal(|ui| {
                        if let Some(sel) = *v.selected
                            && ui.button("Throw selected").clicked()
                        {
                            out.push(PackCmd::Throw(sel, 0));
                            *v.selected = None;
                        }
                    });
                    for hint in [
                        "Click a slot to pick up, click another to put down.",
                        "Right-click places one. Shift-click moves between bag and belt.",
                        "Q throws the selected stack. E or Esc closes.",
                    ] {
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
    ui.set_min_height(300.0);
    if v.mode == Mode::Creative {
        theme::heading(ui, "Creative", theme::GOLD);
        ui.label(
            RichText::new("Click any item to put it in the selected belt slot.")
                .italics()
                .color(theme::TEXT_DIM),
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(if v.chest.is_some() { 470.0 } else { 350.0 })
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
        station_tag(ui, v.stations.table, "table");
        station_tag(ui, v.stations.furnace, "furnace");
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
    let height = if v.chest.is_some() { 420.0 } else { 300.0 };
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(height)
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
                                match r.station {
                                    Station::Table => ui.label(note("· table".into())),
                                    Station::Furnace => {
                                        ui.label(note(format!("· furnace, {} fuel", r.fuel)))
                                    }
                                    Station::None => ui.label(""),
                                };
                            });
                        });
                    });
                });
            }
        });
}
