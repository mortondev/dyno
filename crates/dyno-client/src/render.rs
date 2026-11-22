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

struct VIn { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32>, @location(3) light: f32 };
struct VOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) wpos: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) color: vec4<f32>,
  @location(3) light: f32,
};

@vertex fn vs(v: VIn) -> VOut {
  var o: VOut;
  o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
  o.wpos = v.pos;
  o.normal = v.normal;
  o.color = v.color;
  o.light = v.light;
  return o;
}

fn hash3(p: vec3<f32>) -> f32 {
  return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

@fragment fn fs(i: VOut) -> @location(0) vec4<f32> {
  // 16 x 16 texel grain so blocks read as pixel-art surfaces
  let t = floor(i.wpos * 16.0 - i.normal * 0.01);
  let grain = select(1.0, 0.86 + 0.24 * hash3(t), dot(i.normal, i.normal) > 0.5);
  // daylight (u.cam.w) or warm block light, whichever is brighter
  let bl = i.light * i.light;
  let lit = max(u.light.rgb, vec3<f32>(1.0, 0.80, 0.55) * bl * 1.1);
  var c = i.color.rgb * grain * min(i.color.a, 1.0) * lit;
  if (i.color.a > 1.9) { c = i.color.rgb * grain * 1.1; }
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
  let lit = max(u.light.rgb, vec3<f32>(1.0, 0.80, 0.55) * i.light * i.light);
  let c = mix(i.color.rgb * lit, u.fog.rgb, f);
  return vec4<f32>(c, i.color.a);
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
                let (rgb, alpha) = block_color(b);
                let lin = srgb(rgb);
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
                    let mut corners = [([0f32; 3], 0f32, 0f32); 4];
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
                        corners[i] = (p, shade_ao, bl);
                    }
                    let nf = [n[0] as f32, n[1] as f32, n[2] as f32];
                    let col = if tr {
                        [lin[0] * shade, lin[1] * shade, lin[2] * shade]
                    } else {
                        lin
                    };
                    // flip the diagonal so ambient occlusion interpolates without seams
                    let order = if corners[0].1 + corners[2].1 > corners[1].1 + corners[3].1 {
                        [0, 1, 2, 0, 2, 3]
                    } else {
                        [1, 2, 3, 1, 3, 0]
                    };
                    let out = if tr { &mut trans } else { &mut opaque };
                    for i in order {
                        let (p, l, bl) = corners[i];
                        out.push(Vertex {
                            pos: p,
                            normal: nf,
                            color: [col[0], col[1], col[2], l],
                            light: bl,
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
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let vlayout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32],
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
