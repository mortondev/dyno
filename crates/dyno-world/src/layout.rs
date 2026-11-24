//! The layout of a world as data (`world/layout.json` in the pack): how
//! big it is, the shape of the land, the river and lakes, zones and what
//! grows in them, roads, and every place with the stamps that stand there.
//!
//! Positions are (x, z) offsets from the origin (the spawn hearth) in
//! metres, written for a world of `design` size; north is −z, east is +x. A
//! layout whose `size` is smaller than its `design` draws every distance
//! between places in proportion (`spread`), so one value takes the same
//! data from a development size to the full one. Sizes of things (stamps,
//! road widths, trees, pads) are never scaled.
//!
//! Nothing here touches a world: [`crate::worldgen`] builds one from a
//! layout and the pack's stamps.

use super::{hash, value_noise};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// The layout that ships with Dyno: the Hearthblock pack's (for tools and
/// tests; the server reads its pack).
pub const DEFAULT_JSON: &str = include_str!("../../../content/world/layout.json");

fn design_size() -> [u32; 3] {
    [2048, 256, 2048]
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    /// bumped when the world must be rebuilt from the layout (the server
    /// keeps the version its saved world was built from)
    pub version: u32,
    /// the world this layout builds
    pub size: [u32; 3],
    /// the size the positions are written for
    #[serde(default = "design_size")]
    pub design: [u32; 3],
    pub seed: u32,
    pub terrain: Terrain,
    #[serde(default)]
    pub zones: Vec<Zone>,
    pub flora: Flora,
    /// road kinds ("road", "trail"): width and surface
    pub ways: BTreeMap<String, Way>,
    #[serde(default)]
    pub roads: Vec<Road>,
    pub places: Vec<Place>,
    /// how a world saved before this layout maps onto it
    #[serde(default)]
    pub migrate: Option<Migrate>,
    /// where the pack's kinds of water (`render.json`, `water.kinds`) are
    /// seen: still water round places. Rivers carry their own kind.
    #[serde(default)]
    pub water: Vec<WaterAt>,
}

/// A kind of water round a place.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterAt {
    pub kind: String,
    pub place: String,
    /// offset from the place, in metres
    #[serde(default)]
    pub at: [i32; 2],
    pub radius: [f32; 2],
    #[serde(default)]
    pub feather: Option<f32>,
}

/// The world this layout replaces: its width, for positions out in the
/// country (places say where they were with `was`).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Migrate {
    pub from_size: u32,
}

/// Where a place was in the world before this layout: an offset from that
/// world's spawn hearth, and how far it reached.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Was {
    pub at: [i32; 2],
    pub radius: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    /// water level (the y of the highest water block)
    pub sea: i32,
    /// height of open ground before anything shapes it
    pub ground: f32,
    /// layers of rolling noise: wavelength in metres and amplitude
    #[serde(default)]
    pub rolling: Vec<Wave>,
    #[serde(default)]
    pub hills: Vec<Hill>,
    #[serde(default)]
    pub ridges: Vec<Ridge>,
    /// the mountains that close the zone at the edge of the world
    #[serde(default)]
    pub rim: Option<Rim>,
    #[serde(default)]
    pub lakes: Vec<Lake>,
    #[serde(default)]
    pub rivers: Vec<River>,
    /// bare rock above this height over the open ground, snow above the second
    #[serde(default)]
    pub rock_line: Option<[f32; 2]>,
    /// how the water's edge is made: stretches of shelving stone, and
    /// muddy shallows between them where reeds can grow
    #[serde(default)]
    pub banks: Option<Banks>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Banks {
    /// the stone of the stony stretches, with weights
    pub stone: BTreeMap<String, u32>,
    /// share of the shore that is stone (0 to 1), and the length of a stretch
    pub share: f32,
    pub scale: f32,
    /// the block of the muddy margins, at and just under the waterline
    pub mud: String,
    /// one column in this many of a stony stretch carries a boulder
    #[serde(default)]
    pub boulder_every: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub scale: f32,
    pub amp: f32,
}

/// A hill or a group of hills: an oval that rises `rise` metres.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hill {
    pub at: [f32; 2],
    pub radius: [f32; 2],
    pub rise: f32,
    /// 0 is one smooth dome; towards 1 it breaks into separate tops
    #[serde(default)]
    pub rough: f32,
}

