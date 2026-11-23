//! wgpu renderer: smooth ground and leaf canopies over the block grid, cubes
//! for built blocks, water, ground clutter, sun shadows and fog, plus a block
//! highlight and boxes for everything that moves.

use crate::light::Light;
use crate::terrain_mesh::{self, Glow, SectionGeo, Tuft};
use bytemuck::{Pod, Zeroable};
use dyno_world::{self as w, BlockId, World};
use egui_wgpu::wgpu;
use glam::{Mat4, Vec3};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub const SECTION: i32 = 16;
pub const SKY: [f32; 3] = [0.62, 0.78, 0.96];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    /// rgb and a brightness multiplier (ambient occlusion and face shade)
    pub color: [f32; 4],
    /// block light 0..1
    pub light: f32,
    /// sky light 0..1
    pub sky: f32,
    /// texture: u, v, layer (negative: untextured, use `color`), and
    /// animation (frame count + seconds per frame / 100)
    pub tex: [f32; 4],
}

/// No texture: the vertex colour is the surface colour.
pub const NO_TEX: [f32; 4] = [0.0, 0.0, -1.0, 0.0];

// What a vertex is, told by `tex[2]`. From 0 up it is a texture layer plus
// one of these bands; below 0 it is untextured.
/// Smooth ground: textured by world position. `tex[1]` is the layer for steep
/// faces; `tex[0]`, `color[1]` and `color[0]` are a neighbouring ground's
/// layers and how much of it shows; `color[2]` is 1 to keep the tile crisp.
pub const KIND_TERRAIN: f32 = 4096.0;
/// Leaves: textured by world position, alpha-tested, swaying in the wind.
pub const KIND_FOLIAGE: f32 = 8192.0;
/// The water surface: waves, shore foam (`normal[0]` is how near the shore is).
pub const KIND_WATER: f32 = 12288.0;
/// Ground clutter: `normal` is the offset from the root, `tex[0]` the sway.
const KIND_CLUTTER: f32 = -2.0;
/// A decal just above the ground: drawn a little in front of what it lies on.
const KIND_DECAL: f32 = -3.0;
/// A halo: a camera-facing square, added to the picture.
const KIND_GLOW: f32 = -4.0;

/// The block textures from the pack.
pub fn textures() -> &'static crate::textures::Textures {
    crate::textures::current()
}

/// How a layer animates, as the shader wants it: frame count plus seconds
/// per frame / 100. Zero for a still layer.
pub(crate) fn layer_anim(layer: u16) -> f32 {
    textures()
        .animated
        .iter()
        .find(|(first, _, _)| *first == layer)
        .map(|(_, n, spf)| *n as f32 + (spf / 100.0).min(0.99))
        .unwrap_or(0.0)
}

/// A texture coordinate across a whole tile, kept half a texel in from the
/// edge: with smooth filtering and repeating tiles, the very edge would pull
/// in colour from the far side.
pub(crate) fn tile_uv(t: f32) -> f32 {
    let inset = 0.5 / crate::textures::TILE as f32;
    inset + (1.0 - 2.0 * inset) * t
}

/// Texture coordinates for one face corner, plus the layer and animation.
pub(crate) fn face_tex(b: BlockId, face: usize, u: f32, v: f32) -> [f32; 4] {
    let layer = textures().face(b, face);
    [u, v, layer as f32, layer_anim(layer)]
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    cam: [f32; 4],
    fog: [f32; 4],
    sky_top: [f32; 4],
    light: [f32; 4],
    sun: [f32; 4],
    /// world to sun-space (shadow map) transform
    shadow_vp: [[f32; 4]; 4],
}

/// Sky and light state for one frame. Colours are sRGB.
#[derive(Clone, Copy, Debug)]
pub struct Sky {
    pub top: [f32; 3],
    pub horizon: [f32; 3],
    /// colour and strength of daylight or moonlight on blocks
    pub light: [f32; 3],
    /// direction towards the sun
    pub sun_dir: [f32; 3],
    /// 0 by day, 1 at night
    pub stars: f32,
    /// 0 clear sky .. 1 overcast
    pub cover: f32,
}

/// Depth-only pass from the sun.
const SHADOW_SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, inv_view_proj: mat4x4<f32>, cam: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, light: vec4<f32>, sun: vec4<f32>, shadow_vp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@vertex fn vs(@location(0) pos: vec3<f32>, @location(5) tex: vec4<f32>) -> @builtin(position) vec4<f32> {
  // decals and halos cast no shadow
  if (tex.z < -2.5) { return vec4<f32>(2.0, 2.0, 2.0, 1.0); }
  return u.shadow_vp * vec4<f32>(pos, 1.0);
}
"#;

/// Size of the shadow map, and how far around the camera it reaches.
const SHADOW_SIZE: u32 = 2048;
const SHADOW_HALF: f32 = 72.0;

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, inv_view_proj: mat4x4<f32>, cam: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, light: vec4<f32>, sun: vec4<f32>, shadow_vp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: U;

@group(0) @binding(1) var tex_array: texture_2d_array<f32>;
@group(0) @binding(2) var tex_samp: sampler;
@group(0) @binding(3) var shadow_map: texture_depth_2d;
@group(0) @binding(4) var shadow_samp: sampler_comparison;
// the render settings from the pack (look.rs)
struct Look { leaf_under: vec4<f32>, leaf_over: vec4<f32>, tone_low: vec4<f32>, tone_high: vec4<f32>, grade_tint: vec4<f32>, grade: vec4<f32>, water_deep: vec4<f32>, water_shallow: vec4<f32>, foam: vec4<f32>, misc: vec4<f32> };
@group(0) @binding(5) var<uniform> look: Look;

// What a vertex is, from tex.z (see the KIND constants in render.rs):
//   below 0: untextured (-1 plain, -2 ground clutter, -3 ground decal, -4 halo)
//   0..: textured with its own UVs
//   4096..: smooth ground, textured by world position
//   8192..: foliage, textured by world position, swaying
//   12288..: water surface
const K_TERRAIN: f32 = 4096.0;
const K_FOLIAGE: f32 = 8192.0;
const K_WATER: f32 = 12288.0;

// 1 in sunlight, 0 in shadow; soft-edged. Four taps: the sampler compares
// and blends 2 x 2 texels at each, so together they cover the same patch a
// 3 x 3 loop of taps did, for less than half the cost.
fn sun_visible(wpos: vec3<f32>, n: vec3<f32>) -> f32 {
  let p = u.shadow_vp * vec4<f32>(wpos + n * 0.04, 1.0);
  let ndc = p.xyz / p.w;
  let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0) { return 1.0; }
  let o = 0.9 / 2048.0;
  let z = ndc.z - 0.0015;
  return 0.25 * (textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(-o, -o), z)
    + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(o, -o), z)
    + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(-o, o), z)
    + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(o, o), z));
}

// The colour grade: tint, saturation, contrast round mid grey, and a soft
// roll-off of highlights instead of clipping.
fn grade(c0: vec3<f32>) -> vec3<f32> {
  var c = c0 * look.grade_tint.rgb;
  let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
  c = mix(vec3<f32>(l), c, look.grade_tint.w);
  c = (c - 0.18) * look.grade.x + 0.18;
  let knee = vec3<f32>(look.grade.y);
  let k = max(c - knee, vec3<f32>(0.0));
  c = min(c, knee) + k / (vec3<f32>(1.0) + k * look.grade.z);
  return max(c, vec3<f32>(0.0));
}

// Integer hashes of whole-number points. (A sin-based hash loses
// precision far from zero and breaks the clouds into hard shards.)
fn mixu(h0: u32) -> u32 {
  var h = h0;
  h = (h ^ (h >> 16u)) * 0x7feb352du;
  h = (h ^ (h >> 15u)) * 0x846ca68bu;
  return h ^ (h >> 16u);
}
fn hash3(p: vec3<f32>) -> f32 {
  let q = bitcast<vec3<u32>>(vec3<i32>(p));
  return f32(mixu(q.x * 0x9E3779B1u ^ mixu(q.y * 0x85EBCA77u ^ mixu(q.z))) >> 8u) / 16777216.0;
}
fn hash2(p: vec2<f32>) -> f32 {
  let q = bitcast<vec2<u32>>(vec2<i32>(p));
  return f32(mixu(q.x * 0x9E3779B1u ^ mixu(q.y)) >> 8u) / 16777216.0;
}
fn vnoise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let s = f * f * (3.0 - 2.0 * f);
  let a = hash2(i);
  let b = hash2(i + vec2<f32>(1.0, 0.0));
  let c = hash2(i + vec2<f32>(0.0, 1.0));
  let d = hash2(i + vec2<f32>(1.0, 1.0));
  return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

fn world_tone(p: vec2<f32>) -> f32 {
  return vnoise(p * 0.06) * 0.6 + vnoise(p * 0.23 + vec2<f32>(13.7, 5.1)) * 0.4;
}

// Height of the water's swell at a place and time.
fn swell(p: vec2<f32>, t: f32) -> f32 {
  return sin(p.x * 0.9 + t * 1.3) * 0.03 + sin(p.y * 1.3 - t * 1.1) * 0.025 + sin((p.x + p.y) * 0.45 + t * 0.7) * 0.03;
}

struct VIn { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32>, @location(3) light: f32, @location(4) sky: f32, @location(5) tex: vec4<f32> };
struct VOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) wpos: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) color: vec4<f32>,
  @location(3) light: f32,
  @location(4) sky: f32,
  @location(5) tex: vec4<f32>,
  // slow noise over the world, worked out per vertex: tone of ground and leaves
  @location(6) tone: f32,
};

