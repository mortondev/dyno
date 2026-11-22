//! Hearthstead: the starting town, stamped into the world around the spawn hearth.
//!
//! The town is about 70 × 70 blocks. The middle is levelled to the floor of the
//! hearth ruin; further out the ground follows a smoothed copy of the terrain,
//! with stone-brick retaining walls where a building pad cuts into a hill.
//!
//! Only air and natural blocks are ever replaced, so anything a player built
//! inside the footprint stays where it is.

use super::*;

const MOSSY: BlockId = 19;
const DARK_PLANKS: BlockId = 20;
const WOOL_WHITE: BlockId = 23;
const WOOL_RED: BlockId = 25;
const WOOL_YELLOW: BlockId = 27;
const WOOL_BLUE: BlockId = 30;
const WOOL_PURPLE: BlockId = 31;

/// The middle of the town is flat out to this distance from the hearth.
const FLAT: i32 = 22;
/// The low wall around the town.
const WALL: i32 = 31;
/// The ground follows the smoothed terrain from here on.
const EDGE: i32 = 33;
/// Beyond this distance nothing is touched.
const OUTER: i32 = 38;
/// Half the side of the level grid.
const GRID: i32 = OUTER + 4;

/// Blocks the town may replace: air and what the generator makes.
fn natural(b: BlockId) -> bool {
    matches!(
        b,
        AIR | GRASS
            | DIRT
            | STONE
            | SAND
            | SNOW
            | WATER
            | LOG
            | LEAVES
            | GLOWSHROOM
            | BIRCH_LOG
            | BIRCH_LEAVES
            | PINE_LEAVES
            | BERRY_BUSH
            | COAL_ORE
            | IRON_ORE
    )
}

/// Trees and plants: not ground.
fn plant(b: BlockId) -> bool {
    matches!(
        b,
        LOG | LEAVES | GLOWSHROOM | BIRCH_LOG | BIRCH_LEAVES | PINE_LEAVES | BERRY_BUSH
    )
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The buried marker of a finished town: two blocks under the hearth floor.
fn marker(hearth: [i32; 3]) -> [([i32; 3], BlockId); 2] {
    let [x, y, z] = hearth;
    [([x, y - 3, z], TIMBER_BEAM), ([x, y - 4, z], STONE_BRICKS)]
}

/// Build Hearthstead around the spawn hearth. Returns true if the world
/// changed, false if the town is already there.
pub fn build_hearthstead(world: &mut World, hearth: [i32; 3]) -> bool {
    if marker(hearth)
        .iter()
        .all(|(p, b)| world.get(p[0], p[1], p[2]) == *b)
    {
        return false;
    }
    let orig = world.clone();
    let mut t = Town {
        w: world,
        orig,
        o: [hearth[0], hearth[1] - 1, hearth[2]],
        lift: 0,
        lvl: Vec::new(),
        nat: Vec::new(),
    };
    t.plan_levels();
    t.grade();
    t.plaza();
    t.roads();
    t.perimeter();
    t.inn();
    t.tower();
    t.smithy();
    t.library();
    t.market();
    t.well();
    t.cottages();
    t.barn();
    t.field();
    t.paddock();
    t.pond();
    t.gardens();
    t.prune_leaves();
    // the marker is forced: a buried ore must not make the town rebuild
    for (p, b) in marker(hearth) {
        world.set(p[0], p[1], p[2], b);
    }
    true
}

#[derive(Clone, Copy)]
struct Rect {
    x0: i32,
    x1: i32,
    z0: i32,
    z1: i32,
}

impl Rect {
    fn grow(self, n: i32) -> Rect {
        Rect {
            x0: self.x0 - n,
            x1: self.x1 + n,
            z0: self.z0 - n,
            z1: self.z1 + n,
        }
    }

    /// The cells of the outline.
    fn ring(self) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for x in self.x0..=self.x1 {
            v.push((x, self.z0));
            v.push((x, self.z1));
        }
        for z in self.z0 + 1..self.z1 {
            v.push((self.x0, z));
            v.push((self.x1, z));
        }
        v
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    N,
    S,
    W,
    E,
}

#[derive(Clone, Copy)]
struct Style {
    /// the lowest course of the walls
    base: BlockId,
    wall: BlockId,
    /// corners, studs and beams
    frame: BlockId,
    roof: BlockId,
    floor: BlockId,
}

/// Side, coordinate of the open half, coordinate of the closed leaf.
type Door = (Side, i32, Option<i32>);

struct House {
    r: Rect,
    /// height of the walls above the floor
    h: i32,
    style: Style,
    /// the ridge runs east-west
    ridge_x: bool,
    /// side, coordinate of the open half, coordinate of the closed leaf
    door: Option<Door>,
    /// extra horizontal beams
    beams: &'static [i32],
    /// window rows as (lowest y, highest y)
    windows: &'static [(i32, i32)],
}

struct Town<'a> {
    w: &'a mut World,
    /// the world before the town: decides what may be replaced
    orig: World,
    /// hearth x, floor y of the plaza, hearth z
    o: [i32; 3],
    /// ground level of the thing being built, relative to the plaza
    lift: i32,
    /// planned ground level per column, relative to the plaza
    lvl: Vec<i32>,
    /// natural ground level per column, relative to the plaza
    nat: Vec<i32>,
}

