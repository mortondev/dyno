//! After the world is drawn: the passes that finish the picture. The world
//! is drawn into a picture that can hold light brighter than white; then
//!
//! - **ambient occlusion** darkens creases and the ground under things,
//!   worked out from the depth of the picture at half size;
//! - **bloom** spreads the light of anything brighter than a threshold
//!   (lamps, fires, the sun) round it;
//! - the **grade** sets exposure, tint, saturation and contrast and rolls
//!   highlights off into the screen's range.
//!
//! Strengths come from the pack (`render.json`: `post`, `grade`, and the
//! mood of the place); which passes run is the player's graphics setting.

use crate::atmos::Frame;
use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;
use glam::{Mat4, Vec3};
use serde::Deserialize;

/// `post` in render.json.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct PostLook {
    /// multiplies the picture before the grade
    pub exposure: f32,
    pub bloom: BloomLook,
    pub ao: AoLook,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct BloomLook {
    /// light above this brightness blooms; `knee` softens the cut
    pub threshold: f32,
    pub knee: f32,
    /// 0: no bloom
    pub strength: f32,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AoLook {
    /// how far round a point is searched for what closes it in (metres)
    pub radius: f32,
    /// 0: none
    pub strength: f32,
    /// fades out by this distance (metres)
    pub reach: f32,
}

impl Default for PostLook {
    fn default() -> Self {
        PostLook {
            exposure: 1.0,
            bloom: BloomLook::default(),
            ao: AoLook::default(),
        }
    }
}

impl Default for BloomLook {
    fn default() -> Self {
        BloomLook {
            threshold: 1.0,
            knee: 0.5,
            strength: 0.0,
        }
    }
}

impl Default for AoLook {
    fn default() -> Self {
        AoLook {
            radius: 0.8,
            strength: 0.0,
            reach: 70.0,
        }
    }
}

/// The player's graphics setting: what the costly passes may spend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality {
    /// width of each shadow cascade's map
    pub shadow_size: u32,
    pub ssao: bool,
    pub bloom: bool,
}

