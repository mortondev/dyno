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
use crate::voxmodel::{Cache, Ink, Mesh, Paint, Smooth, Vox, emit, key_of, view_distance};
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
    /// things carried at the rig's other sockets ("offhand", "shield",
    /// "back"): a held kind and its colour
    pub carry: BTreeMap<String, (String, [f32; 3])>,
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
    /// things carried at other sockets: socket → (held kind, colour)
    #[serde(default)]
    carry: BTreeMap<String, (String, [f32; 3])>,
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
    /// several features packed in one field: this one is
    /// `(field / div) % modulo`
    #[serde(default = "one_u8")]
    div: u8,
    #[serde(default)]
    modulo: Option<u8>,
    /// how the character creator shows it: a label and a name per value
    /// (none: the creator does not offer it as a stepper)
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    names: Vec<String>,
}

fn one_u8() -> u8 {
    1
}

impl FieldSpec {
    fn get(&self, raw: u8) -> u8 {
        let v = raw / self.div.max(1);
        let v = match self.modulo {
            Some(m) if m > 0 => v % m,
            _ => v,
        };
        v.min(self.max)
    }

    fn put(&self, raw: u8, v: u8) -> u8 {
        let div = self.div.max(1) as u16;
        let span = self.modulo.map(|m| m as u16).unwrap_or(256 / div);
        let raw = raw as u16;
        let old = (raw / div) % span;
        (raw - old * div + (v as u16 % span) * div).min(255) as u8
    }
}

/// A stepper in the character creator, from the pack: the appearance
/// field it edits, how many values it has, their names.
#[derive(Clone, Debug)]
pub struct Stepper {
    pub label: String,
    pub field: String,
    pub names: Vec<String>,
    spec: FieldSpec,
}

impl Stepper {
    /// The value shown for a raw field value.
    pub fn get(&self, raw: u8) -> u8 {
        self.spec.get(raw)
    }

    /// The raw field value with this stepper's value set to `v`.
    pub fn put(&self, raw: u8, v: u8) -> u8 {
        self.spec.put(raw, v)
    }

    pub fn count(&self) -> u8 {
        (self.spec.max as usize + 1).min(self.names.len().max(1)) as u8
    }
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
    /// what a player wears before any gear
    #[serde(default)]
    base: Option<LookSpec>,
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
    /// keeps its detail when meshed smooth: pinned corners, its own colour
    /// (eyes, trim, emblems)
    #[serde(default)]
    crisp: bool,
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
    /// voxels per rig unit in this grid (default: the figure's), so a
    /// head can be modelled finer than the body
    #[serde(default)]
    detail: Option<f32>,
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
    #[serde(default)]
    lods: LodsSpec,
    /// rig units per block (rig positions, clip offsets)
    #[serde(default = "fine")]
    per_block: f32,
    /// voxels per rig unit: 2 models the figure at twice the density of
    /// its rig, so rigs and clips keep their numbers
    #[serde(default = "one")]
    detail: f32,
    /// how smooth levels of detail are meshed
    #[serde(default)]
    smooth: Option<SmoothSpec>,
    /// features that follow from others when a look does not set them
    /// (an old `armor` value standing for a chest piece, a robe for the
    /// clips that a cloth chest implies)
    #[serde(default)]
    derive: Vec<DeriveSpec>,
    grids: Vec<GridSpec>,
    palette: Vec<Option<RecipeSpec>>,
}

/// Levels of detail: a count (each half the last, blocky, changing at 14
/// and 40 blocks) or a list.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum LodsSpec {
    Count(usize),
    List(Vec<LodSpec>),
}

impl Default for LodsSpec {
    fn default() -> Self {
        LodsSpec::Count(3)
    }
}

