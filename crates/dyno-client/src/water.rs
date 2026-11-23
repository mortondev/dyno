//! Water: its surface mesh, its shader and its pass.
//!
//! The look is data (`render.json`, `water`): a base look, named kinds of
//! water that change parts of it, and where each kind lies: still water in
//! areas, running water along courses, both placed from the spawn hearth.
//! The shader works the kind out per pixel, so a new pack shows at once.
//!
//! The pass runs after the solid world is drawn. It reads that picture and
//! its depth, which gives:
//! - colour by the depth of water over the bed, clear at the very shore;
//! - the bed seen through the surface, bent by the ripples;
//! - the sky, sun, moon, land, buildings and lights mirrored (rays marched
//!   across the screen; where a ray leaves it, a plain sky);
//! - foam where the water runs out against a shore or an obstacle.
//!
//! Everything on the surface is laid out in voxel cells (an eighth of a
//! metre near the camera), so ripples, glints and foam are small squares.
//!
//! `quality`: 0 uses neither picture nor depth for colour (the depth of
//! water comes from the mesh, the mirror is a plain sky); 1 adds the bed
//! and the far land and sky in the mirror; 2 adds the marched rays.
//!
//! The mesh (`surface`, `bank`, `side`) reads only the section's grid:
//! - how near the shore a corner is, and how deep the water under it;
//! - white water where surfaces of different levels meet (a river stepping
//!   down), with the way it runs there;
//! - falling sheets down the open sides of water, and over a sill that
//!   holds back higher water.

use crate::look;
use crate::render::{self, KIND_WATER, Vertex};
use crate::terrain_mesh::{DX, DY, DZ, F_OPAQUE, F_WATER, Grid, WATER_DROP, at};
use bytemuck::{Pod, Zeroable};
use dyno_world::{self as w, World};
use egui_wgpu::wgpu;

/// Kinds of water the shader holds: the base look and seven named ones.
pub const MAX_KINDS: usize = 8;
pub const MAX_AREAS: usize = 8;
/// Points of all courses together.
pub const MAX_COURSE: usize = 32;
/// The deepest water the mesh tells apart, in blocks.
const DEEP: i32 = 4;
/// How far white water spreads from a step in the water's level, in blocks.
const SPLASH: i32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq)]
struct Kind {
    /// rgb, and the depth at which it shows fully
    deep: [f32; 4],
    /// rgb, and the clarity
    shallow: [f32; 4],
    /// rgb, and the depth foam forms in
    foam: [f32; 4],
    /// rgb under water, and its density
    fog: [f32; 4],
    /// reflect, ripple, sparkle, flow
    surf: [f32; 4],
    /// lilies
    life: [f32; 4],
}

/// The shader's `WaterU`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct WaterUniform {
    kinds: [Kind; MAX_KINDS],
    /// centre x, z in the world; 1 / half-axes
    areas: [[f32; 4]; MAX_AREAS],
    /// kind, feather
    area_kind: [[f32; 4]; MAX_AREAS],
    /// x, z in the world, half width, kind + 1 (negative: a course ends here)
    course: [[f32; 4]; MAX_COURSE],
    /// areas, course points, quality, 1 with the eye under water
    counts: [f32; 4],
    /// width, height, the surface over a sunken eye, 0
    screen: [f32; 4],
    /// the colours of a lily pad and of its flower
    lily: [f32; 4],
    lily_flower: [f32; 4],
}

fn kind_of(b: &look::Water, k: Option<&look::WaterKind>) -> Kind {
    let d = look::WaterKind::default();
    let k = k.unwrap_or(&d);
    let v = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
    Kind {
        deep: v(k.deep.unwrap_or(b.deep), k.depth.unwrap_or(b.depth)),
        shallow: v(
            k.shallow.unwrap_or(b.shallow),
            k.clarity.unwrap_or(b.clarity),
        ),
        foam: v(
            k.foam.unwrap_or(b.foam),
            k.foam_width.unwrap_or(b.foam_width),
        ),
        fog: v(
            k.fog.unwrap_or(b.fog),
            k.fog_density.unwrap_or(b.fog_density),
        ),
        surf: [
            k.reflect.unwrap_or(b.reflect),
            k.ripple.unwrap_or(b.ripple),
            k.sparkle.unwrap_or(b.sparkle),
            k.flow.unwrap_or(b.flow),
        ],
        life: [k.lilies.unwrap_or(b.lilies), 0.0, 0.0, 0.0],
    }
}

/// What the frame is seen with, besides the pack's look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// the spawn hearth (x, z): areas and courses are placed from it
    pub origin: [f32; 2],
    pub quality: u8,
    /// the height of the surface over the eye, when the eye is under water
    pub sunk: Option<f32>,
    pub size: [u32; 2],
}

/// The look as the shader wants it. Kinds, areas and points beyond what
/// the shader holds are left out; an area or course of an unknown kind
/// takes the base look.
pub fn uniform(l: &look::Water, v: &View) -> WaterUniform {
    let mut u = WaterUniform::zeroed();
    u.kinds = [kind_of(l, None); MAX_KINDS];
    let names: Vec<&String> = l.kinds.keys().take(MAX_KINDS - 1).collect();
    for (i, name) in names.iter().enumerate() {
        u.kinds[i + 1] = kind_of(l, l.kinds.get(*name));
    }
    let index = |name: &String| {
        names
            .iter()
            .position(|n| *n == name)
            .map_or(0.0, |i| (i + 1) as f32)
    };
    let mut n = 0;
    for a in l.areas.iter().take(MAX_AREAS) {
        u.areas[n] = [
            v.origin[0] + a.at[0],
            v.origin[1] + a.at[1],
            1.0 / a.radius[0].max(0.01),
            1.0 / a.radius[1].max(0.01),
        ];
        u.area_kind[n] = [index(&a.kind), a.feather.clamp(0.0, 1.0), 0.0, 0.0];
        n += 1;
    }
    let mut m = 0;
    for c in &l.courses {
        if c.points.len() < 2 || m + c.points.len() > MAX_COURSE {
            continue;
        }
        let k = index(&c.kind) + 1.0;
        for (i, p) in c.points.iter().enumerate() {
            let last = i + 1 == c.points.len();
            u.course[m] = [
                v.origin[0] + p[0],
                v.origin[1] + p[1],
                p[2],
                if last { -k } else { k },
            ];
            m += 1;
        }
    }
    u.counts = [
        n as f32,
        m as f32,
        v.quality as f32,
        v.sunk.is_some() as u8 as f32,
    ];
    u.screen = [
        v.size[0] as f32,
        v.size[1] as f32,
        v.sunk.unwrap_or(0.0),
        0.0,
    ];
    u.lily = [l.lily[0], l.lily[1], l.lily[2], 0.0];
    u.lily_flower = [l.lily_flower[0], l.lily_flower[1], l.lily_flower[2], 0.0];
    u
}

