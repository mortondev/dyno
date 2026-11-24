//! Voxel particle effects: smoke, embers, sparks, fireflies, motes, falling
//! leaves, dust. An effect is data (`life.json`): how many small cubes a
//! second, how they move, grow and change colour, whether they glow.
//!
//! Emitters are placed by data too: blocks that match a rule (a brazier, the
//! top of a chimney, a crystal) and points scattered over the land by area,
//! ground and time of day (fireflies over night meadows, wisps in the
//! barrow wood).
//!
//! A particle is a pure function of its emitter, its number and the clock:
//! nothing is stored or stepped, so any moment can be drawn (a still
//! picture shows a full plume) and the cost is only what is in view.

use crate::life::{self, Counts, LifeData, Out, Place, Rng, Scene, Where, time_gate};
use crate::render::{self, Vertex};
use dyno_world::{self as w, BlockId};
use glam::Vec3;
use serde::Deserialize;
use std::collections::HashMap;

/// One effect: a stream of small cubes.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Effect {
    /// cubes a second from one emitter
    pub rate: f32,
    /// seconds each lives: shortest and longest
    pub life: [f32; 2],
    /// half the size of the box they start in, metres
    pub spawn: [f32; 3],
    /// starting velocity, metres a second, and how much it varies
    pub velocity: [f32; 3],
    pub spread: [f32; 3],
    /// constant push: gravity (negative y) or lift
    pub accel: [f32; 3],
    /// how quickly the starting velocity dies away (per second; 0: never)
    pub drag: f32,
    /// how much of the wind it takes
    pub wind: f32,
    /// a meander: metres to each side, and turns a second
    pub wander: [f32; 2],
    /// edge of the cube, metres, at birth and at death
    pub size: [f32; 2],
    /// colours through its life (sRGB)
    pub colors: Vec<[f32; 3]>,
    /// how much each cube's brightness differs from the next
    pub shade: f32,
    /// radius of the halo, metres (0: lit by the world like anything else;
    /// more: the cube shines and wears a halo)
    pub glow: f32,
    /// strength of the halo
    pub glow_alpha: f32,
    /// twinkles a second (fireflies)
    pub blink: f32,
    /// part of its life spent growing in, and shrinking out
    pub fade: [f32; 2],
    /// time of day it runs (see `Where::time`)
    pub time: Option<[f32; 2]>,
    /// how far away it is still drawn, metres
    pub reach: f32,
    /// cubes born together each time (a splash, a shower of sparks);
    /// `rate` then counts the bursts
    pub burst: u8,
}

impl Default for Effect {
    fn default() -> Self {
        Effect {
            rate: 1.0,
            life: [1.0, 1.0],
            spawn: [0.0; 3],
            velocity: [0.0; 3],
            spread: [0.0; 3],
            accel: [0.0; 3],
            drag: 0.0,
            wind: 0.0,
            wander: [0.0, 0.0],
            size: [0.1, 0.1],
            colors: vec![[1.0; 3]],
            shade: 0.0,
            glow: 0.0,
            glow_alpha: 0.6,
            blink: 0.0,
            fade: [0.1, 0.3],
            time: None,
            reach: 64.0,
            burst: 1,
        }
    }
}

/// A block that gives off an effect.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct BlockRule {
    pub effect: String,
    pub block: String,
    /// the block over it must be this (for example "air")
    pub above: Option<String>,
    /// the block under it must be this
    pub below: Option<String>,
    /// it stands alone: none of its own kind beside it, this many blocks down
    pub alone: u8,
    /// one of these within a block sideways and `near_down` below (a
    /// chimney is a lone stone column next to a roof)
    pub near: Vec<String>,
    pub near_down: i32,
    /// where in the block the effect starts
    pub offset: [f32; 3],
    /// the share of matching blocks that get one
    pub chance: f32,
    /// where it may be: areas, ground, time of day
    #[serde(rename = "where")]
    pub place: Where,
}

impl Default for BlockRule {
    fn default() -> Self {
        BlockRule {
            effect: String::new(),
            block: String::new(),
            above: None,
            below: None,
            alone: 0,
            near: Vec::new(),
            near_down: 3,
            offset: [0.5, 1.0, 0.5],
            chance: 1.0,
            place: Where::default(),
        }
    }
}