@vertex fn vs(v: VIn) -> VOut {
  var o: VOut;
  var pos = v.pos;
  var normal = v.normal;
  var tex = v.tex;
  // how far to pull the depth towards the camera (decals and halos)
  var pull = 0.0;
  var tone = 0.5;
  let t = u.cam.w;
  let kz = v.tex.z;
  if (kz >= K_WATER) {
    pos.y = pos.y + swell(pos.xz, t);
  } else if (kz >= K_FOLIAGE) {
    tone = vnoise(pos.xz * 0.35 + pos.y * 0.21);
    // leaves stir in the wind
    let ph = pos.x * 0.37 + pos.z * 0.29 + pos.y * 0.23;
    pos.x = pos.x + sin(t * 1.2 + ph) * 0.045 + sin(t * 2.7 + ph * 2.3) * 0.015;
    pos.z = pos.z + cos(t * 1.0 + ph * 1.3) * 0.045;
    pos.y = pos.y + sin(t * 1.6 + ph * 1.7) * 0.02;
  } else if (kz < -1.5 && kz > -2.5) {
    // ground clutter: `normal` is the offset from the root, tex.x how much
    // the vertex sways, tex.y its phase; it shrinks away with distance
    let root = pos - v.normal;
    let fade = 1.0 - smoothstep(30.0, 42.0, distance(root, u.cam.xyz));
    let s = v.tex.x * (sin(t * 1.7 + v.tex.y + root.x * 0.6) * 0.07 + sin(t * 3.1 + v.tex.y * 1.7 + root.z * 0.8) * 0.03);
    pos = root + (v.normal + vec3<f32>(s, 0.0, s * 0.6)) * fade;
    normal = vec3<f32>(0.0, 1.0, 0.0);
    tone = world_tone(root.xz);
  } else if (kz >= K_TERRAIN) {
    tone = world_tone(pos.xz);
  } else if (kz >= 0.0) {
    // built blocks: the same slow drift of tone, so a wall is not one tile
    // stamped over and over
    tone = world_tone(pos.xz * 1.7 + vec2<f32>(pos.y * 0.9, -pos.y * 0.6));
  } else if (kz < -2.5 && kz > -3.5) {
    pull = 0.35;
  } else if (kz < -3.5) {
    // a halo: a square that faces the camera; `normal` is corner x, y and size
    let f = normalize(pos - u.cam.xyz);
    let r = normalize(cross(f, vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(0.0001, 0.0, 0.0));
    let up = cross(r, f);
    pos = pos + (r * v.normal.x + up * v.normal.y) * v.normal.z;
    tex = vec4<f32>(v.normal.xy, kz, 0.0);
    normal = vec3<f32>(0.0);
    pull = 0.95;
  }
  o.clip = u.view_proj * vec4<f32>(pos, 1.0);
  if (pull > 0.0) {
    let nearer = u.view_proj * vec4<f32>(pos + normalize(u.cam.xyz - pos) * pull, 1.0);
    o.clip.z = nearer.z / nearer.w * o.clip.w;
  }
  o.wpos = pos;
  o.normal = normal;
  o.color = v.color;
  o.light = v.light;
  o.sky = v.sky;
  o.tex = tex;
  o.tone = tone;
  return o;
}

// The layer to show now: animated layers step through their frames.
fn anim_layer(layer: f32, anim: f32) -> i32 {
  var l = layer;
  let frames = floor(anim);
  if (frames > 1.0) {
    let spf = max(fract(anim) * 100.0, 0.02);
    l = l + (floor(u.cam.w / spf) % frames);
  }
  return i32(l + 0.5);
}

// How much each of the three flat projections shows for a normal: only the
// one or two faces the surface leans to.
fn tri_weights(n: vec3<f32>) -> vec3<f32> {
  let w = max(abs(n) - vec3<f32>(0.5), vec3<f32>(0.0)) + vec3<f32>(0.00001);
  return w / (w.x + w.y + w.z);
}

// A layer laid over the world from the side (x) or from the front (z).
// Gradients are passed in so this may sit inside branches.
fn side_x(layer: i32, p: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> vec4<f32> {
  return textureSampleGrad(tex_array, tex_samp, vec2<f32>(p.z, -p.y), layer, vec2<f32>(dx.z, -dx.y), vec2<f32>(dy.z, -dy.y));
}
fn side_z(layer: i32, p: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> vec4<f32> {
  return textureSampleGrad(tex_array, tex_samp, vec2<f32>(p.x, -p.y), layer, vec2<f32>(dx.x, -dx.y), vec2<f32>(dy.x, -dy.y));
}

// A ground tile, and a larger, turned copy of it under that: broad strokes
// that hide the repeat.
fn broad(layer: i32, uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>) -> vec3<f32> {
  let m = mat2x2<f32>(0.27, 0.09, -0.09, 0.27);
  // soft and magnified, so a plain mip level will do (cheaper than gradients);
  // 9.1 is 32 texels across a tile, times the copy's scale of 0.285
  let lod = log2(max(max(length(dx), length(dy)) * 9.1, 1.0));
  return textureSampleLevel(tex_array, tex_samp, m * uv, layer, lod).rgb;
}
fn paint(layer: i32, uv: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, soft: f32) -> vec3<f32> {
  var c = textureSampleGrad(tex_array, tex_samp, uv, layer, dx, dy).rgb;
  if (soft > 0.001) { c = mix(c, broad(layer, uv, dx, dy), soft); }
  return c;
}

// One kind of ground: its top layer where it lies level, its steep layer on
// slopes, walls and ceilings. `near` is the top layer seen from above, if
// the caller has sampled it already (`have`). `soft` is how much of the
// broad copy goes in: none for ore and the like, whose detail matters.
fn ground(top: i32, steep: i32, level0: f32, w3: vec3<f32>, p: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>, near: vec3<f32>, have: bool, soft: f32) -> vec3<f32> {
  var level = level0;
  if (top == steep) { level = 1.0; }
  var c = vec3<f32>(0.0);
  if (w3.y > 0.001) {
    var cy = vec3<f32>(0.0);
    if (level > 0.001) {
      if (!have) {
        cy += level * paint(top, p.xz, dx.xz, dy.xz, soft);
      } else if (soft > 0.001) {
        cy += level * mix(near, broad(top, p.xz, dx.xz, dy.xz), soft);
      } else {
        cy += level * near;
      }
    }
    if (level < 0.999) { cy += (1.0 - level) * paint(steep, p.xz, dx.xz, dy.xz, soft); }
    c += w3.y * cy;
  }
  if (w3.x > 0.001) { c += w3.x * paint(steep, vec2<f32>(p.z, -p.y), vec2<f32>(dx.z, -dx.y), vec2<f32>(dy.z, -dy.y), soft); }
  if (w3.z > 0.001) { c += w3.z * paint(steep, vec2<f32>(p.x, -p.y), vec2<f32>(dx.x, -dx.y), vec2<f32>(dy.x, -dy.y), soft); }
  return c;
}

// Broad, slow changes of tone, so the ground is not one repeated tile.
// The amount comes from the vertex shader.
fn ground_tone(m: f32) -> vec3<f32> {
  return mix(look.tone_low.rgb, look.tone_high.rgb, m);
}

@fragment fn fs(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
  let dpx = dpdx(i.wpos);
  let dpy = dpdy(i.wpos);
  let kz = i.tex.z;
  // leaf texture repeats per metre
  let leaf_scale = look.misc.x;
  // One plain sample for every pixel, taken before any branch, so the
  // hardware finds the mip level itself (the cheap path): the face's own
  // UVs, or for ground and leaves the view from above.
  var uv0 = i.tex.xy;
  var l0 = 0;
  if (kz >= K_FOLIAGE) {
    // leaves: larger painted clumps than one tile a block
    uv0 = i.wpos.xz * leaf_scale;
    l0 = anim_layer(kz - K_FOLIAGE, i.tex.w);
  } else if (kz >= K_TERRAIN) {
    uv0 = i.wpos.xz;
    l0 = anim_layer(kz - K_TERRAIN, i.tex.w);
  } else if (kz >= 0.0) {
    l0 = anim_layer(kz, i.tex.w);
  }
  let tx0 = textureSample(tex_array, tex_samp, uv0, l0);
  let d = distance(i.wpos, u.cam.xyz);
  let has_n = dot(i.normal, i.normal) > 0.25;
  var n = vec3<f32>(0.0, 1.0, 0.0);
  if (has_n) { n = normalize(i.normal); }
  // painted relief: the texture's brightness taken as height (dark mortar
  // and cracks sink, stones stand out), turned into a bent normal from
  // screen-space slopes; fades out with distance, where it would shimmer
  let hgt = dot(tx0.rgb, vec3<f32>(0.3, 0.55, 0.15));
  let dhx = dpdx(hgt);
  let dhy = dpdy(hgt);
  var bent = n;
  if (has_n && kz >= 0.0 && kz < K_FOLIAGE && d < 40.0) {
    let r1 = cross(dpy, n);
    let r2 = cross(n, dpx);
    let det = dot(dpx, r1);
    let g = sign(det) * (dhx * r1 + dhy * r2);
    // smooth ground: only on its level tops, which show the texture seen from above
    let k = look.grade.w * (1.0 - smoothstep(18.0, 40.0, d)) * select(1.0, smoothstep(0.6, 0.9, n.y), kz >= K_TERRAIN);
    bent = normalize(abs(det) * n - g * k);
  }
  var base = i.color.rgb;
  var grain = 1.0;
  // 0: hard sunlight; towards 1: light wraps round the form (foliage)
  var wrap = 0.0;
  var dim = 1.0;
  if (kz >= K_FOLIAGE) {
    let w3 = tri_weights(n);
    var tx = w3.y * tx0;
    if (w3.x > 0.001) { tx += w3.x * side_x(l0, i.wpos * leaf_scale, dpx * leaf_scale, dpy * leaf_scale); }
    if (w3.z > 0.001) { tx += w3.z * side_z(l0, i.wpos * leaf_scale, dpx * leaf_scale, dpy * leaf_scale); }
    // gaps between the leaves close up with distance, where they would only flicker
    let cut = mix(0.5, 0.06, smoothstep(18.0, 44.0, d));
    if (tx.a < cut) { discard; }
    // big soft patches of light and dark over the canopy
    // big soft patches of light and dark; cool, deep shade underneath and
    // warm sunlit crowns, so each canopy reads as one painted form
    base = base * tx.rgb * (look.leaf_under.w + look.leaf_over.w * i.tone);
    base = base * mix(look.leaf_under.rgb, look.leaf_over.rgb, smoothstep(-0.4, 0.85, n.y));
    if (!front) { n = -n; dim = 0.72; }
    // darker underneath and inside
    dim = dim * (0.74 + 0.26 * smoothstep(-0.6, 0.5, n.y));
    wrap = 0.55;
  } else if (kz >= K_TERRAIN) {
    let w3 = tri_weights(n);
    // the top layer on level ground, giving way to the steep layer on
    // slopes, along a ragged line
    var level = smoothstep(0.40, 0.62, n.y);
    if (n.y > 0.3 && n.y < 0.72) {
      level = smoothstep(0.40, 0.62, n.y + (vnoise(i.wpos.xz * 1.3 + i.wpos.y * 0.7) - 0.5) * 0.16);
    }
    var lc = i32(i.tex.y + 0.5);
    // a block with one animated texture all round: its steep faces move too
    if (lc == i32(kz - K_TERRAIN + 0.5)) { lc = l0; }
    // color.b: 1 on ground whose own detail must stay crisp (ore, emberstone)
    var c = ground(l0, lc, level, w3, i.wpos, dpx, dpy, tx0.rgb, true, 0.42 * (1.0 - i.color.b));
    // fade into the neighbouring kind of ground along a ragged, painted edge
    let wb = i.color.r;
    if (wb > 0.002) {
      let nb = vnoise(i.wpos.xz * 2.1 + i.wpos.y * 1.3) * 0.6 + vnoise(i.wpos.xz * 0.63 + vec2<f32>(3.1, 7.7)) * 0.4;
      // both sides of the edge must draw the same line: one adds the noise, the other takes it away
      let s = select(-1.0, 1.0, kz - K_TERRAIN < i.tex.x);
      let k = smoothstep(0.38, 0.62, wb + s * (nb - 0.5) * 0.5);
      if (k > 0.001) {
        c = mix(c, ground(i32(i.tex.x + 0.5), i32(i.color.g + 0.5), level, w3, i.wpos, dpx, dpy, c, false, 0.42), k);
      }
    }
    base = c * ground_tone(i.tone);
  } else if (kz < -1.5 && kz > -2.5) {
    base = base * ground_tone(i.tone);
  } else if (kz >= 0.0) {
    if (tx0.a < 0.5) { discard; }
    base = base * tx0.rgb * mix(look.tone_low.w, look.tone_high.w, i.tone);
  } else if (kz > -1.5) {
    // 16 x 16 texel grain on untextured boxes so they read as pixel art
    let t = floor(i.wpos * 16.0 - i.normal * 0.01);
    grain = select(1.0, 0.86 + 0.24 * hash3(t), has_n);
  }
  // sun or moon light (with shadows and the angle of the surface), soft sky
  // light from above, or warm block light, whichever is brighter
  let bl = i.light * i.light;
  let sl = i.sky * i.sky;
  let ldir = normalize(select(-u.sun.xyz, u.sun.xyz, u.sun.y > 0.0));
  let lam = dot(select(n, bent, kz >= 0.0 && kz < K_FOLIAGE), ldir);
  let ndl = select(0.75, max(mix(lam, lam * 0.5 + 0.5, wrap), 0.0), has_n);
  let vis = sun_visible(i.wpos, n);
  let up = select(0.75, 0.6 + 0.4 * n.y, has_n);
  // cool, blue-violet shade; warm direct sun
  let sky_amb = mix(u.sky_top.rgb, vec3<f32>(0.45, 0.5, 0.75), 0.5) * 0.42 * up;
  let direct = u.light.rgb * vec3<f32>(1.08, 1.0, 0.9) * (0.2 + 1.0 * ndl * vis);
  let daylight = (direct + sky_amb) * (sl * 0.97 + 0.03);
  let lit = max(daylight, vec3<f32>(1.0, 0.78, 0.5) * bl * 1.15);
  var c = base * grain * min(i.color.a, 1.0) * lit * dim;
  if (i.color.a > 1.9) { c = base * grain * 1.1; }
  let f = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  // height haze in low valleys, then distance fog
  let haze = clamp((26.0 - i.wpos.y) / 26.0, 0.0, 1.0) * clamp(d / 110.0, 0.0, 1.0) * look.misc.y;
  c = mix(c, u.fog.rgb, max(f, haze));
  return vec4<f32>(grade(c), 1.0);
}

struct SkyOut { @builtin(position) clip: vec4<f32>, @location(0) ndc: vec2<f32> };

@vertex fn vs_sky(@builtin(vertex_index) vi: u32) -> SkyOut {
  let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u)) * 2.0 - 1.0;
  var o: SkyOut;
  o.clip = vec4<f32>(p, 1.0, 1.0);
  o.ndc = p;
  return o;
}

@fragment fn fs_sky(i: SkyOut) -> @location(0) vec4<f32> {
  let far = u.inv_view_proj * vec4<f32>(i.ndc, 1.0, 1.0);
  let dir = normalize(far.xyz / far.w - u.cam.xyz);
  let h = clamp(dir.y, -1.0, 1.0);
  var c = mix(u.fog.rgb, u.sky_top.rgb, smoothstep(-0.02, 0.55, h));
  c = mix(c, u.fog.rgb * 0.6, smoothstep(0.0, -0.4, h));
  let sd = dot(dir, u.sun.xyz);
  // sun glow and disc
  c += vec3<f32>(1.0, 0.75, 0.45) * pow(max(sd, 0.0), 48.0) * 0.6 * (1.0 - u.sun.w * 0.8);
  if (sd > 0.9992) { c = mix(c, vec3<f32>(1.0, 0.97, 0.88), 0.95); }
  // moon opposite the sun
  if (-sd > 0.9994) { c = mix(c, vec3<f32>(0.85, 0.88, 0.95), u.sun.w); }
  // painted clouds: soft billows lit from the sun side, with warm rims
  if (h > 0.01) {
    let q = dir.xz / (h + 0.08) * 1.1 + vec2<f32>(u.cam.w * 0.010, u.cam.w * 0.004);
    let n = vnoise(q * 0.8) * 0.5 + vnoise(q * 1.9 + 7.3) * 0.3 + vnoise(q * 4.3 + 2.1) * 0.2;
    let over = u.sky_top.w;
    let cov = smoothstep(0.46 - over * 0.5, 0.78 - over * 0.45, n) * smoothstep(0.01, 0.25, h);
    // brighter where the cloud thins and faces the sun
    let edge = 1.0 - smoothstep(0.55, 0.8, n);
    let toward = max(dot(normalize(vec3<f32>(dir.x, 0.0, dir.z) + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(u.sun.x, 0.0, u.sun.z) + vec3<f32>(0.0, 0.001, 0.0))), 0.0);
    let body = mix(u.fog.rgb * 0.9, vec3<f32>(1.0, 0.98, 0.95), 0.6 - over * 0.45) * (u.light.rgb * 0.8 + 0.2) * (1.0 - over * 0.35);
    let rim = vec3<f32>(1.0, 0.82, 0.55) * (u.light.rgb * 0.9 + 0.1);
    let shade = mix(u.sky_top.rgb * 0.8, body, 0.65);
    var cc = mix(shade, body, smoothstep(0.5, 0.9, n));
    cc = mix(cc, rim, edge * toward * 0.5);
    c = mix(c, cc, cov * 0.92);
  }
  // stars
  if (u.sun.w > 0.01 && h > 0.0) {
    let cell = floor(dir * 220.0);
    let r = hash3(cell);
    if (r > 0.9965) { c += vec3<f32>(0.9, 0.92, 1.0) * u.sun.w * (r - 0.9965) * 220.0 * smoothstep(0.0, 0.2, h); }
  }
  return vec4<f32>(grade(c), 1.0);
}

@fragment fn fs_translucent(i: VOut) -> @location(0) vec4<f32> {
  let kz = i.tex.z;
  var l0 = 0;
  if (kz >= 0.0 && kz < K_TERRAIN) { l0 = anim_layer(kz, i.tex.w); }
  let tx0 = textureSample(tex_array, tex_samp, i.tex.xy, l0);
  let d = distance(i.wpos, u.cam.xyz);
  let f = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  let lit = max(u.light.rgb * (i.sky * i.sky * 0.97 + 0.03), vec3<f32>(1.0, 0.80, 0.55) * i.light * i.light);
  var c = i.color.rgb * lit;
  var a = i.color.a;
  if (kz >= K_WATER) {
    // painted water: deep blue out in the open, clear turquoise in the
    // shallows, slow swells of light, sky at grazing angles, foam at the shore
    let t = u.cam.w;
    let p = i.wpos.xz;
    let shore = clamp(i.normal.x, 0.0, 1.0);
    let shallow = smoothstep(0.0, 0.85, shore);
    var wc = mix(look.water_deep.rgb, look.water_shallow.rgb, shallow);
    let s1 = vnoise(p * 0.55 + vec2<f32>(t * 0.22, t * 0.13));
    let s2 = vnoise(p * 1.7 - vec2<f32>(t * 0.25, t * 0.31));
    // long brush strokes of lighter water, drifting
    let s3 = vnoise(vec2<f32>(p.x * 0.35 + t * 0.05, p.y * 1.9 - t * 0.12));
    wc = wc * (0.82 + 0.3 * (s1 * 0.65 + s2 * 0.35) + 0.22 * smoothstep(0.55, 0.9, s3));
    c = wc * lit;
    a = mix(0.84, 0.46, shallow);
    let v = normalize(u.cam.xyz - i.wpos);
    let fres = pow(1.0 - clamp(v.y, 0.0, 1.0), 4.0);
    c = mix(c, mix(u.sky_top.rgb, u.fog.rgb, 0.35) * (u.light.rgb * 0.5 + 0.5), fres * 0.5);
    a = mix(a, 0.88, fres);
    // the sun glints on the swells
    let wn = normalize(vec3<f32>((s1 - 0.5) * 0.3, 1.0, (s2 - 0.5) * 0.3));
    let sd = max(dot(reflect(-v, wn), u.sun.xyz), 0.0);
    c += vec3<f32>(1.0, 0.85, 0.6) * pow(sd, 90.0) * 0.7 * (1.0 - u.sun.w);
    // foam where the water laps the shore: a ragged, streaky band
    if (shore > 0.15) {
      let fz = vnoise(p * 2.6 + vec2<f32>(t * 0.3, -t * 0.22));
      let lap = 0.05 * sin(t * 1.4 + p.x * 0.8 + p.y * 0.6);
      let band = smoothstep(0.42, 0.66, shore + (fz - 0.5) * 0.38 + lap);
      let streak = 0.5 + 0.5 * smoothstep(0.3, 0.7, vnoise(p * 6.0 - vec2<f32>(t * 0.25, t * 0.4)));
      let foam = band * streak * 0.75;
      c = mix(c, look.foam.rgb * (lit * 0.9 + u.light.rgb * 0.1), foam);
      a = mix(a, 0.92, foam);
    }
  } else if (kz >= 0.0) {
    c = c * tx0.rgb;
    a = tx0.a;
    if (a < 0.02) { discard; }
  }
  c = mix(c, u.fog.rgb, f);
  return vec4<f32>(grade(c), a);
}

// Halos: added on top of the picture, brightest in the dark.
@fragment fn fs_glow(i: VOut) -> @location(0) vec4<f32> {
  let r2 = dot(i.tex.xy, i.tex.xy);
  let core = max(exp(-r2 * 3.5) - 0.03, 0.0);
  let d = distance(i.wpos, u.cam.xyz);
  let fog = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  let day = dot(u.light.rgb, vec3<f32>(0.3, 0.6, 0.1));
  let flicker = 0.92 + 0.08 * sin(u.cam.w * 7.0 + i.wpos.x * 3.1 + i.wpos.z * 1.7);
  let k = core * i.color.a * flicker * (1.0 - fog) * mix(1.0, 0.3, clamp(day * 1.4, 0.0, 1.0));
  return vec4<f32>(i.color.rgb * k, 0.0);
}
"#;

fn srgb(c: [f32; 3]) -> [f32; 3] {
    [c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2)]
}

