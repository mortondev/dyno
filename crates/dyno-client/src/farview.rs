//! The far view: the coarse heightmap the server streams beyond the near
//! sections (`dyno_world::far`), drawn as blocky columns of 4 m, so the
//! land's silhouette and its landmarks show from 400–800 m.
//!
//! A tile's mesh leaves out the cells round the camera where the near view
//! is drawn (`cut`), and is remade as the camera moves on; each cell is a
//! top and the sides it shows above its neighbours.

use crate::render::{NO_TEX, Vertex, block_color, srgb};
use dyno_world::far::{CELL, FarTile, TILE_CELLS, TilePos};
use std::collections::HashMap;

/// The tiles held, by tile position.
pub type FarTiles = HashMap<TilePos, FarTile>;

/// The height of the cell at world cell coordinates, if its tile is held.
fn height_at(tiles: &FarTiles, cx: i32, cz: i32) -> Option<u16> {
    let t = [cx.div_euclid(TILE_CELLS), cz.div_euclid(TILE_CELLS)];
    tiles.get(&t).map(|tile| {
        tile.cell(cx.rem_euclid(TILE_CELLS), cz.rem_euclid(TILE_CELLS))
            .0
    })
}

fn quad(out: &mut Vec<Vertex>, p: [[f32; 3]; 4], rgb: [f32; 3], shade: f32) {
    for i in [0, 1, 2, 0, 2, 3] {
        out.push(Vertex {
            pos: p[i],
            normal: [0.0; 3],
            color: [rgb[0], rgb[1], rgb[2], shade],
            light: 0.0,
            sky: 1.0,
            tex: NO_TEX,
        });
    }
}

/// The mesh of one tile: every cell further than `cut` metres from
/// `centre` (x, z) as a column.
pub fn mesh_tile(tiles: &FarTiles, at: TilePos, centre: [f32; 2], cut: f32) -> Vec<Vertex> {
    let mut out = Vec::new();
    let Some(tile) = tiles.get(&at) else {
        return out;
    };
    let c = CELL as f32;
    for lz in 0..TILE_CELLS {
        for lx in 0..TILE_CELLS {
            let (h, b) = tile.cell(lx, lz);
            if h == 0 {
                continue;
            }
            let (gx, gz) = (at[0] * TILE_CELLS + lx, at[1] * TILE_CELLS + lz);
            let (x0, z0) = (gx as f32 * c, gz as f32 * c);
            if (x0 + c * 0.5 - centre[0]).hypot(z0 + c * 0.5 - centre[1]) < cut {
                continue;
            }
            let rgb = srgb(block_color(b).0);
            let y = h as f32;
            let (x1, z1) = (x0 + c, z0 + c);
            quad(
                &mut out,
                [[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
                rgb,
                1.0,
            );
            // the sides it shows above each neighbour (an unknown neighbour:
            // a short skirt, so no gap shows)
            for (dx, dz, shade) in [(1, 0, 0.8), (-1, 0, 0.8), (0, 1, 0.68), (0, -1, 0.68)] {
                let nh = height_at(tiles, gx + dx, gz + dz)
                    .map(|v| v as f32)
                    .unwrap_or(y - 8.0);
                if nh >= y {
                    continue;
                }
                let p = match (dx, dz) {
                    (1, _) => [[x1, nh, z0], [x1, nh, z1], [x1, y, z1], [x1, y, z0]],
                    (-1, _) => [[x0, nh, z1], [x0, nh, z0], [x0, y, z0], [x0, y, z1]],
                    (_, 1) => [[x1, nh, z1], [x0, nh, z1], [x0, y, z1], [x1, y, z1]],
                    _ => [[x0, nh, z0], [x1, nh, z0], [x1, y, z0], [x0, y, z0]],
                };
                quad(&mut out, p, rgb, shade);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_tiles_mesh_outside_the_cut() {
        let world = dyno_world::World::generate([128, 64, 128], 7);
        let mut tiles = FarTiles::new();
        for tz in 0..2 {
            for tx in 0..2 {
                tiles.insert([tx, tz], FarTile::build(&world, [tx, tz]));
            }
        }
        let all = mesh_tile(&tiles, [0, 0], [-1000.0, -1000.0], 0.0);
        // every cell has its top
        assert!(all.len() >= 256 * 6, "{}", all.len());
        // round the camera, nothing: the near view is drawn there
        let cut = mesh_tile(&tiles, [0, 0], [32.0, 32.0], 30.0);
        assert!(cut.len() < all.len() / 2);
        assert!(
            cut.iter()
                .all(|v| (v.pos[0] - 32.0).hypot(v.pos[2] - 32.0) >= 30.0 - 6.0)
        );
        assert!(mesh_tile(&tiles, [5, 5], [0.0, 0.0], 0.0).is_empty());
    }
}
