//! Leaf crowns as chunky voxel clumps, coloured from the pack.
//!
//! Which cells hold leaves is the world's business. This draws them: each
//! leaf block is cut into small cubes (`cells` a side, half a metre at 2),
//! the ones on the outside of a crown worn away here and there so the crown
//! is a mass of clumps rather than a box, each cube with its own colour from
//! the block's palette, darker in the creases between clumps and towards the
//! underside of the crown. An area of the map (the rules' area keys) may turn
//! a share of its trees to other tones: autumn golds and reds, blossom.
//! All of it comes from `foliage.json`; without the file (or with
//! `cells` 0) canopies are drawn smooth, as the terrain mesher draws them.
//!
//! What stands where is a pure function of world coordinates and the leaf
//! blocks around, so neighbouring sections agree at their borders.

use crate::assets;
use crate::render::{self, KIND_FOLIAGE, SECTION, Vertex};
use crate::terrain_mesh::{F_LEAVES, F_OPAQUE, Grid, SectionGeo, at};
use dyno_world::light::Light;
use dyno_world::{self as w};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// The pack file.
pub const FOLIAGE_JSON: &str = "foliage.json";

const N: i32 = SECTION;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
struct FoliageFile {
    /// cubes along the side of a leaf block (0: smooth canopies)
    cells: u8,
    /// how much of a crown's outside is worn away, 0 to 1
    erode: f32,
    /// brightness in the tightest crease between clumps (1: none)
    crease: f32,
    /// brightness at the underside of a deep crown, and how many blocks of
    /// leaves overhead count as deep
    under: f32,
    depth: f32,
    /// how much each cube's brightness strays from its colour
    vary: f32,
    /// colours (sRGB, 0 to 255) per leaf block key; each cube takes one
    leaves: BTreeMap<String, Vec<[u8; 3]>>,
    /// the blocks trees are made of: leaves near one take their tone from
    /// where it stands, so a tree is one tone
    trunks: Vec<String>,
    /// per area of the map: trees that turn to other tones
    areas: BTreeMap<String, AreaTones>,
}