/// Display colour of a block (sRGB) and its opacity.
pub fn block_color(b: BlockId) -> ([f32; 3], f32) {
    let d = w::block(b);
    (d.color, d.alpha)
}

type Face = ([i32; 3], [i32; 3], [i32; 3], f32);
const FACES: [Face; 6] = [
    // normal, u axis, v axis, face shade
    ([1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
    ([-1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
    ([0, 1, 0], [1, 0, 0], [0, 0, 1], 1.00),
    ([0, -1, 0], [1, 0, 0], [0, 0, 1], 0.55),
    ([0, 0, 1], [1, 0, 0], [0, 1, 0], 0.68),
    ([0, 0, -1], [1, 0, 0], [0, 1, 0], 0.68),
];

/// Build the opaque and translucent meshes of one 16³ section.
pub fn mesh_section(
    world: &World,
    light: &Light,
    sx: i32,
    sy: i32,
    sz: i32,
) -> (Vec<Vertex>, Vec<Vertex>) {
    let geo = terrain_mesh::mesh(world, light, sx, sy, sz);
    (geo.opaque, geo.trans)
}

/// Whether a section of a streamed world can be meshed: it and every
/// neighbour (the 26 round it that are in the world) have arrived.
pub fn section_ready(world: &World, s: [i32; 3]) -> bool {
    let d = world.section_dims();
    if !world.has_section(s) {
        return false;
    }
    for dy in -1..=1 {
        for dz in -1..=1 {
            for dx in -1..=1 {
                let n = [s[0] + dx, s[1] + dy, s[2] + dz];
                if (0..3).all(|i| n[i] >= 0 && n[i] < d[i]) && !world.has_section(n) {
                    return false;
                }
            }
        }
    }
    true
}

/// Mesh many sections at once, on every core.
pub(crate) fn mesh_many(world: &World, light: &Light, list: &[[i32; 3]]) -> Vec<SectionGeo> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(list.len().div_ceil(2))
        .min(16);
    let one = |s: &[i32; 3]| terrain_mesh::mesh(world, light, s[0], s[1], s[2]);
    if threads <= 1 {
        return list.iter().map(one).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let parts: Vec<Vec<(usize, SectionGeo)>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(s) = list.get(i) else { break };
                        done.push((i, one(s)));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|h| h.join().expect("a meshing thread panicked"))
            .collect()
    });
    let mut out: Vec<SectionGeo> = list.iter().map(|_| SectionGeo::default()).collect();
    for (i, geo) in parts.into_iter().flatten() {
        out[i] = geo;
    }
    out
}

/// Add a coloured box to a vertex list (used for players and particles).
pub fn push_box(out: &mut Vec<Vertex>, min: Vec3, max: Vec3, rgb: [f32; 3], light: f32) {
    let c = srgb(rgb);
    for (n, u, v, shade) in FACES {
        let nf = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32);
        let uf = Vec3::new(u[0] as f32, u[1] as f32, u[2] as f32);
        let vf = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5;
        let fc = center + nf * half;
        let hu = uf * half;
        let hv = vf * half;
        let p = [fc - hu - hv, fc + hu - hv, fc + hu + hv, fc - hu + hv];
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(Vertex {
                pos: p[i].into(),
                normal: [0.0; 3],
                color: [c[0], c[1], c[2], shade],
                light,
                sky: 1.0,
                tex: NO_TEX,
            });
        }
    }
}

