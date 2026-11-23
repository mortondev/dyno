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
//! { "blocks": { "red_roof_tiles": { "id": 49, "tags": ["roof", "chamfer"],
//!                 "shape": { "rules": "roof", "vox": "shapes/roof_red.vox" } } },
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

use crate::light::Light;
use crate::render::{self, Vertex};
use crate::terrain_mesh::{
    DX, DY, DZ, F_DETAIL, F_OPAQUE, Glow, Grid, SectionGeo, at, hash3,
};
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
    blocks: BTreeMap<String, BlockJson>,
    #[serde(default)]
    shape_rules: BTreeMap<String, RuleSetJson>,
    #[serde(default)]
    shape_files: BTreeMap<String, ShapeFileJson>,
}

#[derive(Deserialize)]
struct BlockJson {
    id: u16,
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
    parts: Vec<PartJson>,
}

#[derive(Deserialize)]
struct PartJson {
    #[allow(dead_code)]
    name: String,
    features: Vec<CondJson>,
    table: BTreeMap<String, Option<EntryJson>>,
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

struct Entry {
    model: u16,
    glow: Option<[f32; 3]>,
}

struct Part {
    features: Vec<Cond>,
    table: HashMap<u32, Option<Entry>>,
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
    parts: Vec<Part>,
}

struct RuleSet {
    models: Vec<String>,
    classes: Vec<Class>,
}

/// A meshed shape model: quads in voxels of the cell's local frame.
struct Quad {
    p: [[f32; 3]; 4],
    /// 0 +x, 1 -x, 2 +y, 3 -y, 4 +z, 5 -z
    face: u8,
    pal: u8,
    /// lies on the cell's own face: hidden when the cell beside is opaque
    edge: bool,
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
        for (name, b) in &f.blocks {
            if b.id < 256 {
                p.names.insert(name.clone(), b.id as BlockId);
            }
        }
        for b in f.blocks.values() {
            let id = b.id as usize;
            if id >= 256 {
                continue;
            }
            for t in &b.tags {
                p.tags[id] |= p.tag_bit(t);
            }
        }
        let mut set_index = HashMap::new();
        for (name, s) in &f.shape_rules {
            let mut classes = Vec::new();
            for c in &s.classes {
                let id = p.class_id(&c.class);
                let when = c.when.as_ref().map(|w| compile(w, &mut p));
                let score = c.score.iter().map(|(k, w)| (*k, compile(w, &mut p))).collect();
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
                    parts.push(Part { features, table });
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
                    parts,
                });
            }
            set_index.insert(name.clone(), p.sets.len() as u16);
            p.sets.push(RuleSet {
                models: s.models.clone(),
                classes,
            });
        }
        p.file_palettes = f
            .shape_files
            .iter()
            .map(|(k, v)| (k.clone(), v.palette.clone()))
            .collect();
        for b in f.blocks.values() {
            let (Some(s), true) = (&b.shape, b.id < 256) else {
                continue;
            };
            let Some(set) = set_index.get(&s.rules).copied() else {
                continue;
            };
            let file = match p.files.iter().position(|(v, k, _)| *v == s.vox && *k == set) {
                Some(i) => i,
                None => {
                    p.files.push((s.vox.clone(), set, OnceLock::new()));
                    p.files.len() - 1
                }
            };
            p.shape[b.id as usize] = Some((set, file as u16));
        }
        Ok(p)
    }

    /// The meshed models of a shape file, read from the pack on first use.
    fn models(&self, file: u16) -> &Models {
        let (path, set, cell) = &self.files[file as usize];
        cell.get_or_init(|| {
            let set = &self.sets[*set as usize];
            let vox = crate::assets::get(path).and_then(|b| crate::vox::read(&b).ok());
            let models = set
                .models
                .iter()
                .map(|name| {
                    vox.as_ref()
                        .and_then(|v| v.model(name))
                        .map(|m| mesh_model(&m.voxels))
                        .unwrap_or_default()
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
pub(crate) fn pack() -> &'static Pack {
    static P: OnceLock<Pack> = OnceLock::new();
    P.get_or_init(|| {
        crate::assets::ensure_local_pack();
        match crate::assets::text("blocks.json").map(|t| Pack::parse(&t)) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                eprintln!("{e}");
                Pack::empty()
            }
            None => Pack::empty(),
        }
    })
}

/// Whether a block carries one of the engine's tags ([`tag`]).
#[inline]
pub(crate) fn has(b: BlockId, t: u32) -> bool {
    pack().tags[b as usize] & t != 0
}

/// Blocks drawn by the shape mesher.
pub(crate) fn is_detail(b: BlockId) -> bool {
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
                let s: f32 = cl.score.iter().filter(|(_, k)| x.eval(k)).map(|(w, _)| w).sum();
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

/// For tools: the class a classified cell took (name and turn) and the
/// value of each feature of its first part.
#[cfg(test)]
pub(crate) fn features_at(p: &Pack, g: &Grid, c: usize) -> Option<(String, u8, Vec<bool>)> {
    let v = *g.cls.get(c)?;
    let (id, q) = (v >> 2, (v & 3) as u8);
    if id == 0 {
        return None;
    }
    let (set, _) = p.shape[g.ids[c] as usize]?;
    let cl = p.sets[set as usize].classes.iter().find(|k| k.id == id)?;
    let x = Ctx { p, g, c, q };
    let f = cl
        .parts
        .first()
        .map(|part| part.features.iter().map(|k| x.eval(k)).collect())
        .unwrap_or_default();
    Some((p.classes[id as usize].clone(), q, f))
}

/// For tools: read a pack from text.
#[cfg(test)]
pub(crate) fn parse_pack(text: &str) -> Result<Pack, String> {
    Pack::parse(text)
}

/// The class a cell was given, by name (for tests and tools).
#[cfg(test)]
pub(crate) fn class_name(g: &Grid, c: usize) -> Option<(&'static str, u8)> {
    let v = *g.cls.get(c)?;
    (v >> 2 != 0).then(|| (pack().classes[(v >> 2) as usize].as_str(), (v & 3) as u8))
}

// ----------------------------------------------------------------- meshing

/// Greedy-mesh a model's voxels into quads, one per run of faces of one
/// colour.
fn mesh_model(voxels: &[([i32; 3], u8)]) -> Vec<Quad> {
    if voxels.is_empty() {
        return Vec::new();
    }
    let (mut lo, mut hi) = ([i32::MAX; 3], [i32::MIN; 3]);
    for (p, _) in voxels {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a] + 1);
        }
    }
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let idx = |p: [i32; 3]| -> Option<usize> {
        let r = [p[0] - lo[0], p[1] - lo[1], p[2] - lo[2]];
        (r.iter().zip(size).all(|(v, s)| (0..s).contains(v)))
            .then(|| ((r[1] * size[2] + r[2]) * size[0] + r[0]) as usize)
    };
    let mut cells = vec![0u8; (size[0] * size[1] * size[2]) as usize];
    for (p, c) in voxels {
        if let Some(i) = idx(*p) {
            cells[i] = (*c).max(1);
        }
    }
    let get = |p: [i32; 3]| idx(p).map(|i| cells[i]).unwrap_or(0);
    let mut out = Vec::new();
    let full = FINE as i32;
    for d in 0..3usize {
        let (ua, va) = ((d + 1) % 3, (d + 2) % 3);
        let (nu, nv) = (size[ua], size[va]);
        let mut mask = vec![0u8; (nu * nv) as usize];
        for dir in [1i32, -1] {
            for s in 0..size[d] {
                let mut any = false;
                for v in 0..nv {
                    for u in 0..nu {
                        let mut p = [0; 3];
                        p[d] = lo[d] + s;
                        p[ua] = lo[ua] + u;
                        p[va] = lo[va] + v;
                        let c = get(p);
                        let mut q = p;
                        q[d] += dir;
                        let k = if c != 0 && get(q) == 0 { c } else { 0 };
                        any |= k != 0;
                        mask[(v * nu + u) as usize] = k;
                    }
                }
                if !any {
                    continue;
                }
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
                        let plane = lo[d] + s + i32::from(dir > 0);
                        let corner = |cu: i32, cv: i32| {
                            let mut p = [0f32; 3];
                            p[d] = plane as f32;
                            p[ua] = (lo[ua] + cu) as f32;
                            p[va] = (lo[va] + cv) as f32;
                            p
                        };
                        let quad = [
                            corner(u, v),
                            corner(u + wd, v),
                            corner(u + wd, v + ht),
                            corner(u, v + ht),
                        ];
                        // facing out of the cell from its own boundary
                        let edge = (dir > 0 && plane == full) || (dir < 0 && plane == 0);
                        out.push(Quad {
                            p: quad,
                            face: (d * 2) as u8 + u8::from(dir < 0),
                            pal: k,
                            edge,
                        });
                        u += wd;
                    }
                }
            }
        }
    }
    out
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
                occ: u8::from(
                    fl & F_DETAIL != 0 && fl & crate::terrain_mesh::F_DARK != 0,
                ),
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

