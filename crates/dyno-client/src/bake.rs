//! One-off bake: runs today's model builders and pose functions and writes
//! the content pack's figures (`figures.json`, `models/*.vox`), rigs
//! (`rigs/*.json`) and clips (`anims/*.json`).
//!
//! `cargo test -p dyno-client --release bake_figures -- --ignored --nocapture`

use super::*;
use crate::anim::{self, At, MAX_BONES};
use crate::{figure, vox};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The voxels of a built grid.
pub struct Snapshot {
    pub min: [i32; 3],
    pub max: [i32; 3],
    pub voxels: Vec<([i32; 3], u8, [f32; 3], bool)>,
}

impl Snapshot {
    pub fn of(v: &Vox) -> Snapshot {
        let (min, max) = v.bounds();
        let mut voxels = Vec::new();
        for y in min[1]..max[1] {
            for z in min[2]..max[2] {
                for x in min[0]..max[0] {
                    if let Some((ink, part)) = v.voxel(x, y, z) {
                        let (c, g) = v.color(ink);
                        voxels.push(([x, y, z], part, c, g));
                    }
                }
            }
        }
        Snapshot { min, max, voxels }
    }
}

fn take_snapshots() -> Vec<Snapshot> {
    SNAPSHOTS.with(|s| std::mem::take(&mut *s.borrow_mut()))
}

// ------------------------------------------------------------- recipes

#[derive(Clone, Copy, Debug)]
struct Recipe {
    slot: Option<u8>,
    k: [f32; 3],
    c: [f32; 3],
    glow: bool,
}

impl Recipe {
    fn key(&self) -> (Option<u8>, [i64; 3], [i64; 3], bool) {
        let q = |v: [f32; 3]| v.map(|x| (x as f64 * 1e5).round() as i64);
        (self.slot, q(self.k), q(self.c), self.glow)
    }
}

#[derive(Default)]
struct Recipes {
    list: Vec<Recipe>,
    index: HashMap<(Option<u8>, [i64; 3], [i64; 3], bool), u16>,
}

impl Recipes {
    fn intern(&mut self, r: Recipe) -> u16 {
        let k = r.key();
        if let Some(i) = self.index.get(&k) {
            return *i;
        }
        let i = self.list.len() as u16;
        self.list.push(r);
        self.index.insert(k, i);
        i
    }
}

/// Colours to probe a builder with: each slot a distinct colour, and the
/// same with one slot moved.
struct Probes {
    slots: Vec<&'static str>,
    base: [f32; 3],
}

impl Probes {
    fn color(&self, slot: usize, moved: Option<usize>, random: bool) -> [f32; 3] {
        let i = slot as f32;
        if random {
            return [
                self.base[0] - 0.12 + 0.013 * i,
                self.base[1] - 0.10 + 0.005 * i,
                self.base[2] - 0.11 + 0.008 * i,
            ];
        }
        let mut c = [
            self.base[0] + 0.011 * i,
            self.base[1] + 0.007 * i,
            self.base[2] - 0.009 * i,
        ];
        if moved == Some(slot) {
            c = [c[0] + 0.04, c[1] + 0.03, c[2] + 0.05];
        }
        c
    }
}

// --------------------------------------------------------------- effects

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum M {
    Set,
    Add,
    Erase,
    /// recolour what is there, keeping its part
    Dye,
    /// add where empty, else recolour what is there
    AddDye,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Entry {
    mode: M,
    part: u8,
    recipe: u16,
}

type Effect = Vec<(u32, Entry)>;

/// A grid's bounds and cell numbering.
#[derive(Clone, Copy)]
struct Bounds {
    min: [i32; 3],
    max: [i32; 3],
}

impl Bounds {
    fn size(&self) -> [i32; 3] {
        [0, 1, 2].map(|a| self.max[a] - self.min[a])
    }
    fn cell(&self, p: [i32; 3]) -> u32 {
        let s = self.size();
        let q = [0, 1, 2].map(|a| p[a] - self.min[a]);
        ((q[1] * s[2] + q[2]) * s[0] + q[0]) as u32
    }
    fn at(&self, c: u32) -> [i32; 3] {
        let s = self.size();
        let c = c as i32;
        [
            c % s[0] + self.min[0],
            c / (s[0] * s[2]) + self.min[1],
            (c / s[0]) % s[2] + self.min[2],
        ]
    }
}

const SENT_PART: u8 = 250;
const SENT_RGB: [f32; 3] = [0.999, 0.0123, 0.987];

/// How a group of builder code changes a grid: set, add or erase per cell.
/// With `layered`, it is run over an empty grid and over a full one, to
/// tell setting from adding; otherwise everything it leaves is set.
type Run<'a> = dyn Fn(&mut Vox, &dyn Fn(&str) -> u8, &dyn Fn(&str) -> [f32; 3]) + Sync + 'a;

struct GroupDef<'a> {
    name: String,
    features: Vec<(&'static str, Vec<u8>)>,
    /// feature → (value → class, class of the rest)
    map: Vec<(&'static str, Vec<(u8, u8)>, u8)>,
    probes: Probes,
    layered: bool,
    /// which snapshot of a builder run is this group's grid (for builders
    /// that make their own grids)
    snapshot: Option<usize>,
    bounds: Bounds,
    run: Box<Run<'a>>,
}

impl GroupDef<'_> {
    /// Run the code with these feature values and colours; the grid it
    /// leaves.
    fn grid(&self, vals: &[u8], col: &dyn Fn(&str) -> [f32; 3], full: bool) -> Snapshot {
        let f = |name: &str| -> u8 {
            match self.features.iter().position(|(n, _)| *n == name) {
                Some(i) => vals[i],
                None => default_feature(name),
            }
        };
        match self.snapshot {
            Some(i) => {
                take_snapshots();
                let mut v = Vox::new([0; 3], [1; 3]);
                (self.run)(&mut v, &f, col);
                take_snapshots().swap_remove(i)
            }
            None => {
                let mut v = Vox::new(self.bounds.min, self.bounds.max);
                if full {
                    v.part = SENT_PART;
                    let s = v.ink(SENT_RGB);
                    let (lo, hi) = (self.bounds.min, self.bounds.max);
                    v.fill([lo[0], hi[0]], [lo[1], hi[1]], [lo[2], hi[2]], s);
                    v.part = 0;
                }
                (self.run)(&mut v, &f, col);
                Snapshot::of(&v)
            }
        }
    }

    fn effect(&self, vals: &[u8], recipes: &Mutex<Recipes>) -> Result<Effect, String> {
        let slots = &self.probes.slots;
        let col_of = |moved: Option<usize>, random: bool| {
            move |name: &str| -> [f32; 3] {
                let i = slots.iter().position(|s| *s == name).unwrap_or(slots.len() + 3);
                self.probes.color(i, moved, random)
            }
        };
        let bounds = self.bounds;
        // what the code leaves over an empty grid, and over a full one (only
        // the cells it changed)
        type Seen = HashMap<u32, (u8, [f32; 3], bool)>;
        type Full = HashMap<u32, Option<(u8, [f32; 3], bool)>>;
        let runs = |moved: Option<usize>, random: bool| -> (Seen, Full) {
            let col = col_of(moved, random);
            let a = self.grid(vals, &col, false);
            let a: Seen = a
                .voxels
                .iter()
                .map(|(p, part, c, g)| (bounds.cell(*p), (*part, *c, *g)))
                .collect();
            let mut b: Full = HashMap::new();
            if self.layered {
                let s = self.grid(vals, &col, true);
                let n = bounds.size().iter().product::<i32>() as usize;
                let mut seen = vec![false; n];
                for (p, part, c, g) in &s.voxels {
                    let cell = bounds.cell(*p);
                    seen[cell as usize] = true;
                    if !(*part == SENT_PART && *c == SENT_RGB) {
                        b.insert(cell, Some((*part, *c, *g)));
                    }
                }
                for (cell, s) in seen.iter().enumerate() {
                    if !s {
                        b.insert(cell as u32, None);
                    }
                }
            }
            (a, b)
        };
        let (a0, b0) = runs(None, false);
        let moved: Vec<(Seen, Full)> = (0..slots.len()).map(|k| runs(Some(k), false)).collect();
        let random = runs(None, true);
        let mut cells: Vec<u32> = a0.keys().chain(b0.keys()).copied().collect();
        cells.sort_unstable();
        cells.dedup();
        // the colour a cell ends with under a run
        let color = |r: &(Seen, Full), cell: u32| -> Option<([f32; 3], u8)> {
            r.0.get(&cell)
                .map(|x| (x.1, x.0))
                .or_else(|| r.1.get(&cell).copied().flatten().map(|x| (x.1, x.0)))
        };
        let mut out = Vec::with_capacity(cells.len());
        let mut rec = recipes.lock().unwrap();
        for cell in cells {
            let ea = a0.get(&cell).copied();
            let eb = if self.layered { b0.get(&cell).copied() } else { ea.map(Some) };
            let (mode, part, c0, glow) = match (ea, eb) {
                (None, None) => continue,
                // untouched over a full grid
                (Some(a), None) => (M::Add, a.0, a.1, a.2),
                (Some(a), Some(Some(b))) if a == b => (M::Set, a.0, a.1, a.2),
                (Some(a), Some(Some(b))) if b.0 == SENT_PART && (a.1, a.2) == (b.1, b.2) => {
                    (M::AddDye, a.0, a.1, a.2)
                }
                (None, Some(None)) => {
                    out.push((
                        cell,
                        Entry {
                            mode: M::Erase,
                            part: 0,
                            recipe: 0,
                        },
                    ));
                    continue;
                }
                (None, Some(Some(b))) if b.0 == SENT_PART => (M::Dye, 0, b.1, b.2),
                _ => {
                    return Err(format!(
                        "{} {vals:?}: cell {:?} is {ea:?} over nothing and {eb:?} over something",
                        self.name,
                        bounds.at(cell)
                    ));
                }
            };
            let mut slot = None;
            for (k, m) in moved.iter().enumerate() {
                let Some((ck, _)) = color(m, cell) else {
                    return Err(format!("{} {vals:?}: a cell comes and goes with colour", self.name));
                };
                if (0..3).any(|i| (ck[i] - c0[i]).abs() > 1e-6) {
                    if slot.is_some() {
                        return Err(format!(
                            "{} {vals:?}: cell {:?} follows two colours",
                            self.name,
                            bounds.at(cell)
                        ));
                    }
                    slot = Some(k);
                }
            }
            let r = match slot {
                None => Recipe {
                    slot: None,
                    k: [0.0; 3],
                    c: c0,
                    glow,
                },
                Some(k) => {
                    let (p0, p1) = (
                        self.probes.color(k, None, false),
                        self.probes.color(k, Some(k), false),
                    );
                    let c1 = color(&moved[k], cell).unwrap().0;
                    let kk: [f32; 3] = std::array::from_fn(|i| (c1[i] - c0[i]) / (p1[i] - p0[i]));
                    let cc: [f32; 3] = std::array::from_fn(|i| c0[i] - kk[i] * p0[i]);
                    Recipe {
                        slot: Some(k as u8),
                        k: kk,
                        c: cc,
                        glow,
                    }
                }
            };
            // check against the random colours
            let want = color(&random, cell).map(|x| x.0).unwrap_or([9.0; 3]);
            let base = match r.slot {
                Some(k) => self.probes.color(k as usize, None, true),
                None => [0.0; 3],
            };
            let got: [f32; 3] = std::array::from_fn(|i| (r.k[i] * base[i] + r.c[i]).clamp(0.0, 1.0));
            if (0..3).any(|i| (got[i] - want[i]).abs() > 3e-4) {
                return Err(format!(
                    "{} {vals:?}: cell {:?} colour {want:?} is not {got:?} ({r:?})",
                    self.name,
                    bounds.at(cell)
                ));
            }
            out.push((
                cell,
                Entry {
                    mode,
                    part,
                    recipe: rec.intern(r),
                },
            ));
        }
        Ok(out)
    }
}

fn default_feature(name: &str) -> u8 {
    match name {
        "build" | "armor" | "emblem" | "hair_style" | "cape" | "gloves" | "ears" | "quality" => 1,
        _ => 0,
    }
}

type Content = Vec<(u32, Option<Entry>)>;

fn diff(prev: &Effect, cur: &Effect) -> Content {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < prev.len() || j < cur.len() {
        match (prev.get(i), cur.get(j)) {
            (Some(a), Some(b)) if a.0 == b.0 => {
                if a.1 != b.1 {
                    out.push((b.0, Some(b.1)));
                }
                i += 1;
                j += 1;
            }
            (Some(a), Some(b)) if a.0 < b.0 => {
                out.push((a.0, None));
                i += 1;
            }
            (Some(a), None) => {
                out.push((a.0, None));
                i += 1;
            }
            (_, Some(b)) => {
                out.push((b.0, Some(b.1)));
                j += 1;
            }
            (None, None) => break,
        }
    }
    out
}

fn hash_of(c: &Content) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    c.hash(&mut h);
    h.finish()
}

