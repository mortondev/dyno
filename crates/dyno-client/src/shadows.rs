//! Sun and moon shadows: three shadow maps (cascades) of growing reach laid
//! round the view, from a fine one for what stands by the camera to a wide
//! one for the far side of the valley. Each is snapped to its own texels,
//! so shadows hold still as the camera moves and turns.

use crate::render::Vertex;
use egui_wgpu::wgpu;
use glam::{Mat4, Vec3};

pub const CASCADES: usize = 3;
/// Half the width (metres) of the ground each cascade covers.
const HALF: [f32; CASCADES] = [14.0, 46.0, 124.0];
/// How far ahead of the camera each cascade's centre sits, as a part of its
/// half width: more of the map lies where you look.
const AHEAD: f32 = 0.55;
/// Casters this far (metres) towards and away from the light are kept.
pub const DEPTH_RANGE: f32 = 260.0;

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Depth only, from the light. `vs_packed`: the world's packed meshes (pack.rs).
const SHADER: &str = r#"
struct C { vp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> c: C;
@vertex fn vs_packed(v: PIn) -> @builtin(position) vec4<f32> {
  if (v.pos.w < -2) { return vec4<f32>(2.0, 2.0, 2.0, 1.0); }
  return c.vp * vec4<f32>(unpack_pos(v), 1.0);
}
@vertex fn vs(@location(0) pos: vec3<f32>, @location(5) tex: vec4<f32>) -> @builtin(position) vec4<f32> {
  // decals and halos cast no shadow
  if (tex.z < -2.5) { return vec4<f32>(2.0, 2.0, 2.0, 1.0); }
  return c.vp * vec4<f32>(pos, 1.0);
}
"#;

/// One cascade's place this frame.
#[derive(Clone, Copy)]
pub struct Cascade {
    pub vp: Mat4,
    /// world size of one texel
    pub texel: f32,
    /// the direction towards the light the map is drawn from
    pub to_light: Vec3,
    /// the light's view, its centre there and its half width: for culling
    view: Mat4,
    centre: Vec3,
    half: f32,
}

impl Cascade {
    /// Whether a ball could cast into this cascade.
    pub fn reaches(&self, p: Vec3, r: f32) -> bool {
        let q = self.view.transform_point3(p);
        (q.x - self.centre.x).abs() < self.half + r && (q.y - self.centre.y).abs() < self.half + r
    }
}

/// The direction towards the light the shadow maps are drawn from, for a
/// light at `light_dir` (towards it).
pub fn to_light(light_dir: [f32; 3]) -> Vec3 {
    let dir = Vec3::from(light_dir).normalize_or(Vec3::Y);
    // a light on the horizon would smear its map along the ground
    Vec3::new(dir.x, dir.y.max(0.06), dir.z).normalize()
}

/// The cascades for a camera at `cam` looking along `forward`, lit from
/// `light_dir` (towards the light).
pub fn cascades(cam: Vec3, forward: Vec3, light_dir: [f32; 3], size: u32) -> [Cascade; CASCADES] {
    let dir = to_light(light_dir);
    let up = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    let view = Mat4::look_to_rh(Vec3::ZERO, -dir, up);
    let flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    HALF.map(|half| {
        let texel = half * 2.0 / size as f32;
        let c = view.transform_point3(cam + flat * half * AHEAD);
        // the centre moves in whole texels
        let c = Vec3::new(
            (c.x / texel).floor() * texel,
            (c.y / texel).floor() * texel,
            c.z,
        );
        let proj = Mat4::orthographic_rh(
            c.x - half,
            c.x + half,
            c.y - half,
            c.y + half,
            -c.z - DEPTH_RANGE,
            -c.z + DEPTH_RANGE,
        );
        Cascade {
            vp: proj * view,
            to_light: dir,
            texel,
            view,
            centre: c,
            half,
        }
    })
}

pub struct Shadows {
    pub size: u32,
    /// all the cascades, for the world shader
    pub view: wgpu::TextureView,
    layers: [wgpu::TextureView; CASCADES],
    bufs: [wgpu::Buffer; CASCADES],
    binds: [wgpu::BindGroup; CASCADES],
    /// casters in the plain vertex format (set when a cascade's pass begins)
    pub pipe: wgpu::RenderPipeline,
    /// casters in the packed format, with the section corners in buffer 1
    pub packed_pipe: wgpu::RenderPipeline,
}

impl Shadows {
    pub fn new(device: &wgpu::Device, size: u32, vlayout: &wgpu::VertexBufferLayout) -> Shadows {
        let size = size.clamp(256, 4096);
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layers = std::array::from_fn(|i| {
            tex.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: i as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let bufs: [wgpu::Buffer; CASCADES] = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cascade"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let binds = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cascade"),
                layout: &bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: bufs[i].as_entire_binding(),
                }],
            })
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow"),
            source: wgpu::ShaderSource::Wgsl([crate::pack::WGSL, SHADER].concat().into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let make = |entry: &str, buffers: &[wgpu::VertexBufferLayout]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shadow"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    buffers,
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
                    // only for slopes; the world shader lifts the point it
                    // looks up off its surface instead (atmos.rs)
                    bias: wgpu::DepthBiasState {
                        constant: 1,
                        slope_scale: 1.5,
                        clamp: 0.0,
                    },
                }),
                multisample: Default::default(),
                fragment: None,
                multiview: None,
                cache: None,
            })
        };
        let pipe = make("vs", std::slice::from_ref(vlayout));
        let packed_pipe = make(
            "vs_packed",
            &[crate::pack::layout(), crate::pack::origin_layout()],
        );
        Shadows {
            packed_pipe,
            size,
            view,
            layers,
            bufs,
            binds,
            pipe,
        }
    }

    /// Draw the cascades. `draw` is called once for each with its render
    /// pass, to draw whatever casts into it; `done` after each pass ends.
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        cascades: &[Cascade; CASCADES],
        mut draw: impl FnMut(&mut wgpu::RenderPass, &Cascade),
        mut done: impl FnMut(&mut wgpu::CommandEncoder, usize),
    ) {
        for (i, c) in cascades.iter().enumerate() {
            queue.write_buffer(
                &self.bufs[i],
                0,
                bytemuck::bytes_of(&c.vp.to_cols_array_2d()),
            );
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.layers[i],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.pipe);
            rp.set_bind_group(0, &self.binds[i], &[]);
            draw(&mut rp, c);
            drop(rp);
            done(enc, i);
        }
    }
}