impl Quality {
    pub const NAMES: [&'static str; 4] = ["low", "medium", "high", "ultra"];

    /// 0 low, 1 medium, 2 high (the default), 3 ultra.
    pub fn level(n: u8) -> Quality {
        match n {
            0 => Quality {
                shadow_size: 1024,
                ssao: false,
                bloom: false,
            },
            1 => Quality {
                shadow_size: 1536,
                ssao: false,
                bloom: true,
            },
            2 => Quality {
                shadow_size: 2048,
                ssao: true,
                bloom: true,
            },
            _ => Quality {
                shadow_size: 4096,
                ssao: true,
                bloom: true,
            },
        }
    }

    /// `DYNO_GRAPHICS=low|medium|high|ultra` overrides the saved setting
    /// (shots and measurements).
    pub fn from_env(saved: u8) -> Quality {
        let n = std::env::var("DYNO_GRAPHICS")
            .ok()
            .and_then(|v| Self::NAMES.iter().position(|n| *n == v.trim()))
            .map_or(saved, |i| i as u8);
        Self::level(n)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    inv_view_proj: [[f32; 4]; 4],
    /// xyz: the camera; w: pixels a metre spans at one metre's distance
    cam: [f32; 4],
    /// width, height, 1 / width, 1 / height of the picture
    size: [f32; 4],
    /// radius, strength, reach, 0
    ao: [f32; 4],
    /// threshold, knee, strength, 0
    bloom: [f32; 4],
    /// rgb: tint; w: saturation
    grade_tint: [f32; 4],
    /// contrast, knee, rolloff, exposure
    grade: [f32; 4],
}

const SHADER: &str = r#"
struct P { inv_view_proj: mat4x4<f32>, cam: vec4<f32>, size: vec4<f32>, ao: vec4<f32>, bloom: vec4<f32>, grade_tint: vec4<f32>, grade: vec4<f32> };
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var a_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var b_tex: texture_2d<f32>;
@group(0) @binding(4) var depth_tex: texture_depth_2d;

struct VOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs(@builtin(vertex_index) vi: u32) -> VOut {
  let q = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
  var o: VOut;
  o.clip = vec4<f32>(q * 2.0 - 1.0, 0.0, 1.0);
  o.uv = vec2<f32>(q.x, 1.0 - q.y);
  return o;
}

// Where in the world the picture's pixel lies.
fn world_at(pix: vec2<i32>) -> vec3<f32> {
  let c = clamp(pix, vec2<i32>(0), vec2<i32>(p.size.xy) - vec2<i32>(1));
  let z = textureLoad(depth_tex, c, 0);
  let ndc = vec2<f32>((f32(c.x) + 0.5) * p.size.z * 2.0 - 1.0, 1.0 - (f32(c.y) + 0.5) * p.size.w * 2.0);
  let w = p.inv_view_proj * vec4<f32>(ndc, z, 1.0);
  return w.xyz / w.w;
}

// Ambient occlusion at half size: r how open the point is (1: open sky),
// g its distance, for the blur to keep edges by.
@fragment fn fs_ao(i: VOut) -> @location(0) vec4<f32> {
  let hp = vec2<i32>(i.clip.xy);
  let pix = hp * 2;
  let z = textureLoad(depth_tex, pix, 0);
  if (z >= 1.0) { return vec4<f32>(1.0, 100000.0, 0.0, 1.0); }
  let P = world_at(pix);
  let dist = distance(P, p.cam.xyz);
  if (dist > p.ao.z) { return vec4<f32>(1.0, dist, 0.0, 1.0); }
  // the surface's normal from its neighbours: on each axis the nearer one,
  // so an edge does not bend it
  let l = P - world_at(pix + vec2<i32>(-1, 0));
  let r = world_at(pix + vec2<i32>(1, 0)) - P;
  let up = P - world_at(pix + vec2<i32>(0, -1));
  let dn = world_at(pix + vec2<i32>(0, 1)) - P;
  var dx = r;
  if (dot(l, l) < dot(r, r)) { dx = l; }
  var dy = dn;
  if (dot(up, up) < dot(dn, dn)) { dy = up; }
  var n = normalize(cross(dy, dx));
  if (dot(n, p.cam.xyz - P) < 0.0) { n = -n; }
  // the search radius on screen, kept within reason near and far
  let rpx = clamp(p.ao.x * p.cam.w / dist, 4.0, 72.0);
  let rad = rpx * dist / p.cam.w;
  // each pixel of a 4 x 4 block turns the pattern differently; the blur
  // that follows takes in a whole block
  let turn = f32((hp.x & 3) + (hp.y & 3) * 4) * 0.3927 + f32((hp.x & 3) * 7 % 4) * 1.5708;
  var occ = 0.0;
  for (var k = 0; k < 8; k++) {
    let a = turn + f32(k) * 2.39996;
    let rr = rpx * sqrt((f32(k) + 0.5) / 8.0);
    let S = world_at(pix + vec2<i32>(vec2<f32>(cos(a), sin(a)) * rr));
    let v = S - P;
    let vv = dot(v, v);
    // how far above the surface the neighbour stands, as an angle; things
    // beyond the radius do not count
    occ += max(dot(v, n) / sqrt(vv + 0.0001) - 0.12, 0.0) * max(1.0 - vv / (rad * rad), 0.0);
  }
  let ao = clamp(1.0 - occ / 8.0 * 2.2 * p.ao.y, 0.0, 1.0);
  return vec4<f32>(ao, dist, 0.0, 1.0);
}

// Blur the occlusion over a 4 x 4 block (keeping to surfaces at the
// pixel's own distance) and multiply the picture by it.
@fragment fn fs_ao_apply(i: VOut) -> @location(0) vec4<f32> {
  let pix = vec2<i32>(i.clip.xy);
  let z = textureLoad(depth_tex, pix, 0);
  if (z >= 1.0) { return vec4<f32>(1.0); }
  let dist = distance(world_at(pix), p.cam.xyz);
  if (dist > p.ao.z) { return vec4<f32>(1.0); }
  let hp = pix / 2;
  let lim = vec2<i32>(textureDimensions(a_tex)) - vec2<i32>(1);
  let tol = 0.05 * dist + 0.12;
  var sum = 0.0;
  var wsum = 0.0;
  for (var y = -2; y < 2; y++) {
    for (var x = -2; x < 2; x++) {
      let t = textureLoad(a_tex, clamp(hp + vec2<i32>(x, y), vec2<i32>(0), lim), 0);
      let w = max(1.0 - abs(t.g - dist) / tol, 0.0) + 0.001;
      sum += t.r * w;
      wsum += w;
    }
  }
  let ao = mix(1.0, sum / wsum, 1.0 - smoothstep(p.ao.z * 0.6, p.ao.z, dist));
  return vec4<f32>(ao, ao, ao, 1.0);
}

fn box4(uv: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
  return 0.25 * (textureSampleLevel(a_tex, samp, uv + texel * vec2<f32>(-1.0, -1.0), 0.0).rgb
    + textureSampleLevel(a_tex, samp, uv + texel * vec2<f32>(1.0, -1.0), 0.0).rgb
    + textureSampleLevel(a_tex, samp, uv + texel * vec2<f32>(-1.0, 1.0), 0.0).rgb
    + textureSampleLevel(a_tex, samp, uv + texel * vec2<f32>(1.0, 1.0), 0.0).rgb);
}

// The picture at half size, keeping only what is brighter than the threshold.
@fragment fn fs_bloom_first(i: VOut) -> @location(0) vec4<f32> {
  let texel = 1.0 / vec2<f32>(textureDimensions(a_tex));
  let c = min(box4(i.uv, texel), vec3<f32>(24.0));
  let br = max(c.r, max(c.g, c.b));
  let knee = max(p.bloom.y, 0.0001);
  let soft = clamp(br - p.bloom.x + knee, 0.0, 2.0 * knee);
  let k = max(soft * soft / (4.0 * knee), br - p.bloom.x) / max(br, 0.0001);
  return vec4<f32>(c * k, 1.0);
}

@fragment fn fs_bloom_down(i: VOut) -> @location(0) vec4<f32> {
  let texel = 1.0 / vec2<f32>(textureDimensions(a_tex));
  return vec4<f32>(box4(i.uv, texel), 1.0);
}

// Added to the next larger level.
@fragment fn fs_bloom_up(i: VOut) -> @location(0) vec4<f32> {
  let texel = 1.0 / vec2<f32>(textureDimensions(a_tex));
  return vec4<f32>(box4(i.uv, texel * 0.75), 1.0);
}

// The colour grade: exposure, tint, saturation, contrast round mid grey,
// and a soft roll-off of highlights instead of clipping.
fn grade(c0: vec3<f32>) -> vec3<f32> {
  var c = c0 * p.grade.w * p.grade_tint.rgb;
  let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
  c = mix(vec3<f32>(l), c, p.grade_tint.w);
  c = (c - 0.18) * p.grade.x + 0.18;
  let knee = vec3<f32>(p.grade.y);
  let k = max(c - knee, vec3<f32>(0.0));
  c = min(c, knee) + k / (vec3<f32>(1.0) + k * p.grade.z);
  return max(c, vec3<f32>(0.0));
}

@fragment fn fs_composite(i: VOut) -> @location(0) vec4<f32> {
  var c = textureLoad(a_tex, vec2<i32>(i.clip.xy), 0).rgb;
  if (p.bloom.z > 0.0) { c += textureSampleLevel(b_tex, samp, i.uv, 0.0).rgb * p.bloom.z; }
  return vec4<f32>(grade(c), 1.0);
}
"#;

const BLOOM_LEVELS: usize = 5;

pub struct Post {
    /// the format the world is drawn in
    pub format: wgpu::TextureFormat,
    pub quality: Quality,
    /// the world's picture, before the grade
    pub hdr: wgpu::TextureView,
    /// the same as a texture, for passes that copy it (water.rs)
    pub picture: wgpu::Texture,
    bgl: wgpu::BindGroupLayout,
    samp: wgpu::Sampler,
    ubuf: wgpu::Buffer,
    ao: wgpu::TextureView,
    mips: Vec<wgpu::TextureView>,
    /// bind groups reading: the picture and its bloom, the picture alone,
    /// the occlusion, each bloom level
    from_hdr: wgpu::BindGroup,
    from_picture: wgpu::BindGroup,
    from_ao: wgpu::BindGroup,
    from_mip: Vec<wgpu::BindGroup>,
    ao_pipe: wgpu::RenderPipeline,
    ao_apply_pipe: wgpu::RenderPipeline,
    first_pipe: wgpu::RenderPipeline,
    down_pipe: wgpu::RenderPipeline,
    up_pipe: wgpu::RenderPipeline,
    composite_pipe: wgpu::RenderPipeline,
    size: [u32; 2],
    /// set by `prepare`: whether this frame has occlusion and bloom
    ao_on: bool,
    bloom_on: bool,
}

const AO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

fn target(
    device: &wgpu::Device,
    label: &str,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// What `Post::new` and `Post::resize` build for a picture size.
struct Targets {
    hdr: wgpu::TextureView,
    picture: wgpu::Texture,
    ao: wgpu::TextureView,
    mips: Vec<wgpu::TextureView>,
    from_hdr: wgpu::BindGroup,
    from_picture: wgpu::BindGroup,
    from_ao: wgpu::BindGroup,
    from_mip: Vec<wgpu::BindGroup>,
}

impl Post {
    /// `out`: the format of the screen (or the image) the finished picture goes to.
    pub fn new(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        out: wgpu::TextureFormat,
        w: u32,
        h: u32,
        depth: &wgpu::TextureView,
        quality: Quality,
    ) -> Post {
        // light brighter than white needs a float picture; where the
        // adapter cannot draw into one, an ordinary picture stands in
        // (bright things clip, and bloom has less to work with)
        let need = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let float = wgpu::TextureFormat::Rgba16Float;
        let features = adapter.get_texture_format_features(float);
        let format = if features.allowed_usages.contains(need)
            && features
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE)
        {
            float
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        };
        let ao_format = if format == float {
            AO_FORMAT
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        let quality = Quality {
            // without a float picture the occlusion has nowhere to keep distances
            ssao: quality.ssao && format == float,
            ..quality
        };
        let tex = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                tex(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                tex(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let samp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipe = |fs: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        };
        let keep_alpha = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let multiply = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: keep_alpha,
        };
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: keep_alpha,
        };
        let ao_pipe = pipe("fs_ao", ao_format, None);
        let ao_apply_pipe = pipe("fs_ao_apply", format, Some(multiply));
        let first_pipe = pipe("fs_bloom_first", format, None);
        let down_pipe = pipe("fs_bloom_down", format, None);
        let up_pipe = pipe("fs_bloom_up", format, Some(add));
        let composite_pipe = pipe("fs_composite", out, None);
        let t = Self::targets(device, &bgl, &ubuf, &samp, format, ao_format, w, h, depth);
        Post {
            format,
            quality,
            hdr: t.hdr,
            picture: t.picture,
            bgl,
            samp,
            ubuf,
            ao: t.ao,
            mips: t.mips,
            from_hdr: t.from_hdr,
            from_picture: t.from_picture,
            from_ao: t.from_ao,
            from_mip: t.from_mip,
            ao_pipe,
            ao_apply_pipe,
            first_pipe,
            down_pipe,
            up_pipe,
            composite_pipe,
            size: [w.max(1), h.max(1)],
            ao_on: false,
            bloom_on: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn targets(
        device: &wgpu::Device,
        bgl: &wgpu::BindGroupLayout,
        ubuf: &wgpu::Buffer,
        samp: &wgpu::Sampler,
        format: wgpu::TextureFormat,
        ao_format: wgpu::TextureFormat,
        w: u32,
        h: u32,
        depth: &wgpu::TextureView,
    ) -> Targets {
        let (w, h) = (w.max(1), h.max(1));
        let picture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("picture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let hdr = picture.create_view(&wgpu::TextureViewDescriptor::default());
        let ao = target(device, "occlusion", w.div_ceil(2), h.div_ceil(2), ao_format);
        let mips: Vec<wgpu::TextureView> = (0..BLOOM_LEVELS)
            .map(|i| {
                let d = 2u32 << i;
                target(device, "bloom", (w / d).max(1), (h / d).max(1), format)
            })
            .collect();
        let bind = |a: &wgpu::TextureView, b: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post"),
                layout: bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: ubuf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(samp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                ],
            })
        };
        let from_hdr = bind(&hdr, &mips[0]);
        // the first bloom pass draws into mips[0], so it cannot also read it
        let from_picture = bind(&hdr, &hdr);
        let from_ao = bind(&ao, &ao);
        let from_mip = mips.iter().map(|m| bind(m, m)).collect();
        Targets {
            hdr,
            picture,
            ao,
            mips,
            from_hdr,
            from_picture,
            from_ao,
            from_mip,
        }
    }

    /// The picture changed size (and with it the depth it is drawn against).
    pub fn resize(&mut self, device: &wgpu::Device, w: u32, h: u32, depth: &wgpu::TextureView) {
        let ao_format = if self.format == wgpu::TextureFormat::Rgba16Float {
            AO_FORMAT
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        let t = Self::targets(
            device,
            &self.bgl,
            &self.ubuf,
            &self.samp,
            self.format,
            ao_format,
            w,
            h,
            depth,
        );
        self.hdr = t.hdr;
        self.picture = t.picture;
        self.ao = t.ao;
        self.mips = t.mips;
        self.from_hdr = t.from_hdr;
        self.from_picture = t.from_picture;
        self.from_ao = t.from_ao;
        self.from_mip = t.from_mip;
        self.size = [w.max(1), h.max(1)];
    }

    /// Set this frame's strengths. `view`: the camera's transform and position.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        look: &crate::look::RenderSettings,
        env: &Frame,
        view: Option<(Mat4, Vec3)>,
    ) {
        let (w, h) = (self.size[0] as f32, self.size[1] as f32);
        let (inv, cam, px_per_m) = match view {
            Some((vp, cam)) => {
                let inv = vp.inverse();
                // how many pixels a metre spans at one metre's distance
                let fwd = (inv.project_point3(Vec3::new(0.0, 0.0, 0.5)) - cam).normalize_or_zero();
                let right = (inv.project_point3(Vec3::new(1.0, 0.0, 0.5))
                    - inv.project_point3(Vec3::new(-1.0, 0.0, 0.5)))
                .normalize_or_zero();
                let a = vp.project_point3(cam + fwd);
                let b = vp.project_point3(cam + fwd + right);
                (inv, cam, (b.x - a.x).abs() * w * 0.5)
            }
            None => (Mat4::IDENTITY, Vec3::ZERO, 1.0),
        };
        let p = &look.post;
        self.ao_on = view.is_some() && self.quality.ssao && p.ao.strength > 0.0;
        self.bloom_on = view.is_some() && self.quality.bloom && p.bloom.strength > 0.0;
        let g = &look.grade;
        let params = Params {
            inv_view_proj: inv.to_cols_array_2d(),
            cam: [cam.x, cam.y, cam.z, px_per_m.max(1.0)],
            size: [w, h, 1.0 / w, 1.0 / h],
            ao: [p.ao.radius, p.ao.strength, p.ao.reach.max(1.0), 0.0],
            bloom: [
                p.bloom.threshold,
                p.bloom.knee,
                if self.bloom_on { p.bloom.strength } else { 0.0 },
                0.0,
            ],
            grade_tint: [env.tint[0], env.tint[1], env.tint[2], env.saturation],
            grade: [env.contrast, g.knee, g.rolloff, env.exposure],
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&params));
    }

    fn pass(
        enc: &mut wgpu::CommandEncoder,
        to: &wgpu::TextureView,
        clear: bool,
        pipe: &wgpu::RenderPipeline,
        bind: &wgpu::BindGroup,
    ) {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: to,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rp.set_pipeline(pipe);
        rp.set_bind_group(0, bind, &[]);
        rp.draw(0..3, 0..1);
    }

    /// Darken the creases of what has been drawn so far (the solid world):
    /// between the solid pass and the sky, water and lights.
    pub fn occlude(&self, enc: &mut wgpu::CommandEncoder) {
        if !self.ao_on {
            return;
        }
        Self::pass(enc, &self.ao, true, &self.ao_pipe, &self.from_mip[0]);
        Self::pass(enc, &self.hdr, false, &self.ao_apply_pipe, &self.from_ao);
    }

    /// Bloom, then the graded picture into `out`.
    pub fn finish(&self, enc: &mut wgpu::CommandEncoder, out: &wgpu::TextureView) {
        if self.bloom_on {
            Self::pass(
                enc,
                &self.mips[0],
                true,
                &self.first_pipe,
                &self.from_picture,
            );
            for i in 1..self.mips.len() {
                Self::pass(
                    enc,
                    &self.mips[i],
                    true,
                    &self.down_pipe,
                    &self.from_mip[i - 1],
                );
            }
            for i in (0..self.mips.len() - 1).rev() {
                Self::pass(
                    enc,
                    &self.mips[i],
                    false,
                    &self.up_pipe,
                    &self.from_mip[i + 1],
                );
            }
        }
        Self::pass(enc, out, true, &self.composite_pipe, &self.from_hdr);
    }
}
