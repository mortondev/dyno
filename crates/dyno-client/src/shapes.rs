//! Block tags and detail shapes, from the pack's `blocks.json` and the
//! `.vox` files in `shapes/`.
//!
//! The world keeps its 1 m block grid (storage, light, collision); only the
//! picture changes. A block with a `shape` is drawn as fine voxels (16 a
//! block) instead of a cube. Which voxels is data: the block's rule set
//! picks a *class* (a slope, a post, a pillar...) and a quarter turn from the
//! blocks around it, and each of the class's *parts* picks a model from a
//! table keyed by a few neighbour tests ("features"). Conditions only read
//! blocks within a cell or two, so every section decides a shared cell the
//! same way and section borders stay seamless.
//!
//! `blocks.json`:
//!
//! ```json
//! { "blocks": [ { "id": 49, "key": "red_roof_tiles", "textures": {...},
//!                  "tags": ["roof", "chamfer"],
//!                  "shape": { "rules": "roof", "vox": "shapes/roof_red.vox" } } ],
//!   "shape_rules": { "roof": { "models": ["rows.0", ...], "classes": [
//!       { "class": "slope", "fills": false, "turns": [1, 3, 0, 2], "pick": "best",
//!         "score": [[2, {"at": [0, 1, 1], "is": "roof"}], ...], "min": 1,
//!         "parts": [ { "name": "rows", "features": [cond, ...],
//!                      "table": { "0110": { "model": "rows.3" }, "0000": null } } ] } ] } },
//!   "shape_files": { "shapes/roof_red.vox": { "palette": {
//!       "1": { "block": "self", "faces": [1, 1, 1], "tint": [1, 1, 1] },
//!       "2": { "at": [0, -1, 0], "faces": [1, 1, 1] } } } } }
//! ```
//!
//! A condition is `{"at": [dx, dy, dz], "is": what}` (a tag, `self`,
//! `solid`, `opaque` or `class:<name>`), or `{"all": [...]}`,
//! `{"any": [...]}`, `{"not": c}`, `true`. Offsets are in the turned frame:
//! local +z is the way the class faces. A palette entry is a material: a
//! block's face texture (`faces` for top, sides and bottom) with a tint;
//! `at` takes the block at that offset instead (a gable wall is "the block
//! below"), and `if`/`else` choose between two.
//!
//! How the voxels are drawn is data too:
//!
//! - **Slopes.** Where a model's voxels of one colour step diagonally (a
//!   45° run of convex corners, two or more long), the mesher draws one
//!   sloped polygon across the run instead of the steps, merged greedily
//!   along the slope and its length; faces across the run are cut to match.
//!   A palette entry with `"slope": false` keeps its steps (the lip of a
//!   course of shingles).
//! - **Texture.** By default a face takes its texture by its place in the
//!   cell (slopes: along the slope, a repeat per block of rise). A palette
//!   entry with `"uv": "fit"` stretches the texture over that colour's
//!   bounds in the model (a crate's planks, a door).
//! - **Round parts.** A part with `"round": {"axis": 1, "sides": 8, "wrap":
//!   2, "vscale": 1, "phase": 0, "flat": false, "only": [colours]}` is drawn
//!   as a solid of revolution with the voxels' cross-section area, slice by
//!   slice (trunks, posts, barrels, pillars); `wrap` is texture repeats
//!   round it, `vscale` repeats per block along it, `flat` gives each side
//!   one normal. Its ends are capped unless they reach the cell's side.
//! - **Variation per place.** A class's `"spin": {"salt", "steps", "step"}`
//!   turns it by `(hash(x, salt, z) % steps) * step` radians (a twisted
//!   trunk); a part's `"lift"` (same form, in voxels) raises it (flowers
//!   standing higher in one bed than the next). `jitter` is a small turn.
//! - **Joins.** A part with `"joins": true` drops its faces on the cell's
//!   sides where the next cell is the same block in the same class and turn
//!   (the inside of a long roof).

use crate::light::Light;
use crate::render::{self, Vertex};
use crate::terrain_mesh::{DX, DY, DZ, F_DETAIL, F_OPAQUE, Glow, Grid, SectionGeo, at, hash3};
use dyno_world::{self as w, BlockId};
use glam::Vec3;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

const N: i32 = render::SECTION;
/// Voxels per block in a shape.
pub const FINE: f32 = 16.0;

// ------------------------------------------------------------------- json

#[derive(Deserialize)]
struct FileJson {
    blocks: Vec<BlockJson>,
    #[serde(default)]
    shape_rules: BTreeMap<String, RuleSetJson>,
    #[serde(default)]
    shape_files: BTreeMap<String, ShapeFileJson>,
    /// fine voxel terrain (`fine.rs`)
    #[serde(default)]
    terrain: crate::fine::TerrainJson,
}

#[derive(Deserialize)]
struct BlockJson {
    id: u16,
    key: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    shape: Option<ShapeRefJson>,
}

#[derive(Deserialize)]
struct ShapeRefJson {
    rules: String,
    vox: String,
}

#[derive(Deserialize)]
struct RuleSetJson {
    models: Vec<String>,
    classes: Vec<ClassJson>,
}

fn turn0() -> Vec<u8> {
    vec![0]
}

#[derive(Deserialize)]
struct ClassJson {
    class: String,
    #[serde(default)]
    fills: bool,
    #[serde(default = "turn0")]
    turns: Vec<u8>,
    #[serde(default)]
    pick: String,
    #[serde(default)]
    when: Option<CondJson>,
    #[serde(default)]
    score: Vec<(f32, CondJson)>,
    #[serde(default)]
    min: f32,
    #[serde(default)]
    jitter: f32,
    #[serde(default)]
    spin: Option<VaryJson>,
    #[serde(default)]
    parts: Vec<PartJson>,
}

#[derive(Deserialize)]
struct PartJson {
    #[allow(dead_code)]
    name: String,
    features: Vec<CondJson>,
    table: BTreeMap<String, Option<EntryJson>>,
    #[serde(default)]
    round: Option<RoundJson>,
    #[serde(default)]
    lift: Option<VaryJson>,
    /// its faces on the cell's sides are hidden by a neighbour of the same
    /// block, class and turn (a shape that runs on from cell to cell)
    #[serde(default)]
    joins: bool,
}

#[derive(Deserialize, Clone)]
struct EntryJson {
    model: String,
    #[serde(default)]
    glow: Option<[f32; 3]>,
}

#[derive(Deserialize)]
struct ShapeFileJson {
    palette: BTreeMap<String, MatJson>,
}

fn one3() -> [f32; 3] {
    [1.0; 3]
}
fn side3() -> [u8; 3] {
    [1; 3]
}

