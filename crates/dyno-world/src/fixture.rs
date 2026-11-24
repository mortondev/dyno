//! A seeded natural terrain for tests (feature `test-fixtures`): cheap to
//! make at any size, with everything meshing, light and streaming have to
//! cope with. Nothing that ships builds its world this way.

use crate::*;

impl World {
    /// A small natural world from a seed, for tests: meadows, forests of oak
    /// and birch, pine on the hills, snowy peaks, lakes with sandy shores,
    /// caves and ore. The game's world is built from the pack's layout
    /// ([`crate::worldgen::build`]); this is not in shipped programs.
    pub fn test_terrain(size: [u32; 3], seed: u32) -> Self {
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
