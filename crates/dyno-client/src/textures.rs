//! Procedural 32×32 hand-painted textures for every block face and item sprite.
//!
//! Nothing here is loaded from disk: each texture is painted by code, so the
//! game ships no image files. The painters share one small toolkit:
//!
//! - [`Surf`]: a surface under paint. Every pixel has a flat colour, a
//!   height and a painted light value. [`Surf::render`] lights the heights
//!   from the top-left, so a raised stone gets its bevel highlight and its
//!   shadow without the painter drawing them.
//! - [`tone`]: one colour at any light value. Shadows lean towards a cool
//!   blue-violet and highlights towards a warm gold, which gives every
//!   material the same ambient light. Values are rounded to a few bands, so
//!   shading falls into broad brush shapes and not into smooth gradients.
//! - Tileable noise and tileable Voronoi cells. All coordinates wrap at the
//!   tile edge, so a wall of one block shows no seams.
//! - Brush shapes: tapered strokes, discs, boxes and polygons.
//!
//! Light comes from the top-left in every texture.
#![allow(clippy::needless_range_loop)] // pixel loops read better with indices

use dyno_world as w;
use dyno_world::BLOCKS;
use dyno_world::content::{ArmorType, Gear, Icon, ItemView, Slot, ToolKind, WeaponKind};
use std::f32::consts::{PI, TAU};

pub const TILE: usize = 32;
pub type Rgba = [u8; TILE * TILE * 4];

/// Tile size as a signed number, for wrap-around arithmetic.
const S: i32 = TILE as i32;
const SF: f32 = TILE as f32;
const N: usize = TILE * TILE;
/// Turns a pair of offsets by 45 degrees.
const DIAG: f32 = std::f32::consts::FRAC_1_SQRT_2;

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
/// What a painter sets for one pixel: colour, height and light value.
type Dab = (Rgb, f32, f32);
const CLEAR: Px = [0, 0, 0, 0];

const IRON: Rgb = [0.44, 0.45, 0.51];
const DARK_IRON: Rgb = [0.25, 0.25, 0.31];
const GOLD: Rgb = [0.93, 0.71, 0.24];
const HANDLE: Rgb = [0.52, 0.36, 0.21];

pub struct Textures {
    /// every texture, as an index into this list
    pub layers: Vec<Rgba>,
    /// per block ID: layer for [top, side, bottom]
    pub block_faces: Vec<[u16; 3]>,
    /// layers that animate: (first layer, frame count, seconds per frame)
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

// ---------------------------------------------------------------- colour

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

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

fn px(c: Rgb, a: u8) -> Px {
    let c = to_u8(c);
    [c[0], c[1], c[2], a]
}

/// The shadow tint and the highlight tint shared by every material.
const COOL: Rgb = [0.30, 0.24, 0.66];
const WARM: Rgb = [1.0, 0.88, 0.56];

/// A colour at light value `v`: 0 is the colour itself, -1 a deep shadow
/// and +1 a strong highlight.
fn tone(c: Rgb, v: f32) -> Rgb {
    let l = luma(c);
    if v < 0.0 {
        let t = (-v).min(1.5);
        let target = l * (1.0 - 0.60 * t).max(0.10);
        let d = saturate(with_luma(c, target), 1.0 + 0.22 * t);
        mix(d, with_luma(COOL, target), (0.30 * t).min(0.5))
    } else {
        let t = v.min(1.5);
        let target = l + t * (0.07 + 0.30 * l).min((0.97 - l).max(0.0));
        let d = with_luma(c, target);
        mix(d, with_luma(WARM, target), (0.24 * t).min(0.4))
    }
}

/// A slightly different colour for one stone or board among many: `r` in
/// 0..1 picks how far it leans cool or warm.
fn vary(c: Rgb, r: f32, amount: f32) -> Rgb {
    let l = luma(c);
    let to = if r < 0.5 {
        with_luma([0.48, 0.52, 0.92], l)
    } else {
        with_luma([0.96, 0.70, 0.40], l)
    };
    mix(c, to, amount * (r - 0.5).abs() * 2.0)
}

/// A point on a colour gradient. `stops` are in rising order.
fn grad(stops: &[(f32, Rgb)], t: f32) -> Rgb {
    let mut last = stops[0];
    for &s in stops {
        if t <= s.0 {
            let span = (s.0 - last.0).max(1e-4);
            return mix(last.1, s.1, ((t - last.0) / span).clamp(0.0, 1.0));
        }
        last = s;
    }
    last.1
}

/// Colours of fire, from soot (0) to the white-hot core (1).
fn fire(heat: f32) -> Rgb {
    let heat = (heat.clamp(0.0, 1.0) * 7.0).round() / 7.0;
    grad(
        &[
            (0.0, [0.15, 0.09, 0.12]),
            (0.22, [0.46, 0.13, 0.11]),
            (0.45, [0.82, 0.29, 0.11]),
            (0.66, [1.0, 0.54, 0.19]),
            (0.85, [1.0, 0.79, 0.36]),
            (1.0, [1.0, 0.95, 0.72]),
        ],
        heat,
    )
}

// ---------------------------------------------------------------- noise

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

/// Smooth noise that tiles over the tile, with `cx` × `cy` lattice cells
/// across it. Fewer cells give larger blotches.
fn tnoise(x: f32, y: f32, cx: i32, cy: i32, seed: u32) -> f32 {
    let fx = x * cx as f32 / SF;
    let fy = y * cy as f32 / SF;
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

/// Two octaves of [`pnoise`]: broad blotches with a softer, finer layer.
fn fbm(x: i32, y: i32, cells: i32, seed: u32) -> f32 {
    0.68 * pnoise(x, y, cells, cells, seed) + 0.32 * pnoise(x, y, cells * 2, cells * 2, seed ^ 0x51)
}

/// Signed shortest offset from `b` to `a` on the wrapping tile.
fn wrap_delta(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(SF);
    if d > SF / 2.0 { d - SF } else { d }
}

/// A jittered `gx` × `gy` grid of points that tiles. `jitter` 0 is a plain
/// grid and 1 lets a point move anywhere in its grid cell.
fn sites(gx: i32, gy: i32, jitter: f32, seed: u32) -> Vec<(f32, f32)> {
    let (cw, ch) = (SF / gx as f32, SF / gy as f32);
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

// ---------------------------------------------------------------- canvas

/// One finished picture. Coordinates wrap.
#[derive(Clone)]
struct Img(Vec<Px>);

fn at(x: i32, y: i32) -> usize {
    (y.rem_euclid(S) * S + x.rem_euclid(S)) as usize
}

fn inside(x: i32, y: i32) -> bool {
    (0..S).contains(&x) && (0..S).contains(&y)
}

impl Img {
    fn new(c: Px) -> Img {
        Img(vec![c; N])
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
        let mut out = [0u8; N * 4];
        for (o, p) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.0.iter()) {
            *o = *p;
        }
        out
    }

    /// Pull the average colour towards `target`, so that a block keeps the
    /// colour that the map and the particles use for it.
    fn recenter(mut self, target: Rgb, amount: f32) -> Img {
        let mut sum = [0f32; 3];
        let mut n = 0f32;
        for p in self.0.iter().filter(|p| p[3] > 0) {
            let c = to_rgb(*p);
            sum = [sum[0] + c[0], sum[1] + c[1], sum[2] + c[2]];
            n += 1.0;
        }
        if n == 0.0 {
            return self;
        }
        let mut gain = [1f32; 3];
        for k in 0..3 {
            let mean = (sum[k] / n).max(0.02);
            gain[k] = (1.0 + (target[k] / mean - 1.0) * amount).clamp(0.7, 1.4);
        }
        for p in self.0.iter_mut() {
            let c = to_rgb(*p);
            *p = px([c[0] * gain[0], c[1] * gain[1], c[2] * gain[2]], p[3]);
        }
        self
    }
}

/// A surface under paint: colour, height and light value per pixel.
#[derive(Clone)]
struct Surf {
    c: Vec<Rgb>,
    h: Vec<f32>,
    v: Vec<f32>,
    a: Vec<u8>,
    /// pixels that give off light keep their colour as painted
    raw: Vec<bool>,
    /// item sprites clip at the tile edge; block faces wrap around it
    clip: bool,
    /// draw only what falls outside the tile and wraps back in
    spill: bool,
    /// number of light bands per unit of value
    bands: f32,
}

impl Surf {
    fn new(c: Rgb) -> Surf {
        Surf {
            c: vec![c; N],
            h: vec![0.0; N],
            v: vec![0.0; N],
            a: vec![255; N],
            raw: vec![false; N],
            clip: false,
            spill: false,
            bands: 7.0,
        }
    }

    /// An empty, see-through surface for an item sprite.
    fn sprite() -> Surf {
        let mut s = Surf::new([0.0; 3]);
        s.a = vec![0; N];
        s.clip = true;
        s
    }

    fn from_fn(f: impl Fn(i32, i32) -> Dab) -> Surf {
        let mut s = Surf::new([0.0; 3]);
        for y in 0..S {
            for x in 0..S {
                s.put(x, y, f(x, y));
            }
        }
        s
    }

    fn put(&mut self, x: i32, y: i32, dab: Dab) {
        let within = inside(x, y);
        if (self.clip && !within) || (self.spill && within) {
            return;
        }
        let i = at(x, y);
        self.c[i] = dab.0;
        self.h[i] = dab.1;
        self.v[i] = dab.2;
        self.a[i] = 255;
        self.raw[i] = false;
    }

    /// A pixel that shines: its colour is used as it is.
    fn glow(&mut self, x: i32, y: i32, c: Rgb) {
        if self.clip && !inside(x, y) {
            return;
        }
        let i = at(x, y);
        self.c[i] = c;
        self.a[i] = 255;
        self.raw[i] = true;
    }

    /// Lighten or darken what is already painted.
    fn shade(&mut self, x: i32, y: i32, dv: f32) {
        if self.clip && !inside(x, y) {
            return;
        }
        self.v[at(x, y)] += dv;
    }

    /// Tint what is already painted.
    fn tint(&mut self, x: i32, y: i32, c: Rgb, t: f32) {
        let i = at(x, y);
        self.c[i] = mix(self.c[i], c, t);
    }

    fn height(&self, x: i32, y: i32) -> f32 {
        self.h[at(x, y)]
    }

    /// Paint every pixel for which `f` returns something. `f` gets the pixel
    /// centre.
    fn paint(&mut self, f: impl Fn(f32, f32) -> Option<Dab>) {
        for y in 0..S {
            for x in 0..S {
                if let Some(d) = f(x as f32 + 0.5, y as f32 + 0.5) {
                    self.put(x, y, d);
                }
            }
        }
    }

    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, f: impl Fn(i32, i32) -> Dab) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.put(x, y, f(x, y));
            }
        }
    }

    /// A brush stroke from `a` to `b`. `r` is its half-width along the way
    /// (0 at `a`, 1 at `b`). `f` gets the place along the stroke and the
    /// place across it, from -1 on the shaded side to 1 on the lit side.
    fn stroke(
        &mut self,
        a: (f32, f32),
        b: (f32, f32),
        r: impl Fn(f32) -> f32,
        f: impl Fn(f32, f32) -> Dab,
    ) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        let len = len2.sqrt();
        // the normal that points towards the light
        let (mut nx, mut ny) = (-dy / len, dx / len);
        if nx + ny > 0.0 {
            (nx, ny) = (-nx, -ny);
        }
        let rmax = (0..=8).map(|i| r(i as f32 / 8.0)).fold(0.0, f32::max) + 1.0;
        let (x0, x1) = (
            (a.0.min(b.0) - rmax).floor() as i32,
            (a.0.max(b.0) + rmax).ceil() as i32,
        );
        let (y0, y1) = (
            (a.1.min(b.1) - rmax).floor() as i32,
            (a.1.max(b.1) + rmax).ceil() as i32,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
                let t = ((px * dx + py * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (px - dx * t, py - dy * t);
                // a radius that is not a number is the tip of a shape whose
                // width is a power of a sine: nothing to draw there
                let rad = r(t);
                if rad.is_nan() || qx.hypot(qy) > rad {
                    continue;
                }
                let s = (qx * nx + qy * ny) / rad.max(1e-3);
                self.put(x, y, f(t, s));
            }
        }
    }

    /// A stroke through several points. `r` and `f` see the whole path as
    /// 0..1.
    fn path(&mut self, pts: &[(f32, f32)], r: impl Fn(f32) -> f32, f: impl Fn(f32, f32) -> Dab) {
        let n = (pts.len() - 1) as f32;
        for (k, pair) in pts.windows(2).enumerate() {
            let t0 = k as f32 / n;
            self.stroke(
                pair[0],
                pair[1],
                |t| r((t0 + t / n).min(1.0)),
                |t, s| f((t0 + t / n).min(1.0), s),
            );
        }
    }

    /// A disc. `f` gets the distance from the centre (0..1) and how far the
    /// pixel is towards the light (-1..1).
    fn disc(&mut self, cx: f32, cy: f32, r: f32, f: impl Fn(f32, f32) -> Dab) {
        let (x0, x1) = ((cx - r - 1.0).floor() as i32, (cx + r + 1.0).ceil() as i32);
        let (y0, y1) = ((cy - r - 1.0).floor() as i32, (cy + r + 1.0).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let d = dx.hypot(dy);
                if d <= r {
                    self.put(x, y, f(d / r, -(dx + dy) / (r * 1.414)));
                }
            }
        }
    }

    /// A convex polygon. `f` gets the pixel centre.
    fn poly(&mut self, pts: &[(f32, f32)], f: impl Fn(f32, f32) -> Dab) {
        for y in 0..S {
            for x in 0..S {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let mut sign = 0.0f32;
                let mut within = true;
                for k in 0..pts.len() {
                    let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
                    let cross = (b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0);
                    if cross * sign < 0.0 {
                        within = false;
                        break;
                    }
                    if cross != 0.0 {
                        sign = cross;
                    }
                }
                if within {
                    self.put(x, y, f(px, py));
                }
            }
        }
    }

    /// Light the surface from the top-left and turn it into a picture.
    /// `relief` is how strongly a step in height shows.
    fn render(&self, relief: f32) -> Img {
        let mut img = Img::new(CLEAR);
        for y in 0..S {
            for x in 0..S {
                let i = at(x, y);
                if self.a[i] == 0 {
                    continue;
                }
                if self.raw[i] {
                    img.0[i] = px(self.c[i], self.a[i]);
                    continue;
                }
                let h = self.h[i];
                let near = |dx: i32, dy: i32| {
                    let (nx, ny) = (x + dx, y + dy);
                    if (self.clip && !inside(nx, ny)) || self.a[at(nx, ny)] == 0 {
                        h
                    } else {
                        self.h[at(nx, ny)]
                    }
                };
                let edge = 2.0 * h - near(-1, 0) - near(0, -1) + 0.5 * (h - near(-2, -2));
                let v = self.v[i] + relief * edge.clamp(-1.6, 1.6);
                let v = (v * self.bands).round() / self.bands;
                img.0[i] = px(tone(self.c[i], v), self.a[i]);
            }
        }
        img
    }
}

/// A small domed stud: a rivet or a nail head.
fn rivet(s: &mut Surf, x: f32, y: f32, r: f32, c: Rgb) {
    s.disc(x, y, r + 0.7, |_, _| (scale(c, 0.5), 0.2, -0.6));
    s.disc(x, y, r, |d, lit| {
        (c, 1.0 + 0.4 * (1.0 - d), 0.15 + lit * 0.6)
    });
}

// ---------------------------------------------------------------- earth and stone

fn dirt_surf(base: Rgb) -> Surf {
    let clods = sites(4, 4, 0.9, 21);
    let mut s = Surf::from_fn(|x, y| {
        let hit = cell(x, y, &clods);
        let i = hit.i as i32;
        let edge = smooth((hit.d2 - hit.d1) / 4.0);
        let n = fbm(x, y, 3, 23) - 0.5;
        (
            vary(base, rnd(i, 1, 22), 0.10),
            edge * 0.16,
            (rnd(i, 0, 22) - 0.5) * 0.30 + n * 0.5 - (1.0 - edge) * 0.12,
        )
    });
    // crumbs: short dark flecks
    for k in 0..9 {
        let (x, y) = (rnd(k, 0, 25) * SF, rnd(k, 1, 25) * SF);
        let len = 1.0 + rnd(k, 2, 25) * 2.0;
        s.stroke(
            (x, y),
            (x + len, y + (rnd(k, 3, 25) - 0.5) * 1.5),
            |_| 0.6,
            |_, _| (base, -0.1, -0.55),
        );
    }
    // pebbles: pale stones, each with a dark rim
    let stone = mix(base, [0.72, 0.69, 0.66], 0.45);
    for (k, &(x, y)) in sites(3, 3, 1.0, 24).iter().enumerate().take(5) {
        let r = 1.0 + rnd(k as i32, 4, 25) * 1.1;
        s.disc(x, y, r + 0.8, |_, _| (base, 0.1, -0.4));
        s.disc(x, y, r, |d, lit| {
            (stone, 0.5 + 0.5 * (1.0 - d * d), 0.05 + lit * 0.35)
        });
    }
    s
}

fn grass_top_surf(base: Rgb) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 3, 31) - 0.5;
        (base, 0.0, -0.38 + n * 0.5)
    });
    let mut tufts = sites(6, 6, 1.0, 32);
    tufts.sort_by(|a, b| a.1.total_cmp(&b.1));
    let lush = saturate(mix(base, [0.20, 0.62, 0.30], 0.25), 1.1);
    let sunny = mix(base, [0.72, 0.80, 0.26], 0.30);
    let draw = |s: &mut Surf, k: i32, tx: f32, ty: f32| {
        let pick = rnd(k, 0, 33);
        let hue = if pick < 0.4 {
            lush
        } else if pick > 0.75 {
            sunny
        } else {
            base
        };
        let blades = 3 + (hash(k, 1, 33) % 2) as i32;
        let lift = (rnd(k, 20, 33) - 0.5) * 0.3;
        for j in 0..blades {
            let fan =
                (j as f32 - (blades - 1) as f32 / 2.0) * 0.6 + (rnd(k, 2 + j, 33) - 0.5) * 0.4;
            let len = 4.5 + rnd(k, 8 + j, 33) * 3.5;
            let tip = (tx + fan.sin() * len, ty - fan.cos() * len);
            s.stroke(
                (tx, ty),
                tip,
                |t| 1.3 * (1.0 - t) + 0.4,
                |t, lit| (hue, 0.0, -0.5 + 0.95 * t + lit * 0.12 + lift),
            );
        }
    };
    for (k, &(tx, ty)) in tufts.iter().enumerate() {
        draw(&mut s, k as i32, tx, ty);
    }
    // Blades at the top edge reach into the tile above, where they must
    // lie over the tufts of the bottom rows.
    s.spill = true;
    for (k, &(tx, ty)) in tufts.iter().enumerate() {
        draw(&mut s, k as i32, tx, ty);
    }
    s.spill = false;
    s
}

