//! The person: bodies, heads, appearance features, clothes and armour, as
//! layers of `models/person.vox`, its entry in `figures.json` and the
//! `person` rig in `rigs/biped.json`.

use super::body::{Body, Bp};
use super::sdf::*;
use super::{Cell, Grid, Mode, Palette, edit_json, when, write};
use glam::{Vec3, vec3};
use serde_json::{Value, json};
use std::path::Path;

/// The main grid's parts, as the rig names them.
pub const PARTS: [&str; 15] = [
    "torso", "flap.f", "flap.b", "skirt", "arm_u.r", "arm_u.l", "arm_l.r", "arm_l.l", "thigh.r",
    "thigh.l", "shin.r", "shin.l", "cape.0", "cape.1", "cape.2",
];

/// Palette entries, made once.
pub struct Inks {
    pub skin: [u8; 3],
    pub skin_shadow: u8,
    pub blush: u8,
    pub lips: u8,
    pub mouth: u8,
    pub eye_white: u8,
    pub iris: u8,
    pub pupil: u8,
    pub glint: u8,
    pub lash: u8,
    pub brow: u8,
    pub hair: [u8; 4],
    pub tunic: [u8; 3],
    pub trim: [u8; 2],
    pub pants: [u8; 2],
    pub boots: [u8; 3],
    pub sole: u8,
    pub belt: u8,
    pub buckle: u8,
    pub cape: [u8; 3],
    pub apron: [u8; 2],
    pub sash: [u8; 2],
    pub scarf: [u8; 2],
    pub hood: [u8; 2],
    pub wax: u8,
    pub flame: [u8; 2],
    pub iron: u8,
    pub line: u8,
}

impl Inks {
    pub fn new(p: &mut Palette) -> Inks {
        let s = |p: &mut Palette, slot: &str, k: f32| p.slot(slot, k, [0.0; 3], false);
        let sc = |p: &mut Palette, slot: &str, k: f32| p.slot(slot, k, [0.0; 3], true);
        Inks {
            skin: [s(p, "skin", 0.93), s(p, "skin", 1.0), s(p, "skin", 1.06)],
            skin_shadow: s(p, "skin", 0.8),
            blush: p.slot("skin", 0.94, [0.05, -0.015, -0.015], false),
            lips: p.slot("skin", 0.88, [0.06, -0.01, -0.01], false),
            mouth: p.slot("skin", 0.42, [0.06, 0.0, 0.0], true),
            eye_white: p.fixed([0.93, 0.92, 0.88], false, true),
            iris: sc(p, "eyes", 1.0),
            pupil: p.fixed([0.05, 0.04, 0.05], false, true),
            glint: p.fixed([1.0, 1.0, 1.0], false, true),
            lash: p.fixed([0.1, 0.07, 0.06], false, true),
            brow: sc(p, "hair", 0.8),
            hair: [
                s(p, "hair", 0.72),
                s(p, "hair", 0.9),
                s(p, "hair", 1.05),
                s(p, "hair", 1.22),
            ],
            tunic: [s(p, "tunic", 0.74), s(p, "tunic", 1.0), s(p, "tunic", 1.14)],
            trim: [sc(p, "trim", 0.72), sc(p, "trim", 1.0)],
            pants: [s(p, "pants", 0.8), s(p, "pants", 1.0)],
            boots: [s(p, "boots", 0.65), s(p, "boots", 1.0), s(p, "boots", 1.3)],
            sole: p.fixed([0.09, 0.07, 0.06], false, false),
            belt: s(p, "boots", 0.78),
            buckle: p.fixed([0.86, 0.72, 0.36], false, true),
            cape: [s(p, "cape", 0.74), s(p, "cape", 1.0), s(p, "cape", 1.18)],
            apron: [s(p, "apron", 0.82), s(p, "apron", 1.0)],
            sash: [s(p, "sash", 0.78), s(p, "sash", 1.0)],
            scarf: [s(p, "scarf", 0.78), s(p, "scarf", 1.0)],
            hood: [
                p.fixed([0.3, 0.23, 0.17], false, false),
                p.fixed([0.37, 0.28, 0.21], false, false),
            ],
            wax: p.fixed([0.94, 0.9, 0.78], false, false),
            flame: [
                p.fixed([1.0, 0.62, 0.12], true, true),
                p.fixed([1.0, 0.95, 0.62], true, true),
            ],
            iron: p.fixed([0.32, 0.31, 0.33], false, false),
            line: p.slot("skin", 0.72, [0.0; 3], true),
        }
    }
}

