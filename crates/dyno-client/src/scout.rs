//! Choosing where a stand shot's figure stands (`dyno scout`).
//!
//! For each landmark of a list (data: where it is, its footprint and
//! height, how far off to stand), the scout streams the world around it
//! and tries the places a player could walk to from the landmark's standing
//! spot. Each place is judged through the camera `dyno shot --stand` would
//! use there: the play camera must get its full distance back, sit clear of
//! leaves and walls, see the figure, and show the landmark. The best place
//! of each landmark is written out as camera data.

use crate::net::{self, NetEvent};
use crate::shot::{self, Chase};
use dyno_protocol::{ClientMsg, ServerMsg};
use dyno_world::{self as w, BlockId, World};
use glam::Vec3;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct ScoutOptions {
    pub addr: String,
    /// the landmarks to find cameras for (JSON, see [`Spec`])
    pub landmarks: PathBuf,
    /// the camera data to write; cameras of landmarks not scouted are kept
    pub out: PathBuf,
    /// only these landmarks (empty: all)
    pub only: Vec<String>,
}

/// The landmark list. Positions are metres from the world's hearth (x east,
/// y up, z south), as content positions are.
#[derive(Deserialize)]
struct Spec {
    /// block keys of road and trail surfaces: standing on one is preferred
    #[serde(default)]
    paths: Vec<String>,
    defaults: Limits,
    landmarks: Vec<Landmark>,
}

/// How a landmark is to be framed; a landmark's own values replace the
/// defaults.
#[derive(Deserialize, Clone, Default)]
struct Limits {
    /// stand between this many metres from the landmark's centre
    near: Option<f32>,
    far: Option<f32>,
    /// camera distance behind the figure
    zoom: Option<f32>,
    /// degrees down the camera may look, the preferred one first
    pitch: Option<Vec<f32>>,
    /// let walls pull the camera in to no less than this (indoors)
    min_zoom: Option<f32>,
}

#[derive(Deserialize)]
struct Landmark {
    name: String,
    /// centre of the footprint (x, z)
    at: [f32; 2],
    /// half-sizes of the footprint (x, z)
    half: [f32; 2],
    /// height above its base
    height: f32,
    /// height of its base (default: the ground around the footprint)
    base: Option<f32>,
    /// a place a player can stand at the landmark (x, z, or x, y, z);
    /// default: the nearest open ground to the centre
    anchor: Option<Vec<f32>>,
    /// time of day of its shot, 0 to 1
    time: f64,
    #[serde(flatten)]
    limits: Limits,
}

/// The box a landmark fills.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub lo: Vec3,
    pub hi: Vec3,
}

impl Target {
    fn holds(&self, p: Vec3, grow: f32) -> bool {
        p.cmpge(self.lo - grow).all() && p.cmple(self.hi + grow).all()
    }
    fn centre(&self) -> Vec3 {
        (self.lo + self.hi) * 0.5
    }
    /// Horizontal distance from a point to the centre of the footprint.
    fn distance(&self, x: f32, z: f32) -> f32 {
        let c = self.centre();
        (c.x - x).hypot(c.z - z)
    }
}

/// The first cell along a ray that is not air: how far, and what is there.
/// Water stops a ray, as it fills the frame.
pub fn cast(world: &World, origin: Vec3, dir: Vec3, max: f32) -> Option<(f32, BlockId)> {
    let mut cell = origin.floor().as_ivec3();
    let step = dir.signum().as_ivec3();
    let delta = dir.abs().recip();
    let mut tmax = Vec3::ZERO;
    for i in 0..3 {
        let o = origin[i];
        let frac = if dir[i] > 0.0 {
            o.floor() + 1.0 - o
        } else {
            o - o.floor()
        };
        tmax[i] = if delta[i].is_finite() {
            frac * delta[i]
        } else {
            f32::INFINITY
        };
    }
    let mut t = 0.0;
    while t <= max {
        let b = world.get(cell.x, cell.y, cell.z);
        if b != w::AIR {
            return Some((t, b));
        }
        let a = if tmax.x < tmax.y && tmax.x < tmax.z {
            0
        } else if tmax.y < tmax.z {
            1
        } else {
            2
        };
        cell[a] += step[a];
        t = tmax[a];
        tmax[a] += delta[a];
    }
    None
}

