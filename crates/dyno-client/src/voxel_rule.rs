//! The tests that hold the voxel rule (docs/art/voxel-rule.md): everything
//! drawn is cubes on the ladder of sizes (`ladder.rs`).
//!
//! - **The mesh test** (`everything_drawn_is_cubes_on_the_ladder`) takes
//!   what every mesher makes (sections of the built world round the
//!   gauntlet's cameras at every level of detail, every shape model, cover
//!   model, critter, figure, particle and far-view column) and checks each
//!   face: it is an axis-aligned rectangle in its object's frame, its stored
//!   normal is the face's own, and its corners lie on the object's grid.
//! - **The pack test** (`the_pack_keeps_the_voxel_rule`) reads the pack's
//!   data: no round parts, slopes, chamfers, turned props, tapers, bevels,
//!   scales or smooth meshes, and every model on a ladder step.
//! - **The shader test** (`the_shaders_keep_cubes_whole`) looks for the
//!   places a shader moves vertices or bends normals.
//!
//! What another lane has still to fix is listed in the allowlists below,
//! each entry with its owner and the day it was written. The lists only
//! shrink: an entry that no longer matches anything fails the test too.
//! docs/art/voxel-audit.md has the whole audit.

use std::collections::BTreeMap;

/// One triangle of an object: its corners, and the normals stored with them
/// when the stream stores normals.
pub(crate) struct Tri {
    pub p: [[f32; 3]; 3],
    pub n: Option<[[f32; 3]; 3]>,
}

/// Something drawn, in its own frame.
pub(crate) struct Object {
    /// what the report and the allowlist call it
    pub name: String,
    /// which part of it (for the example in the report)
    pub part: String,
    /// cubes per unit of its positions
    pub per_unit: f32,
    /// cubes per metre, for the ladder (the same as `per_unit` for
    /// something whose positions are in metres)
    pub per_metre: f32,
    /// a corner of its grid; `None`: its first vertex is taken as one
    pub origin: Option<[f32; 3]>,
    pub tris: Vec<Tri>,
}

impl Object {
    /// From quads of four corners, each drawn as (0, 1, 2) and (0, 2, 3).
    pub(crate) fn from_quads(
        name: impl Into<String>,
        part: impl Into<String>,
        per_unit: f32,
        origin: Option<[f32; 3]>,
        quads: impl IntoIterator<Item = ([[f32; 3]; 4], Option<[[f32; 3]; 4]>)>,
    ) -> Object {
        let mut tris = Vec::new();
        for (p, n) in quads {
            for [a, b, c] in [[0, 1, 2], [0, 2, 3]] {
                tris.push(Tri {
                    p: [p[a], p[b], p[c]],
                    n: n.map(|n| [n[a], n[b], n[c]]),
                });
            }
        }
        Object {
            name: name.into(),
            part: part.into(),
            per_unit,
            per_metre: per_unit,
            origin,
            tris,
        }
    }

    /// From a triangle list: (position, stored normal) three at a time.
    pub(crate) fn from_list(
        name: impl Into<String>,
        part: impl Into<String>,
        per_unit: f32,
        origin: Option<[f32; 3]>,
        verts: &[([f32; 3], Option<[f32; 3]>)],
    ) -> Object {
        let tris = verts
            .chunks_exact(3)
            .map(|t| Tri {
                p: [t[0].0, t[1].0, t[2].0],
                n: match (t[0].1, t[1].1, t[2].1) {
                    (Some(a), Some(b), Some(c)) => Some([a, b, c]),
                    _ => None,
                },
            })
            .collect();
        Object {
            name: name.into(),
            part: part.into(),
            per_unit,
            per_metre: per_unit,
            origin,
            tris,
        }
    }
}

/// What is wrong, by "object: problem": how many faces, and one of them.
pub(crate) type Report = BTreeMap<String, (usize, String)>;

fn note(report: &mut Report, o: &Object, problem: &str, at: [f32; 3]) {
    let e = report
        .entry(format!("{}: {problem}", o.name))
        .or_insert_with(|| (0, format!("{} at {at:?}", o.part)));
    e.0 += 1;
}

