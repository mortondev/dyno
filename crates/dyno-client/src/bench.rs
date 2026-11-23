//! Measurements for `dyno bench` (docs/design/performance.md): what a frame
//! costs on the graphics card and the processor, what it draws pass by
//! pass, what the world's meshes and the pictures weigh in memory, and how
//! long a section takes to mesh.
//!
//! The renderer counts as it draws (`FrameStats`); with timing on it also
//! stamps the GPU's clock between passes (`GpuTimer`). `dyno shot` runs the
//! scene asked for (`request`) and leaves the result (`take_result`).

use egui_wgpu::wgpu;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// The passes of a frame, in the order they are drawn.
pub const PASSES: [&str; 10] = [
    "shadow near",
    "shadow mid",
    "shadow far",
    "world",
    "occlusion",
    "sky",
    "water",
    "translucent",
    "post",
    "interface",
];
pub const SHADOW: usize = 0;
pub const WORLD: usize = 3;
pub const OCCLUSION: usize = 4;
pub const SKY: usize = 5;
pub const WATER: usize = 6;
pub const TRANSLUCENT: usize = 7;
pub const POST: usize = 8;
pub const INTERFACE: usize = 9;

/// What one frame drew, pass by pass, and what it cost.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub draws: [u32; PASSES.len()],
    pub triangles: [u64; PASSES.len()],
    /// of the world pass's triangles, the ground cover's
    pub cover_triangles: u64,
    pub cover_instances: u32,
    /// sections drawn in the world pass, of those resident
    pub sections_shown: u32,
    pub sections: u32,
    /// milliseconds on the graphics card, per pass (timing on)
    pub gpu_ms: [f64; PASSES.len()],
    /// the whole frame on the graphics card (timing on)
    pub gpu_total_ms: f64,
    /// building and submitting the frame on this thread, in milliseconds:
    /// by the clock on the wall, and of the thread's own processor time
    /// (which a busy machine does not stretch)
    pub cpu_ms: f64,
    pub cpu_own_ms: f64,
}

/// Times a stretch of work on this thread both ways.
pub struct Stopwatch {
    wall: std::time::Instant,
    own: Option<u64>,
}

impl Stopwatch {
    pub fn start() -> Stopwatch {
        Stopwatch {
            wall: std::time::Instant::now(),
            own: thread_cpu_nanos(),
        }
    }

    /// Milliseconds since the start: on the wall, and of this thread's own
    /// processor time (the wall's where the system cannot say).
    pub fn ms(&self) -> (f64, f64) {
        let wall = self.wall.elapsed().as_secs_f64() * 1e3;
        let own = self
            .own
            .zip(thread_cpu_nanos())
            .map_or(wall, |(a, b)| b.saturating_sub(a) as f64 / 1e6);
        (wall, own)
    }
}

impl FrameStats {
    #[inline]
    pub fn draw(&mut self, pass: usize, triangles: u64) {
        self.draws[pass] += 1;
        self.triangles[pass] += triangles;
    }
}

static TIMING: AtomicBool = AtomicBool::new(false);

/// Ask renderers made from now on to time their passes on the GPU (where
/// the adapter can), and the meshers to keep a sample per section.
pub fn set_timing(on: bool) {
    TIMING.store(on, Ordering::Relaxed);
}

pub fn timing() -> bool {
    TIMING.load(Ordering::Relaxed)
}

/// The device features GPU timing needs.
pub fn timing_features() -> wgpu::Features {
    wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
}

/// Stamps of the GPU's clock between the passes of a frame.
pub struct GpuTimer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    /// nanoseconds a tick
    period: f32,
    next: u32,
    /// which pass ended at each stamp after the first
    ends: Vec<usize>,
}

const STAMPS: u32 = PASSES.len() as u32 + 1;

impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<GpuTimer> {
        if !timing() || !device.features().contains(timing_features()) {
            return None;
        }
        let size = STAMPS as u64 * 8;
        let buf = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timer"),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        Some(GpuTimer {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("timer"),
                ty: wgpu::QueryType::Timestamp,
                count: STAMPS,
            }),
            resolve: buf(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC),
            read: buf(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ),
            period: queue.get_timestamp_period(),
            next: 0,
            ends: Vec::new(),
        })
    }

    /// The frame starts here.
    pub fn begin(&mut self, enc: &mut wgpu::CommandEncoder) {
        self.next = 0;
        self.ends.clear();
        enc.write_timestamp(&self.set, 0);
        self.next = 1;
    }

    /// `pass` ends here.
    pub fn end(&mut self, enc: &mut wgpu::CommandEncoder, pass: usize) {
        if self.next == 0 || self.next >= STAMPS {
            return;
        }
        enc.write_timestamp(&self.set, self.next);
        self.next += 1;
        self.ends.push(pass);
    }

    /// The last of the frame's commands: bring the stamps where they can be read.
    pub fn resolve(&mut self, enc: &mut wgpu::CommandEncoder) {
        if self.next < 2 {
            return;
        }
        enc.resolve_query_set(&self.set, 0..self.next, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.read, 0, self.next as u64 * 8);
    }

    /// After the frame was submitted: wait for it and fill in the times.
    pub fn read(&mut self, device: &wgpu::Device, stats: &mut FrameStats) {
        if self.next < 2 {
            return;
        }
        let n = self.next as usize;
        let slice = self.read.slice(..n as u64 * 8);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        {
            let data = slice.get_mapped_range();
            let t: Vec<u64> = data
                .as_chunks::<8>()
                .0
                .iter()
                .map(|b| u64::from_le_bytes(*b))
                .collect();
            let ms = |a: u64, b: u64| b.saturating_sub(a) as f64 * self.period as f64 / 1e6;
            for (i, pass) in self.ends.iter().enumerate() {
                stats.gpu_ms[*pass] += ms(t[i], t[i + 1]);
            }
            stats.gpu_total_ms = ms(t[0], t[n - 1]);
        }
        self.read.unmap();
        self.next = 0;
    }
}

/// Bytes a texture of this shape takes on the card (its mip chain included).
pub fn texture_bytes(format: wgpu::TextureFormat, w: u32, h: u32, layers: u32, mips: u32) -> u64 {
    let px = format
        .block_copy_size(Some(wgpu::TextureAspect::All))
        .or_else(|| format.block_copy_size(Some(wgpu::TextureAspect::DepthOnly)))
        .unwrap_or(4) as u64;
    (0..mips)
        .map(|m| (w >> m).max(1) as u64 * (h >> m).max(1) as u64)
        .sum::<u64>()
        * layers as u64
        * px
}

/// Memory on the graphics card, in bytes.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    /// the near world's meshes
    pub world_vertices: u64,
    /// the far view's
    pub far_vertices: u64,
    /// the cover's models and instances
    pub cover: u64,
    /// the shared index of the ground's quads
    pub index: u64,
    /// block textures
    pub textures: u64,
    /// the shadow maps
    pub shadow_maps: u64,
    /// the picture, its depth, occlusion, bloom and what the water reads
    pub targets: u64,
}

impl Memory {
    /// Everything held as vertices and indices.
    pub fn vertex_total(&self) -> u64 {
        self.world_vertices + self.far_vertices + self.cover + self.index
    }
    pub fn texture_total(&self) -> u64 {
        self.textures + self.shadow_maps + self.targets
    }
}

/// The player's settings a tier pins (settings.rs).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tier {
    pub name: String,
    /// shadow detail, occlusion, bloom: 0 low .. 3 ultra
    pub graphics: u8,
    pub ground_cover: f32,
    pub leaf_detail: f32,
    pub life: u8,
    pub water_quality: u8,
}

/// A camera flight: straight runs between points (metres from the hearth),
/// a few metres over the ground.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Flight {
    pub name: String,
    pub points: Vec<[f32; 2]>,
    /// metres a second
    pub speed: f32,
    pub seconds: f32,
}

impl Flight {
    /// Where the camera is `t` seconds in, the way it heads, and whether
    /// the flight is over. `origin` is added to the points.
    pub fn at(&self, t: f32, origin: [f32; 2]) -> ([f32; 2], [f32; 2], bool) {
        let p = |i: usize| {
            let q = self.points[i.min(self.points.len() - 1)];
            [q[0] + origin[0], q[1] + origin[1]]
        };
        let mut left = t.min(self.seconds) * self.speed;
        let mut head = [0.0, -1.0];
        for i in 0..self.points.len().saturating_sub(1) {
            let (a, b) = (p(i), p(i + 1));
            let d = [b[0] - a[0], b[1] - a[1]];
            let len = d[0].hypot(d[1]).max(1e-3);
            head = [d[0] / len, d[1] / len];
            if left <= len {
                let k = left / len;
                return ([a[0] + d[0] * k, a[1] + d[1] * k], head, t >= self.seconds);
            }
            left -= len;
        }
        (p(self.points.len().saturating_sub(1)), head, true)
    }
}

