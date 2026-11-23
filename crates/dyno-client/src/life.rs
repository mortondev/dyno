//! Ambient life: particle effects (smoke, embers, fireflies, motes, falling
//! leaves) and small cosmetic critters (birds, butterflies, ducks, deer).
//!
//! Everything here is decoration the client makes for itself from the pack
//! file `life.json`: the server neither knows nor pays for it. The file
//! says what the effects look like, which blocks give them off, where they
//! are scattered (by area, ground and time of day), what critters there are
//! and where they live. The engine only knows how to play an effect and a
//! handful of behaviours.
//!
//! A quality level (settings, or `DYNO_LIFE=0|1|2`) scales how far away
//! and how many: 0 draws nothing.

use crate::assets;
use crate::critters::{self, Critters};
use crate::particles::{self, Particles};
use crate::render::Vertex;
use dyno_world::content::Content;
use dyno_world::light::Light;
use dyno_world::{self as w, BlockId, World};
use glam::Vec3;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// The pack file.
pub const LIFE_JSON: &str = "life.json";

#[derive(Deserialize, Default, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct LifeData {
    /// metres a second the air moves (x, z): smoke leans this way
    pub wind: [f32; 2],
    pub effects: BTreeMap<String, particles::Effect>,
    /// blocks that give off an effect
    pub emitters: Vec<particles::BlockRule>,
    /// effects scattered over the land
    pub ambient: Vec<particles::Scatter>,
    pub critters: BTreeMap<String, critters::Kind>,
    /// where critters live
    pub wildlife: Vec<critters::Spawn>,
}

impl LifeData {
    pub fn parse(text: &str) -> Result<LifeData, String> {
        serde_json::from_str(text).map_err(|e| format!("{LIFE_JSON}: {e}"))
    }
}

/// The data in the pack now, parsed again only when the file changes.
pub fn data() -> Option<Arc<LifeData>> {
    type Cache = Option<(Arc<[u8]>, Arc<LifeData>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let bytes = assets::get(LIFE_JSON)?;
    let mut c = CACHE.lock().unwrap();
    if let Some((b, d)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return Some(d.clone());
    }
    let d = Arc::new(
        std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(LifeData::parse)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                LifeData::default()
            }),
    );
    *c = Some((bytes, d.clone()));
    Some(d)
}

/// How much life to draw: 0 none, 1 a little and near, 2 all of it.
pub fn quality(setting: u8) -> u8 {
    std::env::var("DYNO_LIFE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(setting)
        .min(2)
}

/// Where something may be: in which areas, on what ground, at what time.
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Where {
    /// area keys from the rules (empty: anywhere)
    pub areas: Vec<String>,
    pub not_areas: Vec<String>,
    /// the block on top of the ground there (empty: any)
    pub ground: Vec<String>,
    /// one of these blocks within `beside_r` columns, at ground level
    pub beside: Vec<String>,
    pub beside_r: i32,
    /// none of these blocks within `beside_r` columns
    pub away: Vec<String>,
    /// open to the sky (no leaves or roof overhead)
    pub open: bool,
    /// under leaves or a roof
    pub covered: bool,
    /// time of day window (0 midnight... as the server counts it); may wrap
    pub time: Option<[f32; 2]>,
}

/// How strongly a time window holds at `t`: 1 inside, 0 outside, a short
/// ramp at each end so things fade in and out.
pub fn time_gate(window: Option<[f32; 2]>, t: f32) -> f32 {
    let Some([a, b]) = window else {
        return 1.0;
    };
    const RAMP: f32 = 0.012;
    let len = (b - a).rem_euclid(1.0);
    let x = (t - a).rem_euclid(1.0);
    if x > len {
        return 0.0;
    }
    (x / RAMP).min((len - x) / RAMP).clamp(0.0, 1.0)
}

/// What the frame knows about the world, handed to effects and critters.
pub struct Scene<'a> {
    pub world: &'a World,
    pub light: &'a Light,
    pub content: &'a Content,
    pub hearth: Option<[i32; 3]>,
    /// time of day, 0 to 1
    pub time: f32,
    pub eye: Vec3,
    pub dir: Vec3,
    /// people the critters shy from (you and the players near you)
    pub people: &'a [Vec3],
}