fn grass_side_surf(grass: Rgb, dirt: &Surf) -> Surf {
    let mut s = dirt.clone();
    let turf = |x: i32| {
        4.5 + 3.0 * tnoise(x as f32 + 0.5, 0.5, 4, 1, 41)
            + 2.0 * tnoise(x as f32 + 0.5, 0.5, 8, 1, 42)
    };
    for x in 0..S {
        let depth = turf(x);
        for y in 0..depth.ceil() as i32 {
            let t = y as f32 / depth;
            let lip = if y < 2 { 0.25 } else { 0.0 };
            s.put(x, y, (grass, 0.7, 0.30 + lip - 0.75 * t));
        }
    }
    // the fringe: blades that hang over the dirt
    let sunny = mix(grass, [0.72, 0.80, 0.26], 0.3);
    for k in 0..18 {
        let x = (k as f32 + rnd(k, 0, 43)) * SF / 18.0;
        let len = 5.0 + rnd(k, 1, 43) * 8.0;
        let lean = (rnd(k, 2, 43) - 0.5) * 5.0;
        let hue = if rnd(k, 3, 43) > 0.6 { sunny } else { grass };
        let wide = 1.1 + rnd(k, 4, 43) * 0.7;
        s.stroke(
            (x, 1.5),
            (x + lean, 1.5 + len),
            |t| wide * (1.0 - t) + 0.35,
            |t, lit| (hue, 0.7, 0.30 - 0.55 * t + lit * 0.28),
        );
    }
    // sunlit tips along the top
    for x in 0..S {
        if pnoise(x, 0, 10, 1, 44) > 0.55 {
            s.shade(x, 0, 0.35);
        }
    }
    // the shadow that the fringe throws on the dirt
    for y in 1..S {
        for x in 0..S {
            if s.height(x, y) > 0.5 {
                continue;
            }
            let over = [(0, -1), (0, -2), (-1, -1), (-1, -2)]
                .iter()
                .filter(|d| y + d.1 >= 0 && s.height(x + d.0, y + d.1) > 0.5)
                .count();
            s.shade(x, y, -0.16 * over as f32);
        }
    }
    s
}

/// A dark crack through `pts`. The lower lip catches the light.
fn crack(s: &mut Surf, pts: &[(f32, f32)], c: Rgb) {
    s.path(pts, |t| 0.3 + 0.25 * (t * PI).sin(), |_, _| (c, -0.2, -0.6));
}

/// Stone: broad plates like layered rock, some standing proud of the
/// others, and a few hairline cracks.
fn stone_surf(base: Rgb, seed: u32) -> Surf {
    let plates = sites(4, 5, 0.9, seed);
    let mut s = Surf::from_fn(|x, y| {
        let wx = (pnoise(x, y, 4, 4, seed + 1) - 0.5) * 5.0;
        let wy = (pnoise(x, y, 4, 4, seed + 2) - 0.5) * 5.0;
        let hit = nearest(x as f32 + 0.5 + wx, y as f32 + 0.5 + wy, &plates, 0.3, 1.7);
        let i = hit.i as i32;
        let lift = (rnd(i, 0, seed + 3) * 4.0).floor() / 4.0;
        let n = fbm(x, y, 3, seed + 4) - 0.5;
        (
            vary(base, rnd(i, 1, seed + 3), 0.14),
            lift * 0.30,
            (lift - 0.4) * 0.30 + n * 0.40,
        )
    });
    crack(&mut s, &[(4.0, 8.0), (6.5, 10.0), (10.0, 10.5)], base);
    crack(&mut s, &[(19.0, 22.0), (22.0, 24.0), (23.5, 27.0)], base);
    s
}

fn sand_surf(base: Rgb) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let (u, v) = (x as f32 / SF * TAU, y as f32 / SF * TAU);
        // wind ripples: wavy crests, each with a soft shadow under it
        let wob = 1.3 * (u + 0.6).sin() + 0.5 * (u * 2.0 + v).sin();
        let ripple = (v * 4.0 + wob).sin();
        let fade = pnoise(x, y, 3, 3, 93);
        let n = fbm(x, y, 3, 92) - 0.5;
        (base, 0.0, ripple * 0.20 * (0.4 + fade) + n * 0.25)
    });
    for k in 0..7 {
        let (x, y) = (rnd(k, 0, 94) * SF, rnd(k, 1, 94) * SF);
        s.disc(x, y, 0.9, |_, lit| {
            (scale(base, 0.9), 0.3, -0.2 + lit * 0.3)
        });
    }
    s.bands = 9.0;
    s
}

fn clay_surf(base: Rgb) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let (u, v) = (x as f32 / SF * TAU, y as f32 / SF * TAU);
        let wob = 0.9 * u.sin() + 0.4 * (u * 2.0 + 1.0).sin();
        let band = (v * 3.0 + wob).sin() * 0.6 + (v * 5.0 - wob * 1.3 + 1.7).sin() * 0.4;
        let fade = pnoise(x, y, 3, 4, 101);
        (base, 0.0, band * 0.28 * (0.35 + fade))
    });
    for k in 0..4 {
        let (x, y) = (rnd(k, 0, 102) * SF, rnd(k, 1, 102) * SF);
        s.disc(x, y, 1.1, |_, _| (base, -0.4, -0.45));
    }
    s
}

fn snow_surf(base: Rgb) -> Surf {
    // Snow is too bright for highlights, so its shape is all in the
    // shadows: soft wind-blown drifts with blue hollows.
    let mut s = Surf::from_fn(|x, y| {
        let n = 0.75 * pnoise(x, y, 3, 4, 111) + 0.25 * pnoise(x, y, 6, 6, 113);
        let hollow = smooth((0.5 - n) * 3.5 + 0.2);
        (base, 0.0, -0.30 * hollow)
    });
    for k in 0..6 {
        let (x, y) = ((rnd(k, 0, 112) * SF) as i32, (rnd(k, 1, 112) * SF) as i32);
        s.shade(x, y, 0.6);
    }
    s.bands = 9.0;
    s
}

/// Rounded stones with dark mortar between them. Mortar pixels have height
/// 0. `round` is how many pixels the edge of a stone takes to rise.
fn stones_surf(
    base: Rgb,
    sites: &[(f32, f32)],
    gap: f32,
    boxy: f32,
    round: f32,
    seed: u32,
) -> Surf {
    let mortar = saturate(scale(base, 0.8), 1.2);
    Surf::from_fn(|x, y| {
        let wx = (pnoise(x, y, 4, 4, seed ^ 0x11) - 0.5) * 2.4;
        let wy = (pnoise(x, y, 4, 4, seed ^ 0x22) - 0.5) * 2.4;
        let hit = nearest(x as f32 + 0.5 + wx, y as f32 + 0.5 + wy, sites, boxy, 1.0);
        let edge = hit.d2 - hit.d1 - gap;
        if edge < 0.0 {
            return (mortar, 0.0, -0.95);
        }
        let i = hit.i as i32;
        let dome = smooth(edge / round);
        let n = fbm(x, y, 4, seed ^ 0x33) - 0.5;
        (
            vary(base, rnd(i, 1, seed), 0.20),
            0.30 + 0.55 * dome,
            (rnd(i, 0, seed) - 0.5) * 0.50 + n * 0.30 - (1.0 - dome) * 0.12,
        )
    })
}

fn cobble_surf(base: Rgb) -> Surf {
    stones_surf(base, &sites(4, 4, 0.85, 51), 1.3, 0.15, 3.0, 52)
}

fn mossy_surf(stone: Rgb, moss: Rgb) -> Surf {
    let mut s = cobble_surf(stone);
    // Moss fills the gaps in some places and creeps onto the stones there.
    for y in 0..S {
        for x in 0..S {
            let n = 0.7 * pnoise(x, y, 3, 3, 191) + 0.3 * pnoise(x, y, 7, 7, 192);
            let h = s.height(x, y);
            let limit = if h == 0.0 {
                0.50
            } else if h < 0.6 {
                0.56
            } else {
                0.64
            };
            if n > limit {
                let deep = if h == 0.0 { -0.45 } else { 0.0 };
                s.put(x, y, (moss, h + 0.25, deep + (n - limit) * 2.2 - 0.15));
            }
        }
    }
    s
}

/// One block in a wall of courses.
struct Blk {
    row: i32,
    col: i32,
    /// pixels to the left, right, top and bottom edge of the block; the
    /// last column and the last row are the mortar joint
    l: i32,
    r: i32,
    t: i32,
    b: i32,
}

impl Blk {
    fn mortar(&self) -> bool {
        self.r == 0 || self.b == 0
    }

    /// Distance to the nearest edge of the face of the block.
    fn edge(&self) -> i32 {
        self.l.min(self.t).min(self.r - 1).min(self.b - 1)
    }
}

/// Find the block at a pixel. `heights` are the course heights and must add
/// up to the tile; `len` is the block length and must divide the tile. Each
/// course is shifted sideways by its own amount.
fn course(x: i32, y: i32, heights: &[i32], len: i32, seed: u32) -> Blk {
    let y = y.rem_euclid(S);
    let (mut row, mut top) = (0, 0);
    for (k, h) in heights.iter().enumerate() {
        if y < top + h {
            row = k as i32;
            break;
        }
        top += h;
    }
    let height = heights[row as usize];
    let shift = if row % 2 == 0 { 0 } else { len / 2 } + (hash(row, 0, seed) % 5) as i32 - 2;
    let bx = (x + shift).rem_euclid(S);
    Blk {
        row,
        col: bx / len,
        l: bx % len,
        r: len - 1 - bx % len,
        t: y - top,
        b: height - 1 - (y - top),
    }
}

/// A wall of bricks or cut stone.
fn masonry(
    base: Rgb,
    mortar: Rgb,
    heights: &[i32],
    len: i32,
    seed: u32,
    hue: f32,
    contrast: f32,
) -> Surf {
    Surf::from_fn(|x, y| {
        let b = course(x, y, heights, len, seed);
        if b.mortar() {
            return (mortar, 0.0, -0.25);
        }
        let h = hash(b.col, b.row, seed);
        let r = |k: i32| rnd(b.col * 7 + k, b.row, seed);
        // a chipped corner
        if b.l + b.t == 0 && h.is_multiple_of(3) || (b.r - 1) + (b.b - 1) == 0 && h % 4 == 1 {
            return (mortar, 0.0, -0.25);
        }
        let n = fbm(x + b.col * 11, y + b.row * 5, 4, seed ^ 0x66) - 0.5;
        // each block is a little lighter towards its top-left
        let slope = (b.r - b.l) as f32 / len as f32 * 0.12 + (b.b - b.t) as f32 * 0.02;
        (
            vary(base, r(1), hue),
            if b.edge() == 0 { 0.55 } else { 0.85 },
            (r(0) - 0.5) * contrast + n * 0.40 + slope,
        )
    })
}

fn brick_surf(base: Rgb) -> Surf {
    let mortar = [0.74, 0.68, 0.58];
    let mut s = masonry(base, mortar, &[8, 8, 8, 8], 16, 131, 0.30, 0.50);
    // the course above shades the mortar bed
    for y in 0..S {
        for x in 0..S {
            if s.height(x, y) == 0.0 && s.height(x, y - 1) > 0.0 {
                s.shade(x, y, -0.3);
            }
        }
    }
    s
}

fn stone_brick_surf(base: Rgb, seed: u32) -> Surf {
    let mortar = saturate(scale(base, 0.42), 1.3);
    masonry(base, mortar, &[11, 10, 11], 16, seed, 0.16, 0.40)
}

fn basalt_side_surf(base: Rgb) -> Surf {
    /// first column of each of the six pillars
    const EDGES: [i32; 7] = [0, 5, 11, 15, 21, 27, 32];
    Surf::from_fn(|x, y| {
        let k = EDGES.iter().rposition(|e| *e <= x).unwrap_or(0).min(5);
        let (start, end) = (EDGES[k], EDGES[k + 1]);
        let ki = k as i32;
        // each pillar is broken by a joint or two
        let joint = (hash(ki, 0, 141) % 32) as i32;
        let dy = (y - joint).rem_euclid(if k % 2 == 0 { 32 } else { 16 });
        if x == end - 1 || dy == 0 {
            return (base, 0.0, -0.9);
        }
        let across = (x - start) as f32 / (end - start - 1) as f32;
        let n = pnoise(x, y, 8, 3, 142) - 0.5;
        (
            vary(base, rnd(ki, 1, 141), 0.2),
            0.6,
            0.35 - across * 0.7 + n * 0.3 + (rnd(ki, 2, 141) - 0.5) * 0.4,
        )
    })
}

/// The top of basalt pillars: six-sided cells with dark gaps.
fn basalt_top_surf(base: Rgb) -> Surf {
    // a honeycomb: every other row of sites is shifted by half a cell
    let mut pillars = Vec::new();
    for j in 0..4 {
        for i in 0..4 {
            let shift = if j % 2 == 0 { 0.0 } else { 4.0 };
            let jx = (rnd(i, j, 143) - 0.5) * 2.0;
            pillars.push((i as f32 * 8.0 + shift + 2.0 + jx, j as f32 * 8.0 + 4.0));
        }
    }
    stones_surf(base, &pillars, 0.9, 0.0, 1.6, 144)
}

fn marble_surf(base: Rgb) -> Surf {
    let vein = [0.52, 0.54, 0.64];
    let mut s = Surf::from_fn(|x, y| {
        let cloud = fbm(x, y, 2, 152) - 0.5;
        // veins run where two layers of noise cross
        let a = (fbm(x, y, 2, 153) - 0.5 + (pnoise(x, y, 5, 5, 154) - 0.5) * 0.25).abs();
        let b = (fbm(x, y, 3, 155) - 0.5).abs();
        let line = smooth(1.0 - a / 0.028).max(smooth(1.0 - b / 0.016) * 0.6);
        (
            mix(base, vein, line * 0.40),
            0.0,
            cloud * 0.30 - 0.12 - line * 0.18,
        )
    });
    s.bands = 10.0;
    s
}

fn terracotta_surf(base: Rgb) -> Surf {
    Surf::from_fn(|x, y| {
        let v = y as f32 / SF * TAU;
        let band = (v * 2.0 + 0.5).sin() * 0.5 + (v * 5.0 + 2.0).sin() * 0.5;
        let fade = pnoise(x, y, 3, 6, 161);
        let n = fbm(x, y, 3, 162) - 0.5;
        (base, 0.0, band * 0.22 * (0.3 + fade) + n * 0.25)
    })
}

fn bedrock_surf(base: Rgb) -> Surf {
    let chunks = sites(4, 4, 1.0, 171);
    Surf::from_fn(|x, y| {
        let hit = nearest(x as f32 + 0.5, y as f32 + 0.5, &chunks, 0.7, 1.0);
        let i = hit.i as i32;
        let lift = rnd(i, 0, 172);
        if hit.d2 - hit.d1 < 0.8 {
            return (base, 0.0, -0.8);
        }
        // each chunk is a tilted facet
        let tilt = -(hit.dx + hit.dy) * 0.05;
        (
            vary(base, rnd(i, 1, 172), 0.25),
            0.3 + lift * 0.6,
            (lift - 0.5) * 1.1 + tilt + 0.35,
        )
    })
}

/// Stone with ore lumps set into it, in clusters.
fn ore_surf(stone: &Surf, stone_c: Rgb, ore: Rgb, seed: u32) -> Surf {
    let mut s = stone.clone();
    for (k, &(cx, cy)) in sites(3, 3, 0.9, seed).iter().enumerate() {
        let k = k as i32;
        if k % 4 == 3 {
            continue;
        }
        for j in 0..3 {
            let turn = rnd(k, j, seed + 1) * TAU;
            let far = if j == 0 {
                0.0
            } else {
                2.6 + rnd(k, j + 3, seed + 1)
            };
            let (x, y) = (cx + turn.cos() * far, cy + turn.sin() * far);
            let r = if j == 0 { 2.5 } else { 1.5 } + rnd(k, j + 6, seed + 1) * 0.6;
            s.disc(x, y, r + 0.9, |_, _| (stone_c, 0.15, -0.75));
            s.disc(x, y, r, |d, lit| {
                // flat facets, not a smooth ball
                let facet = (lit * 2.0).round() / 2.0;
                (ore, 0.6 + 0.3 * (1.0 - d), 0.15 + facet * 0.7)
            });
            s.shade((x - r * 0.4) as i32, (y - r * 0.4) as i32, 0.6);
        }
    }
    s
}

/// Emberstone: dark rock split by glowing veins. `pulse` 0..1 is how far the
/// glow has swelled.
fn ember_surf(pulse: f32) -> Surf {
    let rock = [0.27, 0.18, 0.19];
    let chunks = sites(3, 3, 1.0, 181);
    let mut s = Surf::new(rock);
    for y in 0..S {
        for x in 0..S {
            let wx = (pnoise(x, y, 4, 4, 184) - 0.5) * 3.0;
            let hit = nearest(x as f32 + 0.5 + wx, y as f32 + 0.5, &chunks, 0.2, 1.0);
            let gap = hit.d2 - hit.d1;
            // veins are hotter in some stretches than others
            let hot = 0.50 + 0.65 * pnoise(x, y, 3, 3, 182);
            let heat = (1.0 - gap / 3.6) * hot + pulse * 0.12;
            if heat > 0.36 {
                s.glow(x, y, fire(0.25 + (heat - 0.36) * 1.5));
            } else {
                let i = hit.i as i32;
                let rim = smooth((heat - 0.12) / 0.24);
                let tilt = -(hit.dx + hit.dy) * 0.045;
                s.put(
                    x,
                    y,
                    (
                        mix(rock, [0.62, 0.22, 0.12], rim * 0.7),
                        0.5,
                        (rnd(i, 0, 183) - 0.5) * 0.5 + tilt + rim * 0.3,
                    ),
                );
            }
        }
    }
    s
}

// ---------------------------------------------------------------- wood and leaves

/// Boards with long painted grain and dark gaps. `n` boards cross the tile;
/// they run sideways, or up and down when `upright`. With `joints`, each
/// board is cut once along its length.
fn boards(base: Rgb, n: i32, upright: bool, joints: bool, seed: u32) -> Surf {
    let wide = S / n;
    Surf::from_fn(|x, y| {
        let (along, across) = if upright { (y, x) } else { (x, y) };
        let (board, row) = (across / wide, across % wide);
        let col = vary(base, rnd(board, 2, seed), 0.16);
        let tone_v = (rnd(board, 1, seed) - 0.5) * 0.40;
        if row == wide - 1 {
            return (col, 0.0, -1.1);
        }
        let from_joint = (along - (hash(board, 0, seed) % 32) as i32).rem_euclid(S);
        if joints && from_joint == 0 {
            return (col, 0.0, -0.9);
        }
        // long grain: noise stretched along the board
        let (a, c) = (
            along as f32 + 0.5 + board as f32 * 13.0,
            across as f32 + 0.5,
        );
        let grain = tnoise(a, c, 2, 16, seed ^ 0x77) - 0.5;
        let fine = tnoise(a, c, 4, 32, seed ^ 0x78) - 0.5;
        let mut v = tone_v + grain * 0.75 + fine * 0.30;
        // nails either side of a joint
        let nail = joints
            && (from_joint == 2 || from_joint == S - 2)
            && (row == 1 || row == wide - 3)
            && wide >= 6;
        if nail {
            v = -0.8;
        }
        if row == wide - 2 {
            v -= 0.15;
        }
        (col, 0.6, v)
    })
}

/// A knot: dark rings in the grain.
fn knot(s: &mut Surf, x: f32, y: f32, r: f32, c: Rgb) {
    s.disc(x, y, r, |d, lit| {
        let ring = if d < 0.45 { -0.75 } else { -0.25 + lit * 0.2 };
        (c, 0.6, ring)
    });
}

