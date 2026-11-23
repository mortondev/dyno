//! The world's solid meshes as the graphics card holds them: 24 bytes a
//! vertex instead of the meshers' 60 (`render::Vertex`), and four vertices
//! a quad instead of six.
//!
//! The meshers still write plain `Vertex`es; a section's are packed once,
//! on the thread that meshed it (`Geo::pack`), and uploaded as they are.
//! Translucent faces and halos are few and keep the plain format.
//!
//! What is kept, and how exactly:
//! - **position:** metres from the section's corner in steps of 1/2048 m
//!   (the grid is the world's, so two sections put a shared corner at the
//!   same place), from 8 m before the section to 8 m past it;
//! - **kind** (`tex[2]`: the texture layer and what the vertex is): whole;
//! - **normal:** a signed byte a part (exact along the axes);
//! - **colour:** ten bits a part of its square root, up to 1.28 (dark
//!   colours keep their steps fine; white is exact); **brightness:** steps
//!   of 1/800 up to 1.26, or "glows" (2);
//! - **light and sky:** ten bits each;
//! - **texture coordinates:** steps of 1/2048 tile within ±16 tiles
//!   (blocks and shapes), or of 1/64 (leaves, whose faces count squares);
//!   fine voxel ground keeps its voxels a metre, the way its face looks,
//!   its saturation (hundredths) and jitter (thousandths) whole instead;
//! - **animation** (`tex[3]`): a whole part up to 15 and a fraction in 255
//!   steps: of 1/10000 for textures (frames, and seconds a frame in
//!   hundredths: up to 15 frames, 2.55 s a frame), of 1/100 for leaves
//!   (squares a side, and how much they differ).
//!
//! The vertex shader undoes this (`WGSL`) and hands the rest of the shader
//! what the plain format would have.

use crate::render::{KIND_FOLIAGE, KIND_VOXEL, KIND_WATER, SECTION, Vertex};
use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;

/// Steps a metre of a packed position.
pub const STEPS: f32 = 2048.0;
/// Metres before the section's corner a packed position can reach.
pub const BEFORE: f32 = 8.0;
/// The largest colour part that packs: such that 1 packs exactly (as 904 of 1023).
const COLOR_MAX: f32 = (1023.0 / 904.0) * (1023.0 / 904.0);
/// Steps of brightness to 1.
const SHADE_STEPS: f32 = 800.0;
/// The brightness code of a glowing face (`color[3]` of 2).
const SHADE_GLOW: u32 = 1015;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq, Eq, Hash, Debug)]
pub struct Packed {
    /// position, and the kind
    pub pos: [i16; 4],
    /// the normal, and the low part of the brightness
    pub normal: [i8; 4],
    /// colour (ten bits a part) and the high two bits of the brightness
    pub color: u32,
    /// texture coordinates (by kind)
    pub uv: [i16; 2],
    /// from the lowest bit: block light and sky light (ten bits each),
    /// then the animation: its whole part (four bits) and its fraction (eight)
    pub light: u32,
}

/// The layout of `Packed`, at locations 0 to 4; the section's corner comes
/// from a second buffer, one entry a section (`origin_layout`), at 6.
pub fn layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Sint16x4, 1 => Snorm8x4, 2 => Unorm10_10_10_2, 3 => Sint16x2, 4 => Uint32
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Packed>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

/// A section's corner, in metres: one entry a section, read by instance.
pub fn origin_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![6 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: 16,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRS,
    }
}

