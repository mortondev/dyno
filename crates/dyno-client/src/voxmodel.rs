//! Fine voxel models: dense grids with a palette (composed from pack data), meshed once
//! (hidden faces culled, flat areas merged, ambient occlusion baked per vertex)
//! and then stamped into the frame's vertex list with a transform.

use crate::render::{NO_TEX, Vertex};
use glam::{Affine3A, Vec3, Vec3A};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[path = "smooth.rs"]
mod smooth;
pub use smooth::Smooth;
use smooth::face_normals;

/// A palette index. 0 is empty space.
pub type Ink = u8;

/// A dense voxel grid with a palette. Every voxel also belongs to a part, so
/// one grid can hold a whole articulated figure in its rest pose.
pub struct Vox {
    min: [i32; 3],
    size: [i32; 3],
    cells: Vec<Ink>,
    parts: Vec<u8>,
    /// sRGB colour and whether it glows
    palette: Vec<([f32; 3], bool)>,
    /// inks whose detail stays crisp when meshed smooth (eyes, trim)
    crisp: Vec<bool>,
    lookup: HashMap<[u8; 4], Ink>,
    /// the part that newly set voxels join
    pub part: u8,
    /// edge of a voxel in the units meshes are measured in (1 at full detail)
    unit: f32,
    /// the solid inside of the model at half detail, made when first meshed
    core: OnceCell<Box<Vox>>,
}

/// Brightness for 0 to 3 open corners around a vertex.
const AO: [f32; 4] = [0.50, 0.68, 0.85, 1.0];

impl Vox {
    /// A grid covering `min..max` (max exclusive) on each axis.
    pub fn new(min: [i32; 3], max: [i32; 3]) -> Vox {
        let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        let n = (size[0] * size[1] * size[2]) as usize;
        Vox {
            min,
            size,
            cells: vec![0; n],
            parts: vec![0; n],
            palette: vec![([0.0; 3], false)],
            crisp: vec![false],
            lookup: HashMap::new(),
            part: 0,
            unit: 1.0,
            core: OnceCell::new(),
        }
    }

    /// The inside of the model at half the detail: a voxel wherever a whole
    /// 2 x 2 x 2 block is solid and of one part. It lies wholly within the
    /// model, so it can stand in for faces that are only needed for shadows.
    fn inside(&self) -> Vox {
        let lo = self.min.map(|m| m.div_euclid(2));
        let (_, max) = self.bounds();
        let hi = max.map(|m| (m + 1).div_euclid(2));
        let mut out = Vox::new(lo, hi);
        out.palette = self.palette.clone();
        out.crisp = self.crisp.clone();
        out.unit = self.unit * 2.0;
        for y in lo[1]..hi[1] {
            for z in lo[2]..hi[2] {
                for x in lo[0]..hi[0] {
                    let first = self.idx(x * 2, y * 2, z * 2);
                    let Some((ink, part)) = first.map(|i| (self.cells[i], self.parts[i])) else {
                        continue;
                    };
                    let whole = ink != 0
                        && (1..8).all(|i| {
                            let p = (x * 2 + (i & 1), y * 2 + ((i >> 1) & 1), z * 2 + (i >> 2));
                            self.get_in(part, p.0, p.1, p.2) != 0
                        });
                    if whole && let Some(at) = out.idx(x, y, z) {
                        out.cells[at] = ink;
                        out.parts[at] = part;
                    }
                }
            }
        }
        out
    }

    /// A copy of the model (without its cached inside).
    pub fn copy(&self) -> Vox {
        Vox {
            min: self.min,
            size: self.size,
            cells: self.cells.clone(),
            parts: self.parts.clone(),
            palette: self.palette.clone(),
            crisp: self.crisp.clone(),
            lookup: self.lookup.clone(),
            part: self.part,
            unit: self.unit,
            core: OnceCell::new(),
        }
    }

