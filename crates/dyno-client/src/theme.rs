//! The look of the interface: carved dark wood, gilded trim, parchment pages
//! and braziers ("light is safety"). Fonts, global egui visuals, and painter
//! helpers for frames, bars and action slots. Everything is drawn with shapes.
//!
//! The fonts come from the pack (`fonts/fonts.json`); egui's own font
//! stands in until they arrive.

use egui::epaint::{Mesh, Shadow};
use egui::{
    Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect,
    Sense, Shape, Stroke, StrokeKind, Vec2,
};
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::{Arc, LazyLock};

// ---- palette ----

pub const WOOD_DARK: Color32 = Color32::from_rgb(22, 16, 12);
pub const WOOD: Color32 = Color32::from_rgb(40, 29, 20);
pub const WOOD_LIGHT: Color32 = Color32::from_rgb(70, 49, 31);
pub const OUTLINE: Color32 = Color32::from_rgb(10, 7, 5);
pub const GOLD: Color32 = Color32::from_rgb(226, 182, 90);
pub const GOLD_BRIGHT: Color32 = Color32::from_rgb(255, 226, 140);
pub const GOLD_DARK: Color32 = Color32::from_rgb(132, 96, 42);
pub const BRONZE: Color32 = Color32::from_rgb(96, 72, 38);
pub const TEXT: Color32 = Color32::from_rgb(236, 224, 200);
pub const TEXT_DIM: Color32 = Color32::from_rgb(170, 154, 128);
pub const PARCHMENT: Color32 = Color32::from_rgb(232, 215, 176);
pub const PARCHMENT_EDGE: Color32 = Color32::from_rgb(198, 172, 122);
pub const INK: Color32 = Color32::from_rgb(52, 36, 22);
pub const INK_FADED: Color32 = Color32::from_rgb(112, 88, 60);
/// quest titles and headings on parchment: burnished gold ink
pub const INK_GOLD: Color32 = Color32::from_rgb(134, 84, 14);
pub const INK_GREEN: Color32 = Color32::from_rgb(46, 110, 40);
pub const LEATHER: Color32 = Color32::from_rgb(74, 48, 30);
pub const LEATHER_DARK: Color32 = Color32::from_rgb(48, 31, 20);
pub const STITCH: Color32 = Color32::from_rgb(186, 150, 96);
pub const EMBER: Color32 = Color32::from_rgb(255, 150, 50);
pub const GOOD: Color32 = Color32::from_rgb(140, 220, 140);
pub const HEALTH: Color32 = Color32::from_rgb(58, 168, 62);
pub const MANA: Color32 = Color32::from_rgb(56, 112, 216);
pub const RAGE: Color32 = Color32::from_rgb(200, 48, 40);
pub const ENERGY: Color32 = Color32::from_rgb(226, 196, 60);

// ---- fonts ----

static TITLE: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("title".into()));
static BODY: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("body".into()));
static BOLD: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("body-bold".into()));

/// The display font (Cinzel). Only valid on a context that had [`apply`] called.
pub fn title_font(size: f32) -> FontId {
    FontId::new(size, TITLE.clone())
}

/// The body font (Crimson Text); the same as `FontId::proportional` after [`apply`].
pub fn body_font(size: f32) -> FontId {
    FontId::new(size, BODY.clone())
}

/// The semi-bold body font, for numbers on bars and small labels.
pub fn bold_font(size: f32) -> FontId {
    FontId::new(size, BOLD.clone())
}

/// The pack file that names the fonts.
pub const FONTS_JSON: &str = "fonts/fonts.json";

/// `fonts/fonts.json`: font files by name, and the families built from them.
#[derive(serde::Deserialize)]
pub struct FontsFile {
    pub faces: HashMap<String, FontFace>,
    /// "proportional", "body", "body-bold" and "title": face names, first
    /// choice first
    pub families: HashMap<String, Vec<String>>,
}

#[derive(serde::Deserialize)]
pub struct FontFace {
    pub file: String,
    #[serde(default = "one")]
    pub scale: f32,
}

fn one() -> f32 {
    1.0
}

/// The font that stands in before the pack's arrive, and stays as the
/// fallback: one of egui's own (its others, symbols and emoji, are 1 MB the
/// game does not need; the client carries no content).
pub fn base_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert(
        "Ubuntu-Light".into(),
        Arc::new(FontData::from_static(epaint_default_fonts::UBUNTU_LIGHT)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.insert(family, vec!["Ubuntu-Light".into()]);
    }
    fonts
}

/// A new egui context with the stand-in font (egui has none of its own here).
pub fn new_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_fonts(base_fonts());
    ctx
}

/// The font set from the pack, as far as it has arrived; egui's own fonts
/// stand in for the rest and stay as fallbacks (symbols, emoji). Also
/// returns the bytes it was built from, to tell when to build it again.
fn pack_fonts() -> (FontDefinitions, Vec<usize>) {
    let mut fonts = base_fonts();
    let addr = |b: &Option<Arc<[u8]>>| b.as_ref().map(|b| b.as_ptr() as usize).unwrap_or(0);
    let json = crate::assets::get(FONTS_JSON);
    let mut source = vec![addr(&json)];
    let file: Option<FontsFile> = json.and_then(|b| serde_json::from_slice(&b).ok());
    let mut lists: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(f) = file {
        let mut faces: Vec<_> = f.faces.iter().collect();
        faces.sort_by_key(|(name, _)| *name);
        for (name, face) in faces {
            let bytes = crate::assets::get(&face.file);
            source.push(addr(&bytes));
            if let Some(b) = bytes {
                let tweak = egui::FontTweak {
                    scale: face.scale,
                    ..Default::default()
                };
                fonts.font_data.insert(
                    name.clone(),
                    Arc::new(FontData::from_owned(b.to_vec()).tweak(tweak)),
                );
            }
        }
        for (family, names) in f.families {
            let have = names
                .into_iter()
                .filter(|n| fonts.font_data.contains_key(n))
                .collect();
            lists.insert(family, have);
        }
    }
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut put = |family: FontFamily, key: &str| {
        let first = lists.get(key).cloned().unwrap_or_default();
        let list = first.into_iter().chain(fallback.iter().cloned()).collect();
        fonts.families.insert(family, list);
    };
    put(FontFamily::Proportional, "proportional");
    put(BODY.clone(), "body");
    put(BOLD.clone(), "body-bold");
    put(TITLE.clone(), "title");
    (fonts, source)
}

/// What a context's fonts were built from: the asset store's generation
/// when last checked, and the font files' bytes.
#[derive(Clone, Default)]
struct FontsFrom {
    generation: u64,
    source: Vec<usize>,
}

fn set_pack_fonts(ctx: &egui::Context, force: bool) {
    let id = egui::Id::new("dyno-pack-fonts");
    let was: FontsFrom = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    let generation = crate::assets::generation();
    if !force && was.generation == generation {
        return;
    }
    let (fonts, source) = pack_fonts();
    if force || source != was.source {
        ctx.set_fonts(fonts);
    }
    ctx.data_mut(|d| d.insert_temp(id, FontsFrom { generation, source }));
}

/// Install the pack's fonts if more of them have arrived since the last
/// call. Cheap; call every frame.
pub fn refresh_fonts(ctx: &egui::Context) {
    set_pack_fonts(ctx, false);
}

