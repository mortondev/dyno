//! The compact form of animation clips (docs/design/animation.md §7).
//!
//! Artists edit `anims/<skeleton>.json`; the pack build
//! (`dyno_server::pack`) compiles each into `anims/<skeleton>.clips`, which
//! is what the server streams. The client decodes it back into the same
//! JSON shape and reads it as it reads the source.
//!
//! The encoding is the JSON tree in binary, with a string table, except for
//! channels: a `[[t, v], ...]` list under a clip's `time` or `stride` is
//! - thinned: keys that linear interpolation reproduces within a tolerance
//!   are dropped (Ramer–Douglas–Peucker on the curve);
//! - quantised: times on a grid of ticks (1/1920 s, or 1/4096 of a stride), values in
//!   steps of their kind (0.01° for rotations, 0.002 voxels for offsets,
//!   0.0005 for shape keys), delta coded as variable-length integers;
//! - shared: a channel equal to one already written is a reference.
//!
//! What it changes is bounded: a decoded channel is within [`tolerance`]
//! plus half a step of the source at every key, its time on the grid.

use serde_json::{Map, Number, Value};
use std::collections::HashMap;

const MAGIC: &[u8; 4] = b"DYCL";
const VERSION: u8 = 1;

const T_NULL: u8 = 0;
const T_FALSE: u8 = 1;
const T_TRUE: u8 = 2;
const T_FLOAT: u8 = 3;
const T_INT: u8 = 4;
const T_STR: u8 = 5;
const T_ARRAY: u8 = 6;
const T_OBJECT: u8 = 7;
const T_KEYS: u8 = 8;
const T_SHARED: u8 = 9;

/// Ticks per unit of time: seconds for `time` channels, stride cycles for
/// `stride` ones.
pub const TICKS_TIME: f64 = 1920.0;
pub const TICKS_STRIDE: f64 = 4096.0;

/// A key time on the tick grid of its section.
pub fn snap(t: f64, ticks: f64) -> f64 {
    (t * ticks).round() / ticks
}

/// What a channel drives, by its name: (quantisation step, tolerance).
fn kind(name: &str) -> (f64, f64) {
    match name.rsplit_once('.').map(|(_, a)| a) {
        Some(a) if a.starts_with('r') => (0.01, 0.1),
        Some(a) if a.starts_with('o') => (0.002, 0.02),
        _ => (0.0005, 0.002),
    }
}

/// The largest distance between the source and the decoded form of a
/// channel named `name`, for tests.
pub fn tolerance(name: &str) -> f64 {
    let (q, tol) = kind(name);
    tol + q * 0.5 + 1e-6
}

// ------------------------------------------------------------- encoding

fn varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

fn unzigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

/// The keys of a channel, if `v` is one: `[[t, v], ...]`.
fn as_keys(v: &Value) -> Option<Vec<(f64, f64)>> {
    let a = v.as_array()?;
    if a.is_empty() {
        return None;
    }
    a.iter()
        .map(|k| {
            let p = k.as_array().filter(|p| p.len() == 2)?;
            Some((p[0].as_f64()?, p[1].as_f64()?))
        })
        .collect()
}

