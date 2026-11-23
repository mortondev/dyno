//! Building a world from a layout and the pack's stamps. The engine knows
//! how to shape land from the layout's primitives, lay roads, put stamps
//! down and scatter vegetation by rules; what the world is comes from data.
//!
//! The work happens on a height map first (land, water, pads, roads), which
//! is then written to the world; stamps and vegetation go on top.
//!
//! A world may be smaller than its layout (tests, the title stage): it is
//! then the window of that size round the origin, and whatever lies outside
//! is left out.

use crate::layout::{Land, Layout, Place, Species, rnd, spline};
use crate::stamp::Stamps;
use crate::*;
use std::collections::BTreeMap;

/// Column flags: a road or trail runs here.
pub const ROAD: u8 = 1;
/// Column flags: river or lake.
pub const WET: u8 = 2;
/// Column flags: the levelled ground of a place, or under a stamp.
pub const SITE: u8 = 4;
/// Column flags: a pad that keeps its wild trees.
const WOODED: u8 = 8;

/// A road as it was built.
#[derive(Clone, Debug)]
pub struct BuiltRoad {
    pub key: String,
    pub way: String,
    pub width: i32,
    /// the centre line, about a metre apart: world position of the block
    /// that is walked on
    pub line: Vec<[i32; 3]>,
}

/// What was built where: for the server, tests and tools.
#[derive(Clone, Debug, Default)]
pub struct Built {
    /// the spawn hearth
    pub hearth: [i32; 3],
    /// every place inside the world: key and the free block at its centre
    pub places: BTreeMap<String, [i32; 3]>,
    /// every stamp placed: place key, stamp name, anchor, quarter turns
    pub stamps: Vec<(String, String, [i32; 3], u8)>,
    pub roads: Vec<BuiltRoad>,
    /// planted trees: species, the ground block they stand on
    pub trees: Vec<(String, [i32; 3])>,
    /// column flags (ROAD, WET, SITE), x fastest
    pub flags: Vec<u8>,
}

