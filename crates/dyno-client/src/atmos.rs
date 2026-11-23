//! Light and air: how sunlight, moonlight, sky light and lamps fall on the
//! world, and how distance and low ground haze it. The strengths and colours
//! come from the pack (`render.json`: `light`, `atmosphere`); this module
//! holds their formats, the uniform block the shaders read (`Env`) and the
//! shader code that uses it. Without the pack file the picture is plain:
//! white light, no haze, no fog.

use crate::render::Sky;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use serde::Deserialize;

/// `light` in render.json. Colours are sRGB.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Lighting {
    /// multiplies direct sun or moon light
    pub sun_tint: [f32; 3],
    /// how much of the direct light reaches faces turned away and in shadow
    pub fill: f32,
    /// colour of the light from the sky, by day and by night, and its strength
    pub ambient_day: [f32; 3],
    pub ambient_night: [f32; 3],
    pub ambient: f32,
    /// colour of lamp and fire light, and its strength by day and by night
    pub lamp: [f32; 3],
    pub lamp_day: f32,
    pub lamp_night: f32,
    /// how much daylight drowns lamp light (0: added in full; 1: hidden in full sun)
    pub lamp_hide: f32,
    /// brightness of glowing blocks; above 1 they bloom
    pub emissive: f32,
    /// light that catches the edges of things facing the moon (or the sun)
    pub rim: [f32; 3],
    pub rim_day: f32,
    pub rim_night: f32,
    pub rim_power: f32,
    /// how dark sun and moon shadows are (0: none)
    pub shadow: f32,
    pub moon_shadow: f32,
    /// the moon's path: 1 is straight opposite the sun, less keeps it lower;
    /// `moon_lift` raises it from the horizon
    pub moon_arc: f32,
    pub moon_lift: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            sun_tint: [1.0; 3],
            fill: 0.2,
            ambient_day: [0.45, 0.5, 0.75],
            ambient_night: [0.45, 0.5, 0.75],
            ambient: 0.42,
            lamp: [1.0, 0.78, 0.5],
            lamp_day: 1.15,
            lamp_night: 1.15,
            lamp_hide: 1.0,
            emissive: 1.1,
            rim: [1.0; 3],
            rim_day: 0.0,
            rim_night: 0.0,
            rim_power: 3.0,
            shadow: 1.0,
            moon_shadow: 1.0,
            moon_arc: 1.0,
            moon_lift: 0.0,
        }
    }
}

/// `atmosphere` in render.json: aerial haze with distance, and fog lying in
/// low ground. Colours are sRGB.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Atmosphere {
    /// haze begins this far away (metres) and thickens by `haze_density`
    /// a metre up to `haze_max`
    pub haze_start: f32,
    pub haze_density: f32,
    pub haze_max: f32,
    pub haze_day: [f32; 3],
    pub haze_night: [f32; 3],
    /// how far the haze colour follows the horizon colour (sunsets, weather)
    pub haze_follow: f32,
    /// fog below this height (world y), thinning upwards by `fog_falloff` a metre
    pub fog_height: f32,
    pub fog_falloff: f32,
    pub fog_density: f32,
    pub fog_max: f32,
    /// the fog begins this far from the eye (metres): the foreground stays clear
    pub fog_start: f32,
    pub fog_day: [f32; 3],
    pub fog_night: [f32; 3],
    /// the fog is this many times thicker at dawn
    pub fog_dawn: f32,
    /// where the fade into the sky at the edge of the drawn world begins,
    /// as a part of the draw distance
    pub edge: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Atmosphere {
            haze_start: 0.0,
            haze_density: 0.0,
            haze_max: 0.0,
            haze_day: [0.7; 3],
            haze_night: [0.1; 3],
            haze_follow: 1.0,
            fog_height: 0.0,
            fog_falloff: 0.2,
            fog_density: 0.0,
            fog_max: 0.0,
            fog_start: 0.0,
            fog_day: [0.8; 3],
            fog_night: [0.1; 3],
            fog_dawn: 1.0,
            edge: 0.55,
        }
    }
}