/// A mountain ridge along a line.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ridge {
    pub key: String,
    pub path: Vec<[f32; 2]>,
    /// half-width of its foot
    pub width: f32,
    pub rise: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rim {
    pub width: f32,
    pub rise: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lake {
    pub key: String,
    pub at: [f32; 2],
    pub radius: [f32; 2],
    pub depth: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct River {
    pub key: String,
    /// the kind of water it runs with (`render.json`, `water.kinds`)
    #[serde(default = "river_look")]
    pub look: String,
    pub path: Vec<[f32; 2]>,
    pub width: f32,
    pub depth: f32,
    /// how far the valley's sides reach from the banks
    pub valley: f32,
}

/// A region with its own vegetation. The first zone that holds a column wins.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub key: String,
    pub at: [f32; 2],
    pub radius: [f32; 2],
    /// a mix from `flora.mixes`
    pub flora: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flora {
    pub species: BTreeMap<String, Species>,
    pub mixes: BTreeMap<String, Mix>,
    /// the mix of land outside every zone
    pub default: String,
}

/// A kind of tree: every size is a range in metres.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Species {
    pub trunk: String,
    /// absent: a bare snag
    #[serde(default)]
    pub leaves: Option<String>,
    pub height: [i32; 2],
    /// trunk thickness
    pub girth: [i32; 2],
    pub crown: Crown,
    /// height of the lowest leaves, as a share of the tree's height
    #[serde(default = "default_skirt")]
    pub skirt: f32,
    /// limbs that reach out of the trunk into the crown
    #[serde(default)]
    pub limbs: u8,
    /// the trunk spreads at the foot
    #[serde(default)]
    pub roots: bool,
    /// share of the crown's leaves that are missing
    #[serde(default)]
    pub thin: f32,
    /// a second, lower ring of roots beyond the first (with `roots`)
    #[serde(default)]
    pub flare: bool,
}

fn default_skirt() -> f32 {
    0.45
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crown {
    /// "round" (a lumpy dome of leaf clumps), "tall" (clumps up the trunk,
    /// as a birch), "weeping" (a dome with curtains hanging from its rim),
    /// "cone" (tiers), "none" (bare: `limbs` are its branches)
    pub shape: String,
    /// width across, a range
    #[serde(default)]
    pub width: [i32; 2],
    /// how many clumps stand round the middle one, a range
    #[serde(default = "default_clumps")]
    pub clumps: [u32; 2],
    /// the middle clump's radius, as a share of half the width
    #[serde(default = "default_core")]
    pub core: f32,
    /// the other clumps' radius, as a share of half the width, a range
    #[serde(default = "default_lump")]
    pub lump: [f32; 2],
    /// how high in the crown the other clumps sit (0 its foot, 1 its top),
    /// a range
    #[serde(default = "default_rise")]
    pub rise: [f32; 2],
    /// how much sky shows between the clumps (0: they run together)
    #[serde(default)]
    pub gaps: f32,
    /// "weeping": how far the curtains hang below the dome, a range
    #[serde(default)]
    pub droop: [i32; 2],
}

fn default_clumps() -> [u32; 2] {
    [3, 5]
}

fn default_core() -> f32 {
    0.8
}

fn default_lump() -> [f32; 2] {
    [0.42, 0.54]
}

fn default_rise() -> [f32; 2] {
    [0.3, 0.55]
}

/// What grows in a zone: woods in clumps, open ground between them.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mix {
    /// share of the land that is wood (0 to 1)
    pub cover: f32,
    /// size of the clumps of wood, in metres
    pub clump: f32,
    /// metres between trees inside a wood
    pub spacing: f32,
    /// species and weights
    pub trees: BTreeMap<String, u32>,
    /// lone trees on open ground, per hectare
    #[serde(default)]
    pub lone: f32,
    /// small plants and stones: block, per hectare, and whether it wants
    /// wood ("wood"), open ground ("open") or either ("any")
    #[serde(default)]
    pub ground: Vec<Dress>,
    /// patches of another surface block (bare earth, rock)
    #[serde(default)]
    pub patches: Option<Patch>,
    /// stamps strewn over the land (boulders, crags, fallen logs): per
    /// hectare, on "wood", "open" or "any" ground
    #[serde(default)]
    pub scatter: Vec<Strewn>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strewn {
    pub stamp: String,
    pub per_ha: f32,
    #[serde(default = "any")]
    pub on: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dress {
    pub block: String,
    pub per_ha: f32,
    #[serde(default = "any")]
    pub on: String,
}

fn any() -> String {
    "any".into()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub block: String,
    pub share: f32,
    pub scale: f32,
}

/// A kind of road.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Way {
    pub width: i32,
    /// surface blocks with weights
    pub surface: BTreeMap<String, u32>,
    /// a stamp set beside the way every so many metres (lamp posts)
    #[serde(default)]
    pub lamp: Option<String>,
    #[serde(default)]
    pub lamp_every: f32,
    /// the block of the kerb: the outermost metre on each side
    #[serde(default)]
    pub kerb: Option<String>,
    /// crosses shallow water on a raised bed instead of a bridge
    #[serde(default)]
    pub ford: bool,
    #[serde(default = "cobble")]
    pub ford_bed: String,
}

fn cobble() -> String {
    "cobble".into()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Road {
    pub key: String,
    pub way: String,
    /// points along it: a place key, or an offset `[x, z]`
    pub path: Vec<Stop>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    Place(String),
    At([f32; 2]),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub key: String,
    #[serde(default)]
    pub name: String,
    /// "hub", "site", "wayside", "camp", "view"…: for tools and tests
    pub kind: String,
    pub at: [f32; 2],
    /// how far the place reaches (areas of the rules take this radius)
    #[serde(default)]
    pub radius: f32,
    /// the radius is a stretch of country: it grows with the map
    #[serde(default)]
    pub wide: bool,
    /// levelled ground
    #[serde(default)]
    pub pad: Option<Pad>,
    /// a paved middle (a town square)
    #[serde(default)]
    pub plaza: Option<Plaza>,
    /// "ground" (the land's own height, the default) or "water" (just
    /// above the water: landings, things that stand in a lake)
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub stamps: Vec<Put>,
    #[serde(default)]
    pub trees: Vec<Planted>,
    #[serde(default)]
    pub was: Option<Was>,
    /// counts as something to see from the road (density tests)
    #[serde(default = "yes")]
    pub sight: bool,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pad {
    pub radius: [f32; 2],
    #[serde(default = "default_blend")]
    pub blend: f32,
    /// surface blocks with weights; absent keeps grass
    #[serde(default)]
    pub surface: Option<BTreeMap<String, u32>>,
    /// keep the wild trees (a camp in a wood)
    #[serde(default)]
    pub wooded: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plaza {
    pub radius: f32,
    pub surface: BTreeMap<String, u32>,
}

fn default_blend() -> f32 {
    10.0
}

/// A stamp at a place.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Put {
    pub stamp: String,
    /// offset from the place, in metres (never scaled)
    #[serde(default)]
    pub at: [i32; 2],
    /// quarter turns clockwise
    #[serde(default)]
    pub rot: u8,
    /// metres above (or below) the ground it stands on
    #[serde(default)]
    pub dy: i32,
    /// "pad" (the place's level), "ground" (the land under it, levelled
    /// under its footprint), "float" (the land under it, as it is)
    #[serde(default)]
    pub fit: Option<String>,
}

/// A tree planted by hand (hero trees at landmarks and glades).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Planted {
    pub species: String,
    #[serde(default)]
    pub at: [i32; 2],
}

fn river_look() -> String {
    "river".into()
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Distance from a point to a polyline.
pub(crate) fn path_distance(p: [f32; 2], path: &[[f32; 2]]) -> f32 {
    let mut best = f32::MAX;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (vx, vz) = (b[0] - a[0], b[1] - a[1]);
        let len2 = vx * vx + vz * vz;
        let t = if len2 > 0.0 {
            (((p[0] - a[0]) * vx + (p[1] - a[1]) * vz) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (dx, dz) = (p[0] - a[0] - vx * t, p[1] - a[1] - vz * t);
        best = best.min((dx * dx + dz * dz).sqrt());
    }
    if path.len() == 1 {
        best = ((p[0] - path[0][0]).powi(2) + (p[1] - path[0][1]).powi(2)).sqrt();
    }
    best
}

/// A smooth curve through points, sampled about every `step` metres.
pub(crate) fn spline(pts: &[[f32; 2]], step: f32) -> Vec<[f32; 2]> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let at = |i: isize| pts[i.clamp(0, n as isize - 1) as usize];
    let mut out = Vec::new();
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (
            at(i as isize - 1),
            at(i as isize),
            at(i as isize + 1),
            at(i as isize + 2),
        );
        let len = ((p2[0] - p1[0]).powi(2) + (p2[1] - p1[1]).powi(2)).sqrt();
        let steps = (len / step).ceil().max(1.0) as usize;
        for k in 0..steps {
            let t = k as f32 / steps as f32;
            let (t2, t3) = (t * t, t * t * t);
            let c = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b
                    + (c - a) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (3.0 * b - a - 3.0 * c + d) * t3)
            };
            out.push([c(p0[0], p1[0], p2[0], p3[0]), c(p0[1], p1[1], p2[1], p3[1])]);
        }
    }
    out.push(pts[n - 1]);
    out
}

impl Layout {
    pub fn from_json(text: &str) -> Result<Layout, String> {
        let l: Layout = serde_json::from_str(text).map_err(|e| format!("layout: {e}"))?;
        l.check()?;
        Ok(l)
    }

    /// How much distances between places are drawn in: the world's width
    /// over the width the layout is written for.
    pub fn spread(&self) -> f32 {
        self.size[0] as f32 / self.design[0] as f32
    }

    /// How much of the designed height of hills and ridges fits under this
    /// world's sky.
    pub fn rise(&self) -> f32 {
        let room = |h: u32| h as f32 - self.terrain.ground - 34.0;
        (room(self.size[1]) / room(self.design[1])).clamp(0.2, 1.0)
    }

    /// A design position as an offset from the origin in this world.
    pub fn pos(&self, at: [f32; 2]) -> [f32; 2] {
        let s = self.spread();
        [at[0] * s, at[1] * s]
    }

    pub fn place(&self, key: &str) -> Option<&Place> {
        self.places.iter().find(|p| p.key == key)
    }

    /// Where a place is: its offset from the origin, in whole metres.
    pub fn place_at(&self, p: &Place) -> [i32; 2] {
        let a = self.pos(p.at);
        [a[0].round() as i32, a[1].round() as i32]
    }

    /// The reach of a place in this world.
    pub fn place_radius(&self, p: &Place) -> f32 {
        if p.wide {
            (p.radius * self.spread()).max(p.radius.min(40.0))
        } else {
            p.radius
        }
    }

    /// The rivers as they run in this world: key, width, and the centre
    /// line as offsets from the origin, a point about every 6 m. This is the
    /// one source of a river's course (the land is carved along it); what
    /// draws flowing water should read it from here.
    pub fn river_courses(&self) -> Vec<(&str, f32, Vec<[f32; 2]>)> {
        self.terrain
            .rivers
            .iter()
            .map(|r| {
                let pts: Vec<[f32; 2]> = r.path.iter().map(|p| self.pos(*p)).collect();
                (r.key.as_str(), r.width, spline(&pts, 6.0))
            })
            .collect()
    }

    /// Put the water where this layout has it, in a pack's render settings:
    /// `water.courses` (the rivers, as points with half-widths) and
    /// `water.areas` (kinds of still water round places), both from the
    /// spawn hearth. The layout is the one source of where water is; the
    /// render settings say how each kind looks.
    pub fn place_water(&self, render: &mut Value) -> Result<(), String> {
        let Some(water) = render.get_mut("water").and_then(|w| w.as_object_mut()) else {
            return Ok(());
        };
        let courses: Vec<Value> = self
            .terrain
            .rivers
            .iter()
            .zip(self.river_courses())
            .map(|(r, (_, width, line))| {
                // a point about every 24 m follows the bends closely enough
                let points: Vec<Value> = line
                    .iter()
                    .step_by(4)
                    .chain(line.last())
                    .map(|p| serde_json::json!([p[0].round(), p[1].round(), width / 2.0]))
                    .collect();
                serde_json::json!({ "kind": r.look, "points": points })
            })
            .collect();
        let mut areas = Vec::new();
        for a in &self.water {
            let p = self
                .place(&a.place)
                .ok_or(format!("layout: water at unknown place \"{}\"", a.place))?;
            let c = self.place_at(p);
            let mut v = serde_json::json!({
                "kind": a.kind,
                "at": [c[0] + a.at[0], c[1] + a.at[1]],
                "radius": a.radius,
            });
            if let Some(f) = a.feather {
                v["feather"] = serde_json::json!(f);
            }
            areas.push(v);
        }
        water.insert("courses".into(), Value::Array(courses));
        water.insert("areas".into(), Value::Array(areas));
        Ok(())
    }

    /// The points of a road as offsets in this world.
    pub fn road_points(&self, r: &Road) -> Vec<[f32; 2]> {
        r.path
            .iter()
            .filter_map(|s| match s {
                Stop::At(p) => Some(self.pos(*p)),
                Stop::Place(k) => self.place(k).map(|p| self.pos(p.at)),
            })
            .collect()
    }

    fn noise(&self, x: f32, z: f32, scale: f32, salt: u32) -> f32 {
        value_noise(x / scale, z / scale, self.seed.wrapping_add(salt))
    }

    /// The land's own height at an offset from the origin, before pads and
    /// roads: a pure function of the layout. For many columns, make one
    /// [`Land`] and ask it.
    pub fn height(&self, x: f32, z: f32) -> f32 {
        Land::new(self).height(x, z)
    }

    /// The level of a place's ground: the y of its top block of earth.
    pub fn level(&self, p: &Place) -> i32 {
        if p.level.as_deref() == Some("water") {
            return self.terrain.sea + 1;
        }
        let a = self.place_at(p);
        (self.height(a[0] as f32, a[1] as f32).round() as i32).max(self.terrain.sea + 1)
    }

    /// The mix that grows at an offset.
    pub fn mix_at(&self, x: f32, z: f32) -> &str {
        let s = self.spread();
        let n = (self.noise(x, z, 41.0, 90) - 0.5) * 0.3;
        for zone in &self.zones {
            let c = self.pos(zone.at);
            let (rx, rz) = (zone.radius[0] * s, zone.radius[1] * s);
            let m = (((x - c[0]) / rx).powi(2) + ((z - c[1]) / rz).powi(2)).sqrt() + n;
            if m < 1.0 {
                return &zone.flora;
            }
        }
        &self.flora.default
    }

    /// Pick from weights with a hash.
    pub(crate) fn pick(of: &BTreeMap<String, u32>, r: u32) -> Option<&str> {
        let total: u32 = of.values().sum();
        if total == 0 {
            return None;
        }
        let mut n = r % total;
        for (k, w) in of {
            if n < *w {
                return Some(k);
            }
            n -= w;
        }
        None
    }

    /// Everything a layout must get right on its own, before any stamp or
    /// world is involved.
    fn check(&self) -> Result<(), String> {
        let block = |k: &str| {
            crate::block_by_key(k)
                .map(|_| ())
                .ok_or(format!("layout: unknown block \"{k}\""))
        };
        if let Some(b) = &self.terrain.banks {
            block(&b.mud)?;
            for k in b.stone.keys() {
                block(k)?;
            }
        }
        if self.size.iter().any(|&s| s < 16) || self.design.iter().any(|&s| s < 16) {
            return Err("layout: the world is too small".into());
        }
        let mut seen = std::collections::HashSet::new();
        for p in &self.places {
            if !seen.insert(&p.key) {
                return Err(format!("layout: place \"{}\" is listed twice", p.key));
            }
            for t in &p.trees {
                if !self.flora.species.contains_key(&t.species) {
                    return Err(format!(
                        "layout: place \"{}\" plants unknown species \"{}\"",
                        p.key, t.species
                    ));
                }
            }
            if let Some(l) = &p.level
                && l != "ground"
                && l != "water"
            {
                return Err(format!("layout: place \"{}\" has level \"{l}\"", p.key));
            }
            for s in p
                .pad
                .iter()
                .filter_map(|pad| pad.surface.as_ref())
                .chain(p.plaza.iter().map(|z| &z.surface))
            {
                for k in s.keys() {
                    block(k)?;
                }
            }
            for put in &p.stamps {
                if let Some(f) = &put.fit
                    && !["pad", "ground", "float"].contains(&f.as_str())
                {
                    return Err(format!("layout: place \"{}\" has fit \"{f}\"", p.key));
                }
            }
        }
        if self.places.first().map(|p| p.kind.as_str()) != Some("hub") {
            return Err("layout: the first place must be the starting hub".into());
        }
        for r in &self.roads {
            let way = self.ways.get(&r.way).ok_or(format!(
                "layout: road \"{}\" has unknown way \"{}\"",
                r.key, r.way
            ))?;
            for k in way.surface.keys().chain(way.kerb.iter()) {
                block(k)?;
            }
            for s in &r.path {
                if let Stop::Place(k) = s
                    && self.place(k).is_none()
                {
                    return Err(format!(
                        "layout: road \"{}\" goes to unknown place \"{k}\"",
                        r.key
                    ));
                }
            }
            if r.path.len() < 2 {
                return Err(format!("layout: road \"{}\" needs two points", r.key));
            }
        }
        for (k, sp) in &self.flora.species {
            block(&sp.trunk)?;
            if let Some(l) = &sp.leaves {
                block(l)?;
            }
            if sp.height[0] > sp.height[1] || sp.girth[0] > sp.girth[1] || sp.girth[0] < 1 {
                return Err(format!("layout: species \"{k}\" has a bad size range"));
            }
            if !["round", "tall", "weeping", "cone", "none"].contains(&sp.crown.shape.as_str()) {
                return Err(format!("layout: species \"{k}\" has an unknown crown"));
            }
        }
        for (k, m) in &self.flora.mixes {
            for sp in m.trees.keys() {
                if !self.flora.species.contains_key(sp) {
                    return Err(format!(
                        "layout: mix \"{k}\" grows unknown species \"{sp}\""
                    ));
                }
            }
            for d in &m.ground {
                block(&d.block)?;
                if !["any", "wood", "open"].contains(&d.on.as_str()) {
                    return Err(format!("layout: mix \"{k}\" has ground on \"{}\"", d.on));
                }
            }
            if let Some(p) = &m.patches {
                block(&p.block)?;
            }
            if !(0.0..=1.0).contains(&m.cover) || m.spacing < 2.0 || m.clump < 8.0 {
                return Err(format!(
                    "layout: mix \"{k}\" has a bad cover, clump or spacing"
                ));
            }
        }
        let mix = |k: &str| {
            self.flora
                .mixes
                .contains_key(k)
                .then_some(())
                .ok_or(format!("layout: unknown flora mix \"{k}\""))
        };
        mix(&self.flora.default)?;
        for z in &self.zones {
            mix(&z.flora)?;
        }
        Ok(())
    }

    /// Check the layout against the pack's stamps.
    pub fn check_stamps(&self, stamps: &crate::stamp::Stamps) -> Result<(), String> {
        for p in &self.places {
            for put in &p.stamps {
                if stamps.get(&put.stamp).is_none() {
                    return Err(format!(
                        "layout: place \"{}\" uses unknown stamp \"{}\"",
                        p.key, put.stamp
                    ));
                }
            }
        }
        for (k, m) in &self.flora.mixes {
            for sc in &m.scatter {
                if stamps.get(&sc.stamp).is_none() {
                    return Err(format!(
                        "layout: mix \"{k}\" strews unknown stamp \"{}\"",
                        sc.stamp
                    ));
                }
            }
        }
        for (k, w) in &self.ways {
            if let Some(l) = &w.lamp
                && stamps.get(l).is_none()
            {
                return Err(format!("layout: way \"{k}\" uses unknown stamp \"{l}\""));
            }
        }
        Ok(())
    }

    /// Rules place things by place: any object in the rules with a
    /// `"place"` key has its `"at"` (an offset from that place, `[x, z]` or
    /// `[x, y, z]`) turned into an offset from the origin, which is what the
    /// rules' readers (server and client) work with. `"radius": "place"`
    /// takes the place's reach, and `"below": n` (underground camps) becomes
    /// a height `"y"` n metres under the place's ground.
    pub fn resolve_rules(&self, rules: &mut Value) -> Result<(), String> {
        let hub = self.level(&self.places[0]) + 1;
        // someone placed from an area's centre (`"area": "windmill"`)
        // stands at the height of that area's place: heights are the
        // layout's to give, whatever the land was when the rules were written
        let mut heights: BTreeMap<String, i64> = BTreeMap::new();
        for a in rules
            .get("areas")
            .and_then(|a| a.as_array())
            .into_iter()
            .flatten()
        {
            if let (Some(key), Some(place)) = (
                a.get("key").and_then(|k| k.as_str()),
                a.get("place").and_then(|k| k.as_str()),
            ) && let Some(p) = self.place(place)
            {
                heights.insert(key.to_string(), (self.level(p) + 1 - hub) as i64);
            }
        }
        for n in rules
            .get_mut("npcs")
            .and_then(|n| n.as_array_mut())
            .into_iter()
            .flatten()
        {
            let up = n
                .get("area")
                .and_then(|a| a.as_str())
                .and_then(|a| heights.get(a))
                .copied();
            if let (Some(up), Some(at)) = (up, n.get_mut("at").and_then(|a| a.as_array_mut()))
                && at.len() == 3
            {
                at[1] = Value::from(up);
            }
        }
        self.resolve(rules, hub)
    }

    fn resolve(&self, v: &mut Value, hub: i32) -> Result<(), String> {
        match v {
            Value::Array(a) => {
                for x in a {
                    self.resolve(x, hub)?;
                }
            }
            Value::Object(m) => {
                if let Some(key) = m.get("place").and_then(|k| k.as_str()).map(str::to_string) {
                    let p = self
                        .place(&key)
                        .ok_or(format!("rules: unknown place \"{key}\""))?;
                    let c = self.place_at(p);
                    let up = self.level(p) + 1 - hub;
                    let nums: Vec<i64> = m
                        .get("at")
                        .and_then(|a| a.as_array())
                        .map(|a| a.iter().filter_map(|n| n.as_i64()).collect())
                        .unwrap_or_default();
                    let at = match nums.as_slice() {
                        [] => vec![c[0] as i64, c[1] as i64],
                        [x, z] => vec![c[0] as i64 + x, c[1] as i64 + z],
                        [x, y, z] => vec![c[0] as i64 + x, up as i64 + y, c[1] as i64 + z],
                        _ => return Err(format!("rules: bad \"at\" at place \"{key}\"")),
                    };
                    m.insert("at".into(), Value::from(at));
                    if m.get("radius").and_then(|r| r.as_str()) == Some("place") {
                        m.insert("radius".into(), Value::from(self.place_radius(p).round()));
                    }
                    if let Some(d) = m.remove("below").and_then(|d| d.as_i64()) {
                        m.insert("y".into(), Value::from(up as i64 - d));
                    }
                    m.remove("place");
                }
                for x in m.values_mut() {
                    self.resolve(x, hub)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// The shape of the land of a layout, ready to be asked about many columns.
pub struct Land<'a> {
    l: &'a Layout,
    rivers: Vec<Vec<[f32; 2]>>,
    ridges: Vec<Vec<[f32; 2]>>,
}

impl<'a> Land<'a> {
    pub fn new(l: &'a Layout) -> Self {
        let scaled = |p: &Vec<[f32; 2]>| p.iter().map(|q| l.pos(*q)).collect::<Vec<_>>();
        Land {
            l,
            rivers: l
                .terrain
                .rivers
                .iter()
                .map(|r| spline(&scaled(&r.path), 6.0))
                .collect(),
            ridges: l.terrain.ridges.iter().map(|r| scaled(&r.path)).collect(),
        }
    }

    /// 0 at a river's banks and in a lake, rising to 1 where the land is
    /// its own again; with the bed's height.
    pub fn wet(&self, x: f32, z: f32) -> (f32, f32) {
        let t = &self.l.terrain;
        let s = self.l.spread();
        let sea = t.sea as f32;
        let (mut keep, mut bed) = (1.0f32, sea);
        for (r, path) in t.rivers.iter().zip(&self.rivers) {
            let wob = (self.l.noise(x, z, 37.0, 71) - 0.5) * 6.0;
            let d = path_distance([x, z], path) + wob;
            let half = r.width * 0.5;
            let k = smooth((d - half * 0.6) / (half * 0.4 + 2.0));
            // the bed under the water, then the valley side
            let side = smooth((d - half) / r.valley);
            let here = k * side.max(0.02);
            if here < keep {
                keep = here;
                bed = sea - r.depth * (1.0 - k) + 1.6 * k;
            }
        }
        for l in &t.lakes {
            let c = self.l.pos(l.at);
            let (rx, rz) = (l.radius[0] * s.max(0.6), l.radius[1] * s.max(0.6));
            let wob = (self.l.noise(x, z, 29.0, 72) - 0.5) * 0.28;
            let m = (((x - c[0]) / rx).powi(2) + ((z - c[1]) / rz).powi(2)).sqrt() + wob;
            let k = smooth((m - 0.82) / 0.18);
            let side = smooth((m - 1.0) / 0.5);
            let here = k * side.max(0.02);
            if here < keep {
                keep = here;
                bed = sea - l.depth * (1.0 - smooth(m / 0.9)).max(0.12) * (1.0 - k) + 1.6 * k;
            }
        }
        (keep, bed)
    }

    /// The land's own height at an offset from the origin, before pads and
    /// roads: a pure function of the layout.
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let t = &self.l.terrain;
        let s = self.l.spread();
        let v = self.l.rise();
        let mut h = t.ground;
        for (i, w) in t.rolling.iter().enumerate() {
            h += (self.l.noise(x, z, w.scale, i as u32) - 0.5) * 2.0 * w.amp;
        }
        for (i, hill) in t.hills.iter().enumerate() {
            let c = self.l.pos(hill.at);
            let (rx, rz) = (hill.radius[0] * s.max(0.5), hill.radius[1] * s.max(0.5));
            let m = (((x - c[0]) / rx).powi(2) + ((z - c[1]) / rz).powi(2)).sqrt();
            if m >= 1.0 {
                continue;
            }
            let tops = self.l.noise(x, z, 46.0, 30 + i as u32);
            let shape = 1.0 - hill.rough + hill.rough * 2.0 * tops;
            h += hill.rise * v.max(0.6) * smooth(1.0 - m) * shape;
        }
        let crag = 1.0 - (self.l.noise(x, z, 58.0, 50) * 2.0 - 1.0).abs();
        let crest = 0.55 + 0.45 * self.l.noise(x, z, 150.0, 51) + 0.25 * (crag - 0.5);
        // mountains: where a ridge runs into the rim the higher one stands
        let mut mount = 0.0f32;
        for (r, path) in t.ridges.iter().zip(&self.ridges) {
            let d = path_distance([x, z], path);
            let w = r.width * s.max(0.6);
            if d < w {
                mount = mount.max(r.rise * v * smooth(1.0 - d / w) * crest);
            }
        }
        if let Some(rim) = t.rim {
            let half = [self.l.size[0] as f32 / 2.0, self.l.size[2] as f32 / 2.0];
            let edge = (half[0] - x.abs()).min(half[1] - z.abs());
            let w = rim.width * s.max(0.6);
            if edge < w {
                mount = mount.max(rim.rise * v * smooth(1.0 - edge / w) * crest);
            }
        }
        h += mount;
        // no puddles: only rivers and lakes go below the water
        h = h.max(t.sea as f32 + 2.5);
        let (keep, bed) = self.wet(x, z);
        bed + (h - bed) * keep
    }
}

/// A hash for layout work, by column and salt.
pub(crate) fn rnd(x: i32, z: i32, salt: u32) -> u32 {
    hash(x, z, salt ^ 0x1a70)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = r#"{
      "version": 1, "size": [512, 128, 512], "design": [1024, 128, 1024], "seed": 5,
      "terrain": {"sea": 20, "ground": 30, "rolling": [{"scale": 80, "amp": 3}]},
      "flora": {"species": {}, "mixes": {"open": {"cover": 0, "clump": 50, "spacing": 8, "trees": {}}},
                "default": "open"},
      "ways": {},
      "places": [
        {"key": "town", "kind": "hub", "at": [0, 0], "radius": 30},
        {"key": "mine", "kind": "site", "at": [400, -100], "radius": 200, "wide": true}
      ]
    }"#;

    #[test]
    fn positions_spread_with_the_size_and_rules_follow_places() {
        let l = Layout::from_json(SMALL).unwrap();
        assert_eq!(l.spread(), 0.5);
        let mine = l.place("mine").unwrap();
        assert_eq!(l.place_at(mine), [200, -50]);
        assert_eq!(l.place_radius(mine), 100.0);
        let mut rules: Value = serde_json::from_str(
            r#"{"areas": [{"place": "mine", "radius": "place",
                 "camps": [{"place": "mine", "at": [3, -4], "below": 7}]}],
                "npcs": [{"place": "town", "at": [1, 0, 2]}]}"#,
        )
        .unwrap();
        l.resolve_rules(&mut rules).unwrap();
        assert_eq!(rules["areas"][0]["at"], serde_json::json!([200, -50]));
        assert_eq!(rules["areas"][0]["radius"], serde_json::json!(100.0));
        assert_eq!(
            rules["areas"][0]["camps"][0]["at"],
            serde_json::json!([203, -54])
        );
        let up = l.level(mine) - l.level(&l.places[0]);
        assert_eq!(
            rules["areas"][0]["camps"][0]["y"],
            serde_json::json!(up - 7)
        );
        assert_eq!(rules["npcs"][0]["at"], serde_json::json!([1, 0, 2]));
        assert!(rules["npcs"][0].get("place").is_none());
        let mut bad: Value = serde_json::from_str(r#"{"place": "nowhere"}"#).unwrap();
        assert!(l.resolve_rules(&mut bad).is_err());
    }
}