/// The height of the water's surface over a point that is under water.
pub fn sunk(world: &World, eye: [f32; 3]) -> Option<f32> {
    let (x, z) = (eye[0].floor() as i32, eye[2].floor() as i32);
    let mut y = eye[1].floor() as i32;
    if world.get(x, y, z) != w::WATER {
        return None;
    }
    while world.get(x, y + 1, z) == w::WATER {
        y += 1;
    }
    let top = (y + 1) as f32 - WATER_DROP;
    (eye[1] < top).then_some(top)
}

/// The quality asked for from outside (`DYNO_WATER_QUALITY=0..2`), for
/// measuring and for shots.
pub fn quality_override() -> Option<u8> {
    std::env::var("DYNO_WATER_QUALITY")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .map(|q| q.min(2))
}

// ---------------------------------------------------------------- the mesh

/// True for a water cell whose top is open: it has a surface.
#[inline]
pub(crate) fn has_surface(g: &Grid, c: usize) -> bool {
    g.fl[c] & F_WATER != 0 && g.fl[c + DY] & (F_WATER | F_OPAQUE) == 0
}

/// Whether the column holds a surface a little above (+1, +2) or below
/// (-1, -2) the level `y`.
#[inline]
fn step_at(g: &Grid, x: i32, y: i32, z: i32) -> i32 {
    let c = at(x, y, z);
    if has_surface(g, c) {
        0
    } else if has_surface(g, c + DY) {
        1
    } else if has_surface(g, c + 2 * DY) {
        2
    } else if has_surface(g, c - DY) {
        -1
    } else if has_surface(g, c - 2 * DY) {
        -2
    } else {
        0
    }
}

fn water_vertex(layer: u16) -> Vertex {
    Vertex {
        pos: [0.0; 3],
        // how near the shore, how deep (of `DEEP`), how white
        normal: [0.0, 0.0, 0.0],
        // the way the water runs here (x, z about 0.5), 1 on a falling
        // sheet, unused
        color: [0.5, 0.5, 0.0, 1.0],
        light: 0.0,
        sky: 0.0,
        // unused, the height above the foot of a fall, the kind, the frames
        tex: [
            0.0,
            0.0,
            KIND_WATER + layer as f32,
            render::layer_anim(layer),
        ],
    }
}

/// Corners buried in a bank take the light of the others.
fn share_light(vs: &mut [Vertex; 4]) {
    let lit: Vec<(f32, f32)> = vs
        .iter()
        .filter(|v| v.light >= 0.0)
        .map(|v| (v.light, v.sky))
        .collect();
    let n = lit.len().max(1) as f32;
    let mean = lit
        .iter()
        .fold((0.0, 0.0), |a, l| (a.0 + l.0 / n, a.1 + l.1 / n));
    for v in vs.iter_mut().filter(|v| v.light < 0.0) {
        (v.light, v.sky) = mean;
    }
}

/// The water surface over one cell.
pub(crate) fn surface(
    g: &Grid,
    lget: &dyn Fn([i32; 3]) -> f32,
    lsky: &dyn Fn([i32; 3]) -> f32,
    cell: [i32; 3],
    layer: u16,
    out: &mut Vec<Vertex>,
) {
    let [x, y, z] = cell;
    let h = (g.o[1] + y + 1) as f32 - WATER_DROP;
    let mut vs = [water_vertex(layer); 4];
    // is any water round here at another level? (most water is level)
    let mut stepped = false;
    'look: for dz in -SPLASH - 1..=SPLASH + 1 {
        for dx in -SPLASH - 1..=SPLASH + 1 {
            if step_at(g, x + dx, y, z + dz) != 0 {
                stepped = true;
                break 'look;
            }
        }
    }
    for (i, (du, dv)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate() {
        let (cx, cz) = (x + du, z + dv);
        // the four cells around this corner, at the water's level and above it
        let (mut wet, mut open, mut bl, mut sl, mut deep) = (0u32, 0u32, 0.0, 0.0, 0);
        for (ox, oz) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
            let c = at(cx + ox, y, cz + oz);
            if has_surface(g, c) {
                wet += 1;
                deep += (0..DEEP)
                    .take_while(|k| g.fl[c - *k as usize * DY] & F_WATER != 0)
                    .count();
            }
            if g.fl[at(cx + ox, y + 1, cz + oz)] & F_OPAQUE == 0 {
                open += 1;
                bl += lget([cx + ox, y + 1, cz + oz]);
                sl += lsky([cx + ox, y + 1, cz + oz]);
            }
        }
        vs[i].pos = [(g.o[0] + cx) as f32, h, (g.o[2] + cz) as f32];
        vs[i].normal[0] = 1.0 - wet as f32 / 4.0;
        vs[i].normal[1] = deep as f32 / (4 * DEEP) as f32;
        if stepped {
            // white water near a step in the level: strongest under the
            // higher water, a little above the lip; it runs from high to low
            let (mut white, mut run) = (0.0f32, [0.0f32; 2]);
            for dz in -SPLASH..SPLASH {
                for dx in -SPLASH..SPLASH {
                    let s = step_at(g, cx + dx, y, cz + dz);
                    if s == 0 {
                        continue;
                    }
                    let v = [dx as f32 + 0.5, dz as f32 + 0.5];
                    let d = (v[0] * v[0] + v[1] * v[1]).sqrt();
                    // (a sill a block wide may stand between the two)
                    let k = (1.0 - (d - 1.0) / SPLASH as f32).clamp(0.0, 1.0);
                    let sign = if s > 0 { -1.0 } else { 1.0 };
                    white = white.max(if s > 0 { k } else { k * 0.6 });
                    run[0] += sign * v[0] / d * k;
                    run[1] += sign * v[1] / d * k;
                }
            }
            let len = (run[0] * run[0] + run[1] * run[1]).sqrt().max(1.0);
            vs[i].normal[2] = white;
            vs[i].color[0] = 0.5 + 0.5 * run[0] / len * white;
            vs[i].color[1] = 0.5 + 0.5 * run[1] / len * white;
        }
        // light from the open cells above; a corner buried in the bank is
        // marked and takes its light from the others below
        let k = 1.0 / (open.max(1) as f32 * 15.0);
        vs[i].light = if open == 0 { -1.0 } else { bl * k };
        vs[i].sky = sl * k;
    }
    share_light(&mut vs);
    // split so the shore factor interpolates evenly
    let order =
        if (vs[0].normal[0] - vs[2].normal[0]).abs() <= (vs[1].normal[0] - vs[3].normal[0]).abs() {
            [0, 1, 2, 0, 2, 3]
        } else {
            [1, 2, 3, 1, 3, 0]
        };
    out.extend(order.into_iter().map(|i| vs[i]));
}