impl Default for FoliageFile {
    fn default() -> Self {
        FoliageFile {
            cells: 0,
            erode: 0.4,
            crease: 0.6,
            under: 0.7,
            depth: 4.0,
            vary: 0.1,
            leaves: BTreeMap::new(),
            trunks: Vec::new(),
            areas: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AreaTones {
    /// the leaf blocks that turn
    blocks: Vec<String>,
    /// metres across a patch of one tone (about a tree)
    patch: f32,
    /// each takes its share of the trees; the rest keep their own colours
    tones: Vec<Tone>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct Tone {
    share: f32,
    colors: Vec<[u8; 3]>,
}

type Palette = Vec<[f32; 3]>;

struct Area {
    key: String,
    blocks: [bool; 256],
    patch: f32,
    /// (share of the trees up to and including this tone, its colours)
    tones: Vec<(f32, Palette)>,
}

/// The pack's foliage, ready to mesh.
pub struct Foliage {
    cells: i32,
    erode: f32,
    crease: f32,
    under: f32,
    depth: f32,
    vary: f32,
    /// per block ID
    palettes: Vec<Palette>,
    trunks: [bool; 256],
    areas: Vec<Area>,
}

fn linear(c: &[u8; 3]) -> [f32; 3] {
    c.map(|v| (v as f32 / 255.0).powf(2.2))
}

impl Foliage {
    fn load(text: &str) -> Result<Foliage, String> {
        let f: FoliageFile =
            serde_json::from_str(text).map_err(|e| format!("{FOLIAGE_JSON}: {e}"))?;
        let mut palettes = vec![Vec::new(); 256];
        for (key, colors) in &f.leaves {
            if let Some(b) = w::block_by_key(key) {
                palettes[b as usize] = colors.iter().map(linear).collect();
            }
        }
        let mut trunks = [false; 256];
        for b in f.trunks.iter().filter_map(|k| w::block_by_key(k)) {
            trunks[b as usize] = true;
        }
        let areas = f
            .areas
            .iter()
            .map(|(key, a)| {
                let mut blocks = [false; 256];
                for b in a.blocks.iter().filter_map(|k| w::block_by_key(k)) {
                    blocks[b as usize] = true;
                }
                let mut upto = 0.0;
                let tones = a
                    .tones
                    .iter()
                    .filter(|t| !t.colors.is_empty())
                    .map(|t| {
                        upto += t.share;
                        (upto, t.colors.iter().map(linear).collect())
                    })
                    .collect();
                Area {
                    key: key.clone(),
                    blocks,
                    patch: a.patch.max(1.0),
                    tones,
                }
            })
            .collect();
        Ok(Foliage {
            cells: (f.cells as i32).min(4),
            erode: f.erode.clamp(0.0, 1.0),
            crease: f.crease.clamp(0.0, 1.0),
            under: f.under.clamp(0.0, 1.0),
            depth: f.depth.max(1.0),
            vary: f.vary.clamp(0.0, 0.5),
            palettes,
            trunks,
            areas,
        })
    }

    /// Whether leaves are drawn as cubes (else the smooth canopy).
    pub fn voxels(&self) -> bool {
        self.cells > 0
    }
}

/// The foliage in the pack now, read again only when the file changes.
pub fn current() -> Option<Arc<Foliage>> {
    type Cache = Option<(Arc<[u8]>, Arc<Foliage>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let bytes = assets::get(FOLIAGE_JSON)?;
    let mut c = CACHE.lock().unwrap();
    if let Some((b, f)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return Some(f.clone());
    }
    let f = std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())
        .and_then(Foliage::load)
        .map_err(|e| eprintln!("{e}"))
        .ok()?;
    let f = Arc::new(f);
    *c = Some((bytes, f.clone()));
    Some(f)
}

#[inline]
fn hash(x: i32, y: i32, z: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D)
        ^ salt.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

#[inline]
fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / 16_777_216.0
}

/// (normal, the two edges of the face)
type Face = ([i32; 3], [i32; 3], [i32; 3]);
const FACES: [Face; 6] = [
    ([1, 0, 0], [0, 1, 0], [0, 0, 1]),
    ([-1, 0, 0], [0, 0, 1], [0, 1, 0]),
    ([0, 1, 0], [0, 0, 1], [1, 0, 0]),
    ([0, -1, 0], [1, 0, 0], [0, 0, 1]),
    ([0, 0, 1], [1, 0, 0], [0, 1, 0]),
    ([0, 0, -1], [0, 1, 0], [1, 0, 0]),
];

const EMPTY: u8 = 0;
const LEAF: u8 = 1;
const SOLID: u8 = 2;

/// Where the nearest trunk block stands, as blocks from `c` (x, z): at the
/// block's own height or up to four below it, at most three blocks away.
fn nearest_trunk(f: &Foliage, g: &Grid, c: [i32; 3]) -> Option<(i32, i32)> {
    for r in 1..=3i32 {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dz.abs()) != r {
                    continue;
                }
                for dy in -4..=0 {
                    if f.trunks[g.ids[at(c[0] + dx, c[1] + dy, c[2] + dz)] as usize] {
                        return Some((dx, dz));
                    }
                }
            }
        }
    }
    None
}

