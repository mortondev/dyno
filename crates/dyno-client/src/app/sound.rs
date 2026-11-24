//! Where the game meets its sounds: what the frame and the server's
//! messages mean for the ear. Which sound an event is, and how loud, is
//! the pack's (`audio.json`, audio.rs); this only says what happened.

use super::Play;
use crate::ambience::{self, Ambience, Env, Search};
use crate::audio::Audio;
use crate::music::{Music, Scene};
use dyno_protocol::{Act, Actor};
use dyno_world as w;
use glam::Vec3;

/// What the last frame looked like, to hear what changed.
#[derive(Default)]
pub struct Heard {
    /// which panels were open (a bit each)
    panels: u32,
    /// false until a frame in the world has been seen (joining opens
    /// nothing)
    known: bool,
    /// the sound of the place, the sounding blocks near by, the music
    ambience: Ambience,
    search: Search,
    music: Music,
    /// how much sky reaches the listener (it glides: a doorway is not a
    /// switch)
    sky: f32,
    /// the place the player was in last frame, and those found so far
    zone: String,
    found: std::collections::HashSet<String>,
}

/// The event of a block's material: `break.stone`, `place.wood`, `dig.earth`.
pub fn of_block(what: &str, b: w::BlockId) -> String {
    format!("{what}.{:?}", w::block(b).material).to_lowercase()
}

/// The panels open now, a bit each.
fn panels(p: &Play) -> u32 {
    [
        p.panel.is_some(),
        p.screen_tab.is_some(),
        p.picker_open,
        p.menu_open,
        p.loot.is_some(),
        p.trade.is_some(),
    ]
    .iter()
    .enumerate()
    .fold(0, |bits, (i, open)| bits | (*open as u32) << i)
}

/// Once a frame, after the interface is drawn.
pub fn frame(audio: &mut Audio, heard: &mut Heard, p: Option<&Play>, ctx: &egui::Context, dt: f32) {
    if let Some(s) = crate::options::sound() {
        audio.set_volumes(s.volumes());
    }
    match p {
        Some(p) => {
            let dir = p.cam_dir();
            audio.listener(p.player.eye(), dir.cross(Vec3::Y));
            // a panel opening or closing
            let now = panels(p);
            if heard.known {
                if now & !heard.panels != 0 {
                    audio.play("ui.open", 1.0);
                } else if heard.panels & !now != 0 {
                    audio.play("ui.close", 1.0);
                }
            }
            heard.panels = now;
            // the place: its beds and voices, what stands near, its theme
            let eye = p.player.eye();
            let cell = [
                eye.x.floor() as i32,
                eye.y.floor() as i32,
                eye.z.floor() as i32,
            ];
            let sky = p.light.sky(cell[0], cell[1], cell[2]) as f32 / 15.0;
            heard.sky = if heard.known {
                heard.sky + (sky - heard.sky) * (dt * 2.0).min(1.0)
            } else {
                sky
            };
            let env = Env {
                content: &p.content,
                hearth: p.hearth,
                pos: eye,
                time: (p.time + p.time_at.elapsed().as_secs_f32() / p.day_seconds).fract(),
                weather: (p.weather.0, p.weather_mix),
                sky: heard.sky,
            };
            if let Some(d) = ambience::data() {
                heard.search.step(&p.world, &d, eye);
            }
            heard.ambience.update(audio, &env, &heard.search.found, dt);
            // a fight: you are attacking, or you were just hurt with a
            // hostile creature near (a fall is not a fight)
            let foe_near = || {
                p.entities.values().any(|e| {
                    matches!(e.kind, super::EntityKind::Creature(c)
                        if p.content.creature(c).is_some_and(|c| c.hostile))
                        && e.pos().distance(eye) < 24.0
                })
            };
            let combat =
                p.dead.is_none() && (p.attacking.is_some() || (p.hurt_flash > 0.0 && foe_near()));
            let areas = env.areas();
            let h = p.hearth.unwrap_or([0; 3]);
            let scene = Scene::World {
                at: [eye.x - h[0] as f32, eye.z - h[2] as f32],
                areas: &areas,
                combat,
            };
            heard.music.update(audio, scene, dt);
            // a named place entered for the first time (the one you wake
            // in is not a discovery)
            if p.zone != heard.zone {
                heard.zone = p.zone.clone();
                // (nor the first place named after you arrive)
                let first = heard.found.is_empty();
                let new = !p.zone.is_empty() && heard.found.insert(p.zone.clone());
                if new && !first && heard.known {
                    heard.music.stinger(audio, "discovery");
                }
            }
            heard.known = true;
        }
        None => {
            if heard.known {
                heard.ambience.hush(audio);
                heard.found.clear();
                heard.zone.clear();
            }
            heard.known = false;
            heard.panels = 0;
            heard.music.update(audio, Scene::Title, dt);
        }
    }
    // a button, a tab, a slot: anything of the interface that was clicked
    if ctx.interaction_snapshot(|s| s.clicked.is_some()) && ctx.is_pointer_over_area() {
        audio.play("ui.click", 1.0);
    }
    audio.tick(dt);
}

/// Someone else (or something) swings or shoots: heard from where they are.
pub fn act(audio: &mut Audio, p: &Play, actor: &Actor, act: &Act) {
    if !matches!(act, Act::Swing | Act::Shoot) {
        return;
    }
    let at = match actor {
        Actor::Player(id) if *id != p.my_id => {
            p.remotes.get(id).map(|r| Vec3::from(r.current().pos))
        }
        Actor::Entity(id) => p.entities.get(id).map(|e| e.pos()),
        _ => None,
    };
    match (at, act) {
        (Some(at), Act::Swing) => audio.play_at("combat.swing", at, 0.8),
        (Some(at), _) => audio.play_at("combat.shoot", at, 0.8),
        _ => {}
    }
}

/// A blow of mine landed on `entity`.
pub fn struck(audio: &mut Audio, p: &Play, entity: Option<u32>, crit: bool, swing: bool) {
    if swing {
        audio.play("combat.swing", 1.0);
    }
    if crit && let Some(e) = entity.and_then(|e| p.entities.get(&e)) {
        audio.play_at("combat.crit", e.pos(), 1.0);
    }
}

/// My foot came down on `below`.
pub fn footstep(audio: &mut Audio, p: &Play, below: w::BlockId) {
    let feet = p.world.get(
        p.player.pos.x.floor() as i32,
        (p.player.pos.y + 0.1).floor() as i32,
        p.player.pos.z.floor() as i32,
    );
    audio.footstep(&w::block_key(below), feet == w::WATER, 1.0);
}