const SIDES: [[i32; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

/// A sheet of falling water down the side of the cell that faces `dir`,
/// from `top` (a height in the world) down to the first water or ground
/// under it. `lip` pushes it out from the face.
#[allow(clippy::too_many_arguments)]
fn sheet(
    g: &Grid,
    lget: &dyn Fn([i32; 3]) -> f32,
    lsky: &dyn Fn([i32; 3]) -> f32,
    cell: [i32; 3],
    dir: [i32; 2],
    top: f32,
    lip: f32,
    layer: u16,
    out: &mut Vec<Vertex>,
) {
    let [x, y, z] = cell;
    let (nx, nz) = (x + dir[0], z + dir[1]);
    // down the open column in front of the face
    let mut foot = (g.o[1] + y) as f32;
    for k in 1..=3 {
        let c = at(nx, y - k, nz);
        if g.fl[c] & F_WATER != 0 {
            // into the pool below, a little under its surface
            foot = (g.o[1] + y - k + 1) as f32 - WATER_DROP - 0.06;
            break;
        }
        if g.fl[c] & F_OPAQUE != 0 {
            break;
        }
        foot = (g.o[1] + y - k) as f32;
    }
    if top - foot < 0.05 {
        return;
    }
    let (bl, sl) = (lget([nx, y, nz]) / 15.0, lsky([nx, y, nz]) / 15.0);
    // the face: its centre line and the half-width along it
    let (fx, fz) = (
        (g.o[0] + x) as f32 + 0.5 + dir[0] as f32 * (0.5 + lip),
        (g.o[2] + z) as f32 + 0.5 + dir[1] as f32 * (0.5 + lip),
    );
    let (ax, az) = (dir[1] as f32 * 0.5, dir[0] as f32 * 0.5);
    let mut v = water_vertex(layer);
    v.light = bl;
    v.sky = sl;
    v.normal = [0.0, 0.0, 1.0];
    v.color = [
        0.5 + 0.5 * dir[0] as f32,
        0.5 + 0.5 * dir[1] as f32,
        1.0,
        1.0,
    ];
    let corner = |s: f32, h: f32| {
        let mut q = v;
        q.pos = [fx + ax * s, h, fz + az * s];
        q.tex[1] = h - foot;
        q
    };
    let q = [
        corner(-1.0, foot),
        corner(1.0, foot),
        corner(1.0, top),
        corner(-1.0, top),
    ];
    out.extend([0, 1, 2, 0, 2, 3].into_iter().map(|i| q[i]));
}

/// The side of a water cell that faces `dir`, where no water and nothing
/// solid is. Into the open air the water falls. Where something stands in
/// the water (a post, a reed), the surface carries on round it instead.
pub(crate) fn side(
    g: &Grid,
    lget: &dyn Fn([i32; 3]) -> f32,
    lsky: &dyn Fn([i32; 3]) -> f32,
    cell: [i32; 3],
    dir: [i32; 2],
    layer: u16,
    out: &mut Vec<Vertex>,
) {
    let [x, y, z] = cell;
    let c = at(x, y, z);
    let (nx, nz) = (x + dir[0], z + dir[1]);
    if g.ids[at(nx, y, nz)] != w::AIR {
        // the first of its wet neighbours draws the surface over it
        let first = SIDES
            .iter()
            .find(|d| has_surface(g, at(nx - d[0], y, nz - d[1])));
        if has_surface(g, c) && first == Some(&dir) {
            surface(g, lget, lsky, [nx, y, nz], layer, out);
        }
        return;
    }
    let top = (g.o[1] + y + 1) as f32 - if has_surface(g, c) { WATER_DROP } else { 0.0 };
    sheet(g, lget, lsky, cell, dir, top, 0.0, layer, out);
}

/// Solid ground or stone at the water's edge.
/// - The shore: the surface of the water beside it carries on under it
///   (smooth ground slopes back from the water).
/// - A sill: with higher water behind it and lower water in front, the
///   water runs over it and falls.
pub(crate) fn bank(
    g: &Grid,
    lget: &dyn Fn([i32; 3]) -> f32,
    lsky: &dyn Fn([i32; 3]) -> f32,
    cell: [i32; 3],
    layer: u16,
    shore: bool,
    out: &mut Vec<Vertex>,
) {
    let [x, y, z] = cell;
    let c = at(x, y, z);
    if g.fl[c + DY] & F_WATER != 0 {
        return;
    }
    if shore {
        let near = [
            c - DX,
            c + DX,
            c - DZ,
            c + DZ,
            c - DX - DZ,
            c + DX - DZ,
            c - DX + DZ,
            c + DX + DZ,
        ];
        if near.iter().any(|q| has_surface(g, *q)) {
            surface(g, lget, lsky, cell, layer, out);
        }
    }
    if g.fl[c + DY] & F_OPAQUE != 0 {
        return;
    }
    let mut over = false;
    for dir in SIDES {
        let behind = at(x - dir[0], y, z - dir[1]);
        let front = at(x + dir[0], y, z + dir[1]);
        if !has_surface(g, behind) || g.fl[front] & (F_OPAQUE | F_WATER) != 0 {
            continue;
        }
        let lower = (1..=3).any(|k| has_surface(g, front - k as usize * DY))
            && (1..=3)
                .take_while(|k| g.fl[front - *k as usize * DY] & F_OPAQUE == 0)
                .any(|k| g.fl[front - k as usize * DY] & F_WATER != 0);
        if !lower {
            continue;
        }
        let top = (g.o[1] + y + 1) as f32 + 0.04;
        sheet(g, lget, lsky, cell, dir, top, 0.04, layer, out);
        if !over {
            over = true;
            // the water running over the top of the sill
            let (bl, sl) = (lget([x, y + 1, z]) / 15.0, lsky([x, y + 1, z]) / 15.0);
            let mut v = water_vertex(layer);
            v.light = bl;
            v.sky = sl;
            v.normal = [0.0, 0.0, 1.0];
            v.color = [
                0.5 + 0.5 * dir[0] as f32,
                0.5 + 0.5 * dir[1] as f32,
                0.0,
                1.0,
            ];
            let q = [(0, 0), (1, 0), (1, 1), (0, 1)].map(|(du, dv)| {
                let mut q = v;
                // out to the lip in front, and back into the water behind
                let (px, pz) = (
                    du as f32 + (du as f32 - 0.5) * 0.08,
                    dv as f32 + (dv as f32 - 0.5) * 0.08,
                );
                q.pos = [(g.o[0] + x) as f32 + px, top, (g.o[2] + z) as f32 + pz];
                q
            });
            out.extend([0, 1, 2, 0, 2, 3].into_iter().map(|i| q[i]));
        }
    }
}

// ------------------------------------------------------------- the shader

/// Follows the head of the world shader (the uniforms, the look, the sun's
/// shadow, the grade, the noise and the swell).
const SHADER: &str = r#"
struct WKind { deep: vec4<f32>, shallow: vec4<f32>, foam: vec4<f32>, fog: vec4<f32>, surf: vec4<f32>, life: vec4<f32> };
struct WaterU {
  kinds: array<WKind, 8>,
  areas: array<vec4<f32>, 8>,
  area_kind: array<vec4<f32>, 8>,
  course: array<vec4<f32>, 32>,
  counts: vec4<f32>,
  screen: vec4<f32>,
  lily: vec4<f32>,
  lily_flower: vec4<f32>,
};
@group(1) @binding(0) var<uniform> wu: WaterU;
@group(1) @binding(1) var scene_tex: texture_2d<f32>;
@group(1) @binding(2) var scene_samp: sampler;
@group(1) @binding(3) var scene_depth: texture_depth_2d;

fn mix_kind(a: WKind, b: WKind, k: f32) -> WKind {
  var o: WKind;
  o.deep = mix(a.deep, b.deep, k);
  o.shallow = mix(a.shallow, b.shallow, k);
  o.foam = mix(a.foam, b.foam, k);
  o.fog = mix(a.fog, b.fog, k);
  o.surf = mix(a.surf, b.surf, k);
  o.life = mix(a.life, b.life, k);
  return o;
}

// The kind of water at a place: the base look, the course it lies on (how
// much, and its kind), then the areas over it.
fn kind_at(p: vec2<f32>, on_course: f32, course_kind: f32) -> WKind {
  var t = wu.kinds[0];
  if (on_course > 0.001) { t = mix_kind(t, wu.kinds[i32(course_kind + 0.5)], on_course); }
  let n = i32(wu.counts.x);
  for (var i = 0; i < n; i = i + 1) {
    let a = wu.areas[i];
    let e = length((p - a.xy) * a.zw);
    let f = max(wu.area_kind[i].y, 0.001);
    let k = 1.0 - smoothstep(1.0 - f, 1.0, e);
    if (k > 0.0) { t = mix_kind(t, wu.kinds[i32(wu.area_kind[i].x + 0.5)], k); }
  }
  return t;
}

struct WIn { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32>, @location(3) light: f32, @location(4) sky: f32, @location(5) tex: vec4<f32> };
struct WOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) wpos: vec3<f32>,
  // how near the shore, how deep (of 4 m), how white
  @location(1) shore: vec3<f32>,
  // the way the water runs at a step (x, z), 1 on a falling sheet, the height above its foot
  @location(2) flow: vec4<f32>,
  // block light, sky light
  @location(3) light: vec2<f32>,
  // the way the course runs (x, z, scaled by how much this is on it), how much, its kind
  @location(4) course: vec4<f32>,
};