#[derive(Deserialize, Clone)]
struct MatJson {
    #[serde(default)]
    block: Option<String>,
    #[serde(default)]
    at: Option<[i32; 3]>,
    #[serde(default = "side3")]
    faces: [u8; 3],
    #[serde(default = "one3")]
    tint: [f32; 3],
    #[serde(default)]
    glow: bool,
    #[serde(default)]
    clear: bool,
    #[serde(default, rename = "if")]
    cond: Option<CondJson>,
    #[serde(default, rename = "else")]
    other: Option<Box<MatJson>>,
    /// `"fit"`: the texture stretched over the colour's bounds in a model
    #[serde(default)]
    uv: Option<String>,
    /// `false`: voxels of this colour keep their steps instead of joining
    /// a slope
    #[serde(default)]
    slope: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub(crate) enum CondJson {
    At { at: [i32; 3], is: String },
    All { all: Vec<CondJson> },
    Any { any: Vec<CondJson> },
    Not { not: Box<CondJson> },
    Bool(bool),
}

// --------------------------------------------------------------- compiled

#[derive(Clone, Debug)]
enum Cond {
    Const(bool),
    Tag([i32; 3], u32),
    Same([i32; 3]),
    Solid([i32; 3]),
    Opaque([i32; 3]),
    Class([i32; 3], u16),
    All(Vec<Cond>),
    Any(Vec<Cond>),
    Not(Box<Cond>),
}

/// A material: whose texture, which faces, how tinted.
#[derive(Clone, Debug)]
enum Src {
    /// the block being drawn
    Own,
    Block(BlockId),
    At([i32; 3]),
}

#[derive(Clone, Debug)]
struct Mat {
    src: Src,
    faces: [u8; 3],
    tint: [f32; 3],
    glow: bool,
    clear: bool,
    cond: Option<(Cond, Box<Mat>)>,
}

#[derive(Clone)]
struct Entry {
    model: u16,
    glow: Option<[f32; 3]>,
}

struct Part {
    features: Vec<Cond>,
    table: HashMap<u32, Option<Entry>>,
    /// `table` for every pattern, when there are few features
    dense: Vec<Option<Entry>>,
    lift: Option<VaryJson>,
    joins: bool,
}

struct Class {
    id: u16,
    fills: bool,
    turns: Vec<u8>,
    best: bool,
    when: Option<Cond>,
    score: Vec<(f32, Cond)>,
    min: f32,
    jitter: f32,
    spin: Option<VaryJson>,
    parts: Vec<Part>,
}

struct RuleSet {
    models: Vec<String>,
    /// per model: drawn round (the part it belongs to says so)
    round: Vec<Option<RoundJson>>,
    classes: Vec<Class>,
}

struct Models {
    /// by the rule set's model index
    models: Vec<Vec<Quad>>,
    palette: Vec<Option<Mat>>,
}

pub(crate) struct Pack {
    tags: Box<[u32; 256]>,
    tag_names: Vec<String>,
    /// rule set and shape file of each block with a shape
    shape: Box<[Option<(u16, u16)>; 256]>,
    sets: Vec<RuleSet>,
    files: Vec<(String, u16, OnceLock<Models>)>,
    file_palettes: BTreeMap<String, BTreeMap<String, MatJson>>,
    names: HashMap<String, BlockId>,
    classes: Vec<String>,
    /// the meshers' flags of each block (`terrain_mesh::F_*`)
    pub(crate) flags: [u8; 256],
    /// how natural ground is drawn as fine voxels
    pub(crate) terrain: crate::fine::Params,
}

/// Tag bits the engine itself reads.
pub(crate) mod tag {
    pub const TERRAIN: u32 = 1;
    pub const PAVING: u32 = 2;
    pub const CHAMFER: u32 = 4;
    pub const ON_PAVING: u32 = 8;
}
const ENGINE_TAGS: [&str; 4] = ["terrain", "paving", "chamfer", "on_paving"];

fn compile(c: &CondJson, p: &mut Pack) -> Cond {
    match c {
        CondJson::Bool(b) => Cond::Const(*b),
        CondJson::All { all } => Cond::All(all.iter().map(|c| compile(c, p)).collect()),
        CondJson::Any { any } => Cond::Any(any.iter().map(|c| compile(c, p)).collect()),
        CondJson::Not { not } => Cond::Not(Box::new(compile(not, p))),
        CondJson::At { at, is } => match is.as_str() {
            "self" => Cond::Same(*at),
            "solid" => Cond::Solid(*at),
            "opaque" => Cond::Opaque(*at),
            s if s.starts_with("class:") => Cond::Class(*at, p.class_id(&s[6..])),
            t => Cond::Tag(*at, p.tag_bit(t)),
        },
    }
}

impl Pack {
    fn empty() -> Pack {
        let mut p = Pack {
            tags: Box::new([0; 256]),
            tag_names: Vec::new(),
            shape: Box::new([None; 256]),
            sets: Vec::new(),
            files: Vec::new(),
            file_palettes: BTreeMap::new(),
            names: HashMap::new(),
            classes: vec![String::new()],
            flags: [0; 256],
            terrain: crate::fine::Params::default(),
        };
        for t in ENGINE_TAGS {
            p.tag_bit(t);
        }
        p
    }

    fn tag_bit(&mut self, t: &str) -> u32 {
        let i = match self.tag_names.iter().position(|n| n == t) {
            Some(i) => i,
            None => {
                if self.tag_names.len() >= 32 {
                    return 0;
                }
                self.tag_names.push(t.to_string());
                self.tag_names.len() - 1
            }
        };
        1 << i
    }

    /// A class's name, for measuring.
    #[cfg(test)]
    pub(crate) fn class_name(&self, id: u16) -> String {
        self.classes.get(id as usize).cloned().unwrap_or_default()
    }

    fn class_id(&mut self, name: &str) -> u16 {
        match self.classes.iter().position(|n| n == name) {
            Some(i) => i as u16,
            None => {
                self.classes.push(name.to_string());
                (self.classes.len() - 1) as u16
            }
        }
    }

    fn mat(&mut self, m: &MatJson) -> Mat {
        let src = match (&m.at, m.block.as_deref()) {
            (Some(at), _) => Src::At(*at),
            (None, Some("self") | None) => Src::Own,
            (None, Some(name)) => Src::Block(self.names.get(name).copied().unwrap_or(w::AIR)),
        };
        let cond = match (&m.cond, &m.other) {
            (Some(c), Some(o)) => Some((compile(c, self), Box::new(self.mat(o)))),
            _ => None,
        };
        Mat {
            src,
            faces: m.faces,
            tint: m.tint,
            glow: m.glow,
            clear: m.clear,
            cond,
        }
    }

