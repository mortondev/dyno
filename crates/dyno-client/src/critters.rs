//! Ambient critters: birds wheeling overhead, butterflies, ducks, herons,
//! rabbits, deer, crabs. They are cosmetic and live only in this client:
//! the server neither knows nor pays for them, and nobody can fight, loot
//! or count on one.
//!
//! `life.json` says what kinds there are (a small `.vox` model with a few
//! frames, a size, one of the behaviours below) and where they live (areas,
//! ground, time of day). The land is cut into squares; a share of the
//! squares near the camera hold a group, always the same group in the same
//! place.

use crate::assets;
use crate::life::{self, Counts, LifeData, Out, Place, Rng, Scene, Where, time_gate};
use crate::vox;
use crate::voxmodel::{self, Mesh, Paint, Vox};
use dyno_world as w;
use glam::{Affine3A, Mat3, Vec3};
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;

/// What a critter does. The engine's part: everything else is data.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Behave {
    /// wheel in the air round a slowly drifting point
    Flock,
    /// flit about near the ground
    Flutter,
    /// paddle about on water
    Swim,
    /// stand still, now and then dip the head
    Wade,
    /// stand and feed, amble a few steps, bolt from people
    #[default]
    Graze,
    /// short sideways dashes
    Scuttle,
}

/// One kind of critter.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Kind {
    /// the `.vox` file: frames named `idle`, `move1`, `move2` and `alt`
    /// (feeding, head down); only `idle` is required. The model faces +z
    /// and stands on y = 0, centred on x = 0, z = 0.
    pub model: String,
    /// metres per voxel
    pub unit: f32,
    pub behave: Behave,
    /// metres a second
    pub speed: f32,
    /// how far it strays from home, metres
    pub range: f32,
    /// it bolts when someone comes this close (0: never)
    pub flee: f32,
    /// frames a second while moving
    pub fps: f32,
    /// metres above the ground (fliers): lowest and highest
    pub height: [f32; 2],
    /// how high each step bounces, metres (rabbits hop)
    pub hop: f32,
    /// how far away it is still drawn, metres
    pub reach: f32,
}

impl Default for Kind {
    fn default() -> Self {
        Kind {
            model: String::new(),
            unit: 0.06,
            behave: Behave::Graze,
            speed: 1.0,
            range: 6.0,
            flee: 0.0,
            fps: 4.0,
            height: [0.0, 0.0],
            hop: 0.0,
            reach: 60.0,
        }
    }
}

/// Where a kind lives.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Spawn {
    pub critter: String,
    /// side of a square of land, metres
    pub cell: f32,
    /// the share of squares that hold a group
    pub chance: f32,
    /// how many in a group: fewest and most
    pub group: [u8; 2],
    /// how far apart a group stands, metres
    pub spread: f32,
    /// more than 0: instead of living in squares of land, one group keeps
    /// this many metres ahead of where the camera looks (a flock in the
    /// sky of most views, never a sky full of them)
    pub ahead: f32,
    /// where it may be: areas, ground, time of day
    #[serde(rename = "where")]
    pub place: Where,
}

