//! The round minimap in the top-right corner: a top-down view of the land
//! around you, with markers for people, quest givers, nearby foes and the
//! places your quests send you to.
//!
//! The terrain is drawn into a texture that covers a little more than the
//! visible circle and is only rebuilt when you have walked away from where it
//! was built, when the zoom changes, when a block changes nearby, or every few
//! seconds. Between rebuilds the circle slides over the texture, so the map
//! moves smoothly with you.

use crate::game::Remotes;
use crate::rpg_ui::{self, Marker};
use crate::theme;
use dyno_protocol::{Content, EntityKind};
use dyno_world::{self as w, World};
use egui::{
    Align2, Color32, ColorImage, Mesh, Pos2, Rect, Sense, Shape, Stroke, TextureHandle,
    TextureOptions, Vec2, epaint::Vertex,
};
use glam::Vec3;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::time::{Duration, Instant};

/// Blocks across the visible circle at each zoom level.
pub const ZOOMS: [f32; 3] = [64.0, 100.0, 160.0];
/// Texels along each side of the terrain texture (at every zoom).
pub const TEXELS: usize = 160;
/// Texels across the visible circle; the rest is margin to walk into.
const VISIBLE_TEXELS: f32 = 128.0;
/// Radius of the map circle at 1280x720.
const RADIUS: f32 = 85.0;
/// Width of the wooden ring around the circle.
const RING: f32 = 7.0;
/// Hostile creatures show only when this close.
const FOE_RANGE: f32 = 30.0;

/// One built terrain texture: which part of the world it shows.
struct Built {
    /// world x, z of the texture's top-left corner
    origin: [f32; 2],
    /// blocks per texel
    bpt: f32,
    /// the player position it was built around
    centre: [f32; 2],
    at: Instant,
}

pub struct Minimap {
    zoom: usize,
    tex: Option<TextureHandle>,
    built: Option<Built>,
    dirty: bool,
    /// how long the last terrain rebuild took, in milliseconds
    pub last_build_ms: f32,
}

impl Default for Minimap {
    fn default() -> Self {
        Self::new()
    }
}

/// What the minimap needs to know about the game this frame.
pub struct View<'a> {
    pub world: &'a World,
    pub content: &'a Content,
    pub me: &'a rpg_ui::Me<'a>,
    /// feet position and facing (the character's, not the camera's)
    pub pos: Vec3,
    pub yaw: f32,
    pub zone: &'a str,
    /// time of day, 0..1 with 0 at sunrise and 0.25 at noon
    pub time: f32,
    pub remotes: &'a Remotes,
    /// creatures and NPCs: kind, position, own name (rare elites)
    pub entities: Vec<(EntityKind, Vec3, Option<&'a str>)>,
    /// mouse-look is on: no hover tooltips
    pub captured: bool,
}

impl Minimap {
    pub fn new() -> Self {
        Minimap {
            zoom: 1,
            tex: None,
            built: None,
            dirty: false,
            last_build_ms: 0.0,
        }
    }

    /// Blocks across the visible circle.
    pub fn span(&self) -> f32 {
        ZOOMS[self.zoom]
    }

    /// A block changed somewhere: rebuild soon if it is on the map.
    pub fn block_changed(&mut self, pos: [i32; 3]) {
        self.region_changed(pos, pos);
    }

    /// Blocks changed in a box (corners inclusive): rebuild soon if it overlaps the map.
    pub fn region_changed(&mut self, a: [i32; 3], b: [i32; 3]) {
        if let Some(t) = &self.built {
            let size = TEXELS as f32 * t.bpt;
            let (x0, x1) = (a[0].min(b[0]) as f32, a[0].max(b[0]) as f32 + 1.0);
            let (z0, z1) = (a[2].min(b[2]) as f32, a[2].max(b[2]) as f32 + 1.0);
            if x1 > t.origin[0]
                && x0 < t.origin[0] + size
                && z1 > t.origin[1]
                && z0 < t.origin[1] + size
            {
                self.dirty = true;
            }
        }
    }