/// Install the fonts and the dark, gilded visuals. Call once per `egui::Context`,
/// before its first frame.
pub fn apply(ctx: &egui::Context) {
    set_pack_fonts(ctx, true);

    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals(visuals());
    ctx.style_mut(|s| {
        use egui::TextStyle as T;
        s.text_styles.insert(T::Heading, title_font(21.0));
        s.text_styles.insert(T::Body, body_font(15.0));
        s.text_styles.insert(T::Button, body_font(15.0));
        s.text_styles.insert(T::Small, body_font(12.0));
        s.spacing.button_padding = Vec2::new(10.0, 4.0);
        s.spacing.window_margin = egui::Margin::same(12);
    });
}

fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = WOOD_DARK;
    v.window_fill = Color32::from_rgb(32, 23, 17);
    v.window_stroke = Stroke::new(1.5_f32, GOLD_DARK);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 22,
        spread: 2,
        color: Color32::from_black_alpha(150),
    };
    v.popup_shadow = Shadow {
        offset: [0, 3],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    v.window_highlight_topmost = false;
    v.extreme_bg_color = Color32::from_rgb(14, 10, 8);
    v.faint_bg_color = Color32::from_rgb(44, 32, 23);
    v.code_bg_color = Color32::from_rgb(28, 20, 15);
    v.hyperlink_color = Color32::from_rgb(140, 190, 214);
    v.selection.bg_fill = GOLD_DARK;
    v.selection.stroke = Stroke::new(1.0_f32, GOLD_BRIGHT);
    let r = CornerRadius::same(4);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = Color32::from_rgb(32, 23, 17);
    w.noninteractive.weak_bg_fill = Color32::from_rgb(32, 23, 17);
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BRONZE);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.noninteractive.corner_radius = r;
    w.inactive.bg_fill = Color32::from_rgb(64, 45, 28);
    w.inactive.weak_bg_fill = Color32::from_rgb(64, 45, 28);
    w.inactive.bg_stroke = Stroke::new(1.0_f32, GOLD_DARK);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(240, 216, 160));
    w.inactive.corner_radius = r;
    w.hovered.bg_fill = Color32::from_rgb(88, 61, 35);
    w.hovered.weak_bg_fill = Color32::from_rgb(88, 61, 35);
    w.hovered.bg_stroke = Stroke::new(1.5_f32, GOLD);
    w.hovered.fg_stroke = Stroke::new(1.5_f32, GOLD_BRIGHT);
    w.hovered.corner_radius = r;
    w.active.bg_fill = Color32::from_rgb(110, 76, 38);
    w.active.weak_bg_fill = Color32::from_rgb(110, 76, 38);
    w.active.bg_stroke = Stroke::new(1.5_f32, GOLD_BRIGHT);
    w.active.fg_stroke = Stroke::new(2.0_f32, Color32::from_rgb(255, 244, 210));
    w.active.corner_radius = r;
    w.open.bg_fill = Color32::from_rgb(52, 37, 24);
    w.open.weak_bg_fill = Color32::from_rgb(52, 37, 24);
    w.open.bg_stroke = Stroke::new(1.0_f32, GOLD);
    w.open.fg_stroke = Stroke::new(1.0_f32, GOLD_BRIGHT);
    w.open.corner_radius = r;
    v
}

// ---- small shape helpers ----

fn alpha(c: Color32, k: f32) -> Color32 {
    c.gamma_multiply(k.clamp(0.0, 1.0))
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// A rectangle shaded from `top` to `bottom`.
pub fn gradient_rect(painter: &egui::Painter, r: Rect, top: Color32, bottom: Color32) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(m));
}

/// A rectangle shaded from `left` to `right`.
pub fn gradient_rect_h(painter: &egui::Painter, r: Rect, left: Color32, right: Color32) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), left);
    m.colored_vertex(r.right_top(), right);
    m.colored_vertex(r.right_bottom(), right);
    m.colored_vertex(r.left_bottom(), left);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(m));
}

/// A small square stood on its corner.
pub fn diamond(painter: &egui::Painter, c: Pos2, r: f32, fill: Color32, stroke: Stroke) {
    painter.add(Shape::convex_polygon(
        vec![
            c + Vec2::new(0.0, -r),
            c + Vec2::new(r, 0.0),
            c + Vec2::new(0.0, r),
            c + Vec2::new(-r, 0.0),
        ],
        fill,
        stroke,
    ));
}

/// Text with a soft dark shadow, for anything drawn over the world. Returns its rect.
pub fn shadow_text(
    painter: &egui::Painter,
    pos: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
) -> Rect {
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let rect = anchor.anchor_size(pos, galley.size());
    let shade = Color32::from_black_alpha((color.a() as f32 * 0.85) as u8);
    painter.galley_with_override_text_color(rect.min + Vec2::new(1.0, 1.5), galley.clone(), shade);
    painter.galley(rect.min, galley, color);
    rect
}

/// A decorative divider: two tapering lines and a diamond in the middle.
pub fn divider(painter: &egui::Painter, center: Pos2, width: f32, color: Color32) {
    let h = width * 0.5;
    let gap = 9.0;
    if h > gap + 4.0 {
        for s in [-1.0, 1.0] {
            let a = center + Vec2::new(s * gap, 0.0);
            let b = center + Vec2::new(s * h, 0.0);
            let mid = center + Vec2::new(s * (gap + (h - gap) * 0.45), 0.0);
            painter.line_segment([a, mid], Stroke::new(1.5_f32, color));
            painter.line_segment([mid, b], Stroke::new(1.0_f32, alpha(color, 0.45)));
            painter.circle_filled(a, 1.6, color);
        }
    }
    diamond(painter, center, 4.5, color, Stroke::NONE);
    diamond(
        painter,
        center,
        2.0,
        alpha(Color32::BLACK, 0.35),
        Stroke::NONE,
    );
}

/// A divider as a widget, across the available width.
pub fn ui_divider(ui: &mut egui::Ui, color: Color32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), Sense::hover());
    divider(ui.painter(), r.center(), r.width() * 0.9, color);
}

/// A gilded medallion with a glyph: quest markers, level badges, portraits.
pub fn medallion(
    painter: &egui::Painter,
    c: Pos2,
    r: f32,
    fill: Color32,
    glyph: &str,
    glyph_color: Color32,
) {
    painter.circle_filled(
        c + Vec2::new(0.0, 1.0),
        r + 1.5,
        Color32::from_black_alpha(150),
    );
    painter.circle_filled(c, r, fill);
    painter.circle_stroke(c, r - 0.5, Stroke::new(1.5_f32, GOLD));
    painter.circle_stroke(c, r + 0.8, Stroke::new(1.0_f32, OUTLINE));
    if !glyph.is_empty() {
        let size = (r * 1.25).max(9.0);
        painter.text(
            c + Vec2::new(0.0, -size * 0.04),
            Align2::CENTER_CENTER,
            glyph,
            bold_font(size),
            glyph_color,
        );
    }
}

/// A tick box for an objective; a green check when done.
pub fn check_mark(painter: &egui::Painter, r: Rect, done: bool, ink: Color32, check: Color32) {
    let b = Rect::from_center_size(r.center(), Vec2::splat(r.width().min(r.height()).min(11.0)));
    painter.rect_stroke(
        b,
        2.0,
        Stroke::new(1.0_f32, alpha(ink, 0.8)),
        StrokeKind::Inside,
    );
    if done {
        let p = |x: f32, y: f32| b.min + Vec2::new(b.width() * x, b.height() * y);
        painter.add(Shape::line(
            vec![p(0.15, 0.5), p(0.42, 0.82), p(1.05, -0.1)],
            Stroke::new(2.2_f32, check),
        ));
    }
}