    /// The same model at half the detail, for drawing far away: each 2 x 2 x 2
    /// block becomes one voxel, of the commonest ink on its surface. Meshes
    /// of the result come out the same size as the original's.
    pub fn half(&self) -> Vox {
        let lo = self.min.map(|m| m.div_euclid(2));
        let (_, max) = self.bounds();
        let hi = max.map(|m| (m + 1).div_euclid(2));
        let mut out = Vox::new(lo, hi);
        out.palette = self.palette.clone();
        out.crisp = self.crisp.clone();
        out.lookup = self.lookup.clone();
        out.unit = self.unit * 2.0;
        for y in lo[1]..hi[1] {
            for z in lo[2]..hi[2] {
                for x in lo[0]..hi[0] {
                    // (ink, part, weight) of what the block holds; voxels that
                    // show count for much more than buried ones
                    let mut seen = [(0u8, 0u8, 0u8); 8];
                    let mut kinds = 0;
                    let mut total = 0;
                    for i in 0..8 {
                        let p = (x * 2 + (i & 1), y * 2 + ((i >> 1) & 1), z * 2 + (i >> 2));
                        let Some(at) = self.idx(p.0, p.1, p.2) else {
                            continue;
                        };
                        let c = self.cells[at];
                        if c == 0 {
                            continue;
                        }
                        total += 1;
                        let weight = if self.exposed(p.0, p.1, p.2) { 9 } else { 1 };
                        match seen[..kinds].iter_mut().find(|s| s.0 == c) {
                            Some(s) => s.2 += weight,
                            None => {
                                seen[kinds] = (c, self.parts[at], weight);
                                kinds += 1;
                            }
                        }
                    }
                    if total < 2 {
                        continue;
                    }
                    let lum = |ink: u8| {
                        let c = self.palette[ink as usize].0;
                        ((c[0] + c[1] + c[2]) / 3.0 * 100.0) as i32
                    };
                    // on a tie a strong dark detail wins (eyes, mouths);
                    // between near tones the lighter does (stripes, folds)
                    let top = seen[..kinds].iter().map(|s| s.2).max().unwrap_or(0);
                    let tied = || seen[..kinds].iter().filter(|s| s.2 == top);
                    let lo = tied().map(|s| lum(s.0)).min().unwrap_or(0);
                    let hi = tied().map(|s| lum(s.0)).max().unwrap_or(0);
                    let best = if hi - lo > 30 {
                        tied().min_by_key(|s| lum(s.0)).copied()
                    } else {
                        tied().max_by_key(|s| lum(s.0)).copied()
                    };
                    if let Some((c, part, _)) = best
                        && let Some(at) = out.idx(x, y, z)
                    {
                        out.cells[at] = c;
                        out.parts[at] = part;
                    }
                }
            }
        }
        out
    }

    fn intern(&mut self, rgb: [f32; 3], glow: bool) -> Ink {
        self.intern_as(rgb, glow, false)
    }

    /// The ink for a colour that glows or not, and whose detail stays crisp
    /// or not when meshed smooth.
    pub fn ink_as(&mut self, rgb: [f32; 3], glow: bool, crisp: bool) -> Ink {
        self.intern_as(rgb, glow, crisp)
    }

    fn intern_as(&mut self, rgb: [f32; 3], glow: bool, crisp: bool) -> Ink {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        let key = [
            q(rgb[0]),
            q(rgb[1]),
            q(rgb[2]),
            glow as u8 | (crisp as u8) << 1,
        ];
        if let Some(i) = self.lookup.get(&key) {
            return *i;
        }
        if self.palette.len() >= 255 {
            // out of inks: reuse the nearest
            let mut best = (1u8, f32::MAX);
            for (i, (c, g)) in self.palette.iter().enumerate().skip(1) {
                let d = (c[0] - rgb[0]).powi(2) + (c[1] - rgb[1]).powi(2) + (c[2] - rgb[2]).powi(2);
                if *g == glow && d < best.1 {
                    best = (i as u8, d);
                }
            }
            return best.0;
        }
        let i = self.palette.len() as Ink;
        self.palette.push((
            [
                rgb[0].clamp(0.0, 1.0),
                rgb[1].clamp(0.0, 1.0),
                rgb[2].clamp(0.0, 1.0),
            ],
            glow,
        ));
        self.crisp.push(crisp);
        self.lookup.insert(key, i);
        i
    }