impl Scene<'_> {
    /// The key of the smallest area holding (x, z).
    pub fn area(&self, x: f32, z: f32) -> Option<&str> {
        let h = self.hearth?;
        self.content
            .areas
            .iter()
            .filter(|a| {
                let (dx, dz) = (
                    x - (h[0] + a.at[0]) as f32,
                    z - (h[2] + a.at[1]) as f32,
                );
                dx * dx + dz * dz <= a.radius * a.radius
            })
            .min_by(|a, b| a.radius.total_cmp(&b.radius))
            .map(|a| a.key.as_str())
    }

    /// Whether (x, z) lies in any area called `key` (areas overlap: a
    /// village sits inside its meadow).
    pub fn in_area(&self, key: &str, x: f32, z: f32) -> bool {
        let Some(h) = self.hearth else {
            return false;
        };
        self.content.areas.iter().any(|a| {
            let (dx, dz) = (
                x - (h[0] + a.at[0]) as f32,
                z - (h[2] + a.at[1]) as f32,
            );
            a.key == key && dx * dx + dz * dz <= a.radius * a.radius
        })
    }

    /// Block light and sky light at a point, 0 to 1.
    pub fn light_at(&self, p: Vec3) -> (f32, f32) {
        let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        (
            self.light.get(x, y, z) as f32 / 15.0,
            self.light.sky(x, y, z) as f32 / 15.0,
        )
    }
}

/// The top of the land at a column: the highest block that is not air, a
/// tree or a plant, with whether anything hangs over it. Water counts.
/// None where the world has not arrived.
pub fn column(world: &World, x: i32, z: i32) -> Option<(i32, BlockId, bool)> {
    let top = world.section_dims()[1] * w::SECTION - 1;
    let mut covered = false;
    let mut known = false;
    for y in (0..=top).rev() {
        if y % w::SECTION == w::SECTION - 1 || !known {
            known = world.known(x, y, z);
            if !known {
                // a missing section below loaded ones: nothing to stand on
                if covered {
                    return None;
                }
                continue;
            }
        }
        let b = world.get(x, y, z);
        if b == w::AIR {
            continue;
        }
        if w::is_leaves(b) || b == w::LOG || b == w::BIRCH_LOG {
            covered = true;
            continue;
        }
        if !w::is_solid(b) && b != w::WATER {
            // torches, lanterns, plants
            continue;
        }
        return Some((y, b, covered));
    }
    None
}

/// Block keys as ids, dropping names the block table does not have.
pub fn block_ids(keys: &[String]) -> Vec<BlockId> {
    keys.iter().filter_map(|k| w::block_by_key(k)).collect()
}

/// A [`Where`] with its block names looked up.
#[derive(Debug, Clone, Default)]
pub struct Place {
    areas: Vec<String>,
    not_areas: Vec<String>,
    ground: Vec<BlockId>,
    beside: Vec<BlockId>,
    away: Vec<BlockId>,
    beside_r: i32,
    open: bool,
    covered: bool,
    pub time: Option<[f32; 2]>,
}

impl Place {
    pub fn new(w: &Where) -> Place {
        Place {
            areas: w.areas.clone(),
            not_areas: w.not_areas.clone(),
            ground: block_ids(&w.ground),
            beside: block_ids(&w.beside),
            away: block_ids(&w.away),
            beside_r: w.beside_r.clamp(1, 6),
            open: w.open,
            covered: w.covered,
            time: w.time,
        }
    }

    /// Whether the areas allow (x, z).
    pub fn area_ok(&self, s: &Scene, x: f32, z: f32) -> bool {
        (self.areas.is_empty() || self.areas.iter().any(|k| s.in_area(k, x, z)))
            && !self.not_areas.iter().any(|k| s.in_area(k, x, z))
    }