/// Draw the section's leaf blocks as clumps of cubes.
pub(crate) fn mesh(f: &Foliage, g: &Grid, light: &Light, geo: &mut SectionGeo) {
    let n = f.cells;
    // which cubes are there, for the section's blocks and one block round them
    let dim = ((N + 2) * n) as usize;
    let idx = |x: i32, y: i32, z: i32| {
        ((y + n) as usize * dim + (z + n) as usize) * dim + (x + n) as usize
    };
    let mut fill = vec![EMPTY; dim * dim * dim];
    let leafy = |x: i32, y: i32, z: i32| g.fl[at(x, y, z)] & (F_LEAVES | F_OPAQUE) != 0;
    for by in -1..=N {
        for bz in -1..=N {
            for bx in -1..=N {
                let fl = g.fl[at(bx, by, bz)];
                if fl & (F_LEAVES | F_OPAQUE) == 0 {
                    continue;
                }
                let leaf = fl & F_LEAVES != 0;
                // the open sides of the block
                let open = |d: i32, axis: usize| {
                    let mut p = [bx, by, bz];
                    p[axis] += d;
                    !leafy(p[0], p[1], p[2])
                };
                let sides = if leaf {
                    [
                        [open(-1, 0), open(1, 0)],
                        [open(-1, 1), open(1, 1)],
                        [open(-1, 2), open(1, 2)],
                    ]
                } else {
                    [[false; 2]; 3]
                };
                for j in 0..n {
                    for k in 0..n {
                        for i in 0..n {
                            let c = [i, j, k];
                            let mut v = if leaf { LEAF } else { SOLID };
                            if leaf {
                                // a cube on an open side may be worn away,
                                // more readily on an edge or a corner
                                let exposed = (0..3)
                                    .filter(|a| {
                                        (c[*a] == 0 && sides[*a][0])
                                            || (c[*a] == n - 1 && sides[*a][1])
                                    })
                                    .count();
                                let chance = f.erode * [0.0, 0.25, 0.85, 1.25][exposed];
                                let (wx, wy, wz) = (
                                    (g.o[0] + bx) * n + i,
                                    (g.o[1] + by) * n + j,
                                    (g.o[2] + bz) * n + k,
                                );
                                if exposed > 0 && unit(hash(wx, wy, wz, 1)) < chance {
                                    v = EMPTY;
                                }
                            }
                            fill[idx(bx * n + i, by * n + j, bz * n + k)] = v;
                        }
                    }
                }
            }
        }
    }
    let t = render::textures();
    let areas = crate::cover::areas();
    let s = 1.0 / n as f32;
    for by in 0..N {
        for bz in 0..N {
            for bx in 0..N {
                let c = at(bx, by, bz);
                if g.fl[c] & F_LEAVES == 0 {
                    continue;
                }
                let b = g.ids[c];
                let (wx, wy, wz) = (g.o[0] + bx, g.o[1] + by, g.o[2] + bz);
                // the block's colours: its own, or a tone of the area's
                let mut palette = &f.palettes[b as usize];
                if let Some(a) = crate::cover::area_at(&areas, wx as f32 + 0.5, wz as f32 + 0.5)
                    .and_then(|key| f.areas.iter().find(|a| a.key == key))
                    && a.blocks[b as usize]
                {
                    // the tone of the nearest trunk or bough, so a tree is
                    // one tone; of the place itself when none is near
                    let (tx, tz) = nearest_trunk(f, g, [bx, by, bz])
                        .map_or((wx, wz), |(dx, dz)| (wx + dx, wz + dz));
                    let v = crate::cover::noise(tx as f32 / a.patch, tz as f32 / a.patch, 41);
                    // (the noise gathers round the middle: spread it out)
                    let v = ((v - 0.5) * 1.9 + 0.5).clamp(0.0, 0.999);
                    if let Some((_, p)) = a.tones.iter().find(|(upto, _)| v < *upto) {
                        palette = p;
                    }
                }
                // how deep under the top of the crown the block lies
                let mut over = 0;
                while over < f.depth as i32
                    && over < 3
                    && g.fl[at(bx, by + 1 + over, bz)] & F_LEAVES != 0
                {
                    over += 1;
                }
                let layer = t.face(b, 0);
                let tex = [1.0, 0.0, KIND_FOLIAGE + layer as f32, 0.0];
                for j in 0..n {
                    for k in 0..n {
                        for i in 0..n {
                            let p = [bx * n + i, by * n + j, bz * n + k];
                            if fill[idx(p[0], p[1], p[2])] != LEAF {
                                continue;
                            }
                            let (cx, cy, cz) = (wx * n + i, wy * n + j, wz * n + k);
                            let h = hash(cx, cy, cz, 2);
                            let rgb = if palette.is_empty() {
                                [1.0; 3]
                            } else {
                                palette[(h >> 10) as usize % palette.len()]
                            };
                            let depth = over as f32 + (n - 1 - j) as f32 * s;
                            let shade = (1.0 - (1.0 - f.under) * (depth / f.depth).min(1.0))
                                * (1.0 + f.vary * (unit(h) * 2.0 - 1.0));
                            for (nrm, u, v) in FACES {
                                let q = [p[0] + nrm[0], p[1] + nrm[1], p[2] + nrm[2]];
                                if fill[idx(q[0], q[1], q[2])] != EMPTY {
                                    continue;
                                }
                                // the light of the block the face looks into
                                let lb =
                                    [q[0].div_euclid(n), q[1].div_euclid(n), q[2].div_euclid(n)];
                                let (bl, sl) = g.light(light, lb);
                                let o = [0, 1, 2].map(|a| p[a] + nrm[a].max(0));
                                let mut quad = [Vertex {
                                    pos: [0.0; 3],
                                    normal: [nrm[0] as f32, nrm[1] as f32, nrm[2] as f32],
                                    color: [rgb[0], rgb[1], rgb[2], 1.0],
                                    light: bl / 15.0,
                                    sky: sl / 15.0,
                                    tex,
                                }; 4];
                                let mut ao = [0u8; 4];
                                for (vi, (a, b2)) in
                                    [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate()
                                {
                                    let (su, sv) = (a * 2 - 1, b2 * 2 - 1);
                                    let at_q = |du: i32, dv: i32| {
                                        fill[idx(
                                            q[0] + du * u[0] + dv * v[0],
                                            q[1] + du * u[1] + dv * v[1],
                                            q[2] + du * u[2] + dv * v[2],
                                        )] != EMPTY
                                    };
                                    let (s1, s2, cn) = (at_q(su, 0), at_q(0, sv), at_q(su, sv));
                                    ao[vi] = if s1 && s2 {
                                        3
                                    } else {
                                        s1 as u8 + s2 as u8 + cn as u8
                                    };
                                    let crease = 1.0 - (1.0 - f.crease) * ao[vi] as f32 / 3.0;
                                    quad[vi].pos = [0, 1, 2].map(|ax| {
                                        g.o[ax] as f32 + (o[ax] + a * u[ax] + b2 * v[ax]) as f32 * s
                                    });
                                    quad[vi].color[3] = shade * crease;
                                }
                                // split so the shading runs evenly across the face
                                let order = if ao[0] + ao[2] > ao[1] + ao[3] {
                                    [1, 2, 3, 1, 3, 0]
                                } else {
                                    [0, 1, 2, 0, 2, 3]
                                };
                                geo.opaque.extend(order.map(|vi| quad[vi]));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Arc<Foliage> {
        // (loads the whole pack, whatever other tests put in the store first)
        crate::shapes::pack();
        current().expect("the pack has foliage.json")
    }

    /// The pack's foliage file reads, and every block and colour list in it
    /// is usable.
    #[test]
    fn the_packs_foliage_reads() {
        // (loads the whole pack, whatever other tests put in the store first)
        crate::shapes::pack();
        let text = assets::text(FOLIAGE_JSON).unwrap();
        let file: FoliageFile = serde_json::from_str(&text).unwrap();
        for (key, colors) in &file.leaves {
            let b = w::block_by_key(key).unwrap_or_else(|| panic!("no block '{key}'"));
            assert!(w::is_leaves(b), "{key} is not leaves");
            assert!(!colors.is_empty(), "{key}");
        }
        for (key, a) in &file.areas {
            assert!(
                a.tones.iter().map(|t| t.share).sum::<f32>() <= 1.001,
                "{key}"
            );
            for b in &a.blocks {
                assert!(w::block_by_key(b).is_some_and(w::is_leaves), "{key}: {b}");
            }
        }
        assert!(pack().voxels());
    }

    /// Neighbouring sections draw the same cubes where they meet: no face
    /// of one section's leaves lies inside another's, and the crowns have no
    /// faces at the border that the other side would have hidden.
    #[test]
    fn crowns_agree_across_sections() {
        let f = pack();
        let mut world = crate::terrain_mesh::test_world([48, 48, 48]);
        // a crown that straddles the corner where eight sections meet
        for y in 10..22 {
            for z in 10..22 {
                for x in 10..22 {
                    let d = (x - 16) * (x - 16) + (y - 16) * (y - 16) + (z - 16) * (z - 16);
                    if d < 30 {
                        world.set(x, y, z, w::LEAVES);
                    }
                }
            }
        }
        let light = Light::new(&world);
        // every face drawn says the cube behind it is there and the one in
        // front of it is not: no section may contradict another
        let (mut full, mut open) = (
            std::collections::HashSet::new(),
            std::collections::HashSet::new(),
        );
        let mut faces = 0;
        let n = f.cells as f32;
        for sy in 0..3 {
            for sz in 0..3 {
                for sx in 0..3 {
                    let g = Grid::load(&world, &light, [sx * N, sy * N, sz * N]);
                    let mut geo = SectionGeo::default();
                    mesh(&f, &g, &light, &mut geo);
                    for tri in geo.opaque.chunks(6) {
                        let mid = tri.iter().fold([0f32; 3], |a, v| {
                            [0, 1, 2].map(|i| a[i] + v.pos[i] * n / 6.0)
                        });
                        let nrm = tri[0].normal;
                        let cube =
                            |side: f32| [0, 1, 2].map(|i| (mid[i] + nrm[i] * side).floor() as i32);
                        full.insert(cube(-0.5));
                        open.insert(cube(0.5));
                        faces += 1;
                    }
                }
            }
        }
        assert!(faces > 100, "the crown has faces: {faces}");
        assert!(full.is_disjoint(&open), "sections disagree about a cube");
        // and the whole world meshed as one piece gives the same faces
        let mut again = 0;
        for sy in 0..3 {
            for sz in 0..3 {
                for sx in 0..3 {
                    again += crate::terrain_mesh::mesh(&world, &light, sx, sy, sz)
                        .opaque
                        .iter()
                        .filter(|v| v.tex[2] >= KIND_FOLIAGE && v.tex[0] > 0.5)
                        .count()
                        / 6;
                }
            }
        }
        assert_eq!(again, faces);
    }
}