/// Keys that keep the curve within `tol` (the ends always stay).
fn thin(keys: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
    if keys.len() <= 2 {
        return keys.to_vec();
    }
    let mut keep = vec![false; keys.len()];
    keep[0] = true;
    keep[keys.len() - 1] = true;
    let mut stack = vec![(0usize, keys.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let ((t0, v0), (t1, v1)) = (keys[a], keys[b]);
        let mut worst = (0.0f64, 0usize);
        for (i, &(t, v)) in keys.iter().enumerate().take(b).skip(a + 1) {
            let at = if t1 > t0 {
                v0 + (v1 - v0) * (t - t0) / (t1 - t0)
            } else {
                // a jump: anything between is off the line
                f64::INFINITY
            };
            let d = (v - at).abs();
            if d > worst.0 {
                worst = (d, i);
            }
        }
        if worst.0 > tol {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    keys.iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(k, _)| *k)
        .collect()
}

struct Writer {
    strings: Vec<String>,
    index: HashMap<String, u64>,
    body: Vec<u8>,
    /// channels written, by their encoding: the index of each
    shared: HashMap<Vec<u8>, u64>,
    channels: u64,
}

impl Writer {
    fn string(&mut self, s: &str) -> u64 {
        if let Some(i) = self.index.get(s) {
            return *i;
        }
        let i = self.strings.len() as u64;
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), i);
        i
    }

    fn channel(&mut self, name: &str, keys: &[(f64, f64)], ticks: f64) {
        let (q, tol) = kind(name);
        // on the tick grid first, so thinning measures what plays
        let keys: Vec<(f64, f64)> = keys.iter().map(|&(t, v)| (snap(t, ticks), v)).collect();
        let keys = thin(&keys, tol);
        let mut enc = Vec::new();
        varint(&mut enc, keys.len() as u64);
        varint(&mut enc, ticks as u64);
        // the step, in millionths
        varint(&mut enc, (q * 1e6).round() as u64);
        let (mut pt, mut pv) = (0i64, 0i64);
        for (t, v) in &keys {
            let (t, v) = ((t * ticks).round() as i64, (v / q).round() as i64);
            varint(&mut enc, zigzag(t - pt));
            varint(&mut enc, zigzag(v - pv));
            (pt, pv) = (t, v);
        }
        match self.shared.get(&enc) {
            Some(i) => {
                self.body.push(T_SHARED);
                varint(&mut self.body, *i);
            }
            None => {
                self.shared.insert(enc.clone(), self.channels);
                self.channels += 1;
                self.body.push(T_KEYS);
                self.body.extend(enc);
            }
        }
    }

    /// `section`: the clip section this value is a channel list of
    /// ("time" or "stride").
    fn value(&mut self, v: &Value, section: Option<f64>, name: &str) {
        if let Some(ticks) = section
            && let Some(keys) = as_keys(v)
        {
            self.channel(name, &keys, ticks);
            return;
        }
        match v {
            Value::Null => self.body.push(T_NULL),
            Value::Bool(false) => self.body.push(T_FALSE),
            Value::Bool(true) => self.body.push(T_TRUE),
            Value::Number(n) => match n.as_i64() {
                Some(i) => {
                    self.body.push(T_INT);
                    varint(&mut self.body, zigzag(i));
                }
                None => {
                    self.body.push(T_FLOAT);
                    let f = n.as_f64().unwrap_or(0.0) as f32;
                    self.body.extend(f.to_le_bytes());
                }
            },
            Value::String(s) => {
                let i = self.string(s);
                self.body.push(T_STR);
                varint(&mut self.body, i);
            }
            Value::Array(a) => {
                self.body.push(T_ARRAY);
                varint(&mut self.body, a.len() as u64);
                for x in a {
                    self.value(x, None, "");
                }
            }
            Value::Object(o) => {
                self.body.push(T_OBJECT);
                varint(&mut self.body, o.len() as u64);
                for (k, x) in o {
                    let i = self.string(k);
                    varint(&mut self.body, i);
                    let sec = match (section, k.as_str()) {
                        (None, "time") => Some(TICKS_TIME),
                        (None, "stride") => Some(TICKS_STRIDE),
                        _ => None,
                    };
                    // channel names are the keys of a section
                    self.value(x, sec.or(section), k);
                }
            }
        }
    }
}

/// Compile a clips file (JSON) into its compact form.
pub fn encode(json: &[u8]) -> Result<Vec<u8>, String> {
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let mut w = Writer {
        strings: Vec::new(),
        index: HashMap::new(),
        body: Vec::new(),
        shared: HashMap::new(),
        channels: 0,
    };
    w.value(&v, None, "");
    let mut out = MAGIC.to_vec();
    out.push(VERSION);
    varint(&mut out, w.strings.len() as u64);
    for s in &w.strings {
        varint(&mut out, s.len() as u64);
        out.extend(s.as_bytes());
    }
    out.extend(w.body);
    Ok(out)
}