/// Add a box with its own axes (used for the block in the player's hand).
pub fn push_oriented_box(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axes: [Vec3; 3],
    half: Vec3,
    rgb: [f32; 3],
    emissive: bool,
    light: f32,
) {
    let c = srgb(rgb);
    for (n, u, v, shade) in FACES {
        let pick = |a: [i32; 3]| {
            axes[0] * (a[0] as f32 * half.x)
                + axes[1] * (a[1] as f32 * half.y)
                + axes[2] * (a[2] as f32 * half.z)
        };
        let (nf, uf, vf) = (pick(n), pick(u), pick(v));
        let fc = center + nf;
        let p = [fc - uf - vf, fc + uf - vf, fc + uf + vf, fc + vf - uf];
        let a = if emissive { 2.0 } else { shade };
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(Vertex {
                pos: p[i].into(),
                normal: [0.0; 3],
                color: [c[0], c[1], c[2], a],
                light,
                sky: 1.0,
                tex: NO_TEX,
            });
        }
    }
}

/// A torch-like post: a narrow textured stick, 3/16 wide and 11/16 tall, glowing.
pub(crate) fn push_post(out: &mut Vec<Vertex>, center: Vec3, b: BlockId, light: f32) {
    push_post_oriented(out, center, [Vec3::X, Vec3::Y, Vec3::Z], 1.0, b, light);
}

/// A post with any orientation and size (held and dropped torches).
pub fn push_post_oriented(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axes: [Vec3; 3],
    scale: f32,
    b: BlockId,
    light: f32,
) {
    let half = Vec3::new(1.5 / 16.0, 11.0 / 32.0, 1.5 / 16.0) * scale;
    let (u0, u1) = (6.5 / 16.0, 9.5 / 16.0);
    for (n, u, v, _) in FACES {
        let pick = |a: [i32; 3]| {
            axes[0] * (a[0] as f32 * half.x)
                + axes[1] * (a[1] as f32 * half.y)
                + axes[2] * (a[2] as f32 * half.z)
        };
        let face = match n[1] {
            1 => 0,
            -1 => 2,
            _ => 1,
        };
        let (nf, uf, vf) = (pick(n), pick(u), pick(v));
        let fc = center + nf;
        let cs = [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        for i in [0, 1, 2, 0, 2, 3] {
            let (su, sv) = cs[i];
            let (tu, tv) = if face == 1 {
                (u0 + (u1 - u0) * (su + 1.0) * 0.5, tile_uv((1.0 - sv) * 0.5))
            } else {
                (
                    u0 + (u1 - u0) * (su + 1.0) * 0.5,
                    u0 + (u1 - u0) * (sv + 1.0) * 0.5,
                )
            };
            out.push(Vertex {
                pos: (fc + uf * su + vf * sv).into(),
                normal: [0.0; 3],
                color: [1.0, 1.0, 1.0, 2.0],
                light,
                sky: 1.0,
                tex: face_tex(b, face, tu, tv),
            });
        }
    }
}

/// A small textured block, for dropped and held blocks.
pub fn push_block_cube(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axes: [Vec3; 3],
    half: f32,
    b: BlockId,
    light: f32,
) {
    let glow = w::emission(b) > 0;
    for (n, u, v, shade) in FACES {
        let pick = |a: [i32; 3]| {
            (axes[0] * a[0] as f32 + axes[1] * a[1] as f32 + axes[2] * a[2] as f32) * half
        };
        let face = match n[1] {
            1 => 0,
            -1 => 2,
            _ => 1,
        };
        let (nf, uf, vf) = (pick(n), pick(u), pick(v));
        let fc = center + nf;
        let cs = [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        for i in [0, 1, 2, 0, 2, 3] {
            let (su, sv) = cs[i];
            let (tu, tv) = if face == 1 {
                (tile_uv((su + 1.0) * 0.5), tile_uv((1.0 - sv) * 0.5))
            } else {
                (tile_uv((su + 1.0) * 0.5), tile_uv((sv + 1.0) * 0.5))
            };
            out.push(Vertex {
                pos: (fc + uf * su + vf * sv).into(),
                normal: [0.0; 3],
                color: [1.0, 1.0, 1.0, if glow { 2.0 } else { shade }],
                light,
                sky: 1.0,
                tex: face_tex(b, face, tu, tv),
            });
        }
    }
}

/// Upload every block texture as one array texture, with mipmaps for distance.
fn upload_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    t: &crate::textures::Textures,
) -> wgpu::TextureView {
    use crate::textures::TILE;
    let layers = t.layers.len().max(1) as u32;
    let mips = TILE.trailing_zeros() + 1; // 16 px: 16, 8, 4, 2, 1
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blocks"),
        size: wgpu::Extent3d {
            width: TILE as u32,
            height: TILE as u32,
            depth_or_array_layers: layers,
        },
        mip_level_count: mips,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (li, layer) in t.layers.iter().enumerate() {
        let mut img: Vec<u8> = layer.to_vec();
        bleed(&mut img, TILE);
        let mut size = TILE;
        for mip in 0..mips {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: mip,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: li as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &img,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size as u32 * 4),
                    rows_per_image: Some(size as u32),
                },
                wgpu::Extent3d {
                    width: size as u32,
                    height: size as u32,
                    depth_or_array_layers: 1,
                },
            );
            if size == 1 {
                break;
            }
            // halve: average in linear light, weighting colour by alpha
            let half = size / 2;
            let mut next = vec![0u8; half * half * 4];
            let lin = |c: u8| (c as f32 / 255.0).powf(2.2);
            for y in 0..half {
                for x in 0..half {
                    let mut acc = [0.0f32; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let i = ((y * 2 + dy) * size + x * 2 + dx) * 4;
                        let a = img[i + 3] as f32 / 255.0;
                        for k in 0..3 {
                            acc[k] += lin(img[i + k]) * a;
                        }
                        acc[3] += a;
                    }
                    let o = (y * half + x) * 4;
                    for k in 0..3 {
                        // nothing solid here: keep the colour underneath, so
                        // smooth filtering has no dark fringe to pull in
                        let v = if acc[3] > 0.0 {
                            acc[k] / acc[3]
                        } else {
                            [(0, 0), (1, 0), (0, 1), (1, 1)]
                                .iter()
                                .map(
                                    |(dx, dy)| lin(img[((y * 2 + dy) * size + x * 2 + dx) * 4 + k]),
                                )
                                .sum::<f32>()
                                / 4.0
                        };
                        next[o + k] = (v.powf(1.0 / 2.2) * 255.0).round() as u8;
                    }
                    next[o + 3] = (acc[3] / 4.0 * 255.0).round() as u8;
                }
            }
            img = next;
            size = half;
        }
    }
    tex.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