    fn parse(text: &str) -> Result<Pack, String> {
        let f: FileJson = serde_json::from_str(text).map_err(|e| format!("blocks.json: {e}"))?;
        let mut p = Pack::empty();
        for b in &f.blocks {
            if b.id < 256 {
                p.names.insert(b.key.clone(), b.id as BlockId);
            }
        }
        for b in &f.blocks {
            let id = b.id as usize;
            if id >= 256 {
                continue;
            }
            for t in &b.tags {
                p.tags[id] |= p.tag_bit(t);
            }
        }
        p.terrain = crate::fine::Params::compile(&f.terrain, &p.names);
        let mut set_index = HashMap::new();
        for (name, s) in &f.shape_rules {
            let mut classes = Vec::new();
            let mut round = vec![None; s.models.len()];
            for c in &s.classes {
                let id = p.class_id(&c.class);
                let when = c.when.as_ref().map(|w| compile(w, &mut p));
                let score = c
                    .score
                    .iter()
                    .map(|(k, w)| (*k, compile(w, &mut p)))
                    .collect();
                let mut parts = Vec::new();
                for part in &c.parts {
                    let features = part.features.iter().map(|f| compile(f, &mut p)).collect();
                    let mut table = HashMap::new();
                    for (bits, e) in &part.table {
                        let key = bits
                            .bytes()
                            .enumerate()
                            .fold(0u32, |k, (i, ch)| k | (u32::from(ch == b'1') << i));
                        let e = e.as_ref().and_then(|e| {
                            Some(Entry {
                                model: s.models.iter().position(|m| *m == e.model)? as u16,
                                glow: e.glow,
                            })
                        });
                        table.insert(key, e);
                    }
                    // every pattern of a few features, looked up directly;
                    // one never seen when baked takes the nearest that was
                    let nearest = |bits: u32| {
                        table
                            .iter()
                            .min_by_key(|(k, _)| ((*k ^ bits).count_ones(), **k))
                            .and_then(|(_, e)| e.clone())
                    };
                    let dense: Vec<Option<Entry>> = if part.features.len() <= 12 {
                        (0..1u32 << part.features.len())
                            .map(|b| match table.get(&b) {
                                Some(e) => e.clone(),
                                None => nearest(b),
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    if let Some(r) = &part.round {
                        for e in part.table.values().flatten() {
                            if let Some(m) = s.models.iter().position(|m| *m == e.model) {
                                round[m] = Some(r.clone());
                            }
                        }
                    }
                    parts.push(Part {
                        features,
                        table,
                        dense,
                        lift: part.lift,
                        joins: part.joins,
                    });
                }
                classes.push(Class {
                    id,
                    fills: c.fills,
                    turns: c.turns.clone(),
                    best: c.pick == "best",
                    when,
                    score,
                    min: c.min,
                    jitter: c.jitter,
                    spin: c.spin,
                    parts,
                });
            }
            set_index.insert(name.clone(), p.sets.len() as u16);
            p.sets.push(RuleSet {
                models: s.models.clone(),
                round,
                classes,
            });
        }
        p.file_palettes = f
            .shape_files
            .iter()
            .map(|(k, v)| (k.clone(), v.palette.clone()))
            .collect();
        for b in &f.blocks {
            let (Some(s), true) = (&b.shape, b.id < 256) else {
                continue;
            };
            let Some(set) = set_index.get(&s.rules).copied() else {
                continue;
            };
            let file = match p
                .files
                .iter()
                .position(|(v, k, _)| *v == s.vox && *k == set)
            {
                Some(i) => i,
                None => {
                    p.files.push((s.vox.clone(), set, OnceLock::new()));
                    p.files.len() - 1
                }
            };
            p.shape[b.id as usize] = Some((set, file as u16));
        }
        p.flags = crate::terrain_mesh::flags_with(
            |b| p.shape[b as usize].is_some(),
            |b| p.tags[b as usize] & tag::TERRAIN != 0,
        );
        Ok(p)
    }

    /// The meshed models of a shape file, read from the pack on first use.
    fn models(&self, file: u16) -> &Models {
        let (path, set, cell) = &self.files[file as usize];
        cell.get_or_init(|| {
            let set = &self.sets[*set as usize];
            let vox = crate::assets::get(path).and_then(|b| crate::vox::read(&b).ok());
            let pal = self.file_palettes.get(path);
            let fit: Vec<u8> = pal
                .map(|p| {
                    p.iter()
                        .filter(|(_, m)| m.uv.as_deref() == Some("fit"))
                        .filter_map(|(k, _)| k.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
            // colours that keep their steps (a course of shingles)
            let mut steps = [false; 256];
            for (k, m) in pal.into_iter().flatten() {
                if let (Ok(i), Some(false)) = (k.parse::<usize>(), m.slope) {
                    steps[i.min(255)] = true;
                }
            }
            let models = set
                .models
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let Some(m) = vox.as_ref().and_then(|v| v.model(name)) else {
                        return Vec::new();
                    };
                    // fitted colours span their own bounds in the model
                    let mut uv = [Uv::Cell; 256];
                    for k in &fit {
                        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                        for (p, c) in &m.voxels {
                            if c == k {
                                for a in 0..3 {
                                    lo[a] = lo[a].min(p[a] as f32);
                                    hi[a] = hi[a].max(p[a] as f32 + 1.0);
                                }
                            }
                        }
                        if lo[0] <= hi[0] {
                            uv[*k as usize] = Uv::Fit(lo, hi);
                        }
                    }
                    mesh_model(&m.voxels, &uv, &steps, set.round[i].as_ref())
                })
                .collect();
            let mut palette = vec![None; 256];
            // palettes compile against the block names; a throwaway pack
            // does the compiling so `self` stays shared
            let mut scratch = Pack::empty();
            scratch.names = self.names.clone();
            scratch.tag_names = self.tag_names.clone();
            scratch.classes = self.classes.clone();
            if let Some(pal) = self.file_palettes.get(path) {
                for (k, m) in pal {
                    if let Ok(i) = k.parse::<usize>()
                        && i < 256
                    {
                        palette[i] = Some(scratch.mat(m));
                    }
                }
            }
            Models { models, palette }
        })
    }
}

/// The pack's block table and shapes.
///
/// Read again when the asset store changes (a pack arriving from the
/// server); each thread keeps the one it last saw, so asking is cheap.
pub(crate) fn pack() -> &'static Pack {
    thread_local! {
        static LOCAL: std::cell::Cell<Option<(u64, &'static Pack)>> =
            const { std::cell::Cell::new(None) };
    }
    let g = crate::assets::generation();
    if let Some((seen, p)) = LOCAL.with(|l| l.get())
        && seen == g
    {
        return p;
    }
    static CURRENT: std::sync::Mutex<Option<(u64, &'static Pack)>> = std::sync::Mutex::new(None);
    let mut cur = CURRENT.lock().unwrap_or_else(|e| e.into_inner());
    crate::assets::load_local_pack();
    // tests share one store, and another may have put a file in it first
    #[cfg(test)]
    if crate::assets::get("blocks.json").is_none() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let _ = crate::assets::load_dir(&dir);
    }
    let g = crate::assets::generation();
    let text = crate::assets::get("blocks.json");
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.as_deref().hash(&mut h);
        h.finish()
    };
    let p = match *cur {
        Some((h, p)) if h == hash => p,
        _ => {
            let parsed = match text
                .as_deref()
                .map(|t| Pack::parse(&String::from_utf8_lossy(t)))
            {
                Some(Ok(p)) => p,
                Some(Err(e)) => {
                    eprintln!("{e}");
                    Pack::empty()
                }
                None => Pack::empty(),
            };
            // packs change rarely (once a join); the old one may still be
            // in use on another thread, so it is kept
            let p: &'static Pack = Box::leak(Box::new(parsed));
            *cur = Some((hash, p));
            p
        }
    };
    LOCAL.with(|l| l.set(Some((g, p))));
    p
}

/// Whether a block carries one of the engine's tags ([`tag`]).
#[inline]
pub(crate) fn has(b: BlockId, t: u32) -> bool {
    pack().tags[b as usize] & t != 0
}

/// Blocks drawn by the shape mesher.
#[cfg(test)]
fn is_detail(b: BlockId) -> bool {
    pack().shape[b as usize].is_some()
}

// ---------------------------------------------------------------- deciding

/// A local offset turned by `q` quarter turns into a world step.
#[inline]
fn turn(d: [i32; 3], q: u8) -> [i32; 3] {
    match q & 3 {
        0 => d,
        1 => [d[2], d[1], -d[0]],
        2 => [-d[0], d[1], -d[2]],
        _ => [-d[2], d[1], d[0]],
    }
}

#[inline]
fn step(c: usize, d: [i32; 3]) -> usize {
    (c as isize
        + d[0] as isize * DX as isize
        + d[1] as isize * DY as isize
        + d[2] as isize * DZ as isize) as usize
}

struct Ctx<'a> {
    p: &'a Pack,
    g: &'a Grid,
    c: usize,
    q: u8,
}

impl Ctx<'_> {
    fn eval(&self, k: &Cond) -> bool {
        let n = |d: &[i32; 3]| step(self.c, turn(*d, self.q));
        match k {
            Cond::Const(b) => *b,
            Cond::Tag(d, m) => self.p.tags[self.g.ids[n(d)] as usize] & m != 0,
            Cond::Same(d) => self.g.ids[n(d)] == self.g.ids[self.c],
            Cond::Solid(d) => w::is_solid(self.g.ids[n(d)]),
            Cond::Opaque(d) => self.g.fl[n(d)] & F_OPAQUE != 0,
            Cond::Class(d, id) => self.g.cls.get(n(d)).is_some_and(|c| (*c >> 2) == *id),
            Cond::All(v) => v.iter().all(|k| self.eval(k)),
            Cond::Any(v) => v.iter().any(|k| self.eval(k)),
            Cond::Not(k) => !self.eval(k),
        }
    }
}

/// The class (and quarter turn) a shaped block takes here.
fn decide(p: &Pack, g: &Grid, c: usize, set: &RuleSet) -> Option<(usize, u8)> {
    for (i, cl) in set.classes.iter().enumerate() {
        if cl.best {
            let mut best: Option<(f32, u8)> = None;
            for &q in &cl.turns {
                let x = Ctx { p, g, c, q };
                if cl.when.as_ref().is_some_and(|k| !x.eval(k)) {
                    continue;
                }
                let s: f32 = cl
                    .score
                    .iter()
                    .filter(|(_, k)| x.eval(k))
                    .map(|(w, _)| w)
                    .sum();
                if best.is_none_or(|b| s > b.0) {
                    best = Some((s, q));
                }
            }
            if let Some((s, q)) = best
                && s >= cl.min
            {
                return Some((i, q));
            }
        } else {
            for &q in &cl.turns {
                let x = Ctx { p, g, c, q };
                if cl.when.as_ref().is_none_or(|k| x.eval(k)) {
                    return Some((i, q));
                }
            }
        }
    }
    None
}

/// Decide each shaped block's class for the cells the section uses, and
/// which of them still fill their cell (the rest stop hiding their
/// neighbours' faces). Called on a freshly loaded grid.
pub(crate) fn classify(g: &mut Grid) {
    classify_with(pack(), g)
}

/// [`classify`] against a given pack.
pub(crate) fn classify_with(p: &Pack, g: &mut Grid) {
    g.cls = vec![0; g.ids.len()];
    let mut thin = Vec::new();
    for y in -3..N + 3 {
        for z in -3..N + 3 {
            for x in -3..N + 3 {
                let c = at(x, y, z);
                if g.fl[c] & F_DETAIL == 0 {
                    continue;
                }
                let Some((set, _)) = p.shape[g.ids[c] as usize] else {
                    continue;
                };
                let set = &p.sets[set as usize];
                if let Some((i, q)) = decide(p, g, c, set) {
                    let cl = &set.classes[i];
                    g.cls[c] = (cl.id << 2) | q as u16;
                    if !cl.fills {
                        thin.push(c);
                    }
                }
            }
        }
    }
    for c in thin {
        g.fl[c] &= !F_OPAQUE;
    }
}

// ----------------------------------------------------------------- meshing

/// A meshed shape model: polygons in voxels of the cell's local frame.
struct Quad {
    p: [[f32; 3]; 4],
    /// normal at each corner (round parts are smooth)
    n: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    /// three corners, not four
    tri: bool,
    /// which of the material's faces: 0 top, 1 sides, 2 bottom
    tex: u8,
    pal: u8,
    /// the cell face (0 +x, 1 -x, 2 +y, 3 -y, 4 +z, 5 -z) it lies on,
    /// facing out: hidden when the cell beside is opaque; 6 for none
    cover: u8,
}

/// A part drawn as a solid of revolution (`"round"` in a part).
#[derive(Deserialize, Clone, Debug)]
struct RoundJson {
    /// 0 x, 1 y, 2 z
    #[serde(default = "axis_y")]
    axis: usize,
    sides: usize,
    /// texture repeats round it
    #[serde(default = "one")]
    wrap: f32,
    /// texture repeats per block along it
    #[serde(default = "one")]
    vscale: f32,
    /// only these colours (all when empty)
    #[serde(default)]
    only: Vec<u8>,
    /// turn of the first corner, degrees
    #[serde(default)]
    phase: f32,
    /// one normal per side (a post), not smooth (a log)
    #[serde(default)]
    flat: bool,
}

fn axis_y() -> usize {
    1
}
fn one() -> f32 {
    1.0
}

/// A choice per world column: `(hash(x, salt, z) % steps) * step`.
#[derive(Deserialize, Clone, Copy, Debug)]
struct VaryJson {
    salt: i32,
    steps: u32,
    step: f32,
}

impl VaryJson {
    fn at(&self, wp: [i32; 3]) -> f32 {
        (hash3(wp[0], self.salt, wp[2]) % self.steps.max(1)) as f32 * self.step
    }
}

/// How a colour's texture is laid on: by place in the cell (`Cell`), or
/// stretched over the colour's own bounds in the model (`Fit`).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Uv {
    Cell,
    Fit([f32; 3], [f32; 3]),
}

/// Texture coordinates of a point on a face, by its place in the cell
/// (sides run down from the top, as on cubes). `p` in blocks.
fn map_uv(p: Vec3, n: Vec3) -> [f32; 2] {
    let a = n.abs();
    if a.y >= a.x && a.y >= a.z {
        [p.x, p.z]
    } else if a.x >= a.z {
        [if n.x > 0.0 { 1.0 - p.z } else { p.z }, 1.0 - p.y]
    } else {
        [if n.z < 0.0 { 1.0 - p.x } else { p.x }, 1.0 - p.y]
    }
}

fn tex_of(n: Vec3) -> u8 {
    if n.y > 0.5 {
        0
    } else if n.y < -0.5 {
        2
    } else {
        1
    }
}

/// Fewest convex corners in a row along a diagonal that make a slope.
const MIN_RUN: i32 = 2;

/// The voxels of a model in a box, with a border of empty space.
struct Vox {
    lo: [i32; 3],
    size: [i32; 3],
    cells: Vec<u8>,
}

impl Vox {
    fn new(voxels: &[([i32; 3], u8)]) -> Vox {
        let (mut lo, mut hi) = ([i32::MAX; 3], [i32::MIN; 3]);
        for (p, _) in voxels {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a] - 1);
                hi[a] = hi[a].max(p[a] + 2);
            }
        }
        let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let mut v = Vox {
            lo,
            size,
            cells: vec![0; (size[0] * size[1] * size[2]).max(0) as usize],
        };
        for (p, c) in voxels {
            if let Some(i) = v.idx(*p) {
                v.cells[i] = (*c).max(1);
            }
        }
        v
    }

    #[inline]
    fn idx(&self, p: [i32; 3]) -> Option<usize> {
        let r = [p[0] - self.lo[0], p[1] - self.lo[1], p[2] - self.lo[2]];
        (r.iter().zip(self.size).all(|(v, s)| (0..s).contains(v)))
            .then(|| ((r[1] * self.size[2] + r[2]) * self.size[0] + r[0]) as usize)
    }

    #[inline]
    fn get(&self, p: [i32; 3]) -> u8 {
        self.idx(p).map(|i| self.cells[i]).unwrap_or(0)
    }
}

