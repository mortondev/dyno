//! The sound of a place: beds (air that goes round and round: a meadow by
//! day, the night, the lake), scattered voices (a bird, an owl, a drip)
//! and the things that sound where they stand (a fire, a forge, a river).
//!
//! All of it is the pack's, in one streamed file
//! (`sounds/amb/ambience.json`): which sound, and the rule for where and
//! when, written as the life lane writes its own (`life::Where`: areas and
//! the time of day; here also height and weather). The engine only weighs
//! the rules where the listener stands and keeps the mixer told.

use crate::assets;
use crate::audio::{Audio, Event};
use crate::life::{self, Where};
use dyno_world::content::Content;
use dyno_world::{self as w, BlockId, World};
use glam::Vec3;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// The pack file. It streams: it is asked for when ambience first runs.
pub const AMBIENCE_JSON: &str = "sounds/amb/ambience.json";

/// A sound that goes round for as long, and as loud, as its rule holds.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Bed {
    pub sound: String,
    #[serde(rename = "where")]
    pub place: Where,
    /// metres above the spawn hearth it is heard between (wind on hills)
    pub height: Option<[f32; 2]>,
    /// only in this weather (2 rain), as strong as the weather is
    pub weather: Option<u8>,
    /// only outdoors (it fades under a roof or underground)
    pub outdoors: bool,
    pub gain: f32,
}

impl Default for Bed {
    fn default() -> Self {
        Bed {
            sound: String::new(),
            place: Where::default(),
            height: None,
            weather: None,
            outdoors: false,
            gain: 1.0,
        }
    }
}

/// A voice heard now and then from somewhere round about.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Scatter {
    pub sound: String,
    #[serde(rename = "where")]
    pub place: Where,
    pub outdoors: bool,
    /// seconds between one and the next, least and most
    pub every: [f32; 2],
    /// metres away, least and most
    pub distance: [f32; 2],
    pub gain: f32,
}

impl Default for Scatter {
    fn default() -> Self {
        Scatter {
            sound: String::new(),
            place: Where::default(),
            outdoors: false,
            every: [6.0, 14.0],
            distance: [8.0, 24.0],
            gain: 1.0,
        }
    }
}

/// A block that sounds where it stands: the nearest one is heard.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Emitter {
    pub sound: String,
    pub block: String,
    /// the block over it must be this (water under air is a surface)
    pub above: Option<String>,
    #[serde(rename = "where")]
    pub place: Where,
    /// it sounds now and then (seconds, least and most: a hammer) rather
    /// than going round (a fire)
    pub every: Option<[f32; 2]>,
    pub gain: f32,
}

impl Default for Emitter {
    fn default() -> Self {
        Emitter {
            sound: String::new(),
            block: String::new(),
            above: None,
            place: Where::default(),
            every: None,
            gain: 1.0,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct AmbienceData {
    /// the sounds, as `audio.json` describes its own
    pub events: BTreeMap<String, Event>,
    /// seconds for a bed to come or go as you cross into a place
    pub fade: f32,
    /// metres round the listener that are searched for emitters
    pub search: i32,
    pub beds: Vec<Bed>,
    pub scatter: Vec<Scatter>,
    pub emitters: Vec<Emitter>,
}

impl AmbienceData {
    pub fn parse(text: &str) -> Result<AmbienceData, String> {
        serde_json::from_str(text).map_err(|e| format!("{AMBIENCE_JSON}: {e}"))
    }
}

/// The data in the pack now (asked for if it has not come).
pub fn data() -> Option<Arc<AmbienceData>> {
    type Cache = Option<(Arc<[u8]>, Arc<AmbienceData>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let Some(bytes) = assets::get(AMBIENCE_JSON) else {
        assets::request(AMBIENCE_JSON);
        return None;
    };
    let mut c = CACHE.lock().unwrap();
    if let Some((b, d)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return Some(d.clone());
    }
    let d = Arc::new(
        std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(AmbienceData::parse)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                AmbienceData::default()
            }),
    );
    *c = Some((bytes, d.clone()));
    Some(d)
}

/// Where and when the listener is.
pub struct Env<'a> {
    pub content: &'a Content,
    pub hearth: Option<[i32; 3]>,
    pub pos: Vec3,
    /// time of day, 0 to 1 (0 sunrise, 0.5 sunset)
    pub time: f32,
    /// the weather's kind and strength
    pub weather: (u8, f32),
    /// how much sky reaches the listener: 1 in the open, 0 indoors or
    /// underground
    pub sky: f32,
}

impl Env<'_> {
    /// Whether the listener is in any area called `key`.
    pub fn in_area(&self, key: &str) -> bool {
        let Some(h) = self.hearth else {
            return false;
        };
        self.content.areas.iter().any(|a| {
            let dx = self.pos.x - (h[0] + a.at[0]) as f32;
            let dz = self.pos.z - (h[2] + a.at[1]) as f32;
            a.key == key && dx * dx + dz * dz <= a.radius * a.radius
        })
    }

