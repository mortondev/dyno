//! Shaped meshing for voxel models: surface nets over the whole model,
//! from crisp cubes (`bevel` 0: every corner on the voxel lattice, as the
//! blocky mesher) through a gentle bevel to smooth (`bevel` 1, relaxed),
//! with soft normals, painted colour, ambient occlusion and simplification.
//!
//! The voxels stay the source: every face of the blocky mesh becomes one
//! quad here, but its corners sit on the dual grid (one vertex per cell of
//! eight voxels that the surface passes through) and are relaxed towards
//! their neighbours, never further than half a voxel from where the voxels
//! put them. Crisp inks (eyes, trim) pin their corners and keep their
//! colour to themselves.

use super::{Ink, Mesh, MeshVert, Vox};
use glam::Vec3;

/// How a model is meshed smooth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Smooth {
    /// how far corners move from the voxel lattice towards the smooth
    /// surface: 0 crisp cubes, a little a bevel, 1 surface nets
    pub bevel: f32,
    /// shading: 0 flat per face, 1 smooth normals across faces
    pub soft: f32,
    /// relaxation passes: 0 keeps the voxel steps' corners, more rounds
    /// them off (never further than half a voxel from the voxels)
    pub relax: u8,
    /// blend the colours of neighbouring voxels at each corner, for a
    /// painted look (crisp inks never blend)
    pub blend: bool,
    /// how dark creases get (0: no ambient occlusion)
    pub occlusion: f32,
    /// simplify the surface wherever that moves it less than this many
    /// voxels (0: keep every face)
    pub simplify: f32,
}

impl Default for Smooth {
    fn default() -> Self {
        Smooth {
            bevel: 1.0,
            soft: 1.0,
            relax: 4,
            blend: true,
            occlusion: 1.0,
            simplify: 0.0,
        }
    }
}

/// One face of the surface: its four corner vertices, the voxel's part
/// and ink, and which way it looks (index into `DIRS`).
struct NetQuad {
    v: [u32; 4],
    part: u8,
    ink: Ink,
    dir: u8,
}

const DIRS: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// The eight voxels round dual cell `k` (which covers voxels k-1 and k on
/// each axis).
fn corners(k: [i32; 3]) -> [[i32; 3]; 8] {
    std::array::from_fn(|i| {
        let i = i as i32;
        [
            k[0] - 1 + (i & 1),
            k[1] - 1 + ((i >> 1) & 1),
            k[2] - 1 + ((i >> 2) & 1),
        ]
    })
}

/// Each triangle's normal.
pub(super) fn face_normals(verts: &[MeshVert]) -> Vec<[f32; 3]> {
    verts
        .chunks_exact(3)
        .map(|t| {
            let a = Vec3::from(t[0].pos);
            let b = Vec3::from(t[1].pos);
            let c = Vec3::from(t[2].pos);
            (b - a).cross(c - a).normalize_or_zero().into()
        })
        .collect()
}

impl Vox {
    /// The cell index of a voxel in grid-local coordinates, if inside.
    fn local(&self, a: [i32; 3]) -> Option<usize> {
        (0..3)
            .all(|i| a[i] >= 0 && a[i] < self.size[i])
            .then(|| ((a[1] * self.size[2] + a[2]) * self.size[0] + a[0]) as usize)
    }

    fn local_solid(&self, a: [i32; 3]) -> bool {
        self.local(a).is_some_and(|i| self.cells[i] != 0)
    }

