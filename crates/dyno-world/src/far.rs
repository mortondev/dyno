//! The far view: a coarse heightmap of the whole world, so the land and
//! its landmarks (a watchtower, a windmill, a headframe, a hero tree) can be
//! seen from 400–800 m, beyond the 160 m of streamed sections
//! (docs/design/data-and-streaming.md §4, docs/design/scale.md).
//!
//! The world is cut into cells of 4 × 4 columns, and the cells into tiles
//! of 16 × 16 (64 m). For each cell the server records two things: the
//! highest block that is not air and its kind (a tower or a tree's crown wins
//! its cell, so tall things stand out), and the ground: the lowest top of
//! the ground in the cell (trees, plants and water left out) and its kind,
//! which never stands above the near view's land, so it can be drawn under
//! it to close every gap at the seam. It is made from the world itself (no
//! content in code), streamed tile by tile like sections, and drawn as
//! blocky columns.

use crate::{BlockId, SECTION, World, rle, unrle};

/// Columns along a cell's edge (metres).
pub const CELL: i32 = 4;
/// Cells along a tile's edge.
pub const TILE_CELLS: i32 = 16;
/// Metres along a tile's edge.
pub const TILE: i32 = CELL * TILE_CELLS;
const CELLS: usize = (TILE_CELLS * TILE_CELLS) as usize;

/// A tile position (tiles, not metres).
pub type TilePos = [i32; 2];

/// One tile of the far view.
#[derive(Clone, Debug, PartialEq)]
pub struct FarTile {
    /// per cell, x fastest: the height of the top of its highest block (0:
    /// nothing there)
    pub heights: Vec<u16>,
    /// per cell: that block
    pub blocks: Vec<BlockId>,
    /// per cell: the height of the lowest top of the ground (0: none)
    pub ground: Vec<u16>,
    /// per cell: that ground's block
    pub ground_blocks: Vec<BlockId>,
}

/// The top of the ground in a column: the highest solid block that is not
/// part of a tree or a plant, and its height + 1.
fn ground_top(world: &World, x: i32, z: i32, from: i32) -> Option<(u16, BlockId)> {
    let mut y = from;
    while y >= 0 {
        let b = world.get(x, y, z);
        let plant = crate::is_leaves(b) || matches!(b, crate::LOG | crate::BIRCH_LOG);
        if crate::is_solid(b) && !plant {
            return Some(((y + 1) as u16, b));
        }
        y -= 1;
    }
    None
}

/// The land as it lies (not built on, not grown on): a cell's top made of
/// these takes the block most of it is made of.
fn bare_land(b: BlockId) -> bool {
    use crate::*;
    matches!(
        b,
        GRASS | DIRT | SAND | SNOW | STONE | CLAY | COAL_ORE | IRON_ORE
    )
}

impl FarTile {
    /// The tile as the world stands.
    pub fn build(world: &World, tile: TilePos) -> FarTile {
        let mut heights = vec![0u16; CELLS];
        let mut blocks = vec![crate::AIR; CELLS];
        let mut ground = vec![u16::MAX; CELLS];
        let mut ground_blocks = vec![crate::AIR; CELLS];
        for cz in 0..TILE_CELLS {
            for cx in 0..TILE_CELLS {
                let k = (cz * TILE_CELLS + cx) as usize;
                // the blocks the land's tops are made of, and whether the
                // highest top is land (not a tree, a roof or water)
                let mut land: Vec<(BlockId, u8)> = Vec::new();
                let mut highest_is_land = false;
                for dz in 0..CELL {
                    for dx in 0..CELL {
                        let x = tile[0] * TILE + cx * CELL + dx;
                        let z = tile[1] * TILE + cz * CELL + dz;
                        let Some((y, b)) = world.column_top(x, z) else {
                            ground[k] = 0;
                            continue;
                        };
                        let g = ground_top(world, x, z, y);
                        let is_land = bare_land(b) && g.is_some_and(|(g, _)| g == (y + 1) as u16);
                        if (y + 1) as u16 > heights[k] {
                            heights[k] = (y + 1) as u16;
                            blocks[k] = b;
                            highest_is_land = is_land;
                        }
                        if is_land {
                            match land.iter_mut().find(|(lb, _)| *lb == b) {
                                Some(e) => e.1 += 1,
                                None => land.push((b, 1)),
                            }
                        }
                        match g {
                            Some((g, gb)) if g < ground[k] => {
                                ground[k] = g;
                                ground_blocks[k] = gb;
                            }
                            None => ground[k] = 0,
                            _ => {}
                        }
                    }
                }
                if ground[k] == u16::MAX {
                    ground[k] = 0;
                }
                // bare land at the top: the block most of it is made of, so
                // one outcrop or boulder on a grass slope does not make the
                // whole cell stone (a pale square from afar); a tree, a roof
                // or water at the top stays what it is
                if highest_is_land && let Some((b, _)) = land.iter().max_by_key(|(_, n)| *n) {
                    blocks[k] = *b;
                }
            }
        }
        FarTile {
            heights,
            blocks,
            ground,
            ground_blocks,
        }
    }

