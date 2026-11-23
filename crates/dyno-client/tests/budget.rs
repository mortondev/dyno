//! The weight of the fixed scenes of the performance budget
//! (docs/design/performance.md), counted without a graphics card: the
//! vertex memory of the world a player holds at each of the bench's
//! cameras, the triangles of its meshes, and the triangles of the ground
//! cover round the camera. Counts, not times: machines differ, counts do not.
//!
//! The test fails when a count grew more than 15% over the recorded ones
//! (`tools/bench/counts.json`). When a change is meant to add that much
//! (new vegetation, a rebuilt world), record the counts again and say why
//! in the commit:
//!
//! ```sh
//! DYNO_BLESS=1 cargo test -p dyno-client --test budget
//! ```
//!
//! The scenes are those of `dyno bench` (`tools/bench/scenes.json`): the
//! world as a fresh server builds it from the pack's layout, the sections
//! within the server's view of where each camera's figure stands, lit and
//! meshed by the world worker for a camera there, at the default settings.

use dyno_client::worldwork::WorldWork;
use dyno_client::{cover, net, render, shot};
use dyno_world::World;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A count may grow this much over the recorded one.
const TOLERANCE: f64 = 0.15;

#[derive(Deserialize)]
struct Scenes {
    cameras: Vec<String>,
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
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct Counts {
    /// bytes of vertices the world's meshes take on the card
    vertex_bytes: u64,
    /// triangles of those meshes (all the sections held, not only those in view)
    triangles: u64,
    /// triangles of the ground cover drawn round the camera
    cover_triangles: u64,
}

#[derive(Serialize, Deserialize)]
struct Recorded {
    note: String,
    scenes: BTreeMap<String, Counts>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The world a player standing at `at` holds, meshed for a camera at `eye`.
fn weigh(world: &World, at: [f32; 3], eye: Vec3, dir: Vec3) -> Counts {
    let view = dyno_server::stream::StreamConfig::default().view;
    let mut held = World::sparse(world.size);
    let mut list = Vec::new();
    for s in net::sections_around(world, at, view) {
        let data = World::prepare_section(&world.encode_section(s)).unwrap();
        held.put_prepared(s, data.clone()).unwrap();
        list.push((s, data));
    }
    // the worker knows where the camera is before the land arrives, so
    // every section is meshed once, at its level of detail from there
    let mut work = WorldWork::start(World::sparse(world.size));
    let mut light = dyno_client::light::Light::blank(&held);
    work.focus(eye.into(), dir.into());
    work.put(list);
    let mut c = Counts {
        vertex_bytes: 0,
        triangles: 0,
        cover_triangles: 0,
    };
    let mut spots = Vec::new();
    let t0 = Instant::now();
    let mut seen = false;
    loop {
        work.poll(&mut light);
        for (_, geo) in work.queue.drain(..) {
            seen = true;
            // (sections with nothing to draw come unpacked and empty)
            let solid = geo.packed.unwrap_or_default();
            let plain = geo.trans.len() + geo.glows.len() * 6;
            c.vertex_bytes += std::mem::size_of_val(&solid.verts[..]) as u64
                + (plain * std::mem::size_of::<render::Vertex>()) as u64;
            c.triangles += solid.triangles() + plain as u64 / 3;
            spots.push(geo.cover);
        }
        if seen && work.idle() && work.queue.is_empty() {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(600),
            "the world worker finishes"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    c.cover_triangles = cover::weigh(eye, 1.0, spots.iter().map(|s| s.as_slice())).1;
    c
}

#[test]
fn the_fixed_scenes_keep_their_weight() {
    let scenes: Scenes = read(&root().join("tools/bench/scenes.json"));
    let cameras: Cameras = read(&root().join("tools/concepts/cameras.json"));
    // the world as a fresh server builds it: the whole of the pack's layout
    let pack = dyno_server::pack::Pack::builtin();
    let layout = pack.layout.as_ref().expect("the pack has a layout");
    let (world, built) = dyno_world::worldgen::build(layout, &pack.stamps, layout.size).unwrap();
    let h = built.hearth.map(|v| v as f32);
    let mut now = BTreeMap::new();
    for name in &scenes.cameras {
        let cam = cameras
            .cameras
            .iter()
            .find(|c| &c.name == name)
            .unwrap_or_else(|| panic!("no camera called {name}"));
        // where the figure stands and the play camera behind it, as
        // `dyno shot --stand` has them
        let at = [cam.stand[0] + h[0], cam.stand[2] + h[2]];
        let face = [cam.face[0] + h[0], cam.face[2] + h[2]];
        let (x, z) = (at[0].floor() as i32, at[1].floor() as i32);
        let y = shot::standing_y(&world, x, z, Some(cam.stand[1] + h[1]))
            .unwrap_or_else(|| panic!("{name}: nowhere to stand"));
        let feet = Vec3::new(at[0], y, at[1]);
        let yaw = shot::yaw_toward(at, face);
        let chase = shot::chase(&world, feet, yaw, -cam.pitch.to_radians(), cam.zoom);
        let c = weigh(&world, feet.into(), chase.eye, chase.dir);
        assert!(
            c.vertex_bytes > 1_000_000 && c.triangles > 100_000,
            "{name}: {c:?}"
        );
        now.insert(name.clone(), c);
    }
    // every value of every vertex of those meshes fitted its packed field
    assert_eq!(
        dyno_client::pack::cut(),
        0,
        "vertices of the world's meshes did not fit the packed format (pack.rs)"
    );
    let path = root().join("tools/bench/counts.json");
    if std::env::var_os("DYNO_BLESS").is_some() {
        let rec = Recorded {
            note: "The weight of the bench's still scenes, recorded by `DYNO_BLESS=1 cargo test -p dyno-client --test budget` (crates/dyno-client/tests/budget.rs): the test fails when one grows more than 15% over these. Do not edit by hand.".into(),
            scenes: now,
        };
        std::fs::write(&path, serde_json::to_string_pretty(&rec).unwrap() + "\n").unwrap();
        return;
    }
    let rec: Recorded = read(&path);
    let mut over = Vec::new();
    for (name, c) in &now {
        let Some(was) = rec.scenes.get(name) else {
            over.push(format!("{name}: not recorded"));
            continue;
        };
        for (what, a, b) in [
            ("vertex memory (bytes)", was.vertex_bytes, c.vertex_bytes),
            ("world triangles", was.triangles, c.triangles),
            ("cover triangles", was.cover_triangles, c.cover_triangles),
        ] {
            if b as f64 > a as f64 * (1.0 + TOLERANCE) {
                over.push(format!(
                    "{name}: {what} {a} -> {b} (+{:.1}%)",
                    (b as f64 / a.max(1) as f64 - 1.0) * 100.0
                ));
            }
        }
    }
    assert!(
        over.is_empty(),
        "the fixed scenes grew more than {}% over tools/bench/counts.json (if that is meant, record them again: DYNO_BLESS=1 cargo test -p dyno-client --test budget):\n  {}",
        TOLERANCE * 100.0,
        over.join("\n  ")
    );
}