fn planks_surf(base: Rgb, seed: u32) -> Surf {
    let mut s = boards(base, 4, false, true, seed);
    let k = seed as i32;
    knot(
        &mut s,
        4.0 + rnd(k, 0, 105) * 24.0,
        3.5 + 8.0 * (hash(k, 1, 105) % 4) as f32,
        1.6,
        base,
    );
    s
}

/// Bark: tall plates that stand proud of the dark grooves between them.
fn bark_surf(base: Rgb, seed: u32) -> Surf {
    // sites with x and y swapped, so that cells come out tall and not wide
    let plates: Vec<(f32, f32)> = sites(5, 3, 0.9, seed).iter().map(|p| (p.1, p.0)).collect();
    Surf::from_fn(|x, y| {
        let wx = (pnoise(x, y, 6, 3, seed + 1) - 0.5) * 2.5;
        let hit = nearest(y as f32 + 0.5, x as f32 + 0.5 + wx, &plates, 0.0, 4.2);
        let i = hit.i as i32;
        let edge = hit.d2 - hit.d1 - 0.55;
        if edge < 0.0 {
            return (base, 0.0, -0.7);
        }
        let rise = smooth(edge / 1.6);
        let streak = pnoise(x, y, 10, 2, seed + 2) - 0.5;
        (
            vary(base, rnd(i, 1, seed), 0.18),
            0.3 + 0.5 * rise,
            (rnd(i, 0, seed) - 0.5) * 0.35 + streak * 0.35 - (1.0 - rise) * 0.2 + 0.1,
        )
    })
}

/// The cut end of a log: rings inside a rim of bark.
fn log_top_surf(bark: Rgb, wood: Rgb, seed: u32) -> Surf {
    Surf::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
        let round = dx.hypot(dy);
        let square = dx.abs().max(dy.abs());
        let d = round * 0.45 + square * 0.55;
        if d > 13.3 {
            let n = pnoise(x, y, 8, 8, seed + 1) - 0.5;
            return (bark, 0.9, -0.25 + n * 0.5);
        }
        if d > 12.3 {
            return (bark, 0.3, -0.95);
        }
        let wobble = (pnoise(x, y, 4, 4, seed) - 0.5) * 2.2;
        let ring = ((d + wobble) * 1.25).sin();
        let heart = smooth(1.0 - d / 5.0);
        (
            mix(wood, scale(wood, 0.8), heart),
            0.5,
            ring * 0.32 + 0.1 - heart * 0.2,
        )
    })
}

fn birch_bark_surf(base: Rgb) -> Surf {
    let dark = [0.25, 0.22, 0.24];
    let mut s = Surf::from_fn(|x, y| {
        // soft upright streaks
        let n = 0.7 * pnoise(x, y, 5, 2, 81) + 0.3 * pnoise(x, y, 10, 3, 82) - 0.5;
        (base, 0.0, n * 0.55 - 0.1)
    });
    // dark dashes across the trunk
    for k in 0..11 {
        let (x, y) = (rnd(k, 0, 83) * SF, (k as f32 + rnd(k, 1, 83)) * SF / 11.0);
        let len = 2.5 + rnd(k, 2, 83) * 5.5;
        let thick = 0.5 + rnd(k, 3, 83) * 0.6;
        s.stroke(
            (x, y),
            (x + len, y + (rnd(k, 4, 83) - 0.5)),
            |t| thick * (t * PI).sin().max(0.5),
            |_, _| (dark, -0.3, 0.0),
        );
    }
    // a knot: a dark eye with a lid over it
    s.stroke(
        (13.0, 17.5),
        (18.0, 17.5),
        |t| 1.5 * (t * PI).sin(),
        |_, lit| (dark, -0.3, lit * 0.3),
    );
    s
}

/// How one kind of leaf is drawn.
struct Leaf {
    len: f32,
    wide: f32,
    /// leaves per row and column in each of the three layers
    grid: i32,
    /// needles all slant the same way
    needles: bool,
}

const BROAD: Leaf = Leaf {
    len: 10.5,
    wide: 3.3,
    grid: 4,
    needles: false,
};
const ROUND: Leaf = Leaf {
    len: 6.6,
    wide: 2.9,
    grid: 5,
    needles: false,
};
const NEEDLE: Leaf = Leaf {
    len: 8.5,
    wide: 1.25,
    grid: 7,
    needles: true,
};

/// Leaves in three layers: dim ones behind, lit ones in front. Each leaf
/// hangs tip down, has a lit and a shaded half, and throws a shadow on what
/// lies under it. Pixels that no leaf covers keep height -1.
fn leaves_surf(base: Rgb, seed: u32, kind: &Leaf) -> Surf {
    let deep = mix(base, [0.08, 0.30, 0.30], 0.35);
    let mut s = Surf::new(deep);
    for i in 0..N {
        s.h[i] = -1.0;
        s.v[i] = -1.1;
    }
    let sunny = mix(base, [0.74, 0.82, 0.25], 0.30);
    for layer in 0..3 {
        let lf = layer as f32;
        for (k, &(cx, cy)) in sites(kind.grid, kind.grid, 1.0, seed + layer)
            .iter()
            .enumerate()
        {
            let k = k as i32;
            let h = hash(k, 0, seed + 10 + layer);
            if layer > 0 && h.is_multiple_of(7) {
                continue;
            }
            let spread = if kind.needles { 1.3 } else { 2.3 };
            let turn = PI / 2.0 + (rnd(k, 1, seed + layer) - 0.5) * spread;
            let len = kind.len * (0.8 + rnd(k, 2, seed + layer) * 0.4);
            let (ox, oy) = (turn.cos() * len / 2.0, turn.sin() * len / 2.0);
            let (a, b) = ((cx - ox, cy - oy), (cx + ox, cy + oy));
            let wide = kind.wide;
            let shape = move |t: f32| wide * (t * PI).sin().powf(0.7);
            let col = match (h >> 8) % 4 {
                0 if layer == 2 => sunny,
                1 if layer < 2 => deep,
                _ => base,
            };
            let level = -0.55 + lf * 0.42;
            // the shadow under the leaf
            s.stroke(
                (a.0 + 0.9, a.1 + 1.0),
                (b.0 + 0.9, b.1 + 1.0),
                shape,
                |_, _| (deep, lf * 0.3 - 0.15, level - 0.75),
            );
            s.stroke(a, b, shape, |t, lit| {
                let v = if kind.needles {
                    lit * 0.25
                } else if lit.abs() < 0.16 {
                    -0.12
                } else if lit > 0.0 {
                    0.24
                } else {
                    -0.16
                };
                (col, lf * 0.3, level + v + t * 0.15)
            });
        }
    }
    s
}

/// Leaves with see-through holes where no leaf lies. A hole is kept only
/// as part of a cluster, and no more than `most` pixels are opened.
fn leaves_img(base: Rgb, seed: u32, kind: &Leaf, most: usize) -> Img {
    let s = leaves_surf(base, seed, kind);
    let mut img = s.render(0.5);
    let open = |x: i32, y: i32| s.height(x, y) < -0.5;
    let mut holes: Vec<(u32, i32, i32)> = Vec::new();
    for y in 0..S {
        for x in 0..S {
            let around = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter(|d| open(x + d.0, y + d.1))
                .count();
            if open(x, y) && around >= 2 {
                // whole 4×4 patches open or stay shut together
                holes.push((hash(x / 4, y / 4, seed), x, y));
            }
        }
    }
    holes.sort();
    for &(_, x, y) in holes.iter().take(most) {
        let p = img.get(x, y);
        img.set(x, y, [p[0], p[1], p[2], 0]);
    }
    img
}

// ---------------------------------------------------------------- cloth, glass, water

/// Wool: soft folds of cloth with a loose twill of yarn.
fn wool_surf(base: Rgb) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 3, 116) - 0.5;
        let stitch = match (x + 2 * (y / 2)) % 4 {
            0 => -0.13,
            2 => 0.10,
            _ => 0.0,
        };
        (base, 0.0, n * 0.45 + stitch)
    });
    s.bands = 10.0;
    s
}

fn glass_img(base: Rgb) -> Img {
    let mut img = Img::new(px(tone(base, 0.3), 46));
    for i in 0..S {
        for k in 0..2 {
            // the frame, and the faint shadow that it throws on the pane
            img.set(i, k, px(tone(base, 0.8 - k as f32 * 0.3), 255));
            img.set(k, i, px(tone(base, 0.8 - k as f32 * 0.3), 255));
            img.set(i, S - 1 - k, px(tone(base, -0.5 + k as f32 * 0.2), 255));
            img.set(S - 1 - k, i, px(tone(base, -0.5 + k as f32 * 0.2), 255));
        }
    }
    for i in 2..S - 2 {
        img.set(i, 2, px(tone(base, -0.3), 100));
        img.set(2, i, px(tone(base, -0.3), 100));
    }
    // glints: slanted strokes of light
    let glints: [(i32, i32, i32, u8); 4] = [
        (6, 15, 9, 230),
        (10, 18, 7, 180),
        (22, 25, 4, 150),
        (9, 12, 3, 120),
    ];
    for &(x, y, len, a) in &glints {
        for i in 0..len {
            let end = i == 0 || i + 1 == len;
            img.set(x + i, y - i, [250, 253, 255, if end { a / 2 } else { a }]);
            img.set(x + i + 1, y - i, [250, 253, 255, a / 2]);
        }
    }
    img
}

fn ice_img(base: Rgb) -> Img {
    let frost = [0.93, 0.97, 1.0];
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 2, 131) - 0.5;
        // a sheen that runs slantwise across the block
        let sheen = ((x - y) as f32 / SF * TAU + n * 3.0).sin();
        (base, 0.0, n * 0.5 + sheen * 0.16)
    });
    let cracks: [&[(f32, f32)]; 4] = [
        &[(4.0, 6.0), (9.0, 9.5), (13.5, 9.0), (18.0, 13.0)],
        &[(9.0, 9.5), (8.0, 14.5)],
        &[(19.0, 23.0), (24.5, 24.5), (29.0, 22.0)],
        &[(24.5, 24.5), (25.5, 29.5)],
    ];
    for line in cracks {
        s.path(
            line,
            |t| 0.3 + 0.35 * (t * PI).sin(),
            |_, _| (frost, 0.0, 0.5),
        );
    }
    for k in 0..5 {
        let (x, y) = ((rnd(k, 0, 134) * SF) as i32, (rnd(k, 1, 134) * SF) as i32);
        s.put(x, y, (frost, 0.0, 0.5));
        s.put(x + 1, y - 1, (frost, 0.0, 0.2));
    }
    s.bands = 10.0;
    let mut img = s.render(0.0);
    for (i, p) in img.0.iter_mut().enumerate() {
        p[3] = if s.c[i] == frost { 228 } else { 176 };
    }
    img
}

/// Eight frames of water: crossing swells with bright crest lines. Every
/// wave has a whole number of cycles over the tile and over the loop, so
/// each frame tiles and the animation repeats without a jump.
fn water_frames(base: Rgb) -> Vec<Img> {
    let foam = [0.80, 0.94, 1.0];
    let deep = mix(base, [0.10, 0.18, 0.55], 0.5);
    (0..8)
        .map(|f| {
            let ph = f as f32 / 8.0 * TAU;
            Img::from_fn(|x, y| {
                let (u, v) = ((x as f32 + 0.5) / SF * TAU, (y as f32 + 0.5) / SF * TAU);
                let wx = 0.9 * (v * 2.0 + ph).sin() + 0.5 * (u + v - ph).sin();
                let wy = 0.9 * (u * 2.0 - ph).cos();
                let a = (u * 2.0 + v + wx + ph).sin();
                let b = (v * 3.0 - u + wy - ph).sin();
                let n = a * 0.55 + b * 0.45;
                // crests run where the two swells cancel
                let crest = 1.0 - (n.abs() / 0.11).min(1.0);
                if crest > 0.45 {
                    px(mix(base, foam, 0.75), 208)
                } else if crest > 0.0 {
                    px(mix(base, foam, 0.35), 196)
                } else if n > 0.45 {
                    px(mix(base, foam, 0.14), 184)
                } else if n < -0.5 {
                    px(deep, 184)
                } else if n < 0.0 {
                    px(mix(base, deep, 0.45), 180)
                } else {
                    px(base, 180)
                }
            })
        })
        .collect()
}

// ---------------------------------------------------------------- made things

/// Grain for a piece of wood that is drawn by hand, along x or along y.
fn grain(x: i32, y: i32, upright: bool, seed: u32) -> f32 {
    let (a, c) = if upright { (y, x) } else { (x, y) };
    (tnoise(a as f32 + 0.5, c as f32 + 0.5, 2, 16, seed) - 0.5) * 0.7
}

/// A raised wooden frame `wide` pixels broad around the tile.
fn frame(s: &mut Surf, wide: i32, c: Rgb, seed: u32) {
    for y in 0..S {
        for x in 0..S {
            let (ex, ey) = (x.min(S - 1 - x), y.min(S - 1 - y));
            if ex.min(ey) >= wide {
                continue;
            }
            let mut v = grain(x, y, ex < ey, seed) + 0.05;
            if ex == ey {
                v = -0.6;
            }
            if x == 0 || y == 0 {
                v += 0.3;
            }
            if x == S - 1 || y == S - 1 {
                v -= 0.35;
            }
            s.put(x, y, (c, 0.95, v));
        }
    }
}

/// A flat iron strap between two rows or two columns, with a lit upper
/// edge and a shaded lower one.
fn strap(s: &mut Surf, from: i32, to: i32, upright: bool, c: Rgb) {
    for i in from..to {
        for j in 0..S {
            let (x, y) = if upright { (i, j) } else { (j, i) };
            let v = if i == from {
                0.45
            } else if i == to - 1 {
                -0.35
            } else {
                0.0
            };
            let n = pnoise(x, y, 6, 6, 207) - 0.5;
            s.put(x, y, (c, 1.0, v + n * 0.3));
        }
    }
}

fn table_top_surf(base: Rgb) -> Surf {
    let mut s = boards(base, 4, false, false, 211);
    for i in 0..N {
        s.h[i] *= 0.5;
    }
    // the scored 3×3 grid
    for i in 4..S - 4 {
        for g in [12, 20] {
            s.put(i, g, (base, 0.0, -0.75));
            s.put(g, i, (base, 0.0, -0.75));
        }
    }
    frame(&mut s, 4, scale(base, 0.74), 212);
    for (x, y) in [(2.0, 2.0), (30.0, 2.0), (2.0, 30.0), (30.0, 30.0)] {
        rivet(&mut s, x, y, 1.1, IRON);
    }
    s
}

fn table_side_surf(base: Rgb) -> Surf {
    let mut s = boards(base, 4, false, true, 213);
    let dark = scale(base, 0.74);
    for i in 0..N {
        s.h[i] *= 0.5;
    }
    // legs
    for y in 4..S {
        for x in (0..4).chain(28..32) {
            s.put(
                x,
                y,
                (
                    dark,
                    0.9,
                    grain(x, y, true, 214) + if x % 28 == 0 { 0.3 } else { 0.0 },
                ),
            );
        }
    }
    // the edge of the table top, and its shadow
    s.rect(0, 0, S, 4, |x, y| {
        (
            base,
            1.0,
            0.45 - y as f32 * 0.2 + grain(x, y, false, 215) * 0.5,
        )
    });
    for x in 0..S {
        s.shade(x, 4, -0.5);
        s.shade(x, 5, -0.25);
    }
    // a hammer hangs on the right
    s.stroke(
        (22.0, 13.0),
        (22.0, 27.0),
        |_| 1.2,
        |_, lit| (HANDLE, 1.0, 0.25 + lit * 0.4),
    );
    s.rect(18, 9, 27, 14, |x, y| {
        (
            IRON,
            1.2,
            0.5 - (y - 9) as f32 * 0.2 - if x == 26 { 0.3 } else { 0.0 },
        )
    });
    // and a saw on the left
    s.rect(6, 16, 16, 21, |x, y| {
        let tooth = y == 20 && x % 2 == 0;
        (
            IRON,
            1.0,
            if tooth {
                -0.9
            } else {
                0.6 - (y - 16) as f32 * 0.15
            },
        )
    });
    s.rect(6, 12, 10, 17, |x, y| {
        let hole = (x == 7 || x == 8) && (y == 14 || y == 13);
        (
            HANDLE,
            1.2,
            if hole {
                -1.0
            } else {
                0.3 - (x - 6) as f32 * 0.15
            },
        )
    });
    s
}

/// A stone slab with a raised rim: the top and bottom of the furnace.
fn slab_surf(base: Rgb, seed: u32) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 3, seed) - 0.5;
        let edge = x.min(y).min(S - 1 - x).min(S - 1 - y);
        let (h, v) = match edge {
            0..=2 => (1.0, 0.1),
            3 => (0.0, -0.8),
            _ => (0.5, -0.1),
        };
        (base, h, v + n * 0.5)
    });
    for i in 0..S {
        s.shade(i, 0, 0.3);
        s.shade(0, i, 0.3);
        s.shade(i, S - 1, -0.35);
        s.shade(S - 1, i, -0.35);
    }
    s
}

fn furnace_side_surf(base: Rgb) -> Surf {
    let mortar = saturate(scale(base, 0.45), 1.3);
    let mut s = masonry(base, mortar, &[8, 8, 8, 8], 16, 221, 0.15, 0.45);
    // distance outside the arched mouth; negative inside it
    let mouth = |x: f32, y: f32| {
        let dx = (x - 16.0).abs();
        if y > 17.0 {
            (dx - 8.0).max(y - 27.0)
        } else {
            dx.hypot(y - 17.0) - 8.0
        }
    };
    let coals = sites(5, 5, 1.0, 222);
    for y in 0..S {
        for x in 0..S {
            let d = mouth(x as f32 + 0.5, y as f32 + 0.5);
            if d > 2.0 {
                continue;
            }
            if d > 0.0 {
                // the iron frame
                s.put(x, y, (DARK_IRON, 1.1, 0.1 - (y as f32 - 14.0) * 0.02));
                continue;
            }
            let hit = cell(x, y, &coals);
            let low = (y as f32 - 18.0) / 9.0;
            let heat = low + (1.0 - (hit.d2 - hit.d1) / 2.5) * 0.45;
            if heat > 0.45 {
                s.glow(x, y, fire(heat * 0.9));
            } else {
                let lit = smooth(low + 0.5);
                s.put(
                    x,
                    y,
                    (
                        mix([0.12, 0.08, 0.09], [0.5, 0.16, 0.08], lit * 0.6),
                        0.2,
                        -0.3,
                    ),
                );
            }
            // grate bars in front of the fire
            if y >= 21 && x % 4 == 1 {
                s.put(x, y, ([0.10, 0.08, 0.10], 0.9, 0.0));
            }
        }
    }
    s
}

