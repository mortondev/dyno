//! Procedural 16×16 pixel-art textures for every block face and item sprite.
//!
//! Nothing here is loaded from disk: each texture is painted by code, so the
//! game ships no image files. The painters share one small toolkit:
//!
//! - [`Ramp`]: six shades built from a block's base colour. Shadows lean
//!   towards a cool indigo and highlights towards a warm cream, which gives
//!   every material the same ambient light.
//! - [`Map`]: a 16×16 grid of ramp steps. Painters draw shapes as steps and
//!   convert to colour at the end, so a texture never has more colours than
//!   its ramps.
//! - Tileable noise and tileable Voronoi cells. All coordinates wrap at 16,
//!   so a wall of one block shows no seams.
//! - [`art`]: small pictures written as text, for the parts that need a
//!   drawn shape (a latch, a tool head, a berry).
//!
//! Light comes from the top-left in every texture.

use dyno_world as w;
use dyno_world::BLOCKS;
use dyno_world::content::{Icon, ItemView, ToolKind};
use std::f32::consts::TAU;

pub const TILE: usize = 16;
pub type Rgba = [u8; TILE * TILE * 4];

/// Tile size as a signed number, for wrap-around arithmetic.
const S: i32 = TILE as i32;

// Block IDs that `dyno_world` has no constants for.
const BASALT: u8 = 15;
const MARBLE: u8 = 16;
const TERRACOTTA: u8 = 18;
const MOSSY_COBBLE: u8 = 19;
const DARK_PLANKS: u8 = 20;
const BIRCH_PLANKS: u8 = 21;
const ICE: u8 = 22;
/// The ten wool colours, White to Pink.
const WOOL_FIRST: u8 = 23;
const WOOL_LAST: u8 = 32;

type Px = [u8; 4];
type Rgb = [f32; 3];
const CLEAR: Px = [0, 0, 0, 0];

pub struct Textures {
    /// every texture, as an index into this list
    pub layers: Vec<Rgba>,
    /// per block ID: layer for [top, side, bottom]
    pub block_faces: Vec<[u16; 3]>,
    /// layers that animate: (first layer, frame count, seconds per frame); water and emberstone at least
    pub animated: Vec<(u16, u16, f32)>,
}

impl Textures {
    /// Build every block texture. Deterministic: same output every run.
    pub fn blocks() -> Textures {
        build_blocks()
    }

    /// Layer for a block face: face 0 = top, 1 = side, 2 = bottom.
    ///
    /// An unknown block ID gets the last layer, which is a neutral tile.
    pub fn face(&self, block: u8, face: usize) -> u16 {
        match self.block_faces.get(block as usize) {
            Some(faces) => faces[face.min(2)],
            None => (self.layers.len() - 1) as u16,
        }
    }
}

// ---------------------------------------------------------------- toolkit

/// One 16×16 picture. Coordinates wrap, so shapes drawn over an edge come
/// back in on the other side.
#[derive(Clone)]
struct Img([Px; TILE * TILE]);

fn at(x: i32, y: i32) -> usize {
    (y.rem_euclid(S) * S + x.rem_euclid(S)) as usize
}

fn inside(x: i32, y: i32) -> bool {
    (0..S).contains(&x) && (0..S).contains(&y)
}

impl Img {
    fn new(c: Px) -> Img {
        Img([c; TILE * TILE])
    }

    fn from_fn(f: impl Fn(i32, i32) -> Px) -> Img {
        let mut img = Img::new(CLEAR);
        for y in 0..S {
            for x in 0..S {
                img.set(x, y, f(x, y));
            }
        }
        img
    }

    fn get(&self, x: i32, y: i32) -> Px {
        self.0[at(x, y)]
    }

    fn set(&mut self, x: i32, y: i32, c: Px) {
        self.0[at(x, y)] = c;
    }

    fn rgba(&self) -> Rgba {
        let mut out = [0u8; TILE * TILE * 4];
        for (o, p) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.0.iter()) {
            *o = *p;
        }
        out
    }
}

/// A 16×16 grid of ramp steps (0 = darkest, 5 = lightest). Wraps like [`Img`].
#[derive(Clone)]
struct Map([i8; TILE * TILE]);

impl Map {
    fn new(v: i32) -> Map {
        Map([v as i8; TILE * TILE])
    }

    fn from_fn(f: impl Fn(i32, i32) -> i32) -> Map {
        let mut m = Map::new(0);
        for y in 0..S {
            for x in 0..S {
                m.set(x, y, f(x, y));
            }
        }
        m
    }

    fn get(&self, x: i32, y: i32) -> i32 {
        self.0[at(x, y)] as i32
    }

    fn set(&mut self, x: i32, y: i32, v: i32) {
        self.0[at(x, y)] = v.clamp(-8, 8) as i8;
    }

    /// Replace lone pixels by a neighbour, so shades form clusters and not
    /// single-pixel noise.
    fn despeckle(&mut self) {
        let src = self.clone();
        for y in 0..S {
            for x in 0..S {
                let v = src.get(x, y);
                let around = [
                    src.get(x - 1, y),
                    src.get(x + 1, y),
                    src.get(x, y - 1),
                    src.get(x, y + 1),
                ];
                if around.iter().all(|n| *n != v) {
                    self.set(x, y, around[0]);
                }
            }
        }
    }

    /// Colour every step with one ramp.
    fn img(&self, r: &Ramp) -> Img {
        Img::from_fn(|x, y| r.px(self.get(x, y)))
    }
}

fn luma(c: Rgb) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(c: Rgb, k: f32) -> Rgb {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// Move a colour to a target brightness: darken by scaling, lighten by
/// scaling as far as the brightest channel allows and then towards cream.
fn with_luma(c: Rgb, target: f32) -> Rgb {
    let l = luma(c).max(1e-4);
    if target <= l {
        return scale(c, target / l);
    }
    let top = c[0].max(c[1]).max(c[2]).max(1e-4);
    let k = (target / l).min(1.0 / top);
    let c = scale(c, k);
    let l = luma(c);
    let cream = [1.0, 0.98, 0.92];
    let t = ((target - l) / (luma(cream) - l).max(1e-4)).clamp(0.0, 1.0);
    mix(c, cream, t)
}

/// Push a colour away from grey (`k` > 1) or towards it (`k` < 1).
fn saturate(c: Rgb, k: f32) -> Rgb {
    let g = luma(c);
    [
        (g + (c[0] - g) * k).clamp(0.0, 1.0),
        (g + (c[1] - g) * k).clamp(0.0, 1.0),
        (g + (c[2] - g) * k).clamp(0.0, 1.0),
    ]
}

fn to_u8(c: Rgb) -> [u8; 3] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
    ]
}

fn to_rgb(c: Px) -> Rgb {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
    ]
}

/// The shadow tint and the highlight tint shared by every material.
const COOL: Rgb = [0.30, 0.24, 0.62];
const WARM: Rgb = [1.0, 0.90, 0.62];

/// Six shades of one material. Step 3 is the base colour, steps 0 to 2 are
/// shadows and steps 4 and 5 are highlights.
#[derive(Clone, Copy)]
struct Ramp([[u8; 3]; 6]);

impl Ramp {
    fn new(base: Rgb) -> Ramp {
        Ramp::with(base, 1.0)
    }

    /// `contrast` scales the brightness range: below 1 for soft materials.
    fn with(base: Rgb, contrast: f32) -> Ramp {
        let l = luma(base);
        let mut down = 0.28 * contrast;
        let mut up = 0.18 * contrast;
        // Near-black and near-white bases have no room on one side, so the
        // range moves to the other side instead of clipping flat.
        let room_down = (l - 0.03).max(0.0);
        let room_up = (0.975 - l).max(0.0);
        if down > room_down {
            up += (down - room_down) * 0.55;
            down = room_down;
        }
        if up > room_up {
            down = (down + (up - room_up) * 0.8).min(room_down);
            up = room_up;
        }
        let offsets = [-down, -down * 0.58, -down * 0.25, 0.0, up * 0.48, up];
        let mut out = [[0u8; 3]; 6];
        for (i, slot) in out.iter_mut().enumerate() {
            let target = l + offsets[i];
            let steps = (i as f32 - 3.0).abs();
            let mut c = with_luma(base, target);
            if i < 3 {
                c = saturate(c, 1.0 + 0.10 * steps);
                c = mix(c, with_luma(COOL, target), 0.075 * steps);
            } else if i > 3 {
                c = saturate(c, 1.0 - 0.05 * steps);
                c = mix(c, with_luma(WARM, target), 0.11 * steps);
            }
            *slot = to_u8(c);
        }
        Ramp(out)
    }

    /// A ramp for things that give off light: the highlights go to a pale
    /// glow of the same hue instead of to cream.
    fn glow(base: Rgb) -> Ramp {
        let mut r = Ramp::with(base, 1.2);
        let pale = mix(base, [1.0, 1.0, 0.92], 0.5);
        r.0[4] = to_u8(saturate(pale, 1.15));
        r.0[5] = to_u8(mix(base, [1.0, 1.0, 0.95], 0.85));
        r
    }

    /// Colours of fire, from soot to the white-hot core. Fire gets lighter
    /// towards yellow, so this ramp is written out.
    fn fire() -> Ramp {
        Ramp([
            [40, 24, 30],
            [124, 38, 28],
            [208, 80, 30],
            [255, 140, 52],
            [255, 200, 92],
            [255, 240, 180],
        ])
    }

    fn rgb(&self, step: i32) -> [u8; 3] {
        self.0[step.clamp(0, 5) as usize]
    }

    fn px(&self, step: i32) -> Px {
        self.pxa(step, 255)
    }

    fn pxa(&self, step: i32, alpha: u8) -> Px {
        let c = self.rgb(step);
        [c[0], c[1], c[2], alpha]
    }
}

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x85EB_CA6B)
        ^ (y as u32).wrapping_mul(0xC2B2_AE35)
        ^ seed.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    h
}

/// A fixed pseudo-random number in 0..1 for a cell.
fn rnd(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / 16_777_216.0
}