fn par_map<T: Send, R: Send>(items: Vec<T>, f: impl Fn(T) -> R + Sync) -> Vec<R> {
    let n = items.len();
    let slots: Vec<Mutex<Option<T>>> = items.into_iter().map(|t| Mutex::new(Some(t))).collect();
    let out: Vec<Mutex<Option<R>>> = (0..n).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let t = slots[i].lock().unwrap().take().unwrap();
                    *out[i].lock().unwrap() = Some(f(t));
                }
            });
        }
    });
    out.into_iter().map(|m| m.into_inner().unwrap().unwrap()).collect()
}

/// A layer of a group: when it applies, and what it does.
struct LayerOut {
    name: String,
    when: Vec<BTreeMap<String, Vec<u8>>>,
    content: Content,
}

/// Bake a group: its base, then for each feature in turn a layer per value
/// that changes it, keyed by only the earlier features it depends on.
fn bake_group(g: &GroupDef, recipes: &Mutex<Recipes>) -> Result<Vec<LayerOut>, String> {
    let n = g.features.len();
    let defaults: Vec<u8> = g.features.iter().map(|(_, d)| d[0]).collect();
    let base = g.effect(&defaults, recipes)?;
    let mut layers = vec![LayerOut {
        name: format!("{}/base", g.name),
        when: Vec::new(),
        content: base.iter().map(|(c, e)| (*c, Some(*e))).collect(),
    }];
    for i in 0..n {
        // every combination of the earlier features
        let mut parents: Vec<Vec<u8>> = vec![Vec::new()];
        for (_, dom) in &g.features[..i] {
            parents = parents
                .into_iter()
                .flat_map(|p| {
                    dom.iter().map(move |v| {
                        let mut q = p.clone();
                        q.push(*v);
                        q
                    })
                })
                .collect();
        }
        let values: Vec<u8> = g.features[i].1[1..].to_vec();
        let results = par_map(parents, |parent| -> Result<Vec<(Vec<u8>, Content)>, String> {
            let mut vals = parent.clone();
            vals.extend_from_slice(&defaults[i..]);
            let prev = g.effect(&vals, recipes)?;
            let mut out = Vec::new();
            for v in &values {
                vals[i] = *v;
                let cur = g.effect(&vals, recipes)?;
                let mut key = parent.clone();
                key.push(*v);
                out.push((key, diff(&prev, &cur)));
            }
            Ok(out)
        });
        let mut keyed: Vec<(Vec<u8>, u64)> = Vec::new();
        let mut contents: HashMap<u64, Content> = HashMap::new();
        for r in results {
            for (key, content) in r? {
                let h = hash_of(&content);
                contents.entry(h).or_insert(content);
                keyed.push((key, h));
            }
        }
        // the earlier features this one depends on
        let mut deps: Vec<usize> = (0..i).collect();
        for j in (0..i).rev() {
            let trial: Vec<usize> = deps.iter().copied().filter(|d| *d != j).collect();
            let mut seen: HashMap<Vec<u8>, u64> = HashMap::new();
            let ok = keyed.iter().all(|(key, h)| {
                let mut k: Vec<u8> = trial.iter().map(|d| key[*d]).collect();
                k.push(key[i]);
                *seen.entry(k).or_insert(*h) == *h
            });
            if ok {
                deps = trial;
            }
        }
        // one layer per distinct non-empty content
        let mut by_content: Vec<(u64, Vec<Vec<u8>>)> = Vec::new();
        for (key, h) in &keyed {
            if contents[h].is_empty() {
                continue;
            }
            let mut k: Vec<u8> = deps.iter().map(|d| key[*d]).collect();
            k.push(key[i]);
            match by_content.iter_mut().find(|(x, _)| x == h) {
                Some((_, keys)) => {
                    if !keys.contains(&k) {
                        keys.push(k)
                    }
                }
                None => by_content.push((*h, vec![k])),
            }
        }
        let names: Vec<&str> = deps
            .iter()
            .map(|d| g.features[*d].0)
            .chain([g.features[i].0])
            .collect();
        for (h, keys) in by_content {
            let first = &keys[0];
            let mut label = format!("{}={}", names[names.len() - 1], first[first.len() - 1]);
            for (n, v) in names.iter().zip(first).take(names.len() - 1) {
                label.push_str(&format!(",{n}={v}"));
            }
            let mut name = format!("{}/{label}", g.name);
            if layers.iter().any(|l| l.name == name) {
                name = format!("{name}+{}", layers.len());
            }
            layers.push(LayerOut {
                name,
                when: keys
                    .iter()
                    .map(|k| {
                        names
                            .iter()
                            .zip(k)
                            .map(|(n, v)| (n.to_string(), vec![*v]))
                            .collect()
                    })
                    .collect(),
                content: contents[&h].clone(),
            });
        }
    }
    Ok(layers)
}

// ----------------------------------------------------------------- output

/// JSON with arrays of numbers kept on one line, so clips and palettes
/// stay readable.
fn json_text(v: &Value) -> String {
    fn flat(v: &Value) -> bool {
        match v {
            Value::Array(a) => a.iter().all(|x| x.is_number() || (x.is_array() && flat(x))),
            _ => false,
        }
    }
    fn go(v: &Value, ind: usize, out: &mut String) {
        let pad = "  ".repeat(ind + 1);
        match v {
            Value::Object(m) if !m.is_empty() => {
                out.push_str("{\n");
                for (i, (k, x)) in m.iter().enumerate() {
                    out.push_str(&pad);
                    out.push_str(&serde_json::to_string(k).unwrap());
                    out.push_str(": ");
                    go(x, ind + 1, out);
                    if i + 1 < m.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(ind));
                out.push('}');
            }
            Value::Array(a) if !a.is_empty() && !flat(v) => {
                out.push_str("[\n");
                for (i, x) in a.iter().enumerate() {
                    out.push_str(&pad);
                    go(x, ind + 1, out);
                    if i + 1 < a.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(ind));
                out.push(']');
            }
            _ => out.push_str(&serde_json::to_string(v).unwrap()),
        }
    }
    let mut s = String::new();
    go(v, 0, &mut s);
    s.push('\n');
    s
}

fn round(x: f32, places: i32) -> Value {
    let k = 10f64.powi(places);
    let r = (x as f64 * k).round() / k;
    if r == r.trunc() && r.abs() < 1e9 {
        json!(r as i64)
    } else {
        json!(r)
    }
}

fn round3(v: [f32; 3], places: i32) -> Value {
    Value::Array(v.iter().map(|x| round(*x, places)).collect())
}