impl Town<'_> {
    fn free(&self, x: i32, y: i32, z: i32) -> bool {
        let p = [self.o[0] + x, self.o[1] + y, self.o[2] + z];
        self.w.in_bounds(p) && natural(self.orig.get(p[0], p[1], p[2]))
    }

    /// Set a block at an offset from the hearth, `y` above the current
    /// ground level, unless something a player made is in the way.
    fn put(&mut self, x: i32, y: i32, z: i32, b: BlockId) {
        let y = y + self.lift;
        if self.free(x, y, z) {
            self.w.set(self.o[0] + x, self.o[1] + y, self.o[2] + z, b);
        }
    }

    fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.w
            .get(self.o[0] + x, self.o[1] + y + self.lift, self.o[2] + z)
    }

    fn fill(&mut self, a: [i32; 3], b: [i32; 3], block: BlockId) {
        for y in a[1]..=b[1] {
            for z in a[2]..=b[2] {
                for x in a[0]..=b[0] {
                    self.put(x, y, z, block);
                }
            }
        }
    }

    fn column(&mut self, x: i32, z: i32, ys: std::ops::RangeInclusive<i32>, b: BlockId) {
        for y in ys {
            self.put(x, y, z, b);
        }
    }

    fn cell(x: i32, z: i32) -> Option<usize> {
        if x.abs() > GRID || z.abs() > GRID {
            return None;
        }
        Some(((z + GRID) * (2 * GRID + 1) + x + GRID) as usize)
    }

    fn level(&self, x: i32, z: i32) -> i32 {
        Self::cell(x, z).map_or(0, |i| self.lvl[i])
    }

    fn set_level(&mut self, x: i32, z: i32, l: i32) {
        if let Some(i) = Self::cell(x, z) {
            self.lvl[i] = l;
        }
    }

    /// Build on the ground of this column from now on.
    fn stand(&mut self, x: i32, z: i32) {
        self.lift = self.level(x, z);
    }

    /// Replace the ground block of a column.
    fn pave(&mut self, x: i32, z: i32, b: BlockId) {
        let keep = self.lift;
        self.stand(x, z);
        self.put(x, 0, z, b);
        self.lift = keep;
    }

    /// Flagstones with the odd cobble, for roads and yards.
    fn pave_road(&mut self, x: i32, z: i32) {
        let b = if hash(x, z, 91).is_multiple_of(9) {
            COBBLE
        } else {
            FLAGSTONES
        };
        self.pave(x, z, b);
    }

    fn pave_rect(&mut self, r: Rect) {
        for z in r.z0..=r.z1 {
            for x in r.x0..=r.x1 {
                self.pave_road(x, z);
            }
        }
    }

    // ---------------------------------------------------------------- ground

    /// Natural ground of a column in the original world, relative to the
    /// plaza: the highest solid block that is not a plant, or the water surface.
    fn ground(&self, x: i32, z: i32) -> i32 {
        let (wx, wz) = (self.o[0] + x, self.o[2] + z);
        let mut y = self.orig.size[1] as i32 - 1;
        while y > 0 {
            let b = self.orig.get(wx, y, wz);
            if b == WATER || (is_solid(b) && !plant(b)) {
                break;
            }
            y -= 1;
        }
        y - self.o[1]
    }

    /// Decide the ground level of every column: flat in the middle, a smoothed
    /// copy of the terrain towards the edge, the terrain itself outside.
    fn plan_levels(&mut self) {
        let n = (2 * GRID + 1) as usize;
        self.nat = vec![0; n * n];
        self.lvl = vec![0; n * n];
        for z in -GRID..=GRID {
            for x in -GRID..=GRID {
                self.nat[Self::cell(x, z).unwrap()] = self.ground(x, z);
            }
        }
        let nat = |s: &Self, x: i32, z: i32| {
            s.nat[Self::cell(x.clamp(-GRID, GRID), z.clamp(-GRID, GRID)).unwrap()]
        };
        for z in -GRID..=GRID {
            for x in -GRID..=GRID {
                let r = x.abs().max(z.abs());
                let here = nat(self, x, z);
                let l = if r >= OUTER {
                    here
                } else {
                    let mut sum = 0;
                    for dz in -3..=3 {
                        for dx in -3..=3 {
                            sum += nat(self, x + dx, z + dz).clamp(-6, 6);
                        }
                    }
                    let soft = sum as f32 / 49.0;
                    let t = smooth((r - FLAT) as f32 / (EDGE - FLAT) as f32);
                    let u = smooth((r - EDGE) as f32 / (OUTER - EDGE) as f32);
                    (t * (soft * (1.0 - u) + here as f32 * u)).round() as i32
                };
                self.set_level(x, z, l);
            }
        }
        // roads climb at most one block per step
        for dir in [[0, -1], [1, 0], [0, 1]] {
            let mut l = 0;
            for i in 0..=OUTER {
                let (cx, cz) = (dir[0] * i, dir[1] * i);
                l = self.level(cx, cz).clamp(l - 1, l + 1);
                for k in -2..=2 {
                    self.set_level(cx + dir[1] * k, cz + dir[0] * k, l);
                }
            }
        }
        // level pads for the buildings outside the flat middle
        for (r, road) in PADS {
            let l = self.level(road[0], road[1]);
            let r = r.grow(2);
            for z in r.z0..=r.z1 {
                for x in r.x0..=r.x1 {
                    self.set_level(x, z, l);
                }
            }
        }
    }

    /// Move the earth: cut what is above the planned level, fill what is
    /// below, turf the top and face the big steps with stone bricks.
    fn grade(&mut self) {
        let top = self.orig.size[1] as i32 - 1 - self.o[1];
        for z in -OUTER..=OUTER {
            for x in -OUTER..=OUTER {
                let i = Self::cell(x, z).unwrap();
                let (t, n) = (self.lvl[i], self.nat[i]);
                let r = x.abs().max(z.abs());
                if r > WALL + 1 && t == n {
                    continue;
                }
                let (wx, wz) = (self.o[0] + x, self.o[2] + z);
                for y in t + 1..=top {
                    if self.orig.get(wx, self.o[1] + y, wz) != WATER {
                        self.put(x, y, z, AIR);
                    }
                }
                for d in 0..48 {
                    let y = t - d;
                    let b = self.orig.get(wx, self.o[1] + y, wz);
                    if b == AIR || b == WATER || plant(b) || (d == 0 && natural(b)) {
                        self.put(x, y, z, if d == 0 { GRASS } else { DIRT });
                    } else if d <= 2 && matches!(b, GRASS | SAND | SNOW) {
                        self.put(x, y, z, DIRT);
                    } else if d > 2 {
                        break;
                    }
                }
            }
        }
        for z in -EDGE..=EDGE {
            for x in -EDGE..=EDGE {
                let t = self.level(x, z);
                let low = [[1, 0], [-1, 0], [0, 1], [0, -1]]
                    .iter()
                    .map(|d| self.level(x + d[0], z + d[1]))
                    .min()
                    .unwrap();
                if t - low >= 2 {
                    for y in low + 1..=t {
                        let b = if hash3(x, y, z, 3).is_multiple_of(6) {
                            MOSSY
                        } else {
                            STONE_BRICKS
                        };
                        self.put(x, y, z, b);
                    }
                }
            }
        }
    }

    /// Remove leaves that lost their tree.
    fn prune_leaves(&mut self) {
        let (lo, hi) = (-OUTER - 4, OUTER + 4);
        let sy = self.w.size[1] as i32;
        for z in lo..=hi {
            for x in lo..=hi {
                let (wx, wz) = (self.o[0] + x, self.o[2] + z);
                for y in 0..sy {
                    if !is_leaves(self.w.get(wx, y, wz)) {
                        continue;
                    }
                    let mut held = false;
                    'find: for dy in -7..=1 {
                        for dz in -3..=3 {
                            for dx in -3..=3 {
                                if matches!(self.w.get(wx + dx, y + dy, wz + dz), LOG | BIRCH_LOG) {
                                    held = true;
                                    break 'find;
                                }
                            }
                        }
                    }
                    if !held {
                        self.w.set(wx, y, wz, AIR);
                    }
                }
            }
        }
    }

    // ----------------------------------------------------------- town fabric

    fn lamp(&mut self, x: i32, z: i32) {
        self.stand(x, z);
        if (1..=4).all(|y| self.get(x, y, z) == AIR) {
            self.column(x, z, 1..=3, TIMBER_BEAM);
            self.put(x, 4, z, LANTERN);
        }
        self.lift = 0;
    }

    fn plaza(&mut self) {
        for z in -7i32..=7 {
            for x in -7i32..=7 {
                let r2 = x * x + z * z;
                if r2 > 45 {
                    continue;
                }
                let b = if r2 > 36 {
                    STONE_BRICKS
                } else if hash(x, z, 17).is_multiple_of(11) {
                    COBBLE
                } else {
                    FLAGSTONES
                };
                // the old floor of the ruin was laid by the generator: re-pave it
                let (wx, wy, wz) = (self.o[0] + x, self.o[1], self.o[2] + z);
                if r2 <= 20 && matches!(self.orig.get(wx, wy, wz), COBBLE | MOSSY) {
                    let old = if hash(x, z, 23).is_multiple_of(9) {
                        MOSSY
                    } else {
                        b
                    };
                    self.w.set(wx, wy, wz, old);
                } else {
                    self.put(x, 0, z, b);
                }
            }
        }
        for (sx, sz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            self.lamp(6 * sx, 6 * sz);
            self.put(7 * sx, 1, 6 * sz, FLOWER_BED);
            self.put(6 * sx, 1, 7 * sz, FLOWER_BED);
        }
    }

    fn roads(&mut self) {
        for (n, dir) in [[0, -1], [1, 0], [0, 1]].into_iter().enumerate() {
            let side = [dir[1], dir[0]];
            for i in 6..=OUTER {
                let (cx, cz) = (dir[0] * i, dir[1] * i);
                for k in -1..=1 {
                    self.pave_road(cx + side[0] * k, cz + side[1] * k);
                }
                // a ragged end where the road leaves the town
                if i > WALL + 3 && hash(cx, cz, 5).is_multiple_of(2) {
                    break;
                }
                if i % 8 == 0 && i < WALL {
                    let k = if (i / 8 + n as i32) % 2 == 0 { 2 } else { -2 };
                    self.lamp(cx + side[0] * k, cz + side[1] * k);
                }
            }
            // the gate: two poles with banners hanging over the road
            let (cx, cz) = (dir[0] * WALL, dir[1] * WALL);
            for k in [-3, 3] {
                let (x, z) = (cx + side[0] * k, cz + side[1] * k);
                let (ax, az) = (
                    cx + side[0] * (k - k.signum()),
                    cz + side[1] * (k - k.signum()),
                );
                self.stand(x, z);
                self.column(x, z, 1..=2, STONE_BRICKS);
                self.column(x, z, 3..=6, TIMBER_BEAM);
                self.put(ax, 6, az, TIMBER_BEAM);
                self.column(ax, az, 4..=5, RED_BANNER);
                self.put(x, 7, z, LANTERN);
                self.lift = 0;
            }
        }
    }

    /// A low cobble wall with timber posts around the town, open at the roads.
    fn perimeter(&mut self) {
        let ring = Rect {
            x0: -WALL,
            x1: WALL,
            z0: -WALL,
            z1: WALL,
        };
        for (x, z) in ring.ring() {
            let on_road = (x.abs() <= 3 && z == -WALL)
                || (z.abs() <= 3 && x == WALL)
                || (x.abs() <= 3 && z == WALL);
            if on_road {
                continue;
            }
            self.stand(x, z);
            let along = if x.abs() == WALL { z } else { x };
            if x == -WALL && z.abs() < WALL {
                // the west side is a paling fence
                if along.rem_euclid(2) == 0 {
                    self.put(x, 1, z, TIMBER_BEAM);
                }
                if along.rem_euclid(6) == 0 {
                    self.put(x, 2, z, TIMBER_BEAM);
                }
                continue;
            }
            let b = if hash(x, z, 41).is_multiple_of(3) {
                MOSSY
            } else {
                COBBLE
            };
            self.put(x, 1, z, b);
            if along.rem_euclid(6) == 0 || (x.abs() == WALL && z.abs() == WALL) {
                self.put(x, 1, z, TIMBER_BEAM);
                self.put(x, 2, z, TIMBER_BEAM);
            }
        }
        self.lift = 0;
    }

    // -------------------------------------------------------------- building

    fn wall_cell(r: Rect, side: Side, i: i32) -> (i32, i32) {
        match side {
            Side::N => (i, r.z0),
            Side::S => (i, r.z1),
            Side::W => (r.x0, i),
            Side::E => (r.x1, i),
        }
    }

    fn outward(side: Side) -> (i32, i32) {
        match side {
            Side::N => (0, -1),
            Side::S => (0, 1),
            Side::W => (-1, 0),
            Side::E => (1, 0),
        }
    }

    /// Floor, walls with windows and window boxes, door and roof.
    fn shell(&mut self, h: &House) {
        let (r, s) = (h.r, h.style);
        self.fill([r.x0, 0, r.z0], [r.x1, 0, r.z1], s.floor);
        self.fill([r.x0 + 1, 1, r.z0 + 1], [r.x1 - 1, h.h, r.z1 - 1], AIR);
        for side in [Side::N, Side::S, Side::W, Side::E] {
            let (lo, hi) = match side {
                Side::N | Side::S => (r.x0, r.x1),
                _ => (r.z0, r.z1),
            };
            let (ox, oz) = Self::outward(side);
            for i in lo..=hi {
                let (x, z) = Self::wall_cell(r, side, i);
                let d = (i - lo).min(hi - i);
                for y in 1..=h.h {
                    let glass = d % 4 == 2 && h.windows.iter().any(|(a, b)| (*a..=*b).contains(&y));
                    let b = if d == 0 || y == h.h || h.beams.contains(&y) {
                        s.frame
                    } else if y == 1 {
                        s.base
                    } else if glass {
                        LEADED_WINDOW
                    } else if d % 4 == 0 {
                        s.frame
                    } else {
                        s.wall
                    };
                    self.put(x, y, z, b);
                }
                if d % 4 == 2 && h.windows.iter().any(|(a, _)| *a <= 3) {
                    self.put(x + ox, 1, z + oz, FLOWER_BED);
                }
            }
        }
        if let Some((side, gap, leaf)) = h.door {
            let (ox, oz) = Self::outward(side);
            let (x, z) = Self::wall_cell(r, side, gap);
            self.column(x, z, 1..=2, AIR);
            self.put(x, 3, z, s.frame);
            self.put(x + ox, 1, z + oz, AIR);
            let mut far = gap - 1;
            if let Some(leaf) = leaf {
                let (x, z) = Self::wall_cell(r, side, leaf);
                self.column(x, z, 1..=2, WOODEN_DOOR);
                self.put(x, 3, z, s.frame);
                self.put(x + ox, 1, z + oz, AIR);
                far = gap - (leaf - gap);
            }
            // something left by the door
            let (x, z) = Self::wall_cell(r, side, far);
            let b = if hash(x, z, 77).is_multiple_of(2) {
                CRATE
            } else {
                BARREL
            };
            self.put(x + ox, 1, z + oz, b);
        }
        self.roof(r, h.h, h.ridge_x, s);
    }

    /// A stepped pitched roof with an overhang of one block and filled gables.
    fn roof(&mut self, r: Rect, h: i32, ridge_x: bool, s: Style) {
        let (lo, hi, la, lb) = if ridge_x {
            (r.z0, r.z1, r.x0, r.x1)
        } else {
            (r.x0, r.x1, r.z0, r.z1)
        };
        for k in 0.. {
            let (a, b) = (lo - 1 + k, hi + 1 - k);
            if a > b {
                break;
            }
            let y = h + k;
            let set = |t: &mut Self, l: i32, c: i32, block: BlockId| {
                if ridge_x {
                    t.put(l, y, c, block);
                } else {
                    t.put(c, y, l, block);
                }
            };
            for l in la - 1..=lb + 1 {
                set(self, l, a, s.roof);
                set(self, l, b, s.roof);
            }
            if k > 0 {
                for c in a + 1..b {
                    let block = if c - lo == hi - c { s.frame } else { s.wall };
                    set(self, la, c, block);
                    set(self, lb, c, block);
                }
            }
        }
    }

    /// Height of the ridge of a roof over walls of height `h`.
    fn ridge(span: i32, h: i32) -> i32 {
        h + (span + 1) / 2
    }

    /// A chimney on the outside of a wall, with a fire inside.
    fn fireplace(&mut self, x: i32, z: i32, inward: i32, top: i32) {
        self.column(x, z, 1..=top, STONE_BRICKS);
        self.column(x - inward, z, 0..=top, STONE_BRICKS);
        self.put(x - inward, top + 1, z, COBBLE);
        let fx = x + inward;
        for dz in [-1, 1] {
            self.column(fx, z + dz, 1..=3, STONE_BRICKS);
        }
        self.put(fx, 1, z, BRAZIER);
        self.put(fx, 2, z, AIR);
        self.put(fx, 3, z, STONE_BRICKS);
    }

    /// A plank on a post.
    fn table(&mut self, x: i32, y: i32, z: i32) {
        self.put(x, y, z, TORCH);
        self.put(x, y + 1, z, PLANKS);
    }

    /// Steps up to the next floor: stacks of rising height along `dx`, and
    /// the hole in the floor above them.
    fn stairs(&mut self, from: [i32; 3], dx: i32, rise: i32, block: BlockId) {
        let [x, y, z] = from;
        for i in 0..rise {
            let sx = x + dx * i;
            self.column(sx, z, y + 1..=y + 1 + i, block);
            if i >= rise - 2 {
                self.column(sx, z, y + 2 + i..=y + rise + 2, AIR);
            }
        }
    }

    // ------------------------------------------------------------- buildings

    /// The Wayfarer's Rest: two storeys of timber and plaster under red tiles.
    fn inn(&mut self) {
        let r = Rect {
            x0: -21,
            x1: -9,
            z0: -5,
            z1: 5,
        };
        let style = Style {
            base: STONE_BRICKS,
            wall: PLASTER,
            frame: TIMBER_BEAM,
            roof: RED_ROOF_TILES,
            floor: PLANKS,
        };
        self.shell(&House {
            r,
            h: 9,
            style,
            ridge_x: true,
            door: Some((Side::E, 0, Some(-1))),
            beams: &[5],
            windows: &[(2, 3), (7, 8)],
        });
        // a second leaf: the door stands open in the middle
        self.column(-9, 1, 1..=2, WOODEN_DOOR);
        self.put(-9, 3, 1, TIMBER_BEAM);
        self.put(-8, 1, 1, AIR);
        // the sign over the door, lanterns beside it, an attic window
        self.put(-8, 6, 0, TIMBER_BEAM);
        self.column(-8, 0, 4..=5, RED_BANNER);
        self.put(-8, 3, -2, LANTERN);
        self.put(-8, 3, 2, LANTERN);
        self.put(-9, 12, 0, LEADED_WINDOW);
        self.put(-21, 12, -2, LEADED_WINDOW);
        self.put(-21, 12, 2, LEADED_WINDOW);
        self.column(-8, -5, 1..=2, BARREL);
        self.put(-8, 1, 5, CRATE);
        self.put(-8, 1, -4, BARREL);
        // upper floor
        self.fill([-20, 5, -4], [-10, 5, 4], PLANKS);
        self.fireplace(-21, 0, 1, Self::ridge(11, 9) + 1);
        self.stairs([-12, 0, 4], -1, 4, DARK_PLANKS);
        // the common room: counter, kitchen, casks, tables
        self.fill([-17, 1, -4], [-17, 1, -2], DARK_PLANKS);
        self.put(-18, 1, -2, DARK_PLANKS);
        self.put(-20, 1, -4, FURNACE);
        self.put(-20, 1, -3, CRAFTING_TABLE);
        self.column(-19, -4, 1..=2, BARREL);
        self.put(-18, 1, -4, BARREL);
        self.column(-20, 4, 1..=2, BARREL);
        self.put(-19, 1, 4, BARREL);
        self.put(-20, 1, 3, CRATE);
        for (x, z) in [(-12, -3), (-15, -2), (-12, 0), (-16, 2)] {
            self.table(x, 1, z);
        }
        self.put(-14, 4, 0, LANTERN);
        self.put(-18, 4, 2, LANTERN);
        self.put(-11, 4, -3, LANTERN);
        // the guest rooms upstairs
        self.fill([-15, 6, -4], [-15, 8, -2], PLASTER);
        self.fill([-15, 6, 0], [-15, 8, 1], PLASTER);
        for z in [-4, -2, 2] {
            self.put(-20, 6, z, BED);
            self.put(-19, 6, z, BED);
        }
        self.put(-20, 6, 4, CHEST);
        self.put(-16, 6, -4, CHEST);
        self.put(-10, 6, -4, BOOKSHELF);
        self.put(-10, 7, -4, LANTERN);
        self.put(-18, 9, 0, LANTERN);
        // the forecourt
        self.pave_rect(Rect {
            x0: -8,
            x1: -6,
            z0: -3,
            z1: 3,
        });
        self.put(-8, 1, -3, FLOWER_BED);
        self.put(-8, 1, 3, FLOWER_BED);
    }

    /// The Watch tower: four storeys of stone, a walk with battlements, a beacon.
    fn tower(&mut self) {
        let r = Rect {
            x0: 4,
            x1: 10,
            z0: -16,
            z1: -10,
        };
        let top = 16;
        self.fill([r.x0, 0, r.z0], [r.x1, 0, r.z1], STONE_BRICKS);
        self.fill([r.x0 + 1, 1, r.z0 + 1], [r.x1 - 1, top, r.z1 - 1], AIR);
        for (x, z) in r.ring() {
            let corner = (x == r.x0 || x == r.x1) && (z == r.z0 || z == r.z1);
            for y in 1..=top {
                let n = hash3(x, y, z, 5) % 12;
                let b = if corner {
                    STONE_BRICKS
                } else if y <= 3 && n < 3 {
                    MOSSY
                } else if n == 3 {
                    COBBLE
                } else {
                    STONE_BRICKS
                };
                self.put(x, y, z, b);
            }
        }
        // a footing around the base
        for (x, z) in r.grow(1).ring() {
            if (x, z) != (3, -13) && (x, z) != (3, -12) {
                self.put(x, 0, z, STONE_BRICKS);
            }
        }
        for s in 0..4 {
            let y = 4 * s;
            if s > 0 {
                self.fill([5, y, -15], [9, y, -11], DARK_PLANKS);
            }
            // arrow slits, except where the stairs run
            let wy = y + 2;
            if s % 2 == 1 {
                self.put(7, wy, -16, LEADED_WINDOW);
            } else {
                self.put(7, wy, -10, LEADED_WINDOW);
            }
            self.put(10, wy, -13, LEADED_WINDOW);
            if s > 0 {
                self.put(4, wy, -13, LEADED_WINDOW);
            }
        }
        // the walk: a deck one block wider than the tower, on corbels
        let deck = r.grow(1);
        self.fill(
            [deck.x0, top, deck.z0],
            [deck.x1, top, deck.z1],
            STONE_BRICKS,
        );
        for (x, z) in deck.ring() {
            if (x + z).rem_euclid(2) == 0 {
                self.put(x, top - 1, z, STONE_BRICKS);
                self.put(x, top + 2, z, STONE_BRICKS);
            }
            self.put(x, top + 1, z, STONE_BRICKS);
        }
        for s in 0..4 {
            let y = 4 * s;
            if s % 2 == 0 {
                self.stairs([5, y, -15], 1, 3, PLANKS);
            } else {
                self.stairs([9, y, -11], -1, 3, PLANKS);
            }
        }
        // the beacon
        self.column(7, -13, top + 1..=top + 3, STONE_BRICKS);
        self.put(7, top + 4, -13, LANTERN);
        for (x, z) in [(6, -13), (8, -13), (7, -14), (7, -12)] {
            self.put(x, top + 3, z, LANTERN);
        }
        // banners of the Watch under the walk
        for (x, z) in [(3, -13), (11, -13), (7, -17), (7, -9)] {
            self.column(x, z, top - 4..=top - 2, BLUE_BANNER);
        }
        // the door, with its leaf swung open, and a banner over it
        self.column(4, -13, 1..=2, AIR);
        self.column(3, -14, 1..=2, WOODEN_DOOR);
        self.column(3, -12, 3..=4, BLUE_BANNER);
        self.column(3, -14, 3..=4, BLUE_BANNER);
        self.put(3, 3, -13, LANTERN);
        // guard room
        self.put(9, 1, -15, BARREL);
        self.put(9, 1, -12, CRAFTING_TABLE);
        self.put(9, 1, -11, CHEST);
        self.put(5, 1, -11, CRATE);
        self.put(6, 1, -11, CRATE);
        self.put(5, 2, -11, LANTERN);
        // bunks
        self.put(9, 5, -15, BED);
        self.put(9, 5, -14, BED);
        self.put(5, 5, -13, BED);
        self.put(5, 5, -12, BED);
        self.put(5, 5, -15, CHEST);
        self.put(5, 6, -15, LANTERN);
        // the captain's study
        self.column(9, -12, 9..=10, BOOKSHELF);
        self.column(9, -13, 9..=10, BOOKSHELF);
        self.put(9, 9, -11, CHEST);
        self.put(5, 9, -11, CRAFTING_TABLE);
        self.put(5, 10, -11, LANTERN);
        self.fill([6, 8, -14], [8, 8, -12], WOOL_BLUE);
        // armoury
        self.put(9, 13, -15, CRATE);
        self.put(9, 13, -14, CRATE);
        self.put(9, 14, -15, LANTERN);
        self.put(5, 13, -15, ANVIL);
        self.put(5, 13, -14, BARREL);
        // the way to the road
        self.pave_rect(Rect {
            x0: 2,
            x1: 3,
            z0: -14,
            z1: -11,
        });
    }

    /// The smithy: an open forge under a slate roof, a stone room behind.
    fn smithy(&mut self) {
        let r = Rect {
            x0: 6,
            x1: 16,
            z0: 5,
            z1: 13,
        };
        let style = Style {
            base: STONE_BRICKS,
            wall: STONE_BRICKS,
            frame: STONE_BRICKS,
            roof: SLATE_ROOF,
            floor: PLANKS,
        };
        self.fill([r.x0, 0, r.z0], [r.x1, 0, 8], COBBLE);
        self.fill([r.x0, 0, 9], [r.x1, 0, r.z1], PLANKS);
        self.fill([r.x0, 1, r.z0], [r.x1, 5, r.z1], AIR);
        let back = Rect { z0: 9, ..r };
        for (x, z) in back.ring() {
            for y in 1..=5 {
                let n = hash3(x, y, z, 9) % 10;
                let b = if n == 0 && y <= 2 {
                    MOSSY
                } else {
                    STONE_BRICKS
                };
                self.put(x, y, z, b);
            }
        }
        for (x, z) in [(9, 13), (13, 13), (16, 11), (6, 11)] {
            self.column(x, z, 2..=3, LEADED_WINDOW);
            let (ox, oz) = if z == 13 {
                (0, 1)
            } else if x == 16 {
                (1, 0)
            } else {
                (-1, 0)
            };
            self.put(x + ox, 1, z + oz, FLOWER_BED);
        }
        // the open forge: pillars, a beam on top, low side walls
        for x in [6, 11, 16] {
            self.column(x, 5, 1..=5, STONE_BRICKS);
        }
        self.fill([6, 5, 5], [16, 5, 5], TIMBER_BEAM);
        for x in [6, 16] {
            self.fill([x, 5, 5], [x, 5, 8], TIMBER_BEAM);
            self.fill([x, 1, 6], [x, 1, 8], STONE_BRICKS);
            self.put(x, 5, 5, STONE_BRICKS);
        }
        self.roof(r, 5, true, style);
        // the forge and its chimney
        self.put(8, 1, 8, BRAZIER);
        self.put(9, 1, 8, FURNACE);
        self.put(10, 1, 8, FURNACE);
        self.column(9, 8, 2..=Self::ridge(9, 5) + 2, STONE_BRICKS);
        self.put(9, 1, 6, ANVIL);
        self.put(7, 1, 8, BARREL);
        self.put(15, 1, 6, CRATE);
        self.column(15, 7, 1..=2, CRATE);
        self.put(15, 1, 8, BARREL);
        self.put(8, 4, 5, LANTERN);
        self.put(14, 4, 5, LANTERN);
        // the room behind
        self.column(13, 9, 1..=2, AIR);
        self.put(12, 4, 9, RED_BANNER);
        self.put(12, 3, 9, RED_BANNER);
        self.put(7, 1, 12, BED);
        self.put(8, 1, 12, BED);
        self.put(7, 1, 10, CHEST);
        self.put(15, 1, 12, CRAFTING_TABLE);
        self.put(15, 2, 12, LANTERN);
        self.put(11, 1, 12, BOOKSHELF);
        self.fill([10, 0, 10], [12, 0, 11], WOOL_RED);
        // the yard
        self.pave_rect(Rect {
            x0: 7,
            x1: 15,
            z0: 2,
            z1: 4,
        });
        self.put(5, 1, 5, CRATE);
        self.put(5, 1, 6, BARREL);
        self.put(17, 1, 5, HAY_BALE);
    }

    /// The Hearthkeeper's house: a thatched library.
    fn library(&mut self) {
        let r = Rect {
            x0: -16,
            x1: -6,
            z0: 9,
            z1: 17,
        };
        let style = Style {
            base: COBBLE,
            wall: PLASTER,
            frame: TIMBER_BEAM,
            roof: THATCH,
            floor: PLANKS,
        };
        self.shell(&House {
            r,
            h: 4,
            style,
            ridge_x: true,
            door: Some((Side::E, 13, Some(12))),
            beams: &[],
            windows: &[(2, 3)],
        });
        self.put(-6, 6, 13, LEADED_WINDOW);
        self.fireplace(-16, 13, 1, Self::ridge(9, 4) + 1);
        // books along the walls, low under the windows
        for x in -14..=-8 {
            let top = if x == -14 || x == -8 { 1 } else { 3 };
            self.column(x, 10, 1..=top, BOOKSHELF);
        }
        for x in -13..=-11 {
            self.column(x, 16, 1..=3, BOOKSHELF);
        }
        self.column(-15, 10, 1..=3, BOOKSHELF);
        self.column(-15, 16, 1..=3, BOOKSHELF);
        self.put(-8, 1, 16, BED);
        self.put(-9, 1, 16, BED);
        self.put(-7, 1, 16, CHEST);
        self.put(-11, 1, 13, CRAFTING_TABLE);
        self.put(-11, 2, 13, LANTERN);
        self.put(-7, 1, 10, BOOKSHELF);
        self.put(-7, 2, 10, LANTERN);
        self.fill([-13, 0, 12], [-9, 0, 14], WOOL_PURPLE);
        self.fill([-12, 0, 13], [-10, 0, 13], WOOL_RED);
        // the path to the road, with a little garden
        self.pave_rect(Rect {
            x0: -5,
            x1: -2,
            z0: 12,
            z1: 14,
        });
        for z in [9, 10, 16, 17] {
            self.put(-4, 1, z, FLOWER_BED);
        }
        self.put(-5, 1, 16, BARREL);
    }

    /// One market stall: four posts, a striped awning, goods on the counter.
    fn stall(&mut self, x: i32, z: i32, cloth: BlockId, goods: [BlockId; 3]) {
        for (px, pz) in [(x, z), (x + 3, z), (x, z + 2), (x + 3, z + 2)] {
            self.column(px, pz, 1..=3, TIMBER_BEAM);
        }
        for dz in -1..=3 {
            for dx in 0..=3 {
                let b = if dx % 2 == 0 { cloth } else { WOOL_WHITE };
                self.put(x + dx, if dz == -1 || dz == 3 { 3 } else { 4 }, z + dz, b);
            }
        }
        self.put(x + 1, 1, z + 2, goods[0]);
        self.put(x + 2, 1, z + 2, goods[1]);
        self.put(x + 1, 1, z, goods[2]);
        self.put(x + 2, 1, z, goods[0]);
        self.put(x + 2, 2, z, goods[1]);
    }

    fn market(&mut self) {
        self.pave_rect(Rect {
            x0: -16,
            x1: -2,
            z0: -17,
            z1: -8,
        });
        self.stall(-15, -16, WOOL_RED, [CRATE, BARREL, HAY_BALE]);
        self.stall(-10, -16, WOOL_BLUE, [BARREL, CRATE, CRATE]);
        self.stall(-15, -11, WOOL_YELLOW, [HAY_BALE, CRATE, BARREL]);
        // goods waiting to be sold
        self.column(-5, -16, 1..=2, CRATE);
        self.put(-4, 1, -16, CRATE);
        self.put(-5, 1, -15, BARREL);
        self.put(-9, 1, -10, BARREL);
        self.column(-9, -9, 1..=2, BARREL);
        self.put(-8, 1, -9, CRATE);
        self.put(-17, 1, -13, HAY_BALE);
        self.lamp(-6, -11);
    }

    /// The well: a stone ring, water, two posts and a little roof.
    fn well(&mut self) {
        let (cx, cz) = (9, -6);
        self.pave_rect(Rect {
            x0: cx - 2,
            x1: cx + 2,
            z0: cz - 2,
            z1: cz + 2,
        });
        for dz in -1..=1 {
            for dx in -1..=1 {
                self.put(cx + dx, -1, cz + dz, STONE_BRICKS);
                if (dx, dz) != (0, 0) {
                    self.put(cx + dx, 0, cz + dz, STONE_BRICKS);
                    self.put(cx + dx, 1, cz + dz, STONE_BRICKS);
                }
            }
        }
        self.put(cx, 0, cz, WATER);
        self.put(cx, 1, cz, WATER);
        for dx in [-1, 1] {
            self.column(cx + dx, cz, 2..=3, TIMBER_BEAM);
        }
        self.fill([cx - 1, 4, cz], [cx + 1, 4, cz], TIMBER_BEAM);
        for dx in -2..=2 {
            self.put(cx + dx, 4, cz - 1, RED_ROOF_TILES);
            self.put(cx + dx, 4, cz + 1, RED_ROOF_TILES);
            self.put(cx + dx, 5, cz, RED_ROOF_TILES);
        }
        self.put(cx + 2, 1, cz + 2, BARREL);
    }

    fn cottage(&mut self, n: usize, style: Style) {
        let (r, road) = PADS[n];
        let (ridge_x, door, path) = COTTAGES[n];
        self.stand(road[0], road[1]);
        self.shell(&House {
            r,
            h: 4,
            style,
            ridge_x,
            door: Some(door),
            beams: &[],
            windows: &[(2, 3)],
        });
        // bed, chest, stove and table in the corners; a lantern on the table
        let (x0, x1, z0, z1) = (r.x0 + 1, r.x1 - 1, r.z0 + 1, r.z1 - 1);
        let west_door = door.0 == Side::W;
        let bx = if west_door { x1 } else { x0 };
        let tx = if west_door { x0 } else { x1 };
        self.put(bx, 1, z0, BED);
        self.put(bx, 1, z0 + 1, BED);
        self.put(bx, 1, z1, CHEST);
        self.put(tx, 1, z1, CRAFTING_TABLE);
        self.put(tx, 2, z1, LANTERN);
        if door.0 != Side::S {
            self.put((x0 + x1) / 2, 1, z1, FURNACE);
            self.put((x0 + x1) / 2 + 1, 1, z1, BARREL);
        }
        if door.0 != Side::N {
            self.put((x0 + x1) / 2, 1, z0, BOOKSHELF);
        }
        let rug = [WOOL_RED, WOOL_BLUE, WOOL_YELLOW, WOOL_PURPLE][n % 4];
        self.fill(
            [(x0 + x1) / 2 - 1, 0, (z0 + z1) / 2 - 1],
            [(x0 + x1) / 2 + 1, 0, (z0 + z1) / 2],
            rug,
        );
        // a chimney through the roof
        let span = if ridge_x {
            r.z1 - r.z0 + 1
        } else {
            r.x1 - r.x0 + 1
        };
        let (cx, cz) = if ridge_x {
            (
                if west_door { r.x1 - 2 } else { r.x0 + 2 },
                (r.z0 + r.z1) / 2,
            )
        } else {
            ((r.x0 + r.x1) / 2, r.z1 - 2)
        };
        let top = Self::ridge(span, 4);
        self.column(cx, cz, top..=top + 1, STONE_BRICKS);
        self.lift = 0;
        self.pave_rect(path);
    }

    fn cottages(&mut self) {
        let plaster = Style {
            base: COBBLE,
            wall: PLASTER,
            frame: TIMBER_BEAM,
            roof: SLATE_ROOF,
            floor: PLANKS,
        };
        self.cottage(0, plaster);
        self.cottage(
            1,
            Style {
                base: COBBLE,
                wall: PLANKS,
                frame: LOG,
                roof: THATCH,
                floor: DARK_PLANKS,
            },
        );
        self.cottage(
            2,
            Style {
                base: STONE_BRICKS,
                roof: THATCH,
                ..plaster
            },
        );
        self.cottage(
            3,
            Style {
                roof: RED_ROOF_TILES,
                frame: DARK_PLANKS,
                ..plaster
            },
        );
    }

    /// The barn: dark boards under a red roof, hay and stalls inside.
    fn barn(&mut self) {
        let (r, road) = PADS[4];
        self.stand(road[0], road[1]);
        let style = Style {
            base: COBBLE,
            wall: DARK_PLANKS,
            frame: LOG,
            roof: RED_ROOF_TILES,
            floor: DIRT,
        };
        self.shell(&House {
            r,
            h: 5,
            style,
            ridge_x: false,
            door: None,
            beams: &[],
            windows: &[],
        });
        let mid = (r.x0 + r.x1) / 2;
        // the great door, both leaves swung open, and the hay loft hatch
        self.fill([mid - 1, 1, r.z0], [mid + 1, 3, r.z0], AIR);
        self.fill([mid - 1, 4, r.z0], [mid + 1, 4, r.z0], LOG);
        for x in [mid - 2, mid + 2] {
            self.column(x, r.z0 - 1, 1..=3, WOODEN_DOOR);
        }
        self.column(mid, r.z0, 6..=7, DARK_PLANKS);
        self.put(mid, 7, r.z0, HAY_BALE);
        self.put(mid, 4, r.z0 - 1, LANTERN);
        self.fill([r.x0 + 1, 0, r.z0 + 1], [r.x1 - 1, 0, r.z1 - 1], DIRT);
        self.fill([mid - 1, 0, r.z0], [mid + 1, 0, r.z1 - 1], COBBLE);
        // stalls along both sides
        for z in [r.z0 + 3, r.z0 + 6] {
            self.fill([r.x0 + 1, 1, z], [r.x0 + 2, 1, z], DARK_PLANKS);
            self.fill([r.x1 - 2, 1, z], [r.x1 - 1, 1, z], DARK_PLANKS);
        }
        self.put(r.x0 + 1, 1, r.z0 + 1, BARREL);
        self.put(r.x1 - 1, 1, r.z0 + 1, CRATE);
        self.put(r.x0 + 1, 1, r.z0 + 4, HAY_BALE);
        self.put(r.x1 - 1, 1, r.z0 + 5, HAY_BALE);
        // hay stacked at the back
        for x in r.x0 + 1..r.x1 {
            for z in r.z1 - 2..r.z1 {
                let top = 1 + (hash(x, z, 31) % 3) as i32 - (r.z1 - 1 - z);
                self.column(x, z, 1..=top, HAY_BALE);
            }
        }
        self.fill([r.x0 + 1, 5, r.z0 + 5], [r.x1 - 1, 5, r.z0 + 5], LOG);
        self.put(mid, 4, r.z0 + 5, LANTERN);
        // hay and a trough outside
        self.column(r.x1 + 1, r.z0 + 1, 1..=2, HAY_BALE);
        self.put(r.x1 + 1, 1, r.z0 + 2, HAY_BALE);
        self.put(r.x0 - 1, 1, r.z0 + 1, BARREL);
        self.lift = 0;
        self.pave_rect(Rect {
            x0: mid - 1,
            x1: mid + 1,
            z0: 2,
            z1: r.z0 - 1,
        });
    }

    /// A fenced field: ripe wheat, rows of greens and a scarecrow.
    fn field(&mut self) {
        let (r, c) = PADS[5];
        self.stand(c[0], c[1]);
        for (x, z) in r.ring() {
            let gate = x == r.x0 && (z == -21 || z == -22);
            if !gate && (x + z).rem_euclid(2) == 0 {
                self.put(x, 1, z, TIMBER_BEAM);
            }
        }
        let mid = (r.x0 + r.x1) / 2;
        for z in r.z0 + 1..r.z1 {
            for x in r.x0 + 1..r.x1 {
                self.put(x, 0, z, DIRT);
                if x == mid || z == -21 || z == -22 {
                    self.put(x, 0, z, GRASS);
                } else if x < mid {
                    // wheat, part of it already cut
                    if x % 2 == 0 && (z < -22 || !hash(x, z, 61).is_multiple_of(3)) {
                        self.put(x, 1, z, HAY_BALE);
                    }
                } else if z % 2 == 0 && !hash(x, z, 62).is_multiple_of(5) {
                    self.put(x, 1, z, FLOWER_BED);
                }
            }
        }
        // the scarecrow
        self.column(mid, -19, 1..=2, TIMBER_BEAM);
        self.put(mid - 1, 2, -19, TIMBER_BEAM);
        self.put(mid + 1, 2, -19, TIMBER_BEAM);
        self.put(mid, 3, -19, HAY_BALE);
        self.put(r.x0 + 1, 1, -20, CRATE);
        self.put(r.x0 + 1, 1, -23, BARREL);
        self.lift = 0;
    }

    /// A paddock beside the barn.
    fn paddock(&mut self) {
        let (r, c) = PADS[6];
        self.stand(c[0], c[1]);
        for (x, z) in r.ring() {
            let gate = z == r.z0 && (x == 22 || x == 23);
            if gate {
                continue;
            }
            if (x + z).rem_euclid(2) == 0 {
                self.put(x, 1, z, TIMBER_BEAM);
            }
            if (x == r.x0 || x == r.x1) && (z == r.z0 || z == r.z1) {
                self.column(x, z, 1..=2, TIMBER_BEAM);
            }
        }
        self.column(r.x1 - 1, r.z1 - 1, 1..=2, HAY_BALE);
        self.put(r.x1 - 2, 1, r.z1 - 1, HAY_BALE);
        self.put(r.x1 - 1, 1, r.z1 - 2, HAY_BALE);
        self.put(r.x0 + 1, 1, r.z0 + 1, BARREL);
        self.put(r.x0 + 2, 1, r.z0 + 1, BARREL);
        for (x, z) in [(18, 23), (19, 23), (18, 24), (23, 25), (21, 22)] {
            self.put(x, 0, z, DIRT);
        }
        self.lift = 0;
    }

    /// A duck pond with a little jetty.
    fn pond(&mut self) {
        let (r, c) = PADS[7];
        self.stand(c[0], c[1]);
        let (cx, cz) = ((r.x0 + r.x1) as f32 / 2.0, (r.z0 + r.z1) as f32 / 2.0);
        for z in r.z0..=r.z1 {
            for x in r.x0..=r.x1 {
                let (dx, dz) = ((x as f32 - cx) / 4.6, (z as f32 - cz) / 3.2);
                let d = dx * dx + dz * dz + (hash(x, z, 71) % 5) as f32 * 0.03;
                if d < 0.72 {
                    self.put(x, 0, z, WATER);
                    self.put(x, -1, z, if d < 0.3 { WATER } else { SAND });
                    self.put(x, -2, z, SAND);
                } else if d < 1.1 {
                    let b = if hash(x, z, 72).is_multiple_of(3) {
                        COBBLE
                    } else {
                        SAND
                    };
                    self.put(x, 0, z, b);
                }
            }
        }
        for x in r.x1 - 3..=r.x1 {
            self.put(x, 0, cz as i32, PLANKS);
        }
        self.put(r.x1, 1, cz as i32 + 1, BARREL);
        self.put(r.x0, 1, r.z0, FLOWER_BED);
        self.put(r.x0 + 1, 1, r.z0, FLOWER_BED);
        self.put(r.x1, 1, r.z1, FLOWER_BED);
        self.put(r.x0 + 4, 1, r.z1 + 1, DARK_PLANKS);
        self.put(r.x0 + 5, 1, r.z1 + 1, DARK_PLANKS);
        self.lift = 0;
    }

    /// A small tree on open grass.
    fn tree(&mut self, x: i32, z: i32, birch: bool) {
        self.stand(x, z);
        let clear = (1..=7).all(|y| self.get(x, y, z) == AIR)
            && [[2, 0], [-2, 0], [0, 2], [0, -2]]
                .iter()
                .all(|d| self.get(x + d[0], 5, z + d[1]) == AIR);
        if !clear || self.get(x, 0, z) != GRASS {
            self.lift = 0;
            return;
        }
        let (log, leaf) = if birch {
            (BIRCH_LOG, BIRCH_LEAVES)
        } else {
            (LOG, LEAVES)
        };
        for dy in 4..=7i32 {
            let rad: i32 = if dy >= 6 { 1 } else { 2 };
            for dz in -rad..=rad {
                for dx in -rad..=rad {
                    let corner = dx.abs() == rad && dz.abs() == rad;
                    if !corner
                        || dy == 4 && rad == 2 && hash3(x + dx, dy, z + dz, 2).is_multiple_of(2)
                    {
                        self.put(x + dx, dy, z + dz, leaf);
                    }
                }
            }
        }
        self.column(x, z, 1..=5, log);
        self.lift = 0;
    }

    /// Trees, flower beds and vegetable plots in the open corners.
    fn gardens(&mut self) {
        let trees = [
            (-25, -12, false),
            (-24, -24, true),
            (-24, 12, true),
            (-22, 25, false),
            (16, 27, true),
            (-27, 2, false),
            (12, -27, true),
            (-15, 28, true),
            (-27, -27, false),
            (-22, -28, true),
            (-17, -27, false),
            (-27, -21, true),
            (-21, -20, false),
            (-27, 9, false),
            (-26, -5, true),
            (28, -10, false),
            (12, 16, true),
            (-17, 21, false),
            (28, 17, true),
        ];
        for (x, z, birch) in trees {
            self.tree(x, z, birch);
        }
        // kitchen garden behind the inn
        for z in [-12, -10] {
            for x in -24..=-20 {
                self.stand(x, z);
                if self.get(x, 1, z) == AIR {
                    self.put(x, 1, z, FLOWER_BED);
                }
            }
        }
        // loose beds along the roads
        for (x, z) in [
            (3, -20),
            (-3, -19),
            (12, 3),
            (20, 3),
            (3, 12),
            (-3, 28),
            (27, -3),
        ] {
            self.stand(x, z);
            if self.get(x, 1, z) == AIR && self.get(x, 0, z) == GRASS {
                self.put(x, 1, z, FLOWER_BED);
            }
        }
        self.lift = 0;
    }
}

