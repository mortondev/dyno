//! `dyno bake-figure`: a dev tool that sculpts figures from parametric
//! shapes (signed distance fields) and writes them as pack data: the
//! `.vox` layers, the figure's entry in `figures.json` and its rig. The
//! output is the data; nothing here is part of the client.
//!
//! A figure is a list of grids (each at its own voxel density); a grid
//! holds groups of layers, each layer a set of voxels with a part, a mode
//! and a palette index, shown when its `when` matches the look.

pub mod body;
pub mod clothes;
pub mod gear;
pub mod person;
pub mod sdf;

use glam::Vec3;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// How a voxel applies (as figure.rs reads it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Set,
    Add,
    Erase,
    Dye,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Set => "set",
            Mode::Add => "add",
            Mode::Erase => "erase",
            Mode::Dye => "dye",
        }
    }
}

/// When a layer shows: any of these sets of feature values.
pub type When = Vec<BTreeMap<String, Vec<u8>>>;

/// `when!(("chest", [1, 2]), ("body", [0]))`: one set of conditions.
pub fn when(conds: &[(&str, &[u8])]) -> When {
    if conds.is_empty() {
        return Vec::new();
    }
    vec![
        conds
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_vec()))
            .collect(),
    ]
}

pub struct Layer {
    pub name: String,
    pub when: When,
    /// voxel → (part, mode, ink); later writes win
    pub cells: BTreeMap<[i32; 3], (u8, Mode, u8)>,
}

pub struct Group {
    pub name: String,
    pub map: Option<Value>,
    pub layers: Vec<Layer>,
}

pub struct Grid {
    pub name: String,
    /// voxels per rig unit
    pub detail: f32,
    /// rig-unit bounds
    pub lo: Vec3,
    pub hi: Vec3,
    pub parts: Vec<String>,
    pub groups: Vec<Group>,
}

/// What a voxel becomes in a layer.
pub type Cell = Option<(u8, Mode, u8)>;

impl Grid {
    pub fn new(name: &str, detail: f32, lo: Vec3, hi: Vec3, parts: &[&str]) -> Grid {
        let mut p = vec![String::new()];
        p.extend(parts.iter().map(|s| s.to_string()));
        Grid {
            name: name.into(),
            detail,
            lo,
            hi,
            parts: p,
            groups: Vec::new(),
        }
    }

    pub fn part(&self, name: &str) -> u8 {
        self.parts
            .iter()
            .position(|p| p == name)
            .unwrap_or_else(|| panic!("grid {}: no part {name}", self.name)) as u8
    }

    pub fn vmin(&self) -> [i32; 3] {
        (self.lo * self.detail).floor().as_ivec3().to_array()
    }

    pub fn vmax(&self) -> [i32; 3] {
        (self.hi * self.detail).ceil().as_ivec3().to_array()
    }

    /// The centre of voxel `v`, in rig units.
    pub fn centre(&self, v: [i32; 3]) -> Vec3 {
        (Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32) + Vec3::splat(0.5)) / self.detail
    }

    /// A layer of the voxels in `lo..hi` (rig units) that `f` fills.
    pub fn layer(&self, name: &str, when: When, lo: Vec3, hi: Vec3, f: impl Fn(Vec3) -> Cell) -> Layer {
        let mut cells = BTreeMap::new();
        let (gmin, gmax) = (self.vmin(), self.vmax());
        let a = (lo * self.detail).floor().as_ivec3().to_array();
        let b = (hi * self.detail).ceil().as_ivec3().to_array();
        for x in a[0].max(gmin[0])..b[0].min(gmax[0]) {
            for y in a[1].max(gmin[1])..b[1].min(gmax[1]) {
                for z in a[2].max(gmin[2])..b[2].min(gmax[2]) {
                    if let Some(c) = f(self.centre([x, y, z])) {
                        cells.insert([x, y, z], c);
                    }
                }
            }
        }
        Layer {
            name: name.into(),
            when,
            cells,
        }
    }

    pub fn push(&mut self, group: &str, layer: Layer) {
        let taken = self
            .groups
            .iter()
            .any(|g| g.layers.iter().any(|l| l.name == layer.name && g.name == group));
        assert!(!taken, "bake-figure: two layers named {group}/{}", layer.name);
        if layer.cells.is_empty() {
            eprintln!("bake-figure: layer {group}/{} is empty", layer.name);
        }
        match self.groups.iter_mut().find(|g| g.name == group) {
            Some(g) => g.layers.push(layer),
            None => self.groups.push(Group {
                name: group.into(),
                map: None,
                layers: vec![layer],
            }),
        }
    }
}