    /// Set the edge of a voxel in the units meshes are measured in (a
    /// figure modelled at twice the density of its rig has 0.5).
    pub fn set_unit(&mut self, unit: f32) {
        self.unit = unit;
    }

    /// The ink for a colour.
    pub fn ink(&mut self, rgb: [f32; 3]) -> Ink {
        self.intern(rgb, false)
    }

    /// An ink that glows: drawn at full brightness whatever the light.
    pub fn glow(&mut self, rgb: [f32; 3]) -> Ink {
        self.intern(rgb, true)
    }

    fn idx(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let (a, b, c) = (x - self.min[0], y - self.min[1], z - self.min[2]);
        if a < 0 || b < 0 || c < 0 || a >= self.size[0] || b >= self.size[1] || c >= self.size[2] {
            return None;
        }
        Some(((b * self.size[2] + c) * self.size[0] + a) as usize)
    }

    /// The ink at a voxel (0 when empty or outside).
    pub fn get(&self, x: i32, y: i32, z: i32) -> Ink {
        self.idx(x, y, z).map(|i| self.cells[i]).unwrap_or(0)
    }

    /// The ink at a voxel if it belongs to `part`.
    fn get_in(&self, part: u8, x: i32, y: i32, z: i32) -> Ink {
        match self.idx(x, y, z) {
            Some(i) if self.parts[i] == part => self.cells[i],
            _ => 0,
        }
    }

    /// The ink and part at a voxel, if it is solid.
    pub fn voxel(&self, x: i32, y: i32, z: i32) -> Option<(Ink, u8)> {
        let i = self.idx(x, y, z)?;
        (self.cells[i] != 0).then(|| (self.cells[i], self.parts[i]))
    }

    /// The colour of an ink and whether it glows.
    pub fn color(&self, ink: Ink) -> ([f32; 3], bool) {
        self.palette[ink as usize]
    }