fn chest_side_surf(base: Rgb) -> Surf {
    let mut s = boards(base, 4, false, false, 231);
    // the seam under the lid
    for x in 0..S {
        s.put(x, 10, (base, 0.0, -1.2));
        s.shade(x, 11, 0.3);
    }
    strap(&mut s, 3, 7, true, IRON);
    strap(&mut s, 25, 29, true, IRON);
    for y in [3.0, 15.0, 27.0] {
        rivet(&mut s, 5.0, y, 1.0, IRON);
        rivet(&mut s, 27.0, y, 1.0, IRON);
    }
    for x in 0..S {
        s.put(x, 10, (base, 0.0, -1.2));
        s.shade(x, 0, 0.3);
        s.shade(x, S - 1, -0.4);
    }
    // the lock
    s.rect(13, 7, 19, 15, |x, y| {
        let rim = x == 13 || y == 7;
        (
            GOLD,
            1.3,
            if rim { 0.5 } else { 0.0 } - (y - 7) as f32 * 0.06,
        )
    });
    s.rect(15, 10, 17, 13, |_, _| ([0.10, 0.08, 0.08], 0.6, -0.5));
    s
}

fn chest_top_surf(base: Rgb) -> Surf {
    let mut s = boards(base, 4, false, false, 232);
    strap(&mut s, 3, 7, true, IRON);
    strap(&mut s, 25, 29, true, IRON);
    for y in [4.0, 16.0, 28.0] {
        rivet(&mut s, 5.0, y, 1.0, IRON);
        rivet(&mut s, 27.0, y, 1.0, IRON);
    }
    for i in 0..S {
        s.shade(i, 0, 0.3);
        s.shade(0, i, 0.3);
        s.shade(i, S - 1, -0.4);
        s.shade(S - 1, i, -0.4);
    }
    s
}

fn bed_top_surf(red: Rgb, wood: Rgb) -> Surf {
    let linen = [0.93, 0.90, 0.84];
    let mut s = Surf::from_fn(|x, y| {
        if y < 3 {
            return (wood, 1.0, grain(x, y, false, 241) + 0.2 - y as f32 * 0.2);
        }
        if y < 13 {
            // two pillows
            let px = if x < 16 { x } else { x - 15 };
            let edge = (px - 2).min(14 - px).min(y - 3).min(12 - y) as f32;
            if edge < 0.0 {
                return (linen, 0.2, -0.5);
            }
            let puff = smooth(edge / 3.0);
            return (linen, 0.4 + 0.5 * puff, -0.25 + puff * 0.3);
        }
        if y < 17 {
            // the sheet folded back over the blanket
            let v = if y == 13 { 0.2 } else { -0.1 * (y - 13) as f32 };
            return if y == 16 {
                (GOLD, 0.7, 0.1)
            } else {
                (linen, 0.8, v)
            };
        }
        // a quilted blanket
        let seam = (x + y) % 8 == 0 || (x - y).rem_euclid(8) == 0;
        let fold = (x as f32 / SF * TAU * 2.0 + y as f32 * 0.2).sin() * 0.15;
        if seam {
            (red, 0.3, -0.35 + fold)
        } else {
            (red, 0.55, 0.05 + fold)
        }
    });
    s.bands = 9.0;
    s
}

fn bed_side_surf(red: Rgb, wood: Rgb) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let fold = (x as f32 / SF * TAU * 3.0).sin();
        if y < 17 {
            let hem = 15.0 + fold * 0.8;
            if (y as f32) < hem - 1.0 {
                let lip = if y < 2 { 0.35 } else { 0.0 };
                return (red, 0.9, fold * 0.22 + lip - y as f32 * 0.012);
            }
            if (y as f32) < hem {
                return (GOLD, 0.9, fold * 0.2);
            }
            return (red, 0.9, fold * 0.2 - 0.3);
        }
        if y < 25 {
            let shadow = if y < 19 { -0.55 } else { 0.0 };
            return (wood, 0.6, grain(x, y, false, 242) + shadow);
        }
        if !(5..27).contains(&x) {
            (wood, 0.6, grain(x, y, true, 243) - 0.1)
        } else {
            ([0.20, 0.14, 0.13], 0.0, -0.6)
        }
    });
    s.bands = 9.0;
    s
}

/// The light of a lantern or a glowshroom: 0 is the dim edge of the glow
/// and 1 the white core.
fn glow_of(base: Rgb, t: f32) -> Rgb {
    let t = (t.clamp(0.0, 1.0) * 6.0).round() / 6.0;
    grad(
        &[
            (0.0, saturate(scale(base, 0.50), 1.5)),
            (0.45, saturate(scale(base, 0.85), 1.2)),
            (0.75, mix(base, [1.0, 1.0, 0.92], 0.35)),
            (1.0, mix(base, [1.0, 1.0, 0.95], 0.9)),
        ],
        t,
    )
}

fn lantern_side_surf(base: Rgb) -> Surf {
    let mut s = Surf::new(DARK_IRON);
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 17.5);
            // a flame-shaped core: narrower above the middle
            let d = (dx * dx * if dy < 0.0 { 1.6 } else { 0.9 } + dy * dy * 0.8).sqrt();
            s.glow(x, y, glow_of(base, 1.08 - d / 14.0));
        }
    }
    // the iron cage
    strap(&mut s, 0, 4, false, DARK_IRON);
    strap(&mut s, 28, 32, false, DARK_IRON);
    strap(&mut s, 0, 3, true, DARK_IRON);
    strap(&mut s, 29, 32, true, DARK_IRON);
    for y in 4..28 {
        s.put(15, y, (DARK_IRON, 1.0, 0.3));
        s.put(16, y, (DARK_IRON, 1.0, -0.3));
    }
    for (x, y) in [(2.0, 2.0), (30.0, 2.0), (2.0, 30.0), (30.0, 30.0)] {
        rivet(&mut s, x, y, 1.0, IRON);
    }
    s
}

fn lantern_top_surf(base: Rgb) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
        let d = dx.hypot(dy) * 0.5 + dx.abs().max(dy.abs()) * 0.5;
        let n = fbm(x, y, 4, 251) - 0.5;
        if d > 13.0 {
            (DARK_IRON, 1.0, n * 0.3)
        } else if d > 11.0 {
            (DARK_IRON, 0.3, -0.5)
        } else {
            // a low dome
            (DARK_IRON, 1.6 - d * 0.08, n * 0.3 + 0.1)
        }
    });
    // vents that let the light out
    for (x, y) in [(15, 5), (15, 25), (5, 15), (25, 15)] {
        for i in 0..2 {
            for j in 0..2 {
                s.glow(
                    x + i,
                    y + j,
                    glow_of(base, 0.5 + 0.3 * (i + j) as f32 / 2.0),
                );
            }
        }
    }
    // the ring to carry it by
    s.disc(15.5, 15.5, 4.6, |d, lit| {
        if d > 0.62 {
            (IRON, 2.0, lit * 0.6)
        } else {
            (DARK_IRON, 1.2, -0.3)
        }
    });
    s
}

/// Glowshroom: a dark fungus body with glowing spots. Each spot is
/// (centre x, centre y, radius).
fn shroom_surf(base: Rgb, spots: &[(f32, f32, f32)], seed: u32) -> Surf {
    let body = [0.15, 0.24, 0.34];
    let mut s = stones_surf(body, &sites(4, 4, 1.0, seed), 0.3, 0.0, 4.0, seed + 1);
    for y in 0..S {
        for x in 0..S {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // distance past the edge of the nearest spot
            let (edge, r) = spots
                .iter()
                .map(|s| (wrap_delta(px, s.0).hypot(wrap_delta(py, s.1)) - s.2, s.2))
                .fold((f32::MAX, 1.0), |a, b| if b.0 < a.0 { b } else { a });
            if edge < 0.0 {
                s.glow(x, y, glow_of(base, 0.35 + (-edge / r) * 0.8));
            } else if edge < 1.0 {
                s.put(x, y, (body, 0.0, -0.9));
            } else if edge < 3.5 {
                // the glow spills onto the body
                s.tint(x, y, base, 0.35 * (1.0 - (edge - 1.0) / 2.5));
                s.shade(x, y, 0.5 * (1.0 - (edge - 1.0) / 2.5));
            }
        }
    }
    s
}

/// The side of a torch post. The renderer shows the strip from column 13
/// to 18 over the full height, so the stick is drawn six pixels wide and
/// repeated across the tile: flame in the top rows, a charred band, then
/// wood with a binding.
fn torch_side_img(frame: i32) -> Img {
    let wood = [0.52, 0.38, 0.22];
    let sway = (frame as f32 / 4.0 * TAU).sin();
    Img::from_fn(|x, y| {
        let across = (x - 13).rem_euclid(6);
        let cx = across as f32 - 2.5;
        if y >= 14 {
            let v = [0.5, 0.3, 0.1, -0.05, -0.3, -0.6][across as usize];
            let c = match y {
                14 => tone(wood, v - 1.3),
                15 => tone(wood, v - 0.8),
                19 | 20 => tone(DARK_IRON, v + 0.2),
                _ => tone(wood, v + (pnoise(across, y, 4, 8, 241) - 0.5) * 0.4),
            };
            return px(c, 255);
        }
        // a flame that narrows and cools towards its tip, and leans as it
        // flickers
        let up = (13 - y) as f32 / 13.0;
        let lean = sway * up * 1.1 + ((frame * 7 + y) % 3 - 1) as f32 * 0.25 * up;
        let half = 3.4 - 2.3 * up;
        let side = (cx - lean).abs() / half;
        let heat = (1.05 - up * 0.55) * (1.0 - side * side * 0.75);
        px(fire(heat.max(0.24)), 255)
    })
}

fn torch_top_img() -> Img {
    Img::from_fn(|x, y| {
        let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
        let d = dx.abs().max(dy.abs()) * 0.5 + dx.hypot(dy) * 0.5;
        px(fire(1.0 - d / 5.5), 255)
    })
}

/// A bed of coals. Returns the colour of a glowing pixel, or what to paint
/// for a dark lump. `middle` is 1 at the heart of the fire and 0 at its edge.
fn coal(x: i32, y: i32, lumps: &[(f32, f32)], middle: f32, pulse: f32) -> Result<Rgb, Dab> {
    let hit = cell(x, y, lumps);
    let heat = (1.0 - (hit.d2 - hit.d1) / 3.2) * 0.62 + middle * 0.34 + pulse * 0.13;
    if heat > 0.47 {
        Ok(fire(0.28 + (heat - 0.47) * 1.5))
    } else {
        let rim = smooth((heat - 0.2) / 0.27);
        Err((
            mix([0.20, 0.15, 0.16], [0.55, 0.17, 0.10], rim * 0.8),
            0.5,
            -(hit.dx + hit.dy) * 0.07 + rim * 0.3,
        ))
    }
}

/// Ring of stones around a pile of embers. `pulse` 0..1 swells the glow.
fn hearth_top_surf(stone: Rgb, pulse: f32) -> Surf {
    let lumps = sites(5, 5, 1.0, 251);
    let mut s = Surf::new(stone);
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let d = dx.hypot(dy) * 0.55 + dx.abs().max(dy.abs()) * 0.45;
            if d < 9.6 {
                match coal(x, y, &lumps, 1.0 - d / 9.6, pulse) {
                    Ok(c) => s.glow(x, y, c),
                    Err(dab) => s.put(x, y, dab),
                }
                continue;
            }
            // twelve stones around the pit
            let turn = (dy.atan2(dx) / TAU + 1.02).fract() * 12.0;
            let k = turn as i32;
            let across = (turn.fract() - 0.5).abs() * 2.0;
            if across > 0.84 || d < 10.4 {
                s.put(x, y, (scale(stone, 0.5), 0.0, -0.9));
                continue;
            }
            // the inner edge of the rim glows from the fire
            let warm = smooth(1.0 - (d - 10.4) / 3.0) * (0.6 + 0.4 * pulse);
            let n = fbm(x, y, 4, 253) - 0.5;
            s.put(
                x,
                y,
                (
                    mix(
                        vary(stone, rnd(k, 1, 252), 0.2),
                        [1.0, 0.45, 0.15],
                        warm * 0.5,
                    ),
                    0.5 + 0.4 * smooth((1.0 - across) * 3.0),
                    (rnd(k, 0, 252) - 0.5) * 0.4 + n * 0.3 + warm * 0.5,
                ),
            );
        }
    }
    s
}

/// Stone blocks with joints that glow from the fire inside.
fn hearth_side_surf(stone: Rgb) -> Surf {
    let mortar = saturate(scale(stone, 0.40), 1.3);
    let mut s = masonry(stone, mortar, &[8, 8, 8, 8], 16, 254, 0.18, 0.45);
    let hot = |x: i32, y: i32| 0.6 * pnoise(x, y, 3, 3, 255) + 0.4 * pnoise(x, y, 6, 6, 256);
    let src = s.clone();
    for y in 0..S {
        for x in 0..S {
            let n = hot(x, y);
            if src.height(x, y) == 0.0 {
                if n > 0.54 {
                    s.glow(x, y, fire(0.30 + (n - 0.54) * 3.0));
                }
                continue;
            }
            // the glow spills onto the stone beside a hot joint
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|d| src.height(x + d.0, y + d.1) == 0.0);
            if near && n > 0.56 {
                s.tint(x, y, [1.0, 0.40, 0.12], 0.45);
                s.shade(x, y, 0.35);
            }
        }
    }
    s
}

fn berry_bush_surf(base: Rgb) -> Surf {
    let mut s = leaves_surf(base, 231, &BROAD);
    // fill the gaps: a bush is solid
    for i in 0..N {
        s.h[i] = s.h[i].max(-0.2);
    }
    let berry = [0.84, 0.13, 0.22];
    for (k, &(cx, cy)) in sites(3, 3, 0.9, 232).iter().enumerate() {
        let k = k as i32;
        if k % 3 == 2 {
            continue;
        }
        for j in 0..3 {
            let turn = rnd(k, j, 233) * TAU;
            let far = if j == 0 { 0.0 } else { 2.9 };
            let (x, y) = (cx + turn.cos() * far, cy + turn.sin() * far);
            s.disc(x, y, 2.6, |_, _| ([0.25, 0.05, 0.12], 0.6, -0.3));
            s.disc(x, y, 1.9, |d, lit| (berry, 0.9, lit * 0.55 - d * 0.15));
            s.glow((x - 0.9) as i32, (y - 0.9) as i32, [1.0, 0.86, 0.84]);
        }
    }
    s
}

/// Plain soft texture for blocks without a painter of their own.
fn generic_img(base: Rgb, alpha: f32) -> Img {
    let a = (alpha.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    let seed = hash((base[0] * 255.0) as i32, (base[1] * 255.0) as i32, 5);
    let mut img = stones_surf(base, &sites(3, 3, 1.0, seed), 0.0, 0.0, 4.0, seed ^ 9).render(0.3);
    for p in img.0.iter_mut() {
        p[3] = a;
    }
    img
}

// ---------------------------------------------------------------- town

fn plaster_surf(base: Rgb) -> Surf {
    let stain = [0.72, 0.62, 0.46];
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 3, 301) - 0.5;
        // subtle stains: damp patches, a little darker and browner
        let damp = smooth((fbm(x, y, 2, 302) - 0.56) * 6.0);
        (mix(base, stain, damp * 0.30), 0.0, n * 0.30 - damp * 0.10)
    });
    // trowel marks: short raised sweeps
    for k in 0..8 {
        let (x, y) = (rnd(k, 0, 303) * SF, rnd(k, 1, 303) * SF);
        let len = 4.0 + rnd(k, 2, 303) * 5.0;
        let bend = (rnd(k, 3, 303) - 0.5) * 3.0;
        s.path(
            &[(x, y), (x + len * 0.5, y + bend), (x + len, y + bend * 0.6)],
            |t| 0.9 * (t * PI).sin(),
            |_, lit| (base, 0.1, 0.10 + lit * 0.12),
        );
    }
    s.bands = 12.0;
    s
}

/// Roof tiles in overlapping rows. A tile hangs over the row below it:
/// `hang` gives how far, for a place across the tile from -1 to 1.
fn roof_surf(base: Rgb, wide: i32, hang: impl Fn(f32, f32) -> f32, hue: f32, seed: u32) -> Surf {
    const ROW: i32 = 8;
    Surf::from_fn(|x, y| {
        let row = y.div_euclid(ROW);
        let ly = y - row * ROW;
        // the tile of a row at this column: (column, pixels from its left)
        let tile = |r: i32| {
            let r = r.rem_euclid(S / ROW);
            let bx = (x + if r % 2 == 0 { 0 } else { wide / 2 }).rem_euclid(S);
            (r, bx / wide, bx % wide)
        };
        let (ur, ucol, ulx) = tile(row - 1);
        let across = (ulx as f32 + 0.5) / wide as f32 * 2.0 - 1.0;
        let reach = hang(across, rnd(ucol, ur, seed));
        let (r, col, lx, down) = if (ly as f32 + 0.5) < reach {
            (ur, ucol, ulx, ROW + ly)
        } else {
            let (r, col, lx) = tile(row);
            (r, col, lx, ly)
        };
        let k = col * 5 + r * 17;
        let slit = lx == 0 && down < ROW;
        // the lower lip of a tile catches the light
        let lip = !slit && {
            let (_, bcol, _) = tile(row);
            let hangs = down >= ROW;
            let below = (ly + 1) as f32 + 0.5;
            (hangs && below >= reach)
                || (!hangs && ly == ROW - 1 && {
                    // the tile of this row hangs on into the next one here?
                    let across = (lx as f32 + 0.5) / wide as f32 * 2.0 - 1.0;
                    hang(across, rnd(bcol, r, seed)) < 0.5
                })
        };
        let slope = down as f32 / (ROW as f32 * 1.5);
        let n = pnoise(x + col * 3, y, 5, 5, seed ^ 0x44) - 0.5;
        (
            vary(base, rnd(k, 1, seed), hue),
            if slit { 0.1 } else { 0.25 + 0.75 * slope },
            (rnd(k, 0, seed) - 0.5) * 0.55 + (slope - 0.5) * 0.3 + n * 0.3
                - if slit { 0.6 } else { 0.0 }
                + if lip { 0.3 } else { 0.0 },
        )
    })
}

/// Straw: long strands laid one way, three coats deep. `turn` is the
/// direction of the strands.
fn straw_surf(base: Rgb, turn: f32, seed: u32) -> Surf {
    let mut s = Surf::new(base);
    for i in 0..N {
        s.v[i] = -0.6;
    }
    let pale = mix(base, [1.0, 0.92, 0.62], 0.4);
    let dull = mix(base, [0.55, 0.42, 0.20], 0.4);
    for coat in 0..3 {
        for (k, &(cx, cy)) in sites(8, 5, 1.0, seed + coat).iter().enumerate() {
            let k = k as i32;
            let a = turn + (rnd(k, 0, seed + coat) - 0.5) * 0.35;
            let len = 7.0 + rnd(k, 1, seed + coat) * 9.0;
            let (ox, oy) = (a.cos() * len / 2.0, a.sin() * len / 2.0);
            let col = match hash(k, 2, seed + coat) % 4 {
                0 => pale,
                1 => dull,
                _ => base,
            };
            let level = -0.38 + coat as f32 * 0.28;
            s.stroke(
                (cx - ox, cy - oy),
                (cx + ox, cy + oy),
                |_| 0.75,
                |_, lit| (col, coat as f32 * 0.08, level + lit * 0.28),
            );
        }
    }
    s
}

