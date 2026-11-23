//! Fine voxel terrain. The world keeps its 1 m block grid (storage, light,
//! physics, saves); the client draws each natural block (the pack's
//! `terrain` tag) as a sub-grid of fine voxels, `per_metre` a side, shaped
//! by the blocks around it:
//!
//! - **Shape.** Each ground layer is blurred over its 3 x 3 neighbourhood
//!   (built blocks do not count), giving a field per block that is positive
//!   in ground and negative in air. A fine voxel is solid where the field,
//!   read between block centres at the voxel's centre, is above zero. Level
//!   ground stays exactly on the block top; a one-block step becomes a
//!   stepped slope about three blocks long; a wall two or more blocks high
//!   stays upright on the grid line, so cliffs keep sharp ledges.
//! - **Raggedness.** Where a layer is mixed (ground and air side by side),
//!   value noise and horizontal strata nudge the field, so slopes, banks
//!   and cliff faces get irregular fine-voxel edges and ledges. Where two
//!   kinds of ground meet, each voxel takes its kind from a point nudged by
//!   noise, so path edges and shorelines are ragged, not straight 1 m lines.
//! - **Cover.** A block with an `under` (grass over dirt, snow over stone)
//!   paints the voxels within `cover_depth` of open air above them, and the
//!   `under` block below that: a grass lip over an earth bank.
//! - **Colour** is per fine voxel in the shader: the block's texture at the
//!   voxel's centre, at the mip level with one texel a voxel, times a hashed
//!   brightness jitter. Geometry merges faces of one kind of ground.
//!
//! Every voxel is decided only by the blocks within two cells of it and its
//! world position, so neighbouring sections agree at their borders. All
//! the numbers are data: `terrain` in `blocks.json`.

use crate::light::Light;
use crate::render::{self, KIND_VOXEL, SECTION, Vertex};
use crate::terrain_mesh::{
    DX, DY, DZ, F_DARK, F_LEAVES, F_OPAQUE, F_PIN, F_TERRAIN, F_WATER, Grid, SectionGeo, Tuft, at,
    hash3,
};
use dyno_world::{self as w, BlockId};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

const N: i32 = SECTION;
/// Cells -1..=N on each axis hold fine voxels: the section's own and one
/// round them, for culling.
const NC: i32 = N + 2;
/// Grid corners -1..=N+1 on each axis.
const LV: i32 = N + 3;
/// The most fine voxels a metre (a row of voxels is a `u16`).
pub const MAX_PER_METRE: u8 = 16;
/// Smallest field magnitude of a block: keeps one-block features from
/// thinning to nothing. Noise stays below it, so a block with only ground
/// round it is solid and one with none round it is empty.
const MARGIN: f32 = 0.15;

// ------------------------------------------------------------------ data

/// `terrain` in `blocks.json`.
#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct TerrainJson {
    /// fine voxels a metre near the camera (1..=16)
    pub per_metre: u8,
    /// coarser levels further out: beyond `beyond` metres, `per_metre`
    pub lod: Vec<LodJson>,
    /// field noise on slopes, banks and cliffs (the field runs -0.5..0.5)
    pub noise: f32,
    /// size of the noise's features, metres
    pub noise_size: f32,
    /// strength of the horizontal strata on cliffs and banks
    pub strata: f32,
    /// height of one stratum, metres
    pub strata_height: f32,
    /// how far kinds of ground reach into each other along a ragged edge, metres
    pub edge: f32,
    /// size of the edge's wiggles, metres
    pub edge_size: f32,
    /// a cover block (one with `under`) paints this deep below open air, metres
    pub cover_depth: f32,
    /// brightness jitter per voxel, for blocks that do not set their own
    pub jitter: f32,
    /// per block (by key)
    pub blocks: BTreeMap<String, BlockJson>,
    pub tufts: TuftsJson,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct LodJson {
    pub beyond: f32,
    pub per_metre: u8,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BlockJson {
    /// what shows under this block's cover (grass: dirt); the block of
    /// that kind just under it is painted as this one
    under: Option<String>,
    /// brightness jitter per voxel
    jitter: Option<f32>,
    /// how far it reaches into other ground and they into it (0: a crisp
    /// block edge), as a multiple of `edge`
    edge: Option<f32>,
}

/// Grass and flowers near the camera, as fine voxels.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct TuftsJson {
    /// blocks (by key) whose open tops grow tufts
    pub on: Vec<String>,
    /// per cent of those blocks with a tuft
    pub per_cent: u32,
    pub kinds: Vec<TuftKind>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct TuftKind {
    pub weight: u32,
    /// size of a voxel, metres
    pub voxel: f32,
    /// blades: at least and at most
    pub blades: [u8; 2],
    /// height of a blade in voxels: at least and at most
    pub height: [u8; 2],
    /// how far from the root a blade stands, in voxels
    pub spread: u8,
    /// colour of a blade's foot and tip, times the ground's colour
    pub foot: [f32; 3],
    pub tip: [f32; 3],
    /// a flower head on the tallest blade, picked from these colours (sRGB)
    pub heads: Vec<[f32; 3]>,
    /// the head's middle (sRGB), drawn on top of it
    pub heart: Option<[f32; 3]>,
}

impl Default for TuftKind {
    fn default() -> Self {
        TuftKind {
            weight: 1,
            voxel: 0.125,
            blades: [3, 5],
            height: [1, 3],
            spread: 2,
            foot: [0.7; 3],
            tip: [1.1; 3],
            heads: Vec::new(),
            heart: None,
        }
    }
}

impl Default for TerrainJson {
    fn default() -> Self {
        TerrainJson {
            per_metre: 8,
            lod: vec![
                LodJson {
                    beyond: 48.0,
                    per_metre: 4,
                },
                LodJson {
                    beyond: 96.0,
                    per_metre: 2,
                },
            ],
            noise: 0.05,
            noise_size: 0.5,
            strata: 0.05,
            strata_height: 0.375,
            edge: 0.3,
            edge_size: 0.5,
            cover_depth: 0.25,
            jitter: 0.08,
            blocks: BTreeMap::new(),
            tufts: TuftsJson::default(),
        }
    }
}

/// The terrain settings, compiled against the block names.
pub(crate) struct Params {
    pub per_metre: u8,
    /// (beyond metres, voxels a metre), nearest first
    pub lod: Vec<(f32, u8)>,
    noise: f32,
    noise_size: f32,
    strata: f32,
    strata_height: f32,
    edge: f32,
    edge_size: f32,
    cover_depth: f32,
    under: [BlockId; 256],
    jitter: [f32; 256],
    edge_of: [f32; 256],
    pub tufts: TuftsJson,
    pub tuft_on: [bool; 256],
}

impl Params {
    pub(crate) fn compile(j: &TerrainJson, names: &HashMap<String, BlockId>) -> Params {
        let mut p = Params {
            per_metre: j.per_metre.clamp(1, MAX_PER_METRE),
            lod: Vec::new(),
            // the noise must stay inside the margin (see MARGIN)
            noise: j.noise.clamp(0.0, 0.08),
            noise_size: j.noise_size.max(0.0),
            strata: j.strata.clamp(0.0, 0.06),
            strata_height: j.strata_height.max(0.0),
            edge: j.edge.clamp(0.0, 0.9),
            edge_size: j.edge_size.max(0.0),
            cover_depth: j.cover_depth.max(0.0),
            under: [0; 256],
            jitter: [j.jitter; 256],
            edge_of: [1.0; 256],
            tufts: j.tufts.clone(),
            tuft_on: [false; 256],
        };
        let mut lod: Vec<(f32, u8)> = j
            .lod
            .iter()
            .map(|l| (l.beyond, l.per_metre.clamp(1, p.per_metre)))
            .collect();
        lod.sort_by(|a, b| a.0.total_cmp(&b.0));
        p.lod = lod;
        for (key, b) in &j.blocks {
            let Some(&id) = names.get(key) else {
                eprintln!("blocks.json: terrain lists unknown block {key}");
                continue;
            };
            let id = id as usize;
            if let Some(u) = &b.under {
                p.under[id] = names.get(u).copied().unwrap_or(0);
            }
            if let Some(v) = b.jitter {
                p.jitter[id] = v;
            }
            if let Some(v) = b.edge {
                p.edge_of[id] = v.max(0.0);
            }
        }
        for key in &j.tufts.on {
            if let Some(&id) = names.get(key) {
                p.tuft_on[id as usize] = true;
            }
        }
        p
    }

    /// Voxels a metre for a section whose nearest point is `d` metres from
    /// the camera (horizontally). `now` is what it has, for hysteresis: it
    /// turns coarser only a little past the line.
    pub(crate) fn per_metre_at(&self, d: f32, now: u8) -> u8 {
        let mut r = self.per_metre;
        for &(beyond, pm) in &self.lod {
            let slack = if now != 0 && now >= r { 8.0 } else { 0.0 };
            if d > beyond + slack {
                r = pm;
            }
        }
        r
    }
}

impl Default for Params {
    fn default() -> Self {
        Params::compile(&TerrainJson::default(), &HashMap::new())
    }
}

fn params() -> &'static Params {
    &crate::shapes::pack().terrain
}