#[inline]
fn add(p: [i32; 3], a: usize, s: i32) -> [i32; 3] {
    let mut q = p;
    q[a] += s;
    q
}

/// A slope orientation: the plane of axes `a` < `b` it lies across, and
/// which way it faces on each (+1 or -1). The extrusion axis is the third.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Orient {
    a: usize,
    b: usize,
    sa: i32,
    sb: i32,
}

const fn orients() -> [Orient; 12] {
    let mut out = [Orient {
        a: 0,
        b: 0,
        sa: 0,
        sb: 0,
    }; 12];
    let pairs = [(0, 1), (0, 2), (1, 2)];
    let mut i = 0;
    let mut k = 0;
    while k < 3 {
        let mut s = 0;
        while s < 4 {
            out[i] = Orient {
                a: pairs[k].0,
                b: pairs[k].1,
                sa: if s & 1 == 0 { 1 } else { -1 },
                sb: if s & 2 == 0 { 1 } else { -1 },
            };
            i += 1;
            s += 1;
        }
        k += 1;
    }
    out
}

const ORIENTS: [Orient; 12] = orients();

impl Orient {
    fn e(&self) -> usize {
        3 - self.a - self.b
    }

    /// A voxel whose two faces this way are open.
    fn open(&self, v: &Vox, p: [i32; 3]) -> bool {
        v.get(p) != 0 && v.get(add(p, self.a, self.sa)) == 0 && v.get(add(p, self.b, self.sb)) == 0
    }

    /// The next voxel along the slope.
    fn next(&self, p: [i32; 3], k: i32) -> [i32; 3] {
        add(add(p, self.a, -self.sa * k), self.b, self.sb * k)
    }

    /// Face indices (0 +x, 1 -x, ...) of the two faces the slope replaces.
    fn faces(&self) -> [usize; 2] {
        [
            self.a * 2 + usize::from(self.sa < 0),
            self.b * 2 + usize::from(self.sb < 0),
        ]
    }
}

/// Mesh a model: slopes across diagonal runs of voxels, then the faces
/// left, merged into as few polygons as their outlines allow. Round parts
/// become solids of revolution.
fn mesh_model(
    voxels: &[([i32; 3], u8)],
    uv: &[Uv; 256],
    steps: &[bool; 256],
    round: Option<&RoundJson>,
) -> Vec<Quad> {
    let mut out = Vec::new();
    let (lathe, rest): (Vec<_>, Vec<_>) = match round {
        Some(r) => voxels
            .iter()
            .partition(|(_, c)| r.only.is_empty() || r.only.contains(c)),
        None => (Vec::new(), voxels.iter().collect()),
    };
    if let Some(r) = round {
        let mut by: BTreeMap<u8, Vec<[i32; 3]>> = BTreeMap::new();
        for (p, c) in lathe {
            by.entry(*c).or_default().push(*p);
        }
        for (c, ps) in by {
            lathe_model(&ps, c, r, &mut out);
        }
    }
    let rest: Vec<([i32; 3], u8)> = rest.into_iter().copied().collect();
    if rest.is_empty() {
        return out;
    }
    let v = Vox::new(&rest);
    // faces the slopes took, per voxel (bits by face index)
    let mut taken = vec![0u8; v.cells.len()];
    // the slope through each voxel, per orientation (bit i of ORIENTS)
    let mut sloped = vec![0u16; v.cells.len()];
    let pos = |i: usize| -> [i32; 3] {
        let s = v.size;
        let i = i as i32;
        [
            v.lo[0] + i % s[0],
            v.lo[1] + i / (s[0] * s[2]),
            v.lo[2] + (i / s[0]) % s[2],
        ]
    };
    for (oi, o) in ORIENTS.iter().enumerate() {
        let [fa, fb] = o.faces();
        for i in 0..v.cells.len() {
            if v.cells[i] == 0 {
                continue;
            }
            let p = pos(i);
            // a slope runs over voxels of one colour
            let k = v.cells[i];
            let open = |p: [i32; 3]| o.open(&v, p) && v.get(p) == k && !steps[k as usize];
            if !open(p) {
                continue;
            }
            let run = (1..MIN_RUN).take_while(|k| open(o.next(p, *k))).count()
                + (1..MIN_RUN).take_while(|k| open(o.next(p, -k))).count()
                + 1;
            if (run as i32) < MIN_RUN {
                continue;
            }
            if taken[i] & (1 << fa | 1 << fb) != 0 {
                continue;
            }
            taken[i] |= 1 << fa | 1 << fb;
            sloped[i] |= 1 << oi;
        }
    }
    slopes(&v, &sloped, uv, &mut out);
    faces(&v, &taken, &sloped, uv, &mut out);
    out
}

fn uv_at(mode: Uv, p: Vec3, n: Vec3) -> [f32; 2] {
    match mode {
        Uv::Cell => map_uv(p / FINE, n),
        Uv::Fit(lo, hi) => {
            let (lo, hi) = (Vec3::from(lo), Vec3::from(hi));
            map_uv((p - lo) / (hi - lo).max(Vec3::splat(1e-3)), n)
        }
    }
}