    pub fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.get(x, y, z) != 0
    }

    /// Set a voxel; it joins the current part. Ink 0 erases.
    pub fn set(&mut self, x: i32, y: i32, z: i32, c: Ink) {
        self.core.take();
        if let Some(i) = self.idx(x, y, z) {
            self.cells[i] = c;
            self.parts[i] = self.part;
        }
    }

    /// Set a voxel only where there is none yet.
    pub fn add(&mut self, x: i32, y: i32, z: i32, c: Ink) {
        self.core.take();
        if let Some(i) = self.idx(x, y, z)
            && self.cells[i] == 0
        {
            self.cells[i] = c;
            self.parts[i] = self.part;
        }
    }

    /// Recolour a voxel that already exists, keeping its part.
    pub fn dye(&mut self, x: i32, y: i32, z: i32, c: Ink) {
        if let Some(i) = self.idx(x, y, z)
            && self.cells[i] != 0
        {
            self.cells[i] = c;
        }
    }

    /// True if the voxel is solid and has an open side.
    pub fn exposed(&self, x: i32, y: i32, z: i32) -> bool {
        self.solid(x, y, z)
            && [
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ]
            .iter()
            .any(|(a, b, c)| !self.solid(x + a, y + b, z + c))
    }

    /// The grid bounds: (min, max exclusive).
    pub fn bounds(&self) -> ([i32; 3], [i32; 3]) {
        (
            self.min,
            [
                self.min[0] + self.size[0],
                self.min[1] + self.size[1],
                self.min[2] + self.size[2],
            ],
        )
    }

    /// Mesh one part. Positions come out in full-detail voxel units relative
    /// to `pivot`. Faces between voxels of the same part are dropped; other
    /// parts do not hide or shade it, because they move.
    pub fn mesh(&self, part: u8, pivot: [f32; 3]) -> Mesh {
        let mut mesh = self.surface(part, pivot);
        let core = self
            .core
            .get_or_init(|| Box::new(self.inside()))
            .surface(part, pivot);
        mesh.core = Some(Box::new(core));
        mesh
    }

    fn surface(&self, part: u8, pivot: [f32; 3]) -> Mesh {
        let mut verts = Vec::new();
        let mut groups = [Group::default(); 6];
        let unit = self.unit;
        // linear colours, once
        let lin: Vec<[f32; 3]> = self
            .palette
            .iter()
            .map(|(c, _)| [c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2)])
            .collect();
        let mut mask: Vec<u16> = Vec::new();
        for d in 0..3usize {
            let (ua, va) = ((d + 1) % 3, (d + 2) % 3);
            let (nd, nu, nv) = (self.size[d], self.size[ua], self.size[va]);
            mask.clear();
            mask.resize((nu * nv) as usize, 0);
            for dir in [1i32, -1] {
                let face = d * 2 + usize::from(dir < 0);
                groups[face].start = verts.len() as u32;
                groups[face].lo = f32::MAX;
                groups[face].hi = f32::MIN;
                for s in 0..nd {
                    let at = |u: i32, v: i32, dn: i32| -> Ink {
                        let mut p = [0i32; 3];
                        p[d] = s + dn + self.min[d];
                        p[ua] = u + self.min[ua];
                        p[va] = v + self.min[va];
                        self.get_in(part, p[0], p[1], p[2])
                    };
                    let mut any = false;
                    for v in 0..nv {
                        for u in 0..nu {
                            let c = at(u, v, 0);
                            let key = if c == 0 || at(u, v, dir) != 0 {
                                0
                            } else if self.palette[c as usize].1 {
                                c as u16 | 0xFF00
                            } else {
                                let open = |du: i32, dv: i32| -> u16 {
                                    let s1 = at(u + du, v, dir) != 0;
                                    let s2 = at(u, v + dv, dir) != 0;
                                    let cn = at(u + du, v + dv, dir) != 0;
                                    if s1 && s2 {
                                        0
                                    } else {
                                        3 - (s1 as u16 + s2 as u16 + cn as u16)
                                    }
                                };
                                c as u16
                                    | open(-1, -1) << 8
                                    | open(1, -1) << 10
                                    | open(1, 1) << 12
                                    | open(-1, 1) << 14
                            };
                            any |= key != 0;
                            mask[(v * nu + u) as usize] = key;
                        }
                    }
                    if !any {
                        continue;
                    }
                    for v in 0..nv {
                        let mut u = 0;
                        while u < nu {
                            let key = mask[(v * nu + u) as usize];
                            if key == 0 {
                                u += 1;
                                continue;
                            }
                            let a = [
                                (key >> 8) & 3,
                                (key >> 10) & 3,
                                (key >> 12) & 3,
                                (key >> 14) & 3,
                            ];
                            // a run may grow along an axis only if the shade
                            // does not change along it
                            let flat_u = a[0] == a[1] && a[3] == a[2];
                            let flat_v = a[0] == a[3] && a[1] == a[2];
                            let mut w = 1;
                            while flat_u && u + w < nu && mask[(v * nu + u + w) as usize] == key {
                                w += 1;
                            }
                            let mut h = 1;
                            'grow: while flat_v && v + h < nv {
                                for k in 0..w {
                                    if mask[((v + h) * nu + u + k) as usize] != key {
                                        break 'grow;
                                    }
                                }
                                h += 1;
                            }
                            for dv in 0..h {
                                for du in 0..w {
                                    mask[((v + dv) * nu + u + du) as usize] = 0;
                                }
                            }
                            let ink = (key & 0xFF) as usize;
                            let glow = self.palette[ink].1;
                            let rgb = lin[ink];
                            let plane =
                                (s + i32::from(dir > 0) + self.min[d]) as f32 * unit - pivot[d];
                            groups[face].lo = groups[face].lo.min(plane);
                            groups[face].hi = groups[face].hi.max(plane);
                            let corner = |cu: i32, cv: i32, ao: u16| -> MeshVert {
                                let mut p = [0f32; 3];
                                p[d] = plane;
                                p[ua] = (cu + self.min[ua]) as f32 * unit - pivot[ua];
                                p[va] = (cv + self.min[va]) as f32 * unit - pivot[va];
                                MeshVert {
                                    pos: p,
                                    color: [
                                        rgb[0],
                                        rgb[1],
                                        rgb[2],
                                        if glow { 2.0 } else { AO[ao as usize] },
                                    ],
                                }
                            };
                            let q = [
                                corner(u, v, a[0]),
                                corner(u + w, v, a[1]),
                                corner(u + w, v + h, a[2]),
                                corner(u, v + h, a[3]),
                            ];
                            // split along the diagonal that keeps the shading smooth
                            let order = if a[0] + a[2] >= a[1] + a[3] {
                                [0, 1, 2, 0, 2, 3]
                            } else {
                                [1, 2, 3, 1, 3, 0]
                            };
                            verts.extend(order.iter().map(|i| q[*i]));
                            u += w;
                        }
                    }
                }
                groups[face].end = verts.len() as u32;
            }
        }
        Mesh {
            verts,
            groups,
            ..Mesh::default()
        }
    }
}