struct Gen<'a> {
    l: &'a Layout,
    stamps: &'a Stamps,
    /// world size and the world column of the origin
    sx: i32,
    sz: i32,
    cx: i32,
    cz: i32,
    /// ground height per column: the y of the top block of earth
    g: Vec<f32>,
    flags: Vec<u8>,
    /// surface block per column, AIR for whatever the land gives
    paint: Vec<BlockId>,
    /// stamps to place: place key, name, anchor, turns
    puts: Vec<(String, String, [i32; 3], u8)>,
    built: Built,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl<'a> Gen<'a> {
    fn idx(&self, x: i32, z: i32) -> Option<usize> {
        let (wx, wz) = (x + self.cx, z + self.cz);
        (wx >= 0 && wz >= 0 && wx < self.sx && wz < self.sz).then(|| (wz * self.sx + wx) as usize)
    }

    fn ground(&self, x: i32, z: i32) -> f32 {
        let (wx, wz) = (
            (x + self.cx).clamp(0, self.sx - 1),
            (z + self.cz).clamp(0, self.sz - 1),
        );
        self.g[(wz * self.sx + wx) as usize]
    }

    fn top(&self, x: i32, z: i32) -> i32 {
        self.ground(x, z).round() as i32
    }

    /// The land itself, on every core.
    fn land(&mut self) {
        let land = Land::new(self.l);
        let sea = self.l.terrain.sea as f32;
        let (sx, cx, cz) = (self.sx, self.cx, self.cz);
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, 16);
        let rows = (self.sz as usize).div_ceil(threads).max(1);
        let land = &land;
        std::thread::scope(|sc| {
            for (k, (gs, fs)) in self
                .g
                .chunks_mut(rows * sx as usize)
                .zip(self.flags.chunks_mut(rows * sx as usize))
                .enumerate()
            {
                sc.spawn(move || {
                    for (i, (g, f)) in gs.iter_mut().zip(fs.iter_mut()).enumerate() {
                        let x = (i as i32 % sx) - cx;
                        let z = (k * rows) as i32 + i as i32 / sx - cz;
                        let h = land.height(x as f32, z as f32);
                        *g = h;
                        if h.round() < sea {
                            *f |= WET;
                        }
                    }
                });
            }
        });
    }

    /// Is a place (with its pad) inside the world?
    fn holds(&self, at: [i32; 2], r: [f32; 2]) -> bool {
        let m = 3;
        at[0] - r[0] as i32 + self.cx >= m
            && at[1] - r[1] as i32 + self.cz >= m
            && at[0] + r[0] as i32 + self.cx < self.sx - m
            && at[1] + r[1] as i32 + self.cz < self.sz - m
    }

    fn inside(&self, at: [i32; 2]) -> bool {
        self.idx(at[0] - 4, at[1] - 4).is_some() && self.idx(at[0] + 4, at[1] + 4).is_some()
    }

    /// Level an oval of ground and blend its edge into the land.
    fn level(&mut self, c: [i32; 2], r: [f32; 2], level: i32, blend: f32, flag: u8) {
        let reach = blend.max(1.0);
        let (bx, bz) = ((r[0] + reach) as i32 + 1, (r[1] + reach) as i32 + 1);
        for dz in -bz..=bz {
            for dx in -bx..=bx {
                let Some(i) = self.idx(c[0] + dx, c[1] + dz) else {
                    continue;
                };
                let m = ((dx as f32 / r[0]).powi(2) + (dz as f32 / r[1]).powi(2)).sqrt();
                if m <= 1.0 {
                    self.g[i] = level as f32;
                    self.flags[i] = (self.flags[i] & !WET) | flag;
                    continue;
                }
                let d = (m - 1.0) * r[0].min(r[1]);
                if d < reach && self.flags[i] & SITE == 0 {
                    let t = smooth(d / reach);
                    let h = level as f32 + (self.g[i] - level as f32) * t;
                    // a pad at the water keeps its shore
                    if self.flags[i] & WET != 0 && h.round() < self.l.terrain.sea as f32 {
                        self.g[i] = h.min(self.g[i].max(h));
                        continue;
                    }
                    self.g[i] = h;
                    self.flags[i] &= !WET;
                }
            }
        }
    }

    /// Level a box of ground (under a stamp that stands on its own).
    /// With `banks`, only dry land is levelled: the banks under the ends of
    /// a bridge, not the river under its span.
    fn level_box(&mut self, lo: [i32; 2], hi: [i32; 2], level: i32, banks: bool) {
        let blend = 4;
        for z in lo[1] - blend..=hi[1] + blend {
            for x in lo[0] - blend..=hi[0] + blend {
                let Some(i) = self.idx(x, z) else { continue };
                if banks && self.flags[i] & WET != 0 {
                    continue;
                }
                let d = (lo[0] - x).max(x - hi[0]).max(lo[1] - z).max(z - hi[1]);
                if d <= 0 {
                    self.g[i] = level as f32;
                    self.flags[i] = (self.flags[i] & !WET) | SITE;
                } else if self.flags[i] & (SITE | WET) == 0 {
                    let t = smooth(d as f32 / (blend + 1) as f32);
                    self.g[i] = level as f32 + (self.g[i] - level as f32) * t;
                }
            }
        }
    }

    /// Level the places and decide where every stamp goes.
    fn sites(&mut self) -> Result<(), String> {
        let l = self.l;
        for p in &l.places {
            let at = l.place_at(p);
            let reach = p.pad.as_ref().map_or([4.0, 4.0], |pad| pad.radius);
            // a small world (a test, the title stage) may cut the hub's
            // edge off; every other place is whole or left out
            let hub = std::ptr::eq(p, &l.places[0]);
            if !(self.holds(at, reach) || (hub && self.inside(at))) {
                continue;
            }
            let level = if p.level.as_deref() == Some("water") {
                l.terrain.sea + 1
            } else {
                self.top(at[0], at[1]).max(l.terrain.sea + 1)
            };
            if let Some(pad) = &p.pad {
                let flag = SITE | if pad.wooded { WOODED } else { 0 };
                self.level(at, pad.radius, level, pad.blend, flag);
                if let Some(surface) = &pad.surface {
                    let (bx, bz) = (pad.radius[0] as i32, pad.radius[1] as i32);
                    for dz in -bz..=bz {
                        for dx in -bx..=bx {
                            let m = ((dx as f32 / pad.radius[0]).powi(2)
                                + (dz as f32 / pad.radius[1]).powi(2))
                            .sqrt();
                            // a ragged edge
                            let edge =
                                1.0 - 0.12 * (rnd(at[0] + dx, at[1] + dz, 61) % 100) as f32 / 100.0;
                            if m > edge {
                                continue;
                            }
                            if let (Some(i), Some(k)) = (
                                self.idx(at[0] + dx, at[1] + dz),
                                Layout::pick(surface, rnd(at[0] + dx, at[1] + dz, 62)),
                            ) {
                                self.paint[i] = block_by_key(k).unwrap_or(AIR);
                            }
                        }
                    }
                }
            }
            if let Some(plaza) = &p.plaza {
                let b = plaza.radius as i32 + 1;
                for dz in -b..=b {
                    for dx in -b..=b {
                        let ragged = (rnd(at[0] + dx, at[1] + dz, 66) % 100) as f32 * 0.012;
                        if (dx as f32).hypot(dz as f32) > plaza.radius - ragged {
                            continue;
                        }
                        if let (Some(i), Some(k)) = (
                            self.idx(at[0] + dx, at[1] + dz),
                            Layout::pick(&plaza.surface, rnd(at[0] + dx, at[1] + dz, 67)),
                        ) {
                            self.paint[i] = block_by_key(k).unwrap_or(AIR);
                            self.flags[i] |= ROAD;
                        }
                    }
                }
            }
            for put in &p.stamps {
                let stamp = self
                    .stamps
                    .get(&put.stamp)
                    .ok_or(format!("unknown stamp \"{}\"", put.stamp))?;
                let (x, z) = (at[0] + put.at[0], at[1] + put.at[1]);
                let (lo, hi) = stamp.footprint(put.rot);
                let (lo, hi) = ([x + lo[0], z + lo[1]], [x + hi[0], z + hi[1]]);
                if self.idx(lo[0] - 2, lo[1] - 2).is_none()
                    || self.idx(hi[0] + 2, hi[1] + 2).is_none()
                {
                    continue;
                }
                let fit =
                    put.fit
                        .as_deref()
                        .unwrap_or(if p.pad.is_some() { "pad" } else { "ground" });
                let y = match fit {
                    "pad" => {
                        if p.pad.is_none() {
                            self.level_box(lo, hi, level, true);
                        }
                        level
                    }
                    "float" => self.top(x, z),
                    _ => {
                        let y = self.top(x, z).max(l.terrain.sea + 1);
                        self.level_box(lo, hi, y, false);
                        y
                    }
                };
                self.puts.push((
                    p.key.clone(),
                    put.stamp.clone(),
                    [x + self.cx, y + 1 + put.dy, z + self.cz],
                    put.rot,
                ));
            }
            self.built
                .places
                .insert(p.key.clone(), [at[0] + self.cx, level + 1, at[1] + self.cz]);
        }
        Ok(())
    }

    /// Roads and trails: graded along their length, level across, painted.
    fn roads(&mut self) {
        let l = self.l;
        for road in &l.roads {
            let Some(way) = l.ways.get(&road.way) else {
                continue;
            };
            let line = spline(&l.road_points(road), 1.0);
            // the part inside the world
            let line: Vec<[f32; 2]> = line
                .into_iter()
                .filter(|p| self.idx(p[0].round() as i32, p[1].round() as i32).is_some())
                .collect();
            if line.len() < 2 {
                continue;
            }
            let cell = |p: &[f32; 2]| [p[0].round() as i32, p[1].round() as i32];
            // the long profile: the land's height, smoothed, fixed on pads
            // and at the water (bridges and fords)
            let fixed: Vec<bool> = line
                .iter()
                .map(|p| {
                    let c = cell(p);
                    self.idx(c[0], c[1])
                        .is_some_and(|i| self.flags[i] & (SITE | WET) != 0)
                })
                .collect();
            let sea = l.terrain.sea as f32;
            let mut prof: Vec<f32> = line
                .iter()
                .map(|p| {
                    let c = cell(p);
                    self.ground(c[0], c[1]).max(sea + 1.0)
                })
                .collect();
            for _ in 0..6 {
                let old = prof.clone();
                for i in 0..prof.len() {
                    if fixed[i] {
                        continue;
                    }
                    let (a, b) = (i.saturating_sub(8), (i + 8).min(old.len() - 1));
                    prof[i] = old[a..=b].iter().sum::<f32>() / (b - a + 1) as f32;
                }
            }
            // never steeper than a runner takes in stride: ramps are cut
            // and banked out from the fixed stretches
            const GRADE: f32 = 0.6;
            for _ in 0..3 {
                for i in 1..prof.len() {
                    if !fixed[i] {
                        prof[i] = prof[i].clamp(prof[i - 1] - GRADE, prof[i - 1] + GRADE);
                    }
                }
                for i in (0..prof.len() - 1).rev() {
                    if !fixed[i] {
                        prof[i] = prof[i].clamp(prof[i + 1] - GRADE, prof[i + 1] + GRADE);
                    }
                }
            }
            let half = way.width as f32 / 2.0;
            let shoulder = 4.0f32;
            let reach = (half + shoulder).ceil() as i32;
            // nearest point of the line per column
            // (the height is blended over the points close by, so the road
            // climbs evenly however its points fall on the columns)
            let mut near: BTreeMap<usize, (f32, f32, f32)> = BTreeMap::new();
            for (p, h) in line.iter().zip(&prof) {
                let c = cell(p);
                for dz in -reach..=reach {
                    for dx in -reach..=reach {
                        let Some(i) = self.idx(c[0] + dx, c[1] + dz) else {
                            continue;
                        };
                        let d = ((c[0] + dx) as f32 - p[0]).hypot((c[1] + dz) as f32 - p[1]);
                        if d > half + shoulder {
                            continue;
                        }
                        let e = near.entry(i).or_insert((f32::MAX, 0.0, 0.0));
                        e.0 = e.0.min(d);
                        let weight = 1.0 / (0.4 + d).powi(6);
                        e.1 += weight * h;
                        e.2 += weight;
                    }
                }
            }
            for (i, (d, sum, weight)) in near {
                let h = sum / weight;
                let f = self.flags[i];
                // a ragged edge, less so on a narrow trail
                let rag = 0.004 * (half / 3.5).min(1.0).powi(2);
                let on = d <= half - 0.01 + (rnd(i as i32, 7, 63) % 100) as f32 * rag;
                if f & WET != 0 {
                    // a ford: the bed comes up under a trail
                    if way.ford && on {
                        self.g[i] = sea - 1.0;
                        self.paint[i] = block_by_key(&way.ford_bed).unwrap_or(COBBLE);
                        self.flags[i] |= ROAD;
                    }
                    continue;
                }
                if f & SITE == 0 {
                    let t = smooth((d - half) / shoulder);
                    self.g[i] = h + (self.g[i] - h) * t;
                }
                if on {
                    self.flags[i] |= ROAD;
                    if f & SITE == 0 || self.paint[i] == AIR {
                        // the kerb: the outermost metre, a little broken
                        let kerb = way
                            .kerb
                            .as_deref()
                            .filter(|_| d > half - 1.0 && !rnd(i as i32, 5, 68).is_multiple_of(6));
                        if let Some(k) =
                            kerb.or_else(|| Layout::pick(&way.surface, rnd(i as i32, 3, 64)))
                        {
                            self.paint[i] = block_by_key(k).unwrap_or(AIR);
                        }
                    }
                }
            }
            // lamps beside the way
            if let Some(lamp) = &way.lamp
                && way.lamp_every >= 4.0
                && self.stamps.get(lamp).is_some()
            {
                let every = way.lamp_every as usize;
                let mut side = 1.0;
                for i in (every / 2..line.len().saturating_sub(2)).step_by(every) {
                    let (a, b) = (line[i], line[i + 1]);
                    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
                    let n = dx.hypot(dz).max(0.01);
                    let off = half + 1.5;
                    let (x, z) = (
                        (a[0] - dz / n * off * side).round() as i32,
                        (a[1] + dx / n * off * side).round() as i32,
                    );
                    side = -side;
                    let Some(j) = self.idx(x, z) else { continue };
                    if self.flags[j] != 0 {
                        continue;
                    }
                    self.flags[j] |= SITE;
                    self.puts.push((
                        road.key.clone(),
                        lamp.clone(),
                        [x + self.cx, self.top(x, z) + 1, z + self.cz],
                        0,
                    ));
                }
            }
            self.built.roads.push(BuiltRoad {
                key: road.key.clone(),
                way: road.way.clone(),
                width: way.width,
                line: line
                    .iter()
                    .map(|p| {
                        let c = cell(p);
                        [c[0] + self.cx, 0, c[1] + self.cz]
                    })
                    .collect(),
            });
        }
    }

    /// Write the height map to the world.
    fn commit(&mut self, w: &mut World) {
        let l = self.l;
        let sea = l.terrain.sea;
        let v = l.rise();
        let lines = l
            .terrain
            .rock_line
            .map(|r| [l.terrain.ground + r[0] * v, l.terrain.ground + r[1] * v]);
        for wz in 0..self.sz {
            for wx in 0..self.sx {
                let i = (wz * self.sx + wx) as usize;
                let (x, z) = (wx - self.cx, wz - self.cz);
                let h = (self.g[i].round() as i32).clamp(2, w.size[1] as i32 - 2);
                let wet = self.flags[i] & WET != 0 && h < sea;
                // how steep: the largest step to a neighbour
                let steep = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .map(|(dx, dz)| (self.top(x + dx, z + dz) - h).abs())
                    .max()
                    .unwrap_or(0);
                let r = rnd(x, z, 65);
                let mut top = self.paint[i];
                let mut under = DIRT;
                if top == AIR {
                    top = if wet || h <= sea {
                        if r.is_multiple_of(7) { CLAY } else { SAND }
                    } else if steep >= 2 {
                        STONE
                    } else if let Some([rock, snow]) = lines
                        && h as f32 > rock + (r % 5) as f32
                    {
                        if h as f32 > snow + (r % 4) as f32 {
                            SNOW
                        } else {
                            STONE
                        }
                    } else if h == sea + 1 && self.near_water(x, z) {
                        SAND
                    } else {
                        GRASS
                    };
                    if top == GRASS && self.flags[i] == 0 {
                        let mix = &l.flora.mixes[l.mix_at(x as f32, z as f32)];
                        if let Some(p) = &mix.patches
                            && value_noise(
                                x as f32 / p.scale,
                                z as f32 / p.scale,
                                l.seed.wrapping_add(95),
                            ) > 1.0 - p.share
                        {
                            top = block_by_key(&p.block).unwrap_or(GRASS);
                        }
                    }
                }
                if matches!(top, STONE | SNOW) || steep >= 2 {
                    // rock shows in the steps of a steep slope
                    under = STONE;
                } else if matches!(top, SAND | CLAY) {
                    under = SAND;
                }
                w.set(wx, 0, wz, BEDROCK);
                for y in 1..=h {
                    let b = if y == h {
                        top
                    } else if y > h - 4 {
                        under
                    } else {
                        STONE
                    };
                    w.set(wx, y, wz, b);
                }
                if wet {
                    for y in h + 1..=sea {
                        w.set(wx, y, wz, WATER);
                    }
                }
            }
        }
        for road in &mut self.built.roads {
            for p in &mut road.line {
                let i = (p[2] * self.sx + p[0]) as usize;
                p[1] = (self.g[i].round() as i32).clamp(2, w.size[1] as i32 - 2);
            }
        }
    }

    fn near_water(&self, x: i32, z: i32) -> bool {
        (-2..=2).any(|dz| {
            (-2..=2).any(|dx| {
                self.idx(x + dx, z + dz)
                    .is_some_and(|i| self.flags[i] & WET != 0)
            })
        })
    }

    /// Put the stamps down and keep vegetation off them.
    fn stamp(&mut self, w: &mut World) {
        let puts = std::mem::take(&mut self.puts);
        for (place, name, at, rot) in &puts {
            let Some(s) = self.stamps.get(name) else {
                continue;
            };
            s.place(w, *at, *rot);
            for z in 0..s.size[2] {
                for x in 0..s.size[0] {
                    if s.covers(x, z) {
                        let p = s.to_world(*at, *rot, [x, 0, z]);
                        if let Some(i) = self.idx(p[0] - self.cx, p[2] - self.cz) {
                            self.flags[i] |= SITE;
                        }
                    }
                }
            }
            if self.built.hearth == [0, 0, 0]
                && place == &self.l.places[0].key
                && let Some(h) = s.point("hearth", *at, *rot)
            {
                self.built.hearth = h;
            }
        }
        self.built.stamps = puts;
    }

    fn plant(&mut self, w: &mut World, key: &str, sp: &Species, x: i32, z: i32, r: u32) -> bool {
        let Some(i) = self.idx(x, z) else {
            return false;
        };
        let g = self.g[i].round() as i32;
        let tree = grow(sp, r);
        let (height, _, crown) = tree_extent(&tree);
        if g + height >= w.size[1] as i32 - 1 {
            return false;
        }
        let reach = crown / 2 + 2;
        if self.idx(x - reach, z - reach).is_none() || self.idx(x + reach, z + reach).is_none() {
            return false;
        }
        let (wx, wz) = (x + self.cx, z + self.cz);
        for b in tree {
            let [dx, dy, dz] = b.at;
            // boughs keep clear of the roads: room to run and ride under them
            if dy <= 6
                && !b.wood
                && self
                    .idx(x + dx, z + dz)
                    .is_some_and(|j| self.flags[j] & ROAD != 0)
            {
                continue;
            }
            let here = w.get(wx + dx, g + dy, wz + dz);
            if b.wood {
                if here == AIR || is_leaves(here) || matches!(here, GRASS | DIRT) {
                    w.set(wx + dx, g + dy, wz + dz, b.block);
                }
            } else if here == AIR {
                w.set(wx + dx, g + dy, wz + dz, b.block);
            }
        }
        self.built.trees.push((key.to_string(), [wx, g, wz]));
        true
    }

    /// Trees and small plants: woods in clumps, open ground between them.
    fn flora(&mut self, w: &mut World) {
        let l = self.l;
        // the ones planted by hand first
        for p in &l.places {
            if !self.built.places.contains_key(&p.key) {
                continue;
            }
            let at = l.place_at(p);
            for (n, t) in p.trees.iter().enumerate() {
                let sp = &l.flora.species[&t.species];
                // where the layout says, or a little further out if a road
                // has come to run there
                let len = (t.at[0] as f32).hypot(t.at[1] as f32).max(1.0);
                let reach = sp.girth[1] + 2;
                let spot = (0..12).map(|k| {
                    let k = k as f32;
                    (
                        at[0] + t.at[0] + (t.at[0] as f32 / len * k).round() as i32,
                        at[1] + t.at[1] + (t.at[1] as f32 / len * k).round() as i32,
                    )
                });
                let free = spot.into_iter().find(|(x, z)| {
                    (-reach..=reach).all(|dz| {
                        (-reach..=reach).all(|dx| {
                            self.idx(x + dx, z + dz)
                                .is_none_or(|j| self.flags[j] & ROAD == 0)
                        })
                    })
                });
                if let Some((x, z)) = free {
                    self.plant(w, &t.species, sp, x, z, rnd(x, z, 70 + n as u32));
                }
            }
        }
        // one candidate per cell of a grid, jittered
        const CELL: i32 = 4;
        let mut taken = vec![false; self.g.len()];
        for t in &self.built.trees {
            let i = (t.1[2] * self.sx + t.1[0]) as usize;
            taken[i] = true;
        }
        let sea = l.terrain.sea;
        let rock = l
            .terrain
            .rock_line
            .map_or(f32::MAX, |r| l.terrain.ground + r[0] * l.rise());
        for cz in (-self.cz + 12) / CELL..(self.sz - self.cz - 12) / CELL {
            for cx in (-self.cx + 12) / CELL..(self.sx - self.cx - 12) / CELL {
                let r = rnd(cx, cz, 80);
                let (x, z) = (
                    cx * CELL + (r % CELL as u32) as i32,
                    cz * CELL + ((r >> 4) % CELL as u32) as i32,
                );
                let Some(i) = self.idx(x, z) else { continue };
                let f = self.flags[i];
                if f & (ROAD | WET) != 0 || (f & SITE != 0 && f & WOODED == 0) {
                    continue;
                }
                let g = self.g[i].round() as i32;
                if g <= sea
                    || g as f32 > rock
                    || !matches!(w.get(x + self.cx, g, z + self.cz), GRASS | DIRT)
                {
                    continue;
                }
                if w.get(x + self.cx, g + 1, z + self.cz) != AIR {
                    continue;
                }
                let mix = &l.flora.mixes[l.mix_at(x as f32, z as f32)];
                let n = value_noise(
                    x as f32 / mix.clump,
                    z as f32 / mix.clump,
                    l.seed.wrapping_add(96),
                );
                // value noise bunches round the middle: stretch the cover over it
                let wood = n > 0.5 + (0.5 - mix.cover) * 0.72;
                let roll = ((r >> 8) % 100_000) as f32 / 100_000.0;
                let chance = if wood {
                    (CELL as f32 / mix.spacing).powi(2)
                } else {
                    mix.lone * (CELL * CELL) as f32 / 10_000.0
                };
                if roll >= chance {
                    continue;
                }
                let Some(key) = Layout::pick(&mix.trees, r >> 3) else {
                    continue;
                };
                // keep trunks apart
                let gap = (mix.spacing * 0.55) as i32;
                let crowded = (-gap..=gap).any(|dz| {
                    (-gap..=gap).any(|dx| self.idx(x + dx, z + dz).is_some_and(|j| taken[j]))
                });
                if crowded {
                    continue;
                }
                let sp = &l.flora.species[key];
                // not on a road's edge or against a wall
                let girth = sp.girth[1];
                let blocked = (-girth - 1..=girth + 1).any(|dz| {
                    (-girth - 1..=girth + 1).any(|dx| {
                        self.idx(x + dx, z + dz).is_none_or(|j| {
                            self.flags[j] & (ROAD | WET) != 0
                                || (self.flags[j] & SITE != 0 && self.flags[j] & WOODED == 0)
                        })
                    })
                });
                if blocked {
                    continue;
                }
                if self.plant(w, key, sp, x, z, rnd(x, z, 81)) {
                    taken[i] = true;
                }
            }
        }
        // small things on the ground
        for wz in 2..self.sz - 2 {
            for wx in 2..self.sx - 2 {
                let i = (wz * self.sx + wx) as usize;
                if self.flags[i] != 0 {
                    continue;
                }
                let (x, z) = (wx - self.cx, wz - self.cz);
                let g = self.g[i].round() as i32;
                if w.get(wx, g, wz) != GRASS || w.get(wx, g + 1, wz) != AIR {
                    continue;
                }
                let mix = &l.flora.mixes[l.mix_at(x as f32, z as f32)];
                if mix.ground.is_empty() {
                    continue;
                }
                let n = value_noise(
                    x as f32 / mix.clump,
                    z as f32 / mix.clump,
                    l.seed.wrapping_add(96),
                );
                let wood = n > 0.5 + (0.5 - mix.cover) * 0.72;
                let mut roll = (rnd(x, z, 82) % 1_000_000) as f32 / 100.0;
                for d in &mix.ground {
                    if (d.on == "wood" && !wood) || (d.on == "open" && wood) {
                        continue;
                    }
                    if roll < d.per_ha {
                        w.set(wx, g + 1, wz, block_by_key(&d.block).unwrap_or(AIR));
                        break;
                    }
                    roll -= d.per_ha;
                }
            }
        }
    }
}

