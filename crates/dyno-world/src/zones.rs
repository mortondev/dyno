//! The zone around Hearthstead: sub-zones, the Emberrun river, Mirrormere
//! lake, the eastern ridge, roads and the Hollow Mines.
//!
//! The terrain work happens on a height map first (`ground`, `water`,
//! `paint`) and is written to the world by `commit`. Buildings go on top with
//! `put`. Only air and natural blocks are ever replaced: a column that holds
//! anything a player made keeps its terrain, and a player's block is never
//! overwritten.
//!
//! Offsets are (x, z) from the spawn hearth; north is −z, east is +x.

use super::*;
use std::collections::HashSet;

pub(crate) const MOSSY: BlockId = 19;
pub(crate) const DARK_PLANKS: BlockId = 20;
pub(crate) const WOOL_WHITE: BlockId = 23;
pub(crate) const WOOL_BLACK: BlockId = 24;
pub(crate) const WOOL_RED: BlockId = 25;
pub(crate) const WOOL_YELLOW: BlockId = 27;

/// Water level of the generator.
const SEA: i32 = 24;
/// Hearthstead: nothing is touched this close to the hearth.
const TOWN: i32 = 38;

/// Column flags: a road runs here.
pub(crate) const ROAD: u8 = 1;
/// Column flags: river, lake or their banks.
pub(crate) const WET: u8 = 2;
/// Column flags: the levelled ground of a site.
pub(crate) const SITE: u8 = 4;
/// Column flags: a rock wall that roads must not soften.
pub(crate) const ROCK: u8 = 8;

/// Centre of Mirrormere lake and its half-axes.
const LAKE: [i32; 4] = [2, 134, 24, 19];
/// Where the east road crosses the Emberrun.
const BRIDGE: [i32; 2] = [95, 0];
/// The ford on the trail from the windmill.
const FORD: [i32; 2] = [70, 85];
/// The yard in front of the mine entrance.
pub(crate) const YARD: [i32; 2] = [155, -17];
/// The headframe on the ridge above the entrance.
pub(crate) const HEADFRAME: [i32; 2] = [165, -20];
/// Floors of the mine below the yard: galleries, drift, deep seam.
pub(crate) const MINE_DEPTHS: [i32; 3] = [7, 15, 23];

/// Blocks that may be replaced: air and what the generator makes. Clay and
/// emberstone only count where the generator puts them.
pub(crate) fn natural(b: BlockId, y: i32) -> bool {
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
    ) || (b == CLAY && y <= SEA)
        || (b == EMBERSTONE && y < 18)
}

/// Trees and plants: not ground.
pub(crate) fn plant(b: BlockId) -> bool {
    matches!(
        b,
        LOG | LEAVES | GLOWSHROOM | BIRCH_LOG | BIRCH_LEAVES | PINE_LEAVES | BERRY_BUSH
    )
}

fn is_log(b: BlockId) -> bool {
    matches!(b, LOG | BIRCH_LOG)
}

pub(crate) fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn noise(x: i32, z: i32, scale: f32, seed: u32) -> f32 {
    value_noise(x as f32 / scale, z as f32 / scale, seed)
}

/// A hash of a column, for scattering things.
pub(crate) fn rnd(x: i32, z: i32, salt: u32) -> u32 {
    hash(x, z, salt ^ 0x20e5)
}

/// The sub-zones of the map.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Town,
    Hearthmeadow,
    Birchrise,
    Mirrormere,
    Barrowwood,
    EastRoad,
    Ridge,
    Wilds,
}

/// The sub-zone of a column, by its offset from the spawn hearth.
pub fn biome_at(dx: i32, dz: i32) -> Biome {
    if dx.abs().max(dz.abs()) <= TOWN {
        return Biome::Town;
    }
    let n = (noise(dx, dz, 17.0, 0x20e) - 0.5) * 0.24;
    let d = |cx: f32, cz: f32, rx: f32, rz: f32| {
        let (a, b) = ((dx as f32 - cx) / rx, (dz as f32 - cz) / rz);
        (a * a + b * b).sqrt() + n
    };
    if d(0.0, 0.0, 76.0, 76.0) < 1.0 {
        Biome::Hearthmeadow
    } else if d(-30.0, -96.0, 68.0, 52.0) < 1.0 {
        Biome::Birchrise
    } else if d(-124.0, -15.0, 58.0, 82.0) < 1.0 {
        Biome::Barrowwood
    } else if d(0.0, 134.0, 64.0, 50.0) < 1.0 {
        Biome::Mirrormere
    } else if dx >= 138 && (-98..=62).contains(&dz) {
        Biome::Ridge
    } else if d(92.0, 15.0, 60.0, 62.0) < 1.0 {
        Biome::EastRoad
    } else {
        Biome::Wilds
    }
}

/// The buried marker of a finished zone, under the town's own marker.
fn marker(hearth: [i32; 3]) -> [([i32; 3], BlockId); 2] {
    let [x, y, z] = hearth;
    [([x, y - 5, z], FLAGSTONES), ([x, y - 6, z], TIMBER_BEAM)]
}

fn built(world: &World, hearth: [i32; 3]) -> bool {
    marker(hearth)
        .iter()
        .all(|(p, b)| world.get(p[0], p[1], p[2]) == *b)
}

/// Stamp the zone into the world around the spawn hearth: terrain, river,
/// roads, landmarks and the mines. Returns true if the world changed, false
/// if the zone is already there. Whatever does not fit in the world is left
/// out.
pub fn build_world_features(world: &mut World, hearth: [i32; 3]) -> bool {
    if built(world, hearth) {
        return false;
    }
    {
        let mut z = Zone::new(world, hearth);
        z.ridge();
        z.birch_hills();
        z.lake();
        z.river();
        let sites = poi::plan(&mut z);
        z.roads();
        z.commit();
        poi::build(&mut z, &sites);
        z.mines();
        z.dress();
        z.graves();
        z.furniture();
        z.prune();
        z.bury_lanterns();
    }
    // the marker is forced: a buried ore must not make the zone rebuild
    for (p, b) in marker(hearth) {
        world.set(p[0], p[1], p[2], b);
    }
    true
}

/// Is a box of columns, given as offsets from the hearth, inside the world
/// with a margin?
fn fits_in(size: [u32; 3], hearth: [i32; 3], x: [i32; 2], z: [i32; 2]) -> bool {
    let m = 3;
    hearth[0] + x[0] >= m
        && hearth[2] + z[0] >= m
        && hearth[0] + x[1] < size[0] as i32 - m
        && hearth[2] + z[1] < size[2] as i32 - m
}

fn lake_fits(size: [u32; 3], hearth: [i32; 3]) -> bool {
    fits_in(size, hearth, [-40, 110], [-20, 172])
}

fn ridge_fits(size: [u32; 3], hearth: [i32; 3]) -> bool {
    fits_in(size, hearth, [128, 176], [-70, 30])
}

fn mine_fits(size: [u32; 3], hearth: [i32; 3]) -> bool {
    ridge_fits(size, hearth) && fits_in(size, hearth, [142, 184], [-118, -10])
}

/// The standing places of the landmarks, as (key, block position): the spot
/// a quest should point at. This does not look at the world, so every
/// landmark is listed and `y` is the height of the hearth; use
/// [`poi_positions_in`] for the heights and for what was really built.
pub fn poi_positions(hearth: [i32; 3]) -> Vec<(&'static str, [i32; 3])> {
    ANCHORS
        .iter()
        .map(|a| (a.0, [hearth[0] + a.1[0], hearth[1], hearth[2] + a.1[1]]))
        .collect()
}

/// The standing places of the landmarks that fit in this world, with the
/// height of the free block above the floor.
pub fn poi_positions_in(world: &World, hearth: [i32; 3]) -> Vec<(&'static str, [i32; 3])> {
    let size = world.size;
    let floor = |dx: i32, dz: i32| {
        let (x, z) = (hearth[0] + dx, hearth[2] + dz);
        let mut y = size[1] as i32 - 1;
        while y > 0 {
            let b = world.get(x, y, z);
            if is_solid(b) && !plant(b) {
                break;
            }
            y -= 1;
        }
        y
    };
    let mut out = Vec::new();
    for (key, at, need, depth) in ANCHORS {
        let [dx, dz] = at;
        // the same tests the builders make
        let ok = match need {
            Need::Pad([cx, cz, rx, rz]) => fits_in(
                size,
                hearth,
                [cx - rx - 8, cx + rx + 8],
                [cz - rz - 8, cz + rz + 8],
            ),
            Need::Lake => lake_fits(size, hearth),
            Need::Ridge => ridge_fits(size, hearth),
            Need::Mine => mine_fits(size, hearth),
        };
        if !ok {
            continue;
        }
        // underground places hang below a spot on the surface
        let y = match depth {
            Below::Surface => floor(dx, dz),
            Below::Yard(d) => floor(YARD[0], YARD[1]) - d,
            Below::Barrow(d) => floor(-116, 10) - d,
        };
        if key.starts_with("mine_") && floor(YARD[0], YARD[1]) - MINE_DEPTHS[2] < 4 {
            continue;
        }
        out.push((key, [hearth[0] + dx, y + 1, hearth[2] + dz]));
    }
    out
}

#[derive(Clone, Copy)]
enum Need {
    /// the levelled ground of the site: centre and half-sizes
    Pad([i32; 4]),
    Lake,
    Ridge,
    Mine,
}

#[derive(Clone, Copy)]
enum Below {
    Surface,
    Yard(i32),
    Barrow(i32),
}

/// Key, offset from the hearth, what must fit, and where the floor is.
const ANCHORS: [(&str, [i32; 2], Need, Below); 22] = [
    (
        "north_watch",
        [0, -51],
        Need::Pad([0, -56, 5, 5]),
        Below::Surface,
    ),
    (
        "logging_camp",
        [-45, -86],
        Need::Pad([-45, -88, 11, 9]),
        Below::Surface,
    ),
    (
        "hollow_oak",
        [-87, -53],
        Need::Pad([-100, -55, 8, 8]),
        Below::Surface,
    ),
    (
        "old_barrow",
        [-116, 10],
        Need::Pad([-130, 10, 14, 13]),
        Below::Surface,
    ),
    (
        "barrow_crypt",
        [-133, 10],
        Need::Pad([-130, 10, 14, 13]),
        Below::Barrow(4),
    ),
    (
        "barrowwood_graves",
        [-110, -26],
        Need::Pad([-105, -22, 5, 4]),
        Below::Surface,
    ),
    (
        "windmill_farm",
        [10, 95],
        Need::Pad([24, 96, 17, 11]),
        Below::Surface,
    ),
    ("fishers_landing", [-28, 136], Need::Lake, Below::Surface),
    ("sunken_shrine", [8, 159], Need::Lake, Below::Surface),
    (
        "wrecked_caravan",
        [50, -9],
        Need::Pad([50, -12, 4, 2]),
        Below::Surface,
    ),
    (
        "ashcloak_stockade",
        [62, 72],
        Need::Pad([62, 60, 11, 11]),
        Below::Surface,
    ),
    (
        "ashcloak_stockade_yard",
        [62, 61],
        Need::Pad([62, 60, 11, 11]),
        Below::Surface,
    ),
    ("emberford_bridge", [95, 0], Need::Lake, Below::Surface),
    ("emberford_toll_hut", [84, 3], Need::Lake, Below::Surface),
    ("emberrun_ford", [61, 87], Need::Lake, Below::Surface),
    (
        "miners_rest",
        [128, 10],
        Need::Pad([128, 8, 10, 8]),
        Below::Surface,
    ),
    ("ridge_cut", [145, -5], Need::Ridge, Below::Surface),
    ("hollow_mines", [157, -19], Need::Mine, Below::Surface),
    ("mine_headframe", [168, -24], Need::Mine, Below::Surface),
    (
        "mine_upper_galleries",
        [167, -47],
        Need::Mine,
        Below::Yard(7),
    ),
    (
        "mine_flooded_drift",
        [150, -82],
        Need::Mine,
        Below::Yard(15),
    ),
    ("mine_deep_seam", [174, -106], Need::Mine, Below::Yard(23)),
];

