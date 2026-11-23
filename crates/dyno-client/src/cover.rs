//! Ground cover: grass tufts, flowers, ferns, reeds, pebbles and small
//! rocks, as small voxel models from the pack, scattered over the ground by
//! the pack's rules (`cover.json`) and drawn instanced near the camera.
//!
//! The engine knows no plant: a rule names the models to scatter, the blocks
//! they stand on, how many per block, and what thins or gathers them
//! (patches of noise, blocks nearby such as water or paving, the light of
//! the sky, the area of the map). The mesher hands over each open top of the
//! ground (`Top`); `scatter` turns those into `Spot`s on the world thread;
//! `Gpu` copies the spots near the camera into one instance buffer, sorted
//! by model, thinning them with distance and with the player's setting.

use crate::assets;
use crate::terrain_mesh::{Grid, at};
use bytemuck::{Pod, Zeroable};
use dyno_world::{self as w, BlockId};
use egui_wgpu::wgpu;
use glam::Vec3;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

/// The pack file.
pub const COVER_JSON: &str = "cover.json";

/// How far (in blocks) a rule may look for blocks nearby.
const MAX_WITHIN: i32 = 3;

// ------------------------------------------------------------ the pack file

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct CoverFile {
    /// metres per voxel of the models
    voxel: f32,
    /// cover is drawn this far from the camera (at full quality)
    range: f32,
    /// everything grows within this distance; beyond it each rule thins to
    /// its `far` share at `range`
    near: f32,
    /// nearer than this a model is drawn voxel by voxel; beyond it, each
    /// column of voxels as one box shaded from foot to tip
    detail: f32,
    /// named sets of models: a `.vox` file, every model in it a variant
    kinds: BTreeMap<String, KindDef>,
    layers: Vec<LayerDef>,
}

