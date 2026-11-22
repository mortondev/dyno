//! wgpu renderer: voxel sections with ambient occlusion, fog, a block highlight
//! and simple boxes for other players.

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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    cam: [f32; 4],
    fog: [f32; 4],
}

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, cam: vec4<f32>, fog: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;

struct VIn { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32> };
struct VOut {
  @builtin(position) clip: vec4<f32>,
  @location(0) wpos: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) color: vec4<f32>,
};

@vertex fn vs(v: VIn) -> VOut {
  var o: VOut;
  o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
  o.wpos = v.pos;
  o.normal = v.normal;
  o.color = v.color;
  return o;
}

fn hash3(p: vec3<f32>) -> f32 {
  return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

@fragment fn fs(i: VOut) -> @location(0) vec4<f32> {
  // 16 x 16 texel grain so blocks read as pixel-art surfaces
  let t = floor(i.wpos * 16.0 - i.normal * 0.01);
  let grain = select(1.0, 0.86 + 0.24 * hash3(t), dot(i.normal, i.normal) > 0.5);
  var c = i.color.rgb * grain * i.color.a;
  let d = distance(i.wpos, u.cam.xyz);
  let f = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  c = mix(c, u.fog.rgb, f);
  return vec4<f32>(c, select(1.0, 0.62, i.color.a > 1.9));
}

@fragment fn fs_translucent(i: VOut) -> @location(0) vec4<f32> {
  let d = distance(i.wpos, u.cam.xyz);
  let f = clamp((d - u.fog.w * 0.55) / (u.fog.w * 0.45), 0.0, 1.0);
  let c = mix(i.color.rgb, u.fog.rgb, f);
  return vec4<f32>(c, i.color.a);
}
"#;

fn srgb(c: [f32; 3]) -> [f32; 3] {
    [c[0].powf(2.2), c[1].powf(2.2), c[2].powf(2.2)]
}

/// Display colour of a block (sRGB) and its opacity when translucent.
pub fn block_color(b: BlockId) -> ([f32; 3], f32) {
    match b {
        w::GRASS => ([0.42, 0.66, 0.29], 1.0),
        w::DIRT => ([0.53, 0.38, 0.26], 1.0),
        w::STONE => ([0.52, 0.53, 0.56], 1.0),
        w::SAND => ([0.86, 0.81, 0.62], 1.0),
        w::LOG => ([0.42, 0.31, 0.19], 1.0),
        w::LEAVES => ([0.25, 0.50, 0.21], 1.0),
        w::PLANKS => ([0.72, 0.56, 0.35], 1.0),
        w::GLASS => ([0.78, 0.90, 0.96], 0.35),
        w::EMBERSTONE => ([1.0, 0.55, 0.22], 1.0),
        w::BRICK => ([0.66, 0.30, 0.24], 1.0),
        w::SNOW => ([0.94, 0.96, 0.98], 1.0),
        w::COBBLE => ([0.44, 0.45, 0.47], 1.0),
        w::WATER => ([0.20, 0.42, 0.78], 0.62),
        _ => ([1.0, 0.0, 1.0], 1.0),
    }
}

fn translucent(b: BlockId) -> bool {
    matches!(b, w::WATER | w::GLASS)
}

const FACES: [([i32; 3], [i32; 3], [i32; 3], f32); 6] = [
    // normal, u axis, v axis, face shade
    ([1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
    ([-1, 0, 0], [0, 0, 1], [0, 1, 0], 0.80),
    ([0, 1, 0], [1, 0, 0], [0, 0, 1], 1.00),
    ([0, -1, 0], [1, 0, 0], [0, 0, 1], 0.55),
    ([0, 0, 1], [1, 0, 0], [0, 1, 0], 0.68),
    ([0, 0, -1], [1, 0, 0], [0, 1, 0], 0.68),
];

/// Build the opaque and translucent meshes of one 16³ section.
pub fn mesh_section(world: &World, sx: i32, sy: i32, sz: i32) -> (Vec<Vertex>, Vec<Vertex>) {
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
                for (n, u, v, shade) in FACES {
                    let nb = world.get(x + n[0], y + n[1], z + n[2]);
                    if w::is_opaque(nb) || (tr && nb == b) || (!tr && b != w::LEAVES && nb == b) {
                        continue;
                    }
                    if b == w::LEAVES && nb == w::LEAVES {
                        continue;
                    }
                    let base = [x + n[0], y + n[1], z + n[2]];
                    let center = [
                        x as f32 + 0.5 + n[0] as f32 * 0.5,
                        y as f32 + 0.5 + n[1] as f32 * 0.5,
                        z as f32 + 0.5 + n[2] as f32 * 0.5,
                    ];
                    let mut corners = [([0f32; 3], 0f32); 4];
                    for (i, (su, sv)) in [(-1, -1), (1, -1), (1, 1), (-1, 1)].into_iter().enumerate() {
                        let p = [
                            center[0] + 0.5 * (su * u[0] + sv * v[0]) as f32,
                            center[1] + 0.5 * (su * u[1] + sv * v[1]) as f32,
                            center[2] + 0.5 * (su * u[2] + sv * v[2]) as f32,
                        ];
                        let light = if b == w::EMBERSTONE {
                            2.0
                        } else if tr {
                            alpha
                        } else {
                            let s1 = solid(base[0] + su * u[0], base[1] + su * u[1], base[2] + su * u[2]);
                            let s2 = solid(base[0] + sv * v[0], base[1] + sv * v[1], base[2] + sv * v[2]);
                            let c = solid(
                                base[0] + su * u[0] + sv * v[0],
                                base[1] + su * u[1] + sv * v[1],
                                base[2] + su * u[2] + sv * v[2],
                            );
                            let ao = if s1 == 1 && s2 == 1 { 0 } else { 3 - (s1 + s2 + c) };
                            [0.5, 0.66, 0.83, 1.0][ao as usize] * shade
                        };
                        corners[i] = (p, light);
                    }
                    let nf = [n[0] as f32, n[1] as f32, n[2] as f32];
                    let col = if tr { [lin[0] * shade, lin[1] * shade, lin[2] * shade] } else { lin };
                    // flip the diagonal so ambient occlusion interpolates without seams
                    let order = if corners[0].1 + corners[2].1 > corners[1].1 + corners[3].1 {
                        [0, 1, 2, 0, 2, 3]
                    } else {
                        [1, 2, 3, 1, 3, 0]
                    };
                    let out = if tr { &mut trans } else { &mut opaque };
                    for i in order {
                        let (p, l) = corners[i];
                        out.push(Vertex { pos: p, normal: nf, color: [col[0], col[1], col[2], l] });
                    }
                }
            }
        }
    }
    (opaque, trans)
}

