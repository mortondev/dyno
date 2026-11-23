//! Clothes and head features: everyday garments, hair, beards, ears,
//! expressions and the townsfolk's extras (aprons, sashes, scarves...).
//! Each is a layer shown for its feature value, for each body.

use super::body::{Body, Bp};
use super::person::{Inks, body_when, g_part, mottle, part_index};
use super::sdf::*;
use super::{Cell, Grid, Layer, Mode, Palette};
use glam::{Vec3, vec3};

/// Within `off` outside the body and `depth` inside it.
pub fn shell(b: &Body, p: Vec3, off: f32, depth: f32) -> bool {
    let d = b.body(p);
    d < off && d > -depth
}

/// Where along the upper arm (0 shoulder, 1 elbow) or forearm (0 elbow,
/// 1 wrist) a point is.
pub fn along(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    let ab = b - a;
    (p - a).dot(ab) / ab.length_squared()
}

const ALL: (Vec3, Vec3) = (vec3(-18.0, 0.0, -14.0), vec3(18.0, 57.0, 14.0));

fn layer(g: &Grid, name: &str, bi: usize, more: &[(&str, &[u8])], f: impl Fn(Vec3) -> Cell) -> Layer {
    g.layer(&format!("{name},body={bi}"), body_when(bi, more), ALL.0, ALL.1, f)
}