/// The blocks of a tree of a species, standing on the ground block at the
/// origin. `r` varies it within the species' ranges.
pub fn grow(sp: &Species, r: u32) -> Vec<TreeBlock> {
    let mut out = Vec::new();
    let range = |a: [i32; 2], salt: u32| {
        a[0] + (hash(r as i32, salt as i32, 9) % (a[1] - a[0] + 1) as u32) as i32
    };
    let h = range(sp.height, 1);
    // thicker trunks for the taller trees of a species
    let span = (sp.height[1] - sp.height[0]).max(1);
    let girth = (sp.girth[0] + ((h - sp.height[0]) * (sp.girth[1] - sp.girth[0] + 1) / (span + 1)))
        .clamp(sp.girth[0], sp.girth[1]);
    let trunk = block_by_key(&sp.trunk).unwrap_or(LOG);
    let leaves = sp.leaves.as_deref().and_then(block_by_key);
    let wood = |out: &mut Vec<TreeBlock>, at: [i32; 3]| {
        out.push(TreeBlock {
            at,
            block: trunk,
            wood: true,
        })
    };
    let jit = |x: i32, y: i32, z: i32| (hash(x * 7 + y, z * 13 - y, r) % 100) as f32 / 100.0;
    // the trunk stands on cells 0..girth; the tree's axis is their middle
    let mid = (girth - 1) as f32 / 2.0;
    let top = if sp.crown.shape == "none" { h } else { h - 2 };
    for y in 1..=top {
        // a tall trunk tapers near the top
        let g = if girth >= 2 && y > top - (h / 4) {
            girth - 1
        } else {
            girth
        };
        let o = (girth - g) / 2;
        for dz in o..o + g {
            for dx in o..o + g {
                wood(&mut out, [dx, y, dz]);
            }
        }
    }
    if sp.roots {
        // buttress roots at the foot
        let n = if girth >= 2 { 2 } else { 1 };
        for k in 0..girth {
            for (dx, dz) in [(-1, k), (girth, k), (k, -1), (k, girth)] {
                if !hash(dx * 5 + k, dz * 3, r ^ 0x51).is_multiple_of(3) {
                    for y in 1..=n {
                        wood(&mut out, [dx, y, dz]);
                    }
                    // and a toe beyond, where the foot flares
                    if sp.flare && hash(dx * 3 - k, dz * 7 + k, r ^ 0x77).is_multiple_of(2) {
                        let toe = if dx < 0 {
                            [dx - 1, 1, dz]
                        } else if dx >= girth {
                            [dx + 1, 1, dz]
                        } else if dz < 0 {
                            [dx, 1, dz - 1]
                        } else {
                            [dx, 1, dz + 1]
                        };
                        wood(&mut out, toe);
                    }
                }
            }
        }
    }
    let Some(leaves) = leaves else {
        // a dead tree: bare branches that reach out and up, some forked
        // (with no `limbs`, a snag with a broken stub or two)
        let n = sp.limbs.max(1) as i32;
        for k in 0..n {
            let pick = hash(k, 21, r);
            if sp.limbs == 0 && pick & 1 == 0 {
                continue;
            }
            let (dx, dz) =
                [(1, 0), (0, 1), (-1, 0), (0, -1)][((r >> 12) as usize + k as usize) % 4];
            let mut at = [
                if dx > 0 {
                    girth
                } else if dx < 0 {
                    -1
                } else {
                    (pick >> 4) as i32 % girth
                },
                (h as f32 * (0.5 + 0.38 * k as f32 / n as f32)) as i32,
                if dz > 0 {
                    girth
                } else if dz < 0 {
                    -1
                } else {
                    (pick >> 4) as i32 % girth
                },
            ];
            let len = if sp.limbs == 0 {
                1
            } else {
                2 + (pick >> 8) as i32 % 3
            };
            for step in 0..len {
                wood(&mut out, at);
                // out, and up every other step
                at[0] += dx;
                at[2] += dz;
                if step % 2 == 0 {
                    wood(&mut out, at);
                    at[1] += 1;
                }
            }
            if sp.limbs > 0 && (pick >> 16) & 1 == 1 {
                // a fork: a twig to one side of the tip
                wood(&mut out, [at[0] - dx + dz, at[1], at[2] - dz + dx]);
                wood(&mut out, [at[0] - dx + dz, at[1] + 1, at[2] - dz + dx]);
            }
        }
        return out;
    };
    let leaf = |out: &mut Vec<TreeBlock>, at: [i32; 3]| {
        if sp.thin > 0.0 && jit(at[2], at[1], at[0]) < sp.thin {
            return;
        }
        out.push(TreeBlock {
            at,
            block: leaves,
            wood: false,
        })
    };
    let width = range(sp.crown.width, 2).max(3);
    let half = width as f32 / 2.0;
    let skirt = ((h as f32 * sp.skirt).round() as i32).clamp(2, h - 2);
    // leaves keep inside the crown's width, measured from the axis
    let lo = (mid - half + 0.5).ceil() as i32;
    let hi = lo + width - 1;
    match sp.crown.shape.as_str() {
        "cone" => {
            for y in skirt..=h {
                let t = (h - y) as f32 / (h - skirt).max(1) as f32;
                let mut rad = 0.7 + (half - 0.7) * t;
                if (h - y) % 3 == 0 {
                    rad *= 0.5 + 0.22 * (1.0 - sp.crown.gaps).clamp(0.0, 1.0);
                }
                for dz in lo..=hi {
                    for dx in lo..=hi {
                        let d2 = (dx as f32 - mid).powi(2) + (dz as f32 - mid).powi(2);
                        if d2 <= rad * rad * (0.85 + 0.3 * jit(dx, y, dz)) {
                            leaf(&mut out, [dx, y, dz]);
                        }
                    }
                }
            }
            leaf(&mut out, [mid.floor() as i32, h, mid.floor() as i32]);
        }
        _ => {
            // leaf clumps: one on top of the trunk and others round it,
            // each on a limb; how many, how big, how high and how far apart
            // is the species' (a spreading oak, a slender birch)
            let c = &sp.crown;
            let depth = (h - skirt) as f32;
            let unit = |k: i32, salt: i32| (hash(k, salt, r) % 1000) as f32 / 1000.0;
            let tall = c.shape == "tall";
            let core = half * c.core;
            let mut clumps = vec![if tall {
                (mid, h as f32 - core, mid, core, core.max(1.5))
            } else {
                (mid, h as f32 - depth * 0.5, mid, core, depth * 0.5)
            }];
            if tall {
                // and a fuller one at the waist, where the crown is widest
                let cy = skirt as f32 + depth * 0.4;
                clumps.push((mid, cy, mid, half * 0.92, (depth * 0.28).max(1.6)));
            }
            let span = c.clumps[1].saturating_sub(c.clumps[0]) + 1;
            let lobes = c.clumps[0] + (r >> 20) % span + if width >= 13 { 2 } else { 0 };
            for k in 0..lobes {
                let ki = k as i32;
                // round the trunk; a tall crown winds its clumps up it
                let turn = if tall {
                    2.4
                } else {
                    std::f32::consts::TAU / lobes as f32
                };
                let ang = k as f32 * turn + (r % 628) as f32 / 100.0;
                let rr = (half * (c.lump[0] + (c.lump[1] - c.lump[0]) * unit(ki, 3))).max(1.2);
                let up = if tall {
                    (k as f32 + 0.5) / lobes as f32
                } else {
                    c.rise[0] + (c.rise[1] - c.rise[0]) * unit(ki, 5)
                };
                let out_by = if tall {
                    (half - rr) * (1.0 - 0.5 * up)
                } else {
                    half - rr
                }
                .max(0.0);
                let cy = skirt as f32 + depth * up;
                let ry = if tall {
                    (depth / lobes as f32 * 0.8).max(1.6)
                } else {
                    (depth * 0.34).max(2.0)
                };
                clumps.push((
                    mid + ang.cos() * out_by,
                    cy.min(h as f32 - ry * 0.5),
                    mid + ang.sin() * out_by,
                    rr,
                    ry,
                ));
                // the limb that carries it
                if (k as u8) < sp.limbs {
                    let steps = out_by.round() as i32;
                    for s in 1..=steps {
                        let t = s as f32 / steps.max(1) as f32;
                        let x = mid + ang.cos() * out_by * t;
                        let z = mid + ang.sin() * out_by * t;
                        let y0 = if tall { cy - 1.0 } else { skirt as f32 - 1.0 };
                        let y = y0 + (cy - y0) * t;
                        wood(
                            &mut out,
                            [x.round() as i32, y.round() as i32, z.round() as i32],
                        );
                    }
                }
            }
            let edge = 0.8 - c.gaps.clamp(0.0, 0.5);
            let mut lowest = std::collections::HashMap::new();
            for y in skirt - 1..=h {
                for dz in lo..=hi {
                    for dx in lo..=hi {
                        let inside = clumps.iter().any(|(cx, cy, cz, rr, ry)| {
                            let a =
                                ((dx as f32 - cx).powi(2) + (dz as f32 - cz).powi(2)) / (rr * rr);
                            let b = (y as f32 - cy) / ry;
                            a + b * b <= edge + 0.3 * jit(dx, y, dz)
                        });
                        if inside {
                            leaf(&mut out, [dx, y, dz]);
                            lowest.entry((dx, dz)).or_insert(y);
                        }
                    }
                }
            }
            if c.shape == "weeping" && c.droop[1] > 0 {
                // curtains of leaves hang from the rim of the dome
                for ((dx, dz), y) in lowest {
                    let d = ((dx as f32 - mid).powi(2) + (dz as f32 - mid).powi(2)).sqrt();
                    if d < half * 0.5 || jit(dx, 77, dz) < 0.35 {
                        continue;
                    }
                    let len = c.droop[0]
                        + (jit(dz, 91, dx) * (c.droop[1] - c.droop[0] + 1) as f32) as i32;
                    for yy in (y - len).max(2)..y {
                        leaf(&mut out, [dx, yy, dz]);
                    }
                }
            }
            // the top of the trunk is under leaves, however thin the crown
            out.push(TreeBlock {
                at: [mid.floor() as i32, h, mid.floor() as i32],
                block: leaves,
                wood: false,
            });
        }
    }
    out
}