    /// Mesh parts as one smooth surface. Vertices sit on the dual grid
    /// over the whole model, so parts meet without seams at rest. Each part
    /// gets the faces of its own voxels; where it touches another part (a
    /// joint), the cut is capped so it stays closed when bent. Returns a
    /// mesh for each `(part, pivot)`, positions as for [`Vox::mesh`].
    pub fn mesh_smooth(&self, parts: &[(u8, [f32; 3])], opts: Smooth) -> Vec<Mesh> {
        let [sx, sy, sz] = self.size;
        let (dx, dy, dz) = (sx + 1, sy + 1, sz + 1);
        let cid = |k: [i32; 3]| ((k[1] * dz + k[2]) * dx + k[0]) as usize;
        let mut vid = vec![u32::MAX; (dx * dy * dz) as usize];
        let mut pos: Vec<Vec3> = Vec::new();
        let mut cell: Vec<[i32; 3]> = Vec::new();
        let mut quads: Vec<NetQuad> = Vec::new();
        let lin: Vec<Vec3> = self
            .palette
            .iter()
            .map(|(c, _)| Vec3::new(c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2)))
            .collect();
        // the surface: every face between a voxel and empty space
        for y in 0..sy {
            for z in 0..sz {
                for x in 0..sx {
                    let i = ((y * sz + z) * sx + x) as usize;
                    let ink = self.cells[i];
                    if ink == 0 {
                        continue;
                    }
                    let a = [x, y, z];
                    for (dir, e) in DIRS.iter().enumerate() {
                        if self.local_solid([a[0] + e[0], a[1] + e[1], a[2] + e[2]]) {
                            continue;
                        }
                        let d = dir / 2;
                        let (u, w) = ((d + 1) % 3, (d + 2) % 3);
                        let mut base = a;
                        base[d] += i32::from(e[d] > 0);
                        let mut v = [0u32; 4];
                        for (n, (du, dw)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate()
                        {
                            let mut k = base;
                            k[u] += du;
                            k[w] += dw;
                            let at = cid(k);
                            if vid[at] == u32::MAX {
                                vid[at] = pos.len() as u32;
                                let lattice = Vec3::from(k.map(|x| x as f32));
                                pos.push(lattice + (self.net_point(k) - lattice) * opts.bevel);
                                cell.push(k);
                            }
                            v[n] = vid[at];
                        }
                        if e[d] < 0 {
                            v.swap(1, 3);
                        }
                        quads.push(NetQuad {
                            v,
                            part: self.parts[i],
                            ink,
                            dir: dir as u8,
                        });
                    }
                }
            }
        }
        let nv = pos.len();
        let crisp_at = |c: [i32; 3]| {
            self.local(c)
                .is_some_and(|i| self.cells[i] != 0 && self.crisp[self.cells[i] as usize])
        };
        // pinned: a vertex next to a crisp voxel keeps its corners
        let pinned: Vec<bool> = cell
            .iter()
            .map(|k| corners(*k).into_iter().any(crisp_at))
            .collect();
        // neighbours along quad edges
        let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nv];
        for q in &quads {
            for e in 0..4 {
                let (a, b) = (q.v[e], q.v[(e + 1) % 4]);
                if !adj[a as usize].contains(&b) {
                    adj[a as usize].push(b);
                }
                if !adj[b as usize].contains(&a) {
                    adj[b as usize].push(a);
                }
            }
        }
        let mut next = pos.clone();
        for _ in 0..opts.relax {
            for i in 0..nv {
                if pinned[i] || adj[i].is_empty() {
                    next[i] = pos[i];
                    continue;
                }
                let avg =
                    adj[i].iter().map(|j| pos[*j as usize]).sum::<Vec3>() / adj[i].len() as f32;
                let k = Vec3::from(cell[i].map(|x| x as f32));
                next[i] = (pos[i] + (avg - pos[i]) * 0.5)
                    .clamp(k - Vec3::splat(0.5), k + Vec3::splat(0.5));
            }
            std::mem::swap(&mut pos, &mut next);
        }
        // normals: the faces round each vertex
        let mut nrm = vec![Vec3::ZERO; nv];
        for q in &quads {
            let p = q.v.map(|i| pos[i as usize]);
            let mut n = (p[2] - p[0]).cross(p[3] - p[1]);
            let e = Vec3::from(DIRS[q.dir as usize].map(|x| x as f32));
            if n.dot(e) < 0.0 {
                n = -n;
            }
            let n = n.normalize_or_zero();
            for i in q.v {
                nrm[i as usize] += n;
            }
        }
        for n in &mut nrm {
            *n = n.normalize_or_zero();
        }
        let ao = self.occlusion(&pos, &nrm, opts.occlusion);
        // painted colour at each vertex: the voxels round it, crisp and
        // glowing ones apart
        let blend: Vec<Option<Vec3>> = if opts.blend {
            cell.iter()
                .map(|k| {
                    let mut sum = Vec3::ZERO;
                    let mut n = 0.0;
                    for c in corners(*k) {
                        if let Some(i) = self.local(c) {
                            let ink = self.cells[i] as usize;
                            if ink != 0 && !self.crisp[ink] && !self.palette[ink].1 {
                                sum += lin[ink];
                                n += 1.0;
                            }
                        }
                    }
                    (n > 0.0).then(|| sum / n)
                })
                .collect()
        } else {
            Vec::new()
        };
        // two triangles a face, split along the shorter diagonal
        let mut tris: Vec<Tri> = Vec::with_capacity(quads.len() * 2);
        for q in &quads {
            let p = q.v.map(|i| pos[i as usize]);
            let order = if (p[0] - p[2]).length_squared() <= (p[1] - p[3]).length_squared() {
                [[0, 1, 2], [0, 2, 3]]
            } else {
                [[1, 2, 3], [1, 3, 0]]
            };
            for o in order {
                tris.push(Tri {
                    v: o.map(|i| q.v[i]),
                    part: q.part,
                    ink: q.ink,
                });
            }
        }
        if opts.simplify > 0.0 {
            // where parts meet and where crisp or glowing inks end stays put
            let mut seen: Vec<(u8, Ink, bool)> = vec![(0, 0, false); nv];
            let mut locked = vec![false; nv];
            for t in &tris {
                let special = self.crisp[t.ink as usize] || self.palette[t.ink as usize].1;
                for i in t.v {
                    let s = &mut seen[i as usize];
                    if !s.2 {
                        *s = (t.part, t.ink, true);
                    } else if s.0 != t.part
                        || s.1 != t.ink
                            && (special
                                || self.crisp[s.1 as usize]
                                || self.palette[s.1 as usize].1)
                    {
                        locked[i as usize] = true;
                    }
                }
            }
            decimate(&mut pos, &mut tris, &locked, opts.simplify);
        }
        let min = Vec3::from(self.min.map(|x| x as f32));
        let unit = self.unit;
        let place = |p: Vec3, pivot: Vec3| -> [f32; 3] { ((p + min) * unit - pivot).into() };
        // where each part's voxels are, so caps need not scan the grid
        let mut bounds = vec![([i32::MAX; 3], [i32::MIN; 3]); 256];
        for y in 0..sy {
            for z in 0..sz {
                for x in 0..sx {
                    let i = ((y * sz + z) * sx + x) as usize;
                    if self.cells[i] != 0 {
                        let b = &mut bounds[self.parts[i] as usize];
                        for (a, v) in [x, y, z].into_iter().enumerate() {
                            b.0[a] = b.0[a].min(v);
                            b.1[a] = b.1[a].max(v);
                        }
                    }
                }
            }
        }
        parts
            .iter()
            .map(|&(part, pivot)| {
                let pivot = Vec3::from(pivot);
                let mut verts = Vec::new();
                let mut normals = Vec::new();
                for t in tris.iter().filter(|t| t.part == part) {
                    let ink = t.ink as usize;
                    let glow = self.palette[ink].1;
                    let flat = glow || self.crisp[ink] || blend.is_empty();
                    let [a, b, c] = t.v.map(|i| pos[i as usize]);
                    let face = (b - a).cross(c - a).normalize_or_zero();
                    for i in t.v {
                        let i = i as usize;
                        let rgb = match blend.get(i).copied().flatten() {
                            Some(b) if !flat => b * 0.5 + lin[ink] * 0.5,
                            _ => lin[ink],
                        };
                        verts.push(MeshVert {
                            pos: place(pos[i], pivot),
                            color: [rgb.x, rgb.y, rgb.z, if glow { 2.0 } else { ao[i] }],
                        });
                        let n = (face * (1.0 - opts.soft) + nrm[i] * opts.soft).normalize_or(face);
                        normals.push(n.into());
                    }
                }
                // caps where the part meets another, in shadow: those
                // touching the surface follow it, the rest are flat and
                // merged into rectangles
                let quad = |c: [Vec3; 4], n: [f32; 3], ink: Ink, verts: &mut Vec<MeshVert>, normals: &mut Vec<[f32; 3]>| {
                    let rgb = lin[ink as usize] * 0.55;
                    for o in [0, 1, 2, 0, 2, 3] {
                        verts.push(MeshVert {
                            pos: place(c[o], pivot),
                            color: [rgb.x, rgb.y, rgb.z, 0.8],
                        });
                        normals.push(n);
                    }
                };
                let mut flat: std::collections::HashMap<(usize, i32, Ink), std::collections::HashSet<(i32, i32)>> =
                    std::collections::HashMap::new();
                let (lo, hi) = bounds[part as usize];
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        for x in lo[0]..=hi[0] {
                            let i = ((y * sz + z) * sx + x) as usize;
                            if self.cells[i] == 0 || self.parts[i] != part {
                                continue;
                            }
                            let a = [x, y, z];
                            for (dir, e) in DIRS.iter().enumerate() {
                                let Some(j) = self.local([a[0] + e[0], a[1] + e[1], a[2] + e[2]])
                                else {
                                    continue;
                                };
                                if self.cells[j] == 0 || self.parts[j] == part {
                                    continue;
                                }
                                let d = dir / 2;
                                let (u, w) = ((d + 1) % 3, (d + 2) % 3);
                                let mut base = a;
                                base[d] += i32::from(e[d] > 0);
                                let ks = [(0, 0), (1, 0), (1, 1), (0, 1)].map(|(du, dw)| {
                                    let mut k = base;
                                    k[u] += du;
                                    k[w] += dw;
                                    k
                                });
                                if ks.iter().all(|k| vid[cid(*k)] == u32::MAX) {
                                    flat.entry((dir, base[d], self.cells[i])).or_default().insert((base[u], base[w]));
                                    continue;
                                }
                                let mut c = ks.map(|k| match vid[cid(k)] {
                                    u32::MAX => Vec3::from(k.map(|x| x as f32)),
                                    v => pos[v as usize],
                                });
                                if e[d] < 0 {
                                    c.swap(1, 3);
                                }
                                quad(c, e.map(|v| v as f32), self.cells[i], &mut verts, &mut normals);
                            }
                        }
                    }
                }
                for ((dir, plane, ink), mut cells) in flat {
                    let d = dir / 2;
                    let (u, w) = ((d + 1) % 3, (d + 2) % 3);
                    let mut order: Vec<(i32, i32)> = cells.iter().copied().collect();
                    order.sort_by_key(|c| (c.1, c.0));
                    for (u0, w0) in order {
                        if !cells.contains(&(u0, w0)) {
                            continue;
                        }
                        let mut wide = 1;
                        while cells.contains(&(u0 + wide, w0)) {
                            wide += 1;
                        }
                        let mut high = 1;
                        while (0..wide).all(|i| cells.contains(&(u0 + i, w0 + high))) {
                            high += 1;
                        }
                        for j in 0..high {
                            for i in 0..wide {
                                cells.remove(&(u0 + i, w0 + j));
                            }
                        }
                        let at = |du: i32, dw: i32| {
                            let mut k = [0f32; 3];
                            k[d] = plane as f32;
                            k[u] = (u0 + du) as f32;
                            k[w] = (w0 + dw) as f32;
                            Vec3::from(k)
                        };
                        let mut c = [at(0, 0), at(wide, 0), at(wide, high), at(0, high)];
                        let e = DIRS[dir];
                        if e[d] < 0 {
                            c.swap(1, 3);
                        }
                        quad(c, e.map(|v| v as f32), ink, &mut verts, &mut normals);
                    }
                }
                let faces = face_normals(&verts);
                Mesh {
                    verts,
                    normals,
                    faces,
                    ..Mesh::default()
                }
            })
            .collect()
    }

    /// Where the surface crosses dual cell `k`: the mean of the midpoints
    /// of its edges that go from solid to empty (its centre if none do).
    fn net_point(&self, k: [i32; 3]) -> Vec3 {
        let c = corners(k);
        let s: [bool; 8] = std::array::from_fn(|i| self.local_solid(c[i]));
        let mut sum = Vec3::ZERO;
        let mut n = 0;
        for i in 0..8 {
            for axis in 0..3 {
                let j = i | (1 << axis);
                if j != i && s[i] != s[j] {
                    let a = Vec3::from(c[i].map(|x| x as f32 + 0.5));
                    let b = Vec3::from(c[j].map(|x| x as f32 + 0.5));
                    sum += (a + b) * 0.5;
                    n += 1;
                }
            }
        }
        if n > 0 {
            sum / n as f32
        } else {
            Vec3::from(k.map(|x| x as f32))
        }
    }

    /// Ambient occlusion per vertex: how much of the model lies just
    /// outside it, counted from a summed table of the solid voxels.
    fn occlusion(&self, pos: &[Vec3], nrm: &[Vec3], strength: f32) -> Vec<f32> {
        if strength <= 0.0 {
            return vec![1.0; pos.len()];
        }
        let [sx, sy, sz] = self.size;
        let (px, py, pz) = (sx + 1, sy + 1, sz + 1);
        let tid = |x: i32, y: i32, z: i32| ((y * pz + z) * px + x) as usize;
        let mut table = vec![0i32; (px * py * pz) as usize];
        for y in 1..py {
            for z in 1..pz {
                for x in 1..px {
                    let s = i32::from(self.local_solid([x - 1, y - 1, z - 1]));
                    table[tid(x, y, z)] = s
                        + table[tid(x - 1, y, z)]
                        + table[tid(x, y - 1, z)]
                        + table[tid(x, y, z - 1)]
                        - table[tid(x - 1, y - 1, z)]
                        - table[tid(x - 1, y, z - 1)]
                        - table[tid(x, y - 1, z - 1)]
                        + table[tid(x - 1, y - 1, z - 1)];
                }
            }
        }
        let count = |lo: [i32; 3], hi: [i32; 3]| -> i32 {
            let lo = [lo[0].clamp(0, sx), lo[1].clamp(0, sy), lo[2].clamp(0, sz)];
            let hi = [hi[0].clamp(0, sx), hi[1].clamp(0, sy), hi[2].clamp(0, sz)];
            if (0..3).any(|a| hi[a] <= lo[a]) {
                return 0;
            }
            let t = |x, y, z| table[tid(x, y, z)];
            t(hi[0], hi[1], hi[2])
                - t(lo[0], hi[1], hi[2])
                - t(hi[0], lo[1], hi[2])
                - t(hi[0], hi[1], lo[2])
                + t(lo[0], lo[1], hi[2])
                + t(lo[0], hi[1], lo[2])
                + t(hi[0], lo[1], lo[2])
                - t(lo[0], lo[1], lo[2])
        };
        pos.iter()
            .zip(nrm)
            .map(|(p, n)| {
                let q = (*p + *n * 2.0).floor().as_ivec3();
                let r = 2;
                let k = count(
                    [q.x - r, q.y - r, q.z - r],
                    [q.x + r + 1, q.y + r + 1, q.z + r + 1],
                );
                let f = k as f32 / 125.0;
                (1.0 - ((f - 0.08) * 1.6).max(0.0) * strength).clamp(0.45, 1.0)
            })
            .collect()
    }
}