/// The light and air of one frame, worked out from the time of day, the
/// pack's settings and the mood of the place (`sky::sky_at`). Colours are
/// sRGB, as written in the pack.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// towards the sun by day, the moon by night
    pub light_dir: [f32; 3],
    pub shadow: f32,
    pub ambient: [f32; 3],
    pub ambient_strength: f32,
    pub fill: f32,
    pub sun_tint: [f32; 3],
    pub emissive: f32,
    pub lamp: [f32; 3],
    pub lamp_strength: f32,
    pub lamp_hide: f32,
    pub rim: [f32; 3],
    pub rim_strength: f32,
    pub rim_power: f32,
    pub haze: [f32; 3],
    pub haze_follow: f32,
    pub haze_density: f32,
    pub haze_start: f32,
    pub haze_max: f32,
    pub fog: [f32; 3],
    pub fog_density: f32,
    pub fog_height: f32,
    pub fog_falloff: f32,
    pub fog_max: f32,
    pub fog_start: f32,
    pub edge: f32,
    /// the sky dome (sky.rs)
    pub dome: crate::sky::Dome,
    /// the grade of the finished picture (post.rs)
    pub exposure: f32,
    pub tint: [f32; 3],
    pub saturation: f32,
    pub contrast: f32,
}

fn lin(c: [f32; 3]) -> [f32; 3] {
    [c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2)]
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn v4(c: [f32; 3], w: f32) -> [f32; 4] {
    [c[0], c[1], c[2], w]
}

/// The shaders' `Env` block (binding 6). Colours are linear.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct EnvUniform {
    cascade_vp: [[[f32; 4]; 4]; 3],
    /// xyz: depth bias of each cascade; w: one shadow texel in UV
    cascade_bias: [f32; 4],
    /// xyz: how far each cascade lifts a point off its surface (m); w: shadow strength
    cascade_off: [f32; 4],
    light_dir: [f32; 4],
    /// rgb: sky light; w: fill
    ambient: [f32; 4],
    /// rgb: tint of direct light; w: emissive brightness
    sun_tint: [f32; 4],
    /// rgb: lamp light times its strength; w: how much daylight hides it
    lamp: [f32; 4],
    /// rgb: rim light times its strength; w: power
    rim: [f32; 4],
    /// rgb: haze; w: density a metre
    haze: [f32; 4],
    /// start, max, edge start (part of the draw distance), 0
    haze2: [f32; 4],
    /// rgb: fog; w: density
    fog: [f32; 4],
    /// height, falloff, max, start
    fog2: [f32; 4],
    dome: crate::sky::DomeUniform,
}

impl EnvUniform {
    /// `cascades`: the shadow maps' transforms with the world size of one of
    /// their texels (shadows.rs).
    pub fn pack(sky: &Sky, cascades: &[(Mat4, f32); 3], shadow_size: u32) -> EnvUniform {
        let f = &sky.env;
        let horizon = lin(sky.horizon);
        let top = lin(sky.top);
        let light = lin(sky.light);
        let haze = mix3(lin(f.haze), horizon, f.haze_follow);
        let fog = mix3(lin(f.fog), horizon, f.haze_follow * 0.5);
        let ambient = mix3(top, lin(f.ambient), 0.5).map(|v| v * f.ambient_strength);
        // a point is lifted off its surface by about a texel and a half, and
        // compared a little nearer the light: enough against acne, too
        // little to float the shadow off its caster
        let range = crate::shadows::DEPTH_RANGE * 2.0;
        let texel = [cascades[0].1, cascades[1].1, cascades[2].1];
        EnvUniform {
            cascade_vp: [
                cascades[0].0.to_cols_array_2d(),
                cascades[1].0.to_cols_array_2d(),
                cascades[2].0.to_cols_array_2d(),
            ],
            cascade_bias: [
                (0.02 + texel[0] * 1.0) / range,
                (0.02 + texel[1] * 1.0) / range,
                (0.02 + texel[2] * 1.0) / range,
                1.0 / shadow_size as f32,
            ],
            cascade_off: [
                0.015 + texel[0] * 1.5,
                0.015 + texel[1] * 1.5,
                0.015 + texel[2] * 1.5,
                f.shadow,
            ],
            light_dir: v4(f.light_dir, 0.0),
            ambient: v4(ambient, f.fill),
            sun_tint: v4(f.sun_tint, f.emissive),
            lamp: v4(lin(f.lamp).map(|v| v * f.lamp_strength), f.lamp_hide),
            rim: v4(lin(f.rim).map(|v| v * f.rim_strength), f.rim_power.max(0.1)),
            haze: v4(haze, f.haze_density),
            haze2: [f.haze_start, f.haze_max, f.edge, 0.0],
            fog: v4(fog, f.fog_density),
            fog2: [
                f.fog_height,
                f.fog_falloff.max(0.001),
                f.fog_max,
                f.fog_start,
            ],
            dome: f.dome.uniform(horizon, top, light),
        }
    }
}