/// Height, trunk thickness at chest height, and crown width of a tree.
pub fn tree_extent(blocks: &[TreeBlock]) -> (i32, i32, i32) {
    let height = blocks.iter().map(|b| b.at[1]).max().unwrap_or(0);
    let trunk: Vec<i32> = blocks
        .iter()
        .filter(|b| b.wood && b.at[1] == 3 && b.at[2] == 0)
        .map(|b| b.at[0])
        .collect();
    let girth = trunk
        .iter()
        .max()
        .zip(trunk.iter().min())
        .map_or(0, |(a, b)| a - b + 1);
    let xs: Vec<i32> = blocks.iter().filter(|b| !b.wood).map(|b| b.at[0]).collect();
    let crown = xs
        .iter()
        .max()
        .zip(xs.iter().min())
        .map_or(0, |(a, b)| a - b + 1);
    (height, girth, crown)
}

/// Build a world of `size` from a layout: the whole of it, or the window
/// of that size round the origin.
pub fn build(l: &Layout, stamps: &Stamps, size: [u32; 3]) -> Result<(World, Built), String> {
    l.check_stamps(stamps)?;
    let size = [
        size[0].min(l.size[0]),
        size[1].min(l.size[1]),
        size[2].min(l.size[2]),
    ];
    let (sx, sz) = (size[0] as i32, size[2] as i32);
    let n = (sx * sz) as usize;
    let mut w = World::empty(size);
    let mut g = Gen {
        l,
        stamps,
        sx,
        sz,
        cx: sx / 2,
        cz: sz / 2,
        g: vec![0.0; n],
        flags: vec![0; n],
        paint: vec![AIR; n],
        puts: Vec::new(),
        built: Built::default(),
    };
    g.land();
    g.sites()?;
    g.roads();
    g.commit(&mut w);
    g.stamp(&mut w);
    g.flora(&mut w);
    let hub = &l.places[0];
    if g.built.hearth == [0, 0, 0] {
        // a hub without a hearth stamp still gets its hearth
        let at = g.built.places.get(&hub.key).copied().unwrap_or([
            g.cx,
            w.surface(g.cx, g.cz) + 1,
            g.cz,
        ]);
        w.set(at[0], at[1], at[2], HEARTH);
        g.built.hearth = at;
    }
    g.built.flags = g.flags;
    Ok((w, g.built))
}