    /// The keys of the areas the listener is in, smallest first.
    pub fn areas(&self) -> Vec<&str> {
        let Some(h) = self.hearth else {
            return Vec::new();
        };
        let mut inside: Vec<_> = self
            .content
            .areas
            .iter()
            .filter(|a| {
                let dx = self.pos.x - (h[0] + a.at[0]) as f32;
                let dz = self.pos.z - (h[2] + a.at[1]) as f32;
                dx * dx + dz * dz <= a.radius * a.radius
            })
            .collect();
        inside.sort_by(|a, b| a.radius.total_cmp(&b.radius));
        inside.iter().map(|a| a.key.as_str()).collect()
    }

    /// How strongly a rule holds here and now, 0 to 1: its areas and its
    /// hours (the ground, and what is beside it, are for things that
    /// stand somewhere; a listener is asked only where and when).
    pub fn holds(&self, place: &Where) -> f32 {
        if !place.areas.is_empty() && !place.areas.iter().any(|a| self.in_area(a)) {
            return 0.0;
        }
        if place.not_areas.iter().any(|a| self.in_area(a)) {
            return 0.0;
        }
        life::time_gate(place.time, self.time)
    }

    /// How much of an outdoor sound is heard: all of it wherever a fair
    /// share of sky light reaches (under leaves too), none indoors.
    fn open(&self, outdoors: bool) -> f32 {
        if outdoors {
            ((self.sky - 0.2) / 0.3).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// How loud a bed is here and now, before its own gain.
    pub fn bed(&self, b: &Bed) -> f32 {
        let mut level = self.holds(&b.place) * self.open(b.outdoors);
        if let Some([lo, hi]) = b.height {
            let y = self.pos.y - self.hearth.map_or(0.0, |h| h[1] as f32);
            // (over four metres: a hill is climbed, not switched on)
            level *= ((y - lo) / 4.0).min((hi - y) / 4.0).clamp(0.0, 1.0);
        }
        if let Some(kind) = b.weather {
            level *= if self.weather.0 == kind {
                self.weather.1.clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        level
    }
}

/// The blocks round the listener that sound: for each emitter rule, the
/// nearest block that matches. Looked for a slab at a time, so a frame
/// never pays for the whole search.
#[derive(Default)]
pub struct Search {
    /// per rule: the block, and what must be over it
    rules: Vec<(Option<BlockId>, Option<BlockId>)>,
    made_for: Option<Arc<AmbienceData>>,
    /// the pass under way: its centre, the slab it has reached, and the
    /// nearest of each rule so far
    centre: [i32; 3],
    slab: i32,
    so_far: Vec<Option<(f32, Vec3)>>,
    /// the nearest of each rule, from the last pass that finished
    pub found: Vec<Option<Vec3>>,
}

impl Search {
    /// Look at the next slabs of the neighbourhood.
    pub fn step(&mut self, world: &World, d: &Arc<AmbienceData>, ear: Vec3) {
        if self.made_for.as_ref().is_none_or(|m| !Arc::ptr_eq(m, d)) {
            self.rules = d
                .emitters
                .iter()
                .map(|e| {
                    (
                        w::block_by_key(&e.block),
                        e.above.as_deref().and_then(w::block_by_key),
                    )
                })
                .collect();
            self.made_for = Some(d.clone());
            self.found = vec![None; d.emitters.len()];
            self.so_far = vec![None; d.emitters.len()];
            self.slab = i32::MAX;
        }
        let r = d.search.clamp(4, 48);
        if self.slab > r {
            // a new pass, round where the listener is now
            self.centre = [
                ear.x.floor() as i32,
                ear.y.floor() as i32,
                ear.z.floor() as i32,
            ];
            self.slab = -r;
            self.so_far.fill(None);
        }
        const HIGH: i32 = 6;
        let c = self.centre;
        for _ in 0..4 {
            let x = c[0] + self.slab;
            for z in c[2] - r..=c[2] + r {
                for y in c[1] - HIGH..=c[1] + HIGH {
                    let b = world.get(x, y, z);
                    if b == w::AIR {
                        continue;
                    }
                    for (i, (block, above)) in self.rules.iter().enumerate() {
                        if *block != Some(b) || above.is_some_and(|a| world.get(x, y + 1, z) != a) {
                            continue;
                        }
                        let at = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                        let dist = at.distance(ear);
                        if self.so_far[i].is_none_or(|(best, _)| dist < best) {
                            self.so_far[i] = Some((dist, at));
                        }
                    }
                }
            }
            self.slab += 1;
            if self.slab > r {
                for (f, s) in self.found.iter_mut().zip(&self.so_far) {
                    *f = s.map(|(_, at)| at);
                }
                break;
            }
        }
    }
}

// (ids of the mixer's loops: beds, then emitters)
const BED: u32 = 1000;
const EMITTER: u32 = 2000;

/// What the ambience is doing, from frame to frame.
#[derive(Default)]
pub struct Ambience {
    made_for: Option<Arc<AmbienceData>>,
    /// the level of each bed now (it glides to what its rule says)
    beds: Vec<f32>,
    /// seconds until each scattered voice is next heard
    scatter: Vec<f32>,
    /// per emitter: whether its loop is going, and the seconds until it
    /// next sounds (for those that sound now and then)
    emitters: Vec<(bool, f32)>,
    rng: u32,
}

impl Ambience {
    fn random(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn between(&mut self, r: [f32; 2]) -> f32 {
        r[0] + (r[1] - r[0]).max(0.0) * self.random()
    }

    /// Everything stops (leaving the world).
    pub fn hush(&mut self, audio: &mut Audio) {
        for (i, level) in self.beds.iter_mut().enumerate() {
            if *level > 0.0 {
                audio.loop_stop(BED + i as u32);
            }
            *level = 0.0;
        }
        for (i, e) in self.emitters.iter_mut().enumerate() {
            if e.0 {
                audio.loop_stop(EMITTER + i as u32);
            }
            e.0 = false;
        }
    }

    /// Once a frame: weigh the rules here and now, and tell the mixer.
    /// `found` is the nearest block of each emitter rule (`Search::found`).
    pub fn update(&mut self, audio: &mut Audio, env: &Env, found: &[Option<Vec3>], dt: f32) {
        let Some(d) = data() else { return };
        if self.made_for.as_ref().is_none_or(|m| !Arc::ptr_eq(m, &d)) {
            self.hush(audio);
            self.beds = vec![0.0; d.beds.len()];
            self.emitters = vec![(false, 1.0); d.emitters.len()];
            self.scatter = Vec::new();
            for i in 0..d.scatter.len() {
                // (not all at once when you arrive)
                let first = self.random() * d.scatter[i].every[1];
                self.scatter.push(first);
            }
            self.made_for = Some(d.clone());
        }
        let step = (dt / d.fade.max(0.1)).min(1.0);
        for (i, b) in d.beds.iter().enumerate() {
            let want = env.bed(b);
            let level = &mut self.beds[i];
            let was = *level;
            *level += (want - *level).clamp(-step, step);
            if *level > 0.002 {
                // (a file still on its way: the bed waits at nothing)
                if !audio.loop_set(BED + i as u32, &b.sound, *level * b.gain, None) {
                    *level = 0.0;
                }
            } else if was > 0.002 {
                audio.loop_stop(BED + i as u32);
                *level = 0.0;
            }
        }
        for (i, s) in d.scatter.iter().enumerate() {
            let holds = env.holds(&s.place) * env.open(s.outdoors);
            if holds <= 0.0 {
                continue;
            }
            self.scatter[i] -= dt;
            if self.scatter[i] <= 0.0 {
                self.scatter[i] = self.between(s.every);
                let turn = self.random() * std::f32::consts::TAU;
                let far = self.between(s.distance);
                let at = env.pos
                    + Vec3::new(
                        turn.cos() * far,
                        2.0 + 4.0 * self.random(),
                        turn.sin() * far,
                    );
                audio.play_at(&s.sound, at, s.gain * holds);
            }
        }
        for (i, e) in d.emitters.iter().enumerate() {
            let at = found.get(i).copied().flatten();
            let holds = env.holds(&e.place);
            match (at, e.every) {
                (Some(at), None) if holds > 0.0 => {
                    let id = EMITTER + i as u32;
                    self.emitters[i].0 = audio.loop_set(id, &e.sound, e.gain * holds, Some(at));
                }
                (Some(at), Some(every)) if holds > 0.0 => {
                    self.emitters[i].1 -= dt;
                    if self.emitters[i].1 <= 0.0 {
                        audio.play_at(&e.sound, at, e.gain * holds);
                        self.emitters[i].1 = self.between(every);
                    }
                }
                _ => {
                    if self.emitters[i].0 {
                        audio.loop_stop(EMITTER + i as u32);
                        self.emitters[i].0 = false;
                    }
                }
            }
        }
    }

    /// The level of each bed now (for tests and the montage's listing).
    pub fn bed_levels(&self) -> &[f32] {
        &self.beds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_holds_by_hour_height_weather_and_sky() {
        let content = Content::default_pack();
        let env = |time: f32, y: f32, weather: (u8, f32), sky: f32| Env {
            content: &content,
            hearth: Some([0, 0, 0]),
            pos: Vec3::new(0.0, y, 0.0),
            time,
            weather,
            sky,
        };
        let d = AmbienceData::parse(
            r#"{ "beds": [
                { "sound": "day", "where": { "time": [0.0, 0.5] }, "outdoors": true },
                { "sound": "hill", "height": [20, 400] },
                { "sound": "rain", "weather": 2, "gain": 0.5 }
            ] }"#,
        )
        .unwrap();
        let noon = env(0.25, 0.0, (0, 0.0), 1.0);
        assert_eq!(noon.bed(&d.beds[0]), 1.0);
        assert_eq!(noon.bed(&d.beds[1]), 0.0);
        assert_eq!(noon.bed(&d.beds[2]), 0.0);
        // night; half way up the hill's edge; rain at 60%; under a roof
        assert_eq!(env(0.75, 0.0, (0, 0.0), 1.0).bed(&d.beds[0]), 0.0);
        assert_eq!(env(0.25, 22.0, (0, 0.0), 1.0).bed(&d.beds[1]), 0.5);
        assert_eq!(env(0.25, 60.0, (0, 0.0), 1.0).bed(&d.beds[1]), 1.0);
        assert!((env(0.25, 0.0, (2, 0.6), 1.0).bed(&d.beds[2]) - 0.6).abs() < 1e-6);
        assert_eq!(env(0.25, 0.0, (0, 0.0), 0.0).bed(&d.beds[0]), 0.0);
    }

    #[test]
    fn the_nearest_sounding_block_is_found_a_slab_at_a_time() {
        let d = Arc::new(
            AmbienceData::parse(
                r#"{ "search": 8, "emitters": [
                    { "sound": "fire", "block": "hearth" },
                    { "sound": "water", "block": "water", "above": "air" }
                ] }"#,
            )
            .unwrap(),
        );
        let mut world = World::empty([32, 32, 32]);
        let hearth = w::block_by_key("hearth").unwrap();
        world.set(20, 10, 16, hearth);
        world.set(13, 10, 16, hearth);
        // water under water is not a surface; the top of it is
        world.set(16, 8, 20, w::WATER);
        world.set(16, 9, 20, w::WATER);
        let ear = Vec3::new(16.5, 10.5, 16.5);
        let mut s = Search::default();
        s.step(&world, &d, ear);
        assert_eq!(s.found, [None, None], "not in one step");
        for _ in 0..8 {
            s.step(&world, &d, ear);
        }
        assert_eq!(s.found[0], Some(Vec3::new(13.5, 10.5, 16.5)));
        assert_eq!(s.found[1], Some(Vec3::new(16.5, 9.5, 20.5)));
    }
}