/// How a section's terrain is drawn: voxels a metre, and for each side
/// (-x, +x, -z, +z) the neighbour's when it differs (0 when the same): a
/// skirt there hides the cracks between the two.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Lod {
    pub per_metre: u8,
    pub skirt: [u8; 4],
}

impl Lod {
    /// The finest level, with no skirts.
    pub fn finest() -> Lod {
        Lod {
            per_metre: params().per_metre,
            skirt: [0; 4],
        }
    }
}

// ----------------------------------------------------------------- noise

/// A value in -1..1 that drifts smoothly over a lattice `size` voxels a cell.
fn value2(x: i32, z: i32, size: i32, salt: i32) -> f32 {
    let (cx, cz) = (x.div_euclid(size), z.div_euclid(size));
    let (fx, fz) = (
        (x.rem_euclid(size) as f32 + 0.5) / size as f32,
        (z.rem_euclid(size) as f32 + 0.5) / size as f32,
    );
    let h = |a: i32, b: i32| (hash3(a, salt, b) & 0xffff) as f32 / 32767.5 - 1.0;
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let top = h(cx, cz) + (h(cx + 1, cz) - h(cx, cz)) * sx;
    let bot = h(cx, cz + 1) + (h(cx + 1, cz + 1) - h(cx, cz + 1)) * sx;
    top + (bot - top) * sz
}