/// Put one place of a layout (its pad and stamps) on an existing world at
/// `origin`, the block above its ground: for tools and tests that want a
/// town on a small piece of land. Stamps that do not fit are left out.
pub fn build_place(w: &mut World, _l: &Layout, stamps: &Stamps, place: &Place, origin: [i32; 3]) {
    let level = origin[1] - 1;
    if let Some(pad) = &place.pad {
        let (bx, bz) = (pad.radius[0] as i32, pad.radius[1] as i32);
        for dz in -bz..=bz {
            for dx in -bx..=bx {
                let m = ((dx as f32 / pad.radius[0]).powi(2) + (dz as f32 / pad.radius[1]).powi(2))
                    .sqrt();
                let (x, z) = (origin[0] + dx, origin[2] + dz);
                if m > 1.0 || !w.in_bounds([x, 1, z]) {
                    continue;
                }
                for y in level + 1..w.size[1] as i32 {
                    w.set(x, y, z, AIR);
                }
                let top = pad
                    .surface
                    .as_ref()
                    .and_then(|s| Layout::pick(s, rnd(dx, dz, 62)))
                    .and_then(block_by_key)
                    .unwrap_or(GRASS);
                w.set(x, level, z, top);
                for y in (1..level).rev() {
                    if is_solid(w.get(x, y, z)) && !is_leaves(w.get(x, y, z)) {
                        break;
                    }
                    w.set(x, y, z, DIRT);
                }
            }
        }
    }
    for put in &place.stamps {
        let Some(s) = stamps.get(&put.stamp) else {
            continue;
        };
        let at = [
            origin[0] + put.at[0],
            origin[1] + put.dy,
            origin[2] + put.at[1],
        ];
        let (lo, hi) = s.footprint(put.rot);
        if w.in_bounds([at[0] + lo[0], 1, at[2] + lo[1]])
            && w.in_bounds([at[0] + hi[0], 1, at[2] + hi[1]])
        {
            s.place(w, at, put.rot);
        }
    }
}

