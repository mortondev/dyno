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

use crate::render::{FarVertex, srgb};
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

/// How a vertex is drawn (render.rs `vs_far`): the ground, or what stands
/// on it, dithered in past its `fade` distance over `FADE` metres.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Ground,
    Standing(f32),
}

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

/// Mean colours of block faces, and the texture set they were made from.
type Means = (u64, HashMap<(BlockId, usize), [f32; 3]>);

thread_local! {
    /// mean colours of block faces, for the texture set they were made from
    static MEANS: RefCell<Means> = RefCell::new((u64::MAX, HashMap::new()));
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
                for p in px.as_chunks::<4>().0.iter().filter(|p| p[3] > 128) {
                    sum[0] += lin(p[0]);
                    sum[1] += lin(p[1]);
                    sum[2] += lin(p[2]);
                    n += 1.0;
                }
            }
            // the near land shows its texture (times the ground's tone):
            // its mean, or the block's plain colour with no texture yet
            // water: the colour the near view's water shader paints it
            // (render settings, between shallow and deep); other
            // see-through blocks: their own colour
            if b == dyno_world::WATER {
                let w = crate::look::settings().water;
                return [
                    (w.shallow[0] + w.deep[0]) * 0.5,
                    (w.shallow[1] + w.deep[1]) * 0.5,
                    (w.shallow[2] + w.deep[2]) * 0.5,
                ];
            }
            let clear = crate::render::block_color(b).1 < 0.99;
            if n > 0.0 && t.layers.len() > 1 && !clear {
                [sum[0] / n, sum[1] / n, sum[2] / n]
            } else {
                srgb(crate::render::block_color(b).0)
            }
        })
    })
}

fn quad(out: &mut Vec<FarVertex>, p: [[f32; 3]; 4], n: [f32; 3], rgb: [f32; 3], kind: Kind) {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let (w, fade) = match kind {
        Kind::Ground => (0, 0.0),
        Kind::Standing(f) => (1, f),
    };
    // whole metres: every far box is a stack of 1 m cubes
    let at = |v: [f32; 3]| {
        [
            v[0].round() as i16,
            v[1].round() as i16,
            v[2].round() as i16,
            fade.round() as i16,
        ]
    };
    let normal = [
        (n[0] * 127.0) as i8,
        (n[1] * 127.0) as i8,
        (n[2] * 127.0) as i8,
        w,
    ];
    // a quad: four corners, drawn with the shared quad index (0 1 2, 0 2 3)
    for corner in p {
        out.push(FarVertex {
            pos: at(corner),
            color: [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), 255],
            normal,
        });
    }
}

/// A box's top and the sides of it that show above each neighbour's height
/// (down to `bottom` where the neighbour is not known).
#[allow(clippy::too_many_arguments)]
fn column(
    out: &mut Vec<FarVertex>,
    (x0, z0, w): (f32, f32, f32),
    bottom: f32,
    top: f32,
    neighbours: [Option<f32>; 4],
    colours: ([f32; 3], [f32; 3]),
    kind: Kind,
) {
    let (x1, z1) = (x0 + w, z0 + w);
    let y = top;
    quad(
        out,
        [[x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0]],
        [0.0, 1.0, 0.0],
        colours.0,
        kind,
    );
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for (k, (dx, dz)) in dirs.into_iter().enumerate() {
        // down to the neighbour's height, however far below (a cliff, a tree
        // or tower over flat land: cut off short, the side hung in the air
        // with the sky showing under it); an unknown neighbour (not here
        // yet): down to the bottom
        let nh = neighbours[k].unwrap_or(bottom);
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
        quad(out, p, n, colours.1, kind);
    }
}

/// A number in [0, 1) for a cell, the same every time (colour and canopy
/// variation that holds still).
fn hash(x: i32, z: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (z as u32).wrapping_mul(0x85EB_CA77)
        ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65536.0
}

