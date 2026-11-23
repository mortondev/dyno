//! Detail blocks: built blocks drawn as small hand-made meshes instead of
//! 1 m cubes. The world keeps its block grid (storage, light, collision);
//! only the picture changes. Each detail block picks its shape from its own
//! ID and its neighbours (auto-connect):
//!
//! - roof tiles, slate and thatch become straight slopes with shingle
//!   courses, eaves with a fascia board, ridge caps, barge boards and an
//!   underside, closed against the gable walls below;
//! - timber and logs become slim chamfered posts, rails and beams when they
//!   stand free, round trunks in the wild, and proud beams in a wall (a wall
//!   stays closed);
//! - free-standing stone columns become round pillars with a plinth and a
//!   capital;
//! - barrels, crates, hay, anvils, braziers, flower boxes, lanterns,
//!   banners, windows and doors get their own small meshes.
//!
//! A shape reads only the blocks within one cell of it, so every section
//! that holds a block decides the same and section borders stay seamless.
//! Shapes that no longer fill their cell stop hiding their neighbours'
//! faces (the cube mesher draws what is behind them), but still stop light.

use crate::light::Light;
use crate::render::{self, SECTION, Vertex};
use crate::terrain_mesh::{
    DX, DY, DZ, F_DARK, F_DETAIL, F_OPAQUE, Glow, Grid, SectionGeo, at, flags, hash3, is_paving,
    is_terrain,
};
use bytemuck::Zeroable;
use dyno_world::{self as w, BlockId};
use glam::Vec3;
use std::f32::consts::{FRAC_PI_4, PI, TAU};

const N: i32 = SECTION;
/// Most corners a flat polygon may have.
const MAX_POLY: usize = 16;
const MOSSY: BlockId = 19;
const DARK_PLANKS: BlockId = 20;
const WOOL_FIRST: BlockId = 23;
const WOOL_LAST: BlockId = 32;

fn is_roof(b: BlockId) -> bool {
    matches!(b, w::RED_ROOF_TILES | w::SLATE_ROOF | w::THATCH)
}

fn is_cloth(b: BlockId) -> bool {
    (WOOL_FIRST..=WOOL_LAST).contains(&b)
}

/// The blocks a sloped surface of `b` joins with: roofing with roofing,
/// cloth with cloth.
fn sheet_group(b: BlockId) -> fn(BlockId) -> bool {
    if is_cloth(b) { is_cloth } else { is_roof }
}

/// The wall a slope's gable closes against: for roofing a wall below, for
/// cloth more cloth (the closed end of a tent).
fn wedge_below(b: BlockId, below: BlockId) -> Option<BlockId> {
    let ok = if is_cloth(b) {
        is_cloth(below)
    } else {
        wallish(below) && !is_roof(below)
    };
    ok.then_some(below)
}

fn is_wood(b: BlockId) -> bool {
    matches!(b, w::TIMBER_BEAM | w::LOG | w::BIRCH_LOG)
}

fn is_stone(b: BlockId) -> bool {
    matches!(b, w::COBBLE | w::STONE_BRICKS | MOSSY | w::BRICK)
}

fn is_prop(b: BlockId) -> bool {
    matches!(
        b,
        w::BARREL | w::CRATE | w::HAY_BALE | w::ANVIL | w::BRAZIER | w::FLOWER_BED
    )
}

fn is_banner(b: BlockId) -> bool {
    matches!(b, w::RED_BANNER | w::BLUE_BANNER)
}

fn is_pane(b: BlockId) -> bool {
    matches!(b, w::LEADED_WINDOW | w::WOODEN_DOOR)
}

/// Blocks the detail mesher draws.
pub(crate) fn is_detail(b: BlockId) -> bool {
    is_roof(b)
        || is_wood(b)
        || is_stone(b)
        || is_prop(b)
        || is_banner(b)
        || is_pane(b)
        || is_cloth(b)
        || b == w::LANTERN
}

/// Blocks that may stand on paving without making it a wall.
pub(crate) fn stands_on_paving(b: BlockId) -> bool {
    is_prop(b) || is_banner(b) || b == w::LANTERN || b == w::TIMBER_BEAM
}

/// Part of a wall: a block that fills its cell and is not one of the
/// free-standing kinds (posts, roofs, props, cloth).
fn wallish(b: BlockId) -> bool {
    let f = flags()[b as usize];
    (f & F_OPAQUE != 0
        && !is_roof(b)
        && !is_wood(b)
        && !is_prop(b)
        && !is_banner(b)
        && b != w::LANTERN
        && !(WOOL_FIRST..=WOOL_LAST).contains(&b))
        || is_pane(b)
        || b == w::GLASS
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    /// drawn by the cube mesher
    Cube,
    /// a roof slope rising towards a direction (0 +x, 1 -x, 2 +z, 3 -z)
    Slope(u8),
    /// the top of a roof rising from both sides along an axis (0 x, 1 z)
    Peak(u8),
    /// timber in a wall: a full cube standing a little proud
    Beam,
    /// free-standing timber: post, rail or beam
    Stick,
    /// a round tree trunk
    Trunk,
    /// a round stone pillar
    Pillar,
    /// a prop, lantern, banner, window or door
    Prop,
    /// cloth stretched flat over open air (an awning)
    Sheet,
}

impl Shape {
    fn fills(self) -> bool {
        matches!(self, Shape::Cube | Shape::Beam)
    }
}

/// Horizontal steps: +x, -x, +z, -z.
const H4: [isize; 4] = [DX as isize, -(DX as isize), DZ as isize, -(DZ as isize)];
/// Quarter turns that take local +z to each of those directions.
const TURN: [u8; 4] = [1, 3, 0, 2];

#[inline]
fn nb(c: usize, d: isize) -> usize {
    (c as isize + d) as usize
}

fn shape(g: &Grid, c: usize) -> Shape {
    let b = g.ids[c];
    let id = |d: isize| g.ids[nb(c, d)];
    if is_roof(b) {
        return slope_shape(g, c, is_roof).unwrap_or(Shape::Cube);
    }
    if is_cloth(b) {
        // the top of a tent or an awning: cloth with open air above
        let below = id(-(DY as isize));
        if !w::is_solid(id(DY as isize)) && (!w::is_solid(below) || is_cloth(below)) {
            if let Some(s) = slope_shape(g, c, is_cloth) {
                return s;
            }
            if !w::is_solid(below) {
                return Shape::Sheet;
            }
        }
        return Shape::Cube;
    }
    if is_wood(b) {
        let wall_h = H4.iter().any(|d| wallish(id(*d)));
        let wood = |k: usize| is_wood(id(H4[k]));
        let wood_x = wood(0) || wood(1);
        let wood_z = wood(2) || wood(3);
        let wall_v = wallish(id(-(DY as isize))) || wallish(id(DY as isize));
        let corner = wood_x
            && wood_z
            && is_wood(id(-(DY as isize)))
            && w::is_solid(id(DY as isize));
        if wall_h || (wall_v && (wood_x || wood_z)) || corner {
            return Shape::Beam;
        }
        if b != w::TIMBER_BEAM && !wood_x && !wood_z {
            return Shape::Trunk;
        }
        return Shape::Stick;
    }
    if is_stone(b) {
        return if pillar_free(g, c)
            && (pillar_free(g, nb(c, -(DY as isize)))
                || pillar_free(g, nb(c, DY as isize))
                || ground_like(id(-(DY as isize))))
        {
            Shape::Pillar
        } else {
            Shape::Cube
        };
    }
    Shape::Prop
}

/// How a sloped block lies, from the blocks of its group diagonally above
/// and below it: a slope rising one way, the peak between two slopes, or
/// nothing that reads as a slope.
fn slope_shape(g: &Grid, c: usize, group: fn(BlockId) -> bool) -> Option<Shape> {
    let roof = |d: isize| group(g.ids[nb(c, d)]);
    let dy = DY as isize;
    // the top of a two-sided roof: roof steps down on both sides
    for axis in 0..2 {
        let d = H4[axis * 2];
        if roof(-d - dy) && roof(d - dy) && !roof(d + dy) && !roof(-d + dy) {
            return Some(Shape::Peak(axis as u8));
        }
    }
    let mut best = (0, 0u8);
    for (k, d) in H4.iter().enumerate() {
        let score = roof(d + dy) as i32 * 2 + roof(-d - dy) as i32 * 2
            - roof(-d + dy) as i32
            - roof(d - dy) as i32;
        if score > best.0 {
            best = (score, k as u8);
        }
    }
    (best.0 > 0).then_some(Shape::Slope(best.1))
}

fn ground_like(b: BlockId) -> bool {
    is_terrain(b) || is_paving(b)
}

/// A stone block with nothing built beside it (and no roof near it, so
/// chimneys stay square).
fn pillar_free(g: &Grid, c: usize) -> bool {
    if !is_stone(g.ids[c]) {
        return false;
    }
    for d in H4 {
        if wallish(g.ids[nb(c, d)]) {
            return false;
        }
    }
    for d in [
        DX as isize + DZ as isize,
        DX as isize - DZ as isize,
        -(DX as isize) + DZ as isize,
        -(DX as isize) - (DZ as isize),
    ]
    .into_iter()
    .chain(H4)
    {
        if is_roof(g.ids[nb(c, d)]) {
            return false;
        }
    }
    true
}

