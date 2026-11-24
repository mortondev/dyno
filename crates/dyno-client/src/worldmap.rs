//! The world map (M): the land the far view has brought, drawn from above,
//! with the places you have discovered named, the rest dimmed, your quests
//! pinned and yourself on it. See docs/design/ui.md.
//!
//! The picture is made from the far tiles the client already holds (4 m
//! cells: `dyno_world::far`), so it costs no new data; it is redrawn only
//! when more tiles or a new discovery arrive.

use crate::farview::FarTiles;
use crate::rpg_ui::Me;
use crate::theme;
use crate::uitext::{f as tf, t};
use dyno_world::content::Objective;
use dyno_world::far::{CELL, TILE, TILE_CELLS};
use egui::{Align2, Color32, ColorImage, Pos2, Rect, Stroke, TextureHandle, Vec2};
use glam::Vec3;
use std::collections::HashSet;
use std::time::Instant;

/// A place counts as seen this far beyond its radius.
const SEEN_MARGIN: f32 = 1.25;
/// The land this near to you is always shown.
const NEAR: f32 = 170.0;

#[derive(Default)]
pub struct WorldMap {
    tex: Option<TextureHandle>,
    /// what the picture was made from: tiles, discoveries, where I stood
    made: Option<(usize, usize, [i32; 2], Instant)>,
    /// the world x, z of the picture's corner, and its size in metres
    origin: [f32; 2],
    size: [f32; 2],
}

pub struct View<'a> {
    pub far: &'a FarTiles,
    pub me: &'a Me<'a>,
    pub pos: Vec3,
    pub yaw: f32,
    pub discovered: &'a HashSet<String>,
    pub zone: &'a str,
    /// party members near enough to be known: name, position
    pub party: Vec<(String, Vec3)>,
}

/// A pin on the map: where, its glyph, its colour and what it says.
struct Pin {
    at: [f32; 2],
    glyph: &'static str,
    colour: Color32,
    text: String,
}

const QUEST: Color32 = Color32::from_rgb(255, 210, 40);

/// The cells lit on the map: those of discovered places and round `near`.
fn seen(at: [f32; 2], lit: &[([f32; 2], f32)]) -> bool {
    lit.iter()
        .any(|(c, r)| (at[0] - c[0]).hypot(at[1] - c[1]) <= *r)
}

