//! Drawing characters, NPCs, creatures and held items. What they look like
//! and how they move is all content-pack data ([`crate::figure`]): a look
//! is composed once, meshed and cached, then posed by its clips each frame.
//! A figure whose files have not arrived is drawn as a plain stand-in.

use crate::anim::{Anim, Animator, Grip, Pose, key};
use crate::clip::ClipSet;
pub use crate::figure::Look;
use crate::figure::{self, Holding};
use crate::render::{Vertex, push_block_cube};
pub use crate::voxmodel::set_view;
use crate::voxmodel::{Paint, emit};
use dyno_world as w;
use dyno_world::content::WeaponKind;
use glam::{Affine3A, Vec3};
use std::rc::Rc;

/// The seed per-figure motion is offset by when a caller has no animator.
const STILL_SEED: f32 = 0.3;

/// The held kind of what a look holds in its right hand.
fn held_of(look: &Look) -> Option<String> {
    look.weapon.and_then(|(k, _)| figure::weapon_held(k))
}

/// How a person in `look` grips what they hold.
pub fn grip(look: &Look) -> Grip {
    figure::grip_of(held_of(look).as_deref())
}

fn holding_of(look: &Look) -> Option<Holding> {
    let (_, color) = look.weapon?;
    figure::holding(&held_of(look)?, color, 1, false, 0)
}

/// The player figure, its name and its clips for `look`.
fn person(look: &Look) -> Option<(String, Rc<figure::Figure>, Rc<ClipSet>)> {
    let name = figure::player_figure()?;
    let fig = figure::figure(&name)?;
    let set = fig.clips(look, grip(look))?;
    Some((name, fig, set))
}

/// How long a one-shot lasts for people (`None` for a state that loops).
pub fn duration(a: Anim) -> Option<f32> {
    let look = Look::default();
    person(&look).and_then(|(_, _, set)| set.duration(a))
}

/// Ground a person covers per step at `speed` blocks a second.
pub fn person_step(speed: f32) -> f32 {
    figure::player_figure()
        .and_then(|n| figure::figure(&n))
        .map(|f| f.step(speed, 1.0))
        .unwrap_or(1.0)
}

/// Ground a creature covers per step at `speed`, at size `scale`.
pub fn creature_step(model: dyno_world::content::Model, variant: u8, speed: f32, scale: f32) -> f32 {
    figure::creature(&format!("{model:?}"), variant, [0.5; 3])
        .and_then(|(name, _, _)| figure::figure(&name))
        .map(|f| f.step(speed, scale))
        .unwrap_or(scale.max(0.3))
}

/// The skeleton a creature is built on, by name.
pub fn creature_skeleton(model: dyno_world::content::Model, variant: u8) -> Option<String> {
    let (name, _, _) = figure::creature(&format!("{model:?}"), variant, [0.5; 3])?;
    figure::figure(&name)?.skeleton().map(str::to_string)
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
    push_person(
        out,
        feet,
        yaw,
        pitch,
        &Look::player(seed, color),
        light,
        phase,
        walk,
    );
}

/// A pose for callers with no animation state: idle, or walking at `phase`
/// by `walk` (0..1), `phase / 9` seconds in.
fn simple_pose(set: &ClipSet, walk: f32, phase: f32) -> Pose {
    let t = phase / 9.0;
    let idle = set.pose(Anim::Idle, t, phase, STILL_SEED);
    let mut p = if walk > 0.0 {
        idle.blend(
            &set.pose(Anim::Walk, t, phase, STILL_SEED),
            walk.min(1.0),
            u32::MAX,
        )
    } else {
        idle
    };
    p.k[key::CLOCK] = t;
    p
}

/// Draw a person in an outfit, idle or walking (`walk` 0 to 1, the stride
/// at `phase`). Figures with animation state use [`push_person_anim`].
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
    let Some((name, fig, set)) = person(look) else {
        figure::placeholder(out, feet, 1.0, light);
        return;
    };
    let pose = simple_pose(&set, walk, phase);
    let held = holding_of(look);
    figure::draw(
        out,
        &name,
        &fig,
        look,
        feet,
        yaw,
        pitch,
        Paint::lit(light),
        &pose,
        held.as_ref(),
        1.0,
    );
}

/// Draw a person posed by its animator at `now` (seconds).
#[allow(clippy::too_many_arguments)]
pub fn push_person_anim(
    out: &mut Vec<Vertex>,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    look: &Look,
    light: f32,
    animator: &mut Animator,
    now: f32,
) {
    let Some((name, fig, set)) = person(look) else {
        figure::placeholder(out, feet, 1.0, light);
        return;
    };
    let pose = animator.pose(now, &set);
    let held = holding_of(look);
    figure::draw(
        out,
        &name,
        &fig,
        look,
        feet,
        yaw,
        pitch,
        Paint::lit(light),
        &pose,
        held.as_ref(),
        1.0,
    );
}

/// The attack a creature makes: its weapon's for those that hold one, else
/// its natural attack (bite, gore, claw snap, rake, bolt).
pub fn creature_attack(model: dyno_world::content::Model, variant: u8) -> Anim {
    let held = figure::creature(&format!("{model:?}"), variant, [0.5; 3]).and_then(|c| c.2);
    match held {
        Some((kind, _)) => crate::anim::attack_for(figure::grip_of(Some(&kind))),
        None => Anim::AttackUnarmed,
    }
}

