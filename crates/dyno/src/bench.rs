//! `dyno bench`: the fixed scenes of the performance budget
//! (docs/design/performance.md), rendered headless at each graphics tier,
//! measured, and written up as JSON and Markdown.
//!
//! The scenes are data (`tools/bench/scenes.json`): six of the gauntlet's
//! gameplay cameras (`tools/concepts/cameras.json`), a camera flight, the
//! tiers and the world they are measured on. Each still scene is the stand
//! shot of `dyno shot --stand` with nothing that moves in the saved picture,
//! so two builds' pictures can be compared.

use dyno_client::bench::{self, Flight, Request, SceneResult, Tier};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "usage: dyno bench <address>|--local [--port 7720] [--scenes tools/bench/scenes.json] [--cameras tools/concepts/cameras.json]
                  [--tier NAME|all] [--only SCENE] [--no-flight] [--frames N] [--size WxH] [--out DIR] [--label TEXT]
                  [--check REPORT.json] [--tolerance 15]";

#[derive(Deserialize)]
struct Scenes {
    /// a new local world's size and seed
    world: WorldSpec,
    size: [u32; 2],
    /// frames measured at each still camera
    frames: usize,
    /// names in the cameras file
    cameras: Vec<String>,
    flight: Flight,
    /// the tier whose budgets the game is held to
    default_tier: String,
    tiers: Vec<Tier>,
}

#[derive(Deserialize, Serialize, Clone)]
struct WorldSpec {
    size: [u32; 3],
    seed: u32,
}

#[derive(Deserialize)]
struct Cameras {
    cameras: Vec<Camera>,
}