/// What a camera's frame holds.
#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    /// share of the frame that is the landmark
    pub landmark: f32,
    /// distance to the nearest thing in the frame
    pub nearest: f32,
    /// share of the frame filled by things in the way that are not the
    /// landmark: nearer than 10 m in the upper frame (a trunk, a wall, a
    /// bank), or nearer than the ground under the camera anywhere
    pub clutter: f32,
    /// share of the frame that is sky
    pub sky: f32,
}

const RAY: f32 = 96.0;

/// Look through the camera with a coarse grid of rays.
pub fn frame(world: &World, c: &Chase, aspect: f32, target: &Target) -> Frame {
    let (nx, ny) = (48, 27);
    let right = c.dir.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(c.dir);
    let ty = (shot::STAND_FOV.to_radians() * 0.5).tan();
    let tx = ty * aspect;
    let mut f = Frame {
        nearest: RAY,
        ..Default::default()
    };
    let mut cluttered = 0;
    for j in 0..ny {
        for i in 0..nx {
            let u = (i as f32 + 0.5) / nx as f32 * 2.0 - 1.0;
            let v = 1.0 - (j as f32 + 0.5) / ny as f32 * 2.0;
            let d = (c.dir + right * (u * tx) + up * (v * ty)).normalize();
            let high = v > -0.3;
            match cast(world, c.eye, d, RAY) {
                Some((t, _)) => {
                    f.nearest = f.nearest.min(t);
                    if target.holds(c.eye + d * (t + 0.01), 0.5) {
                        f.landmark += 1.0;
                    } else if t < if high { 10.0 } else { 5.0 } {
                        cluttered += 1;
                    }
                }
                None => f.sky += 1.0,
            }
        }
    }
    let n = (nx * ny) as f32;
    f.landmark /= n;
    f.sky /= n;
    f.clutter = cluttered as f32 / n;
    f
}

/// The share of the landmark's box a camera can see: rays to a lattice of
/// points in it either reach the point or end on something inside the box.
pub fn visible(world: &World, eye: Vec3, target: &Target) -> f32 {
    let mut seen = 0;
    let n = 4;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let f = Vec3::new(i as f32, j as f32, k as f32) / (n - 1) as f32;
                let p = target.lo + (target.hi - target.lo) * (0.1 + 0.8 * f);
                let v = p - eye;
                let len = v.length();
                let ok = match cast(world, eye, v / len, len) {
                    None => true,
                    Some((t, _)) => target.holds(eye + v / len * (t + 0.01), 0.5),
                };
                seen += ok as u32;
            }
        }
    }
    seen as f32 / (n * n * n) as f32
}

/// The share of the box's corners inside the frame, or None if its centre
/// or the middle of its top is outside.
fn in_frame(c: &Chase, aspect: f32, t: &Target) -> Option<f32> {
    let right = c.dir.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(c.dir);
    let ty = (shot::STAND_FOV.to_radians() * 0.5).tan();
    let tx = ty * aspect;
    let mid = t.centre();
    let mut pts = Vec::new();
    for i in 0..8 {
        let pick = |b: bool, lo: f32, hi: f32| if b { hi } else { lo };
        pts.push(Vec3::new(
            pick(i & 1 > 0, t.lo.x, t.hi.x),
            pick(i & 2 > 0, t.lo.y, t.hi.y),
            pick(i & 4 > 0, t.lo.z, t.hi.z),
        ));
    }
    let shows = |p: Vec3| {
        let v = p - c.eye;
        let z = v.dot(c.dir);
        z > 0.1 && (v.dot(right) / z).abs() < tx * 0.92 && (v.dot(up) / z).abs() < ty * 0.92
    };
    if !shows(mid) || !shows(Vec3::new(mid.x, t.hi.y, mid.z)) {
        return None;
    }
    Some(pts.iter().filter(|p| shows(**p)).count() as f32 / pts.len() as f32)
}

