//! Character, NPC, creature and held-item models, built from boxes.

use crate::render::{Vertex, push_block_cube, push_oriented_box, push_post_oriented};
use dyno_world as w;
use glam::Vec3;

/// How a person looks: colours and pieces of an outfit.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub skin: [f32; 3],
    pub hair: [f32; 3],
    /// 0 bald, 1 short, 2 long, 3 hood
    pub hair_style: u8,
    pub beard: bool,
    /// main garment (tabard or robe)
    pub tunic: [f32; 3],
    /// trim, belt buckle and embroidery
    pub trim: [f32; 3],
    pub pants: [f32; 3],
    pub boots: [f32; 3],
    /// metal shoulder guards
    pub pauldrons: bool,
    /// a long robe instead of trousers
    pub robe: bool,
    pub apron: Option<[f32; 3]>,
    pub cape: Option<[f32; 3]>,
}

fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

impl Look {
    /// A traveller's look from a seed and a signature colour.
    pub fn player(seed: u32, color: [f32; 3]) -> Look {
        let h = hash(seed);
        let skins = [[0.96, 0.80, 0.66], [0.87, 0.68, 0.52], [0.72, 0.52, 0.38], [0.55, 0.38, 0.27], [0.98, 0.86, 0.75]];
        let hairs = [[0.18, 0.12, 0.08], [0.45, 0.28, 0.12], [0.85, 0.68, 0.35], [0.62, 0.22, 0.10], [0.12, 0.12, 0.14], [0.85, 0.85, 0.82]];
        Look {
            skin: skins[(h % 5) as usize],
            hair: hairs[((h >> 3) % 6) as usize],
            hair_style: 1 + ((h >> 6) % 2) as u8,
            beard: (h >> 8) % 3 == 0,
            tunic: color,
            trim: [0.85, 0.68, 0.25],
            pants: [0.30, 0.24, 0.20],
            boots: [0.22, 0.15, 0.10],
            pauldrons: (h >> 10) % 2 == 0,
            robe: false,
            apron: None,
            cape: Some([color[0] * 0.6, color[1] * 0.6, color[2] * 0.6]),
        }
    }

    /// The look of a named townsperson; others get one from their colour.
    pub fn npc(key: &str, color: [f32; 3]) -> Look {
        let mut l = Look::player(key.bytes().map(|b| b as u32).sum(), color);
        l.cape = None;
        match key {
            "brannoc" => {
                l.skin = [0.92, 0.76, 0.62];
                l.hair = [0.88, 0.88, 0.86];
                l.hair_style = 2;
                l.beard = true;
                l.robe = true;
                l.tunic = [0.45, 0.25, 0.18];
                l.trim = [0.90, 0.70, 0.30];
                l.pauldrons = false;
            }
            "maren" => {
                l.hair = [0.62, 0.22, 0.10];
                l.hair_style = 2;
                l.beard = false;
                l.tunic = [0.20, 0.30, 0.62];
                l.trim = [0.85, 0.75, 0.40];
                l.pauldrons = true;
                l.cape = Some([0.12, 0.18, 0.42]);
            }
            "tilda" => {
                l.hair = [0.45, 0.28, 0.12];
                l.hair_style = 2;
                l.beard = false;
                l.tunic = [0.62, 0.38, 0.22];
                l.pauldrons = false;
                l.apron = Some([0.92, 0.90, 0.82]);
            }
            "hollis" => {
                l.skin = [0.72, 0.52, 0.38];
                l.hair_style = 0;
                l.beard = true;
                l.hair = [0.20, 0.14, 0.10];
                l.tunic = [0.40, 0.36, 0.34];
                l.pauldrons = false;
                l.apron = Some([0.35, 0.24, 0.14]);
            }
            _ => {}
        }
        l
    }
}

/// Draw a player: legs and arms swing with `walk` (0 standing, 1 walking).
#[allow(clippy::too_many_arguments)]
pub fn push_avatar(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    color: [f32; 3],
    light: f32,
    phase: f32,
    walk: f32,
) {
    let seed = (color[0] * 997.0 + color[1] * 131.0 + color[2] * 17.0) as u32;
    push_person(out, feet, yaw, pitch, &Look::player(seed, color), light, phase, walk);
}

