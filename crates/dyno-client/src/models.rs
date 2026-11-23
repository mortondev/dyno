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
pub fn creature_step(
    model: dyno_world::content::Model,
    variant: u8,
    speed: f32,
    scale: f32,
) -> f32 {
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
    let found =
        figure::creature(&format!("{model:?}"), variant, color).and_then(|(name, look, held)| {
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
    let show = |out: &mut Vec<Vertex>,
                kind: &str,
                color: [f32; 3],
                quality: u8,
                glows: bool,
                frame: u8| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player_color;
    use dyno_world::content::Content;

    /// The local pack, loaded once (other tests may already have put
    /// files of their own in the store).
    fn pack() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
            crate::assets::load_dir(&dir).expect("the content pack");
        });
    }

    /// Every figure in the pack, with the looks and grips it is drawn with.
    fn figures() -> Vec<(String, Look, Grip)> {
        let mut list = Vec::new();
        for (name, weapon) in [
            ("unarmed", None),
            ("sword", Some(WeaponKind::Sword)),
            ("axe", Some(WeaponKind::Axe)),
            ("staff", Some(WeaponKind::Staff)),
            ("bow", Some(WeaponKind::Bow)),
        ] {
            for robe in [0, 1] {
                let mut l = Look::player(3, [0.6, 0.2, 0.2]);
                l.weapon = weapon.map(|k| (k, [0.6, 0.6, 0.65]));
                l.set("robe", robe);
                let g = grip(&l);
                list.push((format!("person {name} robe {robe}"), l, g));
            }
        }
        for c in &Content::default_pack().creatures {
            let (fig, look, held) = figure::creature(&format!("{:?}", c.model), c.variant, c.color)
                .unwrap_or_else(|| panic!("{}: no figure", c.key));
            let g = figure::grip_of(held.as_ref().map(|h| h.0.as_str()));
            list.push((fig, look, g));
        }
        list
    }

    /// The pack test: every figure's skeleton has a clip for every
    /// standard animation, and none of them is a silent copy of idle.
    #[test]
    fn every_figure_has_every_animation() {
        pack();
        let mut bad = Vec::new();
        for (name, look, g) in figures() {
            let fig_name = if name.starts_with("person ") {
                figure::player_figure().unwrap()
            } else {
                name.clone()
            };
            let fig =
                figure::figure(&fig_name).unwrap_or_else(|| panic!("{fig_name} does not load"));
            let set = fig
                .clips(&look, g)
                .unwrap_or_else(|| panic!("{name}: no clips"));
            for &a in Anim::ALL {
                let Some(c) = set.clip(a) else {
                    bad.push(format!("{name}: no clip for {a:?}"));
                    continue;
                };
                if a.one_shot() != c.duration.is_some() {
                    bad.push(format!("{name} {a:?}: one-shot or loop is wrong"));
                }
                if a == Anim::Idle {
                    continue;
                }
                let differs = [0.0f32, 0.2, 0.5, 0.9, 2.3].iter().any(|&t| {
                    let p = set.pose(a, t, 1.0 + t * 3.0, 0.3);
                    p.is_finite() && p.distance(&set.pose(Anim::Idle, t, 1.0 + t * 3.0, 0.3)) > 0.05
                });
                if !differs {
                    bad.push(format!("{name} does nothing for {a:?}"));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Geometry was made, every vertex is a number, and the figure stays in
    /// its box.
    fn check(
        out: &[Vertex],
        feet: Vec3,
        scale: f32,
        loose: bool,
        what: &str,
        bad: &mut Vec<String>,
    ) {
        if out.is_empty() {
            bad.push(format!("{what}: no geometry"));
            return;
        }
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in out {
            let p = Vec3::from(v.pos);
            if !p.is_finite() {
                bad.push(format!("{what}: a vertex is not a number"));
                return;
            }
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let (lo, hi) = ((lo - feet) / scale, (hi - feet) / scale);
        if lo.y < if loose { -0.5 } else { -0.13 } {
            bad.push(format!("{what}: sinks {:.2} blocks underground", -lo.y));
        }
        if hi.y > 3.0 {
            bad.push(format!("{what}: reaches {:.2} blocks up", hi.y));
        }
        for d in [lo.x, lo.z, hi.x, hi.z] {
            if d.abs() > 2.6 {
                bad.push(format!("{what}: reaches {d:.2} blocks out"));
            }
        }
    }

    /// When to sample an animation: through a one-shot and well past its
    /// end, or across several seconds of a loop.
    fn times(a: Anim) -> Vec<f32> {
        match duration(a) {
            Some(d) => vec![0.0, d * 0.3, d * 0.5, d * 0.8, d, d * 3.0],
            None => vec![0.0, 0.7, 2.3, 4.6, 7.9, 30.1],
        }
    }

    #[test]
    fn every_figure_performs_every_animation() {
        pack();
        set_view(None);
        let content = Content::default_pack();
        let feet = Vec3::new(100.0, 50.0, 100.0);
        let (mut out, mut bad) = (Vec::new(), Vec::new());
        for c in &content.creatures {
            for &a in Anim::ALL {
                for t in times(a) {
                    let mut an = Animator::still(a, 1.3 + t * 5.0);
                    out.clear();
                    push_creature(
                        &mut out, c.model, c.variant, feet, 0.4, c.color, 1.0, 0.0, c.scale,
                        &mut an, t,
                    );
                    check(
                        &out,
                        feet,
                        c.scale,
                        a == Anim::Swim,
                        &format!("{} {a:?} at {t}", c.key),
                        &mut bad,
                    );
                }
            }
        }
        for (name, look, _) in figures().into_iter().filter(|f| f.0.starts_with("person ")) {
            for &a in Anim::ALL {
                for t in times(a) {
                    let mut an = Animator::still(a, 1.3 + t * 5.0);
                    out.clear();
                    push_person_anim(&mut out, feet, 0.4, 0.1, &look, 1.0, &mut an, t);
                    check(
                        &out,
                        feet,
                        1.0,
                        a == Anim::Swim,
                        &format!("{name} {a:?} at {t}"),
                        &mut bad,
                    );
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Changing state halfway through anything stays well formed.
    #[test]
    fn crossfades_stay_well_formed() {
        pack();
        set_view(None);
        let feet = Vec3::new(10.0, 5.0, 10.0);
        let (mut out, mut bad) = (Vec::new(), Vec::new());
        for c in Content::default_pack().creatures.iter().take(12) {
            for &from in Anim::ALL {
                for &to in Anim::ALL.iter().step_by(3) {
                    let mut an = Animator::new(5, 0.0);
                    an.drive(0.0, from, 0.0, 1.0);
                    out.clear();
                    push_creature(
                        &mut out, c.model, c.variant, feet, 0.0, c.color, 1.0, 0.0, 1.0, &mut an,
                        0.6,
                    );
                    an.drive(0.7, to, 0.4, 1.0);
                    an.play(0.7, Anim::HitReact);
                    out.clear();
                    push_creature(
                        &mut out, c.model, c.variant, feet, 0.0, c.color, 1.0, 0.0, 1.0, &mut an,
                        0.8,
                    );
                    check(
                        &out,
                        feet,
                        1.0,
                        true,
                        &format!("{} {from:?} to {to:?}", c.key),
                        &mut bad,
                    );
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn looks_and_held_items_come_from_the_pack() {
        pack();
        let a = Look::from_appearance(&Default::default(), [0.5; 3]);
        assert!(a.color("skin").is_some() && !Look::options("skin").is_empty());
        let maren = Look::npc("maren", [0.4, 0.5, 0.6]);
        assert_eq!(maren.weapon.map(|w| w.0), Some(WeaponKind::Sword));
        assert_eq!(grip(&maren), Grip::OneHand);
        for c in &Content::default_pack().creatures {
            assert!(creature_attack(c.model, c.variant).one_shot(), "{}", c.key);
        }
        let mut out = Vec::new();
        let content = Content::default_pack();
        push_held(
            &mut out,
            &content,
            None,
            Vec3::ZERO,
            [Vec3::X, Vec3::Y, Vec3::Z],
            1.0,
        );
        assert!(!out.is_empty(), "the empty hand is drawn");
    }

    /// The best of several runs of 60 frames: 20 walking players and every
    /// creature four times, all animated.
    /// `cargo test -p dyno-client --release animated_crowd -- --nocapture`
    #[test]
    fn animated_crowd_cost() {
        pack();
        fn best(mut f: impl FnMut(f32)) -> f64 {
            let mut best = f64::MAX;
            for r in 0..7 {
                let start = std::time::Instant::now();
                for i in 0..60 {
                    f((r * 60 + i) as f32 * 0.016);
                }
                best = best.min(start.elapsed().as_secs_f64() * 1000.0 / 60.0);
            }
            best
        }
        let content = Content::default_pack();
        let looks: Vec<Look> = (0..20).map(|i| Look::player(i, player_color(i))).collect();
        let mut out = Vec::new();
        set_view(Some((Vec3::new(-2.0, 1.6, 3.0), Vec3::new(0.4, 0.8, 0.3))));
        let mut people: Vec<Animator> = (0..20).map(|i| Animator::new(i, 0.0)).collect();
        let p = best(|t| {
            out.clear();
            for (i, l) in looks.iter().enumerate() {
                let at = Vec3::new(i as f32 * 0.6, 0.0, (i % 3) as f32);
                people[i].drive(t, Anim::Walk, 0.07, 1.0);
                push_person_anim(&mut out, at, i as f32, 0.1, l, 0.8, &mut people[i], t);
            }
        });
        let pv = out.len();
        let n = content.creatures.len();
        let mut beasts: Vec<Animator> = (0..n * 4).map(|i| Animator::new(i as u32, 0.0)).collect();
        let c = best(|t| {
            out.clear();
            for k in 0..4 {
                for (i, c) in content.creatures.iter().enumerate() {
                    let at = Vec3::new(i as f32 * 1.5, 0.0, k as f32 * 2.0);
                    let a = &mut beasts[k * n + i];
                    a.drive(t, Anim::Walk, 0.05, 1.0);
                    push_creature(
                        &mut out, c.model, c.variant, at, i as f32, c.color, 0.8, 0.0, 1.0, a, t,
                    );
                }
            }
        });
        println!(
            "AB people: {p:.3} ms ({pv} verts); creatures: {c:.3} ms ({} verts)",
            out.len()
        );
        set_view(None);
    }

    /// Prints the cost of drawing 20 people, and of composing a new look.
    /// `cargo test -p dyno-client --release crowd_is_cheap -- --nocapture`
    #[test]
    fn crowd_is_cheap() {
        pack();
        let looks: Vec<Look> = ["brannoc", "maren", "tilda", "hollis"]
            .iter()
            .map(|k| Look::npc(k, [0.4, 0.5, 0.6]))
            .chain((0..16).map(|i| Look::player(i, player_color(i))))
            .collect();
        let start = std::time::Instant::now();
        let mut out = Vec::new();
        for (i, l) in looks.iter().enumerate() {
            push_person(
                &mut out,
                Vec3::new(i as f32, 0.0, 0.0),
                0.0,
                0.0,
                l,
                0.8,
                0.0,
                0.0,
            );
        }
        println!(
            "composing and meshing {} looks: {:.1} ms each",
            looks.len(),
            start.elapsed().as_secs_f64() * 1000.0 / looks.len() as f64
        );
        let start = std::time::Instant::now();
        for i in 0..20u32 {
            let l = Look::player(1000 + i, player_color(i));
            push_person(&mut out, Vec3::ZERO, 0.0, 0.0, &l, 0.8, 0.0, 0.0);
        }
        println!(
            "then each new look: {:.1} ms",
            start.elapsed().as_secs_f64() * 1000.0 / 20.0
        );
        for (name, view, spacing) in [
            ("no view, all full detail", false, 1.0f32),
            ("camera known, all within 14 blocks", true, 0.6),
            ("camera known, spread over 60 blocks", true, 3.0),
        ] {
            set_view(view.then_some((Vec3::new(-2.0, 1.6, 3.0), Vec3::new(0.4, 0.8, 0.3))));
            let draw = |out: &mut Vec<Vertex>, t: f32| {
                out.clear();
                for (i, l) in looks.iter().enumerate() {
                    let at = Vec3::new(i as f32 * spacing, 0.0, (i % 3) as f32);
                    push_person(out, at, i as f32, 0.1, l, 0.8, t, 1.0);
                }
            };
            draw(&mut out, 0.0);
            let n = 300;
            let start = std::time::Instant::now();
            for i in 0..n {
                draw(&mut out, i as f32 * 0.1);
            }
            let per = start.elapsed().as_secs_f64() * 1000.0 / n as f64;
            println!(
                "{} people, {name}: {} vertices, {per:.3} ms per frame",
                looks.len(),
                out.len()
            );
        }
        set_view(None);
        assert!(!out.is_empty());
    }
}