/// Gold corner pieces just inside `r`: a bracket and a diamond at each corner.
pub fn corner_pieces(painter: &egui::Painter, r: Rect, len: f32, color: Color32) {
    let len = len.min(r.width() * 0.3).min(r.height() * 0.3);
    for (corner, sx, sy) in [
        (r.left_top(), 1.0, 1.0),
        (r.right_top(), -1.0, 1.0),
        (r.right_bottom(), -1.0, -1.0),
        (r.left_bottom(), 1.0, -1.0),
    ] {
        let o = corner + Vec2::new(sx * 2.5, sy * 2.5);
        painter.add(Shape::line(
            vec![
                o + Vec2::new(sx * len, 0.0),
                o + Vec2::new(sx * 3.0, 0.0),
                o + Vec2::new(0.0, sy * 3.0),
                o + Vec2::new(0.0, sy * len),
            ],
            Stroke::new(2.5_f32, color),
        ));
        // inner filigree: a short second bracket and a diamond
        let i = corner + Vec2::new(sx * 7.5, sy * 7.5);
        painter.add(Shape::line(
            vec![
                i + Vec2::new(sx * len * 0.55, 0.0),
                i,
                i + Vec2::new(0.0, sy * len * 0.55),
            ],
            Stroke::new(1.0_f32, alpha(color, 0.7)),
        ));
        diamond(
            painter,
            corner + Vec2::new(sx * 4.5, sy * 4.5),
            3.2,
            GOLD_BRIGHT,
            Stroke::new(1.0_f32, OUTLINE),
        );
        painter.circle_filled(o + Vec2::new(sx * len, 0.0), 1.8, color);
        painter.circle_filled(o + Vec2::new(0.0, sy * len), 1.8, color);
    }
}

// ---- frames ----

/// An ornate frame: carved dark wood with gold trim and corner pieces.
/// Fills `rect`; put content inside `rect.shrink(10.0)`.
pub fn panel_frame(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(
        rect.expand(2.0).translate(Vec2::new(0.0, 2.0)),
        8.0,
        Color32::from_black_alpha(110),
    );
    painter.rect_filled(rect, 6.0, WOOD_DARK);
    gradient_rect(
        painter,
        rect.shrink(3.0),
        Color32::from_rgb(50, 36, 25),
        Color32::from_rgb(27, 19, 14),
    );
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.5_f32, OUTLINE),
        StrokeKind::Outside,
    );
    painter.rect_stroke(
        rect.shrink(1.0),
        5.0,
        Stroke::new(1.5_f32, GOLD_DARK),
        StrokeKind::Inside,
    );
    painter.rect_stroke(
        rect.shrink(5.0),
        3.0,
        Stroke::new(1.0_f32, alpha(GOLD_DARK, 0.45)),
        StrokeKind::Inside,
    );
    corner_pieces(painter, rect, 13.0, GOLD);
}

/// A light, see-through backdrop for text over the world (the quest tracker).
pub fn backdrop(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(14, 10, 7, 120));
    painter.rect_stroke(
        rect,
        5.0,
        Stroke::new(1.0_f32, alpha(GOLD_DARK, 0.5)),
        StrokeKind::Inside,
    );
}

/// A flame brazier standing on `base` (the bottom centre of its bowl), `size` wide.
pub fn brazier(painter: &egui::Painter, base: Pos2, size: f32) {
    let t = painter.ctx().input(|i| i.time) as f32;
    let flick = (t * 9.0).sin() * 0.5 + (t * 23.0).sin() * 0.25;
    let s = size;
    let bowl_h = s * 0.34;
    let rim = base - Vec2::new(0.0, bowl_h);
    // glow
    let glow_c = rim - Vec2::new(0.0, s * 0.4);
    painter.circle_filled(glow_c, s * (1.25 + flick * 0.04), alpha(EMBER, 0.07));
    painter.circle_filled(glow_c, s * 0.8, alpha(EMBER, 0.10));
    painter.circle_filled(glow_c, s * 0.45, alpha(GOLD_BRIGHT, 0.12));
    // flame: outer, middle, core
    let flame = |w: f32, h: f32, sway: f32, col: Color32| {
        painter.add(Shape::convex_polygon(
            vec![
                rim + Vec2::new(-w * 0.75, 0.0),
                rim + Vec2::new(-w, -h * 0.34),
                rim + Vec2::new(-w * 0.45 + sway * 0.4, -h * 0.74),
                rim + Vec2::new(sway, -h),
                rim + Vec2::new(w * 0.55 + sway * 0.4, -h * 0.68),
                rim + Vec2::new(w, -h * 0.30),
                rim + Vec2::new(w * 0.75, 0.0),
            ],
            col,
            Stroke::NONE,
        ));
    };
    let sway = flick * s * 0.07;
    flame(
        s * 0.36,
        s * (1.05 + flick * 0.06),
        sway,
        Color32::from_rgb(232, 96, 28),
    );
    flame(s * 0.26, s * 0.78, sway * 0.7, EMBER);
    flame(
        s * 0.15,
        s * 0.5,
        sway * 0.4,
        Color32::from_rgb(255, 232, 150),
    );
    // bowl
    painter.add(Shape::convex_polygon(
        vec![
            rim + Vec2::new(-s * 0.5, 0.0),
            rim + Vec2::new(s * 0.5, 0.0),
            base + Vec2::new(s * 0.2, 0.0),
            base + Vec2::new(-s * 0.2, 0.0),
        ],
        Color32::from_rgb(64, 50, 34),
        Stroke::new(1.0_f32, OUTLINE),
    ));
    painter.add(Shape::convex_polygon(
        vec![
            rim + Vec2::new(-s * 0.36, s * 0.07),
            rim + Vec2::new(-s * 0.08, s * 0.07),
            base + Vec2::new(-s * 0.06, -s * 0.05),
            base + Vec2::new(-s * 0.17, -s * 0.05),
        ],
        Color32::from_rgb(104, 82, 50),
        Stroke::NONE,
    ));
    let rim_r = Rect::from_center_size(rim, Vec2::new(s * 1.12, s * 0.13));
    painter.rect_filled(rim_r, 2.0, GOLD);
    painter.rect_stroke(
        rim_r,
        2.0,
        Stroke::new(1.0_f32, OUTLINE),
        StrokeKind::Outside,
    );
    painter.line_segment(
        [
            rim_r.left_top() + Vec2::new(2.0, 1.5),
            rim_r.right_top() + Vec2::new(-2.0, 1.5),
        ],
        Stroke::new(1.0_f32, GOLD_BRIGHT),
    );
}

/// How far each end cap of [`action_bar_frame`] reaches outside its rect, sideways.
pub fn action_bar_cap_width(height: f32) -> f32 {
    (height * 0.62).clamp(24.0, 44.0)
}