/// The packed vertex in a shader, and how to read it back: `unpack_pos`
/// for passes that need only the place, `unpack` for everything, as the
/// plain vertex's fields.
pub const WGSL: &str = r#"
struct PIn {
  @location(0) pos: vec4<i32>,
  @location(1) normal: vec4<f32>,
  @location(2) color: vec4<f32>,
  @location(3) uv: vec2<i32>,
  @location(4) light: u32,
  @location(6) origin: vec4<f32>,
};
struct Plain { pos: vec3<f32>, normal: vec3<f32>, color: vec4<f32>, light: f32, sky: f32, tex: vec4<f32> };
fn unpack_pos(v: PIn) -> vec3<f32> {
  return v.origin.xyz + vec3<f32>(v.pos.xyz) / 2048.0 + vec3<f32>(8.0);
}
fn unpack(v: PIn) -> Plain {
  var o: Plain;
  o.pos = unpack_pos(v);
  o.normal = v.normal.xyz;
  let rgb = v.color.rgb * v.color.rgb * 1.2806032;
  let shade = u32(round(v.color.a * 3.0)) * 254u + u32(i32(round(v.normal.w * 127.0)) + 127);
  var a = f32(shade) / 800.0;
  if (shade >= 1015u) { a = 2.0; }
  o.color = vec4<f32>(rgb, a);
  o.light = f32(v.light & 1023u) / 1023.0;
  o.sky = f32((v.light >> 10u) & 1023u) / 1023.0;
  let kz = f32(v.pos.w);
  let whole = f32((v.light >> 20u) & 15u);
  let part = f32(v.light >> 24u);
  var anim = whole + part / 10000.0;
  var uv = vec2<f32>(v.uv) / 2048.0;
  if (kz >= 16384.0) {
    // fine voxel ground: voxels a metre plus 32 times the way the face
    // looks; saturation in hundredths plus jitter
    uv = vec2<f32>(
      f32(v.uv.y >> 10u) + 32.0 * f32(v.uv.x >> 9u),
      f32(v.uv.x & 511) + f32(v.uv.y & 1023) / 1000.0);
  } else if (kz >= 8192.0 && kz < 12288.0) {
    uv = vec2<f32>(v.uv) / 64.0;
    anim = whole + part / 100.0;
  }
  o.tex = vec4<f32>(uv, kz, anim);
  return o;
}
"#;