struct GridOut {
    name: String,
    bounds: Bounds,
    parts: Vec<String>,
    groups: Vec<(String, Vec<(&'static str, Vec<(u8, u8)>, u8)>, Vec<LayerOut>)>,
}

/// Write a figure's `.vox` and return its `figures.json` entry.
fn write_figure(
    root: &Path,
    name: &str,
    grids: &[GridOut],
    recipes: &Recipes,
    slots: &[&str],
    show: &dyn Fn(&str) -> [f32; 3],
    extra: Map<String, Value>,
) -> Result<Value, String> {
    if recipes.list.len() > 254 {
        return Err(format!("{name}: {} colours, over 254", recipes.list.len()));
    }
    let mut file = vox::File::default();
    let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    for (i, r) in recipes.list.iter().enumerate() {
        let base = r.slot.map(|s| show(slots[s as usize])).unwrap_or([0.0; 3]);
        let c: [f32; 3] = std::array::from_fn(|a| (r.k[a] * base[a] + r.c[a]).clamp(0.0, 1.0));
        file.palette[i + 1] = [q(c[0]), q(c[1]), q(c[2]), 255];
    }
    file.palette[255] = [255, 0, 255, 255];
    let mut grids_json = Vec::new();
    for g in grids {
        let mut groups_json = Vec::new();
        for (gname, map, layers) in &g.groups {
            let mut layers_json = Vec::new();
            for l in layers {
                // one model per part and mode
                let mut models: BTreeMap<(String, &str), Vec<([i32; 3], u8)>> = BTreeMap::new();
                for (cell, e) in &l.content {
                    let p = g.bounds.at(*cell);
                    let (part, mode, ink) = match e {
                        None => ("-".to_string(), "drop", 255),
                        Some(e) => match e.mode {
                            M::Erase => ("-".to_string(), "erase", 255),
                            M::Set => (g.parts[e.part as usize].clone(), "set", e.recipe as u8 + 1),
                            M::Add => (g.parts[e.part as usize].clone(), "add", e.recipe as u8 + 1),
                            M::AddDye => (g.parts[e.part as usize].clone(), "add-dye", e.recipe as u8 + 1),
                            M::Dye => ("-".to_string(), "dye", e.recipe as u8 + 1),
                        },
                    };
                    models.entry((part, mode)).or_default().push((p, ink));
                }
                for ((part, mode), voxels) in models {
                    file.models.push(vox::Model {
                        name: format!("{}/{part}/{mode}", l.name),
                        voxels,
                    });
                }
                let mut lj = Map::new();
                lj.insert("layer".into(), json!(l.name));
                if !l.when.is_empty() {
                    lj.insert("when".into(), json!(l.when));
                }
                layers_json.push(Value::Object(lj));
            }
            let mut gj = Map::new();
            gj.insert("name".into(), json!(gname));
            if !map.is_empty() {
                let m: Map<String, Value> = map
                    .iter()
                    .map(|(f, pairs, rest)| {
                        let mut mm: Map<String, Value> =
                            pairs.iter().map(|(v, c)| (v.to_string(), json!(c))).collect();
                        mm.insert("*".into(), json!(rest));
                        (f.to_string(), Value::Object(mm))
                    })
                    .collect();
                gj.insert("map".into(), Value::Object(m));
            }
            gj.insert("layers".into(), Value::Array(layers_json));
            groups_json.push(Value::Object(gj));
        }
        grids_json.push(json!({
            "name": g.name,
            "min": g.bounds.min,
            "max": g.bounds.max,
            "parts": g.parts,
            "groups": groups_json,
        }));
    }
    let path = format!("models/{name}.vox");
    let bytes = vox::write(&file)?;
    std::fs::create_dir_all(root.join("models")).map_err(|e| e.to_string())?;
    std::fs::write(root.join(&path), bytes).map_err(|e| e.to_string())?;
    let mut palette = vec![Value::Null];
    for r in &recipes.list {
        let mut m = Map::new();
        match r.slot {
            Some(s) => {
                m.insert("slot".into(), json!(slots[s as usize]));
                if r.k != [1.0; 3] {
                    m.insert("k".into(), round3(r.k, 6));
                }
                if r.c != [0.0; 3] {
                    m.insert("c".into(), round3(r.c, 6));
                }
            }
            None => {
                m.insert("c".into(), round3(r.c, 6));
            }
        }
        if r.glow {
            m.insert("glow".into(), json!(true));
        }
        palette.push(Value::Object(m));
    }
    let mut fig = Map::new();
    fig.insert("file".into(), json!(path));
    for (k, v) in extra {
        fig.insert(k, v);
    }
    fig.insert("grids".into(), Value::Array(grids_json));
    fig.insert("palette".into(), Value::Array(palette));
    Ok(Value::Object(fig))
}

// ------------------------------------------------------------- the people

const SLOTS: [&str; 11] = [
    "skin", "hair", "eyes", "tunic", "trim", "pants", "boots", "apron", "cape", "scarf", "sash",
];

const ARMORS: [ArmorType; 4] = [
    ArmorType::Cloth,
    ArmorType::Leather,
    ArmorType::Mail,
    ArmorType::Plate,
];

/// An old-style look from feature values and colours.
fn person_look(f: &dyn Fn(&str) -> u8, c: &dyn Fn(&str) -> [f32; 3]) -> Look {
    let opt = |name: &str| (f(name) != 0).then(|| c(name));
    Look {
        skin: c("skin"),
        hair: c("hair"),
        hair_style: f("hair_style"),
        beard: f("beard") != 0,
        tunic: c("tunic"),
        trim: c("trim"),
        pants: c("pants"),
        boots: c("boots"),
        pauldrons: f("pauldrons") != 0,
        robe: f("robe") != 0,
        apron: opt("apron"),
        cape: opt("cape"),
        armor: ARMORS[f("armor") as usize % 4],
        helmet: f("helmet") != 0,
        weapon: None,
        eyes: c("eyes"),
        build: f("build"),
        elder: f("elder") != 0,
        gloves: f("gloves") != 0,
        emblem: f("emblem"),
        ears: f("ears"),
        mood: f("mood"),
        scarf: opt("scarf"),
        sash: opt("sash"),
        candle: f("candle") != 0,
    }
}

fn hw_of(build: u8) -> i32 {
    match build {
        0 => 9,
        2 => 11,
        _ => 10,
    }
}

fn person_groups() -> Vec<GroupDef<'static>> {
    let bounds = Bounds {
        min: [-28, 0, -16],
        max: [28, 82, 16],
    };
    let probes = || Probes {
        slots: SLOTS.to_vec(),
        base: [0.42, 0.47, 0.52],
    };
    let g = |name: &str, features: Vec<(&'static str, Vec<u8>)>, run: Box<Run<'static>>| GroupDef {
        name: name.into(),
        features,
        map: Vec::new(),
        probes: probes(),
        layered: true,
        snapshot: None,
        bounds,
        run,
    };
    let mut torso = g(
        "torso",
        vec![
            ("build", vec![1, 0, 2]),
            ("armor", vec![1, 0, 2, 3]),
            ("robe", vec![0, 1]),
            ("apron", vec![0, 1]),
            ("elder", vec![0, 1]),
            ("emblem", vec![1, 0, 2, 3, 4]),
            ("sash", vec![0, 1]),
            ("hair_style", vec![1, 3]),
            ("pauldrons", vec![0, 1]),
            ("cape", vec![1, 0]),
        ],
        Box::new(|v, f, c| {
            build_torso(v, &person_look(f, c));
        }),
    );
    torso.map = vec![("hair_style", vec![(3, 3)], 1)];
    vec![
        g(
            "legs",
            vec![("robe", vec![0, 1]), ("armor", vec![1, 0, 2, 3])],
            Box::new(|v, f, c| build_legs(v, &person_look(f, c))),
        ),
        torso,
        g(
            "arms",
            vec![
                ("build", vec![1, 0, 2]),
                ("armor", vec![1, 0, 2, 3]),
                ("robe", vec![0, 1]),
                ("gloves", vec![1, 0]),
            ],
            Box::new(|v, f, c| build_arms(v, &person_look(f, c), hw_of(f("build")))),
        ),
        g(
            "cape",
            vec![("cape", vec![1, 0]), ("build", vec![1, 0, 2])],
            Box::new(|v, f, c| build_cape(v, &person_look(f, c), hw_of(f("build")))),
        ),
        g(
            "head",
            vec![
                ("mood", vec![0, 1, 2]),
                ("ears", vec![1, 0, 2, 3]),
                ("hair_style", vec![1, 0, 2, 3, 4]),
                ("beard", vec![0, 1]),
                ("elder", vec![0, 1]),
                ("scarf", vec![0, 1]),
                ("candle", vec![0, 1]),
                ("helmet", vec![0, 1]),
            ],
            Box::new(|v, f, c| build_head(v, &person_look(f, c))),
        ),
    ]
}

const PERSON_PARTS: [&str; 17] = [
    "", "head", "torso", "flap.f", "flap.b", "skirt", "arm_u.r", "arm_u.l", "arm_l.r", "arm_l.l",
    "thigh.r", "thigh.l", "shin.r", "shin.l", "cape.0", "cape.1", "cape.2",
];

// ------------------------------------------------------- one-grid figures

/// A figure built in one go by an old builder, following one colour.
fn single(
    name: &str,
    snapshot: usize,
    features: Vec<(&'static str, Vec<u8>)>,
    run: Box<Run<'static>>,
    base: [f32; 3],
) -> GroupDef<'static> {
    GroupDef {
        name: name.into(),
        features,
        map: Vec::new(),
        probes: Probes {
            slots: vec!["color"],
            base,
        },
        layered: false,
        snapshot: Some(snapshot),
        bounds: Bounds {
            min: [0; 3],
            max: [1; 3],
        },
        run,
    }
}

/// Bake one-grid figures: the bounds come from a first run.
fn bake_single(
    g: &mut GroupDef,
    recipes: &Mutex<Recipes>,
) -> Result<(Bounds, Vec<LayerOut>), String> {
    let defaults: Vec<u8> = g.features.iter().map(|(_, d)| d[0]).collect();
    let s = g.grid(&defaults, &|_| g.probes.base, false);
    g.bounds = Bounds {
        min: s.min,
        max: s.max,
    };
    Ok((g.bounds, bake_group(g, recipes)?))
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Sword => "sword",
        Kind::BattleAxe => "battle_axe",
        Kind::Mace => "mace",
        Kind::Dagger => "dagger",
        Kind::Staff => "staff",
        Kind::Polearm => "polearm",
        Kind::Bow => "bow",
        Kind::Wand => "wand",
        Kind::Shield => "shield",
        Kind::Tome => "tome",
        Kind::Pickaxe => "pickaxe",
        Kind::Hatchet => "hatchet",
        Kind::Shovel => "shovel",
        Kind::Hoe => "hoe",
        Kind::Torch => "torch",
        Kind::Stick => "stick",
        Kind::Lump => "lump",
        Kind::Ingot => "ingot",
        Kind::Gem => "gem",
        Kind::Dust => "dust",
        Kind::Berries => "berries",
        Kind::Meat => "meat",
        Kind::Helm => "helm",
        Kind::Garment => "garment",
        Kind::Ring => "ring",
        Kind::Hand => "hand",
    }
}

const KINDS: [Kind; 26] = [
    Kind::Sword,
    Kind::BattleAxe,
    Kind::Mace,
    Kind::Dagger,
    Kind::Staff,
    Kind::Polearm,
    Kind::Bow,
    Kind::Wand,
    Kind::Shield,
    Kind::Tome,
    Kind::Pickaxe,
    Kind::Hatchet,
    Kind::Shovel,
    Kind::Hoe,
    Kind::Torch,
    Kind::Stick,
    Kind::Lump,
    Kind::Ingot,
    Kind::Gem,
    Kind::Dust,
    Kind::Berries,
    Kind::Meat,
    Kind::Helm,
    Kind::Garment,
    Kind::Ring,
    Kind::Hand,
];

// ------------------------------------------------------------------- rigs

fn v3(x: f32, y: f32, z: f32) -> Value {
    round3([x, y, z], 4)
}

fn bone(name: &str, parent: Option<&str>, at: Value, ops: Vec<Value>) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), json!(name));
    if let Some(p) = parent {
        m.insert("parent".into(), json!(p));
    }
    m.insert("at".into(), at);
    if !ops.is_empty() {
        m.insert("ops".into(), Value::Array(ops));
    }
    Value::Object(m)
}

fn part(part: &str, bone: &str, pivot: Value) -> Value {
    json!({"part": part, "bone": bone, "pivot": pivot})
}

fn by_build(v: [[f32; 3]; 3]) -> Value {
    json!({"by": "build", "values": v.map(|a| round3(a, 4))})
}