/// Draw the tiles from above: each cell the colour of its highest block,
/// lit from the north-west by the slope; what is not `lit` is dim and grey.
/// Returns the picture, the world x, z of its corner and its size in metres.
pub fn render(
    far: &FarTiles,
    palette: &[[f32; 3]; 256],
    lit: &[([f32; 2], f32)],
) -> Option<(ColorImage, [f32; 2], [f32; 2])> {
    let (mut lo, mut hi) = ([i32::MAX; 2], [i32::MIN; 2]);
    for k in far.keys() {
        for a in 0..2 {
            lo[a] = lo[a].min(k[a]);
            hi[a] = hi[a].max(k[a]);
        }
    }
    if far.is_empty() {
        return None;
    }
    let n = TILE_CELLS as usize;
    let (w, h) = (
        (hi[0] - lo[0] + 1) as usize * n,
        (hi[1] - lo[1] + 1) as usize * n,
    );
    let height = |cx: i32, cz: i32| -> f32 {
        let tile = [cx.div_euclid(TILE_CELLS), cz.div_euclid(TILE_CELLS)];
        far.get(&tile).map_or(0.0, |t| {
            let i = cz.rem_euclid(TILE_CELLS) as usize * n + cx.rem_euclid(TILE_CELLS) as usize;
            t.heights[i] as f32
        })
    };
    let mut img = ColorImage::filled([w, h], Color32::from_rgb(24, 19, 15));
    for (pos, tile) in far {
        for cz in 0..n {
            for cx in 0..n {
                let i = cz * n + cx;
                if tile.heights[i] == 0 {
                    continue;
                }
                let (gx, gz) = (
                    pos[0] * TILE_CELLS + cx as i32,
                    pos[1] * TILE_CELLS + cz as i32,
                );
                let c = palette[tile.blocks[i] as usize];
                // the slope toward the light, and a little for height
                let here = tile.heights[i] as f32;
                let slope = (here - height(gx - 1, gz - 1)).clamp(-6.0, 6.0) / 6.0;
                let shade = 0.86 + 0.22 * slope + ((here - 60.0) / 400.0).clamp(-0.06, 0.1);
                let world = [
                    (gx * CELL) as f32 + CELL as f32 * 0.5,
                    (gz * CELL) as f32 + CELL as f32 * 0.5,
                ];
                let rgb = if seen(world, lit) {
                    [c[0] * shade, c[1] * shade, c[2] * shade]
                } else {
                    // not found yet: the shape of the land in faded ink
                    let grey = (c[0] + c[1] + c[2]) / 3.0 * shade;
                    [0.16 + grey * 0.30, 0.13 + grey * 0.26, 0.10 + grey * 0.20]
                };
                let px = (pos[0] - lo[0]) as usize * n + cx;
                let pz = (pos[1] - lo[1]) as usize * n + cz;
                img.pixels[pz * w + px] = Color32::from_rgb(
                    (rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (rgb[2].clamp(0.0, 1.0) * 255.0) as u8,
                );
            }
        }
    }
    Some((
        img,
        [(lo[0] * TILE) as f32, (lo[1] * TILE) as f32],
        [(w as i32 * CELL) as f32, (h as i32 * CELL) as f32],
    ))
}

impl WorldMap {
    fn remake(&mut self, ctx: &egui::Context, v: &View, hearth: [f32; 2]) {
        let stand = [(v.pos.x / 60.0) as i32, (v.pos.z / 60.0) as i32];
        let fresh = self.made.is_some_and(|(tiles, found, at, when)| {
            (tiles == v.far.len() && found == v.discovered.len() && at == stand)
                || when.elapsed().as_secs_f32() < 1.5
        });
        if fresh && self.tex.is_some() {
            return;
        }
        let mut lit: Vec<([f32; 2], f32)> =
            v.me.content
                .areas
                .iter()
                .filter(|a| v.discovered.contains(&a.key))
                .map(|a| {
                    (
                        [hearth[0] + a.at[0] as f32, hearth[1] + a.at[1] as f32],
                        a.radius * SEEN_MARGIN,
                    )
                })
                .collect();
        lit.push(([v.pos.x, v.pos.z], NEAR));
        let palette = crate::minimap::palette();
        if let Some((img, origin, size)) = render(v.far, &palette, &lit) {
            self.tex = Some(ctx.load_texture("world-map", img, egui::TextureOptions::NEAREST));
            self.origin = origin;
            self.size = size;
        }
        self.made = Some((v.far.len(), v.discovered.len(), stand, Instant::now()));
    }

    /// The map window. False when it was closed.
    pub fn window(&mut self, ctx: &egui::Context, v: &View) -> bool {
        let c = v.me.content;
        let h = v.me.hearth.unwrap_or_default();
        let hearth = [h[0] as f32, h[2] as f32];
        self.remake(ctx, v, hearth);
        let rel = |at: [i32; 2]| [hearth[0] + at[0] as f32, hearth[1] + at[1] as f32];
        // quests: where to hand in, where to go, who has work
        let mut pins: Vec<Pin> = Vec::new();
        let npc_at = |n: u16| c.npc(n).map(|n| (rel([n.at[0], n.at[2]]), n.name.clone()));
        for p in v.me.log {
            let Some(q) = c.quest(p.quest) else { continue };
            if v.me.finished(q) {
                if let Some((at, name)) = npc_at(q.ender) {
                    pins.push(Pin {
                        at,
                        glyph: "?",
                        colour: QUEST,
                        text: tf("map.hand_in", &[("quest", &q.title), ("npc", &name)]),
                    });
                }
                continue;
            }
            for (i, o) in q.objectives.iter().enumerate() {
                let n = p.counts.get(i).copied().unwrap_or(0);
                if n >= o.need() {
                    continue;
                }
                let text = format!("{}: {}", q.title, v.me.objective_text(o, n));
                let mut pin = |at: [f32; 2]| {
                    pins.push(Pin {
                        at,
                        glyph: "",
                        colour: QUEST,
                        text: text.clone(),
                    })
                };
                match o {
                    Objective::Talk { npc } => {
                        if let Some((at, _)) = npc_at(*npc) {
                            pin(at);
                        }
                    }
                    Objective::Explore { at, .. } => pin(rel([at[0], at[2]])),
                    Objective::Kill { creature, .. } => {
                        // the nearest few camps of that creature
                        let mut camps: Vec<[f32; 2]> = c
                            .areas
                            .iter()
                            .flat_map(|a| a.camps.iter())
                            .filter(|k| k.creature == *creature && k.y.is_none())
                            .map(|k| rel(k.at))
                            .collect();
                        camps.sort_by(|a, b| {
                            let d = |p: &[f32; 2]| (p[0] - v.pos.x).hypot(p[1] - v.pos.z);
                            d(a).total_cmp(&d(b))
                        });
                        for at in camps.into_iter().take(3) {
                            pin(at);
                        }
                    }
                    Objective::Collect { .. } => {}
                }
            }
        }
        for q in c.quests.iter().filter(|q| v.me.available(q)) {
            if let Some((at, name)) = npc_at(q.giver) {
                pins.push(Pin {
                    at,
                    glyph: "!",
                    colour: QUEST,
                    text: tf("map.offers", &[("npc", &name), ("quest", &q.title)]),
                });
            }
        }
        let mut open = true;
        let side = (ctx.content_rect().height() - 190.0).clamp(320.0, 720.0);
        let (origin, size, tex) = (self.origin, self.size, self.tex.clone());
        theme::window(
            ctx,
            "world-map",
            &t("map.title"),
            v.zone,
            side,
            (Align2::CENTER_CENTER, Vec2::new(0.0, -10.0)),
            &mut open,
            |ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
                crate::probe::mark("map.canvas", rect);
                let p = ui.painter().with_clip_rect(rect);
                p.rect_filled(rect, 3.0, Color32::from_rgb(24, 19, 15));
                // the picture, fitted whole and centred
                let span = size[0].max(size[1]).max(1.0);
                let scale = side / span;
                let land =
                    Rect::from_center_size(rect.center(), Vec2::new(size[0], size[1]) * scale);
                if let Some(tex) = &tex {
                    p.image(
                        tex.id(),
                        land,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    p.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        t("map.empty"),
                        theme::body_font(15.0),
                        theme::TEXT_DIM,
                    );
                }
                let to = |at: [f32; 2]| {
                    land.min + Vec2::new(at[0] - origin[0], at[1] - origin[1]) * scale
                };
                let mut hover: Option<(f32, String)> = None;
                let pointer = resp.hover_pos();
                let mut near = |at: Pos2, r: f32, text: &str| {
                    if let Some(m) = pointer {
                        let d = (m - at).length();
                        if d <= r && hover.as_ref().is_none_or(|(best, _)| d < *best) {
                            hover = Some((d, text.to_string()));
                        }
                    }
                };
                // places: a name where you have been (the rest is for finding)
                for a in &c.areas {
                    let at = to(rel(a.at));
                    if !rect.contains(at) {
                        continue;
                    }
                    if v.discovered.contains(&a.key) {
                        theme::shadow_text(
                            &p,
                            at,
                            Align2::CENTER_CENTER,
                            &a.name,
                            theme::title_font(if a.safe { 14.0 } else { 12.5 }),
                            if a.safe {
                                theme::GOLD_BRIGHT
                            } else {
                                theme::TEXT
                            },
                        );
                        let levels = tf(
                            "map.levels",
                            &[
                                ("from", &a.levels[0].to_string()),
                                ("to", &a.levels[1].to_string()),
                            ],
                        );
                        near(at, 26.0, &format!("{}\n{levels}", a.name));
                    }
                }
                // quest pins
                for pin in &pins {
                    let at = to(pin.at);
                    if !rect.contains(at) {
                        continue;
                    }
                    if pin.glyph.is_empty() {
                        p.circle_filled(at, 5.5, theme::OUTLINE);
                        p.circle_filled(at, 4.0, pin.colour);
                    } else {
                        theme::medallion(
                            &p,
                            at,
                            8.5,
                            Color32::from_rgb(40, 28, 18),
                            pin.glyph,
                            pin.colour,
                        );
                    }
                    near(at, 10.0, &pin.text);
                }
                // companions, then you: an arrow the way you face
                for (name, pos) in &v.party {
                    let at = to([pos.x, pos.z]);
                    p.circle_filled(at, 4.5, theme::OUTLINE);
                    p.circle_filled(at, 3.2, Color32::from_rgb(110, 170, 255));
                    near(at, 8.0, name);
                }
                let me = to([v.pos.x, v.pos.z]);
                let (s, cos) = v.yaw.sin_cos();
                let fwd = Vec2::new(-s, -cos);
                let side_v = Vec2::new(-fwd.y, fwd.x);
                let pts = vec![
                    me + fwd * 9.0,
                    me - fwd * 6.0 + side_v * 6.0,
                    me - fwd * 2.5,
                    me - fwd * 6.0 - side_v * 6.0,
                ];
                p.add(egui::Shape::convex_polygon(
                    vec![pts[0], pts[1], pts[2]],
                    theme::GOLD_BRIGHT,
                    Stroke::NONE,
                ));
                p.add(egui::Shape::convex_polygon(
                    vec![pts[0], pts[2], pts[3]],
                    theme::GOLD_BRIGHT,
                    Stroke::NONE,
                ));
                p.add(egui::Shape::closed_line(
                    pts,
                    Stroke::new(1.5_f32, theme::OUTLINE),
                ));
                p.rect_stroke(
                    rect,
                    3.0,
                    Stroke::new(1.5_f32, theme::GOLD_DARK),
                    egui::StrokeKind::Inside,
                );
                if let (Some((_, text)), Some(m)) = (&hover, pointer) {
                    egui::Area::new(egui::Id::new("map-tip"))
                        .order(egui::Order::Tooltip)
                        .fixed_pos(m + Vec2::new(14.0, 16.0))
                        .constrain(true)
                        .interactable(false)
                        .show(ui.ctx(), |ui| {
                            theme::tooltip(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(text.as_str())
                                        .font(theme::body_font(14.0))
                                        .color(theme::TEXT),
                                );
                            })
                        });
                }
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(t("map.hint"))
                        .font(theme::body_font(13.0))
                        .color(theme::TEXT_DIM),
                );
            },
        );
        open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dyno_world::far::FarTile;

    fn tile(block: u8, height: u16) -> FarTile {
        let n = (TILE_CELLS * TILE_CELLS) as usize;
        FarTile {
            heights: vec![height; n],
            blocks: vec![block; n],
            ground: vec![height; n],
            ground_blocks: vec![block; n],
        }
    }

    #[test]
    fn the_map_covers_every_tile_and_dims_what_is_not_found() {
        let mut far = FarTiles::new();
        far.insert([2, 3], tile(dyno_world::GRASS, 60));
        far.insert([4, 3], tile(dyno_world::GRASS, 60));
        let palette = crate::minimap::palette();
        // found: a circle round the middle of the first tile
        let lit = [([2.5 * TILE as f32, 3.5 * TILE as f32], 20.0)];
        let (img, origin, size) = render(&far, &palette, &lit).unwrap();
        assert_eq!(origin, [2.0 * TILE as f32, 3.0 * TILE as f32]);
        assert_eq!(size, [3.0 * TILE as f32, TILE as f32]);
        let n = TILE_CELLS as usize;
        assert_eq!(img.size, [3 * n, n]);
        let at = |x: usize, z: usize| img.pixels[z * 3 * n + x];
        let (found, unknown, gap) = (at(n / 2, n / 2), at(2 * n + n / 2, n / 2), at(n + 2, 2));
        // found land is green; unknown land is dim and nearly grey; no tile: the backdrop
        assert!(found.g() > found.r() + 30 && found.g() > 120, "{found:?}");
        assert!(
            unknown.g() < 110 && unknown.g().abs_diff(unknown.r()) < 30,
            "{unknown:?}"
        );
        assert_eq!(gap, Color32::from_rgb(24, 19, 15));
        assert!(render(&FarTiles::new(), &palette, &lit).is_none());
    }
}