/// Smooth noise that tiles over the 16-pixel tile, with `cx` × `cy` lattice
/// cells across it. Fewer cells give larger blotches.
fn tnoise(x: f32, y: f32, cx: i32, cy: i32, seed: u32) -> f32 {
    let fx = x * cx as f32 / 16.0;
    let fy = y * cy as f32 / 16.0;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let v = |dx: i32, dy: i32| rnd((ix + dx).rem_euclid(cx), (iy + dy).rem_euclid(cy), seed);
    let top = v(0, 0) + (v(1, 0) - v(0, 0)) * sx;
    let bottom = v(0, 1) + (v(1, 1) - v(0, 1)) * sx;
    top + (bottom - top) * sy
}

/// [`tnoise`] at a pixel centre.
fn pnoise(x: i32, y: i32, cx: i32, cy: i32, seed: u32) -> f32 {
    tnoise(x as f32 + 0.5, y as f32 + 0.5, cx, cy, seed)
}

/// Signed shortest offset from `b` to `a` on the wrapping tile.
fn wrap_delta(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(16.0);
    if d > 8.0 { d - 16.0 } else { d }
}

/// A jittered `gx` × `gy` grid of points that tiles. `jitter` 0 is a plain
/// grid and 1 lets a point move anywhere in its grid cell.
fn sites(gx: i32, gy: i32, jitter: f32, seed: u32) -> Vec<(f32, f32)> {
    let (cw, ch) = (16.0 / gx as f32, 16.0 / gy as f32);
    let mut out = Vec::with_capacity((gx * gy) as usize);
    for j in 0..gy {
        for i in 0..gx {
            let ox = (rnd(i, j, seed) - 0.5) * jitter;
            let oy = (rnd(i, j, seed ^ 0xABCD) - 0.5) * jitter;
            out.push(((i as f32 + 0.5 + ox) * cw, (j as f32 + 0.5 + oy) * ch));
        }
    }
    out
}

/// The site nearest to a point, on the wrapping tile.
struct Hit {
    /// index of the nearest site
    i: usize,
    /// distance to it
    d1: f32,
    /// distance to the runner-up; `d2 - d1` is small near a cell border
    d2: f32,
    /// offset from the site to the point
    dx: f32,
    dy: f32,
}

/// Find the nearest site. `boxy` blends the round distance towards a square
/// one for angular cells. `stretch` above 1 makes cells wider than tall.
fn nearest(px: f32, py: f32, sites: &[(f32, f32)], boxy: f32, stretch: f32) -> Hit {
    let mut hit = Hit {
        i: 0,
        d1: f32::MAX,
        d2: f32::MAX,
        dx: 0.0,
        dy: 0.0,
    };
    for (i, s) in sites.iter().enumerate() {
        let (dx, dy) = (wrap_delta(px, s.0), wrap_delta(py, s.1));
        let (ax, ay) = (dx.abs() / stretch, dy.abs());
        let round = (ax * ax + ay * ay).sqrt();
        let d = round + (ax.max(ay) - round) * boxy;
        if d < hit.d1 {
            hit = Hit {
                i,
                d1: d,
                d2: hit.d1,
                dx,
                dy,
            };
        } else if d < hit.d2 {
            hit.d2 = d;
        }
    }
    hit
}

/// [`nearest`] at a pixel centre, with round cells.
fn cell(x: i32, y: i32, sites: &[(f32, f32)]) -> Hit {
    nearest(x as f32 + 0.5, y as f32 + 0.5, sites, 0.0, 1.0)
}

/// Flat patches with organic edges: each Voronoi cell takes one step from
/// `shades`. Neighbouring cells with the same step merge into larger shapes.
fn patch_map(sites: &[(f32, f32)], stretch: f32, seed: u32, shades: &[i32]) -> Map {
    let mut m = Map::from_fn(|x, y| {
        let wx = (pnoise(x, y, 4, 4, seed) - 0.5) * 2.2;
        let wy = (pnoise(x, y, 4, 4, seed ^ 0x3C) - 0.5) * 2.2;
        let hit = nearest(
            x as f32 + 0.5 + wx,
            y as f32 + 0.5 + wy,
            sites,
            0.0,
            stretch,
        );
        shades[hash(hit.i as i32, 7, seed) as usize % shades.len()]
    });
    m.despeckle();
    m
}

/// Paint a picture from rows of text. Digits pick steps of `ramps[0]`,
/// `a`-`f` steps 0-5 of `ramps[1]`, `A`-`F` of `ramps[2]` and `p`-`u` of
/// `ramps[3]`. Any other character leaves the pixel alone.
fn art(img: &mut Img, x0: i32, y0: i32, rows: &[&str], ramps: &[&Ramp]) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.bytes().enumerate() {
            let (set, step) = match ch {
                b'0'..=b'5' => (0, ch - b'0'),
                b'a'..=b'f' => (1, ch - b'a'),
                b'A'..=b'F' => (2, ch - b'A'),
                b'p'..=b'u' => (3, ch - b'p'),
                _ => continue,
            };
            if let Some(r) = ramps.get(set) {
                img.set(x0 + dx as i32, y0 + dy as i32, r.px(step as i32));
            }
        }
    }
}

/// Like [`art`], but for a [`Map`]: digits set steps, shifted by `shift`.
fn stamp(m: &mut Map, x0: i32, y0: i32, rows: &[&str], shift: i32) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.bytes().enumerate() {
            if ch.is_ascii_digit() {
                m.set(x0 + dx as i32, y0 + dy as i32, (ch - b'0') as i32 + shift);
            }
        }
    }
}

/// Set several pixels of a map to one step.
fn dots(m: &mut Map, points: &[(i32, i32)], step: i32) {
    for &(x, y) in points {
        m.set(x, y, step);
    }
}

// ---------------------------------------------------------------- earth and stone

/// A dark crack along `points`, with the lit lower lip under it.
fn crack(m: &mut Map, points: &[(i32, i32)]) {
    for &(x, y) in points {
        m.set(x, y + 1, 4);
    }
    for (i, &(x, y)) in points.iter().enumerate() {
        let end = i == 0 || i + 1 == points.len();
        m.set(x, y, if end { 2 } else { 1 });
    }
}

/// Stone: flat plates, wider than tall like layered rock, and a few
/// hairline cracks.
fn stone_map() -> Map {
    let flat = patch_map(&sites(4, 6, 0.9, 23), 1.8, 24, &[3, 3, 2, 3, 4, 3, 4, 3]);
    // A lighter plate stands a little proud: it throws a thin shadow on
    // the plate below it.
    let mut m = Map::from_fn(|x, y| {
        let v = flat.get(x, y);
        let under = flat.get(x, y - 1) > v && flat.get(x - 1, y - 1) > v;
        (v - under as i32).max(2)
    });
    crack(&mut m, &[(1, 4), (2, 5), (3, 5), (4, 6)]);
    crack(&mut m, &[(9, 1), (10, 2), (11, 2), (12, 3), (13, 3)]);
    crack(&mut m, &[(7, 11), (8, 11), (9, 12), (10, 13)]);
    m
}

fn dirt_img(base: Rgb) -> Img {
    let r = Ramp::new(base);
    let mut m = patch_map(&sites(4, 4, 1.0, 21), 1.3, 22, &[3, 3, 2, 3, 4, 2, 3]);
    // crumbs: dark pixels in twos and threes
    dots(
        &mut m,
        &[(1, 2), (2, 2), (9, 5), (10, 5), (10, 6), (5, 10), (5, 11)],
        1,
    );
    dots(&mut m, &[(13, 12), (14, 12), (14, 1), (7, 14), (8, 14)], 1);
    let mut img = m.img(&r);
    // Pebbles: a pale stone with a shadow under it.
    let pebble = Ramp::new(mix(base, [0.70, 0.68, 0.66], 0.7));
    art(&mut img, 5, 2, &["a54", ".b1"], &[&pebble, &r]);
    art(&mut img, 12, 8, &["54", "b1"], &[&pebble, &r]);
    art(&mut img, 2, 6, &["4", "b"], &[&pebble, &r]);
    img
}

fn grass_top_map() -> Map {
    let mut m = patch_map(&sites(5, 4, 1.0, 31), 1.0, 32, &[3, 3, 2, 3, 4, 3, 2, 4]);
    // Blades: short strokes that lean both ways, each with a pale tip and a
    // shadow at its foot. (x, y, lean, tall)
    let blades: [(i32, i32, i32, bool); 10] = [
        (1, 1, 0, true),
        (6, 0, 1, false),
        (11, 2, -1, true),
        (14, 6, 0, false),
        (3, 6, 1, true),
        (8, 7, 0, false),
        (5, 11, -1, false),
        (12, 10, 1, true),
        (0, 10, 0, false),
        (9, 13, 0, true),
    ];
    for &(x, y, lean, tall) in &blades {
        m.set(x + lean, y, 5);
        m.set(x, y + 1, 4);
        if tall {
            m.set(x, y + 2, 4);
        }
        let foot = y + 2 + tall as i32;
        m.set(x, foot, 2);
        m.set(x + 1, foot - 1, 2);
    }
    m
}

/// How far the grass fringe hangs down each column of a grass side.
const FRINGE: [i32; 16] = [3, 3, 4, 3, 3, 5, 4, 4, 3, 4, 4, 3, 5, 4, 3, 4];

fn grass_side_img(grass: &Ramp, dirt: &Img, dirt_ramp: &Ramp) -> Img {
    let mut img = dirt.clone();
    for x in 0..S {
        let depth = FRINGE[x as usize];
        for y in 0..depth {
            let step = if y == 0 {
                // sunlit top, with a pale tip here and there
                if matches!(x, 2 | 7 | 8 | 13) { 5 } else { 4 }
            } else if y + 1 == depth {
                2
            } else if y == 1 && rnd(x / 2, 0, 41) > 0.5 {
                4
            } else {
                3
            };
            img.set(x, y, grass.px(step));
        }
        // the shadow that the fringe throws on the dirt
        img.set(x, depth, dirt_ramp.px(1));
    }
    img
}