/// -0.5..0.5 per band.
fn band(b: i32) -> f32 {
    (hash3(b, 0x5157, 0x2f1) & 0xffff) as f32 / 65535.0 - 0.5
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// ----------------------------------------------------------- the voxels

#[cfg(test)]
thread_local! {
    /// Time spent (decide, paint, emit) and counts, for measuring.
    pub(crate) static PROF: std::cell::Cell<[f64; 6]> = const { std::cell::Cell::new([0.0; 6]) };
}
#[cfg(test)]
fn prof(i: usize, ms: f64, n: f64) {
    PROF.with(|p| {
        let mut v = p.get();
        v[i] += ms;
        v[i + 3] += n;
        p.set(v);
    });
}

const UNKNOWN: u32 = u32::MAX;
const EMPTY: u32 = 0;
/// Every voxel is ground.
const SOLID: u32 = 1;
/// Filled by something else (a built block): hides faces, draws none.
const OTHER: u32 = 2;
/// Some voxels are ground: `MIXED + k` is the k-th block of rows in the pool.
const MIXED: u32 = 3;

#[inline]
fn ci(x: i32, y: i32, z: i32) -> usize {
    (((y + 1) * NC + (z + 1)) * NC + (x + 1)) as usize
}

#[inline]
fn li(i: i32, j: i32, k: i32) -> usize {
    (((j + 1) * LV + (k + 1)) * LV + (i + 1)) as usize
}

/// The eight cells around a grid corner, as offsets from the lowest one.
const CELL8: [usize; 8] = [0, DX, DZ, DZ + DX, DY, DY + DX, DY + DZ, DY + DZ + DX];

/// The six directions: normal, and the axis it runs along.
const DIRS: [([i32; 3], usize); 6] = [
    ([1, 0, 0], 0),
    ([-1, 0, 0], 0),
    ([0, 1, 0], 1),
    ([0, -1, 0], 1),
    ([0, 0, 1], 2),
    ([0, 0, -1], 2),
];

/// A face of merged voxel faces, in fine voxels from the section's corner:
/// its direction, the plane it lies in, its extent across the plane (see
/// `corner_of`), and what it shows (kind of ground and light class).
#[derive(Clone, Copy)]
struct Rect {
    dir: u8,
    plane: i32,
    u0: i32,
    v0: i32,
    u1: i32,
    v1: i32,
    key: u32,
}

/// A point of a face's plane, from its (u, v) across it.
#[inline]
fn corner_of(axis: usize, plane: i32, u: i32, v: i32) -> [i32; 3] {
    match axis {
        1 => [u, plane, v],
        2 => [u, v, plane],
        _ => [plane, v, u],
    }
}

/// The fine voxels of one section (and one cell round it), decided lazily.
pub(crate) struct Fine<'a> {
    g: &'a Grid,
    p: &'a Params,
    r: i32,
    full: u16,
    dens: Vec<f32>,
    mix: Vec<f32>,
    state: Vec<u32>,
    /// rows of voxels along x, `r * r` a cell: row (j, k) is y = j, z = k
    pool: Vec<u16>,
    /// per voxel index along an axis: which pair of block centres it lies
    /// between (0: the one before and this, 1: this and the one after) and how far
    ax: Vec<(usize, f32)>,
    /// light at grid corners: block, sky, occlusion; NaN when not yet known
    corners: Vec<[f32; 3]>,
    /// per ground cell: 1 when other kinds of ground lie beside it (its
    /// voxels may take theirs along a ragged edge), 2 when not, 0 unknown
    ragged: Vec<u8>,
}