/// Decide which detail blocks still fill their cell. Called on a freshly
/// loaded grid, for the cells whose faces and corners the section uses.
pub(crate) fn classify(g: &mut Grid) {
    let mut thin = Vec::new();
    for y in -2..N + 2 {
        for z in -2..N + 2 {
            for x in -2..N + 2 {
                let c = at(x, y, z);
                if g.fl[c] & F_DETAIL == 0 {
                    continue;
                }
                if !shape(g, c).fills() {
                    thin.push(c);
                }
            }
        }
    }
    for c in thin {
        g.fl[c] &= !F_OPAQUE;
    }
}

// ------------------------------------------------------------------ kit

/// A texture layer and a tint for it.
#[derive(Clone, Copy)]
struct Mat {
    layer: u16,
    /// the layer's animation, as the shader wants it
    anim: f32,
    tint: [f32; 3],
    /// lit by itself (lantern glass, embers)
    glow: bool,
}

fn mat(b: BlockId, face: usize) -> Mat {
    let layer = render::textures().face(b, face);
    Mat {
        layer,
        anim: render::layer_anim(layer),
        tint: [1.0; 3],
        glow: false,
    }
}

impl Mat {
    fn tint(mut self, k: f32) -> Mat {
        self.tint = [self.tint[0] * k, self.tint[1] * k, self.tint[2] * k];
        self
    }
    fn tinted(mut self, c: [f32; 3]) -> Mat {
        self.tint = c;
        self
    }
    fn glowing(mut self) -> Mat {
        self.glow = true;
        self
    }
}

/// How a box face is mapped: by position in the cell, or the whole
/// texture stretched over the face.
#[derive(Clone, Copy)]
enum Uv {
    Cell,
    Fit,
    /// by position, scaled
    Scale(f32),
}

/// Light and occlusion of every cell of a section's grid, worked out once
/// for all the detail blocks in it.
#[derive(Clone, Copy, Default)]
struct Lit {
    bl: f32,
    sl: f32,
    /// how much the cell closes in what is beside it
    occ: f32,
    /// light can be read here (it does not fill its cell)
    open: bool,
}

fn light_field(g: &Grid, light: &Light) -> Vec<Lit> {
    let mut f = vec![Lit::default(); g.fl.len()];
    for y in -2..N + 2 {
        for z in -2..N + 2 {
            for x in -2..N + 2 {
                let c = at(x, y, z);
                let fl = g.fl[c];
                if fl & F_OPAQUE != 0 {
                    f[c].occ = 1.0;
                    continue;
                }
                let (bl, sl) = g.light(light, [x, y, z]);
                f[c] = Lit {
                    bl,
                    sl,
                    occ: if fl & F_DETAIL != 0 && fl & F_DARK != 0 {
                        0.3
                    } else {
                        0.0
                    },
                    open: true,
                };
            }
        }
    }
    f
}

/// Writes the triangles of one detail block, in the block's own frame: the
/// cell is 0..1 on each axis, turned about its vertical centre line.
struct Kit<'a> {
    g: &'a Grid,
    field: &'a [Lit],
    opaque: &'a mut Vec<Vertex>,
    trans: &'a mut Vec<Vertex>,
    /// world position of the cell's low corner
    base: Vec3,
    cos: f32,
    sin: f32,
    /// translucent output
    clear: bool,
    /// texture repeats per unit along a lathe's axis
    vscale: f32,
}