/// What `dyno shot` is asked to measure.
#[derive(Clone, Debug)]
pub struct Request {
    /// the scene's name, for its result
    pub name: String,
    pub tier: Tier,
    /// frames measured at a still camera
    pub frames: usize,
    /// fly this instead of standing still
    pub flight: Option<Flight>,
}

/// Median, 95th percentile and the worst.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Dist {
    pub median: f64,
    pub p95: f64,
    pub max: f64,
}

impl Dist {
    pub fn of(v: &[f64]) -> Dist {
        if v.is_empty() {
            return Dist::default();
        }
        let mut v = v.to_vec();
        v.sort_by(|a, b| a.total_cmp(b));
        let q = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
        Dist {
            median: q(0.5),
            p95: q(0.95),
            max: q(1.0),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PassResult {
    pub name: String,
    /// draw calls a frame (the median frame's)
    pub draws: u32,
    pub triangles: u64,
    /// median milliseconds on the graphics card (None: not timed)
    pub gpu_ms: Option<f64>,
}

/// Meshing sections on the world thread's cores.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MeshResult {
    pub sections: u64,
    /// milliseconds a section by the clock on the wall
    pub wall_ms: Dist,
    pub wall_mean_ms: f64,
    /// milliseconds a section of its thread's own processor time (None
    /// where the system cannot say)
    pub cpu_ms: Option<Dist>,
    pub cpu_mean_ms: Option<f64>,
}

/// One scene measured at one tier.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SceneResult {
    pub name: String,
    pub tier: String,
    /// "still" or "flight"
    pub kind: String,
    pub frames: usize,
    /// the whole frame on the graphics card (None: the adapter cannot time)
    pub gpu_ms: Option<Dist>,
    /// the main thread's work on a frame (world uploads, ambient life,
    /// building and submitting the frame), of its own processor time
    pub cpu_ms: Dist,
    /// the same by the clock on the wall: longer on a busy machine
    pub wall_ms: Dist,
    /// of that, the ambient life (median)
    pub life_ms: f64,
    pub passes: Vec<PassResult>,
    pub draws: u32,
    pub triangles: u64,
    pub cover_triangles: u64,
    pub cover_instances: u32,
    pub sections_shown: u32,
    pub sections: u32,
    /// vertices held by the near world's meshes
    pub world_vertices: u64,
    pub memory: Memory,
    /// the most the world's vertex memory came to (a flight)
    pub peak_vertex_bytes: u64,
    pub mesh: MeshResult,
    /// frames over 16.7 ms on the main thread, by the clock on the wall
    pub slow_frames: usize,
}

static REQUEST: Mutex<Option<Request>> = Mutex::new(None);
static RESULT: Mutex<Option<SceneResult>> = Mutex::new(None);
static MESH: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());

/// Ask the next `dyno shot` to measure (None: an ordinary shot).
pub fn set_request(r: Option<Request>) {
    *REQUEST.lock().unwrap() = r;
}

pub fn request() -> Option<Request> {
    REQUEST.lock().unwrap().clone()
}

pub fn put_result(r: SceneResult) {
    *RESULT.lock().unwrap() = Some(r);
}

pub fn take_result() -> Option<SceneResult> {
    RESULT.lock().unwrap().take()
}