fn sand_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.75);
    let mut m = patch_map(&sites(3, 3, 1.0, 91), 2.0, 92, &[3, 3, 3, 4]);
    // Wind ripples: three wavy lines, each a pale crest over a soft shadow.
    for (k, y0) in [2, 7, 12].into_iter().enumerate() {
        for x in 0..S {
            if pnoise(x, k as i32 * 5, 4, 3, 93) < 0.30 {
                continue;
            }
            let wave = (x as f32 / 16.0 * TAU + k as f32 * 2.1).sin() * 1.4;
            let y = y0 + wave.round() as i32;
            m.set(x, y, 4);
            if pnoise(x, k as i32 * 5, 5, 3, 94) > 0.45 {
                m.set(x, y + 1, 2);
            }
        }
    }
    // grain
    dots(
        &mut m,
        &[(3, 5), (11, 4), (7, 10), (14, 9), (1, 15), (9, 0)],
        2,
    );
    dots(&mut m, &[(6, 5), (13, 15), (2, 10), (10, 5)], 5);
    m.img(&r)
}

fn clay_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.8);
    const BANDS: [i32; 16] = [3, 4, 3, 3, 2, 3, 3, 4, 4, 3, 2, 2, 3, 3, 4, 3];
    let mut m = Map::from_fn(|x, y| {
        let wob = (x as f32 / 16.0 * TAU).sin() * 1.2 + (x as f32 / 8.0 * TAU + 1.0).sin() * 0.5;
        let band = BANDS[(y + wob.round() as i32).rem_euclid(S) as usize];
        // bands fade in and out along their length
        if band != 3 && pnoise(x, y, 3, 8, 101) < 0.33 {
            3
        } else {
            band
        }
    });
    m.despeckle();
    // pits
    for &(x, y) in &[(3, 2), (12, 6), (6, 12)] {
        m.set(x, y, 1);
        m.set(x + 1, y, 2);
        m.set(x, y + 1, 4);
    }
    m.img(&r)
}

fn snow_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.7);
    // Snow is too bright for highlights, so its shape is all in the
    // shadows: two wind-blown drifts, each with a soft shadow that swells
    // and thins along its length.
    let mut m = Map::from_fn(|x, y| {
        let t = x as f32 / 16.0 * TAU;
        let mut step = 3;
        for (y0, phase) in [(4.0, 0.6), (12.0, 3.4)] {
            let mid = y0 + 1.6 * (t + phase).sin();
            let half = 0.75 + 0.75 * (t * 2.0 + phase * 2.0).sin();
            let d = wrap_delta(y as f32 + 0.5, mid);
            if d.abs() < half {
                // the lower edge of a shadow is a touch deeper
                step = if d > half - 1.0 && half > 1.2 { 1 } else { 2 };
            }
        }
        step
    });
    m.despeckle();
    dots(&mut m, &[(3, 0), (12, 8), (6, 9), (14, 15), (9, 1)], 5);
    m.img(&r)
}

const COBBLE_SITES: [(f32, f32); 8] = [
    (2.5, 2.0),
    (8.5, 1.0),
    (13.5, 4.0),
    (5.5, 6.5),
    (10.5, 9.0),
    (0.5, 10.5),
    (6.0, 12.5),
    (13.0, 14.0),
];

/// Rounded stones with mortar gaps. Returns the steps, with -1 for mortar.
fn cobble_map(sites: &[(f32, f32)], gap: f32, seed: u32) -> Map {
    let stones = Map::from_fn(|x, y| {
        let hit = cell(x, y, sites);
        if hit.d2 - hit.d1 < gap {
            -1
        } else {
            [3, 2, 3, 4][(hash(hit.i as i32, 0, seed) % 4) as usize]
        }
    });
    Map::from_fn(|x, y| {
        let v = stones.get(x, y);
        if v < 0 {
            return -1;
        }
        let lit = stones.get(x - 1, y) < 0 || stones.get(x, y - 1) < 0;
        let shaded = stones.get(x + 1, y) < 0 || stones.get(x, y + 1) < 0;
        v + lit as i32 - shaded as i32
    })
}

/// Colour a cobble map: mortar is darkest right under a stone.
fn cobble_img(m: &Map, r: &Ramp) -> Img {
    Img::from_fn(|x, y| {
        let v = m.get(x, y);
        if v >= 0 {
            r.px(v)
        } else if m.get(x, y - 1) >= 0 && m.get(x - 1, y) >= 0 {
            r.px(0)
        } else {
            r.px(1)
        }
    })
}

fn mossy_img(stone: Rgb, moss: Rgb) -> Img {
    let m = cobble_map(&COBBLE_SITES, 1.05, 51);
    let mut img = cobble_img(&m, &Ramp::with(stone, 1.1));
    let moss = Ramp::new(moss);
    // Moss fills the gaps in some places and creeps onto the stones there.
    let cover = Map::from_fn(|x, y| {
        let n = 0.7 * pnoise(x, y, 3, 3, 191) + 0.3 * pnoise(x, y, 6, 6, 192);
        let in_gap = m.get(x, y) < 0;
        let near_gap = m.get(x + 1, y) < 0 || m.get(x, y + 1) < 0 || m.get(x - 1, y) < 0;
        let limit = if in_gap {
            0.42
        } else if near_gap {
            0.50
        } else {
            0.60
        };
        (n > limit) as i32
    });
    for y in 0..S {
        for x in 0..S {
            if cover.get(x, y) == 0 {
                continue;
            }
            let lit = cover.get(x - 1, y) == 0 || cover.get(x, y - 1) == 0;
            let shaded = cover.get(x + 1, y) == 0 || cover.get(x, y + 1) == 0;
            let deep = m.get(x, y) < 0;
            let step = 3 + lit as i32 - shaded as i32 - deep as i32 * 2;
            img.set(x, y, moss.px(step.max(1)));
        }
    }
    img
}

fn brick_img(base: Rgb, seed: u32) -> Img {
    let clay = [
        Ramp::new(base),
        Ramp::new(mix(base, [0.78, 0.42, 0.26], 0.4)),
        Ramp::new(mix(base, [0.50, 0.24, 0.26], 0.45)),
    ];
    let mortar = Ramp::with([0.76, 0.71, 0.63], 0.8);
    Img::from_fn(|x, y| {
        let (row, ry) = (y / 4, y % 4);
        let bx = (x + if row % 2 == 0 { 0 } else { 4 }).rem_euclid(S);
        let (brick, rx) = (bx / 8, bx % 8);
        if ry == 3 {
            // the course above shades the mortar bed
            return mortar.px(if rx == 7 { 3 } else { 2 });
        }
        if rx == 7 {
            return mortar.px(if ry == 0 { 4 } else { 3 });
        }
        let h = hash(brick, row, seed);
        let r = &clay[[0, 0, 1, 2, 0, 1][(h % 6) as usize]];
        let mut step = match ry {
            0 => 4,
            1 => 3,
            _ => 2,
        };
        if rx == 0 && ry < 2 {
            step = 4;
        }
        if rx == 6 && ry > 0 {
            step = 2;
        }
        // one worn spot per brick
        let spot = 1 + (h >> 8) as i32 % 5;
        if ry == 1 && (rx == spot || rx == spot + 1) {
            step = if (h >> 12).is_multiple_of(2) { 2 } else { 4 };
        }
        r.px(step)
    })
}

/// Stone bricks: two courses of 8×4 blocks, dark joints.
fn stone_brick_map(seed: u32) -> Map {
    Map::from_fn(|x, y| {
        let (row, ry) = (y / 4, y % 4);
        let bx = (x + if row % 2 == 0 { 0 } else { 4 }).rem_euclid(S);
        let (brick, rx) = (bx / 8, bx % 8);
        if ry == 3 || rx == 7 {
            return if ry == 3 && rx != 7 { 0 } else { 1 };
        }
        let h = hash(brick, row, seed);
        let tone = [3, 3, 2, 3][(h % 4) as usize];
        let mut step = tone;
        if ry == 0 || rx == 0 {
            step += 1;
        }
        if (ry == 2 || rx == 6) && !(ry == 0 || rx == 0) {
            step -= 1;
        }
        let spot = 2 + (h >> 8) as i32 % 3;
        if ry == 1 && (rx == spot || rx == spot + 1) {
            step = tone - 1;
        }
        step
    })
}

fn basalt_side_img(base: Rgb) -> Img {
    let r = Ramp::new(base);
    /// first column of each of the five pillars
    const EDGES: [i32; 5] = [0, 3, 7, 10, 13];
    let mut m = Map::from_fn(|x, y| {
        let k = EDGES.iter().rposition(|e| *e <= x).unwrap_or(0);
        let (start, end) = (EDGES[k], EDGES.get(k + 1).copied().unwrap_or(S));
        let tone = [3, 2, 3, 3, 2][k];
        // each pillar is broken by one or two joints
        let joint = (hash(k as i32, 0, 141) % 16) as i32;
        let dy = (y - joint).rem_euclid(if k % 2 == 0 { 16 } else { 8 });
        if x == end - 1 {
            0
        } else if dy == 0 {
            1
        } else if x == start {
            if dy == 1 { 5 } else { 4 }
        } else if dy == 1 {
            tone + 1
        } else {
            tone
        }
    });
    dots(&mut m, &[(1, 6), (5, 2), (8, 12), (11, 5), (14, 10)], 2);
    m.img(&r)
}

/// The top of basalt pillars: six-sided cells with dark gaps.
fn basalt_top_img(base: Rgb) -> Img {
    let r = Ramp::new(base);
    // a honeycomb: every other row of sites is shifted by half a cell
    let mut pillars = Vec::new();
    for j in 0..4 {
        for i in 0..3 {
            let shift = if j % 2 == 0 { 0.0 } else { 16.0 / 6.0 };
            let jx = (rnd(i, j, 143) - 0.5) * 1.2;
            pillars.push((
                i as f32 * 16.0 / 3.0 + shift + 2.0 + jx,
                j as f32 * 4.0 + 2.0,
            ));
        }
    }
    let cells = Map::from_fn(|x, y| {
        let hit = cell(x, y, &pillars);
        if hit.d2 - hit.d1 < 0.85 {
            -1
        } else {
            [3, 2, 3, 3][(hash(hit.i as i32, 0, 144) % 4) as usize]
        }
    });
    Img::from_fn(|x, y| {
        let v = cells.get(x, y);
        if v < 0 {
            return r.px(0);
        }
        let lit = cells.get(x - 1, y) < 0 || cells.get(x, y - 1) < 0;
        let shaded = cells.get(x + 1, y) < 0 && cells.get(x, y + 1) < 0;
        r.px(v + lit as i32 - shaded as i32)
    })
}