fn is_canopy(b: BlockId) -> bool {
    dyno_world::is_leaves(b) || matches!(b, dyno_world::LOG | dyno_world::BIRCH_LOG)
}

/// A colour a little lighter or darker, warmer or cooler, cell by cell.
fn vary(rgb: [f32; 3], amount: f32, gx: i32, gz: i32) -> [f32; 3] {
    let v = (hash(gx, gz, 1) - 0.5) * 2.0 * amount;
    let w = (hash(gx, gz, 2) - 0.5) * amount;
    [
        rgb[0] * (1.0 + v + w),
        rgb[1] * (1.0 + v),
        rgb[2] * (1.0 + v - w),
    ]
}

/// How a far cell is drawn: whether something stands on it, the height of
/// its top (whole metres), and the colours of its top and sides. A wood's
/// crowns step up and down a metre or two cell by cell, and vary in shade,
/// a little darker than the open land, so a wood reads as a wood.
struct Drawn {
    stands: bool,
    canopy: bool,
    top: f32,
    colours: ([f32; 3], [f32; 3]),
}

fn drawn(c: Cell, gx: i32, gz: i32) -> Drawn {
    let (g, gb, h, hb) = c;
    let stands = h > g;
    if stands && is_canopy(hb) {
        let rgb = vary(face_colour(hb, 0), 0.16, gx, gz).map(|v| v * 0.8);
        Drawn {
            stands,
            canopy: true,
            top: h - (hash(gx, gz, 3) * 3.0).floor(),
            colours: (rgb, rgb.map(|v| v * 0.75)),
        }
    } else if stands {
        Drawn {
            stands,
            canopy: false,
            top: h,
            colours: (vary(face_colour(hb, 0), 0.05, gx, gz), face_colour(hb, 1)),
        }
    } else {
        // the ground, a metre low (never above the near land)
        Drawn {
            stands,
            canopy: false,
            top: g - 1.0,
            colours: (vary(face_colour(gb, 0), 0.05, gx, gz), face_colour(gb, 1)),
        }
    }
}

/// The height a cell is drawn at where only the tops are drawn: what stands
/// on it, or its ground.
fn surface(tiles: &FarTiles, gx: i32, gz: i32, step: i32) -> Option<f32> {
    let c = cell_at(tiles, gx, gz, step)?;
    Some(drawn(c, gx.div_euclid(step), gz.div_euclid(step)).top)
}

/// An 8 m block of cells too steep to draw as one: its four cells differ
/// by more than this (a mountainside drawn in 8 m boxes is a wall of big
/// flat faces, each lit as one).
const STEEP: f32 = 4.0;

/// How deep a far tree's crown is drawn inside the fade, over the ground
/// layer: from afar a tree is its crown, on a trunk too thin to see.
const CROWN: f32 = 6.0;

/// The mesh of one tile, as seen from `centre` (x, z): the ground beyond
/// `GROUND_FROM`, and what stands on it beyond `cut` (faded in over
/// `FADE`). Past the fade, one layer: the tops, with no ground under them;
/// beyond `COARSE`, in 8 m cells where the land is gentle enough. Every
/// face is a face of whole 1 m cubes (docs/art/voxel-rule.md).
pub fn mesh_tile(tiles: &FarTiles, at: TilePos, centre: [f32; 2], cut: f32) -> Vec<FarVertex> {
    let mut out = Vec::new();
    if !tiles.contains_key(&at) {
        return out;
    }
    let tile = TILE as f32;
    let tc = [
        at[0] as f32 * tile + tile * 0.5,
        at[1] as f32 * tile + tile * 0.5,
    ];
    let coarse = (tc[0] - centre[0]).hypot(tc[1] - centre[1]) > COARSE;
    let step = if coarse { 2 } else { 1 };
    for lz in (0..TILE_CELLS).step_by(step as usize) {
        for lx in (0..TILE_CELLS).step_by(step as usize) {
            let (gx, gz) = (at[0] * TILE_CELLS + lx, at[1] * TILE_CELLS + lz);
            if !coarse {
                mesh_cell(&mut out, tiles, gx, gz, 1, centre, cut);
                continue;
            }
            let fine: Vec<f32> = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .iter()
                .filter_map(|(dx, dz)| surface(tiles, gx + dx, gz + dz, 1))
                .collect();
            let relief = fine.iter().copied().fold(f32::MIN, f32::max)
                - fine.iter().copied().fold(f32::MAX, f32::min);
            if fine.len() == 4 && relief > STEEP {
                for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    mesh_cell(&mut out, tiles, gx + dx, gz + dz, 1, centre, cut);
                }
            } else {
                mesh_cell(&mut out, tiles, gx, gz, 2, centre, cut);
            }
        }
    }
    out
}

