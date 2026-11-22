//! Section meshing. The world stays a 1 m block grid; this turns it into a
//! softer picture:
//!
//! - natural ground is a smooth surface (surface nets with the topology of the
//!   cube mesh: every exposed block face becomes one quad whose corners sit at
//!   positions computed per grid corner);
//! - leaf blocks get the same treatment on their own field, puffed up;
//! - everything else stays a cube;
//! - water gets a lowered surface that tucks under the shore.
//!
//! A corner position depends only on the blocks around that corner, so two
//! neighbouring sections always agree and the borders are seamless.

use crate::light::Light;
use crate::render::{self, KIND_FOLIAGE, KIND_TERRAIN, KIND_WATER, SECTION, Vertex};
use dyno_world::{self as w, BlockId, World};
use glam::Vec3;

const N: i32 = SECTION;
/// Cells read around the section.
const PAD: i32 = 3;
const G: usize = (N + 2 * PAD) as usize;
/// Grid corners -1..=17 on each axis.
const LV: usize = (N + 3) as usize;
/// Blocks -2..=17 on each axis have a density.
const DB: usize = (N + 4) as usize;

/// How far outside a section its mesh reads blocks. A block edit must re-mesh
/// every section within this distance.
pub const REACH: i32 = PAD + 1;

const F_TERRAIN: u8 = 1;
/// Drawn as a cube: ground corners that touch it stay on the grid.
const F_PIN: u8 = 2;
const F_OPAQUE: u8 = 4;
const F_LEAVES: u8 = 8;
const F_WATER: u8 = 16;

/// Smallest density magnitude of a ground cell. Keeps one-block features
/// from thinning to nothing.
const MARGIN: f32 = 0.15;
/// Leaf canopies: the surface sits where the blurred leaf field crosses this.
const LEAF_ISO: f32 = 0.33;
const LEAF_IN: f32 = 0.2;
const LEAF_OUT: f32 = 0.08;
/// How much of its smoothing a corner keeps on a cliff.
const CLIFF_KEEP: f32 = 0.55;
/// How far the water surface sits below the top of its block.
pub const WATER_DROP: f32 = 0.2;
/// Per cent of open grass blocks that grow a tuft.
const TUFT_PER_CENT: u32 = 62;

/// Natural ground: drawn smooth.
pub fn is_terrain(b: BlockId) -> bool {
    matches!(
        b,
        w::GRASS
            | w::DIRT
            | w::STONE
            | w::SAND
            | w::EMBERSTONE
            | w::SNOW
            | 15 // basalt
            | 16 // marble
            | w::CLAY
            | w::BEDROCK
            | w::COAL_ORE
            | w::IRON_ORE
    )
}

fn flags() -> &'static [u8; 256] {
    static T: std::sync::OnceLock<[u8; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0u8; 256];
        for (b, f) in t.iter_mut().enumerate() {
            let b = b as BlockId;
            if b == w::AIR || b >= w::BLOCK_COUNT {
                continue;
            }
            if w::is_opaque(b) {
                *f |= F_OPAQUE;
            }
            if is_terrain(b) {
                *f |= F_TERRAIN;
            } else if w::is_leaves(b) {
                *f |= F_LEAVES;
            } else if b == w::WATER {
                *f |= F_WATER;
            } else {
                *f |= F_PIN;
            }
        }
        t
    })
}

/// A tuft of grass (or a flower) to draw near the camera.
#[derive(Clone, Copy)]
pub struct Tuft {
    pub pos: [f32; 3],
    pub light: f32,
    pub sky: f32,
    pub seed: u32,
}

/// A soft halo around a lamp.
#[derive(Clone, Copy)]
pub struct Glow {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub size: f32,
}

#[derive(Default)]
pub struct SectionGeo {
    pub opaque: Vec<Vertex>,
    pub trans: Vec<Vertex>,
    pub tufts: Vec<Tuft>,
    pub glows: Vec<Glow>,
}

