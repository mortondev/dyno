//! Block world storage shared by the client and the server (bootstrap).
//!
//! The bootstrap world is one fixed-size grid of cells. Each cell holds one
//! block ID. The world is sent to clients and saved to disk run-length encoded.

use std::io::Write;
use std::path::Path;

pub type BlockId = u8;

pub const AIR: BlockId = 0;
pub const GRASS: BlockId = 1;
pub const DIRT: BlockId = 2;
pub const STONE: BlockId = 3;
pub const SAND: BlockId = 4;
pub const LOG: BlockId = 5;
pub const LEAVES: BlockId = 6;
pub const PLANKS: BlockId = 7;
pub const GLASS: BlockId = 8;
pub const EMBERSTONE: BlockId = 9;
pub const BRICK: BlockId = 10;
pub const SNOW: BlockId = 11;
pub const COBBLE: BlockId = 12;
pub const WATER: BlockId = 13;
pub const LANTERN: BlockId = 14;
pub const GLOWSHROOM: BlockId = 33;

/// How a block sounds when it is broken, placed or walked on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Material {
    Stone,
    Earth,
    Wood,
    Foliage,
    Glass,
    Cloth,
    Water,
}

/// Static data of one block type. The client draws from this; the server checks IDs against it.
#[derive(Clone, Copy, Debug)]
pub struct BlockDef {
    pub name: &'static str,
    /// sRGB colour
    pub color: [f32; 3],
    /// 1.0 for opaque blocks, lower for glass and water
    pub alpha: f32,
    /// Light level 0..=15 that the block gives off
    pub emission: u8,
    pub material: Material,
}

const fn def(name: &'static str, color: [f32; 3], material: Material) -> BlockDef {
    BlockDef {
        name,
        color,
        alpha: 1.0,
        emission: 0,
        material,
    }
}

use Material::*;

/// All block types, indexed by block ID.
pub const BLOCKS: &[BlockDef] = &[
    def("Air", [0.0, 0.0, 0.0], Stone),
    def("Grass", [0.42, 0.66, 0.29], Earth),
    def("Dirt", [0.53, 0.38, 0.26], Earth),
    def("Stone", [0.52, 0.53, 0.56], Stone),
    def("Sand", [0.86, 0.81, 0.62], Earth),
    def("Log", [0.42, 0.31, 0.19], Wood),
    def("Leaves", [0.25, 0.50, 0.21], Foliage),
    def("Planks", [0.72, 0.56, 0.35], Wood),
    BlockDef {
        name: "Glass",
        color: [0.78, 0.90, 0.96],
        alpha: 0.35,
        emission: 0,
        material: Glass,
    },
    BlockDef {
        name: "Emberstone",
        color: [1.0, 0.55, 0.22],
        alpha: 1.0,
        emission: 11,
        material: Stone,
    },
    def("Brick", [0.66, 0.30, 0.24], Stone),
    def("Snow", [0.94, 0.96, 0.98], Earth),
    def("Cobble", [0.44, 0.45, 0.47], Stone),
    BlockDef {
        name: "Water",
        color: [0.20, 0.42, 0.78],
        alpha: 0.62,
        emission: 0,
        material: Water,
    },
    BlockDef {
        name: "Lantern",
        color: [1.0, 0.86, 0.55],
        alpha: 1.0,
        emission: 15,
        material: Glass,
    },
    def("Basalt", [0.20, 0.20, 0.23], Stone),
    def("Marble", [0.90, 0.89, 0.86], Stone),
    def("Clay", [0.62, 0.64, 0.70], Earth),
    def("Terracotta", [0.70, 0.42, 0.30], Stone),
    def("Mossy cobble", [0.38, 0.47, 0.35], Stone),
    def("Dark planks", [0.36, 0.25, 0.16], Wood),
    def("Birch planks", [0.85, 0.76, 0.55], Wood),
    BlockDef {
        name: "Ice",
        color: [0.62, 0.80, 0.96],
        alpha: 0.7,
        emission: 0,
        material: Glass,
    },
    def("White", [0.93, 0.93, 0.93], Cloth),
    def("Black", [0.12, 0.12, 0.14], Cloth),
    def("Red", [0.78, 0.18, 0.17], Cloth),
    def("Orange", [0.93, 0.50, 0.12], Cloth),
    def("Yellow", [0.96, 0.82, 0.20], Cloth),
    def("Lime", [0.50, 0.80, 0.18], Cloth),
    def("Cyan", [0.15, 0.62, 0.68], Cloth),
    def("Blue", [0.20, 0.30, 0.72], Cloth),
    def("Purple", [0.50, 0.24, 0.68], Cloth),
    def("Pink", [0.93, 0.56, 0.68], Cloth),
    BlockDef {
        name: "Glowshroom",
        color: [0.45, 0.95, 0.85],
        alpha: 1.0,
        emission: 13,
        material: Foliage,
    },
];

