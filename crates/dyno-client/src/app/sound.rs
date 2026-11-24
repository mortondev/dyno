//! Where the game meets its sounds: what the frame and the server's
//! messages mean for the ear. Which sound an event is, and how loud, is
//! the pack's (`audio.json`, audio.rs); this only says what happened.

use super::Play;
use crate::audio::Audio;
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
            heard.known = true;
        }
        None => *heard = Heard::default(),
    }
    // a button, a tab, a slot: anything of the interface that was clicked
    if ctx.interaction_snapshot(|s| s.clicked.is_some()) && ctx.is_pointer_over_area() {
        audio.play("ui.click", 1.0);
    }
    audio.tick(dt);
}

/// Someone else (or something) swings or shoots: heard from where they are.
pub fn act(audio: &mut Audio, p: &Play, actor: &Actor, act: &Act) {
    let key = match act {
        Act::Swing => "combat.swing",
        Act::Shoot => "combat.shoot",
        _ => return,
    };
    let at = match actor {
        Actor::Player(id) if *id != p.my_id => {
            p.remotes.get(id).map(|r| Vec3::from(r.current().pos))
        }
        Actor::Entity(id) => p.entities.get(id).map(|e| e.pos()),
        _ => None,
    };
    if let Some(at) = at {
        audio.play_at(key, at, 0.8);
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