@vertex fn vs_water(v: WIn) -> WOut {
  var o: WOut;
  if (v.tex.z < K_WATER) {
    // glass and the like share the buffer: not drawn here
    o.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    return o;
  }
  var pos = v.pos;
  pos.y = pos.y + swell(pos.xz, u.cam.w);
  // the nearest stretch of a course: which way it runs, and its kind
  var best = 1e9;
  var kind = 0.0;
  var dir = vec2<f32>(0.0);
  let n = i32(wu.counts.y);
  for (var i = 0; i + 1 < n; i = i + 1) {
    let a = wu.course[i];
    if (a.w < 0.0) { continue; }
    let b = wu.course[i + 1];
    let ab = b.xy - a.xy;
    let h = clamp(dot(pos.xz - a.xy, ab) / max(dot(ab, ab), 0.0001), 0.0, 1.0);
    let mid = length(pos.xz - a.xy - ab * h);
    let edge = mid - mix(a.z, b.z, h);
    // stretches blend where they join, so the water turns the bend smoothly
    let wgt = 1.0 / pow(1.0 + mid, 4.0);
    dir = dir + normalize(ab) * wgt;
    if (edge < best) { best = edge; kind = a.w - 1.0; }
  }
  let on = 1.0 - smoothstep(1.5, 6.0, best);
  var along = vec2<f32>(0.0);
  if (on > 0.0) { along = normalize(dir + vec2<f32>(0.00001, 0.0)) * on; }
  o.clip = u.view_proj * vec4<f32>(pos, 1.0);
  o.wpos = pos;
  o.shore = v.normal;
  o.flow = vec4<f32>(v.color.xy * 2.0 - 1.0, v.color.z, v.tex.y);
  o.light = vec2<f32>(v.light, v.sky);
  o.course = vec4<f32>(along, on, kind);
  return o;
}

fn world_at(uv: vec2<f32>, z: f32) -> vec3<f32> {
  let h = u.inv_view_proj * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, z, 1.0);
  return h.xyz / h.w;
}
fn depth_at(uv: vec2<f32>) -> f32 {
  let s = vec2<i32>(wu.screen.xy);
  let px = clamp(vec2<i32>(uv * wu.screen.xy), vec2<i32>(0), s - vec2<i32>(1));
  return textureLoad(scene_depth, px, 0);
}
fn scene_at(uv: vec2<f32>) -> vec3<f32> {
  return textureSampleLevel(scene_tex, scene_samp, uv, 0.0).rgb;
}
fn inside(uv: vec2<f32>) -> bool {
  return uv.x > 0.0 && uv.x < 1.0 && uv.y > 0.0 && uv.y < 1.0;
}

// The sky without its clouds, where the mirror cannot see the real one.
fn sky_plain(dir: vec3<f32>) -> vec3<f32> {
  var c = mix(u.fog.rgb, u.sky_top.rgb, smoothstep(-0.02, 0.55, dir.y));
  c += vec3<f32>(1.0, 0.75, 0.45) * pow(max(dot(dir, u.sun.xyz), 0.0), 48.0) * 0.6 * (1.0 - u.sun.w * 0.8);
  return c;
}

// The ripples as a height, 0..1: long crests on still water, rounder
// and busier where it runs.
fn wave(p: vec2<f32>, t: f32, run: f32) -> f32 {
  let s = mix(vec2<f32>(0.42, 1.6), vec2<f32>(1.0, 1.0), run);
  let a = vnoise(p * s + vec2<f32>(t * 0.30, t * 0.11));
  let b = vnoise(p * s * 2.3 + vec2<f32>(17.0 - t * 0.42, 9.0 + t * 0.23));
  return a * 0.62 + b * 0.38;
}
// The same, carried along by the flow: two copies, each sliding a short
// way and fading out as the other fades in, so a bend never smears them.
fn waves(p: vec2<f32>, t: f32, flow: vec2<f32>, run: f32) -> f32 {
  if (dot(flow, flow) < 0.0004) { return wave(p, t, run); }
  let period = 3.0;
  let ph = fract(t / period);
  let ph2 = fract(t / period + 0.5);
  let a = wave(p - flow * ((ph - 0.5) * period), t, run);
  let b = wave(p - flow * ((ph2 - 0.5) * period) + vec2<f32>(5.3, 2.7), t, run);
  return mix(b, a, 1.0 - abs(2.0 * ph - 1.0));
}