/// Footprints of the buildings outside the flat middle, and the road cell
/// that gives each its ground level: four cottages and the barn.
const PADS: [(Rect, [i32; 2]); 8] = [
    (
        Rect {
            x0: -10,
            x1: -4,
            z0: -28,
            z1: -20,
        },
        [0, -24],
    ),
    (
        Rect {
            x0: 15,
            x1: 23,
            z0: -10,
            z1: -4,
        },
        [19, 0],
    ),
    (
        Rect {
            x0: 4,
            x1: 10,
            z0: 18,
            z1: 26,
        },
        [0, 22],
    ),
    (
        Rect {
            x0: -12,
            x1: -4,
            z0: 20,
            z1: 26,
        },
        [0, 23],
    ),
    (
        Rect {
            x0: 19,
            x1: 27,
            z0: 5,
            z1: 15,
        },
        [23, 0],
    ),
    // the field
    (
        Rect {
            x0: 13,
            x1: 27,
            z0: -27,
            z1: -16,
        },
        [20, -21],
    ),
    // the paddock
    (
        Rect {
            x0: 14,
            x1: 27,
            z0: 20,
            z1: 27,
        },
        [20, 20],
    ),
    // the pond
    (
        Rect {
            x0: -27,
            x1: -18,
            z0: 21,
            z1: 27,
        },
        [-22, 21],
    ),
];

