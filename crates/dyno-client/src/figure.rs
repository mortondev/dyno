//! Figures from the content pack: characters, creatures and held items.
//!
//! Everything a figure looks like is data (see docs/design/data-and-streaming.md):
//!
//! - `figures.json` says what figures exist, how a look is chosen (the
//!   character creator's options, how a traveller or townsperson is dressed,
//!   which figure each creature is) and, per figure, how its voxel model is
//!   put together;
//! - `models/<figure>.vox` holds the voxels: layers named
//!   `<group>/<layer>/<part>/<mode>`, composed in order for a look (a hair
//!   style, a beard, plate armour...), with palette entries that follow the
//!   look's colours (skin, hair, tunic...);
//! - `rigs/<skeleton>.json` gives the skeleton's bones, and per figure the
//!   bone tree, pivots, the part drawn on each bone, where a weapon is held,
//!   and secondary motion (capes, rags, motes) as small expressions;
//! - `anims/<skeleton>.json` holds the clips ([`crate::clip`]).
//!
//! The engine composes a look once, meshes it at three levels of detail and
//! caches the meshes; each frame it only poses them.

use crate::anim::{Grip, MAX_BONES, Pose, key};
use crate::clip::{ClipSet, SkeletonSpec};
use crate::expr::Expr;
use crate::render::Vertex;
use crate::voxmodel::{Cache, Ink, Mesh, Paint, Vox, emit, key_of, view_distance};
use crate::{assets, vox};
use dyno_world::content::WeaponKind;
use glam::{Affine3A, Mat3, Vec3};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

// ------------------------------------------------------------------- looks

/// How one figure looks: named features (a hair style, a build, an armour)
/// and named colours (skin, hair, tunic...), as the pack defines them, and
/// what it holds in its right hand.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub features: BTreeMap<String, u8>,
    pub colors: BTreeMap<String, [f32; 3]>,
    pub weapon: Option<(WeaponKind, [f32; 3])>,
}

impl Look {
    /// A feature's value. A garment worn in a colour (cape, apron...) is a
    /// feature of 1 unless set otherwise.
    pub fn feature(&self, name: &str) -> u8 {
        self.features
            .get(name)
            .copied()
            .unwrap_or_else(|| u8::from(self.colors.contains_key(name)))
    }

    pub fn set(&mut self, name: &str, v: u8) {
        self.features.insert(name.to_string(), v);
    }

    pub fn color(&self, name: &str) -> Option<[f32; 3]> {
        self.colors.get(name).copied()
    }

    /// A key for caching the model of this look (not what it holds).
    pub fn key(&self) -> u64 {
        let mut k = Vec::new();
        for (name, v) in &self.features {
            k.push(name_hash(name));
            k.push(*v as u32);
        }
        k.push(0xFFFF_FFFF);
        for (name, c) in &self.colors {
            k.push(name_hash(name));
            k.extend(c.iter().map(|x| x.to_bits()));
        }
        key_of(&k)
    }
}

fn name_hash(s: &str) -> u32 {
    s.bytes().fold(0x811C_9DC5u32, |h, b| {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    })
}

/// The hash a traveller's look is drawn from.
fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

/// A generated value: fixed, picked by bits of the look's hash (or by
/// another feature), or another colour scaled.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Gen<T> {
    Pick {
        pick: Vec<T>,
        #[serde(default)]
        shift: u32,
        /// pick by this feature's value instead of the hash
        #[serde(default)]
        by: Option<String>,
    },
    From {
        from: String,
        #[serde(default = "one")]
        scale: f32,
    },
    Fixed(T),
}

fn one() -> f32 {
    1.0
}

/// Changes to a look: features, colours (`null` takes one off), values
/// mapped to others, and a held weapon.
#[derive(Clone, Debug, Default, Deserialize)]
struct LookSpec {
    /// start from a traveller drawn from this seed ("key": the sum of the
    /// name's bytes)
    #[serde(default)]
    seed: Option<Value>,
    #[serde(default)]
    features: BTreeMap<String, u8>,
    #[serde(default)]
    colors: BTreeMap<String, Option<Gen<[f32; 3]>>>,
    #[serde(default)]
    remap: BTreeMap<String, BTreeMap<String, u8>>,
    #[serde(default)]
    weapon: Option<(String, [f32; 3])>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct GenSpec {
    #[serde(default)]
    colors: BTreeMap<String, Gen<[f32; 3]>>,
    #[serde(default)]
    features: BTreeMap<String, Gen<u8>>,
}

#[derive(Clone, Debug, Deserialize)]
struct OptionList {
    /// the field of the protocol's appearance that picks one
    field: String,
    options: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Deserialize)]
struct FieldSpec {
    field: String,
    #[serde(default = "max_u8")]
    max: u8,
}

fn max_u8() -> u8 {
    255
}

#[derive(Clone, Debug, Default, Deserialize)]
struct AppearanceSpec {
    #[serde(default)]
    colors: BTreeMap<String, OptionList>,
    #[serde(default)]
    features: BTreeMap<String, FieldSpec>,
}

#[derive(Clone, Debug, Deserialize)]
struct PlayerSpec {
    /// the figure players are drawn as
    figure: String,
    #[serde(flatten)]
    rules: GenSpec,
}

