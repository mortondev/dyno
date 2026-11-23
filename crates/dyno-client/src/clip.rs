//! Animation clips from the pack (`anims/<skeleton>.json`), evaluated into
//! [`Pose`]s on a skeleton's named bones.
//!
//! A clip is keyframed channels: per bone a rotation (`rx ry rz`, degrees,
//! applied in the bone's Euler order) and an offset (`ox oy oz`, voxels),
//! plus the shape keys `flow`, `shape` and `spread`. Keys are linear between
//! `[time, value]` pairs and held at the ends. A one-shot has a `duration`
//! (`hold` keeps its last frame); a loop has a `loop` length. Locomotion
//! clips also have `stride` channels keyed by the fraction of a stride cycle,
//! added to the timed ones, so the feet follow the ground covered.

use crate::anim::{Anim, MAX_BONES, Pose, key};
use glam::{EulerRot, Quat, Vec3};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// One channel: a constant, or `[time, value]` keys.
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    Const(f32),
    Keys(Vec<(f32, f32)>),
}

impl Channel {
    pub fn at(&self, t: f32) -> f32 {
        match self {
            Channel::Const(v) => *v,
            Channel::Keys(k) => {
                let Some(first) = k.first() else {
                    return 0.0;
                };
                if t <= first.0 {
                    return first.1;
                }
                // the first key after t
                let i = k.partition_point(|(kt, _)| *kt <= t);
                if i >= k.len() {
                    return k[k.len() - 1].1;
                }
                let ((t0, v0), (t1, v1)) = (k[i - 1], k[i]);
                if t1 > t0 {
                    v0 + (v1 - v0) * (t - t0) / (t1 - t0)
                } else {
                    v1
                }
            }
        }
    }

    fn parse(v: &Value) -> Result<Channel, String> {
        if let Some(n) = v.as_f64() {
            return Ok(Channel::Const(n as f32));
        }
        let arr = v.as_array().ok_or("a channel is a number or [[t, v], ...]")?;
        let mut keys = Vec::with_capacity(arr.len());
        for k in arr {
            let p = k.as_array().filter(|p| p.len() == 2).ok_or("a key is [t, v]")?;
            let f = |i: usize| p[i].as_f64().map(|x| x as f32).ok_or("a key holds numbers");
            keys.push((f(0)?, f(1)?));
        }
        Ok(Channel::Keys(keys))
    }
}

/// What a channel drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// bone, axis 0..3 rotation (radians once parsed)
    Rot(usize, usize),
    /// bone, axis 0..3 offset
    Off(usize, usize),
    /// shape key
    Key(usize),
}

/// One clip.
#[derive(Clone, Debug, Default)]
pub struct Clip {
    /// seconds, for one-shots
    pub duration: Option<f32>,
    /// seconds, for loops
    pub looping: Option<f32>,
    /// holds its last frame when done (a death)
    pub hold: bool,
    pub time: Vec<(Target, Channel)>,
    pub stride: Vec<(Target, Channel)>,
}

/// The clips one kind of figure plays, on one skeleton.
#[derive(Debug)]
pub struct ClipSet {
    pub clips: HashMap<Anim, Rc<Clip>>,
    /// Euler order per bone
    pub order: [EulerRot; MAX_BONES],
    /// bones an action takes while the legs move
    pub upper: u32,
    /// the per-figure seed the clips were made at: loops of a figure with
    /// another seed start further in
    pub seed: f32,
}

impl ClipSet {
    pub fn clip(&self, a: Anim) -> Option<&Clip> {
        self.clips.get(&a).map(|c| &**c)
    }

    /// How long a one-shot lasts in this set (`None` for loops).
    pub fn duration(&self, a: Anim) -> Option<f32> {
        self.clip(a).and_then(|c| c.duration)
    }