// ------------------------------------------------------------- decoding

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
    strings: Vec<String>,
    channels: Vec<Value>,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.b.get(self.at).ok_or("clips: cut short")?;
        self.at += 1;
        Ok(b)
    }

    fn varint(&mut self) -> Result<u64, String> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.byte()?;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("clips: bad number".into())
    }

    fn len(&mut self) -> Result<usize, String> {
        let n = self.varint()? as usize;
        // nothing takes less than a byte
        if n > self.b.len() {
            return Err("clips: bad length".into());
        }
        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        let i = self.varint()? as usize;
        self.strings
            .get(i)
            .cloned()
            .ok_or_else(|| "clips: bad string".into())
    }

    fn num(f: f64) -> Value {
        // the decimals the source could have had, no float noise
        let r = (f * 1e5).round() / 1e5;
        Number::from_f64(r)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }

    fn value(&mut self) -> Result<Value, String> {
        Ok(match self.byte()? {
            T_NULL => Value::Null,
            T_FALSE => Value::Bool(false),
            T_TRUE => Value::Bool(true),
            T_INT => Value::from(unzigzag(self.varint()?)),
            T_FLOAT => {
                let s = self.b.get(self.at..self.at + 4).ok_or("clips: cut short")?;
                self.at += 4;
                Self::num(f32::from_le_bytes(s.try_into().unwrap()) as f64)
            }
            T_STR => Value::String(self.string()?),
            T_ARRAY => {
                let n = self.len()?;
                let mut a = Vec::with_capacity(n);
                for _ in 0..n {
                    a.push(self.value()?);
                }
                Value::Array(a)
            }
            T_OBJECT => {
                let n = self.len()?;
                let mut o = Map::new();
                for _ in 0..n {
                    let k = self.string()?;
                    o.insert(k, self.value()?);
                }
                Value::Object(o)
            }
            T_KEYS => {
                let n = self.len()?;
                let ticks = self.varint()?.max(1) as f64;
                let q = self.varint()? as f64 / 1e6;
                let (mut t, mut v) = (0i64, 0i64);
                let mut keys = Vec::with_capacity(n);
                for _ in 0..n {
                    t += unzigzag(self.varint()?);
                    v += unzigzag(self.varint()?);
                    keys.push(Value::Array(vec![
                        Self::num(t as f64 / ticks),
                        Self::num(v as f64 * q),
                    ]));
                }
                let c = Value::Array(keys);
                self.channels.push(c.clone());
                c
            }
            T_SHARED => {
                let i = self.varint()? as usize;
                self.channels.get(i).cloned().ok_or("clips: bad channel")?
            }
            t => return Err(format!("clips: unknown tag {t}")),
        })
    }
}

/// A compact clips file back in its JSON shape.
pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() < 5 || &bytes[..4] != MAGIC {
        return Err("clips: not a clips file".into());
    }
    if bytes[4] != VERSION {
        return Err(format!(
            "clips: version {} (this reads {VERSION})",
            bytes[4]
        ));
    }
    let mut r = Reader {
        b: bytes,
        at: 5,
        strings: Vec::new(),
        channels: Vec::new(),
    };
    let n = r.len()?;
    for _ in 0..n {
        let len = r.len()?;
        let s = r.b.get(r.at..r.at + len).ok_or("clips: cut short")?;
        r.at += len;
        r.strings
            .push(String::from_utf8(s.to_vec()).map_err(|_| "clips: bad text")?);
    }
    r.value()
}

/// The value of `[[t, v], ...]` keys at `t`, linear between keys and held
/// at the ends (as the client plays them).
pub fn sample(keys: &[(f64, f64)], t: f64) -> f64 {
    let Some(first) = keys.first() else {
        return 0.0;
    };
    if t <= first.0 {
        return first.1;
    }
    let i = keys.partition_point(|k| k.0 <= t);
    if i >= keys.len() {
        return keys[keys.len() - 1].1;
    }
    let ((t0, v0), (t1, v1)) = (keys[i - 1], keys[i]);
    if t1 > t0 {
        v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    } else {
        v1
    }
}

/// One channel of a clips file: (set, clip, section, channel, keys).
pub type Named = (String, String, String, String, Vec<(f64, f64)>);

