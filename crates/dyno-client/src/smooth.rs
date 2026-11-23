//! Smooth meshing for voxel models: surface nets over the whole model,
//! relaxed, with normals per vertex, painted colour and ambient occlusion.
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
    /// relaxation passes: 0 keeps the voxel steps' corners, more rounds
    /// them off (never further than half a voxel from the voxels)
    pub relax: u8,
    /// blend the colours of neighbouring voxels at each corner, for a
    /// painted look (crisp inks never blend)
    pub blend: bool,
    /// how dark creases get (0: no ambient occlusion)
    pub occlusion: f32,
}

impl Default for Smooth {
    fn default() -> Self {
        Smooth {
            relax: 4,
            blend: true,
            occlusion: 1.0,
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
                                pos.push(self.net_point(k));
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
                for q in quads.iter().filter(|q| q.part == part) {
                    let ink = q.ink as usize;
                    let glow = self.palette[ink].1;
                    let flat = glow || self.crisp[ink] || blend.is_empty();
                    let p = q.v.map(|i| pos[i as usize]);
                    // split along the shorter diagonal
                    let order = if (p[0] - p[2]).length_squared() <= (p[1] - p[3]).length_squared()
                    {
                        [0, 1, 2, 0, 2, 3]
                    } else {
                        [1, 2, 3, 1, 3, 0]
                    };
                    for o in order {
                        let i = q.v[o] as usize;
                        let rgb = match blend.get(i).copied().flatten() {
                            Some(b) if !flat => b * 0.5 + lin[ink] * 0.5,
                            _ => lin[ink],
                        };
                        verts.push(MeshVert {
                            pos: place(pos[i], pivot),
                            color: [rgb.x, rgb.y, rgb.z, if glow { 2.0 } else { ao[i] }],
                        });
                        normals.push(nrm[i].into());
                    }
                }
                // caps where the part meets another
                let (lo, hi) = bounds[part as usize];
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        for x in lo[0]..=hi[0] {
                            let i = ((y * sz + z) * sx + x) as usize;
                            let ink = self.cells[i];
                            if ink == 0 || self.parts[i] != part {
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
                                let corner = |du: i32, dw: i32| -> Vec3 {
                                    let mut k = base;
                                    k[u] += du;
                                    k[w] += dw;
                                    match vid[cid(k)] {
                                        u32::MAX => Vec3::from(k.map(|x| x as f32)),
                                        v => pos[v as usize],
                                    }
                                };
                                let mut c =
                                    [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
                                if e[d] < 0 {
                                    c.swap(1, 3);
                                }
                                // the inside of a joint, in shadow
                                let rgb = lin[ink as usize] * 0.4;
                                let n = e.map(|v| v as f32);
                                for o in [0, 1, 2, 0, 2, 3] {
                                    verts.push(MeshVert {
                                        pos: place(c[o], pivot),
                                        color: [rgb.x, rgb.y, rgb.z, 0.7],
                                    });
                                    normals.push(n);
                                }
                            }
                        }
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