    /// The ground at (x, z) if this may be there: the height of its top
    /// (the y of the air above it) and the block. None also where the world
    /// has not arrived yet.
    pub fn ground_at(&self, s: &Scene, x: i32, z: i32) -> Option<(i32, BlockId)> {
        let (y, b, covered) = column(s.world, x, z)?;
        if !self.ground.is_empty() && !self.ground.contains(&b) {
            return None;
        }
        if (self.open && covered) || (self.covered && !covered) {
            return None;
        }
        if !self.beside.is_empty() || !self.away.is_empty() {
            let r = self.beside_r;
            let mut found = false;
            for dz in -r..=r {
                for dx in -r..=r {
                    for dy in -1..=1 {
                        let n = s.world.get(x + dx, y + dy, z + dz);
                        if self.away.contains(&n) {
                            return None;
                        }
                        found |= self.beside.contains(&n);
                    }
                }
            }
            if !self.beside.is_empty() && !found {
                return None;
            }
        }
        Some((y + 1, b))
    }
}

impl Place {
    /// The ground one step on from height `y` (the air over the ground a
    /// walker stands on): the same kind of ground within a block up or
    /// down. Cheaper than `ground_at`, which looks at the whole column and
    /// its surroundings.
    pub fn step_at(&self, s: &Scene, x: i32, y: i32, z: i32) -> Option<(i32, BlockId)> {
        for top in [y - 1, y, y - 2] {
            let b = s.world.get(x, top, z);
            if b == w::AIR || (!w::is_solid(b) && b != w::WATER) {
                continue;
            }
            let over = s.world.get(x, top + 1, z);
            if w::is_solid(over) || over == w::WATER {
                continue;
            }
            if !self.ground.is_empty() && !self.ground.contains(&b) {
                return None;
            }
            return Some((top + 1, b));
        }
        None
    }
}

/// A small, fast hash for placing things: the same inputs always give the
/// same life in the same place.
pub fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_add(0x7F4A_7C15);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

/// A hash of a column of the world.
pub fn hash2(seed: u32, x: i32, z: i32) -> u32 {
    hash(hash(seed, x as u32), z as u32)
}

/// A stream of numbers in 0..1 from a seed.
pub struct Rng(pub u32);

impl Rng {
    pub fn next(&mut self) -> f32 {
        self.0 = hash(self.0, 0x51ED_270B);
        (self.0 >> 8) as f32 / 16_777_216.0
    }
    /// between -1 and 1
    pub fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
    pub fn range(&mut self, r: [f32; 2]) -> f32 {
        r[0] + (r[1] - r[0]) * self.next()
    }
}

/// What was drawn last frame, for the frame-time report.
#[derive(Default, Clone, Copy, Debug)]
pub struct Counts {
    pub emitters: usize,
    pub particles: usize,
    pub critters: usize,
    /// milliseconds spent on the effects and on the critters
    pub particles_ms: f32,
    pub critters_ms: f32,
}

/// All the ambient life round one camera.
#[derive(Default)]
pub struct Life {
    particles: Particles,
    critters: Critters,
    /// the data the state was made for
    made_for: Option<Arc<LifeData>>,
    hearth: Option<[i32; 3]>,
    pub counts: Counts,
}

/// Where the frame's vertices go.
pub struct Out<'a> {
    /// lit, shadow-casting geometry (critters)
    pub boxes: &'a mut Vec<Vertex>,
    /// lit geometry that casts no shadow (smoke, leaves, sparks)
    pub fx: &'a mut Vec<Vertex>,
    /// halos, added over the picture
    pub glow: &'a mut Vec<Vertex>,
}