#[derive(Clone, Debug, Deserialize)]
struct CreatureSpec {
    figure: String,
    #[serde(default)]
    look: Option<LookSpec>,
    #[serde(default)]
    held: Option<(String, [f32; 3])>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HeldKind {
    pub figure: String,
    /// blocks per voxel at true size
    pub unit: f32,
    /// turn about the grip (radians) when carried by a figure
    #[serde(default)]
    pub carry_turn: f32,
    /// how a figure holds it: empty, one_hand, two_hand or bow
    #[serde(default)]
    pub grip: String,
    /// in the first-person hand: size, tilt, roll, lift
    #[serde(default = "fp_default")]
    pub fp: [f32; 4],
}

fn fp_default() -> [f32; 4] {
    [0.75, -0.35, 0.15, 0.03]
}

/// How items are drawn when held.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct HeldSpec {
    #[serde(default)]
    pub kinds: HashMap<String, HeldKind>,
    /// weapon kind ("Sword") → held kind
    #[serde(default)]
    pub weapons: HashMap<String, String>,
    /// tool kind → held kind
    #[serde(default)]
    pub tools: HashMap<String, String>,
    /// gear slot → held kind ("*" for other slots)
    #[serde(default)]
    pub slots: HashMap<String, String>,
    /// item icon → held kind
    #[serde(default)]
    pub icons: HashMap<String, String>,
    /// a lit block on a post (a torch)
    #[serde(default)]
    pub torch: Option<String>,
    /// the empty hand: kind and colour
    #[serde(default)]
    pub hand: Option<(String, [f32; 3])>,
    /// items whose names hold one of these burn from within
    #[serde(default)]
    pub glow_names: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct RecipeSpec {
    /// the look colour this follows; none for a fixed colour
    #[serde(default)]
    slot: Option<String>,
    #[serde(default = "ones")]
    k: [f32; 3],
    #[serde(default)]
    c: [f32; 3],
    #[serde(default)]
    glow: bool,
}

fn ones() -> [f32; 3] {
    [1.0; 3]
}

#[derive(Clone, Debug, Deserialize)]
struct LayerSpec {
    layer: String,
    /// any of these sets of feature values (none: always)
    #[serde(default)]
    when: Vec<BTreeMap<String, Vec<u8>>>,
}

#[derive(Clone, Debug, Deserialize)]
struct GroupSpec {
    /// feature values that look alike to this group: value → class
    /// ("*" for the rest)
    #[serde(default)]
    map: BTreeMap<String, BTreeMap<String, u8>>,
    layers: Vec<LayerSpec>,
}

#[derive(Clone, Debug, Deserialize)]
struct GridSpec {
    name: String,
    min: [i32; 3],
    max: [i32; 3],
    parts: Vec<String>,
    groups: Vec<GroupSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ClipChoice {
    Name(String),
    /// the set named by the grip, with "+feature" for each feature worn
    ByGrip {
        features: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
struct FigureSpec {
    file: String,
    #[serde(default)]
    skeleton: Option<String>,
    #[serde(default)]
    rig: Option<String>,
    #[serde(default)]
    clips: Option<ClipChoice>,
    #[serde(default = "three")]
    lods: usize,
    /// voxels per block
    #[serde(default = "fine")]
    per_block: f32,
    grids: Vec<GridSpec>,
    palette: Vec<Option<RecipeSpec>>,
}

fn three() -> usize {
    3
}
fn fine() -> f32 {
    32.0
}

#[derive(Clone, Debug, Deserialize)]
struct FiguresSpec {
    #[serde(default)]
    appearance: AppearanceSpec,
    player: PlayerSpec,
    #[serde(default)]
    npcs: BTreeMap<String, LookSpec>,
    #[serde(default)]
    creatures: HashMap<String, CreatureSpec>,
    #[serde(default)]
    held: HeldSpec,
    figures: HashMap<String, FigureSpec>,
}

// -------------------------------------------------------------------- rigs

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum VecSpec {
    Fixed([f32; 3]),
    By { by: String, values: Vec<[f32; 3]> },
}

impl Default for VecSpec {
    fn default() -> Self {
        VecSpec::Fixed([0.0; 3])
    }
}

impl VecSpec {
    fn get(&self, look: &Look) -> Vec3 {
        match self {
            VecSpec::Fixed(v) => Vec3::from(*v),
            VecSpec::By { by, values } => {
                let i = (look.feature(by) as usize).min(values.len().saturating_sub(1));
                values.get(i).map(|v| Vec3::from(*v)).unwrap_or(Vec3::ZERO)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct BoneSpec {
    name: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    at: VecSpec,
    #[serde(default)]
    ops: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize)]
struct PartSpec {
    #[serde(default)]
    part: Option<String>,
    #[serde(default)]
    grid: Option<String>,
    bone: String,
    #[serde(default)]
    pivot: VecSpec,
    /// where the held item is drawn
    #[serde(default)]
    held: bool,
    #[serde(default)]
    glow: bool,
    #[serde(default)]
    light: Option<f32>,
}

#[derive(Clone, Debug, Deserialize)]
struct RigSpec {
    #[serde(default)]
    offset: [f32; 3],
    bones: Vec<BoneSpec>,
    parts: Vec<PartSpec>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct StepSpec {
    base: f32,
    per_speed: f32,
    min: f32,
    max: f32,
}

impl StepSpec {
    /// Ground covered per step (half a stride cycle), in blocks, at
    /// `speed` blocks a second, for a figure of size `scale`.
    pub fn step(&self, speed: f32, scale: f32) -> f32 {
        (self.base + self.per_speed * speed).clamp(self.min, self.max) * scale.max(0.3)
    }
}

#[derive(Deserialize)]
struct RigFile {
    #[serde(flatten)]
    skeleton: SkeletonSpec,
    step: StepSpec,
    rigs: HashMap<String, RigSpec>,
}

enum Op {
    Pose,
    Rot(usize, Expr),
    Move([Expr; 3]),
    Scale([Expr; 3]),
}

struct Bone {
    parent: Option<usize>,
    at: VecSpec,
    /// the skeleton bone that poses it
    pose: Option<usize>,
    ops: Vec<Op>,
}

struct Part {
    grid: usize,
    part: u8,
    bone: usize,
    pivot: VecSpec,
    held: bool,
    glow: bool,
    light: Option<f32>,
}

struct Rig {
    offset: Vec3,
    bones: Vec<Bone>,
    parts: Vec<Part>,
}

/// Expression variables: the pose's keys, the camera pitch, and each
/// skeleton bone's offset.
const V_CLOCK: usize = 0;
const V_FLOW: usize = 1;
const V_SHAPE: usize = 2;
const V_SPREAD: usize = 3;
const V_PITCH: usize = 4;
const V_BONES: usize = 5;
const N_VARS: usize = V_BONES + 3 * MAX_BONES;

fn compile_rig(sk: &SkeletonSpec, spec: &RigSpec, grids: &[GridSpec]) -> Result<Rig, String> {
    let var = |name: &str| -> Option<usize> {
        match name {
            "clock" => Some(V_CLOCK),
            "flow" => Some(V_FLOW),
            "shape" => Some(V_SHAPE),
            "spread" => Some(V_SPREAD),
            "pitch" => Some(V_PITCH),
            _ => {
                let (bone, axis) = name.rsplit_once('.')?;
                let b = sk.bone(bone)?;
                let a = match axis {
                    "ox" => 0,
                    "oy" => 1,
                    "oz" => 2,
                    _ => return None,
                };
                Some(V_BONES + 3 * b + a)
            }
        }
    };
    let ex = |v: &Value| -> Result<Expr, String> {
        match v {
            Value::Number(n) => Ok(Expr::Num(n.as_f64().unwrap_or(0.0) as f32)),
            Value::String(s) => Expr::parse(s, &var),
            _ => Err(format!("rig: {v} is not an expression")),
        }
    };
    let ex3 = |v: &Value| -> Result<[Expr; 3], String> {
        match v.as_array() {
            Some(a) if a.len() == 3 => Ok([ex(&a[0])?, ex(&a[1])?, ex(&a[2])?]),
            _ => {
                let e = ex(v)?;
                Ok([e.clone(), e.clone(), e])
            }
        }
    };
    let mut bones = Vec::new();
    let mut names: Vec<&str> = Vec::new();
    for b in &spec.bones {
        let parent = match &b.parent {
            Some(p) => Some(
                names
                    .iter()
                    .position(|n| n == p)
                    .ok_or_else(|| format!("rig: bone {} before its parent {p}", b.name))?,
            ),
            None => None,
        };
        let mut ops = Vec::new();
        for op in &b.ops {
            if op.as_str() == Some("pose") {
                ops.push(Op::Pose);
                continue;
            }
            let obj = op
                .as_object()
                .ok_or("rig: an op is \"pose\" or {name: value}")?;
            for (k, v) in obj {
                ops.push(match k.as_str() {
                    "rx" => Op::Rot(0, ex(v)?),
                    "ry" => Op::Rot(1, ex(v)?),
                    "rz" => Op::Rot(2, ex(v)?),
                    "move" => Op::Move(ex3(v)?),
                    "scale" => Op::Scale(ex3(v)?),
                    _ => return Err(format!("rig: no op {k}")),
                });
            }
        }
        let pose = sk.bone(&b.name);
        if pose.is_some() && !ops.iter().any(|o| matches!(o, Op::Pose)) {
            ops.insert(0, Op::Pose);
        }
        bones.push(Bone {
            parent,
            at: b.at.clone(),
            pose,
            ops,
        });
        names.push(&b.name);
    }
    let mut parts = Vec::new();
    for p in &spec.parts {
        let bone = names
            .iter()
            .position(|n| *n == p.bone)
            .ok_or_else(|| format!("rig: no bone {}", p.bone))?;
        let grid = match &p.grid {
            Some(g) => grids
                .iter()
                .position(|x| &x.name == g)
                .ok_or_else(|| format!("rig: no grid {g}"))?,
            None => 0,
        };
        let part = match &p.part {
            Some(name) => grids
                .get(grid)
                .and_then(|g| g.parts.iter().position(|x| x == name))
                .ok_or_else(|| format!("rig: no part {name}"))? as u8,
            None => 0,
        };
        parts.push(Part {
            grid,
            part,
            bone,
            pivot: p.pivot.clone(),
            held: p.held,
            glow: p.glow,
            light: p.light,
        });
    }
    Ok(Rig {
        offset: Vec3::from(spec.offset),
        bones,
        parts,
    })
}

// ----------------------------------------------------------------- figures

/// How a voxel of a layer applies to the model being built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// put it there, whatever was there
    Set,
    /// only where there is nothing yet
    Add,
    /// make the cell empty
    Erase,
    /// take back what an earlier layer of the group put there
    Drop,
    /// recolour what is there, keeping its part
    Dye,
    /// add where empty, else recolour what is there
    AddDye,
}

struct Layer {
    when: Vec<Vec<(String, Vec<u8>)>>,
    /// cell index, mode, part, palette index
    voxels: Vec<(u32, Mode, u8, u8)>,
}

struct Group {
    map: Vec<(String, HashMap<u8, u8>, Option<u8>)>,
    layers: Vec<Layer>,
}

struct Grid {
    min: [i32; 3],
    max: [i32; 3],
    groups: Vec<Group>,
}

#[derive(Clone, Copy)]
struct Recipe {
    slot: Option<usize>,
    k: [f32; 3],
    c: [f32; 3],
    glow: bool,
}

/// A figure ready to compose and draw.
pub struct Figure {
    grids: Vec<Grid>,
    /// colour slots named by the palette
    slots: Vec<String>,
    palette: Vec<Option<Recipe>>,
    lods: usize,
    per_block: f32,
    rig: Option<Rig>,
    clips: Option<ClipChoice>,
    sets: Option<Rc<HashMap<String, Rc<ClipSet>>>>,
    step: Option<StepSpec>,
    skeleton: Option<String>,
}

impl Grid {
    fn size(&self) -> [i32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    fn cell(&self, p: [i32; 3]) -> Option<u32> {
        let s = self.size();
        let q = [p[0] - self.min[0], p[1] - self.min[1], p[2] - self.min[2]];
        if (0..3).any(|a| q[a] < 0 || q[a] >= s[a]) {
            return None;
        }
        Some(((q[1] * s[2] + q[2]) * s[0] + q[0]) as u32)
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

fn load_figure(spec: &FigureSpec) -> Result<Option<Figure>, String> {
    let Some(bytes) = assets::get(&spec.file) else {
        assets::request(&spec.file);
        return Ok(None);
    };
    used(&spec.file);
    let file = vox::read(&bytes)?;
    let mut slots: Vec<String> = Vec::new();
    let palette = spec
        .palette
        .iter()
        .map(|r| {
            r.as_ref().map(|r| Recipe {
                slot: r.slot.as_ref().map(|s| {
                    slots.iter().position(|x| x == s).unwrap_or_else(|| {
                        slots.push(s.clone());
                        slots.len() - 1
                    })
                }),
                k: r.k,
                c: r.c,
                glow: r.glow,
            })
        })
        .collect();
    // models by layer: "<group>/<layer>/<part>/<mode>"
    let mut by_layer: HashMap<&str, Vec<(&str, Mode, &vox::Model)>> = HashMap::new();
    for m in &file.models {
        let Some((rest, mode)) = m.name.rsplit_once('/') else {
            continue;
        };
        let Some((layer, part)) = rest.rsplit_once('/') else {
            continue;
        };
        let mode = match mode {
            "set" => Mode::Set,
            "add" => Mode::Add,
            "erase" => Mode::Erase,
            "drop" => Mode::Drop,
            "dye" => Mode::Dye,
            "add-dye" => Mode::AddDye,
            _ => return Err(format!("{}: model {} has no mode", spec.file, m.name)),
        };
        by_layer.entry(layer).or_default().push((part, mode, m));
    }
    let mut grids = Vec::new();
    for g in &spec.grids {
        let mut grid = Grid {
            min: g.min,
            max: g.max,
            groups: Vec::new(),
        };
        for gs in &g.groups {
            let mut group = Group {
                map: gs
                    .map
                    .iter()
                    .map(|(f, m)| {
                        let rest = m.get("*").copied();
                        let pairs = m
                            .iter()
                            .filter_map(|(k, v)| k.parse::<u8>().ok().map(|k| (k, *v)))
                            .collect();
                        (f.clone(), pairs, rest)
                    })
                    .collect(),
                layers: Vec::new(),
            };
            for ls in &gs.layers {
                let mut voxels = Vec::new();
                for (part, mode, m) in by_layer.get(ls.layer.as_str()).into_iter().flatten() {
                    let part = match *part {
                        "-" => 0,
                        p => g.parts.iter().position(|x| x == p).ok_or_else(|| {
                            format!("{}: layer {} has unknown part {p}", spec.file, ls.layer)
                        })? as u8,
                    };
                    for (p, ink) in &m.voxels {
                        if let Some(c) = grid.cell(*p) {
                            voxels.push((c, *mode, part, *ink));
                        }
                    }
                }
                group.layers.push(Layer {
                    when: ls
                        .when
                        .iter()
                        .map(|w| w.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .collect(),
                    voxels,
                });
            }
            grid.groups.push(group);
        }
        grids.push(grid);
    }
    let (mut rig, mut sets, mut step) = (None, None, None);
    if let Some(sk) = &spec.skeleton {
        let Some(rf) = rig_file(sk)? else {
            return Ok(None);
        };
        let Some(clips) = clip_file(sk, &rf.skeleton)? else {
            return Ok(None);
        };
        let name = spec.rig.as_deref().unwrap_or("default");
        let rs = rf
            .rigs
            .get(name)
            .ok_or_else(|| format!("rigs/{sk}.json: no rig {name}"))?;
        rig = Some(compile_rig(&rf.skeleton, rs, &spec.grids)?);
        sets = Some(clips);
        step = Some(rf.step);
    }
    Ok(Some(Figure {
        grids,
        slots,
        palette,
        lods: spec.lods.clamp(1, 3),
        per_block: spec.per_block,
        rig,
        clips: spec.clips.clone(),
        sets,
        step,
        skeleton: spec.skeleton.clone(),
    }))
}

fn rig_file(skeleton: &str) -> Result<Option<Rc<RigFile>>, String> {
    LIB.with(|l| {
        if let Some(r) = l.borrow().rigs.get(skeleton) {
            return Ok(Some(r.clone()));
        }
        let path = format!("rigs/{skeleton}.json");
        let Some(text) = assets::text(&path) else {
            assets::request(&path);
            return Ok(None);
        };
        used(&path);
        let r: RigFile = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        let r = Rc::new(r);
        l.borrow_mut().rigs.insert(skeleton.to_string(), r.clone());
        Ok(Some(r))
    })
}

type Sets = Rc<HashMap<String, Rc<ClipSet>>>;

fn clip_file(skeleton: &str, sk: &SkeletonSpec) -> Result<Option<Sets>, String> {
    LIB.with(|l| {
        if let Some(r) = l.borrow().clips.get(skeleton) {
            return Ok(Some(r.clone()));
        }
        // the compact form the server streams, or the source (local packs)
        let compact = format!("anims/{skeleton}.clips");
        let source = format!("anims/{skeleton}.json");
        let (path, v): (&str, Value) = if let Some(b) = assets::get(&compact) {
            let v = dyno_protocol::clips::decode(&b).map_err(|e| format!("{compact}: {e}"))?;
            (&compact, v)
        } else if let Some(text) = assets::text(&source) {
            let v = serde_json::from_str(&text).map_err(|e| format!("{source}: {e}"))?;
            (&source, v)
        } else {
            assets::request(&compact);
            return Ok(None);
        };
        used(path);
        let sets = Rc::new(crate::clip::parse_sets(sk, &v).map_err(|e| format!("{path}: {e}"))?);
        l.borrow_mut()
            .clips
            .insert(skeleton.to_string(), sets.clone());
        Ok(Some(sets))
    })
}

#[derive(Default)]
struct Lib {
    spec: Option<Rc<FiguresSpec>>,
    figures: HashMap<String, Rc<Figure>>,
    rigs: HashMap<String, Rc<RigFile>>,
    clips: HashMap<String, Sets>,
    /// problems already reported
    told: Vec<String>,
    /// the asset store's generation when this was last checked
    generation: u64,
    /// the files loaded so far, and where their bytes were
    used: Vec<(String, usize)>,
}

thread_local! {
    static LIB: RefCell<Lib> = RefCell::new(Lib::default());
    static BUILT: Cache<Built> = Cache::default();
}

fn complain(e: String) {
    LIB.with(|l| {
        let mut l = l.borrow_mut();
        if !l.told.contains(&e) {
            eprintln!("figures: {e}");
            l.told.push(e);
        }
    });
}

/// Where a file's bytes are in the asset store (0 when missing): it changes
/// exactly when the file is added or replaced.
fn addr(path: &str) -> usize {
    assets::get(path).map(|b| b.as_ptr() as usize).unwrap_or(0)
}

/// Note a file what is loaded was made from.
fn used(path: &str) {
    let a = addr(path);
    LIB.with(|l| l.borrow_mut().used.push((path.to_string(), a)));
}

/// Forget everything loaded, for a new pack.
pub fn reset() {
    LIB.with(|l| *l.borrow_mut() = Lib::default());
}

fn spec() -> Option<Rc<FiguresSpec>> {
    // files arrived or changed: look again
    let generation = assets::generation();
    if LIB.with(|l| l.borrow().generation != generation) {
        // a streamed file arriving in play changes nothing already loaded:
        // start again only when a file in use was replaced
        let same = LIB.with(|l| l.borrow().used.iter().all(|(p, a)| addr(p) == *a));
        if same {
            LIB.with(|l| l.borrow_mut().generation = generation);
        } else {
            LIB.with(|l| {
                *l.borrow_mut() = Lib {
                    generation,
                    ..Lib::default()
                }
            });
            BUILT.with(|b| b.clear());
        }
    }
    if let Some(s) = LIB.with(|l| l.borrow().spec.clone()) {
        return Some(s);
    }
    let text = assets::text("figures.json")?;
    used("figures.json");
    match serde_json::from_str::<FiguresSpec>(&text) {
        Ok(s) => {
            let s = Rc::new(s);
            LIB.with(|l| l.borrow_mut().spec = Some(s.clone()));
            Some(s)
        }
        Err(e) => {
            complain(format!("figures.json: {e}"));
            None
        }
    }
}

/// A figure by name, once its files have arrived.
pub fn figure(name: &str) -> Option<Rc<Figure>> {
    if let Some(f) = LIB.with(|l| l.borrow().figures.get(name).cloned()) {
        return Some(f);
    }
    let s = spec()?;
    let Some(fs) = s.figures.get(name) else {
        complain(format!("no figure {name}"));
        return None;
    };
    match load_figure(fs) {
        Ok(Some(f)) => {
            let f = Rc::new(f);
            LIB.with(|l| l.borrow_mut().figures.insert(name.to_string(), f.clone()));
            Some(f)
        }
        Ok(None) => None,
        Err(e) => {
            complain(format!("{name}: {e}"));
            None
        }
    }
}

// ------------------------------------------------------------- choosing

fn gen_color(
    g: &Gen<[f32; 3]>,
    h: u32,
    base: &BTreeMap<String, [f32; 3]>,
    color: [f32; 3],
    look: &Look,
) -> Option<[f32; 3]> {
    match g {
        Gen::Fixed(c) => Some(*c),
        Gen::Pick { pick, shift, by } => {
            let i = match by {
                Some(f) => look.feature(f) as usize,
                None => (h >> shift) as usize,
            };
            (!pick.is_empty()).then(|| pick[i % pick.len()])
        }
        Gen::From { from, scale } => {
            let c = if from == "color" {
                color
            } else {
                *base.get(from)?
            };
            Some(c.map(|x| (x * scale).clamp(0.0, 1.0)))
        }
    }
}

fn gen_feature(g: &Gen<u8>, h: u32, look: &Look) -> u8 {
    match g {
        Gen::Fixed(v) => *v,
        Gen::Pick { pick, shift, by } => {
            let i = match by {
                Some(f) => look.feature(f) as usize,
                None => (h >> shift) as usize,
            };
            pick.get(i % pick.len().max(1)).copied().unwrap_or(0)
        }
        Gen::From { from, .. } => look.feature(from),
    }
}

/// A traveller drawn from a seed, in a signature colour.
fn traveller(s: &FiguresSpec, seed: u32, color: [f32; 3]) -> Look {
    let h = hash(seed);
    let mut l = Look::default();
    let derived = |g: &Gen<u8>| matches!(g, Gen::From { .. } | Gen::Pick { by: Some(_), .. });
    for (name, g) in s.player.rules.features.iter().filter(|(_, g)| !derived(g)) {
        let v = gen_feature(g, h, &l);
        l.set(name, v);
    }
    for (name, g) in s.player.rules.features.iter().filter(|(_, g)| derived(g)) {
        let v = gen_feature(g, h, &l);
        l.set(name, v);
    }
    for (name, g) in &s.player.rules.colors {
        let base = l.colors.clone();
        if let Some(c) = gen_color(g, h, &base, color, &l) {
            l.colors.insert(name.clone(), c);
        }
    }
    l
}

fn apply_spec(s: &FiguresSpec, l: &mut Look, spec: &LookSpec, color: [f32; 3]) {
    for (f, m) in &spec.remap {
        let v = l.feature(f);
        if let Some(n) = m.get(&v.to_string()) {
            l.set(f, *n);
        }
    }
    for (f, v) in &spec.features {
        l.set(f, *v);
    }
    let base = l.colors.clone();
    for (name, g) in &spec.colors {
        match g.as_ref().and_then(|g| gen_color(g, 0, &base, color, l)) {
            Some(c) => {
                l.colors.insert(name.clone(), c);
            }
            None => {
                l.colors.remove(name);
            }
        }
    }
    if let Some((kind, c)) = &spec.weapon {
        l.weapon = weapon_kind(kind).map(|k| (k, *c));
    }
    let _ = s;
}

fn weapon_kind(name: &str) -> Option<WeaponKind> {
    use WeaponKind::*;
    [
        Sword, Axe, Mace, Dagger, Staff, Polearm, Bow, Wand, Shield, Offhand,
    ]
    .into_iter()
    .find(|k| format!("{k:?}") == name)
}

fn look_from_spec(s: &FiguresSpec, spec: &LookSpec, key: &str, color: [f32; 3]) -> Look {
    let seed = match &spec.seed {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
        Some(Value::String(k)) if k == "key" => key.bytes().map(|b| b as u32).sum(),
        _ => 0,
    };
    let mut l = traveller(s, seed, color);
    apply_spec(s, &mut l, spec, color);
    l
}

impl Look {
    /// A traveller's look from a seed and a signature colour.
    pub fn player(seed: u32, color: [f32; 3]) -> Look {
        spec()
            .map(|s| traveller(&s, seed, color))
            .unwrap_or_default()
    }

    /// A player's look from the appearance they chose, in their class colour.
    pub fn from_appearance(a: &dyno_protocol::Appearance, color: [f32; 3]) -> Look {
        let Some(s) = spec() else {
            return Look::default();
        };
        let mut l = traveller(&s, 0, color);
        let field = |name: &str| -> u8 {
            match name {
                "skin" => a.skin,
                "hair_color" => a.hair_color,
                "eyes" => a.eyes,
                "hair_style" => a.hair_style,
                "beard" => a.beard as u8,
                "build" => a.build,
                "mood" => a.mood,
                "ears" => a.ears,
                _ => 0,
            }
        };
        for (name, o) in &s.appearance.colors {
            if !o.options.is_empty() {
                let c = o.options[field(&o.field) as usize % o.options.len()];
                l.colors.insert(name.clone(), c);
            }
        }
        for (name, f) in &s.appearance.features {
            l.set(name, field(&f.field).min(f.max));
        }
        l
    }

    /// The look of a named townsperson; others get one from their colour.
    pub fn npc(key: &str, color: [f32; 3]) -> Look {
        let Some(s) = spec() else {
            return Look::default();
        };
        let mut base = s.npcs.get("*").cloned().unwrap_or_default();
        base.seed.get_or_insert_with(|| Value::String("key".into()));
        let mut l = look_from_spec(&s, &base, key, color);
        if let Some(own) = s.npcs.get(key) {
            apply_spec(&s, &mut l, own, color);
        }
        l
    }

    /// The colours the character creator offers for a colour of the look
    /// ("skin", "hair", "eyes").
    pub fn options(name: &str) -> Vec<[f32; 3]> {
        spec()
            .and_then(|s| s.appearance.colors.get(name).map(|o| o.options.clone()))
            .unwrap_or_default()
    }
}

/// A held kind and its colour.
pub type HeldRef = (String, [f32; 3]);

/// The figure and look of a creature (by its content model and variant),
/// and what it holds.
pub fn creature(
    model: &str,
    variant: u8,
    color: [f32; 3],
) -> Option<(String, Look, Option<HeldRef>)> {
    let s = spec()?;
    let c = s
        .creatures
        .get(&format!("{model}/{variant}"))
        .or_else(|| s.creatures.get(&format!("{model}/*")))?;
    let mut look = match &c.look {
        Some(ls) => look_from_spec(&s, ls, "", color),
        None => Look::default(),
    };
    look.colors.entry("color".into()).or_insert(color);
    Some((c.figure.clone(), look, c.held.clone()))
}

/// The figure players are drawn as.
pub fn player_figure() -> Option<String> {
    spec().map(|s| s.player.figure.clone())
}

/// How items are held.
pub fn held_spec() -> Option<HeldSpec> {
    spec().map(|s| s.held.clone())
}

/// A held kind by name.
pub fn held_kind(name: &str) -> Option<HeldKind> {
    spec()?.held.kinds.get(name).cloned()
}

/// The held kind of a weapon.
pub fn weapon_held(k: WeaponKind) -> Option<String> {
    spec()?.held.weapons.get(&format!("{k:?}")).cloned()
}

/// How a figure grips a weapon.
pub fn grip_of(held: Option<&str>) -> Grip {
    let Some(kind) = held.and_then(held_kind) else {
        return Grip::Empty;
    };
    match kind.grip.as_str() {
        "two_hand" => Grip::TwoHand,
        "bow" => Grip::Bow,
        "empty" => Grip::Empty,
        _ => Grip::OneHand,
    }
}

// ------------------------------------------------------------- building

impl Figure {
    fn class(group: &Group, look: &Look, f: &str) -> u8 {
        let v = look.feature(f);
        match group.map.iter().find(|(n, _, _)| n == f) {
            Some((_, m, rest)) => m.get(&v).copied().or(*rest).unwrap_or(v),
            None => v,
        }
    }

    fn color(&self, ink: u8, look: &Look) -> Option<([f32; 3], bool)> {
        let r = self.palette.get(ink as usize).copied().flatten()?;
        let base = match r.slot {
            Some(s) => look.color(&self.slots[s]).unwrap_or([0.5; 3]),
            None => [0.0; 3],
        };
        Some((
            std::array::from_fn(|i| (r.k[i] * base[i] + r.c[i]).clamp(0.0, 1.0)),
            r.glow,
        ))
    }

    /// The voxel models of a look, one per grid.
    pub fn compose(&self, look: &Look) -> Vec<Vox> {
        let mut out = Vec::new();
        for g in &self.grids {
            let mut v = Vox::new(g.min, g.max);
            let mut inks: [Option<Ink>; 256] = [None; 256];
            let n = (g.size()[0] * g.size()[1] * g.size()[2]) as usize;
            // what the group does to each cell: (mode, part, palette) + 1
            let mut effect: Vec<u32> = vec![0; n];
            let mut touched: Vec<u32> = Vec::new();
            for group in &g.groups {
                for layer in &group.layers {
                    let on = layer.when.is_empty()
                        || layer.when.iter().any(|w| {
                            w.iter()
                                .all(|(f, vals)| vals.contains(&Self::class(group, look, f)))
                        });
                    if !on {
                        continue;
                    }
                    for &(c, mode, part, ink) in &layer.voxels {
                        let slot = &mut effect[c as usize];
                        if *slot == 0 {
                            touched.push(c);
                        }
                        *slot = if mode == Mode::Drop {
                            0
                        } else {
                            1 + ((mode as u32) << 16 | (part as u32) << 8 | ink as u32)
                        };
                    }
                }
                touched.sort_unstable();
                touched.dedup();
                for &c in &touched {
                    let e = std::mem::take(&mut effect[c as usize]);
                    if e == 0 {
                        continue;
                    }
                    let e = e - 1;
                    let (mode, part, ink) = (e >> 16, (e >> 8) as u8, e as u8);
                    let [x, y, z] = g.at(c);
                    v.part = part;
                    if mode == Mode::Erase as u32 {
                        v.set(x, y, z, 0);
                        continue;
                    }
                    let i = match inks[ink as usize] {
                        Some(i) => i,
                        None => {
                            let (rgb, glow) =
                                self.color(ink, look).unwrap_or(([1.0, 0.0, 1.0], false));
                            let i = if glow { v.glow(rgb) } else { v.ink(rgb) };
                            inks[ink as usize] = Some(i);
                            i
                        }
                    };
                    let there = v.solid(x, y, z);
                    match mode {
                        m if m == Mode::Add as u32 => v.add(x, y, z, i),
                        m if m == Mode::Dye as u32 || (m == Mode::AddDye as u32 && there) => {
                            v.dye(x, y, z, i)
                        }
                        _ => v.set(x, y, z, i),
                    }
                }
                touched.clear();
            }
            out.push(v);
        }
        out
    }

    /// The clips this figure plays when holding `grip`.
    pub fn clips(&self, look: &Look, grip: Grip) -> Option<Rc<ClipSet>> {
        let sets = self.sets.as_ref()?;
        let name = match &self.clips {
            Some(ClipChoice::Name(n)) => n.clone(),
            Some(ClipChoice::ByGrip { features }) => {
                let mut n = match grip {
                    Grip::Empty => "empty",
                    Grip::OneHand => "one_hand",
                    Grip::TwoHand => "two_hand",
                    Grip::Bow => "bow",
                }
                .to_string();
                for f in features {
                    if look.feature(f) != 0 {
                        n.push('+');
                        n.push_str(f);
                    }
                }
                n
            }
            None => "default".into(),
        };
        sets.get(&name).or_else(|| sets.get("default")).cloned()
    }

    /// The skeleton it is built on.
    pub fn skeleton(&self) -> Option<&str> {
        self.skeleton.as_deref()
    }

    /// Ground covered per step at `speed` for a figure of size `scale`.
    pub fn step(&self, speed: f32, scale: f32) -> f32 {
        self.step.map(|s| s.step(speed, scale)).unwrap_or(1.0)
    }
}

/// A look's meshes, per level of detail, per rig part.
struct Built {
    lods: Vec<Vec<Mesh>>,
}

fn build(fig: &Figure, look: &Look) -> Built {
    let grids = fig.compose(look);
    let mut levels: Vec<Vec<Vox>> = vec![grids];
    for _ in 1..fig.lods {
        let next = levels[levels.len() - 1].iter().map(Vox::half).collect();
        levels.push(next);
    }
    let lods = levels
        .iter()
        .map(|grids| match &fig.rig {
            Some(rig) => rig
                .parts
                .iter()
                .map(|p| {
                    if p.held {
                        return Mesh::default();
                    }
                    let pivot = p.pivot.get(look);
                    grids[p.grid].mesh(p.part, pivot.into())
                })
                .collect(),
            // no rig: the first part of the first grid, about its origin
            None => vec![grids[0].mesh(1, [0.0; 3])],
        })
        .collect();
    Built { lods }
}

fn built(name: &str, fig: &Figure, look: &Look) -> Rc<Built> {
    let key = key_of(&[
        name_hash(name),
        (look.key() >> 32) as u32,
        look.key() as u32,
    ]);
    BUILT.with(|c| c.get(key, || build(fig, look)))
}

/// Which level of detail to draw at `p`: full nearby, coarser far away, where
/// a fine voxel is smaller than a pixel. Full when the camera is not known.
fn lod_at(p: Vec3) -> usize {
    match view_distance(p) {
        Some(d) if d > 40.0 => 2,
        Some(d) if d > 14.0 => 1,
        _ => 0,
    }
}

/// The frame of a figure standing at `feet`: x right, y up, z forward, in
/// voxels of `1 / per_block` blocks.
fn root(feet: Vec3, yaw: f32, per_block: f32) -> Affine3A {
    let (sy, cy) = yaw.sin_cos();
    let k = 1.0 / per_block;
    Affine3A::from_cols(
        (Vec3::new(cy, 0.0, -sy) * k).into(),
        (Vec3::Y * k).into(),
        (Vec3::new(-sy, 0.0, -cy) * k).into(),
        feet.into(),
    )
}

/// What a figure holds, for drawing: the held item's mesh and fit.
pub struct Holding {
    held: Rc<Built>,
    unit: f32,
    carry_turn: f32,
}

/// The held model of a kind in a colour, of a quality, glowing or not, at
/// an animation frame (a torch's flame).
pub fn holding(
    kind: &str,
    color: [f32; 3],
    quality: u8,
    glows: bool,
    frame: u8,
) -> Option<Holding> {
    let k = held_kind(kind)?;
    let fig = figure(&k.figure)?;
    let mut look = Look::default();
    look.colors.insert("color".into(), color);
    look.set("quality", quality);
    look.set("glows", glows as u8);
    look.set("frame", frame);
    Some(Holding {
        held: built(&k.figure, &fig, &look),
        unit: k.unit,
        carry_turn: k.carry_turn,
    })
}

impl Holding {
    pub fn mesh(&self) -> &Mesh {
        &self.held.lods[0][0]
    }
    pub fn unit(&self) -> f32 {
        self.unit
    }
    pub fn carry_turn(&self) -> f32 {
        self.carry_turn
    }
}

/// Draw a figure posed by `pose`, lit and tinted by `paint`, `scale` times
/// life size, with `held` in its hand. `pitch` tilts the head (the camera's
/// pitch, for players).
#[allow(clippy::too_many_arguments)]
pub fn draw(
    out: &mut Vec<Vertex>,
    name: &str,
    fig: &Figure,
    look: &Look,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    paint: Paint,
    pose: &Pose,
    held: Option<&Holding>,
    scale: f32,
) {
    let Some(rig) = &fig.rig else {
        return;
    };
    let m = built(name, fig, look);
    let m = &m.lods[lod_at(feet).min(m.lods.len() - 1)];
    let mut vars = [0.0f32; N_VARS];
    vars[V_CLOCK] = pose.k[key::CLOCK];
    vars[V_FLOW] = pose.k[key::FLOW];
    vars[V_SHAPE] = pose.k[key::SHAPE];
    vars[V_SPREAD] = pose.k[key::SPREAD];
    vars[V_PITCH] = pitch;
    for b in 0..MAX_BONES {
        for a in 0..3 {
            vars[V_BONES + 3 * b + a] = pose.off[b][a];
        }
    }
    let base = root(feet, yaw, fig.per_block / scale) * Affine3A::from_translation(rig.offset);
    let mut frames: Vec<Affine3A> = Vec::with_capacity(rig.bones.len());
    for b in &rig.bones {
        let parent = b.parent.map(|p| frames[p]).unwrap_or(base);
        let off = b.pose.map(|i| pose.off[i]).unwrap_or(Vec3::ZERO);
        let mut f = parent * Affine3A::from_translation(b.at.get(look) + off);
        for op in &b.ops {
            f = match op {
                Op::Pose => match b.pose {
                    Some(i) => f * Affine3A::from_quat(pose.rot[i]),
                    None => f,
                },
                Op::Rot(axis, e) => {
                    let a = e.eval(&vars);
                    f * Affine3A::from_mat3(match axis {
                        0 => Mat3::from_rotation_x(a),
                        1 => Mat3::from_rotation_y(a),
                        _ => Mat3::from_rotation_z(a),
                    })
                }
                Op::Move(v) => {
                    f * Affine3A::from_translation(Vec3::new(
                        v[0].eval(&vars),
                        v[1].eval(&vars),
                        v[2].eval(&vars),
                    ))
                }
                Op::Scale(v) => {
                    f * Affine3A::from_mat3(Mat3::from_diagonal(Vec3::new(
                        v[0].eval(&vars),
                        v[1].eval(&vars),
                        v[2].eval(&vars),
                    )))
                }
            };
        }
        frames.push(f);
    }
    for (p, mesh) in rig.parts.iter().zip(m) {
        let frame = frames[p.bone];
        let paint = Paint {
            glow: paint.glow || p.glow,
            light: p.light.unwrap_or(paint.light),
            ..paint
        };
        if p.held {
            if let Some(h) = held {
                let fit = Mat3::from_rotation_y(h.carry_turn)
                    * Mat3::from_diagonal(Vec3::splat(h.unit * fig.per_block));
                emit(out, h.mesh(), &(frame * Affine3A::from_mat3(fit)), paint);
            }
            continue;
        }
        emit(out, mesh, &frame, paint);
    }
}

/// A plain stand-in, drawn while a figure's files have not arrived: a grey
/// post the size of a person (`scale` 1).
pub fn placeholder(out: &mut Vec<Vertex>, feet: Vec3, scale: f32, light: f32) {
    let h = 1.78 * scale;
    let w = 0.22 * scale;
    crate::render::push_box(
        out,
        feet + Vec3::new(-w, 0.0, -w),
        feet + Vec3::new(w, h, w),
        [0.55, 0.55, 0.58],
        light,
    );
}
