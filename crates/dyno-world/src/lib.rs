//! Block world storage shared by the client and the server.
//!
//! The server's world is one fixed-size grid of cells. Each cell holds one
//! block ID. It is saved to disk run-length encoded. Clients hold a sparse
//! world: the sections of 16 x 16 x 16 cells the server has streamed to them
//! (docs/design/data-and-streaming.md §4). A cell of a section that has not
//! arrived is *unknown*: `get` answers air (so nothing is drawn there and rays
//! pass through), while `solid` answers true (so nobody falls through ground
//! that has not arrived yet).

pub mod content;
pub mod far;
pub mod light;
pub mod poi;
pub mod village;
pub mod zones;

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

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
pub const BIRCH_LOG: BlockId = 34;
pub const BIRCH_LEAVES: BlockId = 35;
pub const PINE_LEAVES: BlockId = 36;
pub const BEDROCK: BlockId = 37;
pub const COAL_ORE: BlockId = 38;
pub const IRON_ORE: BlockId = 39;
pub const CRAFTING_TABLE: BlockId = 40;
pub const FURNACE: BlockId = 41;
pub const CHEST: BlockId = 42;
pub const BED: BlockId = 43;
pub const BERRY_BUSH: BlockId = 44;
pub const TORCH: BlockId = 45;
pub const HEARTH: BlockId = 46;
// town building blocks
pub const STONE_BRICKS: BlockId = 47;
pub const PLASTER: BlockId = 48;
pub const RED_ROOF_TILES: BlockId = 49;
pub const SLATE_ROOF: BlockId = 50;
pub const THATCH: BlockId = 51;
pub const TIMBER_BEAM: BlockId = 52;
pub const FLAGSTONES: BlockId = 53;
pub const LEADED_WINDOW: BlockId = 54;
pub const WOODEN_DOOR: BlockId = 55;
pub const RED_BANNER: BlockId = 56;
pub const BLUE_BANNER: BlockId = 57;
pub const HAY_BALE: BlockId = 58;
pub const BARREL: BlockId = 59;
pub const CRATE: BlockId = 60;
pub const BOOKSHELF: BlockId = 61;
pub const ANVIL: BlockId = 62;
pub const BRAZIER: BlockId = 63;
pub const FLOWER_BED: BlockId = 64;

/// No Shade spawns or enters within this many blocks of a hearth.
pub const HEARTH_RADIUS: f32 = 16.0;

/// How a block is drawn and collides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// a full cube
    Full,
    /// a thin post in the middle of the cell (torch); not solid
    Post,
}

pub fn shape(b: BlockId) -> Shape {
    match b {
        TORCH => Shape::Post,
        _ => Shape::Full,
    }
}

/// Stable text key of a block, for content files: "Coal ore" is "coal_ore".
pub fn block_key(b: BlockId) -> String {
    block(b).name.to_lowercase().replace(' ', "_")
}

/// Block ID for a key, if any.
pub fn block_by_key(key: &str) -> Option<BlockId> {
    (0..BLOCK_COUNT).find(|b| block_key(*b) == key)
}
pub const CLAY: BlockId = 17;

/// Surface pattern drawn on a block, for blocks that need more than a colour.
pub fn pattern(b: BlockId) -> u8 {
    match b {
        COAL_ORE => 1,
        IRON_ORE => 2,
        EMBERSTONE => 3,
        PLANKS | 20 | 21 | CRAFTING_TABLE | CHEST | LOG | BIRCH_LOG => 4,
        BRICK | FURNACE | COBBLE | 19 => 5,
        BERRY_BUSH => 6,
        _ => 0,
    }
}

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
    def("Birch log", [0.88, 0.87, 0.82], Wood),
    def("Birch leaves", [0.45, 0.62, 0.28], Foliage),
    def("Pine leaves", [0.18, 0.36, 0.22], Foliage),
    def("Bedrock", [0.16, 0.16, 0.17], Stone),
    def("Coal ore", [0.48, 0.49, 0.51], Stone),
    def("Iron ore", [0.53, 0.52, 0.51], Stone),
    def("Crafting table", [0.62, 0.45, 0.27], Wood),
    def("Furnace", [0.38, 0.38, 0.40], Stone),
    def("Chest", [0.66, 0.47, 0.25], Wood),
    def("Bed", [0.78, 0.22, 0.24], Cloth),
    def("Berry bush", [0.22, 0.44, 0.20], Foliage),
    BlockDef {
        name: "Torch",
        color: [1.0, 0.80, 0.38],
        alpha: 1.0,
        emission: 13,
        material: Wood,
    },
    BlockDef {
        name: "Hearth",
        color: [0.66, 0.50, 0.40],
        alpha: 1.0,
        emission: 15,
        material: Stone,
    },
    BlockDef {
        name: "Stone bricks",
        color: [0.55, 0.54, 0.52],
        alpha: 1.0,
        emission: 0,
        material: Stone,
    },
    BlockDef {
        name: "Plaster",
        color: [0.90, 0.85, 0.74],
        alpha: 1.0,
        emission: 0,
        material: Stone,
    },
    BlockDef {
        name: "Red roof tiles",
        color: [0.62, 0.24, 0.18],
        alpha: 1.0,
        emission: 0,
        material: Stone,
    },
    BlockDef {
        name: "Slate roof",
        color: [0.30, 0.36, 0.46],
        alpha: 1.0,
        emission: 0,
        material: Stone,
    },
    BlockDef {
        name: "Thatch",
        color: [0.78, 0.64, 0.32],
        alpha: 1.0,
        emission: 0,
        material: Foliage,
    },
    BlockDef {
        name: "Timber beam",
        color: [0.32, 0.22, 0.14],
        alpha: 1.0,
        emission: 0,
        material: Wood,
    },
    BlockDef {
        name: "Flagstones",
        color: [0.60, 0.57, 0.50],
        alpha: 1.0,
        emission: 0,
        material: Stone,
    },
    BlockDef {
        name: "Leaded window",
        color: [0.55, 0.70, 0.80],
        alpha: 0.6,
        emission: 0,
        material: Glass,
    },
    BlockDef {
        name: "Wooden door",
        color: [0.45, 0.30, 0.18],
        alpha: 1.0,
        emission: 0,
        material: Wood,
    },
    BlockDef {
        name: "Red banner",
        color: [0.65, 0.12, 0.12],
        alpha: 1.0,
        emission: 0,
        material: Cloth,
    },
    BlockDef {
        name: "Blue banner",
        color: [0.15, 0.25, 0.62],
        alpha: 1.0,
        emission: 0,
        material: Cloth,
    },
    BlockDef {
        name: "Hay bale",
        color: [0.85, 0.72, 0.30],
        alpha: 1.0,
        emission: 0,
        material: Foliage,
    },
    BlockDef {
        name: "Barrel",
        color: [0.50, 0.34, 0.20],
        alpha: 1.0,
        emission: 0,
        material: Wood,
    },
    BlockDef {
        name: "Crate",
        color: [0.62, 0.46, 0.28],
        alpha: 1.0,
        emission: 0,
        material: Wood,
    },
    BlockDef {
        name: "Bookshelf",
        color: [0.45, 0.32, 0.20],
        alpha: 1.0,
        emission: 0,
        material: Wood,
    },
    BlockDef {
        name: "Anvil",
        color: [0.25, 0.25, 0.28],
        alpha: 1.0,
        emission: 0,
        material: Stone,
    },
    BlockDef {
        name: "Brazier",
        color: [0.40, 0.30, 0.25],
        alpha: 1.0,
        emission: 15,
        material: Stone,
    },
    BlockDef {
        name: "Flower bed",
        color: [0.40, 0.55, 0.25],
        alpha: 1.0,
        emission: 0,
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
    b != AIR && b != WATER && shape(b) == Shape::Full
}

/// True for blocks that hide the faces of their neighbours and block light.
pub fn is_opaque(b: BlockId) -> bool {
    b != AIR && !is_leaves(b) && block(b).alpha >= 1.0 && shape(b) == Shape::Full
}