    fn needs_rebuild(&self, pos: Vec3) -> bool {
        let Some(b) = &self.built else {
            return true;
        };
        let moved = (pos.x - b.centre[0]).hypot(pos.z - b.centre[1]);
        let age = b.at.elapsed();
        (b.bpt - self.span() / VISIBLE_TEXELS).abs() > 1e-4
            || moved > 4.0
            || (self.dirty && age > Duration::from_millis(250))
            || age > Duration::from_secs(2)
    }

    fn rebuild(&mut self, ctx: &egui::Context, world: &World, pos: Vec3) {
        let started = Instant::now();
        let bpt = self.span() / VISIBLE_TEXELS;
        let half = TEXELS as f32 * bpt * 0.5;
        let origin = [pos.x.floor() + 0.5 - half, pos.z.floor() + 0.5 - half];
        let img = render(world, origin, bpt, TEXELS);
        match &mut self.tex {
            Some(t) => t.set(img, TextureOptions::LINEAR),
            None => self.tex = Some(ctx.load_texture("minimap", img, TextureOptions::LINEAR)),
        }
        self.built = Some(Built {
            origin,
            bpt,
            centre: [pos.x, pos.z],
            at: Instant::now(),
        });
        self.dirty = false;
        self.last_build_ms = started.elapsed().as_secs_f32() * 1000.0;
    }

    /// Draw the minimap at the top right of `screen`. Returns the rect it
    /// covers (zone label, ring and buttons), so the HUD can stack beneath it.
    pub fn show(&mut self, ctx: &egui::Context, painter: &egui::Painter, v: &View) -> Rect {
        let screen = ctx.content_rect();
        if self.needs_rebuild(v.pos) {
            self.rebuild(ctx, v.world, v.pos);
        }
        let r = RADIUS;
        let c = Pos2::new(
            screen.right() - 14.0 - RING - r,
            screen.top() + 36.0 + RING + r,
        );
        let px_per_block = 2.0 * r / self.span();

        // zone name on a small dark plaque above the ring
        let mut top = c.y - r - RING - 4.0;
        let mut left = c.x - r - RING;
        if !v.zone.is_empty() {
            let font = theme::title_font(15.0);
            let galley = painter.layout_no_wrap(v.zone.to_owned(), font.clone(), theme::GOLD);
            let w = (galley.size().x + 28.0).max(2.0 * r * 0.8);
            let plaque =
                Rect::from_center_size(Pos2::new(c.x, screen.top() + 18.0), Vec2::new(w, 22.0));
            painter.rect_filled(
                plaque.translate(Vec2::new(0.0, 1.5)),
                5.0,
                Color32::from_black_alpha(120),
            );
            theme::gradient_rect(
                painter,
                plaque,
                Color32::from_rgba_unmultiplied(48, 34, 22, 235),
                Color32::from_rgba_unmultiplied(22, 15, 10, 235),
            );
            painter.rect_stroke(
                plaque,
                4.0,
                Stroke::new(1.0_f32, theme::GOLD_DARK),
                egui::StrokeKind::Inside,
            );
            for s in [-1.0, 1.0] {
                theme::diamond(
                    painter,
                    Pos2::new(c.x + s * (w * 0.5 - 7.0), plaque.center().y),
                    3.0,
                    theme::GOLD_DARK,
                    Stroke::NONE,
                );
            }
            theme::shadow_text(
                painter,
                plaque.center() + Vec2::new(0.0, 0.5),
                Align2::CENTER_CENTER,
                v.zone,
                font,
                theme::GOLD,
            );
            top = plaque.top();
            left = left.min(plaque.left());
        }

        // shadow and wooden ring behind the map
        painter.circle_filled(
            c + Vec2::new(0.0, 2.0),
            r + RING + 2.0,
            Color32::from_black_alpha(110),
        );
        painter.circle_filled(c, r + RING, theme::WOOD);
        painter.circle_stroke(c, r + RING - 0.5, Stroke::new(1.5_f32, theme::OUTLINE));
        painter.circle_stroke(c, r + RING - 2.0, Stroke::new(1.0_f32, theme::GOLD_DARK));

        // the terrain: a textured disc sliding over the built texture
        if let (Some(tex), Some(b)) = (&self.tex, &self.built) {
            let size = TEXELS as f32 * b.bpt;
            let uv = |d: Vec2| {
                let wx = v.pos.x + d.x / px_per_block;
                let wz = v.pos.z + d.y / px_per_block;
                Pos2::new((wx - b.origin[0]) / size, (wz - b.origin[1]) / size)
            };
            let mut mesh = Mesh::with_texture(tex.id());
            mesh.vertices.push(Vertex {
                pos: c,
                uv: uv(Vec2::ZERO),
                color: Color32::WHITE,
            });
            let n = 72;
            let rim = Color32::from_gray(150);
            for i in 0..=n {
                let a = i as f32 / n as f32 * TAU;
                let d = Vec2::new(a.cos(), a.sin()) * r;
                // a softer second ring keeps the darkening to the edge
                mesh.vertices.push(Vertex {
                    pos: c + d * 0.82,
                    uv: uv(d * 0.82),
                    color: Color32::WHITE,
                });
                mesh.vertices.push(Vertex {
                    pos: c + d,
                    uv: uv(d),
                    color: rim,
                });
            }
            for i in 0..n as u32 {
                let (a0, b0) = (1 + 2 * i, 2 + 2 * i);
                let (a1, b1) = (a0 + 2, b0 + 2);
                mesh.indices.extend_from_slice(&[0, a0, a1]);
                mesh.indices.extend_from_slice(&[a0, b0, b1, a0, b1, a1]);
            }
            painter.add(Shape::mesh(mesh));
        }
        // inner bevel
        painter.circle_stroke(c, r + 0.5, Stroke::new(2.0_f32, theme::OUTLINE));
        painter.circle_stroke(c, r + 2.0, Stroke::new(1.6_f32, theme::GOLD));
        painter.circle_stroke(
            c,
            r + RING - 3.5,
            Stroke::new(1.0_f32, theme::GOLD.gamma_multiply(0.55)),
        );
        // studs on the ring
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU + TAU / 16.0;
            let at = c + Vec2::new(a.cos(), a.sin()) * (r + RING * 0.5 + 0.5);
            painter.circle_filled(at, 1.8, theme::GOLD_DARK);
            painter.circle_filled(at - Vec2::splat(0.5), 0.9, theme::GOLD_BRIGHT);
        }