impl Default for Spawn {
    fn default() -> Self {
        Spawn {
            critter: String::new(),
            cell: 24.0,
            chance: 0.5,
            group: [1, 1],
            spread: 3.0,
            ahead: 0.0,
            place: Where::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Doing {
    /// standing (feeding when `alt`) until the clock passes this
    Stand {
        until: f64,
        alt: bool,
    },
    Go {
        to: Vec3,
        run: bool,
    },
}

struct Critter {
    kind: u16,
    rule: u16,
    cell: (i32, i32),
    home: Vec3,
    pos: Vec3,
    yaw: f32,
    seed: u32,
    doing: Doing,
    /// drawn frame: 0 idle, 1 and 2 moving, 3 alt
    frame: u8,
    lift: f32,
}

struct Rule {
    kind: u16,
    cell: f32,
    chance: f32,
    group: [u8; 2],
    spread: f32,
    ahead: f32,
    place: Place,
}

/// The frames of a kind, meshed.
struct Frames {
    /// idle, move1, move2, alt (each falls back to idle)
    mesh: [Rc<Mesh>; 4],
}

#[derive(Default)]
pub struct Critters {
    kinds: Vec<Kind>,
    rules: Vec<Rule>,
    /// squares already settled (true: a group lives there)
    cells: HashMap<(u16, i32, i32), bool>,
    list: Vec<Critter>,
    frames: HashMap<u16, Option<Rc<Frames>>>,
    /// when the squares are next looked at (the clock), and where the
    /// camera was last time
    populate_at: f64,
    populate_eye: Vec3,
    /// look at the squares this frame whatever the clock says
    pub settle_now: bool,
    populate_ms: f32,
}

fn load(kind: &Kind) -> Option<Frames> {
    let Some(bytes) = assets::get(&kind.model) else {
        assets::request(&kind.model);
        return None;
    };
    let file = vox::read(&bytes)
        .map_err(|e| eprintln!("{}: {e}", kind.model))
        .ok()?;
    let mesh_of = |name: &str| -> Option<Rc<Mesh>> {
        let m = file.model(name)?;
        let (mut lo, mut hi) = ([i32::MAX; 3], [i32::MIN; 3]);
        for (p, _) in &m.voxels {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a] + 1);
            }
        }
        if m.voxels.is_empty() {
            return None;
        }
        let mut v = Vox::new(lo, hi);
        let mut inks = [0u8; 256];
        for (p, c) in &m.voxels {
            if inks[*c as usize] == 0 {
                let rgba = file.palette[*c as usize];
                inks[*c as usize] = v.ink([
                    rgba[0] as f32 / 255.0,
                    rgba[1] as f32 / 255.0,
                    rgba[2] as f32 / 255.0,
                ]);
            }
            v.set(p[0], p[1], p[2], inks[*c as usize]);
        }
        Some(Rc::new(v.mesh(0, [0.0; 3])))
    };
    let idle = mesh_of("idle")?;
    let or_idle = |name: &str| mesh_of(name).unwrap_or_else(|| idle.clone());
    Some(Frames {
        mesh: [
            idle.clone(),
            or_idle("move1"),
            or_idle("move2"),
            or_idle("alt"),
        ],
    })
}

impl Critters {
    pub fn new(d: &LifeData) -> Critters {
        let names: Vec<&String> = d.critters.keys().collect();
        let rules = d
            .wildlife
            .iter()
            .filter_map(|r| {
                Some(Rule {
                    kind: names.iter().position(|n| **n == r.critter)? as u16,
                    cell: r.cell.max(4.0),
                    chance: r.chance,
                    group: r.group,
                    spread: r.spread,
                    ahead: r.ahead,
                    place: Place::new(&r.place),
                })
            })
            .collect();
        Critters {
            kinds: d.critters.values().cloned().collect(),
            rules,
            ..Default::default()
        }
    }