/// The `Env` block and the sky behind everything, for any shader that also
/// declares `u: U`. The edge of the drawn world fades into `sky_backdrop`,
/// so it must be the same in the world shader and the sky shader.
pub const ENV_WGSL: &str = r#"
struct Env {
  cascade_vp: array<mat4x4<f32>, 3>,
  cascade_bias: vec4<f32>,
  cascade_off: vec4<f32>,
  light_dir: vec4<f32>,
  ambient: vec4<f32>,
  sun_tint: vec4<f32>,
  lamp: vec4<f32>,
  rim: vec4<f32>,
  haze: vec4<f32>,
  haze2: vec4<f32>,
  fog: vec4<f32>,
  fog2: vec4<f32>,
  // the sky dome (sky.rs)
  glow: vec4<f32>,
  sun: vec4<f32>,
  moon: vec4<f32>,
  moon_dir: vec4<f32>,
  cloud_lit: vec4<f32>,
  cloud_shade: vec4<f32>,
  cloud: vec4<f32>,
  ridge_near: vec4<f32>,
  ridge_far: vec4<f32>,
  ridge_snow: vec4<f32>,
  stars: vec4<f32>,
};
@group(0) @binding(6) var<uniform> env: Env;

fn e_mix(h0: u32) -> u32 {
  var h = h0;
  h = (h ^ (h >> 16u)) * 0x7feb352du;
  h = (h ^ (h >> 15u)) * 0x846ca68bu;
  return h ^ (h >> 16u);
}
fn e_hash(p: vec2<f32>) -> f32 {
  let q = bitcast<vec2<u32>>(vec2<i32>(floor(p)));
  return f32(e_mix(q.x * 0x9E3779B1u ^ e_mix(q.y)) >> 8u) / 16777216.0;
}
fn e_noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let s = f * f * (3.0 - 2.0 * f);
  return mix(mix(e_hash(i), e_hash(i + vec2<f32>(1.0, 0.0)), s.x),
             mix(e_hash(i + vec2<f32>(0.0, 1.0)), e_hash(i + vec2<f32>(1.0, 1.0)), s.x), s.y);
}

// A far ridge line: its height (as the sine of the angle above the horizon)
// in a direction, in voxel steps.
fn ridge(az: f32, steps: f32, seed: f32, rough: f32) -> f32 {
  let a = floor(az * steps) / steps;
  let p = vec2<f32>(cos(a), sin(a));
  let n = e_noise(p * 2.3 + seed) * 0.55 + e_noise(p * 6.1 + seed * 1.7) * 0.3 + e_noise(p * 17.0 * rough + seed * 2.9) * 0.15;
  return n;
}