/// Leaf blocks: drawn with their neighbours' faces showing through.
pub fn is_leaves(b: BlockId) -> bool {
    matches!(b, LEAVES | BIRCH_LEAVES | PINE_LEAVES)
}

/// Light level a block gives off.
pub fn emission(b: BlockId) -> u8 {
    block(b).emission
}

/// The kinds of tree the world grows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeKind {
    /// a broad oak, 12 to 16 m
    Oak,
    /// a slim birch, about 12 m
    Birch,
    /// a tall pine, 15 to 20 m
    Pine,
    /// a gaunt oak with sparse dark needles (the barrow wood)
    Dark,
    /// a dead snag with a broken limb or two, 8 to 10 m
    Dead,
}

/// One block of a tree, relative to the ground block it grows from.
#[derive(Clone, Copy, Debug)]
pub struct TreeBlock {
    pub at: [i32; 3],
    pub block: BlockId,
    /// trunk and limbs (replace what is there); leaves only fill air
    pub wood: bool,
}

/// A full-grown tree at life size (1 block = 1 m): the blocks of a tree
/// standing on the ground block at the origin. `r` varies it.
pub fn tree_shape(kind: TreeKind, r: u32) -> Vec<TreeBlock> {
    let mut out = Vec::new();
    let wood = |out: &mut Vec<TreeBlock>, at: [i32; 3], block: BlockId| {
        out.push(TreeBlock {
            at,
            block,
            wood: true,
        })
    };
    let leaf = |out: &mut Vec<TreeBlock>, at: [i32; 3], block: BlockId| {
        out.push(TreeBlock {
            at,
            block,
            wood: false,
        })
    };
    let jitter = |x: i32, y: i32, z: i32| (hash(x * 7 + y, z * 13 - y, r) % 100) as f32 / 100.0;
    match kind {
        TreeKind::Oak | TreeKind::Birch | TreeKind::Dark => {
            let (log, leaves, h, rad, ry) = match kind {
                TreeKind::Birch => (BIRCH_LOG, BIRCH_LEAVES, 11 + (r >> 8) as i32 % 3, 2.7, 4.2),
                TreeKind::Dark => (LOG, PINE_LEAVES, 11 + (r >> 8) as i32 % 3, 3.6, 3.6),
                _ => (
                    LOG,
                    LEAVES,
                    12 + (r >> 8) as i32 % 5,
                    3.8 + ((r >> 12) % 3) as f32 * 0.5,
                    3.8,
                ),
            };
            for y in 1..=h - 2 {
                wood(&mut out, [0, y, 0], log);
            }
            // the crown: a lumpy egg round the top of the trunk
            let cy = h as f32 - ry - 0.5;
            let ri = rad.ceil() as i32;
            for y in (cy - ry).floor() as i32..=h {
                for dz in -ri..=ri {
                    for dx in -ri..=ri {
                        let (a, b) = (
                            (dx * dx + dz * dz) as f32 / (rad * rad),
                            (y as f32 - cy) / ry,
                        );
                        let d = a + b * b;
                        let edge = 0.78 + 0.32 * jitter(dx, y, dz);
                        if d > edge || (dx == 0 && dz == 0 && y <= h - 2) {
                            continue;
                        }
                        // the dark wood keeps only a few of its needles
                        if kind == TreeKind::Dark && jitter(dz, y, dx) < 0.45 {
                            continue;
                        }
                        leaf(&mut out, [dx, y, dz], leaves);
                    }
                }
            }
        }
        TreeKind::Pine => {
            let h = 15 + (r >> 8) as i32 % 6;
            for y in 1..=h - 1 {
                wood(&mut out, [0, y, 0], LOG);
            }
            // a cone of tiers, widest low down
            let base = 4;
            for y in base..=h + 1 {
                let t = (h + 1 - y) as f32 / (h + 1 - base) as f32;
                let mut rad = 0.6 + 3.2 * t;
                if (h + 1 - y) % 3 == 0 {
                    rad *= 0.72;
                }
                let ri = rad.ceil() as i32;
                for dz in -ri..=ri {
                    for dx in -ri..=ri {
                        if (dx == 0 && dz == 0 && y < h)
                            || (dx * dx + dz * dz) as f32
                                > rad * rad * (0.85 + 0.3 * jitter(dx, y, dz))
                        {
                            continue;
                        }
                        leaf(&mut out, [dx, y, dz], PINE_LEAVES);
                    }
                }
            }
        }
        TreeKind::Dead => {
            let h = 8 + (r >> 8) as i32 % 3;
            for y in 1..=h {
                wood(&mut out, [0, y, 0], LOG);
            }
            for (n, (dx, dz)) in [(1, 0), (0, 1), (-1, 0), (0, -1)].into_iter().enumerate() {
                if (r >> (12 + n)) & 1 == 0 {
                    continue;
                }
                let y = h - 2 - (n as i32 % 3);
                wood(&mut out, [dx, y, dz], LOG);
                if (r >> (16 + n)) & 1 == 1 {
                    wood(&mut out, [2 * dx, y + 1, 2 * dz], LOG);
                }
            }
        }
    }
    out
}

/// The highest block of a tree above its ground.
pub fn tree_height(blocks: &[TreeBlock]) -> i32 {
    blocks.iter().map(|b| b.at[1]).max().unwrap_or(0)
}

/// The older save format: one run-length stream of every cell.
const MAGIC: &[u8; 5] = b"DYNW1";
/// Saves since the 2 km world: section by section.
const MAGIC2: &[u8; 5] = b"DYNW2";

fn varint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn read_varint(data: &[u8], p: &mut usize) -> Result<u64, String> {
    let mut n: u64 = 0;
    let mut shift = 0;
    loop {
        let byte = *data.get(*p).ok_or("truncated world data")?;
        *p += 1;
        n |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(n);
        }
        shift += 7;
        if shift > 49 {
            return Err("bad run length".into());
        }
    }
}

/// Edge of a section in cells: the world is streamed in sections of 16³.
pub const SECTION: i32 = 16;
/// Cells in one section.
pub const SECTION_CELLS: usize = 16 * 16 * 16;

/// A section position (section coordinates, not cells).
pub type SectionPos = [i32; 3];

/// The section that holds a cell.
pub fn section_of(p: [i32; 3]) -> SectionPos {
    [
        p[0].div_euclid(SECTION),
        p[1].div_euclid(SECTION),
        p[2].div_euclid(SECTION),
    ]
}

#[inline]
fn local_index(x: i32, y: i32, z: i32) -> usize {
    let (lx, ly, lz) = (
        x.rem_euclid(SECTION),
        y.rem_euclid(SECTION),
        z.rem_euclid(SECTION),
    );
    ((ly * SECTION + lz) * SECTION + lx) as usize
}

/// Widest world (blocks, east-west and north-south).
pub const MAX_SIDE: u32 = 4096;
/// Tallest world (blocks).
pub const MAX_HEIGHT: u32 = 512;

/// A section's cells packed: a palette and 1, 2 or 4 bits a cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Packed {
    palette: Vec<BlockId>,
    /// bits a cell: 1, 2 or 4
    bits: u8,
    data: Box<[u8]>,
}

impl Packed {
    /// None if the section holds more than 16 kinds of block.
    pub(crate) fn pack(cells: &[BlockId; SECTION_CELLS]) -> Option<Packed> {
        let mut palette: Vec<BlockId> = Vec::new();
        let mut slot = [u8::MAX; 256];
        for &b in cells.iter() {
            if slot[b as usize] == u8::MAX {
                if palette.len() == 16 {
                    return None;
                }
                slot[b as usize] = palette.len() as u8;
                palette.push(b);
            }
        }
        let bits: u8 = match palette.len() {
            0..=2 => 1,
            3..=4 => 2,
            _ => 4,
        };
        let per = (8 / bits) as usize;
        let mut data = vec![0u8; SECTION_CELLS / per].into_boxed_slice();
        for (i, &b) in cells.iter().enumerate() {
            data[i / per] |= slot[b as usize] << ((i % per) as u8 * bits);
        }
        Some(Packed {
            palette,
            bits,
            data,
        })
    }