/// One triangle of the surface: corners, part and ink.
#[derive(Clone, Copy)]
struct Tri {
    v: [u32; 3],
    part: u8,
    ink: Ink,
}

/// A plane's error quadric (symmetric 4x4, upper triangle).
#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane(n: Vec3, d: f32, w: f32) -> Quadric {
        let (a, b, c, d) = (n.x as f64, n.y as f64, n.z as f64, d as f64);
        let w = w as f64;
        Quadric([
            a * a * w,
            a * b * w,
            a * c * w,
            a * d * w,
            b * b * w,
            b * c * w,
            b * d * w,
            c * c * w,
            c * d * w,
            d * d * w,
        ])
    }

    fn add(&mut self, o: &Quadric) {
        for i in 0..10 {
            self.0[i] += o.0[i];
        }
    }

    fn error(&self, p: Vec3) -> f64 {
        let q = &self.0;
        let (x, y, z) = (p.x as f64, p.y as f64, p.z as f64);
        q[0] * x * x + 2.0 * q[1] * x * y + 2.0 * q[2] * x * z + 2.0 * q[3] * x
            + q[4] * y * y
            + 2.0 * q[5] * y * z
            + 2.0 * q[6] * y
            + q[7] * z * z
            + 2.0 * q[8] * z
            + q[9]
    }
}