/// The action bar backing: a carved plank with gold rails, between two stone
/// pillars that each carry a flame brazier.
///
/// `rect` is the backing behind the slots (slots go inside `rect.shrink(6.0)`).
/// The pillars sit outside it, [`action_bar_cap_width`] wide on each side, and
/// their flames rise about one cap width above `rect.top()`.
pub fn action_bar_frame(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(
        rect.expand2(Vec2::new(2.0, 2.0))
            .translate(Vec2::new(0.0, 3.0)),
        6.0,
        Color32::from_black_alpha(120),
    );
    painter.rect_filled(rect, 4.0, WOOD_DARK);
    gradient_rect(
        painter,
        rect.shrink(2.0),
        Color32::from_rgb(54, 39, 26),
        Color32::from_rgb(26, 18, 13),
    );
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.5_f32, OUTLINE),
        StrokeKind::Outside,
    );
    // gold rails along the top and bottom
    for (y, hi) in [(rect.top() + 2.0, true), (rect.bottom() - 2.0, false)] {
        painter.line_segment(
            [
                Pos2::new(rect.left() + 3.0, y),
                Pos2::new(rect.right() - 3.0, y),
            ],
            Stroke::new(2.0_f32, if hi { GOLD } else { GOLD_DARK }),
        );
    }
    painter.line_segment(
        [
            Pos2::new(rect.left() + 3.0, rect.top() + 4.0),
            Pos2::new(rect.right() - 3.0, rect.top() + 4.0),
        ],
        Stroke::new(1.0_f32, Color32::from_black_alpha(120)),
    );
    // a crest in the middle of the top rail
    let crest = Pos2::new(rect.center().x, rect.top() + 1.0);
    for s in [-1.0, 1.0] {
        painter.add(Shape::convex_polygon(
            vec![
                crest + Vec2::new(s * 5.0, -2.5),
                crest + Vec2::new(s * 26.0, 0.0),
                crest + Vec2::new(s * 5.0, 2.5),
            ],
            GOLD,
            Stroke::new(1.0_f32, OUTLINE),
        ));
    }
    diamond(
        painter,
        crest,
        6.0,
        GOLD_BRIGHT,
        Stroke::new(1.0_f32, OUTLINE),
    );
    diamond(painter, crest, 2.5, EMBER, Stroke::NONE);
    // end caps
    let cw = action_bar_cap_width(rect.height());
    for s in [-1.0_f32, 1.0] {
        let x0 = if s < 0.0 {
            rect.left() - cw + 3.0
        } else {
            rect.right() - 3.0
        };
        let pillar = Rect::from_min_max(
            Pos2::new(x0, rect.top() - 5.0),
            Pos2::new(x0 + cw, rect.bottom() + 3.0),
        );
        painter.rect_filled(
            pillar.translate(Vec2::new(0.0, 3.0)).expand(1.0),
            4.0,
            Color32::from_black_alpha(120),
        );
        painter.rect_filled(pillar, 3.0, Color32::from_rgb(36, 33, 31));
        gradient_rect_h(
            painter,
            pillar.shrink(1.5),
            Color32::from_rgb(92, 86, 80),
            Color32::from_rgb(44, 40, 38),
        );
        // carved flutes
        for k in 1..3 {
            let x = pillar.left() + pillar.width() * k as f32 / 3.0;
            painter.line_segment(
                [
                    Pos2::new(x, pillar.top() + 9.0),
                    Pos2::new(x, pillar.bottom() - 9.0),
                ],
                Stroke::new(1.0_f32, Color32::from_black_alpha(70)),
            );
            painter.line_segment(
                [
                    Pos2::new(x + 1.0, pillar.top() + 9.0),
                    Pos2::new(x + 1.0, pillar.bottom() - 9.0),
                ],
                Stroke::new(1.0_f32, Color32::from_white_alpha(18)),
            );
        }
        painter.rect_stroke(
            pillar,
            3.0,
            Stroke::new(1.5_f32, OUTLINE),
            StrokeKind::Outside,
        );
        // gold bands at the head and foot
        for band in [
            Rect::from_min_max(
                pillar.left_top() + Vec2::new(-2.0, 0.0),
                Pos2::new(pillar.right() + 2.0, pillar.top() + 6.0),
            ),
            Rect::from_min_max(
                Pos2::new(pillar.left() - 2.0, pillar.bottom() - 6.0),
                pillar.right_bottom() + Vec2::new(2.0, 0.0),
            ),
        ] {
            gradient_rect(painter, band, GOLD_BRIGHT, GOLD_DARK);
            painter.rect_stroke(
                band,
                1.0,
                Stroke::new(1.0_f32, OUTLINE),
                StrokeKind::Outside,
            );
        }
        // a lit rune on the pillar face
        diamond(
            painter,
            pillar.center() + Vec2::new(0.0, 2.0),
            4.0,
            alpha(EMBER, 0.9),
            Stroke::new(1.0_f32, GOLD_DARK),
        );
        brazier(
            painter,
            Pos2::new(pillar.center().x, pillar.top() - 1.0),
            cw,
        );
    }
}

/// The dark recess of a slot. Draw it first, then the icon, then [`action_slot`]
/// or [`slot_rim`].
pub fn slot_well(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 4.0, Color32::from_rgb(15, 11, 9));
    gradient_rect(
        painter,
        rect.shrink(3.0),
        Color32::from_rgb(20, 15, 12),
        Color32::from_rgb(38, 28, 21),
    );
}

/// The bevelled bronze rim of a slot, drawn around the icon area. `glow` makes it gold.
pub fn slot_rim(painter: &egui::Painter, rect: Rect, glow: bool) {
    let r = rect;
    painter.rect_stroke(r, 4.0, Stroke::new(1.0_f32, OUTLINE), StrokeKind::Outside);
    painter.rect_stroke(
        r,
        4.0,
        Stroke::new(2.0_f32, if glow { GOLD } else { BRONZE }),
        StrokeKind::Inside,
    );
    // bevel: light from the top left
    let (hi, lo) = if glow {
        (GOLD_BRIGHT, GOLD_DARK)
    } else {
        (
            Color32::from_rgb(156, 122, 68),
            Color32::from_rgb(52, 38, 22),
        )
    };
    let i = r.shrink(1.0);
    painter.add(Shape::line(
        vec![
            i.left_bottom() + Vec2::new(0.0, -3.0),
            i.left_top() + Vec2::new(0.0, 3.0),
            i.left_top() + Vec2::new(3.0, 0.0),
            i.right_top() + Vec2::new(-3.0, 0.0),
        ],
        Stroke::new(1.0_f32, hi),
    ));
    painter.add(Shape::line(
        vec![
            i.right_top() + Vec2::new(0.0, 3.0),
            i.right_bottom() + Vec2::new(0.0, -3.0),
            i.right_bottom() + Vec2::new(-3.0, 0.0),
            i.left_bottom() + Vec2::new(3.0, 0.0),
        ],
        Stroke::new(1.0_f32, lo),
    ));
    // inner shadow, so the icon sits in a recess
    painter.rect_stroke(
        r.shrink(2.5),
        2.0,
        Stroke::new(1.0_f32, Color32::from_black_alpha(150)),
        StrokeKind::Inside,
    );
    if glow {
        painter.rect_stroke(
            r.expand(1.5),
            5.0,
            Stroke::new(2.0_f32, alpha(GOLD_BRIGHT, 0.35)),
            StrokeKind::Outside,
        );
    }
}

