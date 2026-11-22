//! wgpu renderer: voxel sections with ambient occlusion, fog, a block highlight
//! and simple boxes for other players.

use crate::light::Light;
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

/// The block textures, built once.
pub fn textures() -> &'static crate::textures::Textures {
    static T: std::sync::OnceLock<crate::textures::Textures> = std::sync::OnceLock::new();
    T.get_or_init(crate::textures::Textures::blocks)
}

/// Texture coordinates for one face corner, plus the layer and animation.
fn face_tex(b: BlockId, face: usize, u: f32, v: f32) -> [f32; 4] {
    let t = textures();
    let layer = t.face(b, face);
    let anim = t
        .animated
        .iter()
        .find(|(first, _, _)| *first == layer)
        .map(|(_, n, spf)| *n as f32 + (spf / 100.0).min(0.99))
        .unwrap_or(0.0);
    [u, v, layer as f32, anim]
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
}

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, inv_view_proj: mat4x4<f32>, cam: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, light: vec4<f32>, sun: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;

@group(0) @binding(1) var tex_array: texture_2d_array<f32>;
@group(0) @binding(2) var tex_samp: sampler;

struct VIn { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32>, @location(3) light: f32, @location(4) sky: f32, @location(5) tex: vec4<f32> };
struct VOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) wpos: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) color: vec4<f32>,
  @location(3) light: f32,
  @location(4) sky: f32,
  @location(5) tex: vec4<f32>,
};

@vertex fn vs(v: VIn) -> VOut {
  var o: VOut;
  o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
  o.wpos = v.pos;
  o.normal = v.normal;
  o.color = v.color;
  o.light = v.light;
  o.sky = v.sky;
  o.tex = v.tex;
  return o;
}

