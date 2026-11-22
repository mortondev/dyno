//! Character, NPC, creature and held-item models, built from boxes.

use crate::render::{Vertex, push_block_cube, push_oriented_box, push_post_oriented};
use dyno_world as w;
use glam::Vec3;

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
    let (sy, cy) = yaw.sin_cos();
    let right = Vec3::new(cy, 0.0, -sy);
    let fwd = Vec3::new(-sy, 0.0, -cy);
    let up = Vec3::Y;
    let skin = [0.91, 0.76, 0.62];
    let pants = [color[0] * 0.45, color[1] * 0.45, color[2] * 0.5];
    let swing = phase.sin() * 0.7 * walk;
    let limb = |out: &mut Vec<Vertex>, pivot: Vec3, angle: f32, len: f32, w: f32, rgb: [f32; 3]| {
        let (s, c) = angle.sin_cos();
        let u2 = up * c + fwd * s;
        let f2 = fwd * c - up * s;
        render_limb(
            out,
            pivot - u2 * (len * 0.5),
            [right, u2, f2],
            Vec3::new(w, len * 0.5, w),
            rgb,
            light,
        );
    };
    // legs
    let hip = feet + up * 0.75;
    limb(out, hip + right * 0.12, swing, 0.75, 0.115, pants);
    limb(out, hip - right * 0.12, -swing, 0.75, 0.115, pants);
    // body
    push_oriented_box(
        out,
        feet + up * 1.1,
        [right, up, fwd],
        Vec3::new(0.25, 0.35, 0.13),
        color,
        false,
        light,
    );
    // arms
    let shoulder = feet + up * 1.42;
    limb(out, shoulder + right * 0.36, -swing * 0.8, 0.66, 0.1, color);
    limb(out, shoulder - right * 0.36, swing * 0.8, 0.66, 0.1, color);
    // head, tilted by pitch
    let (sp, cp) = (pitch * 0.6).sin_cos();
    let hu = up * cp - fwd * sp;
    let hf = fwd * cp + up * sp;
    let head = feet + up * 1.71;
    push_oriented_box(
        out,
        head,
        [right, hu, hf],
        Vec3::splat(0.24),
        skin,
        false,
        light,
    );
    // eyes
    for side in [-1.0f32, 1.0] {
        push_oriented_box(
            out,
            head + hf * 0.245 + right * (0.09 * side) + hu * 0.03,
            [right, hu, hf],
            Vec3::new(0.04, 0.045, 0.01),
            [0.12, 0.12, 0.16],
            false,
            light,
        );
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
                    push_oriented_box(
                        out,
                        hand - hu * 0.06,
                        axes,
                        Vec3::new(0.02, 0.06, 0.02),
                        wood,
                        false,
                        light,
                    );
                    push_oriented_box(
                        out,
                        hand + hu * 0.0,
                        axes,
                        Vec3::new(0.08, 0.014, 0.022),
                        wood,
                        false,
                        light,
                    );
                    push_oriented_box(
                        out,
                        hand + hu * 0.22,
                        axes,
                        Vec3::new(0.03, 0.21, 0.01),
                        it.color,
                        glow,
                        light,
                    );
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