fn thatch_surf(base: Rgb) -> Surf {
    let mut s = straw_surf(base, PI / 2.0 + 0.08, 311);
    // two courses of thatch: each ends in a ragged edge with a shadow
    // under it
    for x in 0..S {
        let rag = (pnoise(x, 0, 10, 1, 312) * 3.5) as i32;
        for (line, seed) in [(15, 0), (31, 1)] {
            let edge = line - (hash(x, seed, 313) % 2) as i32 - rag / 2;
            for y in edge..edge + 3 {
                s.shade(x, y, -0.42 + 0.14 * (y - edge) as f32);
            }
            s.shade(x, edge - 1, 0.25);
        }
    }
    s
}

fn hay_surf(base: Rgb, seed: u32) -> Surf {
    let mut s = straw_surf(base, 0.05, seed);
    // two bands of twine
    let twine = [0.56, 0.40, 0.22];
    for y in [8, 23] {
        for x in 0..S {
            let twist = (x + y) % 4;
            s.put(x, y, (twine, 0.5, if twist < 2 { 0.3 } else { -0.1 }));
            s.put(x, y + 1, (twine, 0.5, if twist < 2 { -0.2 } else { -0.5 }));
            s.shade(x, y + 2, -0.4);
        }
    }
    s
}

/// Dark oak, hewn by hand: long grain, deep checks and darker edges.
fn timber_surf(base: Rgb, upright: bool, seed: u32) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let (along, across) = if upright { (y, x) } else { (x, y) };
        let (a, c) = (along as f32 + 0.5, across as f32 + 0.5);
        let g = tnoise(a, c, 2, 12, seed) - 0.5;
        let fine = tnoise(a, c, 3, 32, seed + 1) - 0.5;
        // the beam is rounded off towards its edges
        let edge = across.min(S - 1 - across) as f32;
        let round = smooth(edge / 4.0);
        (
            base,
            0.3 * round,
            g * 0.9 + fine * 0.45 - (1.0 - round) * 0.45 + 0.1,
        )
    });
    for k in 0..3 {
        let across = 5.0 + rnd(k, 0, seed + 2) * 22.0;
        let along = rnd(k, 1, seed + 2) * SF;
        let len = 5.0 + rnd(k, 2, seed + 2) * 7.0;
        let (a, b) = if upright {
            ((across, along), (across + 0.5, along + len))
        } else {
            ((along, across), (along + len, across + 0.5))
        };
        s.stroke(a, b, |t| 0.5 * (t * PI).sin(), |_, _| (base, -0.1, -0.8));
    }
    s
}

fn flagstone_surf(base: Rgb, grass: Option<Rgb>, seed: u32) -> Surf {
    let mut s = stones_surf(
        scale(base, 1.08),
        &sites(3, 3, 0.85, seed),
        1.2,
        0.7,
        1.8,
        seed + 1,
    );
    // earth between the slabs, not deep shadow
    for i in 0..N {
        if s.h[i] == 0.0 {
            s.v[i] = -0.6;
        }
    }
    let Some(grass) = grass else {
        return s;
    };
    let grass = mix(scale(grass, 0.8), base, 0.25);
    // grass in the cracks, with a blade over a slab here and there
    for y in 0..S {
        for x in 0..S {
            let n = 0.6 * pnoise(x, y, 4, 4, seed + 2) + 0.4 * pnoise(x, y, 9, 9, seed + 3);
            if s.height(x, y) == 0.0 && n > 0.50 {
                s.put(x, y, (grass, 0.25, -0.5 + (n - 0.50) * 2.5));
            }
        }
    }
    for k in 0..14 {
        let (x, y) = (rnd(k, 0, seed + 4) * SF, rnd(k, 1, seed + 4) * SF);
        let (xi, yi) = (x as i32, y as i32);
        if s.c[at(xi, yi)] != grass {
            continue;
        }
        let fan = (rnd(k, 2, seed + 4) - 0.5) * 1.6;
        let len = 2.0 + rnd(k, 3, seed + 4) * 2.0;
        s.stroke(
            (x, y),
            (x + fan.sin() * len, y - fan.cos() * len),
            |_| 0.6,
            |t, _| (grass, 0.25, -0.2 + t * 0.5),
        );
    }
    s
}

/// A window of small panes in a lattice of lead.
fn window_img(base: Rgb) -> Img {
    let lead = [0.26, 0.27, 0.32];
    let mut s = Surf::from_fn(|x, y| {
        let edge = x.min(y).min(S - 1 - x).min(S - 1 - y);
        let (p, q) = ((x + y).div_euclid(8), (x - y).div_euclid(8));
        if edge < 2 || (x + y) % 8 == 0 || (x - y).rem_euclid(8) == 0 {
            return (lead, 1.0, if edge == 0 { -0.3 } else { 0.0 });
        }
        // each pane has its own tint, and is lighter towards its top
        let r = rnd(p, q, 321);
        let tint = if r < 0.2 {
            mix(base, [0.55, 0.80, 0.62], 0.4)
        } else if r > 0.85 {
            mix(base, [0.90, 0.80, 0.52], 0.35)
        } else {
            base
        };
        let up = ((x - y).rem_euclid(8) as f32 - (x + y).rem_euclid(8) as f32) / 8.0;
        (tint, 0.85, (r - 0.5) * 0.5 + up * 0.35)
    });
    // glints across a few panes
    for (x, y, len) in [(6, 12, 4), (20, 9, 3), (13, 25, 4), (25, 23, 3)] {
        for i in 0..len {
            s.put(x + i, y - i, ([0.95, 0.98, 1.0], 0.85, 0.3));
        }
    }
    let mut img = s.render(0.35);
    for (i, p) in img.0.iter_mut().enumerate() {
        if s.c[i] != lead {
            p[3] = 153;
        }
    }
    img
}

fn door_surf(base: Rgb) -> Surf {
    let mut s = boards(base, 4, true, false, 331);
    // iron bands with a rivet on every plank
    for (from, to) in [(4, 9), (22, 27)] {
        strap(&mut s, from, to, false, DARK_IRON);
        for k in 0..4 {
            rivet(
                &mut s,
                3.5 + k as f32 * 8.0,
                (from + to) as f32 / 2.0,
                1.1,
                IRON,
            );
        }
        for x in 0..S {
            s.shade(x, to, -0.45);
        }
    }
    // the ring handle and its mount
    s.disc(25.0, 13.0, 1.7, |_, lit| (IRON, 1.6, 0.2 + lit * 0.5));
    for y in 0..S {
        for x in 0..S {
            let d = (x as f32 + 0.5 - 25.0).hypot(y as f32 + 0.5 - 16.5);
            if (2.3..3.9).contains(&d) {
                let lit = -((x as f32 - 24.5) + (y as f32 - 16.0)) / 5.0;
                s.put(x, y, (IRON, 1.5, 0.15 + lit * 0.7));
            } else if (3.9..4.9).contains(&d) && y > 16 && x > 24 {
                s.shade(x, y, -0.4);
            }
        }
    }
    s
}

/// A gold flame over a hearth bowl: the sigil of the town.
fn sigil(s: &mut Surf, cx: f32, cy: f32) {
    let gold = |v: f32| (GOLD, 0.9, v);
    // the bowl
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - (cy + 2.0));
            let d = dx.hypot(dy);
            if (6.0..8.2).contains(&d) && dy > 1.5 {
                s.put(x, y, gold(-dx * 0.05 + if d < 6.8 { 0.3 } else { -0.15 }));
            }
        }
    }
    s.rect(
        (cx - 2.5) as i32,
        (cy + 10.0) as i32,
        (cx + 2.5) as i32,
        (cy + 11.5) as i32,
        |_, _| gold(-0.2),
    );
    // three tongues of flame
    let tongue = |s: &mut Surf, foot: (f32, f32), tip: (f32, f32), wide: f32| {
        s.stroke(
            foot,
            tip,
            |t| wide * (1.0 - t).powf(0.7) * (t * 5.0 + 0.4).min(1.0),
            |t, lit| gold(lit * 0.35 + 0.1 + t * 0.2),
        );
    };
    tongue(s, (cx - 3.0, cy + 6.0), (cx - 5.5, cy - 3.5), 2.0);
    tongue(s, (cx + 3.0, cy + 6.0), (cx + 5.5, cy - 2.0), 2.0);
    tongue(s, (cx, cy + 6.5), (cx + 0.5, cy - 9.5), 3.4);
    // the heart of the flame is cut out
    let field = s.c[at(cx as i32, (cy - 11.0) as i32)];
    s.stroke(
        (cx, cy + 4.5),
        (cx, cy),
        |t| 1.2 * (1.0 - t),
        |_, _| (field, 0.5, -0.25),
    );
}

/// Cloth that hangs in soft folds, with gold trim down both edges. With
/// `emblem`, the town's sigil is stitched in the middle.
fn banner_surf(base: Rgb, emblem: bool) -> Surf {
    let mut s = Surf::from_fn(|x, y| {
        let u = x as f32 / SF * TAU;
        let fold = (u * 2.0 + 0.6 * (y as f32 / SF * TAU).sin()).sin();
        let weave = if (x + y) % 2 == 0 { 0.03 } else { -0.03 };
        match x.min(S - 1 - x) {
            0 => (base, 0.5, -0.5),
            1 | 2 => (
                GOLD,
                0.8,
                fold * 0.2 + if (y + x) % 4 < 2 { 0.2 } else { -0.2 },
            ),
            3 => (base, 0.3, -0.45),
            _ => (base, 0.5, fold * 0.30 + weave),
        }
    });
    if emblem {
        sigil(&mut s, 16.0, 15.0);
    } else {
        // from above: a gold border all round
        for i in 0..S {
            for k in 1..3 {
                let v = if (i + k) % 4 < 2 { 0.2 } else { -0.2 };
                s.put(i, k, (GOLD, 0.8, v));
                s.put(i, S - 1 - k, (GOLD, 0.8, v));
            }
        }
    }
    s.bands = 9.0;
    s
}

fn barrel_side_surf(base: Rgb) -> Surf {
    let mut s = boards(base, 8, true, false, 341);
    // the barrel is round: light falls off to either side
    let round = |x: i32| 0.22 - ((x as f32 - 11.5) / 19.0).powi(2) * 0.9;
    for y in 0..S {
        for x in 0..S {
            s.shade(x, y, round(x));
        }
    }
    for (from, to) in [(4, 8), (24, 28)] {
        strap(&mut s, from, to, false, IRON);
        for x in 0..S {
            for y in from..to {
                s.shade(x, y, round(x) * 1.4);
            }
            s.shade(x, to, -0.45);
        }
        rivet(&mut s, 15.0, (from + to) as f32 / 2.0, 1.0, IRON);
    }
    s
}

fn barrel_top_surf(base: Rgb) -> Surf {
    let lid = boards(base, 4, false, false, 342);
    let mut s = Surf::from_fn(|x, y| {
        let d = (x as f32 - 15.5).hypot(y as f32 - 15.5);
        let i = at(x, y);
        if d > 15.6 {
            // the staves, seen from above
            (scale(base, 0.7), 0.6, -0.5)
        } else if d > 13.2 {
            let n = pnoise(x, y, 6, 6, 343) - 0.5;
            (IRON, 1.0, n * 0.3 + if d > 14.6 { -0.2 } else { 0.15 })
        } else if d > 12.2 {
            (scale(base, 0.6), 0.2, -0.7)
        } else {
            (lid.c[i], lid.h[i] * 0.5, lid.v[i])
        }
    });
    // the bung
    s.disc(21.0, 11.5, 2.0, |d, lit| {
        (
            scale(base, 0.8),
            0.6,
            if d > 0.7 { -0.6 } else { lit * 0.4 },
        )
    });
    s
}

fn crate_surf(base: Rgb, cross: bool) -> Surf {
    let mut s = boards(scale(base, 0.88), 4, true, false, 351);
    for i in 0..N {
        s.h[i] *= 0.4;
        s.v[i] -= 0.15;
    }
    let pale = mix(base, [0.86, 0.72, 0.48], 0.3);
    let brace = |s: &mut Surf, a: (f32, f32), b: (f32, f32)| {
        s.stroke(
            a,
            b,
            |_| 2.6,
            |t, lit| {
                let rim = if lit.abs() > 0.72 { -0.45 } else { 0.0 };
                (
                    pale,
                    0.8,
                    lit * 0.18 + rim + (tnoise(t * 96.0, lit * 3.0 + 4.0, 3, 8, 352) - 0.5) * 0.5,
                )
            },
        );
    };
    brace(&mut s, (3.0, 29.0), (29.0, 3.0));
    if cross {
        brace(&mut s, (3.0, 3.0), (29.0, 29.0));
    }
    frame(&mut s, 4, pale, 353);
    for (x, y) in [(2.0, 2.0), (30.0, 2.0), (2.0, 30.0), (30.0, 30.0)] {
        rivet(&mut s, x, y, 0.9, DARK_IRON);
    }
    s
}

fn bookshelf_surf(base: Rgb) -> Surf {
    const COVERS: [Rgb; 8] = [
        [0.62, 0.16, 0.15],
        [0.18, 0.30, 0.58],
        [0.22, 0.45, 0.26],
        [0.74, 0.56, 0.22],
        [0.42, 0.22, 0.50],
        [0.16, 0.46, 0.48],
        [0.46, 0.28, 0.17],
        [0.80, 0.74, 0.60],
    ];
    let mut s = Surf::new(scale(base, 0.4));
    for i in 0..N {
        s.v[i] = -0.7;
    }
    for (shelf, top) in [(0, 2), (1, 17)] {
        let floor = top + 13;
        let (mut x, mut k) = (2, 0);
        while x < 30 {
            let h = hash(k, shelf, 361);
            k += 1;
            if (h >> 12).is_multiple_of(8) {
                // a gap where a book is out
                x += 2;
                continue;
            }
            let wide = (2 + (h % 3) as i32).min(30 - x);
            let y0 = floor - 9 - ((h >> 4) % 4) as i32;
            let c = COVERS[((h >> 8) % 8) as usize];
            let (x0, x1) = (x, x + wide);
            s.rect(x0, y0, x1, floor, |px, py| {
                let side = if px == x0 {
                    0.4
                } else if px == x1 - 1 {
                    -0.45
                } else {
                    0.0
                };
                if py == y0 + 2 || py == floor - 3 {
                    (GOLD, 0.8, side * 0.6 - 0.1)
                } else {
                    (c, 0.8, side + if py == y0 { 0.3 } else { 0.0 })
                }
            });
            x += wide;
        }
    }
    // the case and its shelves
    for y in 0..S {
        for x in 0..S {
            let upright = !(2..30).contains(&x);
            let flat = matches!(y, 0 | 1 | 15 | 16 | 30 | 31);
            if upright || flat {
                let v = grain(x, y, upright, 362) + if y == 15 || y == 0 { 0.3 } else { 0.0 };
                s.put(x, y, (base, 1.0, v));
            }
        }
    }
    s
}

fn anvil_top_surf(base: Rgb) -> Surf {
    let steel = mix(base, [0.62, 0.64, 0.70], 0.6);
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 3, 371) - 0.5;
        let face = (3..29).contains(&x) && (6..26).contains(&y);
        if face {
            // the working face: worn bright in the middle
            let worn =
                1.0 - ((x as f32 - 15.0) / 13.0).powi(2) - ((y as f32 - 16.0) / 10.0).powi(2);
            (steel, 1.0, n * 0.5 + worn * 0.3 - 0.1)
        } else {
            (base, 0.4, n * 0.5 - 0.2)
        }
    });
    // scratches from the hammer
    for k in 0..6 {
        let (x, y) = (6.0 + rnd(k, 0, 372) * 14.0, 9.0 + rnd(k, 1, 372) * 13.0);
        let len = 3.0 + rnd(k, 2, 372) * 4.0;
        s.stroke(
            (x, y),
            (x + len, y - len * 0.6),
            |_| 0.5,
            |_, _| (steel, 1.0, 0.7),
        );
    }
    // the square hardy hole and the round pritchel hole
    s.rect(22, 9, 26, 13, |_, _| (base, 0.0, -1.1));
    s.disc(24.0, 20.5, 1.4, |_, _| (base, 0.0, -1.1));
    s
}

/// The outline of an anvil, raised from a dark plate.
fn anvil_side_surf(base: Rgb) -> Surf {
    let steel = mix(base, [0.55, 0.57, 0.64], 0.5);
    let mut s = Surf::from_fn(|x, y| {
        let n = fbm(x, y, 3, 373) - 0.5;
        (scale(base, 0.55), 0.0, n * 0.4 - 0.3)
    });
    s.paint(|x, y| {
        let dx = x - 16.0;
        let within = if y < 3.0 {
            false
        } else if y < 11.0 {
            // the face, with the horn tapering to the left
            let horn = if x < 9.0 {
                11.0 - (9.0 - x) * 0.65
            } else {
                11.0
            };
            x > 1.0 && x < 30.0 && y < horn
        } else if y < 21.0 {
            // the waist, narrowest in the middle
            dx.abs() < 5.0 + ((y - 16.0) / 5.0).powi(2) * 3.0
        } else {
            // the foot
            dx.abs() < (8.0 + (y - 21.0) * 0.9).min(13.5)
        };
        if !within {
            return None;
        }
        let n = tnoise(x, y, 4, 4, 374) - 0.5;
        let top = if y < 5.0 { 0.5 } else { 0.0 };
        Some((steel, 1.0, n * 0.45 + top - (y - 12.0).max(0.0) * 0.012))
    });
    s
}

fn brazier_top_surf(pulse: f32) -> Surf {
    let lumps = sites(5, 5, 1.0, 381);
    let mut s = Surf::new(DARK_IRON);
    for y in 0..S {
        for x in 0..S {
            let edge = x.min(y).min(S - 1 - x).min(S - 1 - y);
            if edge >= 4 {
                let d = (x as f32 - 15.5).hypot(y as f32 - 15.5);
                match coal(x, y, &lumps, 1.0 - d / 16.0, pulse) {
                    Ok(c) => s.glow(x, y, c),
                    Err(dab) => s.put(x, y, dab),
                }
                continue;
            }
            // the iron rim, warm on the side that faces the fire
            let n = pnoise(x, y, 6, 6, 382) - 0.5;
            let c = if edge == 3 {
                mix(DARK_IRON, [0.9, 0.35, 0.12], 0.5 + 0.2 * pulse)
            } else {
                DARK_IRON
            };
            s.put(x, y, (c, 1.0, n * 0.4 + if edge == 3 { 0.3 } else { 0.0 }));
        }
    }
    for i in 0..S {
        s.shade(i, 0, 0.35);
        s.shade(0, i, 0.35);
        s.shade(i, S - 1, -0.4);
        s.shade(S - 1, i, -0.4);
    }
    for (x, y) in [
        (2.0, 2.0),
        (30.0, 2.0),
        (2.0, 30.0),
        (30.0, 30.0),
        (16.0, 2.0),
        (16.0, 30.0),
        (2.0, 16.0),
        (30.0, 16.0),
    ] {
        rivet(&mut s, x, y, 0.9, IRON);
    }
    s
}