/// Smooth filtering, so the small painted tiles read as paint, not pixels;
/// anisotropy keeps the ground crisp towards the horizon.
fn texture_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("painted"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        anisotropy_clamp: 4,
        ..Default::default()
    })
}

/// What the main shaders see: the resources behind the bind group.
struct Bindings<'a> {
    layout: &'a wgpu::BindGroupLayout,
    ubuf: &'a wgpu::Buffer,
    tex_view: &'a wgpu::TextureView,
    tex_samp: &'a wgpu::Sampler,
    shadow_view: &'a wgpu::TextureView,
    shadow_samp: &'a wgpu::Sampler,
    look_buf: &'a wgpu::Buffer,
}

fn make_bind(device: &wgpu::Device, b: Bindings) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: b.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: b.ubuf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(b.tex_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(b.tex_samp),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(b.shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(b.shadow_samp),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: b.look_buf.as_entire_binding(),
            },
        ],
    })
}

/// Spread colour into fully transparent texels from their solid neighbours
/// (the tile wraps), so smooth filtering does not darken cut-out edges.
fn bleed(img: &mut [u8], size: usize) {
    let mut known: Vec<bool> = img.as_chunks::<4>().0.iter().map(|p| p[3] > 0).collect();
    if known.iter().all(|k| *k) || !known.iter().any(|k| *k) {
        return;
    }
    for _ in 0..size {
        let mut next = known.clone();
        let mut changed = false;
        for y in 0..size {
            for x in 0..size {
                let i = y * size + x;
                if known[i] {
                    continue;
                }
                let (mut acc, mut n) = ([0u32; 3], 0u32);
                for (dx, dy) in [(1, 0), (size - 1, 0), (0, 1), (0, size - 1)] {
                    let j = ((y + dy) % size) * size + (x + dx) % size;
                    if known[j] {
                        for (a, c) in acc.iter_mut().zip(&img[j * 4..j * 4 + 3]) {
                            *a += *c as u32;
                        }
                        n += 1;
                    }
                }
                for (k, a) in acc.iter().enumerate() {
                    if let Some(mean) = a.checked_div(n) {
                        img[i * 4 + k] = mean as u8;
                        next[i] = true;
                        changed = true;
                    }
                }
            }
        }
        known = next;
        if !changed {
            break;
        }
    }
}

/// Mean colour of a texture layer, in linear light.
fn layer_mean(layer: u16) -> [f32; 3] {
    let t = textures();
    let Some(img) = t.layers.get(layer as usize) else {
        return [0.5; 3];
    };
    let (mut acc, mut n) = ([0f32; 3], 0f32);
    for p in img.as_chunks::<4>().0.iter().filter(|p| p[3] > 0) {
        for (a, c) in acc.iter_mut().zip(p) {
            *a += (*c as f32 / 255.0).powf(2.2);
        }
        n += 1.0;
    }
    acc.map(|a| a / n.max(1.0))
}

/// Draw grass tufts and flowers this far from the camera, in sections.
const CLUTTER_SECTIONS: i32 = 3;

/// Turn one tuft into blades of grass, or a small flower.
fn push_tuft(out: &mut Vec<Vertex>, t: &Tuft, grass: [f32; 3]) {
    let mut h = t.seed;
    let mut rnd = move || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        (h >> 8) as f32 / 16_777_216.0
    };
    let root = Vec3::from(t.pos);
    let tone = 0.85 + 0.3 * rnd();
    let warm = rnd();
    // the blades match the painted grass under them, darker at the foot
    let tint = [
        grass[0] * tone * (1.0 + 0.25 * warm),
        grass[1] * tone,
        grass[2] * tone * (1.0 - 0.3 * warm),
    ];
    let mut vert = |pos: Vec3, off: Vec3, rgb: [f32; 3], sway: f32, phase: f32| {
        out.push(Vertex {
            pos: (pos + off).into(),
            normal: off.into(),
            color: [rgb[0], rgb[1], rgb[2], 1.0],
            light: t.light,
            sky: t.sky,
            tex: [sway, phase, KIND_CLUTTER, 0.0],
        });
    };
    let phase = rnd() * std::f32::consts::TAU;
    let flower = t.seed.is_multiple_of(13);
    let blades = if flower {
        3
    } else {
        5 + (t.seed >> 28) as usize % 4
    };
    let height = 0.7 + 0.6 * rnd();
    for _ in 0..blades {
        let a = rnd() * std::f32::consts::TAU;
        let r = rnd() * 0.3;
        let base = root + Vec3::new(a.cos() * r, -0.04, a.sin() * r);
        let facing = rnd() * std::f32::consts::TAU;
        let side = Vec3::new(facing.cos(), 0.0, facing.sin()) * (0.03 + 0.03 * rnd());
        let lean = Vec3::new(a.cos(), 0.0, a.sin()) * (0.04 + 0.14 * rnd());
        let tall = (0.2 + 0.25 * rnd()) * height;
        let foot = tint.map(|c| c * 0.7);
        let tip = [tint[0] * 1.18, tint[1] * 1.12, tint[2] * 0.95];
        vert(base, -side, foot, 0.0, phase);
        vert(base, side, foot, 0.0, phase);
        vert(base, lean + Vec3::Y * tall, tip, 1.0, phase);
    }
    if flower {
        // a stem and a small four-petalled head
        let petals = [
            [0.95, 0.90, 0.80],
            [0.95, 0.72, 0.12],
            [0.80, 0.15, 0.20],
            [0.55, 0.35, 0.85],
            [0.95, 0.55, 0.70],
        ][(t.seed >> 20) as usize % 5]
            .map(|c: f32| c.powf(2.2));
        let stem = tint.map(|c| c * 0.7);
        let tall = 0.3 + 0.14 * rnd();
        let top = Vec3::Y * tall;
        let base = root - Vec3::Y * 0.03;
        vert(base, Vec3::new(-0.018, 0.0, 0.0), stem, 0.0, phase);
        vert(base, Vec3::new(0.018, 0.0, 0.0), stem, 0.0, phase);
        vert(base, top, stem, 1.0, phase);
        let s = 0.075 + 0.03 * rnd();
        let turn = rnd() * std::f32::consts::TAU;
        let (ax, az) = (
            Vec3::new(turn.cos(), 0.25, turn.sin()) * s,
            Vec3::new(-turn.sin(), 0.25, turn.cos()) * s,
        );
        let heart = [1.0, 0.85, 0.25f32].map(|c| c.powf(2.2));
        for (p, q) in [(ax, az), (az, -ax), (-ax, -az), (-az, ax)] {
            vert(base, top, heart, 1.0, phase);
            vert(base, top + p, petals, 1.0, phase);
            vert(base, top + q, petals, 1.0, phase);
        }
    }
}

/// The six vertices of a halo. The vertex shader turns them to the camera.
fn push_glow(out: &mut Vec<Vertex>, g: &Glow) {
    let c = srgb(g.color);
    for (x, y) in [
        (-1.0, -1.0),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
    ] {
        out.push(Vertex {
            pos: g.pos,
            normal: [x, y, g.size],
            color: [c[0], c[1], c[2], 0.5],
            light: 1.0,
            sky: 1.0,
            tex: [0.0, 0.0, KIND_GLOW, 0.0],
        });
    }
}

/// A flat, glowing ring on the ground around a selected target: red for a
/// hostile one, yellow for neutral, green for friendly. `center` is the
/// target's feet. The ring is level; it is drawn a little in front of the
/// ground it lies on, so gentle slopes do not swallow it. On hillsides
/// `push_ground_ring_on` fits better.
pub fn push_ground_ring(out: &mut Vec<Vertex>, center: Vec3, radius: f32, color: [f32; 3]) {
    ring(out, center, radius, color, |_, _| center.y);
}

/// The same ring, laid over the ground: each point of it takes the height
/// of the (smoothed) ground under it, looked up in the world.
pub fn push_ground_ring_on(
    out: &mut Vec<Vertex>,
    world: &World,
    center: Vec3,
    radius: f32,
    color: [f32; 3],
) {
    // top of the ground in a column, near the height of the target's feet
    let top = |x: i32, z: i32| {
        let y0 = center.y.round() as i32 + 2;
        (y0 - 5..=y0)
            .rev()
            .find(|y| w::is_solid(world.get(x, *y, z)) && !w::is_solid(world.get(x, *y + 1, z)))
            .map_or(center.y, |y| (y + 1) as f32)
    };
    // much as the mesh does it: a column's height leans towards its neighbours'
    let soft = |x: i32, z: i32| {
        let h = top(x, z);
        if !terrain_mesh::is_terrain(world.get(x, h as i32 - 1, z)) {
            return h;
        }
        let mut sum = 0.0;
        for dz in -1..=1 {
            for dx in -1..=1 {
                sum += top(x + dx, z + dz).clamp(h - 1.0, h + 1.0);
            }
        }
        h + (sum / 9.0 - h).clamp(-0.4, 0.4)
    };
    ring(out, center, radius, color, |x, z| {
        // between the four nearest column centres
        let (fx, fz) = (x - 0.5, z - 0.5);
        let (x0, z0) = (fx.floor(), fz.floor());
        let (tx, tz) = (fx - x0, fz - z0);
        let (x0, z0) = (x0 as i32, z0 as i32);
        let a = soft(x0, z0) + (soft(x0 + 1, z0) - soft(x0, z0)) * tx;
        let b = soft(x0, z0 + 1) + (soft(x0 + 1, z0 + 1) - soft(x0, z0 + 1)) * tx;
        a + (b - a) * tz
    });
}