    /// Settle the squares near the camera and drop the ones left behind.
    fn populate(&mut self, s: &Scene, clock: f64, radius: f32, thin: f32) {
        let eye = s.eye;
        let far = |rule: &Rule, kind: &Kind, cx: i32, cz: i32| {
            let c = rule.cell;
            let reach = (kind.reach * radius / 70.0).min(radius * 2.0) + c * 1.5;
            ((cx as f32 + 0.5) * c - eye.x).abs() > reach
                || ((cz as f32 + 0.5) * c - eye.z).abs() > reach
        };
        let (rules, kinds) = (&self.rules, &self.kinds);
        self.cells.retain(|(ri, x, z), _| {
            let r = &rules[*ri as usize];
            r.ahead > 0.0 || !far(r, &kinds[r.kind as usize], *x, *z)
        });
        let cells = &self.cells;
        self.list
            .retain(|c| cells.contains_key(&(c.rule, c.cell.0, c.cell.1)));
        for (ri, rule) in self.rules.iter().enumerate() {
            let kind = &self.kinds[rule.kind as usize];
            if rule.ahead > 0.0 {
                // one group that keeps ahead of the view: made anew when
                // the camera has left it far behind or turned its back
                let key = (ri as u16, i32::MAX, i32::MAX);
                let flat = Vec3::new(s.dir.x, 0.0, s.dir.z).normalize_or_zero();
                let home = self
                    .list
                    .iter()
                    .find(|c| c.rule == ri as u16)
                    .map(|c| (c.home - eye) * Vec3::new(1.0, 0.0, 1.0));
                let stale = home.is_none_or(|h| {
                    h.length() > rule.ahead * 2.5 || h.dot(flat) < -rule.ahead * 0.5
                });
                if !stale {
                    continue;
                }
                self.list.retain(|c| c.rule != ri as u16);
                self.cells.insert(key, true);
                let at = eye + flat * rule.ahead;
                if !rule.place.area_ok(s, at.x, at.z) {
                    continue;
                }
                let mut rng = Rng(life::hash2(0xF10C + ri as u32, at.x as i32, at.z as i32));
                let n = rule.group[0]
                    + (rng.unit() * (rule.group[1].saturating_sub(rule.group[0]) + 1) as f32) as u8;
                let centre = Vec3::new(at.x, eye.y - 3.0 + rng.range(kind.height), at.z);
                for _ in 0..n {
                    let home = centre
                        + Vec3::new(rng.signed(), 0.3 * rng.signed(), rng.signed()) * rule.spread;
                    self.list.push(Critter {
                        kind: rule.kind,
                        rule: ri as u16,
                        cell: (i32::MAX, i32::MAX),
                        home,
                        pos: home,
                        yaw: 0.0,
                        seed: rng.0,
                        doing: Doing::Stand {
                            until: clock,
                            alt: false,
                        },
                        frame: 0,
                        lift: 0.0,
                    });
                    rng.unit();
                }
                continue;
            }
            let c = rule.cell;
            let reach = (kind.reach * radius / 70.0).min(radius * 2.0);
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
                    let mut rng = Rng(life::hash2(0xC417 + ri as u32, cx, cz));
                    if rng.unit() >= rule.chance * thin {
                        self.cells.insert(key, false);
                        continue;
                    }
                    let (x, z) = ((cx as f32 + rng.unit()) * c, (cz as f32 + rng.unit()) * c);
                    if life::column(s.world, x.floor() as i32, z.floor() as i32).is_none() {
                        // the land is not here yet: ask again later
                        continue;
                    }
                    let n = rule.group[0]
                        + (rng.unit() * (rule.group[1].saturating_sub(rule.group[0]) + 1) as f32)
                            as u8;
                    let mut any = false;
                    if rule.place.area_ok(s, x, z) {
                        for _ in 0..n {
                            let (mx, mz) = (
                                x + rng.signed() * rule.spread,
                                z + rng.signed() * rule.spread,
                            );
                            let Some((y, _)) =
                                rule.place
                                    .ground_at(s, mx.floor() as i32, mz.floor() as i32)
                            else {
                                continue;
                            };
                            let home = Vec3::new(mx, y as f32 + rng.range(kind.height), mz);
                            any = true;
                            self.list.push(Critter {
                                kind: rule.kind,
                                rule: ri as u16,
                                cell: (cx, cz),
                                home,
                                pos: home,
                                yaw: rng.unit() * std::f32::consts::TAU,
                                seed: rng.0,
                                doing: Doing::Stand {
                                    until: clock + (rng.unit() * 6.0) as f64,
                                    alt: rng.unit() < 0.5,
                                },
                                frame: 0,
                                lift: 0.0,
                            });
                        }
                    }
                    self.cells.insert(key, any);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        s: &Scene,
        _d: &LifeData,
        clock: f64,
        dt: f32,
        quality: u8,
        out: &mut Out,
        counts: &mut Counts,
    ) {
        let (radius, thin) = match quality {
            1 => (40.0, 0.5),
            _ => (70.0, 1.0),
        };
        // twice a second, or when the camera has moved
        if self.settle_now
            || (clock - self.populate_at).abs() > 0.5
            || s.eye.distance_squared(self.populate_eye) > 16.0
        {
            self.settle_now = false;
            self.populate_at = clock;
            self.populate_eye = s.eye;
            let t = std::time::Instant::now();
            self.populate(s, clock, radius, thin);
            self.populate_ms = t.elapsed().as_secs_f32() * 1e3;
        }
        counts.populate_ms = self.populate_ms;
        let dt = dt.clamp(0.0, 0.25);
        for i in 0..self.list.len() {
            let kind = &self.kinds[self.list[i].kind as usize];
            let rule = &self.rules[self.list[i].rule as usize];
            let gate = time_gate(rule.place.time, s.time);
            if gate <= 0.0 {
                continue;
            }
            let c = &mut self.list[i];
            step(c, kind, &rule.place, s, clock, dt);
            let to = c.pos - s.eye;
            let d2 = to.length_squared();
            if d2 > kind.reach * kind.reach || (d2 > 36.0 && to.dot(s.dir) < 0.0) {
                continue;
            }
            let frames = self
                .frames
                .entry(c.kind)
                .or_insert_with(|| load(kind).map(Rc::new));
            let Some(frames) = frames else {
                // the model has not arrived: ask again
                self.frames.remove(&c.kind);
                continue;
            };
            let at = c.pos + Vec3::Y * c.lift;
            let (block, sky) = s.light_at(at + Vec3::Y * 0.3);
            let light = block;
            let xf = Affine3A::from_mat3_translation(
                Mat3::from_rotation_y(c.yaw) * (kind.unit * gate),
                at,
            );
            let before = out.boxes.len();
            voxmodel::emit(
                out.boxes,
                &frames.mesh[c.frame as usize],
                &xf,
                Paint::lit(light),
            );
            // under leaves and roofs the sky light is what dims them
            for v in &mut out.boxes[before..] {
                v.sky = sky;
            }
            counts.critters += 1;
        }
        if std::env::var_os("DYNO_LIFE_DEBUG").is_some() {
            let mut by = vec![0usize; self.kinds.len()];
            for c in &self.list {
                by[c.kind as usize] += 1;
            }
            let names: Vec<&String> = _d.critters.keys().collect();
            let list: Vec<String> = names
                .iter()
                .zip(&by)
                .filter(|(_, n)| **n > 0)
                .map(|(k, n)| format!("{k} {n}"))
                .collect();
            eprintln!("life: critters near: {}", list.join(", "));
        }
    }
}

/// The frame of a two-step cycle at `fps`.
fn cycle(clock: f64, fps: f32, seed: u32) -> u8 {
    1 + (((clock * fps as f64) as i64 + (seed & 1) as i64) & 1) as u8
}

fn heading(v: Vec3) -> f32 {
    v.x.atan2(v.z)
}

/// Turn `yaw` towards `to` by at most `max` radians.
fn turn(yaw: f32, to: f32, max: f32) -> f32 {
    let d =
        (to - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    yaw + d.clamp(-max, max)
}

fn step(c: &mut Critter, k: &Kind, place: &Place, s: &Scene, clock: f64, dt: f32) {
    let mut r = Rng(c.seed);
    let (p1, p2, p3) = (r.unit() * 6.3, r.unit() * 6.3, r.unit() * 6.3);
    let t = clock as f32;
    match k.behave {
        Behave::Flock => {
            // the flock's point drifts; each bird rides its own circle
            let centre = c.home
                + Vec3::new((t * 0.021 + p3).sin(), 0.0, (t * 0.017 + p3 * 1.7).cos()) * k.range;
            let radius = k.range * (0.35 + 0.5 * r.unit());
            let w = k.speed / radius * if c.seed & 2 == 0 { 1.0 } else { -1.0 };
            let a = p1 + t * w;
            let at = |a: f32| {
                centre
                    + Vec3::new(
                        a.cos() * radius,
                        (a * 0.7 + p2).sin() * 1.5,
                        a.sin() * radius * 0.8,
                    )
            };
            c.pos = at(a);
            c.yaw = heading(at(a + 0.05 * w.signum()) - c.pos);
            // flap, then glide a while
            let glide = ((t * 0.23 + p2).sin()) > 0.55;
            c.frame = if glide {
                0
            } else {
                cycle(clock, k.fps, c.seed)
            };
        }
        Behave::Flutter => {
            let at = |t: f32| {
                c.home
                    + Vec3::new(
                        (t * 0.31 * k.speed + p1).sin() + 0.4 * (t * 1.3 * k.speed + p2).sin(),
                        0.0,
                        (t * 0.23 * k.speed + p2).sin() + 0.4 * (t * 1.1 * k.speed + p3).cos(),
                    ) * k.range
                    + Vec3::Y * (0.25 * (t * 2.3 + p3).sin() + 0.12 * (t * 5.1 + p1).sin())
            };
            c.pos = at(t);
            c.yaw = heading(at(t + 0.1) - c.pos);
            c.frame = cycle(clock, k.fps, c.seed);
        }
        Behave::Wade => {
            c.pos = c.home;
            c.yaw += dt * 0.05 * (t * 0.11 + p1).sin();
            c.frame = if (t * 0.19 + p2).sin() > 0.7 { 3 } else { 0 };
        }
        Behave::Swim | Behave::Graze | Behave::Scuttle => walk(c, k, place, s, clock, dt, &mut r),
    }
}

/// Ground and water critters: stand, amble to a spot near home, bolt from
/// people.
fn walk(c: &mut Critter, k: &Kind, place: &Place, s: &Scene, clock: f64, dt: f32, r: &mut Rng) {
    // a fresh throw each time a choice is made
    let mut pick = Rng(life::hash(c.seed, (clock * 7.0) as u32));
    let _ = r;
    // someone too close: away from them
    if k.flee > 0.0 {
        let near = s
            .people
            .iter()
            .map(|p| (*p - c.pos) * Vec3::new(1.0, 0.0, 1.0))
            .min_by(|a, b| a.length_squared().total_cmp(&b.length_squared()));
        if let Some(d) = near
            && d.length_squared() < k.flee * k.flee
            && !matches!(c.doing, Doing::Go { run: true, .. })
        {
            let away = -d.normalize_or_zero();
            let side = Vec3::new(-away.z, 0.0, away.x) * pick.signed() * 0.6;
            c.doing = Doing::Go {
                to: c.pos + (away + side).normalize_or_zero() * (k.flee * 1.6),
                run: true,
            };
        }
    }
    match c.doing {
        Doing::Stand { until, alt } => {
            c.frame = if alt && k.behave != Behave::Swim {
                3
            } else {
                0
            };
            c.lift = 0.0;
            if k.behave == Behave::Swim {
                c.lift = 0.02 * ((clock as f32) * 1.3 + c.seed as f32).sin();
            }
            if clock >= until {
                let a = pick.unit() * std::f32::consts::TAU;
                let d = k.range * (0.3 + 0.7 * pick.unit());
                // crabs dash sideways and short
                let d = if k.behave == Behave::Scuttle {
                    d * 0.5
                } else {
                    d
                };
                c.doing = Doing::Go {
                    to: c.home + Vec3::new(a.cos(), 0.0, a.sin()) * d,
                    run: false,
                };
            }
        }
        Doing::Go { to, run } => {
            let d = (to - c.pos) * Vec3::new(1.0, 0.0, 1.0);
            let dist = d.length();
            let speed = k.speed * if run { 3.2 } else { 1.0 };
            let dir = d.normalize_or_zero();
            let next = c.pos + dir * (speed * dt).min(dist);
            // the ground under the next step must be its kind of ground,
            // and no cliff
            let feet = if k.behave == Behave::Swim {
                // afloat a little under the water line
                (c.pos.y + 0.2).round() as i32
            } else {
                c.pos.y.round() as i32
            };
            let ok = place.step_at(s, next.x.floor() as i32, feet, next.z.floor() as i32);
            let stand = |pick: &mut Rng| Doing::Stand {
                until: clock + (1.5 + pick.unit() * if run { 2.0 } else { 9.0 }) as f64,
                alt: pick.unit() < 0.6,
            };
            match ok {
                Some((y, b)) if dist > 0.05 => {
                    let ground = if b == w::WATER {
                        // afloat: a little under the water line
                        y as f32 - 0.2
                    } else {
                        y as f32
                    };
                    // climb and drop smoothly
                    let ny = c.pos.y + (ground - c.pos.y).clamp(-6.0 * dt, 6.0 * dt);
                    c.pos = Vec3::new(next.x, ny, next.z);
                    let face = if k.behave == Behave::Scuttle {
                        // sideways
                        heading(Vec3::new(-dir.z, 0.0, dir.x))
                    } else {
                        heading(dir)
                    };
                    c.yaw = turn(c.yaw, face, dt * if run { 12.0 } else { 5.0 });
                    let fps = k.fps * if run { 1.8 } else { 1.0 };
                    c.frame = cycle(clock, fps, c.seed);
                    c.lift = k.hop
                        * ((clock as f32 * fps * std::f32::consts::PI).sin().abs())
                        * if run { 1.5 } else { 1.0 };
                }
                _ => c.doing = stand(&mut pick),
            }
        }
    }
}