/// The sloped polygons: per orientation and line, a grid of (place along
/// the line, place along the extrusion) merged greedily.
fn slopes(v: &Vox, sloped: &[u16], uv: &[Uv; 256], out: &mut Vec<Quad>) {
    for (oi, o) in ORIENTS.iter().enumerate() {
        let e = o.e();
        // units by line: (low end along a, along e, colour)
        let mut lines: BTreeMap<i32, Vec<(i32, i32, u8)>> = BTreeMap::new();
        for (i, s) in sloped.iter().enumerate() {
            if s & (1 << oi) == 0 {
                continue;
            }
            let sz = v.size;
            let ii = i as i32;
            let p = [
                v.lo[0] + ii % sz[0],
                v.lo[1] + ii / (sz[0] * sz[2]),
                v.lo[2] + (ii / sz[0]) % sz[2],
            ];
            // the segment from P (far end of the a face) to Q
            let pa = p[o.a] + i32::from(o.sa > 0);
            let pb = p[o.b] + i32::from(o.sb < 0);
            // b = c - sa * sb * a along the line
            let c = pb + o.sa * o.sb * pa;
            lines.entry(c).or_default().push((p[o.a], p[e], v.cells[i]));
        }
        for (c, units) in lines {
            let (mut a0, mut a1, mut e0, mut e1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            for (a, e, _) in &units {
                a0 = a0.min(*a);
                a1 = a1.max(*a);
                e0 = e0.min(*e);
                e1 = e1.max(*e);
            }
            let (na, ne) = (a1 - a0 + 1, e1 - e0 + 1);
            let mut mask = vec![0u8; (na * ne) as usize];
            for (a, e, k) in &units {
                mask[((e - e0) * na + (a - a0)) as usize] = *k;
            }
            let normal = {
                let mut n = Vec3::ZERO;
                n[o.a] = o.sa as f32;
                n[o.b] = o.sb as f32;
                n.normalize()
            };
            let b_at = |a: i32| (c - o.sa * o.sb * a) as f32;
            greedy(&mut mask, na, ne, |u, w, wv, hv, k| {
                let (ga0, ga1) = (a0 + u, a0 + u + wv);
                let (ge0, ge1) = (e0 + w, e0 + w + hv);
                let corner = |a: i32, ee: i32| {
                    let mut p = Vec3::ZERO;
                    p[o.a] = a as f32;
                    p[o.b] = b_at(a);
                    p[e] = ee as f32;
                    p
                };
                let ps = [
                    corner(ga0, ge0),
                    corner(ga1, ge0),
                    corner(ga1, ge1),
                    corner(ga0, ge1),
                ];
                let uvs = ps.map(|p| match uv[k as usize] {
                    Uv::Cell if o.a == 1 || o.b == 1 => [p[e] / FINE, -p.y / FINE],
                    m => uv_at(m, p, normal),
                });
                out.push(Quad {
                    p: ps.map(|p| p.into()),
                    n: [normal.into(); 4],
                    uv: uvs,
                    tri: false,
                    tex: tex_of(normal),
                    pal: k,
                    cover: 6,
                });
            });
        }
    }
}

/// Greedy rectangles of equal values in a `nu` × `nv` mask (cleared as it
/// goes): `f(u, v, width, height, value)`.
fn greedy(mask: &mut [u8], nu: i32, nv: i32, mut f: impl FnMut(i32, i32, i32, i32, u8)) {
    for v in 0..nv {
        let mut u = 0;
        while u < nu {
            let k = mask[(v * nu + u) as usize];
            if k == 0 {
                u += 1;
                continue;
            }
            let mut wd = 1;
            while u + wd < nu && mask[(v * nu + u + wd) as usize] == k {
                wd += 1;
            }
            let mut ht = 1;
            'grow: while v + ht < nv {
                for i in 0..wd {
                    if mask[((v + ht) * nu + u + i) as usize] != k {
                        break 'grow;
                    }
                }
                ht += 1;
            }
            for dv in 0..ht {
                for du in 0..wd {
                    mask[((v + dv) * nu + u + du) as usize] = 0;
                }
            }
            f(u, v, wd, ht, k);
            u += wd;
        }
    }
}

// corner cuts of a face cell, in the face's (u, v)
const BL: u8 = 1;
const BR: u8 = 2;
const TL: u8 = 4;
const TR: u8 = 8;

/// One row's piece of a face: from cell `first` to `last`, its bottom
/// edge from `b0` to `b1` and top edge from `t0` to `t1` (in u).
#[derive(Clone, Copy)]
struct Run {
    last: i32,
    b0: i32,
    b1: i32,
    t0: i32,
    t1: i32,
}

/// The axis-aligned faces left after the slopes, per plane: rows of cells
/// with their cut corners become trapezoids, and rows that continue one
/// another (straight or along a slope) merge.
fn faces(v: &Vox, taken: &[u8], sloped: &[u16], uv: &[Uv; 256], out: &mut Vec<Quad>) {
    let full = FINE as i32;
    let get = |p: [i32; 3]| v.get(p);
    for d in 0..3usize {
        // u is the lower other axis, v the higher
        let (ua, va) = match d {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let (nu, nv) = (v.size[ua], v.size[va]);
        let mut col = vec![0u8; (nu * nv) as usize];
        let mut cut = vec![0u8; (nu * nv) as usize];
        let mut seen = vec![false; (nu * nv) as usize];
        for dir in [1i32, -1] {
            let face = d * 2 + usize::from(dir < 0);
            for s in 0..v.size[d] {
                let mut any = false;
                for w in 0..nv {
                    for u in 0..nu {
                        let mut p = [0; 3];
                        p[d] = v.lo[d] + s;
                        p[ua] = v.lo[ua] + u;
                        p[va] = v.lo[va] + w;
                        let i = (w * nu + u) as usize;
                        col[i] = 0;
                        cut[i] = 0;
                        seen[i] = false;
                        let Some(vi) = v.idx(p) else { continue };
                        let k = v.cells[vi];
                        if k == 0 || taken[vi] & (1 << face) != 0 {
                            continue;
                        }
                        // the corners slopes across this plane cut off a voxel
                        let corners = |vi: usize| {
                            let mut c = 0;
                            for (oi, o) in ORIENTS.iter().enumerate() {
                                if sloped[vi] & (1 << oi) == 0 || o.e() != d {
                                    continue;
                                }
                                // the convex corner, in (u, v)
                                let (cu, cv) = if o.a == ua {
                                    (o.sa > 0, o.sb > 0)
                                } else {
                                    (o.sb > 0, o.sa > 0)
                                };
                                c |= match (cu, cv) {
                                    (false, false) => BL,
                                    (true, false) => BR,
                                    (false, true) => TL,
                                    (true, true) => TR,
                                };
                            }
                            c
                        };
                        let mut c = corners(vi);
                        let q = add(p, d, dir);
                        if get(q) != 0 {
                            // the voxel beside covers this face, but for
                            // the corner its own slope cut away
                            let qc = v.idx(q).map(corners).unwrap_or(0);
                            if qc == 0 {
                                continue;
                            }
                            for (x, opp) in [(BL, TR), (TR, BL), (BR, TL), (TL, BR)] {
                                if qc & x != 0 {
                                    c |= opp;
                                }
                            }
                        }
                        col[i] = k;
                        cut[i] = c;
                        any = true;
                    }
                }
                if !any {
                    continue;
                }
                let plane = v.lo[d] + s + i32::from(dir > 0);
                let edge = (dir > 0 && plane == full) || (dir < 0 && plane == 0);
                let normal = {
                    let mut n = Vec3::ZERO;
                    n[d] = dir as f32;
                    n
                };
                // a row's piece starting at cell `u`, ending by `limit`
                let run = |u: i32, w: i32, limit: i32, seen: &[bool]| -> Option<Run> {
                    let at = |u: i32| (w * nu + u) as usize;
                    if u < 0 || u >= nu || col[at(u)] == 0 || seen[at(u)] {
                        return None;
                    }
                    let k = col[at(u)];
                    let c0 = cut[at(u)];
                    let (b0, t0) = match c0 & (BL | TL) {
                        0 => (u, u),
                        BL => (u + 1, u),
                        TL => (u, u + 1),
                        _ => return None,
                    };
                    let mut last = u;
                    if c0 & (BR | TR) == 0 {
                        while last < limit.min(nu - 1) {
                            let n = at(last + 1);
                            if col[n] != k || seen[n] || cut[n] & (BL | TL) != 0 {
                                break;
                            }
                            last += 1;
                            if cut[n] & (BR | TR) != 0 {
                                break;
                            }
                        }
                    }
                    let (b1, t1) = match cut[at(last)] & (BR | TR) {
                        0 => (last + 1, last + 1),
                        BR => (last, last + 1),
                        TR => (last + 1, last),
                        _ => return None,
                    };
                    Some(Run {
                        last,
                        b0,
                        b1,
                        t0,
                        t1,
                    })
                };
                for w in 0..nv {
                    let mut u = 0;
                    while u < nu {
                        let Some(r) = run(u, w, nu, &seen) else {
                            let i = (w * nu + u) as usize;
                            if col[i] != 0 && !seen[i] {
                                // a cell cut on both sides of one end: drop it
                                seen[i] = true;
                            }
                            u += 1;
                            continue;
                        };
                        let k = col[(w * nu + u) as usize];
                        for x in u..=r.last {
                            seen[(w * nu + x) as usize] = true;
                        }
                        let (dl, dr) = (r.t0 - r.b0, r.t1 - r.b1);
                        // grow upwards while the next row continues it
                        let (mut top, mut t0, mut t1) = (w + 1, r.t0, r.t1);
                        if r.b0 < r.b1 && r.t0 < r.t1 {
                            while top < nv {
                                let start = if dl > 0 {
                                    t0
                                } else if dl < 0 {
                                    t0 - 1
                                } else {
                                    t0
                                };
                                let lim = match dr {
                                    0 => t1 - 1,
                                    1 => t1,
                                    _ => t1 - 1,
                                };
                                let Some(n) = run(start, top, lim, &seen) else {
                                    break;
                                };
                                if col[(top * nu + start) as usize] != k
                                    || n.b0 != t0
                                    || n.b1 != t1
                                    || n.t0 - n.b0 != dl
                                    || n.t1 - n.b1 != dr
                                    || n.t0 >= n.t1
                                {
                                    break;
                                }
                                for x in start..=n.last {
                                    seen[(top * nu + x) as usize] = true;
                                }
                                t0 = n.t0;
                                t1 = n.t1;
                                top += 1;
                            }
                        }
                        let cf = |cu: f32, cv: f32| {
                            let mut p = Vec3::ZERO;
                            p[d] = plane as f32;
                            p[ua] = v.lo[ua] as f32 + cu;
                            p[va] = v.lo[va] as f32 + cv;
                            p
                        };
                        let (mut b0, mut b1) = (r.b0 as f32, r.b1 as f32);
                        let (mut tt0, mut tt1) = (t0 as f32, t1 as f32);
                        let (mut botv, mut topv) = (w as f32, top as f32);
                        // the sides cross inside the row: that end is a point
                        let cross = (b1 - b0) / ((b1 - b0) - (tt1 - tt0));
                        if tt0 > tt1 {
                            let x = b0 + (tt0 - b0) * cross;
                            (tt0, tt1) = (x, x);
                            topv = botv + cross * (topv - botv);
                        } else if b0 > b1 {
                            let x = b0 + (tt0 - b0) * cross;
                            (b0, b1) = (x, x);
                            botv += cross * (topv - botv);
                        }
                        let mut ps = Vec::with_capacity(4);
                        ps.push(cf(b0, botv));
                        if b1 > b0 {
                            ps.push(cf(b1, botv));
                        }
                        ps.push(cf(tt1, topv));
                        if tt1 > tt0 {
                            ps.push(cf(tt0, topv));
                        }
                        if ps.len() >= 3 {
                            let tri = ps.len() == 3;
                            let mut p4 = [[0.0; 3]; 4];
                            let mut uv4 = [[0.0; 2]; 4];
                            for (j, p) in ps.iter().enumerate() {
                                p4[j] = (*p).into();
                                uv4[j] = uv_at(uv[k as usize], *p, normal);
                            }
                            if tri {
                                p4[3] = p4[2];
                                uv4[3] = uv4[2];
                            }
                            out.push(Quad {
                                p: p4,
                                n: [normal.into(); 4],
                                uv: uv4,
                                tri,
                                tex: tex_of(normal),
                                pal: k,
                                cover: if edge { face as u8 } else { 6 },
                            });
                        }
                        u = r.last + 1;
                    }
                }
            }
        }
    }
}

/// Points of a profile kept by Douglas-Peucker within `tol`.
fn simplify(pts: &[(f32, f32)], tol: f32) -> Vec<(f32, f32)> {
    fn go(pts: &[(f32, f32)], tol: f32, out: &mut Vec<(f32, f32)>) {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (mut worst, mut at) = (0.0, 0);
        for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
            let d = ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / len;
            if d > worst {
                worst = d;
                at = i;
            }
        }
        if worst > tol {
            go(&pts[..=at], tol, out);
            go(&pts[at..], tol, out);
        } else {
            out.push(b);
        }
    }
    let mut out = vec![pts[0]];
    if pts.len() > 1 {
        go(pts, tol, &mut out);
    }
    out
}