impl Default for CoverFile {
    fn default() -> Self {
        CoverFile {
            voxel: 1.0 / 16.0,
            range: 48.0,
            near: 12.0,
            detail: 14.0,
            kinds: BTreeMap::new(),
            layers: Vec::new(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct KindDef {
    file: String,
    /// metres per voxel, when not the file's
    voxel: Option<f32>,
    /// how much the wind moves it (0: stone)
    sway: f32,
    /// 0: lit as the solid it is; 1: lit evenly, as the ground under it
    soft: f32,
}

impl Default for KindDef {
    fn default() -> Self {
        KindDef {
            file: String::new(),
            voxel: None,
            sway: 0.0,
            soft: 0.0,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
struct LayerDef {
    /// a note for whoever edits the file
    name: String,
    kinds: Vec<String>,
    /// keys of the blocks it stands on
    on: Vec<String>,
    /// how many per block top, before anything thins it
    per_block: f32,
    /// grows in patches: none where the noise is below `from`, all above `to`
    patch: Option<Patch>,
    /// grows only beside these blocks
    near: Option<Beside>,
    /// thins beside these blocks
    away: Option<Beside>,
    /// only where the sky light (0 to 1) lies in this range
    sky: [f32; 2],
    /// share still drawn at the edge of the range
    far: f32,
    /// tint, between these two at random
    tint: [[f32; 3]; 2],
    /// per area of the map (the rules' area keys): density and tint
    areas: BTreeMap<String, AreaRule>,
    /// density outside the areas named
    elsewhere: f32,
}

impl Default for LayerDef {
    fn default() -> Self {
        LayerDef {
            name: String::new(),
            kinds: Vec::new(),
            on: Vec::new(),
            per_block: 1.0,
            patch: None,
            near: None,
            away: None,
            sky: [0.0, 1.0],
            far: 0.1,
            tint: [[1.0; 3]; 2],
            areas: BTreeMap::new(),
            elsewhere: 1.0,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(deny_unknown_fields)]
struct Patch {
    /// metres across a patch
    scale: f32,
    from: f32,
    to: f32,
    #[serde(default)]
    seed: u32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct Beside {
    blocks: Vec<String>,
    within: u8,
    /// `away` only: the share kept right beside them
    #[serde(default)]
    keep: f32,
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(default, deny_unknown_fields)]
struct AreaRule {
    density: f32,
    tint: [f32; 3],
}

impl Default for AreaRule {
    fn default() -> Self {
        AreaRule {
            density: 1.0,
            tint: [1.0; 3],
        }
    }
}

// ------------------------------------------------------------ compiled

/// One vertex of a model.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ModelVertex {
    /// metres from the root
    pos: [f32; 3],
    /// how far the wind moves it
    sway: f32,
    normal: [i8; 4],
    /// sRGB
    color: [u8; 4],
}

#[derive(Default)]
struct Mesh {
    verts: Vec<ModelVertex>,
    indices: Vec<u32>,
}

struct Model {
    /// voxel by voxel, and each column as one box (for the distance)
    lods: [Mesh; 2],
}

struct Layer {
    models: Vec<u16>,
    per_block: f32,
    patch: Option<Patch>,
    /// (block set, within)
    near: Option<(usize, i32)>,
    /// (block set, within, keep)
    away: Option<(usize, i32, f32)>,
    sky: [f32; 2],
    far: f32,
    tint: [[f32; 3]; 2],
    areas: Vec<(String, AreaRule)>,
    elsewhere: f32,
}

/// The pack's ground cover, ready to scatter and draw.
pub struct Cover {
    pub range: f32,
    near: f32,
    detail: f32,
    models: Vec<Model>,
    layers: Vec<Layer>,
    /// sets of blocks the rules look for nearby
    sets: Vec<[bool; 256]>,
    /// per block: the layers that stand on it
    by_block: Vec<Vec<u16>>,
}

impl Cover {
    fn empty() -> Cover {
        Cover {
            range: 0.0,
            near: 0.0,
            detail: 0.0,
            models: Vec::new(),
            layers: Vec::new(),
            sets: Vec::new(),
            by_block: vec![Vec::new(); 256],
        }
    }

    /// Read the rules and the models they name from the asset store.
    /// A model file that has not arrived leaves its kind empty.
    fn load(text: &str) -> Result<Cover, String> {
        let file: CoverFile =
            serde_json::from_str(text).map_err(|e| format!("{COVER_JSON}: {e}"))?;
        let mut cover = Cover::empty();
        cover.range = file.range.max(1.0);
        cover.near = file.near.clamp(0.0, cover.range);
        cover.detail = file.detail.max(0.0);
        let mut kinds: BTreeMap<&str, Vec<u16>> = BTreeMap::new();
        for (name, k) in &file.kinds {
            let mut ids = Vec::new();
            if let Some(bytes) = assets::get(&k.file) {
                let vox = crate::vox::read(&bytes).map_err(|e| format!("{}: {e}", k.file))?;
                for m in &vox.models {
                    ids.push(cover.models.len() as u16);
                    cover.models.push(build_model(
                        m,
                        &vox.palette,
                        k.voxel.unwrap_or(file.voxel),
                        k,
                    ));
                }
            }
            kinds.insert(name, ids);
        }
        let set = |blocks: &[String], sets: &mut Vec<[bool; 256]>| {
            let mut s = [false; 256];
            for b in blocks.iter().filter_map(|k| w::block_by_key(k)) {
                s[b as usize] = true;
            }
            match sets.iter().position(|o| *o == s) {
                Some(i) => i,
                None => {
                    sets.push(s);
                    sets.len() - 1
                }
            }
        };
        for (li, l) in file.layers.iter().enumerate() {
            let mut models = Vec::new();
            for k in &l.kinds {
                let ids = kinds
                    .get(k.as_str())
                    .ok_or_else(|| format!("{COVER_JSON}: layer {li} names no kind '{k}'"))?;
                models.extend(ids);
            }
            let within = |b: &Beside| (b.within as i32).clamp(1, MAX_WITHIN);
            let layer = Layer {
                models,
                per_block: l.per_block.max(0.0),
                patch: l.patch,
                near: l
                    .near
                    .as_ref()
                    .map(|b| (set(&b.blocks, &mut cover.sets), within(b))),
                away: l
                    .away
                    .as_ref()
                    .map(|b| (set(&b.blocks, &mut cover.sets), within(b), b.keep)),
                sky: l.sky,
                far: l.far.clamp(0.0, 1.0),
                tint: l.tint,
                areas: l.areas.iter().map(|(k, a)| (k.clone(), *a)).collect(),
                elsewhere: l.elsewhere,
            };
            if layer.models.is_empty() {
                continue;
            }
            for b in l.on.iter().filter_map(|k| w::block_by_key(k)) {
                cover.by_block[b as usize].push(cover.layers.len() as u16);
            }
            cover.layers.push(layer);
        }
        Ok(cover)
    }
}

/// The faces of a voxel model that show, as quads: once voxel by voxel, and
/// once with each upright run of voxels as a single box whose colour runs
/// from its foot to its tip (a quarter of the triangles, for the distance).
/// The underside is never seen from above the ground.
fn build_model(m: &crate::vox::Model, palette: &[[u8; 4]; 256], voxel: f32, k: &KindDef) -> Model {
    use std::collections::HashMap;
    let cells: HashMap<[i32; 3], u8> = m.voxels.iter().copied().collect();
    let mut out = Model {
        lods: Default::default(),
    };
    let Some(lo) = m
        .voxels
        .iter()
        .map(|v| v.0)
        .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])])
    else {
        return out;
    };
    let hi = m
        .voxels
        .iter()
        .map(|v| v.0)
        .fold(lo, |a, b| [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]);
    // the root: the middle of the footprint, on the ground
    let mid = [(lo[0] + hi[0] + 1) / 2, lo[1], (lo[2] + hi[2] + 1) / 2];
    // (normal, the two edges of the face, brightness when lit evenly)
    type Face = ([i32; 3], [i32; 3], [i32; 3], f32);
    const FACES: [Face; 5] = [
        ([1, 0, 0], [0, 1, 0], [0, 0, 1], 0.86),
        ([-1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
        ([0, 1, 0], [0, 0, 1], [1, 0, 0], 1.0),
        ([0, 0, 1], [1, 0, 0], [0, 1, 0], 0.92),
        ([0, 0, -1], [0, 1, 0], [1, 0, 0], 0.76),
    ];
    let soft = k.soft.clamp(0.0, 1.0);
    // one face of a box of voxels from `p`, `tall` high: its colour runs
    // from `foot` at the bottom to `tip` at the top
    let quad =
        |mesh: &mut Mesh, p: [i32; 3], tall: i32, face: Face, foot: [u8; 4], tip: [u8; 4]| {
            let (n, u, v, even) = face;
            let size = [1, tall, 1];
            // the face's corner nearest the origin of the box
            let o = [0, 1, 2].map(|i| p[i] - mid[i] + n[i].max(0) * size[i]);
            let nf = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32);
            let nn = nf.lerp(Vec3::Y, soft).normalize_or(Vec3::Y);
            let shade = 1.0 + (even - 1.0) * soft;
            let first = mesh.verts.len() as u32;
            for (a, b) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
                let q = [0, 1, 2].map(|i| (o[i] + (a * u[i] + b * v[i]) * size[i]) as f32 * voxel);
                let rgb = if a * u[1] + b * v[1] > 0 || n[1] > 0 {
                    tip
                } else {
                    foot
                };
                mesh.verts.push(ModelVertex {
                    pos: q,
                    sway: k.sway * (q[1] * 2.5).min(2.0),
                    normal: [
                        (nn.x * 127.0) as i8,
                        (nn.y * 127.0) as i8,
                        (nn.z * 127.0) as i8,
                        0,
                    ],
                    color: [
                        (rgb[0] as f32 * shade) as u8,
                        (rgb[1] as f32 * shade) as u8,
                        (rgb[2] as f32 * shade) as u8,
                        255,
                    ],
                });
            }
            mesh.indices.extend([0, 1, 2, 0, 2, 3].map(|i| first + i));
        };
    let open =
        |p: [i32; 3], n: [i32; 3]| !cells.contains_key(&[p[0] + n[0], p[1] + n[1], p[2] + n[2]]);
    for (p, c) in &m.voxels {
        let rgb = palette[*c as usize];
        for face in FACES {
            if open(*p, face.0) {
                quad(&mut out.lods[0], *p, 1, face, rgb, rgb);
            }
        }
        // the foot of an upright run: the whole run as one box
        if cells.contains_key(&[p[0], p[1] - 1, p[2]]) {
            continue;
        }
        let mut tall = 1;
        while cells.contains_key(&[p[0], p[1] + tall, p[2]]) {
            tall += 1;
        }
        let tip = palette[cells[&[p[0], p[1] + tall - 1, p[2]]] as usize];
        for face in FACES {
            // a side shows if any voxel of the run is open on it
            let shows = if face.0[1] > 0 {
                true
            } else {
                (0..tall).any(|j| open([p[0], p[1] + j, p[2]], face.0))
            };
            if shows {
                quad(&mut out.lods[1], *p, tall, face, rgb, tip);
            }
        }
    }
    out
}

/// The cover in the pack now, read again only when the files it is made
/// from change (not when some other file of the pack arrives).
pub fn current() -> Arc<Cover> {
    struct Made {
        generation: u64,
        /// the files read, as they were
        files: Vec<(String, Option<Arc<[u8]>>)>,
        cover: Arc<Cover>,
    }
    static CACHE: Mutex<Option<Made>> = Mutex::new(None);
    let same = |a: &Option<Arc<[u8]>>, b: &Option<Arc<[u8]>>| match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    };
    let generation = assets::generation();
    let mut c = CACHE.lock().unwrap();
    if let Some(made) = c.as_mut()
        && (made.generation == generation
            || made.files.iter().all(|(p, b)| same(&assets::get(p), b)))
    {
        made.generation = generation;
        return made.cover.clone();
    }
    let json = assets::get(COVER_JSON);
    let text = json
        .as_deref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("{}");
    let mut files = vec![(COVER_JSON.to_string(), json.clone())];
    if let Ok(file) = serde_json::from_str::<CoverFile>(text) {
        for k in file.kinds.values() {
            files.push((k.file.clone(), assets::get(&k.file)));
        }
    }
    let cover = Arc::new(Cover::load(text).unwrap_or_else(|e| {
        eprintln!("{e}");
        Cover::empty()
    }));
    *c = Some(Made {
        generation,
        files,
        cover: cover.clone(),
    });
    cover
}

// ------------------------------------------------------------ areas

/// An area of the map, from the rules: its key, centre (x, z) and radius.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaAt {
    pub key: String,
    pub at: [f32; 2],
    pub radius: f32,
}

static AREAS: RwLock<Option<Arc<Vec<AreaAt>>>> = RwLock::new(None);

/// Tell the scatter where the areas of the map lie (when the rules arrive).
pub fn set_areas(list: Vec<AreaAt>) {
    let mut a = AREAS.write().unwrap();
    if a.as_deref() != Some(&list) {
        *a = Some(Arc::new(list));
    }
}

/// The areas of the rules, which place them from the spawn hearth.
pub fn set_areas_from(content: &w::content::Content, hearth: [i32; 3]) {
    set_areas(
        content
            .areas
            .iter()
            .map(|a| AreaAt {
                key: a.key.clone(),
                at: [
                    (hearth[0] + a.at[0]) as f32 + 0.5,
                    (hearth[2] + a.at[1]) as f32 + 0.5,
                ],
                radius: a.radius,
            })
            .collect(),
    );
}

pub(crate) fn areas() -> Arc<Vec<AreaAt>> {
    AREAS.read().unwrap().clone().unwrap_or_default()
}

/// The key of the smallest area a place lies in. The edge of an area is a
/// little ragged, so nothing that follows areas ends on a ruled line.
pub(crate) fn area_at(areas: &[AreaAt], x: f32, z: f32) -> Option<&str> {
    let ragged = (noise(x * 0.21, z * 0.21, 77) - 0.5) * 8.0;
    areas
        .iter()
        .filter(|a| (x - a.at[0]).hypot(z - a.at[1]) + ragged < a.radius)
        .min_by(|a, b| a.radius.total_cmp(&b.radius))
        .map(|a| a.key.as_str())
}

// ------------------------------------------------------------ scattering

/// An open top of the ground, as the mesher draws it: the block, where its
/// four corners lie (x0z0, x1z0, x1z1, x0z1) and the light over it.
#[derive(Clone, Copy)]
pub struct Top {
    /// section-local cell
    pub cell: [i8; 3],
    pub block: BlockId,
    pub corners: [[f32; 3]; 4],
    pub light: f32,
    pub sky: f32,
}

/// One plant or stone standing in the world.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Spot {
    pos: [f32; 3],
    model: u16,
    /// how far it shows, as a share of the range (255: all the way)
    reach: u8,
    /// dropped at lower quality when above the setting
    keep: u8,
    /// rgb at half scale (128 is 1.0), and the phase of its sway
    tint: [u8; 4],
    light: u8,
    sky: u8,
    turn: u8,
    pad: u8,
}

#[inline]
fn hash(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

#[inline]
fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / 16_777_216.0
}

/// Smooth noise, 0 to 1, about one feature per unit.
pub(crate) fn noise(x: f32, z: f32, seed: u32) -> f32 {
    let (xi, zi) = (x.floor(), z.floor());
    let (fx, fz) = (x - xi, z - zi);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let v = |dx: i32, dz: i32| {
        unit(hash(
            (xi as i32 + dx) as u32
                ^ hash((zi as i32 + dz) as u32 ^ seed.wrapping_mul(0x9E37_79B1)),
        ))
    };
    let a = v(0, 0) + (v(1, 0) - v(0, 0)) * sx;
    let b = v(0, 1) + (v(1, 1) - v(0, 1)) * sx;
    a + (b - a) * sz
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blocks to the nearest block of a set, sideways, looking from a block
/// below the top to two above it (the water beside a bank, the trunk that
/// stands on the ground). `MAX_WITHIN + 1` when there is none near.
fn beside(g: &Grid, c: [i32; 3], set: &[bool; 256]) -> i32 {
    let mut best = MAX_WITHIN + 1;
    for dy in -1..=2 {
        for dz in -MAX_WITHIN..=MAX_WITHIN {
            for dx in -MAX_WITHIN..=MAX_WITHIN {
                let d = dx.abs().max(dz.abs());
                if d < best && d > 0 && set[g.ids[at(c[0] + dx, c[1] + dy, c[2] + dz)] as usize] {
                    best = d;
                }
            }
        }
    }
    best
}

/// Grow the pack's cover on a section's open ground.
pub(crate) fn scatter(g: &Grid, tops: &[Top], out: &mut Vec<Spot>) {
    let cover = current();
    if cover.layers.is_empty() {
        return;
    }
    let areas = areas();
    let near_share = cover.near / cover.range;
    for t in tops {
        let layers = &cover.by_block[t.block as usize];
        if layers.is_empty() {
            continue;
        }
        let c = [t.cell[0] as i32, t.cell[1] as i32, t.cell[2] as i32];
        let (wx, wy, wz) = (g.o[0] + c[0], g.o[1] + c[1], g.o[2] + c[2]);
        let (fx, fz) = (wx as f32 + 0.5, wz as f32 + 0.5);
        let area = area_at(&areas, fx, fz);
        let mut dist = [-1i32; 8];
        let mut beside_set = |i: usize| {
            if i >= dist.len() {
                return beside(g, c, &cover.sets[i]);
            }
            if dist[i] < 0 {
                dist[i] = beside(g, c, &cover.sets[i]);
            }
            dist[i]
        };
        let cell = hash(
            (wx as u32).wrapping_mul(0x9E37_79B1)
                ^ (wy as u32).wrapping_mul(0x85EB_CA77)
                ^ (wz as u32).wrapping_mul(0xC2B2_AE3D),
        );
        for &li in layers {
            let l = &cover.layers[li as usize];
            if t.sky < l.sky[0] || t.sky > l.sky[1] {
                continue;
            }
            let mut density = l.per_block;
            let mut tint = [1.0f32; 3];
            match area.and_then(|k| l.areas.iter().find(|(a, _)| a == k)) {
                Some((_, rule)) => {
                    density *= rule.density;
                    tint = rule.tint;
                }
                None => density *= l.elsewhere,
            }
            if let Some(p) = l.patch {
                let n = noise(fx / p.scale, fz / p.scale, p.seed);
                density *= smoothstep(p.from, p.to, n);
            }
            if density <= 0.0 {
                continue;
            }
            if let Some((set, within)) = l.near {
                let d = beside_set(set);
                if d > within {
                    continue;
                }
                density *= 1.0 - (d - 1) as f32 / (within + 1) as f32;
            }
            if let Some((set, within, keep)) = l.away {
                let d = beside_set(set);
                if d <= within {
                    density *= keep + (1.0 - keep) * (d - 1) as f32 / within as f32;
                }
            }
            let seed = hash(cell ^ (li as u32 + 1).wrapping_mul(0x2545_F491));
            let count = density.floor() as u32 + (unit(seed) < density.fract()) as u32;
            for i in 0..count {
                let h = hash(seed ^ (i + 1).wrapping_mul(0x68E3_1DA4));
                let (h2, h3) = (hash(h ^ 0xA511_E9B3), hash(h ^ 0x1B87_3593));
                let (u, v) = (unit(h), unit(h2));
                let cs = t.corners.map(Vec3::from);
                let pos = cs[0].lerp(cs[1], u).lerp(cs[3].lerp(cs[2], u), v);
                let k = unit(h3);
                let mix = (h3 & 255) as f32 / 255.0;
                let rgb = [0, 1, 2].map(|j| {
                    let c = (l.tint[0][j] + (l.tint[1][j] - l.tint[0][j]) * mix) * tint[j];
                    (c * 128.0).clamp(0.0, 255.0) as u8
                });
                // each stands as far as its lot says: all of them near the
                // camera, the layer's `far` share at the edge of the range
                let reach = if k < l.far {
                    1.0
                } else {
                    near_share + (1.0 - k) / (1.0 - l.far).max(1e-3) * (1.0 - near_share)
                };
                out.push(Spot {
                    pos: pos.into(),
                    model: l.models[(h2 >> 12) as usize % l.models.len()],
                    reach: (reach * 255.0) as u8,
                    keep: (h >> 3) as u8,
                    tint: [rgb[0], rgb[1], rgb[2], (h2 >> 4) as u8],
                    light: (t.light * 255.0) as u8,
                    sky: (t.sky * 255.0) as u8,
                    turn: (h3 >> 9) as u8 & 3,
                    pad: 0,
                });
            }
        }
    }
}

// ------------------------------------------------------------ drawing

/// The vertex stage for cover, added to the world shader: it shares the
/// world's fragment stage, so cover is lit, shadowed, fogged and graded as
/// the ground is.
pub const VERTEX_SHADER: &str = r#"
struct CoverIn {
  @location(0) pos: vec3<f32>,
  @location(1) sway: f32,
  @location(2) normal: vec4<f32>,
  @location(3) color: vec4<f32>,
  // per instance: root and reach; tint (half scale) and phase; light, sky, turn
  @location(4) root: vec4<f32>,
  @location(5) tint: vec4<f32>,
  @location(6) misc: vec4<f32>,
};

@vertex fn vs_cover(v: CoverIn) -> VOut {
  var o: VOut;
  let root = v.root.xyz;
  // shrinks away into the ground at the end of its reach
  let fade = 1.0 - smoothstep(v.root.w * 0.75, v.root.w, distance(root, u.cam.xyz));
  let turn = i32(v.misc.z * 255.0 + 0.5);
  var p = v.pos;
  var n = v.normal.xyz;
  if ((turn & 1) == 1) { p = vec3<f32>(-p.z, p.y, p.x); n = vec3<f32>(-n.z, n.y, n.x); }
  if ((turn & 2) == 2) { p = vec3<f32>(-p.x, p.y, -p.z); n = vec3<f32>(-n.x, n.y, -n.z); }
  let t = u.cam.w;
  let ph = v.tint.w * 6.2832;
  let s = v.sway * (sin(t * 1.7 + ph + root.x * 0.6) * 0.07 + sin(t * 3.1 + ph * 1.7 + root.z * 0.8) * 0.03);
  let pos = root + (p + vec3<f32>(s, 0.0, s * 0.6)) * fade;
  o.clip = u.view_proj * vec4<f32>(pos, 1.0);
  o.wpos = pos;
  o.normal = n;
  o.color = vec4<f32>(pow(v.color.rgb, vec3<f32>(2.2)) * v.tint.rgb * (255.0 / 128.0), 1.0);
  o.light = v.misc.x;
  o.sky = v.misc.y;
  o.tex = vec4<f32>(0.0, 0.0, -2.0, 0.0);
  o.tone = world_tone(root.xz);
  return o;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    /// the root, and how far it shows in metres
    root: [f32; 4],
    tint: [u8; 4],
    /// light, sky, turn
    misc: [u8; 4],
}

/// Rebuild the instances when the camera has moved this far.
const STEP: f32 = 6.0;

/// The models on the graphics card and the instances near the camera.
pub struct Gpu {
    pipe: wgpu::RenderPipeline,
    /// the pack the models were built from
    cover: Arc<Cover>,
    models: Option<(wgpu::Buffer, wgpu::Buffer)>,
    /// per model and level of detail (model × 2 + level): first index, index
    /// count, base vertex
    ranges: Vec<(u32, u32, i32)>,
    instances: Option<wgpu::Buffer>,
    capacity: u64,
    /// per mesh drawn: its place in `ranges`, first instance, one past the last
    draws: Vec<(u32, u32, u32)>,
    /// where the camera was when the instances were made
    at: Option<Vec3>,
    /// 0: none; 1: the pack's density and range
    quality: f32,
    scratch: Vec<Instance>,
}

impl Gpu {
    pub fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        layout: &wgpu::PipelineLayout,
        format: wgpu::TextureFormat,
        depth: wgpu::TextureFormat,
    ) -> Gpu {
        let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cover"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_cover"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ModelVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Snorm8x4, 3 => Unorm8x4],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![4 => Float32x4, 5 => Unorm8x4, 6 => Unorm8x4],
                    },
                ],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        Gpu {
            pipe,
            cover: Arc::new(Cover::empty()),
            models: None,
            ranges: Vec::new(),
            instances: None,
            capacity: 0,
            draws: Vec::new(),
            at: None,
            quality: 1.0,
            scratch: Vec::new(),
        }
    }

    /// Set how much cover is drawn: the player's setting (0: none; 1: the
    /// pack's density and range), unless `DYNO_COVER` says otherwise (tools).
    pub fn set_quality(&mut self, setting: f32) {
        let q = std::env::var("DYNO_COVER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(setting);
        if q != self.quality {
            self.quality = q;
            self.changed();
        }
    }

    /// The ground near the camera changed: make the instances again.
    pub fn changed(&mut self) {
        self.at = None;
    }

    /// A section's ground changed: make the instances again if any of them
    /// could stand in it.
    pub fn changed_in(&mut self, section: [i32; 3]) {
        let n = crate::render::SECTION as f32;
        let reach = self.range() + STEP + n;
        if let Some(at) = self.at {
            let mid = [(section[0] as f32 + 0.5) * n, (section[2] as f32 + 0.5) * n];
            if (mid[0] - at.x).abs() < reach && (mid[1] - at.z).abs() < reach {
                self.at = None;
            }
        }
    }

    /// How far from the camera cover is drawn, in metres.
    pub fn range(&self) -> f32 {
        current().range * self.quality.clamp(0.0, 2.0).sqrt()
    }

    /// Whether the instances must be made again for a camera here.
    pub fn stale(&self, cam: Vec3) -> bool {
        !Arc::ptr_eq(&self.cover, &current())
            || self
                .at
                .is_none_or(|at| at.distance_squared(cam) > STEP * STEP)
    }

    /// Make the instances for a camera at `cam` from the spots of the
    /// sections round it.
    pub fn rebuild<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cam: Vec3,
        sections: impl Iterator<Item = &'a [Spot]>,
    ) {
        use wgpu::util::DeviceExt;
        let t0 = std::time::Instant::now();
        let cover = current();
        if !Arc::ptr_eq(&self.cover, &cover) {
            // the pack changed: the models again
            let (mut verts, mut indices) = (Vec::new(), Vec::new());
            self.ranges.clear();
            for m in cover.models.iter().flat_map(|m| &m.lods) {
                self.ranges.push((
                    indices.len() as u32,
                    m.indices.len() as u32,
                    verts.len() as i32,
                ));
                verts.extend_from_slice(&m.verts);
                indices.extend_from_slice(&m.indices);
            }
            self.models = (!indices.is_empty()).then(|| {
                let make = |contents: &[u8], usage| {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("cover models"),
                        contents,
                        usage,
                    })
                };
                (
                    make(bytemuck::cast_slice(&verts), wgpu::BufferUsages::VERTEX),
                    make(bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX),
                )
            });
            self.cover = cover.clone();
        }
        self.at = Some(cam);
        self.draws.clear();
        let quality = self.quality.clamp(0.0, 2.0);
        let range = self.range();
        if self.models.is_none() || quality <= 0.0 {
            return;
        }
        let keep = (quality * 255.0).min(255.0) as u8;
        // count the instances of each model, then lay them out model by model
        let mut starts = vec![0u32; self.ranges.len() + 1];
        let mut picked: Vec<(u32, Instance)> = Vec::new();
        let detail = cover.detail * cover.detail;
        for spots in sections {
            for s in spots {
                if s.keep > keep || s.model as usize * 2 + 1 >= self.ranges.len() {
                    continue;
                }
                let reach = s.reach as f32 / 255.0 * range;
                let d = Vec3::from(s.pos).distance_squared(cam);
                // those that would come into reach before the next rebuild too
                if d > (reach + STEP) * (reach + STEP) {
                    continue;
                }
                // voxel by voxel near the camera, column by column beyond
                let mesh = s.model as u32 * 2 + (d > detail) as u32;
                starts[mesh as usize + 1] += 1;
                picked.push((
                    mesh,
                    Instance {
                        root: [s.pos[0], s.pos[1], s.pos[2], reach],
                        tint: s.tint,
                        misc: [s.light, s.sky, s.turn, 0],
                    },
                ));
            }
        }
        for i in 1..starts.len() {
            starts[i] += starts[i - 1];
        }
        self.scratch.clear();
        self.scratch.resize(picked.len(), Instance::zeroed());
        let mut next = starts.clone();
        for (m, inst) in picked {
            let slot = &mut next[m as usize];
            self.scratch[*slot as usize] = inst;
            *slot += 1;
        }
        for m in 0..self.ranges.len() {
            if starts[m + 1] > starts[m] {
                self.draws.push((m as u32, starts[m], starts[m + 1]));
            }
        }
        if self.scratch.is_empty() {
            return;
        }
        let bytes: &[u8] = bytemuck::cast_slice(&self.scratch);
        if self.instances.is_none() || self.capacity < bytes.len() as u64 {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            self.instances = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cover instances"),
                size: self.capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(b) = &self.instances {
            queue.write_buffer(b, 0, bytes);
        }
        if std::env::var_os("DYNO_SHOT_STATS").is_some() {
            let (n, tris) = self.drawn();
            eprintln!(
                "cover: {n} instances, {tris} triangles, made in {:.2} ms",
                t0.elapsed().as_secs_f64() * 1e3
            );
        }
    }

    /// Instances and triangles drawn a frame (for measurements).
    pub fn drawn(&self) -> (u32, u64) {
        let mut tris = 0u64;
        for (m, a, b) in &self.draws {
            tris += (self.ranges[*m as usize].1 / 3) as u64 * (*b - *a) as u64;
        }
        (self.scratch.len() as u32, tris)
    }

    /// Draw the cover. The world's bind group must be set.
    pub fn draw(&self, rp: &mut wgpu::RenderPass<'_>) {
        let (Some((verts, indices)), Some(instances)) = (&self.models, &self.instances) else {
            return;
        };
        if self.draws.is_empty() {
            return;
        }
        rp.set_pipeline(&self.pipe);
        rp.set_vertex_buffer(0, verts.slice(..));
        rp.set_vertex_buffer(1, instances.slice(..));
        rp.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
        for (m, a, b) in &self.draws {
            let (first, count, base) = self.ranges[*m as usize];
            rp.draw_indexed(first..first + count, base, *a..*b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pack's cover file reads, every kind it names has models, and
    /// every block and kind a rule names exists.
    #[test]
    fn the_packs_cover_reads() {
        // (loads the whole pack, whatever other tests put in the store first)
        crate::shapes::pack();
        let text = assets::text(COVER_JSON).expect("the pack has cover.json");
        let file: CoverFile = serde_json::from_str(&text).unwrap();
        let cover = Cover::load(&text).unwrap();
        assert!(!cover.models.is_empty() && !cover.layers.is_empty());
        assert_eq!(
            cover.layers.len(),
            file.layers.len(),
            "a layer has no models"
        );
        for (name, k) in &file.kinds {
            let bytes = assets::get(&k.file).unwrap_or_else(|| panic!("{name}: no {}", k.file));
            assert!(
                !crate::vox::read(&bytes).unwrap().models.is_empty(),
                "{name}"
            );
        }
        for l in &file.layers {
            let beside = l.near.iter().chain(&l.away).flat_map(|b| &b.blocks);
            for key in l.on.iter().chain(beside) {
                assert!(
                    w::block_by_key(key).is_some(),
                    "{}: no block '{key}'",
                    l.name
                );
            }
            assert!(!l.on.is_empty() && !l.kinds.is_empty(), "{}", l.name);
        }
        for m in &cover.models {
            let [full, lite] = &m.lods;
            assert!(!full.indices.is_empty() && full.verts.len() < 60_000);
            assert!(!lite.indices.is_empty() && lite.indices.len() <= full.indices.len());
        }
    }

    #[test]
    fn noise_stays_in_range_and_varies() {
        let v: Vec<f32> = (0..200)
            .map(|i| noise(i as f32 * 0.37, i as f32 * 0.91, 3))
            .collect();
        assert!(v.iter().all(|n| (0.0..=1.0).contains(n)));
        let (lo, hi) = v
            .iter()
            .fold((1f32, 0f32), |(l, h), n| (l.min(*n), h.max(*n)));
        assert!(lo < 0.25 && hi > 0.75, "{lo} {hi}");
    }
}