/// `{"if": {"armor": [2]}, "set": {"chest": 3}}`: when every feature in
/// `if` has one of its values, the features in `set` take these values
/// unless the look, or an earlier rule, already set them.
#[derive(Clone, Debug, Deserialize)]
struct DeriveSpec {
    #[serde(rename = "if")]
    cond: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    set: BTreeMap<String, u8>,
    /// colours too, unless the look has them (fixed or from another colour)
    #[serde(default)]
    colors: BTreeMap<String, Gen<[f32; 3]>>,
}

/// How many times a level halves the voxels: the same for every grid, or
/// per grid (a level can keep the head fine while the body coarsens).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Halves {
    All(u8),
    Grids(Vec<u8>),
}

impl Default for Halves {
    fn default() -> Self {
        Halves::All(0)
    }
}

impl Halves {
    fn of(&self, grid: usize) -> u8 {
        match self {
            Halves::All(n) => *n,
            Halves::Grids(v) => v.get(grid).or(v.last()).copied().unwrap_or(0),
        }
    }
}

/// One level of detail.
#[derive(Clone, Debug, Deserialize)]
struct LodSpec {
    /// how many times the voxels are halved
    #[serde(default)]
    half: Halves,
    /// meshed smooth (surface nets) or as blocks
    #[serde(default)]
    smooth: bool,
    /// drawn up to this many blocks from the camera (the last: any distance)
    #[serde(default)]
    until: Option<f32>,
    /// a smooth level is simplified wherever that moves its surface less
    /// than this many of its voxels
    #[serde(default)]
    simplify: f32,
}

impl LodsSpec {
    fn list(&self) -> Vec<LodSpec> {
        match self {
            LodsSpec::List(l) if !l.is_empty() => l.clone(),
            LodsSpec::List(_) => vec![LodSpec {
                half: Halves::All(0),
                smooth: false,
                until: None,
                simplify: 0.0,
            }],
            LodsSpec::Count(n) => {
                let until = [Some(14.0), Some(40.0), None];
                let n = (*n).clamp(1, 3);
                (0..n)
                    .map(|i| LodSpec {
                        half: Halves::All(i as u8),
                        smooth: false,
                        until: if i + 1 == n { None } else { until[i] },
                        simplify: 0.0,
                    })
                    .collect()
            }
        }
    }
}

/// How smooth levels of detail are meshed (see [`Smooth`]).
#[derive(Clone, Copy, Debug, Deserialize)]
struct SmoothSpec {
    #[serde(default = "one")]
    bevel: f32,
    #[serde(default = "one")]
    soft: f32,
    #[serde(default = "four")]
    relax: u8,
    #[serde(default = "yes")]
    blend: bool,
    #[serde(default = "one")]
    occlusion: f32,
}