#[derive(Clone, Copy, PartialEq)]
enum Way {
    /// flagstones and gravel, three wide, with lamps
    Road,
    /// a dirt track
    Trail,
}

/// The crossing of the east road over the river.
#[derive(Clone, Copy)]
pub(crate) struct Bridge {
    /// the first dry columns west and east of the water
    pub(crate) x: [i32; 2],
    /// the block the road runs on
    pub(crate) deck: i32,
}

pub(crate) struct Zone<'a> {
    pub(crate) w: &'a mut World,
    /// the world before the zone: decides what may be replaced
    orig: World,
    /// the spawn hearth
    pub(crate) o: [i32; 3],
    /// origin of the site being built, as (x offset, ground y, z offset)
    pub(crate) at: [i32; 3],
    sx: i32,
    sy: i32,
    sz: i32,
    /// planned ground per column: the highest block of earth
    g: Vec<i32>,
    /// planned water surface per column: the highest water block, 0 for none
    wl: Vec<i32>,
    /// planned surface block per column, AIR for whatever grows there
    paint: Vec<BlockId>,
    /// what `commit` last wrote
    done: Vec<(i32, i32, BlockId)>,
    /// columns that keep their terrain: the town and what players built
    lock: Vec<bool>,
    mask: Vec<u8>,
    /// columns to clear of trees at the next commit
    strip: Vec<bool>,
    /// columns that lost a tree: leaves near them are pruned at the end
    bare: Vec<bool>,
    /// blocks of the server's beacons, which the landmarks replace
    freed: HashSet<[i32; 3]>,
    /// samples of the roads, for lamps
    ways: Vec<Vec<[f32; 2]>>,
    pub(crate) lake_level: Option<i32>,
    pub(crate) bridge: Option<Bridge>,
    pub(crate) has_ridge: bool,
}

impl<'a> Zone<'a> {
    fn new(w: &'a mut World, hearth: [i32; 3]) -> Self {
        let freed = take_beacons(w, hearth);
        let orig = w.clone();
        let [sx, sy, sz] = [w.size[0] as i32, w.size[1] as i32, w.size[2] as i32];
        let n = (sx * sz) as usize;
        let mut z = Zone {
            w,
            orig,
            o: hearth,
            at: [0, 0, 0],
            sx,
            sy,
            sz,
            g: vec![0; n],
            wl: vec![0; n],
            paint: vec![AIR; n],
            done: Vec::new(),
            lock: vec![false; n],
            mask: vec![0; n],
            strip: vec![false; n],
            bare: vec![false; n],
            freed,
            ways: Vec::new(),
            lake_level: None,
            bridge: None,
            has_ridge: false,
        };
        z.scan();
        z
    }

    /// Read the ground, the water and the locked columns from the world.
    fn scan(&mut self) {
        for wz in 0..self.sz {
            for wx in 0..self.sx {
                let i = (wz * self.sx + wx) as usize;
                let (mut ground, mut water, mut lock) = (-1, 0, false);
                for y in (1..self.sy).rev() {
                    let b = self.w.get(wx, y, wz);
                    if b == AIR || plant(b) {
                        continue;
                    }
                    if b == WATER {
                        if ground < 0 && water == 0 {
                            water = y;
                        }
                        continue;
                    }
                    if !natural(b, y) && b != BEDROCK {
                        lock = true;
                    }
                    if ground < 0 {
                        ground = y;
                    }
                }
                let (dx, dz) = (wx - self.o[0], wz - self.o[2]);
                self.g[i] = ground.max(1);
                self.wl[i] = water;
                self.lock[i] = lock || dx.abs().max(dz.abs()) <= TOWN;
            }
        }
        self.done = (0..self.g.len())
            .map(|i| (self.g[i], self.wl[i], AIR))
            .collect();
    }

    // ------------------------------------------------------------ height map

    fn idx(&self, x: i32, z: i32) -> Option<usize> {
        let (wx, wz) = (self.o[0] + x, self.o[2] + z);
        (wx >= 0 && wz >= 0 && wx < self.sx && wz < self.sz).then(|| (wz * self.sx + wx) as usize)
    }

    pub(crate) fn fits(&self, x: [i32; 2], z: [i32; 2]) -> bool {
        fits_in(self.w.size, self.o, x, z)
    }

    /// Planned ground of a column: the y of its highest block of earth.
    pub(crate) fn ground(&self, x: i32, z: i32) -> i32 {
        self.idx(x, z).map_or(self.o[1] - 1, |i| self.g[i])
    }

    /// Planned water surface of a column, 0 if it is dry.
    pub(crate) fn water(&self, x: i32, z: i32) -> i32 {
        self.idx(x, z).map_or(0, |i| {
            if self.wl[i] > self.g[i] {
                self.wl[i]
            } else {
                0
            }
        })
    }

    pub(crate) fn flags(&self, x: i32, z: i32) -> u8 {
        self.idx(x, z).map_or(0, |i| self.mask[i])
    }

    pub(crate) fn locked(&self, x: i32, z: i32) -> bool {
        self.idx(x, z).is_none_or(|i| self.lock[i])
    }

    pub(crate) fn flag(&mut self, x: i32, z: i32, f: u8) {
        if let Some(i) = self.idx(x, z)
            && !self.lock[i]
        {
            self.mask[i] |= f;
            self.strip[i] = true;
        }
    }

    pub(crate) fn set_ground(&mut self, x: i32, z: i32, y: i32) {
        if let Some(i) = self.idx(x, z)
            && !self.lock[i]
        {
            // the cap near the top of the world never lowers a column
            let cap = (self.sy - 24).max(self.g[i]);
            self.g[i] = y.max(3).min(cap);
        }
    }

    fn set_water(&mut self, x: i32, z: i32, y: i32) {
        if let Some(i) = self.idx(x, z)
            && !self.lock[i]
        {
            self.wl[i] = y;
        }
    }

    /// Choose the surface block of a column.
    pub(crate) fn set_paint(&mut self, x: i32, z: i32, b: BlockId) {
        if let Some(i) = self.idx(x, z)
            && !self.lock[i]
        {
            self.paint[i] = b;
        }
    }

    /// Level a rectangle and blend its edges into the land around it.
    /// Returns the level: the y of the ground blocks.
    pub(crate) fn pad(&mut self, c: [i32; 2], r: [i32; 2], lift: i32) -> i32 {
        let mut hs = Vec::new();
        let mut wet = 0;
        for z in c[1] - r[1] - 2..=c[1] + r[1] + 2 {
            for x in c[0] - r[0] - 2..=c[0] + r[0] + 2 {
                hs.push(self.ground(x, z));
                wet = wet.max(self.water(x, z));
            }
        }
        hs.sort_unstable();
        let level = (hs[hs.len() / 2] + lift).max(wet + 1);
        self.pad_at(c, r, level, 6);
        level
    }

    /// Level a rectangle at a given height, with a skirt of `blend` columns.
    pub(crate) fn pad_at(&mut self, c: [i32; 2], r: [i32; 2], level: i32, blend: i32) {
        for z in c[1] - r[1] - blend..=c[1] + r[1] + blend {
            for x in c[0] - r[0] - blend..=c[0] + r[0] + blend {
                let d = ((x - c[0]).abs() - r[0])
                    .max((z - c[1]).abs() - r[1])
                    .max(0);
                if d == 0 {
                    self.set_ground(x, z, level);
                    self.flag(x, z, SITE);
                } else if self.flags(x, z) & (SITE | WET) == 0 {
                    let g = self.ground(x, z).clamp(level - d, level + d);
                    self.set_ground(x, z, g);
                }
            }
        }
    }