/// Simplify the surface by collapsing edges (quadric error), while the
/// error stays under `max_err` (grid units). Locked vertices (where parts
/// meet, the edges of crisp inks) never move.
fn decimate(pos: &mut [Vec3], tris: &mut Vec<Tri>, locked: &[bool], max_err: f32) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let nv = pos.len();
    let mut vt: Vec<Vec<u32>> = vec![Vec::new(); nv];
    let mut q = vec![Quadric::default(); nv];
    for (t, tri) in tris.iter().enumerate() {
        let [a, b, c] = tri.v.map(|i| pos[i as usize]);
        let n = (b - a).cross(c - a);
        let area = n.length();
        if area < 1e-9 {
            continue;
        }
        let n = n / area;
        let k = Quadric::plane(n, -n.dot(a), area);
        for i in tri.v {
            vt[i as usize].push(t as u32);
            q[i as usize].add(&k);
        }
    }
    // borders between parts and inks are held by their locks; give edges
    // round locked vertices no special weight
    let mut alive = vec![true; tris.len()];
    let mut version = vec![0u32; nv];
    let mut gone = vec![false; nv];
    let limit = (max_err as f64) * (max_err as f64);
    // the cost of collapsing u into v (v keeps), and where v goes
    let cost = |u: usize, v: usize, pos: &[Vec3], q: &[Quadric]| -> Option<(f64, Vec3)> {
        if locked[u] {
            return None;
        }
        let mut qq = q[u];
        qq.add(&q[v]);
        let cands: &[Vec3] = if locked[v] {
            &[pos[v]]
        } else {
            &[pos[v], (pos[u] + pos[v]) * 0.5]
        };
        cands
            .iter()
            .map(|p| (qq.error(*p).max(0.0), *p))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    };
    let mut heap: BinaryHeap<Reverse<(u64, u32, u32, u32, u32)>> = BinaryHeap::new();
    let key = |c: f64| (c * 1e9) as u64;
    // an edge goes on the heap once, the cheaper way round
    let push = |heap: &mut BinaryHeap<_>, a: usize, b: usize, pos: &[Vec3], q: &[Quadric], version: &[u32]| {
        let ab = cost(a, b, pos, q).map(|c| c.0);
        let ba = cost(b, a, pos, q).map(|c| c.0);
        let (c, u, v) = match (ab, ba) {
            (Some(x), Some(y)) if y < x => (y, b, a),
            (Some(x), _) => (x, a, b),
            (None, Some(y)) => (y, b, a),
            (None, None) => return,
        };
        if c <= limit {
            heap.push(Reverse((key(c), u as u32, v as u32, version[u], version[v])));
        }
    };
    for tri in tris.iter() {
        for e in 0..3 {
            let (a, b) = (tri.v[e] as usize, tri.v[(e + 1) % 3] as usize);
            if a < b {
                push(&mut heap, a, b, pos, &q, &version);
            }
        }
    }
    let ring = |v: usize, vt: &[Vec<u32>], tris: &[Tri], alive: &[bool]| -> Vec<u32> {
        let mut r: Vec<u32> = Vec::new();
        for &t in &vt[v] {
            if !alive[t as usize] {
                continue;
            }
            for w in tris[t as usize].v {
                if w as usize != v && !r.contains(&w) {
                    r.push(w);
                }
            }
        }
        r
    };
    while let Some(Reverse((_, u, v, vu, vv))) = heap.pop() {
        let (u, v) = (u as usize, v as usize);
        if gone[u] || gone[v] || version[u] != vu || version[v] != vv {
            continue;
        }
        let Some((c, target)) = cost(u, v, pos, &q) else {
            continue;
        };
        if c > limit {
            continue;
        }
        // the link condition keeps the surface a manifold
        let ru = ring(u, &vt, tris, &alive);
        let rv = ring(v, &vt, tris, &alive);
        if !rv.contains(&(u as u32)) {
            continue;
        }
        let common = ru.iter().filter(|w| rv.contains(w)).count();
        // and no vertex gathers a fan of slivers
        if common != 2 || ru.len() + rv.len() - common - 2 > 10 {
            continue;
        }
        // no triangle may flip or fold
        let mut ok = true;
        for &t in vt[u].iter().chain(vt[v].iter()) {
            let t = t as usize;
            if !alive[t] {
                continue;
            }
            let tri = tris[t].v;
            if tri.contains(&(u as u32)) && tri.contains(&(v as u32)) {
                continue;
            }
            let before = tri.map(|i| pos[i as usize]);
            let after = tri.map(|i| {
                if i as usize == u || i as usize == v {
                    target
                } else {
                    pos[i as usize]
                }
            });
            let n0 = (before[1] - before[0]).cross(before[2] - before[0]);
            let n1 = (after[1] - after[0]).cross(after[2] - after[0]);
            if n0.normalize_or_zero().dot(n1.normalize_or_zero()) < 0.35 {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }
        // collapse u into v
        pos[v] = target;
        let qu = q[u];
        q[v].add(&qu);
        gone[u] = true;
        let moved = std::mem::take(&mut vt[u]);
        for t in moved {
            let ti = t as usize;
            if !alive[ti] {
                continue;
            }
            if tris[ti].v.contains(&(v as u32)) {
                alive[ti] = false;
                continue;
            }
            for i in tris[ti].v.iter_mut() {
                if *i as usize == u {
                    *i = v as u32;
                }
            }
            vt[v].push(t);
        }
        vt[v].retain(|t| alive[*t as usize]);
        // only edges at v changed cost
        version[v] += 1;
        for w in ring(v, &vt, tris, &alive) {
            let w = w as usize;
            push(&mut heap, v, w, pos, &q, &version);
        }
    }
    let mut out = Vec::with_capacity(tris.len() / 2);
    for (t, tri) in tris.iter().enumerate() {
        if alive[t] {
            out.push(*tri);
        }
    }
    *tris = out;
}
