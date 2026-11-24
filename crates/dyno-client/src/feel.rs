//! How a blow feels (docs/design/feel-f2.md): the freeze of a hit-stop, the
//! camera's shake, a small creature's stagger, the pulse of a pressed
//! button, the marker of a creature winding up. Every number is the pack's
//! (`feel.json`); a pack without the file gets none of it.
//!
//! The effects made of particles (the burst where a blow lands, the pop of a
//! kill, the stars of a daze) are in `life.json` with the other effects:
//! see `particles.rs`.

use glam::Vec3;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// The pack file.
pub const FEEL_JSON: &str = "feel.json";

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct FeelData {
    pub hit_stop: HitStop,
    pub shake: Shake,
    pub stagger: Stagger,
    pub press: Press,
    pub tell: Tell,
    pub kill: Kill,
}

/// The animation of attacker and target stands still for a moment when a
/// blow lands. Seconds each.
#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct HitStop {
    /// a melee blow of yours
    pub hit: f32,
    /// a critical one
    pub crit: f32,
    /// the one that kills
    pub kill: f32,
    /// a blow you take
    pub taken: f32,
    /// the share of these a target gets from a shot or a spell (the caster
    /// does not freeze)
    pub ranged: f32,
}

/// A kick of the camera: how far it turns at most (degrees) and how long
/// it rings (seconds).
#[derive(Deserialize, Default, Debug, Clone, Copy, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Kick {
    pub degrees: f32,
    pub secs: f32,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Shake {
    /// your own critical blow
    pub crit: Kick,
    /// a heavy blow you take
    pub heavy: Kick,
    /// a blow that takes this share of your health or more is heavy (so is
    /// one a creature wound up for)
    pub heavy_share: f32,
    /// swings a second
    pub rate: f32,
    /// the camera never turns further than this, whatever lands (degrees)
    pub most: f32,
}

/// A critical blow knocks a small creature back a little (drawn only: the
/// server has not moved it).
#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Stagger {
    /// creatures up to this size stagger
    pub scale: f32,
    /// metres, at the furthest
    pub push: f32,
    pub secs: f32,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Press {
    /// how long a pressed button stays lit, seconds
    pub pulse: f32,
    /// a press this near the end of the global cooldown is kept by the
    /// server and used when it ends (the server's own rule: `QUEUE` in
    /// abilities.rs)
    pub queue: f32,
}

/// The marker under a creature winding up a heavy blow: a ring of cubes at
/// the blow's reach that closes as the blow comes.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Tell {
    pub color: [f32; 3],
    /// edge of a cube, metres (a step of the voxel ladder)
    pub cube: f32,
    /// metres from the creature when the creature does not say how far it
    /// reaches
    pub reach: f32,
    /// how much the creature whitens at the end of its wind-up, 0 to 1
    pub flash: f32,
    /// the share of the wind-up's animation (the creature's special move)
    /// that is the wind-up itself: it is stretched over the time the
    /// server gives, and the rest, the blow, follows at its own speed
    pub strike: f32,
}