// The sky with nothing in it but the far ridges and the glow round the sun:
// rgb, and in a how much of it is land (ridges), which hides sun, moon,
// stars and clouds.
fn sky_backdrop(dir: vec3<f32>) -> vec4<f32> {
  let h = clamp(dir.y, -1.0, 1.0);
  var c = mix(u.fog.rgb, u.sky_top.rgb, smoothstep(-0.02, 0.55, h));
  c = mix(c, u.fog.rgb * 0.6, smoothstep(0.0, -0.4, h));
  // the low sky glows on the side of the sun
  let flat_d = normalize(vec3<f32>(dir.x, 0.0, dir.z) + vec3<f32>(0.0001, 0.0, 0.0));
  let flat_s = normalize(vec3<f32>(u.sun.x, 0.0, u.sun.z) + vec3<f32>(0.0001, 0.0, 0.0));
  let toward = max(dot(flat_d, flat_s) * 0.5 + 0.5, 0.0);
  let low = 1.0 - smoothstep(0.0, 0.5, abs(h));
  c += env.glow.rgb * pow(toward, env.glow.w) * low * low;
  var land = 0.0;
  let hs = env.ridge_near.w;
  if (hs > 0.0 && h < 0.21 * hs && h > -0.05) {
    let az = atan2(dir.x, dir.z);
    let q = floor(h * 260.0) / 260.0;
    // three layers, far to near: paler and bluer with distance
    let far_h = (0.045 + 0.150 * ridge(az, 150.0, 3.7, 1.0)) * hs;
    let mid_h = (0.028 + 0.085 * ridge(az + 1.3, 190.0, 11.1, 0.8)) * hs;
    let near_h = (0.012 + 0.048 * ridge(az + 2.9, 240.0, 23.9, 0.6)) * hs;
    let sky_c = c;
    if (q < far_h) {
      var far = env.ridge_far.rgb;
      // snow on the highest tops
      let top = smoothstep(0.125 * hs, 0.150 * hs, q + 0.012 * e_hash(vec2<f32>(floor(az * 150.0), 3.0)));
      far = mix(far, env.ridge_snow.rgb, env.ridge_snow.w * top);
      c = mix(far, sky_c, env.ridge_far.w);
      land = 1.0;
    }
    if (q < mid_h) {
      c = mix(mix(env.ridge_near.rgb, env.ridge_far.rgb, 0.5), sky_c, env.ridge_far.w * 0.6);
      land = 1.0;
    }
    if (q < near_h) {
      c = mix(env.ridge_near.rgb, sky_c, env.ridge_far.w * 0.3);
      land = 1.0;
    }
    // haze lies along the foot of the hills
    c = mix(c, u.fog.rgb, land * (1.0 - smoothstep(0.0, 0.02 * hs, h)) * 0.6);
  }
  // the fog of the low ground, seen all the way to the horizon
  if (env.fog.w > 0.0) {
    let k = env.fog2.y;
    let y0 = max((u.cam.y - env.fog2.x) * k, 0.0);
    let fg = 1.0 - exp(-env.fog.w * exp(-y0) / (max(h, 0.0) * k + 0.004));
    c = mix(c, env.fog.rgb, min(fg, env.fog2.z));
  }
  return vec4<f32>(c, land);
}
"#;

/// Light and air for the world shader (appended to it after `ENV_WGSL`):
/// shadows from the cascades, the light on a surface, and the haze and fog
/// between it and the eye.
pub const LIGHT_WGSL: &str = r#"
fn shadow_tap(uv: vec2<f32>, layer: i32, z: f32) -> f32 {
  let o = 0.9 * env.cascade_bias.w;
  return 0.25 * (textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(-o, -o), layer, z)
    + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(o, -o), layer, z)
    + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(-o, o), layer, z)
    + textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(o, o), layer, z));
}

// A point in one cascade: xy its place on the map, z its depth, w how near
// the edge of the map it is (1 at the edge and beyond).
fn cascade_point(wpos: vec3<f32>, n: vec3<f32>, c: i32) -> vec4<f32> {
  let p = env.cascade_vp[c] * vec4<f32>(wpos + n * env.cascade_off[c], 1.0);
  var edge = max(abs(p.x), abs(p.y));
  if (p.z < 0.0 || p.z > 1.0) { edge = 2.0; }
  return vec4<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5, p.z - env.cascade_bias[c], edge);
}