/// An effect scattered over the land: the land is cut into squares and a
/// share of them hold an emitter.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Scatter {
    pub effect: String,
    /// side of a square, metres
    pub cell: f32,
    /// the share of squares that hold one
    pub chance: f32,
    /// metres above the ground: lowest and highest
    pub height: [f32; 2],
    /// where it may be: areas, ground, time of day
    #[serde(rename = "where")]
    pub place: Where,
}

impl Default for Scatter {
    fn default() -> Self {
        Scatter {
            effect: String::new(),
            cell: 12.0,
            chance: 0.5,
            height: [0.5, 1.5],
            place: Where::default(),
        }
    }
}

#[derive(Clone, Copy)]
struct Emitter {
    pos: Vec3,
    effect: u16,
    seed: u32,
}

struct Rule {
    effect: u16,
    above: Option<BlockId>,
    below: Option<BlockId>,
    alone: u8,
    near: Vec<BlockId>,
    near_down: i32,
    offset: Vec3,
    chance: f32,
    place: Place,
    plain_place: bool,
}

struct Scat {
    effect: u16,
    cell: f32,
    chance: f32,
    height: [f32; 2],
    place: Place,
}

/// The emitters round the camera.
#[derive(Default)]
pub struct Particles {
    effects: Vec<Effect>,
    /// the rules a block id may match
    by_block: HashMap<BlockId, Vec<Rule>>,
    scatter: Vec<Scat>,
    /// block emitters by section
    sections: HashMap<[i32; 3], Vec<Emitter>>,
    /// which section of the ring is looked at next
    cursor: usize,
    /// scattered emitters by (rule, square); None where nothing lives
    cells: HashMap<(u16, i32, i32), Option<Emitter>>,
    /// look at every section this frame
    pub scan_all: bool,
    order: Vec<(f32, Emitter)>,
    /// when the scattered squares are next looked at (the clock), and
    /// where the camera was last time
    scatter_at: f64,
    scatter_eye: Vec3,
    /// sections of the ring looked at so far (the first pass goes faster)
    scanned: usize,
    /// the world's count of arrivals when the pass began
    world_rev: u64,
}

/// Sections looked at per frame, and during the first pass after joining.
const SCAN_PER_FRAME: usize = 1;
const SCAN_AT_FIRST: usize = 8;

impl Particles {
    pub fn new(d: &LifeData) -> Particles {
        let names: Vec<&String> = d.effects.keys().collect();
        let index = |name: &str| names.iter().position(|n| *n == name).map(|i| i as u16);
        let mut by_block: HashMap<BlockId, Vec<Rule>> = HashMap::new();
        for r in &d.emitters {
            let (Some(effect), Some(block)) = (index(&r.effect), w::block_by_key(&r.block)) else {
                continue;
            };
            let p = &r.place;
            by_block.entry(block).or_default().push(Rule {
                effect,
                above: r.above.as_deref().and_then(w::block_by_key),
                below: r.below.as_deref().and_then(w::block_by_key),
                alone: r.alone,
                near: life::block_ids(&r.near),
                near_down: r.near_down,
                offset: r.offset.into(),
                chance: r.chance,
                place: Place::new(p),
                plain_place: p.areas.is_empty() && p.not_areas.is_empty(),
            });
        }
        let scatter = d
            .ambient
            .iter()
            .filter_map(|r| {
                Some(Scat {
                    effect: index(&r.effect)?,
                    cell: r.cell.max(2.0),
                    chance: r.chance,
                    height: r.height,
                    place: Place::new(&r.place),
                })
            })
            .collect();
        Particles {
            effects: d.effects.values().cloned().collect(),
            by_block,
            scatter,
            ..Default::default()
        }
    }

    /// How many block emitters have been found round the camera so far.
    pub fn found(&self) -> usize {
        self.sections.values().map(Vec::len).sum()
    }

    /// The emitters of one section of the world.
    fn scan(&self, s: &Scene, sec: [i32; 3]) -> Vec<Emitter> {
        let mut out = Vec::new();
        let o = sec.map(|v| v * w::SECTION);
        for y in 0..w::SECTION {
            for z in 0..w::SECTION {
                for x in 0..w::SECTION {
                    let p = [o[0] + x, o[1] + y, o[2] + z];
                    let b = s.world.get(p[0], p[1], p[2]);
                    if b == w::AIR {
                        continue;
                    }
                    let Some(rules) = self.by_block.get(&b) else {
                        continue;
                    };
                    for (ri, r) in rules.iter().enumerate() {
                        if let Some(e) = self.matches(s, p, b, r, ri as u32) {
                            out.push(e);
                        }
                    }
                }
            }
        }
        out
    }

