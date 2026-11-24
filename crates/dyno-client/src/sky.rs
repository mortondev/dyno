//! The sky: what the time of day and the place make of the pack's palette
//! (`sky_at`), and the dome drawn behind the world: a gradient from horizon
//! to zenith, a glow where the sun stands low, sun, moon and stars, clouds
//! built of cells like everything else, and far ridges fading into haze.
//! Every colour and size is the pack's (`render.json`: `sky`, `dome`).

use crate::atmos::Frame;
use crate::look::{self, RenderSettings};
use crate::mood;
use crate::render::Sky;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use serde::Deserialize;

/// `dome` in render.json. Colours are sRGB; brightness above 1 blooms.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DomeLook {
    /// the sun's disc high in the sky and near the horizon
    pub sun: [f32; 3],
    pub sun_low: [f32; 3],
    pub sun_bright: f32,
    /// radius in degrees
    pub sun_size: f32,
    pub moon: [f32; 3],
    pub moon_bright: f32,
    pub moon_size: f32,
    /// the moon is drawn this much lower than its light falls from (1: where
    /// it is; less brings it down into view)
    pub moon_height: f32,
    /// the glow low in the sky on the sun's side at sunrise and sunset
    pub glow: [f32; 3],
    pub glow_strength: f32,
    /// how tightly the glow keeps to the sun's side (higher: narrower)
    pub glow_tight: f32,
    pub stars: f32,
    pub clouds: Clouds,
    pub horizon: Horizon,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Clouds {
    /// 0 none .. 1 overcast
    pub cover: f32,
    /// size of a cloud cell, in degrees of sky
    pub size: f32,
    /// drift, in degrees a second
    pub speed: f32,
    pub day: [f32; 3],
    pub night: [f32; 3],
    pub sunset: [f32; 3],
    /// multiplies the lit colour on the undersides
    pub shade: [f32; 3],
}

/// The ridges beyond the edge of the world.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Horizon {
    /// 0: no ridges; 1: the far ridge reaches about 7 degrees
    pub height: f32,
    pub color: [f32; 3],
    /// how much of the sky shows through the far ridge (0 solid .. 1 gone)
    pub haze: f32,
    pub snow: [f32; 3],
    pub snow_amount: f32,
}

impl Default for DomeLook {
    fn default() -> Self {
        DomeLook {
            sun: [1.0, 0.97, 0.88],
            sun_low: [1.0, 0.75, 0.45],
            sun_bright: 1.0,
            sun_size: 2.3,
            moon: [0.85, 0.88, 0.95],
            moon_bright: 1.0,
            moon_size: 2.0,
            moon_height: 1.0,
            glow: [1.0, 0.75, 0.45],
            glow_strength: 0.0,
            glow_tight: 3.0,
            stars: 1.0,
            clouds: Clouds::default(),
            horizon: Horizon::default(),
        }
    }
}

impl Default for Clouds {
    fn default() -> Self {
        Clouds {
            cover: 0.0,
            size: 0.7,
            speed: 0.05,
            day: [1.0; 3],
            night: [0.2; 3],
            sunset: [1.0, 0.8, 0.7],
            shade: [0.7; 3],
        }
    }
}

impl Default for Horizon {
    fn default() -> Self {
        Horizon {
            height: 0.0,
            color: [0.3, 0.4, 0.5],
            haze: 0.6,
            snow: [0.9; 3],
            snow_amount: 0.0,
        }
    }
}

/// The dome of one frame. Colours are sRGB.
#[derive(Clone, Copy, Debug)]
pub struct Dome {
    pub glow: [f32; 3],
    pub glow_strength: f32,
    pub glow_tight: f32,
    pub sun: [f32; 3],
    pub sun_bright: f32,
    pub sun_size: f32,
    pub moon: [f32; 3],
    pub moon_bright: f32,
    pub moon_size: f32,
    pub moon_dir: [f32; 3],
    /// 0 by day .. 1 at night
    pub moon_shown: f32,
    pub cloud_lit: [f32; 3],
    pub cloud_shade: [f32; 3],
    pub cloud_cover: f32,
    pub cloud_size: f32,
    pub cloud_speed: f32,
    pub ridge: [f32; 3],
    pub ridge_height: f32,
    pub ridge_haze: f32,
    pub snow: [f32; 3],
    pub snow_amount: f32,
    pub stars: f32,
}