fn hash3(p: vec3<f32>) -> f32 {
  return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

// The texel for a textured vertex, with animated layers stepping through frames.
fn texel(t: vec4<f32>) -> vec4<f32> {
  var layer = t.z;
  let frames = floor(t.w);
  if (frames > 1.0) {
    let spf = max(fract(t.w) * 100.0, 0.02);
    layer = layer + (floor(u.cam.w / spf) % frames);
  }
  return textureSample(tex_array, tex_samp, t.xy, i32(layer + 0.5));
}

@fragment fn fs(i: VOut) -> @location(0) vec4<f32> {
  let tx = texel(i.tex);
  if (i.tex.z >= 0.0 && tx.a < 0.5) { discard; }
  // 16 x 16 texel grain on untextured boxes so they read as pixel art
  let t = floor(i.wpos * 16.0 - i.normal * 0.01);
  let grain = select(1.0, 0.86 + 0.24 * hash3(t), dot(i.normal, i.normal) > 0.5 && i.tex.z < 0.0);
  // daylight (u.cam.w) or warm block light, whichever is brighter
  let bl = i.light * i.light;
  let sl = i.sky * i.sky;
  let lit = max(u.light.rgb * (sl * 0.97 + 0.03), vec3<f32>(1.0, 0.80, 0.55) * bl * 1.1);
  var base = i.color.rgb;
  if (i.tex.z >= 0.0) { base = base * tx.rgb; }
  let pt = 0;
  if (pt > 0) {
    let h = hash3(t + vec3<f32>(17.0, 3.0, 11.0));
    let h2 = hash3(floor(t / 2.0) + vec3<f32>(5.0, 9.0, 1.0));
    if (pt == 1 && h2 > 0.78) { base = vec3<f32>(0.05, 0.05, 0.06); }
    if (pt == 2 && h2 > 0.80) { base = vec3<f32>(0.80, 0.58, 0.40); }
    if (pt == 3 && h2 > 0.55) { base = vec3<f32>(1.0, 0.75, 0.35); }
    if (pt == 4) {
      // wood: darker line every 4 texels across the grain
      let g = select(t.y, t.x + t.z, abs(i.normal.y) > 0.5);
      if (fract(g / 4.0) < 0.2) { base = base * 0.78; }
    }
    if (pt == 5) {
      // stonework: mortar lines
      let row = floor(t.y / 4.0);
      let col = select(t.x, t.z, abs(i.normal.x) > 0.5) + select(0.0, 4.0, fract(row / 2.0) > 0.4);
      if (fract(t.y / 4.0) < 0.2 || fract(col / 8.0) < 0.1) { base = base * 0.72; }
    }
    if (pt == 6 && h > 0.92) { base = vec3<f32>(0.86, 0.12, 0.22); }
  }
  var c = base * grain * min(i.color.a, 1.0) * lit;
  if (i.color.a > 1.9) { c = base * grain * 1.1; }
  let d = distance(i.wpos, u.cam.xyz);
  let f = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  c = mix(c, u.fog.rgb, f);
  return vec4<f32>(c, 1.0);
}

fn hash2(p: vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
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
  // blocky clouds drifting across the sky
  if (h > 0.015) {
    let q = dir.xz / (h + 0.06) * 1.4 + vec2<f32>(u.cam.w * 0.012, u.cam.w * 0.005);
    let qb = floor(q * 10.0) / 10.0;
    let n = vnoise(qb * 0.9) * 0.62 + vnoise(qb * 2.1 + 7.3) * 0.38;
    let cov = smoothstep(0.55, 0.66, n) * smoothstep(0.015, 0.16, h);
    let cc = mix(u.fog.rgb, vec3<f32>(1.0, 1.0, 1.0), 0.55) * (u.light.rgb * 0.85 + 0.15);
    c = mix(c, cc, cov * 0.9);
  }
  // stars
  if (u.sun.w > 0.01 && h > 0.0) {
    let cell = floor(dir * 220.0);
    let r = hash3(cell);
    if (r > 0.9965) { c += vec3<f32>(0.9, 0.92, 1.0) * u.sun.w * (r - 0.9965) * 220.0 * smoothstep(0.0, 0.2, h); }
  }
  return vec4<f32>(c, 1.0);
}

@fragment fn fs_translucent(i: VOut) -> @location(0) vec4<f32> {
  let d = distance(i.wpos, u.cam.xyz);
  let f = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  let lit = max(u.light.rgb * (i.sky * i.sky * 0.97 + 0.03), vec3<f32>(1.0, 0.80, 0.55) * i.light * i.light);
  let tx = texel(i.tex);
  var c = i.color.rgb * lit;
  var a = i.color.a;
  if (i.tex.z >= 0.0) {
    c = c * tx.rgb;
    a = tx.a;
    if (a < 0.02) { discard; }
  }
  // water (alpha 0.62): moving shimmer, and the sky reflects at grazing angles
  if (abs(i.color.a - 0.62) < 0.02 && i.normal.y > 0.5) {
    let t = u.cam.w;
    let w = sin(i.wpos.x * 1.7 + t * 1.6) * cos(i.wpos.z * 1.3 - t * 1.2) + 0.5 * sin((i.wpos.x + i.wpos.z) * 2.9 + t * 2.3);
    c = c * (1.0 + 0.07 * w);
    let v = normalize(u.cam.xyz - i.wpos);
    let fres = pow(1.0 - clamp(v.y, 0.0, 1.0), 4.0);
    c = mix(c, mix(u.sky_top.rgb, u.fog.rgb, 0.3) * (u.light.rgb * 0.5 + 0.5), fres * 0.45);
    a = mix(a, 0.85, fres);
    let sd = max(dot(reflect(-v, vec3<f32>(0.0, 1.0, 0.0)), u.sun.xyz), 0.0);
    c += vec3<f32>(1.0, 0.85, 0.6) * pow(sd, 120.0) * 0.8 * (1.0 - u.sun.w) * (0.6 + 0.4 * w);
  }
  c = mix(c, u.fog.rgb, f);
  return vec4<f32>(c, a);
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

fn translucent(b: BlockId) -> bool {
    w::block(b).alpha < 1.0
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
    let mut opaque = Vec::new();
    let mut trans = Vec::new();
    let solid = |x: i32, y: i32, z: i32| w::is_opaque(world.get(x, y, z)) as i32;
    for y in sy * SECTION..(sy + 1) * SECTION {
        for z in sz * SECTION..(sz + 1) * SECTION {
            for x in sx * SECTION..(sx + 1) * SECTION {
                let b = world.get(x, y, z);
                if b == w::AIR {
                    continue;
                }
                if w::shape(b) == w::Shape::Post {
                    let c = Vec3::new(x as f32 + 0.5, y as f32 + 11.0 / 32.0, z as f32 + 0.5);
                    let l = light.get(x, y, z) as f32 / 15.0;
                    push_post(&mut opaque, c, b, l);
                    continue;
                }
                let alpha = w::block(b).alpha;
                let tr = translucent(b);
                let emissive = w::emission(b) > 0;
                for (n, u, v, shade) in FACES {
                    let nb = world.get(x + n[0], y + n[1], z + n[2]);
                    if w::is_opaque(nb) || (tr && nb == b) || (w::is_leaves(b) && w::is_leaves(nb))
                    {
                        continue;
                    }
                    let base = [x + n[0], y + n[1], z + n[2]];
                    let center = [
                        x as f32 + 0.5 + n[0] as f32 * 0.5,
                        y as f32 + 0.5 + n[1] as f32 * 0.5,
                        z as f32 + 0.5 + n[2] as f32 * 0.5,
                    ];
                    let mut corners = [([0f32; 3], 0f32, 0f32, 0f32); 4];
                    for (i, (su, sv)) in
                        [(-1, -1), (1, -1), (1, 1), (-1, 1)].into_iter().enumerate()
                    {
                        let p = [
                            center[0] + 0.5 * (su * u[0] + sv * v[0]) as f32,
                            center[1] + 0.5 * (su * u[1] + sv * v[1]) as f32,
                            center[2] + 0.5 * (su * u[2] + sv * v[2]) as f32,
                        ];
                        let a = [
                            base[0] + su * u[0],
                            base[1] + su * u[1],
                            base[2] + su * u[2],
                        ];
                        let bq = [
                            base[0] + sv * v[0],
                            base[1] + sv * v[1],
                            base[2] + sv * v[2],
                        ];
                        let cq = [
                            base[0] + su * u[0] + sv * v[0],
                            base[1] + su * u[1] + sv * v[1],
                            base[2] + su * u[2] + sv * v[2],
                        ];
                        // smooth block light: average the light of the cells around this corner
                        let l0 = light.get(base[0], base[1], base[2]) as f32;
                        let ls = [
                            l0,
                            light.get(a[0], a[1], a[2]) as f32,
                            light.get(bq[0], bq[1], bq[2]) as f32,
                            light.get(cq[0], cq[1], cq[2]) as f32,
                        ];
                        let bl = (ls.iter().sum::<f32>() / 60.0).max(l0 / 15.0 * 0.8);
                        let k0 = light.sky(base[0], base[1], base[2]) as f32;
                        let ks = [
                            k0,
                            light.sky(a[0], a[1], a[2]) as f32,
                            light.sky(bq[0], bq[1], bq[2]) as f32,
                            light.sky(cq[0], cq[1], cq[2]) as f32,
                        ];
                        let sl = (ks.iter().sum::<f32>() / 60.0).max(k0 / 15.0 * 0.8);
                        let shade_ao = if emissive {
                            2.0
                        } else if tr {
                            alpha
                        } else {
                            let (s1, s2, c) = (
                                solid(a[0], a[1], a[2]),
                                solid(bq[0], bq[1], bq[2]),
                                solid(cq[0], cq[1], cq[2]),
                            );
                            let ao = if s1 == 1 && s2 == 1 {
                                0
                            } else {
                                3 - (s1 + s2 + c)
                            };
                            [0.5, 0.66, 0.83, 1.0][ao as usize] * shade
                        };
                        corners[i] = (p, shade_ao, bl, sl);
                    }
                    let nf = [n[0] as f32, n[1] as f32, n[2] as f32];
                    // textured: the texture carries the colour
                    let col = if tr {
                        [shade, shade, shade]
                    } else {
                        [1.0, 1.0, 1.0]
                    };
                    let face_id = match n[1] {
                        1 => 0,
                        -1 => 2,
                        _ => 1,
                    };
                    // flip the diagonal so ambient occlusion interpolates without seams
                    let order = if corners[0].1 + corners[2].1 > corners[1].1 + corners[3].1 {
                        [0, 1, 2, 0, 2, 3]
                    } else {
                        [1, 2, 3, 1, 3, 0]
                    };
                    let out = if tr { &mut trans } else { &mut opaque };
                    for i in order {
                        let (p, l, bl, sl) = corners[i];
                        let (su, sv) =
                            [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][i];
                        // sides: v runs down from the top; mirror u on the far faces
                        let (tu, tv) = if face_id == 1 {
                            let uu = (su + 1.0) * 0.5;
                            (
                                if n[0] + n[2] < 0 { uu } else { 1.0 - uu },
                                (1.0 - sv) * 0.5,
                            )
                        } else {
                            ((su + 1.0) * 0.5, (sv + 1.0) * 0.5)
                        };
                        out.push(Vertex {
                            pos: p,
                            normal: nf,
                            color: [col[0], col[1], col[2], l],
                            light: bl,
                            sky: sl,
                            tex: face_tex(b, face_id, tu, tv),
                        });
                    }
                }
            }
        }
    }
    (opaque, trans)
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

/// Draw a player: legs and arms swing with `walk` (0 standing, 1 walking).
#[allow(clippy::too_many_arguments)]
pub fn push_avatar(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    color: [f32; 3],
    light: f32,
    phase: f32,
    walk: f32,
) {
    let (sy, cy) = yaw.sin_cos();
    let right = Vec3::new(cy, 0.0, -sy);
    let fwd = Vec3::new(-sy, 0.0, -cy);
    let up = Vec3::Y;
    let skin = [0.91, 0.76, 0.62];
    let pants = [color[0] * 0.45, color[1] * 0.45, color[2] * 0.5];
    let swing = phase.sin() * 0.7 * walk;
    let limb = |out: &mut Vec<Vertex>, pivot: Vec3, angle: f32, len: f32, w: f32, rgb: [f32; 3]| {
        let (s, c) = angle.sin_cos();
        let u2 = up * c + fwd * s;
        let f2 = fwd * c - up * s;
        render_limb(
            out,
            pivot - u2 * (len * 0.5),
            [right, u2, f2],
            Vec3::new(w, len * 0.5, w),
            rgb,
            light,
        );
    };
    // legs
    let hip = feet + up * 0.75;
    limb(out, hip + right * 0.12, swing, 0.75, 0.115, pants);
    limb(out, hip - right * 0.12, -swing, 0.75, 0.115, pants);
    // body
    push_oriented_box(
        out,
        feet + up * 1.1,
        [right, up, fwd],
        Vec3::new(0.25, 0.35, 0.13),
        color,
        false,
        light,
    );
    // arms
    let shoulder = feet + up * 1.42;
    limb(out, shoulder + right * 0.36, -swing * 0.8, 0.66, 0.1, color);
    limb(out, shoulder - right * 0.36, swing * 0.8, 0.66, 0.1, color);
    // head, tilted by pitch
    let (sp, cp) = (pitch * 0.6).sin_cos();
    let hu = up * cp - fwd * sp;
    let hf = fwd * cp + up * sp;
    let head = feet + up * 1.71;
    push_oriented_box(
        out,
        head,
        [right, hu, hf],
        Vec3::splat(0.24),
        skin,
        false,
        light,
    );
    // eyes
    for side in [-1.0f32, 1.0] {
        push_oriented_box(
            out,
            head + hf * 0.245 + right * (0.09 * side) + hu * 0.03,
            [right, hu, hf],
            Vec3::new(0.04, 0.045, 0.01),
            [0.12, 0.12, 0.16],
            false,
            light,
        );
    }
}

/// Draw a creature. `flash` (0 to 1) whitens it when hit; `t` is a clock in seconds.
#[allow(clippy::too_many_arguments)]
pub fn push_creature(
    out: &mut Vec<Vertex>,
    model: dyno_world::content::Model,
    feet: Vec3,
    yaw: f32,
    color: [f32; 3],
    light: f32,
    phase: f32,
    walk: f32,
    flash: f32,
    t: f32,
) {
    use dyno_world::content::Model;
    let (sy, cy) = yaw.sin_cos();
    let right = Vec3::new(cy, 0.0, -sy);
    let fwd = Vec3::new(-sy, 0.0, -cy);
    let up = Vec3::Y;
    let hit = |c: [f32; 3]| -> [f32; 3] {
        [
            c[0] + (1.0 - c[0]) * flash,
            c[1] + (1.0 - c[1]) * flash,
            c[2] + (1.0 - c[2]) * flash,
        ]
    };
    let lit = if flash > 0.0 { 1.0 } else { light };
    let bx =
        |out: &mut Vec<Vertex>, c: Vec3, axes: [Vec3; 3], h: Vec3, rgb: [f32; 3], glow: bool| {
            push_oriented_box(out, c, axes, h, hit(rgb), glow || flash > 0.5, lit);
        };
    let swing = phase.sin() * 0.6 * walk;
    match model {
        Model::Quadruped => {
            // a stocky, woolly grazer with a dark face and curled horns
            let wool = color;
            let wool_hi = [color[0] * 1.08, color[1] * 1.08, color[2] * 1.06];
            let face = [0.34, 0.26, 0.21];
            let hoof = [0.24, 0.19, 0.16];
            let horn = [0.92, 0.86, 0.68];
            let body = feet + up * 0.78;
            let axes = [right, up, fwd];
            // legs
            for (sx, sz, ph) in [
                (1.0f32, 1.0f32, 0.0f32),
                (-1.0, 1.0, std::f32::consts::PI),
                (1.0, -1.0, std::f32::consts::PI),
                (-1.0, -1.0, 0.0),
            ] {
                let hip = feet + up * 0.52 + right * (0.2 * sx) + fwd * (0.32 * sz);
                let a = (phase + ph).sin() * 0.55 * walk;
                let (s, c) = a.sin_cos();
                let u2 = up * c + fwd * s;
                let f2 = fwd * c - up * s;
                bx(
                    out,
                    hip - u2 * 0.26,
                    [right, u2, f2],
                    Vec3::new(0.075, 0.26, 0.075),
                    hoof,
                    false,
                );
            }
            // body, with a lighter fleece on top
            bx(out, body, axes, Vec3::new(0.33, 0.27, 0.48), wool, false);
            bx(
                out,
                body + up * 0.2 + fwd * 0.04,
                axes,
                Vec3::new(0.29, 0.12, 0.4),
                wool_hi,
                false,
            );
            // tail
            bx(
                out,
                body - fwd * 0.5 + up * 0.1,
                axes,
                Vec3::new(0.07, 0.09, 0.05),
                wool_hi,
                false,
            );
            // head: grazing dips it now and then when standing
            let graze = if walk < 0.15 {
                ((t * 0.7 + phase * 0.1).sin() - 0.55).max(0.0) * 1.6
            } else {
                0.0
            };
            let neck = body + fwd * 0.5 + up * 0.12;
            let (gs, gc) = (graze * 0.9).sin_cos();
            let hu = up * gc + fwd * gs;
            let hf = fwd * gc - up * gs;
            let head = neck + hf * 0.12 + hu * (0.1 - graze * 0.25);
            bx(
                out,
                head,
                [right, hu, hf],
                Vec3::new(0.16, 0.16, 0.2),
                face,
                false,
            );
            // fleece cap and horns
            bx(
                out,
                head + hu * 0.15 - hf * 0.03,
                [right, hu, hf],
                Vec3::new(0.17, 0.06, 0.14),
                wool_hi,
                false,
            );
            for side in [-1.0f32, 1.0] {
                bx(
                    out,
                    head + right * (0.2 * side) + hu * 0.08 - hf * 0.02,
                    [right, hu, hf],
                    Vec3::new(0.05, 0.07, 0.07),
                    horn,
                    false,
                );
                bx(
                    out,
                    head + right * (0.22 * side) - hu * 0.0 + hf * 0.06,
                    [right, hu, hf],
                    Vec3::new(0.04, 0.04, 0.05),
                    horn,
                    false,
                );
                // eyes
                bx(
                    out,
                    head + hf * 0.205 + right * (0.08 * side) + hu * 0.04,
                    [right, hu, hf],
                    Vec3::new(0.025, 0.03, 0.01),
                    [0.06, 0.05, 0.05],
                    false,
                );
            }
            // muzzle
            bx(
                out,
                head + hf * 0.2 - hu * 0.07,
                [right, hu, hf],
                Vec3::new(0.1, 0.06, 0.03),
                [0.5, 0.4, 0.34],
                false,
            );
        }
        Model::Humanoid => {
            // a Shade: a hooded, legless shape of darkness with two pale eyes
            let bob = (t * 2.1 + phase * 0.05).sin() * 0.07;
            let base = feet + up * (0.12 + bob);
            let dark = color;
            let edge = [color[0] * 1.6, color[1] * 1.4, color[2] * 1.9];
            let axes = [right, up, fwd];
            let lean = 0.18 * walk;
            let (ls, lc) = lean.sin_cos();
            let bu = up * lc + fwd * ls;
            let bf = fwd * lc - up * ls;
            // tattered lower robe, narrowing to wisps
            bx(
                out,
                base + up * 0.55,
                [right, bu, bf],
                Vec3::new(0.23, 0.32, 0.17),
                dark,
                false,
            );
            for (i, (sx, ph)) in [(-0.13f32, 0.0f32), (0.0, 2.1), (0.13, 4.2)]
                .iter()
                .enumerate()
            {
                let sway = (t * 3.0 + ph).sin() * 0.05;
                let len = 0.16 + 0.05 * i as f32;
                bx(
                    out,
                    base + up * (0.2 - len * 0.5) + right * (sx + sway) - fwd * 0.04,
                    axes,
                    Vec3::new(0.06, len, 0.06),
                    dark,
                    false,
                );
            }
            // chest and shoulders
            let chest = base + bu * 1.08;
            bx(
                out,
                chest,
                [right, bu, bf],
                Vec3::new(0.27, 0.3, 0.16),
                dark,
                false,
            );
            bx(
                out,
                chest + bu * 0.26,
                [right, bu, bf],
                Vec3::new(0.3, 0.06, 0.17),
                edge,
                false,
            );
            // long reaching arms
            let reach = 0.7 + 0.5 * walk;
            for side in [-1.0f32, 1.0] {
                let a = -(reach + (t * 2.4 + side).sin() * 0.12 + swing * side * 0.3);
                let (s, c) = a.sin_cos();
                let u2 = bu * c + bf * (-s);
                let f2 = bf * c + bu * s;
                let shoulder = chest + bu * 0.22 + right * (0.33 * side);
                bx(
                    out,
                    shoulder - u2 * 0.4,
                    [right, u2, f2],
                    Vec3::new(0.065, 0.4, 0.065),
                    dark,
                    false,
                );
                // pale fingertips
                bx(
                    out,
                    shoulder - u2 * 0.82,
                    [right, u2, f2],
                    Vec3::new(0.07, 0.05, 0.07),
                    edge,
                    false,
                );
            }
            // hood and face
            let head = chest + bu * 0.55 + bf * 0.06;
            bx(
                out,
                head,
                [right, bu, bf],
                Vec3::new(0.22, 0.24, 0.22),
                dark,
                false,
            );
            bx(
                out,
                head + bu * 0.2 - bf * 0.06,
                [right, bu, bf],
                Vec3::new(0.2, 0.08, 0.2),
                edge,
                false,
            );
            bx(
                out,
                head + bf * 0.2,
                [right, bu, bf],
                Vec3::new(0.16, 0.16, 0.03),
                [0.03, 0.02, 0.05],
                false,
            );
            // the eyes: glowing, so they show in the dark
            let blink = if (t * 0.37 + phase * 0.013).fract() < 0.03 {
                0.2
            } else {
                1.0
            };
            for side in [-1.0f32, 1.0] {
                push_oriented_box(
                    out,
                    head + bf * 0.235 + right * (0.075 * side) + bu * 0.02,
                    [right, bu, bf],
                    Vec3::new(0.035, 0.022 * blink, 0.01),
                    hit([0.86, 0.72, 1.0]),
                    true,
                    1.0,
                );
            }
        }
        Model::Orb => {
            let c = feet + up * (0.8 + (t * 1.7).sin() * 0.15);
            bx(out, c, [right, up, fwd], Vec3::splat(0.16), color, true);
            bx(
                out,
                c,
                [right, up, fwd],
                Vec3::splat(0.22),
                [color[0] * 0.5, color[1] * 0.5, color[2] * 0.5],
                false,
            );
        }
    }
}

/// A torch-like post: a narrow textured stick, 3/16 wide and 11/16 tall, glowing.
fn push_post(out: &mut Vec<Vertex>, center: Vec3, b: BlockId, light: f32) {
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
                (u0 + (u1 - u0) * (su + 1.0) * 0.5, (1.0 - sv) * 0.5)
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
                ((su + 1.0) * 0.5, (1.0 - sv) * 0.5)
            } else {
                ((su + 1.0) * 0.5, (sv + 1.0) * 0.5)
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
) -> (wgpu::TextureView, wgpu::Sampler) {
    use crate::textures::TILE;
    let t = textures();
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
                        let v = if acc[3] > 0.0 { acc[k] / acc[3] } else { 0.0 };
                        next[o + k] = (v.powf(1.0 / 2.2) * 255.0).round() as u8;
                    }
                    next[o + 3] = (acc[3] / 4.0 * 255.0).round() as u8;
                }
            }
            img = next;
            size = half;
        }
    }
    let view = tex.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let samp = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("pixels"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    (view, samp)
}