/// Per cottage: ridge direction, door, and the path from the door to the road.
const COTTAGES: [(bool, Door, Rect); 4] = [
    (
        false,
        (Side::E, -24, Some(-23)),
        Rect {
            x0: -3,
            x1: -2,
            z0: -25,
            z1: -23,
        },
    ),
    (
        true,
        (Side::S, 19, Some(20)),
        Rect {
            x0: 18,
            x1: 20,
            z0: -3,
            z1: -2,
        },
    ),
    (
        false,
        (Side::W, 22, Some(21)),
        Rect {
            x0: 2,
            x1: 3,
            z0: 21,
            z1: 23,
        },
    ),
    (
        true,
        (Side::E, 23, None),
        Rect {
            x0: -3,
            x1: -2,
            z0: 22,
            z1: 24,
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hearthstead_builds_once_and_spares_player_blocks() {
        // the four townsfolk of Hearthstead (others live out in the zones)
        let npcs: Vec<_> = content::Content::default_pack()
            .npcs
            .into_iter()
            .filter(|n| ["brannoc", "maren", "tilda", "hollis"].contains(&n.key.as_str()))
            .collect();
        assert_eq!(npcs.len(), 4);
        // the last two are small worlds, where the town runs into the edges
        for (i, seed) in [20261122u32, 1, 3, 777, 20261122, 5]
            .into_iter()
            .enumerate()
        {
            let size = [[256, 96, 256], [96, 48, 96], [48, 48, 48]][i.saturating_sub(3)];
            let mut w = World::generate(size, seed);
            let h = w.ensure_spawn_hearth(None);
            // a player's chest inside the inn, a player's wall across a road
            let mine = [
                ([h[0] - 15, h[1] + 1, h[2]], CHEST),
                ([h[0], h[1], h[2] + 20], BRICK),
                ([h[0] + 7, h[1] + 9, h[2] - 13], GLASS),
            ];
            for (p, b) in mine {
                w.set(p[0], p[1], p[2], b);
            }
            assert!(build_hearthstead(&mut w, h), "seed {seed}");
            for (p, b) in mine {
                assert_eq!(w.get(p[0], p[1], p[2]), b, "seed {seed}");
            }
            assert_eq!(w.get(h[0], h[1], h[2]), HEARTH);
            assert_eq!(w.find_near(HEARTH, h[0], h[2], 40), Some(h));
            // every NPC stands on a floor near the hearth with headroom
            for n in &npcs {
                let (x, y, z) = (h[0] + n.at[0], h[1] + n.at[1], h[2] + n.at[2]);
                let d2 = n.at[0] * n.at[0] + n.at[2] * n.at[2];
                assert!(d2 <= 22 * 22, "{} is too far", n.key);
                assert!(is_solid(w.get(x, y - 1, z)), "{} has no floor", n.key);
                assert!(!is_solid(w.get(x, y, z)), "{} is in a block", n.key);
                assert!(!is_solid(w.get(x, y + 1, z)), "{} has no headroom", n.key);
            }
            let built = w.clone();
            assert!(!build_hearthstead(&mut w, h), "seed {seed}");
            assert!(w.encode() == built.encode());
        }
    }
}