/// One cell (or 8 m block of cells, `step` 2) of the far view.
fn mesh_cell(
    out: &mut Vec<FarVertex>,
    tiles: &FarTiles,
    gx: i32,
    gz: i32,
    step: i32,
    centre: [f32; 2],
    cut: f32,
) {
    let Some(c) = cell_at(tiles, gx, gz, step) else {
        return;
    };
    let (g, gb, _, _) = c;
    let d = drawn(c, gx.div_euclid(step), gz.div_euclid(step));
    let w = (CELL * step) as f32;
    let standing = Kind::Standing(cut);
    let (x0, z0) = (gx as f32 * CELL as f32, gz as f32 * CELL as f32);
    let dist = (x0 + w * 0.5 - centre[0]).hypot(z0 + w * 0.5 - centre[1]);
    let near = [(step, 0), (-step, 0), (0, step), (0, -step)]
        .map(|(dx, dz)| cell_at(tiles, gx + dx, gz + dz, step));
    if dist >= cut + FADE {
        // out here a heightmap of the tops is all there is to see; an 8 m
        // block's sides go down to the lowest cell along each edge, as
        // its neighbours may be drawn in 4 m cells
        let sides = if step == 1 {
            [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dz)| surface(tiles, gx + dx, gz + dz, 1))
        } else {
            [
                [(2, 0), (2, 1)],
                [(-1, 0), (-1, 1)],
                [(0, 2), (1, 2)],
                [(0, -1), (1, -1)],
            ]
            .map(|edge| {
                edge.iter()
                    .filter_map(|(dx, dz)| surface(tiles, gx + dx, gz + dz, 1))
                    .reduce(f32::min)
            })
        };
        column(
            out,
            (x0, z0, w),
            d.top - 8.0,
            d.top,
            sides,
            d.colours,
            if d.stands { standing } else { Kind::Ground },
        );
        if d.canopy {
            clump(out, (x0, z0, w), &d, gx, gz, standing);
        }
        return;
    }
    // the ground: hidden under the near land, carrying on past it
    if dist >= GROUND_FROM && g > 0.0 {
        let y = g - 1.0;
        column(
            out,
            (x0, z0, w),
            y - 8.0,
            y,
            near.map(|c| c.map(|c| c.0 - 1.0)),
            (vary(face_colour(gb, 0), 0.05, gx, gz), face_colour(gb, 1)),
            Kind::Ground,
        );
    }
    // what stands on it: beyond the near view only, faded in
    if dist >= cut && d.stands {
        if d.canopy {
            // a crown over the ground, not a pillar of leaves down to it
            let bottom = (d.top - CROWN).max(g - 1.0);
            let sides = [(step, 0), (-step, 0), (0, step), (0, -step)].map(|(dx, dz)| {
                cell_at(tiles, gx + dx, gz + dz, step).map(|n| {
                    let nd = drawn(n, (gx + dx).div_euclid(step), (gz + dz).div_euclid(step));
                    let nh = if nd.stands { nd.top } else { bottom };
                    nh.clamp(bottom, d.top)
                })
            });
            column(out, (x0, z0, w), bottom, d.top, sides, d.colours, standing);
            clump(out, (x0, z0, w), &d, gx, gz, standing);
        } else {
            column(
                out,
                (x0, z0, w),
                g - 1.0,
                d.top,
                near.map(|c| c.map(|c| c.2)),
                d.colours,
                standing,
            );
        }
    }
}