/// Number of block IDs in use. IDs at or above this are rejected.
pub const BLOCK_COUNT: BlockId = BLOCKS.len() as BlockId;

pub fn block(b: BlockId) -> &'static BlockDef {
    BLOCKS.get(b as usize).unwrap_or(&BLOCKS[0])
}

/// Blocks a player can place: every block except air and water.
pub fn placeable() -> impl Iterator<Item = BlockId> {
    (1..BLOCK_COUNT).filter(|b| *b != WATER)
}

/// Default hotbar.
pub const HOTBAR: [BlockId; 9] = [
    GRASS, STONE, COBBLE, PLANKS, LOG, GLASS, BRICK, LANTERN, EMBERSTONE,
];

/// True for blocks that the player cannot walk through.
pub fn is_solid(b: BlockId) -> bool {
    b != AIR && b != WATER
}

/// True for blocks that hide the faces of their neighbours and block light.
pub fn is_opaque(b: BlockId) -> bool {
    b != AIR && b != LEAVES && block(b).alpha >= 1.0
}

/// Light level a block gives off.
pub fn emission(b: BlockId) -> u8 {
    block(b).emission
}

const MAGIC: &[u8; 5] = b"DYNW1";

#[derive(Clone)]
pub struct World {
    pub size: [u32; 3],
    cells: Vec<BlockId>,
}

impl World {
    pub fn empty(size: [u32; 3]) -> Self {
        let n = size[0] as usize * size[1] as usize * size[2] as usize;
        World {
            size,
            cells: vec![AIR; n],
        }
    }

    #[inline]
    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let [sx, sy, sz] = self.size;
        if x < 0 || y < 0 || z < 0 || x >= sx as i32 || y >= sy as i32 || z >= sz as i32 {
            return None;
        }
        Some((y as usize * sz as usize + z as usize) * sx as usize + x as usize)
    }

    pub fn in_bounds(&self, p: [i32; 3]) -> bool {
        self.index(p[0], p[1], p[2]).is_some()
    }

    /// Block at a position. Outside the world: air above, stone below.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        match self.index(x, y, z) {
            Some(i) => self.cells[i],
            None => {
                if y < 0 {
                    STONE
                } else {
                    AIR
                }
            }
        }
    }

    /// Set a block. Returns false if the position is outside the world.
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: BlockId) -> bool {
        match self.index(x, y, z) {
            Some(i) => {
                self.cells[i] = b;
                true
            }
            None => false,
        }
    }

    /// Generate the default terrain: hills, sand, a lake, trees and some emberstone.
    pub fn generate(size: [u32; 3], seed: u32) -> Self {
        let mut w = World::empty(size);
        let [sx, sy, sz] = size;
        let water_level = 17;
        for z in 0..sz as i32 {
            for x in 0..sx as i32 {
                let h = height(x, z, seed).clamp(4, sy as i32 - 12);
                for y in 0..=h {
                    let b = if y == h {
                        if h <= water_level + 1 {
                            SAND
                        } else if h > 38 {
                            SNOW
                        } else {
                            GRASS
                        }
                    } else if y > h - 4 {
                        if h <= water_level + 1 { SAND } else { DIRT }
                    } else {
                        STONE
                    };
                    w.set(x, y, z, b);
                }
                for y in h + 1..=water_level {
                    w.set(x, y, z, WATER);
                }
                // glowing mushrooms here and there on grass
                if h > water_level + 1 && h <= 38 && hash(x, z, seed ^ 0x9105).is_multiple_of(173) {
                    w.set(x, h + 1, z, GLOWSHROOM);
                }
                // emberstone veins near the surface of stone areas
                if hash(x, z, seed ^ 0x5eed).is_multiple_of(211) && h > water_level + 2 {
                    w.set(x, h - 2, z, EMBERSTONE);
                }
            }
        }
        // trees
        for z in (3..sz as i32 - 3).step_by(5) {
            for x in (3..sx as i32 - 3).step_by(5) {
                let r = hash(x, z, seed ^ 0x7ee5);
                if !r.is_multiple_of(3) {
                    continue;
                }
                let tx = x + (r % 4) as i32 - 1;
                let tz = z + ((r >> 4) % 4) as i32 - 1;
                let h = height(tx, tz, seed);
                if w.get(tx, h, tz) != GRASS {
                    continue;
                }
                let trunk = 4 + (r >> 8) as i32 % 3;
                for y in 1..=trunk {
                    w.set(tx, h + y, tz, LOG);
                }
                for dy in -2..=1i32 {
                    let rad: i32 = if dy == 1 { 1 } else { 2 };
                    for dz in -rad..=rad {
                        for dx in -rad..=rad {
                            if dx.abs() == rad
                                && dz.abs() == rad
                                && (dy == 1 || hash(tx + dx, tz + dz, dy as u32).is_multiple_of(2))
                            {
                                continue;
                            }
                            let (px, py, pz) = (tx + dx, h + trunk + dy, tz + dz);
                            if w.get(px, py, pz) == AIR {
                                w.set(px, py, pz, LEAVES);
                            }
                        }
                    }
                }
            }
        }
        w
    }

    /// Highest solid block in a column, or -1.
    pub fn surface(&self, x: i32, z: i32) -> i32 {
        for y in (0..self.size[1] as i32).rev() {
            if is_solid(self.get(x, y, z)) {
                return y;
            }
        }
        -1
    }

    /// A spawn point at the centre of the world, on top of the surface.
    pub fn spawn(&self) -> [f32; 3] {
        let x = self.size[0] as i32 / 2;
        let z = self.size[2] as i32 / 2;
        let y = self.surface(x, z).max(self.surface_water(x, z));
        [x as f32 + 0.5, y as f32 + 1.01, z as f32 + 0.5]
    }

    fn surface_water(&self, x: i32, z: i32) -> i32 {
        for y in (0..self.size[1] as i32).rev() {
            if self.get(x, y, z) != AIR {
                return y;
            }
        }
        -1
    }

    /// Run-length encode the world: header, then (block, varint count) pairs.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 * 1024);
        out.extend_from_slice(MAGIC);
        for s in self.size {
            out.extend_from_slice(&s.to_le_bytes());
        }
        let mut i = 0;
        while i < self.cells.len() {
            let b = self.cells[i];
            let mut n = 1usize;
            while i + n < self.cells.len() && self.cells[i + n] == b {
                n += 1;
            }
            out.push(b);
            let mut v = n as u64;
            loop {
                let byte = (v & 0x7f) as u8;
                v >>= 7;
                if v == 0 {
                    out.push(byte);
                    break;
                }
                out.push(byte | 0x80);
            }
            i += n;
        }
        out
    }

    /// Decode data from `encode`. Rejects malformed or oversized input.
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        if data.len() < 17 || &data[..5] != MAGIC {
            return Err("not a Dyno world".into());
        }
        let rd = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
        let size = [rd(5), rd(9), rd(13)];
        if size.iter().any(|&s| s == 0 || s > 1024) || size[1] > 256 {
            return Err(format!("world size {size:?} is not supported"));
        }
        let total = size[0] as usize * size[1] as usize * size[2] as usize;
        let mut cells = Vec::with_capacity(total);
        let mut p = 17;
        while p < data.len() {
            let b = data[p];
            if b >= BLOCK_COUNT {
                return Err(format!("unknown block id {b}"));
            }
            p += 1;
            let mut n: u64 = 0;
            let mut shift = 0;
            loop {
                let byte = *data.get(p).ok_or("truncated world data")?;
                p += 1;
                n |= ((byte & 0x7f) as u64) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
                if shift > 35 {
                    return Err("bad run length".into());
                }
            }
            if cells.len() + n as usize > total {
                return Err("world data is longer than the world".into());
            }
            cells.resize(cells.len() + n as usize, b);
        }
        if cells.len() != total {
            return Err("world data is shorter than the world".into());
        }
        Ok(World { size, cells })
    }

    /// Save atomically: write a temporary file, then rename it.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&self.encode())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::decode(&data)
    }
}