        // north on the ring (under the markers, so an arrow pointing north shows)
        let north = c - Vec2::new(0.0, r + RING * 0.5);
        theme::medallion(
            painter,
            north,
            8.5,
            theme::WOOD_DARK,
            "N",
            theme::GOLD_BRIGHT,
        );

        // markers, far ones first so the nearest sit on top
        let mut hover: Vec<(Pos2, f32, String)> = Vec::new();
        let to_map = |p: Vec3| c + Vec2::new(p.x - v.pos.x, p.z - v.pos.z) * px_per_block;
        let inside = |q: Pos2, margin: f32| (q - c).length() <= r - margin;
        let outline = Color32::from_black_alpha(220);

        // hostile creatures close by: small red dots
        for (kind, p, own) in &v.entities {
            let EntityKind::Creature(id) = kind else {
                continue;
            };
            let Some(cr) = v.content.creature(*id) else {
                continue;
            };
            let d = (p.x - v.pos.x).hypot(p.z - v.pos.z);
            if !cr.hostile || d > FOE_RANGE {
                continue;
            }
            let q = to_map(*p);
            if !inside(q, 3.0) {
                continue;
            }
            let elite = own.is_some();
            let rad = if elite { 3.6 } else { 2.6 };
            painter.circle_filled(q, rad + 1.0, outline);
            painter.circle_filled(q, rad, Color32::from_rgb(222, 52, 40));
            if elite {
                painter.circle_stroke(q, rad + 1.6, Stroke::new(1.0_f32, theme::GOLD));
            }
            let name = match own {
                Some(n) => format!("{n} <{}>", cr.name),
                None => cr.name.clone(),
            };
            hover.push((q, rad + 2.0, name));
        }
        // other players: blue dots
        for rm in v.remotes.values().filter(|r| !r.far) {
            let q = to_map(Vec3::from(rm.current().pos));
            if !inside(q, 3.0) {
                continue;
            }
            painter.circle_filled(q, 4.0, outline);
            painter.circle_filled(q, 3.0, Color32::from_rgb(70, 150, 255));
            painter.circle_filled(q - Vec2::splat(0.8), 1.0, Color32::from_rgb(190, 225, 255));
            hover.push((q, 5.0, rm.name.clone()));
        }
        // NPCs: yellow dots, "!" and "?" for quests; quest enders also at the edge
        let mut glyphs: Vec<(Pos2, &str, Color32, String)> = Vec::new();
        for (kind, p, _) in &v.entities {
            let EntityKind::Npc(id) = kind else {
                continue;
            };
            let Some(npc) = v.content.npc(*id) else {
                continue;
            };
            let q = to_map(*p);
            let marker = v.me.marker(*id);
            let name = if npc.title.is_empty() {
                npc.name.clone()
            } else {
                format!("{} <{}>", npc.name, npc.title)
            };
            match marker {
                Some(m @ (Marker::Available | Marker::Ready)) => {
                    let glyph = if m == Marker::Available { "!" } else { "?" };
                    if inside(q, 6.0) {
                        glyphs.push((q, glyph, QUEST_YELLOW, name));
                    } else if m == Marker::Ready {
                        // where to hand in: pinned to the edge of the map
                        let dir = (q - c).normalized();
                        glyphs.push((c + dir * (r - 7.0), glyph, QUEST_YELLOW, name));
                    }
                }
                Some(Marker::Waiting) if inside(q, 6.0) => {
                    glyphs.push((q, "?", Color32::from_gray(175), name));
                }
                _ if inside(q, 3.0) => {
                    painter.circle_filled(q, 3.4, outline);
                    painter.circle_filled(q, 2.5, Color32::from_rgb(250, 214, 60));
                    hover.push((q, 4.5, name));
                }
                _ => {}
            }
        }
        // places to find: a gold ring, or an arrow at the edge pointing there
        if let Some(h) = v.me.hearth {
            for qp in v.me.log {
                let Some(q) = v.content.quest(qp.quest) else {
                    continue;
                };
                for (i, o) in q.objectives.iter().enumerate() {
                    let w::content::Objective::Explore { name, at, radius } = o else {
                        continue;
                    };
                    if qp.counts.get(i).copied().unwrap_or(0) > 0 {
                        continue;
                    }
                    let target = Vec3::new(
                        (h[0] + at[0]) as f32 + 0.5,
                        0.0,
                        (h[2] + at[2]) as f32 + 0.5,
                    );
                    let q = to_map(target);
                    let label = format!("{name}  ({})", q_title(v.content, qp.quest));
                    // the ring shrinks to stay inside the map near the edge
                    let room = r - 3.0 - (q - c).length();
                    let rr = (radius * px_per_block).min(r * 0.6).min(room).max(5.0);
                    if room >= 5.0 {
                        painter.circle_stroke(q, rr + 1.0, Stroke::new(3.5_f32, outline));
                        painter.circle_stroke(q, rr, Stroke::new(2.0_f32, theme::GOLD_BRIGHT));
                        painter.circle_filled(q, 2.0, theme::GOLD_BRIGHT);
                        hover.push((q, rr.max(6.0), label));
                    } else {
                        let dir = (q - c).normalized();
                        let tip = c + dir * (r - 2.0);
                        let side = Vec2::new(-dir.y, dir.x);
                        let pts = vec![
                            tip,
                            tip - dir * 10.0 + side * 6.0,
                            tip - dir * 7.0,
                            tip - dir * 10.0 - side * 6.0,
                        ];
                        painter.add(Shape::convex_polygon(
                            pts.clone(),
                            theme::GOLD_BRIGHT,
                            Stroke::new(1.2_f32, theme::OUTLINE),
                        ));
                        hover.push((tip - dir * 6.0, 8.0, label));
                    }
                }
            }
        }
        for (q, glyph, colour, name) in glyphs {
            let font = theme::bold_font(17.0);
            for o in [
                Vec2::new(-1.2, 0.0),
                Vec2::new(1.2, 0.0),
                Vec2::new(0.0, -1.2),
                Vec2::new(0.0, 1.2),
                Vec2::new(1.0, 1.6),
            ] {
                painter.text(q + o, Align2::CENTER_CENTER, glyph, font.clone(), outline);
            }
            painter.text(q, Align2::CENTER_CENTER, glyph, font, colour);
            hover.push((q, 7.0, name));
        }