/// A clump on a crown: a cube half the cell wide and a metre (two, on an
/// 8 m cell) high in a corner of it, lighter where the sun is on it, so a
/// wood's top is stepped like the crowns of the near trees.
fn clump(
    out: &mut Vec<FarVertex>,
    (x0, z0, w): (f32, f32, f32),
    d: &Drawn,
    gx: i32,
    gz: i32,
    kind: Kind,
) {
    let r = hash(gx, gz, 4);
    if r < 0.3 {
        return;
    }
    let half = w * 0.5;
    let (cx, cz) = (
        x0 + if r < 0.65 { 0.0 } else { half },
        z0 + if hash(gx, gz, 5) < 0.5 { 0.0 } else { half },
    );
    let high = (w / 4.0).max(1.0);
    let lit = d.colours.0.map(|v| v * 1.12);
    column(
        out,
        (cx, cz, half),
        d.top,
        d.top + high,
        [None; 4],
        (lit, d.colours.1),
        kind,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The voxel rule (docs/art/voxel-rule.md): every face of the far view
    /// is a face of whole 1 m cubes, its normal one of the six axis
    /// directions (whole metres are the vertex format itself).
    #[test]
    fn far_faces_are_cube_faces() {
        let world = dyno_world::World::test_terrain([192, 64, 192], 7);
        let mut tiles = FarTiles::new();
        for tz in 0..3 {
            for tx in 0..3 {
                tiles.insert([tx, tz], FarTile::build(&world, [tx, tz]));
            }
        }
        for (centre, cut) in [
            ([96.0, 96.0], 40.0),
            ([-300.0, 96.0], 0.0),
            ([-700.0, 96.0], 0.0),
        ] {
            for tz in 0..3 {
                for tx in 0..3 {
                    for v in mesh_tile(&tiles, [tx, tz], centre, cut) {
                        let n = v.normal;
                        let axes = [n[0], n[1], n[2]].iter().filter(|c| **c != 0).count();
                        assert_eq!(axes, 1, "a face off the axes: {n:?}");
                        assert!(
                            [n[0], n[1], n[2]]
                                .iter()
                                .all(|c| matches!(c, 0 | 127 | -127))
                        );
                    }
                }
            }
        }
    }

    /// Far off, a wood's top is stepped crowns of varied shade (not one flat
    /// slab of leaf colour).
    #[test]
    fn far_woods_are_stepped_crowns() {
        let mut world = dyno_world::World::test_terrain([128, 64, 128], 7);
        for z in 8..56 {
            for x in 8..56 {
                let g = world.surface(x, z);
                for y in g + 1..g + 11 {
                    world.set(x, y, z, dyno_world::LEAVES);
                }
            }
        }
        let mut tiles = FarTiles::new();
        for tz in 0..2 {
            for tx in 0..2 {
                tiles.insert([tx, tz], FarTile::build(&world, [tx, tz]));
            }
        }
        let mesh = mesh_tile(&tiles, [0, 0], [-300.0, 32.0], 0.0);
        let tops: Vec<&FarVertex> = mesh
            .iter()
            .filter(|v| v.normal[1] > 0)
            .filter(|v| (16..=48).contains(&v.pos[0]) && (16..=48).contains(&v.pos[2]))
            .collect();
        let heights: std::collections::HashSet<i16> = tops.iter().map(|v| v.pos[1]).collect();
        let shades: std::collections::HashSet<[u8; 4]> = tops.iter().map(|v| v.color).collect();
        assert!(heights.len() >= 4, "{} heights of crown", heights.len());
        assert!(shades.len() >= 8, "{} shades of crown", shades.len());
    }

    /// What stands well above the land round it (a tower, a cliff) is drawn
    /// down to that land, with no sky showing under it, out where only the
    /// tops are drawn and inside the fade.
    #[test]
    fn far_columns_stand_on_the_land() {
        let mut world = dyno_world::World::test_terrain([128, 96, 128], 7);
        // a stone tower 30 m over the land, filling the 4 × 4 m cell at (40, 40)
        let g = (40..44)
            .flat_map(|z| (40..44).map(move |x| (x, z)))
            .map(|(x, z)| world.surface(x, z))
            .max()
            .unwrap();
        let top = g + 30;
        assert!(top < 96, "{top}");
        for z in 40..44 {
            for x in 40..44 {
                for y in world.surface(x, z)..=top {
                    world.set(x, y, z, dyno_world::STONE);
                }
            }
        }
        let mut tiles = FarTiles::new();
        for tz in 0..2 {
            for tx in 0..2 {
                tiles.insert([tx, tz], FarTile::build(&world, [tx, tz]));
            }
        }
        // the lowest point of the tower's sides (the side quads that reach its top)
        let low = |mesh: &[FarVertex]| {
            mesh.chunks(4)
                .filter(|q| q[0].normal[1] == 0)
                .filter(|q| q.iter().any(|v| (v.pos[1] as f32) >= top as f32))
                .filter(|q| {
                    q.iter().all(|v| {
                        (40.0..=44.0).contains(&(v.pos[0] as f32))
                            && (40.0..=44.0).contains(&(v.pos[2] as f32))
                    })
                })
                .flat_map(|q| q.iter().map(|v| v.pos[1] as f32))
                .fold(f32::MAX, f32::min)
        };
        // only tops, far away (and past `COARSE`, where the tower keeps its
        // 4 m cell: too steep for an 8 m one); and within the fade
        for (centre, cut) in [
            ([-300.0, 40.0], 0.0),
            ([-500.0, 40.0], 0.0),
            ([-30.0, 40.0], 60.0),
        ] {
            let mesh = mesh_tile(&tiles, [0, 0], centre, cut);
            let bottom = low(&mesh);
            assert!(
                bottom <= g as f32 + 1.0,
                "the tower's sides stop at {bottom}, its top at {}, the land at {g}",
                top + 1
            );
        }
    }

    #[test]
    fn far_tiles_mesh_in_two_layers_round_the_cut() {
        let world = dyno_world::World::test_terrain([128, 64, 128], 7);
        let mut tiles = FarTiles::new();
        for tz in 0..2 {
            for tx in 0..2 {
                tiles.insert([tx, tz], FarTile::build(&world, [tx, tz]));
            }
        }
        let far_away = [-1000.0, -1000.0];
        let all = mesh_tile(&tiles, [0, 0], [-200.0, -200.0], 0.0);
        // every cell has its ground top
        assert!(all.len() >= 256 * 4, "{}", all.len());
        // near the camera: only the ground (under the near land), nothing standing
        let cut = mesh_tile(&tiles, [0, 0], [32.0, 32.0], 200.0);
        assert!(cut.iter().all(|v| v.normal[3] == 0), "only the ground");
        assert!(cut.iter().all(
            |v| ((v.pos[0] as f32) - 32.0).hypot((v.pos[2] as f32) - 32.0) >= GROUND_FROM - 6.0
        ));
        // far off: 8 m cells where the land is gentle, far fewer vertices
        let coarse = mesh_tile(&tiles, [0, 0], far_away, 0.0);
        assert!(
            coarse.len() < all.len() * 3 / 4,
            "{} against {}",
            coarse.len(),
            all.len()
        );
        // the ground never stands above the top
        assert!(all.iter().all(|v| (v.pos[1] as f32) <= 64.0));
        // a far vertex is small
        assert_eq!(std::mem::size_of::<FarVertex>(), 16);
        assert!(mesh_tile(&tiles, [5, 5], [0.0, 0.0], 0.0).is_empty());
    }
}