/// This thread's own processor time so far, in nanoseconds.
pub fn thread_cpu_nanos() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: `ts` is a valid timespec for the call to fill in
        let ok = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) } == 0;
        ok.then(|| ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// A section was meshed: this long by the wall clock and of processor time
/// (0: unknown), in nanoseconds. Kept only while timing is on.
pub fn mesh_sample(wall: u64, cpu: u64) {
    if timing() {
        MESH.lock().unwrap().push((wall, cpu));
    }
}

/// The meshing samples since the last call.
pub fn take_mesh() -> MeshResult {
    let v = std::mem::take(&mut *MESH.lock().unwrap());
    if v.is_empty() {
        return MeshResult::default();
    }
    let wall: Vec<f64> = v.iter().map(|s| s.0 as f64 / 1e6).collect();
    let cpu: Vec<f64> = v.iter().map(|s| s.1 as f64 / 1e6).collect();
    let mean = |x: &[f64]| x.iter().sum::<f64>() / x.len() as f64;
    let timed = v.iter().any(|s| s.1 > 0);
    MeshResult {
        sections: v.len() as u64,
        wall_ms: Dist::of(&wall),
        wall_mean_ms: mean(&wall),
        cpu_ms: timed.then(|| Dist::of(&cpu)),
        cpu_mean_ms: timed.then(|| mean(&cpu)),
    }
}

/// Collects a scene's frames into its result.
#[derive(Default)]
pub struct Collector {
    frames: Vec<FrameStats>,
    cpu: Vec<f64>,
    wall: Vec<f64>,
    life: Vec<f64>,
    peak_vertex_bytes: u64,
}

impl Collector {
    /// One frame: what the renderer counted, the main thread's time on it
    /// before the renderer took over (on the wall, and its own), and the
    /// ambient life's part, in milliseconds.
    pub fn frame(&mut self, f: FrameStats, before: (f64, f64), life_ms: f64, vertex_bytes: u64) {
        self.frames.push(f);
        self.wall.push(before.0 + f.cpu_ms);
        self.cpu.push(before.1 + f.cpu_own_ms);
        self.life.push(life_ms);
        self.peak_vertex_bytes = self.peak_vertex_bytes.max(vertex_bytes);
    }

    pub fn finish(
        self,
        name: &str,
        tier: &str,
        kind: &str,
        world_vertices: u64,
        memory: Memory,
    ) -> SceneResult {
        let n = self.frames.len();
        let timed = self.frames.iter().any(|f| f.gpu_total_ms > 0.0);
        let col = |get: &dyn Fn(&FrameStats) -> f64| -> Vec<f64> {
            self.frames.iter().map(get).collect()
        };
        let median = |get: &dyn Fn(&FrameStats) -> f64| Dist::of(&col(get)).median;
        let passes: Vec<PassResult> = (0..PASSES.len())
            .map(|i| PassResult {
                name: PASSES[i].into(),
                draws: median(&|f| f.draws[i] as f64) as u32,
                triangles: median(&|f| f.triangles[i] as f64) as u64,
                gpu_ms: timed.then(|| median(&|f| f.gpu_ms[i])),
            })
            .collect();
        SceneResult {
            name: name.into(),
            tier: tier.into(),
            kind: kind.into(),
            frames: n,
            gpu_ms: timed.then(|| Dist::of(&col(&|f| f.gpu_total_ms))),
            cpu_ms: Dist::of(&self.cpu),
            wall_ms: Dist::of(&self.wall),
            life_ms: Dist::of(&self.life).median,
            draws: passes.iter().map(|p| p.draws).sum(),
            triangles: passes.iter().map(|p| p.triangles).sum(),
            passes,
            cover_triangles: median(&|f| f.cover_triangles as f64) as u64,
            cover_instances: median(&|f| f.cover_instances as f64) as u32,
            sections_shown: median(&|f| f.sections_shown as f64) as u32,
            sections: median(&|f| f.sections as f64) as u32,
            world_vertices,
            peak_vertex_bytes: self.peak_vertex_bytes.max(memory.vertex_total()),
            memory,
            mesh: take_mesh(),
            slow_frames: self.wall.iter().filter(|c| **c > 16.7).count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flight_follows_its_points() {
        let f = Flight {
            name: "loop".into(),
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
            speed: 5.0,
            seconds: 3.0,
        };
        let (p, head, done) = f.at(1.0, [100.0, 200.0]);
        assert_eq!((p, head, done), ([105.0, 200.0], [1.0, 0.0], false));
        let (p, head, done) = f.at(3.0, [0.0, 0.0]);
        assert_eq!((p, head, done), ([10.0, 5.0], [0.0, 1.0], true));
        // past its last point it stays there
        let long = Flight { seconds: 60.0, ..f };
        assert_eq!(long.at(50.0, [0.0, 0.0]).0, [10.0, 10.0]);
    }

    #[test]
    fn distributions_and_texture_sizes() {
        let d = Dist::of(&[5.0, 1.0, 3.0, 2.0, 4.0]);
        assert_eq!((d.median, d.max), (3.0, 5.0));
        // 16 px with its mips: 256 + 64 + 16 + 4 + 1 texels of 4 bytes, 2 layers
        let b = texture_bytes(wgpu::TextureFormat::Rgba8UnormSrgb, 16, 16, 2, 5);
        assert_eq!(b, 341 * 4 * 2);
        assert_eq!(
            texture_bytes(wgpu::TextureFormat::Depth32Float, 8, 8, 3, 1),
            64 * 4 * 3
        );
    }
}
