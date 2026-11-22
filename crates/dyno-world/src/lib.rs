//! Block world storage shared by the client and the server (bootstrap).
//!
//! The bootstrap world is one fixed-size grid of cells. Each cell holds one
//! block ID. The world is sent to clients and saved to disk run-length encoded.

pub mod content;
pub mod light;
pub mod village;

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
                let chance = if f > 0.62 {
                    9
                } else if f > 0.45 {
                    40
                } else {
                    400
                };
                if !r.is_multiple_of(chance as u32) {
                    continue;
                }
                let h = w.surface(x, z);
                if w.get(x, h, z) != GRASS {
                    continue;
                }
                // keep trees apart
                if (-2..=2).any(|dz| {
                    (-2..=2).any(|dx| matches!(w.get(x + dx, h + 1, z + dz), LOG | BIRCH_LOG))
                }) {
                    continue;
                }
                if h > snow_line - 16 {
                    w.pine(x, h, z, r);
                } else if (r >> 12).is_multiple_of(3) {
                    w.round_tree(x, h, z, r, BIRCH_LOG, BIRCH_LEAVES);
                } else {
                    w.round_tree(x, h, z, r, LOG, LEAVES);
                }
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

    fn round_tree(&mut self, x: i32, h: i32, z: i32, r: u32, log: BlockId, leaves: BlockId) {
        let trunk = 4 + (r >> 8) as i32 % 3;
        for y in 1..=trunk {
            self.set(x, h + y, z, log);
        }
        for dy in -2..=1i32 {
            let rad: i32 = if dy == 1 { 1 } else { 2 };
            for dz in -rad..=rad {
                for dx in -rad..=rad {
                    if dx.abs() == rad
                        && dz.abs() == rad
                        && (dy == 1 || hash(x + dx, z + dz, dy as u32).is_multiple_of(2))
                    {
                        continue;
                    }
                    self.put_if_air(x + dx, h + trunk + dy, z + dz, leaves);
                }
            }
        }
    }

    fn pine(&mut self, x: i32, h: i32, z: i32, r: u32) {
        let trunk = 6 + (r >> 8) as i32 % 4;
        for y in 1..=trunk {
            self.set(x, h + y, z, LOG);
        }
        let mut rad: i32 = 2;
        for y in (3..=trunk + 1).rev().step_by(1) {
            let ry = if (trunk + 1 - y) % 2 == 0 {
                rad.min(1)
            } else {
                rad
            };
            for dz in -ry..=ry {
                for dx in -ry..=ry {
                    if dx.abs() + dz.abs() <= ry + 1 {
                        self.put_if_air(x + dx, h + y, z + dz, PINE_LEAVES);
                    }
                }
            }
            if (trunk + 1 - y) % 3 == 2 && rad < 3 {
                rad += 1;
            }
        }
        self.put_if_air(x, h + trunk + 2, z, PINE_LEAVES);
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