/// A solid of revolution with the same cross-section area as the voxels,
/// slice by slice, about their centre line.
fn lathe_model(ps: &[[i32; 3]], k: u8, r: &RoundJson, out: &mut Vec<Quad>) {
    use std::f32::consts::TAU;
    let ax = r.axis.min(2);
    let (ua, va) = match ax {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    let (mut cu, mut cv) = (0.0f32, 0.0f32);
    for p in ps {
        lo = lo.min(p[ax]);
        hi = hi.max(p[ax]);
        cu += p[ua] as f32 + 0.5;
        cv += p[va] as f32 + 0.5;
    }
    let n = ps.len().max(1) as f32;
    let (cu, cv) = (cu / n, cv / n);
    let mut count = vec![0u32; (hi - lo + 1).max(0) as usize];
    for p in ps {
        count[(p[ax] - lo) as usize] += 1;
    }
    // a few stray voxels at an end are not part of the solid
    let most = count.iter().copied().max().unwrap_or(0);
    while count.len() > 1 && count[0] * 4 < most {
        count.remove(0);
        lo += 1;
    }
    while count.len() > 1 && count[count.len() - 1] * 4 < most {
        count.pop();
        hi -= 1;
    }
    let rad = |c: u32| (c as f32 / std::f32::consts::PI).sqrt();
    let mut prof = vec![(lo as f32, rad(count[0]))];
    for (i, c) in count.iter().enumerate() {
        prof.push((lo as f32 + i as f32 + 0.5, rad(*c)));
    }
    prof.push(((hi + 1) as f32, rad(count[count.len() - 1])));
    let prof = simplify(&prof, 0.45);
    let segs = r.sides.clamp(3, 32);
    let place = |a: f32, rr: f32, t: f32| -> Vec3 {
        let mut p = Vec3::ZERO;
        p[ax] = a;
        p[ua] = cu + rr * t.cos();
        p[va] = cv + rr * t.sin();
        p
    };
    let radial = |t: f32| -> Vec3 {
        let mut d = Vec3::ZERO;
        d[ua] = t.cos();
        d[va] = t.sin();
        d
    };
    let mut axis_dir = Vec3::ZERO;
    axis_dir[ax] = 1.0;
    let phase = r.phase.to_radians();
    let t_at = |s: usize| phase + TAU * s as f32 / segs as f32;
    let u_at = |s: usize| r.wrap * s as f32 / segs as f32;
    for wnd in prof.windows(2) {
        let ((a0, r0), (a1, r1)) = (wnd[0], wnd[1]);
        if r0 <= 0.0 && r1 <= 0.0 {
            continue;
        }
        let len = ((a1 - a0).powi(2) + (r1 - r0).powi(2)).sqrt().max(1e-5);
        let (na, nr) = ((r0 - r1) / len, (a1 - a0).abs() / len);
        let sgn = if a1 >= a0 { 1.0 } else { -1.0 };
        let nrm = |t: f32| (radial(t) * nr + axis_dir * na * sgn).normalize_or_zero();
        for s in 0..segs {
            let (t0, t1) = (t_at(s), t_at(s + 1));
            let p = [
                place(a0, r0, t0),
                place(a0, r0, t1),
                place(a1, r1, t1),
                place(a1, r1, t0),
            ];
            let n = if r.flat {
                [nrm((t0 + t1) * 0.5); 4]
            } else {
                [nrm(t0), nrm(t1), nrm(t1), nrm(t0)]
            };
            let vk = r.vscale / FINE;
            let uv = [
                [u_at(s), -a0 * vk],
                [u_at(s + 1), -a0 * vk],
                [u_at(s + 1), -a1 * vk],
                [u_at(s), -a1 * vk],
            ];
            let tri = r0 <= 0.0 || r1 <= 0.0;
            let (p, n, uv) = if r0 <= 0.0 {
                // a point at the start: keep 0, 2, 3
                (
                    [p[0], p[2], p[3], p[3]],
                    [n[0], n[2], n[3], n[3]],
                    [uv[0], uv[2], uv[3], uv[3]],
                )
            } else {
                (p, n, uv)
            };
            out.push(Quad {
                p: p.map(|v| v.into()),
                n: n.map(|v| v.into()),
                uv,
                tri,
                tex: 1,
                pal: k,
                cover: 6,
            });
        }
    }
    // end caps: fans across the texture
    for end in 0..2 {
        let (a, rr) = if end == 0 {
            prof[0]
        } else {
            prof[prof.len() - 1]
        };
        if rr <= 0.0 {
            continue;
        }
        let nn = if end == 0 { -axis_dir } else { axis_dir };
        // an end at or past the cell's side runs on into the next cell
        if a <= 0.0 || a >= FINE {
            continue;
        }
        let tex = if ax == 1 {
            if end == 0 { 2 } else { 0 }
        } else {
            1
        };
        let disc = |s: usize| {
            let t = t_at(s) - phase;
            [0.5 + 0.5 * t.cos(), 0.5 + 0.5 * t.sin()]
        };
        for s in 1..segs - 1 {
            out.push(Quad {
                p: [
                    place(a, rr, t_at(0)),
                    place(a, rr, t_at(s)),
                    place(a, rr, t_at(s + 1)),
                    place(a, rr, t_at(s + 1)),
                ]
                .map(|v| v.into()),
                n: [nn.into(); 4],
                uv: [disc(0), disc(s), disc(s + 1), disc(s + 1)],
                tri: true,
                tex,
                pal: k,
                cover: 6,
            });
        }
    }
}

/// Light and occlusion of a cell, worked out when first wanted.
#[derive(Clone, Copy, Default)]
struct Lit {
    bl: u8,
    sl: u8,
    /// how much the cell closes in what is beside it: 0 open, 1 a shaped
    /// block, 2 a full cube
    occ: u8,
    known: bool,
}

const OCC: [f32; 3] = [0.0, 0.3, 1.0];

struct Shade<'a> {
    g: &'a Grid,
    light: &'a Light,
    field: Vec<std::cell::Cell<Lit>>,
}