fn clamp_i16(v: f32) -> i16 {
    v.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

/// Pack a vertex of the section whose corner is at `origin` (metres).
pub fn pack(v: &Vertex, origin: [f32; 3]) -> Packed {
    let p = [0, 1, 2].map(|i| clamp_i16((v.pos[i] - origin[i] - BEFORE) * STEPS));
    let kz = v.tex[2];
    let foliage = (KIND_FOLIAGE..KIND_WATER).contains(&kz);
    let n = v.normal.map(|c| (c.clamp(-1.0, 1.0) * 127.0).round() as i8);
    let c = [0, 1, 2]
        .map(|i| ((v.color[i].clamp(0.0, COLOR_MAX) / COLOR_MAX).sqrt() * 1023.0).round() as u32);
    let shade = if v.color[3] >= 1.9 {
        SHADE_GLOW
    } else {
        ((v.color[3].max(0.0) * SHADE_STEPS).round() as u32).min(SHADE_GLOW - 1)
    };
    let (hi, lo) = (shade / 254, (shade % 254) as i32 - 127);
    let uv = if kz >= KIND_VOXEL {
        // voxels a metre and the way the face looks (32 times it);
        // saturation in hundredths, jitter in thousandths
        let dir = (v.tex[0] / 32.0).floor().clamp(0.0, 7.0);
        let per = (v.tex[0] - dir * 32.0).round().clamp(0.0, 31.0) as i16;
        let sat = v.tex[1].floor().clamp(0.0, 511.0);
        let jit = ((v.tex[1] - sat) * 1000.0).round().clamp(0.0, 1023.0) as i16;
        [sat as i16 | (dir as i16) << 9, per << 10 | jit]
    } else if foliage {
        [clamp_i16(v.tex[0] * 64.0), clamp_i16(v.tex[1] * 64.0)]
    } else {
        [clamp_i16(v.tex[0] * 2048.0), clamp_i16(v.tex[1] * 2048.0)]
    };
    let whole = v.tex[3].max(0.0).floor().min(15.0);
    let steps = if foliage { 100.0 } else { 10000.0 };
    let part = (((v.tex[3].max(0.0) - whole) * steps).round() as u32).min(255);
    let ten = |x: f32| (x.clamp(0.0, 1.0) * 1023.0).round() as u32;
    Packed {
        pos: [p[0], p[1], p[2], clamp_i16(kz)],
        normal: [n[0], n[1], n[2], lo as i8],
        color: c[0] | c[1] << 10 | c[2] << 20 | hi << 30,
        uv,
        light: ten(v.light) | ten(v.sky) << 10 | (whole as u32) << 20 | part << 24,
    }
}

/// What the shader reads back from a packed vertex (the same arithmetic as
/// `WGSL`): for tests and tools.
pub fn unpack(p: &Packed, origin: [f32; 3]) -> Vertex {
    let pos = [0, 1, 2].map(|i| origin[i] + p.pos[i] as f32 / STEPS + BEFORE);
    let kz = p.pos[3] as f32;
    let part = |shift: u32| {
        let s = (p.color >> shift & 1023) as f32 / 1023.0;
        s * s * COLOR_MAX
    };
    let shade = (p.color >> 30) * 254 + (p.normal[3] as i32 + 127) as u32;
    let a = if shade >= SHADE_GLOW {
        2.0
    } else {
        shade as f32 / SHADE_STEPS
    };
    let foliage = (KIND_FOLIAGE..KIND_WATER).contains(&kz);
    let steps = if foliage { 100.0 } else { 10000.0 };
    let anim = (p.light >> 20 & 15) as f32 + (p.light >> 24) as f32 / steps;
    let uv = if kz >= KIND_VOXEL {
        [
            (p.uv[1] >> 10) as f32 + 32.0 * (p.uv[0] >> 9) as f32,
            (p.uv[0] & 511) as f32 + (p.uv[1] & 1023) as f32 / 1000.0,
        ]
    } else if foliage {
        [p.uv[0] as f32 / 64.0, p.uv[1] as f32 / 64.0]
    } else {
        [p.uv[0] as f32 / 2048.0, p.uv[1] as f32 / 2048.0]
    };
    Vertex {
        pos,
        normal: [0, 1, 2].map(|i| p.normal[i] as f32 / 127.0),
        color: [part(0), part(10), part(20), a],
        light: (p.light & 1023) as f32 / 1023.0,
        sky: (p.light >> 10 & 1023) as f32 / 1023.0,
        tex: [uv[0], uv[1], kz, anim],
    }
}

/// A section's solid mesh, packed: quads first (four vertices each, drawn
/// with the shared quad index as triangles 0 1 2 and 0 2 3), the ground's
/// before the rest; then the triangles that pair up into no quad.
#[derive(Clone, Default)]
pub struct Geo {
    pub verts: Vec<Packed>,
    /// quads in all, and how many of them are fine voxel ground
    pub quads: u32,
    pub ground_quads: u32,
    /// vertices of loose triangles, after the quads
    pub loose: u32,
    /// vertices as the meshers made them: the ground's (four a quad) and
    /// the rest (three a triangle), for statistics
    pub meshed: [u32; 2],
}

impl Geo {
    /// Pack the ground (four vertices a quad) and the other solid faces
    /// (three a triangle) of the section at `section`.
    pub fn pack(section: [i32; 3], ground: &[Vertex], opaque: &[Vertex]) -> Geo {
        let origin = section.map(|s| (s * SECTION) as f32);
        let mut verts: Vec<Packed> = Vec::with_capacity(ground.len() + opaque.len() * 2 / 3 + 6);
        verts.extend(ground.iter().map(|v| pack(v, origin)));
        let ground_quads = (ground.len() / 4) as u32;
        let mut loose: Vec<Packed> = Vec::new();
        let pairs = opaque.as_chunks::<6>();
        for six in pairs.0 {
            let t: [Packed; 6] = std::array::from_fn(|i| pack(&six[i], origin));
            match quad_of(&t) {
                Some(q) => verts.extend(q),
                None => loose.extend(t),
            }
        }
        loose.extend(pairs.1.iter().map(|v| pack(v, origin)));
        let quads = (verts.len() / 4) as u32;
        let n = loose.len() as u32;
        verts.append(&mut loose);
        Geo {
            verts,
            quads,
            ground_quads,
            loose: n,
            meshed: [ground.len() as u32, opaque.len() as u32],
        }
    }

    /// Triangles drawn.
    pub fn triangles(&self) -> u64 {
        self.quads as u64 * 2 + self.loose as u64 / 3
    }

    /// The ground's vertices read back (four a quad), for tests.
    pub fn ground(&self, section: [i32; 3]) -> Vec<Vertex> {
        let origin = section.map(|s| (s * SECTION) as f32);
        self.verts[..self.ground_quads as usize * 4]
            .iter()
            .map(|p| unpack(p, origin))
            .collect()
    }
}

/// Two triangles that share an edge, as the four corners of a quad whose
/// triangles (0 1 2) and (0 2 3) are the same two, wound the same way.
fn quad_of(t: &[Packed; 6]) -> Option<[Packed; 4]> {
    let (a, b) = (&t[..3], &t[3..]);
    // turn the first so the corner the second lacks is in the middle
    for r in 0..3 {
        let (x, y, z) = (a[r], a[(r + 1) % 3], a[(r + 2) % 3]);
        if b.contains(&y) {
            continue;
        }
        // the second, turned to start at x, must go on to z
        for s in 0..3 {
            if b[s] == x && b[(s + 1) % 3] == z {
                return Some([x, y, z, b[(s + 2) % 3]]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light::Light;
    use crate::render::textures;
    use crate::terrain_mesh;

    fn v(pos: [f32; 3]) -> Vertex {
        Vertex {
            pos,
            normal: [0.0, 1.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            light: 0.0,
            sky: 1.0,
            tex: [0.0, 0.0, 3.0, 0.0],
        }
    }

    #[test]
    fn two_triangles_on_an_edge_become_a_quad_with_the_same_triangles() {
        let o = [0.0; 3];
        let c = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ]
        .map(|p| pack(&v(p), o));
        // every way the meshers split a quad, every turn of both triangles
        for split in [[0, 1, 2, 0, 2, 3], [1, 2, 3, 1, 3, 0]] {
            for r1 in 0..3 {
                for r2 in 0..3 {
                    let six: [Packed; 6] = std::array::from_fn(|i| {
                        let k = if i < 3 {
                            (i + r1) % 3
                        } else {
                            3 + (i - 3 + r2) % 3
                        };
                        c[split[k]]
                    });
                    let q = quad_of(&six).expect("a quad");
                    let same = |a: [Packed; 3], b: [Packed; 3]| {
                        (0..3).any(|r| (0..3).all(|i| a[(i + r) % 3] == b[i]))
                    };
                    let (t1, t2) = ([six[0], six[1], six[2]], [six[3], six[4], six[5]]);
                    assert!(same([q[0], q[1], q[2]], t1), "{split:?} {r1} {r2}");
                    assert!(same([q[0], q[2], q[3]], t2), "{split:?} {r1} {r2}");
                }
            }
        }
        // two triangles apart, or wound against each other, stay triangles
        let far = pack(&v([5.0, 0.0, 5.0]), o);
        assert!(quad_of(&[c[0], c[1], c[2], c[0], far, c[3]]).is_none());
        assert!(quad_of(&[c[0], c[1], c[2], c[0], c[3], c[2]]).is_none());
    }

    /// How far a packed vertex may be from the one the mesher made.
    fn close(a: &Vertex, b: &Vertex) -> Result<(), String> {
        let near = |what: &str, x: f32, y: f32, tol: f32| {
            if (x - y).abs() <= tol {
                Ok(())
            } else {
                Err(format!("{what}: {x} packed as {y}"))
            }
        };
        for i in 0..3 {
            near("position", a.pos[i], b.pos[i], 0.5 / STEPS + 1e-5)?;
            near("normal", a.normal[i], b.normal[i], 0.5 / 127.0 + 1e-6)?;
            // a tenth of a step of an 8-bit picture at its darkest and brightest
            let tol = 0.0014 * a.color[i].max(0.0).sqrt() + 1e-5;
            near("colour", a.color[i], b.color[i], tol)?;
        }
        if a.color[3] >= 1.9 {
            near("glow", 2.0, b.color[3], 0.0)?;
        } else {
            near(
                "brightness",
                a.color[3],
                b.color[3],
                0.5 / SHADE_STEPS + 1e-6,
            )?;
        }
        near("light", a.light, b.light, 0.5 / 1023.0 + 1e-6)?;
        near("sky", a.sky, b.sky, 0.5 / 1023.0 + 1e-6)?;
        near("kind", a.tex[2], b.tex[2], 0.0)?;
        let kz = a.tex[2];
        let leaves = (KIND_FOLIAGE..KIND_WATER).contains(&kz);
        near(
            "animation",
            a.tex[3],
            b.tex[3],
            if leaves { 0.0051 } else { 0.000051 },
        )?;
        if kz >= KIND_VOXEL {
            near("voxels a metre and facing", a.tex[0], b.tex[0], 0.0)?;
            near("saturation and jitter", a.tex[1], b.tex[1], 0.0006)?;
        } else if (KIND_FOLIAGE..KIND_WATER).contains(&kz) {
            near("u", a.tex[0], b.tex[0], 0.5 / 64.0)?;
            near("v", a.tex[1], b.tex[1], 0.5 / 64.0)?;
        } else if kz >= 0.0 {
            near("u", a.tex[0], b.tex[0], 0.5 / 2048.0 + 1e-6)?;
            near("v", a.tex[1], b.tex[1], 0.5 / 2048.0 + 1e-6)?;
        }
        Ok(())
    }

    #[test]
    fn packing_keeps_what_the_shader_reads() {
        let o = [32.0, 16.0, 48.0];
        // ground: on the voxel grid, so its place is kept exactly
        let ground = Vertex {
            pos: [32.0 + 3.125, 16.0 + 15.875, 48.0 + 16.0],
            normal: [0.0, 0.0, -1.0],
            color: [1.1, 1.06, 0.68, 0.7031],
            light: 14.0 / 15.0,
            sky: 0.4,
            tex: [8.0 + 32.0 * 5.0, 135.13, KIND_VOXEL + 7.0, 0.0],
        };
        let back = unpack(&pack(&ground, o), o);
        assert_eq!(back.pos, ground.pos);
        assert_eq!(back.normal, ground.normal);
        close(&ground, &back).unwrap();
        // white is white
        let white = Vertex {
            color: [1.0; 4],
            ..ground
        };
        let back = unpack(&pack(&white, o), o);
        assert!(
            back.color.iter().all(|c| (c - 1.0).abs() < 1e-6),
            "{:?}",
            back.color
        );
        // a glowing face, a leaf, a shaped block past the section's edge
        let glow = Vertex {
            color: [0.9, 0.5, 0.1, 2.0],
            ..ground
        };
        assert_eq!(unpack(&pack(&glow, o), o).color[3], 2.0);
        let leaf = Vertex {
            pos: [32.0 - 0.37, 16.0 + 16.19, 48.0 + 4.5],
            normal: [0.577, 0.577, -0.577],
            color: [0.0055, 0.0619, 0.0152, 1.13],
            light: 0.0,
            sky: 1.0,
            tex: [33.0, 17.0, KIND_FOLIAGE + 12.0, 2.16],
        };
        close(&leaf, &unpack(&pack(&leaf, o), o)).unwrap();
        let shaped = Vertex {
            pos: [32.0 + 16.125, 16.0 - 0.125, 48.0 + 0.05],
            normal: [0.6, 0.8, 0.0],
            color: [0.55, 0.42, 0.25, 0.4592],
            light: 0.5,
            sky: 0.25,
            tex: [3.5, -1.5, 41.0, 8.0014],
        };
        close(&shaped, &unpack(&pack(&shaped, o), o)).unwrap();
    }

    /// Every vertex the meshers make for the town and the land round it
    /// packs within the tolerances, the ground's places exactly; and the
    /// packed mesh draws the same triangles.
    #[test]
    fn the_worlds_meshes_pack_within_tolerance() {
        let world = terrain_mesh::test_world([64, 48, 64]);
        let light = Light::new(&world);
        let _ = textures();
        let (mut all, mut quads) = (0usize, 0usize);
        for sy in 0..3 {
            for sz in 0..4 {
                for sx in 0..4 {
                    let s = [sx, sy, sz];
                    let geo = terrain_mesh::mesh(&world, &light, sx, sy, sz);
                    let o = s.map(|c| (c * SECTION) as f32);
                    for m in geo.ground.iter().chain(&geo.opaque) {
                        let back = unpack(&pack(m, o), o);
                        close(m, &back).unwrap_or_else(|e| panic!("section {s:?}: {e}"));
                    }
                    let packed = Geo::pack(s, &geo.ground, &geo.opaque);
                    for (m, p) in geo.ground.iter().zip(packed.ground(s)) {
                        assert_eq!(m.pos, p.pos, "ground is on the grid");
                    }
                    assert_eq!(
                        packed.triangles() as usize,
                        geo.ground.len() / 2 + geo.opaque.len() / 3
                    );
                    assert_eq!(
                        packed.verts.len(),
                        packed.quads as usize * 4 + packed.loose as usize
                    );
                    all += geo.ground.len() + geo.opaque.len();
                    quads += packed.quads as usize;
                }
            }
        }
        assert!(
            all > 10_000 && quads > 1000,
            "{all} vertices, {quads} quads"
        );
    }
}