fn four() -> u8 {
    4
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
    /// what wearing an item does to the look (by item key)
    #[serde(default)]
    equip: EquipSpec,
    /// looks to show off on a contact sheet (`dyno shot --outfit-sheet`)
    #[serde(default)]
    showcase: ShowcaseSpec,
    figures: HashMap<String, FigureSpec>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct EquipSpec {
    /// item key → the features, colours and carried things it sets
    #[serde(default)]
    items: BTreeMap<String, LookSpec>,
}

/// A look's appearance as the character creator sets it, every field
/// optional.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
struct ShowAppearance {
    skin: u8,
    eyes: u8,
    hair_style: u8,
    hair_color: u8,
    beard: bool,
    build: u8,
    mood: u8,
    ears: u8,
}

#[derive(Clone, Debug, Deserialize)]
struct ShowPerson {
    name: String,
    /// a named townsperson's look instead (by NPC key)
    #[serde(default)]
    npc: Option<String>,
    #[serde(default)]
    appearance: ShowAppearance,
    /// the class colour
    #[serde(default = "grey")]
    color: [f32; 3],
    #[serde(flatten)]
    look: LookSpec,
}

fn grey() -> [f32; 3] {
    [0.5, 0.5, 0.6]
}

#[derive(Clone, Debug, Deserialize)]
struct ShowSet {
    name: String,
    /// item keys worn (through `equip`)
    #[serde(default)]
    wear: Vec<String>,
    #[serde(flatten)]
    look: LookSpec,
    /// which people wear it (indices into `people`; all if absent)
    #[serde(default)]
    on: Option<Vec<usize>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ShowcaseSpec {
    #[serde(default)]
    people: Vec<ShowPerson>,
    #[serde(default)]
    sets: Vec<ShowSet>,
}

// -------------------------------------------------------------------- rigs

/// A value that may vary by features of the look: fixed, a list indexed
/// by one feature, or a map keyed by several features' values joined with
/// commas, where `*` stands for any value (`"1,*"`, `"*"`).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum By<T> {
    One {
        by: String,
        values: Vec<T>,
    },
    Many {
        by: Vec<String>,
        map: BTreeMap<String, T>,
    },
    Fixed(T),
}

impl<T: Clone> By<T> {
    fn get(&self, look: &Look) -> Option<T> {
        match self {
            By::Fixed(v) => Some(v.clone()),
            By::One { by, values } => {
                let i = (look.feature(by) as usize).min(values.len().saturating_sub(1));
                values.get(i).cloned()
            }
            By::Many { by, map } => {
                let vals: Vec<String> = by.iter().map(|f| look.feature(f).to_string()).collect();
                // the exact values first, then with more and more of them
                // left to `*`, the last features first
                let n = vals.len().min(8);
                let mut masks: Vec<u32> = (0..1u32 << n).collect();
                masks.sort_by_key(|m| (m.count_ones(), m.reverse_bits()));
                masks.iter().find_map(|m| {
                    let key: Vec<&str> = vals
                        .iter()
                        .enumerate()
                        .map(|(i, v)| if m >> i & 1 == 1 { "*" } else { v.as_str() })
                        .collect();
                    map.get(&key.join(","))
                        .or_else(|| (*m == (1 << n) - 1).then(|| map.get("*")).flatten())
                        .cloned()
                })
            }
        }
    }
}

type VecSpec = By<[f32; 3]>;

impl Default for VecSpec {
    fn default() -> Self {
        By::Fixed([0.0; 3])
    }
}

impl VecSpec {
    fn vec(&self, look: &Look) -> Vec3 {
        self.get(look).map(Vec3::from).unwrap_or(Vec3::ZERO)
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
    /// where a held item is drawn: `true` for the weapon hand, or the
    /// name of a socket (an off-hand, a shield on the arm, the back)
    #[serde(default)]
    held: HeldAt,
    #[serde(default)]
    glow: bool,
    #[serde(default)]
    light: Option<f32>,
    /// reshape the part: proportions are data (slimmer limbs, a waist),
    /// which may vary by features (a build)
    #[serde(default)]
    shape: Option<By<ShapeSpec>>,
    /// false: meshed as blocks even where the figure is smooth
    #[serde(default)]
    smooth: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(untagged)]
enum HeldAt {
    #[default]
    No,
    Hand(bool),
    Socket(String),
}

impl HeldAt {
    fn socket(&self) -> Option<String> {
        match self {
            HeldAt::Hand(true) => Some(String::new()),
            HeldAt::Socket(s) => Some(s.clone()),
            _ => None,
        }
    }
}

/// How a part's mesh is reshaped about its pivot, in the rig's data.
#[derive(Clone, Copy, Debug, Deserialize)]
struct ShapeSpec {
    /// scale across (x), along (y) and front to back (z)
    #[serde(default = "ones")]
    scale: [f32; 3],
    /// width (x and z) at the part's top and bottom, as factors: a limb that
    /// narrows to the wrist, a torso broad at the shoulders and slim at the waist
    #[serde(default)]
    taper: Option<[f32; 2]>,
    /// taper front to back too (false: only side to side)
    #[serde(default = "yes")]
    taper_depth: bool,
}

fn yes() -> bool {
    true
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
    /// a held item is drawn here: "" for the weapon hand, else a socket
    held: Option<String>,
    glow: bool,
    light: Option<f32>,
    shape: Option<By<ShapeSpec>>,
    smooth: bool,
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
            held: p.held.socket(),
            glow: p.glow,
            light: p.light,
            shape: p.shape.clone(),
            smooth: p.smooth.unwrap_or(true),
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
    detail: f32,
    groups: Vec<Group>,
}

#[derive(Clone, Copy)]
struct Recipe {
    slot: Option<usize>,
    k: [f32; 3],
    c: [f32; 3],
    glow: bool,
    crisp: bool,
}

/// A figure ready to compose and draw.
pub struct Figure {
    grids: Vec<Grid>,
    /// colour slots named by the palette
    slots: Vec<String>,
    palette: Vec<Option<Recipe>>,
    lods: Vec<LodSpec>,
    per_block: f32,
    smooth: Smooth,
    derive: Vec<DeriveSpec>,
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
                crisp: r.crisp,
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
            detail: g.detail.unwrap_or(spec.detail).max(0.01),
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
        lods: spec.lods.list(),
        per_block: spec.per_block,
        derive: spec.derive.clone(),
        smooth: spec
            .smooth
            .map(|s| Smooth {
                bevel: s.bevel,
                soft: s.soft,
                relax: s.relax,
                blend: s.blend,
                occlusion: s.occlusion,
                simplify: 0.0,
            })
            .unwrap_or_default(),
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
    for (socket, h) in &spec.carry {
        l.carry.insert(socket.clone(), h.clone());
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
            l.set(name, f.get(field(&f.field)));
        }
        if let Some(base) = &s.appearance.base {
            apply_spec(&s, &mut l, base, color);
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

    /// Put on items (by key), as `figures.json`'s `equip` says each one
    /// changes the look. Items it does not know change nothing.
    pub fn wear<'a>(&mut self, items: impl IntoIterator<Item = &'a str>) {
        let Some(s) = spec() else {
            return;
        };
        for key in items {
            if let Some(e) = s.equip.items.get(key) {
                apply_spec(&s, self, e, [0.5; 3]);
            }
        }
    }

    /// The steppers the character creator offers (features with a label),
    /// in the pack's order of fields.
    pub fn steppers() -> Vec<Stepper> {
        let Some(s) = spec() else {
            return Vec::new();
        };
        let mut v: Vec<Stepper> = s
            .appearance
            .features
            .values()
            .filter_map(|f| {
                Some(Stepper {
                    label: f.label.clone()?,
                    field: f.field.clone(),
                    names: f.names.clone(),
                    spec: f.clone(),
                })
            })
            .collect();
        let order = ["beard", "build", "mood", "ears"];
        v.sort_by_key(|st| {
            (
                order.iter().position(|f| *f == st.field).unwrap_or(9),
                std::cmp::Reverse(st.spec.div),
            )
        });
        v
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

/// The looks of the showcase in `figures.json`: per set, its name and
/// each person in it.
pub fn showcase() -> Vec<(String, Vec<(String, Look)>)> {
    let Some(s) = spec() else {
        return Vec::new();
    };
    let person = |p: &ShowPerson| -> Look {
        if let Some(key) = &p.npc {
            return Look::npc(key, p.color);
        }
        let a = p.appearance;
        let mut l = Look::from_appearance(
            &dyno_protocol::Appearance {
                skin: a.skin,
                eyes: a.eyes,
                hair_style: a.hair_style,
                hair_color: a.hair_color,
                beard: a.beard,
                build: a.build,
                mood: a.mood,
                ears: a.ears,
            },
            p.color,
        );
        apply_spec(&s, &mut l, &p.look, p.color);
        l
    };
    let all = |on: &Option<Vec<usize>>| -> Vec<usize> {
        on.clone()
            .unwrap_or_else(|| (0..s.showcase.people.len()).collect())
    };
    if s.showcase.sets.is_empty() {
        let people = s
            .showcase
            .people
            .iter()
            .map(|p| (p.name.clone(), person(p)))
            .collect();
        return vec![("People".into(), people)];
    }
    s.showcase
        .sets
        .iter()
        .map(|set| {
            let people = all(&set.on)
                .into_iter()
                .filter_map(|i| s.showcase.people.get(i))
                .map(|p| {
                    let mut l = person(p);
                    l.wear(set.wear.iter().map(String::as_str));
                    apply_spec(&s, &mut l, &set.look, p.color);
                    (p.name.clone(), l)
                })
                .collect();
            (set.name.clone(), people)
        })
        .collect()
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

    fn color(&self, ink: u8, look: &Look) -> Option<([f32; 3], bool, bool)> {
        let r = self.palette.get(ink as usize).copied().flatten()?;
        let base = match r.slot {
            Some(s) => look.color(&self.slots[s]).unwrap_or([0.5; 3]),
            None => [0.0; 3],
        };
        Some((
            std::array::from_fn(|i| (r.k[i] * base[i] + r.c[i]).clamp(0.0, 1.0)),
            r.glow,
            r.crisp,
        ))
    }

    /// The look with the features its figure derives from others.
    pub fn derived(&self, look: &Look) -> Look {
        let mut l = look.clone();
        for d in &self.derive {
            let on = d.cond.iter().all(|(f, vals)| vals.contains(&l.feature(f)));
            if on {
                for (f, v) in &d.set {
                    if !l.features.contains_key(f) {
                        l.set(f, *v);
                    }
                }
                for (name, g) in &d.colors {
                    if !l.colors.contains_key(name) {
                        let base = l.colors.clone();
                        let c = look.color("color").unwrap_or([0.5; 3]);
                        if let Some(c) = gen_color(g, 0, &base, c, &l) {
                            l.colors.insert(name.clone(), c);
                        }
                    }
                }
            }
        }
        l
    }

    /// The voxel models of a look, one per grid.
    pub fn compose(&self, look: &Look) -> Vec<Vox> {
        let mut out = Vec::new();
        for g in &self.grids {
            let mut v = Vox::new(g.min, g.max);
            v.set_unit(1.0 / g.detail);
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
                            let (rgb, glow, crisp) =
                                self.color(ink, look)
                                    .unwrap_or(([1.0, 0.0, 1.0], false, false));
                            let i = v.ink_as(rgb, glow, crisp);
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
        let look = &self.derived(look);
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

/// A look's voxels, and its meshes per level of detail, per rig part,
/// each made the first time it is drawn.
struct Built {
    grids: Vec<Vox>,
    /// where each rig bone sits on its parent, for this look
    ats: Vec<Vec3>,
    lods: Vec<std::cell::OnceCell<Vec<Mesh>>>,
}

impl Built {
    fn level(&self, fig: &Figure, look: &Look, i: usize) -> &[Mesh] {
        let i = i.min(self.lods.len() - 1);
        self.lods[i].get_or_init(|| mesh_level(fig, look, &self.grids, &fig.lods[i]))
    }
}

fn build(fig: &Figure, look: &Look) -> Built {
    let look = &fig.derived(look);
    Built {
        grids: fig.compose(look),
        ats: fig
            .rig
            .iter()
            .flat_map(|r| r.bones.iter().map(|b| b.at.vec(look)))
            .collect(),
        lods: fig
            .lods
            .iter()
            .map(|_| std::cell::OnceCell::new())
            .collect(),
    }
}

/// Mesh one level of detail: halve the voxels, then mesh each rig part
/// (smooth parts together, so they meet without seams) and reshape it.
fn mesh_level(fig: &Figure, look: &Look, full: &[Vox], lod: &LodSpec) -> Vec<Mesh> {
    let mut halved: Vec<Vox> = Vec::new();
    let grids: &[Vox] = if (0..full.len()).all(|g| lod.half.of(g) == 0) {
        full
    } else {
        for (gi, g) in full.iter().enumerate() {
            let n = lod.half.of(gi);
            let mut v = if n == 0 { g.copy() } else { g.half() };
            for _ in 1..n {
                v = v.half();
            }
            halved.push(v);
        }
        &halved
    };
    let opts = Smooth {
        simplify: lod.simplify,
        ..fig.smooth
    };
    let Some(rig) = &fig.rig else {
        // no rig: the first part of the first grid, about its origin
        return vec![if lod.smooth {
            grids[0]
                .mesh_smooth(&[(1, [0.0; 3])], opts)
                .pop()
                .unwrap_or_default()
        } else {
            grids[0].mesh(1, [0.0; 3])
        }];
    };
    let mut out: Vec<Mesh> = rig.parts.iter().map(|_| Mesh::default()).collect();
    for (gi, grid) in grids.iter().enumerate() {
        let smooth: Vec<usize> = (0..rig.parts.len())
            .filter(|&i| {
                let p = &rig.parts[i];
                p.held.is_none() && p.grid == gi && lod.smooth && p.smooth
            })
            .collect();
        if !smooth.is_empty() {
            let asks: Vec<(u8, [f32; 3])> = smooth
                .iter()
                .map(|&i| (rig.parts[i].part, rig.parts[i].pivot.vec(look).into()))
                .collect();
            for (i, m) in smooth.iter().zip(grid.mesh_smooth(&asks, opts)) {
                out[*i] = m;
            }
        }
        for (i, p) in rig.parts.iter().enumerate() {
            if p.held.is_some() || p.grid != gi || smooth.contains(&i) {
                continue;
            }
            out[i] = grid.mesh(p.part, p.pivot.vec(look).into());
        }
    }
    for (p, m) in rig.parts.iter().zip(&mut out) {
        if let Some(s) = p.shape.as_ref().and_then(|s| s.get(look)) {
            reshape(m, s);
        }
    }
    out
}

/// Apply a part's shape from the rig: scale, then a taper from top to bottom.
fn reshape(mesh: &mut Mesh, s: ShapeSpec) {
    let (lo, hi) = mesh.y_range();
    let span = (hi - lo).max(1e-3);
    let [sx, sy, sz] = s.scale;
    let f = move |p: [f32; 3]| {
        let (wx, wz) = match s.taper {
            Some([top, bottom]) => {
                let t = (hi - p[1]) / span;
                let w = top + (bottom - top) * t;
                (w, if s.taper_depth { w } else { 1.0 })
            }
            None => (1.0, 1.0),
        };
        [p[0] * sx * wx, p[1] * sy, p[2] * sz * wz]
    };
    mesh.reshape(&f);
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
fn lod_at(lods: &[LodSpec], p: Vec3) -> usize {
    let Some(d) = view_distance(p) else {
        return 0;
    };
    lods.iter()
        .position(|l| l.until.is_none_or(|u| d <= u))
        .unwrap_or(lods.len().saturating_sub(1))
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
    fig: Rc<Figure>,
    look: Look,
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
        fig,
        look,
        unit: k.unit,
        carry_turn: k.carry_turn,
    })
}

impl Holding {
    pub fn mesh(&self) -> &Mesh {
        &self.held.level(&self.fig, &self.look, 0)[0]
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
    let built = built(name, fig, look);
    let derived = fig.derived(look);
    let m = built.level(fig, &derived, lod_at(&fig.lods, feet));
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
    for (b, at) in rig.bones.iter().zip(&built.ats) {
        let parent = b.parent.map(|p| frames[p]).unwrap_or(base);
        let off = b.pose.map(|i| pose.off[i]).unwrap_or(Vec3::ZERO);
        let mut f = parent * Affine3A::from_translation(*at + off);
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
        if let Some(socket) = &p.held {
            let other = (!socket.is_empty())
                .then(|| look.carry.get(socket))
                .flatten()
                .and_then(|(kind, c)| holding(kind, *c, 1, false, 0));
            let h = if socket.is_empty() {
                held
            } else {
                other.as_ref()
            };
            if let Some(h) = h {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        crate::assets::load_dir(&dir).expect("the content pack");
    }

    /// Wearing an item changes the look as `equip` says, and the figure
    /// composes with it.
    #[test]
    fn wearing_items_changes_the_figure() {
        pack();
        let mut l = Look::from_appearance(&Default::default(), [0.5; 3]);
        let bare = l.clone();
        l.wear(["ironbound_cuirass", "ironbound_pauldrons", "no_such_item"]);
        assert_eq!(l.feature("chest"), 4);
        assert_eq!(l.feature("shoulders"), 4);
        assert!(l.color("chest.main").is_some());
        let fig = figure(&player_figure().unwrap()).unwrap();
        let count = |l: &Look| -> usize {
            fig.compose(&fig.derived(l))
                .iter()
                .map(|v| {
                    let (lo, hi) = v.bounds();
                    let mut n = 0;
                    for x in lo[0]..hi[0] {
                        for y in lo[1]..hi[1] {
                            for z in lo[2]..hi[2] {
                                n += usize::from(v.solid(x, y, z));
                            }
                        }
                    }
                    n
                })
                .sum()
        };
        assert!(count(&l) > count(&bare), "armour adds voxels");
        // old looks keep composing through the derived features
        let mut old = Look::from_appearance(&Default::default(), [0.5; 3]);
        old.set("armor", 2);
        assert_eq!(fig.derived(&old).feature("chest"), 3);
    }

    /// Both bodies stand about 1.78 m (bald, bare-headed).
    #[test]
    fn people_are_life_size() {
        pack();
        for body in [0u8, 1] {
            let mut l = Look::from_appearance(&Default::default(), [0.5; 3]);
            l.set("body", body);
            l.set("hair_style", 0);
            let mut out = Vec::new();
            crate::models::push_person(&mut out, Vec3::ZERO, 0.0, 0.0, &l, 0.8, 0.0, 0.0);
            let (lo, hi) = out.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| {
                (lo.min(v.pos[1]), hi.max(v.pos[1]))
            });
            println!("body {body}: {lo:.3} to {hi:.3} m");
            assert!(
                lo > -0.05 && lo < 0.05,
                "body {body} stands on the ground ({lo})"
            );
            assert!((1.72..1.84).contains(&hi), "body {body} is {hi} m tall");
        }
    }

    /// Prints how many vertices each level of detail of a player costs,
    /// part by part. `cargo test -p dyno-client --release figure_budget -- --nocapture`
    #[test]
    fn figure_budget() {
        pack();
        let name = player_figure().expect("a player figure");
        let fig = figure(&name).expect("the player figure loads");
        let look = Look::player(3, [0.6, 0.2, 0.2]);
        let start = std::time::Instant::now();
        let b = build(&fig, &look);
        let compose = start.elapsed();
        let rig = fig.rig.as_ref().unwrap();
        for i in 0..fig.lods.len() {
            let start = std::time::Instant::now();
            let m = b.level(&fig, &fig.derived(&look), i);
            let took = start.elapsed();
            let total: usize = m.iter().map(|m| m.verts.len()).sum();
            let parts: Vec<String> = rig
                .parts
                .iter()
                .zip(m)
                .filter(|(_, m)| !m.verts.is_empty())
                .map(|(p, m)| format!("{}:{}", p.part, m.verts.len()))
                .collect();
            println!(
                "lod {i}: {total} vertices in {took:?} ({})",
                parts.join(" ")
            );
        }
        println!("compose: {compose:?}");
    }
}