/// Why a camera will not do, or None if it will.
pub fn refuse(
    world: &World,
    feet: Vec3,
    c: &Chase,
    min_zoom: f32,
    clear: f32,
) -> Option<&'static str> {
    if c.reached < min_zoom - 0.05 {
        return Some("the camera is pulled in");
    }
    // nothing at the lens: no leaf, post or water within `clear` metres
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let p = c.eye + Vec3::new(dx as f32, dy as f32, dz as f32) * clear;
                if world.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32) != w::AIR {
                    return Some("something is at the lens");
                }
            }
        }
    }
    // the figure is in sight from head to foot
    for h in [0.15, 0.9, 1.7] {
        let v = feet + Vec3::Y * h - c.eye;
        let len = v.length();
        if cast(world, c.eye, v / len, len - 0.2).is_some() {
            return Some("the figure is hidden");
        }
    }
    None
}

/// Can someone stand with their feet in this cell?
fn stands(world: &World, x: i32, y: i32, z: i32) -> bool {
    shot::standing_y(world, x, z, Some(y as f32)) == Some(y as f32)
}

/// Every cell a player can walk to from `from` without climbing or dropping
/// more than a block at a time, within `reach` metres of the target's
/// centre.
pub fn walkable(world: &World, from: [i32; 3], target: &Target, reach: f32) -> Vec<[i32; 3]> {
    let mut seen: HashMap<[i32; 2], Vec<i32>> = HashMap::new();
    let mut out = Vec::new();
    let mut queue = VecDeque::from([from]);
    seen.entry([from[0], from[2]]).or_default().push(from[1]);
    while let Some([x, y, z]) = queue.pop_front() {
        out.push([x, y, z]);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = (x + dx, z + dz);
            if target.distance(nx as f32 + 0.5, nz as f32 + 0.5) > reach {
                continue;
            }
            for ny in [y, y + 1, y - 1] {
                if !stands(world, nx, ny, nz) {
                    continue;
                }
                let col = seen.entry([nx, nz]).or_default();
                if !col.contains(&ny) {
                    col.push(ny);
                    queue.push_back([nx, ny, nz]);
                }
                break;
            }
        }
    }
    out
}

/// A camera the scout chose.
#[derive(Clone, Debug)]
pub struct Choice {
    pub feet: Vec3,
    pub face: Vec3,
    pub zoom: f32,
    pub pitch: f32,
    pub score: f32,
    pub frame: Frame,
    pub visible: f32,
    pub on_path: bool,
}

/// What the scout needs to know of a landmark, in world coordinates.
pub struct Site {
    pub target: Target,
    pub anchor: [i32; 3],
    pub near: f32,
    pub far: f32,
    pub zoom: f32,
    pub min_zoom: f32,
    pub pitches: Vec<f32>,
}

impl Site {
    /// Indoors (the camera may be pulled in to under half its distance) it
    /// sits nearer the walls.
    fn clear(&self) -> f32 {
        if self.min_zoom < 0.5 * self.zoom {
            0.35
        } else {
            1.0
        }
    }
}

/// How the places tried for a landmark fared.
#[derive(Default, Debug)]
pub struct Tally {
    pub tried: usize,
    pub passed: usize,
    /// why places were turned down, and how many
    pub refused: Vec<(&'static str, usize)>,
}

impl Tally {
    fn no(&mut self, why: &'static str) {
        match self.refused.iter_mut().find(|r| r.0 == why) {
            Some(r) => r.1 += 1,
            None => self.refused.push((why, 1)),
        }
    }
}

impl std::fmt::Display for Tally {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} places tried, {} passed", self.tried, self.passed)?;
        for (why, n) in &self.refused {
            write!(f, "; {n} {why}")?;
        }
        Ok(())
    }
}