impl Shade<'_> {
    fn cell(&self, x: i32, y: i32, z: i32) -> Lit {
        let c = &self.field[at(x, y, z)];
        let l = c.get();
        if l.known {
            return l;
        }
        let fl = self.g.fl[at(x, y, z)];
        let l = if fl & F_OPAQUE != 0 {
            Lit {
                occ: 2,
                known: true,
                ..Lit::default()
            }
        } else {
            let (bl, sl) = self.g.light(self.light, [x, y, z]);
            Lit {
                bl: bl as u8,
                sl: sl as u8,
                occ: u8::from(fl & F_DETAIL != 0 && fl & crate::terrain_mesh::F_DARK != 0),
                known: true,
            }
        };
        c.set(l);
        l
    }

    /// Block light, sky light and occlusion just off a surface at `p` with
    /// normal `n` (world), blended over the eight cells around the point.
    fn at(&self, p: Vec3, n: Vec3) -> (f32, f32, f32) {
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
                    let l = self.cell(cx, cy, cz);
                    occ += wgt * OCC[l.occ as usize];
                    if l.occ < 2 {
                        bl += l.bl as f32 * wgt;
                        sl += l.sl as f32 * wgt;
                        wsum += wgt;
                    }
                }
            }
        }
        let (bl, sl) = if wsum > 0.01 {
            (bl / wsum, sl / wsum)
        } else {
            let c = (p + n * 0.9 - o).floor();
            let l = self.cell(lim(c.x), lim(c.y), lim(c.z));
            (l.bl as f32, l.sl as f32)
        };
        let ao = (1.0 - occ * 0.55).clamp(0.45, 1.0);
        (bl / 15.0, sl / 15.0, ao)
    }
}

/// The world step of each face direction, in the cell's local frame.
const STEPS: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// A texture layer, its animation, the tint and how it is drawn.
#[derive(Clone, Copy)]
struct Look {
    layer: u16,
    anim: f32,
    tint: [f32; 3],
    glow: bool,
    clear: bool,
}

fn resolve(m: &Mat, x: &Ctx, face: usize) -> Look {
    if let Some((k, other)) = &m.cond
        && !x.eval(k)
    {
        return resolve(other, x, face);
    }
    let b = match &m.src {
        Src::Own => x.g.ids[x.c],
        Src::Block(b) => *b,
        Src::At(d) => x.g.ids[step(x.c, turn(*d, x.q))],
    };
    let layer = render::textures().face(b, m.faces[face.min(2)] as usize);
    Look {
        layer,
        anim: render::layer_anim(layer),
        tint: m.tint,
        glow: m.glow,
        clear: m.clear,
    }
}

#[cfg(test)]
thread_local! {
    /// (block, class) -> (cells, vertices) drawn, for measuring
    pub(crate) static STATS: std::cell::RefCell<HashMap<(BlockId, u16), (usize, usize)>> = Default::default();
}