// Short strokes one cell thick, in rows: each row slides at its own pace,
// and strokes come and go along it. `c` is the cell (along the rows,
// across them), `g` its size, `shift` how far the rows have slid, `many`
// how thick they lie (0..1).
fn strokes(c: vec2<f32>, g: f32, shift: f32, many: f32, seed: f32) -> f32 {
  let row = c.y;
  let rh = hash2(vec2<f32>(row, seed));
  let along = (c.x * g + shift + rh * 37.0) / 1.25;
  let seg = floor(along);
  let h = hash2(vec2<f32>(seg, row * 3.0 + seed));
  let len = 0.2 + 0.55 * hash2(vec2<f32>(seg + 91.0, row * 3.0 + seed));
  return step(1.0 - many, h) * step(fract(along), len);
}
// Strokes on still water (along x, drifting), or along the flow: rows on
// the grid axis the water mostly follows, carried a short way and faded
// into a second copy (as `waves`).
fn ripples(cell: vec2<f32>, g: f32, t: f32, flow: vec2<f32>, many: f32, seed: f32) -> f32 {
  let rh = hash2(vec2<f32>(cell.y, seed + 7.0));
  let still = strokes(cell, g, t * (0.12 + 0.2 * rh), many, seed);
  let speed = length(flow);
  if (speed < 0.02) { return still; }
  let period = 3.0;
  let ph = fract(t / period);
  let ph2 = fract(t / period + 0.5);
  let w = 1.0 - abs(2.0 * ph - 1.0);
  let ax = abs(flow.x);
  let az = abs(flow.y);
  var r = 0.0;
  if (ax > az * 0.5) {
    let k = smoothstep(0.5, 1.0, ax / max(az, 0.0001));
    r += k * mix(strokes(cell, g, -flow.x * (ph2 - 0.5) * period + 11.0, many, seed + 1.0), strokes(cell, g, -flow.x * (ph - 0.5) * period, many, seed), w);
  }
  if (az > ax * 0.5) {
    let k = smoothstep(0.5, 1.0, az / max(ax, 0.0001));
    r += k * mix(strokes(cell.yx, g, -flow.y * (ph2 - 0.5) * period + 11.0, many, seed + 3.0), strokes(cell.yx, g, -flow.y * (ph - 0.5) * period, many, seed + 2.0), w);
  }
  return mix(still, min(r, 1.0), smoothstep(0.02, 0.2, speed));
}