/// Draw a creature of a content model and variant, in its colour. `flash`
/// (0 to 1) whitens it when hit; `scale` is its size. It is posed by
/// `animator` at `now` (seconds).
#[allow(clippy::too_many_arguments)]
pub fn push_creature(
    out: &mut Vec<Vertex>,
    model: dyno_world::content::Model,
    variant: u8,
    feet: Vec3,
    yaw: f32,
    color: [f32; 3],
    light: f32,
    flash: f32,
    scale: f32,
    animator: &mut Animator,
    now: f32,
) {
    let paint = Paint {
        flash,
        ..Paint::lit(if flash > 0.0 { 1.0 } else { light })
    };
    let found = figure::creature(&format!("{model:?}"), variant, color).and_then(|(name, look, held)| {
        let fig = figure::figure(&name)?;
        let grip = figure::grip_of(held.as_ref().map(|h| h.0.as_str()));
        let set = fig.clips(&look, grip)?;
        Some((name, fig, look, held, set))
    });
    let Some((name, fig, look, held, set)) = found else {
        figure::placeholder(out, feet, scale, light);
        return;
    };
    let holding = held.and_then(|(k, c)| figure::holding(&k, c, 1, false, 0));
    let pose = animator.pose(now, &set);
    figure::draw(
        out,
        &name,
        &fig,
        &look,
        feet,
        yaw,
        0.0,
        paint,
        &pose,
        holding.as_ref(),
        scale,
    );
}

// ------------------------------------------------------------ held items

/// Which torch flame to show now.
fn flicker() -> u8 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let t = START.get_or_init(Instant::now).elapsed().as_secs_f32();
    ((t * 7.0) as u32 % 3) as u8
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
    let paint = Paint::lit(light);
    let Some(spec) = figure::held_spec() else {
        return;
    };
    // `fp`: size, then `tilt` leans the top away from the eye, `roll` leans
    // it in towards the middle of the view, `lift` raises it along the
    // hand's up
    let draw = |out: &mut Vec<Vertex>, m: &Holding, fp: [f32; 4]| {
        let [size, tilt, roll, lift] = fp;
        let (ts, tc) = tilt.sin_cos();
        let up = ax[1] * tc + ax[2] * ts;
        let fwd = ax[2] * tc - ax[1] * ts;
        let (rs, rc) = roll.sin_cos();
        let (right, up) = (ax[0] * rc + up * rs, up * rc - ax[0] * rs);
        let u = m.unit() * size;
        let xf = Affine3A::from_cols(
            (right * u).into(),
            (up * u).into(),
            (fwd * u).into(),
            (hand + ax[1] * lift).into(),
        );
        emit(out, m.mesh(), &xf, paint);
    };
    let show = |out: &mut Vec<Vertex>, kind: &str, color: [f32; 3], quality: u8, glows: bool, frame: u8| {
        if let (Some(k), Some(m)) = (
            figure::held_kind(kind),
            figure::holding(kind, color, quality, glows, frame),
        ) {
            draw(out, &m, k.fp);
        }
    };
    let Some(item) = item else {
        // a fist, reaching in from the corner
        if let Some((kind, color)) = &spec.hand {
            show(out, kind, *color, 1, false, 0);
        }
        return;
    };
    let it = c.item(item);
    if let Some(b) = it.block {
        if w::shape(b) == w::Shape::Post {
            if w::emission(b) > 0
                && let Some(torch) = &spec.torch
            {
                show(out, torch, it.color, 1, true, flicker());
            } else {
                crate::render::push_post_oriented(out, hand + ax[1] * 0.02, ax, 0.45, b, light);
            }
        } else {
            push_block_cube(out, hand, ax, 0.11, b, light);
        }
        return;
    }
    let glows = it.light > 0 || spec.glow_names.iter().any(|n| it.name.contains(n.as_str()));
    let weapon = it.gear.and_then(|g| g.weapon.as_ref()).map(|w| w.kind);
    let kind = if let Some(k) = weapon {
        spec.weapons.get(&format!("{k:?}"))
    } else if let Some(t) = it.tool {
        spec.tools.get(&format!("{:?}", t.kind))
    } else if let Some(g) = it.gear {
        spec.slots
            .get(&format!("{:?}", g.slot))
            .or_else(|| spec.slots.get("*"))
    } else {
        spec.icons.get(&format!("{:?}", it.icon))
    };
    if let Some(kind) = kind {
        show(out, kind, it.color, it.quality, glows, 0);
    }
}

/// The weapon an item is, with its colour, for `Look::weapon`. `None` for
/// anything that is not a main-hand weapon.
pub fn weapon_look(
    c: &dyno_world::content::Content,
    item: Option<dyno_world::content::ItemId>,
) -> Option<(WeaponKind, [f32; 3])> {
    let it = c.item(item?);
    let kind = match it.gear.and_then(|g| g.weapon.as_ref()) {
        Some(w) => w.kind,
        None if it
            .tool
            .is_some_and(|t| t.kind == dyno_world::content::ToolKind::Sword) =>
        {
            WeaponKind::Sword
        }
        None => return None,
    };
    (!matches!(kind, WeaponKind::Shield | WeaponKind::Offhand)).then_some((kind, it.color))
}