#[derive(Clone, Copy)]
pub struct MeshVert {
    /// voxel units from the part's pivot
    pub pos: [f32; 3],
    /// linear rgb and brightness (ambient occlusion; 2.0 glows)
    pub color: [f32; 4],
}

/// The faces of a mesh that look one way: a run of vertices, and the lowest
/// and highest plane they lie on along their axis.
#[derive(Clone, Copy, Default)]
struct Group {
    start: u32,
    end: u32,
    lo: f32,
    hi: f32,
}

/// A meshed part: a triangle list, six vertices per quad, grouped by the
/// direction the faces look (+x, -x, +y, -y, +z, -z).
#[derive(Default)]
pub struct Mesh {
    pub verts: Vec<MeshVert>,
    groups: [Group; 6],
    /// a coarse mesh of the part's inside. Where the camera cannot see a
    /// side but the sun can, this is drawn in its place: it costs far fewer
    /// vertices and throws the same shadow.
    core: Option<Box<Mesh>>,
    /// a smooth mesh: a normal per vertex (empty for a blocky mesh, whose
    /// faces take theirs from their group)
    normals: Vec<[f32; 3]>,
    /// a smooth mesh: each triangle's face normal, for culling
    faces: Vec<[f32; 3]>,
}

impl Mesh {
    /// True if the mesh is smooth (normals per vertex).
    pub fn smooth(&self) -> bool {
        !self.normals.is_empty()
    }

    /// Move every vertex (and the shadow core's) through `f`, in voxel units
    /// from the part's pivot, and refresh the bounds used to cull faces.
    /// Gentle reshaping only: faces keep the direction they were made with.
    pub fn reshape(&mut self, f: &dyn Fn([f32; 3]) -> [f32; 3]) {
        if self.smooth() {
            // normals follow the local stretch, measured by moving a little
            // along each axis
            for (v, n) in self.verts.iter_mut().zip(&mut self.normals) {
                let p = v.pos;
                let q = f(p);
                let e = 0.25;
                let d = |a: usize| {
                    let mut r = p;
                    r[a] += e;
                    let r = f(r);
                    Vec3::new(r[0] - q[0], r[1] - q[1], r[2] - q[2]) / e
                };
                let j = glam::Mat3::from_cols(d(0), d(1), d(2));
                let m = if j.determinant().abs() > 1e-6 {
                    j.inverse().transpose()
                } else {
                    glam::Mat3::IDENTITY
                };
                *n = (m * Vec3::from(*n)).normalize_or_zero().into();
                v.pos = q;
            }
            self.faces = face_normals(&self.verts);
        } else {
            for v in &mut self.verts {
                v.pos = f(v.pos);
            }
        }
        for (i, g) in self.groups.iter_mut().enumerate() {
            let axis = i / 2;
            let run = &self.verts[g.start as usize..g.end as usize];
            if run.is_empty() {
                continue;
            }
            g.lo = run.iter().map(|v| v.pos[axis]).fold(f32::MAX, f32::min);
            g.hi = run.iter().map(|v| v.pos[axis]).fold(f32::MIN, f32::max);
        }
        if let Some(core) = &mut self.core {
            core.reshape(f);
        }
    }

