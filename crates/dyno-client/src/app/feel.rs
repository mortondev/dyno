//! How blows feel in play (docs/design/feel-f2.md): what the combat
//! messages start (hit-stops, bursts, staggers, the camera's shake, tells,
//! kills) and what each frame draws of them. The numbers are the pack's
//! (`feel.json`, and `life.json`'s `impacts`); the state is `feel::State`.

use super::*;
use crate::feel::{self, Blow, Fallen, Landing};

/// The pack's impacts (none: no bursts).
fn impacts() -> Option<Arc<crate::life::LifeData>> {
    crate::life::data()
}

/// Whether the player lets the camera shake (Options).
fn shaking() -> bool {
    crate::options::interface().shake
}

/// The way from `from` to `to` on screen, as the camera looks: x to the
/// right, y up.
fn on_screen(p: &Play, from: Vec3, to: Vec3) -> [f32; 2] {
    let dir = p.cam_dir();
    let right = dir.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(dir).normalize_or_zero();
    let d = to - from;
    [d.dot(right), d.dot(up)]
}

/// A blow landed (`ServerMsg::Damage`): on a creature (`entity`), or on me.
pub(super) fn damage(p: &mut Play, entity: Option<u32>, amount: f32, crit: bool, swung: bool) {
    let d = feel::data();
    let now = p.anims.now();
    match entity {
        Some(id) => {
            let Some(e) = p.entities.get(&id) else {
                return;
            };
            let EntityKind::Creature(creature) = e.kind else {
                return;
            };
            let at = e.pos();
            let me = p.player.pos;
            let last_swing = p.anims.last_swing;
            let mut f = p.feel.borrow_mut();
            f.struck.insert(id, now);
            // a spell or shot of mine that went off at it just now
            let spell = f
                .spell
                .as_ref()
                .filter(|(_, to, t)| {
                    t.elapsed() < Duration::from_millis(500) && to.distance(at) < 3.5
                })
                .map(|(s, ..)| s.clone());
            let melee = spell.is_none() && at.distance(me) < 4.5 && now - last_swing < 0.6;
            // a bleed's or a burn's tick: no swing, no spell
            let tick = !melee && spell.is_none() && !swung;
            let when = if melee {
                (last_swing + d.hit_stop.contact).max(now)
            } else {
                now
            };
            f.landings.push(Landing {
                at: when,
                entity: id,
                creature,
                pos: at,
                along: (at - me).normalize_or_zero(),
                blow: if crit { Blow::Crit } else { Blow::Hit },
                crit,
                melee,
                tick,
                school: spell.unwrap_or_default(),
            });
        }
        None => {
            // a blow I took: I flinch for a moment; a heavy one kicks the
            // camera away from whoever struck it
            let mut f = p.feel.borrow_mut();
            let heavy_blow = f
                .heavy
                .filter(|(t, _)| t.elapsed() < Duration::from_millis(400));
            let heavy =
                heavy_blow.is_some() || amount >= d.shake.heavy_share * p.max_health.max(1.0);
            p.anims.me(now).freeze(now, d.hit_stop.taken);
            if heavy && shaking() {
                let from = heavy_blow.map(|(_, at)| at).or_else(|| {
                    p.entities
                        .values()
                        .filter(|e| matches!(e.kind, EntityKind::Creature(_)))
                        .map(|e| e.pos())
                        .filter(|q| q.distance(p.player.pos) < 4.0)
                        .min_by(|a, b| {
                            a.distance(p.player.pos)
                                .total_cmp(&b.distance(p.player.pos))
                        })
                });
                let dir = from
                    .map(|q| on_screen(p, q, p.player.pos))
                    .unwrap_or([0.0, -1.0]);
                f.camera.kick(d.shake.heavy, dir);
            }
        }
    }
}