    /// Height and block of a cell of this tile.
    pub fn cell(&self, cx: i32, cz: i32) -> (u16, BlockId) {
        let k = (cz * TILE_CELLS + cx) as usize;
        (self.heights[k], self.blocks[k])
    }

    /// The ground of a cell: its lowest top, and that block.
    pub fn ground(&self, cx: i32, cz: i32) -> (u16, BlockId) {
        let k = (cz * TILE_CELLS + cx) as usize;
        (self.ground[k], self.ground_blocks[k])
    }

    /// Encoded for the wire: the lowest height, the heights above it (a
    /// byte each when they fit, else two), then the blocks run-length encoded.
    pub fn encode(&self) -> Vec<u8> {
        let all = self.heights.iter().chain(&self.ground).copied();
        let base = all.clone().min().unwrap_or(0);
        let top = all.max().unwrap_or(0);
        let mut out = Vec::with_capacity(CELLS * 2 + 64);
        let wide = top - base > 255;
        out.push(if wide { 2 } else { 1 });
        out.extend_from_slice(&base.to_le_bytes());
        for h in self.heights.iter().chain(&self.ground) {
            let v = h - base;
            if wide {
                out.extend_from_slice(&v.to_le_bytes());
            } else {
                out.push(v as u8);
            }
        }
        rle(
            self.blocks.iter().chain(&self.ground_blocks).copied(),
            &mut out,
        );
        out
    }

    pub fn decode(data: &[u8]) -> Result<FarTile, String> {
        let bad = || "a bad far tile".to_string();
        let wide = match data.first() {
            Some(1) => false,
            Some(2) => true,
            _ => return Err(bad()),
        };
        let base =
            u16::from_le_bytes([*data.get(1).ok_or_else(bad)?, *data.get(2).ok_or_else(bad)?]);
        let n = if wide { CELLS * 4 } else { CELLS * 2 };
        let hs = data.get(3..3 + n).ok_or_else(bad)?;
        let mut all: Vec<u16> = (0..CELLS * 2)
            .map(|i| {
                if wide {
                    base.saturating_add(u16::from_le_bytes([hs[i * 2], hs[i * 2 + 1]]))
                } else {
                    base.saturating_add(hs[i] as u16)
                }
            })
            .collect();
        let ground = all.split_off(CELLS);
        let mut blocks = Vec::with_capacity(CELLS * 2);
        unrle(&data[3 + n..], &mut blocks, CELLS * 2)?;
        let ground_blocks = blocks.split_off(CELLS);
        Ok(FarTile {
            heights: all,
            blocks,
            ground,
            ground_blocks,
        })
    }
}

/// How many tiles the world has along x and z.
pub fn tile_dims(world: &World) -> [i32; 2] {
    [
        (world.size[0] as i32 + TILE - 1) / TILE,
        (world.size[2] as i32 + TILE - 1) / TILE,
    ]
}

/// Changes whenever a block in the tile changes (the sum of its sections'
/// counts of changes), so a tile is made and sent again only then.
/// Every tile of a world, encoded, with its revision: what a server keeps so
/// no tick has to build tiles (the first far view of a join comes in a burst).
pub type Encoded = std::collections::HashMap<TilePos, (u64, Vec<u8>)>;