impl Life {
    /// Advance to `clock` (seconds; any steady clock) and draw. `dt` is the
    /// time since the last call; `settle` runs the critters for a while
    /// first (a still picture taken straight after joining).
    pub fn frame(&mut self, s: &Scene, clock: f64, dt: f32, quality: u8, out: &mut Out) {
        self.counts = Counts::default();
        if quality == 0 {
            return;
        }
        let Some(data) = data() else {
            return;
        };
        if self
            .made_for
            .as_ref()
            .is_none_or(|d| !Arc::ptr_eq(d, &data))
            || self.hearth != s.hearth
        {
            // new data (a reload) or another world
            self.particles = Particles::new(&data);
            self.critters = Critters::new(&data);
            self.made_for = Some(data.clone());
            self.hearth = s.hearth;
        }
        let t0 = std::time::Instant::now();
        self.particles
            .frame(s, &data, clock, quality, out, &mut self.counts);
        let t1 = std::time::Instant::now();
        self.critters
            .frame(s, &data, clock, dt, quality, out, &mut self.counts);
        self.counts.particles_ms = (t1 - t0).as_secs_f32() * 1e3;
        self.counts.critters_ms = t1.elapsed().as_secs_f32() * 1e3;
    }

    /// Look at the whole neighbourhood now instead of a little each frame,
    /// and let the critters settle (for a still picture).
    pub fn settle(&mut self, s: &Scene, clock: f64, quality: u8) {
        if quality == 0 {
            return;
        }
        let mut sink = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..40 {
            let mut out = Out {
                boxes: &mut sink.0,
                fx: &mut sink.1,
                glow: &mut sink.2,
            };
            self.particles.scan_all = true;
            self.critters.settle_now = true;
            self.frame(s, clock - (40 - i) as f64 * 0.1, 0.1, quality, &mut out);
            sink.0.clear();
            sink.1.clear();
            sink.2.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_windows_wrap_and_ramp() {
        assert_eq!(time_gate(None, 0.3), 1.0);
        assert_eq!(time_gate(Some([0.1, 0.4]), 0.25), 1.0);
        assert_eq!(time_gate(Some([0.1, 0.4]), 0.5), 0.0);
        // through midnight
        assert_eq!(time_gate(Some([0.8, 0.2]), 0.0), 1.0);
        assert_eq!(time_gate(Some([0.8, 0.2]), 0.5), 0.0);
        let edge = time_gate(Some([0.1, 0.4]), 0.103);
        assert!(edge > 0.0 && edge < 1.0, "{edge}");
    }

    /// The pack's life file parses, and names only things that exist.
    #[test]
    fn the_pack_file_is_sound() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/life.json"
        ))
        .unwrap();
        let d = LifeData::parse(&text).unwrap();
        assert!(!d.effects.is_empty() && !d.critters.is_empty());
        let block = |k: &String| assert!(w::block_by_key(k).is_some(), "no block {k}");
        let place = |p: &Where| {
            p.ground.iter().for_each(block);
            p.beside.iter().for_each(block);
            p.away.iter().for_each(block);
        };
        for r in &d.emitters {
            assert!(d.effects.contains_key(&r.effect), "no effect {}", r.effect);
            block(&r.block);
            r.above.iter().for_each(block);
            r.below.iter().for_each(block);
            r.near.iter().for_each(block);
            place(&r.place);
        }
        for r in &d.ambient {
            assert!(d.effects.contains_key(&r.effect), "no effect {}", r.effect);
            place(&r.place);
        }
        for r in &d.wildlife {
            assert!(d.critters.contains_key(&r.critter), "no critter {}", r.critter);
            place(&r.place);
        }
        for (name, k) in &d.critters {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/");
            let bytes = std::fs::read(format!("{path}{}", k.model))
                .unwrap_or_else(|_| panic!("{name}: no model {}", k.model));
            let file = crate::vox::read(&bytes).unwrap();
            assert!(file.model("idle").is_some(), "{name}: no idle frame");
        }
        for e in d.effects.values() {
            assert!(e.rate > 0.0 && e.life[1] >= e.life[0] && e.life[0] > 0.0);
            assert!(!e.colors.is_empty());
        }
    }
}