fn biped_rigs() -> Value {
    let arm = |s: f32, y: f32| by_build([0, 1, 2].map(|b| [s * (hw_of(b) as f32 + 4.0), y, 0.0]));
    let mut bones = vec![
        bone("pelvis", None, v3(0.0, HIP_Y, 0.0), vec![]),
        bone("thigh.r", Some("pelvis"), v3(LEG_X, 0.0, 0.0), vec![]),
        bone("shin.r", Some("thigh.r"), v3(0.0, KNEE_Y - HIP_Y, 0.0), vec![]),
        bone("thigh.l", Some("pelvis"), v3(-LEG_X, 0.0, 0.0), vec![]),
        bone("shin.l", Some("thigh.l"), v3(0.0, KNEE_Y - HIP_Y, 0.0), vec![]),
        bone("flap.f", Some("pelvis"), v3(0.0, FLAP_Y - HIP_Y, 0.0), vec![json!({"rx": "-shape"})]),
        bone("flap.b", Some("pelvis"), v3(0.0, FLAP_Y - HIP_Y, 0.0), vec![json!({"rx": "shape"})]),
        bone("spine", Some("pelvis"), v3(0.0, WAIST_Y - HIP_Y, 0.0), vec![]),
        bone("cape", Some("spine"), v3(0.0, CAPE_Y[0] - WAIST_Y, CAPE_Z), vec![]),
    ];
    for i in 0..3 {
        let k = i as f32;
        let a = format!(
            "{} + flow*{} + sin(clock*4.5 - {})*{}*(1 + flow)",
            [0.07, 0.05, 0.06][i],
            [0.26, 0.16, 0.12][i],
            k * 0.9,
            0.025 + 0.02 * k
        );
        let (parent, at) = if i == 0 {
            ("cape".to_string(), v3(0.0, 0.0, 0.0))
        } else {
            (format!("cape.{}", i - 1), v3(0.0, -13.0, 0.0))
        };
        bones.push(bone(&format!("cape.{i}"), Some(&parent), at, vec![json!({"rx": a})]));
    }
    for (s, side) in [(1.0, "r"), (-1.0, "l")] {
        bones.push(bone(&format!("upper_arm.{side}"), Some("spine"), arm(s, SHOULDER_Y - WAIST_Y), vec![]));
        bones.push(bone(
            &format!("forearm.{side}"),
            Some(&format!("upper_arm.{side}")),
            v3(0.0, ELBOW_Y - SHOULDER_Y, 0.0),
            vec![],
        ));
    }
    bones.push(bone(
        "grip",
        Some("forearm.r"),
        v3(0.0, 20.5 - ELBOW_Y, 0.5),
        vec![json!({"rx": 1.25}), json!({"rz": -0.22}), json!("pose")],
    ));
    bones.push(bone(
        "head",
        Some("spine"),
        v3(0.0, NECK_Y - WAIST_Y, 0.0),
        vec![json!("pose"), json!({"rx": "-pitch*0.55"}), json!({"scale": HEAD_SCALE})],
    ));
    let mut parts = vec![
        part("thigh.r", "thigh.r", v3(LEG_X, HIP_Y, 0.0)),
        part("shin.r", "shin.r", v3(LEG_X, KNEE_Y, 0.0)),
        part("thigh.l", "thigh.l", v3(-LEG_X, HIP_Y, 0.0)),
        part("shin.l", "shin.l", v3(-LEG_X, KNEE_Y, 0.0)),
        part("skirt", "pelvis", v3(0.0, HIP_Y, 0.0)),
        part("flap.f", "flap.f", v3(0.0, FLAP_Y, 0.0)),
        part("flap.b", "flap.b", v3(0.0, FLAP_Y, 0.0)),
        part("torso", "spine", v3(0.0, WAIST_Y, 0.0)),
    ];
    for i in 0..3 {
        parts.push(part(&format!("cape.{i}"), &format!("cape.{i}"), v3(0.0, CAPE_Y[i], CAPE_Z)));
    }
    for (s, side) in [(1.0, "r"), (-1.0, "l")] {
        parts.push(part(&format!("arm_u.{side}"), &format!("upper_arm.{side}"), arm(s, SHOULDER_Y)));
        parts.push(part(&format!("arm_l.{side}"), &format!("forearm.{side}"), arm(s, ELBOW_Y)));
        if side == "r" {
            parts.push(json!({"bone": "grip", "held": true}));
        }
    }
    parts.push(part("head", "head", v3(0.0, NECK_Y, 0.0)));
    json!({
        "skeleton": "biped",
        "bones": anim::biped::NAMES,
        "upper": ["spine", "head", "upper_arm.r", "upper_arm.l", "forearm.r", "forearm.l", "grip"],
        "step": {"base": 0.55, "per_speed": 0.2, "min": 0.8, "max": 1.45},
        "rigs": {"person": {"bones": bones, "parts": parts}},
    })
}

fn wraith_rigs() -> Value {
    let mut bones = vec![bone("body", None, v3(0.0, S_WAIST[1] + 4.0, 0.0), vec![])];
    for (i, x) in S_TATTER_X.iter().enumerate() {
        let k = i as f32;
        bones.push(bone(
            &format!("tatter.{i}"),
            Some("body"),
            v3(*x, S_HEM_Y - S_WAIST[1], 0.0),
            vec![
                json!({"rx": format!("flow + sin(clock*2.3 + {})*0.1", k * 1.3)}),
                json!({"rz": format!("sin(clock*3 + {})*0.14", k * 2.1)}),
            ],
        ));
    }
    bones.push(bone("hood", Some("body"), v3(0.0, S_NECK[1] - S_WAIST[1], 0.0), vec![]));
    bones.push(bone(
        "eyes",
        Some("hood"),
        v3(0.0, S_EYE[1] - S_NECK[1], S_EYE[2]),
        vec![json!({"scale": [1, "clamp(shape, 0.1, 1.5)", 1]})],
    ));
    for (s, side) in [(1.0, "r"), (-1.0, "l")] {
        bones.push(bone(
            &format!("upper_arm.{side}"),
            Some("body"),
            v3(s * S_SHOULDER.0, S_SHOULDER.1 - S_WAIST[1], 0.0),
            vec![],
        ));
        bones.push(bone(
            &format!("forearm.{side}"),
            Some(&format!("upper_arm.{side}")),
            v3(0.0, S_ELBOW_Y - S_SHOULDER.1, 0.0),
            vec![],
        ));
    }
    let mut parts = vec![part("body", "body", v3(S_WAIST[0], S_WAIST[1], S_WAIST[2]))];
    for (i, x) in S_TATTER_X.iter().enumerate() {
        parts.push(part(&format!("tatter.{i}"), &format!("tatter.{i}"), v3(*x, S_HEM_Y, 0.0)));
    }
    parts.push(part("hood", "hood", v3(S_NECK[0], S_NECK[1], S_NECK[2])));
    parts.push(json!({"part": "eyes", "bone": "eyes", "pivot": v3(S_EYE[0], S_EYE[1], S_EYE[2]), "glow": true, "light": 1.0}));
    for (s, side) in [(1.0, "r"), (-1.0, "l")] {
        parts.push(part(&format!("arm_u.{side}"), &format!("upper_arm.{side}"), v3(s * S_SHOULDER.0, S_SHOULDER.1, 0.0)));
        parts.push(part(&format!("arm_l.{side}"), &format!("forearm.{side}"), v3(s * S_SHOULDER.0, S_ELBOW_Y, 0.0)));
    }
    json!({
        "skeleton": "wraith",
        "bones": anim::wraith::NAMES,
        "upper": ["hood", "upper_arm.r", "upper_arm.l", "forearm.r", "forearm.l"],
        "step": {"base": 1.2, "per_speed": 0.0, "min": 1.2, "max": 1.2},
        "rigs": {"shade": {"bones": bones, "parts": parts}},
    })
}

fn quad_rig_json(neck: [f32; 3], hip: f32, legs: [(f32, f32); 4]) -> Value {
    let names = ["leg.fr", "leg.fl", "leg.br", "leg.bl"];
    let mut bones = vec![bone("body", None, v3(0.0, hip, 0.0), vec![])];
    let mut parts = Vec::new();
    for (i, (x, z)) in legs.iter().enumerate() {
        bones.push(bone(names[i], Some("body"), v3(*x, 0.0, *z), vec![]));
        parts.push(part(names[i], names[i], v3(*x, hip, *z)));
    }
    bones.push(bone("head", Some("body"), v3(neck[0], neck[1] - hip, neck[2]), vec![]));
    parts.push(part("body", "body", v3(0.0, hip, 0.0)));
    parts.push(part("head", "head", v3(neck[0], neck[1], neck[2])));
    json!({"bones": bones, "parts": parts})
}

fn multi_rig_json(legs: &[[f32; 3]], claws: &[[f32; 3]]) -> Value {
    let mut bones = vec![bone("body", None, v3(0.0, 0.0, 0.0), vec![])];
    let mut parts = vec![part("body", "body", v3(0.0, 0.0, 0.0))];
    for (i, p) in legs.iter().enumerate() {
        let n = format!("leg.{i}");
        bones.push(bone(&n, Some("body"), v3(p[0], p[1], p[2]), vec![]));
        parts.push(part(&n, &n, v3(p[0], p[1], p[2])));
    }
    for (i, p) in claws.iter().enumerate() {
        let n = ["claw.r", "claw.l"][i];
        bones.push(bone(n, Some("body"), v3(p[0], p[1], p[2]), vec![]));
        parts.push(part(n, n, v3(p[0], p[1], p[2])));
    }
    json!({"bones": bones, "parts": parts})
}

fn wisp_rigs() -> Value {
    let mut bones = vec![bone("root", None, v3(0.0, 0.85 * FINE, 0.0), vec![])];
    bones.push(bone(
        "core.draw",
        Some("root"),
        v3(0.0, 0.0, 0.0),
        vec![json!({"ry": "clock*0.9"}), json!({"rx": 0.3}), json!({"scale": "max(shape, 0.2)"})],
    ));
    bones.push(bone(
        "veil.draw",
        Some("root"),
        v3(0.0, 0.0, 0.0),
        vec![json!({"ry": "-clock*0.6"}), json!({"scale": "max(veil.ox, 0.2)"})],
    ));
    let mut parts = vec![
        json!({"part": "core", "bone": "core.draw", "glow": true}),
        json!({"part": "veil", "bone": "veil.draw", "glow": true}),
    ];
    for k in 0..5 {
        let kf = k as f32;
        let a = format!("(clock*{} + {})", 1.1 + kf * 0.23, kf * 1.9);
        let r = format!("({}*max(spread, 0.2))", 9.5 + kf * 1.2);
        let th = 0.5 * kf - 1.0;
        bones.push(bone(
            &format!("mote.{k}"),
            Some("root"),
            v3(0.0, 0.0, 0.0),
            vec![
                json!({"rz": th}),
                json!({"move": [format!("cos{a}*{r}"), format!("sin({a}*1.7)*2"), format!("sin{a}*{r}")]}),
                json!({"rz": -th}),
                json!({"ry": format!("{a}*2")}),
                json!({"rx": a}),
            ],
        ));
        parts.push(json!({"part": "mote", "bone": format!("mote.{k}"), "glow": true}));
    }
    let size = "max(shape, 0.2)";
    let fade = format!("min({size}, 1)");
    let dy = "(-(1 - max(flow - 0.25, 0)*0.66))";
    let dz = "(-max(flow, 0))";
    let len = format!("sqrt({dy}*{dy} + {dz}*{dz})");
    for i in 0..4 {
        let kf = i as f32;
        let reach = format!("{}*{fade}", 6.0 + kf * 4.4);
        bones.push(bone(
            &format!("tail.{i}"),
            Some("root"),
            v3(0.0, 0.0, 0.0),
            vec![
                json!({"move": [
                    format!("sin(clock*3.1 - {})*{}", kf * 0.9, 1.0 + kf * 0.8),
                    format!("{dy}/{len}*{reach}"),
                    format!("{dz}/{len}*{reach} + cos(clock*2.3 - {kf})*0.6"),
                ]}),
                json!({"ry": format!("clock + {kf}")}),
                json!({"scale": format!("(1 + sin(clock*5 - {})*0.1)*{fade}", kf * 1.3)}),
            ],
        ));
        parts.push(json!({"part": format!("tail.{i}"), "grid": format!("tail.{i}"), "bone": format!("tail.{i}"), "glow": true}));
    }
    json!({
        "skeleton": "wisp",
        "bones": anim::wisp::NAMES,
        "upper": ["core", "veil"],
        "step": {"base": 1.2, "per_speed": 0.0, "min": 1.2, "max": 1.2},
        "rigs": {"gloomwisp": {"bones": bones, "parts": parts}},
    })
}

