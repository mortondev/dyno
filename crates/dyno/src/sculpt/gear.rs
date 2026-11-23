//! Armour by slot (chest, shoulders, head, hands, waist, legs, feet, back),
//! each slot's feature choosing cloth (1), leather (2), mail (3) or plate
//! (4). An item colours its slot through `<slot>.main`, `<slot>.trim` and
//! `<slot>.accent`, so one sculpt serves every tier of a set.

use super::body::{Body, Bp};
use super::clothes::{along, emblem_shape, flaps, shell, skirt_ring};
use super::person::{Inks, body_when, g_part, part_index};
use super::sdf::*;
use super::{Cell, Grid, Layer, Mode, Palette};
use glam::{Vec3, vec3};

/// The inks of one slot's colours.
#[derive(Clone, Copy)]
pub struct Si {
    pub main: [u8; 4],
    pub trim: [u8; 3],
    pub accent: [u8; 3],
    pub gem: u8,
}

impl Si {
    pub fn new(p: &mut Palette, slot: &str) -> Si {
        let m = format!("{slot}.main");
        let t = format!("{slot}.trim");
        let a = format!("{slot}.accent");
        Si {
            main: [
                p.slot(&m, 0.66, [0.0; 3], false),
                p.slot(&m, 0.86, [0.0; 3], false),
                p.slot(&m, 1.0, [0.0; 3], false),
                p.slot(&m, 1.22, [0.04; 3], false),
            ],
            trim: [
                p.slot(&t, 0.7, [0.0; 3], true),
                p.slot(&t, 1.0, [0.0; 3], true),
                p.slot(&t, 1.25, [0.06; 3], true),
            ],
            accent: [
                p.slot(&a, 0.72, [0.0; 3], false),
                p.slot(&a, 1.0, [0.0; 3], false),
                p.slot(&a, 1.18, [0.0; 3], false),
            ],
            gem: p.slot(&format!("{slot}.gem"), 1.0, [0.0; 3], true),
        }
    }

    /// A metal surface: a highlight on top, darker below, as painted.
    fn metal(&self, b: &Body, p: Vec3) -> u8 {
        let up = b.body(p + vec3(0.0, 0.6, 0.0)) - b.body(p);
        let side = b.body(p + vec3(0.0, 0.0, 0.6)) - b.body(p);
        if up > 0.38 || side > 0.45 && p.y > 38.0 {
            self.main[3]
        } else if up < -0.3 {
            self.main[1]
        } else {
            self.main[2]
        }
    }

    /// Mail: rows of rings, light and dark.
    fn mail(&self, p: Vec3) -> u8 {
        // rows of rings, light over dark
        if (p.y * 1.5).floor() as i32 % 2 == 0 {
            self.main[2]
        } else {
            self.main[1]
        }
    }

    /// Leather: soft mottling.
    fn leather(&self, p: Vec3) -> u8 {
        let n = noise(p * 0.25 + Vec3::splat(11.0));
        if n < 0.14 {
            self.main[1]
        } else if n > 0.86 {
            self.main[3]
        } else {
            self.main[2]
        }
    }

    fn cloth(&self, p: Vec3) -> u8 {
        let n = noise(vec3(p.x * 0.35, p.y * 0.06, p.z * 0.35));
        if n < 0.14 {
            self.main[1]
        } else if n > 0.86 {
            self.main[3]
        } else {
            self.main[2]
        }
    }
}

const ALL: (Vec3, Vec3) = (vec3(-18.0, 0.0, -14.0), vec3(18.0, 57.0, 14.0));

fn layer(
    g: &Grid,
    name: &str,
    bi: usize,
    more: &[(&str, &[u8])],
    f: impl Fn(Vec3) -> Cell,
) -> Layer {
    g.layer(
        &format!("{name},body={bi}"),
        body_when(bi, more),
        ALL.0,
        ALL.1,
        f,
    )
}

pub struct Slots {
    pub chest: Si,
    pub shoulders: Si,
    pub hands: Si,
    pub waist: Si,
    pub legs: Si,
    pub feet: Si,
    pub back: Si,
}

impl Slots {
    pub fn new(p: &mut Palette) -> Slots {
        Slots {
            chest: Si::new(p, "chest"),
            shoulders: Si::new(p, "shoulders"),
            hands: Si::new(p, "hands"),
            waist: Si::new(p, "waist"),
            legs: Si::new(p, "legs"),
            feet: Si::new(p, "feet"),
            back: Si::new(p, "back"),
        }
    }
}