// 1 in sunlight (or moonlight), 0 in shadow; soft-edged. The finest cascade
// that holds the point answers, blending into the next towards its edge.
fn sun_visible(wpos: vec3<f32>, n: vec3<f32>) -> f32 {
  let strength = env.cascade_off.w;
  if (strength <= 0.0) { return 1.0; }
  var vis = 1.0;
  var found = false;
  for (var c = 0; c < 3; c++) {
    let p = cascade_point(wpos, n, c);
    if (p.w < 0.98) {
      vis = shadow_tap(p.xy, c, p.z);
      let k = smoothstep(0.86, 0.98, p.w);
      if (k > 0.0) {
        var next = 1.0;
        if (c < 2) {
          let q = cascade_point(wpos, n, c + 1);
          if (q.w < 0.98) { next = shadow_tap(q.xy, c + 1, q.z); }
        }
        vis = mix(vis, next, k);
      }
      found = true;
      break;
    }
  }
  if (!found) { return 1.0; }
  return mix(1.0, vis, strength);
}

// The light on a surface: sun or moon (with shadows and the angle of the
// surface), soft light from the sky, a rim where it turns from the eye
// towards the light, and warm lamp light, which daylight drowns.
// lam: N.L; wrap: 0 hard light .. 1 light wraps round the form;
// sl, bl: sky and block light (squared).
fn world_light(wpos: vec3<f32>, n: vec3<f32>, lam: f32, wrap: f32, has_n: bool, sl: f32, bl: f32) -> vec3<f32> {
  let ndl = select(0.75, max(mix(lam, lam * 0.5 + 0.5, wrap), 0.0), has_n);
  let vis = sun_visible(wpos, n);
  let up = select(0.75, 0.6 + 0.4 * n.y, has_n);
  let direct = u.light.rgb * env.sun_tint.rgb * (env.ambient.w + ndl * vis);
  var day = direct + env.ambient.rgb * up;
  if (has_n) {
    let v = normalize(u.cam.xyz - wpos);
    let edge = pow(1.0 - clamp(dot(n, v), 0.0, 1.0), env.rim.w);
    day += env.rim.rgb * edge * clamp(dot(n, env.light_dir.xyz) * 0.5 + 0.5, 0.0, 1.0) * (0.35 + 0.65 * vis);
  }
  day = day * (sl * 0.97 + 0.03);
  let lum = dot(day, vec3<f32>(0.3, 0.6, 0.1));
  return day + env.lamp.rgb * bl * (1.0 - env.lamp.w * clamp(lum * 1.4, 0.0, 1.0));
}

// What the air does to a colour seen from the camera: blue haze with
// distance, fog lying in low ground, and at the edge of the drawn world a
// fade into the sky that stands behind it.
fn atmosphere(c: vec3<f32>, wpos: vec3<f32>) -> vec3<f32> {
  return atmosphere_under(c, wpos, 1.0);
}

// The same for a surface that sees `sl` of the sky (0 underground .. 1 in
// the open): haze and fog are lit by the sky, so caves and tunnels keep
// their darkness instead of filling with bright air.
fn atmosphere_under(c: vec3<f32>, wpos: vec3<f32>, sl: f32) -> vec3<f32> {
  let open = 0.12 + 0.88 * clamp(sl, 0.0, 1.0);
  let v = wpos - u.cam.xyz;
  let d = length(v);
  let dir = v / max(d, 0.001);
  let hz = env.haze2.y * (1.0 - exp(-max(d - env.haze2.x, 0.0) * env.haze.w));
  var o = mix(c, env.haze.rgb, hz * open);
  if (env.fog.w > 0.0) {
    // fog thinning upwards from fog2.x: its amount along the ray
    let k = env.fog2.y;
    let y0 = max((u.cam.y - env.fog2.x) * k, 0.0);
    let dy = dir.y * k;
    var t = d;
    if (abs(dy) > 0.0001) { t = (1.0 - exp(-clamp(dy * d, -20.0, 20.0))) / dy; }
    // the stretch nearest the eye is clear
    t = t * max(d - env.fog2.w, 0.0) / max(d, 0.001);
    let fg = 1.0 - exp(-env.fog.w * exp(-y0) * max(t, 0.0));
    o = mix(o, env.fog.rgb, min(fg, env.fog2.z) * open);
  }
  let far = u.fog.w;
  let e = smoothstep(env.haze2.z * far, far, d);
  if (e > 0.0) { o = mix(o, sky_backdrop(dir).rgb, e); }
  return o;
}
"#;