    /// Lowest and highest vertex along y.
    pub fn y_range(&self) -> (f32, f32) {
        self.verts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| {
            (lo.min(v.pos[1]), hi.max(v.pos[1]))
        })
    }

    pub fn quads(&self) -> usize {
        self.verts.len() / 6
    }

    pub fn is_empty(&self) -> bool {
        self.verts.is_empty()
    }
}

/// How a mesh is lit when drawn.
#[derive(Clone, Copy)]
pub struct Paint {
    /// block light 0..1
    pub light: f32,
    /// 0..1: whiten (the hit flash)
    pub flash: f32,
    /// draw at full brightness
    pub glow: bool,
    /// true: real normals, lit per pixel by the renderer, which also adds
    /// its world-anchored grain (it crawls over anything that moves).
    /// false: each face gets a flat shade by its angle to the sun, and the
    /// colours stay clean.
    pub sun: bool,
}

impl Paint {
    pub fn lit(light: f32) -> Paint {
        Paint {
            light,
            flash: 0.0,
            glow: false,
            sun: false,
        }
    }
}

thread_local! {
    /// the camera and the direction towards the sun or moon, if known
    static VIEW: Cell<Option<(Vec3A, Vec3A)>> = const { Cell::new(None) };
}

/// Tell the models where the camera is and where the light comes from
/// (`sun_dir` as in the sky: towards the sun; the moon is opposite). With
/// this, sides the camera cannot see are skipped (or, where the light sees
/// them, drawn as a coarse stand-in that casts their shadow), and far models
/// are drawn coarser. Call it once a frame before drawing; pass
/// `None` to draw everything in full.
pub fn set_view(view: Option<(Vec3, Vec3)>) {
    VIEW.with(|v| {
        v.set(view.map(|(eye, sun)| {
            // as the shadow pass sees it
            let sun = if sun.y < 0.0 { -sun } else { sun }.normalize_or_zero();
            let sun = Vec3::new(sun.x, sun.y.max(0.25), sun.z);
            (eye.into(), sun.into())
        }))
    });
}

/// Distance from the camera to a point, if the camera is known.
pub fn view_distance(p: Vec3) -> Option<f32> {
    VIEW.with(|v| v.get().map(|(eye, _)| eye.distance(p.into())))
}