/// The dark part of a radial cooldown: `remaining` (0..1) of the square is
/// covered, shrinking clockwise from 12 o'clock.
fn cooldown_sweep(painter: &egui::Painter, r: Rect, remaining: f32) {
    let f = remaining.clamp(0.0, 1.0);
    if f <= 0.0 {
        return;
    }
    let c = r.center();
    let half = r.size() * 0.5;
    // a point on the square's edge, at an angle clockwise from straight up
    let edge = |a: f32| {
        let (dx, dy) = (a.sin(), -a.cos());
        let k = 1.0 / dx.abs().max(dy.abs()).max(1e-4);
        c + Vec2::new(dx * k * half.x, dy * k * half.y)
    };
    let start = (1.0 - f) * TAU;
    let shade = Color32::from_black_alpha(165);
    let mut m = Mesh::default();
    m.colored_vertex(c, shade);
    m.colored_vertex(edge(start), shade);
    for k in 0..4 {
        let corner = TAU * (0.125 + 0.25 * k as f32);
        if corner > start {
            m.colored_vertex(edge(corner), shade);
        }
    }
    m.colored_vertex(edge(TAU), shade);
    for i in 1..m.vertices.len() as u32 - 1 {
        m.add_triangle(0, i, i + 1);
    }
    painter.add(Shape::mesh(m));
    // the bright leading edge and the fixed edge at 12 o'clock
    painter.line_segment(
        [c, edge(start)],
        Stroke::new(1.5_f32, alpha(GOLD_BRIGHT, 0.85)),
    );
    painter.line_segment(
        [c, edge(0.0)],
        Stroke::new(1.0_f32, alpha(GOLD_BRIGHT, 0.35)),
    );
}

/// One slot of the action bar, as an overlay.
///
/// Draw order: [`slot_well`]`(rect)`, then the icon inside `rect.shrink(4.0)`,
/// then this. The icon is drawn BEFORE `action_slot`: the red "unusable" tint,
/// the cooldown sweep and the hotkey label go on top of it, and the rim is
/// drawn around it.
///
/// - `hotkey`: label at the top left ("1", "Q", "S-2"); empty for none
/// - `cooldown`: 0..1 of the cooldown still remaining (0 = ready)
/// - `active`: the selected or currently casting slot, gets a gold rim and glow
/// - `usable`: false tints the icon red (no mana, out of range, nothing to use)
pub fn action_slot(
    painter: &egui::Painter,
    rect: Rect,
    hotkey: &str,
    cooldown: f32,
    active: bool,
    usable: bool,
) {
    let inner = rect.shrink(3.0);
    if !usable {
        painter.rect_filled(inner, 2.0, Color32::from_rgba_unmultiplied(170, 24, 20, 96));
    }
    cooldown_sweep(painter, inner, cooldown);
    slot_rim(painter, rect, active);
    if active {
        // gold corner ticks
        for (p, sx, sy) in [
            (rect.left_top(), 1.0, 1.0),
            (rect.right_top(), -1.0, 1.0),
            (rect.right_bottom(), -1.0, -1.0),
            (rect.left_bottom(), 1.0, -1.0),
        ] {
            diamond(
                painter,
                p + Vec2::new(sx * 1.5, sy * 1.5),
                3.0,
                GOLD_BRIGHT,
                Stroke::new(1.0_f32, OUTLINE),
            );
        }
    }
    if !hotkey.is_empty() {
        let col = if !usable {
            Color32::from_rgb(230, 150, 140)
        } else if active {
            GOLD_BRIGHT
        } else {
            Color32::from_rgb(236, 222, 190)
        };
        shadow_text(
            painter,
            rect.left_top() + Vec2::new(5.0, 2.0),
            Align2::LEFT_TOP,
            hotkey,
            bold_font((rect.height() * 0.24).clamp(9.0, 13.0)),
            col,
        );
    }
}

// ---- bars ----

/// A cast bar centred on `center`: `progress` 0..1, with the spell name on it.
pub fn cast_bar(painter: &egui::Painter, center: Pos2, width: f32, progress: f32, label: &str) {
    let r = Rect::from_center_size(center, Vec2::new(width, 20.0));
    let f = progress.clamp(0.0, 1.0);
    painter.rect_filled(
        r.expand(2.0).translate(Vec2::new(0.0, 2.0)),
        5.0,
        Color32::from_black_alpha(110),
    );
    painter.rect_filled(r, 3.0, Color32::from_rgb(18, 13, 10));
    let inner = r.shrink(3.0);
    let fill = Rect::from_min_size(inner.min, Vec2::new(inner.width() * f, inner.height()));
    gradient_rect(
        painter,
        fill,
        Color32::from_rgb(255, 216, 112),
        Color32::from_rgb(206, 124, 30),
    );
    painter.rect_filled(
        Rect::from_min_size(fill.min, Vec2::new(fill.width(), fill.height() * 0.38)),
        0.0,
        Color32::from_white_alpha(46),
    );
    if f > 0.0 && f < 1.0 {
        // the spark at the leading edge
        let x = fill.right();
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new((x - 10.0).max(inner.left()), inner.top()),
                Pos2::new(x, inner.bottom()),
            ),
            0.0,
            Color32::from_white_alpha(40),
        );
        painter.line_segment(
            [Pos2::new(x, r.top() + 1.0), Pos2::new(x, r.bottom() - 1.0)],
            Stroke::new(2.0_f32, Color32::from_rgb(255, 250, 225)),
        );
    }
    painter.rect_stroke(r, 3.0, Stroke::new(1.5_f32, OUTLINE), StrokeKind::Outside);
    painter.rect_stroke(r, 3.0, Stroke::new(1.5_f32, GOLD_DARK), StrokeKind::Inside);
    painter.rect_stroke(
        r.shrink(1.5),
        2.0,
        Stroke::new(1.0_f32, Color32::from_black_alpha(140)),
        StrokeKind::Inside,
    );
    for x in [r.left(), r.right()] {
        diamond(
            painter,
            Pos2::new(x, center.y),
            6.5,
            GOLD,
            Stroke::new(1.0_f32, OUTLINE),
        );
        diamond(painter, Pos2::new(x, center.y), 2.5, EMBER, Stroke::NONE);
    }
    if !label.is_empty() {
        shadow_text(
            painter,
            center - Vec2::new(0.0, 1.0),
            Align2::CENTER_CENTER,
            label,
            bold_font(13.0),
            Color32::WHITE,
        );
    }
}

/// A health, mana or rage bar filling `rect`, with `label` centred on it
/// (pass "" for none; the label is skipped on bars under 10 px tall).
pub fn resource_bar(
    painter: &egui::Painter,
    rect: Rect,
    value: f32,
    max: f32,
    color: Color32,
    label: &str,
) {
    let f = if max > 0.0 {
        (value / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    painter.rect_filled(rect, 2.0, mix(Color32::from_rgb(10, 8, 6), color, 0.16));
    let inner = rect.shrink(1.5);
    let fill = Rect::from_min_size(inner.min, Vec2::new(inner.width() * f, inner.height()));
    gradient_rect(
        painter,
        fill,
        mix(color, Color32::WHITE, 0.22),
        mix(color, Color32::BLACK, 0.36),
    );
    painter.rect_filled(
        Rect::from_min_size(fill.min, Vec2::new(fill.width(), fill.height() * 0.36)),
        0.0,
        Color32::from_white_alpha(38),
    );
    if f > 0.0 && f < 1.0 {
        painter.line_segment(
            [fill.right_top(), fill.right_bottom()],
            Stroke::new(1.0_f32, mix(color, Color32::WHITE, 0.55)),
        );
    }
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0_f32, OUTLINE),
        StrokeKind::Outside,
    );
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0_f32, alpha(GOLD_DARK, 0.9)),
        StrokeKind::Inside,
    );
    if !label.is_empty() && rect.height() >= 10.0 {
        shadow_text(
            painter,
            rect.center() - Vec2::new(0.0, 1.0),
            Align2::CENTER_CENTER,
            label,
            bold_font((rect.height() - 2.0).clamp(10.0, 13.0)),
            Color32::WHITE,
        );
    }
}

