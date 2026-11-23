//! Section meshing. The world stays a 1 m block grid; this turns it into a
//! finer picture:
//!
//! - natural ground is fine voxels, shaped by the blocks around it
//!   (`fine.rs`);
//! - leaf blocks are a smooth, puffed-up surface (surface nets with the
//!   topology of the cube mesh);
//! - shaped blocks are drawn by `shapes.rs`; everything else stays a cube;
//! - water gets a lowered surface that tucks under the shore.
//!
//! What is drawn at a place depends only on the blocks near it, so two
//! neighbouring sections always agree and the borders are seamless.

use crate::fine::{self, Lod};
use crate::light::Light;
use crate::render::{self, KIND_FOLIAGE, SECTION, Vertex};
use dyno_world::{self as w, BlockId, World};
use glam::Vec3;

const N: i32 = SECTION;
/// Cells read around the section.
const PAD: i32 = 4;
const G: usize = (N + 2 * PAD) as usize;
/// Grid corners -1..=17 on each axis.
const LV: usize = (N + 3) as usize;
/// Blocks -2..=17 on each axis have a density.
const DB: usize = (N + 4) as usize;

/// How far outside a section its mesh reads blocks. A block edit must re-mesh
/// every section within this distance.
pub const REACH: i32 = PAD + 1;

pub(crate) const F_TERRAIN: u8 = 1;
/// Built: ground corners that touch it stay on the grid.
pub(crate) const F_PIN: u8 = 2;
/// Fills its whole cell: hides the faces of its neighbours and darkens
/// their corners. A detail block that draws as a smaller shape does not.
pub(crate) const F_OPAQUE: u8 = 4;
pub(crate) const F_LEAVES: u8 = 8;
pub(crate) const F_WATER: u8 = 16;
/// Stops light (the world's `is_opaque`): its own cell holds no light.
pub(crate) const F_DARK: u8 = 32;
/// Drawn by the detail mesher (see `detail.rs`), not as a plain cube.
pub(crate) const F_DETAIL: u8 = 64;

/// Leaf canopies: the surface sits where the blurred leaf field crosses this.
const LEAF_ISO: f32 = 0.37;
const LEAF_IN: f32 = 0.2;
const LEAF_OUT: f32 = 0.08;
/// How far the water surface sits below the top of its block.
pub const WATER_DROP: f32 = 0.2;
/// Natural ground: drawn as fine voxels (the pack's `terrain` tag).
pub fn is_terrain(b: BlockId) -> bool {
    crate::shapes::has(b, crate::shapes::tag::TERRAIN)
}

pub(crate) fn flags() -> &'static [u8; 256] {
    &crate::shapes::pack().flags
}