        // you: an arrow pointing where your character faces (north is up, -z)
        {
            let fwd = Vec2::new(-v.yaw.sin(), -v.yaw.cos());
            let side = Vec2::new(-fwd.y, fwd.x);
            let pts = vec![
                c + fwd * 8.5,
                c - fwd * 5.5 + side * 6.0,
                c - fwd * 2.5,
                c - fwd * 5.5 - side * 6.0,
            ];
            let shadow: Vec<Pos2> = pts.iter().map(|p| *p + Vec2::new(0.8, 1.2)).collect();
            painter.add(Shape::convex_polygon(
                shadow,
                Color32::from_black_alpha(150),
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                pts.clone(),
                theme::GOLD_BRIGHT,
                Stroke::new(1.3_f32, theme::OUTLINE),
            ));
            // a darker right half gives the arrow some relief
            painter.add(Shape::convex_polygon(
                vec![pts[0], pts[1], pts[2]],
                Color32::from_rgb(214, 160, 60),
                Stroke::NONE,
            ));
        }

        // time of day: a sun or moon and the hour, at the lower left of the ring
        let hours = ((v.time + 0.25) * 24.0).rem_euclid(24.0);
        let clock = format!("{:02}:{:02}", hours as u32, (hours.fract() * 60.0) as u32);
        let day = (6.0..18.0).contains(&hours);
        let ca = std::f32::consts::PI * 0.75;
        let cpos = c + Vec2::new(ca.cos(), ca.sin()) * (r + RING * 0.5);
        painter.circle_filled(
            cpos + Vec2::new(0.0, 1.0),
            12.5,
            Color32::from_black_alpha(140),
        );
        painter.circle_filled(cpos, 11.5, theme::WOOD_DARK);
        painter.circle_stroke(cpos, 11.0, Stroke::new(1.5_f32, theme::GOLD));
        if day {
            let sun = Color32::from_rgb(255, 204, 70);
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                let d = Vec2::new(a.cos(), a.sin());
                painter.line_segment([cpos + d * 5.5, cpos + d * 8.0], Stroke::new(1.4_f32, sun));
            }
            painter.circle_filled(cpos, 4.2, sun);
        } else {
            painter.circle_filled(cpos, 6.0, Color32::from_rgb(222, 228, 246));
            painter.circle_filled(cpos + Vec2::new(2.8, -2.0), 5.2, theme::WOOD_DARK);
        }
        // the hour on a small dark pill, readable over a bright sky
        let font = theme::body_font(12.5);
        let size = painter
            .layout_no_wrap(clock.clone(), font.clone(), theme::TEXT)
            .size();
        let ctext = Rect::from_center_size(
            cpos + Vec2::new(-6.0, 14.0 + size.y * 0.5 + 2.0),
            size + Vec2::new(12.0, 3.0),
        );
        painter.rect_filled(ctext, 7.0, Color32::from_rgba_unmultiplied(16, 11, 8, 200));
        painter.rect_stroke(
            ctext,
            7.0,
            Stroke::new(1.0_f32, theme::GOLD_DARK),
            egui::StrokeKind::Inside,
        );
        painter.text(
            ctext.center(),
            Align2::CENTER_CENTER,
            &clock,
            font,
            theme::TEXT,
        );
        hover.push((
            cpos,
            11.0,
            format!("{clock}, {}", if day { "day" } else { "night" }),
        ));

        // zoom buttons at the lower right of the ring
        let btn = |a: f32| c + Vec2::new(a.cos(), a.sin()) * (r + RING * 0.5);
        let plus_at = btn(FRAC_PI_2 * 0.38);
        let minus_at = btn(FRAC_PI_2 * 0.72);
        let mut bottom = (c.y + r + RING).max(ctext.bottom());
        bottom = bottom.max(minus_at.y + 10.0);
        let foot = Rect::from_min_max(
            Pos2::new(left, top),
            Pos2::new(c.x + r + RING + 2.0, bottom + 2.0),
        );

        // interaction: an invisible area so clicks here stay off the world
        let mut clicked = (false, false);
        let mut hovered = (false, false);
        let mut hover_pos = None;
        egui::Area::new(egui::Id::new("minimap_area"))
            .fixed_pos(foot.min)
            // not Background: egui counts only other orders as "over the UI"
            .order(egui::Order::Middle)
            .interactable(true)
            .show(ctx, |ui| {
                // the whole footprint first: later widgets win the hover
                ui.allocate_rect(foot, Sense::hover());
                let disc = Rect::from_center_size(c, Vec2::splat(2.0 * (r + RING)));
                let resp = ui.interact(disc, egui::Id::new("minimap_disc"), Sense::hover());
                let plus = ui.interact(
                    Rect::from_center_size(plus_at, Vec2::splat(20.0)),
                    egui::Id::new("minimap_plus"),
                    Sense::click(),
                );
                let minus = ui.interact(
                    Rect::from_center_size(minus_at, Vec2::splat(20.0)),
                    egui::Id::new("minimap_minus"),
                    Sense::click(),
                );
                clicked = (plus.clicked(), minus.clicked());
                hovered = (plus.hovered(), minus.hovered());
                if plus.hovered() {
                    hover_pos = Some((plus_at, "Zoom in".to_string()));
                } else if minus.hovered() {
                    hover_pos = Some((minus_at, "Zoom out".to_string()));
                } else if !v.captured
                    && let Some(m) = resp.hover_pos()
                {
                    hover_pos = hover
                        .iter()
                        .map(|(q, rad, name)| ((*q - m).length() - rad, q, name))
                        .filter(|(d, _, _)| *d <= 0.0)
                        .min_by(|a, b| a.0.total_cmp(&b.0))
                        .map(|(_, q, name)| (*q, name.clone()));
                }
                if let (Some((_, text)), Some(m)) = (
                    &hover_pos,
                    resp.hover_pos().or(plus.hover_pos()).or(minus.hover_pos()),
                ) {
                    // our own tooltip: at once, and to the left (the map is at the screen edge)
                    egui::Area::new(egui::Id::new("minimap_tip"))
                        .order(egui::Order::Tooltip)
                        .pivot(Align2::RIGHT_TOP)
                        .fixed_pos(m + Vec2::new(-10.0, 14.0))
                        .interactable(false)
                        .show(ui.ctx(), |ui| {
                            theme::tooltip(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(text.as_str())
                                        .font(theme::bold_font(14.0))
                                        .color(theme::GOLD_BRIGHT),
                                );
                            });
                        });
                }
            });
        if clicked.0 && self.zoom > 0 {
            self.zoom -= 1;
        }
        if clicked.1 && self.zoom + 1 < ZOOMS.len() {
            self.zoom += 1;
        }
        for (at, glyph, lit, enabled) in [
            (plus_at, "+", hovered.0, self.zoom > 0),
            (minus_at, "\u{2212}", hovered.1, self.zoom + 1 < ZOOMS.len()),
        ] {
            let fill = if lit {
                theme::WOOD_LIGHT
            } else {
                theme::WOOD_DARK
            };
            let ink = if !enabled {
                theme::TEXT_DIM
            } else if lit {
                theme::GOLD_BRIGHT
            } else {
                theme::GOLD
            };
            theme::medallion(painter, at, 8.5, fill, "", ink);
            painter.text(
                at + Vec2::new(0.0, -0.5),
                Align2::CENTER_CENTER,
                glyph,
                theme::bold_font(15.0),
                ink,
            );
        }
        foot
    }
}