// ------------------------------------------------------------------ clips

type PoseFn<'a> = Box<dyn Fn(Anim, &At) -> Pose + 'a>;

/// Everything a pose function gives, flat: Euler angles, offsets, keys.
fn sample(f: &PoseFn, a: Anim, t: f32, s: f32, bones: usize) -> Vec<f32> {
    anim::CAPTURE.with(|c| *c.borrow_mut() = [[0.0; 3]; MAX_BONES]);
    let p = f(a, &At { t, stride: s, seed: 0.3 });
    let e = anim::CAPTURE.with(|c| *c.borrow());
    let mut out = Vec::with_capacity(bones * 6 + 3);
    for b in 0..bones {
        out.extend_from_slice(&e[b]);
        out.extend_from_slice(&p.off[b].to_array());
    }
    out.extend_from_slice(&p.k[..3]);
    out
}

fn channel_name(names: &[&str], i: usize) -> String {
    let n = names.len() * 6;
    if i >= n {
        return ["flow", "shape", "spread"][i - n].to_string();
    }
    let (b, c) = (i / 6, i % 6);
    format!("{}.{}", names[b], ["rx", "ry", "rz", "ox", "oy", "oz"][c])
}

fn default_of(names: &[&str], i: usize) -> f32 {
    if i == names.len() * 6 + 2 { 1.0 } else { 0.0 }
}

/// Simplify a sampled curve (Ramer-Douglas-Peucker, linear between keys).
fn simplify(ts: &[f32], vs: &[f32], tol: f32) -> Vec<(f32, f32)> {
    let n = ts.len();
    if n < 2 {
        return ts.iter().zip(vs).map(|(t, v)| (*t, *v)).collect();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut worst, mut at) = (0.0f32, a);
        for i in a + 1..b {
            let f = (ts[i] - ts[a]) / (ts[b] - ts[a]);
            let line = vs[a] + (vs[b] - vs[a]) * f;
            let d = (vs[i] - line).abs();
            if d > worst {
                worst = d;
                at = i;
            }
        }
        if worst > tol {
            keep[at] = true;
            stack.push((a, at));
            stack.push((at, b));
        }
    }
    (0..n).filter(|i| keep[*i]).map(|i| (ts[i], vs[i])).collect()
}

fn tolerance(names: &[&str], i: usize) -> f32 {
    if i >= names.len() * 6 {
        0.002
    } else if i % 6 < 3 {
        0.0005
    } else {
        0.01
    }
}

/// A channel as JSON: a number when it does not change.
fn channel_json(names: &[&str], i: usize, ts: &[f32], vs: &[f32]) -> Option<Value> {
    let deg = i < names.len() * 6 && i % 6 < 3;
    let conv = |v: f32| if deg { v.to_degrees() } else { v };
    let lo = vs.iter().cloned().fold(f32::MAX, f32::min);
    let hi = vs.iter().cloned().fold(f32::MIN, f32::max);
    if hi - lo <= tolerance(names, i) * 0.25 {
        let v = vs[0];
        if (v - default_of(names, i)).abs() < 1e-5 {
            return None;
        }
        return Some(round(conv(v), 4));
    }
    let keys = simplify(ts, vs, tolerance(names, i));
    Some(Value::Array(
        keys.iter()
            .map(|(t, v)| json!([round(*t, 4), round(conv(*v), 4)]))
            .collect(),
    ))
}

const RATE: f32 = 120.0;

/// Bake one animation of one pose function into a clip.
fn bake_clip(f: &PoseFn, a: Anim, names: &[&str]) -> (Value, f32) {
    let nb = names.len();
    let nch = nb * 6 + 3;
    let at = |t: f32, s: f32| sample(f, a, t, s, nb);
    // does the stride move it?
    let s_probe = [0.0f32, 0.9, 2.1, 3.7, 5.2];
    let strided = s_probe.iter().any(|s| {
        let (x, y) = (at(0.37, 0.0), at(0.37, *s));
        x.iter().zip(&y).any(|(p, q)| (p - q).abs() > 1e-5)
    });
    let mut clip = Map::new();
    let mut time = Map::new();
    let mut err = 0.0f32;
    let (len, one_shot) = match a.duration() {
        Some(d) => (d, true),
        None => (find_loop(&|t| at(t, 0.0), nch), false),
    };
    let n = (len * RATE).round().max(1.0) as usize;
    let ts: Vec<f32> = (0..=n).map(|i| i as f32 * len / n as f32).collect();
    let mut series: Vec<Vec<f32>> = vec![Vec::with_capacity(ts.len()); nch];
    for t in &ts {
        for (c, v) in at(*t, 0.0).into_iter().enumerate() {
            series[c].push(v);
        }
    }
    if !one_shot {
        // close the loop: the last stretch eases into the start
        let blend = (len * 0.25).min(1.0);
        for (i, t) in ts.iter().enumerate() {
            if *t > len - blend {
                let w = ((t - (len - blend)) / blend).clamp(0.0, 1.0);
                let w = w * w * (3.0 - 2.0 * w);
                let early = at(t - len, 0.0);
                for c in 0..nch {
                    series[c][i] += (early[c] - series[c][i]) * w;
                }
            }
        }
        // a loop's first and last keys meet
        for s in series.iter_mut() {
            let first = s[0];
            *s.last_mut().unwrap() = first;
        }
    }
    for c in 0..nch {
        if let Some(v) = channel_json(names, c, &ts, &series[c]) {
            time.insert(channel_name(names, c), v);
        }
    }
    if one_shot {
        clip.insert("duration".into(), round(len, 4));
        if a == Anim::Death {
            clip.insert("hold".into(), json!(true));
        }
    } else {
        clip.insert("loop".into(), round(len, 4));
    }
    clip.insert("time".into(), Value::Object(time));
    if strided {
        // what the stride adds, over one cycle
        let m = 256;
        let us: Vec<f32> = (0..=m).map(|i| i as f32 / m as f32).collect();
        let zero = at(0.0, 0.0);
        let mut ss: Vec<Vec<f32>> = vec![Vec::new(); nch];
        for u in &us {
            for (c, v) in at(0.0, u * std::f32::consts::TAU).into_iter().enumerate() {
                ss[c].push(v - zero[c]);
            }
        }
        let mut stride = Map::new();
        for c in 0..nch {
            if ss[c].iter().all(|v| v.abs() < 1e-5) {
                continue;
            }
            let keys = simplify(&us, &ss[c], tolerance(names, c));
            let deg = c < nb * 6 && c % 6 < 3;
            stride.insert(
                channel_name(names, c),
                Value::Array(
                    keys.iter()
                        .map(|(u, v)| {
                            json!([round(*u, 4), round(if deg { v.to_degrees() } else { *v }, 4)])
                        })
                        .collect(),
                ),
            );
        }
        clip.insert("stride".into(), Value::Object(stride));
        // how well time + stride rebuilds the pose
        for t in [0.13f32, 0.71, 1.9] {
            for s in [0.4f32, 1.7, 3.3, 5.9] {
                let want = at(t, s);
                let (base, add) = (at(t, 0.0), at(0.0, s));
                for c in 0..nch {
                    err = err.max((base[c] + add[c] - zero[c] - want[c]).abs());
                }
            }
        }
    }
    (Value::Object(clip), err)
}

/// The length of a loop: the shortest period that repeats, or about half a
/// minute with the ends blended.
fn find_loop(at: &dyn Fn(f32) -> Vec<f32>, nch: usize) -> f32 {
    let rate = 60.0;
    let total = 46.0;
    let n = (total * rate) as usize;
    let s: Vec<Vec<f32>> = (0..n).map(|i| at(i as f32 / rate)).collect();
    let window = (8.0 * rate) as usize;
    let mut best = (f32::MAX, 33.0f32);
    for lag in (rate as usize / 2)..(n - window) {
        let mut e = 0.0f32;
        'w: for i in 0..window {
            for c in 0..nch {
                e = e.max((s[i + lag][c] - s[i][c]).abs());
                if e > best.0 && e > 0.003 {
                    break 'w;
                }
            }
        }
        let l = lag as f32 / rate;
        if e < 0.003 {
            return l;
        }
        if (20.0..=40.0).contains(&l) && e < best.0 {
            best = (e, l);
        }
    }
    // idle fidgets come every 11 s, three kinds
    if (best.1 - 33.0).abs() < 1.0 { 33.0 } else { best.1 }
}

fn bake_clips(
    skeleton: &str,
    names: &[&str],
    sets: Vec<(String, PoseFn)>,
) -> (Value, Vec<String>) {
    let mut report = Vec::new();
    let mut out = Map::new();
    let mut default = Map::new();
    for (set, f) in &sets {
        let mut clips = Map::new();
        for &a in Anim::ALL {
            let (clip, err) = bake_clip(f, a, names);
            if err > 0.01 {
                report.push(format!("{skeleton}/{set} {a:?}: stride split off by {err:.4}"));
            }
            // named sets hold only what differs from the default
            if set == "default" || default.get(a.name()) != Some(&clip) {
                clips.insert(a.name().to_string(), clip);
            }
        }
        if set == "default" {
            default = clips.clone();
        }
        out.insert(set.clone(), Value::Object(clips));
    }
    (
        json!({"skeleton": skeleton, "seed": 0.3, "sets": out}),
        report,
    )
}

// ------------------------------------------------------------------ bake

fn pack_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn write_json(root: &Path, path: &str, v: &Value) {
    let p = root.join(path);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, json_text(v)).unwrap();
}