pub fn all(main: &mut Grid, _head: &mut Grid, bodies: &[Body; 2], ink: &Inks, pal: &mut Palette) {
    let s = Slots::new(pal);
    // inside out: legs and feet, chest, waist, hands, shoulders, back
    for (bi, b) in bodies.iter().enumerate() {
        for k in 1..=4u8 {
            main.push("body", legs(main, b, bi, &s.legs, k));
        }
        for k in 1..=4u8 {
            main.push("body", feet(main, b, bi, &s.feet, ink, k));
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        for l in chest(main, b, bi, &s.chest, ink) {
            main.push("body", l);
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        for k in 1..=4u8 {
            main.push("body", waist(main, b, bi, &s.waist, ink, k));
        }
        for k in 1..=4u8 {
            main.push("body", hands(main, b, bi, &s.hands, k));
        }
    }
    for (bi, b) in bodies.iter().enumerate() {
        for k in 1..=4u8 {
            main.push("body", shoulders(main, b, bi, &s.shoulders, k));
        }
        main.push("body", quiver(main, b, bi, &s.back, ink));
    }
}

// ------------------------------------------------------------------- legs

fn legs(g: &Grid, b: &Body, bi: usize, s: &Si, k: u8) -> Layer {
    let b = b.clone();
    let s = *s;
    let name = ["", "legs/cloth", "legs/leather", "legs/mail", "legs/plate"][k as usize];
    layer(g, name, bi, &[("legs", &[k])], move |p| {
        if p.y > 33.8 || p.y < 4.0 {
            return None;
        }
        let bp = b.part_of(p);
        if !matches!(bp, Bp::Thigh | Bp::Shin | Bp::Torso) {
            return None;
        }
        let m = mirror(p);
        let knee = (m - b.knee - vec3(0.0, 0.0, 1.0)).length();
        let off = match k {
            1 => 0.34,
            2 => {
                if knee < 2.2 && p.z > 0.0 {
                    0.65
                } else {
                    0.38
                }
            }
            3 => 0.42,
            _ => {
                // cuisses and greaves at the front, a domed knee cop
                let front = p.z > 0.2 || (m.x - b.hip.x) > 2.0 && p.z > -1.0;
                if knee < 2.3 {
                    1.15 - knee * 0.2
                } else if front && p.y > 5.0 {
                    0.85
                } else {
                    0.42
                }
            }
        };
        if !shell(&b, p, off, 1.4) {
            return None;
        }
        let c = match k {
            1 => s.cloth(p),
            2 => {
                if knee < 2.2 && p.z > 0.0 {
                    if knee > 1.9 { s.trim[0] } else { s.main[1] }
                } else {
                    s.leather(p)
                }
            }
            3 => {
                if bp == Bp::Torso {
                    s.accent[1]
                } else {
                    s.mail(p)
                }
            }
            _ => {
                let front = p.z > 0.2 || (m.x - b.hip.x) > 2.0 && p.z > -1.0;
                if knee < 2.3 {
                    if knee > 1.95 {
                        s.trim[1]
                    } else {
                        s.metal(&b, p)
                    }
                } else if front && p.y > 5.0 {
                    // bands across the thigh plates
                    if (p.y - 22.0).rem_euclid(3.2) < 0.45 {
                        s.main[1]
                    } else {
                        s.metal(&b, p)
                    }
                } else {
                    s.accent[0]
                }
            }
        };
        Some((g_part(&b, p), Mode::Set, c))
    })
}

// ------------------------------------------------------------------- feet

fn feet(g: &Grid, b: &Body, bi: usize, s: &Si, ink: &Inks, k: u8) -> Layer {
    let b = b.clone();
    let s = *s;
    let sole = ink.sole;
    let name = [
        "",
        "feet/slippers",
        "feet/soft-boots",
        "feet/mail-boots",
        "feet/sabatons",
    ][k as usize];
    layer(g, name, bi, &[("feet", &[k])], move |p| {
        let top = match k {
            1 => 4.6,
            2 => 15.2,
            3 => 12.6,
            _ => 14.6,
        };
        if p.y < 0.0 || p.y > top {
            return None;
        }
        let m = mirror(p);
        let cuff = p.y > top - 2.4;
        let toe = (p.z - b.ankle.z - 3.0).max(0.0);
        let off = match k {
            1 => 0.4 + toe * 0.08,
            2 => {
                if cuff {
                    0.95 + (p.y - top + 2.4) * 0.18
                } else {
                    0.55
                }
            }
            3 => {
                if cuff {
                    1.15
                } else {
                    0.62
                }
            }
            _ => {
                if cuff {
                    1.0 + (p.y - top + 2.4) * 0.25
                } else if p.y < 4.2 {
                    0.95 + toe * 0.12
                } else {
                    0.85
                }
            }
        };
        let off = if p.y < 0.8 { off + 0.15 } else { off };
        if !shell(&b, p, off, 1.8) || b.part_of(p) != Bp::Shin {
            return None;
        }
        let c = if p.y < 0.8 {
            sole
        } else {
            match k {
                1 => {
                    if p.y > top - 0.6 {
                        s.trim[1]
                    } else {
                        s.cloth(p)
                    }
                }
                2 => {
                    let strap = [5.0, 8.6].iter().any(|y| (p.y - y).abs() < 0.35);
                    if cuff {
                        if p.y > top - 0.5 {
                            s.main[1]
                        } else {
                            s.main[3]
                        }
                    } else if strap {
                        if m.x > b.ankle.x + 1.0 && p.z.abs() < 0.8 {
                            s.trim[1]
                        } else {
                            s.accent[0]
                        }
                    } else {
                        s.leather(p)
                    }
                }
                3 => {
                    if cuff {
                        s.accent[if noise(p * 1.7) > 0.5 { 1 } else { 2 }]
                    } else if p.y < 4.6 {
                        s.mail(p)
                    } else {
                        s.leather(p)
                    }
                }
                _ => {
                    if cuff && p.y > top - 0.55 || (p.y - 4.4).abs() < 0.3 {
                        s.trim[1]
                    } else if p.y < 4.2
                        && (p.z - b.ankle.z).rem_euclid(1.3) < 0.3
                        && p.z > b.ankle.z + 0.5
                    {
                        s.main[1]
                    } else {
                        s.metal(&b, p)
                    }
                }
            }
        };
        Some((g_part(&b, p), Mode::Set, c))
    })
}

// ------------------------------------------------------------------ chest

fn chest(g: &Grid, b: &Body, bi: usize, s: &Si, ink: &Inks) -> Vec<Layer> {
    let mut out = Vec::new();
    let s = *s;
    let neck = b.head.y - 3.0;
    // 1: a mage's robe, high collar, long bell sleeves, a long skirt
    let bb = b.clone();
    out.push(layer(
        g,
        "chest/robe-cloth",
        bi,
        &[("chest", &[1])],
        move |p| {
            let b = &bb;
            let m = mirror(p);
            let (ok, trim) = match b.part_of(p) {
                Bp::Torso => {
                    // the collar stands up behind the neck
                    let collar = p.y > neck - 1.6 && p.z < 1.4;
                    let off = if collar {
                        1.0 + (p.y - neck + 1.6).max(0.0) * 0.55
                    } else {
                        0.48 + (33.5 - p.y).max(0.0) * 0.4
                    };
                    let open =
                        p.z > 0.6 && p.y > neck - 3.2 && m.x < (p.y - neck + 3.2) * 0.45 + 0.3;
                    (
                        p.y > 30.5 && p.y < neck + 2.4 && shell(b, p, off, 1.6) && !open,
                        collar && p.y > neck + 0.8
                            || p.z > 0.0 && (m.x < 0.55 && p.y < neck - 3.2)
                            || open_edge(p, m, neck),
                    )
                }
                Bp::ArmU => (shell(b, p, 0.42, 1.0), false),
                Bp::ArmL => {
                    let t = along(b.elbow, b.wrist, m);
                    let off = 0.42 + (t - 0.45).max(0.0) * 1.5;
                    (
                        t < 1.0 && shell(b, p, off, 3.0) && b.arm_l(m) < off,
                        t > 0.9,
                    )
                }
                _ => (false, false),
            };
            if !ok {
                return None;
            }
            let c = if trim {
                if noise(p * 2.2) > 0.6 {
                    s.trim[2]
                } else {
                    s.trim[1]
                }
            } else {
                s.cloth(p)
            };
            Some((g_part(b, p), Mode::Set, c))
        },
    ));
    let female = b.female;
    out.push(layer(
        g,
        "skirt/robe-cloth",
        bi,
        &[("chest", &[1])],
        move |p| {
            if p.y > 34.0 || p.y < 1.2 {
                return None;
            }
            let (rx, rf, rb, low) = robe_skirt(p.y, female);
            // a scalloped hem
            let theta = p.x.atan2(p.z);
            let hem = 1.2 + 0.7 * (theta * 9.0).sin().abs();
            if p.y < hem {
                return None;
            }
            let d = skirt_ring(p, rx, rf, rb, 0.25 * low);
            if d.abs() > 0.42 {
                return None;
            }
            let band = p.y < hem + 2.2;
            let c = if band && p.y > hem + 1.6 || p.x.abs() < 0.7 && p.z > 0.0 {
                s.trim[1]
            } else if band {
                if (theta * 18.0).sin() > 0.3 {
                    s.trim[2]
                } else {
                    s.accent[1]
                }
            } else {
                let f = (theta * 7.0).sin();
                if f > 0.55 {
                    s.main[3]
                } else if f < -0.55 {
                    s.main[1]
                } else {
                    s.main[2]
                }
            };
            Some((part_index("skirt"), Mode::Set, c))
        },
    ));
    // 2: a leather jerkin over dark sleeves, with crossed straps
    let bb = b.clone();
    out.push(layer(g, "chest/jerkin", bi, &[("chest", &[2])], move |p| {
        let b = &bb;
        let m = mirror(p);
        let bp = b.part_of(p);
        let c = match bp {
            Bp::Torso => {
                if p.y < 31.0 || p.y > neck + 0.6 {
                    return None;
                }
                let strap = {
                    let d1 = (p.x * 0.62 + (p.y - 41.0) * 0.78).abs();
                    let d2 = (-p.x * 0.62 + (p.y - 41.0) * 0.78).abs();
                    d1.min(d2) < 0.65 && p.y < 48.0
                };
                let off = if strap { 0.95 } else { 0.62 };
                if !shell(b, p, off, 1.3) || v_cut(p, neck, 2.0) {
                    return None;
                }
                if strap {
                    if (m.x - 0.0).abs() < 0.7 && (p.y - 41.0).abs() < 0.7 {
                        ink.buckle
                    } else {
                        s.accent[0]
                    }
                } else if (m.x - 3.6).abs() < 0.12 || (p.y - 38.0).abs() < 0.12 {
                    s.trim[0]
                } else {
                    s.leather(p)
                }
            }
            Bp::ArmU | Bp::ArmL => {
                let t = if bp == Bp::ArmU {
                    0.0
                } else {
                    along(b.elbow, b.wrist, m)
                };
                if t > 0.95 || !shell(b, p, 0.32, 1.0) {
                    return None;
                }
                if t > 0.82 { s.accent[1] } else { s.accent[0] }
            }
            _ => return None,
        };
        Some((g_part(b, p), Mode::Set, c))
    }));
    out.push(flaps(
        g,
        b,
        bi,
        &[("chest", &[2])],
        [s.main[1], s.main[2]],
        s.trim[0],
        23.2,
    ));
    // 3: a mail hauberk under a sleeveless surcoat
    let bb = b.clone();
    out.push(layer(
        g,
        "chest/hauberk",
        bi,
        &[("chest", &[3])],
        move |p| {
            let b = &bb;
            let m = mirror(p);
            let bp = b.part_of(p);
            let c = match bp {
                Bp::Torso => {
                    if p.y < 30.5 || p.y > neck + 0.8 {
                        return None;
                    }
                    let coat = p.y < neck - 1.0 && m.x < 6.6 && (p.z > -0.5 || p.y < 46.0);
                    let off = if coat { 0.78 } else { 0.5 };
                    if !shell(b, p, off, 1.3) {
                        return None;
                    }
                    if coat {
                        if m.x > 5.9 || p.y > neck - 1.6 {
                            s.trim[1]
                        } else {
                            s.cloth_accent(p)
                        }
                    } else {
                        s.mail(p)
                    }
                }
                Bp::ArmU => {
                    if !shell(b, p, 0.45, 1.0) {
                        return None;
                    }
                    s.mail(p)
                }
                Bp::ArmL => {
                    let t = along(b.elbow, b.wrist, m);
                    if t > 0.3 || !shell(b, p, 0.4, 1.0) {
                        return None;
                    }
                    s.mail(p)
                }
                _ => return None,
            };
            Some((g_part(b, p), Mode::Set, c))
        },
    ));
    out.push(flaps(
        g,
        b,
        bi,
        &[("chest", &[3])],
        [s.main[0], s.main[2]],
        s.main[0],
        21.6,
    ));
    // 4: a plate cuirass with a gorget and lames, a tabard over it
    let bb = b.clone();
    out.push(layer(
        g,
        "chest/cuirass",
        bi,
        &[("chest", &[4])],
        move |p| {
            let b = &bb;
            let m = mirror(p);
            let bp = b.part_of(p);
            let c = match bp {
                Bp::Torso => {
                    if p.y < 30.8 || p.y > neck + 1.8 {
                        return None;
                    }
                    let gorget = p.y > neck - 1.4;
                    // overlapping lames below the breastplate
                    let lame = if p.y < 37.6 {
                        ((37.6 - p.y) / 1.5).fract()
                    } else {
                        0.0
                    };
                    let keel = if p.z > 1.0 && m.x < 0.5 && p.y > 37.6 {
                        0.25
                    } else {
                        0.0
                    };
                    let off = if gorget {
                        1.05 + (p.y - neck + 1.4) * 0.2
                    } else {
                        0.92 + lame * 0.3 + keel
                    };
                    if !shell(b, p, off, 1.4) {
                        return None;
                    }
                    let tabard = p.z > 1.5 && m.x < 3.3 && p.y < 45.5 && p.y > 30.8;
                    if tabard && shell(b, p, off + 0.0, 0.2) && b.body(p) > off - 0.35 {
                        let e = (1..=4).find(|e| emblem_shape(*e, p.x, p.y + 1.2));
                        let _ = e;
                        return Some((part_index("torso"), Mode::Set, s.accent[1]));
                    }
                    if gorget && p.y > neck + 1.1 || p.y < 31.3 || (p.y - 37.6).abs() < 0.25 {
                        s.trim[1]
                    } else if lame > 0.82 {
                        s.main[0]
                    } else if (p.y - 46.8).abs() < 0.2 && p.z > 0.0 || m.x > 7.6 && p.y > 40.0 {
                        s.trim[1]
                    } else {
                        s.metal(b, p)
                    }
                }
                Bp::ArmU | Bp::ArmL => {
                    let t = if bp == Bp::ArmU {
                        along(b.shoulder, b.elbow, m)
                    } else {
                        1.0 + along(b.elbow, b.wrist, m)
                    };
                    if t > 1.45 || !shell(b, p, 0.4, 1.0) {
                        return None;
                    }
                    // mail sleeves, a couter at the elbow
                    let elbow = (m - b.elbow - vec3(0.0, 0.0, -1.0)).length();
                    if elbow < 1.9 && shell(b, p, 0.95, 1.0) {
                        if elbow > 1.6 {
                            s.trim[1]
                        } else {
                            s.metal(b, p)
                        }
                    } else {
                        s.mail(p)
                    }
                }
                _ => return None,
            };
            Some((g_part(b, p), Mode::Set, c))
        },
    ));
    // the tabard: a cloth panel hanging from the chest to the knees, front and back
    let bb = b.clone();
    out.push(layer(g, "chest/tabard", bi, &[("chest", &[4])], move |p| {
        let b = &bb;
        let m = mirror(p);
        let top = 45.8;
        if p.y > top || p.y < 30.6 || m.x > 3.4 {
            return None;
        }
        let d = b.body(p);
        if b.part_of(p) != Bp::Torso || p.z.abs() < 1.0 || d > 1.35 || d < 0.75 {
            return None;
        }
        let c = if m.x > 2.85 || p.y > top - 0.55 {
            s.trim[1]
        } else if (1..=4).any(|e| emblem_tier(e) && emblem_shape(e, p.x, p.y + 1.6)) && p.z > 0.0 {
            s.trim[2]
        } else {
            s.cloth_accent(p)
        };
        Some((part_index("torso"), Mode::Set, c))
    }));
    out.push(tabard_flaps(g, b, bi, &s));
    out.push(flaps(
        g,
        b,
        bi,
        &[("chest", &[4])],
        [s.main[1], s.main[2]],
        s.trim[1],
        25.6,
    ));
    out
}

/// The cloth robe's skirt at height `y`: half width, front and back
/// depth, and how far down the legs it is (0 at the hips, 1 at the hem).
pub fn robe_skirt(y: f32, female: bool) -> (f32, f32, f32, f32) {
    let hip = smooth(34.0, 27.0, y);
    let low = smooth(27.0, 1.2, y);
    let w = if female { 0.4 } else { 0.0 };
    (
        6.7 + w + 2.6 * hip + 2.3 * low,
        5.1 + 1.0 * hip + 2.0 * low,
        5.5 + 1.1 * hip + 1.7 * low,
        low,
    )
}

/// The tabard's emblem: a sword (one design for every plate set; tiers
/// differ by colour).
fn emblem_tier(e: u8) -> bool {
    e == 1
}

impl Si {
    fn cloth_accent(&self, p: Vec3) -> u8 {
        let n = noise(vec3(p.x * 0.35, p.y * 0.06, p.z * 0.35));
        if n < 0.14 {
            self.accent[0]
        } else if n > 0.86 {
            self.accent[2]
        } else {
            self.accent[1]
        }
    }
}

/// The lower tabard, in front of and behind the plate skirt.
fn tabard_flaps(g: &Grid, b: &Body, bi: usize, s: &Si) -> Layer {
    let s = *s;
    let female = b.female;
    layer(g, "flap/tabard", bi, &[("chest", &[4])], move |p| {
        let top = 32.8;
        if p.y > top || p.y < 15.5 || p.x.abs() > 3.4 + (top - p.y) * 0.03 {
            return None;
        }
        let drop = top - p.y;
        let lift = if female { 0.2 } else { 0.0 };
        let fz = 5.55 + drop * 0.13 - 0.03 * p.x * p.x + lift;
        let bz = -(6.1 + drop * 0.1 - 0.03 * p.x * p.x + lift);
        let (part, z0) = if p.z > 0.0 {
            ("flap.f", fz)
        } else {
            ("flap.b", bz)
        };
        if (p.z - z0).abs() > 0.4 {
            return None;
        }
        // a pointed hem
        if p.y < 15.5 + (3.4 - p.x.abs()).max(0.0) * 0.6 - 1.0 {
            return None;
        }
        let w = 3.4 + drop * 0.03;
        let c = if p.x.abs() > w - 0.45
            || p.y < 17.4 - p.x.abs() * 0.0 && p.y < 16.9 + (3.4 - p.x.abs()) * 0.6 - 1.0
        {
            s.trim[1]
        } else {
            s.cloth_accent(p)
        };
        Some((part_index(part), Mode::Set, c))
    })
}

fn open_edge(p: Vec3, m: Vec3, neck: f32) -> bool {
    p.z > 0.6
        && p.y > neck - 3.6
        && p.y < neck + 0.2
        && (m.x - ((p.y - neck + 3.2) * 0.45 + 0.3)).abs() < 0.45
}

fn v_cut(p: Vec3, neck: f32, depth: f32) -> bool {
    p.z > 0.5 && p.y > neck - depth && p.x.abs() < (p.y - (neck - depth)) * 0.6 + 0.25
}

// ------------------------------------------------------------------ waist

fn waist(g: &Grid, b: &Body, bi: usize, s: &Si, ink: &Inks, k: u8) -> Layer {
    let b = b.clone();
    let s = *s;
    let buckle_ink = ink.buckle;
    let name = [
        "",
        "waist/sash-cloth",
        "waist/pouches",
        "waist/studded",
        "waist/girdle",
    ][k as usize];
    layer(g, name, bi, &[("waist", &[k])], move |p| {
        let m = mirror(p);
        let (lo, hi) = if k == 1 {
            (31.0, 34.4)
        } else if k == 4 {
            (31.4, 34.4)
        } else {
            (31.7, 33.9)
        };
        let front = p.x.abs() < 1.25 && p.z > 2.0;
        // the sash's tails hang at the left hip; pouches sit on the hips
        let tail = k == 1 && p.x < -2.4 && p.x > -4.4 && p.z > 1.0 && p.y < lo && p.y > 22.0;
        let pouch = k == 2
            && (rbox(m, vec3(5.2, 31.2, 2.4), vec3(0.9, 1.4, 1.2), 0.5) < 0.0 && b.body(p) < 2.4
                || rbox(p, vec3(-2.6, 31.0, -5.3), vec3(1.2, 1.3, 0.8), 0.5) < 0.0);
        if pouch {
            let flap = p.y > 31.9;
            return Some((
                part_index("torso"),
                Mode::Set,
                if flap { s.main[1] } else { s.main[2] },
            ));
        }
        if tail {
            let d = b.body(p);
            if d > 2.2 || d < 0.7 {
                return None;
            }
            return Some((
                part_index("torso"),
                Mode::Set,
                if p.y < 22.8 { s.trim[1] } else { s.cloth(p) },
            ));
        }
        if p.y < lo || p.y > hi {
            return None;
        }
        let off = match k {
            1 => 1.0,
            4 => {
                if front {
                    1.5
                } else {
                    1.05
                }
            }
            _ => {
                if front {
                    1.15
                } else {
                    0.9
                }
            }
        };
        if !shell(&b, p, off, 1.8) || !matches!(b.part_of(p), Bp::Torso | Bp::Thigh) {
            return None;
        }
        let c = match k {
            1 => {
                if (p.y - lo).abs() < 0.4 || (p.y - hi).abs() < 0.4 {
                    s.trim[1]
                } else {
                    s.cloth(p)
                }
            }
            2 => {
                if front {
                    buckle_ink
                } else {
                    s.leather(p)
                }
            }
            3 => {
                let stud = (p.x.atan2(p.z) * 8.0).sin() > 0.85 && (p.y - 32.8).abs() < 0.35;
                if front {
                    buckle_ink
                } else if stud {
                    s.trim[2]
                } else {
                    s.leather(p)
                }
            }
            _ => {
                if front {
                    if (m - vec3(0.0, 32.9, 0.0)).truncate().length() < 0.55 {
                        s.gem
                    } else {
                        s.trim[1]
                    }
                } else if (p.y - lo).abs() < 0.35 || (p.y - hi).abs() < 0.35 {
                    s.trim[0]
                } else {
                    s.metal(&b, p)
                }
            }
        };
        Some((part_index("torso"), Mode::Set, c))
    })
}

// ------------------------------------------------------------------ hands

fn hands(g: &Grid, b: &Body, bi: usize, s: &Si, k: u8) -> Layer {
    let b = b.clone();
    let s = *s;
    let name = [
        "",
        "hands/wraps",
        "hands/gloves",
        "hands/mail-gloves",
        "hands/gauntlets",
    ][k as usize];
    layer(g, name, bi, &[("hands", &[k])], move |p| {
        if b.part_of(p) != Bp::ArmL {
            return None;
        }
        let m = mirror(p);
        let t = along(b.elbow, b.wrist, m);
        let q = b.hand_local(m);
        let in_hand = q.y > 0.15;
        let cuff_top = match k {
            1 => 0.45,
            2 => 0.6,
            3 => 0.55,
            _ => 0.35,
        };
        if t < cuff_top && !in_hand {
            return None;
        }
        if k == 1 && in_hand {
            return None;
        }
        // the cuff flares towards the elbow
        let flare = if in_hand {
            0.0
        } else {
            (1.0 - t).max(0.0) / (1.0 - cuff_top)
        };
        let off = match k {
            1 => 0.45 + flare * 0.3,
            2 => {
                if in_hand {
                    0.22
                } else {
                    0.35 + flare * 0.55
                }
            }
            3 => {
                if in_hand {
                    0.28
                } else {
                    0.4 + flare * 0.6
                }
            }
            _ => {
                if in_hand {
                    let knuckle = (q.y - 3.35).abs() < 0.45 && q.x > -0.2;
                    if knuckle { 0.62 } else { 0.42 }
                } else {
                    0.55 + flare * 1.0
                }
            }
        };
        if !shell(&b, p, off, 1.2) {
            return None;
        }
        let rim = !in_hand && t < cuff_top + 0.07;
        let c = match k {
            1 => {
                if rim || t > 0.93 {
                    s.trim[1]
                } else {
                    s.cloth(p)
                }
            }
            2 => {
                if rim {
                    s.trim[0]
                } else if in_hand {
                    s.main[1]
                } else {
                    s.leather(p)
                }
            }
            3 => {
                if in_hand {
                    s.mail(p)
                } else if rim {
                    s.trim[1]
                } else {
                    s.accent[1]
                }
            }
            _ => {
                if rim {
                    s.trim[1]
                } else if in_hand && (q.y - 3.35).abs() < 0.45 && q.x > -0.2 {
                    s.trim[0]
                } else if in_hand && q.y > 3.8 && ((q.y - 3.8) / 0.6).fract() < 0.25 {
                    s.main[0]
                } else {
                    s.metal(&b, p)
                }
            }
        };
        Some((g_part(&b, p), Mode::Set, c))
    })
}

// -------------------------------------------------------------- shoulders

fn shoulders(g: &Grid, b: &Body, bi: usize, s: &Si, k: u8) -> Layer {
    let b = b.clone();
    let s = *s;
    let name = [
        "",
        "shoulders/mantle",
        "shoulders/pads",
        "shoulders/scales",
        "shoulders/pauldrons",
    ][k as usize];
    let f = if b.female { 0.86 } else { 1.0 };
    layer(g, name, bi, &[("shoulders", &[k])], move |p| {
        let m = mirror(p);
        let side = if p.x >= 0.0 { "arm_u.r" } else { "arm_u.l" };
        let sh = b.shoulder;
        match k {
            1 | 3 => {
                // a mantle over both shoulders, scalloped (cloth) or of scales (mail)
                if p.y < 42.5 || p.y > b.head.y - 1.0 {
                    return None;
                }
                let theta = p.x.atan2(p.z);
                let hem = if k == 1 { 45.0 } else { 44.2 }
                    + 0.9 * (theta * if k == 1 { 6.0 } else { 9.0 }).sin().abs()
                    - (m.x - 6.0).max(0.0) * 0.45;
                if p.y < hem {
                    return None;
                }
                let scale = if k == 3 {
                    ((p.y - hem) / 1.1).fract() * 0.35
                } else {
                    0.0
                };
                let d = b.body(p).min(b.arm_u(m) + 0.2);
                let off = if k == 1 { 0.8 } else { 1.05 } + scale + (p.y - 47.0).max(0.0) * 0.1;
                if d > off || d < 0.25 {
                    return None;
                }
                let part = if m.x > sh.x - 1.2 && b.arm_u(m) < 1.6 {
                    side
                } else {
                    "torso"
                };
                let c = if p.y < hem + 0.6 {
                    if k == 1 { s.trim[1] } else { s.main[0] }
                } else if k == 3 {
                    if scale > 0.29 {
                        s.main[1]
                    } else if scale < 0.07 {
                        s.main[3]
                    } else {
                        s.main[2]
                    }
                } else if (m - vec3(3.9, 46.6, 3.8)).length() < 0.7 {
                    s.gem
                } else {
                    s.cloth(p)
                };
                Some((part_index(part), Mode::Set, c))
            }
            2 => {
                // two overlapping leather caps
                if b.part_of(p) != Bp::ArmU && b.arm_u(m) > 2.0 {
                    return None;
                }
                let c0 = sh + vec3(0.6, 0.6, 0.0);
                let d1 = ellipsoid(m, c0, vec3(3.6, 3.0, 3.7) * f);
                let d2 = ellipsoid(m, c0 + vec3(0.5, -2.2, 0.0), vec3(3.4, 2.2, 3.5) * f);
                let inner = b.arm_u(m);
                let d = if p.y > c0.y - 1.0 { d1 } else { d2 };
                if d > 0.0 || inner < -0.1 || m.x < sh.x - 2.4 * f || p.y < c0.y - 3.6 {
                    return None;
                }
                let rim = d > -0.45 && (p.y < c0.y - 0.6 && p.y > c0.y - 1.3 || p.y < c0.y - 3.0);
                Some((
                    part_index(side),
                    Mode::Set,
                    if rim { s.trim[0] } else { s.leather(p) },
                ))
            }
            _ => {
                // big plate pauldrons: a dome and two lames, rimmed, with a ridge
                let c0 = sh + vec3(0.9, 1.1, 0.0);
                let r = vec3(4.6, 3.4, 4.5) * f;
                let dome = ellipsoid(m, c0, r);
                let l1 = ellipsoid(m, c0 + vec3(0.6, -2.1, 0.0), r * vec3(0.95, 0.8, 0.97));
                let l2 = ellipsoid(m, c0 + vec3(1.1, -3.9, 0.0), r * vec3(0.85, 0.7, 0.92));
                let ridge = cone(
                    m,
                    c0 + vec3(-1.4, 3.0, 0.0),
                    c0 + vec3(2.6, 1.6, 0.0),
                    0.55,
                    0.3,
                );
                let band = if p.y > c0.y - 0.9 {
                    dome
                } else if p.y > c0.y - 2.9 {
                    l1
                } else {
                    l2
                };
                let d = band.min(ridge);
                let inner = b.body(p);
                if d > 0.0 || inner < 0.4 || m.x < sh.x - 3.6 * f || p.y < c0.y - 5.6 {
                    return None;
                }
                let shell_d = -d;
                let c = if ridge < 0.0
                    || shell_d < 0.5 && (p.y - (c0.y - 0.9)).abs() < 0.45
                    || (p.y - (c0.y - 2.9)).abs() < 0.4
                    || p.y < c0.y - 5.0
                {
                    s.trim[1]
                } else if (m - (c0 + vec3(1.6, 0.6, 3.4) * f)).length() < 0.55 {
                    s.gem
                } else {
                    s.metal(&b, p + vec3(0.0, 1.0, 0.0))
                };
                Some((part_index(side), Mode::Set, c))
            }
        }
    })
}

fn quiver(g: &Grid, b: &Body, bi: usize, s: &Si, ink: &Inks) -> Layer {
    let b = b.clone();
    let s = *s;
    let fletch = ink.trim[1];
    layer(g, "back/quiver", bi, &[("quiver", &[1])], move |p| {
        let back = if b.female { -4.4 } else { -4.9 };
        let a = vec3(-3.2, 31.5, back - 1.4);
        let top = vec3(3.6, 46.5, back - 1.9);
        let d = cone(p, a, top, 1.35, 1.6);
        if d < 0.0 {
            let t = along(a, top, p);
            let c = if t > 0.93 || (t - 0.35).abs() < 0.04 {
                s.trim[1]
            } else {
                s.leather(p)
            };
            return Some((part_index("torso"), Mode::Set, c));
        }
        // arrows' fletching out of the top
        let dir = (top - a).normalize();
        for i in 0..4 {
            let o = vec3((i as f32 - 1.5) * 0.6, 0.0, (i % 2) as f32 * 0.5 - 0.25);
            let f0 = top + o;
            if cone(p, f0, f0 + dir * 2.4, 0.32, 0.32) < 0.0 {
                return Some((part_index("torso"), Mode::Set, fletch));
            }
        }
        None
    })
}

// ------------------------------------------------------------------- head

pub fn helms(head: &mut Grid, bodies: &[Body; 2], _ink: &Inks, pal: &mut Palette) {
    let s = Si::new(pal, "head");
    for (bi, b) in bodies.iter().enumerate() {
        for k in 1..=4u8 {
            head.push("head", helm(head, b, bi, &s, k));
        }
    }
}

fn helm(g: &Grid, b: &Body, bi: usize, s: &Si, k: u8) -> Layer {
    let b = b.clone();
    let s = *s;
    let name = ["", "helm/circlet", "helm/cowl", "helm/hood", "helm/helm"][k as usize];
    g.layer(
        &format!("{name},body={bi}"),
        body_when(bi, &[("helm", &[k])]),
        vec3(-7.5, b.head.y - 8.5, -8.5),
        vec3(7.5, b.head.y + 14.0, 8.5),
        move |p| -> Cell {
            let m = mirror(p);
            let y = |v: f32| b.hy(v);
            let cran = b.cranium(p);
            let skull = b.skull(p);
            match k {
                1 => {
                    // a circlet round the brow, a gem at the front
                    let band = (p.y - (y(59.0) - p.z * 0.12)).abs() < 0.32;
                    if !band || cran > 0.75 || cran < 0.1 {
                        return None;
                    }
                    let gem = p.z > 3.0 && m.x < 0.5;
                    Some((1, Mode::Set, if gem { s.gem } else { s.trim[1] }))
                }
                2 | 3 => {
                    // a cowl with a mask (leather) or a hood with a point and a mail coif (mail)
                    let point = if k == 3 {
                        cone(
                            p,
                            vec3(0.0, y(59.5), -3.0),
                            vec3(0.0, y(58.2), -7.2),
                            2.6,
                            0.2,
                        )
                    } else {
                        f32::MAX
                    };
                    let drape = cone(
                        p,
                        vec3(0.0, b.head.y - 0.5, -0.6),
                        vec3(0.0, b.head.y - 3.2, -0.6),
                        3.6,
                        4.6,
                    );
                    let d = cran.min(skull).min(drape + 0.9);
                    let off = 1.15 + noise(p * 0.9) * 0.2;
                    let face =
                        p.z > 1.4 && m.x < 2.9 && p.y < y(58.7) && p.y > y(k as f32 * 0.0 + 51.5);
                    let mask = k == 2 && p.y < y(55.6) && p.z > 0.0;
                    let inside = d < off || point < 0.0;
                    if p.y < b.head.y - 3.4 {
                        return None;
                    }
                    if inside && d > -0.4 && (!face || mask) {
                        let rim = p.z > 1.0 && m.x < 3.6 && p.y < y(59.4) && p.y > y(55.6);
                        let c = if mask {
                            s.accent[1]
                        } else if k == 3 && rim {
                            if (p.y * 2.0).floor() as i32 % 2 == 0 {
                                s.accent[2]
                            } else {
                                s.accent[0]
                            }
                        } else if rim {
                            s.main[1]
                        } else if k == 2 {
                            s.leather(p)
                        } else {
                            s.cloth(p)
                        };
                        return Some((1, Mode::Set, c));
                    }
                    None
                }
                _ => {
                    // an open helm: dome, crest, cheek guards and a nasal
                    let off = 1.15;
                    let crest = m.x < 0.55
                        && p.y > y(58.0)
                        && cran < off + 0.75
                        && cran > 0.2
                        && p.z > -4.8;
                    let dome = cran < off && p.y > y(57.6) - (-p.z).max(0.0) * 0.55 && cran > -0.5;
                    let cheek = skull < off - 0.1
                        && skull > -0.4
                        && p.y > y(53.4)
                        && p.y < y(58.0)
                        && m.x > 1.95
                        && p.z > -1.0;
                    let back = cran.min(skull) < off
                        && cran.min(skull) > -0.4
                        && p.z < -1.0
                        && p.y > y(53.0);
                    let nasal = m.x < 0.28
                        && p.y > y(55.9)
                        && p.y < y(58.0)
                        && skull < 0.7
                        && skull > 0.0
                        && p.z > 3.0;
                    if crest {
                        return Some((1, Mode::Set, s.trim[1]));
                    }
                    if dome || cheek || back || nasal {
                        let brow = (p.y - y(57.85)).abs() < 0.35 && p.z > 0.0;
                        let rivet = p.z > 1.0
                            && (p.y - y(57.85)).abs() < 0.3
                            && ((p.x * 2.4).fract().abs() < 0.12);
                        let c = if rivet {
                            s.trim[2]
                        } else if brow || cheek && (p.y - y(53.6)).abs() < 0.35 {
                            s.trim[1]
                        } else if (m - vec3(0.0, y(59.3), 3.9)).length() < 0.6 {
                            s.gem
                        } else {
                            let up = b.cranium(p + vec3(0.0, 0.5, 0.0)) - cran;
                            if up > 0.3 {
                                s.main[3]
                            } else if up < -0.2 {
                                s.main[1]
                            } else {
                                s.main[2]
                            }
                        };
                        return Some((1, Mode::Set, c));
                    }
                    None
                }
            }
        },
    )
}