/// Draw the section's own shaped blocks.
pub(crate) fn mesh(g: &Grid, light: &Light, geo: &mut SectionGeo) {
    let p = pack();
    let shade = Shade {
        g,
        light,
        field: vec![std::cell::Cell::new(Lit::default()); g.fl.len()],
    };
    for y in 0..N {
        for z in 0..N {
            for x in 0..N {
                let c = at(x, y, z);
                if g.fl[c] & F_DETAIL == 0 || g.cls[c] >> 2 == 0 {
                    continue;
                }
                let b = g.ids[c];
                let Some((set_i, file)) = p.shape[b as usize] else {
                    continue;
                };
                let set = &p.sets[set_i as usize];
                let id = g.cls[c] >> 2;
                let q = (g.cls[c] & 3) as u8;
                let Some(cl) = set.classes.iter().find(|k| k.id == id) else {
                    continue;
                };
                if cl.parts.is_empty() {
                    continue;
                }
                let wp = [g.o[0] + x, g.o[1] + y, g.o[2] + z];
                #[cfg(test)]
                let before = geo.opaque.len() + geo.trans.len();
                emit(p, g, &shade, c, wp, cl, q, file, b, geo);
                #[cfg(test)]
                STATS.with(|s| {
                    let mut s = s.borrow_mut();
                    let e = s.entry((b, cl.id)).or_insert((0, 0));
                    e.0 += 1;
                    e.1 += geo.opaque.len() + geo.trans.len() - before;
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit(
    p: &Pack,
    g: &Grid,
    shade: &Shade,
    c: usize,
    wp: [i32; 3],
    cl: &Class,
    q: u8,
    file: u16,
    b: BlockId,
    geo: &mut SectionGeo,
) {
    let x = Ctx { p, g, c, q };
    let base = Vec3::new(wp[0] as f32, wp[1] as f32, wp[2] as f32);
    // a small turn per column, the same for a stack
    let mut turn_by = if cl.jitter != 0.0 {
        let h = hash3(wp[0], 11, wp[2]);
        ((h & 1023) as f32 / 1023.0 - 0.5) * 0.24 * cl.jitter
    } else {
        0.0
    };
    if let Some(s) = &cl.spin {
        turn_by += s.at(wp);
    }
    let angle = q as f32 * std::f32::consts::FRAC_PI_2 + turn_by;
    let (sin, cos) = if turn_by == 0.0 {
        [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)][q as usize & 3]
    } else {
        angle.sin_cos()
    };
    let dir = |v: Vec3| Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos);
    let centre = base + Vec3::new(0.5, 0.0, 0.5);
    let models = p.models(file);
    let mut looks: Vec<(usize, [Look; 3])> = Vec::with_capacity(8);
    // light by where (to a quarter block) and which way a polygon faces:
    // the polygons of one cell share a few
    let mut lights: Vec<([i8; 6], (f32, f32, f32))> = Vec::with_capacity(16);
    // which of the cell's own faces an opaque cell beside it covers
    let mut covered = [false; 7];
    // and which a cell of the same shape continues
    let mut joined = [false; 7];
    if turn_by == 0.0 {
        for (f, s) in STEPS.iter().enumerate() {
            let n = step(c, turn(*s, q));
            covered[f] = g.fl[n] & F_OPAQUE != 0;
            joined[f] = covered[f] || (g.ids[n] == g.ids[c] && g.cls.get(n) == Some(&g.cls[c]));
        }
    }
    for part in &cl.parts {
        let mut bits = 0u32;
        for (i, f) in part.features.iter().enumerate() {
            if x.eval(f) {
                bits |= 1 << i;
            }
        }
        let entry = match part
            .dense
            .get(bits as usize)
            .or_else(|| part.table.get(&bits))
        {
            Some(e) => e.as_ref(),
            // never seen when baked: the nearest pattern that was
            None => part
                .table
                .iter()
                .min_by_key(|(k, _)| ((*k ^ bits).count_ones(), **k))
                .and_then(|(_, e)| e.as_ref()),
        };
        let Some(entry) = entry else { continue };
        let lift = part.lift.map(|v| v.at(wp) / FINE).unwrap_or(0.0);
        let place = |v: Vec3| centre + dir(v - Vec3::new(0.5, -lift, 0.5));
        if let Some(gl) = entry.glow {
            let at = place(Vec3::from(gl));
            geo.glows.push(Glow {
                pos: at.into(),
                color: w::block(b).color,
                size: 0.9 + w::emission(b) as f32 * 0.06,
            });
        }
        let Some(quads) = models.models.get(entry.model as usize) else {
            continue;
        };
        let covered = if part.joins { &joined } else { &covered };
        for quad in quads {
            if covered[quad.cover as usize] {
                continue;
            }
            let pal = quad.pal as usize;
            let look = match looks.iter().find(|l| l.0 == pal) {
                Some(l) => l.1,
                None => {
                    let Some(Some(m)) = models.palette.get(pal) else {
                        continue;
                    };
                    let l = [0, 1, 2].map(|f| resolve(m, &x, f));
                    looks.push((pal, l));
                    l
                }
            };
            let look = look[quad.tex as usize];
            let world = quad.p.map(|v| place(Vec3::from(v) / FINE));
            let normals = quad.n.map(|n| dir(Vec3::from(n)));
            let k = if quad.tri { 3 } else { 4 };
            let lit = if look.glow {
                (0.0, 0.0, 1.0)
            } else {
                let mid = if quad.tri {
                    (world[0] + world[1] + world[2]) / 3.0
                } else {
                    (world[0] + world[2]) * 0.5
                };
                let n = (normals[0] + normals[2]).normalize_or_zero();
                let r = (mid - base) * 4.0;
                let m = n * 4.0;
                let key = [r.x, r.y, r.z, m.x, m.y, m.z].map(|v| v.round() as i8);
                match lights.iter().find(|l| l.0 == key) {
                    Some(l) => l.1,
                    None => {
                        let l = shade.at(mid, n);
                        lights.push((key, l));
                        l
                    }
                }
            };
            let vs: [Vertex; 4] = std::array::from_fn(|i| Vertex {
                pos: world[i].into(),
                normal: normals[i].into(),
                color: [
                    look.tint[0],
                    look.tint[1],
                    look.tint[2],
                    if look.glow { 2.0 } else { lit.2 },
                ],
                light: lit.0,
                sky: lit.1,
                tex: [quad.uv[i][0], quad.uv[i][1], look.layer as f32, look.anim],
            });
            let out = if look.clear {
                &mut geo.trans
            } else {
                &mut geo.opaque
            };
            if k == 3 {
                out.extend([vs[0], vs[1], vs[2]]);
            } else {
                out.extend([vs[0], vs[1], vs[2], vs[0], vs[2], vs[3]]);
            }
        }
    }
}

/// Every model of every shape file as the mesher draws it, in voxels of
/// the cell (for the voxel rule's mesh test).
#[cfg(test)]
pub(crate) fn voxel_objects() -> Vec<crate::voxel_rule::Object> {
    let p = pack();
    let mut out = Vec::new();
    for (fi, (path, set, _)) in p.files.iter().enumerate() {
        let models = p.models(fi as u16);
        let names = &p.sets[*set as usize].models;
        for (name, quads) in names.iter().zip(&models.models) {
            let mut tris = Vec::new();
            for q in quads {
                let corners: &[[usize; 3]] = if q.tri {
                    &[[0, 1, 2]]
                } else {
                    &[[0, 1, 2], [0, 2, 3]]
                };
                for c in corners {
                    tris.push(crate::voxel_rule::Tri {
                        p: c.map(|i| q.p[i]),
                        n: Some(c.map(|i| q.n[i])),
                    });
                }
            }
            out.push(crate::voxel_rule::Object {
                name: format!("shape {path}"),
                part: name.clone(),
                per_unit: 1.0,
                origin: Some([0.0; 3]),
                tris,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use std::f32::consts::PI;

    /// Prints the quads of each shape model, heaviest first:
    /// `cargo test -p dyno-client shape_weights -- --ignored --nocapture`
    #[test]
    #[ignore = "a measurement"]
    fn shape_weights() {
        let p = pack();
        let mut all = Vec::new();
        for (i, (path, set, _)) in p.files.iter().enumerate() {
            let m = p.models(i as u16);
            for (k, q) in m.models.iter().enumerate() {
                all.push((
                    q.len(),
                    path.clone(),
                    p.sets[*set as usize].models[k].clone(),
                ));
            }
        }
        all.sort_by(|a, b| b.cmp(a));
        for (n, path, name) in all.iter().take(25) {
            println!("{n:5} {path} {name}");
        }
    }

    /// The pack test for shapes: `blocks.json` reads, every rule set's
    /// models are in each shape file that uses it, every table names a
    /// model, and every material names a real block.
    #[test]
    fn every_shape_is_in_the_pack() {
        let p = pack();
        assert!(!p.sets.is_empty(), "no shape rules in blocks.json");
        for (path, set, _) in &p.files {
            let bytes = crate::assets::get(path).unwrap_or_else(|| panic!("{path} missing"));
            let vox = crate::vox::read(&bytes).unwrap();
            for name in &p.sets[*set as usize].models {
                assert!(vox.model(name).is_some(), "{path} has no model {name}");
            }
            let m = p.models(p.files.iter().position(|f| f.0 == *path).unwrap() as u16);
            for (q, quads) in m.models.iter().enumerate() {
                for quad in quads {
                    assert!(
                        m.palette[quad.pal as usize].is_some(),
                        "{path}: model {q} uses colour {} with no material",
                        quad.pal
                    );
                }
            }
        }
        for m in p.file_palettes.values().flat_map(|f| f.values()) {
            if let Some(b) = m.block.as_deref()
                && b != "self"
            {
                assert!(p.names.contains_key(b), "no block {b}");
            }
        }
        for b in [w::RED_ROOF_TILES, w::TIMBER_BEAM, w::LANTERN, w::BARREL] {
            assert!(is_detail(b));
        }
        assert!(has(w::GRASS, tag::TERRAIN) && has(w::COBBLE, tag::PAVING));
    }

    /// Area of a meshed model's polygons facing roughly `n`.
    fn area(quads: &[Quad], n: Vec3) -> f32 {
        quads
            .iter()
            .filter(|q| Vec3::from(q.n[0]).dot(n) > 0.99)
            .map(|q| {
                let p = q.p.map(Vec3::from);
                let a = (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
                if q.tri {
                    a
                } else {
                    a + (p[2] - p[0]).cross(p[3] - p[0]).length() * 0.5
                }
            })
            .sum()
    }

    #[test]
    fn diagonal_runs_become_one_slope() {
        // a slab two voxels thick rising one voxel per voxel along +z,
        // eight voxels wide: a roof in small
        let mut voxels = Vec::new();
        for x in 0..8 {
            for z in 0..12 {
                voxels.push(([x, z, z], 1));
                voxels.push(([x, z + 1, z], 1));
            }
        }
        let quads = mesh_model(&voxels, &[Uv::Cell; 256], &[false; 256], None);
        let up = Vec3::new(0.0, 1.0, -1.0).normalize();
        let slopes: Vec<_> = quads
            .iter()
            .filter(|q| Vec3::from(q.n[0]).dot(up).abs() > 0.99)
            .collect();
        assert_eq!(slopes.len(), 2, "one slope on top, one beneath");
        // the sides are parallelograms, one each
        assert_eq!(quads.iter().filter(|q| q.n[0][0].abs() > 0.99).count(), 2);
        // a closed slab: each side's area matches the other's
        assert!((area(&quads, Vec3::X) - area(&quads, Vec3::NEG_X)).abs() < 1e-3);
        assert!((area(&quads, up) - area(&quads, -up)).abs() < 1e-3);
        // the slope runs the whole length (√2 per voxel risen) and width
        assert!((area(&quads, up) - 8.0 * 12.0 * 2f32.sqrt()).abs() < 1e-2);
        // texture along the slope: a repeat per block of rise
        let top = slopes
            .iter()
            .find(|q| Vec3::from(q.n[0]).dot(up) > 0.0)
            .unwrap();
        for (p, uv) in top.p.iter().zip(top.uv) {
            assert!((uv[1] + p[1] / FINE).abs() < 1e-5);
        }
        // a colour that keeps its steps is not sloped
        let mut steps = [false; 256];
        steps[1] = true;
        let stepped = mesh_model(&voxels, &[Uv::Cell; 256], &steps, None);
        assert!(
            stepped
                .iter()
                .all(|q| q.n[0].iter().filter(|c| c.abs() > 0.5).count() == 1)
        );
    }

    #[test]
    fn round_parts_are_solids_of_revolution() {
        // a voxel cylinder of radius 5 and height 16
        let mut voxels = Vec::new();
        for y in 0..16 {
            for x in 0..16 {
                for z in 0..16 {
                    let (dx, dz) = (x as f32 - 7.5, z as f32 - 7.5);
                    if dx * dx + dz * dz <= 25.0 {
                        voxels.push(([x, y, z], 1));
                    }
                }
            }
        }
        let round = RoundJson {
            axis: 1,
            sides: 8,
            wrap: 2.0,
            vscale: 1.0,
            only: Vec::new(),
            phase: 0.0,
            flat: false,
        };
        let quads = mesh_model(&voxels, &[Uv::Cell; 256], &[false; 256], Some(&round));
        // eight sides, no caps: both ends reach the cell's sides
        assert_eq!(quads.len(), 8);
        for q in &quads {
            for p in q.p {
                let r = Vec2::new(p[0] - 8.0, p[2] - 8.0).length();
                assert!((r - (voxels.len() as f32 / 16.0 / PI).sqrt()).abs() < 0.3);
            }
            assert!(q.uv.iter().all(|uv| (0.0..=2.0).contains(&uv[0])));
        }
        // shorter, it is capped on top
        let short: Vec<_> = voxels.iter().copied().filter(|(p, _)| p[1] < 12).collect();
        let quads = mesh_model(&short, &[Uv::Cell; 256], &[false; 256], Some(&round));
        assert_eq!(quads.iter().filter(|q| q.tri).count(), 6);
    }

    #[test]
    fn fitted_textures_span_the_colour() {
        let mut voxels = Vec::new();
        for x in 2..14 {
            for y in 0..10 {
                for z in 2..14 {
                    voxels.push(([x, y, z], 1));
                }
            }
        }
        let mut uv = [Uv::Cell; 256];
        uv[1] = Uv::Fit([2.0, 0.0, 2.0], [14.0, 10.0, 14.0]);
        let quads = mesh_model(&voxels, &uv, &[false; 256], None);
        assert_eq!(quads.len(), 6);
        for q in &quads {
            let (lo, hi) = q.uv.iter().fold((f32::MAX, f32::MIN), |(lo, hi), uv| {
                (lo.min(uv[0].min(uv[1])), hi.max(uv[0].max(uv[1])))
            });
            assert!(lo.abs() < 1e-5 && (hi - 1.0).abs() < 1e-5, "{lo} {hi}");
        }
    }

    #[test]
    fn variation_per_place_is_steady() {
        let v = VaryJson {
            salt: 11,
            steps: 8,
            step: 0.16,
        };
        let a = v.at([3, 40, 9]);
        // the same column gives the same answer at any height
        assert_eq!(a, v.at([3, 7, 9]));
        assert!((0.0..8.0 * 0.16).contains(&a));
        let all: std::collections::HashSet<u32> =
            (0..64).map(|x| (v.at([x, 0, 5]) * 100.0) as u32).collect();
        assert!(all.len() > 4, "places differ");
    }
}