/// Draw a person in an outfit. Proportions are heroic: big hands, head and shoulders.
#[allow(clippy::too_many_arguments)]
pub fn push_person(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    look: &Look,
    light: f32,
    phase: f32,
    walk: f32,
) {
    let (sy, cy) = yaw.sin_cos();
    let right = Vec3::new(cy, 0.0, -sy);
    let fwd = Vec3::new(-sy, 0.0, -cy);
    let up = Vec3::Y;
    let axes = [right, up, fwd];
    let swing = phase.sin() * 0.7 * walk;
    let b = |out: &mut Vec<Vertex>, c: Vec3, ax: [Vec3; 3], h: Vec3, rgb: [f32; 3]| {
        push_oriented_box(out, c, ax, h, rgb, false, light);
    };
    let dark = |c: [f32; 3], k: f32| [c[0] * k, c[1] * k, c[2] * k];
    let metal = [0.62, 0.64, 0.70];
    // a limb hanging from `pivot`, swung by `angle`
    let limb = |out: &mut Vec<Vertex>, pivot: Vec3, angle: f32, len: f32, w: f32| {
        let (s, c) = angle.sin_cos();
        let u2 = up * c + fwd * s;
        let f2 = fwd * c - up * s;
        (pivot - u2 * (len * 0.5), [right, u2, f2], u2, Vec3::new(w, len * 0.5, w))
    };
    // legs, boots
    let hip = feet + up * 0.78;
    for (side, a) in [(1.0f32, swing), (-1.0, -swing)] {
        let (c, ax, u2, h) = limb(out, hip + right * (0.13 * side), a, 0.78, 0.12);
        if !look.robe {
            b(out, c + u2 * 0.08, ax, Vec3::new(h.x, h.y - 0.08, h.z), look.pants);
        }
        // boot with a turned-down cuff
        let foot = hip + right * (0.13 * side) - u2 * 0.62;
        b(out, foot, ax, Vec3::new(0.14, 0.17, 0.16), look.boots);
        b(out, foot + u2 * 0.15, ax, Vec3::new(0.15, 0.04, 0.17), dark(look.boots, 1.3));
        b(out, foot - u2 * 0.12 + ax[2] * 0.06, ax, Vec3::new(0.13, 0.05, 0.19), dark(look.boots, 0.8));
    }
    // robe skirt, or tabard flaps
    if look.robe {
        b(out, feet + up * 0.5, axes, Vec3::new(0.3, 0.46, 0.2), look.tunic);
        b(out, feet + up * 0.08, axes, Vec3::new(0.32, 0.05, 0.22), look.trim);
    } else {
        b(out, feet + up * 0.68 + fwd * 0.14, axes, Vec3::new(0.18, 0.2, 0.02), look.tunic);
        b(out, feet + up * 0.52 + fwd * 0.155, axes, Vec3::new(0.18, 0.025, 0.02), look.trim);
    }
    // torso, belt and buckle
    let chest = feet + up * 1.17;
    b(out, chest, axes, Vec3::new(0.28, 0.36, 0.16), look.tunic);
    b(out, chest + fwd * 0.165 + up * 0.08, axes, Vec3::new(0.06, 0.24, 0.01), look.trim);
    b(out, feet + up * 0.86, axes, Vec3::new(0.29, 0.05, 0.17), [0.25, 0.16, 0.09]);
    b(out, feet + up * 0.86 + fwd * 0.175, axes, Vec3::new(0.05, 0.045, 0.01), look.trim);
    if let Some(apron) = look.apron {
        b(out, feet + up * 0.9 + fwd * 0.17, axes, Vec3::new(0.22, 0.4, 0.015), apron);
    }
    // cape, swaying
    if let Some(cape) = look.cape {
        let sway = 0.18 + walk * 0.35 + (phase * 0.5).sin() * 0.04;
        let (s, c) = sway.sin_cos();
        let u2 = up * c - fwd * s;
        let f2 = fwd * c + up * s;
        let top = chest + up * 0.3 - fwd * 0.18;
        b(out, top - u2 * 0.55, [right, u2, f2], Vec3::new(0.27, 0.55, 0.02), cape);
    }
    // arms with gloves
    let shoulder = feet + up * 1.45;
    for (side, a) in [(1.0f32, -swing * 0.8), (-1.0, swing * 0.8)] {
        let (c, ax, u2, h) = limb(out, shoulder + right * (0.38 * side), a, 0.68, 0.1);
        b(out, c + u2 * 0.08, ax, Vec3::new(h.x, h.y - 0.08, h.z), look.tunic);
        let hand = shoulder + right * (0.38 * side) - u2 * 0.62;
        b(out, hand, ax, Vec3::new(0.12, 0.1, 0.12), dark(look.boots, 1.2));
        // pauldron
        if look.pauldrons {
            let p = shoulder + right * (0.4 * side) + up * 0.05;
            b(out, p, axes, Vec3::new(0.17, 0.1, 0.19), metal);
            b(out, p + up * 0.1, axes, Vec3::new(0.12, 0.04, 0.14), dark(metal, 1.2));
            b(out, p - up * 0.1, axes, Vec3::new(0.18, 0.025, 0.2), look.trim);
        }
    }
    // head, tilted by pitch
    let (sp, cp) = (pitch * 0.6).sin_cos();
    let hu = up * cp - fwd * sp;
    let hf = fwd * cp + up * sp;
    let hax = [right, hu, hf];
    let head = feet + up * 1.83;
    b(out, head, hax, Vec3::splat(0.26), look.skin);
    // eyes, brows, nose
    for side in [-1.0f32, 1.0] {
        let e = head + hf * 0.265 + right * (0.1 * side) + hu * 0.03;
        b(out, e, hax, Vec3::new(0.045, 0.04, 0.01), [0.98, 0.98, 0.98]);
        b(out, e + right * (0.012 * side), hax, Vec3::new(0.022, 0.035, 0.012), [0.1, 0.12, 0.2]);
        b(out, e + hu * 0.075, hax, Vec3::new(0.06, 0.018, 0.012), dark(look.hair, 0.8));
    }
    b(out, head + hf * 0.275 - hu * 0.04, hax, Vec3::new(0.035, 0.05, 0.03), dark(look.skin, 0.88));
    // hair
    match look.hair_style {
        1 => {
            b(out, head + hu * 0.2 - hf * 0.02, hax, Vec3::new(0.28, 0.09, 0.28), look.hair);
            b(out, head - hf * 0.2 + hu * 0.05, hax, Vec3::new(0.28, 0.2, 0.08), look.hair);
            b(out, head + hf * 0.22 + hu * 0.19 + right * 0.08, hax, Vec3::new(0.14, 0.05, 0.06), look.hair);
        }
        2 => {
            b(out, head + hu * 0.2, hax, Vec3::new(0.29, 0.1, 0.29), look.hair);
            b(out, head - hf * 0.22 - hu * 0.12, hax, Vec3::new(0.29, 0.4, 0.08), look.hair);
            for side in [-1.0f32, 1.0] {
                b(out, head + right * (0.27 * side) - hu * 0.05, hax, Vec3::new(0.04, 0.25, 0.2), look.hair);
            }
        }
        3 => {
            b(out, head + hu * 0.12, hax, Vec3::new(0.31, 0.2, 0.31), look.tunic);
        }
        _ => {}
    }
    if look.beard {
        b(out, head + hf * 0.24 - hu * 0.2, hax, Vec3::new(0.22, 0.14, 0.06), look.hair);
        b(out, head + hf * 0.26 - hu * 0.36, hax, Vec3::new(0.14, 0.08, 0.05), look.hair);
    }
}