const QUEST_YELLOW: Color32 = Color32::from_rgb(255, 210, 40);

fn q_title(content: &Content, quest: u16) -> String {
    content
        .quest(quest)
        .map(|q| q.title.clone())
        .unwrap_or_default()
}

/// Map colour of each block, as seen from above.
fn palette() -> [[f32; 3]; 256] {
    let mut out = [[0.0; 3]; 256];
    for (i, d) in w::BLOCKS.iter().enumerate().take(256) {
        out[i] = d.color;
    }
    // a few tweaks so the map reads well at a glance
    out[w::GRASS as usize] = [0.40, 0.62, 0.27];
    out[w::LEAVES as usize] = [0.20, 0.42, 0.17];
    out[w::PINE_LEAVES as usize] = [0.16, 0.34, 0.20];
    out[w::BIRCH_LEAVES as usize] = [0.38, 0.56, 0.24];
    out[w::COBBLE as usize] = [0.56, 0.54, 0.50];
    out[w::FLAGSTONES as usize] = [0.64, 0.60, 0.53];
    out
}

/// Draw the land under a square of the world, top-down: each texel is the
/// colour of the highest block in its column, lit from the north-west by the
/// slope around it, a little brighter on high ground, and water darkens with
/// depth. `origin` is the world x, z of the top-left corner, `bpt` the blocks
/// per texel. Columns of a streamed world that have not arrived are dark.
pub fn render(world: &World, origin: [f32; 2], bpt: f32, n: usize) -> ColorImage {
    let pal = palette();
    let height = world.size[1] as i32;
    // pass 1: per texel the top block, its height and the water depth
    let mut tops = vec![(0u8, 0i16, 0u8); n * n];
    let cols: Vec<i32> = (0..n)
        .map(|i| (origin[0] + (i as f32 + 0.5) * bpt).floor() as i32)
        .collect();
    let rows: Vec<i32> = (0..n)
        .map(|i| (origin[1] + (i as f32 + 0.5) * bpt).floor() as i32)
        .collect();
    let inside_x = |x: i32| x >= 0 && x < world.size[0] as i32;
    let inside_z = |z: i32| z >= 0 && z < world.size[2] as i32;
    let scan = |x: i32, z: i32| {
        // outside the world, and land that has not streamed in, is dark
        if !inside_x(x) || !inside_z(z) || !world.known(x, height - 1, z) {
            return (0u8, -1i16, 0u8);
        }
        let mut y = height - 1;
        while y >= 0 {
            let b = world.get(x, y, z);
            match b {
                w::AIR | w::TORCH | w::GLASS => y -= 1,
                w::WATER => {
                    let surface = y;
                    while y > 0 && world.get(x, y, z) == w::WATER {
                        y -= 1;
                    }
                    let floor = world.get(x, y, z);
                    return (floor, surface as i16, (surface - y).min(255) as u8);
                }
                _ => return (b, y as i16, 0),
            }
        }
        (0, -1, 0)
    };
    // a few threads share the rows: the scan is most of the cost
    let threads = std::thread::available_parallelism()
        .map(|t| t.get())
        .unwrap_or(1)
        .clamp(1, 4);
    let chunk = n.div_ceil(threads) * n;
    std::thread::scope(|sc| {
        for (c, part) in tops.chunks_mut(chunk).enumerate() {
            let (cols, rows, scan) = (&cols, &rows, &scan);
            sc.spawn(move || {
                for (k, t) in part.iter_mut().enumerate() {
                    let k = c * chunk + k;
                    *t = scan(cols[k % n], rows[k / n]);
                }
            });
        }
    });
    // pass 2: colour, hillshade and depth
    let h_at = |i: usize, j: usize| tops[j * n + i].1 as f32;
    let mut px = vec![Color32::from_rgb(12, 9, 7); n * n];
    let water_shallow = [0.30, 0.58, 0.78];
    let water_deep = [0.07, 0.20, 0.42];
    for j in 0..n {
        for i in 0..n {
            let (b, h, depth) = tops[j * n + i];
            if h < 0 {
                continue;
            }
            let mut c = pal[b as usize];
            if depth > 0 {
                let d = depth as f32;
                let k = ((d - 1.0) / 7.0).clamp(0.0, 1.0);
                let water = lerp3(water_shallow, water_deep, k.sqrt());
                // the bed shows through the shallows
                let see = (0.45 - d * 0.12).max(0.0);
                c = lerp3(water, c, see);
            } else {
                let (il, ir) = (i.saturating_sub(1), (i + 1).min(n - 1));
                let (ju, jd) = (j.saturating_sub(1), (j + 1).min(n - 1));
                let dx = h_at(ir, j) - h_at(il, j);
                let dz = h_at(i, jd) - h_at(i, ju);
                let run = ((ir - il) as f32).max(1.0) * bpt;
                let slope = ((dx + dz) / run).clamp(-2.5, 2.5);
                let shade = 1.0 + 0.22 * slope;
                let lift = 0.86 + 0.28 * ((h as f32 - 28.0) / 50.0).clamp(0.0, 1.0);
                let k = (shade * lift).clamp(0.45, 1.45);
                c = [c[0] * k, c[1] * k, c[2] * k];
            }
            px[j * n + i] = Color32::from_rgb(
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
            );
        }
    }
    ColorImage::new([n, n], px)
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_is_blue_and_land_is_not() {
        let mut world = World::empty([32, 16, 32]);
        for z in 0..32 {
            for x in 0..32 {
                for y in 0..4 {
                    world.set(x, y, z, w::STONE);
                }
                if x < 16 {
                    world.set(x, 4, z, w::GRASS);
                } else {
                    world.set(x, 4, z, w::WATER);
                    world.set(x, 5, z, w::WATER);
                }
            }
        }
        let img = render(&world, [0.0, 0.0], 1.0, 32);
        let land = img.pixels[16 * 32 + 4];
        let sea = img.pixels[16 * 32 + 24];
        assert!(land.g() > land.b(), "grass should be green: {land:?}");
        assert!(sea.b() > sea.r() && sea.b() > sea.g(), "water: {sea:?}");
    }

    #[test]
    fn land_not_streamed_in_is_dark() {
        let world = World::test_terrain([128, 64, 128], 3);
        let mut streamed = World::sparse(world.size);
        // only the western half has arrived
        for s in world.held_list().into_iter().filter(|s| s[0] < 4) {
            streamed.put_section(s, &world.encode_section(s)).unwrap();
        }
        let whole = render(&world, [0.0, 0.0], 1.0, 128);
        let img = render(&streamed, [0.0, 0.0], 1.0, 128);
        let dark = Color32::from_rgb(12, 9, 7);
        assert_eq!(img.pixels[64 * 128 + 100], dark, "the east is unknown");
        assert_ne!(whole.pixels[64 * 128 + 100], dark);
        assert_eq!(
            img.pixels[64 * 128 + 20],
            whole.pixels[64 * 128 + 20],
            "the west is drawn"
        );
    }

    #[test]
    fn outside_the_world_is_dark() {
        let world = World::empty([8, 8, 8]);
        let img = render(&world, [-100.0, -100.0], 1.0, 4);
        assert!(img.pixels.iter().all(|p| p.r() < 30));
    }

    #[test]
    fn rebuild_is_fast() {
        let world = World::test_terrain([384, 96, 384], 7);
        let mut worst: f32 = 0.0;
        for (k, span) in ZOOMS.iter().enumerate() {
            let bpt = span / VISIBLE_TEXELS;
            let mut times: Vec<f32> = (0..9)
                .map(|_| {
                    let t = Instant::now();
                    let img = render(&world, [120.0 + k as f32 * 9.0, 130.0], bpt, TEXELS);
                    assert_eq!(img.size, [TEXELS, TEXELS]);
                    t.elapsed().as_secs_f32() * 1000.0
                })
                .collect();
            times.sort_by(f32::total_cmp);
            println!(
                "minimap rebuild at {span} blocks: best {:.3} ms, median {:.3} ms",
                times[0], times[4]
            );
            worst = worst.max(times[4]);
        }
        // generous bound for debug builds; release is far under 2 ms
        assert!(worst < 200.0, "rebuild took {worst} ms");
    }

    #[test]
    fn changes_on_the_map_mark_it_dirty() {
        let world = World::test_terrain([128, 64, 128], 3);
        let ctx = crate::theme::new_context();
        let mut m = Minimap::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            m.rebuild(ctx, &world, Vec3::new(64.0, 40.0, 64.0));
        });
        assert!(!m.dirty);
        m.block_changed([500, 10, 500]);
        assert!(!m.dirty, "far away");
        m.region_changed([0, 10, 0], [200, 12, 10]);
        assert!(m.dirty, "a fill across the map");
        assert!(m.needs_rebuild(Vec3::new(70.0, 40.0, 64.0)), "walked away");
    }
}