/// The dome's part of the shaders' `Env` block. Colours are linear.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DomeUniform {
    /// rgb: glow times its strength; w: tightness
    glow: [f32; 4],
    /// rgb: the disc; w: cosine of its radius
    sun: [f32; 4],
    moon: [f32; 4],
    /// xyz: towards the moon; w: how much of it shows
    moon_dir: [f32; 4],
    /// rgb: sunlit cloud; w: cover
    cloud_lit: [f32; 4],
    /// rgb: cloud undersides; w: size of a cell (radians)
    cloud_shade: [f32; 4],
    /// x: drift (radians a second)
    cloud: [f32; 4],
    /// rgb: the nearest ridge; w: height of the ridges
    ridge_near: [f32; 4],
    /// rgb: the farthest ridge; w: how much sky shows through it
    ridge_far: [f32; 4],
    /// rgb: snow; w: amount
    ridge_snow: [f32; 4],
    /// x: brightness of the stars
    stars: [f32; 4],
}

fn lin(c: [f32; 3]) -> [f32; 3] {
    [c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2)]
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn mul(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

fn v4(c: [f32; 3], w: f32) -> [f32; 4] {
    [c[0], c[1], c[2], w]
}

impl Dome {
    /// `horizon`, `top` and `light`: the frame's sky colours, linear (after
    /// the weather has greyed them).
    pub fn uniform(&self, horizon: [f32; 3], top: [f32; 3], light: [f32; 3]) -> DomeUniform {
        // the ridges stand in the same light as the land
        let lit = [
            light[0] * 0.75 + top[0] * 0.35,
            light[1] * 0.75 + top[1] * 0.35,
            light[2] * 0.75 + top[2] * 0.35,
        ];
        let ridge = mul(lin(self.ridge), lit);
        let near = lerp(ridge, horizon, 0.2);
        let far = lerp(ridge, lerp(horizon, top, 0.45), 0.6);
        DomeUniform {
            glow: v4(
                lin(self.glow).map(|v| v * self.glow_strength),
                self.glow_tight.max(0.1),
            ),
            sun: v4(
                lin(self.sun).map(|v| v * self.sun_bright),
                self.sun_size.to_radians().cos(),
            ),
            moon: v4(
                lin(self.moon).map(|v| v * self.moon_bright),
                self.moon_size.to_radians().cos(),
            ),
            moon_dir: v4(self.moon_dir, self.moon_shown),
            cloud_lit: v4(lin(self.cloud_lit), self.cloud_cover),
            cloud_shade: v4(
                lin(self.cloud_shade),
                self.cloud_size.max(0.05).to_radians(),
            ),
            cloud: [self.cloud_speed.to_radians(), 0.0, 0.0, 0.0],
            ridge_near: v4(near, self.ridge_height),
            ridge_far: v4(far, self.ridge_haze.clamp(0.0, 1.0)),
            ridge_snow: v4(mul(lin(self.snow), lit), self.snow_amount),
            stars: [self.stars, 0.0, 0.0, 0.0],
        }
    }
}

/// Sky, sun and light for a time of day (0.0 sunrise, 0.25 noon, 0.5 sunset,
/// 0.75 midnight) seen from `at` in the world: the place's mood (mood.rs)
/// tints it and may pull the hour towards its own. `None`: no place, as on
/// the title screen and in model sheets.
pub fn sky_at(t: f32, at: Option<Vec3>) -> Sky {
    let s = look::settings();
    let m = mood::blend(&s, t, at);
    frame_sky(&s, &m, t + m.time_shift)
}

/// A sky for model sheets: the light of the hour with nothing behind the
/// figures but the gradient (no clouds, no ridges).
pub fn studio(t: f32) -> Sky {
    let mut s = sky_at(t, None);
    s.env.dome.ridge_height = 0.0;
    s.env.dome.cloud_cover = 0.0;
    s
}

fn frame_sky(s: &RenderSettings, m: &mood::Blend, t: f32) -> Sky {
    let a = t * std::f32::consts::TAU;
    let e = a.sin();
    let sun = Vec3::new(a.cos(), e, 0.3).normalize();
    // day: 0 at night .. 1 in full daylight
    let k = ((e + 0.1) / 0.42).clamp(0.0, 1.0);
    let k = k * k * (3.0 - 2.0 * k);
    // the sun at the horizon, and just under it
    let g = (-(e / s.sky.sunset_width.max(0.01)).powi(2)).exp();
    let dusk = (-((e + 0.10) / 0.13).powi(2)).exp();
    let add = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            (a[0] + b[0] * t).min(1.0),
            (a[1] + b[1] * t).min(1.0),
            (a[2] + b[2] * t).min(1.0),
        ]
    };
    let p = &s.sky;
    let mut top = lerp(p.night_top, p.day_top, k);
    let mut horizon = lerp(p.night_horizon, p.day_horizon, k);
    horizon = lerp(horizon, p.sunset_horizon, g * 0.75);
    top = add(top, p.sunset_top, g * 0.6);
    top = lerp(top, p.dusk_top, dusk * p.dusk);
    horizon = lerp(horizon, p.dusk_horizon, dusk * p.dusk);
    let mut light = lerp(p.night_light, p.day_light, k);
    light = lerp(light, p.sunset_light, g * 0.55);
    top = mul(top, m.sky_tint);
    horizon = mul(horizon, m.sky_tint);
    light = mul(light, m.light_tint);

    let l = &s.light;
    let moon_dir = Vec3::new(-sun.x, (-sun.y).max(0.0) * l.moon_arc + l.moon_lift, -sun.z)
        .normalize_or(Vec3::Y);
    // a place may hang the moon where it likes it (mood.rs)
    let (az, el) = (m.moon[0].to_radians(), m.moon[1].to_radians());
    let hung = Vec3::new(az.sin() * el.cos(), el.sin(), -az.cos() * el.cos());
    let moon_dir = moon_dir.lerp(hung, m.moon_mix).normalize_or(Vec3::Y);
    let step = |a: f32, b: f32, x: f32| {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    // shadows fade as the sun or the moon sinks to the horizon, where they
    // would stretch without end
    let (light_dir, shadow) = if e > 0.0 {
        (sun, l.shadow * step(0.02, 0.16, e))
    } else {
        (moon_dir, l.moon_shadow * step(0.03, 0.2, -e))
    };
    let a_ = &s.atmosphere;
    // mist gathers at dawn: the sun low in the east
    let dawn = g * a.cos().max(0.0);
    let haze = lerp(a_.haze_night, a_.haze_day, k);
    let fog = lerp(a_.fog_night, a_.fog_day, k);
    let d = &s.dome;
    let c = &d.clouds;
    let warm = (g * 0.85 + dusk * 0.6).min(1.0);
    let cloud_lit = lerp(lerp(c.night, c.day, k), c.sunset, warm * k.max(dusk));
    let cloud_shade = mul(cloud_lit, c.shade);
    let dome = Dome {
        glow: d.glow,
        glow_strength: d.glow_strength * (g + dusk * 0.5).min(1.0),
        glow_tight: d.glow_tight,
        sun: lerp(d.sun_low, d.sun, step(0.0, 0.4, e)),
        sun_bright: d.sun_bright * step(-0.08, -0.02, e),
        sun_size: d.sun_size,
        moon: d.moon,
        moon_bright: d.moon_bright,
        moon_size: d.moon_size,
        // (a moon a place has hung is drawn where it hangs)
        moon_dir: Vec3::new(
            moon_dir.x,
            moon_dir.y * (d.moon_height + (1.0 - d.moon_height) * m.moon_mix),
            moon_dir.z,
        )
        .normalize_or(Vec3::Y)
        .into(),
        moon_shown: 1.0 - k,
        cloud_lit: mul(cloud_lit, m.sky_tint),
        cloud_shade: mul(cloud_shade, m.sky_tint),
        cloud_cover: m.clouds,
        cloud_size: c.size,
        cloud_speed: c.speed,
        ridge: d.horizon.color,
        ridge_height: d.horizon.height,
        ridge_haze: d.horizon.haze,
        snow: d.horizon.snow,
        snow_amount: d.horizon.snow_amount,
        stars: d.stars * (1.0 - k).powi(3),
    };
    let g_ = &s.grade;
    let env = Frame {
        light_dir: light_dir.into(),
        shadow,
        ambient: lerp(l.ambient_night, l.ambient_day, k),
        ambient_strength: l.ambient * m.ambient,
        fill: l.fill,
        sun_tint: l.sun_tint,
        emissive: l.emissive,
        lamp: l.lamp,
        lamp_strength: l.lamp_night + (l.lamp_day - l.lamp_night) * k,
        lamp_hide: l.lamp_hide,
        rim: l.rim,
        rim_strength: l.rim_night + (l.rim_day - l.rim_night) * k,
        rim_power: l.rim_power,
        point: l.point,
        point_reach: l.point_reach,
        haze: lerp(haze, m.haze_color, m.haze_color_mix),
        haze_follow: (a_.haze_follow_night + (a_.haze_follow - a_.haze_follow_night) * k)
            * (1.0 - m.haze_color_mix),
        haze_density: a_.haze_density * m.haze,
        haze_start: a_.haze_start,
        haze_max: a_.haze_max,
        fog: lerp(fog, m.fog_color, m.fog_color_mix),
        fog_density: a_.fog_density * m.fog * (1.0 + (a_.fog_dawn - 1.0) * dawn),
        fog_height: a_.fog_height + m.fog_lift,
        fog_falloff: a_.fog_falloff,
        fog_max: a_.fog_max,
        fog_start: a_.fog_start,
        edge: a_.edge,
        dome,
        exposure: s.post.exposure * m.exposure,
        tint: mul(g_.tint, m.tint),
        saturation: g_.saturation
            * m.saturation
            * (g_.night_saturation + (1.0 - g_.night_saturation) * k),
        contrast: g_.contrast * m.contrast,
    };
    Sky {
        top,
        horizon,
        light,
        sun_dir: sun.into(),
        stars: 1.0 - k,
        cover: 0.0,
        env,
    }
}