/// A big gilded title over the world ("Level 3!", "Day 2"), centred on `center`,
/// with a flourish under it. Fade it with the alpha of `color`
/// (`theme::GOLD_BRIGHT.gamma_multiply(fade)`). Returns the text's rect.
pub fn title_text(
    painter: &egui::Painter,
    center: Pos2,
    text: &str,
    size: f32,
    color: Color32,
) -> Rect {
    let a = color.a() as f32 / 255.0;
    let galley = painter.layout_no_wrap(text.to_owned(), title_font(size), color);
    let rect = Rect::from_center_size(center, galley.size());
    let o = (size * 0.045).max(1.0);
    let dark = Color32::from_rgba_unmultiplied(20, 10, 4, (a * 215.0) as u8);
    painter.galley_with_override_text_color(
        rect.min + Vec2::new(0.0, o * 2.5),
        galley.clone(),
        Color32::from_black_alpha((a * 120.0) as u8),
    );
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        painter.galley_with_override_text_color(
            rect.min + Vec2::new(dx * o, dy * o),
            galley.clone(),
            dark,
        );
    }
    painter.galley(rect.min, galley, color);
    divider(
        painter,
        Pos2::new(center.x, rect.bottom() + size * 0.16),
        (rect.width() * 1.15).max(size * 3.0),
        color,
    );
    rect
}

/// A smaller line under a [`title_text`] ("You feel stronger."), in the body font.
pub fn subtitle_text(painter: &egui::Painter, center: Pos2, text: &str, size: f32, color: Color32) {
    shadow_text(
        painter,
        center,
        Align2::CENTER_CENTER,
        text,
        body_font(size),
        color,
    );
}

// ---- windows ----

/// A framed window with a carved header: the title in the display font, a
/// close button, gold trim and corner pieces. `open` is set to false when the
/// close button is clicked. The content is `width` wide.
#[allow(clippy::too_many_arguments)]
pub fn window<R>(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    subtitle: &str,
    width: f32,
    anchor: (Align2, Vec2),
    open: &mut bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let frame = window_frame();
    let resp = egui::Window::new(title)
        .id(egui::Id::new(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(anchor.0, anchor.1)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(width);
            header(ui, title, subtitle, open);
            add(ui)
        })?;
    window_trim(
        &ctx.layer_painter(resp.response.layer_id),
        resp.response.rect,
    );
    resp.inner
}

fn header(ui: &mut egui::Ui, title: &str, subtitle: &str, open: &mut bool) {
    let h = if subtitle.is_empty() { 34.0 } else { 48.0 };
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    let band = r.expand2(Vec2::new(6.0, 5.0));
    gradient_rect(
        p,
        band,
        Color32::from_rgb(62, 44, 28),
        Color32::from_rgb(34, 25, 18),
    );
    let tc = Pos2::new(r.center().x, r.top() + 17.0);
    let tr = shadow_text(
        p,
        tc,
        Align2::CENTER_CENTER,
        title,
        title_font(21.0),
        GOLD_BRIGHT,
    );
    // flourishes either side of the title
    for s in [-1.0, 1.0] {
        let x0 = if s < 0.0 { tr.left() } else { tr.right() } + s * 12.0;
        let x1 = (x0 + s * 70.0).clamp(r.left() + 30.0, r.right() - 30.0);
        if (x1 - x0) * s > 12.0 {
            p.line_segment(
                [Pos2::new(x0, tc.y), Pos2::new(x1, tc.y)],
                Stroke::new(1.0_f32, alpha(GOLD, 0.7)),
            );
            diamond(p, Pos2::new(x0, tc.y), 3.0, GOLD, Stroke::NONE);
            p.circle_filled(Pos2::new(x1, tc.y), 1.5, GOLD);
        }
    }
    if !subtitle.is_empty() {
        p.text(
            Pos2::new(r.center().x, r.top() + 37.0),
            Align2::CENTER_CENTER,
            subtitle,
            body_font(14.0),
            TEXT_DIM,
        );
    }
    p.line_segment(
        [
            Pos2::new(band.left(), band.bottom()),
            Pos2::new(band.right(), band.bottom()),
        ],
        Stroke::new(1.0_f32, GOLD_DARK),
    );
    // close button
    let cr = Rect::from_center_size(
        Pos2::new(r.right() - 9.0, r.top() + 15.0),
        Vec2::splat(20.0),
    );
    let resp = ui.interact(cr, ui.id().with("close"), Sense::click());
    let p = ui.painter();
    let hot = resp.hovered();
    p.circle_filled(
        cr.center(),
        9.5,
        if hot {
            Color32::from_rgb(140, 40, 28)
        } else {
            Color32::from_rgb(84, 28, 20)
        },
    );
    p.circle_stroke(cr.center(), 9.5, Stroke::new(1.5_f32, GOLD_DARK));
    let d = 3.6;
    let s = Stroke::new(1.8_f32, if hot { GOLD_BRIGHT } else { GOLD });
    p.line_segment(
        [
            cr.center() + Vec2::new(-d, -d),
            cr.center() + Vec2::new(d, d),
        ],
        s,
    );
    p.line_segment(
        [
            cr.center() + Vec2::new(-d, d),
            cr.center() + Vec2::new(d, -d),
        ],
        s,
    );
    if resp.clicked() {
        *open = false;
    }
    ui.add_space(8.0);
}

/// A parchment page: ink-coloured text on warm paper with a darker edge.
pub fn parchment<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let bg = ui.painter().add(Shape::Noop);
    let resp = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            let v = ui.visuals_mut();
            v.widgets.noninteractive.fg_stroke.color = INK;
            v.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(156, 126, 82);
            v.weak_text_color = Some(INK_FADED);
            add(ui)
        });
    let r = resp.response.rect;
    let mut shapes = Vec::with_capacity(8);
    shapes.push(Shape::rect_filled(
        r.translate(Vec2::new(0.0, 2.0)),
        3.0,
        Color32::from_black_alpha(120),
    ));
    shapes.push(Shape::rect_filled(r, 3.0, PARCHMENT_EDGE));
    // lighter towards the middle, like an aged page
    let mut m = Mesh::default();
    let i = r.shrink(3.0);
    let c = r.shrink2(Vec2::new(
        (r.width() * 0.2).min(46.0),
        (r.height() * 0.2).min(40.0),
    ));
    let edge = Color32::from_rgb(216, 194, 148);
    let mid = Color32::from_rgb(238, 224, 190);
    for q in [
        i.left_top(),
        i.right_top(),
        i.right_bottom(),
        i.left_bottom(),
    ] {
        m.colored_vertex(q, edge);
    }
    for q in [
        c.left_top(),
        c.right_top(),
        c.right_bottom(),
        c.left_bottom(),
    ] {
        m.colored_vertex(q, mid);
    }
    for k in 0..4u32 {
        let n = (k + 1) % 4;
        m.add_triangle(k, n, 4 + n);
        m.add_triangle(k, 4 + n, 4 + k);
    }
    m.add_triangle(4, 5, 6);
    m.add_triangle(4, 6, 7);
    shapes.push(Shape::mesh(m));
    shapes.push(Shape::rect_stroke(
        r,
        3.0,
        Stroke::new(1.0_f32, Color32::from_rgb(98, 72, 40)),
        StrokeKind::Inside,
    ));
    shapes.push(Shape::rect_stroke(
        r.shrink(4.0),
        1.0,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(120, 88, 44, 70)),
        StrokeKind::Inside,
    ));
    ui.painter().set(bg, Shape::Vec(shapes));
    resp.inner
}