fn ring(
    out: &mut Vec<Vertex>,
    center: Vec3,
    radius: f32,
    color: [f32; 3],
    ground: impl Fn(f32, f32) -> f32,
) {
    const SEGMENTS: usize = 40;
    let c = srgb(color);
    let lift = |k: f32| {
        [
            c[0] + (1.0 - c[0]) * k,
            c[1] + (1.0 - c[1]) * k,
            c[2] + (1.0 - c[2]) * k,
        ]
    };
    // three bands: a soft outer edge, a bright core, a soft inner edge
    let width = (radius * 0.2).clamp(0.07, 0.16);
    let (edge, core) = (c.map(|v| v * 0.75), lift(0.22));
    let bands = [
        (radius + width * 0.5, radius + width * 0.15, edge, core),
        (radius + width * 0.15, radius - width * 0.15, core, core),
        (radius - width * 0.15, radius - width * 0.5, core, edge),
    ];
    // one height per spoke, so the bands stay joined
    let spokes: Vec<(f32, f32, f32)> = (0..=SEGMENTS)
        .map(|i| {
            let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let (dx, dz) = (a.cos(), a.sin());
            let y = ground(center.x + dx * radius, center.z + dz * radius) + 0.06;
            (dx, dz, y)
        })
        .collect();
    let at = |i: usize, r: f32| {
        let (dx, dz, y) = spokes[i];
        [center.x + dx * r, y, center.z + dz * r]
    };
    for i in 0..SEGMENTS {
        for (r0, r1, c0, c1) in bands {
            let quad = [
                (at(i, r0), c0),
                (at(i + 1, r0), c0),
                (at(i + 1, r1), c1),
                (at(i, r1), c1),
            ];
            for k in [0, 1, 2, 0, 2, 3] {
                let (pos, rgb) = quad[k];
                out.push(Vertex {
                    pos,
                    normal: [0.0; 3],
                    // 2.0: glows, whatever the light
                    color: [rgb[0], rgb[1], rgb[2], 2.0],
                    light: 1.0,
                    sky: 1.0,
                    tex: [0.0, 0.0, KIND_DECAL, 0.0],
                });
            }
        }
    }
}

#[derive(Default)]
struct SectionMesh {
    /// one buffer (one allocation: streaming uploads many sections a
    /// second): the opaque vertices, then the translucent, then the halos
    /// round the section's lamps
    buf: Option<wgpu::Buffer>,
    opaque: u32,
    trans: u32,
    glow: u32,
    /// where grass grows; turned into blades when the camera is near
    tufts: Vec<Tuft>,
    /// meshed (possibly to nothing) since the section last arrived
    built: bool,
}

pub struct Gfx {
    /// None when rendering headless (dyno shot).
    pub window: Option<Arc<winit::window::Window>>,
    surface: Option<wgpu::Surface<'static>>,
    offscreen: Option<wgpu::Texture>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub egui: egui_wgpu::Renderer,
    depth: wgpu::TextureView,
    opaque_pipe: wgpu::RenderPipeline,
    trans_pipe: wgpu::RenderPipeline,
    line_pipe: wgpu::RenderPipeline,
    glow_pipe: wgpu::RenderPipeline,
    sky_pipe: wgpu::RenderPipeline,
    shadow_pipe: wgpu::RenderPipeline,
    shadow_bind: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    bgl: wgpu::BindGroupLayout,
    tex_samp: wgpu::Sampler,
    shadow_samp: wgpu::Sampler,
    /// the render settings (look.rs), written every frame
    look_buf: wgpu::Buffer,
    /// the generation of the block textures on the GPU
    tex_gen: u64,
    /// the asset store's generation when last checked for new textures
    assets_gen: u64,
    sections: Vec<SectionMesh>,
    dims: [i32; 3],
    /// grass and flowers around the camera, and the section column they were built for
    clutter: Option<(wgpu::Buffer, u32)>,
    clutter_at: Option<[i32; 2]>,
    grass_tint: [f32; 3],
    pub adapter_info: String,
    /// When set, the next frame is saved to this path as a PPM image.
    pub screenshot: Option<std::path::PathBuf>,
    started: std::time::Instant,
}