/// Every channel of a clips file.
pub fn channels(v: &Value) -> Vec<Named> {
    let mut out = Vec::new();
    let Some(sets) = v.get("sets").and_then(|s| s.as_object()) else {
        return out;
    };
    for (set, clips) in sets {
        for (clip, c) in clips.as_object().into_iter().flatten() {
            for sec in ["time", "stride"] {
                for (name, ch) in c.get(sec).and_then(|s| s.as_object()).into_iter().flatten() {
                    let keys = as_keys(ch)
                        .or_else(|| ch.as_f64().map(|v| vec![(0.0, v)]))
                        .unwrap_or_default();
                    out.push((set.clone(), clip.clone(), sec.into(), name.clone(), keys));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_within_tolerance() {
        let src = serde_json::json!({
            "seed": 0.3,
            "skeleton": "t",
            "sets": { "default": {
                "Idle": { "loop": 2.5, "time": {
                    "head.rx": [[0, 0], [0.5, 1.0], [1.0, 2.0], [2.0, 0]],
                    "body.oy": [[0, 0.1234], [1.0, 0.1234]],
                    "flow": 0.5
                }},
                "Walk": { "loop": 1, "hold": false,
                    "stride": { "leg.rx": [[0, 0], [0.5, 30], [1, 0]] },
                    "time": { "head.rx": [[0, 0], [0.5, 1.0], [1.0, 2.0], [1.5, 2.04], [2.5, 0]] } }
            }}
        });
        let bytes = encode(src.to_string().as_bytes()).unwrap();
        let back = decode(&bytes).unwrap();
        assert_eq!(back["seed"], serde_json::json!(0.3));
        assert_eq!(
            back["sets"]["default"]["Idle"]["time"]["flow"],
            serde_json::json!(0.5)
        );
        assert_eq!(
            back["sets"]["default"]["Walk"]["hold"],
            serde_json::json!(false)
        );
        let a = channels(&src);
        let b = channels(&back);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(&x.3, &y.3);
            for &(t, v) in &x.4 {
                let tick = if x.2 == "time" {
                    TICKS_TIME
                } else {
                    TICKS_STRIDE
                };
                let d = (sample(&y.4, snap(t, tick)) - v).abs();
                assert!(d <= tolerance(&x.3), "{} at {t}: {d}", x.3);
            }
        }
        // the key on the line is gone; the bend stays
        let rx = b
            .iter()
            .find(|c| c.1 == "Idle" && c.3 == "head.rx")
            .unwrap();
        assert_eq!(rx.4.len(), 3);
        assert!(decode(&bytes[..bytes.len() - 3]).is_err());
        assert!(decode(b"nope").is_err());
    }

    /// Every clips file of the workspace's pack compiles, and plays back
    /// within tolerance of its source at every source key.
    #[test]
    fn the_packs_clips_stay_within_tolerance() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/anims");
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        files.sort();
        assert!(!files.is_empty());
        for p in files {
            let src = std::fs::read(&p).unwrap();
            let bytes = encode(&src).unwrap();
            let back = decode(&bytes).unwrap();
            let v: Value = serde_json::from_slice(&src).unwrap();
            let (a, b) = (channels(&v), channels(&back));
            assert_eq!(a.len(), b.len());
            let (mut keys, mut kept) = (0, 0);
            for (x, y) in a.iter().zip(&b) {
                assert_eq!((&x.0, &x.1, &x.3), (&y.0, &y.1, &y.3));
                keys += x.4.len();
                kept += y.4.len();
                for &(t, v) in &x.4 {
                    let tick = if x.2 == "time" {
                        TICKS_TIME
                    } else {
                        TICKS_STRIDE
                    };
                    let d = (sample(&y.4, snap(t, tick)) - v).abs();
                    assert!(
                        d <= tolerance(&x.3),
                        "{}: {} {} {} at {t}: {d}",
                        p.display(),
                        x.0,
                        x.1,
                        x.3
                    );
                }
            }
            let z = miniz_oxide::deflate::compress_to_vec(&bytes, 9).len();
            eprintln!(
                "{}: {} -> {} bytes ({} deflated), {keys} -> {kept} keys",
                p.display(),
                src.len(),
                bytes.len(),
                z
            );
        }
    }
}