    fn matches(&self, s: &Scene, p: [i32; 3], b: BlockId, r: &Rule, ri: u32) -> Option<Emitter> {
        let get = |dx: i32, dy: i32, dz: i32| s.world.get(p[0] + dx, p[1] + dy, p[2] + dz);
        if r.above.is_some_and(|a| get(0, 1, 0) != a) || r.below.is_some_and(|a| get(0, -1, 0) != a)
        {
            return None;
        }
        let seed = life::hash(life::hash2(ri ^ 0xA11CE, p[0], p[2]), p[1] as u32);
        if r.chance < 1.0 && (seed >> 8) as f32 / 16_777_216.0 >= r.chance {
            return None;
        }
        const SIDES: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for d in 0..r.alone as i32 {
            if SIDES.iter().any(|(dx, dz)| get(*dx, -d, *dz) == b) {
                return None;
            }
        }
        if !r.near.is_empty() {
            let found = (0..=r.near_down)
                .any(|d| (-1..=1).any(|dx| (-1..=1).any(|dz| r.near.contains(&get(dx, -d, dz)))));
            if !found {
                return None;
            }
        }
        let pos = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) + r.offset;
        if !r.plain_place && !r.place.area_ok(s, pos.x, pos.z) {
            return None;
        }
        Some(Emitter {
            pos,
            effect: r.effect,
            seed,
        })
    }

    /// Keep the emitters round the camera up to date: a few sections of
    /// blocks a frame, and the squares of scattered effects.
    fn refresh(&mut self, s: &Scene, radius: f32, clock: f64) {
        let dims = s.world.section_dims();
        let sec = |v: f32| (v.floor() as i32).div_euclid(w::SECTION);
        let r = (radius / w::SECTION as f32).ceil() as i32;
        let (cx, cz) = (sec(s.eye.x), sec(s.eye.z));
        let span = 2 * r + 1;
        let ring = (span * span * dims[1]) as usize;
        // forget what is out of range
        self.sections
            .retain(|k, _| (k[0] - cx).abs() <= r && (k[2] - cz).abs() <= r);
        if !self.by_block.is_empty() && ring > 0 {
            // sections arrived or left: go round again quickly
            let rev = s.world.revision();
            if rev != self.world_rev {
                self.world_rev = rev;
                self.scanned = 0;
            }
            let n = if self.scan_all {
                ring
            } else if self.scanned < ring {
                SCAN_AT_FIRST
            } else {
                SCAN_PER_FRAME
            };
            // the layers of sections nearest the camera's height first (the
            // land is there; the sky and deep rock come last), and sections
            // that are all one block (air, rock) cost nothing
            let layer = (span * span) as usize;
            let eye_sy = sec(s.eye.y).clamp(0, dims[1] - 1);
            let mut done = 0;
            let mut looked = 0;
            while done < n && looked < ring {
                let i = self.cursor % ring;
                self.cursor = self.cursor.wrapping_add(1);
                self.scanned = self.scanned.saturating_add(1);
                looked += 1;
                // layer k is 0, +1, -1, +2, -2... from the camera's, folded
                // back into the world where it runs off the top or bottom
                let k = (i / layer) as i32;
                let off = if k % 2 == 1 { (k + 1) / 2 } else { -(k / 2) };
                let sy = (eye_sy + off).rem_euclid(dims[1]);
                let col = (i % layer) as i32;
                let key = [cx - r + col % span, sy, cz - r + col / span];
                if key[0] < 0 || key[2] < 0 || key[0] >= dims[0] || key[2] >= dims[2] {
                    continue;
                }
                if !s.world.has_section(key) {
                    self.sections.remove(&key);
                    continue;
                }
                if let Some(b) = s.world.section_uniform(key)
                    && !self.by_block.contains_key(&b)
                {
                    self.sections.remove(&key);
                    continue;
                }
                done += 1;
                let found = self.scan(s, key);
                if found.is_empty() {
                    self.sections.remove(&key);
                } else {
                    self.sections.insert(key, found);
                }
            }
        }
        // scattered effects: each square is settled once the land under it
        // has arrived; looked at twice a second or when the camera has moved
        let eye = s.eye;
        let due = self.scan_all
            || (clock - self.scatter_at).abs() > 0.5
            || eye.distance_squared(self.scatter_eye) > 16.0;
        self.scan_all = false;
        if !due {
            return;
        }
        self.scatter_at = clock;
        self.scatter_eye = eye;
        self.cells.retain(|(ri, x, z), _| {
            let c = self.scatter[*ri as usize].cell;
            ((*x as f32 + 0.5) * c - eye.x).abs() <= radius + c * 2.0
                && ((*z as f32 + 0.5) * c - eye.z).abs() <= radius + c * 2.0
        });
        for (ri, sc) in self.scatter.iter().enumerate() {
            let reach = radius.min(self.effects[sc.effect as usize].reach);
            let c = sc.cell;
            let (x0, x1) = (
                ((eye.x - reach) / c).floor() as i32,
                ((eye.x + reach) / c).floor() as i32,
            );
            let (z0, z1) = (
                ((eye.z - reach) / c).floor() as i32,
                ((eye.z + reach) / c).floor() as i32,
            );
            for cz in z0..=z1 {
                for cx in x0..=x1 {
                    let key = (ri as u16, cx, cz);
                    if self.cells.contains_key(&key) {
                        continue;
                    }
                    let mut rng = Rng(life::hash2(0x5CA7 + ri as u32, cx, cz));
                    if rng.unit() >= sc.chance {
                        self.cells.insert(key, None);
                        continue;
                    }
                    let (x, z) = ((cx as f32 + rng.unit()) * c, (cz as f32 + rng.unit()) * c);
                    let (bx, bz) = (x.floor() as i32, z.floor() as i32);
                    if life::column(s.world, bx, bz).is_none() {
                        // not here yet: ask again later
                        continue;
                    }
                    let e = sc
                        .place
                        .ground_at(s, bx, bz)
                        .filter(|_| sc.place.area_ok(s, x, z))
                        .map(|(y, _)| Emitter {
                            pos: Vec3::new(x, y as f32 + rng.range(sc.height), z),
                            effect: sc.effect,
                            seed: rng.0,
                        });
                    self.cells.insert(key, e);
                }
            }
        }
    }

    pub fn frame(
        &mut self,
        s: &Scene,
        d: &LifeData,
        clock: f64,
        quality: u8,
        out: &mut Out,
        counts: &mut Counts,
    ) {
        // how far, how many cubes, how many emitters, and the share of
        // each effect's rate
        let (radius, budget, most, thin) = match quality {
            1 => (40.0, 400usize, 30usize, 0.5),
            _ => (80.0, 1200usize, 72usize, 1.0),
        };
        self.refresh(s, radius, clock);
        // nearest first, so the budget cuts the far ones
        let mut order = std::mem::take(&mut self.order);
        order.clear();
        let scattered = self.cells.values().flatten();
        for e in self.sections.values().flatten().chain(scattered) {
            let fx = &self.effects[e.effect as usize];
            if time_gate(fx.time, s.time) <= 0.0 {
                continue;
            }
            let to = e.pos - s.eye;
            let d2 = to.length_squared();
            let reach = fx.reach.min(radius);
            if d2 > reach * reach {
                continue;
            }
            // behind the camera, or well off to the side of what it sees
            // (with room for a plume to drift into view)
            let ahead = to.dot(s.dir);
            let d = d2.sqrt();
            if (d > 12.0 && ahead < -0.2 * d) || (d > 24.0 && ahead < 0.35 * d) {
                continue;
            }
            order.push((d2, *e));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        if std::env::var_os("DYNO_LIFE_DEBUG").is_some() {
            let mut by = vec![[0usize; 2]; self.effects.len()];
            for e in self.sections.values().flatten() {
                by[e.effect as usize][0] += 1;
            }
            for (_, e) in &order {
                by[e.effect as usize][1] += 1;
            }
            let names: Vec<&String> = d.effects.keys().collect();
            let list: Vec<String> = names
                .iter()
                .zip(&by)
                .filter(|(_, n)| n[0] + n[1] > 0)
                .map(|(k, n)| format!("{k}: {} found, {} drawn", n[0], n[1]))
                .collect();
            eprintln!(
                "life: {} sections with emitters; {}",
                self.sections.len(),
                list.join("; ")
            );
        }
        let wind = Vec3::new(d.wind[0], 0.0, d.wind[1]);
        let start = out.fx.len() + out.glow.len() / 6 * 18;
        for (_, e) in &order {
            let used = (out.fx.len() + out.glow.len() / 6 * 18 - start) / 18;
            if used >= budget || counts.emitters >= most {
                break;
            }
            let fx = &self.effects[e.effect as usize];
            emit(s, fx, e, clock, thin, wind, out);
            counts.emitters += 1;
        }
        counts.particles = (out.fx.len() + out.glow.len() / 6 * 18 - start) / 18;
        self.order = order;
    }
}