fn render_limb(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axes: [Vec3; 3],
    half: Vec3,
    rgb: [f32; 3],
    light: f32,
) {
    push_oriented_box(out, center, axes, half, rgb, false, light);
}

/// Draw a creature. `flash` (0 to 1) whitens it when hit; `t` is a clock in seconds.
#[allow(clippy::too_many_arguments)]
pub fn push_creature(
    out: &mut Vec<Vertex>,
    model: dyno_world::content::Model,
    feet: Vec3,
    yaw: f32,
    color: [f32; 3],
    light: f32,
    phase: f32,
    walk: f32,
    flash: f32,
    t: f32,
) {
    use dyno_world::content::Model;
    let (sy, cy) = yaw.sin_cos();
    let right = Vec3::new(cy, 0.0, -sy);
    let fwd = Vec3::new(-sy, 0.0, -cy);
    let up = Vec3::Y;
    let hit = |c: [f32; 3]| -> [f32; 3] {
        [
            c[0] + (1.0 - c[0]) * flash,
            c[1] + (1.0 - c[1]) * flash,
            c[2] + (1.0 - c[2]) * flash,
        ]
    };
    let lit = if flash > 0.0 { 1.0 } else { light };
    let bx =
        |out: &mut Vec<Vertex>, c: Vec3, axes: [Vec3; 3], h: Vec3, rgb: [f32; 3], glow: bool| {
            push_oriented_box(out, c, axes, h, hit(rgb), glow || flash > 0.5, lit);
        };
    let swing = phase.sin() * 0.6 * walk;
    match model {
        Model::Quadruped => {
            // a stocky, woolly grazer with a dark face and curled horns
            let wool = color;
            let wool_hi = [color[0] * 1.08, color[1] * 1.08, color[2] * 1.06];
            let face = [0.34, 0.26, 0.21];
            let hoof = [0.24, 0.19, 0.16];
            let horn = [0.92, 0.86, 0.68];
            let body = feet + up * 0.78;
            let axes = [right, up, fwd];
            // legs
            for (sx, sz, ph) in [
                (1.0f32, 1.0f32, 0.0f32),
                (-1.0, 1.0, std::f32::consts::PI),
                (1.0, -1.0, std::f32::consts::PI),
                (-1.0, -1.0, 0.0),
            ] {
                let hip = feet + up * 0.52 + right * (0.2 * sx) + fwd * (0.32 * sz);
                let a = (phase + ph).sin() * 0.55 * walk;
                let (s, c) = a.sin_cos();
                let u2 = up * c + fwd * s;
                let f2 = fwd * c - up * s;
                bx(
                    out,
                    hip - u2 * 0.26,
                    [right, u2, f2],
                    Vec3::new(0.075, 0.26, 0.075),
                    hoof,
                    false,
                );
            }
            // body, with a lighter fleece on top
            bx(out, body, axes, Vec3::new(0.33, 0.27, 0.48), wool, false);
            bx(
                out,
                body + up * 0.2 + fwd * 0.04,
                axes,
                Vec3::new(0.29, 0.12, 0.4),
                wool_hi,
                false,
            );
            // tail
            bx(
                out,
                body - fwd * 0.5 + up * 0.1,
                axes,
                Vec3::new(0.07, 0.09, 0.05),
                wool_hi,
                false,
            );
            // head: grazing dips it now and then when standing
            let graze = if walk < 0.15 {
                ((t * 0.7 + phase * 0.1).sin() - 0.55).max(0.0) * 1.6
            } else {
                0.0
            };
            let neck = body + fwd * 0.5 + up * 0.12;
            let (gs, gc) = (graze * 0.9).sin_cos();
            let hu = up * gc + fwd * gs;
            let hf = fwd * gc - up * gs;
            let head = neck + hf * 0.12 + hu * (0.1 - graze * 0.25);
            bx(
                out,
                head,
                [right, hu, hf],
                Vec3::new(0.16, 0.16, 0.2),
                face,
                false,
            );
            // fleece cap and horns
            bx(
                out,
                head + hu * 0.15 - hf * 0.03,
                [right, hu, hf],
                Vec3::new(0.17, 0.06, 0.14),
                wool_hi,
                false,
            );
            for side in [-1.0f32, 1.0] {
                bx(
                    out,
                    head + right * (0.2 * side) + hu * 0.08 - hf * 0.02,
                    [right, hu, hf],
                    Vec3::new(0.05, 0.07, 0.07),
                    horn,
                    false,
                );
                bx(
                    out,
                    head + right * (0.22 * side) - hu * 0.0 + hf * 0.06,
                    [right, hu, hf],
                    Vec3::new(0.04, 0.04, 0.05),
                    horn,
                    false,
                );
                // eyes
                bx(
                    out,
                    head + hf * 0.205 + right * (0.08 * side) + hu * 0.04,
                    [right, hu, hf],
                    Vec3::new(0.025, 0.03, 0.01),
                    [0.06, 0.05, 0.05],
                    false,
                );
            }
            // muzzle
            bx(
                out,
                head + hf * 0.2 - hu * 0.07,
                [right, hu, hf],
                Vec3::new(0.1, 0.06, 0.03),
                [0.5, 0.4, 0.34],
                false,
            );
        }
        Model::Humanoid => {
            // a Shade: a hooded, legless shape of darkness with two pale eyes
            let bob = (t * 2.1 + phase * 0.05).sin() * 0.07;
            let base = feet + up * (0.12 + bob);
            let dark = color;
            let edge = [color[0] * 1.6, color[1] * 1.4, color[2] * 1.9];
            let axes = [right, up, fwd];
            let lean = 0.18 * walk;
            let (ls, lc) = lean.sin_cos();
            let bu = up * lc + fwd * ls;
            let bf = fwd * lc - up * ls;
            // tattered lower robe, narrowing to wisps
            bx(
                out,
                base + up * 0.55,
                [right, bu, bf],
                Vec3::new(0.23, 0.32, 0.17),
                dark,
                false,
            );
            for (i, (sx, ph)) in [(-0.13f32, 0.0f32), (0.0, 2.1), (0.13, 4.2)]
                .iter()
                .enumerate()
            {
                let sway = (t * 3.0 + ph).sin() * 0.05;
                let len = 0.16 + 0.05 * i as f32;
                bx(
                    out,
                    base + up * (0.2 - len * 0.5) + right * (sx + sway) - fwd * 0.04,
                    axes,
                    Vec3::new(0.06, len, 0.06),
                    dark,
                    false,
                );
            }
            // chest and shoulders
            let chest = base + bu * 1.08;
            bx(
                out,
                chest,
                [right, bu, bf],
                Vec3::new(0.27, 0.3, 0.16),
                dark,
                false,
            );
            bx(
                out,
                chest + bu * 0.26,
                [right, bu, bf],
                Vec3::new(0.3, 0.06, 0.17),
                edge,
                false,
            );
            // long reaching arms
            let reach = 0.7 + 0.5 * walk;
            for side in [-1.0f32, 1.0] {
                let a = -(reach + (t * 2.4 + side).sin() * 0.12 + swing * side * 0.3);
                let (s, c) = a.sin_cos();
                let u2 = bu * c + bf * (-s);
                let f2 = bf * c + bu * s;
                let shoulder = chest + bu * 0.22 + right * (0.33 * side);
                bx(
                    out,
                    shoulder - u2 * 0.4,
                    [right, u2, f2],
                    Vec3::new(0.065, 0.4, 0.065),
                    dark,
                    false,
                );
                // pale fingertips
                bx(
                    out,
                    shoulder - u2 * 0.82,
                    [right, u2, f2],
                    Vec3::new(0.07, 0.05, 0.07),
                    edge,
                    false,
                );
            }
            // hood and face
            let head = chest + bu * 0.55 + bf * 0.06;
            bx(
                out,
                head,
                [right, bu, bf],
                Vec3::new(0.22, 0.24, 0.22),
                dark,
                false,
            );
            bx(
                out,
                head + bu * 0.2 - bf * 0.06,
                [right, bu, bf],
                Vec3::new(0.2, 0.08, 0.2),
                edge,
                false,
            );
            bx(
                out,
                head + bf * 0.2,
                [right, bu, bf],
                Vec3::new(0.16, 0.16, 0.03),
                [0.03, 0.02, 0.05],
                false,
            );
            // the eyes: glowing, so they show in the dark
            let blink = if (t * 0.37 + phase * 0.013).fract() < 0.03 {
                0.2
            } else {
                1.0
            };
            for side in [-1.0f32, 1.0] {
                push_oriented_box(
                    out,
                    head + bf * 0.235 + right * (0.075 * side) + bu * 0.02,
                    [right, bu, bf],
                    Vec3::new(0.035, 0.022 * blink, 0.01),
                    hit([0.86, 0.72, 1.0]),
                    true,
                    1.0,
                );
            }
        }
        Model::Orb => {
            let c = feet + up * (0.8 + (t * 1.7).sin() * 0.15);
            bx(out, c, [right, up, fwd], Vec3::splat(0.16), color, true);
            bx(
                out,
                c,
                [right, up, fwd],
                Vec3::splat(0.22),
                [color[0] * 0.5, color[1] * 0.5, color[2] * 0.5],
                false,
            );
        }
    }
}