/// Check an object's faces against the rule.
pub(crate) fn check(o: &Object, report: &mut Report) {
    use std::collections::HashMap;
    let origin = o
        .origin
        .or_else(|| o.tris.first().map(|t| t.p[0]))
        .unwrap_or([0.0; 3]);
    // positions in 1/64 of a cube (a grid corner is a multiple of 64)
    let q = |p: [f32; 3]| -> [i64; 3] {
        [0, 1, 2].map(|a| ((p[a] - origin[a]) as f64 * o.per_unit as f64 * 64.0).round() as i64)
    };
    // half rectangles waiting for their other half: (the long side's ends,
    // the corner the other half turns on) -> how many
    let mut halves: HashMap<([i64; 3], [i64; 3], [i64; 3]), (i32, [f32; 3])> = HashMap::new();
    for t in &o.tris {
        let k = t.p.map(q);
        if k[0] == k[1] || k[1] == k[2] || k[0] == k[2] {
            continue;
        }
        if k.iter().flatten().any(|v| v % 64 != 0) {
            note(report, o, "a corner is off the grid", t.p[0]);
        }
        // the corner with the right angle between two edges along axes
        let along = |a: [i64; 3], b: [i64; 3]| (0..3).filter(|i| a[*i] != b[*i]).count() == 1;
        let Some(r) = (0..3).find(|r| along(k[*r], k[(r + 1) % 3]) && along(k[*r], k[(r + 2) % 3]))
        else {
            note(report, o, "a face is not an axis-aligned rectangle", t.p[0]);
            continue;
        };
        let (a, b) = (k[(r + 1) % 3], k[(r + 2) % 3]);
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let other = [0, 1, 2].map(|i| a[i] + b[i] - k[r][i]);
        // this half is owed by a twin turning on `other`
        if let Some(e) = halves.get_mut(&(lo, hi, k[r]))
            && e.0 > 0
        {
            e.0 -= 1;
        } else {
            halves.entry((lo, hi, other)).or_insert((0, t.p[0])).0 += 1;
        }
        if let Some(n) = t.n {
            let e1 = [0, 1, 2].map(|i| t.p[1][i] - t.p[0][i]);
            let e2 = [0, 1, 2].map(|i| t.p[2][i] - t.p[0][i]);
            let c = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt().max(1e-12);
            for n in n {
                let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                // (a face seen from both sides may store either way)
                let d = (c[0] * n[0] + c[1] * n[1] + c[2] * n[2]) / (len * nl.max(1e-12));
                if nl < 0.5 || d.abs() < 0.9995 {
                    note(report, o, "a stored normal is not the face's own", t.p[0]);
                    break;
                }
            }
        }
    }
    for (n, at) in halves.values() {
        for _ in 0..*n {
            note(report, o, "a triangle is not half of a rectangle", *at);
        }
    }
}