use egui_winit::winit;

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl Gfx {
    /// Render without a window into an image of the given size.
    pub fn new_headless(width: u32, height: u32) -> Result<Self, String> {
        Self::create(None, width, height)
    }

    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, String> {
        let size = window.inner_size();
        Self::create(Some(window), size.width, size.height)
    }

    fn create(
        window: Option<Arc<winit::window::Window>>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        // every way into the renderer (game, shots, sheets) sees the pack
        crate::assets::load_local_pack();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env()
                .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
            ..Default::default()
        });
        let surface = match &window {
            Some(w) => Some(
                instance
                    .create_surface(w.clone())
                    .map_err(|e| format!("surface: {e}"))?,
            ),
            None => None,
        };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("No graphics adapter found: {e}"))?;
        let info = adapter.get_info();
        let adapter_info = format!("{} ({:?})", info.name, info.backend);
        let limits = if info.backend == wgpu::Backend::Gl {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("dyno"),
            required_limits: wgpu::Limits {
                max_texture_dimension_2d: 8192,
                ..limits
            },
            ..Default::default()
        }))
        .map_err(|e| format!("device: {e}"))?;
        let (config, offscreen) = match &surface {
            Some(surface) => {
                let mut config = surface
                    .get_default_config(&adapter, width.max(1), height.max(1))
                    .ok_or("surface is not supported by the adapter")?;
                let caps = surface.get_capabilities(&adapter);
                if let Some(f) = caps.formats.iter().find(|f| f.is_srgb()) {
                    config.format = *f;
                }
                config.present_mode = wgpu::PresentMode::AutoVsync;
                if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) {
                    config.usage |= wgpu::TextureUsages::COPY_SRC;
                }
                surface.configure(&device, &config);
                (config, None)
            }
            None => {
                let format = wgpu::TextureFormat::Rgba8UnormSrgb;
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    format,
                    width: width.max(1),
                    height: height.max(1),
                    present_mode: wgpu::PresentMode::Fifo,
                    desired_maximum_frame_latency: 2,
                    alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                    view_formats: vec![],
                };
                let tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("offscreen"),
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: config.usage,
                    view_formats: &[],
                });
                (config, Some(tex))
            }
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("voxel"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let look_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("look"),
            size: std::mem::size_of::<crate::look::LookUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let tex_gen = crate::textures::generation();
        let tex_view = upload_textures(&device, &queue, textures());
        let tex_samp = texture_sampler(&device);
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow"),
            size: wgpu::Extent3d {
                width: SHADOW_SIZE,
                height: SHADOW_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_samp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind = make_bind(
            &device,
            Bindings {
                layout: &bgl,
                ubuf: &ubuf,
                tex_view: &tex_view,
                tex_samp: &tex_samp,
                shadow_view: &shadow_view,
                shadow_samp: &shadow_samp,
                look_buf: &look_buf,
            },
        );
        // the shadow pass only needs the uniforms
        let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let shadow_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow"),
            layout: &shadow_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        let shadow_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[&shadow_bgl],
            push_constant_ranges: &[],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let vlayout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32, 4 => Float32, 5 => Float32x4],
        };
        let make = |fs: &str,
                    topo: wgpu::PrimitiveTopology,
                    blend: Option<wgpu::BlendState>,
                    depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: std::slice::from_ref(&vlayout),
                },
                primitive: wgpu::PrimitiveState {
                    topology: topo,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: depth_write,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        };
        let opaque_pipe = make("fs", wgpu::PrimitiveTopology::TriangleList, None, true);
        let trans_pipe = make(
            "fs_translucent",
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );
        let sky_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let line_pipe = make(
            "fs_translucent",
            wgpu::PrimitiveTopology::LineList,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );
        // halos add their light to the picture
        let glow_pipe = make(
            "fs_glow",
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            false,
        );
        let shadow_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState {
                module: &shadow_module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: std::slice::from_ref(&vlayout),
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview: None,
            cache: None,
        });
        let egui = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );
        let depth = depth_view(&device, config.width, config.height);
        Ok(Gfx {
            window,
            surface,
            offscreen,
            device,
            queue,
            config,
            egui,
            depth,
            opaque_pipe,
            trans_pipe,
            line_pipe,
            glow_pipe,
            sky_pipe,
            shadow_pipe,
            shadow_bind,
            shadow_view,
            ubuf,
            bind,
            bgl,
            tex_samp,
            shadow_samp,
            look_buf,
            tex_gen,
            assets_gen: crate::assets::generation(),
            sections: Vec::new(),
            dims: [0; 3],
            clutter: None,
            clutter_at: None,
            grass_tint: layer_mean(textures().face(w::GRASS, 0)),
            adapter_info,
            screenshot: None,
            started: std::time::Instant::now(),
        })
    }

    /// Upload the block textures again if they changed (more of the pack
    /// arrived and [`crate::textures::reload`] took it in). Layer numbers
    /// stay the same while `blocks.json` does, so the meshes stay right.
    fn refresh_textures(&mut self) {
        let ag = crate::assets::generation();
        if ag != self.assets_gen {
            self.assets_gen = ag;
            crate::textures::reload();
        }
        let tg = crate::textures::generation();
        if tg == self.tex_gen {
            return;
        }
        self.tex_gen = tg;
        let tex_view = upload_textures(&self.device, &self.queue, textures());
        self.bind = make_bind(
            &self.device,
            Bindings {
                layout: &self.bgl,
                ubuf: &self.ubuf,
                tex_view: &tex_view,
                tex_samp: &self.tex_samp,
                shadow_view: &self.shadow_view,
                shadow_samp: &self.shadow_samp,
                look_buf: &self.look_buf,
            },
        );
        self.grass_tint = layer_mean(textures().face(w::GRASS, 0));
        self.clutter_at = None;
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        if let Some(s) = &self.surface {
            s.configure(&self.device, &self.config);
        }
        self.depth = depth_view(&self.device, w, h);
    }

    fn upload(&self, v: &[Vertex]) -> Option<(wgpu::Buffer, u32)> {
        if v.is_empty() {
            return None;
        }
        // a plain buffer filled through the queue's pooled staging memory
        // (create_buffer_init makes a staging buffer of its own each time,
        // and streaming uploads many sections a second)
        let bytes: &[u8] = bytemuck::cast_slice(v);
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buf, 0, bytes);
        Some((buf, v.len() as u32))
    }

    /// Upload one section's mesh.
    fn store(&mut self, i: usize, geo: SectionGeo) {
        let SectionGeo {
            mut opaque,
            trans,
            glows,
            tufts,
        } = geo;
        let (o, t) = (opaque.len() as u32, trans.len() as u32);
        opaque.extend(trans);
        for g in &glows {
            push_glow(&mut opaque, g);
        }
        let glow = opaque.len() as u32 - o - t;
        self.sections[i] = SectionMesh {
            buf: self.upload(&opaque).map(|(b, _)| b),
            opaque: o,
            trans: t,
            glow,
            tufts,
            built: true,
        };
    }

    /// Mesh a list of sections and upload them. In a streamed world, only
    /// sections whose neighbours have all arrived are meshed (the mesh and
    /// the light read past a section's edge); the others wait for them.
    fn remesh(&mut self, world: &World, light: &Light, list: &[[i32; 3]]) {
        let ready: Vec<[i32; 3]>;
        let list = if world.is_sparse() {
            ready = list
                .iter()
                .filter(|s| section_ready(world, **s))
                .copied()
                .collect();
            &ready[..]
        } else {
            list
        };
        // in batches, so a whole world's vertices are never all in memory at once
        for batch in list.chunks(256) {
            for (s, geo) in batch.iter().zip(mesh_many(world, light, batch)) {
                let i = ((s[1] * self.dims[2] + s[2]) * self.dims[0] + s[0]) as usize;
                self.store(i, geo);
            }
        }
        // grass near the camera may have changed
        self.clutter_at = None;
    }

    /// Mesh the whole world.
    pub fn load_world(&mut self, world: &World, light: &Light) {
        let d = [
            (world.size[0] as i32 + SECTION - 1) / SECTION,
            (world.size[1] as i32 + SECTION - 1) / SECTION,
            (world.size[2] as i32 + SECTION - 1) / SECTION,
        ];
        self.dims = d;
        self.sections.clear();
        self.sections
            .resize_with((d[0] * d[1] * d[2]) as usize, SectionMesh::default);
        let mut list = Vec::with_capacity(self.sections.len());
        for sy in 0..d[1] {
            for sz in 0..d[2] {
                for sx in 0..d[0] {
                    list.push([sx, sy, sz]);
                }
            }
        }
        self.remesh(world, light, &list);
    }

    /// Start drawing a streamed world: its size, nothing meshed yet (the
    /// world worker meshes sections as they arrive: `put_mesh`).
    pub fn begin_world(&mut self, world: &World) {
        self.dims = world.section_dims();
        let d = self.dims;
        self.sections.clear();
        self.sections
            .resize_with((d[0] * d[1] * d[2]) as usize, SectionMesh::default);
        self.clutter_at = None;
    }

    /// Store (upload) a section's mesh made elsewhere.
    pub fn put_mesh(&mut self, s: [i32; 3], geo: SectionGeo) {
        let d = self.dims;
        if !(0..3).all(|i| s[i] >= 0 && s[i] < d[i]) {
            return;
        }
        self.store(((s[1] * d[2] + s[2]) * d[0] + s[0]) as usize, geo);
        // grass near the camera may have changed
        if let Some(at) = self.clutter_at
            && (s[0] - at[0]).abs() <= CLUTTER_SECTIONS
            && (s[2] - at[1]).abs() <= CLUTTER_SECTIONS
        {
            self.clutter_at = None;
        }
    }

    /// Whether a section has been meshed since it last arrived.
    pub fn is_built(&self, s: [i32; 3]) -> bool {
        let d = self.dims;
        (0..3).all(|i| s[i] >= 0 && s[i] < d[i])
            && self.sections[((s[1] * d[2] + s[2]) * d[0] + s[0]) as usize].built
    }

    /// A section left a streamed world: stop drawing it.
    pub fn section_gone(&mut self, s: [i32; 3]) {
        let d = self.dims;
        if (0..3).all(|i| s[i] >= 0 && s[i] < d[i]) {
            let i = ((s[1] * d[2] + s[2]) * d[0] + s[0]) as usize;
            self.sections[i] = SectionMesh::default();
            self.clutter_at = None;
        }
    }

    /// Wait until the GPU has finished all submitted work (for timing).
    pub fn wait_idle(&self) {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }

    /// Opaque and translucent vertices held by the world meshes (for stats).
    pub fn world_vertices(&self) -> (u64, u64) {
        self.sections
            .iter()
            .fold((0, 0), |a, s| (a.0 + s.opaque as u64, a.1 + s.trans as u64))
    }

    /// Re-mesh every section that touches the box [lo - margin, hi + margin].
    pub fn region_changed(
        &mut self,
        world: &World,
        light: &Light,
        lo: [i32; 3],
        hi: [i32; 3],
        margin: i32,
    ) {
        if self.sections.is_empty() {
            return;
        }
        // a section's smooth mesh reads a few blocks past its own edge
        let margin = margin.max(terrain_mesh::REACH);
        let s0 = |v: i32, i: usize| ((v - margin).div_euclid(SECTION)).clamp(0, self.dims[i] - 1);
        let s1 = |v: i32, i: usize| ((v + margin).div_euclid(SECTION)).clamp(0, self.dims[i] - 1);
        let mut list = Vec::new();
        for sy in s0(lo[1], 1)..=s1(hi[1], 1) {
            for sz in s0(lo[2], 2)..=s1(hi[2], 2) {
                for sx in s0(lo[0], 0)..=s1(hi[0], 0) {
                    list.push([sx, sy, sz]);
                }
            }
        }
        self.remesh(world, light, &list);
    }

    /// Re-mesh the section that holds a cell, and its neighbours when the cell is on an edge.
    pub fn block_changed(&mut self, world: &World, light: &Light, p: [i32; 3], radius: i32) {
        self.region_changed(world, light, p, p, radius);
    }

    /// Rebuild the grass and flowers when the camera enters a new section
    /// column, or the ground near it changed.
    fn update_clutter(&mut self, cam: Vec3) {
        let at = [
            (cam.x.floor() as i32).div_euclid(SECTION),
            (cam.z.floor() as i32).div_euclid(SECTION),
        ];
        if self.clutter_at == Some(at) {
            return;
        }
        self.clutter_at = Some(at);
        let d = self.dims;
        let mut verts = Vec::new();
        for sz in (at[1] - CLUTTER_SECTIONS).max(0)..=(at[1] + CLUTTER_SECTIONS).min(d[2] - 1) {
            for sx in (at[0] - CLUTTER_SECTIONS).max(0)..=(at[0] + CLUTTER_SECTIONS).min(d[0] - 1) {
                for sy in 0..d[1] {
                    let i = ((sy * d[2] + sz) * d[0] + sx) as usize;
                    for t in &self.sections[i].tufts {
                        push_tuft(&mut verts, t, self.grass_tint);
                    }
                }
            }
        }
        self.clutter = self.upload(&verts);
    }

    /// Draw one frame: the world (if any), extra boxes, lines, then egui.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        view_proj: Option<(Mat4, Vec3, f32)>,
        sky: &Sky,
        boxes: &[Vertex],
        lines: &[Vertex],
        egui_ctx: &egui::Context,
        egui_out: egui::FullOutput,
    ) {
        self.refresh_textures();
        let look = crate::look::settings().uniform();
        self.queue
            .write_buffer(&self.look_buf, 0, bytemuck::bytes_of(&look));
        if let Some((_, cam, _)) = view_proj {
            self.update_clutter(cam);
        }
        let surface_tex = match &self.surface {
            Some(surface) => match surface.get_current_texture() {
                Ok(f) => Some(f),
                Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                    surface.configure(&self.device, &self.config);
                    return;
                }
                Err(_) => return,
            },
            None => None,
        };
        let target: &wgpu::Texture = match (&surface_tex, &self.offscreen) {
            (Some(f), _) => &f.texture,
            (None, Some(t)) => t,
            (None, None) => return,
        };
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let fog = srgb(sky.horizon);
        let top = srgb(sky.top);
        let light = srgb(sky.light);
        if let Some((vp, cam, dist)) = view_proj {
            let u = Uniforms {
                view_proj: vp.to_cols_array_2d(),
                inv_view_proj: vp.inverse().to_cols_array_2d(),
                cam: [
                    cam.x,
                    cam.y,
                    cam.z,
                    self.started.elapsed().as_secs_f32() % 100_000.0,
                ],
                fog: [fog[0], fog[1], fog[2], dist],
                sky_top: [top[0], top[1], top[2], sky.cover],
                light: [light[0], light[1], light[2], 0.0],
                sun: [sky.sun_dir[0], sky.sun_dir[1], sky.sun_dir[2], sky.stars],
                shadow_vp: shadow_matrix(cam, sky.sun_dir).to_cols_array_2d(),
            };
            self.queue
                .write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let box_buf = self.upload(boxes);
        let line_buf = self.upload(lines);

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        if let Some((_, cam, _)) = view_proj {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.shadow_pipe);
            rp.set_bind_group(0, &self.shadow_bind, &[]);
            let reach = SHADOW_HALF + SECTION as f32 * 2.0;
            for (i, s) in self.sections.iter().enumerate() {
                let (Some(b), n) = (&s.buf, s.opaque) else {
                    continue;
                };
                if n == 0 {
                    continue;
                }
                let d = self.dims;
                let (sx, sz) = ((i as i32) % d[0], (i as i32 / d[0]) % d[2]);
                let cx = sx as f32 * SECTION as f32 + SECTION as f32 * 0.5;
                let cz = sz as f32 * SECTION as f32 + SECTION as f32 * 0.5;
                if (cx - cam.x).abs() > reach || (cz - cam.z).abs() > reach {
                    continue;
                }
                rp.set_vertex_buffer(0, b.slice(..));
                rp.draw(0..n, 0..1);
            }
            if let Some((b, n)) = &box_buf {
                rp.set_vertex_buffer(0, b.slice(..));
                rp.draw(0..*n, 0..1);
            }
        }
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: fog[0] as f64,
                            g: fog[1] as f64,
                            b: fog[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some((vp, _, dist)) = view_proj {
                rp.set_bind_group(0, &self.bind, &[]);
                rp.set_pipeline(&self.sky_pipe);
                rp.draw(0..3, 0..1);
                rp.set_pipeline(&self.opaque_pipe);
                let visible = |i: usize| self.section_visible(i, vp, dist);
                for (i, s) in self.sections.iter().enumerate() {
                    if let Some(b) = &s.buf
                        && s.opaque > 0
                        && visible(i)
                    {
                        rp.set_vertex_buffer(0, b.slice(..));
                        rp.draw(0..s.opaque, 0..1);
                    }
                }
                if let Some((b, n)) = &self.clutter {
                    rp.set_vertex_buffer(0, b.slice(..));
                    rp.draw(0..*n, 0..1);
                }
                if let Some((b, n)) = &box_buf {
                    rp.set_vertex_buffer(0, b.slice(..));
                    rp.draw(0..*n, 0..1);
                }
                rp.set_pipeline(&self.trans_pipe);
                for (i, s) in self.sections.iter().enumerate() {
                    if let Some(b) = &s.buf
                        && s.trans > 0
                        && visible(i)
                    {
                        rp.set_vertex_buffer(0, b.slice(..));
                        rp.draw(s.opaque..s.opaque + s.trans, 0..1);
                    }
                }
                rp.set_pipeline(&self.glow_pipe);
                for (i, s) in self.sections.iter().enumerate() {
                    if let Some(b) = &s.buf
                        && s.glow > 0
                        && visible(i)
                    {
                        let start = s.opaque + s.trans;
                        rp.set_vertex_buffer(0, b.slice(..));
                        rp.draw(start..start + s.glow, 0..1);
                    }
                }
                if let Some((b, n)) = &line_buf {
                    rp.set_pipeline(&self.line_pipe);
                    rp.set_vertex_buffer(0, b.slice(..));
                    rp.draw(0..*n, 0..1);
                }
            }
        }

        // egui
        let ppp = egui_ctx.pixels_per_point();
        let jobs = egui_ctx.tessellate(egui_out.shapes, ppp);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: ppp,
        };
        for (id, delta) in &egui_out.textures_delta.set {
            self.egui
                .update_texture(&self.device, &self.queue, *id, delta);
        }
        let extra = self
            .egui
            .update_buffers(&self.device, &self.queue, &mut enc, &jobs, &screen);
        {
            let rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            let mut rp = rp.forget_lifetime();
            self.egui.render(&mut rp, &jobs, &screen);
        }
        for id in &egui_out.textures_delta.free {
            self.egui.free_texture(id);
        }
        // optional screenshot: copy the frame into a buffer before presenting
        let shot = match self.screenshot.take() {
            Some(path) if self.config.usage.contains(wgpu::TextureUsages::COPY_SRC) => {
                let (w, h) = (self.config.width, self.config.height);
                let padded = (w * 4).div_ceil(256) * 256;
                let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("screenshot"),
                    size: (padded * h) as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                enc.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: target,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buf,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(padded),
                            rows_per_image: Some(h),
                        },
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
                Some((path, buf, padded, w, h))
            }
            _ => None,
        };
        self.queue
            .submit(extra.into_iter().chain(std::iter::once(enc.finish())));
        if let Some((path, buf, padded, w, h)) = shot {
            buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            let data = buf.slice(..).get_mapped_range();
            let bgra = matches!(
                self.config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            );
            let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
            for y in 0..h {
                let row = &data[(y * padded) as usize..(y * padded + w * 4) as usize];
                for px in row.as_chunks::<4>().0 {
                    if bgra {
                        out.extend_from_slice(&[px[2], px[1], px[0]]);
                    } else {
                        out.extend_from_slice(&px[..3]);
                    }
                }
            }
            drop(data);
            buf.unmap();
            match std::fs::write(&path, out) {
                Ok(()) => eprintln!("screenshot saved: {}", path.display()),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
        }
        if let Some(f) = surface_tex {
            if let Some(w) = &self.window {
                w.pre_present_notify();
            }
            f.present();
        }
    }

    fn section_visible(&self, i: usize, vp: Mat4, dist: f32) -> bool {
        let d = self.dims;
        let i = i as i32;
        let sx = i % d[0];
        let sz = (i / d[0]) % d[2];
        let sy = i / (d[0] * d[2]);
        let c = Vec3::new(
            (sx * SECTION + SECTION / 2) as f32,
            (sy * SECTION + SECTION / 2) as f32,
            (sz * SECTION + SECTION / 2) as f32,
        );
        let r = SECTION as f32 * 0.87;
        let clip = vp * c.extend(1.0);
        if clip.w < -r {
            return false;
        }
        if clip.w > dist + r * 2.0 {
            return false;
        }
        let w = clip.w.abs() + r * 2.0;
        clip.x.abs() <= w * 1.2 && clip.y.abs() <= w * 1.2
    }
}