/// The vertex layout the shadow pass shares with the world.
pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32, 4 => Float32, 5 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cascades_hold_still_within_a_texel_and_cover_the_view() {
        let sun = [0.5, 0.7, 0.3];
        let a = cascades(Vec3::new(100.0, 40.0, 100.0), Vec3::NEG_Z, sun, 2048);
        // a step smaller than a texel leaves each map where it was, or
        // moves it by whole texels
        let b = cascades(Vec3::new(100.003, 40.0, 100.0), Vec3::NEG_Z, sun, 2048);
        for (x, y) in a.iter().zip(&b) {
            let d = (x.centre - y.centre).truncate() / x.texel;
            assert!((d.x - d.x.round()).abs() < 1e-3 && (d.y - d.y.round()).abs() < 1e-3);
        }
        // ground 150 m ahead is in the widest map; the camera's feet in the finest
        let ahead = a[2].vp.project_point3(Vec3::new(100.0, 30.0, -50.0));
        assert!(ahead.x.abs() < 1.0 && ahead.y.abs() < 1.0 && (0.0..1.0).contains(&ahead.z));
        let feet = a[0].vp.project_point3(Vec3::new(100.0, 38.0, 100.0));
        assert!(feet.x.abs() < 1.0 && feet.y.abs() < 1.0);
        assert!(a[0].reaches(Vec3::new(100.0, 38.0, 100.0), 1.0));
        assert!(!a[0].reaches(Vec3::new(300.0, 38.0, 100.0), 14.0));
    }
}