/// Reading the workspace's pack from disk, for tools and tests inside the
/// workspace (the server gets its pack from `dyno-server`).
pub mod dev {
    use super::*;

    /// The workspace pack's layout and stamps, if this program runs where
    /// the workspace is.
    pub fn pack() -> Option<(Layout, Stamps)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let text = std::fs::read_to_string(root.join("world/layout.json")).ok()?;
        let layout = Layout::from_json(&text).ok()?;
        let mut files = Vec::new();
        for e in std::fs::read_dir(root.join("stamps")).ok()?.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Ok(b) = std::fs::read(e.path()) {
                files.push((format!("stamps/{name}"), b));
            }
        }
        let stamps =
            Stamps::from_files(files.iter().map(|(p, b)| (p.as_str(), b.as_slice()))).ok()?;
        Some((layout, stamps))
    }

    /// Put the layout's hub round a hearth of an existing world.
    pub fn hub(world: &mut World, hearth: [i32; 3]) -> bool {
        let Some((layout, stamps)) = pack() else {
            return false;
        };
        build_place(world, &layout, &stamps, &layout.places[0], hearth);
        world.set(hearth[0], hearth[1], hearth[2], HEARTH);
        true
    }
}

/// A top-down picture of a world, one pixel per column: RGB rows, north
/// up. Land is shaded by its slope; roads, water and roofs show as they are.
pub fn map(w: &World) -> Vec<u8> {
    let (sx, sz) = (w.size[0] as i32, w.size[2] as i32);
    let tops: Vec<i32> = (0..sz)
        .flat_map(|z| (0..sx).map(move |x| (x, z)))
        .map(|(x, z)| {
            (0..w.size[1] as i32)
                .rev()
                .find(|y| w.get(x, *y, z) != AIR)
                .unwrap_or(0)
        })
        .collect();
    let mut out = Vec::with_capacity((sx * sz * 3) as usize);
    for z in 0..sz {
        for x in 0..sx {
            let y = tops[(z * sx + x) as usize];
            let b = w.get(x, y, z);
            let c = block(b).color;
            let west = tops[(z * sx + (x - 1).max(0)) as usize];
            let north = tops[((z - 1).max(0) * sx + x) as usize];
            // lit from the north-west; higher land is a little lighter
            let slope = ((y - west) + (y - north)) as f32;
            let shade = if b == WATER {
                1.0
            } else {
                (0.86 + slope * 0.07 + (y as f32 - 50.0) * 0.0035).clamp(0.5, 1.35)
            };
            for ch in c {
                out.push((ch * shade * 255.0).clamp(0.0, 255.0) as u8);
            }
        }
    }
    out
}