/// The sun's view of the area around the camera, snapped to shadow texels so
/// shadows do not shimmer as you walk.
fn shadow_matrix(cam: Vec3, sun_dir: [f32; 3]) -> Mat4 {
    let mut dir = Vec3::from(sun_dir).normalize_or_zero();
    if dir.y < 0.0 {
        dir = -dir; // the moon is opposite the sun
    }
    let dir = Vec3::new(dir.x, dir.y.max(0.25), dir.z).normalize();
    let up = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    let view = Mat4::look_to_rh(Vec3::ZERO, -dir, up);
    // snap the centre to whole texels in light space
    let texel = SHADOW_HALF * 2.0 / SHADOW_SIZE as f32;
    let c = view.transform_point3(cam);
    let c = Vec3::new(
        (c.x / texel).floor() * texel,
        (c.y / texel).floor() * texel,
        c.z,
    );
    let proj = Mat4::orthographic_rh(
        c.x - SHADOW_HALF,
        c.x + SHADOW_HALF,
        c.y - SHADOW_HALF,
        c.y + SHADOW_HALF,
        -c.z - 200.0,
        -c.z + 200.0,
    );
    proj * view
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Timing on the real starting world: Hearthstead and the zones, as a
    /// fresh server builds them. Run with:
    /// `cargo test --release -p dyno-client town_meshing_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn town_meshing_speed() {
        let mut world = World::generate([384, 96, 384], 20261122);
        let hearth = world.ensure_spawn_hearth(None);
        w::village::build_hearthstead(&mut world, hearth);
        w::zones::build_world_features(&mut world, hearth);
        let light = Light::new(&world);
        let _ = textures();
        // the sections of the town: 64 blocks round the hearth
        let mut town = Vec::new();
        let mut all = Vec::new();
        for sy in 0..6 {
            for sz in 0..24 {
                for sx in 0..24 {
                    all.push([sx, sy, sz]);
                    let c = [sx * SECTION + 8, sz * SECTION + 8];
                    if (c[0] - hearth[0]).abs() <= 40 && (c[1] - hearth[2]).abs() <= 40 {
                        town.push([sx, sy, sz]);
                    }
                }
            }
        }
        // each section's best time over several runs, so other work on a
        // busy machine does not count
        let mut best = vec![f64::MAX; town.len()];
        let mut verts = 0usize;
        for _ in 0..9 {
            verts = 0;
            for (i, s) in town.iter().enumerate() {
                let t = std::time::Instant::now();
                let geo = terrain_mesh::mesh(&world, &light, s[0], s[1], s[2]);
                best[i] = best[i].min(t.elapsed().as_secs_f64() * 1e3);
                verts += geo.opaque.len() + geo.trans.len();
            }
        }
        let sum: f64 = best.iter().sum();
        let worst = best.iter().copied().fold(0.0, f64::max);
        let start = std::time::Instant::now();
        let n: usize = all
            .chunks(256)
            .map(|b| mesh_many(&world, &light, b).len())
            .sum();
        let many = start.elapsed().as_secs_f64();
        eprintln!(
            "town: {} sections, {:.3} ms each on average, worst {worst:.2} ms, {verts} vertices; \
             whole world ({n} sections) on all cores {:.2} s",
            town.len(),
            sum / town.len() as f64,
            many
        );
    }

    /// Timing. Run with:
    /// `cargo test --release -p dyno-client meshing_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn meshing_speed() {
        let world = terrain_mesh::test_world([384, 96, 384]);
        let light = Light::new(&world);
        let mut list = Vec::new();
        for sy in 0..6 {
            for sz in 0..24 {
                for sx in 0..24 {
                    list.push([sx, sy, sz]);
                }
            }
        }
        // build the textures first: that is a one-off, not part of meshing
        let _ = textures();
        let (mut worst, mut busy, mut verts) = (0.0f64, 0, 0usize);
        let start = std::time::Instant::now();
        for s in &list {
            let t = std::time::Instant::now();
            let geo = terrain_mesh::mesh(&world, &light, s[0], s[1], s[2]);
            let ms = t.elapsed().as_secs_f64() * 1e3;
            worst = worst.max(ms);
            if !geo.opaque.is_empty() {
                busy += 1;
            }
            verts += geo.opaque.len() + geo.trans.len();
        }
        let one = start.elapsed().as_secs_f64();
        let start = std::time::Instant::now();
        let all: usize = list
            .chunks(256)
            .map(|b| mesh_many(&world, &light, b).len())
            .sum();
        let many = start.elapsed().as_secs_f64();
        eprintln!(
            "{all} sections ({busy} with a mesh, {verts} vertices): one thread {one:.2} s \
             ({:.3} ms each on average, worst {worst:.2} ms); all cores {many:.2} s",
            one * 1e3 / list.len() as f64
        );
    }
}