    /// Stop water from standing beside air: raise dry banks to the water
    /// level, and put a stone sill where the river steps down.
    fn seal(&mut self) {
        for _ in 0..4 {
            let mut changed = false;
            for z in -self.o[2]..self.sz - self.o[2] {
                for x in -self.o[0]..self.sx - self.o[0] {
                    let Some(i) = self.idx(x, z) else { continue };
                    if self.wl[i] <= self.g[i] {
                        self.wl[i] = 0;
                        continue;
                    }
                    let level = self.wl[i];
                    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let Some(n) = self.idx(x + dx, z + dz) else {
                            continue;
                        };
                        let wet = self.wl[n] > self.g[n];
                        if (wet && self.wl[n] >= level) || (!wet && self.g[n] >= level) {
                            continue;
                        }
                        changed = true;
                        if !wet && !self.lock[n] {
                            self.g[n] = level;
                        } else if !self.lock[i] {
                            // a sill: this column becomes a rock at the surface
                            self.g[i] = level;
                            self.wl[i] = 0;
                            self.paint[i] = if rnd(x, z, 3).is_multiple_of(3) {
                                MOSSY
                            } else {
                                COBBLE
                            };
                            break;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// How far a column stands above its lowest neighbour.
    fn drop_of(&self, x: i32, z: i32) -> i32 {
        let g = self.ground(x, z);
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .map(|d| g - self.ground(x + d.0, z + d.1))
            .max()
            .unwrap_or(0)
    }

    /// Write the planned terrain to the world: move the earth, turf it, fill
    /// the water. Can be called again after more planning.
    pub(crate) fn commit(&mut self) {
        self.seal();
        let mut wetted = Vec::new();
        for wz in 0..self.sz {
            for wx in 0..self.sx {
                let i = (wz * self.sx + wx) as usize;
                let now = (self.g[i], self.wl[i], self.paint[i]);
                if now == self.done[i] && !self.strip[i] {
                    continue;
                }
                let (x, z) = (wx - self.o[0], wz - self.o[2]);
                let (g0, w0, _) = self.done[i];
                let (g1, w1, paint) = now;
                // plants: everything on roads and sites, else what stood on
                // the moved ground
                let (lo, hi) = (g0.min(g1), g0.max(g1));
                let mut fell = self.strip[i];
                if !fell && g0 != g1 {
                    fell = (lo + 1..=hi + 2).any(|y| is_log(self.w.get(wx, y, wz)));
                    for y in lo + 1..=hi + 2 {
                        if plant(self.w.get(wx, y, wz)) {
                            self.w.set(wx, y, wz, AIR);
                        }
                    }
                }
                if fell {
                    for y in lo + 1..self.sy {
                        let b = self.w.get(wx, y, wz);
                        if is_log(b) || (self.strip[i] && plant(b)) {
                            self.w.set(wx, y, wz, AIR);
                        }
                    }
                    self.bare[i] = true;
                }
                self.strip[i] = false;
                if now == self.done[i] {
                    continue;
                }
                for y in g1 + 1..=g0 {
                    self.w.set(wx, y, wz, AIR);
                }
                for y in g0 + 1..g1 {
                    self.w.set(wx, y, wz, STONE);
                }
                let wet = w1 > g1;
                let steep = self.drop_of(x, z) >= 3;
                let shore = !wet
                    && g1 <= SEA + 1
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .any(|d| self.water(x + d.0, z + d.1) > 0);
                let top = if paint != AIR {
                    paint
                } else if wet {
                    if rnd(x, z, 9).is_multiple_of(9) {
                        CLAY
                    } else {
                        SAND
                    }
                } else if g1 >= 58 + (rnd(x, z, 1) % 4) as i32 {
                    SNOW
                } else if steep || g1 > 50 {
                    STONE
                } else if shore {
                    SAND
                } else {
                    GRASS
                };
                self.w.set(wx, g1, wz, top);
                if g1 != g0 {
                    let sub = match top {
                        SAND | CLAY => SAND,
                        STONE | SNOW | COBBLE | MOSSY => STONE,
                        _ => DIRT,
                    };
                    for y in g1 - 3..g1 {
                        let b = self.w.get(wx, y, wz);
                        if matches!(b, AIR | GRASS | DIRT | SAND | STONE | SNOW | WATER | CLAY) {
                            self.w.set(wx, y, wz, sub);
                        }
                    }
                }
                for y in g1 + 1..=w1 {
                    self.w.set(wx, y, wz, WATER);
                }
                for y in g1.max(w1) + 1..=w0 {
                    if self.w.get(wx, y, wz) == WATER {
                        self.w.set(wx, y, wz, AIR);
                    }
                }
                if wet {
                    // keep the bed closed over caves
                    for y in g1 - 2..g1 {
                        if self.w.get(wx, y, wz) == AIR {
                            self.w.set(wx, y, wz, STONE);
                        }
                    }
                }
                self.done[i] = now;
                if wet {
                    wetted.push((x, z));
                }
            }
        }
        // a cave that opens into the side of new water is walled up
        for (x, z) in wetted {
            let (g, level) = (self.ground(x, z), self.water(x, z));
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if self.locked(x + dx, z + dz) {
                    continue;
                }
                let (wx, wz) = (self.o[0] + x + dx, self.o[2] + z + dz);
                for y in g + 1..=level {
                    if self.w.get(wx, y, wz) == AIR {
                        self.w.set(wx, y, wz, STONE);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------- terrain

    /// The line of the cut through the ridge, from its foot to the mine yard.
    fn cut_distance(x: i32, z: i32) -> f32 {
        let line = [[136.0, 4.0], [145.0, -5.0], [150.0, -10.0], [155.0, -17.0]];
        let mut best = f32::MAX;
        for s in line.windows(2) {
            let (a, b) = (s[0], s[1]);
            let (vx, vz): (f32, f32) = (b[0] - a[0], b[1] - a[1]);
            let t = (((x as f32 - a[0]) * vx + (z as f32 - a[1]) * vz) / (vx * vx + vz * vz))
                .clamp(0.0, 1.0);
            let (px, pz) = (a[0] + vx * t - x as f32, a[1] + vz * t - z as f32);
            best = best.min((px * px + pz * pz).sqrt());
        }
        best
    }

    /// Raise the eastern ridge: a wall of rock with a plateau behind it, a
    /// five-wide cut where the road goes through, and a bowl for the mine yard.
    fn ridge(&mut self) {
        if !ridge_fits(self.w.size, self.o) {
            return;
        }
        self.has_ridge = true;
        for z in -98..=62 {
            let foot = 139.0 + (value_noise(z as f32 / 19.0, 0.5, 71) - 0.5) * 9.0;
            let taper = smooth((z + 98) as f32 / 26.0) * smooth((62 - z) as f32 / 26.0);
            for x in 128..=self.sx - self.o[0] {
                let cut = Self::cut_distance(x, z);
                if cut < 2.6 || poi::in_yard(x, z) {
                    continue;
                }
                let t = smooth((x as f32 - foot) / 5.0);
                let rim = (1.0 - ((x as f32 - foot - 8.0) / 5.0).powi(2)).max(0.0);
                let rise = taper * (t * (12.0 + 5.0 * noise(x, z, 14.0, 72)) + 4.0 * rim);
                if rise >= 0.5 {
                    self.set_ground(x, z, self.ground(x, z) + rise.round() as i32);
                    self.set_water(x, z, 0);
                    if cut < 7.0 || (x - YARD[0]).abs().max((z - YARD[1]).abs()) < 12 {
                        self.flag(x, z, ROCK);
                    }
                }
            }
        }
    }

    /// Lift Birchrise into hills, and raise a knoll for the North Watch.
    fn birch_hills(&mut self) {
        let hills = self.fits([-64, -26], [-105, -71]);
        let knoll = self.fits([-13, 13], [-69, -43]);
        for z in -170..=-20 {
            for x in -110..=50 {
                let (ax, az) = ((x + 30) as f32 / 78.0, (z + 94) as f32 / 62.0);
                let t = smooth(1.0 - (ax * ax + az * az).sqrt());
                let bump = 0.6 * noise(x, z, 26.0, 81) + 0.4 * noise(x, z, 11.0, 82);
                let mut rise = if hills { t * (3.0 + 15.0 * bump) } else { 0.0 };
                let k = (((x * x) + (z + 56) * (z + 56)) as f32).sqrt();
                if knoll {
                    rise = rise.max(5.0 * smooth(1.0 - k / 16.0));
                }
                if rise >= 0.5 && self.idx(x, z).is_some() {
                    let floor = self.ground(x, z).max(self.water(x, z) - 2);
                    self.set_ground(x, z, floor + rise.round() as i32);
                }
            }
        }
    }

    /// How deep inside the lake a column is: below 1 is water.
    fn lake_metric(x: i32, z: i32) -> f32 {
        let e = |c: [f32; 2], r: [f32; 2]| {
            let (a, b) = ((x as f32 - c[0]) / r[0], (z as f32 - c[1]) / r[1]);
            (a * a + b * b).sqrt()
        };
        let main = e(
            [LAKE[0] as f32, LAKE[1] as f32],
            [LAKE[2] as f32, LAKE[3] as f32],
        ) + (noise(x, z, 9.0, 91) - 0.5) * 0.3;
        // the bay of the Sunken Shrine
        main.min(e([20.0, 154.0], [10.0, 8.0]))
    }

    /// Dig Mirrormere.
    fn lake(&mut self) {
        if !lake_fits(self.w.size, self.o) {
            return;
        }
        let [cx, cz, rx, rz] = LAKE;
        let mut hs = Vec::new();
        for z in cz - rz..=cz + rz {
            for x in cx - rx..=cx + rx {
                if Self::lake_metric(x, z) < 0.8 {
                    hs.push(self.ground(x, z).max(self.water(x, z)));
                }
            }
        }
        hs.sort_unstable();
        let level = (hs[hs.len() / 5] - 1).max(SEA);
        self.lake_level = Some(level);
        for z in cz - rz - 22..=cz + rz + 22 {
            for x in cx - rx - 22..=cx + rx + 22 {
                let e = Self::lake_metric(x, z);
                if e < 1.0 {
                    let depth = 1 + (4.0 * smooth((1.0 - e) * 2.2)).round() as i32;
                    let bed = self.ground(x, z).min(level - depth);
                    self.set_ground(x, z, bed);
                    self.set_water(x, z, level);
                    self.flag(x, z, WET);
                } else if e < 1.6 {
                    let out = (e - 1.0) * rz as f32;
                    let cap = level + 1 + (out * 0.5) as i32;
                    if self.ground(x, z) > cap {
                        self.set_ground(x, z, cap);
                    }
                    if out < 2.5 && self.ground(x, z) <= level + 1 {
                        self.set_paint(x, z, SAND);
                    }
                    if out < 1.5 {
                        self.flag(x, z, WET);
                    }
                }
            }
        }
    }

    /// Carve the Emberrun from the high ground in the north-east down to the
    /// lake. The water level only ever steps down on the way.
    fn river(&mut self) {
        let Some(lake) = self.lake_level else { return };
        let pts: [[f32; 3]; 15] = [
            [84.0, -172.0, 4.0],
            [90.0, -150.0, 2.5],
            [84.0, -122.0, 3.0],
            [94.0, -96.0, 3.0],
            [101.0, -66.0, 3.0],
            [93.0, -36.0, 3.5],
            [95.0, -14.0, 6.0],
            [95.0, 0.0, 7.0],
            [95.0, 14.0, 6.0],
            [89.0, 34.0, 3.5],
            [85.0, 58.0, 3.5],
            [FORD[0] as f32, FORD[1] as f32, 5.5],
            [52.0, 104.0, 3.5],
            [36.0, 118.0, 4.0],
            [24.0, 127.0, 4.0],
        ];
        let mut line = spline(&pts, 1.0);
        // only the part inside the world, counted from the lake
        let keep = line
            .iter()
            .rposition(|p| self.idx(p[0] as i32, p[1] as i32).is_none())
            .map_or(0, |i| i + 1);
        line.drain(..keep);
        // the level: one below the lowest bank, never rising downstream
        let mut level = Vec::with_capacity(line.len());
        let mut run = i32::MAX;
        for p in &line {
            let (cx, cz, r) = (p[0] as i32, p[1] as i32, p[2].ceil() as i32 + 1);
            let mut lo = i32::MAX;
            for z in cz - r..=cz + r {
                for x in cx - r..=cx + r {
                    if self.idx(x, z).is_none() {
                        continue;
                    }
                    let wet = self.water(x, z);
                    lo = lo.min(if wet > 0 { wet + 1 } else { self.ground(x, z) });
                }
            }
            run = run.min(lo - 1).max(lake);
            level.push(run);
        }
        let n = line.len();
        for (i, l) in level.iter_mut().enumerate() {
            *l = (*l).min(lake + ((n - 1 - i) / 9) as i32);
        }
        // every column takes the nearest stretch of river
        let mut best: Vec<(f32, u32)> = vec![(f32::MAX, 0); self.g.len()];
        for (i, p) in line.iter().enumerate() {
            let r = p[2].ceil() as i32 + 10;
            let (cx, cz) = (p[0].round() as i32, p[1].round() as i32);
            for z in cz - r..=cz + r {
                for x in cx - r..=cx + r {
                    let Some(c) = self.idx(x, z) else { continue };
                    let d = ((x as f32 - p[0]).powi(2) + (z as f32 - p[1]).powi(2)).sqrt();
                    let m = d - p[2];
                    if m < best[c].0 {
                        best[c] = (m, i as u32);
                    }
                }
            }
        }
        for z in -self.o[2]..self.sz - self.o[2] {
            for x in -self.o[0]..self.sx - self.o[0] {
                let Some(c) = self.idx(x, z) else { continue };
                let (m, i) = best[c];
                if m >= 9.0 || Self::lake_metric(x, z) < 1.0 {
                    continue;
                }
                let p = line[i as usize];
                let l = level[i as usize];
                let ford = (p[0] - FORD[0] as f32).hypot(p[1] - FORD[1] as f32) < 6.0;
                if m <= 0.0 {
                    let depth = if ford {
                        1
                    } else {
                        1 + (2.0 * smooth(-m / p[2] * 1.6)).round() as i32
                    };
                    self.set_ground(x, z, self.ground(x, z).min(l - depth));
                    self.set_water(x, z, l);
                    self.flag(x, z, WET);
                    if ford {
                        let b = [COBBLE, FLAGSTONES, COBBLE, SAND][(rnd(x, z, 12) % 4) as usize];
                        self.set_paint(x, z, b);
                    }
                } else {
                    let cap = l + 1 + (m * 0.8) as i32;
                    if self.ground(x, z) > cap {
                        self.set_ground(x, z, cap);
                    }
                    if m <= 1.3 {
                        self.flag(x, z, WET);
                        if rnd(x, z, 13).is_multiple_of(4) && self.ground(x, z) <= l + 1 {
                            self.set_paint(x, z, if ford { COBBLE } else { SAND });
                        }
                    }
                }
            }
        }
        // stepping stones across the ford, on the line of the trail
        for x in FORD[0] - 9..=FORD[0] + 9 {
            let z = FORD[1] + ((FORD[0] - x) as f32 * 0.2).round() as i32;
            for k in 0..=1 {
                let level = self.water(x, z + k);
                if level > 0 && (x + k).rem_euclid(2) == 0 && !self.locked(x, z + k) {
                    self.set_ground(x, z + k, level);
                    self.set_water(x, z + k, 0);
                    self.set_paint(x, z + k, if x % 3 == 0 { MOSSY } else { FLAGSTONES });
                }
            }
        }
        // the bridge: where the water ends on the line of the road
        let [bx, bz] = BRIDGE;
        let water = self.water(bx, bz);
        if water > 0 {
            let dry = |s: &Self, x: i32| (-2..=2).all(|k| s.water(x, bz + k) == 0);
            let west = (1..20).map(|k| bx - k).find(|x| dry(self, *x));
            let east = (1..20).map(|k| bx + k).find(|x| dry(self, *x));
            if let (Some(w), Some(e)) = (west, east) {
                self.bridge = Some(Bridge {
                    x: [w, e],
                    deck: water + 3,
                });
            }
        }
    }

    // --------------------------------------------------------------- roads

    fn roads(&mut self) {
        // north gate, past the North Watch, west to the Logging Camp
        self.way(
            Way::Road,
            &[
                [0, -39],
                [-1, -46],
                [-7, -53],
                [-9, -62],
                [-20, -72],
                [-33, -86],
                [-56, -86],
            ],
        );
        // on to the Hollow Oak and the Old Barrow
        self.way(
            Way::Trail,
            &[
                [-56, -86],
                [-72, -80],
                [-84, -66],
                [-88, -53],
                [-100, -38],
                [-110, -20],
                [-119, -2],
                [-116, 10],
            ],
        );
        // south gate to the windmill and Fisher's Landing
        self.way(
            Way::Road,
            &[
                [0, 39],
                [2, 55],
                [5, 74],
                [6, 90],
                [4, 101],
                [-8, 109],
                [-22, 115],
                [-30, 124],
                [-30, 133],
            ],
        );
        // round the south shore to the Sunken Shrine
        self.way(
            Way::Trail,
            &[[-31, 144], [-24, 155], [-10, 160], [3, 161], [9, 159]],
        );
        // east gate, over the bridge, to Miner's Rest and the mine yard
        self.way(
            Way::Road,
            &[
                [39, 0],
                [46, -4],
                [52, -7],
                [62, -6],
                [74, -2],
                [85, 0],
                [105, 0],
                [116, 4],
                [124, 7],
                [136, 7],
                [140, 1],
                [145, -5],
                [150, -10],
                [YARD[0], YARD[1]],
            ],
        );
        // from the windmill over the ford and up the east bank
        self.way(
            Way::Trail,
            &[
                [5, 80],
                [22, 82],
                [44, 85],
                [58, 87],
                [FORD[0], FORD[1]],
                [82, 82],
                [94, 68],
                [103, 48],
                [110, 28],
                [116, 14],
                [120, 7],
            ],
        );
        // the spur to the stockade
        self.way(Way::Trail, &[[58, 87], [60, 78], [62, 71]]);
        // the last ragged steps inside the town's edge
        for dir in [[0, -1], [1, 0], [0, 1]] {
            for i in 34..=TOWN {
                for k in -1..=1 {
                    let (x, z) = (dir[0] * i - dir[1] * k, dir[1] * i + dir[0] * k);
                    let y = self.ground(x, z);
                    let (wx, wz) = (self.o[0] + x, self.o[2] + z);
                    if matches!(self.w.get(wx, y, wz), GRASS | DIRT)
                        && self.w.get(wx, y + 1, wz) == AIR
                    {
                        let b = if rnd(x, z, 91).is_multiple_of(9) {
                            COBBLE
                        } else {
                            FLAGSTONES
                        };
                        self.w.set(wx, y, wz, b);
                    }
                }
            }
        }
    }

    /// Lay a road or a trail along some points: plan its slope so it climbs
    /// at most one block in two, shape the ground under it and pave it.
    fn way(&mut self, kind: Way, pts: &[[i32; 2]]) {
        let pts: Vec<[f32; 3]> = pts
            .iter()
            .map(|p| [p[0] as f32, p[1] as f32, 0.0])
            .collect();
        let mut line = spline(&pts, 0.5);
        if let Some(end) = line.iter().position(|p| {
            !self.fits(
                [p[0] as i32 - 6, p[0] as i32 + 6],
                [p[1] as i32 - 6, p[1] as i32 + 6],
            )
        }) {
            line.truncate(end);
        }
        if line.len() < 16 {
            return;
        }
        let n = line.len();
        let cell = |p: &[f32; 3]| (p[0].round() as i32, p[1].round() as i32);
        // the natural profile, with the fixed points: sites, town, bridge
        let mut raw = vec![f32::NAN; n];
        let mut pin = vec![false; n];
        let mut skip = vec![false; n];
        for (i, p) in line.iter().enumerate() {
            let (x, z) = cell(p);
            let f = self.flags(x, z);
            if let Some(b) = self.bridge
                && (b.x[0] - 2..=b.x[1] + 2).contains(&x)
                && (z - BRIDGE[1]).abs() <= 3
            {
                raw[i] = b.deck as f32;
                pin[i] = true;
                skip[i] = (b.x[0]..=b.x[1]).contains(&x);
            } else if f & WET != 0 {
                skip[i] = true;
            } else {
                raw[i] = self.ground(x, z) as f32;
                pin[i] = f & SITE != 0
                    || self.locked(x, z)
                    || i == 0 && pts[0][0].abs().max(pts[0][1].abs()) <= 39.0;
            }
        }
        let mut p: Vec<f32> = (0..n)
            .map(|i| {
                if pin[i] {
                    return raw[i];
                }
                let (mut sum, mut k) = (0.0, 0.0);
                for v in &raw[i.saturating_sub(10)..(i + 11).min(n)] {
                    if !v.is_nan() {
                        sum += v;
                        k += 1.0;
                    }
                }
                if k > 0.0 { sum / k } else { f32::NAN }
            })
            .collect();
        // gaps (water) take the last known height
        let mut last = p.iter().copied().find(|v| !v.is_nan()).unwrap_or(0.0);
        for v in p.iter_mut() {
            if v.is_nan() {
                *v = last;
            }
            last = *v;
        }
        // limit the slope, then pull the road to its fixed points
        let s = 0.25;
        for i in 1..n {
            if !pin[i] {
                p[i] = p[i].min(p[i - 1] + s);
            }
        }
        for i in (0..n - 1).rev() {
            if !pin[i] {
                p[i] = p[i].min(p[i + 1] + s);
            }
        }
        let pins: Vec<(usize, f32)> = (0..n).filter(|i| pin[*i]).map(|i| (i, p[i])).collect();
        for (i, v) in p.iter_mut().enumerate() {
            if pin[i] {
                continue;
            }
            for (j, h) in &pins {
                let reach = s * (i as f32 - *j as f32).abs();
                *v = v.clamp(h - reach, h + reach);
            }
        }
        // every column near the road takes its nearest sample
        let half = if kind == Way::Road { 1.5 } else { 0.9 };
        let mut near: std::collections::HashMap<(i32, i32), (f32, usize)> =
            std::collections::HashMap::new();
        for (i, q) in line.iter().enumerate() {
            if skip[i] {
                continue;
            }
            let (cx, cz) = cell(q);
            for z in cz - 5..=cz + 5 {
                for x in cx - 5..=cx + 5 {
                    let d = (x as f32 - q[0]).hypot(z as f32 - q[1]);
                    let e = near.entry((x, z)).or_insert((f32::MAX, 0));
                    if d < e.0 {
                        *e = (d, i);
                    }
                }
            }
        }
        for ((x, z), (d, i)) in near {
            let f = self.flags(x, z);
            if f & WET != 0 || self.locked(x, z) {
                continue;
            }
            let level = p[i].round() as i32;
            if d <= half {
                if f & SITE == 0 {
                    self.set_ground(x, z, level);
                }
                self.flag(x, z, ROAD);
                let r = rnd(x, z, 31) % 100;
                // flagstones near the town and the hubs, gravel between
                let town = (x * x + z * z) < 60 * 60 || f & SITE != 0;
                let b = match kind {
                    Way::Road if town => {
                        [FLAGSTONES, FLAGSTONES, FLAGSTONES, COBBLE][(r % 4) as usize]
                    }
                    Way::Road if r < 50 => COBBLE,
                    Way::Road if r < 72 => FLAGSTONES,
                    Way::Road if r < 88 => DIRT,
                    Way::Road => MOSSY,
                    Way::Trail if r < 72 => DIRT,
                    Way::Trail if r < 90 => COBBLE,
                    Way::Trail => AIR,
                };
                if f & SITE == 0 || kind == Way::Road {
                    self.set_paint(x, z, b);
                }
            } else if f & (SITE | ROAD | ROCK) == 0 && d < 4.6 {
                // keep the verges of a road clear of trees
                if kind == Way::Road
                    && d < 3.6
                    && let Some(c) = self.idx(x, z)
                {
                    self.strip[c] = true;
                }
                let slack = (d - half).ceil() as i32;
                let g = self.ground(x, z).clamp(level - slack, level + slack);
                self.set_ground(x, z, g);
            }
        }
        self.ways.push(
            line.iter()
                .zip(&skip)
                .filter(|(_, s)| !**s)
                .map(|(q, _)| [q[0], q[1]])
                .collect(),
        );
        if kind == Way::Trail {
            // trails have no lamps
            let last = self.ways.len() - 1;
            self.ways[last].clear();
        }
    }

    // ------------------------------------------------------------- building

    fn free(&self, x: i32, y: i32, z: i32) -> bool {
        let p = [self.o[0] + x, y, self.o[2] + z];
        self.w.in_bounds(p)
            && x.abs().max(z.abs()) > TOWN
            && (natural(self.orig.get(p[0], p[1], p[2]), y) || self.freed.contains(&p))
    }

    /// Set a block at an offset from the hearth and an absolute height,
    /// unless something a player made is in the way.
    pub(crate) fn set(&mut self, x: i32, y: i32, z: i32, b: BlockId) {
        if self.free(x, y, z) {
            self.w.set(self.o[0] + x, y, self.o[2] + z, b);
        }
    }

    pub(crate) fn block(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.w.get(self.o[0] + x, y, self.o[2] + z)
    }

    /// Build at this site from now on: `put` is relative to it.
    pub(crate) fn origin(&mut self, x: i32, level: i32, z: i32) {
        self.at = [x, level, z];
    }

    /// Set a block relative to the site: y 0 is the ground block, 1 stands on it.
    pub(crate) fn put(&mut self, x: i32, y: i32, z: i32, b: BlockId) {
        self.set(self.at[0] + x, self.at[1] + y, self.at[2] + z, b);
    }

    pub(crate) fn look(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.block(self.at[0] + x, self.at[1] + y, self.at[2] + z)
    }

    pub(crate) fn fill(&mut self, a: [i32; 3], b: [i32; 3], block: BlockId) {
        for y in a[1]..=b[1] {
            for z in a[2]..=b[2] {
                for x in a[0]..=b[0] {
                    self.put(x, y, z, block);
                }
            }
        }
    }

    pub(crate) fn column(&mut self, x: i32, z: i32, ys: std::ops::RangeInclusive<i32>, b: BlockId) {
        for y in ys {
            self.put(x, y, z, b);
        }
    }

    /// Ground of a column relative to the site.
    pub(crate) fn rise(&self, x: i32, z: i32) -> i32 {
        self.ground(self.at[0] + x, self.at[2] + z) - self.at[1]
    }

    /// A lamp post, if there is room for one.
    pub(crate) fn lamp(&mut self, x: i32, z: i32) -> bool {
        let g = self.ground(x, z);
        let clear = self.flags(x, z) & (ROAD | WET) == 0
            && !self.locked(x, z)
            && self.water(x, z) == 0
            && is_solid(self.block(x, g, z))
            && (1..=4).all(|k| self.block(x, g + k, z) == AIR);
        if clear {
            for k in 1..=3 {
                self.set(x, g + k, z, TIMBER_BEAM);
            }
            self.set(x, g + 4, z, LANTERN);
        }
        clear
    }

    /// A signpost: a post with an arm and a banner hanging from it.
    pub(crate) fn signpost(&mut self, x: i32, z: i32, arm: [i32; 2], banner: BlockId) {
        let g = self.ground(x, z);
        if self.flags(x, z) & (ROAD | WET) != 0
            || self.locked(x, z)
            || !(1..=4).all(|k| self.block(x, g + k, z) == AIR)
        {
            return;
        }
        for k in 1..=4 {
            self.set(x, g + k, z, if k < 4 { DARK_PLANKS } else { PLANKS });
        }
        self.set(x + arm[0], g + 4, z + arm[1], PLANKS);
        self.set(x + arm[0], g + 3, z + arm[1], banner);
        self.set(x - arm[0], g + 3, z - arm[1], PLANKS);
    }

    /// Lamps along the roads and signposts at the forks.
    fn furniture(&mut self) {
        let ways = std::mem::take(&mut self.ways);
        for (n, line) in ways.iter().enumerate() {
            let mut side = if n % 2 == 0 { 1.0 } else { -1.0 };
            for (i, q) in line.iter().enumerate() {
                if i % 44 != 20 || i + 2 >= line.len() {
                    continue;
                }
                let (dx, dz) = (line[i + 2][0] - q[0], line[i + 2][1] - q[1]);
                let l = dx.hypot(dz).max(0.01);
                let (x, z) = (
                    (q[0] - dz / l * 2.6 * side).round() as i32,
                    (q[1] + dx / l * 2.6 * side).round() as i32,
                );
                if self.flags(x, z) & SITE == 0 && self.lamp(x, z) {
                    side = -side;
                }
            }
        }
        self.signpost(-12, -62, [0, -1], BLUE_BANNER);
        self.signpost(8, 79, [1, 0], RED_BANNER);
        self.signpost(56, 90, [0, -1], RED_BANNER);
        self.signpost(122, 4, [0, 1], BLUE_BANNER);
        self.signpost(137, 10, [1, 0], BLUE_BANNER);
        self.signpost(-58, -83, [-1, 0], RED_BANNER);
        self.signpost(-33, 121, [0, 1], BLUE_BANNER);
    }

    // --------------------------------------------------------------- mines

    /// Dig one block of the mine, unless that would open onto water.
    fn dig(&mut self, x: i32, y: i32, z: i32) {
        let wet = [
            (0, 0, 0),
            (1, 0, 0),
            (-1, 0, 0),
            (0, 0, 1),
            (0, 0, -1),
            (0, 1, 0),
        ]
        .iter()
        .any(|d| self.block(x + d.0, y + d.1, z + d.2) == WATER);
        if !wet && y > 2 {
            self.set(x, y, z, AIR);
        }
    }

    /// A floor block under a dug cell: planks over a void, rock otherwise.
    fn floor(&mut self, x: i32, y: i32, z: i32, rail: bool) {
        let b = self.block(x, y, z);
        if b == AIR || b == WATER {
            self.set(x, y, z, PLANKS);
        } else if rail {
            self.set(
                x,
                y,
                z,
                if (x + z).rem_euclid(3) == 0 {
                    DARK_PLANKS
                } else {
                    PLANKS
                },
            );
        } else if matches!(b, GRASS | DIRT | SAND) || rnd(x * 7 + y, z, 51).is_multiple_of(6) {
            self.set(x, y, z, COBBLE);
        }
    }

    /// A gallery from `a` to `b` along x or z, three wide and three high,
    /// with timber sets, a plank track and the odd lantern. The floor steps
    /// down evenly if the ends differ in height.
    fn gallery(&mut self, a: [i32; 3], b: [i32; 3], lit: u32) {
        let along_x = a[2] == b[2];
        let len = if along_x {
            (b[0] - a[0]).abs()
        } else {
            (b[2] - a[2]).abs()
        };
        let step = if along_x {
            (b[0] - a[0]).signum()
        } else {
            (b[2] - a[2]).signum()
        };
        let slope = a[1] != b[1];
        for i in 0..=len {
            let y = a[1] + ((b[1] - a[1]) as f32 * i as f32 / len.max(1) as f32).round() as i32;
            let (cx, cz) = if along_x {
                (a[0] + step * i, a[2])
            } else {
                (a[0], a[2] + step * i)
            };
            let cell = |k: i32| if along_x { (cx, cz + k) } else { (cx + k, cz) };
            let top = if slope { 4 } else { 3 };
            for k in -1..=1 {
                let (x, z) = cell(k);
                for h in 1..=top {
                    self.dig(x, y + h, z);
                }
                self.floor(x, y, z, k == 0 && !slope);
            }
            if i % 4 == 2 {
                // a timber set: posts in the walls, a beam under the roof
                let broken = rnd(cx, cz, 52).is_multiple_of(7);
                for k in [-2, 2] {
                    let (x, z) = cell(k);
                    for h in 1..top {
                        if !(broken && k == 2 && h > 1) {
                            self.set(x, y + h, z, TIMBER_BEAM);
                        }
                    }
                }
                for k in -2..=2 {
                    let (x, z) = cell(k);
                    if !(broken && k >= 0) {
                        self.set(x, y + top, z, TIMBER_BEAM);
                    }
                }
                if broken {
                    // the roof came down here
                    let (x, z) = cell(1);
                    self.set(x, y + 1, z, COBBLE);
                    self.set(x, y + top, z, AIR);
                } else if lit > 0 && ((i / 4) as u32).is_multiple_of(lit) {
                    let (x, z) = cell(0);
                    self.set(x, y + top, z, LANTERN);
                }
            }
        }
    }

    fn chamber(&mut self, x: [i32; 2], z: [i32; 2], y: i32, h: i32) {
        for cz in z[0]..=z[1] {
            for cx in x[0]..=x[1] {
                for k in 1..=h {
                    self.dig(cx, y + k, cz);
                }
                self.floor(cx, y, cz, false);
            }
        }
    }

    /// Put ore in the rock around the dug cells of a box.
    fn veins(&mut self, x: [i32; 2], z: [i32; 2], y: [i32; 2], every: u32, ores: &[BlockId]) {
        for cy in y[0]..=y[1] {
            for cz in z[0]..=z[1] {
                for cx in x[0]..=x[1] {
                    if self.block(cx, cy, cz) != STONE
                        || !hash3(cx, cy, cz, 0x0ae).is_multiple_of(every)
                    {
                        continue;
                    }
                    let open = [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)]
                        .iter()
                        .any(|d| self.block(cx + d.0, cy + d.1, cz + d.2) == AIR);
                    if !open {
                        continue;
                    }
                    let ore =
                        ores[(hash3(cx / 3, cy / 3, cz / 3, 0x0af) % ores.len() as u32) as usize];
                    for d in [
                        (0, 0, 0),
                        (1, 0, 0),
                        (0, 1, 0),
                        (0, 0, 1),
                        (-1, 0, 0),
                        (0, 0, -1),
                    ] {
                        let (px, py, pz) = (cx + d.0, cy + d.1, cz + d.2);
                        if self.block(px, py, pz) == STONE
                            && !hash3(px, py, pz, 0x0b0).is_multiple_of(3)
                        {
                            self.set(px, py, pz, ore);
                        }
                    }
                }
            }
        }
    }

    /// Flood a stretch of floor two deep, and wall the water in.
    fn flood(&mut self, x: [i32; 2], z: [i32; 2], y: i32) {
        let mut cells = Vec::new();
        for cz in z[0]..=z[1] {
            for cx in x[0]..=x[1] {
                if self.block(cx, y + 1, cz) != AIR {
                    continue;
                }
                for k in [0, -1] {
                    self.set(cx, y + k, cz, WATER);
                    if self.block(cx, y + k, cz) == WATER {
                        cells.push((cx, y + k, cz));
                    }
                }
            }
        }
        for (cx, cy, cz) in cells {
            for d in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)] {
                let (px, py, pz) = (cx + d.0, cy + d.1, cz + d.2);
                if self.block(px, py, pz) == AIR {
                    self.set(px, py, pz, COBBLE);
                }
            }
        }
    }

    /// The Hollow Mines: the entrance in the yard wall under the headframe,
    /// the Upper Galleries, the Flooded Drift and the Deep Seam.
    fn mines(&mut self) {
        if !self.has_ridge || !mine_fits(self.w.size, self.o) {
            return;
        }
        let l0 = self.ground(YARD[0], YARD[1]);
        let [l1, l2, l3] = MINE_DEPTHS.map(|d| l0 - d);
        if l3 < 4 {
            return;
        }
        let top = self.ground(HEADFRAME[0], HEADFRAME[1]);
        // the adit and the hall under the shaft
        self.gallery([161, l0, -20], [178, l0, -20], 2);
        self.chamber([163, 167], [-22, -18], l0, 5);
        for y in l0 + 6..top {
            for z in -21..=-19 {
                for x in 164..=166 {
                    self.dig(x, y, z);
                }
            }
        }
        // the cage hangs on its rope where the hoist stopped
        for y in l0 + 5..=top + 12 {
            self.set(
                165,
                y,
                -20,
                if y == top + 9 { LANTERN } else { TIMBER_BEAM },
            );
        }
        for z in -21..=-19 {
            for x in 164..=166 {
                self.set(x, l0 + 4, z, PLANKS);
            }
        }
        self.set(163, l0 + 1, -22, CRATE);
        self.set(163, l0 + 1, -18, BARREL);
        self.set(167, l0 + 1, -18, BARREL);
        self.set(167, l0 + 2, -18, LANTERN);
        // down to the Upper Galleries
        self.gallery([178, l0, -22], [178, l1, -36], 3);
        self.gallery([150, l1, -38], [181, l1, -38], 2);
        self.gallery([178, l1, -40], [178, l1, -66], 3);
        self.gallery([158, l1, -40], [158, l1, -63], 3);
        self.gallery([158, l1, -58], [181, l1, -58], 4);
        self.gallery([168, l1, -40], [168, l1, -43], 0);
        self.gallery([168, l1, -53], [168, l1, -56], 0);
        self.chamber([164, 172], [-52, -44], l1, 5);
        self.chamber([146, 151], [-41, -35], l1, 4);
        // the stope: pillars left standing, a cart, the last shift's ore
        for (x, z) in [(165, -51), (171, -51), (165, -45), (171, -45)] {
            for h in 1..=5 {
                self.set(x, l1 + h, z, if h == 5 { TIMBER_BEAM } else { STONE });
            }
        }
        self.set(168, l1 + 1, -48, CRATE);
        self.set(169, l1 + 1, -48, CRATE);
        self.set(168, l1, -48, PLANKS);
        self.set(168, l1, -47, PLANKS);
        self.set(168, l1, -49, PLANKS);
        self.set(172, l1 + 1, -52, BARREL);
        self.set(164, l1 + 1, -44, IRON_ORE);
        self.set(164, l1 + 2, -44, LANTERN);
        self.set(172, l1 + 4, -48, LANTERN);
        // the west end fell in
        for (x, z, h) in [
            (146, -38, 3),
            (147, -39, 2),
            (146, -40, 2),
            (147, -37, 1),
            (148, -38, 1),
        ] {
            for k in 1..=h {
                self.set(x, l1 + k, z, COBBLE);
            }
        }
        self.set(150, l1 + 1, -36, CRATE);
        self.set(151, l1 + 1, -41, TIMBER_BEAM);
        self.set(150, l1 + 1, -41, TIMBER_BEAM);
        self.set(150, l1 + 2, -41, TIMBER_BEAM);
        self.veins(
            [144, 184],
            [-68, -18],
            [l1, l0 + 4],
            13,
            &[COAL_ORE, IRON_ORE, COAL_ORE],
        );
        // down to the Flooded Drift
        self.gallery([158, l1, -64], [158, l2, -80], 4);
        self.gallery([146, l2, -82], [181, l2, -82], 5);
        self.chamber([147, 154], [-91, -85], l2, 4);
        self.gallery([150, l2, -84], [150, l2, -85], 0);
        self.gallery([170, l2, -84], [170, l2, -93], 0);
        self.chamber([167, 173], [-97, -93], l2, 3);
        // water stands in the low middle of the drift and in the pump room
        self.flood([161, 176], [-83, -81], l2);
        self.flood([147, 154], [-91, -87], l2);
        self.flood([167, 173], [-97, -94], l2);
        // what is left of the board walk
        for x in (161..=176).filter(|x| !(166..=170).contains(x)) {
            self.set(x, l2, -83, PLANKS);
        }
        self.set(166, l2, -83, PLANKS);
        for (x, z) in [
            (162, -81),
            (175, -81),
            (148, -86),
            (153, -91),
            (168, -93),
            (172, -97),
        ] {
            self.set(x, l2 + 1, z, GLOWSHROOM);
            if self.block(x, l2, z) == WATER {
                self.set(x, l2, z, MOSSY);
            }
        }
        self.set(154, l2 + 1, -85, BARREL);
        self.set(154, l2 + 2, -85, BARREL);
        self.set(153, l2 + 1, -85, BARREL);
        self.set(147, l2 + 1, -85, CRATE);
        self.set(146, l2 + 1, -81, CRATE);
        self.veins(
            [144, 184],
            [-99, -79],
            [l2, l2 + 4],
            7,
            &[IRON_ORE, COAL_ORE, IRON_ORE],
        );
        // down to the Deep Seam
        self.gallery([180, l2, -84], [180, l3, -100], 4);
        self.gallery([180, l3, -101], [180, l3, -102], 0);
        self.chamber([168, 181], [-116, -103], l3, 7);
        // the seam: a band of coal round the walls, embers in it
        for z in -117..=-102 {
            for x in 167..=182 {
                let wall = x == 167 || x == 182 || z == -117 || z == -102;
                if !wall {
                    continue;
                }
                for h in 2..=4 {
                    if self.block(x, l3 + h, z) != AIR && natural(self.block(x, l3 + h, z), 0) {
                        let b = if hash3(x, h, z, 0x5e).is_multiple_of(9) {
                            EMBERSTONE
                        } else if hash3(x, h, z, 0x5f).is_multiple_of(6) {
                            STONE
                        } else {
                            COAL_ORE
                        };
                        self.set(x, l3 + h, z, b);
                    }
                }
            }
        }
        // pit props hold the roof; the foreman's bench stands on a dais
        for (x, z) in [(171, -113), (178, -113), (171, -106), (178, -106)] {
            for h in 1..=7 {
                self.set(x, l3 + h, z, TIMBER_BEAM);
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                self.set(x + dx, l3 + 7, z + dz, TIMBER_BEAM);
            }
        }
        for z in -116..=-114 {
            for x in 172..=177 {
                self.set(x, l3 + 1, z, STONE_BRICKS);
            }
        }
        self.set(174, l3 + 2, -116, ANVIL);
        self.set(175, l3 + 2, -116, CRATE);
        self.set(172, l3 + 2, -116, BRAZIER);
        self.set(177, l3 + 2, -116, BRAZIER);
        self.set(169, l3 + 1, -104, CRATE);
        self.set(169, l3 + 1, -105, CRATE);
        self.set(169, l3 + 2, -104, CRATE);
        self.set(168, l3 + 1, -110, BARREL);
        self.set(181, l3 + 1, -110, COAL_ORE);
        self.set(181, l3 + 1, -111, COAL_ORE);
        self.set(181, l3 + 2, -111, COAL_ORE);
        self.set(174, l3 + 7, -109, LANTERN);
    }

    // ------------------------------------------------------------ dressing

    /// Plant a tree on open grass. `kind`: 0 oak, 1 birch, 2 dead, 3 dark.
    pub(crate) fn tree(&mut self, x: i32, z: i32, kind: u8, r: u32) {
        let g = self.ground(x, z);
        if self.flags(x, z) != 0
            || self.locked(x, z)
            || self.block(x, g, z) != GRASS && !(kind == 2 && self.block(x, g, z) == DIRT)
            || (1..=6).any(|k| self.block(x, g + k, z) != AIR)
        {
            return;
        }
        for dz in -2..=2 {
            for dx in -2..=2 {
                if is_log(self.block(x + dx, g + 1, z + dz))
                    || is_log(self.block(x + dx, g + 2, z + dz))
                {
                    return;
                }
            }
        }
        let (log, leaf) = match kind {
            1 => (BIRCH_LOG, BIRCH_LEAVES),
            3 => (LOG, PINE_LEAVES),
            _ => (LOG, LEAVES),
        };
        let trunk = 4 + (r >> 8) as i32 % 3 + i32::from(kind == 1);
        for y in 1..=trunk {
            self.set(x, g + y, z, log);
        }
        if kind == 2 {
            // a dead snag: a bare trunk with a broken limb or two
            for (n, (dx, dz)) in [(1, 0), (0, 1), (-1, 0), (0, -1)].into_iter().enumerate() {
                if (r >> (12 + n)) & 1 == 0 {
                    continue;
                }
                let h = g + trunk - 1 - n as i32 % 2;
                self.set(x + dx, h, z + dz, LOG);
                if (r >> (16 + n)) & 1 == 1 {
                    self.set(x + 2 * dx, h + 1, z + 2 * dz, LOG);
                }
            }
            return;
        }
        for dy in -2..=1i32 {
            let rad: i32 = if dy == 1 { 1 } else { 2 };
            for dz in -rad..=rad {
                for dx in -rad..=rad {
                    let corner = dx.abs() == rad && dz.abs() == rad;
                    if corner && (dy == 1 || hash(x + dx, z + dz, dy as u32).is_multiple_of(2)) {
                        continue;
                    }
                    if kind == 3 && hash3(x + dx, dy, z + dz, 7).is_multiple_of(3) {
                        continue;
                    }
                    if self.block(x + dx, g + trunk + dy, z + dz) == AIR {
                        self.set(x + dx, g + trunk + dy, z + dz, leaf);
                    }
                }
            }
        }
    }

    /// Cut down the tree whose trunk stands in this column.
    pub(crate) fn fell(&mut self, x: i32, z: i32, stump: bool) {
        let g = self.ground(x, z);
        let Some(i) = self.idx(x, z) else { return };
        if self.lock[i] {
            return;
        }
        for y in g + 1 + i32::from(stump)..self.sy {
            if is_log(self.block(x, y, z)) {
                self.set(x, y, z, AIR);
            } else if y > g + 3 {
                break;
            }
        }
        self.bare[i] = true;
    }

    /// Give every sub-zone its look: pasture, birch wood, dead wood.
    fn dress(&mut self) {
        for z in -self.o[2] + 3..self.sz - self.o[2] - 3 {
            for x in -self.o[0] + 3..self.sx - self.o[0] - 3 {
                if self.locked(x, z) || self.flags(x, z) != 0 || self.water(x, z) > 0 {
                    continue;
                }
                let g = self.ground(x, z);
                let r = rnd(x, z, 0xd2e5);
                let trunk = is_log(self.block(x, g + 1, z));
                let top = self.block(x, g, z);
                match biome_at(x, z) {
                    Biome::Hearthmeadow => {
                        if trunk && r % 100 < 84 {
                            self.fell(x, z, false);
                        } else if top == GRASS
                            && self.block(x, g + 1, z) == AIR
                            && r.is_multiple_of(260)
                        {
                            self.set(x, g + 1, z, FLOWER_BED);
                        }
                    }
                    Biome::Birchrise => {
                        for y in g + 1..(g + 14).min(self.sy) {
                            match self.block(x, y, z) {
                                LOG => self.set(x, y, z, BIRCH_LOG),
                                LEAVES => self.set(x, y, z, BIRCH_LEAVES),
                                _ => {}
                            }
                        }
                        let dense = noise(x, z, 21.0, 83) > 0.35;
                        if r.is_multiple_of(if dense { 13 } else { 60 }) {
                            self.tree(x, z, 1, r);
                        }
                    }
                    Biome::Barrowwood => {
                        for y in g + 1..(g + 14).min(self.sy) {
                            match self.block(x, y, z) {
                                BIRCH_LOG => self.set(x, y, z, LOG),
                                GLOWSHROOM | BERRY_BUSH => self.set(x, y, z, AIR),
                                b if is_leaves(b) => {
                                    let keep = hash3(x, y, z, 0xdead) % 10 < 3;
                                    self.set(x, y, z, if keep { PINE_LEAVES } else { AIR });
                                }
                                _ => {}
                            }
                        }
                        if top == GRASS && noise(x, z, 7.0, 84) > 0.62 {
                            self.set(x, g, z, DIRT);
                        }
                        if r.is_multiple_of(34) {
                            self.tree(x, z, if r.is_multiple_of(7) { 3 } else { 2 }, r >> 3);
                        } else if top == GRASS
                            && r.is_multiple_of(310)
                            && self.block(x, g + 1, z) == AIR
                        {
                            self.set(x, g + 1, z, MOSSY);
                        }
                    }
                    Biome::Mirrormere => {
                        if trunk && r % 100 < 35 {
                            self.fell(x, z, false);
                        } else if top == GRASS
                            && self.block(x, g + 1, z) == AIR
                            && r.is_multiple_of(150)
                        {
                            self.set(x, g + 1, z, FLOWER_BED);
                        }
                    }
                    Biome::EastRoad => {
                        if trunk && r % 100 < 55 {
                            self.fell(x, z, r.is_multiple_of(3));
                        }
                    }
                    Biome::Ridge => {
                        if top == GRASS && noise(x, z, 6.0, 85) > 0.6 {
                            self.set(x, g, z, if r.is_multiple_of(3) { COBBLE } else { STONE });
                        } else if r.is_multiple_of(90) {
                            self.tree(x, z, 3, r);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Grave clusters in Barrowwood: leaning stones, sunken beds, an open pit.
    fn graves(&mut self) {
        for (n, (cx, cz, count)) in [
            (-110, -26, 9),
            (-140, -10, 6),
            (-114, 24, 8),
            (-142, 32, 5),
            (-98, 4, 4),
            (-128, -44, 6),
        ]
        .into_iter()
        .enumerate()
        {
            if !self.fits([cx - 4, cx + 14], [cz - 4, cz + 12]) {
                continue;
            }
            for k in 0..count {
                let r = rnd(cx + k, cz, 0x9a7e);
                let x = cx + (k % 4) * 3 + (r % 2) as i32;
                let z = cz + (k / 4) * 4 + ((r >> 2) % 2) as i32;
                if self.flags(x, z) != 0 || self.flags(x, z + 1) != 0 || self.locked(x, z) {
                    continue;
                }
                let g = self.ground(x, z);
                if self.block(x, g + 1, z) != AIR || self.ground(x, z + 1) != g {
                    continue;
                }
                let stone = [COBBLE, MOSSY, STONE_BRICKS, MOSSY][(r >> 4) as usize % 4];
                self.set(x, g + 1, z, stone);
                if (r >> 7).is_multiple_of(3) {
                    self.set(x, g + 2, z, stone);
                }
                // the bed of the grave; one in each yard lies open
                let open = k == 2 + n as i32 % 3;
                for dz in 1..=2 {
                    if self.ground(x, z + dz) == g && self.flags(x, z + dz) == 0 {
                        self.set(x, g, z + dz, if open { AIR } else { DIRT });
                        if open {
                            self.set(x, g - 1, z + dz, DIRT);
                        }
                    }
                }
            }
            // what is left of the wall
            for i in -2..=12 {
                for (x, z) in [
                    (cx + i, cz - 2),
                    (cx + i, cz + 10),
                    (cx - 2, cz + i),
                    (cx + 12, cz + i),
                ] {
                    if !(cz - 2..=cz + 10).contains(&z) || !rnd(x, z, 0x9a7f).is_multiple_of(2) {
                        continue;
                    }
                    let g = self.ground(x, z);
                    if self.flags(x, z) == 0 && !self.locked(x, z) && self.block(x, g + 1, z) == AIR
                    {
                        self.set(
                            x,
                            g + 1,
                            z,
                            if rnd(x, z, 5).is_multiple_of(2) {
                                MOSSY
                            } else {
                                COBBLE
                            },
                        );
                    }
                }
            }
            self.tree(cx + 13, cz + 5, 2, 0x5a5a5 + n as u32 * 977);
            self.tree(cx - 3, cz + 1, 2, 0xa5a5a + n as u32 * 131);
        }
    }

    /// Bury a lantern under every landmark spot that has none, where it
    /// lights nothing: see [`BEACONS`].
    fn bury_lanterns(&mut self) {
        for [bx, bz] in BEACONS {
            if !self.fits([bx - 16, bx + 16], [bz - 16, bz + 16])
                || self
                    .w
                    .find_near(LANTERN, self.o[0] + bx, self.o[2] + bz, 1)
                    .is_some()
            {
                continue;
            }
            let spots = [
                (0, 0),
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
            ];
            'find: for y in (4..self.sy - 2).rev() {
                for (dx, dz) in spots {
                    let (x, z) = (bx + dx, bz + dz);
                    let here = self.block(x, y, z);
                    let closed = [
                        (1, 0, 0),
                        (-1, 0, 0),
                        (0, 1, 0),
                        (0, -1, 0),
                        (0, 0, 1),
                        (0, 0, -1),
                    ]
                    .iter()
                    .all(|d| is_opaque(self.block(x + d.0, y + d.1, z + d.2)));
                    if closed && is_opaque(here) && self.free(x, y, z) {
                        self.set(x, y, z, LANTERN);
                        break 'find;
                    }
                }
            }
        }
    }

    /// Remove leaves that lost their tree.
    fn prune(&mut self) {
        let mut near = vec![false; self.bare.len()];
        for wz in 0..self.sz {
            for wx in 0..self.sx {
                if !self.bare[(wz * self.sx + wx) as usize] {
                    continue;
                }
                for z in (wz - 3).max(0)..=(wz + 3).min(self.sz - 1) {
                    for x in (wx - 3).max(0)..=(wx + 3).min(self.sx - 1) {
                        near[(z * self.sx + x) as usize] = true;
                    }
                }
            }
        }
        for wz in 0..self.sz {
            for wx in 0..self.sx {
                if !near[(wz * self.sx + wx) as usize] {
                    continue;
                }
                for y in 0..self.sy {
                    if !is_leaves(self.w.get(wx, y, wz)) || !is_leaves(self.orig.get(wx, y, wz)) {
                        continue;
                    }
                    let mut held = false;
                    'find: for dy in -7..=1 {
                        for dz in -3..=3 {
                            for dx in -3..=3 {
                                if is_log(self.w.get(wx + dx, y + dy, wz + dz)) {
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
}

/// The landmark spots of the plan. The server stamps a beacon (a cobble
/// pillar with a lantern) on every quest spot that has no lantern within one
/// column, so each of these gets a lantern of its own: a real one where it
/// fits the scene, a buried one elsewhere.
const BEACONS: [[i32; 2]; 12] = [
    [0, -56],
    [-45, -88],
    [-100, -55],
    [-130, 10],
    [15, 95],
    [-25, 135],
    [20, 155],
    [50, -12],
    [62, 60],
    [95, 0],
    [130, 8],
    [165, -20],
];

/// Take away the beacons the server stamped on the landmark spots, so the
/// landmarks can stand in their place. Returns the cells they stood in.
fn take_beacons(w: &mut World, hearth: [i32; 3]) -> HashSet<[i32; 3]> {
    let mut out = HashSet::new();
    for [dx, dz] in BEACONS {
        let (x, z) = (hearth[0] + dx, hearth[2] + dz);
        if !fits_in(w.size, hearth, [dx - 16, dx + 16], [dz - 16, dz + 16]) {
            continue;
        }
        let s = w.surface(x, z);
        let pillar = w.get(x, s, z) == LANTERN
            && w.get(x, s - 1, z) == MOSSY
            && (2..=4).all(|k| w.get(x, s - k, z) == COBBLE)
            && w.get(x, s - 5, z) == MOSSY;
        if !pillar {
            continue;
        }
        for k in 0..=4 {
            w.set(x, s - k, z, AIR);
            out.insert([x, s - k, z]);
        }
        for (ax, az) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            let p = [x + ax, s - 5, z + az];
            if w.get(p[0], p[1], p[2]) == MOSSY {
                let b = if !is_solid(w.get(p[0], p[1] - 1, p[2])) {
                    AIR
                } else if w.get(p[0], p[1] + 1, p[2]) == AIR {
                    GRASS
                } else {
                    DIRT
                };
                w.set(p[0], p[1], p[2], b);
                out.insert(p);
            }
        }
    }
    out
}

/// Points along a Catmull-Rom curve through `pts`, about `step` apart.
fn spline(pts: &[[f32; 3]], step: f32) -> Vec<[f32; 3]> {
    let n = pts.len();
    let mut out = Vec::new();
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (
            pts[i.saturating_sub(1)],
            pts[i],
            pts[i + 1],
            pts[(i + 2).min(n - 1)],
        );
        let len = (p2[0] - p1[0]).hypot(p2[1] - p1[1]);
        let k = (len / step).ceil().max(1.0) as usize;
        for j in 0..k {
            let t = j as f32 / k as f32;
            let mut q = [0.0; 3];
            for (c, v) in q.iter_mut().enumerate() {
                let (a, b, d, e) = (p0[c], p1[c], p2[c], p3[c]);
                *v = 0.5
                    * (2.0 * b
                        + (d - a) * t
                        + (2.0 * a - 5.0 * b + 4.0 * d - e) * t * t
                        + (3.0 * b - a - 3.0 * d + e) * t * t * t);
            }
            out.push(q);
        }
    }
    out.push(pts[n - 1]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the server's `build_landmarks` does at a quest spot.
    fn stamp_beacon(w: &mut World, h: [i32; 3], at: [i32; 2]) -> bool {
        let (x, z) = (h[0] + at[0], h[2] + at[1]);
        if !w.in_bounds([x, 1, z]) || w.find_near(LANTERN, x, z, 1).is_some() {
            return false;
        }
        let y = w.surface(x, z);
        for (dx, dz) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            w.set(x + dx, y, z + dz, 19);
        }
        for k in 1..=4 {
            w.set(x, y + k, z, if k == 4 { 19 } else { COBBLE });
        }
        w.set(x, y + 5, z, LANTERN);
        true
    }

    /// Water cells with air beside or below them.
    fn leaks(w: &World) -> usize {
        let [sx, sy, sz] = w.size.map(|v| v as i32);
        let mut n = 0;
        for y in 1..sy {
            for z in 1..sz - 1 {
                for x in 1..sx - 1 {
                    if w.get(x, y, z) == WATER
                        && [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)]
                            .iter()
                            .any(|d| w.get(x + d.0, y + d.1, z + d.2) == AIR)
                    {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    /// Can a player walk, jump one block, drop three or swim from `a` to `b`?
    fn reachable(w: &World, a: [i32; 3], b: [i32; 3]) -> bool {
        let open = |p: [i32; 3]| !is_solid(w.get(p[0], p[1], p[2]));
        let stand = |p: [i32; 3]| {
            open(p)
                && open([p[0], p[1] + 1, p[2]])
                && (is_solid(w.get(p[0], p[1] - 1, p[2]))
                    || w.get(p[0], p[1], p[2]) == WATER
                    || w.get(p[0], p[1] - 1, p[2]) == WATER)
        };
        assert!(stand(a), "start {a:?} is not a standing place");
        assert!(stand(b), "goal {b:?} is not a standing place");
        let mut seen = HashSet::from([a]);
        let mut queue = std::collections::VecDeque::from([a]);
        while let Some(p) = queue.pop_front() {
            if p == b {
                return true;
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                for dy in -3..=1 {
                    let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                    let far = (q[0] - a[0]).abs().max((q[2] - a[2]).abs()) > 120;
                    if !far && stand(q) && seen.insert(q) {
                        queue.push_back(q);
                    }
                }
            }
        }
        false
    }

    #[test]
    fn zone_builds_once_and_spares_player_blocks() {
        let mut w = World::generate([384, 96, 384], 20261122);
        let h = w.ensure_spawn_hearth(None);
        for at in [[0, -56], [-130, 10], [20, 155], [62, 60]] {
            assert!(stamp_beacon(&mut w, h, at));
        }
        village::build_hearthstead(&mut w, h);
        // a player's blocks: in the tower, on a road, in the river, in the
        // lake, in the mill and deep in the rock where a gallery will run
        let top = |w: &World, dx: i32, dz: i32| w.surface(h[0] + dx, h[2] + dz);
        let gallery = top(&w, YARD[0], YARD[1]) - MINE_DEPTHS[0] + 1;
        let mine = [
            ([h[0] + 2, top(&w, 2, -57) + 1, h[2] - 57], CHEST),
            ([h[0], top(&w, 0, -45) + 1, h[2] - 45], CHEST),
            ([h[0] + 101, top(&w, 101, -66) + 1, h[2] - 66], CHEST),
            ([h[0], top(&w, 0, 134) + 1, h[2] + 134], BRICK),
            ([h[0] + 15, top(&w, 15, 95) + 3, h[2] + 95], GLASS),
            ([h[0] + 170, gallery, h[2] - 38], CHEST),
            ([h[0] + 62, top(&w, 62, 51) + 2, h[2] + 51], CHEST),
        ];
        for (p, b) in mine {
            w.set(p[0], p[1], p[2], b);
        }
        let before = leaks(&w);
        assert!(build_world_features(&mut w, h));
        for (p, b) in mine {
            assert_eq!(w.get(p[0], p[1], p[2]), b, "player block at {p:?}");
        }
        assert!(leaks(&w) <= before, "water stands beside air");
        // the town is untouched, and there are two new hearths
        assert_eq!(w.get(h[0], h[1], h[2]), HEARTH);
        let hearths = w.cells.iter().filter(|b| **b == HEARTH).count();
        assert_eq!(hearths, 3);
        // the server finds a lantern at every landmark: no beacon comes back
        for at in BEACONS {
            assert!(!stamp_beacon(&mut w, h, at), "beacon at {at:?}");
        }
        // every landmark is there, with a floor and headroom
        let pois = poi_positions_in(&w, h);
        assert_eq!(pois.len(), poi_positions(h).len());
        for (key, p) in &pois {
            assert!(is_solid(w.get(p[0], p[1] - 1, p[2])), "{key} has no floor");
            assert!(!is_solid(w.get(p[0], p[1], p[2])), "{key} is in a block");
            assert!(
                !is_solid(w.get(p[0], p[1] + 1, p[2])),
                "{key} has no headroom"
            );
        }
        let at = |key: &str| pois.iter().find(|p| p.0 == key).unwrap().1;
        // the mine can be walked from the yard to the Deep Seam, the crypt
        // from the porch to the burial room, the town to the hubs by road
        assert!(reachable(
            &w,
            at("hollow_mines"),
            at("mine_upper_galleries")
        ));
        assert!(reachable(&w, at("hollow_mines"), at("mine_flooded_drift")));
        assert!(reachable(&w, at("hollow_mines"), at("mine_deep_seam")));
        assert!(reachable(&w, at("old_barrow"), at("barrow_crypt")));
        let crypt = at("barrow_crypt");
        assert!(reachable(&w, crypt, [crypt[0] - 1, crypt[1], crypt[2] - 7]));
        assert!(reachable(&w, at("mine_deep_seam"), at("hollow_mines")));
        assert!(reachable(&w, at("barrow_crypt"), at("old_barrow")));
        assert!(reachable(&w, at("miners_rest"), at("hollow_mines")));
        assert!(reachable(&w, at("emberford_toll_hut"), at("miners_rest")));
        let built = w.clone();
        assert!(!build_world_features(&mut w, h));
        assert!(w.encode() == built.encode());
    }

    #[test]
    fn zone_fits_smaller_worlds() {
        for (size, seed) in [
            ([256, 96, 256], 20261122),
            ([96, 48, 96], 3),
            ([48, 48, 48], 5),
        ] {
            let mut w = World::generate(size, seed);
            let h = w.ensure_spawn_hearth(None);
            village::build_hearthstead(&mut w, h);
            let chest = [h[0] + 20, w.surface(h[0] + 20, h[2] - 40) + 1, h[2] - 40];
            let placed = w.set(chest[0], chest[1], chest[2], CHEST);
            let before = leaks(&w);
            let earth = |w: &World| -> Vec<i32> {
                let [sx, sy, sz] = size.map(|v| v as i32);
                (0..sx * sz)
                    .map(|i| {
                        (0..sy)
                            .rev()
                            .find(|y| {
                                let b = w.get(i % sx, *y, i / sx);
                                is_solid(b) && !plant(b) && b != FLOWER_BED && b != CHEST
                            })
                            .unwrap_or(0)
                    })
                    .collect()
            };
            let ground = earth(&w);
            assert!(build_world_features(&mut w, h), "size {size:?}");
            // nothing fits in the small worlds: the land keeps its shape
            if size[0] < 256 {
                assert!(earth(&w) == ground, "size {size:?}");
            }
            assert!(!placed || w.get(chest[0], chest[1], chest[2]) == CHEST);
            assert_eq!(w.get(h[0], h[1], h[2]), HEARTH);
            assert!(leaks(&w) <= before, "size {size:?}");
            let pois = poi_positions_in(&w, h);
            let has = |key: &str| pois.iter().any(|p| p.0 == key);
            assert_eq!(has("north_watch"), size[0] == 256, "size {size:?}");
            assert!(!has("hollow_mines") && !has("fishers_landing"));
            for (key, p) in &pois {
                assert!(is_solid(w.get(p[0], p[1] - 1, p[2])), "{key} has no floor");
                assert!(!is_solid(w.get(p[0], p[1], p[2])), "{key} is in a block");
            }
            assert!(!build_world_features(&mut w, h));
        }
    }

    /// Dev aid: build the default world, save it and draw a map of it.
    #[test]
    #[ignore]
    fn zz_map() {
        let mut w = World::generate([384, 96, 384], 20261122);
        let h = w.ensure_spawn_hearth(None);
        for at in [[0, -56], [-130, 10], [20, 155], [62, 60]] {
            stamp_beacon(&mut w, h, at);
        }
        village::build_hearthstead(&mut w, h);
        let t = std::time::Instant::now();
        assert!(build_world_features(&mut w, h));
        println!("hearth {h:?}, built in {:?}", t.elapsed());
        for (k, p) in poi_positions_in(&w, h) {
            println!("{k} {p:?}");
        }
        for at in BEACONS {
            assert!(!stamp_beacon(&mut w, h, at), "beacon at {at:?}");
        }
        if let Ok(p) = std::env::var("WORLD_OUT") {
            w.save(Path::new(&p)).unwrap();
        }
        let mut img = b"P6\n384 384\n255\n".to_vec();
        for z in 0..384 {
            for x in 0..384 {
                let mut y = 95;
                while y > 0 && w.get(x, y, z) == AIR {
                    y -= 1;
                }
                let b = w.get(x, y, z);
                let c = block(b).color;
                let s = 0.45 + (y as f32 - 20.0) / 45.0;
                img.extend(c.map(|v| (v * s * 255.0).clamp(0.0, 255.0) as u8));
            }
        }
        if let Ok(p) = std::env::var("MAP_OUT") {
            std::fs::write(p, img).unwrap();
        }
    }
}