#[test]
#[ignore = "one-off: writes the content pack's figures"]
fn bake_figures() {
    let root = pack_dir();
    let only = std::env::var("BAKE").ok();
    let want = |what: &str| only.as_deref().is_none_or(|o| o.split(',').any(|x| x == what));
    let mut figures = Map::new();
    let start = std::time::Instant::now();
    // ---- people
    if want("person") {
        let recipes = Mutex::new(Recipes::default());
        let mut groups = Vec::new();
        for g in person_groups() {
            let t = std::time::Instant::now();
            let layers = bake_group(&g, &recipes).unwrap();
            println!(
                "person/{}: {} layers, {} voxels, {:.1}s",
                g.name,
                layers.len(),
                layers.iter().map(|l| l.content.len()).sum::<usize>(),
                t.elapsed().as_secs_f32()
            );
            groups.push((g.name.clone(), g.map.clone(), layers));
        }
        let grids = vec![GridOut {
            name: "main".into(),
            bounds: Bounds {
                min: [-28, 0, -16],
                max: [28, 82, 16],
            },
            parts: PERSON_PARTS.iter().map(|s| s.to_string()).collect(),
            groups,
        }];
        let show = Look::player(0, [0.62, 0.18, 0.16]);
        let shown = |s: &str| -> [f32; 3] {
            match s {
                "skin" => show.skin,
                "hair" => show.hair,
                "eyes" => show.eyes,
                "tunic" => show.tunic,
                "trim" => show.trim,
                "pants" => show.pants,
                "boots" => show.boots,
                "apron" => [0.92, 0.90, 0.82],
                "cape" => show.cape.unwrap_or([0.3, 0.1, 0.1]),
                "scarf" => [0.52, 0.12, 0.10],
                _ => [0.72, 0.13, 0.10],
            }
        };
        let mut extra = Map::new();
        extra.insert("skeleton".into(), json!("biped"));
        extra.insert("rig".into(), json!("person"));
        extra.insert("clips".into(), json!({"features": ["robe"]}));
        let rec = recipes.into_inner().unwrap();
        let f = write_figure(&root, "person", &grids, &rec, &SLOTS, &shown, extra).unwrap();
        figures.insert("person".into(), f);
    }
    // ---- creatures
    type Build = fn([f32; 3]);
    let creatures: [(&str, Build, [f32; 3], &[&str], &str, &str); 5] = [
        ("grazer", |c| drop(build_grazer(c)), [0.86, 0.82, 0.7], &["", "body", "head", "leg.fr", "leg.fl", "leg.br", "leg.bl"], "quadruped", "grazer"),
        ("thornboar", |c| drop(build_thornboar(c)), [0.33, 0.22, 0.15], &["", "body", "head", "leg.fr", "leg.fl", "leg.br", "leg.bl"], "quadruped", "thornboar"),
        ("shade", |c| drop(build_shade(c)), [0.16, 0.12, 0.22], &["", "body", "hood", "eyes", "arm_u.r", "arm_u.l", "arm_l.r", "arm_l.l", "tatter.0", "tatter.1", "tatter.2", "tatter.3", "tatter.4"], "wraith", "shade"),
        ("mirepincher", |c| drop(build_mirepincher(c)), [0.38, 0.42, 0.24], &["", "body", "claw.r", "claw.l", "leg.0", "leg.1", "leg.2", "leg.3", "leg.4", "leg.5"], "multileg", "mirepincher"),
        ("websnare", |c| drop(build_websnare(c)), [0.80, 0.78, 0.72], &["", "body", "leg.0", "leg.1", "leg.2", "leg.3", "leg.4", "leg.5", "leg.6", "leg.7"], "multileg", "websnare"),
    ];
    for (name, build, show, parts, skeleton, rig) in creatures {
        if !want(name) {
            continue;
        }
        let recipes = Mutex::new(Recipes::default());
        let mut g = single(name, 0, Vec::new(), Box::new(move |_, _, c| build(c("color"))), [0.30, 0.33, 0.36]);
        let (bounds, layers) = bake_single(&mut g, &recipes).unwrap();
        let grids = vec![GridOut {
            name: "main".into(),
            bounds,
            parts: parts.iter().map(|s| s.to_string()).collect(),
            groups: vec![(name.to_string(), Vec::new(), layers)],
        }];
        let mut extra = Map::new();
        extra.insert("skeleton".into(), json!(skeleton));
        extra.insert("rig".into(), json!(rig));
        extra.insert("clips".into(), json!(rig));
        let rec = recipes.into_inner().unwrap();
        let f = write_figure(&root, name, &grids, &rec, &["color"], &|_| show, extra).unwrap();
        figures.insert(name.into(), f);
        println!("{name}: done");
    }
    if want("gloomwisp") {
        let recipes = Mutex::new(Recipes::default());
        let mut grids = Vec::new();
        for (i, gname) in ["main", "tail.0", "tail.1", "tail.2", "tail.3"].into_iter().enumerate() {
            let mut g = single(
                if i == 0 { "gloomwisp" } else { gname },
                i,
                Vec::new(),
                Box::new(|_, _, c| drop(build_wisp(c("color")))),
                [0.30, 0.33, 0.36],
            );
            let (bounds, layers) = bake_single(&mut g, &recipes).unwrap();
            let parts = if i == 0 {
                vec!["".to_string(), "core".into(), "veil".into(), "mote".into()]
            } else {
                vec!["".to_string(), gname.to_string()]
            };
            grids.push(GridOut {
                name: gname.into(),
                bounds,
                parts,
                groups: vec![(g.name.clone(), Vec::new(), layers)],
            });
        }
        let mut extra = Map::new();
        extra.insert("skeleton".into(), json!("wisp"));
        extra.insert("rig".into(), json!("gloomwisp"));
        extra.insert("clips".into(), json!("default"));
        let rec = recipes.into_inner().unwrap();
        let f = write_figure(&root, "gloomwisp", &grids, &rec, &["color"], &|_| [0.35, 0.85, 1.0], extra).unwrap();
        figures.insert("gloomwisp".into(), f);
    }
    // ---- held things
    let mut kinds = Map::new();
    for kind in KINDS {
        let name = kind_name(kind);
        let mut features = vec![("quality", vec![1u8, 0, 2, 3, 4]), ("glows", vec![0u8, 1])];
        if kind == Kind::Torch {
            features.push(("frame", vec![0, 1, 2]));
        }
        let (real_unit, carry) = {
            let h = build_held(kind, [0.5; 3], 1, false, 0);
            (h.unit, h.carry_turn)
        };
        let grip = match kind {
            Kind::BattleAxe | Kind::Staff | Kind::Polearm => "two_hand",
            Kind::Bow => "bow",
            _ => "one_hand",
        };
        let fp: [f32; 4] = match kind {
            Kind::Staff | Kind::Polearm => [0.36, 0.4, 0.15, 0.0],
            Kind::Sword | Kind::BattleAxe => [0.5, 0.35, 0.18, 0.0],
            Kind::Bow => [0.42, 0.2, 0.12, 0.12],
            Kind::Shield => [0.5, -0.1, 0.1, 0.1],
            Kind::Mace | Kind::Dagger | Kind::Wand => [0.6, 0.35, 0.18, 0.0],
            Kind::Pickaxe | Kind::Hatchet | Kind::Shovel | Kind::Hoe => [0.58, 0.35, 0.18, 0.0],
            Kind::Tome => [0.7, -0.2, 0.1, 0.05],
            Kind::Torch => [0.62, 0.25, 0.12, 0.03],
            Kind::Hand => [0.8, 0.75, 0.35, -0.02],
            _ => [0.75, -0.35, 0.15, 0.03],
        };
        kinds.insert(
            name.into(),
            json!({
                "figure": format!("held.{name}"),
                "unit": real_unit,
                "carry_turn": round(carry, 6),
                "grip": grip,
                "fp": fp.map(|x| round(x, 4)),
            }),
        );
        if !want("held") {
            continue;
        }
        let recipes = Mutex::new(Recipes::default());
        let mut g = single(
            &format!("held.{name}"),
            0,
            features,
            Box::new(move |_, f, c| {
                drop(build_held(kind, c("color"), f("quality"), f("glows") != 0, f("frame")));
            }),
            [0.42, 0.47, 0.52],
        );
        let (bounds, layers) = bake_single(&mut g, &recipes).unwrap();
        let grids = vec![GridOut {
            name: "main".into(),
            bounds,
            parts: vec!["".into(), "item".into()],
            groups: vec![(g.name.clone(), Vec::new(), layers)],
        }];
        let mut extra = Map::new();
        extra.insert("lods".into(), json!(1));
        let rec = recipes.into_inner().unwrap();
        let fname = format!("held.{name}");
        let f = write_figure(&root, &fname, &grids, &rec, &["color"], &|_| [0.62, 0.63, 0.68], extra).unwrap();
        figures.insert(fname, f);
    }
    println!("models: {:.1}s", start.elapsed().as_secs_f32());
    // ---- rigs
    let quads = json!({
        "skeleton": "quadruped",
        "bones": anim::quad::NAMES,
        "upper": ["head"],
        "step": {"base": 0.4, "per_speed": 0.1, "min": 0.5, "max": 0.95},
        "rigs": {
            "grazer": quad_rig_json(G_NECK, G_HIP_Y, G_LEGS),
            "thornboar": quad_rig_json([0.0, 19.0, 11.0], 11.0, [(5.5, 8.5), (-5.5, 8.5), (5.5, -9.5), (-5.5, -9.5)]),
        },
    });
    let crab_legs: Vec<[f32; 3]> = [1.0f32, -1.0]
        .iter()
        .flat_map(|s| [-5.0f32, -0.5, 4.0].map(|x| [x, 7.5, s * 9.0]))
        .collect();
    let spider_legs: Vec<[f32; 3]> = [1.0f32, -1.0]
        .iter()
        .flat_map(|s| [9.0f32, 6.0, 3.0, 0.0].map(|z| [s * 4.5, 10.0, z]))
        .collect();
    let multis = json!({
        "skeleton": "multileg",
        "bones": anim::multi::NAMES,
        "order": {"claw.r": "zyx", "claw.l": "zyx"},
        "upper": ["claw.r", "claw.l"],
        "step": {"base": 0.3, "per_speed": 0.05, "min": 0.35, "max": 0.55},
        "rigs": {
            "mirepincher": multi_rig_json(&crab_legs, &[[7.0, 7.5, 7.5], [7.0, 7.5, -7.5]]),
            "websnare": multi_rig_json(&spider_legs, &[]),
        },
    });
    if want("rigs") {
        write_json(&root, "rigs/biped.json", &biped_rigs());
        write_json(&root, "rigs/wraith.json", &wraith_rigs());
        write_json(&root, "rigs/quadruped.json", &quads);
        write_json(&root, "rigs/multileg.json", &multis);
        write_json(&root, "rigs/wisp.json", &wisp_rigs());
    }
    // ---- clips
    if want("clips") {
        let mut report = Vec::new();
        let mut biped_sets: Vec<(String, PoseFn)> = Vec::new();
        for (gname, grip) in [
            ("empty", Grip::Empty),
            ("one_hand", Grip::OneHand),
            ("two_hand", Grip::TwoHand),
            ("bow", Grip::Bow),
        ] {
            for robe in [false, true] {
                let name = if gname == "empty" && !robe {
                    "default".to_string()
                } else if robe {
                    format!("{gname}+robe")
                } else {
                    gname.to_string()
                };
                let rig = BipedRig { grip, robe };
                biped_sets.push((name, Box::new(move |a, at| anim::biped(a, at, &rig))));
            }
        }
        let grazer = quad_rig(&build_grazer([0.8, 0.8, 0.7])[0]);
        let boar = quad_rig(&build_thornboar([0.33, 0.22, 0.15])[0]);
        let crab = multi_rig(&build_mirepincher([0.38, 0.42, 0.24])[0]);
        let spider = multi_rig(&build_websnare([0.8, 0.78, 0.72])[0]);
        for (skeleton, names, sets) in [
            ("biped", &anim::biped::NAMES[..], biped_sets),
            ("wraith", &anim::wraith::NAMES[..], vec![("default".to_string(), Box::new(anim::wraith) as PoseFn)]),
            (
                "quadruped",
                &anim::quad::NAMES[..],
                vec![
                    ("default".to_string(), Box::new(move |a: Anim, at: &At| anim::quadruped(a, at, &grazer)) as PoseFn),
                    ("thornboar".to_string(), Box::new(move |a: Anim, at: &At| anim::quadruped(a, at, &boar)) as PoseFn),
                ],
            ),
            (
                "multileg",
                &anim::multi::NAMES[..],
                vec![
                    ("default".to_string(), Box::new(move |a: Anim, at: &At| anim::multileg(a, at, &crab)) as PoseFn),
                    ("websnare".to_string(), Box::new(move |a: Anim, at: &At| anim::multileg(a, at, &spider)) as PoseFn),
                ],
            ),
            ("wisp", &anim::wisp::NAMES[..], vec![("default".to_string(), Box::new(anim::wisp) as PoseFn)]),
        ] {
            let t = std::time::Instant::now();
            let (v, r) = bake_clips(skeleton, names, sets);
            report.extend(r);
            write_json(&root, &format!("anims/{skeleton}.json"), &v);
            println!("anims/{skeleton}: {:.1}s", t.elapsed().as_secs_f32());
        }
        for r in report {
            println!("{r}");
        }
    }
    // ---- figures.json
    if only.is_none() || want("figures") {
        let path = root.join("figures.json");
        let mut all: Map<String, Value> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let mut figs = all.get("figures").and_then(|f| f.as_object()).cloned().unwrap_or_default();
        for (k, v) in figures {
            figs.insert(k, v);
        }
        for (k, v) in look_rules(kinds) {
            all.insert(k, v);
        }
        all.insert("figures".into(), Value::Object(figs));
        std::fs::write(&path, json_text(&Value::Object(all))).unwrap();
    }
    println!("bake: {:.1}s", start.elapsed().as_secs_f32());
}

