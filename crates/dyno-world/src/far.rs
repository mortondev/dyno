//! The far view: a coarse heightmap of the whole world, so the land and
//! its landmarks (a watchtower, a windmill, a headframe, a hero tree) can be
//! seen from 400–800 m, beyond the 160 m of streamed sections
//! (docs/design/data-and-streaming.md §4, docs/design/scale.md).
//!
//! The world is cut into cells of 4 × 4 columns, and the cells into tiles
//! of 16 × 16 (64 m). For each cell the server records the highest block
//! that is not air and its kind; a tower or a tree's crown wins its cell, so
//! tall things stand out. It is made from the world itself (no content in
//! code), streamed tile by tile like sections, and drawn as blocky columns.

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
}

impl FarTile {
    /// The tile as the world stands.
    pub fn build(world: &World, tile: TilePos) -> FarTile {
        let mut heights = vec![0u16; CELLS];
        let mut blocks = vec![crate::AIR; CELLS];
        for cz in 0..TILE_CELLS {
            for cx in 0..TILE_CELLS {
                let k = (cz * TILE_CELLS + cx) as usize;
                for dz in 0..CELL {
                    for dx in 0..CELL {
                        let x = tile[0] * TILE + cx * CELL + dx;
                        let z = tile[1] * TILE + cz * CELL + dz;
                        if let Some((y, b)) = world.column_top(x, z)
                            && (y + 1) as u16 > heights[k]
                        {
                            heights[k] = (y + 1) as u16;
                            blocks[k] = b;
                        }
                    }
                }
            }
        }
        FarTile { heights, blocks }
    }

    /// Height and block of a cell of this tile.
    pub fn cell(&self, cx: i32, cz: i32) -> (u16, BlockId) {
        let k = (cz * TILE_CELLS + cx) as usize;
        (self.heights[k], self.blocks[k])
    }

    /// Encoded for the wire: the lowest height, the heights above it (a
    /// byte each when they fit, else two), then the blocks run-length encoded.
    pub fn encode(&self) -> Vec<u8> {
        let base = self.heights.iter().copied().min().unwrap_or(0);
        let top = self.heights.iter().copied().max().unwrap_or(0);
        let mut out = Vec::with_capacity(CELLS + 64);
        let wide = top - base > 255;
        out.push(if wide { 2 } else { 1 });
        out.extend_from_slice(&base.to_le_bytes());
        for h in &self.heights {
            let v = h - base;
            if wide {
                out.extend_from_slice(&v.to_le_bytes());
            } else {
                out.push(v as u8);
            }
        }
        rle(self.blocks.iter().copied(), &mut out);
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
        let n = if wide { CELLS * 2 } else { CELLS };
        let hs = data.get(3..3 + n).ok_or_else(bad)?;
        let heights = (0..CELLS)
            .map(|i| {
                if wide {
                    base.saturating_add(u16::from_le_bytes([hs[i * 2], hs[i * 2 + 1]]))
                } else {
                    base.saturating_add(hs[i] as u16)
                }
            })
            .collect();
        let mut blocks = Vec::with_capacity(CELLS);
        unrle(&data[3 + n..], &mut blocks, CELLS)?;
        Ok(FarTile { heights, blocks })
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
        // the wire form round-trips, and is small
        let e = t.encode();
        assert!(e.len() < 600, "{} bytes", e.len());
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
}