/// Match what was found against an allowlist of (prefix of the key, owner,
/// date): what is left over, and the entries that matched nothing.
pub(crate) fn sift<'a>(
    found: impl Iterator<Item = (&'a String, String)>,
    allowed: &[(&'static str, &'static str, &'static str)],
) -> (Vec<String>, Vec<String>) {
    let mut used = vec![false; allowed.len()];
    let mut left = Vec::new();
    for (key, detail) in found {
        match allowed.iter().position(|a| key.starts_with(a.0)) {
            Some(i) => used[i] = true,
            None => left.push(format!("{key} ({detail})")),
        }
    }
    let stale = allowed
        .iter()
        .zip(used)
        .filter(|(_, u)| !u)
        .map(|(a, _)| format!("{} (owner: {}, since {})", a.0, a.1, a.2))
        .collect();
    (left, stale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fine::Lod;
    use crate::render::{KIND_FOLIAGE, KIND_VOXEL, KIND_WATER, Vertex};
    use crate::{ladder, terrain_mesh};
    use dyno_world::World;
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    // ------------------------------------------------------- allowlists

    /// Meshes another lane has still to bring to the rule: (the start of the
    /// report's key, the owner, the day written). Only ever remove entries.
    const MESH_ALLOWED: &[(&str, &str, &str)] = &[];

    /// Pack data another lane has still to bring to the rule.
    const PACK_ALLOWED: &[(&str, &str, &str)] = &[];

    /// Shader code another lane has still to bring to the rule.
    const SHADER_ALLOWED: &[(&str, &str, &str)] = &[];

    // --------------------------------------------------------- the pack

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn pack_file(path: &str) -> Value {
        let p = root().join("content").join(path);
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    fn load_pack() {
        crate::assets::load_dir(&root().join("content")).expect("the content pack");
    }

    fn fail(what: &str, left: Vec<String>, stale: Vec<String>) {
        let mut msg = String::new();
        if !left.is_empty() {
            msg += &format!(
                "{what}: {} against the voxel rule (docs/art/voxel-rule.md):\n  {}\n",
                left.len(),
                left.join("\n  ")
            );
        }
        if !stale.is_empty() {
            msg += &format!(
                "{what}: allowlist entries that no longer match anything (remove them: the list only shrinks):\n  {}\n",
                stale.join("\n  ")
            );
        }
        assert!(msg.is_empty(), "{msg}");
    }

    // -------------------------------------------------------- mesh test

    fn world_tris(name: &str, part: &str, per_metre: f32, verts: &[Vertex], normals: bool) -> Object {
        let list: Vec<([f32; 3], Option<[f32; 3]>)> = verts
            .iter()
            .map(|v| (v.pos, normals.then_some(v.normal)))
            .collect();
        Object::from_list(name, part, per_metre, Some([0.0; 3]), &list)
    }

    /// Sections of the built world round the bench's six cameras (the
    /// plaza's roofs and props, woods, a farm, a lake, a mine), as the
    /// meshers make them: the ground at each level of detail, with and
    /// without skirts, leaves in clumps and as whole blocks.
    fn world_objects(report: &mut Report) {
        #[derive(serde::Deserialize)]
        struct Scenes {
            cameras: Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct Cameras {
            cameras: Vec<Camera>,
        }
        #[derive(serde::Deserialize)]
        struct Camera {
            name: String,
            stand: [f32; 3],
        }
        let read = |p: &str| std::fs::read_to_string(root().join(p)).unwrap();
        let scenes: Scenes = serde_json::from_str(&read("tools/bench/scenes.json")).unwrap();
        let cameras: Cameras = serde_json::from_str(&read("tools/concepts/cameras.json")).unwrap();
        let pack = dyno_server::pack::Pack::builtin();
        let layout = pack.layout.as_ref().expect("the pack has a layout");
        let (world, built) =
            dyno_world::worldgen::build(layout, &pack.stamps, layout.size).unwrap();
        let p = crate::shapes::pack();
        let levels: Vec<u8> = std::iter::once(p.terrain.per_metre)
            .chain(p.terrain.lod.iter().map(|l| l.1))
            .collect();
        let leaf_cells = crate::foliage::current()
            .map(|f| f.cells_per_metre())
            .unwrap_or(1.0);
        let dims = world.section_dims();
        let n = dyno_world::SECTION;
        for name in &scenes.cameras {
            let cam = cameras
                .cameras
                .iter()
                .find(|c| &c.name == name)
                .unwrap_or_else(|| panic!("no camera called {name}"));
            let at = [
                (cam.stand[0] + built.hearth[0] as f32) as i32,
                (cam.stand[2] + built.hearth[2] as f32) as i32,
            ];
            let (cx, cz) = (at[0].div_euclid(n), at[1].div_euclid(n));
            // the sections meshed, and a ring round them for their borders
            let mut held = World::sparse(world.size);
            for sz in cz - 3..=cz + 3 {
                for sx in cx - 3..=cx + 3 {
                    if sx < 0 || sz < 0 || sx >= dims[0] || sz >= dims[2] {
                        continue;
                    }
                    for sy in 0..dims[1] {
                        let s = [sx, sy, sz];
                        let data = World::prepare_section(&world.encode_section(s)).unwrap();
                        held.put_prepared(s, data).unwrap();
                    }
                }
            }
            let light = crate::light::Light::new(&held);
            let far = Some([-1.0e6, 0.0, -1.0e6]);
            for sz in cz - 2..=cz + 2 {
                for sx in cx - 2..=cx + 2 {
                    if sx < 0 || sz < 0 || sx >= dims[0] || sz >= dims[2] {
                        continue;
                    }
                    let inner = (sx - cx).abs() <= 1 && (sz - cz).abs() <= 1;
                    for sy in 0..dims[1] {
                        let s = [sx, sy, sz];
                        for (i, per_metre) in levels.iter().enumerate() {
                            // the coarser levels on the middle sections only
                            if i > 0 && !inner {
                                continue;
                            }
                            let lod = Lod {
                                per_metre: *per_metre,
                                // a coarser neighbour on two sides
                                skirt: if i == 0 && inner {
                                    [0, levels[levels.len() - 1], 0, levels[1.min(levels.len() - 1)]]
                                } else {
                                    [0; 4]
                                },
                                shade: i == 0,
                            };
                            let focus = if i == 0 { None } else { far };
                            let geo = terrain_mesh::mesh_at(&held, &light, s, focus, None, lod);
                            let part = format!("{name}, section {s:?}");
                            let quads = geo.ground.chunks_exact(4).map(|q| {
                                (
                                    [q[0].pos, q[1].pos, q[2].pos, q[3].pos],
                                    Some([q[0].normal, q[1].normal, q[2].normal, q[3].normal]),
                                )
                            });
                            check(
                                &Object::from_quads(
                                    format!("world: the ground at {per_metre} a metre"),
                                    part.clone(),
                                    *per_metre as f32,
                                    Some([0.0; 3]),
                                    quads,
                                ),
                                report,
                            );
                            let (mut leaves, mut blocks, mut other) = (Vec::new(), Vec::new(), Vec::new());
                            for t in geo.opaque.chunks_exact(3) {
                                let k = t[0].tex[2];
                                if (KIND_FOLIAGE..KIND_WATER).contains(&k) {
                                    leaves.extend_from_slice(t);
                                } else if (0.0..KIND_FOLIAGE).contains(&k) {
                                    blocks.extend_from_slice(t);
                                } else {
                                    assert!(k < KIND_VOXEL, "ground among the built blocks");
                                    other.extend_from_slice(t);
                                }
                            }
                            let (mut water, mut clear) = (Vec::new(), Vec::new());
                            for t in geo.trans.chunks_exact(3) {
                                if t[0].tex[2] >= KIND_WATER {
                                    water.extend_from_slice(t);
                                } else {
                                    clear.extend_from_slice(t);
                                }
                            }
                            let fine = crate::shapes::FINE;
                            for (what, per, verts, normals) in [
                                ("world: leaves", leaf_cells, &leaves, true),
                                ("world: built blocks and shapes", fine, &blocks, true),
                                ("world: untextured boxes", fine, &other, true),
                                // (water's vertices carry the shore, the depth
                                // and the white water where others carry a normal)
                                ("world: water", fine, &water, false),
                                ("world: glass and other translucent blocks", fine, &clear, true),
                            ] {
                                check(&world_tris(what, &part, per, verts, normals), report);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Every face of everything drawn is an axis-aligned rectangle on its
    /// object's grid, with its own normal.
    #[test]
    fn everything_drawn_is_cubes_on_the_ladder() {
        load_pack();
        let mut report = Report::new();
        let mut objects = Vec::new();
        objects.extend(crate::shapes::voxel_objects());
        objects.extend(crate::cover::voxel_objects());
        objects.extend(crate::critters::voxel_objects());
        objects.extend(crate::particles::voxel_objects());
        objects.extend(crate::farview::voxel_objects());
        objects.extend(crate::figure::voxel_objects());
        objects.extend(crate::render::voxel_objects());
        assert!(objects.len() > 100, "{} objects", objects.len());
        for o in &objects {
            if ladder::step_of_per_metre(o.per_metre).is_none() {
                report
                    .entry(format!("{}: its cubes are off the ladder", o.name))
                    .or_insert_with(|| (0, format!("{}: {} m", o.part, 1.0 / o.per_metre)))
                    .0 += 1;
            }
            check(o, &mut report);
        }
        world_objects(&mut report);
        let (left, stale) = sift(
            report
                .iter()
                .map(|(k, (n, eg))| (k, format!("{n} faces, such as {eg}"))),
            MESH_ALLOWED,
        );
        fail("meshes", left, stale);
    }

    // -------------------------------------------------------- pack test

    fn num(v: &Value) -> f32 {
        v.as_f64().unwrap_or(f64::NAN) as f32
    }

    fn pack_violations() -> BTreeMap<String, String> {
        let mut bad: BTreeMap<String, String> = BTreeMap::new();
        let mut add = |key: String, why: String| {
            bad.insert(key, why);
        };

        // blocks.json: the ground, tags, shape rules, shape files
        let blocks = pack_file("blocks.json");
        let t = &blocks["terrain"];
        for (what, v) in [
            ("per_metre", &t["per_metre"]),
            ("colour_per_metre", &t["colour_per_metre"]),
        ] {
            if ladder::step_of_per_metre(num(v)).is_none() {
                add(format!("blocks.json: terrain.{what}"), format!("{v} a metre is off the ladder"));
            }
        }
        for l in t["lod"].as_array().into_iter().flatten() {
            if ladder::step_of_per_metre(num(&l["per_metre"])).is_none() {
                add(
                    "blocks.json: terrain.lod".into(),
                    format!("{} a metre is off the ladder", l["per_metre"]),
                );
            }
        }
        for key in ["smooth", "shade_gentle"] {
            if !t[key].is_null() {
                add(
                    format!("blocks.json: terrain.{key}"),
                    "the ground is lit by its faces, not as the slope its steps make".into(),
                );
            }
        }
        for (name, b) in t["blocks"].as_object().into_iter().flatten() {
            if !b["smooth"].is_null() {
                add(
                    format!("blocks.json: terrain.blocks.{name}.smooth"),
                    "the ground is lit by its faces, not as the slope its steps make".into(),
                );
            }
        }
        for b in blocks["blocks"].as_array().into_iter().flatten() {
            let tags = b["tags"].as_array().into_iter().flatten();
            if tags.into_iter().any(|t| t == "chamfer") {
                add(
                    format!("blocks.json: block {}: tag chamfer", b["key"].as_str().unwrap_or("?")),
                    "cubes have no chamfers".into(),
                );
            }
        }
        for (set, rules) in blocks["shape_rules"].as_object().into_iter().flatten() {
            for class in rules["classes"].as_array().into_iter().flatten() {
                let cname = class["class"].as_str().unwrap_or("?");
                let at = format!("blocks.json: shape_rules.{set}.{cname}");
                if class["jitter"].as_f64().unwrap_or(0.0) != 0.0 {
                    add(format!("{at}: jitter"), "a shape turns by quarter turns only".into());
                }
                if !class["spin"].is_null() {
                    add(format!("{at}: spin"), "a shape turns by quarter turns only".into());
                }
                for part in class["parts"].as_array().into_iter().flatten() {
                    let pname = part["name"].as_str().unwrap_or("?");
                    if !part["round"].is_null() {
                        add(format!("{at}.{pname}: round"), "no solids of revolution".into());
                    }
                    let lift = &part["lift"];
                    if !lift.is_null() && num(&lift["step"]).fract() != 0.0 {
                        add(
                            format!("{at}.{pname}: lift"),
                            format!("a lift of {} voxels leaves the grid", lift["step"]),
                        );
                    }
                }
            }
        }
        let mut shape_files: Vec<String> = Vec::new();
        for b in blocks["blocks"].as_array().into_iter().flatten() {
            if let Some(f) = b["shape"]["vox"].as_str()
                && !shape_files.iter().any(|s| s == f)
            {
                shape_files.push(f.to_string());
            }
        }
        for f in &shape_files {
            let file = &blocks["shape_files"][f];
            let step = file["step"].as_u64();
            if !step.is_some_and(|s| (1..=6).contains(&s)) {
                add(format!("blocks.json: shape_files.{f}: step"), "names no ladder step (1 to 6)".into());
            }
            for (k, m) in file["palette"].as_object().into_iter().flatten() {
                if !m["slope"].is_null() {
                    add(
                        format!("blocks.json: shape_files.{f}.palette.{k}: slope"),
                        "there are no slopes to keep steps from".into(),
                    );
                }
            }
        }

        // cover.json and foliage.json
        let cover = pack_file("cover.json");
        if ladder::step_of(num(&cover["voxel"])).is_none() {
            add("cover.json: voxel".into(), format!("{} m is off the ladder", cover["voxel"]));
        }
        let (mut sways, mut softs) = (0, 0);
        for (name, k) in cover["kinds"].as_object().into_iter().flatten() {
            if !k["voxel"].is_null() && ladder::step_of(num(&k["voxel"])).is_none() {
                add(format!("cover.json: kinds.{name}.voxel"), format!("{} m is off the ladder", k["voxel"]));
            }
            if k["sway"].as_f64().unwrap_or(0.0) != 0.0 {
                sways += 1;
            }
            if k["soft"].as_f64().unwrap_or(0.0) != 0.0 {
                softs += 1;
            }
        }
        if softs > 0 {
            add(
                "cover.json: soft".into(),
                format!("{softs} kinds lean their faces' normals to the sky: a side lit as a slope"),
            );
        }
        if sways > 0 {
            add(
                "cover.json: sway".into(),
                format!("{sways} kinds bend in the wind, vertex by vertex: their cubes shear"),
            );
        }
        let foliage = pack_file("foliage.json");
        let cells = num(&foliage["cells"]);
        if ladder::step_of_per_metre(cells).is_none() {
            add(
                "foliage.json: cells".into(),
                format!("{cells} cubes a side of a leaf block is off the ladder (0: smooth canopies)"),
            );
        }
        let grain = cells * num(&foliage["grain"]);
        if grain != 0.0 && ladder::step_of_per_metre(grain).is_none() {
            add("foliage.json: grain".into(), format!("squares of {grain} a metre are off the ladder"));
        }

        // life.json: critters (particles are snapped to the ladder as drawn)
        let life = pack_file("life.json");
        for (name, k) in life["critters"].as_object().into_iter().flatten() {
            let unit = k["unit"].as_f64().map(|v| v as f32).unwrap_or(0.06);
            if ladder::step_of(unit).is_none() {
                add(format!("life.json: critters.{name}.unit"), format!("voxels of {unit} m are off the ladder"));
            }
        }

        // figures.json and the rigs
        let figures = pack_file("figures.json");
        for (name, f) in figures["figures"].as_object().into_iter().flatten() {
            let per_block = f["per_block"].as_f64().unwrap_or(32.0);
            let detail = f["detail"].as_f64().unwrap_or(1.0);
            for g in f["grids"].as_array().into_iter().flatten() {
                let per = (per_block * g["detail"].as_f64().unwrap_or(detail)) as f32;
                if !name.starts_with("held.") && ladder::step_of_per_metre(per).is_none() {
                    add(
                        format!("figures.json: figures.{name}: grid {}", g["name"].as_str().unwrap_or("?")),
                        format!("{per} voxels a block is off the ladder"),
                    );
                }
            }
            if !f["smooth"].is_null() {
                add(format!("figures.json: figures.{name}.smooth"), "no bevels or smooth meshes".into());
            }
            for l in f["lods"].as_array().into_iter().flatten() {
                if l["smooth"].as_bool() == Some(true) {
                    add(format!("figures.json: figures.{name}.lods: smooth"), "no smooth meshes".into());
                }
            }
        }
        for (name, k) in figures["held"]["kinds"].as_object().into_iter().flatten() {
            let unit = num(&k["unit"]);
            if ladder::step_of(unit).is_none() {
                add(
                    format!("figures.json: held.kinds.{name}.unit"),
                    format!("voxels of {unit} blocks are off the ladder"),
                );
            }
        }
        let rigs = root().join("content/rigs");
        let mut files: Vec<_> = std::fs::read_dir(&rigs)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        files.sort();
        for file in files {
            let rig = pack_file(&format!("rigs/{file}"));
            for (name, r) in rig["rigs"].as_object().into_iter().flatten() {
                let at = format!("rigs/{file}: {name}");
                for p in r["parts"].as_array().into_iter().flatten() {
                    let part = p["part"].as_str().or(p["name"].as_str()).unwrap_or("?");
                    let shapes: Vec<&Value> = match &p["shape"] {
                        Value::Null => Vec::new(),
                        s if s["values"].is_array() => s["values"].as_array().unwrap().iter().collect(),
                        s => vec![s],
                    };
                    for s in shapes {
                        if !s["taper"].is_null() {
                            add(format!("{at}: part {part}: taper"), "no tapers".into());
                        }
                        let sc = &s["scale"];
                        if !sc.is_null() && sc.as_array().is_none_or(|a| a.iter().any(|v| v != 1.0)) {
                            add(
                                format!("{at}: part {part}: scale"),
                                format!("a part scaled by {sc} leaves the ladder"),
                            );
                        }
                    }
                    if p["smooth"].as_bool() == Some(true) {
                        add(format!("{at}: part {part}: smooth"), "no smooth meshes".into());
                    }
                }
                for b in r["bones"].as_array().into_iter().flatten() {
                    for op in b["ops"].as_array().into_iter().flatten() {
                        if !op["scale"].is_null() {
                            add(
                                format!("{at}: bone {}: scale", b["name"].as_str().unwrap_or("?")),
                                format!("a part scaled as it moves ({}) leaves the ladder", op["scale"]),
                            );
                        }
                    }
                }
            }
        }

        // the rules: creatures drawn larger or smaller than they are modelled
        let rules = pack_file("rules/hearthblock.json");
        fn scales(v: &Value, out: &mut Vec<(String, f64)>) {
            match v {
                Value::Object(m) => {
                    if let (Some(s), Some(k)) = (m.get("scale").and_then(|s| s.as_f64()), m.get("key")) {
                        out.push((k.as_str().unwrap_or("?").to_string(), s));
                    }
                    for c in m.values() {
                        scales(c, out);
                    }
                }
                Value::Array(a) => a.iter().for_each(|c| scales(c, out)),
                _ => {}
            }
        }
        let mut list = Vec::new();
        scales(&rules, &mut list);
        for (key, s) in list {
            if s != 1.0 {
                add(
                    format!("rules/hearthblock.json: {key}: scale"),
                    format!("drawn at {s} times its model: its cubes leave the ladder"),
                );
            }
        }
        bad
    }

    /// The pack's data keeps the rule: no round parts, slopes, chamfers,
    /// turned props, tapers, bevels, scales or smooth meshes, and every
    /// model on a ladder step.
    #[test]
    fn the_pack_keeps_the_voxel_rule() {
        let bad = pack_violations();
        let (left, stale) = sift(bad.iter().map(|(k, v)| (k, v.clone())), PACK_ALLOWED);
        fail("the pack", left, stale);
    }

    // ------------------------------------------------------ shader test

    /// Places a shader (or the code that feeds one) breaks cubes: the file,
    /// a line of it that is only there while it does, and what it does.
    const SHADER_MARKS: &[(&str, &str, &str)] = &[
        (
            "render.rs",
            "pos.y = pos.y + swell(pos.xz, t);",
            "the water's surface swells smoothly: a flat cube face no more",
        ),
        (
            "water.rs",
            "pos.y = pos.y + swell(pos.xz, u.cam.w);",
            "the water's surface swells smoothly: a flat cube face no more",
        ),
        (
            "render.rs",
            "pos.x = pos.x + sin(t * 1.2 + ph)",
            "leaves stir vertex by vertex: their cubes shear",
        ),
        (
            "cover.rs",
            "let s = v.sway * (sin(",
            "ground cover sways vertex by vertex: its cubes shear",
        ),
        (
            "cover.rs",
            "* fade;",
            "ground cover shrinks into the ground at the end of its reach: cubes off the ladder",
        ),
        (
            "render.rs",
            "bent = normalize(abs(det) * n - g * k);",
            "painted relief bends the normal of built blocks inside a face",
        ),
        (
            "sky.rs",
            "if (sd > env.sun.w) { c = env.sun.rgb; }",
            "the sun is a round disc",
        ),
        (
            "sky.rs",
            "if (md > env.moon.w) {",
            "the moon is a round disc",
        ),
        (
            "render.rs",
            "const SEGMENTS: usize = 40;",
            "the ring round a target is a circle of 40 segments",
        ),
        (
            "terrain_mesh.rs",
            "None => Nets::new(&g).run(light, &mut geo),",
            "leaves fall back to a smooth surface when the pack has no foliage.json",
        ),
        (
            "render.rs",
            "mag_filter: wgpu::FilterMode::Linear,\n        min_filter: wgpu::FilterMode::Linear,\n        mipmap_filter: wgpu::FilterMode::Linear,\n        anisotropy_clamp: 4,",
            "block textures are smoothed up close: a face's picture is not a grid of cubes",
        ),
    ];

    /// No shader moves the corners of cubes apart or bends a face's normal.
    #[test]
    fn the_shaders_keep_cubes_whole() {
        let mut bad: BTreeMap<String, String> = BTreeMap::new();
        for (file, mark, what) in SHADER_MARKS {
            let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
            let text = std::fs::read_to_string(&p).unwrap();
            if text.contains(mark) {
                bad.insert(format!("{file}: {what}"), mark.lines().next().unwrap_or("").to_string());
            }
        }
        let (left, stale) = sift(bad.iter().map(|(k, v)| (k, v.clone())), SHADER_ALLOWED);
        fail("shaders", left, stale);
    }

    // ------------------------------------------------------ the checker

    fn cube(lo: [f32; 3], size: f32) -> Vec<([[f32; 3]; 4], Option<[[f32; 3]; 4]>)> {
        let mut out = Vec::new();
        for axis in 0..3 {
            for sign in [-1.0f32, 1.0] {
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                let mut n = [0.0; 3];
                n[axis] = sign;
                let corner = |a: f32, b: f32| {
                    let mut p = lo;
                    p[axis] += if sign > 0.0 { size } else { 0.0 };
                    p[u] += a * size;
                    p[v] += b * size;
                    p
                };
                out.push((
                    [corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0)],
                    Some([n; 4]),
                ));
            }
        }
        out
    }

    #[test]
    fn the_checker_passes_cubes_and_catches_the_rest() {
        let run = |o: Object| {
            let mut r = Report::new();
            check(&o, &mut r);
            r.keys().cloned().collect::<Vec<_>>().join("; ")
        };
        // a cube on its grid
        assert_eq!(run(Object::from_quads("a", "", 16.0, Some([0.0; 3]), cube([1.0, 2.0, 3.0], 0.0625))), "");
        // off the grid
        let off = run(Object::from_quads("a", "", 16.0, Some([0.0; 3]), cube([1.01, 2.0, 3.0], 0.0625)));
        assert!(off.contains("off the grid"), "{off}");
        // a cube of a size off the ladder
        let size = run(Object::from_quads("a", "", 16.0, None, cube([0.0; 3], 0.07)));
        assert!(size.contains("off the grid"), "{size}");
        // a slope: one quad over a stair
        let slope = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]];
        let s = run(Object::from_quads("a", "", 1.0, Some([0.0; 3]), [(slope, None)]));
        assert!(s.contains("not an axis-aligned rectangle"), "{s}");
        // a flat triangle with a slanted side (a gable end drawn as one polygon)
        let gable = Object::from_list(
            "a",
            "",
            1.0,
            Some([0.0; 3]),
            &[
                ([0.0, 0.0, 0.0], None),
                ([4.0, 0.0, 0.0], None),
                ([0.0, 4.0, 0.0], None),
            ],
        );
        let g = run(gable);
        assert!(g.contains("not half of a rectangle"), "{g}");
        // a bent normal on a level face
        let mut top = cube([0.0; 3], 1.0);
        for (_, n) in &mut top {
            *n = Some([[0.3, 0.9, 0.3]; 4]);
        }
        let b = run(Object::from_quads("a", "", 1.0, Some([0.0; 3]), top));
        assert!(b.contains("stored normal"), "{b}");
        // a cube turned a few degrees
        let (sn, cs) = 0.1f32.sin_cos();
        let turned = cube([0.0; 3], 1.0)
            .into_iter()
            .map(|(p, _)| (p.map(|p| [p[0] * cs + p[2] * sn, p[1], -p[0] * sn + p[2] * cs]), None));
        let t = run(Object::from_quads("a", "", 1.0, Some([0.0; 3]), turned));
        assert!(t.contains("not an axis-aligned rectangle"), "{t}");
    }

    /// Figure cost: vertices at each level of detail, summed over the looks
    /// the mesh test meshes (`cargo test -p dyno-client --lib figure_cost
    /// -- --ignored --nocapture`), for the figures' before and after.
    #[test]
    #[ignore]
    fn figure_cost() {
        load_pack();
        let mut by: BTreeMap<(String, String), usize> = BTreeMap::new();
        for o in crate::figure::voxel_objects() {
            let level = o.part.split(',').next().unwrap_or("").to_string();
            let who = o.name.trim_end_matches(" (smooth)").to_string();
            let who = if who.starts_with("figure held.") {
                "figure held.*".to_string()
            } else {
                who
            };
            *by.entry((who, level)).or_default() += o.tris.len() * 3;
        }
        for ((who, level), n) in by {
            println!("{who}\t{level}\t{n}");
        }
    }

    #[test]
    fn allowlists_only_shrink() {
        let found = [("a: x".to_string(), String::new()), ("b: y".to_string(), String::new())];
        let (left, stale) = sift(
            found.iter().map(|(k, v)| (k, v.clone())),
            &[("a:", "someone", "2026-11-24"), ("c:", "someone", "2026-11-24")],
        );
        assert_eq!(left, vec!["b: y ()".to_string()]);
        assert_eq!(stale.len(), 1);
        assert!(stale[0].starts_with("c:"));
    }
}