/// Build and encode every tile of a world, on every core (the 2 km world:
/// 1,024 tiles, about 0.7 MB).
pub fn encode_all(world: &World) -> Encoded {
    let d = tile_dims(world);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 8);
    let rows: Vec<i32> = (0..d[1]).collect();
    std::thread::scope(|sc| {
        let jobs: Vec<_> = rows
            .chunks(rows.len().div_ceil(threads).max(1))
            .map(|part| {
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for &tz in part {
                        for tx in 0..d[0] {
                            let t = [tx, tz];
                            let data = FarTile::build(world, t).encode();
                            out.push((t, (tile_rev(world, t), data)));
                        }
                    }
                    out
                })
            })
            .collect();
        jobs.into_iter()
            .flat_map(|j| j.join().expect("a far tile thread panicked"))
            .collect()
    })
}

pub fn tile_rev(world: &World, tile: TilePos) -> u64 {
    let per = TILE / SECTION;
    let d = world.section_dims();
    let mut sum = 0u64;
    for sz in tile[1] * per..(tile[1] + 1) * per {
        for sx in tile[0] * per..(tile[0] + 1) * per {
            for sy in 0..d[1] {
                sum += world.section_rev([sx, sy, sz]) as u64;
            }
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tower_stands_out_in_the_far_view() {
        let mut w = World::generate([128, 64, 128], 7);
        let (x, z) = (70, 70);
        let g = w.surface(x, z);
        for y in g + 1..g + 21 {
            w.set(x, y, z, crate::STONE_BRICKS);
        }
        let t = FarTile::build(&w, [1, 1]);
        let (cx, cz) = ((x - TILE) / CELL, (z - TILE) / CELL);
        let (h, b) = t.cell(cx, cz);
        assert_eq!(h as i32, g + 21, "the tower's top");
        assert_eq!(b, crate::STONE_BRICKS);
        // its neighbours are the land round it
        let (h2, _) = t.cell(cx + 2, cz);
        assert!((h2 as i32) < g + 21, "{h2} beside a tower of {}", g + 21);
        // the ground under the tower is the land, never above the tallest top
        let (gh, _) = t.ground(cx, cz);
        assert!(
            (gh as i32) <= g + 1,
            "ground {gh} under a tower on {}",
            g + 1
        );
        assert!((0..16).all(|z| (0..16).all(|x| t.ground(x, z).0 <= t.cell(x, z).0)));
        // the wire form round-trips, and is small
        let e = t.encode();
        assert!(e.len() < 1000, "{} bytes", e.len());
        assert_eq!(FarTile::decode(&e).unwrap(), t);
        assert!(FarTile::decode(&e[..10]).is_err());
        // an edit changes the tile's revision; the far tile next door does not change
        let r = tile_rev(&w, [1, 1]);
        let r0 = tile_rev(&w, [0, 0]);
        w.set(x, g + 25, z, crate::LANTERN);
        assert_ne!(tile_rev(&w, [1, 1]), r);
        assert_eq!(tile_rev(&w, [0, 0]), r0);
        assert_eq!(tile_dims(&w), [2, 2]);
    }

    /// A boulder on a grass slope does not make its cell stone: bare land
    /// takes the block most of the cell's top is made of.
    #[test]
    fn a_cell_of_bare_land_is_what_most_of_it_is() {
        let mut w = World::generate([128, 64, 128], 7);
        let (x0, z0) = (80, 80);
        // a grass cell with one stone a block above the rest
        let top = (0..CELL)
            .flat_map(|dz| (0..CELL).map(move |dx| (dx, dz)))
            .map(|(dx, dz)| w.surface(x0 + dx, z0 + dz))
            .max()
            .unwrap();
        for dz in 0..CELL {
            for dx in 0..CELL {
                let (x, z) = (x0 + dx, z0 + dz);
                for y in w.surface(x, z) + 1..=top + 4 {
                    w.set(x, y, z, crate::AIR);
                }
                for y in w.surface(x, z)..=top {
                    w.set(x, y, z, crate::DIRT);
                }
                w.set(x, top, z, crate::GRASS);
            }
        }
        w.set(x0 + 1, top + 1, z0 + 2, crate::STONE);
        let (cx, cz) = ((x0 - TILE) / CELL, (z0 - TILE) / CELL);
        let t = FarTile::build(&w, [1, 1]);
        assert_eq!(t.cell(cx, cz), ((top + 2) as u16, crate::GRASS));
    }
}