/// Add a coloured box to a vertex list (used for players).
pub fn push_box(out: &mut Vec<Vertex>, min: Vec3, max: Vec3, rgb: [f32; 3]) {
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
            out.push(Vertex { pos: p[i].into(), normal: [0.0; 3], color: [c[0], c[1], c[2], shade] });
        }
    }
}

struct SectionMesh {
    opaque: Option<(wgpu::Buffer, u32)>,
    trans: Option<(wgpu::Buffer, u32)>,
}

pub struct Gfx {
    pub window: Arc<winit::window::Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub egui: egui_wgpu::Renderer,
    depth: wgpu::TextureView,
    opaque_pipe: wgpu::RenderPipeline,
    trans_pipe: wgpu::RenderPipeline,
    line_pipe: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    sections: Vec<SectionMesh>,
    dims: [i32; 3],
    pub adapter_info: String,
}

use egui_winit::winit;

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
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
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).map_err(|e| format!("surface: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
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
            required_limits: wgpu::Limits { max_texture_dimension_2d: 8192, ..limits },
            ..Default::default()
        }))
        .map_err(|e| format!("device: {e}"))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface is not supported by the adapter")?;
        let caps = surface.get_capabilities(&adapter);
        if let Some(f) = caps.formats.iter().find(|f| f.is_srgb()) {
            config.format = *f;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&device, &config);

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
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let vlayout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4],
        };
        let make = |fs: &str, topo: wgpu::PrimitiveTopology, blend: Option<wgpu::BlendState>, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: std::slice::from_ref(&vlayout),
                },
                primitive: wgpu::PrimitiveState { topology: topo, cull_mode: None, ..Default::default() },
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
        let line_pipe = make("fs_translucent", wgpu::PrimitiveTopology::LineList, Some(wgpu::BlendState::ALPHA_BLENDING), false);
        let egui = egui_wgpu::Renderer::new(&device, config.format, egui_wgpu::RendererOptions::default());
        let depth = depth_view(&device, config.width, config.height);
        Ok(Gfx {
            window,
            surface,
            device,
            queue,
            config,
            egui,
            depth,
            opaque_pipe,
            trans_pipe,
            line_pipe,
            ubuf,
            bind,
            sections: Vec::new(),
            dims: [0; 3],
            adapter_info,
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth = depth_view(&self.device, w, h);
    }

    fn upload(&self, v: &[Vertex]) -> Option<(wgpu::Buffer, u32)> {
        if v.is_empty() {
            return None;
        }
        let buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(v),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Some((buf, v.len() as u32))
    }

    /// Mesh the whole world.
    pub fn load_world(&mut self, world: &World) {
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
                    let (o, t) = mesh_section(world, sx, sy, sz);
                    self.sections.push(SectionMesh { opaque: self.upload(&o), trans: self.upload(&t) });
                }
            }
        }
    }

    /// Re-mesh the section that holds a cell, and its neighbours when the cell is on an edge.
    pub fn block_changed(&mut self, world: &World, p: [i32; 3]) {
        let mut todo = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                    let s = [q[0].div_euclid(SECTION), q[1].div_euclid(SECTION), q[2].div_euclid(SECTION)];
                    if (0..3).all(|i| s[i] >= 0 && s[i] < self.dims[i]) && !todo.contains(&s) {
                        todo.push(s);
                    }
                }
            }
        }
        for s in todo {
            let (o, t) = mesh_section(world, s[0], s[1], s[2]);
            let i = ((s[1] * self.dims[2] + s[2]) * self.dims[0] + s[0]) as usize;
            self.sections[i] = SectionMesh { opaque: self.upload(&o), trans: self.upload(&t) };
        }
    }

    /// Draw one frame: the world (if any), extra boxes, lines, then egui.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        view_proj: Option<(Mat4, Vec3, f32)>,
        boxes: &[Vertex],
        lines: &[Vertex],
        egui_ctx: &egui::Context,
        egui_out: egui::FullOutput,
    ) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sky = srgb(SKY);
        if let Some((vp, cam, dist)) = view_proj {
            let u = Uniforms {
                view_proj: vp.to_cols_array_2d(),
                cam: [cam.x, cam.y, cam.z, 0.0],
                fog: [sky[0], sky[1], sky[2], dist],
            };
            self.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let box_buf = self.upload(boxes);
        let line_buf = self.upload(lines);

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: sky[0] as f64,
                            g: sky[1] as f64,
                            b: sky[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some((vp, _, dist)) = view_proj {
                rp.set_bind_group(0, &self.bind, &[]);
                rp.set_pipeline(&self.opaque_pipe);
                let visible = |i: usize| self.section_visible(i, vp, dist);
                for (i, s) in self.sections.iter().enumerate() {
                    if let Some((b, n)) = &s.opaque {
                        if visible(i) {
                            rp.set_vertex_buffer(0, b.slice(..));
                            rp.draw(0..*n, 0..1);
                        }
                    }
                }
                if let Some((b, n)) = &box_buf {
                    rp.set_vertex_buffer(0, b.slice(..));
                    rp.draw(0..*n, 0..1);
                }
                rp.set_pipeline(&self.trans_pipe);
                for (i, s) in self.sections.iter().enumerate() {
                    if let Some((b, n)) = &s.trans {
                        if visible(i) {
                            rp.set_vertex_buffer(0, b.slice(..));
                            rp.draw(0..*n, 0..1);
                        }
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
        let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [self.config.width, self.config.height], pixels_per_point: ppp };
        for (id, delta) in &egui_out.textures_delta.set {
            self.egui.update_texture(&self.device, &self.queue, *id, delta);
        }
        let extra = self.egui.update_buffers(&self.device, &self.queue, &mut enc, &jobs, &screen);
        {
            let rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
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
        self.queue.submit(extra.into_iter().chain(std::iter::once(enc.finish())));
        self.window.pre_present_notify();
        frame.present();
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