    /// The pose of `a` at `t` seconds in, at stride phase `stride`
    /// (radians) for a figure with seed `seed`.
    pub fn pose(&self, a: Anim, t: f32, stride: f32, seed: f32) -> Pose {
        let Some(c) = self.clip(a) else {
            return Pose::REST;
        };
        let t = match (c.duration, c.looping) {
            (Some(d), _) => t.clamp(0.0, d),
            (None, Some(l)) if l > 0.0 => (t + (seed - self.seed).rem_euclid(1.0) * l).rem_euclid(l),
            _ => t,
        };
        let u = (stride / std::f32::consts::TAU).rem_euclid(1.0);
        let mut euler = [[0.0f32; 3]; MAX_BONES];
        let mut p = Pose::REST;
        // timed channels give the value; stride channels add to it
        let mut apply = |list: &[(Target, Channel)], at: f32, add: bool| {
            for (target, ch) in list {
                let v = ch.at(at);
                match *target {
                    Target::Rot(b, a) => euler[b][a] += v,
                    Target::Off(b, a) => p.off[b][a] += v,
                    Target::Key(k) if add => p.k[k] += v,
                    Target::Key(k) => p.k[k] = v,
                }
            }
        };
        apply(&c.time, t, false);
        apply(&c.stride, u, true);
        for (b, e) in euler.iter().enumerate() {
            if e != &[0.0; 3] {
                p.rot[b] = rotation(self.order[b], *e);
            }
        }
        p
    }
}

/// A rotation from Euler angles (x, y, z) in an order: `YXZ` is y then x
/// then z from the outside in.
pub fn rotation(order: EulerRot, e: [f32; 3]) -> Quat {
    let (rx, ry, rz) = (
        Quat::from_rotation_x(e[0]),
        Quat::from_rotation_y(e[1]),
        Quat::from_rotation_z(e[2]),
    );
    match order {
        EulerRot::ZYX => rz * ry * rx,
        EulerRot::XYZ => rx * ry * rz,
        EulerRot::XZY => rx * rz * ry,
        EulerRot::YZX => ry * rz * rx,
        EulerRot::ZXY => rz * rx * ry,
        _ => ry * rx * rz,
    }
}

/// An Euler order by name ("yxz", "zyx"...).
pub fn order(name: &str) -> EulerRot {
    match name {
        "zyx" => EulerRot::ZYX,
        "xyz" => EulerRot::XYZ,
        "xzy" => EulerRot::XZY,
        "yzx" => EulerRot::YZX,
        "zxy" => EulerRot::ZXY,
        _ => EulerRot::YXZ,
    }
}

/// The skeleton file's head: bone names, orders, the upper body.
#[derive(Deserialize)]
pub struct SkeletonSpec {
    pub bones: Vec<String>,
    #[serde(default)]
    pub order: HashMap<String, String>,
    #[serde(default)]
    pub upper: Vec<String>,
}

impl SkeletonSpec {
    pub fn bone(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b == name)
    }

    pub fn orders(&self) -> [EulerRot; MAX_BONES] {
        let mut o = [EulerRot::YXZ; MAX_BONES];
        for (i, b) in self.bones.iter().enumerate().take(MAX_BONES) {
            if let Some(n) = self.order.get(b) {
                o[i] = order(n);
            }
        }
        o
    }

    pub fn upper_mask(&self) -> u32 {
        self.upper
            .iter()
            .filter_map(|b| self.bone(b))
            .fold(0, |m, i| m | 1 << i)
    }
}

/// Parse the channels of one clip section ("time" or "stride").
fn channels(sk: &SkeletonSpec, v: Option<&Value>) -> Result<Vec<(Target, Channel)>, String> {
    let mut out = Vec::new();
    let Some(map) = v.and_then(|v| v.as_object()) else {
        return Ok(out);
    };
    for (name, ch) in map {
        let target = match name.as_str() {
            "flow" => Target::Key(key::FLOW),
            "shape" => Target::Key(key::SHAPE),
            "spread" => Target::Key(key::SPREAD),
            _ => {
                let (bone, axis) = name
                    .rsplit_once('.')
                    .ok_or_else(|| format!("channel {name}: bone.channel"))?;
                let b = sk.bone(bone).ok_or_else(|| format!("channel {name}: no bone {bone}"))?;
                let a = |c: u8| (c - b'x') as usize;
                match axis.as_bytes() {
                    [b'r', c @ b'x'..=b'z'] => Target::Rot(b, a(*c)),
                    [b'o', c @ b'x'..=b'z'] => Target::Off(b, a(*c)),
                    _ => return Err(format!("channel {name}: rx ry rz ox oy oz")),
                }
            }
        };
        let mut ch = Channel::parse(ch).map_err(|e| format!("channel {name}: {e}"))?;
        if let Target::Rot(..) = target {
            let r = |v: f32| v.to_radians();
            ch = match ch {
                Channel::Const(v) => Channel::Const(r(v)),
                Channel::Keys(k) => Channel::Keys(k.into_iter().map(|(t, v)| (t, r(v))).collect()),
            };
        }
        out.push((target, ch));
    }
    Ok(out)
}