/// A creature does something the feel cares for; true when that is all
/// there is to it (the caller plays nothing more).
pub(super) fn act(p: &mut Play, id: u32, act: &dyno_protocol::Act) -> bool {
    use dyno_protocol::Act;
    let now = p.anims.now();
    match *act {
        Act::WindUp { time } => {
            let strike = feel::data().tell.strike;
            let (a, _) = p.anims.entity(id, now);
            a.wind_up(now, Anim::Special, strike, time);
            p.feel.borrow_mut().windups.insert(id, (now, time));
            true
        }
        // the heavy blow comes down (its animation is the wind-up's, going on)
        Act::Special => {
            let mut f = p.feel.borrow_mut();
            if f.windups.remove(&id).is_none() {
                return false;
            }
            if let Some(e) = p.entities.get(&id) {
                f.heavy = Some((Instant::now(), e.pos()));
            }
            true
        }
        // broken off (a stun): it lets go of the blow
        Act::CastEnd { done: false } => {
            if p.feel.borrow_mut().windups.remove(&id).is_none() {
                return false;
            }
            let (a, _) = p.anims.entity(id, now);
            a.cancel_action(now);
            true
        }
        _ => false,
    }
}

/// A creature is gone (`ServerMsg::EntityGone`): if it fell, it pops, and
/// its figure stays and falls until its body comes.
pub(super) fn gone(p: &mut Play, id: u32) {
    let Some(e) = p.entities.get(&id) else {
        return;
    };
    let EntityKind::Creature(creature) = e.kind else {
        return;
    };
    let Some(c) = p.content.creature(creature) else {
        return;
    };
    let now = p.anims.now();
    let pos = p.ground.feet(&p.world, e.pos());
    let hurt = e
        .hurt_at
        .is_some_and(|t| t.elapsed() < Duration::from_secs(2));
    let mut f = p.feel.borrow_mut();
    let mine = f.struck.remove(&id).is_some();
    // it did not just wander off
    if !(mine || hurt) {
        return;
    }
    // my blow that is still to land on it is the killing one
    let mut pending = false;
    for l in f.landings.iter_mut().filter(|l| l.entity == id) {
        l.blow = Blow::Kill;
        pending = true;
    }
    if !pending {
        if let Some(life) = impacts() {
            let at = pos + Vec3::Y * 0.6 * c.scale;
            f.bits
                .burst(&life.impacts.kill, at, Vec3::ZERO, c.scale.max(0.5));
        }
        if mine {
            let (_, me) = feel::data().stop_of(true, Blow::Kill);
            p.anims.me(now).freeze(now, me);
        }
    }
    let mut anim = crate::anim::Animator::new(id, now);
    anim.drive_at(now, Anim::Death, pos, 1.0);
    f.fallen.push(Fallen {
        creature,
        pos,
        yaw: e.yaw,
        since: now,
        anim,
    });
}

/// The server refused a press: take back what it began to show.
pub(super) fn refused(p: &mut Play, ability: u16) {
    let now = p.anims.now();
    let pressed = p.feel.borrow_mut().pressed.take();
    if let Some(pr) = pressed.filter(|pr| pr.ability == ability) {
        if pr.animated {
            p.anims.me(now).cancel_action(now);
        }
    } else {
        p.feel.borrow_mut().pressed = pressed;
    }
    if p.my_cast.is_some_and(|c| c.0 == ability) {
        p.my_cast = None;
        p.anims.casts.remove(&p.my_id);
    }
    crate::audio::emit("ui.error");
}

/// What a press shows at once, before the server answers: how long until
/// the global cooldown (or a cast) is over, and whether the server will
/// keep it for then (`press.queue`), or refuse it.
pub(super) fn busy(p: &Play) -> (bool, bool) {
    let now = Instant::now();
    let gcd = p.gcd.0.saturating_duration_since(now).as_secs_f32();
    let cast = p
        .my_cast
        .map(|(_, at, secs)| (secs - at.elapsed().as_secs_f32()).max(0.0))
        .unwrap_or(0.0);
    let left = gcd.max(cast);
    let queue = feel::data().press.queue;
    (left > 0.0 && left <= queue, left > queue)
}

/// Light the pressed key.
pub(super) fn press(p: &mut Play, slot: usize, ability: u16, queued: bool, animated: bool) {
    p.feel.borrow_mut().pressed = Some(feel::Pressed {
        slot,
        ability,
        at: Instant::now(),
        queued,
        animated,
    });
}