/// Pick a shade from a few by a smooth noise, for a painted look.
/// Mostly the middle shade, with a few broad patches of the others: clean
/// colours, as the voxel style wants (and big faces merge into few quads).
pub fn mottle(shades: &[u8], p: Vec3, freq: f32, seed: f32) -> u8 {
    let n = noise(p * freq * 0.3 + Vec3::splat(seed));
    let mid = shades[shades.len() / 2];
    if n < 0.14 {
        shades[0]
    } else if n > 0.86 {
        shades[shades.len() - 1]
    } else {
        mid
    }
}

/// The look features of the two bodies.
pub fn bodies() -> [Body; 2] {
    [Body::male(), Body::female()]
}

pub fn body_when(bi: usize, more: &[(&str, &[u8])]) -> super::When {
    let b = [bi as u8];
    let mut c: Vec<(&str, &[u8])> = vec![("body", &b)];
    c.extend_from_slice(more);
    when(&c)
}

/// The whole figure: grids and palette.
pub fn sculpt() -> (Vec<Grid>, Palette) {
    let mut pal = Palette::default();
    let ink = Inks::new(&mut pal);
    let mut main = Grid::new(
        "main",
        2.0,
        vec3(-18.0, 0.0, -14.0),
        vec3(18.0, 57.0, 14.0),
        &PARTS,
    );
    let mut head = Grid::new(
        "head",
        2.5,
        vec3(-7.5, 43.5, -8.5),
        vec3(7.5, 66.0, 8.5),
        &["head"],
    );
    let bodies = bodies();
    // the bare body and head
    for (bi, b) in bodies.iter().enumerate() {
        main.push("body", bare_body(&main, b, bi, &ink));
        head.push("head", bare_head(&head, b, bi, &ink));
    }
    super::clothes::all(&mut main, &mut head, &bodies, &ink, &mut pal);
    (vec![main, head], pal)
}