impl Kit<'_> {
    fn turn(&mut self, angle: f32) {
        self.cos = angle.cos();
        self.sin = angle.sin();
    }

    fn quarter(&mut self, q: u8) {
        self.turn(q as f32 * PI * 0.5);
    }

    #[inline]
    fn dir(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            v.x * self.cos + v.z * self.sin,
            v.y,
            -v.x * self.sin + v.z * self.cos,
        )
    }

    #[inline]
    fn pos(&self, p: Vec3) -> Vec3 {
        self.base + Vec3::new(0.5, 0.0, 0.5) + self.dir(p - Vec3::new(0.5, 0.0, 0.5))
    }

    /// Block light, sky light and occlusion just off a surface, blended
    /// over the eight cells around the point.
    fn shade(&self, p: Vec3, n: Vec3) -> (f32, f32, f32) {
        let g = self.g;
        let o = Vec3::new(g.o[0] as f32, g.o[1] as f32, g.o[2] as f32);
        let q = p + n * 0.5 - o - 0.5;
        let i0 = q.floor();
        let f = q - i0;
        let lim = |v: f32| (v as i32).clamp(-2, N + 1);
        let (x0, y0, z0) = (lim(i0.x), lim(i0.y), lim(i0.z));
        let (x1, y1, z1) = (lim(i0.x + 1.0), lim(i0.y + 1.0), lim(i0.z + 1.0));
        let (mut bl, mut sl, mut wsum, mut occ) = (0.0, 0.0, 0.0, 0.0);
        for (cy, wy) in [(y0, 1.0 - f.y), (y1, f.y)] {
            for (cz, wz) in [(z0, 1.0 - f.z), (z1, f.z)] {
                for (cx, wx) in [(x0, 1.0 - f.x), (x1, f.x)] {
                    let wgt = wx * wy * wz;
                    let l = self.field[at(cx, cy, cz)];
                    occ += wgt * l.occ;
                    if l.open {
                        bl += l.bl * wgt;
                        sl += l.sl * wgt;
                        wsum += wgt;
                    }
                }
            }
        }
        let (bl, sl) = if wsum > 0.01 {
            (bl / wsum, sl / wsum)
        } else {
            let c = (p + n * 0.9 - o).floor();
            let l = self.field[at(lim(c.x), lim(c.y), lim(c.z))];
            (l.bl, l.sl)
        };
        let ao = (1.0 - occ * 0.55).clamp(0.45, 1.0);
        (bl / 15.0, sl / 15.0, ao)
    }

    fn vertex(&self, p: Vec3, n: Vec3, uv: [f32; 2], m: Mat) -> Vertex {
        let (wp, wn) = (self.pos(p), self.dir(n).normalize_or_zero());
        let lit = if m.glow { (0.0, 0.0, 1.0) } else { self.shade(wp, wn) };
        Self::finish(wp, wn, uv, m, lit)
    }

    /// A vertex at world position `wp` with world normal `wn`, already lit.
    #[inline]
    fn finish(wp: Vec3, wn: Vec3, uv: [f32; 2], m: Mat, lit: (f32, f32, f32)) -> Vertex {
        Vertex {
            pos: wp.into(),
            normal: wn.into(),
            color: [
                m.tint[0],
                m.tint[1],
                m.tint[2],
                if m.glow { 2.0 } else { lit.2 },
            ],
            light: lit.0,
            sky: lit.1,
            tex: [uv[0], uv[1], m.layer as f32, m.anim],
        }
    }

    fn out(&mut self) -> &mut Vec<Vertex> {
        if self.clear {
            self.trans
        } else {
            self.opaque
        }
    }

    /// A flat polygon (convex, corners in order) with one normal.
    fn poly(&mut self, ps: &[Vec3], n: Vec3, uvs: &[[f32; 2]], m: Mat) {
        let mut vs = [Vertex::zeroed(); MAX_POLY];
        let k = ps.len().min(MAX_POLY);
        // a small polygon takes one light, from its middle
        let (mut lo, mut hi) = (ps[0], ps[0]);
        for p in &ps[..k] {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let wn = self.dir(n).normalize_or_zero();
        let one = (hi - lo).max_element() < 0.4;
        let lit = if m.glow {
            (0.0, 0.0, 1.0)
        } else if one {
            self.shade(self.pos((lo + hi) * 0.5), wn)
        } else {
            (0.0, 0.0, 0.0)
        };
        for i in 0..k {
            let wp = self.pos(ps[i]);
            let l = if one || m.glow { lit } else { self.shade(wp, wn) };
            vs[i] = Self::finish(wp, wn, uvs[i], m, l);
        }
        let out = self.out();
        for i in 1..k - 1 {
            out.extend([vs[0], vs[i], vs[i + 1]]);
        }
    }

    /// A polygon mapped by position (`Uv::Cell` or `Uv::Scale`).
    fn poly_at(&mut self, ps: &[Vec3], n: Vec3, uv: Uv, m: Mat) {
        let mut uvs = [[0.0; 2]; MAX_POLY];
        for (i, p) in ps.iter().take(MAX_POLY).enumerate() {
            uvs[i] = map_uv(*p, n, uv, None);
        }
        self.poly(ps, n, &uvs[..ps.len().min(MAX_POLY)], m);
    }

    /// An axis-aligned box. `mats` are top, sides, bottom. `skip` bits:
    /// +x, -x, +y, -y, +z, -z.
    fn cuboid(&mut self, lo: Vec3, hi: Vec3, mats: [Mat; 3], uv: Uv, skip: u8) {
        self.bevel(lo, hi, 0.0, mats, uv, skip);
    }

    /// A box with its edges bevelled by `e`.
    fn bevel(&mut self, lo: Vec3, hi: Vec3, e: f32, mats: [Mat; 3], uv: Uv, skip: u8) {
        let e = e.min((hi - lo).min_element() * 0.45).max(0.0);
        let fit = Some((lo, hi));
        // faces
        for (k, (n, a, b)) in BOX_FACES.iter().enumerate() {
            if skip & (1 << k) != 0 {
                continue;
            }
            let (n, a, b) = (*n, *a, *b);
            let ax = |v: Vec3| v.abs().max_position();
            let (ia, ib, inn) = (ax(a), ax(b), ax(n));
            let mut corner = |sa: f32, sb: f32| {
                let mut p = Vec3::ZERO;
                p[inn] = if n[inn] > 0.0 { hi[inn] } else { lo[inn] };
                p[ia] = if sa > 0.0 { hi[ia] - e } else { lo[ia] + e };
                p[ib] = if sb > 0.0 { hi[ib] - e } else { lo[ib] + e };
                p
            };
            let ps = [
                corner(-1.0, -1.0),
                corner(1.0, -1.0),
                corner(1.0, 1.0),
                corner(-1.0, 1.0),
            ];
            let m = match n.y as i32 {
                1 => mats[0],
                -1 => mats[2],
                _ => mats[1],
            };
            let uvs = ps.map(|p| map_uv(p, n, uv, if matches!(uv, Uv::Fit) { fit } else { None }));
            self.poly(&ps, n, &uvs, m);
        }
        if e <= 0.0 {
            return;
        }
        // edges: a strip between each pair of neighbouring faces
        for i in 0..3 {
            for sa in [-1.0f32, 1.0] {
                for sb in [-1.0f32, 1.0] {
                    let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                    let pick = |v: f32, ax: usize, inset: bool| {
                        if v > 0.0 {
                            hi[ax] - if inset { e } else { 0.0 }
                        } else {
                            lo[ax] + if inset { e } else { 0.0 }
                        }
                    };
                    let mut p = [Vec3::ZERO; 4];
                    for (q, (along, on_j)) in
                        [(-1.0, true), (1.0, true), (1.0, false), (-1.0, false)]
                            .into_iter()
                            .enumerate()
                    {
                        p[q][i] = if along > 0.0 { hi[i] - e } else { lo[i] + e };
                        p[q][j] = pick(sa, j, !on_j);
                        p[q][k] = pick(sb, k, on_j);
                    }
                    let mut n = Vec3::ZERO;
                    n[j] = sa;
                    n[k] = sb;
                    let n = n.normalize();
                    let m = if n.y > 0.5 {
                        mats[0]
                    } else if n.y < -0.5 {
                        mats[2]
                    } else {
                        mats[1]
                    };
                    let uvs =
                        p.map(|q| map_uv(q, n, uv, if matches!(uv, Uv::Fit) { fit } else { None }));
                    self.poly(&p, n, &uvs, m);
                }
            }
        }
        // corners
        for sx in [-1.0f32, 1.0] {
            for sy in [-1.0f32, 1.0] {
                for sz in [-1.0f32, 1.0] {
                    let c = |v: f32, ax: usize, inset: bool| {
                        if v > 0.0 {
                            hi[ax] - if inset { e } else { 0.0 }
                        } else {
                            lo[ax] + if inset { e } else { 0.0 }
                        }
                    };
                    let ps = [
                        Vec3::new(c(sx, 0, false), c(sy, 1, true), c(sz, 2, true)),
                        Vec3::new(c(sx, 0, true), c(sy, 1, false), c(sz, 2, true)),
                        Vec3::new(c(sx, 0, true), c(sy, 1, true), c(sz, 2, false)),
                    ];
                    let n = Vec3::new(sx, sy, sz).normalize();
                    let m = if sy > 0.0 { mats[0] } else { mats[1] };
                    let uvs = ps.map(|q| map_uv(q, n, uv, None));
                    self.poly(&ps, n, &uvs, m);
                }
            }
        }
    }

    /// A box from eight corners (any shape that is roughly a box): low
    /// face 0..4 and high face 4..8 in the same order. Normals face out.
    fn hexa(&mut self, c: [Vec3; 8], uv: Uv, m: Mat, skip: u8) {
        let mid = c.iter().fold(Vec3::ZERO, |a, p| a + *p) / 8.0;
        let faces: [[usize; 4]; 6] = [
            [0, 1, 2, 3],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        for (k, f) in faces.iter().enumerate() {
            if skip & (1 << k) != 0 {
                continue;
            }
            let ps = [c[f[0]], c[f[1]], c[f[2]], c[f[3]]];
            let mut n = (ps[2] - ps[0]).cross(ps[3] - ps[1]).normalize_or_zero();
            let centre = (ps[0] + ps[1] + ps[2] + ps[3]) / 4.0;
            if n.dot(centre - mid) < 0.0 {
                n = -n;
            }
            self.poly_at(&ps, n, uv, m);
        }
    }

    /// A solid of revolution along `axis` (0 x, 1 y, 2 z) through `at`
    /// (the other two coordinates). `profile` is (distance along, radius).
    /// `flat` gives each side its own normal (chamfered posts). `u_turns`
    /// is how many times the texture wraps round.
    #[allow(clippy::too_many_arguments)]
    fn lathe(
        &mut self,
        axis: usize,
        at: [f32; 2],
        profile: &[(f32, f32)],
        segs: usize,
        phase: f32,
        flat: bool,
        u_turns: f32,
        m: Mat,
        caps: [Option<Mat>; 2],
    ) {
        let place = |a: f32, r: f32, t: f32| -> (Vec3, Vec3) {
            let (s, c) = (t.sin(), t.cos());
            let (u, v) = (at[0] + r * c, at[1] + r * s);
            let radial = (c, s);
            match axis {
                0 => (Vec3::new(a, u, v), Vec3::new(0.0, radial.0, radial.1)),
                1 => (Vec3::new(u, a, v), Vec3::new(radial.0, 0.0, radial.1)),
                _ => (Vec3::new(u, v, a), Vec3::new(radial.0, radial.1, 0.0)),
            }
        };
        let axis_dir = {
            let mut d = Vec3::ZERO;
            d[axis] = 1.0;
            d
        };
        let vk = self.vscale;
        let mut ring: Vec<Vertex> = Vec::with_capacity((segs + 1) * 2);
        for w in profile.windows(2) {
            let ((a0, r0), (a1, r1)) = (w[0], w[1]);
            if r0 <= 0.0 && r1 <= 0.0 {
                continue;
            }
            // slope of the side: tips the normal along the axis
            let len = ((a1 - a0).powi(2) + (r1 - r0).powi(2)).sqrt().max(1e-5);
            let (na, nr) = ((r0 - r1) / len, (a1 - a0).abs() / len);
            let sgn = if a1 >= a0 { 1.0 } else { -1.0 };
            let nrm = |d: Vec3| (d * nr + axis_dir * na * sgn).normalize_or_zero();
            let u_at = |s: usize| u_turns * s as f32 / segs as f32;
            let t_at = |s: usize| phase + TAU * s as f32 / segs as f32;
            if flat {
                // one normal and one light per side
                for s in 0..segs {
                    let (t0, t1) = (t_at(s), t_at(s + 1));
                    let (_, dm) = place(0.0, 1.0, (t0 + t1) * 0.5);
                    let n = nrm(dm);
                    let ps = [
                        place(a0, r0, t0).0,
                        place(a0, r0, t1).0,
                        place(a1, r1, t1).0,
                        place(a1, r1, t0).0,
                    ];
                    let uvs = [
                        [u_at(s), -a0 * vk],
                        [u_at(s + 1), -a0 * vk],
                        [u_at(s + 1), -a1 * vk],
                        [u_at(s), -a1 * vk],
                    ];
                    let wn = self.dir(n).normalize_or_zero();
                    let mid = self.pos((ps[0] + ps[2]) * 0.5);
                    let lit = if m.glow { (0.0, 0.0, 1.0) } else { self.shade(mid, wn) };
                    let v = ps.map(|p| self.pos(p));
                    let q = [0, 1, 2, 3].map(|i| Self::finish(v[i], wn, uvs[i], m, lit));
                    let out = self.out();
                    out.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
                }
            } else {
                // each ring vertex lit once, shared by the two sides it joins
                ring.clear();
                for (a, r) in [(a0, r0), (a1, r1)] {
                    for s in 0..=segs {
                        let (p, d) = place(a, r, t_at(s));
                        ring.push(self.vertex(p, nrm(d), [u_at(s), -a * vk], m));
                    }
                }
                let row = segs + 1;
                for s in 0..segs {
                    let q = [ring[s], ring[s + 1], ring[row + s + 1], ring[row + s]];
                    let out = self.out();
                    out.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
                }
            }
        }
        // end caps: discs mapped across the texture
        for (end, cm) in caps.iter().enumerate() {
            let Some(cm) = cm else { continue };
            let (a, r) = if end == 0 {
                profile[0]
            } else {
                profile[profile.len() - 1]
            };
            if r <= 0.0 {
                continue;
            }
            let n = if end == 0 { -axis_dir } else { axis_dir };
            let mut ps = Vec::with_capacity(segs);
            let mut uvs = Vec::with_capacity(segs);
            for s in 0..segs {
                let t = phase + TAU * s as f32 / segs as f32;
                let (p, _) = place(a, r, t);
                ps.push(p);
                uvs.push([0.5 + 0.5 * t.cos(), 0.5 + 0.5 * t.sin()]);
            }
            self.poly(&ps, n, &uvs, *cm);
        }
    }
}

/// The six faces of a box: normal and the two axes across it.
const BOX_FACES: [(Vec3, Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::Z, Vec3::Y),
    (Vec3::NEG_X, Vec3::Z, Vec3::Y),
    (Vec3::Y, Vec3::X, Vec3::Z),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    (Vec3::Z, Vec3::X, Vec3::Y),
    (Vec3::NEG_Z, Vec3::X, Vec3::Y),
];
const SKIP_PX: u8 = 1;
const SKIP_NX: u8 = 2;
const SKIP_PY: u8 = 4;
const SKIP_NY: u8 = 8;
const SKIP_PZ: u8 = 16;
const SKIP_NZ: u8 = 32;

/// Texture coordinates for a point on a surface, projected along the
/// normal's main axis. Sides run down from the top, as on cubes.
fn map_uv(p: Vec3, n: Vec3, uv: Uv, fit: Option<(Vec3, Vec3)>) -> [f32; 2] {
    let q = match (uv, fit) {
        (Uv::Fit, Some((lo, hi))) => (p - lo) / (hi - lo).max(Vec3::splat(1e-4)),
        (Uv::Scale(s), _) => p * s,
        _ => p,
    };
    let a = n.abs();
    if a.y >= a.x && a.y >= a.z {
        [q.x, q.z]
    } else if a.x >= a.z {
        [if n.x > 0.0 { 1.0 - q.z } else { q.z }, 1.0 - q.y]
    } else {
        [if n.z < 0.0 { 1.0 - q.x } else { q.x }, 1.0 - q.y]
    }
}

// --------------------------------------------------------------- meshing

/// Draw the section's own detail blocks.
pub(crate) fn mesh(g: &Grid, light: &Light, geo: &mut SectionGeo) {
    let SectionGeo {
        opaque,
        trans,
        glows,
        ..
    } = geo;
    let field = light_field(g, light);
    let mut kit = Kit {
        g,
        field: &field,
        opaque,
        trans,
        base: Vec3::ZERO,
        cos: 1.0,
        sin: 0.0,
        clear: false,
        vscale: 1.0,
    };
    for y in 0..N {
        for z in 0..N {
            for x in 0..N {
                let c = at(x, y, z);
                if g.fl[c] & F_DETAIL == 0 {
                    continue;
                }
                let s = shape(g, c);
                if s == Shape::Cube {
                    continue;
                }
                let wp = [g.o[0] + x, g.o[1] + y, g.o[2] + z];
                kit.base = Vec3::new(wp[0] as f32, wp[1] as f32, wp[2] as f32);
                kit.turn(0.0);
                kit.clear = false;
                let b = g.ids[c];
                match s {
                    Shape::Slope(d) => roof_slope(&mut kit, c, d),
                    Shape::Peak(a) => roof_peak(&mut kit, c, a),
                    Shape::Beam => beam(&mut kit, c),
                    Shape::Stick => stick(&mut kit, c),
                    Shape::Trunk => trunk(&mut kit, c),
                    Shape::Pillar => pillar(&mut kit, c),
                    Shape::Sheet => sheet(&mut kit, c),
                    Shape::Prop => {
                        if let Some(glow) = prop(&mut kit, c, b, wp) {
                            glows.push(glow);
                        }
                    }
                    Shape::Cube => {}
                }
            }
        }
    }
}

fn wood_mat() -> Mat {
    mat(w::TIMBER_BEAM, 1)
}

fn iron_mat() -> Mat {
    mat(w::BRAZIER, 2).tint(0.9)
}

/// Shape numbers of each roof material: lift of the top over the cell's
/// diagonal, thickness, shingle courses per cell and the step between them.
fn roof_params(b: BlockId) -> (f32, f32, usize, f32) {
    if is_cloth(b) {
        return (0.03, 0.05, 1, 0.0);
    }
    match b {
        w::THATCH => (0.20, 0.36, 2, 0.08),
        w::SLATE_ROOF => (0.09, 0.15, 4, 0.04),
        _ => (0.10, 0.17, 4, 0.05),
    }
}

/// How far an eave reaches out past the last cell of a roof.
const EAVE: f32 = 0.34;

/// One side of a roof over `s` in [s0, s1] (local +z is up the slope, the
/// top surface is at height s + lift). `ends`: draw the gable ends at
/// local x = 0 and x = 1. `wedge`: the wall under the slope, to close the
/// gap between the wall top and the slab.
#[allow(clippy::too_many_arguments)]
fn slope_part(
    k: &mut Kit,
    b: BlockId,
    s0: f32,
    s1: f32,
    eave: bool,
    ends: [bool; 2],
    wedge: Option<BlockId>,
    wedge_back: bool,
) {
    let (lift, thick, courses, step) = roof_params(b);
    let cloth = is_cloth(b);
    let top_m = mat(b, 1);
    let under_m = if cloth {
        mat(b, 1).tint(0.8)
    } else {
        mat(DARK_PLANKS, 1).tint(0.85)
    };
    let eave = eave && !cloth;
    let trim = wood_mat().tint(0.85);
    let x0 = if ends[0] { -0.04 } else { 0.0 };
    let x1 = if ends[1] { 1.04 } else { 1.0 };
    // the courses of shingles, lowest first
    let mut rows: Vec<(f32, f32)> = Vec::new();
    let s_lo = if eave { s0 - EAVE } else { s0 };
    if eave {
        rows.push((s_lo, s0));
    }
    let per = 1.0 / courses as f32;
    let mut s = s0;
    while s < s1 - 1e-4 {
        let e = (s + per).min(s1);
        rows.push((s, e));
        s = e;
    }
    // across the slope in world terms, so the texture runs on between cells
    let across = k.dir(Vec3::X);
    let u_of = |k: &Kit, p: Vec3| k.pos(p).dot(across);
    let cell_y = k.base.y;
    for (i, (a, e)) in rows.iter().copied().enumerate() {
        let h = e - a;
        let lip = if i == 0 && !eave { step * 0.5 } else { step };
        let (ya, ye) = (a + lift + lip, e + lift);
        let ps = [
            Vec3::new(x0, ya, a),
            Vec3::new(x1, ya, a),
            Vec3::new(x1, ye, e),
            Vec3::new(x0, ye, e),
        ];
        let dyds = (ye - ya) / h;
        let n = Vec3::new(0.0, 1.0, -dyds).normalize();
        let uvs: Vec<[f32; 2]> = ps
            .iter()
            .map(|p| [u_of(k, *p), -(cell_y + p.z)])
            .collect();
        k.poly(&ps, n, &uvs, top_m);
        // the lower edge of the course, standing on the one below
        let lo = if i == 0 { ya - lip - thick * 0.6 } else { a + lift };
        if i > 0 || eave {
            let ps = [
                Vec3::new(x0, lo, a),
                Vec3::new(x1, lo, a),
                Vec3::new(x1, ya, a),
                Vec3::new(x0, ya, a),
            ];
            let uvs: Vec<[f32; 2]> = ps
                .iter()
                .map(|p| [u_of(k, *p), -(cell_y + a) + (p.y - ya) * 0.5])
                .collect();
            k.poly(&ps, Vec3::new(0.0, -0.2, -1.0).normalize(), &uvs, top_m.tint(0.8));
        }
    }
    // underside
    let (ua, ue) = (s_lo + lift - thick, s1 + lift - thick);
    let n = Vec3::new(0.0, -1.0, 1.0).normalize();
    k.poly_at(
        &[
            Vec3::new(x0, ua, s_lo),
            Vec3::new(x1, ua, s_lo),
            Vec3::new(x1, ue, s1),
            Vec3::new(x0, ue, s1),
        ],
        n,
        Uv::Cell,
        under_m,
    );
    // rafter ends under the eave
    if eave {
        let fa = s_lo;
        for rx in [0.25f32, 0.75] {
            let (w0, w1) = (rx - 0.05, rx + 0.05);
            let y = |s: f32| s + lift - thick;
            k.hexa(
                [
                    Vec3::new(w0, y(fa + 0.04) - 0.12, fa + 0.04),
                    Vec3::new(w1, y(fa + 0.04) - 0.12, fa + 0.04),
                    Vec3::new(w1, y(s0 + 0.2) - 0.12, s0 + 0.2),
                    Vec3::new(w0, y(s0 + 0.2) - 0.12, s0 + 0.2),
                    Vec3::new(w0, y(fa + 0.04), fa + 0.04),
                    Vec3::new(w1, y(fa + 0.04), fa + 0.04),
                    Vec3::new(w1, y(s0 + 0.2), s0 + 0.2),
                    Vec3::new(w0, y(s0 + 0.2), s0 + 0.2),
                ],
                Uv::Cell,
                trim,
                2,
            );
        }
        // fascia board along the eave
        let (fy0, fy1) = (s_lo + lift - thick - 0.05, s_lo + lift + step + 0.01);
        k.cuboid(
            Vec3::new(x0, fy0, s_lo - 0.05),
            Vec3::new(x1, fy1, s_lo),
            [trim; 3],
            Uv::Cell,
            0,
        );
    }
    // gable ends: close the slab, and a barge board along it
    for (side, open) in ends.iter().enumerate() {
        if !*open || cloth {
            continue;
        }
        let x = if side == 0 { x0 } else { x1 };
        let n = Vec3::new(if side == 0 { -1.0 } else { 1.0 }, 0.0, 0.0);
        k.poly_at(
            &[
                Vec3::new(x, s_lo + lift - thick, s_lo),
                Vec3::new(x, s1 + lift - thick, s1),
                Vec3::new(x, s1 + lift, s1),
                Vec3::new(x, s_lo + lift + step, s_lo),
            ],
            n,
            Uv::Cell,
            under_m,
        );
        let (bx0, bx1) = if side == 0 {
            (x - 0.05, x)
        } else {
            (x, x + 0.05)
        };
        let yl = |s: f32| s + lift - thick - 0.06;
        let yh = |s: f32| s + lift + step + 0.05;
        k.hexa(
            [
                Vec3::new(bx0, yl(s_lo), s_lo),
                Vec3::new(bx1, yl(s_lo), s_lo),
                Vec3::new(bx1, yl(s1), s1),
                Vec3::new(bx0, yl(s1), s1),
                Vec3::new(bx0, yh(s_lo), s_lo),
                Vec3::new(bx1, yh(s_lo), s_lo),
                Vec3::new(bx1, yh(s1), s1),
                Vec3::new(bx0, yh(s1), s1),
            ],
            Uv::Cell,
            trim,
            0,
        );
    }
    // the gable wall under the slab
    if let Some(wb) = wedge {
        let wm = mat(wb, 1);
        let sw = (thick - lift).max(0.0);
        let yu = |s: f32| s + lift - thick;
        if s1 > sw {
            for (side, x) in [(0, 0.0f32), (1, 1.0)] {
                if !ends[side] && !wedge_open(k, side) {
                    continue;
                }
                let n = Vec3::new(if side == 0 { -1.0 } else { 1.0 }, 0.0, 0.0);
                k.poly_at(
                    &[
                        Vec3::new(x, 0.0, sw.max(s0)),
                        Vec3::new(x, 0.0, s1),
                        Vec3::new(x, yu(s1), s1),
                        Vec3::new(x, yu(sw.max(s0)).max(0.0), sw.max(s0)),
                    ],
                    n,
                    Uv::Cell,
                    wm,
                );
            }
            if wedge_back {
                k.poly_at(
                    &[
                        Vec3::new(0.0, 0.0, s1),
                        Vec3::new(1.0, 0.0, s1),
                        Vec3::new(1.0, yu(s1), s1),
                        Vec3::new(0.0, yu(s1), s1),
                    ],
                    Vec3::Z,
                    Uv::Cell,
                    wm,
                );
            }
        }
    }
}

/// True when the gable wedge's side faces the open (its neighbour across
/// the roof is not a wedge too). `side` 0 is local -x.
fn wedge_open(k: &Kit, side: usize) -> bool {
    // world step for local ∓x
    let d = k.dir(if side == 0 { Vec3::NEG_X } else { Vec3::X });
    let q = (k.base + Vec3::splat(0.5) + d).floor();
    let g = k.g;
    let l = [
        q.x as i32 - g.o[0],
        q.y as i32 - g.o[1],
        q.z as i32 - g.o[2],
    ];
    let c = at(l[0], l[1], l[2]);
    let b = g.ids[c];
    !(sheet_group(k.g.ids[at(
        (k.base.x as i32) - g.o[0],
        (k.base.y as i32) - g.o[1],
        (k.base.z as i32) - g.o[2],
    )])(b)
        && wedge_below(b, g.ids[c - DY]).is_some())
}

fn roof_slope(k: &mut Kit, c: usize, d: u8) {
    let g = k.g;
    let b = g.ids[c];
    let dd = H4[d as usize];
    let dy = DY as isize;
    let group = sheet_group(b);
    let roof = |q: isize| group(g.ids[nb(c, q)]);
    k.quarter(TURN[d as usize]);
    // local ±x in grid steps
    let lx = |k: &Kit, s: f32| {
        let v = k.dir(Vec3::new(s, 0.0, 0.0));
        (v.x.round() as isize) * DX as isize + (v.z.round() as isize) * DZ as isize
    };
    let (left, right) = (lx(k, -1.0), lx(k, 1.0));
    let ends = [!roof(left), !roof(right)];
    let eave = !roof(-dd - dy);
    let wedge = wedge_below(b, g.ids[nb(c, -dy)]);
    let back_open = g.fl[nb(c, dd)] & F_OPAQUE == 0 && !roof(dd);
    slope_part(k, b, 0.0, 1.0, eave, ends, wedge, back_open);
    // the top edge: a ridge cap where the other side meets it, or a capped
    // edge where the roof stops in the open
    if !roof(dd + dy) && !is_cloth(b) {
        let (lift, _, _, _) = roof_params(b);
        let ahead = g.ids[nb(c, dd)];
        if is_roof(ahead) || !wallish(ahead) {
            let cap = mat(b, 1).tint(0.72);
            let (x0, x1) = (
                if ends[0] { -0.06 } else { 0.0 },
                if ends[1] { 1.06 } else { 1.0 },
            );
            let z1 = if is_roof(ahead) { 1.0 } else { 1.04 };
            k.cuboid(
                Vec3::new(x0, 0.84 + lift, 0.84),
                Vec3::new(x1, 1.0 + lift + 0.09, z1),
                [cap; 3],
                Uv::Cell,
                if is_roof(ahead) { SKIP_PZ } else { 0 },
            );
        }
    }
}

fn roof_peak(k: &mut Kit, c: usize, axis: u8) {
    let g = k.g;
    let b = g.ids[c];
    let wedge = wedge_below(b, g.ids[c - DY]);
    let group = sheet_group(b);
    // local +z up the near side: quarter turn so the rise runs along `axis`
    let q = if axis == 0 { 1 } else { 0 };
    let (lift, _, _, _) = roof_params(b);
    for half in 0..2u8 {
        k.quarter(q + half * 2);
        let lx = |k: &Kit, s: f32| {
            let v = k.dir(Vec3::new(s, 0.0, 0.0));
            (v.x.round() as isize) * DX as isize + (v.z.round() as isize) * DZ as isize
        };
        let ends = [
            !group(g.ids[nb(c, lx(k, -1.0))]),
            !group(g.ids[nb(c, lx(k, 1.0))]),
        ];
        // each half runs from its cell edge up to the middle
        slope_part(k, b, 0.0, 0.5, false, ends, wedge, false);
        if half == 1 && !is_cloth(b) {
            // ridge roll along the top
            let cap = mat(b, 1).tint(0.72);
            let (x0, x1) = (
                if ends[0] { -0.08 } else { 0.0 },
                if ends[1] { 1.08 } else { 1.0 },
            );
            k.lathe(
                0,
                [0.5 + lift + 0.02, 0.5],
                &[(x0, 0.11), (x1, 0.11)],
                8,
                FRAC_PI_4 * 0.5,
                false,
                1.0,
                cap,
                [Some(cap), Some(cap)],
            );
        }
    }
}

/// Timber in a wall: stands a few centimetres proud of the plaster on the
/// open sides, with its edges showing where the run of timber ends.
fn beam(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let m = mat(b, 1);
    const P: f32 = 0.045;
    let proud = |q: usize| is_wood(g.ids[q]) && g.fl[q] & F_OPAQUE != 0;
    let dirs: [(isize, usize, bool); 6] = [
        (DX as isize, 0, true),
        (-(DX as isize), 0, false),
        (DY as isize, 1, true),
        (-(DY as isize), 1, false),
        (DZ as isize, 2, true),
        (-(DZ as isize), 2, false),
    ];
    for (d, axis, pos) in dirs {
        let q = nb(c, d);
        if g.fl[q] & F_OPAQUE != 0 || is_roof(g.ids[q]) {
            continue;
        }
        let (mut lo, mut hi) = (Vec3::ZERO, Vec3::ONE);
        if pos {
            lo[axis] = 1.0;
            hi[axis] = 1.0 + P;
        } else {
            lo[axis] = -P;
            hi[axis] = 0.0;
        }
        // the face against the block itself is hidden
        let mut skip = 1u8 << (axis * 2 + usize::from(pos));
        // sides of the proud skin: only where the run of timber ends
        for (d2, a2, p2) in dirs {
            if a2 == axis {
                continue;
            }
            let n2 = nb(c, d2);
            if proud(n2) && g.fl[nb(n2, d)] & F_OPAQUE == 0 {
                skip |= 1 << (a2 * 2 + usize::from(!p2));
            }
        }
        // the grain runs along the timber: side texture on every face
        k.cuboid(lo, hi, [m; 3], Uv::Cell, skip);
    }
}

/// Free-standing timber: a chamfered post where it stands on something or
/// holds something up, rails or a beam towards the timber beside it.
fn stick(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let dy = DY as isize;
    let id = |d: isize| g.ids[nb(c, d)];
    let side = mat(b, 1);
    let end = mat(b, 0);
    let below = id(-dy);
    let above = id(dy);
    let down = w::is_solid(below) && !is_banner(below) && below != w::LANTERN;
    let up = w::is_solid(above) && !is_banner(above);
    let arms: Vec<usize> = (0..4)
        .filter(|k4| {
            let n = id(H4[*k4]);
            is_wood(n) || (wallish(n) && !down && !up)
        })
        .collect();
    let round = b != w::TIMBER_BEAM;
    let r = if round { 0.2 } else { 0.15 };
    let segs = if round { 10 } else { 8 };
    let phase = if round { 0.0 } else { PI / 8.0 };
    let is_post = up || down || arms.is_empty();
    if is_post {
        let y0 = if down { 0.0 } else { 0.08 };
        let y1 = if up { 1.0 } else { 0.9 };
        k.lathe(
            1,
            [0.5, 0.5],
            &[(y0, r), (y1, r)],
            segs,
            phase,
            !round,
            1.0,
            side,
            [None, (!up).then_some(end)],
        );
        if down && !is_wood(below) && !is_stone(below) {
            // a stone footing
            let foot = mat(w::STONE_BRICKS, 1).tint(0.9);
            k.lathe(
                1,
                [0.5, 0.5],
                &[(0.0, r + 0.1), (0.1, r + 0.09), (0.15, r + 0.02)],
                segs,
                phase,
                true,
                1.0,
                foot,
                [None, None],
            );
        }
        if !up {
            // a cap
            k.lathe(
                1,
                [0.5, 0.5],
                &[(y1, r + 0.03), (y1 + 0.05, r + 0.03), (y1 + 0.1, 0.0)],
                segs,
                phase,
                true,
                1.0,
                end.tint(0.85),
                [Some(end), None],
            );
        }
        if !down {
            k.lathe(
                1,
                [0.5, 0.5],
                &[(y0 - 0.06, 0.0), (y0, r)],
                segs,
                phase,
                true,
                1.0,
                end,
                [None, None],
            );
        }
        for k4 in arms {
            let d = H4[k4];
            let n = nb(c, d);
            // a fence when the neighbour is a post too; else one beam
            let n_post = w::is_solid(g.ids[n - DY]) && !is_banner(g.ids[n - DY]);
            let heights: &[(f32, f32)] = if n_post && down {
                &[(0.28, 0.1), (0.7, 0.1)]
            } else {
                &[(0.5, 0.24)]
            };
            for (hy, hh) in heights {
                let t = if *hh > 0.15 { 0.12 } else { 0.05 };
                let (mut lo, mut hi) = (
                    Vec3::new(0.5 - t, hy - hh * 0.5, 0.5 - t),
                    Vec3::new(0.5 + t, hy + hh * 0.5, 0.5 + t),
                );
                match k4 {
                    0 => hi.x = 1.0,
                    1 => lo.x = 0.0,
                    2 => hi.z = 1.0,
                    _ => lo.z = 0.0,
                }
                let skip = [SKIP_PX, SKIP_NX, SKIP_PZ, SKIP_NZ][k4];
                k.cuboid(lo, hi, [side; 3], Uv::Cell, skip);
            }
        }
    } else {
        // a beam: along its connections, through the cell
        let along_x = arms.iter().any(|a| *a < 2);
        let (k_lo, k_hi) = if along_x { (1, 0) } else { (3, 2) };
        let lo_end = if arms.contains(&k_lo) { 0.0 } else { 0.06 };
        let hi_end = if arms.contains(&k_hi) { 1.0 } else { 0.94 };
        let t = 0.13;
        let (lo, hi, skip) = if along_x {
            (
                Vec3::new(lo_end, 0.5 - t, 0.5 - t),
                Vec3::new(hi_end, 0.5 + t, 0.5 + t),
                (if lo_end == 0.0 { SKIP_NX } else { 0 }) | (if hi_end == 1.0 { SKIP_PX } else { 0 }),
            )
        } else {
            (
                Vec3::new(0.5 - t, 0.5 - t, lo_end),
                Vec3::new(0.5 + t, 0.5 + t, hi_end),
                (if lo_end == 0.0 { SKIP_NZ } else { 0 }) | (if hi_end == 1.0 { SKIP_PZ } else { 0 }),
            )
        };
        k.cuboid(lo, hi, [side, side, side], Uv::Cell, skip);
    }
}

/// A tree trunk: round, flaring into roots on the ground, thinning into
/// the canopy.
fn trunk(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let dy = DY as isize;
    let below = g.ids[nb(c, -dy)];
    let above = g.ids[nb(c, dy)];
    let birch = b == w::BIRCH_LOG;
    // thinner towards the top of the tree
    let logs_above = (1..=2)
        .take_while(|i| {
            let q = c as isize + dy * i;
            q < g.ids.len() as isize && g.ids[q as usize] == b
        })
        .count();
    let r_at = |n: usize| {
        let base = if birch { 0.2 } else { 0.27 };
        base * (0.78 + 0.11 * n as f32)
    };
    let r0 = r_at(logs_above + if b == below { 1 } else { 0 }).min(r_at(3));
    let r1 = r_at(logs_above);
    let mut prof = Vec::new();
    if b != below && w::is_solid(below) {
        // roots
        prof.extend([(0.0, r0 + 0.16), (0.12, r0 + 0.06), (0.35, r0)]);
    } else {
        prof.push((0.0, r0));
    }
    if w::is_leaves(above) || above == b {
        prof.push((1.0, r1));
    } else {
        prof.extend([(0.85, r1), (0.95, r1 * 0.7)]);
    }
    // the same turn all the way up the trunk, so its cells meet
    let h = hash3(k.base.x as i32, 7, k.base.z as i32);
    k.lathe(
        1,
        [0.5, 0.5],
        &prof,
        10,
        (h & 255) as f32 / 40.0,
        false,
        2.0,
        mat(b, 1),
        [None, (!w::is_leaves(above) && above != b).then_some(mat(b, 0))],
    );
}

/// A round stone pillar: a plinth where it stands, a capital at its head.
fn pillar(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let dy = DY as isize;
    let col = |q: usize| {
        is_stone(g.ids[q])
            && pillar_free(g, q)
            && g.fl[q] & F_DETAIL != 0
            && g.fl[q] & F_OPAQUE == 0
    };
    let foot = !col(nb(c, -dy));
    let head = !col(nb(c, dy));
    let m = mat(b, 1);
    let stone = mat(w::STONE_BRICKS, 1);
    let r = 0.36;
    let mut y0 = 0.0;
    let mut y1 = 1.0;
    if foot {
        k.bevel(
            Vec3::new(0.04, 0.0, 0.04),
            Vec3::new(0.96, 0.16, 0.96),
            0.04,
            [stone; 3],
            Uv::Cell,
            SKIP_NY,
        );
        k.lathe(
            1,
            [0.5, 0.5],
            &[(0.16, r + 0.08), (0.22, r + 0.08), (0.28, r + 0.02)],
            12,
            0.0,
            false,
            2.0,
            stone,
            [None, None],
        );
        y0 = 0.28;
    }
    if head {
        k.lathe(
            1,
            [0.5, 0.5],
            &[(0.74, r + 0.01), (0.8, r + 0.07), (0.86, r + 0.09)],
            12,
            0.0,
            false,
            2.0,
            stone,
            [None, None],
        );
        k.bevel(
            Vec3::new(0.03, 0.86, 0.03),
            Vec3::new(0.97, 1.0, 0.97),
            0.04,
            [stone; 3],
            Uv::Cell,
            0,
        );
        y1 = 0.74;
    }
    // a slight swell up the shaft; the stones a little smaller than on a wall
    let mid = (y0 + y1) * 0.5;
    k.vscale = 1.5;
    k.lathe(
        1,
        [0.5, 0.5],
        &[(y0, r + 0.01), (mid, r + 0.02), (y1, r)],
        12,
        0.0,
        false,
        3.5,
        m,
        [None, None],
    );
    k.vscale = 1.0;
}

/// An awning: cloth stretched over open air, sagging a little, with a
/// scalloped flap where it ends.
fn sheet(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let m = mat(b, 1);
    let under = m.tint(0.78);
    let open: Vec<bool> = H4.iter().map(|d| !is_cloth(g.ids[nb(c, *d)])).collect();
    // corner heights: lower where the cloth ends (it hangs from its posts)
    let h = |x: f32, z: f32| {
        let mut y = 0.9;
        if (open[1] && x < 0.5) || (open[0] && x > 0.5) {
            y -= 0.04;
        }
        if (open[3] && z < 0.5) || (open[2] && z > 0.5) {
            y -= 0.04;
        }
        y
    };
    let pts = |dy: f32| {
        [
            Vec3::new(0.0, h(0.0, 0.0) + dy, 0.0),
            Vec3::new(1.0, h(1.0, 0.0) + dy, 0.0),
            Vec3::new(1.0, h(1.0, 1.0) + dy, 1.0),
            Vec3::new(0.0, h(0.0, 1.0) + dy, 1.0),
        ]
    };
    k.poly_at(&pts(0.04), Vec3::Y, Uv::Cell, m);
    k.poly_at(&pts(0.0), Vec3::NEG_Y, Uv::Cell, under);
    // flaps on the open sides
    for (k4, is_open) in open.iter().enumerate() {
        if !is_open {
            continue;
        }
        k.quarter(TURN[k4]);
        let top = 0.86;
        let n = 4;
        for i in 0..n {
            let (x0, x1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
            let xm = (x0 + x1) * 0.5;
            let ps = [
                Vec3::new(x0, top, 1.0),
                Vec3::new(x1, top, 1.0),
                Vec3::new(x1, top - 0.2, 1.0),
                Vec3::new(xm, top - 0.27, 1.0),
                Vec3::new(x0, top - 0.2, 1.0),
            ];
            k.poly_at(&ps, Vec3::Z, Uv::Cell, m);
            let back: Vec<Vec3> = ps.iter().rev().map(|p| *p - Vec3::Z * 0.02).collect();
            k.poly_at(&back, Vec3::NEG_Z, Uv::Cell, under);
        }
        k.turn(0.0);
    }
}

// ----------------------------------------------------------------- props

fn prop(k: &mut Kit, c: usize, b: BlockId, wp: [i32; 3]) -> Option<Glow> {
    let g = k.g;
    let dy = DY as isize;
    let below = g.ids[nb(c, -dy)];
    let above = g.ids[nb(c, dy)];
    // a small turn per column, the same for a stack
    let h = hash3(wp[0], 11, wp[2]);
    let wiggle = ((h & 1023) as f32 / 1023.0 - 0.5) * 0.24;
    match b {
        w::BARREL => {
            k.turn(wiggle * 3.0);
            let top = if above == b { 1.0 } else { 0.94 };
            let r = 0.36;
            let prof = [
                (0.0, r - 0.05),
                (0.22, r + 0.02),
                (0.5, r + 0.04),
                (0.78, r + 0.02),
                (top, r - 0.05),
            ];
            k.lathe(
                1,
                [0.5, 0.5],
                &prof,
                12,
                0.0,
                false,
                3.0,
                mat(b, 1),
                [None, None],
            );
            if above != b {
                // the lid, sunk inside the rim
                k.lathe(
                    1,
                    [0.5, 0.5],
                    &[(top, r - 0.05), (top - 0.03, r - 0.08)],
                    12,
                    0.0,
                    false,
                    3.0,
                    mat(b, 1).tint(0.8),
                    [None, Some(mat(b, 0))],
                );
            }
            None
        }
        w::CRATE => {
            k.turn(wiggle);
            let top = if above == b { 1.0 } else { 0.86 };
            let m = [mat(b, 0), mat(b, 1), mat(b, 0)];
            k.bevel(
                Vec3::new(0.07, 0.0, 0.07),
                Vec3::new(0.93, top, 0.93),
                0.03,
                m,
                Uv::Fit,
                0,
            );
            None
        }
        w::HAY_BALE => {
            k.turn(wiggle * 0.5);
            let top = if above == b { 1.0 } else { 0.82 };
            k.bevel(
                Vec3::new(0.03, 0.0, 0.06),
                Vec3::new(0.97, top, 0.94),
                0.12,
                [mat(b, 0), mat(b, 1), mat(b, 0)],
                Uv::Fit,
                0,
            );
            // twine
            let twine = mat(w::HAY_BALE, 1).tinted([0.55, 0.42, 0.25]);
            for x in [0.3f32, 0.7] {
                k.cuboid(
                    Vec3::new(x - 0.025, -0.0, 0.05),
                    Vec3::new(x + 0.025, top + 0.012, 0.95),
                    [twine; 3],
                    Uv::Cell,
                    SKIP_NY,
                );
            }
            None
        }
        w::ANVIL => {
            k.turn(wiggle);
            // on a stump
            k.lathe(
                1,
                [0.5, 0.5],
                &[(0.0, 0.34), (0.4, 0.31)],
                12,
                0.0,
                false,
                2.0,
                mat(w::LOG, 1),
                [None, Some(mat(w::LOG, 0))],
            );
            let iron = [mat(b, 0), mat(b, 1), mat(b, 2)];
            k.bevel(
                Vec3::new(0.3, 0.4, 0.33),
                Vec3::new(0.7, 0.48, 0.67),
                0.02,
                iron,
                Uv::Fit,
                0,
            );
            k.bevel(
                Vec3::new(0.38, 0.48, 0.4),
                Vec3::new(0.62, 0.62, 0.6),
                0.02,
                iron,
                Uv::Fit,
                0,
            );
            k.bevel(
                Vec3::new(0.18, 0.62, 0.34),
                Vec3::new(0.78, 0.76, 0.66),
                0.025,
                iron,
                Uv::Fit,
                0,
            );
            k.lathe(
                0,
                [0.71, 0.5],
                &[(0.78, 0.07), (0.98, 0.0)],
                8,
                0.0,
                false,
                1.0,
                iron[1],
                [Some(iron[1]), None],
            );
            None
        }
        w::BRAZIER => {
            let iron = iron_mat();
            for i in 0..3 {
                let t = i as f32 * TAU / 3.0 + 0.4;
                let (x, z) = (0.5 + 0.27 * t.cos(), 0.5 + 0.27 * t.sin());
                k.lathe(
                    1,
                    [x, z],
                    &[(0.0, 0.035), (0.55, 0.03)],
                    6,
                    0.0,
                    true,
                    1.0,
                    iron,
                    [None, None],
                );
            }
            k.lathe(
                1,
                [0.5, 0.5],
                &[
                    (0.4, 0.06),
                    (0.48, 0.24),
                    (0.6, 0.36),
                    (0.68, 0.42),
                    (0.72, 0.42),
                ],
                14,
                0.0,
                false,
                3.0,
                iron,
                [Some(iron), None],
            );
            // the coals
            k.lathe(
                1,
                [0.5, 0.5],
                &[(0.66, 0.39), (0.7, 0.3), (0.74, 0.0)],
                14,
                0.0,
                false,
                1.0,
                mat(b, 0).glowing(),
                [None, None],
            );
            Some(Glow {
                pos: [k.base.x + 0.5, k.base.y + 0.95, k.base.z + 0.5],
                color: w::block(b).color,
                size: 0.9 + w::emission(b) as f32 * 0.06,
            })
        }
        w::FLOWER_BED => {
            let wood = mat(DARK_PLANKS, 1);
            k.cuboid(
                Vec3::new(0.04, 0.0, 0.04),
                Vec3::new(0.96, 0.42, 0.96),
                [wood; 3],
                Uv::Cell,
                SKIP_PY | SKIP_NY,
            );
            // a rim round the top of the box
            k.cuboid(
                Vec3::new(0.02, 0.36, 0.02),
                Vec3::new(0.98, 0.44, 0.98),
                [wood.tint(0.85); 3],
                Uv::Cell,
                SKIP_PY | SKIP_NY,
            );
            // a soft mound of flowers
            let hgt = 0.64 + (h & 7) as f32 * 0.01;
            k.bevel(
                Vec3::new(0.07, 0.3, 0.07),
                Vec3::new(0.93, hgt, 0.93),
                0.12,
                [mat(b, 0); 3],
                Uv::Fit,
                SKIP_NY,
            );
            None
        }
        w::LANTERN => Some(lantern(k, c)),
        w::RED_BANNER | w::BLUE_BANNER => {
            banner(k, c);
            None
        }
        w::LEADED_WINDOW | w::WOODEN_DOOR => {
            pane(k, c);
            None
        }
        _ => None,
    }
}

/// A small iron lantern: on whatever is below it, hanging from what is
/// above, or on a bracket from a wall.
fn lantern(k: &mut Kit, c: usize) -> Glow {
    let g = k.g;
    let dy = DY as isize;
    let below = g.ids[nb(c, -dy)];
    let above = g.ids[nb(c, dy)];
    let iron = iron_mat();
    let glass = mat(w::LANTERN, 1).glowing();
    let wall = H4.iter().position(|d| wallish(g.ids[nb(c, *d)]));
    let sits = w::is_solid(below) && !is_banner(below);
    let hangs = !sits && w::is_solid(above);
    let (w2, hgt) = (0.2, 0.46);
    let y0 = if sits {
        0.0
    } else if hangs {
        0.3
    } else if wall.is_some() {
        0.22
    } else {
        0.0
    };
    if let (false, false, Some(d)) = (sits, hangs, wall) {
        // a bracket from the wall
        k.quarter(TURN[d]);
        k.cuboid(
            Vec3::new(0.47, 0.86, 0.5),
            Vec3::new(0.53, 0.92, 1.0),
            [iron; 3],
            Uv::Cell,
            0,
        );
        k.cuboid(
            Vec3::new(0.47, 0.55, 0.92),
            Vec3::new(0.53, 0.92, 1.0),
            [iron; 3],
            Uv::Cell,
            0,
        );
        k.turn(0.0);
    }
    if hangs || (!sits && wall.is_some()) {
        let top = if hangs { 1.0 } else { 0.9 };
        k.cuboid(
            Vec3::new(0.485, y0 + hgt + 0.12, 0.485),
            Vec3::new(0.515, top, 0.515),
            [iron; 3],
            Uv::Cell,
            0,
        );
    }
    // base, glass, corner posts, roof
    k.cuboid(
        Vec3::new(0.5 - w2 - 0.03, y0, 0.5 - w2 - 0.03),
        Vec3::new(0.5 + w2 + 0.03, y0 + 0.06, 0.5 + w2 + 0.03),
        [iron; 3],
        Uv::Cell,
        if sits { SKIP_NY } else { 0 },
    );
    k.cuboid(
        Vec3::new(0.5 - w2 + 0.02, y0 + 0.06, 0.5 - w2 + 0.02),
        Vec3::new(0.5 + w2 - 0.02, y0 + hgt - 0.06, 0.5 + w2 - 0.02),
        [glass; 3],
        Uv::Fit,
        SKIP_NY | SKIP_PY,
    );
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let (x, z) = (0.5 + sx * (w2 - 0.01), 0.5 + sz * (w2 - 0.01));
        k.cuboid(
            Vec3::new(x - 0.025, y0 + 0.06, z - 0.025),
            Vec3::new(x + 0.025, y0 + hgt - 0.06, z + 0.025),
            [iron; 3],
            Uv::Cell,
            SKIP_NY | SKIP_PY,
        );
    }
    k.lathe(
        1,
        [0.5, 0.5],
        &[
            (y0 + hgt - 0.06, w2 * 1.45),
            (y0 + hgt - 0.02, w2 * 1.45),
            (y0 + hgt + 0.1, 0.06),
            (y0 + hgt + 0.14, 0.0),
        ],
        4,
        FRAC_PI_4,
        true,
        1.0,
        iron,
        [Some(iron), None],
    );
    let mid = y0 + hgt * 0.5;
    Glow {
        pos: [k.base.x + 0.5, k.base.y + mid, k.base.z + 0.5],
        color: w::block(w::LANTERN).color,
        size: 0.9 + w::emission(w::LANTERN) as f32 * 0.06,
    }
}

/// A banner: thin cloth hung flat on a wall, or from the beam above it.
fn banner(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let dy = DY as isize;
    let above = g.ids[nb(c, dy)];
    let below = g.ids[nb(c, -dy)];
    let wall = H4.iter().position(|d| wallish(g.ids[nb(c, *d)]));
    // the cloth hangs in the local x-y plane, at depth z
    let (z0, z1) = match wall {
        Some(d) => {
            k.quarter(TURN[d]);
            (0.9, 0.94)
        }
        None => {
            // free: across whatever beam holds it up
            let along_x = (0..2).any(|i| is_wood(g.ids[nb(c, dy + H4[i])]));
            k.quarter(if along_x { 0 } else { 1 });
            (0.48, 0.52)
        }
    };
    let top_end = !is_banner(above);
    let bottom_end = !is_banner(below);
    let y1 = if top_end { 0.9 } else { 1.0 };
    let (x0, x1) = (0.1, 0.9);
    let m = mat(b, 1);
    let fit = |p: Vec3| [(p.x - x0) / (x1 - x0), 1.0 - p.y];
    let mut outline = vec![Vec3::new(x0, y1, 0.0), Vec3::new(x1, y1, 0.0)];
    if bottom_end {
        // a swallowtail
        outline.extend([
            Vec3::new(x1, 0.02, 0.0),
            Vec3::new(0.5, 0.22, 0.0),
            Vec3::new(x0, 0.02, 0.0),
        ]);
    } else {
        outline.extend([Vec3::new(x1, 0.0, 0.0), Vec3::new(x0, 0.0, 0.0)]);
    }
    for (z, n) in [(z0, Vec3::NEG_Z), (z1, Vec3::Z)] {
        let ps: Vec<Vec3> = outline.iter().map(|p| Vec3::new(p.x, p.y, z)).collect();
        let uvs: Vec<[f32; 2]> = ps.iter().map(|p| fit(*p)).collect();
        if bottom_end {
            // concave: draw as two halves
            k.poly(
                &[ps[0], ps[3], ps[4]],
                n,
                &[uvs[0], uvs[3], uvs[4]],
                m,
            );
            k.poly(
                &[ps[0], ps[1], ps[2], ps[3]],
                n,
                &[uvs[0], uvs[1], uvs[2], uvs[3]],
                m,
            );
        } else {
            k.poly(&ps, n, &uvs, m);
        }
    }
    // edges
    for (x, n) in [(x0, Vec3::NEG_X), (x1, Vec3::X)] {
        let yb = if bottom_end { 0.02 } else { 0.0 };
        k.poly_at(
            &[
                Vec3::new(x, yb, z0),
                Vec3::new(x, yb, z1),
                Vec3::new(x, y1, z1),
                Vec3::new(x, y1, z0),
            ],
            n,
            Uv::Cell,
            m.tint(0.8),
        );
    }
    if top_end {
        let rod = mat(DARK_PLANKS, 1);
        let zc = (z0 + z1) * 0.5;
        k.lathe(
            0,
            [0.93, zc],
            &[(0.03, 0.03), (0.97, 0.03)],
            6,
            0.0,
            false,
            1.0,
            rod,
            [Some(rod), Some(rod)],
        );
        let gold = mat(w::LANTERN, 0).tinted([1.0, 0.85, 0.45]);
        for x in [0.03f32, 0.97] {
            k.lathe(
                1,
                [x, zc],
                &[(0.89, 0.0), (0.91, 0.05), (0.97, 0.05), (0.99, 0.0)],
                6,
                0.0,
                false,
                1.0,
                gold,
                [None, None],
            );
        }
    }
}

/// A window or a door, in the plane of its wall: a frame with depth,
/// mullions and a sill round a pane, or a plank door with its ironwork.
fn pane(k: &mut Kit, c: usize) {
    let g = k.g;
    let b = g.ids[c];
    let dy = DY as isize;
    let id = |d: isize| g.ids[nb(c, d)];
    let solidish = |q: BlockId| wallish(q) || is_wood(q);
    // the wall runs along local x
    let along_x = (solidish(id(H4[0])) as i32 + solidish(id(H4[1])) as i32)
        >= (solidish(id(H4[2])) as i32 + solidish(id(H4[3])) as i32);
    k.quarter(if along_x { 0 } else { 1 });
    let (left, right) = if along_x {
        (id(H4[1]), id(H4[0]))
    } else {
        // a quarter turn takes local -x to world +z
        (id(H4[2]), id(H4[3]))
    };
    let above = id(dy);
    let below = id(-dy);
    let frame_of = |q: BlockId| {
        if wallish(q) && q != w::LEADED_WINDOW && q != w::WOODEN_DOOR && q != w::GLASS {
            mat(q, 1)
        } else {
            wood_mat()
        }
    };
    if b == w::WOODEN_DOOR {
        let door = mat(b, 1);
        let edge = mat(b, 0);
        k.cuboid(
            Vec3::new(0.0, 0.0, 0.44),
            Vec3::new(1.0, 1.0, 0.56),
            [edge, door, edge],
            Uv::Fit,
            (if below == b { SKIP_NY } else { 0 }) | (if above == b { SKIP_PY } else { 0 }),
        );
        // iron straps standing off the planks on both faces
        let iron = iron_mat();
        for hy in [0.2f32, 0.78] {
            for (za, zb) in [(0.42, 0.44), (0.56, 0.58)] {
                k.cuboid(
                    Vec3::new(0.06, hy - 0.04, za),
                    Vec3::new(0.82, hy + 0.04, zb),
                    [iron; 3],
                    Uv::Cell,
                    0,
                );
            }
        }
        if above == b {
            // the ring pull, on the lower leaf
        } else if below == b {
            for (za, zb) in [(0.4, 0.44), (0.56, 0.6)] {
                k.cuboid(
                    Vec3::new(0.78, 0.0, za),
                    Vec3::new(0.86, 0.08, zb),
                    [iron; 3],
                    Uv::Cell,
                    0,
                );
            }
        }
        return;
    }
    // window frame: jambs, head and sill where the window ends
    let t = 0.1;
    if left != b {
        let m = frame_of(left);
        k.cuboid(Vec3::new(0.0, 0.0, 0.0), Vec3::new(t, 1.0, 1.0), [m; 3], Uv::Cell, SKIP_NX);
    }
    if right != b {
        let m = frame_of(right);
        k.cuboid(Vec3::new(1.0 - t, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0), [m; 3], Uv::Cell, SKIP_PX);
    }
    if above != b {
        let m = frame_of(above);
        k.cuboid(Vec3::new(0.0, 1.0 - t, 0.0), Vec3::new(1.0, 1.0, 1.0), [m; 3], Uv::Cell, SKIP_PY);
    }
    if below != b {
        let m = frame_of(below);
        k.cuboid(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, t, 1.0), [m; 3], Uv::Cell, SKIP_NY);
        // a sill on both faces
        let sill = wood_mat().tint(0.9);
        for (za, zb, skip) in [(-0.1f32, 0.0f32, SKIP_PZ), (1.0, 1.1, SKIP_NZ)] {
            k.cuboid(
                Vec3::new(-0.05, 0.02, za),
                Vec3::new(1.05, 0.1, zb),
                [sill; 3],
                Uv::Cell,
                skip,
            );
        }
    }
    // mullion and transom
    let wood = wood_mat().tint(0.8);
    k.cuboid(Vec3::new(0.47, 0.0, 0.42), Vec3::new(0.53, 1.0, 0.58), [wood; 3], Uv::Cell, SKIP_NY | SKIP_PY);
    k.cuboid(Vec3::new(0.0, 0.47, 0.42), Vec3::new(1.0, 0.53, 0.58), [wood; 3], Uv::Cell, SKIP_NX | SKIP_PX);
    // the glass, both faces
    k.clear = true;
    let glass = mat(b, 1);
    for (z, n) in [(0.49f32, Vec3::NEG_Z), (0.51, Vec3::Z)] {
        let ps = [
            Vec3::new(0.0, 0.0, z),
            Vec3::new(1.0, 0.0, z),
            Vec3::new(1.0, 1.0, z),
            Vec3::new(0.0, 1.0, z),
        ];
        k.poly_at(&ps, n, Uv::Cell, glass);
    }
    k.clear = false;
}