fn marble_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.75);
    let mut m = Map::from_fn(|x, y| {
        let c = 0.6 * pnoise(x, y, 2, 2, 152) + 0.4 * pnoise(x, y, 4, 4, 157);
        if c > 0.57 { 4 } else { 3 }
    });
    m.despeckle();
    // Two veins cross the tile from left to right. Each ends where the
    // other begins, so together they make one endless line over a wall.
    // (start y, rise across the tile, step, seed)
    let veins: [(f32, f32, i32, u32); 2] = [(2.0, 10.0, 1, 153), (12.0, 6.0, 2, 154)];
    for &(y0, rise, step, seed) in &veins {
        let mut last: Option<i32> = None;
        for x in 0..=S {
            let t = x as f32 / 16.0;
            let wobble = (pnoise(x, 0, 4, 1, seed) - 0.5) * 5.0 * (t * TAU / 2.0).sin();
            let y = (y0 + rise * t + wobble).round() as i32;
            let from = last.unwrap_or(y);
            for yy in from.min(y)..=from.max(y) {
                if x < S {
                    m.set(x, yy, step);
                }
            }
            // a soft edge under the darker vein
            if step == 1 && x < S && x % 5 != 0 {
                m.set(x, from.max(y) + 1, 2);
            }
            last = Some(y);
        }
    }
    // a short branch
    dots(&mut m, &[(7, 5), (8, 4), (9, 4), (10, 3), (11, 3)], 2);
    m.img(&r)
}

fn terracotta_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.7);
    const BANDS: [i32; 16] = [3, 3, 4, 3, 3, 2, 2, 3, 3, 3, 4, 4, 3, 2, 3, 3];
    let mut m = Map::from_fn(|x, y| {
        let band = BANDS[y as usize];
        if band != 3 && pnoise(x, y, 3, 16, 161) < 0.38 {
            3
        } else {
            band
        }
    });
    dots(
        &mut m,
        &[(2, 1), (9, 3), (13, 8), (5, 9), (11, 13), (0, 14)],
        2,
    );
    dots(&mut m, &[(6, 0), (14, 4), (3, 12)], 4);
    m.img(&r)
}

fn bedrock_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 1.25);
    let chunks = sites(4, 4, 1.0, 171);
    let owner = |x: i32, y: i32| nearest(x as f32 + 0.5, y as f32 + 0.5, &chunks, 0.7, 1.0).i;
    Img::from_fn(|x, y| {
        let me = owner(x, y);
        let tone = [1, 3, 2, 4, 1, 2, 3][(hash(me as i32, 0, 172) % 7) as usize];
        let lit = owner(x - 1, y) != me || owner(x, y - 1) != me;
        let shaded = owner(x + 1, y) != me || owner(x, y + 1) != me;
        r.px(tone + lit as i32 - shaded as i32 * 2)
    })
}

/// Small ore lumps as ramp steps. Lit on the top-left.
const NUGGETS: [[&str; 4]; 4] = [
    [".45.", "4332", "3321", ".21."],
    ["45..", "332.", ".21.", "...."],
    [".4..", "433.", "3321", ".21."],
    ["..45", ".432", ".321", "...."],
];

/// Stone with ore lumps set into it. Each lump throws a shadow on the stone.
fn ore_img(stone: &Img, stone_ramp: &Ramp, ore: &Ramp, layout: &[(i32, i32, usize)]) -> Img {
    let mut img = stone.clone();
    let mut mask = Map::new(0);
    for &(x, y, shape) in layout {
        stamp(&mut mask, x, y, &NUGGETS[shape], 0);
    }
    for y in 0..S {
        for x in 0..S {
            let v = mask.get(x, y);
            if v > 0 {
                img.set(x, y, ore.px(v));
            } else if mask.get(x - 1, y) > 0 || mask.get(x, y - 1) > 0 {
                img.set(x, y, stone_ramp.px(1));
            } else if mask.get(x + 1, y) > 0 && mask.get(x, y + 1) > 0 {
                img.set(x, y, stone_ramp.px(4));
            }
        }
    }
    img
}

/// Emberstone: dark rock split by glowing veins. `pulse` 0..1 is how far the
/// glow has swelled.
fn ember_img(pulse: f32) -> Img {
    let rock = Ramp::new([0.27, 0.18, 0.19]);
    let fire = Ramp::fire();
    let chunks = sites(3, 3, 1.0, 181);
    Img::from_fn(|x, y| {
        let hit = cell(x, y, &chunks);
        let gap = hit.d2 - hit.d1;
        // veins are hotter in some stretches than others
        let hot = 0.55 + 0.6 * pnoise(x, y, 3, 3, 182);
        let heat = (1.0 - gap / 2.0) * hot + pulse * 0.13;
        if heat > 0.92 {
            fire.px(5)
        } else if heat > 0.74 {
            fire.px(4)
        } else if heat > 0.52 {
            fire.px(3)
        } else if heat > 0.34 {
            fire.px(2)
        } else if heat > 0.20 {
            fire.px(1)
        } else {
            let tone = [3, 2, 3][(hash(hit.i as i32, 0, 183) % 3) as usize];
            let lit = hit.dx + hit.dy < -2.2;
            rock.px(tone + lit as i32)
        }
    })
}

// ---------------------------------------------------------------- wood and leaves

/// Which board joint each of the four plank rows has.
const JOINTS: [i32; 4] = [5, 13, 2, 10];

fn planks_img(base: Rgb, seed: u32) -> Img {
    let a = Ramp::new(base);
    let b = Ramp::new(saturate(scale(base, 0.91), 1.08));
    let mut img = Img::from_fn(|x, y| {
        let (board, row) = (y / 4, y % 4);
        let joint = JOINTS[board as usize];
        // One board in four is a darker piece of wood. A row has one joint
        // per tile, so the board runs on across the tile edge and the whole
        // row must share a tone.
        let r = if (board + seed as i32) % 4 == 1 {
            &b
        } else {
            &a
        };
        if row == 3 {
            return r.px(if x == joint { 0 } else { 1 });
        }
        if x == joint {
            return r.px(1);
        }
        let grain = pnoise(x, y, 4, 16, seed ^ 0x77);
        let mut step = if row == 0 { 4 } else { 3 };
        if grain < 0.30 {
            step -= 1;
        } else if grain > 0.76 && row == 1 {
            step += 1;
        }
        if (x - joint).rem_euclid(S) == 1 && row < 2 {
            step = 4;
        }
        r.px(step)
    });
    // one knot
    let (kx, ky) = (8 + (seed % 5) as i32, 9);
    img.set(kx, ky, a.px(1));
    img.set(kx + 1, ky, a.px(0));
    img.set(kx - 1, ky, a.px(2));
    img.set(kx + 2, ky, a.px(4));
    img
}

/// Bark: ridges that run up the trunk, split by dark grooves that jog
/// sideways and break where plates of bark meet.
fn bark_map(seed: u32) -> Map {
    let mut m = Map::new(3);
    for (k, base_x) in [2, 6, 10, 14].into_iter().enumerate() {
        for y in 0..S {
            // each groove runs in two stretches, staggered from its neighbours
            let along = y + k as i32 * 5;
            let h = hash(k as i32, (along / 8) % 2, seed ^ 0x5A);
            let lean = if along % 8 >= 4 {
                (h % 3) as i32 - 1
            } else {
                0
            };
            let x = base_x + lean;
            if along % 8 == 0 {
                m.set(x, y, 2);
                continue;
            }
            m.set(x - 1, y, 2);
            m.set(x + 1, y, 4);
            m.set(x, y, 1);
        }
    }
    // nicks across the ridges
    dots(&mut m, &[(4, 3), (8, 10), (12, 6), (0, 13), (5, 14)], 2);
    m
}

/// The cut end of a log: rings inside a rim of bark.
fn log_top_img(bark: &Ramp, wood: &Ramp, seed: u32) -> Img {
    const RINGS: [i32; 7] = [1, 4, 3, 4, 2, 4, 3];
    Img::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        let round = (dx * dx + dy * dy).sqrt();
        let square = dx.abs().max(dy.abs());
        let d = round * 0.4 + square * 0.6;
        if d > 6.6 {
            // bark rim, lit on the top-left
            let lit = dx + dy < 0.0;
            let outer = square > 7.0;
            return bark.px(match (outer, lit) {
                (true, true) => 3,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 0,
            });
        }
        let wobble = (pnoise(x, y, 4, 4, seed) - 0.5) * 1.1;
        let ring = (d * 0.95 + wobble).max(0.0) as usize;
        wood.px(RINGS[ring.min(6)])
    })
}

fn birch_bark_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.8);
    let dark = Ramp::new([0.36, 0.32, 0.33]);
    // soft upright streaks
    let mut m = Map::from_fn(|x, y| {
        let n = 0.7 * pnoise(x, y, 4, 2, 81) + 0.3 * pnoise(x, y, 8, 3, 82);
        if n < 0.36 {
            2
        } else if n > 0.62 {
            4
        } else {
            3
        }
    });
    m.despeckle();
    let mut img = m.img(&r);
    // dark dashes across the trunk: (x, y, length)
    let dashes: [(i32, i32, i32); 8] = [
        (1, 1, 3),
        (9, 2, 4),
        (5, 5, 2),
        (0, 8, 2),
        (12, 6, 3),
        (6, 14, 4),
        (14, 11, 3),
        (2, 12, 2),
    ];
    for &(x, y, len) in &dashes {
        for i in 0..len {
            let end = i == 0 || i + 1 == len;
            img.set(x + i, y, dark.px(if end { 3 } else { 1 }));
            if !end {
                img.set(x + i, y + 1, r.px(2));
            }
        }
    }
    // a knot: a dark eye with a lid over it
    art(&mut img, 6, 8, &[".22.", "2112", ".33."], &[&dark]);
    img
}

/// Small leaf shapes as ramp steps. Lit on the top-left.
const LEAF_SHAPES: [[&str; 3]; 4] = [
    ["45.", "433", ".32"],
    [".44", "433", "32."],
    ["44.", "433", ".3."],
    [".5.", "433", "332"],
];