/// Everything in figures.json besides the figures: looks, creatures, held.
fn look_rules(kinds: Map<String, Value>) -> Map<String, Value> {
    let c3 = |c: [f32; 3]| round3(c, 4);
    let list = |v: &[[f32; 3]]| Value::Array(v.iter().map(|c| c3(*c)).collect());
    let mut m = Map::new();
    m.insert(
        "appearance".into(),
        json!({
            "colors": {
                "skin": {"field": "skin", "options": list(&Look::SKINS)},
                "hair": {"field": "hair_color", "options": list(&Look::HAIRS)},
                "eyes": {"field": "eyes", "options": list(&Look::EYES)},
            },
            "features": {
                "hair_style": {"field": "hair_style", "max": 4},
                "beard": {"field": "beard", "max": 1},
                "build": {"field": "build", "max": 2},
                "mood": {"field": "mood", "max": 2},
                "ears": {"field": "ears", "max": 2},
            },
        }),
    );
    m.insert(
        "player".into(),
        json!({
            "figure": "person",
            "colors": {
                "skin": {"pick": list(&[[0.96, 0.80, 0.66], [0.87, 0.68, 0.52], [0.72, 0.52, 0.38], [0.55, 0.38, 0.27], [0.98, 0.86, 0.75]]), "shift": 0},
                "hair": {"pick": list(&[[0.18, 0.12, 0.08], [0.45, 0.28, 0.12], [0.85, 0.68, 0.35], [0.62, 0.22, 0.10], [0.12, 0.12, 0.14], [0.85, 0.85, 0.82]]), "shift": 3},
                "eyes": {"pick": list(&[[0.25, 0.42, 0.70], [0.30, 0.52, 0.30], [0.42, 0.28, 0.16], [0.45, 0.48, 0.55]]), "shift": 14},
                "tunic": {"from": "color"},
                "trim": c3([0.85, 0.68, 0.25]),
                "pants": c3([0.30, 0.24, 0.20]),
                "boots": c3([0.22, 0.15, 0.10]),
                "cape": {"from": "color", "scale": 0.6},
            },
            "features": {
                "hair_style": {"pick": [1, 2], "shift": 6},
                "beard": {"pick": [1, 0, 0], "shift": 8},
                "pauldrons": {"pick": [1, 0], "shift": 10},
                "armor": {"pick": [0, 1, 2, 3], "shift": 12},
                "emblem": {"pick": [1, 2, 3, 4], "shift": 16},
                "ears": {"pick": [2, 1, 1, 1, 1], "shift": 18},
                "mood": {"pick": [0, 1, 2], "shift": 21},
                "gloves": {"pick": [0, 1, 1, 1], "by": "armor"},
                "build": 1,
            },
        }),
    );
    m.insert(
        "npcs".into(),
        json!({
            "*": {"colors": {"cape": null}, "features": {"ears": 1}, "remap": {"armor": {"2": 1, "3": 1}}},
            "brannoc": {
                "colors": {"skin": c3([0.92, 0.76, 0.62]), "hair": c3([0.90, 0.90, 0.88]), "tunic": c3([0.45, 0.25, 0.18]), "trim": c3([0.90, 0.70, 0.30]), "eyes": c3([0.35, 0.50, 0.70])},
                "features": {"hair_style": 2, "beard": 1, "elder": 1, "robe": 1, "armor": 0, "gloves": 0, "pauldrons": 0, "emblem": 0, "build": 0, "mood": 2},
            },
            "maren": {
                "colors": {"skin": c3([0.96, 0.80, 0.66]), "hair": c3([0.68, 0.22, 0.10]), "tunic": c3([0.20, 0.30, 0.62]), "trim": c3([0.85, 0.75, 0.40]), "cape": c3([0.12, 0.18, 0.42]), "eyes": c3([0.28, 0.55, 0.36])},
                "features": {"hair_style": 2, "beard": 0, "pauldrons": 1, "armor": 2, "gloves": 1, "emblem": 4, "mood": 1},
                "weapon": ["Sword", c3([0.72, 0.72, 0.78])],
            },
            "tilda": {
                "colors": {"skin": c3([0.97, 0.82, 0.70]), "hair": c3([0.45, 0.28, 0.12]), "tunic": c3([0.62, 0.38, 0.22]), "trim": c3([0.80, 0.30, 0.25]), "apron": c3([0.92, 0.90, 0.82]), "eyes": c3([0.42, 0.28, 0.16])},
                "features": {"hair_style": 4, "beard": 0, "pauldrons": 0, "armor": 0, "gloves": 0, "robe": 1, "emblem": 0, "build": 0, "mood": 0},
            },
            "hollis": {
                "colors": {"skin": c3([0.72, 0.52, 0.38]), "hair": c3([0.20, 0.14, 0.10]), "tunic": c3([0.40, 0.36, 0.34]), "apron": c3([0.35, 0.24, 0.14]), "eyes": c3([0.30, 0.22, 0.14])},
                "features": {"hair_style": 0, "beard": 1, "pauldrons": 0, "armor": 1, "gloves": 1, "emblem": 0, "build": 2, "mood": 1},
            },
        }),
    );
    let ashcloak = |leader: bool| -> Value {
        json!({
            "seed": if leader { 11 } else { 5 },
            "colors": {
                "skin": c3(if leader { [0.72, 0.52, 0.38] } else { [0.84, 0.66, 0.52] }),
                "hair": c3([0.16, 0.12, 0.10]),
                "tunic": {"from": "color"},
                "trim": c3([0.50, 0.42, 0.30]),
                "pants": c3([0.22, 0.20, 0.19]),
                "boots": c3([0.20, 0.14, 0.10]),
                "cape": {"from": "color", "scale": 0.78},
                "eyes": c3([0.30, 0.24, 0.18]),
                "apron": null,
                "scarf": if leader { c3([0.52, 0.12, 0.10]) } else { json!({"from": "color", "scale": 0.5}) },
                "sash": if leader { c3([0.72, 0.13, 0.10]) } else { Value::Null },
            },
            "features": {
                "hair_style": 3, "beard": 0, "armor": if leader { 2 } else { 1 }, "pauldrons": leader as u8,
                "gloves": 1, "robe": 0, "helmet": 0, "build": if leader { 2 } else { 1 }, "elder": 0,
                "emblem": 0, "ears": 0, "mood": 1,
            },
        })
    };
    m.insert(
        "creatures".into(),
        json!({
            "Quadruped/0": {"figure": "grazer"},
            "Quadruped/1": {"figure": "thornboar"},
            "Quadruped/2": {"figure": "mirepincher"},
            "Quadruped/3": {"figure": "websnare"},
            "Quadruped/*": {"figure": "grazer"},
            "Humanoid/1": {"figure": "person", "look": ashcloak(false), "held": ["dagger", c3([0.62, 0.63, 0.68])]},
            "Humanoid/2": {"figure": "person", "look": ashcloak(true), "held": ["battle_axe", c3([0.55, 0.56, 0.62])]},
            "Humanoid/3": {
                "figure": "person",
                "look": {
                    "seed": 2,
                    "colors": {
                        "skin": {"from": "color"}, "tunic": c3([0.36, 0.27, 0.19]), "trim": c3([0.52, 0.40, 0.24]),
                        "pants": c3([0.24, 0.19, 0.15]), "boots": {"from": "color", "scale": 0.7}, "cape": null,
                        "apron": null, "eyes": c3([0.98, 0.78, 0.20]),
                    },
                    "features": {
                        "hair_style": 0, "beard": 0, "armor": 0, "pauldrons": 0, "gloves": 0, "robe": 0, "helmet": 0,
                        "build": 0, "elder": 0, "emblem": 0, "ears": 3, "mood": 0, "candle": 1,
                    },
                },
                "held": ["pickaxe", c3([0.50, 0.50, 0.55])],
            },
            "Humanoid/*": {"figure": "shade"},
            "Orb/*": {"figure": "gloomwisp"},
        }),
    );
    m.insert(
        "held".into(),
        json!({
            "kinds": kinds,
            "weapons": {"Sword": "sword", "Axe": "battle_axe", "Mace": "mace", "Dagger": "dagger", "Staff": "staff", "Polearm": "polearm", "Bow": "bow", "Wand": "wand", "Shield": "shield", "Offhand": "tome"},
            "tools": {"Pickaxe": "pickaxe", "Axe": "hatchet", "Shovel": "shovel", "Hoe": "hoe", "Sword": "sword"},
            "slots": {"Head": "helm", "Ring": "ring", "Neck": "ring", "Trinket": "ring", "*": "garment"},
            "icons": {"Stick": "stick", "Ingot": "ingot", "Gem": "gem", "Dust": "dust", "Berries": "berries", "Meat": "meat", "Lump": "lump", "Block": "lump", "Tool": "lump"},
            "torch": "torch",
            "hand": ["hand", c3([0.91, 0.76, 0.62])],
            "glow_names": ["Ember"],
        }),
    );
    m
}