/// A stitched leather panel (the bags).
pub fn leather<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let bg = ui.painter().add(Shape::Noop);
    let resp = egui::Frame::new()
        .inner_margin(egui::Margin::same(12))
        .show(ui, add);
    let r = resp.response.rect;
    let mut shapes = Vec::with_capacity(24);
    shapes.push(Shape::rect_filled(
        r.translate(Vec2::new(0.0, 2.0)),
        7.0,
        Color32::from_black_alpha(120),
    ));
    shapes.push(Shape::rect_filled(r, 7.0, LEATHER_DARK));
    let mut m = Mesh::default();
    let i = r.shrink(2.5);
    m.colored_vertex(i.left_top(), Color32::from_rgb(92, 60, 37));
    m.colored_vertex(i.right_top(), Color32::from_rgb(80, 52, 32));
    m.colored_vertex(i.right_bottom(), Color32::from_rgb(56, 36, 23));
    m.colored_vertex(i.left_bottom(), Color32::from_rgb(64, 41, 26));
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    shapes.push(Shape::mesh(m));
    shapes.push(Shape::rect_stroke(
        r,
        7.0,
        Stroke::new(1.5_f32, OUTLINE),
        StrokeKind::Outside,
    ));
    shapes.push(Shape::rect_stroke(
        r,
        7.0,
        Stroke::new(1.0_f32, Color32::from_rgb(112, 76, 46)),
        StrokeKind::Inside,
    ));
    // stitching
    let s = r.shrink(5.5);
    let path = [
        s.left_top(),
        s.right_top(),
        s.right_bottom(),
        s.left_bottom(),
        s.left_top(),
    ];
    shapes.extend(Shape::dashed_line(
        &path,
        Stroke::new(1.2_f32, alpha(STITCH, 0.75)),
        5.0,
        4.0,
    ));
    // gold studs at the corners
    for q in [
        s.left_top(),
        s.right_top(),
        s.right_bottom(),
        s.left_bottom(),
    ] {
        shapes.push(Shape::circle_filled(q, 3.4, OUTLINE));
        shapes.push(Shape::circle_filled(q, 2.6, GOLD));
        shapes.push(Shape::circle_filled(
            q - Vec2::new(0.7, 0.7),
            1.0,
            GOLD_BRIGHT,
        ));
    }
    ui.painter().set(bg, Shape::Vec(shapes));
    resp.inner
}

/// A section heading in the display font, with a rule after it.
pub fn heading(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text)
                .font(title_font(14.0))
                .color(color),
        );
        let (r, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().max(8.0), 14.0),
            Sense::hover(),
        );
        let y = r.center().y + 1.0;
        ui.painter().line_segment(
            [Pos2::new(r.left() + 2.0, y), Pos2::new(r.right(), y)],
            Stroke::new(1.0_f32, alpha(color, 0.45)),
        );
        diamond(
            ui.painter(),
            Pos2::new(r.left() + 4.0, y),
            2.5,
            color,
            Stroke::NONE,
        );
    });
}

/// The frame of a carved window, for `egui::Window::frame` (use it with
/// `.title_bar(false)`, then draw [`window_trim`] over the window's rect after
/// `show`). [`window`] does all of this and adds a header.
pub fn window_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(34, 25, 18))
        .stroke(Stroke::new(2.0_f32, OUTLINE))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(14))
        .shadow(Shadow {
            offset: [0, 8],
            blur: 26,
            spread: 3,
            color: Color32::from_black_alpha(170),
        })
}

/// The gold trim, corner pieces and top crest of a window, drawn over `r`
/// (the rect of the response `egui::Window::show` returns) after its content:
/// `theme::window_trim(&ctx.layer_painter(resp.response.layer_id), resp.response.rect)`.
pub fn window_trim(p: &egui::Painter, r: Rect) {
    p.rect_stroke(
        r.shrink(3.0),
        6.0,
        Stroke::new(1.5_f32, GOLD_DARK),
        StrokeKind::Inside,
    );
    p.rect_stroke(
        r.shrink(6.0),
        4.0,
        Stroke::new(1.0_f32, alpha(GOLD_DARK, 0.4)),
        StrokeKind::Inside,
    );
    corner_pieces(p, r.shrink(1.0), 16.0, GOLD);
    // a crest at the top centre
    let crest = Pos2::new(r.center().x, r.top() + 3.5);
    for s in [-1.0, 1.0] {
        p.add(Shape::convex_polygon(
            vec![
                crest + Vec2::new(s * 6.0, -3.0),
                crest + Vec2::new(s * 40.0, 0.0),
                crest + Vec2::new(s * 6.0, 3.0),
            ],
            GOLD,
            Stroke::new(1.0_f32, OUTLINE),
        ));
    }
    diamond(p, crest, 7.0, GOLD_BRIGHT, Stroke::new(1.0_f32, OUTLINE));
    diamond(p, crest, 3.0, EMBER, Stroke::NONE);
}

/// A tab button for tabbed windows ("Character | Skills | Spellbook"). Put the
/// tabs in a `ui.horizontal` and follow them with [`tab_rule`].
pub fn tab(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), title_font(14.0), GOLD);
    let size = Vec2::new(galley.size().x + 28.0, 30.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    let hot = resp.hovered();
    // selected tabs stand taller and join the page below
    let body = if selected {
        r
    } else {
        Rect::from_min_max(r.min + Vec2::new(0.0, 4.0), r.max)
    };
    let round = CornerRadius {
        nw: 6,
        ne: 6,
        sw: 0,
        se: 0,
    };
    p.rect_filled(body, round, OUTLINE);
    let (top, bottom) = if selected {
        (Color32::from_rgb(96, 68, 38), Color32::from_rgb(52, 37, 24))
    } else if hot {
        (Color32::from_rgb(66, 47, 30), Color32::from_rgb(36, 26, 18))
    } else {
        (Color32::from_rgb(46, 33, 23), Color32::from_rgb(26, 19, 14))
    };
    gradient_rect(
        p,
        Rect::from_min_max(
            body.min + Vec2::new(2.0, 3.0),
            body.max - Vec2::new(2.0, 0.0),
        ),
        top,
        bottom,
    );
    let trim = if selected {
        GOLD
    } else if hot {
        GOLD_DARK
    } else {
        BRONZE
    };
    p.add(Shape::line(
        vec![
            body.left_bottom() + Vec2::new(1.0, 0.0),
            body.left_top() + Vec2::new(1.0, 5.0),
            body.left_top() + Vec2::new(5.0, 1.0),
            body.right_top() + Vec2::new(-5.0, 1.0),
            body.right_top() + Vec2::new(-1.0, 5.0),
            body.right_bottom() + Vec2::new(-1.0, 0.0),
        ],
        Stroke::new(1.5_f32, trim),
    ));
    if selected {
        diamond(
            p,
            Pos2::new(body.center().x, body.top() + 1.0),
            3.5,
            GOLD_BRIGHT,
            Stroke::new(1.0_f32, OUTLINE),
        );
    }
    let col = if selected {
        GOLD_BRIGHT
    } else if hot {
        GOLD
    } else {
        TEXT_DIM
    };
    p.galley_with_override_text_color(
        Pos2::new(
            body.center().x - galley.size().x * 0.5,
            body.center().y - galley.size().y * 0.5 + 1.0,
        ),
        galley,
        col,
    );
    resp
}