/// Round leaves for birch: smaller, so more of the dark shows between them.
const ROUND_LEAVES: [[&str; 3]; 4] = [
    [".5.", "443", ".3."],
    ["54.", "43.", "..."],
    [".5.", "443", ".32"],
    ["45.", "33.", "..."],
];

/// Pine needles: short strokes that all slant the same way.
const NEEDLES: [[&str; 3]; 4] = [
    ["..5", ".43", "32."],
    ["..4", ".43", "33."],
    [".5.", "43.", "3.."],
    ["..4", ".42", "32."],
];

/// Make up to `count` of the deepest pixels see-through. The colour under a
/// hole stays dark, so the texture still works if alpha is ignored.
fn punch_holes(img: &mut Img, m: &Map, r: &Ramp, count: usize, seed: u32) {
    let mut deep: Vec<(u32, i32, i32)> = Vec::new();
    for y in 0..S {
        for x in 0..S {
            if m.get(x, y) <= 1 && (m.get(x + 1, y) <= 1 || m.get(x, y + 1) <= 1) {
                deep.push((hash(x, y, seed), x, y));
            }
        }
    }
    deep.sort();
    for &(_, x, y) in deep.iter().take(count) {
        img.set(x, y, r.pxa(0, 0));
    }
}

/// Leaves in two layers: dim ones behind, lit ones in front. One front
/// leaf in every `skip` is left out, which opens the canopy up.
fn leaves_img(base: Rgb, seed: u32, shapes: &[[&str; 3]; 4], skip: u32, holes: usize) -> Img {
    let r = Ramp::with(base, 1.15);
    let mut m = Map::new(1);
    for layer in 0..2 {
        for cy in 0..4 {
            for cx in 0..4 {
                let h = hash(cx, cy, seed + layer);
                if layer == 1 && h.is_multiple_of(skip) {
                    continue;
                }
                let shift = if layer == 0 { 2 } else { 0 };
                let x = cx * 4 + shift + (h >> 3) as i32 % 3;
                let y = cy * 4 + shift + (h >> 6) as i32 % 3;
                let shape = &shapes[((h >> 9) % 4) as usize];
                stamp(&mut m, x, y, shape, if layer == 0 { -2 } else { 0 });
            }
        }
    }
    let mut img = m.img(&r);
    punch_holes(&mut img, &m, &r, holes, seed);
    img
}

// ---------------------------------------------------------------- cloth, glass, water

/// Wool: a soft knit of 3×2 stitches, each row offset by half a stitch.
fn wool_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.72);
    const KNIT: [&[u8; 4]; 4] = [b"4332", b"3322", b"3243", b"2233"];
    Img::from_fn(|x, y| {
        let step = KNIT[(y % 4) as usize][(x % 4) as usize] - b'0';
        r.px(step as i32)
    })
}

fn glass_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.9);
    let mut img = Img::new(r.pxa(4, 46));
    for i in 1..S - 1 {
        // the frame throws a faint shadow on the pane
        img.set(i, 1, r.pxa(2, 96));
        img.set(1, i, r.pxa(2, 96));
    }
    for i in 0..S {
        img.set(i, 0, r.px(5));
        img.set(0, i, r.px(5));
        img.set(i, S - 1, r.px(2));
        img.set(S - 1, i, r.px(2));
    }
    img.set(S - 1, 0, r.px(3));
    img.set(0, S - 1, r.px(3));
    // two glints and a small echo in the far corner: (x, y, alpha)
    let shine: [(i32, i32, u8); 11] = [
        (3, 7, 150),
        (4, 6, 235),
        (5, 5, 235),
        (6, 4, 235),
        (7, 3, 150),
        (6, 9, 120),
        (7, 8, 200),
        (8, 7, 200),
        (9, 6, 120),
        (11, 12, 110),
        (12, 11, 140),
    ];
    for &(x, y, a) in &shine {
        img.set(x, y, [250, 253, 255, a]);
    }
    img
}

fn ice_img(base: Rgb) -> Img {
    let r = Ramp::with(base, 0.9);
    let m = patch_map(&sites(3, 3, 1.0, 131), 1.6, 132, &[3, 3, 4, 3, 2]);
    let mut img = Img::from_fn(|x, y| r.pxa(m.get(x, y), 176));
    let cracks: [&[(i32, i32)]; 4] = [
        &[(1, 2), (2, 3), (3, 3), (4, 4), (5, 5), (6, 5), (7, 6)],
        &[(4, 4), (5, 3), (5, 2), (6, 1)],
        &[(9, 10), (10, 11), (11, 11), (12, 12), (13, 13), (14, 13)],
        &[(12, 12), (12, 13), (11, 14)],
    ];
    for line in cracks {
        for (i, &(x, y)) in line.iter().enumerate() {
            // a crack fades out at its ends
            let end = i == 0 || i + 1 == line.len();
            img.set(
                x,
                y,
                if end {
                    r.pxa(5, 205)
                } else {
                    [240, 248, 255, 235]
                },
            );
        }
    }
    // frost glints
    for &(x, y) in &[(12, 3), (13, 2), (2, 11), (9, 7)] {
        img.set(x, y, r.pxa(5, 215));
    }
    img
}

/// Eight frames of water. Two wave layers drift one pixel per frame, one
/// sideways and one down. Each repeats every 8 pixels along its drift, so
/// the animation loops and every frame tiles.
fn water_frames(base: Rgb) -> Vec<Img> {
    let r = Ramp::with(base, 0.85);
    (0..8)
        .map(|t| {
            let field = |x: i32, y: i32| {
                let ax = ((x - t).rem_euclid(8) as f32 + 0.5) * 2.0;
                let a = tnoise(ax, y as f32 + 0.5, 2, 5, 121);
                let by = ((y + t).rem_euclid(8) as f32 + 0.5) * 2.0;
                let b = tnoise(x as f32 + 0.5, by, 3, 3, 122);
                0.55 * a + 0.45 * b
            };
            Img::from_fn(|x, y| {
                let v = field(x, y);
                if v > 0.58 {
                    // the top edge of a swell is its crest
                    if field(x, y - 1) <= 0.58 {
                        r.pxa(5, 200)
                    } else {
                        r.pxa(4, 186)
                    }
                } else if v < 0.38 {
                    r.pxa(2, 180)
                } else {
                    r.pxa(3, 180)
                }
            })
        })
        .collect()
}

// ---------------------------------------------------------------- made things

fn table_top_img(base: Rgb) -> Img {
    let r = Ramp::new(base);
    let m = Map::from_fn(|x, y| {
        if x == 15 || y == 15 {
            1
        } else if x == 0 || y == 0 {
            2
        } else if x == 5 || x == 10 || y == 5 || y == 10 {
            // the grooves of the 3×3 grid
            1
        } else if x == 6 || x == 11 || y == 6 || y == 11 || x == 1 || y == 1 {
            4
        } else if pnoise(x, y, 4, 16, 211) < 0.3 {
            2
        } else {
            3
        }
    });
    m.img(&r)
}

const SAW: [&str; 4] = ["......cdc", ".55555d.d", "444444cdc", "3.3.3.3.."];
const HAMMER: [&str; 8] = [
    "45543", "34432", ".dc..", ".dc..", ".dc..", ".dc..", ".dc..", ".cb..",
];

fn table_side_img(base: Rgb) -> Img {
    let r = Ramp::new(base);
    let metal = Ramp::new([0.66, 0.67, 0.72]);
    let dark = Ramp::new(scale(base, 0.5));
    let mut img = planks_img(base, 212);
    for x in 0..S {
        // the edge of the table top, and its shadow
        img.set(x, 0, r.px(5));
        img.set(x, 1, r.px(4));
        img.set(x, 2, r.px(3));
        img.set(x, 3, r.px(0));
    }
    for y in 4..S {
        img.set(0, y, r.px(2));
        img.set(S - 1, y, r.px(1));
    }
    art(&mut img, 1, 9, &SAW, &[&metal, &dark]);
    art(&mut img, 10, 5, &HAMMER, &[&metal, &dark]);
    img
}

const FURNACE_MOUTH: [&str; 11] = [
    "....4444....",
    "..44aaaa42..",
    ".4aaaaaaaa2.",
    ".4aaaaaaaa2.",
    ".4aaaaaaaa2.",
    ".4aaaaaaaa2.",
    ".4aabaabaa2.",
    ".4abcbbcba2.",
    ".4bcdccdcb2.",
    "455555555553",
    "122222222221",
];

fn furnace_side_img(base: Rgb) -> Img {
    let r = Ramp::new(base);
    let mut img = stone_brick_map(221).img(&r);
    art(&mut img, 2, 4, &FURNACE_MOUTH, &[&r, &Ramp::fire()]);
    img
}

/// A stone slab with a raised rim: the top and bottom of the furnace.
fn slab_img(r: &Ramp, seed: u32) -> Img {
    let mut m = patch_map(&sites(3, 3, 1.0, seed), 1.4, seed + 1, &[3, 3, 2, 3]);
    for i in 0..S {
        m.set(i, 0, 4);
        m.set(0, i, 4);
    }
    for i in 0..S {
        m.set(i, 15, 1);
        m.set(15, i, 1);
    }
    for i in 2..14 {
        m.set(i, 2, 1);
        m.set(2, i, 1);
        m.set(i, 13, 4);
        m.set(13, i, 4);
    }
    m.img(r)
}

const CHEST_SIDE: [&str; 16] = [
    "1222222222222221",
    "2444444444444431",
    "2433333333333321",
    "2333233333323321",
    "2222222cc2222221",
    "1000000ed0000001",
    "2444444dc4444431",
    "2433333ab3333321",
    "2333333333333321",
    "2332333333233321",
    "2222222222222221",
    "2444444444444431",
    "2433333333333321",
    "2333323333333321",
    "1222222222222211",
    "0111111111111110",
];

const CHEST_TOP: [&str; 16] = [
    "1222222222222221",
    "244dc444444dc431",
    "243dc333333dc321",
    "233eb332333eb321",
    "222dc222222dc221",
    "244dc444444dc431",
    "243dc333333dc321",
    "233dc333233dc321",
    "233dc333333dc321",
    "222dc222222dc221",
    "244dc444444dc431",
    "243dc333333dc321",
    "233eb333333eb321",
    "233dc323333dc321",
    "122cb222222cb211",
    "0111111111111110",
];