/// Draw the cubes of one emitter that are alive at `clock`.
fn emit(s: &Scene, fx: &Effect, e: &Emitter, clock: f64, thin: f32, wind: Vec3, out: &mut Out) {
    let gate = time_gate(fx.time, s.time);
    let rate = (fx.rate * thin).max(0.001) as f64;
    let period = 1.0 / rate;
    let newest = (clock * rate).floor() as i64;
    let n = (fx.life[1] as f64 * rate).ceil() as i64 + 1;
    let (block_light, sky_light) = if fx.glow > 0.0 {
        (1.0, 1.0)
    } else {
        s.light_at(e.pos)
    };
    let burst = fx.burst.max(1) as i64;
    for j in 0..n * burst {
        let (i, b) = (newest - j / burst, j % burst);
        // the cubes of a burst are born at the same moment
        let born = (i as f64 + Rng(life::hash(e.seed, i as u32)).unit() as f64) * period;
        let mut r = Rng(life::hash(life::hash(e.seed, i as u32), b as u32 + 1));
        let age = (clock - born) as f32;
        let life = r.range(fx.life);
        if age < 0.0 || age >= life {
            continue;
        }
        let u = age / life;
        let mut pos = e.pos + Vec3::new(r.signed(), r.signed(), r.signed()) * Vec3::from(fx.spawn);
        let v = Vec3::from(fx.velocity)
            + Vec3::new(r.signed(), r.signed(), r.signed()) * Vec3::from(fx.spread);
        pos += if fx.drag > 0.0 {
            v * ((1.0 - (-fx.drag * age).exp()) / fx.drag)
        } else {
            v * age
        };
        pos += Vec3::from(fx.accel) * (0.5 * age * age) + wind * (fx.wind * age);
        if fx.wander[0] > 0.0 {
            let f = fx.wander[1] * std::f32::consts::TAU;
            let (p1, p2, p3) = (r.unit() * 6.3, r.unit() * 6.3, r.unit() * 6.3);
            pos += Vec3::new(
                (age * f + p1).sin() + 0.5 * (age * f * 0.37 + p3).sin(),
                0.45 * (age * f * 1.31 + p2).sin(),
                (age * f * 0.83 + p3).sin() + 0.5 * (age * f * 0.29 + p1).sin(),
            ) * fx.wander[0];
        }
        let fade = if u < fx.fade[0] {
            u / fx.fade[0]
        } else if u > 1.0 - fx.fade[1] {
            (1.0 - u) / fx.fade[1]
        } else {
            1.0
        };
        let mut k = fade * gate;
        if fx.blink > 0.0 {
            let b = (age * fx.blink * std::f32::consts::TAU + r.unit() * 6.3).sin();
            k *= ((b + 0.3) * 1.6).clamp(0.0, 1.0);
        }
        let Some(size) = edge(fx, u, k) else {
            continue;
        };
        // colour through life
        let cs = &fx.colors;
        let at = u * (cs.len() - 1) as f32;
        let (a, t) = ((at as usize).min(cs.len() - 1), at.fract());
        let b = (a + 1).min(cs.len() - 1);
        let shade = 1.0 + fx.shade * r.signed();
        let rgb =
            [0, 1, 2].map(|c| ((cs[a][c] + (cs[b][c] - cs[a][c]) * t) * shade).clamp(0.0, 1.0));
        let lin = rgb.map(|c| c.powf(2.2));
        if fx.glow > 0.0 {
            cube(out.fx, s.eye, pos, size * 0.5, lin, 2.0, 1.0, 1.0);
            if k > 0.02 {
                halo(out.glow, pos, lin, fx.glow, fx.glow_alpha * k);
            }
        } else {
            cube(
                out.fx,
                s.eye,
                pos,
                size * 0.5,
                lin,
                1.0,
                block_light,
                sky_light,
            );
        }
    }
}