    #[inline]
    pub(crate) fn get(&self, i: usize) -> BlockId {
        let per = (8 / self.bits) as usize;
        let mask = (1u8 << self.bits) - 1;
        let v = (self.data[i / per] >> ((i % per) as u8 * self.bits)) & mask;
        self.palette[v as usize]
    }

    pub(crate) fn bytes(&self) -> usize {
        self.data.len() + self.palette.len()
    }

    pub(crate) fn unpack(&self) -> [BlockId; SECTION_CELLS] {
        let mut out = [AIR; SECTION_CELLS];
        for (i, c) in out.iter_mut().enumerate() {
            *c = self.get(i);
        }
        out
    }
}

/// One section of the world. Sections are shared between copies of the
/// world (shards) until one of them changes (copy on write).
#[derive(Clone)]
enum Sect {
    /// streamed worlds: not arrived (or dropped)
    Unknown,
    /// every cell the same block (open air, solid rock): costs nothing
    Uniform(BlockId),
    /// a byte a cell: being built or edited
    Raw(Arc<[BlockId; SECTION_CELLS]>),
    /// at rest (`World::compact`): a palette and a few bits a cell
    Packed(Arc<Packed>),
}

impl Sect {
    #[inline]
    fn get(&self, i: usize) -> BlockId {
        match self {
            Sect::Unknown => AIR,
            Sect::Uniform(b) => *b,
            Sect::Raw(c) => c[i],
            Sect::Packed(p) => p.get(i),
        }
    }

    fn cells(&self) -> [BlockId; SECTION_CELLS] {
        match self {
            Sect::Unknown => [AIR; SECTION_CELLS],
            Sect::Uniform(b) => [*b; SECTION_CELLS],
            Sect::Raw(c) => **c,
            Sect::Packed(p) => p.unpack(),
        }
    }

    /// The smallest form of these cells.
    fn compacted(cells: &[BlockId; SECTION_CELLS]) -> Sect {
        let first = cells[0];
        if cells.iter().all(|&b| b == first) {
            return Sect::Uniform(first);
        }
        match Packed::pack(cells) {
            Some(p) => Sect::Packed(Arc::new(p)),
            None => Sect::Raw(Arc::new(*cells)),
        }
    }

    /// Bytes of cells held.
    fn bytes(&self) -> usize {
        match self {
            Sect::Unknown | Sect::Uniform(_) => 0,
            Sect::Raw(_) => SECTION_CELLS,
            Sect::Packed(p) => p.bytes(),
        }
    }

    /// The same storage as another (shared, or the same uniform block).
    fn same(&self, other: &Sect) -> bool {
        match (self, other) {
            (Sect::Uniform(a), Sect::Uniform(b)) => a == b,
            (Sect::Raw(a), Sect::Raw(b)) => Arc::ptr_eq(a, b),
            (Sect::Packed(a), Sect::Packed(b)) => Arc::ptr_eq(a, b),
            (Sect::Unknown, Sect::Unknown) => true,
            _ => false,
        }
    }
}

/// A section decoded and packed, ready to store (`World::prepare_section`);
/// cheap to clone (copies share it).
#[derive(Clone)]
pub struct SectionData(Sect);

/// The block world: 1 m cells in sections of 16³. A section that is all one
/// block (the sky, solid rock) costs nothing; others are packed with a
/// palette once built (`compact`), and copies of the world (shards) share
/// every section until they change it. Clients hold a streamed world: only
/// the sections that arrived (see the crate docs).
#[derive(Clone)]
pub struct World {
    pub size: [u32; 3],
    dims: [i32; 3],
    secs: Vec<Sect>,
    /// a streamed world (clients): sections start unknown
    streamed: bool,
    /// bumped whenever a section arrives or leaves (streamed worlds)
    rev: u64,
    /// a count per section, bumped by every change in it (so the server can
    /// keep sections encoded until they change)
    revs: Vec<u32>,
}

/// Run-length encode cells: (block, varint count) pairs.
pub(crate) fn rle(cells: impl Iterator<Item = BlockId>, out: &mut Vec<u8>) {
    let mut run: Option<(BlockId, u64)> = None;
    let flush = |out: &mut Vec<u8>, b: BlockId, n: u64| {
        out.push(b);
        let mut v = n;
        loop {
            let byte = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(byte);
                break;
            }
            out.push(byte | 0x80);
        }
    };
    for b in cells {
        run = match run {
            Some((rb, n)) if rb == b => Some((rb, n + 1)),
            Some((rb, n)) => {
                flush(out, rb, n);
                Some((b, 1))
            }
            None => Some((b, 1)),
        };
    }
    if let Some((b, n)) = run {
        flush(out, b, n);
    }
}

/// Decode (block, varint count) pairs into `out`, which must come out exactly full.
pub(crate) fn unrle(data: &[u8], out: &mut Vec<BlockId>, total: usize) -> Result<(), String> {
    let mut p = 0;
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
        if out.len() + n as usize > total {
            return Err("world data is longer than the world".into());
        }
        out.resize(out.len() + n as usize, b);
    }
    if out.len() != total {
        return Err("world data is shorter than the world".into());
    }
    Ok(())
}

impl World {
    fn with(size: [u32; 3], fill: Sect, streamed: bool) -> Self {
        let dims = Self::dims_of(size);
        let n = (dims[0] * dims[1] * dims[2]) as usize;
        World {
            size,
            dims,
            secs: vec![fill; n],
            streamed,
            rev: 0,
            revs: vec![0; n],
        }
    }

    /// A world of air.
    pub fn empty(size: [u32; 3]) -> Self {
        Self::with(size, Sect::Uniform(AIR), false)
    }

    /// A streamed world with nothing in it yet: every cell is unknown until
    /// its section arrives (`put_section`).
    pub fn sparse(size: [u32; 3]) -> Self {
        Self::with(size, Sect::Unknown, true)
    }

    fn dims_of(size: [u32; 3]) -> [i32; 3] {
        [
            (size[0] as i32 + SECTION - 1) / SECTION,
            (size[1] as i32 + SECTION - 1) / SECTION,
            (size[2] as i32 + SECTION - 1) / SECTION,
        ]
    }

    /// How many sections the world has along each axis.
    pub fn section_dims(&self) -> [i32; 3] {
        self.dims
    }

    /// A streamed (sparse) world, as clients hold it.
    pub fn is_sparse(&self) -> bool {
        self.streamed
    }

    #[inline]
    fn sect_index(&self, s: SectionPos) -> Option<usize> {
        let d = self.dims;
        if s[0] < 0 || s[1] < 0 || s[2] < 0 || s[0] >= d[0] || s[1] >= d[1] || s[2] >= d[2] {
            return None;
        }
        Some(((s[1] * d[2] + s[2]) * d[0] + s[0]) as usize)
    }

    /// The section holding a cell inside the world, and the cell's place in it.
    #[inline]
    fn locate(&self, x: i32, y: i32, z: i32) -> Option<(usize, usize)> {
        let [sx, sy, sz] = self.size;
        if x < 0 || y < 0 || z < 0 || x >= sx as i32 || y >= sy as i32 || z >= sz as i32 {
            return None;
        }
        let d = self.dims;
        let i = (((y / SECTION) * d[2] + z / SECTION) * d[0] + x / SECTION) as usize;
        Some((i, local_index(x, y, z)))
    }

    /// Whether a section is in the world (and, for a streamed world, has arrived).
    pub fn has_section(&self, s: SectionPos) -> bool {
        self.sect_index(s)
            .is_some_and(|i| !matches!(self.secs[i], Sect::Unknown))
    }

    /// Whether a cell's content is known. Outside the world it is (air
    /// above, stone below); in a streamed world, only once its section is here.
    #[inline]
    pub fn known(&self, x: i32, y: i32, z: i32) -> bool {
        !self.streamed
            || self
                .locate(x, y, z)
                .is_none_or(|(i, _)| !matches!(self.secs[i], Sect::Unknown))
    }