const BED_TOP: [&str; 16] = [
    "BCCCCCCCCCCCCCCA",
    "cdddddddddddddcb",
    "cdeffffffffffdcb",
    "cdefeeeeeeeeedcb",
    "cdeeeeeeeeeeddcb",
    "cdddddddddddcccb",
    "bccccccccccccccb",
    "4555555555555543",
    "3444444444444432",
    "2222222222222221",
    "3443333333333432",
    "3433333333333432",
    "3433334333333432",
    "3433333333333432",
    "3433333333343432",
    "2322222222222321",
];

const BED_SIDE: [&str; 16] = [
    "4555555555555554",
    "3444444444444443",
    "3433333333333432",
    "3333333333333332",
    "3333343333333332",
    "ddeeeeeeeeeeeedc",
    "3333333333333332",
    "2222222222222221",
    "CDDDDDDDDDDDDDDB",
    "BCCCCCCCCCCCCCCA",
    "ABBBBBBBBBBBBBBA",
    "CDBAAAAAAAAAACDB",
    "CDBAAAAAAAAAACDB",
    "CDBAAAAAAAAAACDB",
    "CCBAAAAAAAAAACCB",
    "BBAAAAAAAAAAABBA",
];

/// The iron cage of a lantern, drawn over its glow.
const LANTERN_CAGE: [&str; 16] = [
    "bccccccccccccccb",
    "abbbbbbbbbbbbbba",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "c..............a",
    "bccccccccccccccb",
    "aaaaaaaaaaaaaaaa",
];

const LANTERN_TOP: [&str; 16] = [
    "bcccccccccccccca",
    "cdddddddddddddba",
    "cdccccccccccccba",
    "cdc4cccccccc4cba",
    "cdccccccccccccba",
    "cdccccbddbccccba",
    "cdcccbdaadbcccba",
    "cdcccdaaaadcccba",
    "cdcccdaaaabcccba",
    "cdcccbdaabacccba",
    "cdccccbbaaccccba",
    "cdccccccccccccba",
    "cdc4cccccccc4cba",
    "cdbbbbbbbbbbbbba",
    "cbbbbbbbbbbbbbaa",
    "aaaaaaaaaaaaaaaa",
];

fn lantern_glow(glow: &Ramp) -> Img {
    Img::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 8.0);
        // a flame-shaped core: narrower above the middle
        let d = (dx * dx * if dy < 0.0 { 1.5 } else { 0.9 } + dy * dy * 0.8).sqrt();
        let step = if d < 2.2 {
            5
        } else if d < 4.4 {
            4
        } else if d < 7.0 {
            3
        } else {
            2
        };
        // thin glass bars, seen against the light
        let bar = x == 5 || x == 10 || y == 9;
        glow.px(step - (bar && step < 5) as i32)
    })
}

/// Glowshroom: a dark fungus body with glowing spots. Each spot is
/// (centre x, centre y, radius).
fn shroom_img(base: Rgb, spots: &[(f32, f32, f32)], seed: u32) -> Img {
    let body = Ramp::new([0.15, 0.22, 0.33]);
    let glow = Ramp::glow(base);
    let flesh = patch_map(&sites(4, 4, 1.0, seed), 1.0, seed + 1, &[3, 2, 3, 2, 4]);
    Img::from_fn(|x, y| {
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        // distance past the edge of the nearest spot
        let edge = spots
            .iter()
            .map(|s| wrap_delta(px, s.0).hypot(wrap_delta(py, s.1)) - s.2)
            .fold(f32::MAX, f32::min);
        if edge < -1.4 {
            glow.px(5)
        } else if edge < -0.5 {
            glow.px(4)
        } else if edge < 0.2 {
            glow.px(3)
        } else if edge < 1.0 {
            glow.px(1)
        } else if edge < 1.8 {
            glow.px(0)
        } else {
            body.px(flesh.get(x, y))
        }
    })
}

/// The side of a torch post. The renderer shows the strip from column 6.5
/// to 9.5 over the full height, so the stick is drawn four pixels wide in
/// columns 6 to 9 and repeated across the tile: flame in the top six rows,
/// a charred band, then wood.
fn torch_side_img(frame: i32) -> Img {
    let fire = Ramp::fire();
    let wood = Ramp::new([0.52, 0.38, 0.22]);
    /// heat of each flame row, in the middle of the stick and at its edges
    const MIDDLE: [i32; 7] = [5, 5, 5, 4, 4, 3, 2];
    const EDGE: [i32; 7] = [4, 4, 4, 3, 3, 2, 1];
    Img::from_fn(|x, y| {
        // 0 and 3 are the edges of the stick, 1 and 2 its middle
        let across = (x + 2) % 4;
        if y >= 7 {
            let step = [4, 3, 2, 1][across as usize];
            return wood.px(if y == 7 { step - 2 } else { step });
        }
        if y == 6 {
            // the charred tip, with an ember that wanders
            return if across == 1 + frame % 2 {
                fire.px(1)
            } else {
                wood.px(0)
            };
        }
        // the flame flickers: each side rises and falls by a row
        let side = x.rem_euclid(S) / 2;
        let flicker = (hash(side, frame, 241) % 2) as i32;
        let table = if across == 1 || across == 2 {
            &MIDDLE
        } else {
            &EDGE
        };
        fire.px(table[(y + flicker) as usize])
    })
}

fn torch_top_img() -> Img {
    let fire = Ramp::fire();
    Img::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        let d = dx.abs().max(dy.abs()) * 0.5 + dx.hypot(dy) * 0.5;
        fire.px(5 - (d / 2.3) as i32)
    })
}

/// Ring of stones around a pile of embers. `pulse` 0..1 swells the glow.
fn hearth_top_img(stone: &Ramp, pulse: f32) -> Img {
    let fire = Ramp::fire();
    let coal = Ramp::new([0.21, 0.15, 0.16]);
    let embers = [
        (6.5, 6.0),
        (9.5, 6.5),
        (8.0, 8.5),
        (5.5, 9.0),
        (10.5, 9.5),
        (7.5, 11.0),
        (8.5, 4.5),
    ];
    // Steps of the rim stones, -1 for the joints and -2 for the fire pit.
    let rim = Map::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        let d = dx.hypot(dy) * 0.5 + dx.abs().max(dy.abs()) * 0.5;
        if d < 4.7 {
            return -2;
        }
        // twelve stones around the pit
        let turn = (dy.atan2(dx) / TAU + 1.02).fract() * 12.0;
        if turn.fract() < 0.14 {
            -1
        } else {
            [3, 2, 3, 4, 3][(hash(turn as i32, 0, 252) % 5) as usize]
        }
    });
    Img::from_fn(|x, y| {
        let v = rim.get(x, y);
        if v == -1 {
            return stone.px(0);
        }
        if v >= 0 {
            // the inner edge of the rim glows from the fire
            if (-1..=1).any(|d| rim.get(x + d, y) == -2 || rim.get(x, y + d) == -2) {
                return fire.px(2);
            }
            let lit = rim.get(x - 1, y) == -1 || rim.get(x, y - 1) == -1 || x == 0 || y == 0;
            let shaded = rim.get(x + 1, y) == -1 || rim.get(x, y + 1) == -1 || x == 15 || y == 15;
            return stone.px(v + lit as i32 - shaded as i32);
        }
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        let hit = cell(x, y, &embers);
        let middle = 1.0 - dx.hypot(dy) / 5.0;
        let heat = (1.0 - (hit.d2 - hit.d1) / 2.4) * 0.7 + middle * 0.35 + pulse * 0.12;
        if heat > 0.92 {
            fire.px(5)
        } else if heat > 0.78 {
            fire.px(4)
        } else if heat > 0.62 {
            fire.px(3)
        } else if heat > 0.48 {
            fire.px(2)
        } else if heat > 0.36 {
            fire.px(1)
        } else {
            coal.px(if hit.dx + hit.dy < -0.5 { 3 } else { 2 })
        }
    })
}

/// Stone bricks with cracks that glow from the fire inside.
fn hearth_side_img(stone: &Ramp) -> Img {
    let fire = Ramp::fire();
    let mut img = stone_brick_map(253).img(stone);
    let cracks: [&[(i32, i32)]; 4] = [
        &[(1, 1), (2, 2), (3, 2), (4, 3), (4, 4), (5, 5), (6, 5)],
        &[(9, 4), (10, 5), (11, 5), (12, 6), (12, 7), (13, 8), (14, 8)],
        &[(4, 9), (5, 10), (6, 10), (7, 11), (8, 11), (9, 12), (9, 13)],
        &[(12, 12), (13, 13), (14, 13)],
    ];
    for line in cracks {
        for (i, &(x, y)) in line.iter().enumerate() {
            let from_end = i.min(line.len() - 1 - i);
            img.set(x, y, fire.px(2 + from_end.min(3) as i32));
            // the glow spills onto the stone under the crack
            if img.get(x, y + 1) == stone.px(0) || from_end > 1 {
                img.set(x, y + 1, fire.px(1));
            }
        }
    }
    img
}

const BERRY_CLUSTERS: [[&str; 4]; 2] = [
    ["54a.", "3254", "aa32", "..aa"],
    [".54.", "a32a", "5454", "3232"],
];

fn berry_bush_img(base: Rgb) -> Img {
    let mut img = leaves_img(base, 231, &LEAF_SHAPES, 9, 0);
    let berry = Ramp::new([0.82, 0.14, 0.22]);
    let leaf = Ramp::with(base, 1.15);
    let clusters: [(i32, i32, usize); 4] = [(1, 2, 0), (9, 1, 1), (4, 9, 1), (11, 10, 0)];
    for &(x, y, kind) in &clusters {
        art(&mut img, x, y, &BERRY_CLUSTERS[kind], &[&berry, &leaf]);
    }
    img
}