/// A blow's contact has come: stop the figures, throw the burst, stagger
/// a small creature, kick the camera on a crit.
fn land(p: &Play, f: &mut feel::State, book: &mut Anims, l: &Landing, now: f32) {
    let d = feel::data();
    let Some(c) = p.content.creature(l.creature) else {
        return;
    };
    let pos = p.entities.get(&l.entity).map(|e| e.pos()).unwrap_or(l.pos);
    if !l.tick {
        let (target, me) = d.stop_of(l.melee, l.blow);
        if let Some(a) = book.entities.get_mut(&l.entity) {
            a.freeze(now, target);
        }
        if me > 0.0 {
            book.me(now).freeze(now, me);
        }
    }
    if let Some(life) = impacts() {
        let im = &life.impacts;
        // where it meets the target: its front, at its middle
        let at = pos + Vec3::Y * 0.9 * c.scale - l.along * 0.3 * c.scale;
        let more = match l.blow {
            _ if l.tick => 0.5,
            Blow::Hit => 1.0,
            Blow::Crit | Blow::Kill => im.crit.max(1.0),
        };
        if let Some(b) = im.material(&c.material) {
            f.bits.burst(b, at, l.along, more);
        }
        let own = if l.melee { "weapon" } else { l.school.as_str() };
        if !l.tick
            && let Some(b) = im.schools.get(own)
        {
            f.bits.burst(b, at, l.along, more);
        }
        if l.blow == Blow::Kill {
            let mid = p.ground.feet(&p.world, pos) + Vec3::Y * 0.6 * c.scale;
            f.bits.burst(&im.kill, mid, Vec3::ZERO, c.scale.max(0.5));
        }
    }
    if l.crit && !l.tick && c.scale <= d.stagger.scale {
        f.stagger.insert(
            l.entity,
            (
                now,
                Vec3::new(l.along.x, 0.0, l.along.z).normalize_or_zero(),
            ),
        );
    }
    // (your own crits only: a kill that is not one does not shake)
    if l.crit && !l.tick && shaking() {
        // thrown the way the blow goes, and down
        let [x, _] = on_screen(p, p.player.pos, pos);
        f.camera.kick(d.shake.crit, [x.signum() * 0.5, -1.0]);
    }
}

/// Each frame, before the figures are posed: the blows whose contact has
/// come land.
pub(super) fn before_figures(p: &Play, book: &mut Anims) {
    let now = book.now();
    let d = feel::data();
    let mut f = p.feel.borrow_mut();
    let due: Vec<Landing> = {
        let (due, later): (Vec<_>, Vec<_>) = f.landings.drain(..).partition(|l| l.at <= now);
        f.landings = later;
        due
    };
    for l in &due {
        land(p, &mut f, book, l, now);
    }
    f.prune(now, &d);
    // the queued pulse ends with the global cooldown
    if f.pressed.is_some_and(|pr| {
        let over = pr.at.elapsed().as_secs_f32();
        if pr.queued {
            over > 1.6 || (over > 0.05 && Instant::now() >= p.gcd.0)
        } else {
            over > d.press.pulse.max(0.0)
        }
    }) {
        f.pressed = None;
    }
}

/// The bursts in flight, and the figures of creatures that fell and left no
/// body (yet).
pub(super) fn draw(p: &Play, boxes: &mut Vec<Vertex>, book: &mut Anims, dt: f32) {
    let now = book.now();
    let mut f = p.feel.borrow_mut();
    let light = |v: Vec3| {
        p.light
            .get(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32) as f32
            / 15.0
    };
    f.bits.frame(dt, boxes, light);
    // a body has come where one fell: it is drawn instead
    f.fallen.retain(|fl| {
        !p.entities.values().any(|e| {
            matches!(e.kind, EntityKind::Corpse { creature, .. } if creature == fl.creature)
                && e.pos().distance(fl.pos) < 1.5
        })
    });
    for fl in &mut f.fallen {
        let Some(c) = p.content.creature(fl.creature) else {
            continue;
        };
        fl.anim.drive_at(now, Anim::Death, fl.pos, 1.0);
        let l = light(fl.pos + Vec3::Y * 0.5);
        models::push_creature(
            boxes,
            c.model,
            c.variant,
            fl.pos,
            fl.yaw,
            [c.color[0] * 0.75, c.color[1] * 0.75, c.color[2] * 0.75],
            l,
            0.0,
            c.scale,
            &mut fl.anim,
            now,
        );
    }
}