/// An iron fire basket on legs: coals glow between its bars.
fn brazier_side_surf() -> Surf {
    let lumps = sites(5, 6, 1.0, 383);
    let mut s = Surf::new([0.09, 0.08, 0.10]);
    for y in 0..S {
        for x in 0..S {
            if (5..21).contains(&y) {
                let middle = 1.0 - (y as f32 - 11.0).abs() / 9.0;
                match coal(x, y, &lumps, middle, 0.5) {
                    Ok(c) => s.glow(x, y, c),
                    Err(dab) => s.put(x, y, dab),
                }
            } else if y >= 25 {
                // the dark under the basket, lit a little from above
                let v = -0.2 - (y - 25) as f32 * 0.1;
                s.put(x, y, ([0.30, 0.13, 0.10], 0.0, v));
            }
        }
    }
    // bars
    for x in (1..S).step_by(6) {
        for y in 5..21 {
            s.put(x, y, (DARK_IRON, 1.0, 0.25));
            s.put(x + 1, y, (DARK_IRON, 1.0, -0.35));
        }
    }
    strap(&mut s, 0, 5, false, DARK_IRON);
    strap(&mut s, 21, 25, false, DARK_IRON);
    for k in 0..4 {
        rivet(&mut s, 4.0 + k as f32 * 8.0, 2.5, 1.0, IRON);
    }
    // legs
    for (from, to) in [(2, 6), (14, 18), (26, 30)] {
        for y in 25..S {
            for x in from..to {
                let v = if x == from {
                    0.4
                } else if x == to - 1 {
                    -0.4
                } else {
                    0.0
                };
                s.put(x, y, (DARK_IRON, 1.0, v));
            }
        }
    }
    s
}

/// A small flower seen from the front or from above.
fn flower(s: &mut Surf, x: f32, y: f32, petal: Rgb, r: f32) {
    let dark = scale(petal, 0.45);
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU - 0.3;
        let (px, py) = (x + a.cos() * r * 1.25, y + a.sin() * r * 1.25);
        s.disc(px, py, r + 0.6, |_, _| (dark, 0.9, -0.2));
    }
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU - 0.3;
        let (px, py) = (x + a.cos() * r * 1.25, y + a.sin() * r * 1.25);
        s.disc(px, py, r, |d, lit| {
            (petal, 1.0, 0.15 + lit * 0.45 - d * 0.1)
        });
    }
    s.disc(x, y, r * 0.75, |_, lit| {
        ([0.98, 0.80, 0.22], 1.1, 0.1 + lit * 0.4)
    });
}

const PETALS: [Rgb; 6] = [
    [0.90, 0.22, 0.30],
    [0.98, 0.86, 0.30],
    [0.52, 0.42, 0.92],
    [0.97, 0.95, 0.92],
    [0.96, 0.52, 0.72],
    [0.98, 0.58, 0.20],
];

/// A leaf of a flower bed.
fn bed_leaf(s: &mut Surf, x: f32, y: f32, turn: f32, len: f32, c: Rgb, level: f32) {
    let (ox, oy) = (turn.cos() * len, turn.sin() * len);
    let shape = |t: f32| 2.0 * (t * PI).sin().powf(0.7);
    s.stroke(
        (x, y),
        (x + ox, y + oy),
        |t| shape(t) + 0.8,
        |_, _| (c, 0.4, level - 0.7),
    );
    s.stroke((x, y), (x + ox, y + oy), shape, |t, lit| {
        (c, 0.6, level + lit.signum() * 0.2 + t * 0.2)
    });
}

fn flower_bed_top_surf(green: Rgb) -> Surf {
    let mut s = dirt_surf([0.34, 0.23, 0.16]);
    for (k, &(x, y)) in sites(6, 6, 1.0, 391).iter().enumerate() {
        let k = k as i32;
        let c = vary(green, rnd(k, 0, 392), 0.3);
        bed_leaf(
            &mut s,
            x,
            y,
            rnd(k, 1, 392) * TAU,
            5.5 + rnd(k, 2, 392) * 2.0,
            c,
            (rnd(k, 3, 392) - 0.5) * 0.5,
        );
    }
    for (k, &(x, y)) in sites(3, 3, 0.8, 393).iter().enumerate() {
        if k % 3 == 1 {
            continue;
        }
        let r = 1.4 + rnd(k as i32, 0, 394) * 0.4;
        flower(&mut s, x, y, PETALS[(k * 5 + 1) % 6], r);
    }
    s
}

fn flower_bed_side_surf(green: Rgb, dirt: &Surf) -> Surf {
    let mut s = dirt.clone();
    for i in 0..N {
        s.c[i] = mix(s.c[i], [0.34, 0.23, 0.16], 0.6);
    }
    for x in 0..S {
        let depth = 13.0 + 5.0 * tnoise(x as f32 + 0.5, 0.5, 5, 1, 395);
        for y in 0..depth as i32 {
            s.put(x, y, (green, 0.3, -0.75));
        }
    }
    // leaves that grow up and out, and spill over the soil
    for layer in 0..2 {
        for k in 0..12 {
            let x = (k as f32 + rnd(k, layer, 396)) * SF / 12.0;
            let y = 15.0 - layer as f32 * 6.0 + rnd(k, 2 + layer, 396) * 3.0;
            let turn = -PI / 2.0 + (rnd(k, 4 + layer, 396) - 0.5) * 2.4;
            let turn = if layer == 0 && k % 3 == 0 {
                -turn
            } else {
                turn
            };
            let c = vary(green, rnd(k, 6 + layer, 396), 0.3);
            bed_leaf(&mut s, x, y, turn, 6.5, c, -0.25 + layer as f32 * 0.3);
        }
    }
    for k in 0..5 {
        let x = (k as f32 + 0.2 + rnd(k, 0, 397) * 0.6) * SF / 5.0;
        let y = 4.0 + rnd(k, 1, 397) * 7.0;
        flower(&mut s, x, y, PETALS[(k as usize * 2 + 3) % 6], 1.5);
    }
    // the soil under the leaves is in shadow
    for y in 1..S {
        for x in 0..S {
            if s.height(x, y) < 0.3 && s.height(x, y - 1) >= 0.3 {
                s.shade(x, y, -0.4);
            }
        }
    }
    s
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
    let dirt_c = color_of(w::DIRT);
    let dirt = dirt_surf(dirt_c);
    let dirt_l = b.add(&dirt.render(0.45).recenter(dirt_c, 0.8));
    let stone_c = color_of(w::STONE);
    let stone = stone_surf(stone_c, 23);
    let stone_l = b.add(&stone.render(0.6).recenter(stone_c, 0.8));
    let planks_c = color_of(w::PLANKS);

    for (id, slot) in faces.iter_mut().enumerate() {
        let id = id as u8;
        let base = color_of(id);
        // a plain material: one picture, pulled to the block's colour
        let plain =
            |b: &mut Builder, s: Surf, relief: f32| b.all(&s.render(relief).recenter(base, 0.8));
        *slot = match id {
            w::AIR => [air; 3],
            w::GRASS => {
                // a deeper, less yellow green than the block's swatch colour
                let lawn = mix(base, [0.30, 0.56, 0.27], 0.45);
                let top = b.add(&grass_top_surf(lawn).render(0.0).recenter(lawn, 0.8));
                let side = b.add(&grass_side_surf(base, &dirt).render(0.45));
                [top, side, dirt_l]
            }
            w::DIRT => [dirt_l; 3],
            w::STONE => [stone_l; 3],
            w::SAND => plain(&mut b, sand_surf(base), 0.5),
            w::LOG | w::BIRCH_LOG => {
                let (side, wood) = if id == w::LOG {
                    (bark_surf(base, 71), mix(base, [0.86, 0.70, 0.46], 0.7))
                } else {
                    (birch_bark_surf(base), [0.87, 0.78, 0.58])
                };
                let rim = if id == w::LOG { base } else { scale(base, 0.8) };
                let end = b.add(&log_top_surf(rim, wood, 72 + id as u32).render(0.5));
                let side = b.add(&side.render(0.6).recenter(base, 0.8));
                [end, side, end]
            }
            w::LEAVES => b.all(&leaves_img(base, 61, &BROAD, 70).recenter(base, 0.6)),
            w::PLANKS | DARK_PLANKS | BIRCH_PLANKS => {
                plain(&mut b, planks_surf(base, 100 + id as u32), 0.5)
            }
            w::GLASS => b.all(&glass_img(base)),
            w::EMBERSTONE => {
                let frames: Vec<Img> = (0..4)
                    .map(|f| ember_surf(pulse(f, 4)).render(0.5))
                    .collect();
                [b.frames(&frames, 0.35); 3]
            }
            w::BRICK => plain(&mut b, brick_surf(base), 0.5),
            w::SNOW => plain(&mut b, snow_surf(base), 0.3),
            w::COBBLE => plain(&mut b, cobble_surf(base), 0.7),
            w::WATER => [b.frames(&water_frames(base), 0.2); 3],
            w::LANTERN => {
                let top = b.add(&lantern_top_surf(base).render(0.5));
                [top, b.add(&lantern_side_surf(base).render(0.5)), top]
            }
            BASALT => {
                let top = b.add(&basalt_top_surf(base).render(0.7).recenter(base, 0.8));
                let side = b.add(&basalt_side_surf(base).render(0.6).recenter(base, 0.8));
                [top, side, top]
            }
            MARBLE => plain(&mut b, marble_surf(base), 0.3),
            w::CLAY => plain(&mut b, clay_surf(base), 0.4),
            TERRACOTTA => plain(&mut b, terracotta_surf(base), 0.4),
            MOSSY_COBBLE => plain(
                &mut b,
                mossy_surf(color_of(w::COBBLE), [0.30, 0.50, 0.20]),
                0.7,
            ),
            ICE => b.all(&ice_img(base)),
            WOOL_FIRST..=WOOL_LAST => plain(&mut b, wool_surf(base), 0.0),
            w::GLOWSHROOM => {
                let top = [
                    (9.0, 9.0, 5.6),
                    (24.0, 21.0, 4.8),
                    (23.0, 6.0, 3.0),
                    (7.0, 24.0, 3.2),
                    (16.0, 15.0, 1.8),
                ];
                let side = [
                    (7.0, 6.0, 4.4),
                    (22.0, 11.0, 3.2),
                    (14.0, 21.0, 4.0),
                    (28.0, 25.0, 2.6),
                    (3.0, 23.0, 2.0),
                    (17.0, 2.0, 1.8),
                ];
                let under = [(8.0, 8.0, 2.0), (22.0, 12.0, 1.6), (14.0, 24.0, 2.0)];
                [
                    b.add(&shroom_surf(base, &top, 261).render(0.5)),
                    b.add(&shroom_surf(base, &side, 262).render(0.5)),
                    b.add(&shroom_surf(base, &under, 263).render(0.5)),
                ]
            }
            w::BIRCH_LEAVES => b.all(&leaves_img(base, 62, &ROUND, 110).recenter(base, 0.6)),
            w::PINE_LEAVES => b.all(&leaves_img(base, 63, &NEEDLE, 60).recenter(base, 0.6)),
            w::BEDROCK => plain(&mut b, bedrock_surf(base), 0.7),
            w::COAL_ORE => b.all(&ore_surf(&stone, stone_c, [0.13, 0.13, 0.17], 191).render(0.6)),
            w::IRON_ORE => b.all(&ore_surf(&stone, stone_c, [0.82, 0.58, 0.42], 193).render(0.6)),
            w::CRAFTING_TABLE => [
                b.add(&table_top_surf(base).render(0.5)),
                b.add(&table_side_surf(base).render(0.5)),
                b.add(&planks_surf(base, 213).render(0.5)),
            ],
            w::FURNACE => {
                let slab = b.add(&slab_surf(base, 222).render(0.5));
                [slab, b.add(&furnace_side_surf(base).render(0.5)), slab]
            }
            w::CHEST => [
                b.add(&chest_top_surf(base).render(0.5)),
                b.add(&chest_side_surf(base).render(0.5)),
                b.add(&planks_surf(base, 214).render(0.5)),
            ],
            w::BED => [
                b.add(&bed_top_surf(base, planks_c).render(0.5)),
                b.add(&bed_side_surf(base, planks_c).render(0.5)),
                b.add(&planks_surf(planks_c, 215).render(0.5)),
            ],
            w::BERRY_BUSH => b.all(&berry_bush_surf(base).render(0.5)),
            w::TORCH => {
                let frames: Vec<Img> = (0..4).map(torch_side_img).collect();
                let wood = [0.52, 0.38, 0.22];
                let end = Img::from_fn(|x, y| {
                    let d = (2 * x - 31).abs().max((2 * y - 31).abs()) / 2;
                    px(tone(wood, ((d as f32 * 1.3).sin()) * 0.4 - 0.1), 255)
                });
                [
                    b.add(&torch_top_img()),
                    b.frames(&frames, 0.14),
                    b.add(&end),
                ]
            }
            w::HEARTH => {
                // The stone is the block's colour pulled towards grey, so
                // that the embers stand out against it.
                let stone = mix(base, [0.50, 0.47, 0.47], 0.45);
                let frames: Vec<Img> = (0..4)
                    .map(|f| hearth_top_surf(stone, pulse(f, 4)).render(0.6))
                    .collect();
                [
                    b.frames(&frames, 0.3),
                    b.add(&hearth_side_surf(stone).render(0.5)),
                    b.add(&slab_surf(stone, 254).render(0.5)),
                ]
            }
            w::STONE_BRICKS => plain(&mut b, stone_brick_surf(base, 401), 0.5),
            w::PLASTER => plain(&mut b, plaster_surf(base), 0.5),
            w::RED_ROOF_TILES => {
                let scallop = |across: f32, _: f32| (1.0 - across * across).max(0.0).sqrt() * 4.2;
                plain(&mut b, roof_surf(base, 8, scallop, 0.35, 402), 0.5)
            }
            w::SLATE_ROOF => {
                // square slates with clipped corners and uneven lower edges
                let slate = |across: f32, r: f32| {
                    if across.abs() > 0.8 {
                        0.0
                    } else {
                        1.0 + (r * 3.0).floor()
                    }
                };
                plain(&mut b, roof_surf(base, 16, slate, 0.25, 403), 0.5)
            }
            w::THATCH => plain(&mut b, thatch_surf(base), 0.5),
            w::TIMBER_BEAM => {
                let end = b.add(
                    &log_top_surf(scale(base, 0.8), base, 404)
                        .render(0.4)
                        .recenter(base, 0.8),
                );
                let side = b.add(&timber_surf(base, true, 405).render(0.5).recenter(base, 0.8));
                [end, side, end]
            }
            w::FLAGSTONES => {
                let top = flagstone_surf(base, Some(color_of(w::GRASS)), 406);
                let side = b.add(
                    &flagstone_surf(base, None, 407)
                        .render(0.6)
                        .recenter(base, 0.8),
                );
                [b.add(&top.render(0.6)), side, side]
            }
            w::LEADED_WINDOW => b.all(&window_img(base)),
            w::WOODEN_DOOR => {
                let end = b.add(&boards(scale(base, 0.8), 4, true, false, 408).render(0.5));
                [end, b.add(&door_surf(base).render(0.5)), end]
            }
            w::RED_BANNER | w::BLUE_BANNER => {
                let top = b.add(&banner_surf(base, false).render(0.4));
                [top, b.add(&banner_surf(base, true).render(0.4)), top]
            }
            w::HAY_BALE => {
                let top = b.add(&straw_surf(base, 0.05, 409).render(0.5).recenter(base, 0.8));
                let side = b.add(&hay_surf(base, 410).render(0.5).recenter(base, 0.8));
                [top, side, top]
            }
            w::BARREL => {
                let top = b.add(&barrel_top_surf(base).render(0.5));
                [top, b.add(&barrel_side_surf(base).render(0.5)), top]
            }
            w::CRATE => {
                let top = b.add(&crate_surf(base, true).render(0.5));
                [top, b.add(&crate_surf(base, false).render(0.5)), top]
            }
            w::BOOKSHELF => {
                let end = b.add(&planks_surf(base, 411).render(0.5));
                [end, b.add(&bookshelf_surf(base).render(0.5)), end]
            }
            w::ANVIL => [
                b.add(&anvil_top_surf(base).render(0.5)),
                b.add(&anvil_side_surf(base).render(0.5)),
                b.add(&slab_surf(base, 412).render(0.5)),
            ],
            w::BRAZIER => {
                let frames: Vec<Img> = (0..4)
                    .map(|f| brazier_top_surf(pulse(f, 4)).render(0.5))
                    .collect();
                [
                    b.frames(&frames, 0.3),
                    b.add(&brazier_side_surf().render(0.5)),
                    b.add(&slab_surf(DARK_IRON, 413).render(0.5)),
                ]
            }
            w::FLOWER_BED => [
                b.add(&flower_bed_top_surf(base).render(0.4)),
                b.add(&flower_bed_side_surf(base, &dirt).render(0.4)),
                dirt_l,
            ],
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

/// A wooden handle from the grip at `a` to the head at `b`.
fn handle(s: &mut Surf, a: (f32, f32), b: (f32, f32)) {
    s.stroke(
        a,
        b,
        |_| 1.6,
        |t, lit| {
            let grip = t < 0.24;
            let wrap = grip && ((t * 50.0) as i32) % 3 == 0;
            (
                if grip { scale(HANDLE, 0.72) } else { HANDLE },
                0.0,
                lit * 0.45 - 0.05 - if wrap { 0.35 } else { 0.0 },
            )
        },
    );
}

/// A rough lump: a blob with flat facets, lit from the top-left. The shape
/// depends on `seed`, so coal and raw iron do not look alike.
fn lump(s: &mut Surf, col: Rgb, seed: u32) {
    let facets: Vec<(f32, f32)> = (0..7)
        .map(|k| (8.0 + rnd(k, 0, seed) * 16.0, 9.0 + rnd(k, 1, seed) * 15.0))
        .collect();
    s.paint(|x, y| {
        let (dx, dy) = (x - 16.0, y - 16.5);
        // the outline is an uneven seven-sided shape
        let turn = (dy.atan2(dx) / TAU + 1.0).fract() * 7.0;
        let (k, t) = (turn as i32, turn.fract());
        let corner = |k: i32| 8.2 + 4.0 * rnd(k % 7, 2, seed);
        let edge = corner(k) + (corner(k + 1) - corner(k)) * t;
        if (dx / 1.15).hypot(dy / 0.95) > edge {
            return None;
        }
        let hit = nearest(x, y, &facets, 0.3, 1.0);
        let i = hit.i as i32;
        let tilt = -(hit.dx + hit.dy) * 0.07;
        let light = -(dx + dy * 1.2) / 14.0;
        Some((
            col,
            rnd(i, 3, seed) * 0.45,
            (rnd(i, 4, seed) - 0.5) * 0.5 + tilt + light * 0.7,
        ))
    });
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
                let c = with_luma(c, luma(c).min(0.20));
                img.set(x, y, px(c, 255));
            }
        }
    }
}

/// A soft glow around a sprite, for things that give off light.
fn halo(img: &mut Img, c: Rgb) {
    for (ring, alpha) in [150u8, 90, 40].into_iter().enumerate() {
        let src = img.clone();
        for y in 1..S - 1 {
            for x in 1..S - 1 {
                if src.get(x, y)[3] != 0 {
                    continue;
                }
                let lit = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|d| src.get(x + d.0, y + d.1)[3] != 0);
                // keep the corners of the icon clear
                let corner = x.min(S - 1 - x) + y.min(S - 1 - y) < 4;
                if lit && !corner {
                    img.set(
                        x,
                        y,
                        px(mix(c, [1.0, 0.9, 0.6], 0.3 - ring as f32 * 0.1), alpha),
                    );
                }
            }
        }
    }
}