/// Plain soft texture for blocks without a painter of their own.
fn generic_img(base: Rgb, alpha: f32) -> Img {
    let r = Ramp::with(base, 0.8);
    let a = (alpha.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let seed = hash((base[0] * 255.0) as i32, (base[1] * 255.0) as i32, 5);
    let m = patch_map(&sites(3, 3, 1.0, seed), 1.3, seed ^ 9, &[3, 3, 2, 4]);
    Img::from_fn(|x, y| r.pxa(m.get(x, y), a))
}

// ---------------------------------------------------------------- blocks

#[derive(Default)]
struct Builder {
    layers: Vec<Rgba>,
    animated: Vec<(u16, u16, f32)>,
}

impl Builder {
    fn add(&mut self, img: &Img) -> u16 {
        self.layers.push(img.rgba());
        (self.layers.len() - 1) as u16
    }

    /// One texture on all three faces.
    fn all(&mut self, img: &Img) -> [u16; 3] {
        [self.add(img); 3]
    }

    /// Add the frames of an animation as consecutive layers.
    fn frames(&mut self, frames: &[Img], secs: f32) -> u16 {
        let first = self.layers.len() as u16;
        for f in frames {
            self.add(f);
        }
        self.animated.push((first, frames.len() as u16, secs));
        first
    }
}

/// Base colour of a block.
fn color_of(id: u8) -> Rgb {
    w::block(id).color
}

/// A swell that rises and falls over `count` frames: 0, up to 1, back to 0.
fn pulse(frame: usize, count: usize) -> f32 {
    0.5 - 0.5 * (frame as f32 / count as f32 * TAU).cos()
}

fn build_blocks() -> Textures {
    let mut b = Builder::default();
    let mut faces = vec![[0u16; 3]; BLOCKS.len()];

    // Textures that several blocks share are built once.
    let air = b.add(&Img::new(CLEAR));
    let dirt_ramp = Ramp::new(color_of(w::DIRT));
    let dirt = dirt_img(color_of(w::DIRT));
    let dirt_l = b.add(&dirt);
    let stone_ramp = Ramp::with(color_of(w::STONE), 1.1);
    let stone = stone_map().img(&stone_ramp);
    let stone_l = b.add(&stone);

    for (id, slot) in faces.iter_mut().enumerate() {
        let id = id as u8;
        let base = color_of(id);
        *slot = match id {
            w::AIR => [air; 3],
            w::GRASS => {
                let grass = Ramp::new(base);
                let top = b.add(&grass_top_map().img(&grass));
                let side = b.add(&grass_side_img(&grass, &dirt, &dirt_ramp));
                [top, side, dirt_l]
            }
            w::DIRT => [dirt_l; 3],
            w::STONE => [stone_l; 3],
            w::SAND => b.all(&sand_img(base)),
            w::LOG | w::BIRCH_LOG => {
                let (side, wood) = if id == w::LOG {
                    let bark = Ramp::new(base);
                    (bark_map(71).img(&bark), mix(base, [0.86, 0.70, 0.46], 0.7))
                } else {
                    (birch_bark_img(base), [0.87, 0.78, 0.58])
                };
                let rim = Ramp::new(if id == w::LOG { base } else { scale(base, 0.9) });
                let end = b.add(&log_top_img(&rim, &Ramp::new(wood), 72 + id as u32));
                [end, b.add(&side), end]
            }
            w::LEAVES => b.all(&leaves_img(base, 61, &LEAF_SHAPES, 9, 14)),
            w::PLANKS | DARK_PLANKS | BIRCH_PLANKS => b.all(&planks_img(base, 100 + id as u32)),
            w::GLASS => b.all(&glass_img(base)),
            w::EMBERSTONE => {
                let frames: Vec<Img> = (0..4).map(|f| ember_img(pulse(f, 4))).collect();
                [b.frames(&frames, 0.35); 3]
            }
            w::BRICK => b.all(&brick_img(base, 131)),
            w::SNOW => b.all(&snow_img(base)),
            w::COBBLE => {
                let m = cobble_map(&COBBLE_SITES, 1.05, 51);
                b.all(&cobble_img(&m, &Ramp::with(base, 1.1)))
            }
            w::WATER => [b.frames(&water_frames(base), 0.2); 3],
            w::LANTERN => {
                let iron = Ramp::new([0.24, 0.23, 0.29]);
                let glow = Ramp::glow(base);
                let mut side = lantern_glow(&glow);
                art(&mut side, 0, 0, &LANTERN_CAGE, &[&glow, &iron]);
                let mut top = Img::new(iron.px(2));
                art(&mut top, 0, 0, &LANTERN_TOP, &[&glow, &iron]);
                let top = b.add(&top);
                [top, b.add(&side), top]
            }
            BASALT => {
                let top = b.add(&basalt_top_img(base));
                [top, b.add(&basalt_side_img(base)), top]
            }
            MARBLE => b.all(&marble_img(base)),
            w::CLAY => b.all(&clay_img(base)),
            TERRACOTTA => b.all(&terracotta_img(base)),
            MOSSY_COBBLE => b.all(&mossy_img(color_of(w::COBBLE), [0.30, 0.50, 0.20])),
            ICE => b.all(&ice_img(base)),
            WOOL_FIRST..=WOOL_LAST => b.all(&wool_img(base)),
            w::GLOWSHROOM => {
                let top = [
                    (4.5, 4.5, 2.8),
                    (12.0, 10.5, 2.4),
                    (11.5, 3.0, 1.5),
                    (3.5, 12.0, 1.6),
                    (8.0, 7.5, 0.9),
                ];
                let side = [
                    (3.5, 3.0, 2.2),
                    (11.0, 5.5, 1.6),
                    (7.0, 10.5, 2.0),
                    (14.0, 12.5, 1.3),
                    (1.5, 11.5, 1.0),
                    (8.5, 1.0, 0.9),
                ];
                let under = [(4.0, 4.0, 1.0), (11.0, 6.0, 0.8), (7.0, 12.0, 1.0)];
                [
                    b.add(&shroom_img(base, &top, 261)),
                    b.add(&shroom_img(base, &side, 262)),
                    b.add(&shroom_img(base, &under, 263)),
                ]
            }
            w::BIRCH_LEAVES => b.all(&leaves_img(base, 62, &ROUND_LEAVES, 5, 22)),
            w::PINE_LEAVES => b.all(&leaves_img(base, 63, &NEEDLES, 9, 10)),
            w::BEDROCK => b.all(&bedrock_img(base)),
            w::COAL_ORE => {
                let coal = Ramp::new([0.13, 0.13, 0.16]);
                let layout = [(1, 1, 0), (9, 3, 2), (4, 9, 3), (11, 11, 0), (13, 7, 1)];
                b.all(&ore_img(&stone, &stone_ramp, &coal, &layout))
            }
            w::IRON_ORE => {
                let iron = Ramp::new([0.80, 0.60, 0.48]);
                let layout = [(2, 2, 2), (10, 1, 1), (6, 7, 0), (12, 10, 3), (1, 11, 1)];
                b.all(&ore_img(&stone, &stone_ramp, &iron, &layout))
            }
            w::CRAFTING_TABLE => [
                b.add(&table_top_img(base)),
                b.add(&table_side_img(base)),
                b.add(&planks_img(base, 213)),
            ],
            w::FURNACE => {
                let slab = b.add(&slab_img(&Ramp::new(base), 222));
                [slab, b.add(&furnace_side_img(base)), slab]
            }
            w::CHEST => {
                let wood = Ramp::new(base);
                let iron = Ramp::new([0.62, 0.63, 0.68]);
                let mut top = Img::new(CLEAR);
                art(&mut top, 0, 0, &CHEST_TOP, &[&wood, &iron]);
                let mut side = Img::new(CLEAR);
                art(&mut side, 0, 0, &CHEST_SIDE, &[&wood, &iron]);
                [b.add(&top), b.add(&side), b.add(&planks_img(base, 214))]
            }
            w::BED => {
                let red = Ramp::new(base);
                let linen = Ramp::with([0.93, 0.92, 0.88], 0.7);
                let wood = Ramp::new(color_of(w::PLANKS));
                let mut top = Img::new(CLEAR);
                art(&mut top, 0, 0, &BED_TOP, &[&red, &linen, &wood]);
                let mut side = Img::new(CLEAR);
                art(&mut side, 0, 0, &BED_SIDE, &[&red, &linen, &wood]);
                [
                    b.add(&top),
                    b.add(&side),
                    b.add(&planks_img(color_of(w::PLANKS), 215)),
                ]
            }
            w::BERRY_BUSH => b.all(&berry_bush_img(base)),
            w::TORCH => {
                let frames: Vec<Img> = (0..4).map(torch_side_img).collect();
                let wood = Ramp::new([0.52, 0.38, 0.22]);
                let end = Map::from_fn(|x, y| {
                    let d = (2 * x - 15).abs().max((2 * y - 15).abs()) / 2;
                    [1, 4, 3, 4, 2, 3, 2, 1][d as usize]
                });
                [
                    b.add(&torch_top_img()),
                    b.frames(&frames, 0.14),
                    b.add(&end.img(&wood)),
                ]
            }
            w::HEARTH => {
                // The stone is the block's colour pulled towards grey, so
                // that the embers stand out against it.
                let stone = Ramp::new(mix(base, [0.50, 0.47, 0.47], 0.45));
                let frames: Vec<Img> = (0..4)
                    .map(|f| hearth_top_img(&stone, pulse(f, 4)))
                    .collect();
                [
                    b.frames(&frames, 0.3),
                    b.add(&hearth_side_img(&stone)),
                    b.add(&slab_img(&stone, 254)),
                ]
            }
            // a block added after this was written: its own colour, no detail
            _ => b.all(&generic_img(base, w::block(id).alpha)),
        };
    }
    // the last layer stands in for unknown block IDs
    b.add(&generic_img([0.45, 0.42, 0.48], 1.0));
    Textures {
        layers: b.layers,
        block_faces: faces,
        animated: b.animated,
    }
}

// ---------------------------------------------------------------- items

// Item pictures. Digits are steps of the item's own colour, `a`-`f` are the
// second ramp (a handle, a stem, a bone) and `A`-`F` the third.

const PICKAXE: [&str; 16] = [
    "................",
    ".......455544...",
    "....4454333332..",
    "..44332...23432.",
    "..43.......d432.",
    "..2.......ec432.",
    ".........ec..42.",
    "........ec...42.",
    ".......ec....42.",
    "......ec....42..",
    ".....ec.....42..",
    "....ec......42..",
    "...ec......32...",
    "..ec......221...",
    ".dc.............",
    "................",
];

const AXE: [&str; 16] = [
    "................",
    "...454..........",
    "..45544......ec.",
    "..554433....ec..",
    "..554433322ec...",
    "..55443332ec....",
    "..54433..ec.....",
    "..5443..ec......",
    "...432.ec.......",
    "......ec........",
    ".....ec.........",
    "....ec..........",
    "...ec...........",
    "..ec............",
    ".dc.............",
    "................",
];

const SHOVEL: [&str; 16] = [
    "................",
    "...........454..",
    ".........455443.",
    "........4554432.",
    ".......45544332.",
    ".......5544332..",
    ".......543332...",
    "........4322....",
    ".......ec.......",
    "......ec........",
    ".....ec.........",
    "....ec..........",
    "...ec...........",
    "..ec............",
    ".dc.............",
    "................",
];

const SWORD: [&str; 16] = [
    "................",
    ".............54.",
    "............543.",
    "...........543..",
    "..........543...",
    ".........543....",
    "........543.....",
    "..2....543......",
    "..32..543.......",
    "...32543........",
    "....3432........",
    "...ec232........",
    "..ec..32........",
    ".432...1........",
    ".32.............",
    "................",
];

const HOE: [&str; 16] = [
    "................",
    "................",
    ".....45555443...",
    "....45333332ec..",
    "....432....ec...",
    "....32....ec....",
    ".........ec.....",
    "........ec......",
    ".......ec.......",
    "......ec........",
    ".....ec.........",
    "....ec..........",
    "...ec...........",
    "..ec............",
    ".dc.............",
    "................",
];

const STICK: [&str; 16] = [
    "................",
    "................",
    "............45..",
    "...........453..",
    "..........432...",
    "....4....432....",
    ".....3..432.....",
    "......2432......",
    "......432.......",
    ".....432........",
    "....432.........",
    "...432..........",
    "..432...........",
    "..32............",
    "................",
    "................",
];

const INGOT: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    ".....55555555...",
    "....5555555554..",
    "...45555555543..",
    "..3333333333322.",
    ".33333333333322.",
    ".33333333333221.",
    ".22222222222211.",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const GEM: [&str; 16] = [
    "................",
    "..........45....",
    ".........4554...",
    "........45543...",
    ".......455433...",
    "......4554332...",
    ".....4554332....",
    "....4554332.....",
    "...45543322.4...",
    "...4543322.454..",
    "...443322.4432..",
    "....3221..3322..",
    ".....21....21...",
    "................",
    "................",
    "................",
];

const DUST: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    ".......44.......",
    "......4543......",
    ".....454332.....",
    "....45433322....",
    "...4543333221...",
    "..454333322221..",
    "..322222221111..",
    "................",
    "................",
];