/// Where a creature is drawn (pushed off its place by a stagger) and how
/// white (the end of a wind-up).
pub(super) fn creature_look(p: &Play, id: u32, pos: Vec3, flash: f32, now: f32) -> (Vec3, f32) {
    let d = feel::data();
    let f = p.feel.borrow();
    let pos = pos + f.stagger_offset(&d.stagger, id, now);
    let flash = match f.winding(id, now) {
        Some(u) => flash.max(d.tell.flash * u * u),
        None => flash,
    };
    (pos, flash)
}

/// Marks round a creature: the ring of a heavy blow it winds up (at its
/// reach, filling from the middle as the blow comes), the stars of a stun.
#[allow(clippy::too_many_arguments)]
pub(super) fn creature_marks(
    p: &Play,
    boxes: &mut Vec<Vertex>,
    id: u32,
    c: &dyno_world::content::Creature,
    feet: Vec3,
    top: f32,
    now: f32,
    clock: f32,
) {
    let d = feel::data();
    let f = p.feel.borrow();
    if let Some(u) = f.winding(id, now) {
        let t = &d.tell;
        let reach = c.heavy.map(|h| h.reach).unwrap_or(t.reach);
        let cube = feel::on_ladder(t.cube);
        // (on the ground, a cube's height clear of it)
        let base = feet + Vec3::Y * 0.02;
        for (lo, hi) in feel::ring_boxes(base, reach, cube) {
            render::push_box(boxes, lo, hi, t.color, 1.0);
        }
        // the closing ring: snapped again to whole cubes as it grows
        let inner = (reach * u / cube).floor() * cube;
        if inner >= cube * 2.0 && inner < reach - cube {
            let dim = [t.color[0] * 0.8, t.color[1] * 0.8, t.color[2] * 0.8];
            for (lo, hi) in feel::ring_boxes(base, inner, cube) {
                render::push_box(boxes, lo, hi, dim, 1.0);
            }
        }
    }
    drop(f);
    // stunned: stars of cubes going round its head
    let stunned = p.entity_auras.get(&id).is_some_and(|list| {
        let now = Instant::now();
        list.iter()
            .any(|(a, until)| *until > now && p.content.ability(*a).is_some_and(|a| a.stun > 0.0))
    });
    if stunned && let Some(life) = impacts() {
        let z = &life.impacts.daze;
        let cube = feel::on_ladder(z.cube);
        let head = feet + Vec3::Y * (top + cube * 2.0);
        let r = z.radius * c.scale.max(0.6);
        for k in 0..z.cubes {
            let a = clock * z.turns * std::f32::consts::TAU
                + k as f32 / z.cubes.max(1) as f32 * std::f32::consts::TAU;
            let q = head + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            let h = cube * 0.5;
            render::push_box(boxes, q - Vec3::splat(h), q + Vec3::splat(h), z.color, 1.0);
        }
    }
}

/// A body that has just come: it holds its first pose for a killing
/// blow's stop, and its loot sparkles once a moment after it falls.
#[allow(clippy::too_many_arguments)]
pub(super) fn corpse(
    p: &Play,
    id: u32,
    a: &mut crate::anim::Animator,
    fresh: bool,
    new: bool,
    loot: bool,
    at: Vec3,
    now: f32,
) {
    let d = feel::data();
    let mut f = p.feel.borrow_mut();
    if new && fresh {
        a.freeze(now, d.hit_stop.kill);
    }
    if loot
        && a.base() == Anim::Death
        && a.base_time(now) >= d.kill.sparkle_after
        && a.base_time(now) < d.kill.sparkle_after + 0.5
        && f.sparkled.insert(id)
        && let Some(life) = impacts()
    {
        f.bits.burst(&life.impacts.loot, at, Vec3::ZERO, 1.0);
    }
    if f.sparkled.len() > 64 {
        let keep: HashSet<u32> = p.entities.keys().copied().collect();
        f.sparkled.retain(|i| keep.contains(i));
    }
}

/// The view, turned by the camera's shake (only the picture: where you
/// look and aim is not).
pub(super) fn view_dir(p: &Play, dir: Vec3) -> Vec3 {
    let d = feel::data();
    p.feel.borrow_mut().camera.turn(&d.shake, dir)
}