fn bare_body(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> super::Layer {
    let b = b.clone();
    g.layer(
        &format!("base,body={bi}"),
        body_when(bi, &[]),
        vec3(-16.0, 0.0, -8.0),
        vec3(16.0, 56.0, 9.0),
        move |p| -> Cell {
            if b.body(p) >= 0.0 {
                return None;
            }
            let part = g_part(&b, p);
            let mut c = mottle(&ink.skin, p, 0.45, 3.0);
            // knuckles and elbows a little flushed
            if matches!(b.part_of(p), Bp::ArmL) {
                let q = b.hand_local(mirror(p));
                if q.y > 3.0 && q.y < 3.7 && q.x > 0.1 {
                    c = ink.blush;
                }
            }
            Some((part, Mode::Set, c))
        },
    )
}

/// The main grid's part index for a point of the bare body.
pub fn g_part(b: &Body, p: Vec3) -> u8 {
    let name = b.part_name(p);
    PARTS.iter().position(|x| *x == name).unwrap() as u8 + 1
}

pub fn part_index(name: &str) -> u8 {
    PARTS.iter().position(|x| *x == name).unwrap() as u8 + 1
}

fn bare_head(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> super::Layer {
    let b = b.clone();
    g.layer(
        &format!("base,body={bi}"),
        body_when(bi, &[]),
        vec3(-6.0, b.head.y - 3.0, -6.5),
        vec3(6.0, b.head.y + 11.0, 7.0),
        move |p| -> Cell {
            let skull = b.skull(p);
            let eyes = b.eyes(p);
            if skull >= 0.0 && eyes >= 0.0 {
                return None;
            }
            let m = mirror(p);
            let y = |v: f32| b.hy(v);
            if eyes < 0.0 && skull >= -0.05 {
                // the eye: white, iris, pupil, a glint
                let q = m - b.eye_centre(true);
                let r = (q.x * q.x + q.y * q.y).sqrt();
                let c = if q.z > 0.2 && r < 0.22 {
                    ink.pupil
                } else if q.z > 0.15 && (q.x + 0.22).abs() < 0.14 && (q.y - 0.24).abs() < 0.14 {
                    ink.glint
                } else if q.z > 0.1 && r < 0.52 {
                    ink.iris
                } else {
                    ink.eye_white
                };
                return Some((1, Mode::Set, c));
            }
            let mut c = mottle(&ink.skin, p, 0.9, 7.0);
            let e = b.eye_centre(true);
            // the upper lid line
            let dq = m - e;
            if dq.z > -0.2 && dq.y > 0.35 && dq.y < 0.75 && dq.x.abs() < 0.85 && skull > -0.35 {
                c = ink.lash;
            }
            // under the brow, a little shadow
            else if (m.y - y(57.25)).abs() < 0.3 && m.x < 2.5 && p.z > 3.0 && skull > -0.3 {
                c = ink.skin_shadow;
            }
            // lips
            else if m.x < 1.05 && p.y < y(54.05) && p.y > y(53.55) && p.z > 3.5 && skull > -0.3 {
                c = ink.lips;
            }
            // cheeks and nose tip
            else if (sphere(m, vec3(2.3, y(55.6), 3.0), 1.0) < 0.0
                || sphere(p, vec3(0.0, y(55.3), 5.3), 0.45) < 0.0)
                && skull > -0.4
            {
                c = ink.blush;
            }
            Some((1, Mode::Set, c))
        },
    )
}

// ----------------------------------------------------------------- rig

/// The person rig: bones where the sculpt puts the joints, for each body
/// (and build, which widens the shoulders and hips).
pub fn rig(bodies: &[Body; 2]) -> Value {
    let builds = [0.92f32, 1.0, 1.1];
    // a vector by body, and by body and build when `wide` (x scaled)
    let by_body = |f: &dyn Fn(&Body) -> Vec3| -> Value {
        json!({"by": "body", "values": bodies.iter().map(|b| r3(f(b))).collect::<Vec<_>>()})
    };
    let by_build = |f: &dyn Fn(&Body) -> Vec3| -> Value {
        let mut map = serde_json::Map::new();
        for (bi, b) in bodies.iter().enumerate() {
            for (k, s) in builds.iter().enumerate() {
                let v = f(b);
                map.insert(format!("{bi},{k}"), r3(vec3(v.x * s, v.y, v.z)));
            }
        }
        json!({"by": ["body", "build"], "map": map})
    };
    let pel = |b: &Body| vec3(0.0, b.pelvis_y, 0.0);
    let hand = |b: &Body| {
        let (dir, _, f) = b.hand_frame();
        b.wrist + dir * 2.4 + f * 0.35
    };
    let bones = json!([
        {"name": "pelvis", "at": by_body(&|b| pel(b))},
        {"name": "thigh.r", "parent": "pelvis", "at": by_build(&|b| b.hip - pel(b))},
        {"name": "shin.r", "parent": "thigh.r", "at": by_body(&|b| b.knee - b.hip)},
        {"name": "thigh.l", "parent": "pelvis", "at": by_build(&|b| flip(b.hip) - pel(b))},
        {"name": "shin.l", "parent": "thigh.l", "at": by_body(&|b| flip(b.knee - b.hip))},
        {"name": "flap.f", "parent": "pelvis", "at": [0, 2, 0], "ops": [{"rx": "-shape"}]},
        {"name": "flap.b", "parent": "pelvis", "at": [0, 2, 0], "ops": [{"rx": "shape"}]},
        {"name": "spine", "parent": "pelvis", "at": [0, 0, 0]},
        {"name": "cape", "parent": "spine", "at": by_body(&|b| b.cape - pel(b))},
        {"name": "cape.0", "parent": "cape", "at": [0, 0, 0], "ops": [{"rx": "0.07 + flow*0.26 + sin(clock*4.5 - 0)*0.025*(1 + flow)"}]},
        {"name": "cape.1", "parent": "cape.0", "at": [0, -13, 0], "ops": [{"rx": "0.05 + flow*0.16 + sin(clock*4.5 - 0.9)*0.045*(1 + flow)"}]},
        {"name": "cape.2", "parent": "cape.1", "at": [0, -13, 0], "ops": [{"rx": "0.06 + flow*0.12 + sin(clock*4.5 - 1.8)*0.065*(1 + flow)"}]},
        {"name": "upper_arm.r", "parent": "spine", "at": by_build(&|b| b.shoulder - pel(b))},
        {"name": "forearm.r", "parent": "upper_arm.r", "at": by_body(&|b| b.elbow - b.shoulder)},
        {"name": "upper_arm.l", "parent": "spine", "at": by_build(&|b| flip(b.shoulder) - pel(b))},
        {"name": "forearm.l", "parent": "upper_arm.l", "at": by_body(&|b| flip(b.elbow - b.shoulder))},
        {"name": "grip", "parent": "forearm.r", "at": by_body(&|b| hand(b) - b.elbow), "ops": [{"rx": 1.25}, {"rz": -0.22}, "pose"]},
        {"name": "head", "parent": "spine", "at": by_body(&|b| b.head - pel(b)), "ops": ["pose", {"rx": "-pitch*0.55"}]},
        // sockets: where equipment and carried things attach
        {"name": "socket.offhand", "parent": "forearm.l", "at": by_body(&|b| flip(hand(b) - b.elbow)), "ops": [{"rx": 1.25}, {"rz": 0.22}]},
        {"name": "socket.shield", "parent": "forearm.l", "at": by_body(&|b| flip((b.wrist - b.elbow) * 0.55 + vec3(1.6, 0.0, 0.0))), "ops": [{"ry": -std::f32::consts::FRAC_PI_2}]},
        {"name": "socket.back", "parent": "spine", "at": by_body(&|b| vec3(0.0, 41.0, -6.2) - pel(b) + vec3(0.0, b.head.y - 51.8, 0.0)), "ops": [{"rz": 0.6}, {"rx": std::f32::consts::PI}]},
        {"name": "socket.helm", "parent": "head", "at": by_body(&|b| vec3(0.0, b.hy(61.0), -0.6) - b.head)},
        {"name": "socket.shoulder.r", "parent": "upper_arm.r", "at": [0.4, 1.2, 0]},
        {"name": "socket.shoulder.l", "parent": "upper_arm.l", "at": [-0.4, 1.2, 0]},
        {"name": "socket.cloak", "parent": "spine", "at": by_body(&|b| b.cape - pel(b))},
        {"name": "socket.buckle", "parent": "pelvis", "at": [0, 3.0, 5.2]}
    ]);
    // build widens the trunk, the arms and the legs
    let shape = |s: [[f32; 3]; 3]| json!({"by": "build", "values": s.iter().map(|v| json!({"scale": v})).collect::<Vec<_>>()});
    // one girth for every part, so seams between them stay closed
    let trunk = shape([[0.93, 1.0, 0.93], [1.0, 1.0, 1.0], [1.1, 1.0, 1.08]]);
    let limb = trunk.clone();
    let leg = trunk.clone();
    let flat = shape([[0.94, 1.0, 1.0], [1.0, 1.0, 1.0], [1.08, 1.0, 1.05]]);
    let pivot = |f: &dyn Fn(&Body) -> Vec3| by_body(f);
    let parts = json!([
        {"bone": "thigh.r", "part": "thigh.r", "pivot": pivot(&|b| b.hip), "shape": leg},
        {"bone": "shin.r", "part": "shin.r", "pivot": pivot(&|b| b.knee), "shape": leg},
        {"bone": "thigh.l", "part": "thigh.l", "pivot": pivot(&|b| flip(b.hip)), "shape": leg},
        {"bone": "shin.l", "part": "shin.l", "pivot": pivot(&|b| flip(b.knee)), "shape": leg},
        {"bone": "pelvis", "part": "skirt", "pivot": pivot(&|b| pel(b)), "shape": trunk},
        {"bone": "flap.f", "part": "flap.f", "pivot": pivot(&|b| pel(b) + vec3(0.0, 2.0, 0.0)), "shape": flat},
        {"bone": "flap.b", "part": "flap.b", "pivot": pivot(&|b| pel(b) + vec3(0.0, 2.0, 0.0)), "shape": flat},
        {"bone": "spine", "part": "torso", "pivot": pivot(&|b| pel(b)), "shape": trunk},
        {"bone": "cape.0", "part": "cape.0", "pivot": pivot(&|b| b.cape), "shape": flat},
        {"bone": "cape.1", "part": "cape.1", "pivot": pivot(&|b| b.cape - vec3(0.0, 13.0, 0.0)), "shape": flat},
        {"bone": "cape.2", "part": "cape.2", "pivot": pivot(&|b| b.cape - vec3(0.0, 26.0, 0.0)), "shape": flat},
        {"bone": "upper_arm.r", "part": "arm_u.r", "pivot": pivot(&|b| b.shoulder), "shape": limb},
        {"bone": "forearm.r", "part": "arm_l.r", "pivot": pivot(&|b| b.elbow), "shape": limb},
        {"bone": "grip", "held": true},
        {"bone": "upper_arm.l", "part": "arm_u.l", "pivot": pivot(&|b| flip(b.shoulder)), "shape": limb},
        {"bone": "forearm.l", "part": "arm_l.l", "pivot": pivot(&|b| flip(b.elbow)), "shape": limb},
        {"bone": "socket.offhand", "held": "offhand"},
        {"bone": "socket.shield", "held": "shield"},
        {"bone": "socket.back", "held": "back"},
        {"bone": "head", "grid": "head", "part": "head", "pivot": pivot(&|b| b.head)}
    ]);
    json!({"bones": bones, "parts": parts})
}

fn flip(v: Vec3) -> Vec3 {
    vec3(-v.x, v.y, v.z)
}

fn r3(v: Vec3) -> Value {
    let r = |x: f32| (x * 100.0).round() / 100.0;
    json!([r(v.x), r(v.y), r(v.z)])
}

/// Sculpt the person and write it into the pack at `content`.
pub fn bake(content: &Path) -> Result<String, String> {
    let (grids, pal) = sculpt();
    let vox = content.join("models/person.vox");
    let gj = write(&grids, &pal, &vox)?;
    let size = std::fs::metadata(&vox).map(|m| m.len()).unwrap_or(0);
    let layers: usize = grids
        .iter()
        .map(|g| g.groups.iter().map(|x| x.layers.len()).sum::<usize>())
        .sum();
    edit_json(&content.join("figures.json"), |v| {
        let f = &mut v["figures"]["person"];
        f["file"] = json!("models/person.vox");
        f["skeleton"] = json!("biped");
        f["rig"] = json!("person");
        f["clips"] = json!({"features": ["robe"]});
        f["per_block"] = json!(34.19);
        f["detail"] = json!(2.0);
        // crisp voxels (the art direction: every surface reads as voxels),
        // fine up close and coarser with distance; the shaped mesher's
        // gentle bevel (`"smooth": true` with `"smooth": {"bevel": 0.2,
        // "relax": 0, "blend": false}`) is there for figures that can
        // afford it
        if let Some(o) = f.as_object_mut() {
            o.remove("smooth");
        }
        f["lods"] = json!([
            {"until": 3.5},
            {"half": 1, "until": 11.0},
            {"half": 2, "until": 40.0},
            {"half": 3}
        ]);
        f["grids"] = gj;
        f["palette"] = pal.json();
    })?;
    let bodies = bodies();
    edit_json(&content.join("rigs/biped.json"), |v| {
        v["rigs"]["person"] = rig(&bodies);
    })?;
    Ok(format!(
        "baked the person: {layers} layers, {} palette entries, {} ({} KB)",
        pal.list.len(),
        vox.display(),
        size / 1024
    ))
}
