//! The far view: the coarse heightmap the server streams beyond the near
//! sections (`dyno_world::far`), drawn as blocky columns, so the land's
//! silhouette and its landmarks show from 400–800 m.
//!
//! Two layers, so the seam with the near view does not show:
//! - **the ground** (each cell's lowest top of the land) is drawn from 64 m
//!   out, half a metre low: under the near view it is hidden by the near
//!   land (never above it), and where the near view ends, however raggedly,
//!   it carries on with no gap;
//! - **what stands on it** (the highest block in the cell: trees, roofs, a
//!   tower, a ridge) rises only beyond the near view, faded in over
//!   `FADE` metres with a fine dither (discarded pixels, so it needs no
//!   sorting), so there is no hard line.
//!
//! Colours are the mean of each block's own texture (the top for tops, the
//! side for sides), and the shader tones, lights and fogs them as it does the
//! near land. Cells beyond `COARSE` metres are drawn 8 m wide (2 × 2 cells
//! as one: the lowest ground, the highest top), a quarter of the vertices.

use crate::render::{Vertex, srgb};
use dyno_world::BlockId;
use dyno_world::far::{CELL, FarTile, TILE, TILE_CELLS, TilePos};
use std::cell::RefCell;
use std::collections::HashMap;

/// The tiles held, by tile position.
pub type FarTiles = HashMap<TilePos, FarTile>;

/// The ground layer starts this far out (inside the near view, hidden by it).
pub const GROUND_FROM: f32 = 64.0;
/// What stands on the ground fades in over this many metres past the cut.
pub const FADE: f32 = 32.0;
/// Beyond this, cells are drawn 8 m wide.
pub const COARSE: f32 = 400.0;

/// Shader code (render.rs): the ground tone path, lit like the land.
const K_GROUND: f32 = -2.0;
/// Shader code: the same, dithered in from tex.x over tex.y metres.
const K_FADE_IN: f32 = -5.0;

/// A cell, or a 2 × 2 block of cells: (ground, its block, top, its block).
type Cell = (f32, BlockId, f32, BlockId);

/// The cell (or block of `step` × `step` cells) at world cell coordinates,
/// if its tile is held.
fn cell_at(tiles: &FarTiles, cx: i32, cz: i32, step: i32) -> Option<Cell> {
    let (x0, z0) = (cx.div_euclid(step) * step, cz.div_euclid(step) * step);
    let mut out: Option<Cell> = None;
    for dz in 0..step {
        for dx in 0..step {
            let (x, z) = (x0 + dx, z0 + dz);
            let t = [x.div_euclid(TILE_CELLS), z.div_euclid(TILE_CELLS)];
            let tile = tiles.get(&t)?;
            let (lx, lz) = (x.rem_euclid(TILE_CELLS), z.rem_euclid(TILE_CELLS));
            let (h, hb) = tile.cell(lx, lz);
            let (g, gb) = tile.ground(lx, lz);
            let (g, h) = (g as f32, h as f32);
            out = Some(match out {
                None => (g, gb, h, hb),
                Some(c) => {
                    let (g, gb) = if g < c.0 { (g, gb) } else { (c.0, c.1) };
                    let (h, hb) = if h > c.2 { (h, hb) } else { (c.2, c.3) };
                    (g, gb, h, hb)
                }
            });
        }
    }
    out
}

thread_local! {
    /// mean colours of block faces, for the texture set they were made from
    static MEANS: RefCell<(u64, HashMap<(BlockId, usize), [f32; 3]>)> = RefCell::new((u64::MAX, HashMap::new()));
}

/// The mean colour (linear) of a block's face texture (0 top, 1 side), or
/// its plain colour when it has no texture.
fn face_colour(b: BlockId, face: usize) -> [f32; 3] {
    let generation = crate::textures::generation();
    MEANS.with(|m| {
        let mut m = m.borrow_mut();
        if m.0 != generation {
            *m = (generation, HashMap::new());
        }
        *m.1.entry((b, face)).or_insert_with(|| {
            let t = crate::textures::current();
            let layer = t.face(b, face) as usize;
            let lin = |v: u8| (v as f32 / 255.0).powf(2.2);
            let mut sum = [0.0f32; 3];
            let mut n = 0.0;
            if let Some(px) = t.layers.get(layer) {
                for p in px.chunks_exact(4).filter(|p| p[3] > 128) {
                    sum[0] += lin(p[0]);
                    sum[1] += lin(p[1]);
                    sum[2] += lin(p[2]);
                    n += 1.0;
                }
            }
            // the near land shows its texture (times the ground's tone):
            // its mean, or the block's plain colour with no texture yet
            if n > 0.0 && t.layers.len() > 1 {
                [sum[0] / n, sum[1] / n, sum[2] / n]
            } else {
                srgb(crate::render::block_color(b).0)
            }
        })
    })
}

fn quad(out: &mut Vec<Vertex>, p: [[f32; 3]; 4], n: [f32; 3], rgb: [f32; 3], tex: [f32; 4]) {
    for i in [0, 1, 2, 0, 2, 3] {
        out.push(Vertex {
            pos: p[i],
            normal: n,
            color: [rgb[0], rgb[1], rgb[2], 1.0],
            light: 0.0,
            sky: 1.0,
            tex,
        });
    }
}