/// The edge of a cube `u` of the way through its life, at strength `k` (its
/// fade, its blink): the size the effect asks for there, taken to the
/// nearest step of the cube ladder (`ladder.rs`), so a puff of smoke grows
/// by doubling and an ember dies by halving. `None` once it is smaller
/// than the finest cube.
fn edge(fx: &Effect, u: f32, k: f32) -> Option<f32> {
    let size = (fx.size[0] + (fx.size[1] - fx.size[0]) * u)
        * if fx.glow > 0.0 { 0.5 + 0.5 * k } else { k };
    crate::ladder::snap(size)
}

/// The three faces of a small cube that the camera can see.
#[allow(clippy::too_many_arguments)]
fn cube(
    out: &mut Vec<Vertex>,
    eye: Vec3,
    c: Vec3,
    h: f32,
    rgb: [f32; 3],
    bright: f32,
    light: f32,
    sky: f32,
) {
    let to = eye - c;
    for axis in 0..3 {
        let sign = if to[axis] >= 0.0 { 1.0 } else { -1.0 };
        let mut n = Vec3::ZERO;
        n[axis] = sign;
        let (mut a, mut b) = (Vec3::ZERO, Vec3::ZERO);
        a[(axis + 1) % 3] = h;
        b[(axis + 2) % 3] = h;
        let fc = c + n * h;
        let p = [fc - a - b, fc + a - b, fc + a + b, fc - a + b];
        // top faces brightest, as the boxes of the world are shaded
        let shade = if bright > 1.9 {
            bright
        } else {
            match axis {
                1 if sign > 0.0 => 1.0,
                1 => 0.55,
                0 => 0.8,
                _ => 0.68,
            }
        };
        let order = if sign > 0.0 {
            [0, 1, 2, 0, 2, 3]
        } else {
            [0, 2, 1, 0, 3, 2]
        };
        for i in order {
            out.push(Vertex {
                pos: p[i].into(),
                normal: [0.0; 3],
                color: [rgb[0], rgb[1], rgb[2], shade],
                light,
                sky,
                tex: render::NO_TEX,
            });
        }
    }
}