/// The sky shader, after the `U` block and `atmos::ENV_WGSL`.
pub const SKY_WGSL: &str = r#"
struct SkyOut { @builtin(position) clip: vec4<f32>, @location(0) ndc: vec2<f32> };

// One triangle over the screen, at the far plane: the dome shows only where
// nothing was drawn.
@vertex fn vs_sky(@builtin(vertex_index) vi: u32) -> SkyOut {
  let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u)) * 2.0 - 1.0;
  var o: SkyOut;
  o.clip = vec4<f32>(p, 1.0, 1.0);
  o.ndc = p;
  return o;
}

// One row of clouds: heaps of square cells standing on a level base, as
// cumulus looks from the ground. x: how sunlit the cell is (0 underside ..
// 1 top); y: 1 where there is cloud.
fn cloud_row(az: f32, el: f32, base: f32, cs: f32, seed: f32, thr: f32) -> vec2<f32> {
  let y = (el - base) / cs;
  if (y < 0.0 || y > 16.0) { return vec2<f32>(0.0); }
  let a = (floor(az / cs) + 0.5) * cs;
  let p = vec2<f32>(cos(a), sin(a));
  // where the heaps stand, and the lumps on them
  let big = e_noise(p * (0.085 / cs) + vec2<f32>(seed, seed * 0.37));
  let lump = e_noise(p * (0.42 / cs) + vec2<f32>(seed * 1.7, 4.1));
  // a heap rises steeply at its edge and is lumpy on top, not a peak
  let hgt = smoothstep(0.0, 0.07, big - thr) * (2.0 + 4.5 * lump + 12.0 * max(big - thr, 0.0));
  let cy = floor(y);
  if (cy >= floor(hgt)) { return vec2<f32>(0.0); }
  return vec2<f32>(smoothstep(0.0, max(hgt * 0.5, 2.0), cy + 0.5), 1.0);
}