/// The best place to stand for a landmark, and how the places tried fared.
pub fn choose(world: &World, s: &Site, paths: &[BlockId]) -> (Option<Choice>, Tally) {
    let aspect = 16.0 / 9.0;
    let face = s.target.centre();
    let cells = walkable(world, s.anchor, &s.target, s.far);
    let mut tally = Tally::default();
    // a first pass with the preferred pitch: cheap tests only
    let mut firsts = Vec::new();
    for [x, y, z] in cells {
        let feet = Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5);
        let d = s.target.distance(feet.x, feet.z);
        if d < s.near || d > s.far {
            continue;
        }
        tally.tried += 1;
        let yaw = shot::yaw_toward([feet.x, feet.z], [face.x, face.z]);
        let c = shot::chase(world, feet, yaw, -s.pitches[0].to_radians(), s.zoom);
        if let Some(why) = refuse(world, feet, &c, s.min_zoom, s.clear()) {
            tally.no(why);
            continue;
        }
        let seen = visible(world, c.eye, &s.target);
        if seen < 0.3 {
            tally.no("the landmark is hidden");
            continue;
        }
        let on_path = paths.contains(&world.get(x, y - 1, z));
        let open = world.surface(x, z) < y;
        let rough = seen + 0.15 * on_path as u8 as f32 + 0.1 * open as u8 as f32;
        firsts.push((rough, feet, yaw, on_path, open));
    }
    tally.passed = firsts.len();
    firsts.sort_by(|a, b| b.0.total_cmp(&a.0));
    // spread the full test over the ground: at most one place per 3 m cell
    let mut taken: Vec<[i32; 2]> = Vec::new();
    let mut best: Option<Choice> = None;
    for (_, feet, yaw, on_path, open) in firsts {
        let cell = [(feet.x / 3.0) as i32, (feet.z / 3.0) as i32];
        if taken.contains(&cell) {
            continue;
        }
        if taken.len() >= 400 {
            break;
        }
        taken.push(cell);
        for (k, pitch) in s.pitches.iter().enumerate() {
            let c = shot::chase(world, feet, yaw, -pitch.to_radians(), s.zoom);
            if refuse(world, feet, &c, s.min_zoom, s.clear()).is_some() {
                continue;
            }
            let Some(framed) = in_frame(&c, aspect, &s.target) else {
                tally.no("the landmark's top is out of the frame");
                continue;
            };
            let f = frame(world, &c, aspect, &s.target);
            if f.nearest < 4.0 * s.clear() {
                tally.no("something fills the foreground");
                continue;
            }
            if f.landmark < 0.01 {
                tally.no("the landmark is too small in the frame");
                continue;
            }
            let seen = visible(world, c.eye, &s.target);
            // on the ground the landmark stands on, not up a tower or a wall
            let above = (feet.y - s.target.lo.y - 4.0).max(0.0);
            let score = 3.0 * seen
                + 2.0 * framed
                + 10.0 * f.landmark.min(0.3)
                + 0.4 * on_path as u8 as f32
                + 0.2 * open as u8 as f32
                - 6.0 * f.clutter
                - 0.1 * k as f32
                - 0.15 * above
                - 0.5 * (s.zoom - c.reached) / s.zoom;
            if best.as_ref().is_none_or(|b| score > b.score) {
                best = Some(Choice {
                    feet,
                    face,
                    zoom: s.zoom,
                    pitch: *pitch,
                    score,
                    frame: f,
                    visible: seen,
                    on_path,
                });
            }
        }
    }
    (best, tally)
}

/// The nearest cell to (x, z) where someone can stand and walk away from
/// (not the top of a pillar): under the open sky, or near `y` when given.
fn stand_near(world: &World, x: i32, z: i32, y: Option<f32>, t: &Target) -> Option<[i32; 3]> {
    for r in 0..12i32 {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dz.abs()) != r {
                    continue;
                }
                if let Some(h) = shot::standing_y(world, x + dx, z + dz, y) {
                    let at = [x + dx, h as i32, z + dz];
                    let d = t.distance(at[0] as f32, at[2] as f32);
                    if walkable(world, at, t, d + 8.0).len() >= 40 {
                        return Some(at);
                    }
                }
            }
        }
    }
    None
}

/// The ground level around a footprint: the median height of its border.
fn base_of(world: &World, lo: [i32; 2], hi: [i32; 2]) -> f32 {
    let mut hs = Vec::new();
    for x in lo[0] - 2..=hi[0] + 2 {
        for z in lo[1] - 2..=hi[1] + 2 {
            if x < lo[0] || x > hi[0] || z < lo[1] || z > hi[1] {
                hs.push(world.ground(x, z) + 1);
            }
        }
    }
    hs.sort();
    hs[hs.len() / 2] as f32
}