impl<'a> Fine<'a> {
    pub(crate) fn new(g: &'a Grid, p: &'a Params, per_metre: u8) -> Fine<'a> {
        let r = per_metre.clamp(1, MAX_PER_METRE) as i32;
        let n = g.fl.len();
        let ax = (0..r)
            .map(|i| {
                let s = (i as f32 + 0.5) / r as f32 - 0.5;
                if s < 0.0 { (0, 1.0 + s) } else { (1, s) }
            })
            .collect();
        Fine {
            g,
            p,
            r,
            full: if r == 16 { u16::MAX } else { (1u16 << r) - 1 },
            dens: vec![f32::NAN; n],
            mix: vec![0.0; n],
            state: vec![UNKNOWN; (NC * NC * NC) as usize],
            pool: Vec::new(),
            ax,
            corners: vec![[f32::NAN; 3]; (LV * LV * LV) as usize],
            ragged: vec![0; n],
        }
    }

    /// The field of a block: positive in ground, negative elsewhere. Also
    /// notes whether its layer is mixed round it (where noise applies).
    fn density(&mut self, x: i32, y: i32, z: i32) -> f32 {
        let c = at(x, y, z);
        let d = self.dens[c];
        if !d.is_nan() {
            return d;
        }
        let g = self.g;
        let (mut s, mut n) = (0u32, 0u32);
        for oz in [c - DZ, c, c + DZ] {
            for q in [oz - DX, oz, oz + DX] {
                let f = g.fl[q];
                if f & F_PIN == 0 {
                    n += 1;
                    s += (f & F_TERRAIN != 0) as u32;
                }
            }
        }
        let d = s as f32 / n.max(1) as f32 - 0.5;
        let d = if g.fl[c] & F_TERRAIN != 0 {
            d.max(MARGIN)
        } else {
            d.min(-MARGIN)
        };
        self.dens[c] = d;
        self.mix[c] = if s > 0 && s < n { 1.0 } else { 0.0 };
        d
    }

    /// The state of a cell (-1..=N on each axis).
    #[inline]
    fn cell(&mut self, x: i32, y: i32, z: i32) -> u32 {
        let k = ci(x, y, z);
        let s = self.state[k];
        if s != UNKNOWN {
            return s;
        }
        let s = self.decide(x, y, z);
        self.state[k] = s;
        s
    }

    fn decide(&mut self, x: i32, y: i32, z: i32) -> u32 {
        let g = self.g;
        let c = at(x, y, z);
        let f = g.fl[c];
        let terrain = f & F_TERRAIN != 0;
        // ground may grow into air and water, nothing else
        let fill = f & !F_WATER == 0;
        if !terrain && !fill {
            return if f & F_OPAQUE != 0 { OTHER } else { EMPTY };
        }
        // ground all round: solid; no ground round: empty (the noise stays
        // inside the margin, so the field cannot cross zero)
        let mut ground = 0;
        for dy in [c - DY, c, c + DY] {
            for dz in [dy - DZ, dy, dy + DZ] {
                ground += (g.fl[dz - DX] & F_TERRAIN != 0) as u32
                    + (g.fl[dz] & F_TERRAIN != 0) as u32
                    + (g.fl[dz + DX] & F_TERRAIN != 0) as u32;
            }
        }
        if terrain && ground == 27 {
            return SOLID;
        }
        if !terrain && ground == 0 {
            return EMPTY;
        }
        #[cfg(test)]
        let t0 = std::time::Instant::now();
        // the field round the cell, and the noise's weight
        let mut d = [[[0f32; 3]; 3]; 3];
        let mut m = [[[0f32; 3]; 3]; 3];
        let mut wmax = 0f32;
        for b in 0..3 {
            for kz in 0..3 {
                for kx in 0..3 {
                    let (px, py, pz) = (x + kx as i32 - 1, y + b as i32 - 1, z + kz as i32 - 1);
                    d[b][kz][kx] = self.density(px, py, pz);
                    let mm = self.mix[at(px, py, pz)];
                    m[b][kz][kx] = mm;
                    wmax = wmax.max(mm);
                }
            }
        }
        let r = self.r;
        let p = self.p;
        let amp = p.noise + p.strata * 0.5;
        // bounds over the voxel centres: the field less and plus the most
        // the noise can do there are trilinear too, so their extremes are
        // at the corners of the box of voxel centres
        let tau = 0.5 - 0.5 / r as f32;
        let axb = [(0usize, 1.0 - tau), (1, 0.0), (1, tau)];
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for &(ly, ty) in &axb {
            for &(lz, tz) in &axb {
                for &(lx, tx) in &axb {
                    let v = tri(&d, ly, ty, lz, tz, lx, tx);
                    let w = if wmax > 0.0 {
                        tri(&m, ly, ty, lz, tz, lx, tx) * amp
                    } else {
                        0.0
                    };
                    lo = lo.min(v - w);
                    hi = hi.max(v + w);
                }
            }
        }
        let pins = if terrain { self.pins(c) } else { 0 };
        if terrain && lo > 0.0 {
            return SOLID;
        }
        if !terrain && hi <= 0.0 {
            return EMPTY;
        }
        // voxel by voxel
        let rr = (r * r) as usize;
        let base = self.pool.len();
        self.pool.resize(base + rr, 0);
        let o = self.g.o;
        let (wx0, wy0, wz0) = ((o[0] + x) * r, (o[1] + y) * r, (o[2] + z) * r);
        let ns = ((p.noise_size * r as f32).round() as i32).max(1);
        let hs = ((p.strata_height * r as f32).round() as i32).max(1);
        for kz in 0..r {
            let (lz, tz) = self.ax[kz as usize];
            for ix in 0..r {
                let (lx, tx) = self.ax[ix as usize];
                let col = |a: &[[[f32; 3]; 3]; 3], b: usize| {
                    let row0 = lerp(a[b][lz][lx], a[b][lz][lx + 1], tx);
                    let row1 = lerp(a[b][lz + 1][lx], a[b][lz + 1][lx + 1], tx);
                    lerp(row0, row1, tz)
                };
                let v = [col(&d, 0), col(&d, 1), col(&d, 2)];
                let (wx, wz) = (wx0 + ix, wz0 + kz);
                let bit = 1u16 << ix;
                if wmax == 0.0 {
                    for jy in 0..r {
                        let (ly, ty) = self.ax[jy as usize];
                        if lerp(v[ly], v[ly + 1], ty) > 0.0 {
                            self.pool[base + (jy * r + kz) as usize] |= bit;
                        }
                    }
                    continue;
                }
                let wv = [col(&m, 0), col(&m, 1), col(&m, 2)];
                let n2 = value2(wx, wz, ns, 0x3a1) * p.noise;
                let sh = (value2(wx, wz, hs * 6, 0x7c3) * hs as f32 * 1.5).round() as i32;
                // strata: one value a band, the band shifted per column
                let (mut seen, mut sv) = (i32::MIN, 0.0);
                for jy in 0..r {
                    let (ly, ty) = self.ax[jy as usize];
                    let mut val = lerp(v[ly], v[ly + 1], ty);
                    let wt = lerp(wv[ly], wv[ly + 1], ty);
                    if wt > 0.0 {
                        let b = (wy0 + jy + sh).div_euclid(hs);
                        if b != seen {
                            seen = b;
                            sv = band(b) * p.strata;
                        }
                        val += wt * (n2 + sv);
                    }
                    if val > 0.0 {
                        self.pool[base + (jy * r + kz) as usize] |= bit;
                    }
                }
            }
        }
        // ground touching something drawn by another mesher keeps that
        // side whole: the other side hides its face against it
        if pins != 0 {
            let full = self.full;
            let rows = &mut self.pool[base..base + rr];
            for j in 0..r {
                for k in 0..r {
                    let row = &mut rows[(j * r + k) as usize];
                    if pins & 1 != 0 {
                        *row |= 1;
                    }
                    if pins & 2 != 0 {
                        *row |= 1 << (r - 1);
                    }
                    if (pins & 4 != 0 && j == 0)
                        || (pins & 8 != 0 && j == r - 1)
                        || (pins & 16 != 0 && k == 0)
                        || (pins & 32 != 0 && k == r - 1)
                    {
                        *row = full;
                    }
                }
            }
        }
        #[cfg(test)]
        prof(0, t0.elapsed().as_secs_f64() * 1e3, 1.0);
        let rows = &self.pool[base..base + rr];
        if rows.iter().all(|r| *r == 0) {
            self.pool.truncate(base);
            return EMPTY;
        }
        if rows.iter().all(|r| *r == self.full) {
            self.pool.truncate(base);
            return SOLID;
        }
        MIXED + (base / rr) as u32
    }

    /// Sides of a ground cell (bits: -x, +x, -y, +y, -z, +z) against
    /// blocks drawn by another mesher.
    fn pins(&self, c: usize) -> u8 {
        let g = self.g;
        let mut bits = 0;
        for (i, q) in [c - DX, c + DX, c - DY, c + DY, c - DZ, c + DZ]
            .into_iter()
            .enumerate()
        {
            let f = g.fl[q];
            if f & F_TERRAIN == 0 && f & (F_PIN | F_LEAVES) != 0 {
                bits |= 1 << i;
            }
        }
        bits
    }

    /// Row (j, k) of a cell: bits along x.
    #[inline]
    fn row(&self, s: u32, j: i32, k: i32) -> u16 {
        match s {
            EMPTY => 0,
            SOLID | OTHER => self.full,
            m => self.pool[(m - MIXED) as usize * (self.r * self.r) as usize + (j * self.r + k) as usize],
        }
    }

    /// Whether the voxel at fine (x, y, z), counted from the section's
    /// corner, is filled (by ground or anything else).
    pub(crate) fn solid(&mut self, x: i32, y: i32, z: i32) -> bool {
        let r = self.r;
        let (cx, cy, cz) = (x.div_euclid(r), y.div_euclid(r), z.div_euclid(r));
        if !(-1..=N).contains(&cx) || !(-1..=N).contains(&cy) || !(-1..=N).contains(&cz) {
            return false;
        }
        let s = self.cell(cx, cy, cz);
        self.row(s, y - cy * r, z - cz * r) >> (x - cx * r) & 1 != 0
    }

    /// The block a ground cell is painted as: the block above it when that
    /// is its cover (earth under grass is grass).
    #[inline]
    fn material(&self, c: usize) -> BlockId {
        let (b, up) = (self.g.ids[c], self.g.ids[c + DY]);
        if self.p.under[up as usize] == b && b != 0 {
            up
        } else {
            b
        }
    }

    /// Whether other kinds of ground lie beside a ground cell on its layer.
    fn is_ragged(&mut self, c: usize) -> bool {
        match self.ragged[c] {
            1 => return true,
            2 => return false,
            _ => {}
        }
        let g = self.g;
        let p = self.p;
        let m = self.material(c);
        let mut out = false;
        if p.edge * p.edge_of[m as usize] > 0.0 {
            for q in [
                c - DX - DZ,
                c - DZ,
                c + DX - DZ,
                c - DX,
                c + DX,
                c - DX + DZ,
                c + DZ,
                c + DX + DZ,
            ] {
                if g.fl[q] & F_TERRAIN != 0 {
                    let mq = self.material(q);
                    if mq != m && p.edge_of[mq as usize] > 0.0 {
                        out = true;
                        break;
                    }
                }
            }
        }
        self.ragged[c] = if out { 1 } else { 2 };
        out
    }

    /// The ground cell a voxel that grew into air takes its kind from: the
    /// nearest, the one below first. `f` is where the voxel is in its cell.
    fn source(&self, cell: [i32; 3], f: [f32; 3]) -> [i32; 3] {
        let g = self.g;
        let mut best = (f32::MAX, cell);
        for dy in -1..=1 {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let q = [cell[0] + dx, cell[1] + dy, cell[2] + dz];
                    if g.fl[at(q[0], q[1], q[2])] & F_TERRAIN == 0 {
                        continue;
                    }
                    let gap = |d: i32, f: f32| match d {
                        -1 => f,
                        1 => 1.0 - f,
                        _ => 0.0,
                    };
                    let (a, b, c) = (gap(dx, f[0]), gap(dy, f[1]), gap(dz, f[2]));
                    let s = a * a + b * b + c * c + dy as f32 * 0.3;
                    if s < best.0 {
                        best = (s, q);
                    }
                }
            }
        }
        best.1
    }

    /// The block whose colours a solid voxel shows. `covered`: the voxels
    /// `cover_depth` above it are all filled.
    fn paint(&mut self, x: i32, y: i32, z: i32, covered: bool) -> BlockId {
        let r = self.r;
        let (cx, cy, cz) = (x.div_euclid(r), y.div_euclid(r), z.div_euclid(r));
        let g = self.g;
        let p = self.p;
        let rf = r as f32;
        let f = [
            ((x - cx * r) as f32 + 0.5) / rf,
            ((y - cy * r) as f32 + 0.5) / rf,
            ((z - cz * r) as f32 + 0.5) / rf,
        ];
        let mut cell = [cx, cy, cz];
        if g.fl[at(cx, cy, cz)] & F_TERRAIN == 0 {
            cell = self.source(cell, f);
        }
        let c = at(cell[0], cell[1], cell[2]);
        let mut m = self.material(c);
        // a ragged edge where kinds of ground meet: take the kind from a
        // point nudged by noise, on the same layer
        if self.is_ragged(c) {
            let reach = p.edge * p.edge_of[m as usize];
            let (wx, wz) = (g.o[0] * r + x, g.o[2] * r + z);
            let es = ((p.edge_size * rf).round() as i32).max(1);
            let h = hash3(wx, g.o[1] * r + y, wz);
            let jit = |s: u32| ((h >> s) & 255) as f32 / 255.0 - 0.5;
            let ox = (value2(wx, wz, es, 0x1d7) + jit(0) * 0.5) * reach;
            let oz = (value2(wx, wz, es, 0x2e9) + jit(8) * 0.5) * reach;
            let (qx, qz) = (
                (cx as f32 + f[0] + ox).floor() as i32,
                (cz as f32 + f[2] + oz).floor() as i32,
            );
            if (qx, qz) != (cell[0], cell[2])
                && (qx - cell[0]).abs() <= 1
                && (qz - cell[2]).abs() <= 1
            {
                let q = at(qx, cell[1], qz);
                if g.fl[q] & F_TERRAIN != 0 {
                    let mq = self.material(q);
                    if mq != m && p.edge_of[mq as usize] > 0.0 {
                        m = mq;
                    }
                }
            }
        }
        // cover: the top voxels show the block, deeper ones what is under it
        let under = p.under[m as usize];
        if under != 0 && covered {
            m = under;
        }
        m
    }

    /// Block light, sky light (0..1) and occlusion at a grid corner (local).
    /// A corner buried in ground or built blocks takes the mean of the open
    /// corners beside it, so a voxel face part way up a cell is lit like
    /// the air in front of it. Only built blocks darken the ground's
    /// corners: ground beside ground is shaped by its voxels and lit by
    /// their facing.
    fn corner(&mut self, light: &Light, i: i32, j: i32, k: i32) -> [f32; 3] {
        let idx = li(i, j, k);
        let v = self.corners[idx];
        if !v[0].is_nan() {
            return v;
        }
        let v = match self.raw_corner(light, i, j, k) {
            Some(v) => v,
            None => {
                let (mut sum, mut n) = ([0f32; 3], 0.0);
                for (di, dj, dk) in [
                    (0, 1, 0),
                    (1, 0, 0),
                    (-1, 0, 0),
                    (0, 0, 1),
                    (0, 0, -1),
                    (0, -1, 0),
                ] {
                    if let Some(v) = self.raw_corner(light, i + di, j + dj, k + dk) {
                        for e in 0..3 {
                            sum[e] += v[e];
                        }
                        n += 1.0;
                    }
                }
                if n > 0.0 {
                    sum.map(|s| s / n)
                } else {
                    [0.0, 0.0, 1.0]
                }
            }
        };
        self.corners[idx] = v;
        v
    }

    /// A corner's light from the open cells round it; None when none are.
    fn raw_corner(&self, light: &Light, i: i32, j: i32, k: i32) -> Option<[f32; 3]> {
        let g = self.g;
        let base = at(i - 1, j - 1, k - 1);
        let (mut bl, mut sl, mut open, mut built) = (0u32, 0u32, 0u32, 0u32);
        for (n, o) in CELL8.iter().enumerate() {
            let f = g.fl[base + o];
            if f & (F_OPAQUE | F_TERRAIN) == F_OPAQUE {
                built += 1;
            }
            if f & F_DARK != 0 {
                continue;
            }
            let (x, y, z) = (
                g.o[0] + i - 1 + (n & 1) as i32,
                g.o[1] + j - 1 + ((n >> 2) & 1) as i32,
                g.o[2] + k - 1 + ((n >> 1) & 1) as i32,
            );
            bl += light.get(x, y, z) as u32;
            sl += light.sky(x, y, z) as u32;
            open += 1;
        }
        if open == 0 {
            return None;
        }
        let k15 = 1.0 / (open as f32 * 15.0);
        let ao = (1.0 - 0.1 * built as f32).max(0.6);
        Some([bl as f32 * k15, sl as f32 * k15, ao])
    }

    /// A light class for a cell: the same small number for cells whose
    /// eight corners all have one light (faces may merge across them),
    /// else one of its own.
    fn light_class(
        &mut self,
        light: &Light,
        x: i32,
        y: i32,
        z: i32,
        classes: &mut HashMap<[u32; 3], u32>,
    ) -> u32 {
        let first = self.corner(light, x, y, z);
        for n in 1..8 {
            let v = self.corner(
                light,
                x + (n & 1) as i32,
                y + ((n >> 2) & 1) as i32,
                z + ((n >> 1) & 1) as i32,
            );
            if v != first {
                return 0x8000 + ci(x, y, z) as u32;
            }
        }
        let n = classes.len() as u32 + 1;
        *classes.entry(first.map(f32::to_bits)).or_insert(n)
    }

    /// Light at a vertex, given in fine voxels from the section's corner:
    /// blended between the grid corners round it.
    fn light_at(&mut self, light: &Light, p: [i32; 3]) -> [f32; 3] {
        let r = self.r;
        let b = p.map(|v| v.div_euclid(r).clamp(-1, N));
        let f = [0, 1, 2].map(|a| (p[a] - b[a] * r) as f32 / r as f32);
        let mut c = [[0f32; 3]; 8];
        for (n, v) in c.iter_mut().enumerate() {
            *v = self.corner(
                light,
                b[0] + (n & 1) as i32,
                b[1] + ((n >> 2) & 1) as i32,
                b[2] + ((n >> 1) & 1) as i32,
            );
        }
        let mut out = [0f32; 3];
        for (e, o) in out.iter_mut().enumerate() {
            let x00 = lerp(c[0][e], c[1][e], f[0]);
            let x01 = lerp(c[2][e], c[3][e], f[0]);
            let x10 = lerp(c[4][e], c[5][e], f[0]);
            let x11 = lerp(c[6][e], c[7][e], f[0]);
            *o = lerp(lerp(x00, x01, f[2]), lerp(x10, x11, f[2]), f[1]);
        }
        out
    }

    /// Draw the section's own ground.
    pub(crate) fn run(&mut self, light: &Light, lod: Lod, geo: &mut SectionGeo) {
        let r = self.r;
        let ru = r as usize;
        let rr = ru * ru;
        let p = self.p;
        let depth = ((p.cover_depth * r as f32).round() as i32).max(1);
        let mut own = vec![0u16; rr];
        // per row: voxels whose `depth` voxels above are all filled
        let mut covered = vec![0u16; rr];
        let mut face = vec![0u16; rr];
        let mut keys = vec![0u8; rr];
        let mut rects: Vec<Rect> = Vec::new();
        let mut classes: HashMap<[u32; 3], u32> = HashMap::new();
        #[cfg(test)]
        let mut paint_ms = (0.0, 0.0);
        for y in 0..N {
            for z in 0..N {
                for x in 0..N {
                    let s = self.cell(x, y, z);
                    if s != SOLID && s < MIXED {
                        continue;
                    }
                    let sides = [
                        (x == N - 1).then_some(1),
                        (x == 0).then_some(0),
                        None,
                        None,
                        (z == N - 1).then_some(3),
                        (z == 0).then_some(2),
                    ];
                    let nbs = DIRS.map(|(n, _)| self.cell(x + n[0], y + n[1], z + n[2]));
                    let skirts = sides.map(|side| side.map_or(0, |k| lod.skirt[k]));
                    if s == SOLID
                        && nbs.iter().all(|n| *n == SOLID || *n == OTHER)
                        && skirts.iter().all(|k| *k == 0)
                    {
                        continue;
                    }
                    for j in 0..r {
                        for k in 0..r {
                            own[(j * r + k) as usize] = self.row(s, j, k);
                        }
                    }
                    // what lies above each voxel, for the cover
                    let above = nbs[2];
                    let above2 = if depth > r {
                        self.cell(x, y + 2, z)
                    } else {
                        SOLID
                    };
                    for j in 0..r {
                        for k in 0..r {
                            let mut m = own[(j * r + k) as usize];
                            for h in 1..=depth {
                                let jj = j + h;
                                m &= if jj < r {
                                    own[(jj * r + k) as usize]
                                } else if jj < 2 * r {
                                    self.row(above, jj - r, k)
                                } else {
                                    self.row(above2, (jj - 2 * r).min(r - 1), k)
                                };
                                if m == 0 {
                                    break;
                                }
                            }
                            covered[(j * r + k) as usize] = m;
                        }
                    }
                    let class = self.light_class(light, x, y, z, &mut classes);
                    let (bx, by, bz) = (x * r, y * r, z * r);
                    for (d, (n, axis)) in DIRS.iter().enumerate() {
                        let ns = nbs[d];
                        if (ns == SOLID || ns == OTHER) && s == SOLID && skirts[d] == 0 {
                            continue;
                        }
                        // the faces: solid here, open in front
                        for j in 0..r {
                            for k in 0..r {
                                let a = own[(j * r + k) as usize];
                                let front = match d {
                                    0 => (a >> 1) | ((self.row(ns, j, k) & 1) << (r - 1)),
                                    1 => ((a << 1) & self.full) | (self.row(ns, j, k) >> (r - 1)),
                                    2 if j + 1 < r => own[((j + 1) * r + k) as usize],
                                    2 => self.row(ns, 0, k),
                                    3 if j > 0 => own[((j - 1) * r + k) as usize],
                                    3 => self.row(ns, r - 1, k),
                                    4 if k + 1 < r => own[(j * r + k + 1) as usize],
                                    4 => self.row(ns, j, 0),
                                    _ if k > 0 => own[(j * r + k - 1) as usize],
                                    _ => self.row(ns, j, r - 1),
                                };
                                face[(j * r + k) as usize] = a & !front;
                            }
                        }
                        // a skirt on a section side next to a different
                        // level: the outermost voxels near the surface
                        // show their side even where the neighbour is solid
                        if skirts[d] != 0 {
                            let coarse = skirts[d].min(lod.per_metre) as i32;
                            let deep = (2 * r + coarse - 1) / coarse;
                            let edge = if d == 0 || d == 4 { r - 1 } else { 0 };
                            for j in 0..r {
                                for k in 0..r {
                                    let a = own[(j * r + k) as usize];
                                    for i in 0..r {
                                        let on_edge = if d < 2 { i == edge } else { k == edge };
                                        if !on_edge || a >> i & 1 == 0 {
                                            continue;
                                        }
                                        let (vx, vy, vz) = (bx + i, by + j, bz + k);
                                        if (1..=deep).any(|h| !self.solid(vx, vy + h, vz)) {
                                            face[(j * r + k) as usize] |= 1 << i;
                                        }
                                    }
                                }
                            }
                        }
                        if face.iter().all(|f| *f == 0) {
                            continue;
                        }
                        #[cfg(test)]
                        let t0 = std::time::Instant::now();
                        // slices across the normal, merged within the cell
                        for sl in 0..r {
                            let mut any = false;
                            for v in 0..r {
                                for u in 0..r {
                                    let (i, j, k) = match axis {
                                        1 => (u, sl, v),
                                        2 => (u, v, sl),
                                        _ => (sl, v, u),
                                    };
                                    let row = (j * r + k) as usize;
                                    let key = if face[row] >> i & 1 != 0 {
                                        any = true;
                                        let cov = covered[row] >> i & 1 != 0;
                                        self.paint(bx + i, by + j, bz + k, cov)
                                    } else {
                                        0
                                    };
                                    keys[(v * r + u) as usize] = key;
                                }
                            }
                            if !any {
                                continue;
                            }
                            let plane = sl + if n[*axis] > 0 { 1 } else { 0 };
                            let (cu, cv, cp) = match axis {
                                1 => (bx, bz, by),
                                2 => (bx, by, bz),
                                _ => (bz, by, bx),
                            };
                            greedy(&mut keys, ru, |u, v, du, dv, key| {
                                rects.push(Rect {
                                    dir: d as u8,
                                    plane: cp + plane,
                                    u0: cu + u as i32,
                                    v0: cv + v as i32,
                                    u1: cu + (u + du) as i32,
                                    v1: cv + (v + dv) as i32,
                                    key: class << 8 | key as u32,
                                });
                            });
                        }
                        #[cfg(test)]
                        {
                            paint_ms.0 += t0.elapsed().as_secs_f64() * 1e3;
                            paint_ms.1 += 1.0;
                        }
                    }
                }
            }
        }
        #[cfg(test)]
        prof(1, paint_ms.0, paint_ms.1);
        #[cfg(test)]
        let t0 = std::time::Instant::now();
        merge(&mut rects);
        geo.opaque.reserve(rects.len() * 6);
        for q in &rects {
            self.emit(light, q, geo);
        }
        #[cfg(test)]
        prof(2, t0.elapsed().as_secs_f64() * 1e3, rects.len() as f64);
        self.tufts(light, geo);
    }

    /// Two triangles for a merged face.
    fn emit(&mut self, light: &Light, q: &Rect, geo: &mut SectionGeo) {
        let (n, axis) = DIRS[q.dir as usize];
        let mut c = [
            corner_of(axis, q.plane, q.u0, q.v0),
            corner_of(axis, q.plane, q.u1, q.v0),
            corner_of(axis, q.plane, q.u1, q.v1),
            corner_of(axis, q.plane, q.u0, q.v1),
        ];
        // anticlockwise seen from outside
        let e1 = [0, 1, 2].map(|a| c[1][a] - c[0][a]);
        let e2 = [0, 1, 2].map(|a| c[3][a] - c[0][a]);
        let cr = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        if cr[0] * n[0] + cr[1] * n[1] + cr[2] * n[2] < 0 {
            c.swap(1, 3);
        }
        let b = (q.key & 0xff) as BlockId;
        let r = self.r;
        let o = self.g.o;
        let t = render::textures();
        let p = self.p;
        let layer = if p.under[b as usize] != 0 {
            t.face(b, 0)
        } else {
            match q.dir {
                2 => t.face(b, 0),
                3 => t.face(b, 2),
                _ => t.face(b, 1),
            }
        };
        let glow = w::emission(b) > 0;
        let tex = [
            r as f32,
            0.0,
            KIND_VOXEL + layer as f32,
            render::layer_anim(layer),
        ];
        let jit = p.jitter[b as usize];
        let ls = c.map(|c| self.light_at(light, c));
        let pos = c.map(|c| {
            [
                (o[0] * r + c[0]) as f32 / r as f32,
                (o[1] * r + c[1]) as f32 / r as f32,
                (o[2] * r + c[2]) as f32 / r as f32,
            ]
        });
        let nf = n.map(|v| v as f32);
        // split so occlusion interpolates evenly
        let order = if ls[0][2] + ls[2][2] > ls[1][2] + ls[3][2] {
            [0, 1, 2, 0, 2, 3]
        } else {
            [1, 2, 3, 1, 3, 0]
        };
        for i in order {
            geo.opaque.push(Vertex {
                pos: pos[i],
                normal: nf,
                color: [jit, 0.0, 0.0, if glow { 2.0 } else { ls[i][2] }],
                light: ls[i][0],
                sky: ls[i][1],
                tex,
            });
        }
    }

    /// Tufts of grass and flowers on open ground that grows them, drawn as
    /// fine voxels near the camera.
    fn tufts(&mut self, light: &Light, geo: &mut SectionGeo) {
        let p = self.p;
        if p.tufts.per_cent == 0 || p.tufts.kinds.is_empty() {
            return;
        }
        let g = self.g;
        let r = self.r;
        for y in 0..N {
            for z in 0..N {
                for x in 0..N {
                    let c = at(x, y, z);
                    if g.fl[c] & F_TERRAIN == 0 || g.ids[c + DY] != w::AIR {
                        continue;
                    }
                    if !p.tuft_on[self.material(c) as usize] {
                        continue;
                    }
                    let (wx, wy, wz) = (g.o[0] + x, g.o[1] + y, g.o[2] + z);
                    let h = hash3(wx, wy, wz);
                    if h % 100 >= p.tufts.per_cent {
                        continue;
                    }
                    let (fx, fz) = ((h >> 8) as i32 % r, (h >> 16) as i32 % r);
                    // the top of that fine column
                    let (vx, vz) = (x * r + fx, z * r + fz);
                    let Some(top) = (y * r..(y + 2) * r)
                        .rev()
                        .find(|vy| self.solid(vx, *vy, vz) && !self.solid(vx, vy + 1, vz))
                    else {
                        continue;
                    };
                    let rf = r as f32;
                    geo.tufts.push(Tuft {
                        pos: [
                            (g.o[0] * r + vx) as f32 / rf + 0.5 / rf,
                            (g.o[1] * r + top + 1) as f32 / rf,
                            (g.o[2] * r + vz) as f32 / rf + 0.5 / rf,
                        ],
                        light: light.get(wx, wy + 1, wz) as f32 / 15.0,
                        sky: light.sky(wx, wy + 1, wz) as f32 / 15.0,
                        seed: h,
                    });
                }
            }
        }
    }
}