fn render_limb(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axes: [Vec3; 3],
    half: Vec3,
    rgb: [f32; 3],
    light: f32,
) {
    push_oriented_box(out, center, axes, half, rgb, false, light);
}

struct SectionMesh {
    opaque: Option<(wgpu::Buffer, u32)>,
    trans: Option<(wgpu::Buffer, u32)>,
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
    sky_pipe: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    sections: Vec<SectionMesh>,
    dims: [i32; 3],
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
        let (tex_view, tex_samp) = upload_textures(&device, &queue);
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
            ],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubuf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&tex_samp),
                },
            ],
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
            sky_pipe,
            ubuf,
            bind,
            sections: Vec::new(),
            dims: [0; 3],
            adapter_info,
            screenshot: None,
            started: std::time::Instant::now(),
        })
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
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(v),
                usage: wgpu::BufferUsages::VERTEX,
            });
        Some((buf, v.len() as u32))
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
        for sy in 0..d[1] {
            for sz in 0..d[2] {
                for sx in 0..d[0] {
                    let (o, t) = mesh_section(world, light, sx, sy, sz);
                    self.sections.push(SectionMesh {
                        opaque: self.upload(&o),
                        trans: self.upload(&t),
                    });
                }
            }
        }
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
        let s0 = |v: i32, i: usize| ((v - margin).div_euclid(SECTION)).clamp(0, self.dims[i] - 1);
        let s1 = |v: i32, i: usize| ((v + margin).div_euclid(SECTION)).clamp(0, self.dims[i] - 1);
        for sy in s0(lo[1], 1)..=s1(hi[1], 1) {
            for sz in s0(lo[2], 2)..=s1(hi[2], 2) {
                for sx in s0(lo[0], 0)..=s1(hi[0], 0) {
                    let (o, t) = mesh_section(world, light, sx, sy, sz);
                    let i = ((sy * self.dims[2] + sz) * self.dims[0] + sx) as usize;
                    self.sections[i] = SectionMesh {
                        opaque: self.upload(&o),
                        trans: self.upload(&t),
                    };
                }
            }
        }
    }

    /// Re-mesh the section that holds a cell, and its neighbours when the cell is on an edge.
    pub fn block_changed(&mut self, world: &World, light: &Light, p: [i32; 3], radius: i32) {
        let mut todo = Vec::new();
        for dx in [-radius, 0, radius] {
            for dy in [-radius, 0, radius] {
                for dz in [-radius, 0, radius] {
                    let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                    let s = [
                        q[0].div_euclid(SECTION),
                        q[1].div_euclid(SECTION),
                        q[2].div_euclid(SECTION),
                    ];
                    if (0..3).all(|i| s[i] >= 0 && s[i] < self.dims[i]) && !todo.contains(&s) {
                        todo.push(s);
                    }
                }
            }
        }
        for s in todo {
            let (o, t) = mesh_section(world, light, s[0], s[1], s[2]);
            let i = ((s[1] * self.dims[2] + s[2]) * self.dims[0] + s[0]) as usize;
            self.sections[i] = SectionMesh {
                opaque: self.upload(&o),
                trans: self.upload(&t),
            };
        }
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
                sky_top: [top[0], top[1], top[2], 0.0],
                light: [light[0], light[1], light[2], 0.0],
                sun: [sky.sun_dir[0], sky.sun_dir[1], sky.sun_dir[2], sky.stars],
            };
            self.queue
                .write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let box_buf = self.upload(boxes);
        let line_buf = self.upload(lines);

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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
                    if let Some((b, n)) = &s.opaque
                        && visible(i)
                    {
                        rp.set_vertex_buffer(0, b.slice(..));
                        rp.draw(0..*n, 0..1);
                    }
                }
                if let Some((b, n)) = &box_buf {
                    rp.set_vertex_buffer(0, b.slice(..));
                    rp.draw(0..*n, 0..1);
                }
                rp.set_pipeline(&self.trans_pipe);
                for (i, s) in self.sections.iter().enumerate() {
                    if let Some((b, n)) = &s.trans
                        && visible(i)
                    {
                        rp.set_vertex_buffer(0, b.slice(..));
                        rp.draw(0..*n, 0..1);
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

/// Draw the item held in the first-person hand. `ax` are the hand's axes
/// (right, up, forward) and `hand` its centre.
pub fn push_held(
    out: &mut Vec<Vertex>,
    c: &dyno_world::content::Content,
    item: Option<dyno_world::content::ItemId>,
    hand: Vec3,
    ax: [Vec3; 3],
    light: f32,
) {
    use dyno_world::content::ToolKind;
    let skin = [0.91, 0.76, 0.62];
    let Some(item) = item else {
        push_oriented_box(
            out,
            hand - ax[1] * 0.06,
            ax,
            Vec3::new(0.06, 0.18, 0.06),
            skin,
            false,
            light,
        );
        return;
    };
    let it = c.item(item);
    let glow = it.light > 0 || it.block.is_some_and(|b| w::emission(b) > 0);
    if let Some(b) = it.block {
        let _ = glow;
        if w::shape(b) == w::Shape::Post {
            push_post_oriented(out, hand + ax[1] * 0.02, ax, 0.45, b, light);
        } else {
            push_block_cube(out, hand, ax, 0.11, b, light);
        }
        return;
    }
    // tilt tools forward so the head points up and away
    let (s, k) = (-0.55f32).sin_cos();
    let hu = ax[1] * k + ax[2] * s;
    let hf = ax[2] * k - ax[1] * s;
    let axes = [ax[0], hu, hf];
    let wood = [0.55, 0.40, 0.24];
    match it.tool.map(|t| t.kind) {
        Some(kind) => {
            if kind != ToolKind::Sword {
                push_oriented_box(
                    out,
                    hand + hu * 0.04,
                    axes,
                    Vec3::new(0.022, 0.17, 0.022),
                    wood,
                    false,
                    light,
                );
            }
            match kind {
                ToolKind::Pickaxe => push_oriented_box(
                    out,
                    hand + hu * 0.2,
                    axes,
                    Vec3::new(0.14, 0.028, 0.03),
                    it.color,
                    glow,
                    light,
                ),
                ToolKind::Axe => push_oriented_box(
                    out,
                    hand + hu * 0.16 + ax[0] * 0.05,
                    axes,
                    Vec3::new(0.065, 0.06, 0.02),
                    it.color,
                    glow,
                    light,
                ),
                ToolKind::Shovel | ToolKind::Hoe => push_oriented_box(
                    out,
                    hand + hu * 0.25,
                    axes,
                    Vec3::new(0.05, 0.07, 0.014),
                    it.color,
                    glow,
                    light,
                ),
                ToolKind::Sword => {
                    push_oriented_box(
                        out,
                        hand - hu * 0.06,
                        axes,
                        Vec3::new(0.02, 0.06, 0.02),
                        wood,
                        false,
                        light,
                    );
                    push_oriented_box(
                        out,
                        hand + hu * 0.0,
                        axes,
                        Vec3::new(0.08, 0.014, 0.022),
                        wood,
                        false,
                        light,
                    );
                    push_oriented_box(
                        out,
                        hand + hu * 0.22,
                        axes,
                        Vec3::new(0.03, 0.21, 0.01),
                        it.color,
                        glow,
                        light,
                    );
                }
            }
        }
        None => push_oriented_box(
            out,
            hand,
            axes,
            Vec3::new(0.08, 0.08, 0.02),
            it.color,
            glow,
            light,
        ),
    }
}