/// Stream until every section within `r` metres of `at` is held.
fn stream(j: &mut net::Joined, at: [f32; 3], r: f32) -> Result<(), String> {
    let started = Instant::now();
    loop {
        let missing = net::sections_around(&j.world, at, r)
            .iter()
            .any(|s| !j.world.has_section(*s));
        if !missing {
            return Ok(());
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err(format!("the world around {at:?} did not stream in"));
        }
        match j.conn.rx.recv_timeout(Duration::from_millis(50)) {
            Ok(NetEvent::Msg(
                m @ (ServerMsg::Sections(_)
                | ServerMsg::SectionsGone(_)
                | ServerMsg::Block { .. }
                | ServerMsg::Fill { .. }),
            )) => {
                net::apply_world(&mut j.world, &m);
            }
            Ok(NetEvent::Closed(r)) => return Err(r),
            _ => {}
        }
    }
}

/// Find a camera for every landmark and write the camera data. Returns a
/// report, one line per landmark.
pub fn scout(o: ScoutOptions) -> Result<String, String> {
    let text =
        std::fs::read(&o.landmarks).map_err(|e| format!("{}: {e}", o.landmarks.display()))?;
    let spec: Spec =
        serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", o.landmarks.display()))?;
    let paths = spec
        .paths
        .iter()
        .map(|k| dyno_world::block_by_key(k).ok_or_else(|| format!("no block called {k}")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut j = net::connect_with(
        &o.addr,
        "scout",
        net::JoinOptions {
            observer: true,
            ..Default::default()
        },
        Duration::from_secs(10),
    )?;
    let h = Vec3::from(j.hearth.map(|v| v as f32));
    let view = j.view;
    // cameras already chosen stay, unless their landmark is scouted again
    let mut cameras: Vec<serde_json::Value> = std::fs::read(&o.out)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.get("cameras").and_then(|c| c.as_array().cloned()))
        .unwrap_or_default();
    let mut report = vec![format!("hearth {:?}", j.hearth)];
    let mut failed = Vec::new();
    for l in &spec.landmarks {
        if !o.only.is_empty() && !o.only.contains(&l.name) {
            continue;
        }
        let lim = |f: fn(&Limits) -> Option<f32>, name: &str| {
            f(&l.limits)
                .or_else(|| f(&spec.defaults))
                .ok_or_else(|| format!("{}: no {name}", l.name))
        };
        let (near, far) = (lim(|l| l.near, "near")?, lim(|l| l.far, "far")?);
        let zoom = lim(|l| l.zoom, "zoom")?;
        let min_zoom = l.limits.min_zoom.or(spec.defaults.min_zoom).unwrap_or(zoom);
        let pitches = l
            .limits
            .pitch
            .clone()
            .or_else(|| spec.defaults.pitch.clone())
            .filter(|p| !p.is_empty())
            .ok_or_else(|| format!("{}: no pitch", l.name))?;
        let centre = [h.x + l.at[0], h.y, h.z + l.at[1]];
        let _ = j.conn.tx.send(ClientMsg::ViewAt { pos: centre });
        // rays reach RAY from a camera that may be `far + zoom` out
        let need = (far + zoom + RAY).min(view - 2.0 * w::SECTION as f32);
        stream(&mut j, centre, need)?;
        let world = &j.world;
        let lo = [
            (centre[0] - l.half[0]).floor() as i32,
            (centre[2] - l.half[1]).floor() as i32,
        ];
        let hi = [
            (centre[0] + l.half[0]).ceil() as i32,
            (centre[2] + l.half[1]).ceil() as i32,
        ];
        let base = l
            .base
            .map(|b| b + h.y)
            .unwrap_or_else(|| base_of(world, lo, hi));
        let target = Target {
            lo: Vec3::new(lo[0] as f32, base - 1.0, lo[1] as f32),
            hi: Vec3::new(hi[0] as f32, base + l.height, hi[1] as f32),
        };
        let a = l.anchor.clone().unwrap_or_else(|| l.at.to_vec());
        let (ax, ay, az) = match a[..] {
            [x, z] => (x, None, z),
            [x, y, z] => (x, Some(y + h.y), z),
            _ => return Err(format!("{}: anchor is x, z or x, y, z", l.name)),
        };
        let Some(anchor) = stand_near(
            world,
            (h.x + ax).floor() as i32,
            (h.z + az).floor() as i32,
            ay,
            &target,
        ) else {
            report.push(format!("{}: nowhere to stand at its anchor", l.name));
            failed.push(l.name.clone());
            continue;
        };
        let site = Site {
            target,
            anchor,
            near,
            far,
            zoom,
            min_zoom,
            pitches,
        };
        let (best, tally) = choose(world, &site, &paths);
        let Some(c) = best else {
            report.push(format!("{}: no camera from {anchor:?} ({tally})", l.name));
            failed.push(l.name.clone());
            continue;
        };
        report.push(format!(
            "{}: stand {:.1}, {:.1}, {:.1} ({:.0} m away{}), pitch {}; sees {:.0}% of it, {:.0}% of the frame, clutter {:.0}%, nearest {:.1} m; base {base}; {tally}",
            l.name,
            c.feet.x,
            c.feet.y,
            c.feet.z,
            target.distance(c.feet.x, c.feet.z),
            if c.on_path { ", on a path" } else { "" },
            c.pitch,
            c.visible * 100.0,
            c.frame.landmark * 100.0,
            c.frame.clutter * 100.0,
            c.frame.nearest,
        ));
        let r1 = |v: f32| (v * 10.0).round() / 10.0;
        let rel = |v: Vec3| {
            let v = v - h;
            [r1(v.x), r1(v.y), r1(v.z)]
        };
        let entry = serde_json::json!({
            "name": l.name,
            "stand": rel(c.feet),
            "face": rel(c.face),
            "zoom": c.zoom,
            "pitch": c.pitch,
            "time": l.time,
        });
        match cameras.iter_mut().find(|e| e["name"] == l.name.as_str()) {
            Some(e) => *e = entry,
            None => cameras.push(entry),
        }
    }
    // in the order of the landmark list
    let order = |e: &serde_json::Value| {
        spec.landmarks
            .iter()
            .position(|l| e["name"] == l.name.as_str())
            .unwrap_or(usize::MAX)
    };
    cameras.sort_by_key(order);
    // one camera a line, its fields in reading order
    let lines: Vec<String> = cameras
        .iter()
        .map(|c| {
            let fields: Vec<String> = ["name", "stand", "face", "zoom", "pitch", "time"]
                .iter()
                .map(|k| format!("\"{k}\": {}", c[*k]))
                .collect();
            format!("  {{{}}}", fields.join(", "))
        })
        .collect();
    let doc = format!(
        "{{\n \"note\": \"Gameplay cameras of the visual gauntlet, chosen by `dyno scout` from landmarks.json: do not edit by hand. stand and face are metres from the world's hearth (x east, y up, z south); zoom is metres behind the figure, pitch degrees down, time the time of day 0-1. Render with `dyno shot ADDR --from-hearth --stand x,y,z --face x,y,z --zoom Z --pitch P --time T`.\",\n \"cameras\": [\n{}\n ]\n}}\n",
        lines.join(",\n")
    );
    std::fs::write(&o.out, doc).map_err(|e| format!("{}: {e}", o.out.display()))?;
    report.push(format!("wrote {}", o.out.display()));
    if failed.is_empty() {
        Ok(report.join("\n"))
    } else {
        Err(format!(
            "{}\nno camera for: {}",
            report.join("\n"),
            failed.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat field at y = 4 with a 6 m tower in the middle.
    fn field() -> (World, Target) {
        let mut world = World::empty([96, 48, 96]);
        for x in 0..96 {
            for z in 0..96 {
                for y in 0..=4 {
                    world.set(x, y, z, w::STONE);
                }
            }
        }
        for x in 46..50 {
            for z in 46..50 {
                for y in 5..11 {
                    world.set(x, y, z, w::BRICK);
                }
            }
        }
        let target = Target {
            lo: Vec3::new(46.0, 4.0, 46.0),
            hi: Vec3::new(50.0, 11.0, 50.0),
        };
        (world, target)
    }

    fn site(target: Target) -> Site {
        Site {
            target,
            anchor: [48, 5, 52],
            near: 20.0,
            far: 30.0,
            zoom: 9.0,
            min_zoom: 9.0,
            pitches: vec![15.0],
        }
    }

    #[test]
    fn the_chase_camera_is_the_zoom_behind_and_above() {
        let (world, _) = field();
        let feet = Vec3::new(48.5, 5.0, 75.5);
        let c = shot::chase(&world, feet, 0.0, -15f32.to_radians(), 9.0);
        assert!((c.reached - 9.0).abs() < 0.11, "{c:?}");
        // behind (to the south of) a figure that faces north, above its head
        assert!(c.eye.z > feet.z + 8.0 && c.eye.y > feet.y + 3.5, "{c:?}");
        assert!(refuse(&world, feet, &c, 9.0, 1.0).is_none());
    }

    #[test]
    fn a_wall_behind_pulls_the_camera_in_and_is_refused() {
        let (mut world, _) = field();
        for x in 40..58 {
            for y in 5..20 {
                world.set(x, y, 80, w::STONE);
            }
        }
        let feet = Vec3::new(48.5, 5.0, 75.5);
        let c = shot::chase(&world, feet, 0.0, -15f32.to_radians(), 9.0);
        assert!(c.reached < 5.0, "{c:?}");
        assert_eq!(
            refuse(&world, feet, &c, 9.0, 1.0),
            Some("the camera is pulled in")
        );
    }

    #[test]
    fn leaves_at_the_lens_are_refused() {
        let (mut world, _) = field();
        let feet = Vec3::new(48.5, 5.0, 75.5);
        let c = shot::chase(&world, feet, 0.0, -15f32.to_radians(), 9.0);
        let e = c.eye.floor().as_ivec3();
        world.set(e.x + 1, e.y, e.z, w::LEAVES);
        // leaves do not stop the game's camera
        let c = shot::chase(&world, feet, 0.0, -15f32.to_radians(), 9.0);
        assert!((c.reached - 9.0).abs() < 0.11);
        assert_eq!(
            refuse(&world, feet, &c, 9.0, 1.0),
            Some("something is at the lens")
        );
    }

    #[test]
    fn the_scout_stands_where_the_landmark_shows() {
        let (mut world, target) = field();
        // a wall hides the tower from the south
        for x in 30..66 {
            for y in 5..30 {
                world.set(x, y, 60, w::STONE);
            }
        }
        let mut s = site(target);
        s.anchor = [48, 5, 44];
        let (best, tally) = choose(&world, &s, &[]);
        let c = best.expect("a camera");
        assert!(tally.tried > tally.passed && tally.passed > 0, "{tally}");
        assert!(c.feet.z < 46.0, "stood behind the wall: {c:?}");
        let d = target.distance(c.feet.x, c.feet.z);
        assert!((20.0..=30.0).contains(&d), "{d}");
        assert!(c.visible > 0.6 && c.frame.landmark > 0.02, "{c:?}");
    }

    #[test]
    fn nobody_stands_on_a_roof_or_in_water() {
        let (mut world, target) = field();
        // a flat roof 4 m up, and a pond
        for x in 60..70 {
            for z in 40..50 {
                world.set(x, 9, z, w::PLANKS);
            }
        }
        for x in 20..30 {
            for z in 40..50 {
                world.set(x, 4, z, w::WATER);
            }
        }
        let cells = walkable(&world, [48, 5, 52], &target, 40.0);
        assert!(cells.len() > 1000);
        assert!(cells.iter().all(|c| c[1] == 5), "climbed onto the roof");
        assert!(
            !cells
                .iter()
                .any(|c| (20..30).contains(&c[0]) && (40..50).contains(&c[2]))
        );
        // under the roof is fine to walk, by the height asked for
        assert_eq!(shot::standing_y(&world, 65, 45, Some(5.0)), Some(5.0));
        assert_eq!(shot::standing_y(&world, 65, 45, None), Some(10.0));
        assert_eq!(shot::standing_y(&world, 25, 45, None), None);
    }
}