/// Texture coordinates of a point on a face, by its place in the cell
/// (sides run down from the top, as on cubes).
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

const NORMALS: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

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
                emit(p, g, &shade, c, wp, cl, q, file, b, geo);
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
    let jitter = if cl.jitter != 0.0 {
        let h = hash3(wp[0], 11, wp[2]);
        ((h & 1023) as f32 / 1023.0 - 0.5) * 0.24 * cl.jitter
    } else {
        0.0
    };
    let angle = q as f32 * std::f32::consts::FRAC_PI_2 + jitter;
    let (sin, cos) = if jitter == 0.0 {
        [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)][q as usize & 3]
    } else {
        angle.sin_cos()
    };
    let dir = |v: Vec3| Vec3::new(v.x * cos + v.z * sin, v.y, -v.x * sin + v.z * cos);
    let place = |v: Vec3| base + Vec3::new(0.5, 0.0, 0.5) + dir(v - Vec3::new(0.5, 0.0, 0.5));
    let models = p.models(file);
    let mut looks: [Option<[Look; 3]>; 256] = [None; 256];
    // which of the cell's own faces another opaque cell covers
    let mut covered = [false; 6];
    if jitter == 0.0 {
        for (f, s) in STEPS.iter().enumerate() {
            covered[f] = g.fl[step(c, turn(*s, q))] & F_OPAQUE != 0;
        }
    }
    for part in &cl.parts {
        let mut bits = 0u32;
        for (i, f) in part.features.iter().enumerate() {
            if x.eval(f) {
                bits |= 1 << i;
            }
        }
        let entry = match part.table.get(&bits) {
            Some(e) => e.as_ref(),
            // never seen when baked: the nearest pattern that was
            None => part
                .table
                .iter()
                .min_by_key(|(k, _)| ((*k ^ bits).count_ones(), **k))
                .and_then(|(_, e)| e.as_ref()),
        };
        let Some(entry) = entry else { continue };
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
        for quad in quads {
            if quad.edge && covered[quad.face as usize] {
                continue;
            }
            let pal = quad.pal as usize;
            let look = match looks[pal] {
                Some(l) => l,
                None => {
                    let Some(Some(m)) = models.palette.get(pal) else {
                        continue;
                    };
                    let l = [0, 1, 2].map(|f| resolve(m, &x, f));
                    looks[pal] = Some(l);
                    l
                }
            };
            let n_local = NORMALS[quad.face as usize];
            let face_class = match quad.face {
                2 => 0,
                3 => 2,
                _ => 1,
            };
            let look = look[face_class];
            let local = quad.p.map(|v| Vec3::from(v) / FINE);
            let world = local.map(place);
            let wn = dir(n_local);
            let mid = (world[0] + world[2]) * 0.5;
            let lit = if look.glow {
                (0.0, 0.0, 1.0)
            } else {
                shade.at(mid, wn)
            };
            let vs = [0, 1, 2, 3].map(|i| {
                let uv = map_uv(local[i], n_local);
                Vertex {
                    pos: world[i].into(),
                    normal: wn.into(),
                    color: [
                        look.tint[0],
                        look.tint[1],
                        look.tint[2],
                        if look.glow { 2.0 } else { lit.2 },
                    ],
                    light: lit.0,
                    sky: lit.1,
                    tex: [uv[0], uv[1], look.layer as f32, look.anim],
                }
            });
            let out = if look.clear {
                &mut geo.trans
            } else {
                &mut geo.opaque
            };
            out.extend([vs[0], vs[1], vs[2], vs[0], vs[2], vs[3]]);
        }
    }
}

/// Bits of a table key as the JSON writes them: feature 0 first.
pub(crate) fn bits_key(bits: u32, n: usize) -> String {
    (0..n).map(|i| if bits >> i & 1 == 1 { '1' } else { '0' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