#[derive(Deserialize)]
struct Camera {
    name: String,
    stand: [f32; 3],
    face: [f32; 3],
    zoom: f32,
    pitch: f32,
    time: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Report {
    pub label: String,
    pub version: String,
    /// "release" or "debug"
    pub build: String,
    pub adapter: String,
    pub size: [u32; 2],
    /// the world measured: "local 384x96x384 seed 20261122" or an address
    pub world: String,
    /// the machine's load average when the run began (times are noisy when
    /// it is busy; counts are not)
    pub load: String,
    pub default_tier: String,
    pub tiers: Vec<Tier>,
    pub scenes: Vec<SceneResult>,
}

pub struct Options {
    pub addr: Option<String>,
    pub local: bool,
    pub port: u16,
    pub scenes: PathBuf,
    pub cameras: PathBuf,
    pub tier: String,
    pub only: Option<String>,
    pub flight: bool,
    pub frames: Option<usize>,
    pub size: Option<[u32; 2]>,
    pub out: PathBuf,
    pub label: String,
    pub check: Option<PathBuf>,
    pub tolerance: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            addr: None,
            local: false,
            port: 7720,
            scenes: "tools/bench/scenes.json".into(),
            cameras: "tools/concepts/cameras.json".into(),
            tier: "all".into(),
            only: None,
            flight: true,
            frames: None,
            size: None,
            out: "bench".into(),
            label: String::new(),
            check: None,
            tolerance: 15.0,
        }
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Run the scenes, write `report.json`, `report.md` and the pictures under
/// the output folder. Err with what regressed when `--check` fails.
pub fn run(o: Options) -> Result<String, String> {
    let scenes: Scenes = read(&o.scenes)?;
    let cameras: Cameras = read(&o.cameras)?;
    let tiers: Vec<Tier> = scenes
        .tiers
        .iter()
        .filter(|t| o.tier == "all" || t.name == o.tier)
        .cloned()
        .collect();
    if tiers.is_empty() {
        return Err(format!("no tier called {}", o.tier));
    }
    let size = o.size.unwrap_or(scenes.size);
    let frames = o.frames.unwrap_or(scenes.frames);
    std::fs::create_dir_all(&o.out).map_err(|e| e.to_string())?;
    let load = std::fs::read_to_string("/proc/loadavg")
        .map(|s| s.split(' ').take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_else(|_| "unknown".into());

    let mut _server = None;
    let (addr, world) = if o.local {
        let dir = std::env::temp_dir().join(format!("dyno-bench-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let cfg = dyno_server::Config {
            bind: ([127, 0, 0, 1], o.port).into(),
            world_path: dir.join("world.dynw"),
            world_size: scenes.world.size,
            seed: scenes.world.seed,
            ..Default::default()
        };
        let h = dyno_server::start(cfg).map_err(|e| e.to_string())?;
        let addr = format!("127.0.0.1:{}", h.local_addr.port());
        _server = Some((h, dir));
        let [x, y, z] = scenes.world.size;
        (
            addr,
            format!("local {x}x{y}x{z} seed {}", scenes.world.seed),
        )
    } else {
        let a = o.addr.clone().ok_or("give a server address or --local")?;
        (a.clone(), a)
    };

    bench::set_timing(true);
    let mut results: Vec<SceneResult> = Vec::new();
    let mut adapter = String::new();
    for tier in &tiers {
        dyno_client::post::Quality::force(Some(tier.graphics));
        let dir = o.out.join(&tier.name);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let shot = |name: &str| dyno_client::shot::ShotOptions {
            addr: addr.clone(),
            name: "bench".into(),
            pos: None,
            yaw: 0.0,
            pitch: -0.15,
            time: None,
            width: size[0],
            height: size[1],
            out: dir.join(format!("{name}.png")),
            ui: "hud".into(),
            player: None,
            zoom: None,
            front: None,
            stand: None,
        };
        for name in &scenes.cameras {
            if o.only.as_ref().is_some_and(|n| n != name) {
                continue;
            }
            let cam = cameras
                .cameras
                .iter()
                .find(|c| &c.name == name)
                .ok_or_else(|| format!("no camera called {name} in {}", o.cameras.display()))?;
            bench::set_request(Some(Request {
                name: name.clone(),
                tier: tier.clone(),
                frames,
                flight: None,
            }));
            let mut s = shot(name);
            s.time = Some(cam.time);
            s.zoom = Some(cam.zoom);
            s.stand = Some(dyno_client::shot::Stand {
                at: [cam.stand[0], cam.stand[2]],
                near_y: Some(cam.stand[1]),
                face: cam.face,
                pitch: cam.pitch,
                from_hearth: true,
            });
            dyno_client::shot::shot(s).map_err(|e| format!("{name} ({}): {e}", tier.name))?;
            let r = bench::take_result().ok_or("the shot measured nothing")?;
            eprintln!("{}", line(&r));
            results.push(r);
        }
        if o.flight && o.only.as_ref().is_none_or(|n| *n == scenes.flight.name) {
            bench::set_request(Some(Request {
                name: scenes.flight.name.clone(),
                tier: tier.clone(),
                frames,
                flight: Some(scenes.flight.clone()),
            }));
            dyno_client::shot::shot(shot(&scenes.flight.name))
                .map_err(|e| format!("{} ({}): {e}", scenes.flight.name, tier.name))?;
            let r = bench::take_result().ok_or("the flight measured nothing")?;
            eprintln!("{}", line(&r));
            results.push(r);
        }
        if adapter.is_empty() {
            adapter = dyno_client::render::last_adapter();
        }
    }
    bench::set_request(None);
    dyno_client::post::Quality::force(None);

    let report = Report {
        label: o.label.clone(),
        version: dyno_update::VERSION.into(),
        build: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .into(),
        adapter,
        size,
        world,
        load,
        default_tier: scenes.default_tier.clone(),
        tiers,
        scenes: results,
    };
    let json = o.out.join("report.json");
    std::fs::write(
        &json,
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let md = o.out.join("report.md");
    std::fs::write(&md, markdown(&report)).map_err(|e| e.to_string())?;
    if let Some((_, dir)) = &_server {
        let _ = std::fs::remove_dir_all(dir);
    }
    let mut out = format!("wrote {} and {}", json.display(), md.display());
    if let Some(path) = &o.check {
        let base: Report = read(path)?;
        let bad = check(&base, &report, o.tolerance);
        if !bad.is_empty() {
            return Err(format!(
                "over {}% more than {}:\n  {}",
                o.tolerance,
                path.display(),
                bad.join("\n  ")
            ));
        }
        out += &format!("; counts within {}% of {}", o.tolerance, path.display());
    }
    Ok(out)
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn line(r: &SceneResult) -> String {
    format!(
        "{:>7} {:<20} GPU {} ms (95% {}), CPU {:.2} ms ({:.2} on the wall), {} draws, {:.2} M triangles, vertices {:.0} MB, mesh {:.2} ms a section",
        r.tier,
        r.name,
        r.gpu_ms.map_or("-".into(), |d| format!("{:.2}", d.median)),
        r.gpu_ms.map_or("-".into(), |d| format!("{:.2}", d.p95)),
        r.cpu_ms.median,
        r.wall_ms.median,
        r.draws,
        r.triangles as f64 / 1e6,
        mb(r.memory.vertex_total()),
        r.mesh.cpu_mean_ms.unwrap_or(r.mesh.wall_mean_ms),
    )
}

/// The counts of the still scenes that grew by more than `tolerance`
/// percent over `base`. Times are not compared: machines differ.
pub fn check(base: &Report, now: &Report, tolerance: f64) -> Vec<String> {
    let mut bad = Vec::new();
    for r in now.scenes.iter().filter(|r| r.kind == "still") {
        let Some(b) = base
            .scenes
            .iter()
            .find(|b| b.name == r.name && b.tier == r.tier)
        else {
            continue;
        };
        let pairs = [
            (
                "vertex memory (bytes)",
                b.memory.vertex_total(),
                r.memory.vertex_total(),
            ),
            ("triangles a frame", b.triangles, r.triangles),
        ];
        for (what, was, is) in pairs {
            if is as f64 > was as f64 * (1.0 + tolerance / 100.0) {
                bad.push(format!(
                    "{} at {}: {what} {was} -> {is} (+{:.1}%)",
                    r.name,
                    r.tier,
                    (is as f64 / was.max(1) as f64 - 1.0) * 100.0
                ));
            }
        }
    }
    bad
}

pub fn markdown(r: &Report) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# dyno bench{}\n",
        if r.label.is_empty() {
            String::new()
        } else {
            format!(": {}", r.label)
        }
    );
    let _ = writeln!(
        s,
        "- build: dyno {} ({})\n- adapter: {}\n- picture: {}x{}\n- world: {}\n- load average at the start: {}\n",
        r.version, r.build, r.adapter, r.size[0], r.size[1], r.world, r.load
    );
    let _ = writeln!(
        s,
        "| tier | graphics | ground cover | leaf detail | life | water |\n|---|---|---|---|---|---|"
    );
    for t in &r.tiers {
        let _ = writeln!(
            s,
            "| {}{} | {} | {} | {} | {} | {} |",
            t.name,
            if t.name == r.default_tier {
                " (default)"
            } else {
                ""
            },
            t.graphics,
            t.ground_cover,
            t.leaf_detail,
            t.life,
            t.water_quality
        );
    }
    for t in &r.tiers {
        let scenes: Vec<&SceneResult> = r.scenes.iter().filter(|x| x.tier == t.name).collect();
        if scenes.is_empty() {
            continue;
        }
        let _ = writeln!(s, "\n## {}\n", t.name);
        let _ = writeln!(
            s,
            "| scene | GPU ms (median) | GPU ms (95%) | CPU ms (median) | CPU ms (95%) | wall ms (median) | draw calls | triangles | cover triangles | vertex memory MB | texture memory MB | mesh ms a section (CPU mean / 95%) |\n|---|---|---|---|---|---|---|---|---|---|---|---|"
        );
        for x in &scenes {
            let gpu = |f: fn(&bench::Dist) -> f64| {
                x.gpu_ms
                    .as_ref()
                    .map_or("-".into(), |d| format!("{:.2}", f(d)))
            };
            let mesh = match (&x.mesh.cpu_mean_ms, &x.mesh.cpu_ms) {
                (Some(m), Some(d)) => format!("{m:.2} / {:.2}", d.p95),
                _ => format!(
                    "{:.2} / {:.2} (wall)",
                    x.mesh.wall_mean_ms, x.mesh.wall_ms.p95
                ),
            };
            let _ = writeln!(
                s,
                "| {} | {} | {} | {:.2} | {:.2} | {:.2} | {} | {:.2} M | {:.2} M | {:.0} | {:.0} | {} |",
                x.name,
                gpu(|d| d.median),
                gpu(|d| d.p95),
                x.cpu_ms.median,
                x.cpu_ms.p95,
                x.wall_ms.median,
                x.draws,
                x.triangles as f64 / 1e6,
                x.cover_triangles as f64 / 1e6,
                mb(x.peak_vertex_bytes.max(x.memory.vertex_total())),
                mb(x.memory.texture_total()),
                mesh
            );
        }
        let _ = writeln!(s, "\nBy pass (median GPU ms / draw calls / triangles):\n");
        let _ = write!(s, "| scene |");
        for p in bench::PASSES {
            let _ = write!(s, " {p} |");
        }
        let _ = write!(s, "\n|---|");
        for _ in bench::PASSES {
            let _ = write!(s, "---|");
        }
        let _ = writeln!(s);
        for x in &scenes {
            let _ = write!(s, "| {} |", x.name);
            for p in &x.passes {
                let ms = p.gpu_ms.map_or("-".into(), |m| format!("{m:.2}"));
                let tris = if p.triangles >= 100_000 {
                    format!("{:.2} M", p.triangles as f64 / 1e6)
                } else {
                    p.triangles.to_string()
                };
                let _ = write!(s, " {ms} / {} / {tris} |", p.draws);
            }
            let _ = writeln!(s);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(vertex: u64, triangles: u64) -> Report {
        Report {
            label: String::new(),
            version: String::new(),
            build: String::new(),
            adapter: String::new(),
            size: [1, 1],
            world: String::new(),
            load: String::new(),
            default_tier: "high".into(),
            tiers: Vec::new(),
            scenes: vec![SceneResult {
                name: "meadow".into(),
                tier: "high".into(),
                kind: "still".into(),
                triangles,
                memory: bench::Memory {
                    world_vertices: vertex,
                    ..Default::default()
                },
                ..Default::default()
            }],
        }
    }

    #[test]
    fn the_check_fails_past_its_tolerance_only() {
        let base = report(1000, 500);
        assert!(check(&base, &report(1100, 560), 15.0).is_empty());
        assert!(check(&base, &report(400, 100), 15.0).is_empty());
        assert_eq!(check(&base, &report(1200, 500), 15.0).len(), 1);
        assert_eq!(check(&base, &report(1200, 600), 15.0).len(), 2);
        // and the report it reads is the one it writes
        let text = serde_json::to_string(&base).unwrap();
        let back: Report = serde_json::from_str(&text).unwrap();
        assert_eq!(back.scenes, base.scenes);
        assert!(markdown(&base).contains("| meadow |"));
    }
}