impl Default for Tell {
    fn default() -> Self {
        Tell {
            color: [1.0, 0.3, 0.2],
            cube: 0.125,
            reach: 2.5,
            flash: 0.0,
            strike: 0.75,
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Kill {
    /// a creature that leaves no body (burnt up by light) still falls: its
    /// figure stays this long, seconds
    pub linger: f32,
    /// the loot's sparkle bursts this long after the fall begins, seconds
    pub sparkle_after: f32,
}

impl FeelData {
    pub fn parse(text: &str) -> Result<FeelData, String> {
        serde_json::from_str(text).map_err(|e| format!("{FEEL_JSON}: {e}"))
    }

    /// How long a blow of yours stops the animation: of the target, and of
    /// you (none for a shot or a spell).
    pub fn stop_for(&self, melee: bool, crit: bool) -> (f32, f32) {
        let h = &self.hit_stop;
        let secs = if crit { h.crit } else { h.hit };
        if melee {
            (secs, secs)
        } else {
            (secs * h.ranged, 0.0)
        }
    }
}

/// The data in the pack now, parsed again only when the file changes.
pub fn data() -> Arc<FeelData> {
    type Cache = Option<(Arc<[u8]>, Arc<FeelData>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    static NONE: Mutex<Option<Arc<FeelData>>> = Mutex::new(None);
    let Some(bytes) = crate::assets::get(FEEL_JSON) else {
        return NONE
            .lock()
            .unwrap()
            .get_or_insert_with(Default::default)
            .clone();
    };
    let mut c = CACHE.lock().unwrap();
    if let Some((b, d)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return d.clone();
    }
    let d = Arc::new(
        std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(FeelData::parse)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                FeelData::default()
            }),
    );
    *c = Some((bytes, d.clone()));
    d
}

/// The camera's shake: kicks that ring down. Only the picture turns; where
/// the character looks, and so where it walks and aims, is untouched.
#[derive(Default)]
pub struct Camera {
    kicks: Vec<(Instant, [f32; 2], Kick)>,
}

impl Camera {
    /// Kick the camera: `dir` is the way the blow goes on screen (x to the
    /// right, y up); the camera is thrown that way first.
    pub fn kick(&mut self, kick: Kick, dir: [f32; 2]) {
        if kick.degrees <= 0.0 || kick.secs <= 0.0 {
            return;
        }
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        let dir = if len > 1e-3 {
            [dir[0] / len, dir[1] / len]
        } else {
            [0.0, 1.0]
        };
        self.kicks.push((Instant::now(), dir, kick));
        if self.kicks.len() > 8 {
            self.kicks.remove(0);
        }
    }

    /// How far the camera is turned now: to the right and up, in radians.
    pub fn offset(&mut self, d: &Shake) -> [f32; 2] {
        self.offset_at(d, Instant::now())
    }

    fn offset_at(&mut self, d: &Shake, now: Instant) -> [f32; 2] {
        self.kicks
            .retain(|(at, _, k)| now.duration_since(*at).as_secs_f32() < k.secs);
        let mut sum = [0.0f32; 2];
        for (at, dir, k) in &self.kicks {
            let t = now.duration_since(*at).as_secs_f32();
            let u = t / k.secs;
            // thrown out at once, then swinging back and forth as it dies
            let a = k.degrees * (1.0 - u) * (1.0 - u) * (std::f32::consts::TAU * d.rate * t).cos();
            sum[0] += dir[0] * a;
            sum[1] += dir[1] * a;
        }
        let len = (sum[0] * sum[0] + sum[1] * sum[1]).sqrt();
        let most = d.most.max(0.0);
        if len > most && len > 0.0 {
            sum = [sum[0] / len * most, sum[1] / len * most];
        }
        [sum[0].to_radians(), sum[1].to_radians()]
    }

    /// Turn a view direction by the shake.
    pub fn turn(&mut self, d: &Shake, dir: Vec3) -> Vec3 {
        let [x, y] = self.offset(d);
        if x == 0.0 && y == 0.0 {
            return dir;
        }
        let right = dir.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(dir).normalize_or_zero();
        (dir + right * x.tan() + up * y.tan()).normalize_or_zero()
    }
}

/// How far a staggered creature stands from its place `t` seconds after
/// the blow: out fast, back slower.
pub fn stagger_at(d: &Stagger, t: f32) -> f32 {
    if d.secs <= 0.0 || t < 0.0 || t >= d.secs {
        return 0.0;
    }
    let u = t / d.secs;
    let out = 0.25;
    d.push
        * if u < out {
            u / out
        } else {
            1.0 - (u - out) / (1.0 - out)
        }
}

/// The cells of a ring of cubes: the cubes of edge `cube` whose middles lie
/// within half a cube of a circle of `radius`, as (x, z) offsets from the
/// middle, on the grid of the cube. A stepped circle, as the voxel rule has
/// round things.
pub fn ring_cells(radius: f32, cube: f32) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    if cube <= 0.0 || radius <= 0.0 {
        return out;
    }
    let n = (radius / cube).ceil() as i32 + 1;
    for iz in -n..=n {
        for ix in -n..=n {
            let (x, z) = (ix as f32 * cube, iz as f32 * cube);
            if ((x * x + z * z).sqrt() - radius).abs() <= cube * 0.5 {
                out.push([x, z]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn pack() -> FeelData {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/feel.json");
        FeelData::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// The pack's numbers are inside what the design asks for
    /// (docs/design/feel-f2.md).
    #[test]
    fn the_pack_keeps_to_the_design() {
        let d = pack();
        let h = &d.hit_stop;
        assert!((0.04..=0.08).contains(&h.hit), "a hit stops 40 to 80 ms");
        assert!(h.crit > h.hit && h.kill > h.crit, "longer on crits and kills");
        assert!(h.kill <= 0.2, "a kill's stop is still a moment");
        assert!(d.shake.most > 0.0 && d.shake.most <= 2.0, "the shake is capped");
        assert!(d.shake.crit.secs <= 0.3 && d.shake.heavy.secs <= 0.4, "short");
        assert!(d.shake.crit.degrees <= d.shake.most);
        assert!(d.press.pulse > 0.0 && d.press.queue > 0.0);
        // the voxel rule: a marker's cubes are a step of the ladder
        let ladder = [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125];
        assert!(ladder.contains(&d.tell.cube), "the tell's cubes: {}", d.tell.cube);
    }

    #[test]
    fn a_pack_without_the_file_feels_as_before() {
        let d = FeelData::default();
        assert_eq!(d.stop_for(true, true), (0.0, 0.0));
        let mut cam = Camera::default();
        cam.kick(d.shake.crit, [1.0, 0.0]);
        assert_eq!(cam.offset(&d.shake), [0.0, 0.0]);
        assert_eq!(stagger_at(&d.stagger, 0.1), 0.0);
    }

    #[test]
    fn only_melee_stops_the_attacker() {
        let d = pack();
        let (target, me) = d.stop_for(true, false);
        assert_eq!((target, me), (d.hit_stop.hit, d.hit_stop.hit));
        let (target, me) = d.stop_for(false, true);
        assert_eq!(me, 0.0);
        assert!(target < d.hit_stop.crit);
    }

    #[test]
    fn the_shake_is_thrown_the_way_of_the_blow_capped_and_soon_over() {
        let d = pack().shake;
        let mut cam = Camera::default();
        let t0 = Instant::now();
        // many blows at once never turn the camera past the cap
        for _ in 0..8 {
            cam.kicks.push((t0, [1.0, 0.0], d.heavy));
        }
        let [x, y] = cam.offset_at(&d, t0);
        assert!(x > 0.0 && y == 0.0, "to the right first");
        assert!(x.to_degrees() <= d.most + 1e-4, "{} degrees", x.to_degrees());
        // and it is over when its time is
        let after = t0 + Duration::from_secs_f32(d.heavy.secs + 0.01);
        assert_eq!(cam.offset_at(&d, after), [0.0, 0.0]);
        assert!(cam.kicks.is_empty());
    }

    #[test]
    fn a_stagger_goes_out_and_comes_back() {
        let d = Stagger {
            scale: 1.0,
            push: 0.3,
            secs: 0.4,
        };
        assert_eq!(stagger_at(&d, 0.0), 0.0);
        assert!((stagger_at(&d, 0.1) - 0.3).abs() < 1e-4);
        assert!(stagger_at(&d, 0.3) < 0.3);
        assert_eq!(stagger_at(&d, 0.4), 0.0);
    }

    #[test]
    fn a_ring_is_cubes_on_one_grid_round_a_circle() {
        let cells = ring_cells(2.0, 0.125);
        assert!(cells.len() > 60, "{} cubes", cells.len());
        for [x, z] in &cells {
            // on the grid of the cube
            assert_eq!((x / 0.125).fract(), 0.0);
            assert_eq!((z / 0.125).fract(), 0.0);
            let r = (x * x + z * z).sqrt();
            assert!((r - 2.0).abs() <= 0.0626, "{r}");
        }
    }
}