/// Palette recipes: index → how its colour follows the look.
#[derive(Default)]
pub struct Palette {
    pub list: Vec<Value>,
    /// a colour for the `.vox` file's own palette (for viewing in an editor)
    pub preview: Vec<[u8; 3]>,
}

impl Palette {
    fn add(&mut self, v: Value, preview: [f32; 3]) -> u8 {
        if let Some(i) = self.list.iter().position(|x| *x == v) {
            return i as u8 + 1;
        }
        assert!(self.list.len() < 255, "bake-figure: out of palette entries");
        self.list.push(v);
        self.preview
            .push(preview.map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8));
        self.list.len() as u8
    }

    /// A shade of a look colour: `k * colour + c`.
    pub fn slot(&mut self, slot: &str, k: f32, c: [f32; 3], crisp: bool) -> u8 {
        let r = |x: f32| (x * 1000.0).round() / 1000.0;
        let mut v = json!({"slot": slot, "k": [r(k), r(k), r(k)]});
        if c != [0.0; 3] {
            v["c"] = json!(c.map(r));
        }
        if crisp {
            v["crisp"] = json!(true);
        }
        self.add(v, [0.6 * k + c[0], 0.5 * k + c[1], 0.45 * k + c[2]])
    }

    /// A fixed colour.
    pub fn fixed(&mut self, c: [f32; 3], glow: bool, crisp: bool) -> u8 {
        let r = |x: f32| (x * 1000.0).round() / 1000.0;
        let mut v = json!({"c": c.map(r)});
        if glow {
            v["glow"] = json!(true);
        }
        if crisp {
            v["crisp"] = json!(true);
        }
        self.add(v, c)
    }

    pub fn json(&self) -> Value {
        let mut v = vec![Value::Null];
        v.extend(self.list.iter().cloned());
        Value::Array(v)
    }
}

/// Write the grids' layers as a `.vox` file and return the grids' JSON.
pub fn write(grids: &[Grid], palette: &Palette, vox_path: &std::path::Path) -> Result<Value, String> {
    use dyno_client::vox;
    let mut file = vox::File::default();
    for (i, c) in palette.preview.iter().enumerate() {
        file.palette[i + 1] = [c[0], c[1], c[2], 255];
    }
    let mut gj = Vec::new();
    for g in grids {
        let mut groups = Vec::new();
        for group in &g.groups {
            let mut layers = Vec::new();
            for l in &group.layers {
                let name = format!("{}/{}", group.name, l.name);
                let mut models: BTreeMap<(u8, Mode), Vec<([i32; 3], u8)>> = BTreeMap::new();
                for (p, (part, mode, ink)) in &l.cells {
                    // an erase needs no colour, but MagicaVoxel needs one
                    models.entry((*part, *mode)).or_default().push((*p, (*ink).max(1)));
                }
                for ((part, mode), voxels) in models {
                    let pn = if part == 0 { "-".to_string() } else { g.parts[part as usize].clone() };
                    file.models.push(vox::Model {
                        name: format!("{name}/{pn}/{}", mode.name()),
                        voxels,
                    });
                }
                let mut lj = json!({"layer": name});
                if !l.when.is_empty() {
                    lj["when"] = json!(l.when);
                }
                layers.push(lj);
            }
            let mut gjv = json!({"layers": layers});
            if let Some(m) = &group.map {
                gjv["map"] = m.clone();
            }
            groups.push(gjv);
        }
        gj.push(json!({
            "name": g.name,
            "detail": g.detail,
            "min": g.vmin(),
            "max": g.vmax(),
            "parts": g.parts,
            "groups": groups,
        }));
    }
    let bytes = vox::write(&file)?;
    std::fs::write(vox_path, &bytes).map_err(|e| format!("{}: {e}", vox_path.display()))?;
    Ok(Value::Array(gj))
}

/// Read a JSON file, change it, and write it back pretty-printed.
pub fn edit_json(path: &std::path::Path, f: impl FnOnce(&mut Value)) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    f(&mut v);
    // as the pack's other JSON: one space per level
    let mut out = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
    serde::Serialize::serialize(&v, &mut ser).map_err(|e| e.to_string())?;
    out.push(b'\n');
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}