/// Parse a clip.
pub fn parse_clip(sk: &SkeletonSpec, v: &Value) -> Result<Clip, String> {
    let num = |k: &str| v.get(k).and_then(|x| x.as_f64()).map(|x| x as f32);
    Ok(Clip {
        duration: num("duration"),
        looping: num("loop"),
        hold: v.get("hold").and_then(|h| h.as_bool()).unwrap_or(false),
        time: channels(sk, v.get("time"))?,
        stride: channels(sk, v.get("stride"))?,
    })
}

/// Every clip set in a clips file, by name: each named set falls back to the
/// set `default` for clips it does not give.
pub fn parse_sets(sk: &SkeletonSpec, v: &Value) -> Result<HashMap<String, Rc<ClipSet>>, String> {
    let seed = v.get("seed").and_then(|s| s.as_f64()).unwrap_or(0.0) as f32;
    let sets = v
        .get("sets")
        .and_then(|s| s.as_object())
        .ok_or("clips: no sets")?;
    let mut parsed: HashMap<String, HashMap<Anim, Rc<Clip>>> = HashMap::new();
    for (name, set) in sets {
        let mut clips = HashMap::new();
        for (anim, clip) in set.as_object().ok_or("clips: a set is an object")? {
            let Some(a) = Anim::by_name(anim) else {
                return Err(format!("clips: no animation called {anim}"));
            };
            clips.insert(
                a,
                Rc::new(parse_clip(sk, clip).map_err(|e| format!("{name} {anim}: {e}"))?),
            );
        }
        parsed.insert(name.clone(), clips);
    }
    let default = parsed.get("default").cloned().unwrap_or_default();
    Ok(parsed
        .into_iter()
        .map(|(name, mut clips)| {
            for (a, c) in &default {
                clips.entry(*a).or_insert_with(|| c.clone());
            }
            let set = ClipSet {
                clips,
                order: sk.orders(),
                upper: sk.upper_mask(),
                seed,
            };
            (name, Rc::new(set))
        })
        .collect())
}

impl Pose {
    /// Offsets as a vector, for a bone.
    pub fn offset(&self, b: usize) -> Vec3 {
        self.off.get(b).copied().unwrap_or(Vec3::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_interpolate_and_hold() {
        let c = Channel::Keys(vec![(0.0, 0.0), (1.0, 2.0), (3.0, 2.0)]);
        assert_eq!(c.at(-1.0), 0.0);
        assert_eq!(c.at(0.5), 1.0);
        assert_eq!(c.at(2.0), 2.0);
        assert_eq!(c.at(9.0), 2.0);
        assert_eq!(Channel::Const(4.0).at(3.0), 4.0);
    }

    #[test]
    fn clips_parse_and_pose() {
        let sk: SkeletonSpec = serde_json::from_value(serde_json::json!({
            "bones": ["body", "head"], "order": {"head": "zyx"}, "upper": ["head"]
        }))
        .unwrap();
        let file = serde_json::json!({
            "seed": 0.0,
            "sets": {
                "default": {
                    "Idle": {"loop": 2.0, "time": {"head.rx": [[0, 0], [1, 90], [2, 0]], "flow": 0.5}},
                    "Walk": {"loop": 1.0, "time": {}, "stride": {"body.oy": [[0, 0], [0.5, 2], [1, 0]]}}
                },
                "big": {"Idle": {"loop": 1.0, "time": {"body.ox": 3}}}
            }
        });
        let sets = parse_sets(&sk, &file).unwrap();
        let d = &sets["default"];
        assert_eq!(d.upper, 2);
        let p = d.pose(Anim::Idle, 1.0, 0.0, 0.0);
        assert!((p.rot[1].angle_between(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))).abs() < 1e-5);
        assert_eq!(p.k[key::FLOW], 0.5);
        let w = d.pose(Anim::Walk, 0.3, std::f32::consts::PI, 0.0);
        assert!((w.off[0].y - 2.0).abs() < 1e-5);
        // a set falls back to the default for what it lacks
        assert!(sets["big"].clip(Anim::Walk).is_some());
        assert_eq!(sets["big"].pose(Anim::Idle, 0.0, 0.0, 0.0).off[0].x, 3.0);
    }
}