@fragment fn fs_water(i: WOut) -> @location(0) vec4<f32> {
  // how what the vertices carry changes across the screen, to snap it to cells
  let dpx = dpdx(i.wpos.xz);
  let dpy = dpdy(i.wpos.xz);
  let dsx = dpdx(i.shore);
  let dsy = dpdy(i.shore);
  let zs = textureLoad(scene_depth, vec2<i32>(i.clip.xy), 0);
  // hidden behind the land
  if (i.clip.z > zs) { discard; }
  let quality = wu.counts.z;
  let have = quality > 0.5;
  let t = u.cam.w;
  let p = i.wpos;
  let to_cam = u.cam.xyz - p;
  let d = length(to_cam);
  let V = to_cam / d;
  let uv = i.clip.xy / wu.screen.xy;
  let fall = i.flow.z;
  let K = kind_at(p.xz, i.course.z, i.course.w);

  // voxel cells: an eighth of a metre near, doubling with distance
  let g = 0.125 * exp2(clamp(floor(log2(d / 9.0)), 0.0, 3.0));
  let cell = floor(p.xz / g);
  let q = (cell + 0.5) * g;
  var sh = i.shore;
  let det = dpx.x * dpy.y - dpx.y * dpy.x;
  if (abs(det) > 1e-10 && fall < 0.5) {
    let off = q - p.xz;
    let a = (off.x * dpy.y - off.y * dpy.x) / det;
    let b = (dpx.x * off.y - dpx.y * off.x) / det;
    sh = clamp(i.shore + dsx * a + dsy * b, vec3<f32>(0.0), vec3<f32>(1.0));
  }
  let white = sh.z;

  // light: the sun or moon (less in shadow), or lamps
  let sl = i.light.y * i.light.y;
  let bl = i.light.x * i.light.x;
  var vis = 1.0;
  if (have) { vis = sun_visible(p, vec3<f32>(0.0, 1.0, 0.0)); }
  let lit = max(u.light.rgb * (0.5 + 0.5 * vis) * (sl * 0.97 + 0.03), vec3<f32>(1.0, 0.80, 0.55) * bl);
  let fogk = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);

  // the way the water runs: along its course, and down a step
  let run = clamp(i.course.z + white, 0.0, 1.0);
  let flow = i.course.xy * K.surf.w + i.flow.xy * 1.6;

  // ripples, one height a cell; the surface leans by its slope
  let h0 = waves(q, t, flow, run);
  let hx = waves(q + vec2<f32>(g, 0.0), t, flow, run);
  let hz = waves(q + vec2<f32>(0.0, g), t, flow, run);
  let lean = vec2<f32>(hx - h0, hz - h0) / g * (0.03 * K.surf.y * (1.0 + 1.5 * white)) / (1.0 + d * 0.012);
  var n = normalize(vec3<f32>(-lean.x, 1.0, -lean.y));
  // bright strokes, thicker on the swells, and a few dark ones
  let many = clamp((0.10 + 0.5 * smoothstep(0.35, 0.75, h0)) * K.surf.y, 0.0, 0.8);
  let crest = ripples(cell, g, t, flow, many, 0.0);
  let trough = ripples(cell + vec2<f32>(5.0, 3.0), g, t, flow, many * 0.3, 40.0) * (1.0 - crest);

  let sunk = wu.counts.w > 0.5 && u.cam.y < p.y;
  if (sunk && fall < 0.5) {
    // the surface from below: the world above through a bright window
    // overhead, the deep mirrored further out
    var above = grade(sky_plain(vec3<f32>(0.0, 1.0, 0.0)));
    if (have) { above = scene_at(uv + lean * 0.6); }
    let out_of_window = smoothstep(0.30, 0.55, 1.0 - abs(V.y));
    var c = mix(above * 1.08 + vec3<f32>(0.02), grade(K.deep.rgb * lit * 1.4), out_of_window);
    c = c * (1.0 + 0.25 * crest);
    return vec4<f32>(c, select(mix(0.55, 0.95, out_of_window), 1.0, have));
  }

  // how deep the water is here, and how far the eye looks through it
  let baked = sh.y * 4.0;
  var dv = baked;
  var through = baked / max(V.y, 0.15);
  var dvq = baked;
  var edge = smoothstep(0.0, 0.12, baked + (1.0 - sh.x) * 0.5);
  var behind = vec3<f32>(0.0);
  var bed_c = vec3<f32>(0.0);
  if (have) {
    behind = scene_at(uv);
    bed_c = behind;
    if (zs < 1.0) {
      let bed = world_at(uv, zs);
      dv = max(p.y - bed.y, 0.0);
      through = distance(bed, p);
    } else {
      dv = 40.0;
      through = 200.0;
    }
    dvq = dv;
    edge = smoothstep(0.0, 0.035, dv);
    if (fall < 0.5) {
      // the bed, bent by the ripples (never something in front of the water)
      let ruv = uv + lean * (0.5 * min(dv, 1.0)) / (1.0 + d * 0.05);
      let zr = depth_at(ruv);
      if (zr > i.clip.z && inside(ruv)) {
        bed_c = scene_at(ruv);
        if (zr < 1.0) {
          let bed = world_at(ruv, zr);
          dv = max(p.y - bed.y, 0.0);
          through = distance(bed, p);
        }
      }
      // the depth at the middle of this cell: foam in squares
      let cq = u.view_proj * vec4<f32>(q.x, p.y, q.y, 1.0);
      let nq = cq.xyz / cq.w;
      let uq = vec2<f32>(nq.x * 0.5 + 0.5, 0.5 - nq.y * 0.5);
      if (cq.w > 0.0 && inside(uq)) {
        let zq = depth_at(uq);
        if (zq >= nq.z && zq < 1.0) { dvq = max(p.y - world_at(uq, zq).y, 0.0); }
      }
    }
  }

  // the body of the water: clear, then the shallow colour, then the deep
  let cover_sh = 1.0 - exp(-(through + 0.3 * smoothstep(0.0, 0.25, dv)) / max(K.shallow.w, 0.05));
  let cover_dp = 1.0 - exp(-3.0 * dv / max(K.deep.w, 0.05));
  var body = mix(K.shallow.rgb, K.deep.rgb, cover_dp) * lit;
  body = body * (1.0 + 0.16 * (h0 - 0.5) + 0.55 * crest - 0.12 * trough) + crest * lit * 0.03;
  var alpha = clamp(max(cover_sh, cover_dp), 0.0, 1.0);
  var col = grade(mix(body, u.fog.rgb, fogk));
  if (have) {
    col = mix(bed_c, col, alpha);
    // crests catch the light over clear water too
    col = col * (1.0 + (0.22 * crest - 0.08 * trough) * (1.0 - alpha));
    alpha = 1.0;
  }

  if (fall > 0.5) {
    // a falling sheet: streaks sliding down, white at the foot and the lip
    let across = floor((p.x + p.z) * 8.0);
    let hc = hash2(vec2<f32>(across, 3.0));
    let down = floor((p.y + t * (2.0 + 1.5 * hc)) * 8.0 / 3.0);
    let streak = hash2(vec2<f32>(across, down));
    let foot = 1.0 - smoothstep(0.1, 0.55, i.flow.w + (hc - 0.5) * 0.25);
    let k = clamp(max(smoothstep(0.35, 0.75, streak * 0.65 + hc * 0.45), foot), 0.0, 1.0);
    let sheet_c = mix(K.shallow.rgb * 1.25, K.foam.rgb, k) * (lit * 0.92 + u.light.rgb * 0.08);
    let c = grade(mix(sheet_c, u.fog.rgb, fogk));
    let a = mix(0.72, 0.97, k);
    if (have) { return vec4<f32>(mix(behind, c, a), 1.0); }
    return vec4<f32>(c, a);
  }

  // the mirror
  let ndv = max(dot(V, n), 0.0);
  let fres = (0.02 + 0.98 * pow(1.0 - ndv, 5.0)) * K.surf.x;
  var R = reflect(-V, n);
  R.y = abs(R.y) + 0.01;
  R = normalize(R);
  var refl = grade(sky_plain(R));
  if (have) {
    var got = false;
    var hit = vec2<f32>(0.0);
    if (quality > 1.5) {
      // march the mirrored ray across the screen, in longer and longer steps
      var len = 0.35 + 0.3 * hash2(cell);
      var last = 0.0;
      for (var s = 0; s < 22; s = s + 1) {
        let at = p + R * len;
        let c = u.view_proj * vec4<f32>(at, 1.0);
        if (c.w <= 0.0) { break; }
        let nd = c.xyz / c.w;
        let suv = vec2<f32>(nd.x * 0.5 + 0.5, 0.5 - nd.y * 0.5);
        if (!inside(suv)) { break; }
        let zd = depth_at(suv);
        if (zd < nd.z) {
          let gap = distance(u.cam.xyz, at) - distance(u.cam.xyz, world_at(suv, zd));
          if (gap < max(1.2, (len - last) * 1.5)) { got = true; hit = suv; }
          break;
        }
        last = len;
        len = len * 1.27 + 0.2;
      }
    }
    if (!got) {
      // what lies far off along the ray: the sky itself, or distant land
      let c = u.view_proj * vec4<f32>(R, 0.0);
      if (c.w > 0.0) {
        let nd = c.xyz / c.w;
        let suv = vec2<f32>(nd.x * 0.5 + 0.5, 0.5 - nd.y * 0.5);
        if (inside(suv)) {
          let zd = depth_at(suv);
          if (zd >= 1.0 || distance(u.cam.xyz, world_at(suv, zd)) > 45.0) { got = true; hit = suv; }
        }
      }
    }
    if (got) {
      let rim = smoothstep(0.0, 0.07, min(min(hit.x, 1.0 - hit.x), min(hit.y, 1.0 - hit.y)));
      refl = mix(refl, scene_at(hit), rim);
    }
  }
  col = mix(col, refl, fres);
  alpha = mix(alpha, 1.0, fres);

  // glints of the sun or the moon: whole cells flash, each leaning its own way
  let moon = u.sun.y <= 0.0;
  let ldir = normalize(select(u.sun.xyz, -u.sun.xyz, moon));
  let tick = floor(t * 5.0);
  let j = vec2<f32>(hash2(cell + vec2<f32>(tick * 7.0, 3.0)), hash2(cell + vec2<f32>(11.0, tick * 13.0))) - 0.5;
  let ns = normalize(vec3<f32>(-lean.x * 2.5 + j.x * 0.16, 1.0, -lean.y * 2.5 + j.y * 0.16));
  let glint = smoothstep(0.990, 0.996, dot(reflect(-V, ns), ldir));
  let glint_c = select(vec3<f32>(1.0, 0.93, 0.78) * u.light.rgb, vec3<f32>(0.62, 0.72, 0.95) * u.sun.w, moon);
  col += glint_c * (glint * K.surf.z * vis * sl * 0.9);
  // and by day a cell here and there on a swell twinkles
  let twinkle = step(0.9975, hash2(cell + vec2<f32>(tick * 3.0, tick * 17.0))) * smoothstep(0.5, 0.7, h0);
  col += vec3<f32>(1.0, 0.97, 0.9) * u.light.rgb * (twinkle * K.surf.z * vis * sl * (1.0 - u.sun.w) * 0.55);

  // foam, in squares: against the shore and anything standing in the
  // water, a broken line a little way out, and all over white water
  let fn1 = waves(q * 1.7 + vec2<f32>(3.1, 8.2), t, flow * 1.7, 1.0);
  let lap = 0.5 + 0.5 * sin(t * 1.2 + vnoise(q * 0.35) * 6.283);
  let fw = K.foam.w;
  var foam = 0.0;
  if (have) {
    foam = 1.0 - smoothstep(fw * (0.30 + 0.5 * lap * fn1), fw * (0.42 + 0.62 * lap * fn1), dvq);
    let ring = smoothstep(fw * 1.5, fw * 1.7, dvq) * (1.0 - smoothstep(fw * 2.1, fw * 2.3, dvq)) * step(0.58, fn1);
    foam = max(foam, ring * 0.6);
    foam = max(foam, smoothstep(0.80, 0.86, sh.x + (fn1 - 0.5) * 0.3 + 0.05 * lap));
  } else {
    foam = smoothstep(0.62, 0.68, sh.x + (fn1 - 0.5) * 0.38 + 0.06 * lap);
  }
  let churn = hash2(cell + vec2<f32>(floor(t * 7.0) * 5.0, 1.0));
  foam = max(foam, smoothstep(0.42, 0.52, white * (0.55 + 0.75 * fn1) + (churn - 0.5) * 0.25 * white));
  let foam_c = grade(mix(K.foam.rgb * (lit * 0.94 + u.light.rgb * 0.06), u.fog.rgb, fogk));
  col = mix(col, foam_c, foam);
  alpha = max(alpha, foam);

  // lily pads on calm shallows, in clumps: a disc of cells with a notch,
  // some with a flower
  let calm = (1.0 - smoothstep(0.02, 0.2, length(flow))) * (1.0 - smoothstep(0.0, 0.15, white));
  let shallows = 1.0 - smoothstep(1.2, 2.2, max(dvq, baked * 0.6));
  let lilies = K.life.x * calm * shallows * (1.0 - foam) * smoothstep(0.2, 0.4, max(dvq, baked));
  if (lilies > 0.001 && g < 0.6) {
    {
      {
        let spot = floor(q / 0.9);
        let hs = hash2(spot);
        let clump = smoothstep(0.45, 0.7, vnoise(spot * 0.23 + vec2<f32>(4.0, 9.0)));
        if (hs < lilies * clump) {
          let mid = (spot + 0.42 + 0.16 * vec2<f32>(hash2(spot + vec2<f32>(7.0, 1.0)), hash2(spot + vec2<f32>(2.0, 5.0)))) * 0.9;
          let r = q - mid + vec2<f32>(0.01 * sin(t * 0.7 + hs * 40.0), 0.0);
          let rad = 0.20 + 0.15 * hash2(spot + vec2<f32>(3.0, 3.0));
          let turn = hs * 251.0;
          let notch = dot(normalize(r + vec2<f32>(0.0001)), vec2<f32>(cos(turn), sin(turn))) > 0.86 && length(r) > 0.07;
          if (length(r) < rad && !notch) {
            var pad = wu.lily.rgb * (0.82 + 0.36 * hash2(cell + vec2<f32>(9.0, 4.0)));
            if (length(r) > rad - g * 1.1) { pad = pad * 0.72; }
            if (hash2(spot + vec2<f32>(8.0, 8.0)) < 0.35 && max(abs(r.x), abs(r.y)) < 0.095) { pad = wu.lily_flower.rgb; }
            col = grade(mix(pad * (lit * 0.95 + u.light.rgb * 0.05), u.fog.rgb, fogk));
            alpha = 1.0;
          }
        }
      }
    }
  }

  // the very edge thins to nothing
  if (have) { return vec4<f32>(mix(behind, col, edge), 1.0); }
  return vec4<f32>(col, alpha * edge);
}