/// Stamp a mesh into the vertex list. `xf` maps voxel units (from the part's
/// pivot) to the world, so it carries the rotation, place and voxel size.
pub fn emit(out: &mut Vec<Vertex>, mesh: &Mesh, xf: &Affine3A, paint: Paint) {
    if mesh.verts.is_empty() {
        return;
    }
    let m = xf.matrix3;
    let t = xf.translation;
    // the camera and the light, seen from the mesh
    let world_view = VIEW.with(|v| v.get());
    let view = world_view.map(|(eye, sun)| {
        let inv = m.inverse();
        (inv * (eye - t), inv * sun)
    });
    // without a known sun, light comes from above and a little to the front
    let sun_dir = world_view
        .map(|(_, sun)| sun)
        .unwrap_or(Vec3A::new(0.35, 0.85, 0.4))
        .normalize_or_zero();
    let k = paint.flash.clamp(0.0, 1.0);
    let plain = k <= 0.0 && !paint.glow;
    let glow = paint.glow || k > 0.5;
    out.reserve(mesh.verts.len());
    if mesh.smooth() {
        // normals go through the inverse transpose; a triangle is skipped
        // when it faces away from both the camera and the light
        let nm = m.inverse().transpose();
        let light = paint.light;
        for (tri, (vs, f)) in mesh
            .verts
            .as_chunks::<3>()
            .0
            .iter()
            .zip(&mesh.faces)
            .enumerate()
        {
            if let Some((eye, sun)) = view {
                let f = Vec3A::from(*f);
                if (eye - Vec3A::from(vs[0].pos)).dot(f) < 0.0 && sun.dot(f) <= 0.0 {
                    continue;
                }
            }
            for (v, n) in vs.iter().zip(&mesh.normals[tri * 3..tri * 3 + 3]) {
                let p = m * Vec3A::from(v.pos) + t;
                let n = (nm * Vec3A::from(*n)).normalize_or_zero();
                let (normal, shade) = if paint.sun {
                    (n.into(), 1.0)
                } else {
                    let s = 0.54 + 0.42 * n.dot(sun_dir).max(0.0) + 0.12 * (n.y * 0.5 + 0.5);
                    ([0.0; 3], s.min(1.0))
                };
                let c = v.color;
                let a = if glow || c[3] > 1.9 {
                    2.0
                } else {
                    c[3] * shade
                };
                out.push(Vertex {
                    pos: p.into(),
                    normal,
                    color: [
                        c[0] + (1.0 - c[0]) * k,
                        c[1] + (1.0 - c[1]) * k,
                        c[2] + (1.0 - c[2]) * k,
                        a,
                    ],
                    light,
                    sky: 1.0,
                    tex: NO_TEX,
                });
            }
        }
        return;
    }
    for (i, g) in mesh.groups.iter().enumerate() {
        let (axis, sign) = (i / 2, if i % 2 == 0 { 1.0 } else { -1.0 });
        let mut verts = &mesh.verts[g.start as usize..g.end as usize];
        if let Some((eye, sun)) = view {
            let seen = if sign > 0.0 {
                eye[axis] > g.lo
            } else {
                eye[axis] < g.hi
            };
            if !seen {
                // out of sight: only its shadow matters, if the light sees it
                verts = match &mesh.core {
                    Some(core) if sun[axis] * sign > 0.0 => {
                        let g = core.groups[i];
                        &core.verts[g.start as usize..g.end as usize]
                    }
                    _ => &[],
                };
            }
        }
        if verts.is_empty() {
            continue;
        }
        let n = m.col(axis).normalize_or_zero() * sign;
        let (normal, shade) = if paint.sun {
            (n.into(), 1.0)
        } else {
            // shade each face by how it meets the sun, with a little more
            // light from the sky above than from the ground below
            let s = 0.54 + 0.42 * n.dot(sun_dir).max(0.0) + 0.12 * (n.y * 0.5 + 0.5);
            ([0.0; 3], s.min(1.0))
        };
        let light = paint.light;
        if plain {
            out.extend(verts.iter().map(|v| {
                let p = m * Vec3A::from(v.pos) + t;
                let c = v.color;
                let a = if c[3] > 1.9 { 2.0 } else { c[3] * shade };
                Vertex {
                    pos: p.into(),
                    normal,
                    color: [c[0], c[1], c[2], a],
                    light,
                    sky: 1.0,
                    tex: NO_TEX,
                }
            }));
        } else {
            out.extend(verts.iter().map(|v| {
                let p = m * Vec3A::from(v.pos) + t;
                let c = v.color;
                let a = if glow || c[3] > 1.9 {
                    2.0
                } else {
                    c[3] * shade
                };
                Vertex {
                    pos: p.into(),
                    normal,
                    color: [
                        c[0] + (1.0 - c[0]) * k,
                        c[1] + (1.0 - c[1]) * k,
                        c[2] + (1.0 - c[2]) * k,
                        a,
                    ],
                    light,
                    sky: 1.0,
                    tex: NO_TEX,
                }
            }));
        }
    }
}

/// Built models, kept by key so each is built once.
pub struct Cache<T> {
    map: RefCell<HashMap<u64, Rc<T>>>,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Cache {
            map: RefCell::new(HashMap::new()),
        }
    }
}

impl<T> Cache<T> {
    /// Forget everything built.
    pub fn clear(&self) {
        self.map.borrow_mut().clear();
    }

    /// The model for `key`, built on first use.
    pub fn get(&self, key: u64, build: impl FnOnce() -> T) -> Rc<T> {
        if let Some(m) = self.map.borrow().get(&key) {
            return m.clone();
        }
        let m = Rc::new(build());
        let mut map = self.map.borrow_mut();
        // looks are few; a runaway caller must not grow this for ever
        if map.len() >= 512 {
            map.clear();
        }
        map.insert(key, m.clone());
        m
    }
}

/// Mix values into a cache key.
pub fn key_of(parts: &[u32]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for p in parts {
        h ^= *p as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
        h ^= h >> 29;
    }
    h
}