/// A soft halo that faces the camera.
fn halo(out: &mut Vec<Vertex>, pos: Vec3, rgb: [f32; 3], size: f32, alpha: f32) {
    for (x, y) in [
        (-1.0, -1.0),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
    ] {
        out.push(Vertex {
            pos: pos.into(),
            normal: [x, y, size],
            color: [rgb[0], rgb[1], rgb[2], alpha],
            light: 1.0,
            sky: 1.0,
            tex: [0.0, 0.0, render::KIND_GLOW, 0.0],
        });
    }
}

/// Every effect's cubes through their lives, each in its own frame (for
/// the voxel rule's mesh test).
#[cfg(test)]
pub(crate) fn voxel_objects() -> Vec<crate::voxel_rule::Object> {
    let d = life::data().expect("life.json");
    let mut out = Vec::new();
    for (name, fx) in &d.effects {
        for i in 0..=40 {
            for k in [0.1, 0.35, 0.7, 1.0] {
                let Some(size) = edge(fx, i as f32 / 40.0, k) else {
                    continue;
                };
                let mut verts = Vec::new();
                let c = Vec3::splat(size * 0.5);
                cube(&mut verts, Vec3::new(9.0, 7.0, -5.0), c, size * 0.5, [1.0; 3], 1.0, 1.0, 1.0);
                let list: Vec<([f32; 3], Option<[f32; 3]>)> =
                    verts.iter().map(|v| (v.pos, None)).collect();
                out.push(crate::voxel_rule::Object::from_list(
                    format!("particles {name}"),
                    format!("a cube of {size} m"),
                    1.0 / size,
                    Some([0.0; 3]),
                    &list,
                ));
            }
        }
    }
    out
}