struct UnderOut { @builtin(position) clip: vec4<f32>, @location(0) ndc: vec2<f32> };

@vertex fn vs_under(@builtin(vertex_index) vi: u32) -> UnderOut {
  let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u)) * 2.0 - 1.0;
  var o: UnderOut;
  o.clip = vec4<f32>(p, 0.0, 1.0);
  o.ndc = p;
  return o;
}

// With the eye under water: everything fades into the water's colour.
@fragment fn fs_under(i: UnderOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(i.ndc.x * 0.5 + 0.5, 0.5 - i.ndc.y * 0.5);
  let z = depth_at(uv);
  let at = world_at(uv, min(z, 0.99999));
  let dir = normalize(at - u.cam.xyz);
  var dist = distance(at, u.cam.xyz);
  if (z >= 1.0) { dist = 400.0; }
  // only as far as the surface: beyond it is air
  if (dir.y > 0.001) { dist = min(dist, (wu.screen.z - u.cam.y) / dir.y); }
  let K = kind_at(u.cam.xz, 0.0, 0.0);
  let k = 1.0 - exp(-dist * K.fog.w);
  let day = max(dot(u.light.rgb, vec3<f32>(0.3, 0.6, 0.1)), 0.06);
  let c = grade(K.fog.rgb * day);
  return vec4<f32>(c, clamp(0.22 + 0.78 * k, 0.0, 1.0));
}
"#;

// --------------------------------------------------------------- the pass

/// The water pass of the renderer.
pub struct WaterGfx {
    pipe: wgpu::RenderPipeline,
    under_pipe: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    bind: wgpu::BindGroup,
    ubuf: wgpu::Buffer,
    samp: wgpu::Sampler,
    scene: wgpu::Texture,
    format: wgpu::TextureFormat,
    /// the picture of the solid world can be copied (not on every surface)
    can_copy: bool,
    /// 0..2, see the module; `DYNO_WATER_QUALITY` overrides it
    pub quality: u8,
    /// the spawn hearth (x, z) of the world being drawn
    pub origin: [f32; 2],
    /// the height of the surface over the eye, when the eye is under water
    pub sunk: Option<f32>,
}

fn scene_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    w: u32,
    h: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("water scene"),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