@fragment fn fs_sky(i: SkyOut) -> @location(0) vec4<f32> {
  let far = u.inv_view_proj * vec4<f32>(i.ndc, 1.0, 1.0);
  let dir = normalize(far.xyz / far.w - u.cam.xyz);
  let h = dir.y;
  let b = sky_backdrop(dir);
  var c = b.rgb;
  if (b.a > 0.5) { return vec4<f32>(c, 1.0); }
  // the sun: a soft glow and a disc bright enough to bloom
  let sd = dot(dir, u.sun.xyz);
  if (h > -0.02) {
    c += env.sun.rgb * (pow(max(sd, 0.0), 700.0) * 0.10 + pow(max(sd, 0.0), 40.0) * 0.02);
    if (sd > env.sun.w) { c = env.sun.rgb; }
  }
  // stars, twinkling
  if (env.stars.x > 0.01 && h > 0.0) {
    let cell = floor(dir * 190.0);
    let r = e_hash(vec2<f32>(cell.x + cell.z * 131.0, cell.y - cell.z * 57.0));
    if (r > 0.9945) {
      let tw = 0.75 + 0.25 * sin(u.cam.w * (1.0 + r * 3.0) + r * 900.0);
      c += vec3<f32>(0.9, 0.92, 1.0) * env.stars.x * (r - 0.9945) * 180.0 * tw * smoothstep(0.0, 0.2, h);
    }
  }
  // the moon: a disc of cells with darker seas, and a halo
  let shown = env.moon_dir.w;
  if (shown > 0.01) {
    let md = dot(dir, env.moon_dir.xyz);
    c += env.moon.rgb * (pow(max(md, 0.0), 500.0) * 0.10 + pow(max(md, 0.0), 30.0) * 0.015) * shown;
    if (md > env.moon.w) {
      let r = normalize(cross(env.moon_dir.xyz, vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(0.0001, 0.0, 0.0));
      let up = cross(r, env.moon_dir.xyz);
      let size = sqrt(max(1.0 - env.moon.w * env.moon.w, 0.000001));
      let p = vec2<f32>(dot(dir, r), dot(dir, up)) / size;
      let cell = floor(p * 5.0);
      let sea = step(0.7, e_hash(cell + vec2<f32>(40.0, 17.0)));
      c = mix(c, env.moon.rgb * (1.0 - 0.3 * sea), shown);
    }
  }
  // clouds: rows of heaped cells, the far rows low, small and hazed, the
  // near rows higher and larger, each drifting at its own pace
  let cover = clamp(env.cloud_lit.w + u.sky_top.w * 0.7, 0.0, 1.0);
  if (h > 0.02 && h < 0.6 && cover > 0.0) {
    let az = atan2(dir.x, dir.z);
    let el = asin(h);
    let cs = env.cloud_shade.w;
    let thr = mix(0.86, 0.30, cover);
    let t = u.cam.w * env.cloud.x;
    for (var r = 0; r < 3; r++) {
      let k = f32(r);
      let row = cloud_row(az + t * (0.5 + 0.25 * k), el, 0.15 + 0.11 * k + 0.02 * k * k, cs * (0.6 + 0.25 * k), 11.3 + 37.7 * k, thr + 0.02 * (2.0 - k));
      if (row.y > 0.0) {
        let col = mix(env.cloud_shade.rgb, env.cloud_lit.rgb, row.x);
        // far rows sink into the haze
        c = mix(c, col, 0.66 + 0.15 * k);
      }
    }
  }
  return vec4<f32>(c, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_is_lit_by_a_moon_above_the_horizon() {
        let s = RenderSettings::default();
        let m = mood::Blend::plain(&s);
        for i in 0..100 {
            let sky = frame_sky(&s, &m, i as f32 / 100.0);
            assert!(sky.env.light_dir[1] >= 0.0, "light from below at {i}");
            assert!((0.0..=1.0).contains(&sky.env.shadow));
        }
    }
}