/// The gold rule under a row of [`tab`]s, across the available width.
pub fn tab_rule(ui: &mut egui::Ui) {
    let gap = ui.spacing().item_spacing.y;
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 3.0), Sense::hover());
    let y = r.top() - gap;
    ui.painter().line_segment(
        [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
        Stroke::new(1.5_f32, GOLD_DARK),
    );
}

/// The colour of an item quality: 0 poor (grey), 1 common (white),
/// 2 uncommon (green), 3 rare (blue), 4 epic (purple).
pub fn quality_color(q: u8) -> Color32 {
    match q {
        0 => Color32::from_rgb(157, 157, 157),
        1 => Color32::from_rgb(240, 236, 224),
        2 => Color32::from_rgb(64, 220, 60),
        3 => Color32::from_rgb(64, 140, 255),
        _ => Color32::from_rgb(176, 84, 240),
    }
}

/// An equipment or bag slot with a quality-coloured border (see
/// [`quality_color`]; poor and common keep the plain bronze rim). The well and
/// rim are drawn here; the caller draws the icon inside `rect.shrink(4.0)`
/// AFTERWARDS. `empty_hint` is a ghost label for an empty slot ("Head").
pub fn item_slot(painter: &egui::Painter, rect: Rect, quality: u8, empty_hint: Option<&str>) {
    slot_well(painter, rect);
    slot_rim(painter, rect, false);
    if quality >= 2 {
        let c = quality_color(quality);
        painter.rect_stroke(rect, 4.0, Stroke::new(2.0_f32, c), StrokeKind::Inside);
        painter.rect_stroke(
            rect.shrink(2.0),
            2.0,
            Stroke::new(1.0_f32, Color32::from_black_alpha(150)),
            StrokeKind::Inside,
        );
        painter.rect_stroke(
            rect.expand(1.0),
            5.0,
            Stroke::new(2.0_f32, alpha(c, 0.25)),
            StrokeKind::Outside,
        );
    }
    if let Some(hint) = empty_hint {
        painter.with_clip_rect(rect.shrink(3.0)).text(
            rect.center(),
            Align2::CENTER_CENTER,
            hint,
            body_font((rect.height() * 0.24).clamp(9.0, 13.0)),
            Color32::from_rgb(110, 92, 70),
        );
    }
}

/// The frame of a tooltip: a dark panel with a thin gold border.
pub fn tooltip_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(16, 11, 8, 244))
        .stroke(Stroke::new(1.0_f32, GOLD_DARK))
        .corner_radius(5.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .shadow(Shadow {
            offset: [0, 3],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(150),
        })
}

/// An item or ability tooltip panel, for panels you place yourself (in an
/// `egui::Area` at the pointer, or inline); lines wrap at 260 px. Inside
/// `response.on_hover_ui(..)` egui already draws its own popup frame, which
/// [`apply`] themes the same way, so there just add the lines without this.
pub fn tooltip<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    tooltip_frame()
        .show(ui, |ui| {
            ui.set_max_width(260.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.vertical(add_contents).inner
        })
        .inner
}

/// The first line of a tooltip: a name in its quality colour.
pub fn tooltip_title(ui: &mut egui::Ui, name: &str, color: Color32) {
    ui.label(egui::RichText::new(name).font(bold_font(16.0)).color(color));
}

/// One character stat: the name at the left, the value at the right in `color`.
pub fn stat_row(ui: &mut egui::Ui, name: &str, value: &str, color: Color32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
    let p = ui.painter();
    let left = p.text(
        Pos2::new(r.left() + 2.0, r.center().y),
        Align2::LEFT_CENTER,
        name,
        body_font(14.5),
        TEXT_DIM,
    );
    let right = p.text(
        Pos2::new(r.right() - 2.0, r.center().y),
        Align2::RIGHT_CENTER,
        value,
        bold_font(14.5),
        color,
    );
    // a dotted leader between them
    let y = r.center().y + 4.0;
    let mut x = left.right() + 6.0;
    while x < right.left() - 6.0 {
        p.circle_filled(Pos2::new(x, y), 0.7, alpha(BRONZE, 0.9));
        x += 5.0;
    }
}

/// A gallery of the HUD pieces: a usage example, and a way to look at them all
/// in one screenshot (call it from the HUD painter while working on the theme).
pub fn gallery(painter: &egui::Painter, screen: Rect) {
    let c = screen.center();
    let slot = 46.0;
    let n = 10;
    let w = n as f32 * (slot + 4.0) + 8.0;
    let bar = Rect::from_min_size(
        Pos2::new(c.x - w * 0.5, screen.bottom() - 170.0),
        Vec2::new(w, slot + 12.0),
    );
    action_bar_frame(painter, bar);
    let keys = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
    for (i, key) in keys.iter().enumerate() {
        let r = Rect::from_min_size(
            bar.min + Vec2::new(6.0 + i as f32 * (slot + 4.0), 6.0),
            Vec2::splat(slot),
        );
        slot_well(painter, r);
        if i < 7 {
            let col = [MANA, RAGE, HEALTH, ENERGY, EMBER, GOLD, TEXT_DIM][i];
            painter.rect_filled(r.shrink(4.0), 2.0, col);
            diamond(
                painter,
                r.center(),
                11.0,
                mix(col, Color32::WHITE, 0.5),
                Stroke::NONE,
            );
        }
        let cd = [0.0, 0.3, 0.0, 0.8, 0.55, 0.0, 0.07, 0.0, 0.0, 0.0][i];
        action_slot(painter, r, key, cd, i == 2, i != 5);
    }
    cast_bar(
        painter,
        Pos2::new(c.x, bar.top() - 60.0),
        260.0,
        0.62,
        "Kindle Flame",
    );
    let rb = |k: f32, v: f32, col: Color32, l: &str| {
        resource_bar(
            painter,
            Rect::from_min_size(
                Pos2::new(c.x - 320.0, 150.0 + k * 26.0),
                Vec2::new(220.0, 16.0),
            ),
            v,
            100.0,
            col,
            l,
        );
    };
    rb(0.0, 72.0, HEALTH, "72 / 100");
    rb(1.0, 40.0, MANA, "40 / 100");
    rb(2.0, 90.0, RAGE, "Rage 90");
    resource_bar(
        painter,
        Rect::from_min_size(Pos2::new(c.x - 320.0, 232.0), Vec2::new(220.0, 7.0)),
        55.0,
        100.0,
        ENERGY,
        "",
    );
    let pf = Rect::from_min_size(Pos2::new(c.x + 60.0, 140.0), Vec2::new(260.0, 120.0));
    panel_frame(painter, pf);
    painter.text(
        pf.center(),
        Align2::CENTER_CENTER,
        "panel_frame",
        title_font(16.0),
        GOLD,
    );
    let names = ["Head", "Chest", "Legs", "", ""];
    for (q, name) in names.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(c.x + 60.0 + q as f32 * 52.0, 270.0),
            Vec2::splat(46.0),
        );
        item_slot(painter, r, q as u8, (q < 3).then_some(*name));
        if q >= 3 {
            diamond(
                painter,
                r.center(),
                12.0,
                quality_color(q as u8),
                Stroke::NONE,
            );
        }
    }
    title_text(
        painter,
        Pos2::new(c.x, 330.0),
        "Level 3!",
        54.0,
        GOLD_BRIGHT,
    );
    subtitle_text(
        painter,
        Pos2::new(c.x, 388.0),
        "Day 2 dawns. The braziers held.",
        18.0,
        TEXT,
    );
}