/// Join faces that line up across cells: same direction, plane, kind and
/// light class, side by side with the same extent, first along u, then v.
fn merge(rects: &mut Vec<Rect>) {
    rects.sort_unstable_by_key(|q| (q.dir, q.plane, q.key, q.v0, q.v1, q.u0));
    let mut a: Vec<Rect> = Vec::with_capacity(rects.len());
    for q in rects.drain(..) {
        if let Some(l) = a.last_mut()
            && (l.dir, l.plane, l.key, l.v0, l.v1) == (q.dir, q.plane, q.key, q.v0, q.v1)
            && l.u1 == q.u0
        {
            l.u1 = q.u1;
            continue;
        }
        a.push(q);
    }
    a.sort_unstable_by_key(|q| (q.dir, q.plane, q.key, q.u0, q.u1, q.v0));
    for q in a {
        if let Some(l) = rects.last_mut()
            && (l.dir, l.plane, l.key, l.u0, l.u1) == (q.dir, q.plane, q.key, q.u0, q.u1)
            && l.v1 == q.v0
        {
            l.v1 = q.v1;
            continue;
        }
        rects.push(q);
    }
}

/// Trilinear read of a 3 x 3 x 3 field: (pair, how far) along y, z and x.
#[inline]
fn tri(d: &[[[f32; 3]; 3]; 3], ly: usize, ty: f32, lz: usize, tz: f32, lx: usize, tx: f32) -> f32 {
    let at = |b: usize| {
        let r0 = lerp(d[b][lz][lx], d[b][lz][lx + 1], tx);
        let r1 = lerp(d[b][lz + 1][lx], d[b][lz + 1][lx + 1], tx);
        lerp(r0, r1, tz)
    };
    lerp(at(ly), at(ly + 1), ty)
}