#[inline]
pub fn hash3(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

/// The texture layer a ground block shows on steep faces: earth under grass,
/// rock under snow, its own side otherwise.
fn steep_layer(b: BlockId) -> u16 {
    let t = render::textures();
    match b {
        w::GRASS => t.face(w::DIRT, 1),
        w::SNOW => t.face(w::STONE, 1),
        _ => t.face(b, 1),
    }
}

/// What a ground block is painted as. Earth just under grass counts as
/// grass, and rock just under snow as snow: on a slope the cover runs on
/// down over it, and only a steep face shows what is underneath.
#[inline]
fn material(g: &Grid, c: usize) -> BlockId {
    match (g.ids[c], g.ids[c + DY]) {
        (w::DIRT, w::GRASS) => w::GRASS,
        (w::STONE, w::SNOW) => w::SNOW,
        (b, _) => b,
    }
}

/// The blocks of one section and a border around it, in section-local cells.
struct Grid {
    ids: Vec<BlockId>,
    fl: Vec<u8>,
    /// world position of local cell (0, 0, 0)
    o: [i32; 3],
}

const DX: usize = 1;
const DZ: usize = G;
const DY: usize = G * G;

#[inline]
fn at(x: i32, y: i32, z: i32) -> usize {
    ((y + PAD) as usize * G + (z + PAD) as usize) * G + (x + PAD) as usize
}

#[inline]
fn step(c: usize, d: [i32; 3]) -> usize {
    (c as isize + d[0] as isize + d[2] as isize * DZ as isize + d[1] as isize * DY as isize)
        as usize
}

type Face = ([i32; 3], [i32; 3], [i32; 3], f32);
/// normal, u axis, v axis, face shade (as the cube mesher has always had them)
const FACES: [Face; 6] = [
    ([1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
    ([-1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
    ([0, 1, 0], [1, 0, 0], [0, 0, 1], 1.00),
    ([0, -1, 0], [1, 0, 0], [0, 0, 1], 0.55),
    ([0, 0, 1], [1, 0, 0], [0, 1, 0], 0.68),
    ([0, 0, -1], [1, 0, 0], [0, 1, 0], 0.68),
];
const QUAD: [(i32, i32); 4] = [(-1, -1), (1, -1), (1, 1), (-1, 1)];

#[inline]
fn li(i: i32, j: i32, k: i32) -> usize {
    ((j + 1) as usize * LV + (k + 1) as usize) * LV + (i + 1) as usize
}

#[inline]
fn di(x: i32, y: i32, z: i32) -> usize {
    ((y + 2) as usize * DB + (z + 2) as usize) * DB + (x + 2) as usize
}

/// The eight cells around a grid corner, as offsets from the lowest one:
/// bit 0 is +x, bit 1 is +z, bit 2 is +y.
const CELL8: [usize; 8] = [0, DX, DZ, DZ + DX, DY, DY + DX, DY + DZ, DY + DZ + DX];

/// One smooth surface over the grid: the ground, or the leaves.
struct Nets<'a> {
    g: &'a Grid,
    leaves: bool,
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    /// bit 0: position known; bit 1: light known
    state: Vec<u8>,
    dens: Vec<f32>,
    lit: Vec<[f32; 3]>,
}

struct Quad {
    block: BlockId,
    dir: u8,
    /// grid corners, in order around the face
    c: [[i8; 3]; 4],
    /// cell of the block, local
    cell: [i8; 3],
}

impl<'a> Nets<'a> {
    fn new(g: &'a Grid, leaves: bool) -> Self {
        let n = LV * LV * LV;
        Nets {
            g,
            leaves,
            pos: vec![[0.0; 3]; n],
            nrm: vec![[0.0; 3]; n],
            state: vec![0; n],
            dens: vec![f32::NAN; DB * DB * DB],
            lit: vec![[0.0; 3]; n],
        }
    }

    /// Smoothed field at a cell: positive inside, negative outside, never
    /// the wrong sign for the cell.
    fn density(&mut self, x: i32, y: i32, z: i32) -> f32 {
        let k = di(x, y, z);
        let d = self.dens[k];
        if !d.is_nan() {
            return d;
        }
        let g = self.g;
        let c = at(x, y, z);
        let d = if self.leaves {
            // leaves: blur in all directions, so canopies come out round
            let mut s = 0u32;
            for oy in [c - DY, c, c + DY] {
                for oz in [oy - DZ, oy, oy + DZ] {
                    for q in [oz - DX, oz, oz + DX] {
                        s += (g.fl[q] & F_LEAVES != 0) as u32;
                    }
                }
            }
            let d = s as f32 / 27.0 - LEAF_ISO;
            if g.fl[c] & F_LEAVES != 0 {
                d.max(LEAF_IN)
            } else {
                d.min(-LEAF_OUT)
            }
        } else {
            // ground: blur each layer on its own, so flat ground stays at the
            // block top and walls stay upright; built blocks do not count
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
            if g.fl[c] & F_TERRAIN != 0 {
                d.max(MARGIN)
            } else {
                d.min(-MARGIN)
            }
        };
        self.dens[k] = d;
        d
    }

    /// True when a grid corner lies on a wall of ground at least two blocks
    /// high: two side-by-side columns differ the same way in two layers
    /// running up or down from the corner.
    fn on_cliff(&self, i: i32, j: i32, k: i32) -> bool {
        let g = self.g;
        let solid = |c: usize| (g.fl[c] & F_TERRAIN != 0) as i8;
        let base = at(i - 1, j - 1, k - 1);
        // the four pairs of neighbouring columns round the corner
        for (a, b) in [(0, DX), (DZ, DZ + DX), (0, DZ), (DX, DX + DZ)] {
            // the layer just below the corner, the one below that, and the two above
            let below = solid(base + a) - solid(base + b);
            let above = solid(base + DY + a) - solid(base + DY + b);
            if below != 0
                && (below == solid(base - DY + a) - solid(base - DY + b) || below == above)
            {
                return true;
            }
            if above != 0 && above == solid(base + 2 * DY + a) - solid(base + 2 * DY + b) {
                return true;
            }
        }
        false
    }

    /// World position of a grid corner. It is the whole-number corner plus an
    /// offset that depends only on the blocks around it, so every section
    /// that shares the corner gets the same bits.
    fn vertex(&mut self, i: i32, j: i32, k: i32) -> [f32; 3] {
        let idx = li(i, j, k);
        if self.state[idx] & 1 != 0 {
            return self.pos[idx];
        }
        let mut p = [0f32; 3];
        let g = self.g;
        let want = if self.leaves { F_LEAVES } else { F_TERRAIN };
        let base = at(i - 1, j - 1, k - 1);
        let (mut cnt, mut any) = (0u32, 0u8);
        for o in CELL8 {
            let f = g.fl[base + o];
            cnt += (f & want != 0) as u32;
            any |= f;
        }
        let pinned = !self.leaves && any & F_PIN != 0;
        if !pinned && cnt != 0 && cnt != 8 {
            let mut d = [0f32; 8];
            for (c, d) in d.iter_mut().enumerate() {
                *d = self.density(
                    i - 1 + (c & 1) as i32,
                    j - 1 + ((c >> 2) & 1) as i32,
                    k - 1 + ((c >> 1) & 1) as i32,
                );
            }
            // mean of the points where the surface crosses the cell's edges
            let mut sum = [0f32; 3];
            let mut n = 0u32;
            for (bit, axis) in [(1usize, 0usize), (2, 2), (4, 1)] {
                for a in 0..8usize {
                    if a & bit != 0 {
                        continue;
                    }
                    let (da, db) = (d[a], d[a | bit]);
                    if (da > 0.0) == (db > 0.0) {
                        continue;
                    }
                    let mut q = [
                        (a & 1) as f32 - 0.5,
                        ((a >> 2) & 1) as f32 - 0.5,
                        ((a >> 1) & 1) as f32 - 0.5,
                    ];
                    q[axis] = da / (da - db) - 0.5;
                    sum[0] += q[0];
                    sum[1] += q[1];
                    sum[2] += q[2];
                    n += 1;
                }
            }
            if n > 0 {
                // on a wall two or more blocks high, stay close to the grid:
                // cliffs keep their edge, single steps melt into slopes
                let keep = if !self.leaves && self.on_cliff(i, j, k) {
                    CLIFF_KEEP
                } else {
                    1.0
                } / n as f32;
                p[0] += sum[0] * keep;
                p[1] += sum[1] * keep;
                p[2] += sum[2] * keep;
            }
            if self.leaves {
                // a little disorder, the same from whichever section looks
                let h = hash3(g.o[0] + i, g.o[1] + j, g.o[2] + k);
                let r = |s: u32| ((h >> s) & 255) as f32 / 255.0 - 0.5;
                p[0] += r(0) * 0.22;
                p[1] += r(8) * 0.22;
                p[2] += r(16) * 0.22;
            }
        }
        let p = [
            (g.o[0] + i) as f32 + p[0],
            (g.o[1] + j) as f32 + p[1],
            (g.o[2] + k) as f32 + p[2],
        ];
        self.pos[idx] = p;
        self.state[idx] |= 1;
        p
    }

    /// Block light, sky light and ambient occlusion at a grid corner.
    fn light(&mut self, light: &Light, c: [i8; 3], n: Vec3) -> [f32; 3] {
        let (i, j, k) = (c[0] as i32, c[1] as i32, c[2] as i32);
        let idx = li(i, j, k);
        if self.state[idx] & 2 != 0 {
            return self.lit[idx];
        }
        let g = self.g;
        let base = at(i - 1, j - 1, k - 1);
        let (mut bl, mut sl, mut open) = (0u32, 0u32, 0u32);
        for (ci, o) in CELL8.iter().enumerate() {
            if g.fl[base + o] & F_OPAQUE != 0 {
                continue;
            }
            let (x, y, z) = (
                g.o[0] + i - 1 + (ci & 1) as i32,
                g.o[1] + j - 1 + ((ci >> 2) & 1) as i32,
                g.o[2] + k - 1 + ((ci >> 1) & 1) as i32,
            );
            bl += light.get(x, y, z) as u32;
            sl += light.sky(x, y, z) as u32;
            open += 1;
        }
        let k15 = 1.0 / (open.max(1) as f32 * 15.0);
        // occlusion: how closed in the corner is, one step out along the normal
        // (so an even slope shades evenly, and the foot of a wall goes darker)
        let ao = if self.leaves {
            // canopies: darker in the folds between the puffs, lighter on them
            let leafy = CELL8
                .iter()
                .filter(|o| g.fl[base + **o] & F_LEAVES != 0)
                .count();
            (1.12 - 0.075 * leafy as f32).min(1.0)
        } else {
            let q = [
                i + (n.x * 0.9).round() as i32,
                j + (n.y * 0.9).round() as i32,
                k + (n.z * 0.9).round() as i32,
            ];
            let base = at(q[0] - 1, q[1] - 1, q[2] - 1);
            let solid = CELL8
                .iter()
                .filter(|o| g.fl[base + **o] & F_OPAQUE != 0)
                .count();
            (1.0 - 0.075 * solid as f32).max(0.5)
        };
        let v = [bl as f32 * k15, sl as f32 * k15, ao];
        self.lit[idx] = v;
        self.state[idx] |= 2;
        v
    }

    /// The kinds of ground that meet at a grid corner: (block, how many) over
    /// the exposed ground blocks round it.
    fn census(&self, c: [i8; 3]) -> [(BlockId, u8); 4] {
        let g = self.g;
        let base = at(c[0] as i32 - 1, c[1] as i32 - 1, c[2] as i32 - 1);
        let mut out = [(0u8, 0u8); 4];
        for o in CELL8 {
            let q = base + o;
            if g.fl[q] & F_TERRAIN == 0 {
                continue;
            }
            let open = [q - DX, q + DX, q - DZ, q + DZ, q - DY, q + DY]
                .iter()
                .any(|n| g.fl[*n] & F_OPAQUE == 0);
            if !open {
                continue;
            }
            let id = material(g, q);
            if let Some(e) = out.iter_mut().find(|e| e.1 == 0 || e.0 == id) {
                *e = (id, e.1 + 1);
            }
        }
        out
    }

    /// Mesh the surface: quads for the section's own blocks, with normals
    /// that also take in the faces just outside it.
    fn run(&mut self, light: &Light, geo: &mut SectionGeo) {
        let g = self.g;
        let (want, hide) = if self.leaves {
            (F_LEAVES, F_OPAQUE | F_LEAVES)
        } else {
            (F_TERRAIN, F_OPAQUE)
        };
        let mut quads: Vec<Quad> = Vec::new();
        for y in -1..=N {
            for z in -1..=N {
                for x in -1..=N {
                    let c = at(x, y, z);
                    if g.fl[c] & want == 0 {
                        continue;
                    }
                    let own = (0..N).contains(&x) && (0..N).contains(&y) && (0..N).contains(&z);
                    for (dir, (n, u, v, _)) in FACES.iter().enumerate() {
                        if g.fl[step(c, *n)] & hide != 0 {
                            continue;
                        }
                        let mut cs = [[0i8; 3]; 4];
                        let mut ps = [Vec3::ZERO; 4];
                        for (q, (su, sv)) in QUAD.iter().enumerate() {
                            let l = [
                                x + (n[0] + su * u[0] + sv * v[0] + 1) / 2,
                                y + (n[1] + su * u[1] + sv * v[1] + 1) / 2,
                                z + (n[2] + su * u[2] + sv * v[2] + 1) / 2,
                            ];
                            cs[q] = [l[0] as i8, l[1] as i8, l[2] as i8];
                            ps[q] = Vec3::from(self.vertex(l[0], l[1], l[2]));
                        }
                        // area-weighted normal, pointing out of the block
                        let mut fnorm = (ps[2] - ps[0]).cross(ps[3] - ps[1]);
                        let nf = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32);
                        if fnorm.dot(nf) < 0.0 {
                            fnorm = -fnorm;
                        }
                        for c in cs {
                            let a = &mut self.nrm[li(c[0] as i32, c[1] as i32, c[2] as i32)];
                            a[0] += fnorm.x;
                            a[1] += fnorm.y;
                            a[2] += fnorm.z;
                        }
                        if own {
                            quads.push(Quad {
                                block: if self.leaves {
                                    g.ids[c]
                                } else {
                                    material(g, c)
                                },
                                dir: dir as u8,
                                c: cs,
                                cell: [x as i8, y as i8, z as i8],
                            });
                        }
                    }
                }
            }
        }
        let t = render::textures();
        geo.opaque.reserve(quads.len() * 6);
        for q in &quads {
            let b = q.block;
            let n = FACES[q.dir as usize].0;
            let nf = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32);
            let mut ps = [Vec3::ZERO; 4];
            let mut ns = [Vec3::ZERO; 4];
            let mut ls = [[0f32; 3]; 4];
            for i in 0..4 {
                let c = q.c[i];
                let idx = li(c[0] as i32, c[1] as i32, c[2] as i32);
                ps[i] = Vec3::from(self.pos[idx]);
                let nn = Vec3::from(self.nrm[idx]).normalize_or_zero();
                ns[i] = if nn == Vec3::ZERO { nf } else { nn };
                ls[i] = self.light(light, c, ns[i]);
            }
            // wind the corners anticlockwise seen from outside
            let mut order = [0usize, 1, 2, 3];
            if (ps[2] - ps[0]).cross(ps[3] - ps[1]).dot(nf) < 0.0 {
                order = [0, 3, 2, 1];
            }
            // split along the shorter diagonal
            let [a, b1, c, d] = order;
            let tris = if ps[a].distance_squared(ps[c]) <= ps[b1].distance_squared(ps[d]) {
                [a, b1, c, a, c, d]
            } else {
                [b1, c, d, b1, d, a]
            };
            let top = t.face(b, 0);
            if self.leaves {
                let tex = [0.0, 0.0, KIND_FOLIAGE + top as f32, render::layer_anim(top)];
                for i in tris {
                    geo.opaque.push(Vertex {
                        pos: ps[i].into(),
                        normal: ns[i].into(),
                        color: [1.0, 1.0, 1.0, ls[i][2]],
                        light: ls[i][0],
                        sky: ls[i][1],
                        tex,
                    });
                }
                continue;
            }
            let glow = w::emission(b) > 0;
            let anim = render::layer_anim(top);
            // a neighbouring kind of ground to fade into, and how much of it
            // there is at each corner
            let mut mix = [0f32; 4];
            let mut other = None;
            if !glow && anim == 0.0 {
                let cs = q.c.map(|c| self.census(c));
                let mut best = 0u32;
                for (id, _) in cs.iter().flatten().filter(|(id, n)| *n > 0 && *id != b) {
                    let total: u32 = cs
                        .iter()
                        .flatten()
                        .filter(|(i, _)| i == id)
                        .map(|(_, n)| *n as u32)
                        .sum();
                    if total > best || (total == best && Some(*id) < other) {
                        best = total;
                        other = Some(*id);
                    }
                }
                if let Some(o) = other {
                    for (m, c) in mix.iter_mut().zip(&cs) {
                        let count = |id: BlockId| {
                            c.iter()
                                .find(|(i, n)| *i == id && *n > 0)
                                .map_or(0.0, |(_, n)| *n as f32)
                        };
                        let (mine, theirs) = (count(b), count(o));
                        *m = theirs / (mine + theirs).max(1.0);
                    }
                }
            }
            let (ob_top, ob_steep) = match other {
                Some(o) => (t.face(o, 0) as f32, steep_layer(o) as f32),
                None => (0.0, 0.0),
            };
            let tex = [
                ob_top,
                steep_layer(b) as f32,
                KIND_TERRAIN + top as f32,
                anim,
            ];
            let mut put = |p: Vec3, n: Vec3, l: [f32; 3], m: f32| {
                geo.opaque.push(Vertex {
                    pos: p.into(),
                    normal: n.into(),
                    color: [m, ob_steep, 0.0, if glow { 2.0 } else { l[2] }],
                    light: l[0],
                    sky: l[1],
                    tex,
                });
            };
            if other.is_some() {
                // a fan round the middle, which is all this block's own ground
                let pm = (ps[0] + ps[1] + ps[2] + ps[3]) * 0.25;
                let nm = (ns[0] + ns[1] + ns[2] + ns[3]).normalize_or(nf);
                let lm = [0, 1, 2].map(|k| (ls[0][k] + ls[1][k] + ls[2][k] + ls[3][k]) * 0.25);
                for e in 0..4 {
                    let (i0, i1) = (order[e], order[(e + 1) % 4]);
                    put(ps[i0], ns[i0], ls[i0], mix[i0]);
                    put(ps[i1], ns[i1], ls[i1], mix[i1]);
                    put(pm, nm, lm, 0.0);
                }
            } else {
                for i in tris {
                    put(ps[i], ns[i], ls[i], 0.0);
                }
            }
            // grass tufts on open, fairly level grass
            if b == w::GRASS && q.dir == 2 {
                // (earth painted as grass never gets here: its top is covered)
                let [x, y, z] = [q.cell[0] as i32, q.cell[1] as i32, q.cell[2] as i32];
                let (wx, wy, wz) = (g.o[0] + x, g.o[1] + y, g.o[2] + z);
                let h = hash3(wx, wy, wz);
                let up = (ns[0] + ns[1] + ns[2] + ns[3]).normalize_or_zero();
                if h % 100 < TUFT_PER_CENT && g.ids[at(x, y + 1, z)] == w::AIR && up.y > 0.8 {
                    // QUAD order on the top face: (x, z), (x+1, z), (x+1, z+1), (x, z+1)
                    let fx = 0.15 + 0.7 * ((h >> 8) & 255) as f32 / 255.0;
                    let fz = 0.15 + 0.7 * ((h >> 16) & 255) as f32 / 255.0;
                    let p = ps[0].lerp(ps[1], fx).lerp(ps[3].lerp(ps[2], fx), fz);
                    geo.tufts.push(Tuft {
                        pos: p.into(),
                        light: light.get(wx, wy + 1, wz) as f32 / 15.0,
                        sky: light.sky(wx, wy + 1, wz) as f32 / 15.0,
                        seed: h,
                    });
                }
            }
        }
    }
}

/// True for a water cell whose top is open: it has a surface.
#[inline]
fn water_surface(g: &Grid, c: usize) -> bool {
    g.fl[c] & F_WATER != 0 && g.fl[c + DY] & (F_WATER | F_OPAQUE) == 0
}

/// Cubes, posts and water for the section's own blocks.
fn cubes(g: &Grid, light: &Light, has_water: bool, geo: &mut SectionGeo) {
    let lget = |p: [i32; 3]| light.get(g.o[0] + p[0], g.o[1] + p[1], g.o[2] + p[2]) as f32;
    let lsky = |p: [i32; 3]| light.sky(g.o[0] + p[0], g.o[1] + p[1], g.o[2] + p[2]) as f32;
    let solid = |p: [i32; 3]| (g.fl[at(p[0], p[1], p[2])] & F_OPAQUE != 0) as i32;
    // half a texel in from the tile edge, so smooth filtering does not pull
    // in the far side of the tile
    let inset = 0.5 / crate::textures::TILE as f32;
    let tile = |t: f32| inset + (1.0 - 2.0 * inset) * t;
    let wt = render::textures().face(w::WATER, 0);
    for y in 0..N {
        for z in 0..N {
            for x in 0..N {
                let c = at(x, y, z);
                let b = g.ids[c];
                let f = g.fl[c];
                if b == w::AIR || f & F_LEAVES != 0 {
                    continue;
                }
                let wpos = [g.o[0] + x, g.o[1] + y, g.o[2] + z];
                if f & F_TERRAIN != 0 {
                    // the shore: carry the water surface on under the ground,
                    // which now slopes down into it
                    if has_water && g.fl[c + DY] & (F_WATER | F_OPAQUE) == 0 {
                        let near = [
                            c - DX,
                            c + DX,
                            c - DZ,
                            c + DZ,
                            c - DX - DZ,
                            c + DX - DZ,
                            c - DX + DZ,
                            c + DX + DZ,
                        ];
                        if near.iter().any(|q| water_surface(g, *q)) {
                            water_top(g, &lget, &lsky, [x, y, z], wt, &mut geo.trans);
                        }
                    }
                    continue;
                }
                if w::shape(b) == w::Shape::Post {
                    let p = Vec3::new(
                        wpos[0] as f32 + 0.5,
                        wpos[1] as f32 + 11.0 / 32.0,
                        wpos[2] as f32 + 0.5,
                    );
                    render::push_post(&mut geo.opaque, p, b, lget([x, y, z]) / 15.0);
                    geo.glows.push(Glow {
                        pos: [p.x, wpos[1] as f32 + 0.62, p.z],
                        color: w::block(b).color,
                        size: 0.9,
                    });
                    continue;
                }
                let alpha = w::block(b).alpha;
                let tr = alpha < 1.0;
                let emissive = w::emission(b) > 0;
                if emissive {
                    geo.glows.push(Glow {
                        pos: [
                            wpos[0] as f32 + 0.5,
                            wpos[1] as f32 + 0.5,
                            wpos[2] as f32 + 0.5,
                        ],
                        color: w::block(b).color,
                        size: 0.9 + w::emission(b) as f32 * 0.06,
                    });
                }
                let water = b == w::WATER;
                let surface = water && g.fl[c + DY] & F_WATER == 0;
                for (n, u, v, shade) in FACES {
                    let nc = step(c, n);
                    if g.fl[nc] & F_OPAQUE != 0 || (tr && g.ids[nc] == b) {
                        continue;
                    }
                    if water && n[1] == 1 {
                        water_top(g, &lget, &lsky, [x, y, z], wt, &mut geo.trans);
                        continue;
                    }
                    let base = [x + n[0], y + n[1], z + n[2]];
                    let center = [
                        wpos[0] as f32 + 0.5 + n[0] as f32 * 0.5,
                        wpos[1] as f32 + 0.5 + n[1] as f32 * 0.5,
                        wpos[2] as f32 + 0.5 + n[2] as f32 * 0.5,
                    ];
                    let mut corners = [([0f32; 3], 0f32, 0f32, 0f32); 4];
                    for (i, (su, sv)) in QUAD.into_iter().enumerate() {
                        let mut p = [
                            center[0] + 0.5 * (su * u[0] + sv * v[0]) as f32,
                            center[1] + 0.5 * (su * u[1] + sv * v[1]) as f32,
                            center[2] + 0.5 * (su * u[2] + sv * v[2]) as f32,
                        ];
                        if surface && p[1] > wpos[1] as f32 + 0.5 {
                            p[1] -= WATER_DROP;
                        }
                        let a = [
                            base[0] + su * u[0],
                            base[1] + su * u[1],
                            base[2] + su * u[2],
                        ];
                        let bq = [
                            base[0] + sv * v[0],
                            base[1] + sv * v[1],
                            base[2] + sv * v[2],
                        ];
                        let cq = [
                            base[0] + su * u[0] + sv * v[0],
                            base[1] + su * u[1] + sv * v[1],
                            base[2] + su * u[2] + sv * v[2],
                        ];
                        // smooth light: average the cells around this corner
                        let l0 = lget(base);
                        let bl = ((l0 + lget(a) + lget(bq) + lget(cq)) / 60.0).max(l0 / 15.0 * 0.8);
                        let k0 = lsky(base);
                        let sl = ((k0 + lsky(a) + lsky(bq) + lsky(cq)) / 60.0).max(k0 / 15.0 * 0.8);
                        let shade_ao = if emissive {
                            2.0
                        } else if tr {
                            alpha
                        } else {
                            let (s1, s2, s3) = (solid(a), solid(bq), solid(cq));
                            let ao = if s1 == 1 && s2 == 1 {
                                0
                            } else {
                                3 - (s1 + s2 + s3)
                            };
                            [0.5, 0.66, 0.83, 1.0][ao as usize]
                        };
                        corners[i] = (p, shade_ao, bl, sl);
                    }
                    let nf = [n[0] as f32, n[1] as f32, n[2] as f32];
                    // textured: the texture carries the colour
                    let col = if tr { shade } else { 1.0 };
                    let face_id = match n[1] {
                        1 => 0,
                        -1 => 2,
                        _ => 1,
                    };
                    // flip the diagonal so ambient occlusion interpolates without seams
                    let order = if corners[0].1 + corners[2].1 > corners[1].1 + corners[3].1 {
                        [0, 1, 2, 0, 2, 3]
                    } else {
                        [1, 2, 3, 1, 3, 0]
                    };
                    let layer = render::face_tex(b, face_id, 0.0, 0.0);
                    let out = if tr { &mut geo.trans } else { &mut geo.opaque };
                    for i in order {
                        let (p, l, bl, sl) = corners[i];
                        let (su, sv) = (QUAD[i].0 as f32, QUAD[i].1 as f32);
                        // sides: v runs down from the top; mirror u on the far faces
                        let (tu, tv) = if face_id == 1 {
                            let uu = (su + 1.0) * 0.5;
                            (
                                if n[0] + n[2] < 0 { uu } else { 1.0 - uu },
                                (1.0 - sv) * 0.5,
                            )
                        } else {
                            ((su + 1.0) * 0.5, (sv + 1.0) * 0.5)
                        };
                        out.push(Vertex {
                            pos: p,
                            normal: nf,
                            color: [col, col, col, l],
                            light: bl,
                            sky: sl,
                            tex: [tile(tu), tile(tv), layer[2], layer[3]],
                        });
                    }
                }
            }
        }
    }
}

/// The water surface over one cell. `normal.x` carries how close the shore
/// is (0 open water, 1 land all round), for the foam line.
fn water_top(
    g: &Grid,
    lget: &dyn Fn([i32; 3]) -> f32,
    lsky: &dyn Fn([i32; 3]) -> f32,
    cell: [i32; 3],
    layer: u16,
    out: &mut Vec<Vertex>,
) {
    let [x, y, z] = cell;
    let h = (g.o[1] + y + 1) as f32 - WATER_DROP;
    let mut vs = [Vertex {
        pos: [0.0; 3],
        normal: [0.0, 1.0, 0.0],
        color: [1.0, 1.0, 1.0, w::block(w::WATER).alpha],
        light: 0.0,
        sky: 0.0,
        tex: [
            0.0,
            0.0,
            KIND_WATER + layer as f32,
            render::layer_anim(layer),
        ],
    }; 4];
    for (i, (du, dv)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
        let (cx, cz) = (x + du, z + dv);
        // the four cells around this corner, at the water's level and above it
        let (mut wet, mut bl, mut sl) = (0u32, 0.0, 0.0);
        for (ox, oz) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
            wet += water_surface(g, at(cx + ox, y, cz + oz)) as u32;
            bl += lget([cx + ox, y + 1, cz + oz]);
            sl += lsky([cx + ox, y + 1, cz + oz]);
        }
        vs[i].pos = [(g.o[0] + cx) as f32, h, (g.o[2] + cz) as f32];
        vs[i].normal[0] = 1.0 - wet as f32 / 4.0;
        vs[i].light = bl / 60.0;
        vs[i].sky = sl / 60.0;
    }
    // split so the shore factor interpolates evenly
    let order =
        if (vs[0].normal[0] - vs[2].normal[0]).abs() <= (vs[1].normal[0] - vs[3].normal[0]).abs() {
            [0, 1, 2, 0, 2, 3]
        } else {
            [1, 2, 3, 1, 3, 0]
        };
    out.extend(order.into_iter().map(|i| vs[i]));
}

/// Build everything drawn for one 16³ section.
pub fn mesh(world: &World, light: &Light, sx: i32, sy: i32, sz: i32) -> SectionGeo {
    let mut geo = SectionGeo::default();
    let o = [sx * N, sy * N, sz * N];
    let lut = flags();
    let mut g = Grid {
        ids: vec![0; G * G * G],
        fl: vec![0; G * G * G],
        o,
    };
    let (mut own, mut all, mut every) = (0u8, 0u8, 0xffu8);
    let mut i = 0;
    for y in -PAD..N + PAD {
        let inner_y = (0..N).contains(&y);
        for z in -PAD..N + PAD {
            let inner_z = inner_y && (0..N).contains(&z);
            for x in -PAD..N + PAD {
                let b = world.get(o[0] + x, o[1] + y, o[2] + z);
                let f = lut[b as usize];
                g.ids[i] = b;
                g.fl[i] = f;
                i += 1;
                all |= f;
                every &= f;
                if inner_z && (0..N).contains(&x) {
                    own |= f;
                }
            }
        }
    }
    // nothing here, or nothing but rock all round: nothing to draw
    if own == 0 || every & F_OPAQUE != 0 {
        return geo;
    }
    if own & F_TERRAIN != 0 {
        Nets::new(&g, false).run(light, &mut geo);
    }
    if own & F_LEAVES != 0 {
        Nets::new(&g, true).run(light, &mut geo);
    }
    let has_water = all & F_WATER != 0;
    if own & (F_PIN | F_WATER) != 0 || (has_water && own & F_TERRAIN != 0) {
        cubes(&g, light, has_water, &mut geo);
    }
    geo
}

/// A made-up landscape for tests and timing: rolling hills, a lake with
/// beaches, cliffs, trees and a few built blocks.
#[cfg(test)]
pub(crate) fn test_world(size: [u32; 3]) -> World {
    let mut world = World::empty(size);
    let [sx, sy, sz] = size.map(|v| v as i32);
    let water = 26;
    for z in 0..sz {
        for x in 0..sx {
            let (fx, fz) = (x as f32, z as f32);
            let mut h = 30.0
                + 9.0 * (fx * 0.031).sin() * (fz * 0.027).cos()
                + 5.0 * (fx * 0.083 + 1.3).sin()
                + 4.0 * (fz * 0.071 + 0.4).cos()
                + 14.0 * ((fx * 0.011).sin() * (fz * 0.013).sin()).powi(3);
            if (fx * 0.02).sin() + (fz * 0.017).cos() > 1.55 {
                h += 9.0; // a mesa, for cliffs
            }
            let h = (h as i32).clamp(4, sy - 14);
            for y in 0..=h {
                let b = if y == h {
                    if h <= water + 1 {
                        w::SAND
                    } else if h > 50 {
                        w::SNOW
                    } else {
                        w::GRASS
                    }
                } else if y > h - 4 {
                    if h <= water + 1 { w::SAND } else { w::DIRT }
                } else if hash3(x / 2, y / 2, z / 2).is_multiple_of(97) {
                    w::COAL_ORE
                } else {
                    w::STONE
                };
                world.set(x, y, z, b);
            }
            for y in h + 1..=water {
                world.set(x, y, z, w::WATER);
            }
            let r = hash3(x, 0, z);
            if h > water + 1
                && h < 50
                && r.is_multiple_of(61)
                && x > 3
                && z > 3
                && x < sx - 4
                && z < sz - 4
            {
                for y in h + 1..h + 5 {
                    world.set(x, y, z, w::LOG);
                }
                for (dx, dy, dz) in (-2..=2)
                    .flat_map(|dx| (3..=6).flat_map(move |dy| (-2..=2).map(move |dz| (dx, dy, dz))))
                {
                    let far = dx * dx + dz * dz + (dy - 4) * (dy - 4);
                    if far <= 6 && world.get(x + dx, h + dy, z + dz) == w::AIR {
                        world.set(x + dx, h + dy, z + dz, w::LEAVES);
                    }
                }
            } else if h > water + 1 && r.is_multiple_of(211) {
                world.set(x, h + 1, z, w::COBBLE);
                world.set(x, h + 2, z, w::TORCH);
            }
        }
    }
    world
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Ground, built blocks and air in lumps, with caves and overhangs. All
    /// of it opaque, so the mesh should be a closed skin. The bottom layer
    /// is air: nothing touches the rock under the world.
    fn lumpy() -> World {
        let mut world = World::empty([48, 48, 48]);
        for z in 0..48 {
            for x in 0..48 {
                let hill = 14 + (hash3(x / 5, 0, z / 5) % 14) as i32;
                for y in 1..46 {
                    let lump = hash3(x / 3, y / 3, z / 3) % 100;
                    let fine = hash3(x, y, z) % 100;
                    let b = if y < hill {
                        if lump < 12 {
                            w::AIR
                        } else if fine < 3 {
                            w::COBBLE
                        } else if lump < 40 {
                            w::STONE
                        } else if y < hill - 1 {
                            w::DIRT
                        } else if lump < 55 {
                            w::SAND
                        } else {
                            w::GRASS
                        }
                    } else if lump < 6 {
                        w::STONE
                    } else if fine < 1 {
                        w::PLANKS
                    } else {
                        w::AIR
                    };
                    world.set(x, y, z, b);
                }
            }
        }
        world
    }

    fn all_sections(world: &World) -> Vec<SectionGeo> {
        let light = Light::new(world);
        let d = world.size.map(|v| v as i32 / N);
        let mut out = Vec::new();
        for sy in 0..d[1] {
            for sz in 0..d[2] {
                for sx in 0..d[0] {
                    out.push(mesh(world, &light, sx, sy, sz));
                }
            }
        }
        out
    }

    #[test]
    fn the_skin_has_no_holes() {
        let geos = all_sections(&lumpy());
        let mut edges: HashMap<([u32; 3], [u32; 3]), u32> = HashMap::new();
        let mut triangles = 0;
        for geo in &geos {
            assert!(geo.trans.is_empty());
            for tri in geo.opaque.as_chunks::<3>().0 {
                triangles += 1;
                for e in 0..3 {
                    let a = tri[e].pos.map(f32::to_bits);
                    let b = tri[(e + 1) % 3].pos.map(f32::to_bits);
                    assert_ne!(a, b, "a triangle has collapsed");
                    *edges.entry((a.min(b), a.max(b))).or_default() += 1;
                }
            }
        }
        assert!(triangles > 10_000);
        // every edge is shared by an even number of triangles, bit for bit:
        // no cracks inside a section, between sections, or where ground
        // meets built blocks
        let open = edges.values().filter(|n| *n % 2 == 1).count();
        assert_eq!(open, 0, "{open} open edges of {}", edges.len());
    }

    #[test]
    fn sections_agree_at_their_borders() {
        let geos = all_sections(&lumpy());
        // same corner, same normal and light, whichever section drew it
        let mut seen: HashMap<[u32; 3], [u32; 6]> = HashMap::new();
        let mut shared = 0;
        for geo in &geos {
            let mut mine: HashMap<[u32; 3], [u32; 6]> = HashMap::new();
            for v in geo.opaque.iter().filter(|v| v.tex[2] >= KIND_TERRAIN) {
                let at = v.pos.map(f32::to_bits);
                let what = [
                    v.normal[0],
                    v.normal[1],
                    v.normal[2],
                    v.light,
                    v.sky,
                    v.color[3],
                ]
                .map(f32::to_bits);
                assert_eq!(*mine.entry(at).or_insert(what), what);
            }
            for (at, what) in mine {
                if let Some(old) = seen.insert(at, what) {
                    assert_eq!(old, what, "two sections disagree at a shared corner");
                    shared += 1;
                }
            }
        }
        assert!(shared > 500, "only {shared} shared corners");
    }

    #[test]
    fn flat_ground_sits_on_the_grid() {
        let mut world = World::empty([48, 32, 48]);
        for z in 0..48 {
            for x in 0..48 {
                for y in 0..10 {
                    world.set(x, y, z, if y == 9 { w::GRASS } else { w::DIRT });
                }
            }
        }
        // a step and a cliff, well away from the part we check
        for z in 0..48 {
            world.set(40, 10, z, w::GRASS);
            for y in 10..14 {
                for x in 44..48 {
                    world.set(x, y, z, w::STONE);
                }
            }
        }
        let geos = all_sections(&world);
        let mut tops = 0;
        for v in geos.iter().flat_map(|g| &g.opaque) {
            if v.pos[0] < 4.0 || v.pos[0] > 36.0 || v.pos[2] < 4.0 || v.pos[2] > 44.0 {
                continue;
            }
            assert_eq!(v.pos[1], 10.0);
            assert_eq!(v.normal, [0.0, 1.0, 0.0]);
            assert_eq!(v.color[3], 1.0);
            tops += 1;
        }
        assert!(tops > 1000);
        // the cliff face stays upright: its middle corners keep their x
        let wall = geos
            .iter()
            .flat_map(|g| &g.opaque)
            .filter(|v| {
                (11.9..12.1).contains(&v.pos[1])
                    && (43.5..44.5).contains(&v.pos[0])
                    && (4.0..44.0).contains(&v.pos[2])
            })
            .inspect(|v| assert_eq!(v.pos[0], 44.0))
            .count();
        assert!(wall > 20);
    }
}