impl WaterGfx {
    /// `head` is the head of the world shader; `world` the layout of its
    /// bind group (group 0 here too); `depth` the world's depth buffer.
    pub fn new(
        device: &wgpu::Device,
        head: &str,
        world: &wgpu::BindGroupLayout,
        config: &wgpu::SurfaceConfiguration,
        depth: &wgpu::TextureView,
    ) -> WaterGfx {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water"),
            source: wgpu::ShaderSource::Wgsl([head, SHADER].concat().into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                entry(
                    3,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
            ],
        });
        let pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water"),
            bind_group_layouts: &[world, &layout],
            push_constant_ranges: &[],
        });
        let vlayout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32, 4 => Float32, 5 => Float32x4],
        };
        let make = |vs: &str, fs: &str, buffers: &[wgpu::VertexBufferLayout]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&pipe_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                // the shader tests against the world's depth itself, which
                // it reads anyway
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        };
        let pipe = make("vs_water", "fs_water", std::slice::from_ref(&vlayout));
        let under_pipe = make("vs_under", "fs_under", &[]);
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water"),
            size: std::mem::size_of::<WaterUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let samp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("water scene"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let scene = scene_texture(device, config.format, config.width, config.height);
        let bind = Self::bind(device, &layout, &ubuf, &scene, &samp, depth);
        WaterGfx {
            pipe,
            under_pipe,
            layout,
            bind,
            ubuf,
            samp,
            scene,
            format: config.format,
            can_copy: config.usage.contains(wgpu::TextureUsages::COPY_SRC),
            quality: 2,
            origin: [0.0; 2],
            sunk: None,
        }
    }

    fn bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        ubuf: &wgpu::Buffer,
        scene: &wgpu::Texture,
        samp: &wgpu::Sampler,
        depth: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &scene.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(samp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
            ],
        })
    }

    /// The picture changed size: `depth` is the new depth buffer.
    pub fn resize(&mut self, device: &wgpu::Device, depth: &wgpu::TextureView, w: u32, h: u32) {
        self.scene = scene_texture(device, self.format, w, h);
        self.bind = Self::bind(
            device,
            &self.layout,
            &self.ubuf,
            &self.scene,
            &self.samp,
            depth,
        );
    }

    /// The quality in force: what was asked for, as far as this surface allows.
    pub fn quality(&self) -> u8 {
        let q = quality_override().unwrap_or(self.quality).min(2);
        if self.can_copy { q } else { 0 }
    }

    /// Where the eye is, in a world whose hearth is at `hearth`.
    pub fn look_from(&mut self, world: &World, eye: [f32; 3], hearth: [i32; 3]) {
        self.origin = [hearth[0] as f32, hearth[2] as f32];
        self.sunk = sunk(world, eye);
    }

    /// Start the water pass over the solid world drawn in `target`: the
    /// caller draws the translucent part of each section's mesh, then
    /// calls `finish`.
    pub fn begin<'a>(
        &self,
        queue: &wgpu::Queue,
        enc: &'a mut wgpu::CommandEncoder,
        look: &look::Water,
        target: &wgpu::Texture,
        view: &wgpu::TextureView,
        world: &wgpu::BindGroup,
    ) -> wgpu::RenderPass<'a> {
        let size = [self.scene.width(), self.scene.height()];
        let quality = self.quality();
        let u = uniform(
            look,
            &View {
                origin: self.origin,
                quality,
                sunk: self.sunk,
                size,
            },
        );
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        if quality > 0 {
            enc.copy_texture_to_texture(
                target.as_image_copy(),
                self.scene.as_image_copy(),
                wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
            );
        }
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("water"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rp.set_bind_group(0, world, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.set_pipeline(&self.pipe);
        rp
    }

    /// End the water pass: with the eye under water, the fog of it.
    pub fn finish(&self, rp: &mut wgpu::RenderPass) {
        if self.sunk.is_some() {
            rp.set_pipeline(&self.under_pipe);
            rp.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> look::Water {
        look::RenderSettings::parse(
            r#"{"water": {
                "deep": [0.1, 0.2, 0.3], "clarity": 2.0, "flow": 0.2,
                "kinds": {
                    "brook": {"flow": 1.5, "foam_width": 0.4},
                    "bog": {"deep": [0.0, 0.1, 0.0]}
                },
                "areas": [{"kind": "bog", "at": [10, -20], "radius": [8, 4]}],
                "courses": [
                    {"kind": "brook", "points": [[0, 0, 2], [0, 10, 3], [5, 20, 3]]},
                    {"kind": "nowhere", "points": [[1, 1, 1]]}
                ]
            }}"#,
        )
        .unwrap()
        .water
    }

    #[test]
    fn kinds_change_what_they_name_and_keep_the_rest() {
        let u = uniform(
            &look(),
            &View {
                origin: [100.0, 200.0],
                quality: 2,
                sunk: Some(31.5),
                size: [640, 360],
            },
        );
        // the kinds in name order after the base: bog, brook
        assert_eq!(u.kinds[0].deep, [0.1, 0.2, 0.3, 3.0]);
        assert_eq!(u.kinds[0].life[0], 0.0);
        assert_eq!(u.kinds[1].deep, [0.0, 0.1, 0.0, 3.0]);
        assert_eq!(u.kinds[1].shallow[3], 2.0);
        assert_eq!(u.kinds[2].surf[3], 1.5);
        assert_eq!(u.kinds[2].foam[3], 0.4);
        assert_eq!(u.kinds[2].deep, u.kinds[0].deep);
        // kinds not named are the base
        assert_eq!(u.kinds[7], u.kinds[0]);
        // the area, placed from the hearth
        assert_eq!(u.areas[0], [110.0, 180.0, 1.0 / 8.0, 1.0 / 4.0]);
        assert_eq!(u.area_kind[0][..2], [1.0, 0.25]);
        // the course: its kind + 1, negative on the last point; a course of
        // one point is left out
        assert_eq!(u.course[0], [100.0, 200.0, 2.0, 3.0]);
        assert_eq!(u.course[2], [105.0, 220.0, 3.0, -3.0]);
        assert_eq!(u.counts, [1.0, 3.0, 2.0, 1.0]);
        assert_eq!(u.screen, [640.0, 360.0, 31.5, 0.0]);
    }

    #[test]
    fn the_uniform_is_laid_out_as_the_shader_has_it() {
        assert_eq!(std::mem::size_of::<Kind>(), 96);
        assert_eq!(
            std::mem::size_of::<WaterUniform>(),
            96 * 8 + 16 * 8 + 16 * 8 + 16 * 32 + 64
        );
    }

    /// A pool on a shelf beside a lower pool, with a stone sill between.
    fn stepped() -> World {
        let mut world = World::empty([48, 32, 48]);
        for z in 0..48 {
            for x in 0..48 {
                for y in 0..8 {
                    world.set(x, y, z, w::STONE);
                }
                if x < 24 {
                    world.set(x, 8, z, w::STONE);
                    world.set(x, 9, z, w::WATER);
                    world.set(x, 10, z, w::WATER);
                } else if x == 24 {
                    for y in 8..=10 {
                        world.set(x, y, z, w::STONE);
                    }
                } else {
                    world.set(x, 8, z, w::WATER);
                }
            }
        }
        world
    }

    fn water_of(world: &World, s: [i32; 3]) -> Vec<Vertex> {
        let light = crate::light::Light::new(world);
        crate::terrain_mesh::mesh(world, &light, s[0], s[1], s[2])
            .trans
            .into_iter()
            .filter(|v| v.tex[2] >= KIND_WATER)
            .collect()
    }

    #[test]
    fn the_mesh_knows_depth_steps_and_falls() {
        let world = stepped();
        let vs = water_of(&world, [1, 0, 1]);
        assert!(!vs.is_empty());
        // far from the step, on the shelf: two blocks deep, calm
        let calm = vs
            .iter()
            .find(|v| v.pos == [18.0, 11.0 - WATER_DROP, 20.0])
            .expect("a corner on the shelf");
        assert_eq!(calm.normal[1], 2.0 / DEEP as f32);
        assert_eq!(calm.normal[2], 0.0);
        // under the sill, in the lower pool: white, and running away from it
        let below = vs
            .iter()
            .find(|v| v.pos == [25.0, 9.0 - WATER_DROP, 20.0] && v.color[2] == 0.0)
            .expect("a corner under the sill");
        assert!(below.normal[2] > 0.5, "{:?}", below.normal);
        assert!(below.color[0] > 0.6, "{:?}", below.color);
        // water runs over the sill and falls to the lower pool
        let sheet: Vec<&Vertex> = vs.iter().filter(|v| v.color[2] == 1.0).collect();
        assert!(!sheet.is_empty());
        let (lo, hi) = sheet.iter().fold((f32::MAX, f32::MIN), |a, v| {
            (a.0.min(v.pos[1]), a.1.max(v.pos[1]))
        });
        assert!(hi > 11.0 && lo < 9.0 - WATER_DROP, "{lo} {hi}");
        assert!(sheet.iter().all(|v| v.pos[0] > 25.0));
    }

    #[test]
    fn an_eye_under_the_surface_is_sunk() {
        let world = stepped();
        assert_eq!(sunk(&world, [10.5, 9.5, 10.5]), Some(11.0 - WATER_DROP));
        assert_eq!(sunk(&world, [10.5, 10.9, 10.5]), None);
        assert_eq!(sunk(&world, [10.5, 12.0, 10.5]), None);
    }
}