const BERRIES: [&str; 16] = [
    "................",
    "..........DE....",
    ".........DEEC...",
    ".......bbCDC....",
    "......b..b......",
    ".....b....b.....",
    "...454....454...",
    "..45433..45433..",
    "..43332..43332..",
    "..333224543322..",
    "...2224543322...",
    "......43332.....",
    "......33322.....",
    ".......222......",
    "................",
    "................",
];

const RAW_MEAT: [&str; 16] = [
    "................",
    "................",
    "........4554....",
    "......4455543...",
    ".....44554DD32..",
    "....4455DDE432..",
    "....45DDE44332..",
    "....4DE4443322..",
    "....344433322...",
    "...e.3333222....",
    "..edc.2222......",
    ".edc............",
    ".dc.............",
    "................",
    "................",
    "................",
];

const COOKED_MEAT: [&str; 16] = [
    "................",
    "................",
    "........4554....",
    "......4451543...",
    ".....445144312..",
    "....4451444132..",
    "....4514441332..",
    "....4144413322..",
    "....344413322...",
    "...e.3333222....",
    "..edc.2222......",
    ".edc............",
    ".dc.............",
    "................",
    "................",
    "................",
];

const CUBE: [&str; 16] = [
    "................",
    "......4444......",
    "....44555544....",
    "..445555555544..",
    ".45555555555554.",
    ".34455555555442.",
    ".33344555544222.",
    ".33333445522222.",
    ".33333332222222.",
    ".33333332222222.",
    ".33333332222222.",
    ".33333332222222.",
    "..3333332222221.",
    "....33332222....",
    "......3322......",
    "................",
];

/// A rough lump: a blob with flat facets, lit from the top-left. The shape
/// depends on `seed`, so coal and raw iron do not look alike.
fn lump(img: &mut Img, r: &Ramp, seed: u32) {
    let facets: Vec<(f32, f32)> = (0..6)
        .map(|k| (4.5 + rnd(k, 0, seed) * 7.0, 5.0 + rnd(k, 1, seed) * 6.5))
        .collect();
    let mut mask = Map::new(0);
    for y in 1..S - 1 {
        for x in 1..S - 1 {
            let (dx, dy) = (x as f32 - 7.5, y as f32 - 8.0);
            // the outline is an uneven seven-sided shape
            let turn = (dy.atan2(dx) / TAU + 1.0).fract() * 7.0;
            let (k, t) = (turn as i32, turn.fract());
            let corner = |k: i32| 3.9 + 2.3 * rnd(k % 7, 2, seed);
            let edge = corner(k) + (corner(k + 1) - corner(k)) * t;
            if (dx / 1.2).hypot(dy / 0.95) > edge {
                continue;
            }
            let hit = cell(x, y, &facets);
            let tone = [3, 2, 4, 3, 2, 4][hit.i];
            let light = -(dx + dy * 1.2) / 6.0;
            let mut step = tone;
            if light > 0.35 {
                step += 1;
            }
            if light < -0.4 {
                step -= 1;
            }
            mask.set(x, y, step.clamp(1, 4));
        }
    }
    mask.despeckle();
    for y in 0..S {
        for x in 0..S {
            let v = mask.get(x, y);
            if v == 0 || !inside(x, y) {
                continue;
            }
            // rim light where the top-left of the lump meets the air
            let rim = mask.get(x - 1, y) == 0 || mask.get(x, y - 1) == 0;
            let under = mask.get(x + 1, y) == 0 || mask.get(x, y + 1) == 0;
            img.set(x, y, r.px(if rim && !under { 5 } else { v }));
        }
    }
}

/// Draw a one-pixel outline around a sprite. Each outline pixel is a dark,
/// cool version of the colour next to it, so the outline follows the
/// material instead of being flat black.
fn outline(img: &mut Img) {
    let src = img.clone();
    for y in 0..S {
        for x in 0..S {
            if src.get(x, y)[3] != 0 {
                continue;
            }
            let mut darkest: Option<Rgb> = None;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if !inside(nx, ny) || src.get(nx, ny)[3] == 0 {
                    continue;
                }
                let c = to_rgb(src.get(nx, ny));
                let dark = mix(scale(c, 0.36), with_luma(COOL, luma(c) * 0.3), 0.35);
                if darkest.is_none_or(|d| luma(dark) < luma(d)) {
                    darkest = Some(dark);
                }
            }
            if let Some(c) = darkest {
                let c = to_u8(with_luma(c, luma(c).min(0.22)));
                img.set(x, y, [c[0], c[1], c[2], 255]);
            }
        }
    }
}

/// A 16×16 sprite with transparent background for an item that is not a block.
pub fn item_sprite(item: &ItemView) -> Rgba {
    let main = if item.light > 0 {
        Ramp::glow(item.color)
    } else if luma(item.color) > 0.75 {
        // pale metal needs room above its base colour for a highlight
        Ramp::with(scale(item.color, 0.9), 1.25)
    } else {
        Ramp::with(item.color, 1.1)
    };
    let wood = Ramp::new([0.50, 0.36, 0.22]);
    let seed = hash(
        (item.color[0] * 255.0) as i32,
        (item.color[1] * 255.0) as i32,
        (item.color[2] * 255.0) as u32,
    );
    let mut img = Img::new(CLEAR);
    let mut sparkles: &[(i32, i32)] = &[];
    match item.icon {
        Icon::Tool => match item.tool {
            Some(tool) => {
                let rows = match tool.kind {
                    ToolKind::Pickaxe => &PICKAXE,
                    ToolKind::Axe => &AXE,
                    ToolKind::Shovel => &SHOVEL,
                    ToolKind::Sword => &SWORD,
                    _ => &HOE,
                };
                art(&mut img, 0, 0, rows, &[&main, &wood]);
            }
            None => lump(&mut img, &main, seed),
        },
        Icon::Stick => art(&mut img, 0, 0, &STICK, &[&main]),
        Icon::Lump => lump(&mut img, &main, seed),
        Icon::Ingot => art(&mut img, 0, 0, &INGOT, &[&main]),
        Icon::Gem => {
            art(&mut img, 0, 0, &GEM, &[&main]);
            if item.light > 0 {
                sparkles = &[(12, 3), (3, 6)];
            }
        }
        Icon::Dust => {
            art(&mut img, 0, 0, &DUST, &[&main]);
            sparkles = &[(4, 4), (11, 3), (13, 8)];
        }
        Icon::Berries => {
            let leaf = Ramp::new([0.30, 0.56, 0.24]);
            art(&mut img, 0, 0, &BERRIES, &[&main, &wood, &leaf]);
        }
        Icon::Meat => {
            // Raw and cooked meat share an icon. Cooked meat is brown: it
            // has much less blue than green.
            let cooked = item.color[2] < item.color[1] * 0.8;
            let bone = Ramp::with([0.90, 0.87, 0.78], 0.8);
            let fat = Ramp::with([0.96, 0.86, 0.80], 0.6);
            let rows = if cooked { &COOKED_MEAT } else { &RAW_MEAT };
            art(&mut img, 0, 0, rows, &[&main, &bone, &fat]);
        }
        _ => {
            if item.block.is_some() {
                art(&mut img, 0, 0, &CUBE, &[&main]);
            } else {
                lump(&mut img, &main, seed);
            }
        }
    }
    outline(&mut img);
    // Sparkles go on after the outline, so they stay small: a bright centre
    // with four dimmer arms.
    for &(x, y) in sparkles {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if inside(x + dx, y + dy) && img.get(x + dx, y + dy)[3] == 0 {
                img.set(x + dx, y + dy, main.pxa(4, 200));
            }
        }
        img.set(x, y, [255, 252, 240, 255]);
    }
    img.rgba()
}