fn hash(x: i32, z: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (z as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}

fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let (xi, zi) = (x.floor() as i32, z.floor() as i32);
    let (fx, fz) = (x - xi as f32, z - zi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let v = |a: i32, b: i32| (hash(a, b, seed) & 0xffff) as f32 / 65535.0;
    let (u, w) = (s(fx), s(fz));
    let top = v(xi, zi) * (1.0 - u) + v(xi + 1, zi) * u;
    let bot = v(xi, zi + 1) * (1.0 - u) + v(xi + 1, zi + 1) * u;
    top * (1.0 - w) + bot * w
}

fn height(x: i32, z: i32, seed: u32) -> i32 {
    let (fx, fz) = (x as f32, z as f32);
    let n = value_noise(fx / 48.0, fz / 48.0, seed) * 0.6
        + value_noise(fx / 20.0, fz / 20.0, seed + 1) * 0.3
        + value_noise(fx / 8.0, fz / 8.0, seed + 2) * 0.1;
    let hills = value_noise(fx / 90.0, fz / 90.0, seed + 3);
    (14.0 + n * 14.0 + hills * hills * 26.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_roundtrip() {
        let w = World::generate([64, 48, 64], 7);
        let d = w.encode();
        let w2 = World::decode(&d).unwrap();
        assert_eq!(w.cells, w2.cells);
        assert!(d.len() < w.cells.len() / 4, "compression should work");
    }

    #[test]
    fn rejects_garbage() {
        assert!(World::decode(b"nope").is_err());
        let mut d = World::empty([4, 4, 4]).encode();
        d.push(200);
        assert!(World::decode(&d).is_err());
    }

    #[test]
    fn spawn_is_above_ground() {
        let w = World::generate([64, 48, 64], 1);
        let s = w.spawn();
        assert_eq!(w.get(s[0] as i32, s[1] as i32, s[2] as i32), AIR);
    }
}