/// The flags of every block, given which are shaped and which are ground.
pub(crate) fn flags_with(
    detail: impl Fn(BlockId) -> bool,
    terrain: impl Fn(BlockId) -> bool,
) -> [u8; 256] {
    {
        let mut t = [0u8; 256];
        for (b, f) in t.iter_mut().enumerate() {
            let b = b as BlockId;
            if b == w::AIR || b >= w::BLOCK_COUNT {
                continue;
            }
            if w::is_opaque(b) {
                *f |= F_OPAQUE | F_DARK;
            }
            if detail(b) {
                *f |= F_DETAIL;
            }
            if terrain(b) {
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
    }
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
    /// fine voxel ground: four corners a quad, drawn with the shared quad
    /// index (triangles 0 1 2 and 0 2 3)
    pub ground: Vec<Vertex>,
    pub trans: Vec<Vertex>,
    /// the open tops of the ground, for the cover to grow on (emptied once
    /// it has)
    pub tops: Vec<crate::cover::Top>,
    /// plants and stones standing on the ground (`cover.rs`)
    pub cover: Vec<crate::cover::Spot>,
    /// leaves drawn as cubes: in clumps (true) or a cube a block (false);
    /// None with no leaves, or smooth ones
    pub leaves_fine: Option<bool>,
    pub glows: Vec<Glow>,
    /// the level of detail its ground was meshed at
    pub lod: Lod,
    /// the drawn ground under each open top: (cell, voxels a metre, the
    /// height of each fine column's top), for seating the cover
    pub seats: Vec<([i8; 3], u8, Vec<f32>)>,
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

/// The blocks of one section and a border around it, in section-local cells.
pub(crate) struct Grid {
    pub(crate) ids: Vec<BlockId>,
    pub(crate) fl: Vec<u8>,
    /// world position of local cell (0, 0, 0)
    pub(crate) o: [i32; 3],
    /// the world's size in blocks
    pub(crate) size: [i32; 3],
    /// light (block, sky) for detail blocks that stop light, borrowed from
    /// their open neighbours; empty when there are none
    patch: Vec<[u8; 2]>,
    /// each shaped block's class and quarter turn (`class << 2 | turn`),
    /// decided by `shapes::classify`; 0 for the rest
    pub(crate) cls: Vec<u16>,
}

/// Blocks that pave the ground: drawn as part of the fine voxel ground
/// where they lie flush with it.
pub(crate) fn is_paving(b: BlockId) -> bool {
    crate::shapes::has(b, crate::shapes::tag::PAVING)
}

impl Grid {
    /// Read the blocks of the section at `o` and its border, and decide
    /// what shape each is drawn as.
    fn load_shape(world: &World, o: [i32; 3]) -> Grid {
        let mut g = Grid::load_with(world, o, flags());
        if g.fl.iter().any(|f| f & F_DETAIL != 0) {
            crate::shapes::classify(&mut g);
        }
        g
    }

    /// The blocks of the section and its border with the given flags, and
    /// paving laid as ground; shapes not yet decided.
    pub(crate) fn load_with(world: &World, o: [i32; 3], lut: &[u8; 256]) -> Grid {
        let mut g = Grid {
            ids: vec![0; G * G * G],
            fl: vec![0; G * G * G],
            o,
            size: world.size.map(|v| v as i32),
            patch: Vec::new(),
            cls: Vec::new(),
        };
        let mut i = 0;
        let mut all = 0u8;
        for y in -PAD..N + PAD {
            for z in -PAD..N + PAD {
                for x in -PAD..N + PAD {
                    let b = world.get(o[0] + x, o[1] + y, o[2] + z);
                    let f = lut[b as usize];
                    g.ids[i] = b;
                    g.fl[i] = f;
                    all |= f;
                    i += 1;
                }
            }
        }
        if all & F_PIN != 0 {
            g.pave(lut);
        }
        g
    }

    /// The same, with the light that detail blocks borrow.
    pub(crate) fn load(world: &World, light: &Light, o: [i32; 3]) -> Grid {
        let mut g = Grid::load_shape(world, o);
        if g.fl.iter().any(|f| f & F_DETAIL != 0) {
            g.patch_light(light);
        }
        g
    }

    /// Paving that lies flush with the ground becomes ground: it is drawn
    /// as fine voxels and meets the grass along a ragged edge.
    /// A cell counts when it has solid ground under it, nothing built on
    /// top, other ground beside it and at most one open side (so low walls
    /// and the tops of pillars stay built). Reads one cell round each cell
    /// it changes, so every cell the meshers read (-3..N+3) is decided the
    /// same way by every section that holds it.
    fn pave(&mut self, lut: &[u8; 256]) {
        // props and posts may stand on paving; anything else on it is a wall
        let built = |b: BlockId| {
            lut[b as usize] & F_OPAQUE != 0
                && !is_paving(b)
                && !crate::shapes::has(b, crate::shapes::tag::ON_PAVING)
        };
        let mut set = Vec::new();
        for y in -3..N + 3 {
            for z in -3..N + 3 {
                for x in -3..N + 3 {
                    let c = at(x, y, z);
                    if !is_paving(self.ids[c]) {
                        continue;
                    }
                    let (up, down) = (self.ids[c + DY], self.ids[c - DY]);
                    if built(up) || is_paving(up) || !w::is_solid(down) {
                        continue;
                    }
                    let (mut open, mut ground) = (0, 0);
                    for n in [c - DX, c + DX, c - DZ, c + DZ] {
                        let b = self.ids[n];
                        if !w::is_solid(b) {
                            open += 1;
                        } else if is_terrain(b) || (is_paving(b) && !w::is_solid(self.ids[n + DY]))
                        {
                            ground += 1;
                        }
                    }
                    if open <= 1 && ground >= 1 {
                        set.push(c);
                    }
                }
            }
        }
        for c in set {
            self.fl[c] = (self.fl[c] & !(F_PIN | F_DETAIL)) | F_TERRAIN;
        }
    }

    /// Detail blocks that stop light hold none in their own cell; their
    /// faces take the light of the brightest open cell beside them.
    fn patch_light(&mut self, light: &Light) {
        self.patch = vec![[0, 0]; G * G * G];
        for y in -3..N + 3 {
            for z in -3..N + 3 {
                for x in -3..N + 3 {
                    let c = at(x, y, z);
                    if self.fl[c] & (F_DETAIL | F_DARK) != F_DETAIL | F_DARK {
                        continue;
                    }
                    let mut best = [0u8, 0u8];
                    for (d, n) in [
                        ([-1, 0, 0], c - DX),
                        ([1, 0, 0], c + DX),
                        ([0, 0, -1], c - DZ),
                        ([0, 0, 1], c + DZ),
                        ([0, -1, 0], c - DY),
                        ([0, 1, 0], c + DY),
                    ] {
                        if self.fl[n] & F_DARK != 0 {
                            continue;
                        }
                        let p = [
                            self.o[0] + x + d[0],
                            self.o[1] + y + d[1],
                            self.o[2] + z + d[2],
                        ];
                        best[0] = best[0].max(light.get(p[0], p[1], p[2]));
                        best[1] = best[1].max(light.sky(p[0], p[1], p[2]));
                    }
                    self.patch[c] = best;
                }
            }
        }
    }

    /// Block light and sky light (0..=15) of a local cell.
    #[inline]
    pub(crate) fn light(&self, light: &Light, p: [i32; 3]) -> (f32, f32) {
        let c = at(p[0], p[1], p[2]);
        if !self.patch.is_empty() && self.fl[c] & (F_DETAIL | F_DARK) == F_DETAIL | F_DARK {
            let l = self.patch[c];
            return (l[0] as f32, l[1] as f32);
        }
        let (x, y, z) = (self.o[0] + p[0], self.o[1] + p[1], self.o[2] + p[2]);
        (light.get(x, y, z) as f32, light.sky(x, y, z) as f32)
    }
}

pub(crate) const DX: usize = 1;
pub(crate) const DZ: usize = G;
pub(crate) const DY: usize = G * G;

#[inline]
pub(crate) fn at(x: i32, y: i32, z: i32) -> usize {
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

/// Leaf canopies: one smooth, puffed-up surface over the leaf blocks
/// (surface nets with the topology of the cube mesh: every exposed leaf
/// face becomes one quad whose corners sit at positions computed per grid
/// corner).
struct Nets<'a> {
    g: &'a Grid,
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
}

impl<'a> Nets<'a> {
    fn new(g: &'a Grid) -> Self {
        let n = LV * LV * LV;
        Nets {
            g,
            pos: vec![[0.0; 3]; n],
            nrm: vec![[0.0; 3]; n],
            state: vec![0; n],
            dens: vec![f32::NAN; DB * DB * DB],
            lit: vec![[0.0; 3]; n],
        }
    }

    /// Blurred leaf field at a cell: positive inside, negative outside,
    /// never the wrong sign for the cell. Blurred in all directions, so
    /// canopies come out round.
    fn density(&mut self, x: i32, y: i32, z: i32) -> f32 {
        let k = di(x, y, z);
        let d = self.dens[k];
        if !d.is_nan() {
            return d;
        }
        let g = self.g;
        let c = at(x, y, z);
        let mut s = 0u32;
        for oy in [c - DY, c, c + DY] {
            for oz in [oy - DZ, oy, oy + DZ] {
                for q in [oz - DX, oz, oz + DX] {
                    s += (g.fl[q] & F_LEAVES != 0) as u32;
                }
            }
        }
        let d = s as f32 / 27.0 - LEAF_ISO;
        let d = if g.fl[c] & F_LEAVES != 0 {
            d.max(LEAF_IN)
        } else {
            d.min(-LEAF_OUT)
        };
        self.dens[k] = d;
        d
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
        let base = at(i - 1, j - 1, k - 1);
        let cnt = CELL8
            .iter()
            .filter(|o| g.fl[base + **o] & F_LEAVES != 0)
            .count();
        if cnt != 0 && cnt != 8 {
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
                for a in 0..3 {
                    p[a] += sum[a] / n as f32;
                }
            }
            // a little disorder, the same from whichever section looks
            let h = hash3(g.o[0] + i, g.o[1] + j, g.o[2] + k);
            let r = |s: u32| ((h >> s) & 255) as f32 / 255.0 - 0.5;
            p[0] += r(0) * 0.22;
            p[1] += r(8) * 0.22;
            p[2] += r(16) * 0.22;
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

    /// Block light, sky light and occlusion at a grid corner.
    fn light(&mut self, light: &Light, c: [i8; 3]) -> [f32; 3] {
        let (i, j, k) = (c[0] as i32, c[1] as i32, c[2] as i32);
        let idx = li(i, j, k);
        if self.state[idx] & 2 != 0 {
            return self.lit[idx];
        }
        let g = self.g;
        let base = at(i - 1, j - 1, k - 1);
        let (mut bl, mut sl, mut open) = (0u32, 0u32, 0u32);
        for (ci, o) in CELL8.iter().enumerate() {
            if g.fl[base + o] & F_DARK != 0 {
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
        // darker in the folds between the puffs, lighter on them
        let leafy = CELL8
            .iter()
            .filter(|o| g.fl[base + **o] & F_LEAVES != 0)
            .count();
        let ao = (1.12 - 0.075 * leafy as f32).min(1.0);
        let v = [bl as f32 * k15, sl as f32 * k15, ao];
        self.lit[idx] = v;
        self.state[idx] |= 2;
        v
    }

    /// Mesh the surface: quads for the section's own blocks, with normals
    /// that also take in the faces just outside it.
    fn run(&mut self, light: &Light, geo: &mut SectionGeo) {
        let g = self.g;
        let hide = F_OPAQUE | F_LEAVES;
        let mut quads: Vec<Quad> = Vec::new();
        for y in -1..=N {
            for z in -1..=N {
                for x in -1..=N {
                    let c = at(x, y, z);
                    if g.fl[c] & F_LEAVES == 0 {
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
                                block: g.ids[c],
                                dir: dir as u8,
                                c: cs,
                            });
                        }
                    }
                }
            }
        }
        let t = render::textures();
        geo.opaque.reserve(quads.len() * 6);
        for q in &quads {
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
                ls[i] = self.light(light, c);
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
            let top = t.face(q.block, 0);
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
        }
    }
}

/// Cubes, posts and water for the section's own blocks.
fn cubes(g: &Grid, light: &Light, has_water: bool, geo: &mut SectionGeo) {
    let lget = |p: [i32; 3]| g.light(light, p).0;
    let lsky = |p: [i32; 3]| g.light(light, p).1;
    let solid = |p: [i32; 3]| (g.fl[at(p[0], p[1], p[2])] & F_OPAQUE != 0) as i32;
    let tile = render::tile_uv;
    let wt = render::textures().face(w::WATER, 0);
    for y in 0..N {
        for z in 0..N {
            for x in 0..N {
                let c = at(x, y, z);
                let b = g.ids[c];
                let f = g.fl[c];
                // detail blocks that no longer fill their cell draw themselves
                if b == w::AIR || f & F_LEAVES != 0 || f & (F_DETAIL | F_OPAQUE) == F_DETAIL {
                    continue;
                }
                let wpos = [g.o[0] + x, g.o[1] + y, g.o[2] + z];
                if f & F_TERRAIN != 0 {
                    // the shore: the water surface carries on under the
                    // ground, which slopes back from it (a beach, or a
                    // bank that stands above the water); over a sill, the
                    // water falls
                    if has_water {
                        crate::water::bank(g, &lget, &lsky, [x, y, z], wt, true, &mut geo.trans);
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
                if has_water && f & F_OPAQUE != 0 {
                    // a built sill that holds water back: it runs over
                    crate::water::bank(g, &lget, &lsky, [x, y, z], wt, false, &mut geo.trans);
                }
                // one size for every block, so a chamfer runs on unbroken
                // from one block to the next
                let bevel = if bevels(g, c) { BEVEL } else { 0.0 };
                for (n, u, v, shade) in FACES {
                    let nc = step(c, n);
                    if g.fl[nc] & F_OPAQUE != 0 || (tr && g.ids[nc] == b) {
                        continue;
                    }
                    if water && n[1] == 1 {
                        crate::water::surface(g, &lget, &lsky, [x, y, z], wt, &mut geo.trans);
                        continue;
                    }
                    if water && n[1] == 0 {
                        // an open side: the water falls
                        let d = [n[0], n[2]];
                        crate::water::side(g, &lget, &lsky, [x, y, z], d, wt, &mut geo.trans);
                        continue;
                    }
                    let base = [x + n[0], y + n[1], z + n[2]];
                    let center = [
                        wpos[0] as f32 + 0.5 + n[0] as f32 * 0.5,
                        wpos[1] as f32 + 0.5 + n[1] as f32 * 0.5,
                        wpos[2] as f32 + 0.5 + n[2] as f32 * 0.5,
                    ];
                    // a convex edge (open beyond it, as well as in front)
                    // is bevelled: the face pulls back from it and a slanted
                    // strip joins it to the face round the corner
                    let convex = |e: [i32; 3]| bevel > 0.0 && g.fl[step(c, e)] & F_OPAQUE == 0;
                    let mut corners = [([0f32; 3], 0f32, 0f32, 0f32, 0f32, 0f32); 4];
                    for (i, (su, sv)) in QUAD.into_iter().enumerate() {
                        let eu = [su * u[0], su * u[1], su * u[2]];
                        let ev = [sv * v[0], sv * v[1], sv * v[2]];
                        let ku = 0.5 - if convex(eu) { bevel } else { 0.0 };
                        let kv = 0.5 - if convex(ev) { bevel } else { 0.0 };
                        let mut p = [
                            center[0] + ku * (su * u[0]) as f32 + kv * (sv * v[0]) as f32,
                            center[1] + ku * (su * u[1]) as f32 + kv * (sv * v[1]) as f32,
                            center[2] + ku * (su * u[2]) as f32 + kv * (sv * v[2]) as f32,
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
                            [0.55, 0.7, 0.85, 1.0][ao as usize]
                        };
                        corners[i] = (
                            p,
                            shade_ao,
                            bl,
                            sl,
                            ku * 2.0 * su as f32,
                            kv * 2.0 * sv as f32,
                        );
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
                    // sides: v runs down from the top; mirror u on the far faces
                    let uv = |su: f32, sv: f32| -> [f32; 2] {
                        if face_id == 1 {
                            let uu = (su + 1.0) * 0.5;
                            [
                                tile(if n[0] + n[2] < 0 { uu } else { 1.0 - uu }),
                                tile((1.0 - sv) * 0.5),
                            ]
                        } else {
                            [tile((su + 1.0) * 0.5), tile((sv + 1.0) * 0.5)]
                        }
                    };
                    let vert =
                        |p: [f32; 3], nrm: [f32; 3], l: f32, bl: f32, sl: f32, t: [f32; 2]| {
                            Vertex {
                                pos: p,
                                normal: nrm,
                                color: [col, col, col, l],
                                light: bl,
                                sky: sl,
                                tex: [t[0], t[1], layer[2], layer[3]],
                            }
                        };
                    for i in order {
                        let (p, l, bl, sl, au, av) = corners[i];
                        out.push(vert(p, nf, l, bl, sl, uv(au, av)));
                    }
                    if bevel > 0.0 {
                        let fi = face_index(n);
                        let add = |p: [f32; 3], d: [i32; 3], k: f32| {
                            [
                                p[0] + (d[0] - n[0]) as f32 * k,
                                p[1] + (d[1] - n[1]) as f32 * k,
                                p[2] + (d[2] - n[2]) as f32 * k,
                            ]
                        };
                        // strips along the four edges: (corner, corner, edge direction)
                        for (i, j, eu_side) in
                            [(0, 1, false), (1, 2, true), (2, 3, false), (3, 0, true)]
                        {
                            let (su, sv) = QUAD[i];
                            let e = if eu_side {
                                [su * u[0], su * u[1], su * u[2]]
                            } else {
                                [sv * v[0], sv * v[1], sv * v[2]]
                            };
                            if !convex(e) || face_index(e) < fi {
                                continue;
                            }
                            let nrm = Vec3::new(
                                (n[0] + e[0]) as f32,
                                (n[1] + e[1]) as f32,
                                (n[2] + e[2]) as f32,
                            )
                            .normalize()
                            .to_array();
                            let (pi, li, bli, sli, ui, vi) = corners[i];
                            let (pj, lj, blj, slj, uj, vj) = corners[j];
                            // texture runs on over the edge
                            let ext = |a: f32, b: f32| {
                                let k = bevel * 2.0;
                                if eu_side {
                                    (a + k * su as f32, b)
                                } else {
                                    (a, b + k * sv as f32)
                                }
                            };
                            let (ui2, vi2) = ext(ui, vi);
                            let (uj2, vj2) = ext(uj, vj);
                            let q = [
                                vert(pi, nrm, li, bli, sli, uv(ui, vi)),
                                vert(pj, nrm, lj, blj, slj, uv(uj, vj)),
                                vert(add(pj, e, bevel), nrm, lj, blj, slj, uv(uj2, vj2)),
                                vert(add(pi, e, bevel), nrm, li, bli, sli, uv(ui2, vi2)),
                            ];
                            out.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
                            // where the chamfer stops against a solid block that
                            // does not carry it on, close the notch it leaves
                            for (k, p, l, bl, sl, t) in [
                                (i, pi, li, bli, sli, uv(ui, vi)),
                                (j, pj, lj, blj, slj, uv(uj, vj)),
                            ] {
                                let (su, sv) = QUAD[k];
                                let end = if eu_side {
                                    [sv * v[0], sv * v[1], sv * v[2]]
                                } else {
                                    [su * u[0], su * u[1], su * u[2]]
                                };
                                let q = step(c, end);
                                if g.fl[q] & F_OPAQUE == 0 {
                                    continue;
                                }
                                let carries = bevels(g, q)
                                    && g.fl[step(q, n)] & F_OPAQUE == 0
                                    && g.fl[step(q, e)] & F_OPAQUE == 0;
                                if carries {
                                    continue;
                                }
                                let corner = [
                                    p[0] + e[0] as f32 * bevel,
                                    p[1] + e[1] as f32 * bevel,
                                    p[2] + e[2] as f32 * bevel,
                                ];
                                let nn = [-end[0] as f32, -end[1] as f32, -end[2] as f32];
                                out.extend([
                                    vert(p, nn, l, bl, sl, t),
                                    vert(corner, nn, l, bl, sl, t),
                                    vert(add(p, e, bevel), nn, l, bl, sl, t),
                                ]);
                            }
                        }
                        // corner triangles where two bevels meet
                        for (i, (su, sv)) in QUAD.into_iter().enumerate() {
                            let eu = [su * u[0], su * u[1], su * u[2]];
                            let ev = [sv * v[0], sv * v[1], sv * v[2]];
                            if !convex(eu)
                                || !convex(ev)
                                || face_index(eu) < fi
                                || face_index(ev) < fi
                            {
                                continue;
                            }
                            let nrm = Vec3::new(
                                (n[0] + eu[0] + ev[0]) as f32,
                                (n[1] + eu[1] + ev[1]) as f32,
                                (n[2] + eu[2] + ev[2]) as f32,
                            )
                            .normalize()
                            .to_array();
                            let (p, l, bl, sl, au, av) = corners[i];
                            let t = uv(au, av);
                            out.extend([
                                vert(p, nrm, l, bl, sl, t),
                                vert(add(p, eu, bevel), nrm, l, bl, sl, t),
                                vert(add(p, ev, bevel), nrm, l, bl, sl, t),
                            ]);
                        }
                    }
                }
            }
        }
    }
}

/// How far the convex edges of built cubes are chamfered.
const BEVEL: f32 = 0.05;

#[cfg(test)]
thread_local! {
    /// Tests of the bare skin turn the chamfers off: where a chamfered
    /// block meets one without, its edges meet in T-junctions.
    static NO_BEVEL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// True for a cube the cube mesher draws with chamfered edges: built,
/// opaque, tagged `chamfer` (not glass, not glowing).
fn bevels(g: &Grid, c: usize) -> bool {
    #[cfg(test)]
    if NO_BEVEL.with(|n| n.get()) {
        return false;
    }
    let f = g.fl[c];
    f & (F_PIN | F_OPAQUE) == F_PIN | F_OPAQUE
        && f & (F_TERRAIN | F_LEAVES | F_WATER) == 0
        && crate::shapes::has(g.ids[c], crate::shapes::tag::CHAMFER)
}

/// Index in `FACES` of the face with this normal.
fn face_index(n: [i32; 3]) -> usize {
    match n {
        [1, 0, 0] => 0,
        [-1, 0, 0] => 1,
        [0, 1, 0] => 2,
        [0, -1, 0] => 3,
        [0, 0, 1] => 4,
        _ => 5,
    }
}

/// Build everything drawn for one 16³ section, its ground at the finest
/// level of detail.
pub fn mesh(world: &World, light: &Light, sx: i32, sy: i32, sz: i32) -> SectionGeo {
    mesh_lod(world, light, sx, sy, sz, Lod::finest())
}

/// Build everything drawn for one 16³ section, its ground at `lod`.
pub fn mesh_lod(world: &World, light: &Light, sx: i32, sy: i32, sz: i32, lod: Lod) -> SectionGeo {
    mesh_at(world, light, [sx, sy, sz], None, None, lod)
}

/// Build a section as seen from a camera at `focus`: leaves far from it are
/// drawn coarser (`foliage.rs`; `was` is how its leaves are drawn now), and
/// its ground at the level of detail `lod` (`fine.rs`).
pub fn mesh_at(
    world: &World,
    light: &Light,
    [sx, sy, sz]: [i32; 3],
    focus: Option<[f32; 3]>,
    was: Option<bool>,
    lod: Lod,
) -> SectionGeo {
    let mut geo = SectionGeo {
        lod,
        ..SectionGeo::default()
    };
    let g = Grid::load(world, light, [sx * N, sy * N, sz * N]);
    let (mut own, mut all, mut every) = (0u8, 0u8, 0xffu8);
    let mut i = 0;
    for y in -PAD..N + PAD {
        let inner_y = (0..N).contains(&y);
        for z in -PAD..N + PAD {
            let inner_z = inner_y && (0..N).contains(&z);
            for x in -PAD..N + PAD {
                let f = g.fl[i];
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
    if (own == 0 && all & F_TERRAIN == 0) || every & F_OPAQUE != 0 {
        return geo;
    }
    // ground next door may grow into this section's air
    if all & F_TERRAIN != 0 {
        fine::mesh(&g, light, lod, &mut geo);
    }
    if own & F_LEAVES != 0 {
        // as clumps of cubes when the pack says so (`foliage.rs`), else smooth
        match crate::foliage::current().filter(|f| f.voxels()) {
            Some(f) => {
                let fine = f.fine(focus, [sx, sy, sz], was);
                crate::foliage::mesh(&f, &g, light, &mut geo, fine)
            }
            None => Nets::new(&g).run(light, &mut geo),
        }
    }
    let has_water = all & F_WATER != 0;
    if own & (F_PIN | F_WATER) != 0 || (has_water && own & F_TERRAIN != 0) {
        cubes(&g, light, has_water, &mut geo);
    }
    if own & F_DETAIL != 0 {
        crate::shapes::mesh(&g, light, &mut geo);
    }
    let tops = std::mem::take(&mut geo.tops);
    crate::cover::scatter(&g, &tops, &mut geo.cover);
    // plants and stones stand on the top of the fine column under them,
    // not on a slope between the block's corners
    let seats = std::mem::take(&mut geo.seats);
    if !seats.is_empty() {
        let by_cell: std::collections::HashMap<[i8; 3], (u8, &[f32])> = seats
            .iter()
            .map(|(c, r, h)| (*c, (*r, h.as_slice())))
            .collect();
        for s in &mut geo.cover {
            let p = s.pos();
            let cell = [
                p[0].floor() as i32 - g.o[0],
                (p[1] - 0.5).floor() as i32 - g.o[1],
                p[2].floor() as i32 - g.o[2],
            ];
            // the top it grew on: its own cell, or the one below when the
            // ground there rose into the cell above
            for dy in [0, -1, 1] {
                let key = [cell[0] as i8, (cell[1] + dy) as i8, cell[2] as i8];
                if let Some((r, h)) = by_cell.get(&key) {
                    let r = *r as usize;
                    let i = ((p[0] - p[0].floor()) * r as f32) as usize;
                    let k = ((p[2] - p[2].floor()) * r as f32) as usize;
                    let y = h[k.min(r - 1) * r + i.min(r - 1)];
                    if !y.is_nan() {
                        s.set_height(y);
                    }
                    break;
                }
            }
        }
    }
    geo
}

/// Heights of the drawn ground, so feet stand on the fine voxels rather
/// than on the block grid they were made from: on the top of the fine
/// column under them. Cached per block; clear it when blocks change.
#[derive(Default)]
pub struct Ground {
    cache: std::cell::RefCell<std::collections::HashMap<[i32; 3], Option<Tops>>>,
}

/// The tops of a block's fine columns: voxels a metre, and the heights.
type Tops = (u8, std::rc::Rc<[f32]>);

impl Ground {
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }

    /// The drawn tops of the fine columns of ground block `b`. None when
    /// `b` is not ground with open air above it.
    fn tops(&self, world: &World, b: [i32; 3]) -> Option<Tops> {
        if let Some(c) = self.cache.borrow().get(&b) {
            return c.clone();
        }
        let lut = flags();
        let id = world.get(b[0], b[1], b[2]);
        let top = lut[id as usize];
        let above = lut[world.get(b[0], b[1] + 1, b[2]) as usize];
        let out = if (top & F_TERRAIN == 0 && !is_paving(id)) || above & (F_TERRAIN | F_OPAQUE) != 0
        {
            None
        } else {
            // a grid whose middle holds the block, the same as a section's
            // (only the shape of the ground is wanted, not its light)
            let o = [b[0] - N / 2, b[1] - N / 2, b[2] - N / 2];
            let g = Grid::load_shape(world, o);
            if g.fl[at(N / 2, N / 2, N / 2)] & F_TERRAIN == 0 {
                None
            } else {
                let (r, t) = fine::tops(&g);
                Some((r, t.into()))
            }
        };
        let mut c = self.cache.borrow_mut();
        if c.len() > 4096 {
            c.clear();
        }
        c.insert(b, out.clone());
        out
    }

    /// Where feet at `pos` should be drawn: on the top of the fine column
    /// under them when they stand on ground, otherwise where they are.
    pub fn feet(&self, world: &World, pos: Vec3) -> Vec3 {
        let by = (pos.y - 0.01).floor();
        // only when standing (on the block top), not in the air
        if pos.y - (by + 1.0) > 0.06 {
            return pos;
        }
        let b = [pos.x.floor() as i32, by as i32, pos.z.floor() as i32];
        let Some((r, t)) = self.tops(world, b) else {
            return pos;
        };
        let rf = r as f32;
        let i = (((pos.x - pos.x.floor()) * rf) as usize).min(r as usize - 1);
        let k = (((pos.z - pos.z.floor()) * rf) as usize).min(r as usize - 1);
        let y = t[k * r as usize + i];
        if y.is_nan() {
            return pos;
        }
        let dy = (y - (by + 1.0)).clamp(-0.7, 0.7);
        Vec3::new(pos.x, pos.y + dy, pos.z)
    }

    /// Height of the drawn ground at (x, z), on the topmost block near
    /// height `y`.
    pub fn height(&self, world: &World, x: f32, z: f32, y: f32) -> f32 {
        let (bx, bz) = (x.floor() as i32, z.floor() as i32);
        let y0 = y.round() as i32 + 2;
        let Some(top) = (y0 - 5..=y0).rev().find(|y| {
            w::is_solid(world.get(bx, *y, bz)) && !w::is_solid(world.get(bx, *y + 1, bz))
        }) else {
            return y;
        };
        self.feet(world, Vec3::new(x, (top + 1) as f32, z)).y
    }
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

    /// Ground and air in lumps, with caves and overhangs, and (unless
    /// `bare`) built blocks among it. The bottom layer is air: nothing
    /// touches the rock under the world.
    fn lumpy_with(bare: bool) -> World {
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
                        } else if fine < 3 && !bare {
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
                    } else if fine < 1 && !bare {
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

    fn lumpy() -> World {
        lumpy_with(false)
    }

    fn all_sections(world: &World) -> Vec<SectionGeo> {
        all_sections_at(world, Lod::finest())
    }

    fn all_sections_at(world: &World, lod: Lod) -> Vec<SectionGeo> {
        let light = Light::new(world);
        let d = world.size.map(|v| v as i32 / N);
        let mut out = Vec::new();
        for sy in 0..d[1] {
            for sz in 0..d[2] {
                for sx in 0..d[0] {
                    out.push(mesh_lod(world, &light, sx, sy, sz, lod));
                }
            }
        }
        out
    }

    /// The fine voxel ground's faces, cut into single voxel faces: (the
    /// face's lowest corner in voxels, its axis and which way it faces).
    fn unit_faces(geos: &[SectionGeo], r: f32) -> Vec<([i32; 3], usize, bool)> {
        let mut out = Vec::new();
        for geo in geos {
            for q in geo.ground.as_chunks::<4>().0 {
                let p: Vec<[i32; 3]> = q
                    .iter()
                    .map(|v| {
                        v.pos.map(|c| {
                            let k = c * r;
                            assert_eq!(k, k.round(), "a corner off the voxel grid");
                            k as i32
                        })
                    })
                    .collect();
                let lo = [0, 1, 2].map(|a| p.iter().map(|c| c[a]).min().unwrap());
                let hi = [0, 1, 2].map(|a| p.iter().map(|c| c[a]).max().unwrap());
                let axis = (0..3).find(|a| lo[*a] == hi[*a]).expect("a flat face");
                // which way the face looks (the normal may be bent towards
                // the slope): tex.x is voxels a metre plus 32 times it,
                // plus 256 times the colour cells a metre
                let dir = ((q[0].tex[0] % 256.0) / 32.0).floor() as usize;
                assert_eq!(dir / 2, axis, "direction and plane agree");
                let mut n = [0f32; 3];
                n[axis] = if dir.is_multiple_of(2) { 1.0 } else { -1.0 };
                let out_way = n[axis] > 0.0;
                // the winding agrees with the facing
                let e1 = Vec3::from(q[1].pos) - Vec3::from(q[0].pos);
                let e2 = Vec3::from(q[2].pos) - Vec3::from(q[0].pos);
                assert!(
                    e1.cross(e2).dot(Vec3::from(n)) > 0.0,
                    "a face wound inside out"
                );
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                for a in lo[u]..hi[u] {
                    for b in lo[v]..hi[v] {
                        let mut c = lo;
                        c[u] = a;
                        c[v] = b;
                        out.push((c, axis, out_way));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn the_ground_is_a_closed_skin() {
        // ground alone must be a closed surface of voxel faces: each face
        // drawn once, and each voxel edge met by an even number of faces,
        // inside a section, across section borders, and at every level of
        // detail
        let world = lumpy_with(true);
        for r in [8u8, 4, 1] {
            let geos = all_sections_at(
                &world,
                Lod {
                    per_metre: r,
                    skirt: [0; 4],
                    shade: true,
                },
            );
            let faces = unit_faces(&geos, r as f32);
            assert!(faces.len() > 20_000, "{} faces", faces.len());
            let mut seen = std::collections::HashSet::new();
            let mut edges: HashMap<([i32; 3], usize), u32> = HashMap::new();
            for (c, axis, way) in &faces {
                assert!(
                    seen.insert((*c, *axis, *way)),
                    "a face drawn twice at {c:?}"
                );
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                let (mut cu, mut cv) = (*c, *c);
                cu[u] += 1;
                cv[v] += 1;
                for e in [(*c, u), (*c, v), (cv, u), (cu, v)] {
                    *edges.entry(e).or_default() += 1;
                }
            }
            let open = edges.values().filter(|n| *n % 2 == 1).count();
            let mut odd: Vec<_> = edges
                .iter()
                .filter(|(_, n)| *n % 2 == 1)
                .map(|(k, _)| *k)
                .collect();
            odd.sort();
            for e in odd.iter().take(12) {
                eprintln!("open edge at {:?} along {} (r {r})", e.0, e.1);
            }
            assert_eq!(open, 0, "{r} a metre: {open} open edges of {}", edges.len());
        }
    }

    #[test]
    fn neighbours_decide_shared_voxels_alike() {
        // each section decides the cells one beyond its own; where two
        // sections both decide a voxel, they must agree
        let world = lumpy();
        let p = &crate::shapes::pack().terrain;
        let r = p.per_metre as i32;
        let (mut compared, mut solid) = (0u32, 0u32);
        for (a, b) in [
            ([0, 0, 0], [1, 0, 0]),
            ([1, 0, 1], [1, 1, 1]),
            ([0, 1, 0], [0, 1, 1]),
        ] {
            let ga = Grid::load_shape(&world, a.map(|v| v * N));
            let gb = Grid::load_shape(&world, b.map(|v| v * N));
            let mut fa = fine::Fine::new(&ga, p, r as u8);
            let mut fb = fine::Fine::new(&gb, p, r as u8);
            let d = [0, 1, 2].map(|i| (b[i] - a[i]) * N * r);
            for y in -r..(N + 1) * r {
                for z in -r..(N + 1) * r {
                    for x in -r..(N + 1) * r {
                        let (bx, by, bz) = (x - d[0], y - d[1], z - d[2]);
                        let inside = |v: i32| (-r..(N + 1) * r).contains(&v);
                        if !inside(bx) || !inside(by) || !inside(bz) {
                            continue;
                        }
                        let s = fa.solid(x, y, z);
                        assert_eq!(s, fb.solid(bx, by, bz), "voxel {x} {y} {z} of {a:?}");
                        compared += 1;
                        solid += s as u32;
                    }
                }
            }
        }
        assert!(compared > 100_000 && solid > 10_000, "{compared} {solid}");
    }

    #[test]
    fn ground_keeps_its_side_against_built_blocks() {
        // a wall stands on the ground and against a bank: the ground's
        // voxels that touch it are all there, so its hidden faces never show
        let mut world = World::empty([32, 32, 32]);
        for z in 0..32 {
            for x in 0..32 {
                let h = if x < 16 { 10 } else { 12 };
                for y in 0..=h {
                    world.set(x, y, z, if y == h { w::GRASS } else { w::DIRT });
                }
            }
        }
        for z in 4..28 {
            for y in 11..15 {
                world.set(15, y, z, w::PLANKS);
            }
        }
        let p = &crate::shapes::pack().terrain;
        let r = p.per_metre as i32;
        let g = Grid::load_shape(&world, [8, 4, 8]);
        let mut f = fine::Fine::new(&g, p, r as u8);
        // the bank (x = 16, local 8) against the wall (x = 15, local 7), y 11..=12
        let mut checked = 0;
        for y in 11 - 4..=12 - 4 {
            for z in 0..N {
                for j in 0..r {
                    for k in 0..r {
                        assert!(
                            f.solid(8 * r, y * r + j, z * r + k),
                            "a hole against the wall"
                        );
                        checked += 1;
                    }
                }
            }
        }
        // and the ground under the wall (x = 15, y = 10)
        for z in 0..N {
            for i in 0..r {
                for k in 0..r {
                    assert!(
                        f.solid(7 * r + i, 6 * r + r - 1, z * r + k),
                        "a hole under the wall"
                    );
                }
            }
        }
        assert!(checked > 1000);
    }

    #[test]
    fn sections_agree_at_their_borders() {
        let geos = all_sections(&lumpy());
        // same corner, facing and kind of ground: same light, normal and
        // shading, whichever section drew it
        let mut seen: HashMap<[u32; 8], [u32; 3]> = HashMap::new();
        let mut shared = 0;
        for geo in &geos {
            let mut mine: HashMap<[u32; 8], [u32; 3]> = HashMap::new();
            for v in &geo.ground {
                let p = v.pos.map(f32::to_bits);
                let n = v.normal.map(f32::to_bits);
                let at = [
                    p[0],
                    p[1],
                    p[2],
                    n[0],
                    n[1],
                    n[2],
                    v.tex[0].to_bits(),
                    v.tex[2].to_bits(),
                ];
                let what = [v.light, v.sky, v.color[3]].map(f32::to_bits);
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
        for v in geos.iter().flat_map(|g| g.opaque.iter().chain(&g.ground)) {
            if v.pos[0] < 4.0 || v.pos[0] > 36.0 || v.pos[2] < 4.0 || v.pos[2] > 44.0 {
                continue;
            }
            assert_eq!(v.pos[1], 10.0);
            assert_eq!(v.normal, [0.0, 1.0, 0.0]);
            assert_eq!(v.color[3], 1.0);
            tops += 1;
        }
        assert!(tops >= 6, "{tops}");
        // the cliff face stays upright: within a few voxels of the grid
        // line all the way up, with ledges
        let wall: Vec<f32> = geos
            .iter()
            .flat_map(|g| &g.ground)
            .filter(|v| {
                (11.0..13.0).contains(&v.pos[1])
                    && (42.0..46.0).contains(&v.pos[0])
                    && (4.0..44.0).contains(&v.pos[2])
                    && v.normal == [-1.0, 0.0, 0.0]
            })
            .map(|v| v.pos[0])
            .collect();
        assert!(wall.len() > 20);
        for x in &wall {
            assert!((43.5..=44.5).contains(x), "the wall leans out to {x}");
        }
        assert!(wall.iter().any(|x| *x != wall[0]), "the wall has no ledges");
    }

    #[test]
    fn creases_are_shaded() {
        // level ground meets a cliff: the ground's corners along the foot
        // are darkened and the lip on top is lit (open level ground stays
        // as it is: `flat_ground_sits_on_the_grid`)
        let mut world = World::empty([32, 32, 32]);
        for z in 0..32 {
            for x in 0..32 {
                let h = if x < 16 { 10 } else { 14 };
                for y in 0..=h {
                    // rock: shaded at full strength, lit by its own faces
                    world.set(x, y, z, w::STONE);
                }
            }
        }
        let p = &crate::shapes::pack().terrain;
        let geos = all_sections(&world);
        let tops: Vec<&Vertex> = geos
            .iter()
            .flat_map(|g| &g.ground)
            .filter(|v| v.normal == [0.0, 1.0, 0.0] && (4.0..28.0).contains(&v.pos[2]))
            .collect();
        let foot = tops
            .iter()
            .filter(|v| (11.0..12.0).contains(&v.pos[1]) && (14.0..=16.0).contains(&v.pos[0]))
            .map(|v| v.color[3])
            .fold(f32::MAX, f32::min);
        assert!(
            foot <= p.ao[1] + 1e-4,
            "the crease at the cliff's foot: {foot}"
        );
        let lip = tops
            .iter()
            .filter(|v| v.pos[1] > 14.5)
            .map(|v| v.color[3])
            .fold(f32::MIN, f32::max);
        assert!(lip > 1.0, "the lip on top: {lip}");
    }

    #[test]
    fn feet_follow_the_drawn_ground() {
        let mut world = World::empty([32, 32, 32]);
        for x in 0..32 {
            for z in 0..32 {
                let h = if x < 16 { 10 } else { 11 };
                for y in 0..=h {
                    world.set(x, y, z, if y == h { w::GRASS } else { w::DIRT });
                }
            }
        }
        let g = Ground::default();
        // far from the step: flat, on the block top
        let flat = g.feet(&world, Vec3::new(5.5, 11.0, 16.5));
        assert!((flat.y - 11.0).abs() < 1e-4, "{flat}");
        // next to a one-block step up: the ground rises towards it in steps
        let low = g.feet(&world, Vec3::new(15.9, 11.0, 16.5));
        assert!(low.y > 11.05, "{low}");
        let high = g.feet(&world, Vec3::new(16.1, 12.0, 16.5));
        assert!(high.y < 11.95, "{high}");
        assert!(
            (high.y - low.y).abs() < 0.6,
            "the step is a stair of fine voxels: {low} {high}"
        );
        // on the top of a fine voxel column: a whole number of voxels
        let r = crate::shapes::pack().terrain.per_metre as f32;
        for y in [low.y, high.y] {
            assert!(((y * r) - (y * r).round()).abs() < 1e-3, "{y}");
        }
        // in the air: untouched
        let air = Vec3::new(15.9, 11.5, 16.5);
        assert_eq!(g.feet(&world, air), air);
    }

    /// A small world with Hearthstead built on it: roofs, timber, props,
    /// paving and pillars.
    fn town() -> World {
        let mut world = World::generate([128, 64, 128], 7);
        let hearth = world.ensure_spawn_hearth(None);
        w::village::build_hearthstead(&mut world, hearth);
        world
    }

    #[test]
    fn detail_shapes_agree_across_sections() {
        // how a block is drawn (and whether it hides its neighbours) must
        // not depend on which section is looking at it
        let world = town();
        let light = Light::new(&world);
        let (mut detail, mut compared) = (0, 0);
        for sy in 0..4 {
            for sz in 0..8 {
                for sx in 0..8 {
                    let a = Grid::load(&world, &light, [sx * N, sy * N, sz * N]);
                    for (dx, dy, dz) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
                        let (nx, ny, nz) = (sx + dx, sy + dy, sz + dz);
                        if nx >= 8 || ny >= 4 || nz >= 8 {
                            continue;
                        }
                        let b = Grid::load(&world, &light, [nx * N, ny * N, nz * N]);
                        // cells both grids decide: two either side of the border
                        for y in -3..N + 3 {
                            for z in -3..N + 3 {
                                for x in -3..N + 3 {
                                    let (bx, by, bz) = (x - dx * N, y - dy * N, z - dz * N);
                                    if !(-3..N + 3).contains(&bx)
                                        || !(-3..N + 3).contains(&by)
                                        || !(-3..N + 3).contains(&bz)
                                    {
                                        continue;
                                    }
                                    let fa = a.fl[at(x, y, z)];
                                    let fb = b.fl[at(bx, by, bz)];
                                    assert_eq!(
                                        fa,
                                        fb,
                                        "block {:?} decided differently",
                                        [a.o[0] + x, a.o[1] + y, a.o[2] + z]
                                    );
                                    compared += 1;
                                    detail += (fa & F_DETAIL != 0) as u32;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(
            compared > 10_000 && detail > 200,
            "{compared} cells, {detail} detail"
        );
    }

    #[test]
    fn a_thin_roof_has_no_holes() {
        // grass one block thick over a cave, with ragged steps and air on
        // its own layer all round, where the noise is at work: every fine
        // column of the roof keeps ground
        let mut world = World::empty([32, 32, 32]);
        for z in 0..32 {
            for x in 0..32 {
                let h = 10 + (hash3(x / 2, 0, z / 3) % 2) as i32;
                for y in 0..=h {
                    world.set(x, y, z, if y == h { w::GRASS } else { w::DIRT });
                }
                // the cave, under the roof at y = 10
                if (8..24).contains(&x) && (8..24).contains(&z) {
                    for y in 6..10 {
                        world.set(x, y, z, w::AIR);
                    }
                }
            }
        }
        let p = &crate::shapes::pack().terrain;
        let r = p.per_metre as i32;
        let g = Grid::load_shape(&world, [8, 0, 8]);
        let mut f = fine::Fine::new(&g, p, r as u8);
        let mut checked = 0;
        for z in 0..N * r {
            for x in 0..N * r {
                let any = (10 * r..11 * r).any(|y| f.solid(x, y, z));
                assert!(any, "a hole in the roof at fine {x}, {z}");
                checked += 1;
            }
        }
        assert!(checked > 10_000);
    }

    /// Cost and vertices of the fine voxel ground at each level of detail,
    /// on the made-up landscape:
    /// `cargo test -p dyno-client --release ground_levels_cost -- --ignored --nocapture`
    #[test]
    #[ignore = "a measurement"]
    fn ground_levels_cost() {
        let world = test_world([256, 96, 256]);
        let light = Light::new(&world);
        for r in [8u8, 4, 2, 1] {
            let lod = Lod {
                per_metre: r,
                skirt: [0; 4],
                shade: true,
            };
            let (mut verts, mut ground, mut busy) = (0usize, 0usize, 0);
            let mut hist = [(0u64, 0f64, 0u64); 3];
            let t = std::time::Instant::now();
            for sy in 0..6 {
                for sz in 0..16 {
                    for sx in 0..16 {
                        let geo = mesh_lod(&world, &light, sx, sy, sz, lod);
                        verts += geo.opaque.len() + geo.ground.len();
                        ground += geo.ground.len();
                        busy += (!geo.opaque.is_empty()) as u32;
                        for q in geo.ground.as_chunks::<4>().0 {
                            let n = q[0].normal;
                            let d = if n[1] > 0.5 {
                                0
                            } else if n[1] < -0.5 {
                                1
                            } else {
                                2
                            };
                            let lo = [0, 1, 2]
                                .map(|a| q.iter().map(|v| v.pos[a]).fold(f32::MAX, f32::min));
                            let hi = [0, 1, 2]
                                .map(|a| q.iter().map(|v| v.pos[a]).fold(f32::MIN, f32::max));
                            let ext: Vec<f32> = (0..3)
                                .map(|a| (hi[a] - lo[a]) * r as f32)
                                .filter(|e| *e > 0.0)
                                .collect();
                            let area = ext.iter().product::<f32>();
                            hist[d].0 += 1;
                            hist[d].1 += area as f64;
                            if area <= 1.0 {
                                hist[d].2 += 1;
                            }
                        }
                    }
                }
            }
            let ms = t.elapsed().as_secs_f64() * 1e3;
            let pr = fine::PROF.with(|p| p.replace([0.0; 6]));
            for (name, h) in ["up", "down", "side"].iter().zip(hist) {
                println!(
                    "  {name}: {} quads, {:.1} voxel faces each, {} single",
                    h.0,
                    h.1 / h.0.max(1) as f64,
                    h.2
                );
            }
            println!(
                "  decide {:.0} ms ({} cells), paint {:.0} ms ({} voxels), emit {:.0} ms ({} quads)",
                pr[0], pr[3], pr[1], pr[4], pr[2], pr[5]
            );
            println!(
                "{r} a metre: {busy} sections, {:.3} ms each, {verts} vertices ({ground} ground)",
                ms / busy.max(1) as f64
            );
        }
    }

    /// Remesh cost of the town's sections (best of nine runs each) and the
    /// vertices they make:
    /// `cargo test -p dyno-client --release town_remesh_cost -- --ignored --nocapture`
    #[test]
    #[ignore = "a measurement"]
    fn town_remesh_cost() {
        let world = town();
        let light = Light::new(&world);
        let (mut times, mut verts) = (Vec::new(), 0usize);
        for sy in 0..4 {
            for sz in 0..8 {
                for sx in 0..8 {
                    let mut best = f64::MAX;
                    let mut n = 0;
                    for _ in 0..9 {
                        let t = std::time::Instant::now();
                        let geo = mesh(&world, &light, sx, sy, sz);
                        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
                        n = geo.opaque.len() + geo.trans.len();
                    }
                    if n > 0 {
                        times.push(best);
                        verts += n;
                    }
                }
            }
        }
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        let worst = times.iter().fold(0.0f64, |a, b| a.max(*b));
        let all = {
            let t = std::time::Instant::now();
            for sy in 0..4 {
                for sz in 0..8 {
                    for sx in 0..8 {
                        let _ = mesh(&world, &light, sx, sy, sz);
                    }
                }
            }
            t.elapsed().as_secs_f64() * 1000.0
        };
        println!(
            "town: {} sections, remesh avg {avg:.3} ms, worst {worst:.3} ms, all {all:.1} ms, {verts} vertices",
            times.len()
        );
        crate::shapes::STATS.with(|s| {
            let mut v: Vec<_> = s.borrow().iter().map(|(k, v)| (v.1, v.0, *k)).collect();
            v.sort_by(|a, b| b.cmp(a));
            for (verts, cells, (b, class)) in v.iter().take(12) {
                let class = &crate::shapes::pack().class_name(*class);
                println!("  block {b} {class}: {cells} cells, {verts} vertices");
            }
        });
    }

    #[test]
    fn detail_blocks_mesh_and_light() {
        // every section of the town meshes, the detail shapes make
        // triangles, and none of their vertices is black or broken
        let world = town();
        let light = Light::new(&world);
        let mut tris = 0;
        for sy in 0..4 {
            for sz in 0..8 {
                for sx in 0..8 {
                    let geo = mesh(&world, &light, sx, sy, sz);
                    for v in geo.opaque.iter().chain(&geo.trans) {
                        assert!(v.pos.iter().chain(&v.normal).all(|f| f.is_finite()));
                    }
                    assert_eq!(geo.opaque.len() % 3, 0);
                    tris += geo.opaque.len() / 3;
                }
            }
        }
        assert!(tris > 50_000, "{tris} triangles");
    }
}