pub fn all(main: &mut Grid, head: &mut Grid, bodies: &[Body; 2], ink: &Inks, pal: &mut Palette) {
    for (bi, b) in bodies.iter().enumerate() {
        main.push("body", trousers(main, b, bi, ink));
    }
    for (bi, b) in bodies.iter().enumerate() {
        main.push("body", boots(main, b, bi, ink));
    }
    for (bi, b) in bodies.iter().enumerate() {
        main.push("body", tunic(main, b, bi, ink));
        main.push("body", flaps(main, b, bi, ink, &[("chest", &[0])], [ink.tunic[0], ink.tunic[1]], ink.trim[1], 22.6));
    }
    for (bi, b) in bodies.iter().enumerate() {
        for l in robe(main, b, bi, ink) {
            main.push("body", l);
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        for l in apron(main, b, bi, ink) {
            main.push("body", l);
        }
        for e in 1..=4u8 {
            main.push("body", emblem(main, b, bi, ink, e));
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        main.push("body", belt(main, b, bi, ink));
        main.push("body", sash(main, b, bi, ink));
    }
    super::gear::all(main, head, bodies, ink, pal);
    for (bi, b) in bodies.iter().enumerate() {
        for l in cape(main, b, bi, ink) {
            main.push("body", l);
        }
    }
    // the head
    for (bi, b) in bodies.iter().enumerate() {
        for e in 1..=3u8 {
            head.push("head", ears(head, b, bi, ink, e));
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        for m in 0..=2u8 {
            for l in mood(head, b, bi, ink, m) {
                head.push("head", l);
            }
        }
        head.push("head", elder(head, b, bi, ink));
    }
    for (bi, b) in bodies.iter().enumerate() {
        head.push("head", beard(head, b, bi, ink));
    }
    for (bi, b) in bodies.iter().enumerate() {
        for s in 1..=4u8 {
            head.push("head", hair(head, b, bi, ink, s));
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        head.push("head", scarf(head, b, bi, ink));
        head.push("head", candle(head, b, bi, ink));
    }
    super::gear::helms(head, bodies, ink, pal);
}

// ------------------------------------------------------------- garments

fn trousers(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    layer(g, "legs/trousers", bi, &[("legs", &[0, 1])], move |p| {
        if p.y > 33.8 || p.y < 4.0 || !shell(&b, p, 0.32, 1.4) {
            return None;
        }
        let bp = b.part_of(p);
        if !matches!(bp, Bp::Thigh | Bp::Shin | Bp::Torso) {
            return None;
        }
        // folds run round the leg
        let n = noise(vec3(p.x * 0.5, p.y * 1.4, p.z * 0.5));
        let c = if n > 0.62 { ink.pants[0] } else { ink.pants[1] };
        Some((g_part(&b, p), Mode::Set, c))
    })
}

fn boots(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    layer(g, "feet/boots", bi, &[("feet", &[0, 1])], move |p| {
        if p.y < 0.0 || p.y > 12.6 {
            return None;
        }
        let cuff = p.y > 10.4;
        let off = if cuff { 0.95 + (p.y - 10.4) * 0.12 } else if p.y < 0.8 { 0.75 } else { 0.55 };
        if !shell(&b, p, off, 1.6) || !matches!(b.part_of(p), Bp::Shin) {
            return None;
        }
        let c = if p.y < 0.8 {
            ink.sole
        } else if cuff {
            if p.y > 12.1 { ink.boots[0] } else { ink.boots[2] }
        } else if (p.y - 4.6).abs() < 0.35 {
            ink.boots[0]
        } else {
            mottle(&ink.boots[..2], p, 0.7, 1.0)
        };
        Some((g_part(&b, p), Mode::Set, c))
    })
}

/// The V of an open collar: true where the neck shows.
fn v_neck(p: Vec3, top: f32, depth: f32) -> bool {
    p.z > 0.5 && p.y > top - depth && p.x.abs() < (p.y - (top - depth)) * 0.62 + 0.25
}

fn tunic(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    let top = b.head.y - 4.2;
    layer(g, "chest/tunic", bi, &[("chest", &[0])], move |p| {
        let m = mirror(p);
        let bp = b.part_of(p);
        let (ok, edge) = match bp {
            Bp::Torso => (
                p.y > 31.0 && p.y < top + 1.2 && shell(&b, p, 0.4, 1.2) && !v_neck(p, top, 2.6),
                v_neck(p + vec3(0.0, -0.5, 0.0), top, 2.6) || v_neck(p + vec3(p.x.signum() * 0.45, 0.0, 0.0), top, 2.6),
            ),
            Bp::ArmU => {
                let t = along(b.shoulder, b.elbow, m);
                (t < 0.66 && shell(&b, p, 0.28, 1.0), t > 0.58)
            }
            _ => (false, false),
        };
        if !ok {
            return None;
        }
        let c = if edge {
            ink.trim[1]
        } else {
            // soft folds, running down
            let n = noise(vec3(p.x * 0.7, p.y * 0.12, p.z * 0.7));
            if n < 0.2 { ink.tunic[0] } else if n > 0.82 { ink.tunic[2] } else { ink.tunic[1] }
        };
        Some((g_part(&b, p), Mode::Set, c))
    })
}

/// Front and back flaps below the belt (a tunic's, a jerkin's).
pub fn flaps(
    g: &Grid,
    b: &Body,
    bi: usize,
    _ink: &Inks,
    cond: &[(&str, &[u8])],
    shades: [u8; 2],
    hem: u8,
    bottom: f32,
) -> Layer {
    let b = b.clone();
    let name = format!("flaps/{}", cond.iter().map(|(k, v)| format!("{k}{v:?}")).collect::<String>());
    layer(g, &name, bi, cond, move |p| {
        let top = 32.8;
        if p.y > top || p.y < bottom {
            return None;
        }
        let drop = top - p.y;
        let wide = if b.female { 6.2 } else { 5.6 };
        let w = wide + drop * 0.07;
        if p.x.abs() > w {
            return None;
        }
        let fz = 4.95 + drop * 0.13 - 0.03 * p.x * p.x + if b.female { 0.2 } else { 0.0 };
        let bz = -(5.5 + drop * 0.1 - 0.03 * p.x * p.x + if b.female { 0.3 } else { 0.0 });
        let (part, z0) = if p.z > 0.0 { ("flap.f", fz) } else { ("flap.b", bz) };
        if (p.z - z0).abs() > 0.42 {
            return None;
        }
        let c = if p.y < bottom + 0.9 || p.x.abs() > w - 0.5 {
            hem
        } else if noise(vec3(p.x * 1.3, p.y * 0.2, 0.0)) > 0.6 {
            shades[0]
        } else {
            shades[1]
        };
        Some((part_index(part), Mode::Set, c))
    })
}

fn belt(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    layer(g, "waist/belt", bi, &[("waist", &[0])], move |p| {
        if p.y < 31.8 || p.y > 33.8 {
            return None;
        }
        let buckle = p.x.abs() < 0.95 && p.z > 2.0;
        let off = if buckle { 1.15 } else { 0.85 };
        if !shell(&b, p, off, 1.6) || !matches!(b.part_of(p), Bp::Torso | Bp::Thigh) {
            return None;
        }
        let c = if buckle {
            if p.x.abs() < 0.45 && (p.y - 32.8).abs() < 0.45 { ink.belt } else { ink.buckle }
        } else {
            ink.belt
        };
        Some((part_index("torso"), Mode::Set, c))
    })
}

/// A superellipse ring: the distance (roughly) from a skirt that is `rx`
/// wide and `rf`/`rb` deep at the front/back.
pub fn skirt_ring(p: Vec3, rx: f32, rf: f32, rb: f32, folds: f32) -> f32 {
    let rz = if p.z > 0.0 { rf } else { rb };
    let a = (p.x / rx).abs().powi(4) + (p.z / rz).abs().powi(4);
    let theta = p.x.atan2(p.z);
    (a.powf(0.25) - 1.0) * rz.min(rx) - (theta * 7.0).sin() * folds
}

fn robe(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Vec<Layer> {
    let mut out = Vec::new();
    let bb = b.clone();
    let top = b.head.y - 3.0;
    out.push(layer(g, "chest/robe", bi, &[("robe", &[1])], move |p| {
        let b = &bb;
        let m = mirror(p);
        let (ok, trim) = match b.part_of(p) {
            Bp::Torso => {
                let collar = p.y > top - 2.2;
                let off = if collar { 0.95 } else { 0.45 + (33.5 - p.y).max(0.0) * 0.4 };
                (
                    p.y > 30.5 && p.y < top && shell(b, p, off, 1.6),
                    collar && p.y > top - 0.7 || p.x.abs() < 0.55 && p.z > 0.0 && p.y < top - 2.2,
                )
            }
            Bp::ArmU => (shell(b, p, 0.42, 1.0), false),
            Bp::ArmL => {
                let t = along(b.elbow, b.wrist, m);
                let off = 0.42 + (t - 0.45).max(0.0) * 2.1;
                (t < 1.02 && shell(b, p, off, 2.5) && b.arm_l(m) < off, t > 0.88)
            }
            _ => (false, false),
        };
        if !ok {
            return None;
        }
        let c = if trim {
            ink.trim[1]
        } else {
            let n = noise(vec3(p.x * 0.8, p.y * 0.22, p.z * 0.8));
            if n < 0.32 { ink.tunic[0] } else if n > 0.75 { ink.tunic[2] } else { ink.tunic[1] }
        };
        Some((g_part(b, p), Mode::Set, c))
    }));
    let female = b.female;
    out.push(layer(g, "skirt/robe", bi, &[("robe", &[1])], move |p| {
        if p.y > 34.0 || p.y < 1.4 {
            return None;
        }
        let hip = smooth(34.0, 27.0, p.y);
        let low = smooth(27.0, 1.4, p.y);
        let w = if female { 0.4 } else { 0.0 };
        let rx = 6.6 + w + 2.6 * hip + 1.9 * low;
        let rf = 5.0 + 1.0 * hip + 1.7 * low;
        let rb = 5.4 + 1.1 * hip + 1.3 * low;
        let d = skirt_ring(p, rx, rf, rb, 0.22 * low);
        if d.abs() > 0.4 {
            return None;
        }
        let c = if p.y < 3.0 || p.x.abs() < 0.6 && p.z > 0.0 {
            ink.trim[1]
        } else if p.y < 3.5 {
            ink.trim[0]
        } else {
            let theta = p.x.atan2(p.z);
            if (theta * 7.0).sin() > 0.55 { ink.tunic[2] } else if (theta * 7.0).sin() < -0.55 { ink.tunic[0] } else { ink.tunic[1] }
        };
        Some((part_index("skirt"), Mode::Set, c))
    }));
    out
}

fn apron(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Vec<Layer> {
    let bb = b.clone();
    let mut out = vec![layer(g, "chest/apron", bi, &[("apron", &[1])], move |p| {
        let b = &bb;
        if b.part_of(p) != Bp::Torso || p.y < 31.0 || p.y > 47.6 {
            return None;
        }
        let bib = p.z > 0.6 && p.x.abs() < 4.6 - (p.y - 41.0).max(0.0) * 0.2;
        let strap = p.y > 44.5 && (p.x.abs() - 3.0).abs() < 0.55;
        if !(bib || strap) || !shell(b, p, 0.75, 1.0) {
            return None;
        }
        Some((part_index("torso"), Mode::Set, mottle(&ink.apron, p, 0.6, 5.0)))
    })];
    let female = b.female;
    out.push(layer(g, "flap/apron", bi, &[("apron", &[1])], move |p| {
        let top = 32.8;
        if p.y > top || p.y < 16.0 || p.x.abs() > 5.3 + (top - p.y) * 0.05 {
            return None;
        }
        let fz = 5.5 + (top - p.y) * 0.13 - 0.03 * p.x * p.x + if female { 0.2 } else { 0.0 };
        if (p.z - fz).abs() > 0.4 {
            return None;
        }
        let c = if p.y < 16.8 { ink.apron[0] } else { mottle(&ink.apron, p, 0.6, 5.0) };
        Some((part_index("flap.f"), Mode::Set, c))
    }));
    out
}

fn sash(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    layer(g, "waist/sash", bi, &[("sash", &[1])], move |p| {
        if b.part_of(p) != Bp::Torso {
            return None;
        }
        let n = glam::Vec2::new(17.0, -12.0).normalize();
        let diag = (glam::Vec2::new(p.x - 6.0, p.y - 48.0).dot(n)).abs() < 1.0 && p.y > 30.5;
        let wrap = p.y > 31.2 && p.y < 34.0;
        if !(diag && shell(&b, p, 0.7, 1.0) || wrap && shell(&b, p, 0.95, 1.5)) {
            return None;
        }
        let c = if wrap && (p.y - 32.6).abs() > 1.0 { ink.sash[0] } else { mottle(&ink.sash, p, 0.9, 2.0) };
        Some((part_index("torso"), Mode::Set, c))
    })
}

/// A device on the chest, recoloured into whatever is worn there.
pub fn emblem_shape(e: u8, x: f32, y: f32) -> bool {
    let x = x.abs();
    match e {
        1 => x < 0.45 && (39.3..45.8).contains(&y) || x < 1.7 && (43.4..44.2).contains(&y),
        2 => {
            let r = (x * x + (y - 42.6).powi(2)).sqrt();
            (1.05..1.8).contains(&r) || r < 0.5
        }
        3 => x < 2.1 && ((y - 41.6) - (1.9 - x * 0.95)).abs() < 0.45,
        4 => x * 0.8 + (y - 42.6).abs() * 0.58 < 1.45,
        _ => false,
    }
}

/// The emblem on a tunic's chest.
fn emblem(g: &Grid, b: &Body, bi: usize, ink: &Inks, e: u8) -> Layer {
    let b = b.clone();
    let lift = if b.female { -0.6 } else { 0.0 };
    layer(g, &format!("chest/emblem={e}"), bi, &[("emblem", &[e]), ("chest", &[0])], move |p| {
        (p.z > 2.5 && emblem_shape(e, p.x, p.y - lift) && b.part_of(p) == Bp::Torso)
            .then_some((part_index("torso"), Mode::Dye, ink.trim[1]))
    })
}

fn cape(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Vec<Layer> {
    let bb = b.clone();
    let c0 = b.cape;
    let mut out = vec![layer(g, "back/cape", bi, &[("cape", &[1])], move |p| {
        let b = &bb;
        let top = c0.y + 0.4;
        if p.y > top || p.y < c0.y - 38.5 {
            return None;
        }
        let drop = top - p.y;
        let w = (if b.female { 7.4 } else { 8.6 }) + drop * 0.065;
        if p.x.abs() > w {
            return None;
        }
        let fold = 0.38 * (p.x * 1.05 + drop * 0.05).sin() * smooth(0.0, 8.0, drop);
        // round the shoulders at the top, hang straight below
        let zc = c0.z - 0.4 - drop * 0.055 - fold + (p.x / w).powi(2) * 1.8 * smooth(6.0, 0.0, drop);
        if (p.z - zc).abs() > 0.42 {
            return None;
        }
        let seg = if drop < 13.0 { "cape.0" } else if drop < 26.0 { "cape.1" } else { "cape.2" };
        let c = if drop > 37.6 || p.x.abs() > w - 0.45 {
            ink.trim[0]
        } else if fold > 0.2 {
            ink.cape[2]
        } else if fold < -0.2 {
            ink.cape[0]
        } else {
            ink.cape[1]
        };
        Some((part_index(seg), Mode::Set, c))
    })];
    let bb = b.clone();
    out.push(layer(g, "back/cape-collar", bi, &[("cape", &[1])], move |p| {
        let b = &bb;
        if b.part_of(p) != Bp::Torso || p.y < c0.y - 2.6 || p.y > c0.y + 1.0 {
            return None;
        }
        let clasp = (mirror(p) - vec3(3.7, c0.y - 1.4, 3.3)).length() < 0.8;
        if !(clasp && shell(b, p, 1.3, 1.0) || p.z < 1.8 && shell(b, p, 1.0, 1.0)) {
            return None;
        }
        Some((part_index("torso"), Mode::Set, if clasp { ink.buckle } else { ink.cape[0] }))
    }));
    out
}

// ------------------------------------------------------------------ head

fn hlayer(g: &Grid, name: &str, b: &Body, bi: usize, more: &[(&str, &[u8])], f: impl Fn(Vec3) -> Cell) -> Layer {
    g.layer(
        &format!("{name},body={bi}"),
        body_when(bi, more),
        vec3(-7.5, b.head.y - 8.5, -8.5),
        vec3(7.5, b.head.y + 14.0, 8.5),
        f,
    )
}

fn ears(g: &Grid, b: &Body, bi: usize, ink: &Inks, e: u8) -> Layer {
    let b = b.clone();
    let s = if b.female { 0.92 } else { 1.0 };
    hlayer(g, &format!("ears={e}"), &b.clone(), bi, &[("ears", &[e])], move |p| {
        let m = mirror(p);
        let c = vec3(3.85 * s, b.hy(56.3), 0.1);
        let d = match e {
            1 => ellipsoid(m, c, vec3(0.55, 1.2, 0.85) * s),
            2 => smin(
                ellipsoid(m, c, vec3(0.5, 1.0, 0.75) * s),
                cone(m, c + vec3(0.1, 0.1, -0.1), c + vec3(1.25, 2.6, -1.5) * s, 0.62 * s, 0.08),
                0.5,
            ),
            _ => cone(m, c, c + vec3(2.3, 1.1, -0.7), 1.05 * s, 0.32),
        };
        if d >= 0.0 {
            return None;
        }
        // the inner ear in shadow
        let inner = m.x > c.x + 0.15 && (m - c).length() < 0.7 * s;
        Some((1, Mode::Add, if inner { ink.skin_shadow } else { ink.skin[1] }))
    })
}

fn mood(g: &Grid, b: &Body, bi: usize, ink: &Inks, mood: u8) -> Vec<Layer> {
    let bb = b.clone();
    let f = b.female;
    // brows: raised arcs (cheerful), drawn down to the nose (stern), level (calm)
    let brow_y = move |x: f32| -> f32 {
        let t = ((x - 0.45) / 2.2).clamp(0.0, 1.0);
        let base = if f { 58.05 } else { 57.85 };
        match mood {
            0 => base + 0.3 * (t * std::f32::consts::PI).sin(),
            1 => base - 0.42 + t * 0.55,
            _ => base + 0.1 * (t * std::f32::consts::PI).sin(),
        }
    };
    let mut out = vec![hlayer(g, &format!("brows={mood}"), b, bi, &[("mood", &[mood])], move |p| {
        let b = &bb;
        let m = mirror(p);
        if m.x < 0.45 || m.x > 2.7 || p.z < 2.4 {
            return None;
        }
        let d = b.skull(p);
        let thick = if f { 0.2 } else { 0.3 };
        if d > 0.32 || d < -0.4 || (p.y - b.hy(brow_y(m.x))).abs() > thick {
            return None;
        }
        Some((1, Mode::Set, ink.brow))
    })];
    let bb = b.clone();
    out.push(hlayer(g, &format!("mouth={mood}"), b, bi, &[("mood", &[mood])], move |p| {
        let b = &bb;
        let m = mirror(p);
        let (w, curve) = match mood {
            0 => (1.2, 0.26),
            1 => (0.95, -0.1),
            _ => (1.0, 0.08),
        };
        if m.x > w || p.z < 3.3 {
            return None;
        }
        let y0 = b.hy(54.05) + curve * (m.x / w).powi(2);
        let d = b.skull(p);
        if d > 0.05 || d < -0.4 || (p.y - y0).abs() > 0.1 {
            return None;
        }
        Some((1, Mode::Dye, ink.mouth))
    }));
    let bb = b.clone();
    out.push(hlayer(g, &format!("lids={mood}"), b, bi, &[("mood", &[mood])], move |p| {
        let b = &bb;
        let m = mirror(p);
        let q = m - b.eye_centre(true);
        if q.length() > b.eye_r() + 0.05 || q.z < 0.0 {
            return None;
        }
        // calm eyes are half closed; cheerful ones crinkle from below
        let lid = match mood {
            0 => q.y < -0.42,
            1 => q.y > 0.42,
            _ => q.y > 0.2,
        };
        lid.then_some((1, Mode::Set, if mood == 1 || mood == 2 { ink.skin_shadow } else { ink.skin[1] }))
    }));
    out
}

fn elder(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    hlayer(g, "elder", &b.clone(), bi, &[("elder", &[1])], move |p| {
        let m = mirror(p);
        let d = b.skull(p);
        if d > 0.05 || d < -0.35 {
            return None;
        }
        let y = |v: f32| b.hy(v);
        let forehead = p.z > 2.8 && m.x < 1.9 && [58.75, 59.35].iter().any(|v| (p.y - y(*v)).abs() < 0.08);
        let crow = m.x > 2.2 && m.x < 3.0 && p.z > 1.8 && ((p.y - y(56.85)) - (m.x - 2.2) * 0.4).abs() < 0.08;
        let fold = p.z > 3.0 && (m.x - (1.35 + (y(55.4) - p.y) * 0.25)).abs() < 0.09 && p.y < y(55.4) && p.y > y(53.9);
        (forehead || crow || fold).then_some((1, Mode::Dye, ink.line))
    })
}

fn beard(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    hlayer(g, "beard", &b.clone(), bi, &[("beard", &[1])], move |p| {
        let m = mirror(p);
        let y = |v: f32| b.hy(v);
        let goatee = ellipsoid(p, vec3(0.0, y(52.3), 2.7), vec3(1.7, 1.25, 1.2));
        let n = noise(vec3(m.x * 2.0, p.y * 0.6, p.z * 2.0));
        let off = 0.45 + n * 0.3;
        let d = b.skull(p).min(goatee);
        if d > off || d < -0.5 {
            return None;
        }
        let jaw = p.y < y(55.3) && p.z > -0.9 && p.y > y(50.6);
        let mouth = m.x < 1.3 && p.y > y(53.55) && p.y < y(54.45) && p.z > 3.0;
        let tache = m.x < 1.75 && p.y > y(54.45) && p.y < y(55.15) && p.z > 3.7;
        let cheek = p.y > y(55.0) && p.z > 2.2 && m.x < 2.2;
        if !(jaw || tache) || mouth || cheek && !tache {
            return None;
        }
        let s = noise(vec3(m.x * 3.0, p.y * 0.5, p.z * 3.0));
        let c = if s < 0.3 { ink.hair[0] } else if s > 0.7 { ink.hair[2] } else { ink.hair[1] };
        Some((1, Mode::Set, c))
    })
}

/// Where the hair starts, by the angle round the head (0 front).
fn hairline(b: &Body, theta: f32) -> f32 {
    let c = theta.cos();
    b.hy(58.75) - 1.1 * (1.0 - c) - 1.5 * (-c).max(0.0)
}

fn hair(g: &Grid, b: &Body, bi: usize, ink: &Inks, style: u8) -> Layer {
    let b = b.clone();
    let name = ["", "hair/short", "hair/long", "hair/hood", "hair/bun"][style as usize];
    hlayer(g, name, &b.clone(), bi, &[("hair_style", &[style])], move |p| {
        let m = mirror(p);
        let y = |v: f32| b.hy(v);
        let theta = p.x.atan2(p.z + 0.6);
        let cran = b.cranium(p);
        let skull = b.skull(p);
        if style == 3 {
            // a cloth hood with an opening for the face
            let n = noise(p * 0.8);
            let off = 1.0 + n * 0.25;
            let drape = cone(p, vec3(0.0, b.head.y - 0.5, -0.6), vec3(0.0, b.head.y - 3.2, -0.6), 3.5, 4.4);
            let d = cran.min(skull).min(drape + 0.9);
            let face = p.z > 1.4 && m.x < 2.95 && p.y < y(58.9) && p.y > y(51.8);
            if d > off || d < -0.4 || face || p.y < b.head.y - 3.4 {
                return None;
            }
            let rim = p.z > 1.0 && m.x < 3.6 && p.y < y(59.6);
            return Some((1, Mode::Set, if rim { ink.hood[0] } else { ink.hood[1] }));
        }
        let strand = noise(vec3(theta * 5.0, p.y * 0.35, 0.0));
        let tight = if style == 4 { 0.5 } else { 0.72 };
        // volume over the crown, swept up at the front
        let lift = if style == 4 { 0.2 } else { 0.75 } * smooth(y(57.5), y(60.5), p.y)
            + if style == 1 { 0.5 * smooth(1.0, 3.5, p.z) * smooth(y(58.5), y(60.0), p.y) } else { 0.0 };
        let off = tight + strand * 0.35 + lift;
        let line = hairline(&b, theta);
        let mut shape = cran;
        let mut extra = false;
        if style == 2 {
            let fall = ellipsoid(p, vec3(0.0, y(53.6), -3.3), vec3(3.9, 5.8, 1.8));
            let locks = ellipsoid(m, vec3(3.45, y(54.2), -0.7), vec3(1.05, 4.6, 1.35));
            let long = smin(fall, locks, 1.0);
            if long - off * 0.6 < 0.0 && p.y > y(46.8) && !(p.z > 1.6 && m.x < 3.2) {
                extra = true;
            }
            shape = shape.min(long);
        }
        if style == 4 {
            let bun = sphere(p, vec3(0.0, y(59.5), -4.4), 1.75);
            if bun < 0.0 {
                let tie = (p.z + 3.0).abs() < 0.3;
                return Some((1, Mode::Set, if tie { ink.trim[0] } else { mottle(&ink.hair, p, 1.6, 4.0) }));
            }
        }
        // a swept fringe
        let fringe = if style == 1 {
            ellipsoid(p, vec3(1.3, y(59.9), 3.0), vec3(2.1, 0.8, 1.1))
        } else {
            ellipsoid(p, vec3(0.9, y(59.35), 3.1), vec3(2.3, 0.85, 1.0))
        };
        let in_fringe = style != 4 && fringe < 0.0;
        let covered = (cran < off && cran > -0.6 && p.y > line) || extra || in_fringe;
        if !covered || shape > off + 0.2 && !in_fringe {
            return None;
        }
        // keep the face and ears clear
        if skull < -0.6 && !extra {
            return None;
        }
        let s = noise(vec3(theta * 9.0, p.y * 0.45, 0.0));
        let c = if s < 0.28 { ink.hair[0] } else if s < 0.55 { ink.hair[1] } else if s < 0.82 { ink.hair[2] } else { ink.hair[3] };
        Some((1, Mode::Set, c))
    })
}

fn scarf(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    hlayer(g, "scarf", &b.clone(), bi, &[("scarf", &[1])], move |p| {
        let y = |v: f32| b.hy(v);
        if p.y > y(55.9) || p.y < b.head.y - 2.6 {
            return None;
        }
        let d = b.skull(p);
        let off = 0.45 + noise(p * 1.2) * 0.2;
        if d > off || d < -0.4 {
            return None;
        }
        Some((1, Mode::Set, mottle(&ink.scarf, p, 1.0, 9.0)))
    })
}

fn candle(g: &Grid, b: &Body, bi: usize, ink: &Inks) -> Layer {
    let b = b.clone();
    hlayer(g, "candle", &b.clone(), bi, &[("candle", &[1])], move |p| {
        let top = b.cranium(vec3(0.0, 70.0, -0.6));
        let _ = top;
        let base = b.hy(61.2);
        let r = (p.x * p.x + (p.z + 0.4).powi(2)).sqrt();
        if p.y > base - 0.3 && p.y < base + 0.35 && r < 1.7 {
            return Some((1, Mode::Set, ink.iron));
        }
        if p.y > base && p.y < base + 2.6 && r < 0.75 {
            return Some((1, Mode::Set, ink.wax));
        }
        let f = ellipsoid(p, vec3(0.0, base + 3.25, -0.4), vec3(0.42, 0.75, 0.42));
        if f < 0.0 {
            return Some((1, Mode::Set, if p.y < base + 3.0 { ink.flame[0] } else { ink.flame[1] }));
        }
        None
    })
}