/// Merge equal keys of a square grid (`n` a side, row-major by v) into
/// rectangles: (u, v, width, height, key). Clears the grid.
fn greedy(keys: &mut [u8], n: usize, mut emit: impl FnMut(usize, usize, usize, usize, BlockId)) {
    for v in 0..n {
        let mut u = 0;
        while u < n {
            let k = keys[v * n + u];
            if k == 0 {
                u += 1;
                continue;
            }
            let mut du = 1;
            while u + du < n && keys[v * n + u + du] == k {
                du += 1;
            }
            let mut dv = 1;
            'grow: while v + dv < n {
                for q in u..u + du {
                    if keys[(v + dv) * n + q] != k {
                        break 'grow;
                    }
                }
                dv += 1;
            }
            for vv in v..v + dv {
                for q in u..u + du {
                    keys[vv * n + q] = 0;
                }
            }
            emit(u, v, du, dv, k);
            u += du;
        }
    }
}

/// Draw a section's ground.
pub(crate) fn mesh(g: &Grid, light: &Light, lod: Lod, geo: &mut SectionGeo) {
    let p = params();
    let mut f = Fine::new(g, p, lod.per_metre);
    f.run(light, lod, geo);
}

/// The top of the drawn ground in each fine column of block `b`'s cell,
/// from a grid whose middle cell is `b` (see `terrain_mesh::Ground`):
/// `per_metre`² heights, row-major by z; NaN where the column has no top
/// near the block's.
pub(crate) fn tops(g: &Grid) -> (u8, Vec<f32>) {
    let p = params();
    let r = p.per_metre as i32;
    let mut f = Fine::new(g, p, p.per_metre);
    let (cx, cy, cz) = (N / 2, N / 2, N / 2);
    let mut out = vec![f32::NAN; (r * r) as usize];
    for k in 0..r {
        for i in 0..r {
            let (vx, vz) = (cx * r + i, cz * r + k);
            // from the top of the cell above down to the bottom of this one
            if let Some(top) = ((cy - 1) * r..(cy + 2) * r)
                .rev()
                .find(|vy| f.solid(vx, *vy, vz) && !f.solid(vx, vy + 1, vz))
            {
                out[(k * r + i) as usize] = (g.o[1] * r + top + 1) as f32 / r as f32;
            }
        }
    }
    (r as u8, out)
}