/// A four-pointed glint.
fn sparkle(img: &mut Img, x: i32, y: i32, c: Rgb) {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for (far, alpha) in [(1, 230u8), (2, 130)] {
            let (px_, py) = (x + dx * far, y + dy * far);
            if inside(px_, py) && img.get(px_, py)[3] < alpha {
                img.set(px_, py, px(mix(c, [1.0, 1.0, 0.95], 0.5), alpha));
            }
        }
    }
    img.set(x, y, [255, 253, 244, 255]);
}

/// The working end of a tool, in the colour of its material.
fn tool_head(s: &mut Surf, kind: ToolKind, col: Rgb) {
    let metal = |v: f32| -> Dab { (col, 0.0, v) };
    match kind {
        ToolKind::Pickaxe => {
            handle(s, (5.5, 26.5), (22.0, 10.0));
            let pts: Vec<(f32, f32)> = (0..=10)
                .map(|i| {
                    let a = (-97.0 + 104.0 * i as f32 / 10.0).to_radians();
                    (11.5 + 15.0 * a.cos(), 20.5 + 15.0 * a.sin())
                })
                .collect();
            s.path(
                &pts,
                |t| 0.5 + 2.4 * (t * PI).sin().powf(0.6),
                |t, lit| {
                    let shine = if (t - 0.3).abs() < 0.07 { 0.45 } else { 0.0 };
                    metal(lit.signum() * 0.42 + 0.08 + shine)
                },
            );
        }
        ToolKind::Axe => {
            handle(s, (5.5, 26.5), (23.5, 8.5));
            s.paint(|x, y| {
                let (dx, dy) = (x - 19.0, y - 13.0);
                // along the handle, and out towards the cutting edge
                let (u, v) = ((dx - dy) * DIAG, (-dx - dy) * DIAG);
                let edge = 10.5 - 0.05 * u * u;
                if v < -2.4 || v > edge || u.abs() > 3.0 + 0.52 * v.max(0.0) {
                    return None;
                }
                Some(metal(if v > edge - 1.9 {
                    0.85
                } else if v > edge - 2.7 {
                    -0.35
                } else if v < 0.0 {
                    -0.25
                } else {
                    0.12 + u * 0.06
                }))
            });
        }
        ToolKind::Shovel => {
            handle(s, (5.5, 26.5), (18.5, 13.5));
            s.paint(|x, y| {
                let (dx, dy) = (x - 17.0, y - 15.0);
                let (u, v) = ((dx - dy) * DIAG, (-dx - dy) * DIAG);
                let p = u / 13.5;
                if !(0.0..=1.0).contains(&p) {
                    return None;
                }
                let round = (1.0 - ((p - 0.45) / 0.55).max(0.0).powi(2)).max(0.0).sqrt();
                let wide = 4.8 * (p * 6.0 + 0.3).min(1.0) * round;
                if v.abs() > wide {
                    return None;
                }
                let side = v / wide.max(0.1);
                Some(metal(
                    if side.abs() < 0.2 {
                        0.7
                    } else if side > 0.0 {
                        0.28
                    } else {
                        -0.28
                    } + p * 0.15,
                ))
            });
        }
        ToolKind::Sword => {
            // blade, with a lit and a shaded bevel
            s.stroke(
                (10.5, 21.5),
                (27.5, 4.5),
                |t| 2.5 * ((1.0 - t) * 4.5).min(1.0),
                |t, lit| {
                    metal(if lit.abs() < 0.16 {
                        -0.05
                    } else if lit > 0.0 {
                        0.70 + if (t - 0.45).abs() < 0.08 { 0.4 } else { 0.0 }
                    } else {
                        -0.30
                    })
                },
            );
            // guard, grip and pommel
            s.stroke(
                (6.5, 18.5),
                (13.5, 25.5),
                |_| 1.5,
                |t, lit| (GOLD, 0.0, lit * 0.45 - (t - 0.5).abs() * 0.4),
            );
            s.stroke(
                (5.5, 26.5),
                (9.0, 23.0),
                |_| 1.3,
                |t, lit| {
                    let wrap = ((t * 9.0) as i32) % 2 == 0;
                    (
                        scale(HANDLE, 0.7),
                        0.0,
                        lit * 0.4 - if wrap { 0.3 } else { 0.0 },
                    )
                },
            );
            s.disc(4.8, 27.2, 1.9, |_, lit| (GOLD, 0.0, lit * 0.6));
        }
        ToolKind::Hoe => {
            handle(s, (5.5, 26.5), (23.5, 8.5));
            s.path(
                &[(25.0, 7.5), (19.5, 4.8), (13.0, 5.8), (9.5, 10.0)],
                |t| 2.3 - 1.0 * t,
                |t, lit| metal(lit.signum() * 0.40 + 0.1 + if t > 0.85 { 0.4 } else { 0.0 }),
            );
        }
    }
}

/// A weapon icon, in the colour of its material.
fn weapon(s: &mut Surf, kind: WeaponKind, col: Rgb) {
    let metal = |v: f32| -> Dab { (col, 0.0, v) };
    // a blade with a lit and a shaded bevel
    let blade = |t: f32, lit: f32| {
        metal(if lit.abs() < 0.16 {
            -0.05
        } else if lit > 0.0 {
            0.70 + if (t - 0.45).abs() < 0.08 { 0.4 } else { 0.0 }
        } else {
            -0.30
        })
    };
    let grip = |s: &mut Surf, a: (f32, f32), b: (f32, f32)| {
        s.stroke(
            a,
            b,
            |_| 1.3,
            |t, lit| {
                let wrap = ((t * 9.0) as i32) % 2 == 0;
                (
                    scale(HANDLE, 0.7),
                    0.0,
                    lit * 0.4 - if wrap { 0.3 } else { 0.0 },
                )
            },
        );
    };
    match kind {
        WeaponKind::Sword => tool_head(s, ToolKind::Sword, col),
        WeaponKind::Axe => {
            // a war axe: a blade on both sides of the shaft
            handle(s, (5.5, 26.5), (24.5, 7.5));
            s.paint(|x, y| {
                let (dx, dy) = (x - 19.5, y - 12.5);
                let (u, v) = ((dx - dy) * DIAG, (-dx - dy) * DIAG);
                let out = v.abs();
                let edge = 8.6 - 0.07 * u * u;
                if out > edge || u.abs() > 1.8 + 0.62 * out {
                    return None;
                }
                Some(metal(if out > edge - 1.7 {
                    0.85
                } else if out > edge - 2.4 {
                    -0.35
                } else if v > 0.0 {
                    0.15 + u * 0.05
                } else {
                    -0.15 + u * 0.05
                }))
            });
        }
        WeaponKind::Mace => {
            handle(s, (5.5, 26.5), (19.0, 13.0));
            // flanges around a heavy head
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                let tip = (21.0 + a.cos() * 8.2, 11.0 + a.sin() * 8.2);
                s.stroke(
                    (21.0, 11.0),
                    tip,
                    |t| 2.0 * (1.0 - t) + 0.3,
                    |_, lit| metal(lit.signum() * 0.4),
                );
            }
            s.disc(21.0, 11.0, 5.6, |d, lit| {
                metal(lit * 0.8 - d * d * 0.25 + if d < 0.3 { 0.3 } else { 0.0 })
            });
        }
        WeaponKind::Dagger => {
            s.stroke(
                (13.0, 19.0),
                (25.0, 7.0),
                |t| 2.4 * ((1.0 - t) * 2.2).min(1.0),
                blade,
            );
            s.stroke(
                (9.5, 17.5),
                (14.5, 22.5),
                |_| 1.4,
                |t, lit| (GOLD, 0.0, lit * 0.45 - (t - 0.5).abs() * 0.4),
            );
            grip(s, (7.0, 25.0), (11.5, 20.5));
            s.disc(6.3, 25.7, 1.8, |_, lit| (GOLD, 0.0, lit * 0.6));
        }
        WeaponKind::Staff => {
            handle(s, (5.0, 27.0), (21.5, 10.5));
            // a claw of wood that holds an orb
            s.path(
                &[(20.5, 11.5), (19.0, 6.5), (22.5, 3.0), (27.0, 4.0)],
                |t| 1.6 - 0.8 * t,
                |_, lit| (HANDLE, 0.0, lit * 0.45),
            );
            s.path(
                &[(20.5, 11.5), (26.0, 12.5), (28.5, 9.5)],
                |t| 1.6 - 0.8 * t,
                |_, lit| (HANDLE, 0.0, lit * 0.45 - 0.1),
            );
            s.disc(23.5, 8.0, 3.4, |d, lit| {
                metal(lit * 0.8 + 0.25 - d * d * 0.3)
            });
            s.glow(22, 6, [1.0, 1.0, 0.95]);
        }
        WeaponKind::Polearm => {
            handle(s, (4.5, 27.5), (21.0, 11.0));
            // a spear blade, with a hook under it
            s.stroke(
                (19.0, 13.0),
                (28.0, 4.0),
                |t| 3.0 * (t * 4.0 + 0.3).min(1.0) * (1.0 - t).powf(0.8),
                blade,
            );
            s.path(
                &[(19.5, 12.5), (15.0, 8.5), (16.5, 5.5)],
                |t| 1.5 - 0.9 * t,
                |_, lit| metal(lit.signum() * 0.4 + 0.1),
            );
            s.stroke(
                (17.0, 12.0),
                (20.0, 15.0),
                |_| 1.3,
                |_, lit| (GOLD, 0.0, lit * 0.5),
            );
        }
        WeaponKind::Bow => {
            // the string first, then the limbs over it
            s.stroke(
                (7.0, 25.0),
                (25.0, 7.0),
                |_| 0.6,
                |_, _| ([0.90, 0.88, 0.80], 0.0, 0.1),
            );
            let pts: Vec<(f32, f32)> = (0..=10)
                .map(|i| {
                    // a curve from one tip to the other, bent to the top-left
                    let t = i as f32 / 10.0;
                    let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t);
                    (a * 7.0 + b * 4.5 + c * 25.0, a * 25.0 + b * 4.5 + c * 7.0)
                })
                .collect();
            s.path(
                &pts,
                |t| 0.8 + 1.3 * (t * PI).sin().powf(0.5),
                |t, lit| {
                    let wrap = (t - 0.5).abs() < 0.09;
                    if wrap {
                        (scale(HANDLE, 0.6), 0.0, lit * 0.4)
                    } else {
                        metal(lit * 0.5 + 0.05)
                    }
                },
            );
        }
        WeaponKind::Wand => {
            s.stroke(
                (8.5, 24.5),
                (21.0, 12.0),
                |t| 1.5 - 0.5 * t,
                |t, lit| {
                    let band = (t - 0.2).abs() < 0.04 || (t - 0.8).abs() < 0.04;
                    if band {
                        (GOLD, 0.0, lit * 0.5)
                    } else {
                        (scale(HANDLE, 0.8), 0.0, lit * 0.45)
                    }
                },
            );
            // a crystal at the tip
            s.poly(
                &[(22.5, 5.0), (26.5, 9.0), (22.5, 13.5), (19.0, 10.0)],
                |x, y| metal(if x + y < 32.0 { 0.75 } else { -0.05 }),
            );
            s.glow(22, 8, [1.0, 1.0, 0.95]);
        }
        WeaponKind::Shield => {
            // a heater shield: flat top, sides that curve in to a point
            let half = |y: f32| {
                if y < 4.0 || y > 29.0 {
                    -1.0
                } else if y < 14.0 {
                    10.5
                } else {
                    10.5 * (1.0 - ((y - 14.0) / 15.2).powi(2)).max(0.0).sqrt()
                }
            };
            s.paint(|x, y| {
                let dx = x - 16.0;
                let w = half(y);
                if dx.abs() > w {
                    return None;
                }
                let light = -(dx + (y - 15.0)) / 30.0;
                // an iron rim, and a cross on the field
                let rim = dx.abs() > half(y + 2.2) - 2.2 || dx.abs() > w - 2.2 || y < 6.2;
                if rim {
                    return Some((IRON, 0.5, light * 0.9 + 0.1));
                }
                if dx.abs() < 1.6 || (y - 13.0).abs() < 1.6 {
                    return Some((GOLD, 0.3, light * 0.8));
                }
                Some(metal(light * 1.0 + if dx < 0.0 { 0.12 } else { -0.12 }))
            });
            s.disc(16.0, 13.0, 2.6, |d, lit| (GOLD, 0.6, lit * 0.7 - d * 0.2));
        }
        WeaponKind::Offhand => {
            // a closed book, held in the off hand
            let (a, b, c, d) = ((7.0, 9.0), (21.0, 5.0), (26.0, 21.0), (12.0, 25.0));
            // pages show along the lower and the right edge
            s.poly(
                &[(12.0, 25.0), (26.0, 21.0), (27.5, 23.5), (13.5, 27.5)],
                |x, _| {
                    (
                        [0.93, 0.90, 0.80],
                        0.0,
                        if (x as i32) % 2 == 0 { 0.1 } else { -0.2 },
                    )
                },
            );
            s.poly(
                &[(21.0, 5.0), (22.5, 7.5), (27.5, 23.5), (26.0, 21.0)],
                |_, _| ([0.93, 0.90, 0.80], 0.0, -0.3),
            );
            s.poly(&[a, b, c, d], |x, y| metal(0.25 - (x + y - 24.0) * 0.025));
            // a spine, corner pieces and a clasp
            s.stroke(a, d, |_| 1.3, |_, lit| metal(-0.35 + lit * 0.2));
            for p in [b, c] {
                s.disc(
                    p.0 - 1.4,
                    p.1 + if p.1 < 10.0 { 1.6 } else { -1.6 },
                    1.7,
                    |_, lit| (GOLD, 0.0, lit * 0.6),
                );
            }
            s.poly(
                &[(15.0, 12.0), (19.0, 14.0), (17.5, 18.5), (13.5, 16.5)],
                |_, y| (GOLD, 0.0, 0.5 - (y - 12.0) * 0.12),
            );
        }
    }
}

/// How a pixel of armour is shaded: the weave of cloth, the stitches of
/// leather, the rings of mail or the shine of plate. Returns the colour and
/// the light value to add.
fn armor_style(kind: ArmorType, col: Rgb, x: f32, y: f32) -> (Rgb, f32) {
    let (xi, yi) = (x as i32, y as i32);
    match kind {
        // soft folds that hang down
        ArmorType::Cloth => (col, 0.26 * (x * 0.85 + (y * 0.22).sin() * 1.6).sin()),
        // mottled hide; the seams are added by the caller
        ArmorType::Leather => {
            let n = tnoise(x, y, 5, 5, 421) - 0.5;
            (mix(col, [0.50, 0.33, 0.18], 0.25), n * 0.45)
        }
        // rows of rings, each row shifted by half a ring
        ArmorType::Mail => {
            let row = yi / 2;
            let ring = (xi + if row % 2 == 0 { 0 } else { 2 }).rem_euclid(4);
            let v = match (ring, yi % 2) {
                (0 | 1, 0) => 0.45,
                (3, 1) => -0.55,
                _ => -0.15,
            };
            (mix(col, [0.60, 0.62, 0.68], 0.45), v)
        }
        // polished plates: a hard streak of light, and a lip every few rows
        ArmorType::Plate => {
            let streak = (x + y * 0.7 - 21.0).abs();
            let shine = if streak < 1.5 {
                0.8
            } else if streak < 3.5 {
                0.3
            } else {
                0.0
            };
            let lip = match yi % 7 {
                0 => -0.5,
                1 => 0.35,
                _ => 0.0,
            };
            (mix(col, [0.72, 0.74, 0.80], 0.40), shine + lip)
        }
    }
}

/// Paint one piece of armour: every pixel inside `shape`, in the style of
/// its armour type, with a lit top-left rim and a shaded bottom-right one.
fn armor_piece(s: &mut Surf, kind: ArmorType, col: Rgb, shape: impl Fn(f32, f32) -> bool) {
    s.paint(|x, y| {
        if !shape(x, y) {
            return None;
        }
        let (c, style) = armor_style(kind, col, x, y);
        let light = -((x - 16.0) + (y - 16.0)) / 46.0;
        let mut v = style + light;
        if !shape(x - 1.6, y - 1.6) {
            v += 0.35;
        }
        if !shape(x + 1.6, y + 1.6) {
            v -= 0.40;
        }
        // leather is sewn a little way in from its edge
        let near =
            |d: f32| !(shape(x - d, y) && shape(x + d, y) && shape(x, y - d) && shape(x, y + d));
        if kind == ArmorType::Leather && near(2.6) && !near(1.6) && (x as i32 + y as i32) % 3 != 0 {
            return Some((scale(c, 0.55), 0.0, v - 0.2));
        }
        Some((c, 0.0, v))
    });
}

/// A gold stud, buckle or clasp on a piece of armour.
fn stud(s: &mut Surf, x: f32, y: f32, r: f32) {
    s.disc(x, y, r + 0.8, |_, _| (scale(GOLD, 0.4), 0.0, -0.3));
    s.disc(x, y, r, |d, lit| (GOLD, 0.0, lit * 0.7 + 0.1 - d * 0.1));
}