/// Measurements of a built world against the scale tables: what tests and
/// the map tool read.
pub mod measure {
    use super::*;

    /// The clear height of the doorway whose lowest open cell is `at`.
    pub fn door_height(w: &World, at: [i32; 3]) -> i32 {
        (0..12)
            .take_while(|dy| {
                !is_solid(w.get(at[0], at[1] + dy, at[2]))
                    || w.get(at[0], at[1] + dy, at[2]) == WOODEN_DOOR
            })
            .count() as i32
    }

    /// The width of a road at a point of its centre line: paved (flagged)
    /// columns across it, at right angles to the way it runs.
    pub fn road_width(b: &Built, size: [u32; 3], road: &BuiltRoad, i: usize) -> i32 {
        let n = road.line.len();
        let (a, c) = (
            road.line[i.saturating_sub(3)],
            road.line[(i + 3).min(n - 1)],
        );
        let (dx, dz) = ((c[0] - a[0]) as f32, (c[2] - a[2]) as f32);
        let len = dx.hypot(dz).max(0.01);
        let (nx, nz) = (-dz / len, dx / len);
        let p = road.line[i];
        let on = |t: f32| {
            let (x, z) = (
                (p[0] as f32 + nx * t).round() as i32,
                (p[2] as f32 + nz * t).round() as i32,
            );
            x >= 0
                && z >= 0
                && x < size[0] as i32
                && z < size[2] as i32
                && b.flags[(z * size[0] as i32 + x) as usize] & ROAD != 0
        };
        let mut w = 0.0f32;
        for side in [-1.0f32, 1.0] {
            let mut t = 0.0;
            while t < 30.0 && on(side * (t + 0.5)) {
                t += 0.5;
            }
            w += t;
        }
        w.round() as i32
    }