/// Draw the item held in the first-person hand. `ax` are the hand's axes
/// (right, up, forward) and `hand` its centre.
pub fn push_held(
    out: &mut Vec<Vertex>,
    c: &dyno_world::content::Content,
    item: Option<dyno_world::content::ItemId>,
    hand: Vec3,
    ax: [Vec3; 3],
    light: f32,
) {
    use dyno_world::content::ToolKind;
    let skin = [0.91, 0.76, 0.62];
    let Some(item) = item else {
        push_oriented_box(
            out,
            hand - ax[1] * 0.06,
            ax,
            Vec3::new(0.06, 0.18, 0.06),
            skin,
            false,
            light,
        );
        return;
    };
    let it = c.item(item);
    let glow = it.light > 0 || it.block.is_some_and(|b| w::emission(b) > 0);
    if let Some(b) = it.block {
        let _ = glow;
        if w::shape(b) == w::Shape::Post {
            push_post_oriented(out, hand + ax[1] * 0.02, ax, 0.45, b, light);
        } else {
            push_block_cube(out, hand, ax, 0.11, b, light);
        }
        return;
    }
    // tilt tools forward so the head points up and away
    let (s, k) = (-0.55f32).sin_cos();
    let hu = ax[1] * k + ax[2] * s;
    let hf = ax[2] * k - ax[1] * s;
    let axes = [ax[0], hu, hf];
    let wood = [0.55, 0.40, 0.24];
    match it.tool.map(|t| t.kind) {
        Some(kind) => {
            if kind != ToolKind::Sword {
                push_oriented_box(
                    out,
                    hand + hu * 0.04,
                    axes,
                    Vec3::new(0.022, 0.17, 0.022),
                    wood,
                    false,
                    light,
                );
            }
            match kind {
                ToolKind::Pickaxe => push_oriented_box(
                    out,
                    hand + hu * 0.2,
                    axes,
                    Vec3::new(0.14, 0.028, 0.03),
                    it.color,
                    glow,
                    light,
                ),
                ToolKind::Axe => push_oriented_box(
                    out,
                    hand + hu * 0.16 + ax[0] * 0.05,
                    axes,
                    Vec3::new(0.065, 0.06, 0.02),
                    it.color,
                    glow,
                    light,
                ),
                ToolKind::Shovel | ToolKind::Hoe => push_oriented_box(
                    out,
                    hand + hu * 0.25,
                    axes,
                    Vec3::new(0.05, 0.07, 0.014),
                    it.color,
                    glow,
                    light,
                ),
                ToolKind::Sword => {
                    // a chunky fantasy sword: pommel, wrapped grip, flared guard,
                    // a bevelled blade with a bright edge and a tip
                    let c = it.color;
                    let lite = [(c[0] * 1.35).min(1.0), (c[1] * 1.35).min(1.0), (c[2] * 1.35).min(1.0)];
                    let dark = [c[0] * 0.7, c[1] * 0.7, c[2] * 0.7];
                    let gold = [0.85, 0.66, 0.25];
                    let leather = [0.30, 0.18, 0.10];
                    // modelled at full size, drawn at 60% around the hand
                    let k = 0.6;
                    let bx = |out: &mut Vec<Vertex>, at: Vec3, h: Vec3, rgb: [f32; 3], g: bool| {
                        push_oriented_box(out, hand + (at - hand) * k, axes, h * k, rgb, g, light);
                    };
                    bx(out, hand - hu * 0.13, Vec3::new(0.03, 0.025, 0.03), gold, false);
                    bx(out, hand - hu * 0.05, Vec3::new(0.022, 0.07, 0.022), leather, false);
                    bx(out, hand + hu * 0.03, Vec3::new(0.11, 0.018, 0.028), gold, false);
                    bx(out, hand + hu * 0.03 + ax[0] * 0.11, Vec3::new(0.02, 0.03, 0.03), gold, false);
                    bx(out, hand + hu * 0.03 - ax[0] * 0.11, Vec3::new(0.02, 0.03, 0.03), gold, false);
                    bx(out, hand + hu * 0.3, Vec3::new(0.045, 0.26, 0.012), c, glow);
                    bx(out, hand + hu * 0.3 + ax[0] * 0.035, Vec3::new(0.012, 0.26, 0.013), lite, glow);
                    bx(out, hand + hu * 0.3, Vec3::new(0.008, 0.24, 0.016), dark, glow);
                    bx(out, hand + hu * 0.59, Vec3::new(0.03, 0.03, 0.011), lite, glow);
                    bx(out, hand + hu * 0.63, Vec3::new(0.015, 0.02, 0.01), lite, glow);
                }
            }
        }
        None => push_oriented_box(
            out,
            hand,
            axes,
            Vec3::new(0.08, 0.08, 0.02),
            it.color,
            glow,
            light,
        ),
    }
}