    /// Solid for movement: a solid block, or a cell that has not arrived
    /// yet (nobody walks into or falls through the unknown).
    #[inline]
    pub fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        is_solid(self.get(x, y, z)) || !self.known(x, y, z)
    }

    fn pos_of(&self, i: usize) -> SectionPos {
        let d = self.dims;
        let i = i as i32;
        [i % d[0], i / (d[0] * d[2]), (i / d[0]) % d[2]]
    }

    /// The sections that have arrived (every section, for a whole world).
    pub fn held_list(&self) -> Vec<SectionPos> {
        (0..self.secs.len())
            .filter(|&i| !matches!(self.secs[i], Sect::Unknown))
            .map(|i| self.pos_of(i))
            .collect()
    }

    /// Changes whenever a section arrives or is dropped (for views that
    /// redraw when the world fills in, like the minimap).
    pub fn revision(&self) -> u64 {
        self.rev
    }

    /// Sections held, and the bytes of cells they take.
    pub fn held_sections(&self) -> (usize, usize) {
        self.secs.iter().fold((0, 0), |(n, b), s| match s {
            Sect::Unknown => (n, b),
            s => (n + 1, b + s.bytes()),
        })
    }

    /// Bytes this world takes in memory: its cells and its table of
    /// sections (sections shared with another copy are counted in each).
    pub fn memory_bytes(&self) -> usize {
        self.held_sections().1
            + self.secs.len() * (std::mem::size_of::<Sect>() + std::mem::size_of::<u32>())
    }

    /// Pack every section built or edited since the last call into its
    /// smallest form (all one block: nothing; few kinds: a palette and 1-4
    /// bits a cell). Call it after building a world; edits unpack the
    /// sections they touch.
    pub fn compact(&mut self) {
        for s in &mut self.secs {
            if let Sect::Raw(c) = s {
                *s = Sect::compacted(c);
            }
        }
    }

    /// The highest block in a column that is not air, and its height, or
    /// None (open air down to the bottom, or not arrived). Sections of open
    /// air are skipped whole.
    pub fn column_top(&self, x: i32, z: i32) -> Option<(i32, BlockId)> {
        if !self.in_bounds([x, 0, z]) {
            return None;
        }
        let d = self.dims;
        let (sx, sz) = (x / SECTION, z / SECTION);
        for sy in (0..d[1]).rev() {
            let i = ((sy * d[2] + sz) * d[0] + sx) as usize;
            match &self.secs[i] {
                Sect::Unknown | Sect::Uniform(AIR) => continue,
                s => {
                    let top = (sy * SECTION + SECTION - 1).min(self.size[1] as i32 - 1);
                    for y in (sy * SECTION..=top).rev() {
                        let b = s.get(local_index(x, y, z));
                        if b != AIR {
                            return Some((y, b));
                        }
                    }
                }
            }
        }
        None
    }

    /// Every cell holding this block (fast: sections without it are skipped).
    pub fn positions_of(&self, b: BlockId) -> Vec<[i32; 3]> {
        let mut out = Vec::new();
        for (i, s) in self.secs.iter().enumerate() {
            let maybe = match s {
                Sect::Unknown => false,
                Sect::Uniform(u) => *u == b,
                Sect::Raw(_) => true,
                Sect::Packed(p) => p.palette.contains(&b),
            };
            if !maybe {
                continue;
            }
            let [sx, sy, sz] = self.pos_of(i);
            for k in 0..SECTION_CELLS {
                if s.get(k) == b {
                    let k = k as i32;
                    let p = [
                        sx * SECTION + k % SECTION,
                        sy * SECTION + k / (SECTION * SECTION),
                        sz * SECTION + (k / SECTION) % SECTION,
                    ];
                    if self.in_bounds(p) {
                        out.push(p);
                    }
                }
            }
        }
        out.sort_by_key(|p| (p[0], p[2], p[1]));
        out
    }

    /// The block a section is made of, if it is all one (the sky, solid
    /// rock); None for a mixed section, or one not arrived.
    pub fn section_uniform(&self, s: SectionPos) -> Option<BlockId> {
        match self.sect_index(s).map(|i| &self.secs[i]) {
            Some(Sect::Uniform(b)) => Some(*b),
            _ => None,
        }
    }

    /// How many times a section has changed (for caches of encoded sections).
    pub fn section_rev(&self, s: SectionPos) -> u32 {
        self.sect_index(s).map_or(0, |i| self.revs[i])
    }

    /// A section's cells, those past the edge of the world as air.
    fn section_cells(&self, s: SectionPos) -> [BlockId; SECTION_CELLS] {
        let Some(i) = self.sect_index(s) else {
            return [AIR; SECTION_CELLS];
        };
        let mut c = self.secs[i].cells();
        let [wx, wy, wz] = self.size.map(|v| v as i32);
        let (x0, y0, z0) = (s[0] * SECTION, s[1] * SECTION, s[2] * SECTION);
        if x0 + SECTION > wx || y0 + SECTION > wy || z0 + SECTION > wz {
            for (k, v) in c.iter_mut().enumerate() {
                let k = k as i32;
                let (x, y, z) = (
                    x0 + k % SECTION,
                    y0 + k / (SECTION * SECTION),
                    z0 + (k / SECTION) % SECTION,
                );
                if x >= wx || y >= wy || z >= wz {
                    *v = AIR;
                }
            }
        }
        c
    }

    /// The section's cells, run-length encoded (for streaming). Cells past
    /// the edge of the world are air.
    pub fn encode_section(&self, s: SectionPos) -> Vec<u8> {
        let mut out = Vec::new();
        match self.sect_index(s).map(|i| &self.secs[i]) {
            Some(Sect::Uniform(b)) if self.whole_section(s) => {
                rle(std::iter::repeat_n(*b, SECTION_CELLS), &mut out)
            }
            _ => rle(self.section_cells(s).into_iter(), &mut out),
        }
        out
    }

    fn whole_section(&self, s: SectionPos) -> bool {
        let [wx, wy, wz] = self.size.map(|v| v as i32);
        (s[0] + 1) * SECTION <= wx && (s[1] + 1) * SECTION <= wy && (s[2] + 1) * SECTION <= wz
    }

    /// Store a section (from `encode_section`).
    pub fn put_section(&mut self, s: SectionPos, data: &[u8]) -> Result<(), String> {
        let prepared = Self::prepare_section(data)?;
        self.put_prepared(s, prepared)
    }

    /// Decode and pack a section (from `encode_section`) ahead of storing
    /// it: on another thread, and once for several copies of the world (the
    /// copies share it).
    pub fn prepare_section(data: &[u8]) -> Result<SectionData, String> {
        // all one block (the sky, solid rock: most of a tall world): one run
        let mut p = 1;
        if data.len() > 1
            && data[0] < BLOCK_COUNT
            && read_varint(data, &mut p) == Ok(SECTION_CELLS as u64)
            && p == data.len()
        {
            return Ok(SectionData(Sect::Uniform(data[0])));
        }
        let mut cells = Vec::with_capacity(SECTION_CELLS);
        unrle(data, &mut cells, SECTION_CELLS)?;
        let mut c = [AIR; SECTION_CELLS];
        c.copy_from_slice(&cells);
        Ok(SectionData(Sect::compacted(&c)))
    }

    /// Store a section prepared by `prepare_section`.
    pub fn put_prepared(&mut self, s: SectionPos, data: SectionData) -> Result<(), String> {
        let i = self
            .sect_index(s)
            .ok_or_else(|| format!("section {s:?} is outside the world"))?;
        self.secs[i] = data.0;
        self.revs[i] = self.revs[i].wrapping_add(1);
        self.rev += 1;
        Ok(())
    }

    /// Forget a section (streamed worlds): its cells become unknown again.
    pub fn drop_section(&mut self, s: SectionPos) {
        if self.streamed
            && let Some(i) = self.sect_index(s)
            && !matches!(self.secs[i], Sect::Unknown)
        {
            self.secs[i] = Sect::Unknown;
            self.rev += 1;
        }
    }

    /// Cells of a section where `other` (the same size) differs from this
    /// world, with `other`'s value.
    pub fn diff_section(&self, other: &World, s: SectionPos) -> Vec<([i32; 3], BlockId)> {
        let mut out = Vec::new();
        if self.size != other.size || self.sect_index(s).is_none() {
            return out;
        }
        let (x0, y0, z0) = (s[0] * SECTION, s[1] * SECTION, s[2] * SECTION);
        for y in y0..(y0 + SECTION).min(self.size[1] as i32) {
            for z in z0..(z0 + SECTION).min(self.size[2] as i32) {
                for x in x0..(x0 + SECTION).min(self.size[0] as i32) {
                    let b = other.get(x, y, z);
                    if self.get(x, y, z) != b {
                        out.push(([x, y, z], b));
                    }
                }
            }
        }
        out
    }

    pub fn in_bounds(&self, p: [i32; 3]) -> bool {
        self.locate(p[0], p[1], p[2]).is_some()
    }

    /// Blocks where `other` (the same size) differs from this world, with
    /// `other`'s value. Fast when one is a copy of the other: sections they
    /// still share are skipped.
    pub fn diff(&self, other: &World) -> Vec<([i32; 3], BlockId)> {
        let mut out = Vec::new();
        if self.size != other.size {
            return out;
        }
        for (i, (a, b)) in self.secs.iter().zip(&other.secs).enumerate() {
            if !a.same(b) {
                out.extend(self.diff_section(other, self.pos_of(i)));
            }
        }
        out.sort_by_key(|(p, _)| (p[1], p[2], p[0]));
        out
    }

    /// Block at a position. Outside the world: air above, stone below.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        match self.locate(x, y, z) {
            Some((i, l)) => self.secs[i].get(l),
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
    /// In a streamed world, a cell whose section has not arrived is not
    /// set (the section brings the cell when it comes).
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: BlockId) -> bool {
        let Some((i, l)) = self.locate(x, y, z) else {
            return false;
        };
        let sect = &mut self.secs[i];
        let changed = match sect {
            Sect::Unknown => return false,
            Sect::Uniform(u) if *u == b => false,
            Sect::Uniform(u) => {
                let mut c = [*u; SECTION_CELLS];
                c[l] = b;
                *sect = Sect::Raw(Arc::new(c));
                true
            }
            Sect::Raw(c) => {
                if c[l] == b {
                    false
                } else {
                    Arc::make_mut(c)[l] = b;
                    true
                }
            }
            Sect::Packed(p) => {
                if p.get(l) == b {
                    false
                } else {
                    let mut c = p.unpack();
                    c[l] = b;
                    *sect = Sect::Raw(Arc::new(c));
                    true
                }
            }
        };
        if changed {
            self.revs[i] = self.revs[i].wrapping_add(1);
        }
        true
    }

    /// Fill a row of cells (x0..x1, one y and z) with a block: loading.
    fn fill_row(&mut self, y: i32, z: i32, x0: i32, x1: i32, b: BlockId) {
        let mut x = x0;
        while x < x1 {
            let end = ((x / SECTION + 1) * SECTION).min(x1);
            let Some((i, l)) = self.locate(x, y, z) else {
                return;
            };
            let sect = &mut self.secs[i];
            if !matches!(sect, Sect::Uniform(u) if *u == b) {
                if !matches!(sect, Sect::Raw(_)) {
                    *sect = Sect::Raw(Arc::new(sect.cells()));
                }
                if let Sect::Raw(c) = sect {
                    Arc::make_mut(c)[l..l + (end - x) as usize].fill(b);
                }
            }
            x = end;
        }
    }

    /// Generate the default terrain: meadows, forests of oak and birch, pine on
    /// the hills, snowy mountain peaks, lakes with sandy shores, glowshrooms and
    /// emberstone.
    pub fn generate(size: [u32; 3], seed: u32) -> Self {
        let mut w = World::empty(size);
        let [sx, sy, sz] = size;
        let water_level = 24;
        let snow_line = 58;
        for z in 0..sz as i32 {
            for x in 0..sx as i32 {
                let h = height(x, z, seed).clamp(4, sy as i32 - 16);
                let beach = h <= water_level + 1;
                for y in 0..=h {
                    let b = if y == h {
                        if beach {
                            SAND
                        } else if h >= snow_line + (hash(x, z, seed) % 4) as i32 {
                            SNOW
                        } else if h > snow_line - 8 {
                            STONE
                        } else {
                            GRASS
                        }
                    } else if y > h - 4 {
                        if beach {
                            SAND
                        } else if h > snow_line - 8 {
                            STONE
                        } else {
                            DIRT
                        }
                    } else {
                        STONE
                    };
                    w.set(x, y, z, b);
                }
                for y in h + 1..=water_level {
                    w.set(x, y, z, WATER);
                }
                if !beach
                    && w.get(x, h, z) == GRASS
                    && hash(x, z, seed ^ 0x9105).is_multiple_of(420)
                {
                    w.set(x, h + 1, z, GLOWSHROOM);
                }
            }
        }
        w.carve_and_ore(seed, water_level);
        // trees: density follows the forest noise; kind follows height
        for z in 2..sz as i32 - 2 {
            for x in 2..sx as i32 - 2 {
                let r = hash(x, z, seed ^ 0x7ee5);
                let f = forest(x, z, seed);
                // life-size trees stand further apart than the old small ones
                let chance = if f > 0.62 {
                    14
                } else if f > 0.45 {
                    55
                } else {
                    500
                };
                if !r.is_multiple_of(chance as u32) {
                    continue;
                }
                let h = w.surface(x, z);
                if w.get(x, h, z) != GRASS {
                    continue;
                }
                // keep trees apart
                if (-3..=3).any(|dz| {
                    (-3..=3).any(|dx| matches!(w.get(x + dx, h + 1, z + dz), LOG | BIRCH_LOG))
                }) {
                    continue;
                }
                let kind = if h > snow_line - 16 {
                    TreeKind::Pine
                } else if (r >> 12).is_multiple_of(3) {
                    TreeKind::Birch
                } else {
                    TreeKind::Oak
                };
                w.grow_tree(x, h, z, kind, r);
            }
        }
        // berry bushes at forest edges
        for z in 1..sz as i32 - 1 {
            for x in 1..sx as i32 - 1 {
                let f = forest(x, z, seed);
                if f > 0.4 && f < 0.6 && hash(x, z, seed ^ 0xbe77).is_multiple_of(140) {
                    let h = w.surface(x, z);
                    if w.get(x, h, z) == GRASS {
                        w.set(x, h + 1, z, BERRY_BUSH);
                    }
                }
            }
        }
        w.compact();
        w
    }

    /// Caves, ore veins, clay under lakes and a bedrock floor.
    fn carve_and_ore(&mut self, seed: u32, water_level: i32) {
        let [sx, sy, sz] = [
            self.size[0] as i32,
            self.size[1] as i32,
            self.size[2] as i32,
        ];
        let surf: Vec<i32> = (0..sz)
            .flat_map(|z| (0..sx).map(move |x| (x, z)))
            .map(|(x, z)| self.surface(x, z))
            .collect();
        for z in 0..sz {
            for x in 0..sx {
                let h = surf[(z * sx + x) as usize];
                for y in 1..sy.min(h) {
                    let b = self.get(x, y, z);
                    if b != STONE && b != DIRT {
                        continue;
                    }
                    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                    // spaghetti tunnels where two noise fields cross their middle value
                    let a = value_noise3(fx / 22.0, fy / 14.0, fz / 22.0, seed + 20) - 0.5;
                    let c = value_noise3(fx / 22.0, fy / 14.0, fz / 22.0, seed + 21) - 0.5;
                    let tunnel = a * a + c * c < 0.0028;
                    // larger caverns deep down
                    let cavern =
                        y < 26 && value_noise3(fx / 30.0, fy / 16.0, fz / 30.0, seed + 22) > 0.73;
                    // keep lake beds and a thin crust closed
                    let under_water = h <= water_level + 1 && y >= h - 3;
                    if (tunnel || cavern) && !under_water && y > 2 {
                        self.set(x, y, z, AIR);
                        continue;
                    }
                    if b == STONE {
                        let r = hash3(x, y, z, seed ^ 0x0e);
                        if y < h - 3 && r % 1000 < 6 {
                            self.set(x, y, z, COAL_ORE);
                        } else if y < 44 && y < h - 5 && r % 1000 >= 996 {
                            self.set(x, y, z, IRON_ORE);
                        } else if y < 18 && r % 4000 == 7 {
                            self.set(x, y, z, EMBERSTONE);
                        }
                    }
                }
                // grow ore cells into small veins
                // clay patches under shallow water
                if h <= water_level
                    && hash(x, z, seed ^ 0xc1a7).is_multiple_of(7)
                    && self.get(x, h, z) == SAND
                {
                    self.set(x, h, z, CLAY);
                }
                self.set(x, 0, z, BEDROCK);
                if hash(x, z, seed ^ 0xbed).is_multiple_of(2) {
                    self.set(x, 1, z, BEDROCK);
                }
            }
        }
        // grow veins: every ore cell spreads into a few stone neighbours
        let mut grow = Vec::new();
        for y in 1..sy {
            for z in 0..sz {
                for x in 0..sx {
                    let b = self.get(x, y, z);
                    if matches!(b, COAL_ORE | IRON_ORE) {
                        grow.push((x, y, z, b));
                    }
                }
            }
        }
        for (x, y, z, b) in grow {
            let r = hash3(x, y, z, seed ^ 0x7e1);
            for (i, (dx, dy, dz)) in [
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, 0, 1),
                (0, 0, -1),
                (0, -1, 0),
            ]
            .into_iter()
            .enumerate()
            {
                if (r >> i) & 1 == 1 && self.get(x + dx, y + dy, z + dz) == STONE {
                    self.set(x + dx, y + dy, z + dz, b);
                }
            }
        }
    }

    fn put_if_air(&mut self, x: i32, y: i32, z: i32, b: BlockId) {
        if self.get(x, y, z) == AIR {
            self.set(x, y, z, b);
        }
    }

    /// Grow a tree on the ground block (x, h, z), if it fits under the sky.
    fn grow_tree(&mut self, x: i32, h: i32, z: i32, kind: TreeKind, r: u32) {
        let shape = tree_shape(kind, r);
        if h + tree_height(&shape) >= self.size[1] as i32 - 1 {
            return;
        }
        for b in shape {
            let [dx, dy, dz] = b.at;
            if b.wood {
                self.set(x + dx, h + dy, z + dz, b.block);
            } else {
                self.put_if_air(x + dx, h + dy, z + dz, b.block);
            }
        }
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

    /// The height of the ground at (x, z): the top solid block that is not
    /// part of a tree (leaves and trunks), so nothing stands on a canopy.
    pub fn ground(&self, x: i32, z: i32) -> i32 {
        for y in (0..self.size[1] as i32).rev() {
            let b = self.get(x, y, z);
            if is_solid(b) && !is_leaves(b) && b != LOG && b != BIRCH_LOG {
                return y;
            }
        }
        -1
    }

    /// Find a block within `r` columns of (x, z), at any height.
    pub fn find_near(&self, b: BlockId, x: i32, z: i32, r: i32) -> Option<[i32; 3]> {
        for dz in -r..=r {
            for dx in -r..=r {
                for y in 0..self.size[1] as i32 {
                    if self.get(x + dx, y, z + dz) == b {
                        return Some([x + dx, y, z + dz]);
                    }
                }
            }
        }
        None
    }

    /// Build a ruined hearth at the spawn if there is none: a safe, lit
    /// meeting point. Returns the hearth position.
    ///
    /// `known` is the spawn hearth the server remembers. It is restored if it
    /// went missing. With none remembered, a hearth near the centre is adopted
    /// (worlds from before the server remembered it), or a new ruin is built.
    pub fn ensure_spawn_hearth(&mut self, known: Option<[i32; 3]>) -> [i32; 3] {
        if let Some(p) = known {
            let here = self.get(p[0], p[1], p[2]);
            if here == HEARTH {
                return p;
            }
            if here == AIR {
                self.set(p[0], p[1], p[2], HEARTH);
                return p;
            }
        } else {
            let (cx, cz) = (self.size[0] as i32 / 2, self.size[2] as i32 / 2);
            if let Some(p) = self.find_near(HEARTH, cx, cz, 24) {
                return p;
            }
        }
        self.build_spawn_hearth()
    }

    fn build_spawn_hearth(&mut self) -> [i32; 3] {
        let s = self.open_spawn();
        let (x, z) = (s[0].floor() as i32, s[2].floor() as i32);
        let y = s[1].floor() as i32 - 1;
        // clear a glade (nature only, never a player's building) and lay an old stone floor
        let natural = |b: BlockId| {
            matches!(
                b,
                GRASS | DIRT | SAND | SNOW | STONE | LEAVES | LOG | 33 | 34 | 35 | 36 | 44
            )
        };
        for dz in -8i32..=8 {
            for dx in -8i32..=8 {
                let r2 = dx * dx + dz * dz;
                if r2 > 64 {
                    continue;
                }
                for yy in y + 1..y + 18 {
                    let b = self.get(x + dx, yy, z + dz);
                    // whole trees go; the ground is levelled close in
                    let tree = matches!(b, LEAVES | LOG | 34 | 35 | 36);
                    if natural(b) && (tree || r2 <= 30) {
                        self.set(x + dx, yy, z + dz, AIR);
                    }
                }
                if r2 <= 30 && natural(self.get(x + dx, y, z + dz)) {
                    self.set(x + dx, y, z + dz, GRASS);
                }
                if r2 <= 20
                    && (self.get(x + dx, y, z + dz) == AIR || natural(self.get(x + dx, y, z + dz)))
                {
                    let b = if hash(x + dx, z + dz, 7).is_multiple_of(3) {
                        19
                    } else {
                        COBBLE
                    };
                    self.set(x + dx, y, z + dz, b);
                    for yy in (y - 3)..y {
                        if self.get(x + dx, yy, z + dz) == AIR {
                            self.set(x + dx, yy, z + dz, DIRT);
                        }
                    }
                }
            }
        }
        // broken pillars around the hearth
        for (i, (dx, dz)) in [(-3, -3), (3, -3), (-3, 3), (3, 3)].into_iter().enumerate() {
            let h = [3, 1, 2, 4][i];
            for yy in 1..=h {
                let here = self.get(x + dx, y + yy, z + dz);
                if here == AIR || natural(here) {
                    let b = if yy == h && i % 2 == 0 { 19 } else { COBBLE };
                    self.set(x + dx, y + yy, z + dz, b);
                }
            }
        }
        // the hearth sits on the floor, ringed by low stones
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if !natural(self.get(x + dx, y + 1, z + dz)) {
                continue;
            }
            self.set(x + dx, y + 1, z + dz, AIR);
        }
        self.set(x, y + 1, z, HEARTH);
        [x, y + 1, z]
    }

    /// Where new players appear: beside the spawn hearth if there is one.
    pub fn spawn(&self) -> [f32; 3] {
        let (cx, cz) = (self.size[0] as i32 / 2, self.size[2] as i32 / 2);
        if let Some(h) = self.find_near(HEARTH, cx, cz, 24) {
            return [h[0] as f32 + 2.5, h[1] as f32 + 0.01, h[2] as f32 + 0.5];
        }
        self.open_spawn()
    }

    /// A free standing place beside a hearth.
    pub fn spawn_near(&self, h: [i32; 3]) -> [f32; 3] {
        let free = |x: i32, y: i32, z: i32| {
            is_solid(self.get(x, y - 1, z))
                && !is_solid(self.get(x, y, z))
                && !is_solid(self.get(x, y + 1, z))
        };
        for r in 2..6i32 {
            for (dx, dz) in [
                (r, 0),
                (0, r),
                (-r, 0),
                (0, -r),
                (r, r),
                (-r, r),
                (r, -r),
                (-r, -r),
            ] {
                for dy in [0, 1, -1, 2, -2, 3] {
                    let (x, y, z) = (h[0] + dx, h[1] + dy, h[2] + dz);
                    if free(x, y, z) {
                        return [x as f32 + 0.5, y as f32 + 0.01, z as f32 + 0.5];
                    }
                }
            }
        }
        self.open_spawn()
    }

    /// Open grass near the centre of the world.
    fn open_spawn(&self) -> [f32; 3] {
        let (cx, cz) = (self.size[0] as i32 / 2, self.size[2] as i32 / 2);
        // look for open grass near the centre: no leaves overhead, no water
        for r in 0..40i32 {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let (x, z) = (cx + dx, cz + dz);
                    let y = self.surface(x, z);
                    if y > 0
                        && self.get(x, y, z) == GRASS
                        && (y + 1..self.size[1] as i32).all(|yy| self.get(x, yy, z) == AIR)
                    {
                        return [x as f32 + 0.5, y as f32 + 1.01, z as f32 + 0.5];
                    }
                }
            }
        }
        let y = self.surface(cx, cz).max(self.surface_water(cx, cz));
        [cx as f32 + 0.5, y as f32 + 1.01, cz as f32 + 0.5]
    }

    fn surface_water(&self, x: i32, z: i32) -> i32 {
        for y in (0..self.size[1] as i32).rev() {
            if self.get(x, y, z) != AIR {
                return y;
            }
        }
        -1
    }

    /// Encode the world for saving (`DYNW2`): the size, then each section
    /// in turn: all one block (tag 0, the block), or its cells run-length
    /// encoded (tag 1, a varint length, (block, varint count) pairs).
    /// Unknown sections of a streamed world are written as air.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 * 1024);
        out.extend_from_slice(MAGIC2);
        for s in self.size {
            out.extend_from_slice(&s.to_le_bytes());
        }
        let mut cells = Vec::new();
        for i in 0..self.secs.len() {
            let pos = self.pos_of(i);
            match &self.secs[i] {
                Sect::Unknown => out.extend_from_slice(&[0, AIR]),
                Sect::Uniform(b) if self.whole_section(pos) => out.extend_from_slice(&[0, *b]),
                _ => {
                    cells.clear();
                    rle(self.section_cells(pos).into_iter(), &mut cells);
                    out.push(1);
                    varint(cells.len() as u64, &mut out);
                    out.extend_from_slice(&cells);
                }
            }
        }
        out
    }

    /// Decode a saved world: `DYNW2` (sections), or the older `DYNW1` (every
    /// cell in one run-length stream), so old saves load. Rejects malformed
    /// or oversized input. The world comes back packed (`compact`).
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        if data.len() < 17 || (&data[..5] != MAGIC && &data[..5] != MAGIC2) {
            return Err("not a Dyno world".into());
        }
        let rd = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
        let size = [rd(5), rd(9), rd(13)];
        if size.contains(&0) || size[0] > MAX_SIDE || size[2] > MAX_SIDE || size[1] > MAX_HEIGHT {
            return Err(format!("world size {size:?} is not supported"));
        }
        let mut w = World::empty(size);
        let body = &data[17..];
        if &data[..5] == MAGIC {
            w.decode_v1(body)?;
        } else {
            let mut p = 0;
            let mut cells = Vec::with_capacity(SECTION_CELLS);
            for i in 0..w.secs.len() {
                let tag = *body.get(p).ok_or("truncated world data")?;
                p += 1;
                w.secs[i] = match tag {
                    0 => {
                        let b = *body.get(p).ok_or("truncated world data")?;
                        p += 1;
                        if b >= BLOCK_COUNT {
                            return Err(format!("unknown block id {b}"));
                        }
                        Sect::Uniform(b)
                    }
                    1 => {
                        let n = read_varint(body, &mut p)? as usize;
                        let run = body.get(p..p + n).ok_or("truncated world data")?;
                        p += n;
                        cells.clear();
                        unrle(run, &mut cells, SECTION_CELLS)?;
                        let mut c = [AIR; SECTION_CELLS];
                        c.copy_from_slice(&cells);
                        Sect::compacted(&c)
                    }
                    _ => return Err(format!("bad section tag {tag}")),
                };
            }
            if p != body.len() {
                return Err("world data is longer than the world".into());
            }
        }
        w.compact();
        Ok(w)
    }

    /// The older format: every cell, y then z then x, in one run-length stream.
    fn decode_v1(&mut self, data: &[u8]) -> Result<(), String> {
        let [sx, sy, sz] = self.size.map(|v| v as u64);
        let total = sx * sy * sz;
        let (mut p, mut at) = (0usize, 0u64);
        while p < data.len() {
            let b = data[p];
            if b >= BLOCK_COUNT {
                return Err(format!("unknown block id {b}"));
            }
            p += 1;
            let n = read_varint(data, &mut p)?;
            if at + n > total {
                return Err("world data is longer than the world".into());
            }
            // the run, a row at a time
            let mut left = n;
            while left > 0 {
                let x = at % sx;
                let z = (at / sx) % sz;
                let y = at / (sx * sz);
                let len = left.min(sx - x);
                if b != AIR {
                    self.fill_row(y as i32, z as i32, x as i32, (x + len) as i32, b);
                }
                at += len;
                left -= len;
            }
        }
        if at != total {
            return Err("world data is shorter than the world".into());
        }
        Ok(())
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

fn hash3(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    hash(x ^ y.wrapping_mul(0x632b_e5ab_u32 as i32), z, seed)
}

fn value_noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (fx, fy, fz) = (x - xi as f32, y - yi as f32, z - zi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v, w) = (s(fx), s(fy), s(fz));
    let g =
        |a: i32, b: i32, c: i32| (hash3(xi + a, yi + b, zi + c, seed) & 0xffff) as f32 / 65535.0;
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = l(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = l(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = l(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = l(g(0, 1, 1), g(1, 1, 1), u);
    l(l(x00, x10, v), l(x01, x11, v), w)
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

/// Terrain height: rolling hills, plus ridged mountains in some regions.
fn height(x: i32, z: i32, seed: u32) -> i32 {
    let (fx, fz) = (x as f32, z as f32);
    let n = value_noise(fx / 48.0, fz / 48.0, seed) * 0.6
        + value_noise(fx / 20.0, fz / 20.0, seed + 1) * 0.3
        + value_noise(fx / 8.0, fz / 8.0, seed + 2) * 0.1;
    let hills = value_noise(fx / 90.0, fz / 90.0, seed + 3);
    let region = value_noise(fx / 140.0, fz / 140.0, seed + 4);
    let ridge = 1.0 - (value_noise(fx / 36.0, fz / 36.0, seed + 5) * 2.0 - 1.0).abs();
    let mountain = ((region - 0.55) / 0.3).clamp(0.0, 1.0);
    (18.0 + n * 12.0 + hills * hills * 20.0 + mountain * mountain * ridge * ridge * 46.0) as i32
}

/// 0..1: how forested an area is.
fn forest(x: i32, z: i32, seed: u32) -> f32 {
    value_noise(x as f32 / 60.0, z as f32 / 60.0, seed + 9)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every cell, as the older `DYNW1` saves held them.
    fn encode_v1(w: &World) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        for s in w.size {
            out.extend_from_slice(&s.to_le_bytes());
        }
        let [sx, sy, sz] = w.size.map(|v| v as i32);
        let cells =
            (0..sy).flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| (x, y, z))));
        rle(cells.map(|(x, y, z)| w.get(x, y, z)), &mut out);
        out
    }

    fn same_cells(a: &World, b: &World) -> bool {
        let [sx, sy, sz] = a.size.map(|v| v as i32);
        a.size == b.size
            && (0..sy).all(|y| (0..sz).all(|z| (0..sx).all(|x| a.get(x, y, z) == b.get(x, y, z))))
    }

    #[test]
    fn encode_roundtrip() {
        let w = World::generate([64, 48, 64], 7);
        let d = w.encode();
        assert_eq!(&d[..5], MAGIC2);
        let w2 = World::decode(&d).unwrap();
        assert!(same_cells(&w, &w2));
        assert!(d.len() < 64 * 48 * 64 / 4, "compression should work");
        // a size that is not a whole number of sections
        let mut odd = World::empty([20, 18, 37]);
        odd.set(19, 17, 36, BRICK);
        odd.set(0, 0, 0, STONE);
        let back = World::decode(&odd.encode()).unwrap();
        assert!(same_cells(&odd, &back));
    }

    #[test]
    fn old_saves_still_load() {
        let w = World::generate([64, 48, 64], 7);
        let old = encode_v1(&w);
        assert_eq!(&old[..5], MAGIC);
        let back = World::decode(&old).unwrap();
        assert!(same_cells(&w, &back), "a DYNW1 save loads cell for cell");
        // and saves again in the new form
        assert!(same_cells(&w, &World::decode(&back.encode()).unwrap()));
        let mut short = old.clone();
        short.truncate(old.len() - 2);
        assert!(World::decode(&short).is_err());
    }

    #[test]
    fn big_worlds_are_allowed_and_cheap_when_empty() {
        let w = World::empty([2048, 256, 2048]);
        assert_eq!(w.section_dims(), [128, 16, 128]);
        // the sky costs nothing: only the table of sections
        assert!(w.memory_bytes() < 8 * 1024 * 1024, "{}", w.memory_bytes());
        let back = World::decode(&w.encode()).unwrap();
        assert_eq!(back.size, [2048, 256, 2048]);
        assert!(World::decode(&World::empty([MAX_SIDE + 16, 16, 16]).encode()).is_err());
        assert!(World::decode(&World::empty([16, MAX_HEIGHT + 16, 16]).encode()).is_err());
    }

    /// The 2 km world in memory (docs/design/scale.md):
    /// cargo test --release -p dyno-world two_km -- --ignored --nocapture
    #[test]
    #[ignore]
    fn two_km_world_memory() {
        let t = std::time::Instant::now();
        let w = World::generate([2048, 256, 2048], 20261122);
        let made = t.elapsed();
        let (n, cells) = w.held_sections();
        let kinds = w.secs.iter().fold([0usize; 3], |mut k, s| {
            match s {
                Sect::Uniform(_) => k[0] += 1,
                Sect::Packed(_) => k[1] += 1,
                _ => k[2] += 1,
            }
            k
        });
        let t = std::time::Instant::now();
        let light = crate::light::Light::new(&w);
        let lit = t.elapsed();
        let t = std::time::Instant::now();
        let saved = w.encode();
        let enc = t.elapsed();
        let t = std::time::Instant::now();
        let back = World::decode(&saved).unwrap();
        let dec = t.elapsed();
        assert_eq!(back.size, w.size);
        // a shard: a copy sharing every section, with its own light
        let shard = w.clone();
        eprintln!(
            "2048x256x2048: generated in {:.1} s; {n} sections: {} uniform, {} packed, {} raw; \
             cells {:.1} MB, world in memory {:.1} MB; light {:.1} MB (lit in {:.1} s); \
             save {:.1} MB (encoded in {:.1} s, loaded in {:.1} s); \
             a shard copy adds {:.1} MB of table (and its own light)",
            made.as_secs_f32(),
            kinds[0],
            kinds[1],
            kinds[2],
            cells as f64 / 1048576.0,
            w.memory_bytes() as f64 / 1048576.0,
            light.bytes() as f64 / 1048576.0,
            lit.as_secs_f32(),
            saved.len() as f64 / 1048576.0,
            enc.as_secs_f32(),
            dec.as_secs_f32(),
            (shard.memory_bytes() - shard.held_sections().1) as f64 / 1048576.0
        );
    }

    #[test]
    fn sections_pack_and_copies_share_them() {
        let mut w = World::generate([64, 48, 64], 7);
        let (_, packed) = w.held_sections();
        assert!(packed < 64 * 48 * 64 / 2, "packed: {packed} bytes");
        // a copy (a shard) shares every section until it changes one
        let mut copy = w.clone();
        assert!(w.diff(&copy).is_empty());
        copy.set(10, 40, 10, BRICK);
        copy.set(50, 1, 50, AIR);
        let d = w.diff(&copy);
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d.contains(&([10, 40, 10], BRICK)));
        assert_eq!(w.get(10, 40, 10), AIR, "the original is untouched");
        // an edit unpacks one section; compact packs it again
        w.set(20, 20, 20, LANTERN);
        assert_eq!(w.get(20, 20, 20), LANTERN);
        w.compact();
        assert_eq!(w.get(20, 20, 20), LANTERN);
        assert_eq!(w.positions_of(LANTERN), vec![[20, 20, 20]]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(World::decode(b"nope").is_err());
        let mut d = World::empty([4, 4, 4]).encode();
        d.push(200);
        assert!(World::decode(&d).is_err());
    }

    #[test]
    fn sections_stream_into_a_sparse_world() {
        let w = World::generate([64, 48, 64], 7);
        let mut c = World::sparse(w.size);
        assert_eq!(c.section_dims(), [4, 3, 4]);
        // nothing has arrived: air to look at, solid to stand on
        assert_eq!(c.get(10, 5, 10), AIR);
        assert!(!c.known(10, 5, 10) && c.solid(10, 5, 10));
        assert!(!c.set(10, 5, 10, STONE), "an unknown cell is not set");
        // outside the world is as before
        assert!(c.known(-1, 5, 0) && c.solid(5, -1, 5) && !c.solid(5, 60, 5));
        let rev = c.revision();
        let mut bytes = 0;
        for sy in 0..3 {
            for sz in 0..4 {
                for sx in 0..4 {
                    let d = w.encode_section([sx, sy, sz]);
                    bytes += d.len();
                    c.put_section([sx, sy, sz], &d).unwrap();
                }
            }
        }
        assert!(c.revision() > rev);
        for y in 0..48 {
            for z in 0..64 {
                for x in 0..64 {
                    assert_eq!(c.get(x, y, z), w.get(x, y, z), "cell {x},{y},{z}");
                }
            }
        }
        assert_eq!(c.encode(), w.encode(), "the same world, cell for cell");
        // the sky above the terrain is uniform air and costs no cells
        let (n, mem) = c.held_sections();
        assert_eq!(n, 48);
        assert!(mem < 48 * SECTION_CELLS, "{mem}");
        assert!(bytes < w.encode().len() * 2);
        // edits reach held sections; dropped sections are unknown again
        assert!(c.set(3, 40, 3, BRICK));
        assert_eq!(c.get(3, 40, 3), BRICK);
        c.drop_section([0, 2, 0]);
        assert!(!c.known(3, 40, 3) && c.get(3, 40, 3) == AIR);
        assert!(c.has_section([1, 2, 0]) && !c.has_section([0, 2, 0]));
        // a section's differences between two worlds
        let mut w2 = w.clone();
        w2.set(17, 20, 17, BRICK);
        assert_eq!(w.diff_section(&w2, [1, 1, 1]), vec![([17, 20, 17], BRICK)]);
        assert!(w.diff_section(&w2, [0, 1, 1]).is_empty());
        // bad data is refused
        assert!(c.put_section([0, 0, 0], &[STONE, 5]).is_err());
        assert!(
            c.put_section([9, 0, 0], &w.encode_section([0, 0, 0]))
                .is_err()
        );
    }

    #[test]
    fn spawn_is_above_ground() {
        let w = World::generate([64, 48, 64], 1);
        let s = w.spawn();
        assert_eq!(w.get(s[0] as i32, s[1] as i32, s[2] as i32), AIR);
    }
}