/// A box's top and the sides of it that show above each neighbour's height.
#[allow(clippy::too_many_arguments)]
fn column(
    out: &mut Vec<Vertex>,
    (x0, z0, w): (f32, f32, f32),
    bottom: f32,
    top: f32,
    neighbours: [Option<f32>; 4],
    colours: ([f32; 3], [f32; 3]),
    tex: [f32; 4],
) {
    let (x1, z1) = (x0 + w, z0 + w);
    let y = top;
    quad(
        out,
        [[x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0]],
        [0.0, 1.0, 0.0],
        colours.0,
        tex,
    );
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for (k, (dx, dz)) in dirs.into_iter().enumerate() {
        // an unknown neighbour (not here yet): down to the bottom, so no gap shows
        let nh = neighbours[k].unwrap_or(bottom).max(bottom);
        if nh >= y {
            continue;
        }
        let (p, n) = match (dx, dz) {
            (1, _) => (
                [[x1, nh, z0], [x1, y, z0], [x1, y, z1], [x1, nh, z1]],
                [1.0, 0.0, 0.0],
            ),
            (-1, _) => (
                [[x0, nh, z1], [x0, y, z1], [x0, y, z0], [x0, nh, z0]],
                [-1.0, 0.0, 0.0],
            ),
            (_, 1) => (
                [[x1, nh, z1], [x1, y, z1], [x0, y, z1], [x0, nh, z1]],
                [0.0, 0.0, 1.0],
            ),
            _ => (
                [[x0, nh, z0], [x0, y, z0], [x1, y, z0], [x1, nh, z0]],
                [0.0, 0.0, -1.0],
            ),
        };
        quad(out, p, n, colours.1, tex);
    }
}

/// The mesh of one tile, as seen from `centre` (x, z): the ground beyond
/// `GROUND_FROM`, and what stands on it beyond `cut` (faded in over `FADE`).
pub fn mesh_tile(tiles: &FarTiles, at: TilePos, centre: [f32; 2], cut: f32) -> Vec<Vertex> {
    let mut out = Vec::new();
    if !tiles.contains_key(&at) {
        return out;
    }
    let tile = TILE as f32;
    let tc = [
        at[0] as f32 * tile + tile * 0.5,
        at[1] as f32 * tile + tile * 0.5,
    ];
    let step = if (tc[0] - centre[0]).hypot(tc[1] - centre[1]) > COARSE {
        2
    } else {
        1
    };
    let w = (CELL * step) as f32;
    let fade = [cut, FADE, K_FADE_IN, 0.0];
    let ground = [0.0, 0.0, K_GROUND, 0.0];
    for lz in (0..TILE_CELLS).step_by(step as usize) {
        for lx in (0..TILE_CELLS).step_by(step as usize) {
            let (gx, gz) = (at[0] * TILE_CELLS + lx, at[1] * TILE_CELLS + lz);
            let Some((g, gb, h, hb)) = cell_at(tiles, gx, gz, step) else {
                continue;
            };
            let (x0, z0) = (gx as f32 * CELL as f32, gz as f32 * CELL as f32);
            let dist = (x0 + w * 0.5 - centre[0]).hypot(z0 + w * 0.5 - centre[1]);
            let near = [(step, 0), (-step, 0), (0, step), (0, -step)]
                .map(|(dx, dz)| cell_at(tiles, gx + dx, gz + dz, step));
            // the ground: hidden under the near land, carrying on past it
            if dist >= GROUND_FROM && g > 0.0 {
                let y = g - 0.5;
                column(
                    &mut out,
                    (x0, z0, w),
                    y - 8.0,
                    y,
                    near.map(|c| c.map(|c| c.0 - 0.5)),
                    (face_colour(gb, 0), face_colour(gb, 1)),
                    ground,
                );
            }
            // what stands on it: beyond the near view only, faded in
            if dist >= cut && h > g + 0.5 {
                column(
                    &mut out,
                    (x0, z0, w),
                    g - 0.5,
                    h,
                    near.map(|c| c.map(|c| c.2)),
                    (face_colour(hb, 0), face_colour(hb, 1)),
                    fade,
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_tiles_mesh_in_two_layers_round_the_cut() {
        let world = dyno_world::World::generate([128, 64, 128], 7);
        let mut tiles = FarTiles::new();
        for tz in 0..2 {
            for tx in 0..2 {
                tiles.insert([tx, tz], FarTile::build(&world, [tx, tz]));
            }
        }
        let far_away = [-1000.0, -1000.0];
        let all = mesh_tile(&tiles, [0, 0], [-200.0, -200.0], 0.0);
        // every cell has its ground top
        assert!(all.len() >= 256 * 6, "{}", all.len());
        // near the camera: only the ground (under the near land), nothing standing
        let cut = mesh_tile(&tiles, [0, 0], [32.0, 32.0], 200.0);
        assert!(cut.iter().all(|v| v.tex[2] == K_GROUND));
        assert!(
            cut.iter()
                .all(|v| (v.pos[0] - 32.0).hypot(v.pos[2] - 32.0) >= GROUND_FROM - 6.0)
        );
        // far off: 8 m cells, a quarter of the tops
        let coarse = mesh_tile(&tiles, [0, 0], far_away, 0.0);
        assert!(
            coarse.len() < all.len() / 2,
            "{} against {}",
            coarse.len(),
            all.len()
        );
        // the ground never stands above the top
        assert!(all.iter().all(|v| v.pos[1] <= 64.0));
        assert!(mesh_tile(&tiles, [5, 5], [0.0, 0.0], 0.0).is_empty());
    }
}