/// An armour or jewellery icon for the slot it is worn in.
fn armor(s: &mut Surf, slot: Slot, kind: ArmorType, col: Rgb) {
    let dark = scale(col, 0.45);
    match slot {
        Slot::Head => {
            // a hood or a helm: a dome with an opening for the face
            let face = |x: f32, y: f32| (x - 16.0).abs() < 4.2 && y > 15.0 && y < 26.0;
            armor_piece(s, kind, col, |x, y| {
                let dome = ((x - 16.0) / 10.5).hypot((y - 15.5) / 11.0) < 1.0 && y < 16.0;
                let sides = (x - 16.0).abs() < 10.5 - (y - 16.0).max(0.0) * 0.18
                    && (16.0..26.5).contains(&y);
                (dome || sides) && !face(x, y)
            });
            s.paint(|x, y| face(x, y).then_some((dark, 0.0, -0.75 + (y - 15.0) * 0.02)));
            if kind == ArmorType::Plate || kind == ArmorType::Mail {
                // a nose guard and a crest
                s.rect(15, 14, 17, 22, |x, _| {
                    (IRON, 0.0, if x == 15 { 0.6 } else { 0.0 })
                });
                s.stroke(
                    (16.0, 3.0),
                    (16.0, 13.0),
                    |_| 1.1,
                    |_, lit| (GOLD, 0.0, lit * 0.5 + 0.1),
                );
            }
        }
        Slot::Shoulders => {
            // a pair of pauldrons, each of two overlapping plates
            for (cx, flip) in [(9.5, -1.0), (22.5, 1.0)] {
                armor_piece(s, kind, scale(col, 0.85), |x, y| {
                    ((x - cx - flip * 1.0) / 6.8).hypot((y - 19.5) / 6.0) < 1.0 && y > 17.0
                });
                armor_piece(s, kind, col, |x, y| {
                    ((x - cx) / 7.2).hypot((y - 14.0) / 7.0) < 1.0 && y < 19.0
                });
                stud(s, cx, 12.5, 1.3);
            }
        }
        Slot::Back => {
            // a cloak that widens to a wavy hem
            armor_piece(s, kind, col, |x, y| {
                let t = (y - 5.0) / 23.0;
                let hem = 27.5 + (x * 0.9).sin() * 1.2;
                (0.0..=1.0).contains(&t) && y < hem && (x - 16.0).abs() < 6.0 + t * 6.5
            });
            // the collar and its clasp
            s.stroke(
                (10.5, 6.0),
                (21.5, 6.0),
                |_| 1.7,
                |_, lit| (dark, 0.0, lit * 0.4),
            );
            stud(s, 16.0, 6.5, 1.5);
        }
        Slot::Chest => {
            let neck = |x: f32, y: f32| (x - 16.0).hypot(y - 5.0) < 4.0;
            armor_piece(s, kind, col, |x, y| {
                let body = (x - 16.0).abs() < 7.5 + ((y - 16.0) / 12.0).max(0.0)
                    && (5.5..28.0).contains(&y);
                let sleeves = (x - 16.0).abs() < 12.5
                    && (5.5..14.0 - ((x - 16.0).abs() - 8.0).max(0.0) * 0.3).contains(&y);
                (body || sleeves) && !neck(x, y)
            });
            // a collar, a belt and its buckle
            for y in 0..S {
                for x in 0..S {
                    let d = (x as f32 + 0.5 - 16.0).hypot(y as f32 + 0.5 - 5.0);
                    if (4.0..5.4).contains(&d) && y >= 5 {
                        s.put(x, y, (dark, 0.0, -0.1));
                    }
                }
            }
            s.rect(9, 21, 23, 23, |_, y| {
                (dark, 0.0, if y == 21 { 0.1 } else { -0.3 })
            });
            stud(s, 16.0, 22.0, 1.3);
        }
        Slot::Wrist => {
            // a bracer, wider at the elbow
            armor_piece(s, kind, col, |x, y| {
                (6.0..27.0).contains(&y) && (x - 16.0).abs() < 5.0 + (27.0 - y) * 0.14
            });
            for y in [9, 16, 23] {
                s.rect(10, y, 23, y + 2, |x, yy| {
                    let within = (x as f32 + 0.5 - 16.0).abs() < 5.0 + (27.0 - yy as f32) * 0.14;
                    if within {
                        (dark, 0.0, if yy == y { 0.1 } else { -0.3 })
                    } else {
                        ([0.0; 3], 0.0, 9.0)
                    }
                });
                stud(s, 16.0, y as f32 + 1.0, 1.0);
            }
        }
        Slot::Hands => {
            // a glove: a cuff, a palm, four fingers and a thumb
            armor_piece(s, kind, col, |x, y| {
                let palm = (x - 16.5).abs() < 6.0 && (11.0..21.0).contains(&y);
                let finger = (0..4).any(|k| {
                    let fx = 12.0 + k as f32 * 3.0;
                    let top = [6.0, 4.0, 4.5, 7.0][k];
                    (x - fx).abs() < 1.35 && y > top && y < 12.0
                });
                let thumb = ((x - 9.0) / 2.0).hypot((y - 15.0) / 4.0) < 1.0;
                palm || finger || thumb
            });
            armor_piece(s, kind, scale(col, 0.8), |x, y| {
                (21.0..28.0).contains(&y) && (x - 16.5).abs() < 5.5 + (y - 21.0) * 0.35
            });
            s.rect(11, 21, 23, 22, |_, _| (dark, 0.0, -0.3));
        }
        Slot::Waist => {
            // a belt that curves round the waist
            let band = |x: f32, y: f32| {
                let mid = 15.0 + ((x - 16.0) / 13.0).powi(2) * -3.0 + 2.0;
                (x - 16.0).abs() < 13.5 && (y - mid).abs() < 3.4
            };
            armor_piece(s, kind, col, band);
            for x in [6.0, 10.0, 22.0, 26.0] {
                stud(s, x, 16.6 - ((x - 16.0) / 13.0f32).powi(2) * 3.0, 0.8);
            }
            s.rect(12, 12, 21, 22, |x, y| {
                let edge = x == 12 || y == 12;
                let hole = (14..19).contains(&x) && (14..20).contains(&y);
                if hole {
                    (dark, 0.0, -0.4)
                } else {
                    (
                        GOLD,
                        0.0,
                        if edge { 0.6 } else { 0.0 } - (y - 12) as f32 * 0.05,
                    )
                }
            });
        }
        Slot::Legs => {
            armor_piece(s, kind, col, |x, y| {
                let hips = (x - 16.0).abs() < 8.5 && (4.5..11.0).contains(&y);
                let leg =
                    |cx: f32| (x - cx).abs() < 3.9 - (y - 11.0) * 0.03 && (10.0..28.5).contains(&y);
                hips || leg(11.6) || leg(20.4)
            });
            s.rect(8, 5, 25, 7, |_, y| {
                (dark, 0.0, if y == 5 { 0.1 } else { -0.3 })
            });
            stud(s, 16.0, 6.0, 1.2);
        }
        Slot::Feet => {
            // a boot, seen from the side
            armor_piece(s, kind, col, |x, y| {
                let shaft = (9.0..17.5).contains(&x) && (4.5..21.0).contains(&y);
                let foot = (9.0..26.0).contains(&x)
                    && y >= 18.0 + ((x - 17.0).max(0.0) / 9.0).powi(2) * 3.5
                    && y < 25.5;
                shaft || foot
            });
            // the cuff and the sole
            s.rect(8, 4, 19, 7, |_, y| {
                (scale(col, 0.7), 0.0, if y == 4 { 0.3 } else { -0.2 })
            });
            s.rect(8, 25, 27, 28, |x, y| {
                (
                    [0.20, 0.15, 0.13],
                    0.0,
                    if y == 25 { 0.3 } else { 0.0 } - (x - 8) as f32 * 0.01,
                )
            });
        }
        Slot::Neck => {
            // a chain with a pendant
            for k in 0..22 {
                let a = (k as f32 / 21.0 - 0.5) * 3.6;
                let (x, y) = (16.0 + a.sin() * 9.5, 7.0 + a.cos() * 12.0);
                s.disc(x, y, 1.05, |_, lit| {
                    (GOLD, 0.0, lit * 0.6 + if k % 2 == 0 { 0.15 } else { -0.25 })
                });
            }
            stud(s, 16.0, 21.5, 4.0);
            s.disc(16.0, 21.5, 2.6, |d, lit| {
                (col, 0.0, lit * 0.8 + 0.2 - d * 0.2)
            });
            s.glow(15, 20, [1.0, 1.0, 0.95]);
        }
        Slot::Ring => {
            for y in 0..S {
                for x in 0..S {
                    let (dx, dy) = (x as f32 + 0.5 - 16.0, y as f32 + 0.5 - 18.5);
                    let d = dx.hypot(dy);
                    if (5.6..8.6).contains(&d) {
                        let inner = d < 6.6;
                        let lit = -(dx + dy) / 12.0;
                        s.put(
                            x,
                            y,
                            (
                                GOLD,
                                0.0,
                                if inner {
                                    -lit * 0.8 - 0.1
                                } else {
                                    lit * 0.9 + 0.1
                                },
                            ),
                        );
                    }
                }
            }
            s.disc(16.0, 9.5, 4.0, |_, lit| (GOLD, 0.0, lit * 0.5 - 0.1));
            s.disc(16.0, 9.5, 3.0, |d, lit| {
                (col, 0.0, lit * 0.8 + 0.2 - d * 0.2)
            });
            s.glow(15, 8, [1.0, 1.0, 0.95]);
        }
        _ => {
            // a trinket: a stone in a gold setting, on a loop
            s.disc(16.0, 6.0, 2.6, |d, lit| {
                if d < 0.5 {
                    ([0.0; 3], 0.0, 9.0)
                } else {
                    (GOLD, 0.0, lit * 0.6)
                }
            });
            s.disc(16.0, 17.5, 9.0, |d, lit| {
                (GOLD, 0.0, lit * 0.7 - if d > 0.85 { 0.3 } else { 0.0 })
            });
            s.disc(16.0, 17.5, 6.2, |d, lit| {
                let facet = (lit * 2.5).round() / 2.5;
                (col, 0.0, facet * 0.8 + 0.15 - d * 0.2)
            });
            s.glow(13, 14, [1.0, 1.0, 0.95]);
            s.glow(14, 14, [1.0, 1.0, 0.95]);
        }
    }
    // dabs with a light value of 9 mark pixels to cut out again
    for i in 0..N {
        if s.v[i] > 8.0 {
            s.a[i] = 0;
        }
    }
}

/// An icon for a piece of equipment: a weapon by its kind, or armour by the
/// slot it is worn in. Gear with neither falls back to a lump.
fn gear_icon(s: &mut Surf, gear: &Gear, col: Rgb, seed: u32) {
    if let Some(w) = &gear.weapon {
        weapon(s, w.kind, col);
        return;
    }
    match (gear.slot, gear.armor_type) {
        (Slot::MainHand | Slot::TwoHand, _) => weapon(s, WeaponKind::Sword, col),
        (Slot::OffHand, _) => weapon(s, WeaponKind::Offhand, col),
        (Slot::Ranged, _) => weapon(s, WeaponKind::Bow, col),
        (Slot::Neck | Slot::Ring | Slot::Trinket, kind) => {
            armor(s, gear.slot, kind.unwrap_or(ArmorType::Cloth), col)
        }
        (slot, Some(kind)) => armor(s, slot, kind, col),
        // a cloak has no armour type
        (Slot::Back, None) => armor(s, Slot::Back, ArmorType::Cloth, col),
        (_, None) => lump(s, col, seed),
    }
}

/// The glint colour of an item quality: green, blue or purple.
fn quality_color(quality: u8) -> Rgb {
    match quality {
        2 => [0.35, 1.0, 0.35],
        3 => [0.35, 0.62, 1.0],
        _ => [0.78, 0.42, 1.0],
    }
}

/// A 32×32 icon with transparent background for an item that is not a block.
pub fn item_sprite(item: &ItemView) -> Rgba {
    let glows = item.light > 0;
    // pale metal needs room above its base colour for a highlight
    let col = if luma(item.color) > 0.75 {
        scale(item.color, 0.86)
    } else {
        item.color
    };
    let seed = hash(
        (item.color[0] * 255.0) as i32,
        (item.color[1] * 255.0) as i32,
        (item.color[2] * 255.0) as u32,
    );
    let mut s = Surf::sprite();
    s.bands = 6.0;
    let flat = |v: f32| -> Dab { (col, 0.0, v) };
    let mut sparkles: &[(i32, i32)] = &[];
    match item.icon {
        _ if item.gear.is_some() => {
            if let Some(gear) = item.gear {
                gear_icon(&mut s, gear, col, seed);
            }
        }
        Icon::Tool => match item.tool {
            Some(tool) => tool_head(&mut s, tool.kind, col),
            None => lump(&mut s, col, seed),
        },
        Icon::Stick => {
            s.stroke(
                (6.5, 25.5),
                (25.5, 6.5),
                |t| 1.9 - 0.4 * t,
                |t, lit| {
                    let knot = (t - 0.36).abs() < 0.03 || (t - 0.7).abs() < 0.025;
                    flat(lit * 0.5 - if knot { 0.4 } else { 0.0 })
                },
            );
            s.stroke(
                (15.0, 17.0),
                (19.5, 16.0),
                |t| 1.2 - 0.5 * t,
                |_, lit| flat(lit * 0.5 - 0.1),
            );
            s.disc(25.6, 6.4, 1.5, |_, _| {
                (mix(col, [1.0, 0.9, 0.7], 0.4), 0.0, 0.4)
            });
        }
        Icon::Lump => lump(&mut s, col, seed),
        Icon::Ingot => {
            // a bar seen from above and in front: left end, front and top
            let (a, b, c, d) = ((8.0, 10.5), (24.0, 8.0), (28.0, 13.0), (12.0, 16.5));
            let (low_c, low_d, low_a) = ((28.5, 20.5), (11.0, 24.5), (5.0, 17.5));
            s.poly(&[a, d, low_d, low_a], |_, y| flat(0.20 - (y - 10.0) * 0.02));
            s.poly(&[d, c, low_c, low_d], |_, y| {
                flat(-0.35 - (y - 14.0) * 0.03)
            });
            s.poly(&[a, b, c, d], |x, y| {
                let streak = (x + y * 0.8 - 22.0).abs() < 1.6;
                flat(0.62 + if streak { 0.5 } else { 0.0 })
            });
            // the bright front edge of the top
            s.stroke(d, c, |_| 0.6, |_, _| flat(1.0));
            s.stroke(a, d, |_| 0.6, |_, _| flat(0.9));
        }
        Icon::Gem => {
            // a tall shard with a lit and a shaded face, and two small ones
            s.poly(&[(22.0, 3.0), (15.0, 28.0), (8.5, 19.0)], |_, y| {
                flat(0.75 - (y - 3.0) * 0.02)
            });
            s.poly(&[(22.0, 3.0), (26.5, 12.5), (15.0, 28.0)], |_, y| {
                flat(-0.15 - (y - 3.0) * 0.015)
            });
            s.poly(
                &[(24.5, 17.5), (28.5, 22.0), (25.0, 28.0), (22.0, 23.0)],
                |x, _| flat(if x < 25.2 { 0.6 } else { -0.2 }),
            );
            s.poly(
                &[(7.0, 22.0), (10.5, 25.5), (7.0, 29.0), (4.5, 26.0)],
                |x, _| flat(if x < 7.4 { 0.6 } else { -0.2 }),
            );
            if glows {
                sparkles = &[(24, 5), (6, 12)];
            }
        }
        Icon::Dust => {
            // a small heap
            s.paint(|x, y| {
                let top = 26.0 - 10.5 * (-((x - 16.0) / 6.8).powi(2)).exp();
                if y < top || y > 26.5 || (x - 16.0).abs() > 12.5 {
                    return None;
                }
                let slope = -(x - 16.0) / 12.0;
                let grit = (rnd(x as i32 / 2, y as i32 / 2, seed) - 0.5) * 0.5;
                Some(flat(slope * 0.6 + grit - (y - top) * 0.03 + 0.2))
            });
            sparkles = &[(8, 9), (22, 7), (26, 17)];
        }
        Icon::Berries => {
            let leaf = [0.30, 0.58, 0.24];
            let stem = [0.36, 0.26, 0.16];
            for to in [(10.5, 17.0), (21.0, 18.0), (15.5, 11.0)] {
                s.path(
                    &[(17.5, 5.5), (to.0 * 0.5 + 8.6, to.1 * 0.4 + 4.0), to],
                    |_| 0.8,
                    |_, _| (stem, 0.0, 0.0),
                );
            }
            s.stroke(
                (17.5, 5.5),
                (25.5, 6.0),
                |t| 2.6 * (t * PI).sin().powf(0.7),
                |_, lit| (leaf, 0.0, lit.signum() * 0.3),
            );
            for (x, y) in [(15.5, 13.5), (9.5, 20.5), (20.5, 21.5)] {
                s.disc(x, y, 5.0, |d, lit| flat(lit * 0.75 - d * d * 0.3 + 0.1));
                s.glow(x as i32 - 2, y as i32 - 3, [1.0, 0.90, 0.88]);
                s.glow(x as i32 - 3, y as i32 - 2, [1.0, 0.78, 0.76]);
            }
        }
        Icon::Meat => {
            // Raw and cooked meat share an icon. Cooked meat is brown: it
            // has much less blue than green.
            let cooked = item.color[2] < item.color[1] * 0.8;
            let bone = [0.92, 0.88, 0.78];
            let fat = [0.97, 0.87, 0.80];
            s.stroke(
                (12.5, 19.5),
                (6.0, 26.0),
                |_| 1.5,
                |_, lit| (bone, 0.0, lit * 0.4),
            );
            s.disc(4.8, 25.2, 1.9, |_, lit| (bone, 0.0, lit * 0.4));
            s.disc(6.8, 27.2, 1.9, |_, lit| (bone, 0.0, lit * 0.4 - 0.1));
            s.paint(|x, y| {
                let (dx, dy) = (x - 18.5, y - 13.0);
                let (u, v) = ((dx - dy) * DIAG, (dx + dy) * DIAG);
                let d = (u / 10.5).hypot(v / 7.6);
                if d > 1.0 {
                    return None;
                }
                let light = -(dx + dy) / 16.0;
                if !cooked && d > 0.80 {
                    return Some((fat, 0.0, light * 0.5));
                }
                // marbling when raw, grill marks when cooked
                let line = (v + (u * 0.5).sin() * 1.2 + 20.0).rem_euclid(4.6) < 1.1;
                let mark = if !line {
                    0.0
                } else if cooked {
                    -0.7
                } else {
                    0.6
                };
                Some(flat(light * 0.7 + 0.1 + mark - d * d * 0.2))
            });
        }
        _ => {
            if item.block.is_some() {
                // a cube seen from a corner
                s.poly(
                    &[(16.0, 3.0), (28.0, 9.0), (16.0, 15.0), (4.0, 9.0)],
                    |_, _| flat(0.6),
                );
                s.poly(
                    &[(4.0, 9.0), (16.0, 15.0), (16.0, 29.0), (4.0, 23.0)],
                    |_, y| flat(0.05 - (y - 9.0) * 0.012),
                );
                s.poly(
                    &[(16.0, 15.0), (28.0, 9.0), (28.0, 23.0), (16.0, 29.0)],
                    |_, y| flat(-0.45 - (y - 9.0) * 0.012),
                );
            } else {
                lump(&mut s, col, seed);
            }
        }
    }
    if glows {
        // ember things are hot: no shadow goes darker than a deep orange
        for v in s.v.iter_mut() {
            *v = *v * 0.8 + 0.3;
        }
    }
    let mut img = s.render(0.5);
    outline(&mut img);
    if glows {
        halo(&mut img, item.color);
    }
    for &(x, y) in sparkles {
        sparkle(&mut img, x, y, item.color);
    }
    // uncommon and better: a small glint in the colour of the quality
    if item.quality >= 2 {
        sparkle(&mut img, 26, 6, quality_color(item.quality));
        if item.quality >= 4 {
            sparkle(&mut img, 6, 24, quality_color(item.quality));
        }
    }
    img.rgba()
}

#[cfg(test)]
mod dump_tests {
    /// Write every block face to one image (top, side, bottom per block,
    /// eight blocks a row). Set DYNO_TEX_DUMP to the output .ppm path and
    /// run the ignored test `dump_block_faces`.
    #[test]
    #[ignore]
    fn dump_block_faces() {
        let Some(out) = std::env::var_os("DYNO_TEX_DUMP") else {
            return;
        };
        let t = super::Textures::blocks();
        let n = t.block_faces.len();
        let s = super::TILE;
        let (w, h) = (s * 3 * 8, s * n.div_ceil(8));
        let mut img = vec![0u8; w * h * 3];
        for (id, f) in t.block_faces.iter().enumerate() {
            for (k, layer) in f.iter().enumerate() {
                let l = &t.layers[*layer as usize];
                let (ox, oy) = ((id % 8) * 3 * s + k * s, (id / 8) * s);
                for y in 0..s {
                    for x in 0..s {
                        let p = &l[(y * s + x) * 4..];
                        let q = ((oy + y) * w + ox + x) * 3;
                        img[q..q + 3].copy_from_slice(&p[..3]);
                    }
                }
            }
        }
        let mut f = format!("P6 {w} {h} 255\n").into_bytes();
        f.extend_from_slice(&img);
        std::fs::write(out, f).unwrap();
    }
}