    /// The length of a line of blocks, walked.
    pub fn length(line: &[[i32; 3]]) -> f32 {
        line.windows(2)
            .map(|w| ((w[1][0] - w[0][0]) as f32).hypot((w[1][2] - w[0][2]) as f32))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn species(json: &str) -> Species {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn trees_keep_inside_their_species_ranges() {
        let oak = species(
            r#"{"trunk":"log","leaves":"leaves","height":[11,16],"girth":[1,2],
                "crown":{"shape":"round","width":[8,12]},"limbs":3}"#,
        );
        let hero = species(
            r#"{"trunk":"log","leaves":"leaves","height":[18,27],"girth":[2,3],
                "crown":{"shape":"round","width":[14,18]},"limbs":5,"roots":true}"#,
        );
        let pine = species(
            r#"{"trunk":"log","leaves":"pine_leaves","height":[15,21],"girth":[1,2],
                "crown":{"shape":"cone","width":[7,9]},"skirt":0.25}"#,
        );
        let birch = species(
            r#"{"trunk":"birch_log","leaves":"birch_leaves","height":[11,15],"girth":[1,1],
                "crown":{"shape":"tall","width":[5,7],"clumps":[4,6],"core":0.6,
                "lump":[0.5,0.7],"gaps":0.1},"skirt":0.4,"limbs":6}"#,
        );
        let willow = species(
            r#"{"trunk":"log","leaves":"birch_leaves","height":[11,14],"girth":[2,2],
                "crown":{"shape":"weeping","width":[10,12],"droop":[3,6]},
                "skirt":0.45,"limbs":4,"roots":true,"flare":true}"#,
        );
        // a willow's curtains hang well below where its dome begins
        let w = grow(&willow, 7);
        let low = w.iter().filter(|b| !b.wood).map(|b| b.at[1]).min().unwrap();
        assert!(low <= 4, "the willow weeps to {low}");
        // a dead tree has branches off its trunk
        let dead = species(
            r#"{"trunk":"log","height":[8,11],"girth":[1,2],"crown":{"shape":"none"},"limbs":4}"#,
        );
        let d = grow(&dead, 11);
        assert!(d.iter().all(|b| b.wood));
        assert!(
            d.iter().any(|b| b.at[0] < -1 || b.at[0] > 2),
            "it has branches"
        );
        for (sp, name) in [
            (&oak, "oak"),
            (&hero, "hero"),
            (&pine, "pine"),
            (&birch, "birch"),
            (&willow, "willow"),
        ] {
            let mut seen = (i32::MAX, 0);
            for r in 0..200u32 {
                let t = grow(sp, r.wrapping_mul(2_654_435_761));
                let (h, girth, crown) = tree_extent(&t);
                assert!(
                    (sp.height[0]..=sp.height[1]).contains(&h),
                    "{name} height {h}"
                );
                assert!(
                    (sp.girth[0]..=sp.girth[1]).contains(&girth),
                    "{name} girth {girth}"
                );
                assert!(
                    crown <= sp.crown.width[1] && crown >= sp.crown.width[0] - 2,
                    "{name} crown {crown}"
                );
                seen = (seen.0.min(h), seen.1.max(h));
            }
            assert_eq!(seen, (sp.height[0], sp.height[1]), "{name} uses its range");
        }
    }
}