// ------------------------------------------------------------ checking

fn compare(what: &str, old: &Snapshot, new: &Snapshot, bad: &mut Vec<String>) {
    if old.voxels.len() != new.voxels.len() {
        bad.push(format!("{what}: {} voxels, now {}", old.voxels.len(), new.voxels.len()));
        return;
    }
    let mut worst = 0.0f32;
    for (a, b) in old.voxels.iter().zip(&new.voxels) {
        if a.0 != b.0 || a.1 != b.1 || a.3 != b.3 {
            bad.push(format!("{what}: at {:?} {:?} now at {:?} {:?}", a.0, (a.1, a.3), b.0, (b.1, b.3)));
            return;
        }
        for i in 0..3 {
            worst = worst.max((a.2[i] - b.2[i]).abs());
        }
    }
    if worst > 2e-5 {
        bad.push(format!("{what}: colours off by {worst}"));
    }
}

fn old_person(l: &Look) -> Snapshot {
    take_snapshots();
    drop(build_person(l));
    take_snapshots().swap_remove(0)
}

fn new_grids(name: &str, look: &figure::Look) -> Vec<Snapshot> {
    let f = figure::figure(name).unwrap_or_else(|| panic!("no figure {name}"));
    f.compose(look).iter().map(Snapshot::of).collect()
}

/// A new look with the features and colours of an old one.
fn new_look(l: &Look) -> figure::Look {
    let mut n = figure::Look::default();
    for (k, v) in [
        ("hair_style", l.hair_style),
        ("beard", l.beard as u8),
        ("elder", l.elder as u8),
        ("ears", l.ears),
        ("mood", l.mood),
        ("helmet", l.helmet as u8),
        ("candle", l.candle as u8),
        ("build", l.build),
        ("armor", ARMORS.iter().position(|a| *a == l.armor).unwrap() as u8),
        ("robe", l.robe as u8),
        ("pauldrons", l.pauldrons as u8),
        ("gloves", l.gloves as u8),
        ("emblem", l.emblem),
    ] {
        n.set(k, v);
    }
    for (k, c) in [
        ("skin", Some(l.skin)),
        ("hair", Some(l.hair)),
        ("eyes", Some(l.eyes)),
        ("tunic", Some(l.tunic)),
        ("trim", Some(l.trim)),
        ("pants", Some(l.pants)),
        ("boots", Some(l.boots)),
        ("apron", l.apron),
        ("cape", l.cape),
        ("scarf", l.scarf),
        ("sash", l.sash),
    ] {
        if let Some(c) = c {
            n.colors.insert(k.into(), c);
        }
    }
    n.weapon = l.weapon;
    n
}

#[test]
#[ignore = "one-off: checks the baked figures against the builders"]
fn bake_matches_builders() {
    crate::assets::load_dir(&pack_dir()).unwrap();
    let mut bad = Vec::new();
    let mut checked = 0;
    // people, through the new looks
    let mut pairs: Vec<(String, Look, figure::Look)> = Vec::new();
    for seed in 0..120u32 {
        let c = crate::game::player_color(seed);
        pairs.push((format!("player {seed}"), Look::player(seed, c), figure::Look::player(seed, c)));
    }
    for key in ["brannoc", "maren", "tilda", "hollis", "smith", "guard", "x"] {
        let c = [0.4, 0.5, 0.6];
        pairs.push((format!("npc {key}"), Look::npc(key, c), figure::Look::npc(key, c)));
    }
    for hs in 0..5u8 {
        for beard in [false, true] {
            for build in 0..3u8 {
                for mood in 0..3u8 {
                    for ears in 0..3u8 {
                        let a = dyno_protocol::Appearance {
                            skin: hs + build,
                            hair_color: mood * 3 + ears,
                            eyes: hs,
                            hair_style: hs,
                            beard,
                            build,
                            mood,
                            ears,
                        };
                        let c = [0.3, 0.5, 0.7];
                        pairs.push((
                            format!("appearance {a:?}"),
                            Look::from_appearance(&a, c),
                            figure::Look::from_appearance(&a, c),
                        ));
                    }
                }
            }
        }
    }
    // the studio's and the sheet's
    for i in 0..6u32 {
        let mut l = Look::player(i + 1, crate::game::player_color(i));
        l.armor = ARMORS[(i % 4) as usize];
        l.helmet = i == 4;
        l.hair_style = if i == 5 { 3 } else { l.hair_style };
        l.robe = i == 2;
        if i == 2 {
            l.cape = None;
        }
        pairs.push((format!("studio {i}"), l, new_look(&l)));
    }
    for (name, look, new) in &pairs {
        compare(name, &old_person(look), &new_grids("person", new)[0], &mut bad);
        checked += 1;
    }
    // creatures
    let content = dyno_world::content::Content::default_pack();
    for cr in &content.creatures {
        let (fig, look, _) = figure::creature(&format!("{:?}", cr.model), cr.variant, cr.color).unwrap();
        let old: Vec<Snapshot> = if Skeleton::of(cr.model, cr.variant) == Skeleton::Biped {
            vec![old_person(&humanoid_look(cr.variant, cr.color).0)]
        } else {
            take_snapshots();
            match (cr.model, cr.variant) {
                (dyno_world::content::Model::Orb, _) => drop(build_wisp(cr.color)),
                (_, 1) if Skeleton::of(cr.model, cr.variant) == Skeleton::Quadruped => {
                    drop(build_thornboar(cr.color))
                }
                (_, 2) if Skeleton::of(cr.model, cr.variant) == Skeleton::Multileg => {
                    drop(build_mirepincher(cr.color))
                }
                _ => match Skeleton::of(cr.model, cr.variant) {
                    Skeleton::Multileg => drop(build_websnare(cr.color)),
                    Skeleton::Wraith => drop(build_shade(cr.color)),
                    _ => drop(build_grazer(cr.color)),
                },
            }
            take_snapshots()
        };
        let new = new_grids(&fig, &look);
        if old.len() != new.len() {
            bad.push(format!("{}: {} grids, now {}", cr.key, old.len(), new.len()));
        }
        for (i, (o, n)) in old.iter().zip(&new).enumerate() {
            compare(&format!("{} grid {i}", cr.key), o, n, &mut bad);
        }
        checked += 1;
    }
    // held things
    for kind in KINDS {
        for quality in 0..5u8 {
            for glows in [false, true] {
                for frame in 0..if kind == Kind::Torch { 3 } else { 1 } {
                    for color in [[0.72, 0.72, 0.78], [0.85, 0.3, 0.25], [0.95, 0.9, 0.2]] {
                        take_snapshots();
                        drop(build_held(kind, color, quality, glows, frame));
                        let old = take_snapshots().swap_remove(0);
                        let mut look = figure::Look::default();
                        look.colors.insert("color".into(), color);
                        look.set("quality", quality);
                        look.set("glows", glows as u8);
                        look.set("frame", frame);
                        let new = &new_grids(&format!("held.{}", kind_name(kind)), &look)[0];
                        compare(&format!("{kind:?} q{quality} g{glows} f{frame} {color:?}"), &old, new, &mut bad);
                        checked += 1;
                    }
                }
            }
        }
    }
    for b in bad.iter().take(30) {
        println!("{b}");
    }
    println!("{checked} looks checked, {} differ", bad.len());
    assert!(bad.is_empty());
}

#[test]
#[ignore = "one-off: checks the baked clips against the pose functions"]
fn clips_match_poses() {
    crate::assets::load_dir(&pack_dir()).unwrap();
    let grazer = quad_rig(&build_grazer([0.8, 0.8, 0.7])[0]);
    let boar = quad_rig(&build_thornboar([0.33, 0.22, 0.15])[0]);
    let crab = multi_rig(&build_mirepincher([0.38, 0.42, 0.24])[0]);
    let spider = multi_rig(&build_websnare([0.8, 0.78, 0.72])[0]);
    let mut cases: Vec<(&str, String, PoseFn)> = Vec::new();
    for (gname, grip) in [
        ("empty", Grip::Empty),
        ("one_hand", Grip::OneHand),
        ("two_hand", Grip::TwoHand),
        ("bow", Grip::Bow),
    ] {
        for robe in [false, true] {
            let rig = BipedRig { grip, robe };
            let set = if robe { format!("{gname}+robe") } else { gname.to_string() };
            cases.push(("person", set, Box::new(move |a, at| anim::biped(a, at, &rig))));
        }
    }
    cases.push(("shade", "default".into(), Box::new(anim::wraith)));
    cases.push(("grazer", "default".into(), Box::new(move |a, at| anim::quadruped(a, at, &grazer))));
    cases.push(("thornboar", "thornboar".into(), Box::new(move |a, at| anim::quadruped(a, at, &boar))));
    cases.push(("mirepincher", "default".into(), Box::new(move |a, at| anim::multileg(a, at, &crab))));
    cases.push(("websnare", "websnare".into(), Box::new(move |a, at| anim::multileg(a, at, &spider))));
    cases.push(("gloomwisp", "default".into(), Box::new(anim::wisp)));
    let mut worst: Vec<(f32, String)> = Vec::new();
    for (fig, set, f) in &cases {
        let figure = figure::figure(fig).unwrap();
        let mut look = figure::Look::default();
        if set.ends_with("+robe") {
            look.set("robe", 1);
        }
        let grip = match set.trim_end_matches("+robe") {
            "one_hand" => Grip::OneHand,
            "two_hand" => Grip::TwoHand,
            "bow" => Grip::Bow,
            _ => Grip::Empty,
        };
        let clips = figure.clips(&look, grip).unwrap();
        for &a in Anim::ALL {
            let clip = clips.clip(a).unwrap();
            let end = clip
                .duration
                .unwrap_or_else(|| clip.looping.unwrap() - (clip.looping.unwrap() * 0.25).min(1.0));
            let mut e = 0.0f32;
            let mut t = 0.0;
            while t <= end.min(10.0) {
                for s in [0.3f32, 1.9, 4.4] {
                    let at = At { t, stride: s, seed: 0.3 };
                    let want = f(a, &at);
                    let got = clips.pose(a, t, s, 0.3);
                    for b in 0..MAX_BONES {
                        e = e.max(want.rot[b].angle_between(got.rot[b]));
                        e = e.max(want.off[b].distance(got.off[b]) * 0.1);
                    }
                    for k in 0..3 {
                        e = e.max((want.k[k] - got.k[k]).abs());
                    }
                }
                t += 0.037;
            }
            worst.push((e, format!("{fig}/{set} {a:?}")));
        }
    }
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (e, what) in worst.iter().take(25) {
        println!("{what}: {e:.4}");
    }
    assert!(worst[0].0 < 0.05, "clips stray from the poses");
}
