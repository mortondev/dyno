//! Scripted situations for the ear: what a player hears in a walk, an
//! errand, a quest, a fight, a level-up, and everything at once. Each is a
//! list of moments; the montage tool and the mixing test play them through
//! the real mixer (`Audio::offline`).

use dyno_client::audio::Audio;
use glam::Vec3;

/// A moment: seconds from the start, the event, where it is (None: in
/// your head, as the interface is), and its share of its own level.
pub type Moment = (f32, &'static str, Option<[f32; 3]>, f32);

pub struct Scene {
    pub name: &'static str,
    /// (for the listing the montage tool prints)
    #[allow(dead_code)]
    pub what: &'static str,
    pub secs: f32,
    pub moments: Vec<Moment>,
}

/// Steps at a run, from `t0`, `n` of them.
fn steps(out: &mut Vec<Moment>, key: &'static str, t0: f32, n: usize) {
    for i in 0..n {
        out.push((t0 + i as f32 * 0.4, key, None, 1.0));
    }
}

/// A foe at `at` trading blows with you from `t0` to `t1`: your swing,
/// the blow landing, their swing back.
fn duel(out: &mut Vec<Moment>, at: [f32; 3], t0: f32, t1: f32) {
    let mut t = t0;
    let mut i = 0;
    while t < t1 {
        out.push((t, "combat.swing", None, 1.0));
        out.push((t + 0.18, "combat.hit", Some(at), 1.0));
        out.push((t + 0.9, "combat.swing", Some(at), 0.8));
        if i % 3 == 1 {
            out.push((t + 1.05, "combat.hurt", None, 0.8));
        }
        t += 1.8;
        i += 1;
    }
}

pub fn all() -> Vec<Scene> {
    let ahead = [0.0, 0.0, -2.5];
    let right = [4.0, 0.0, -3.0];
    let left = [-5.0, 0.0, -1.0];

    // 1: the grounds of the valley, five running steps on each
    let mut walk = Vec::new();
    let grounds = [
        "step.grass",
        "step.dirt",
        "step.stone",
        "step.wood",
        "step.sand",
        "step.water",
        "step.snow",
    ];
    for (i, g) in grounds.iter().enumerate() {
        steps(&mut walk, g, 0.3 + i as f32 * 2.3, 5);
    }

    // 2: an errand in town: bags, a merchant, a refusal
    let errand = vec![
        (0.3, "ui.open", None, 1.0),
        (1.0, "ui.click", None, 1.0),
        (1.5, "ui.item_pick", None, 1.0),
        (2.1, "ui.item_drop", None, 1.0),
        (2.7, "ui.click", None, 1.0),
        (3.2, "ui.close", None, 1.0),
        (4.2, "ui.open", None, 1.0),
        (5.0, "ui.click", None, 1.0),
        (5.1, "ui.buy", None, 1.0),
        (6.0, "ui.click", None, 1.0),
        (6.1, "ui.buy", None, 1.0),
        (7.0, "ui.sell", None, 1.0),
        (7.9, "ui.sell", None, 1.0),
        (8.8, "ui.click", None, 1.0),
        (8.9, "ui.error", None, 1.0),
        (9.8, "ui.close", None, 1.0),
    ];

    // 3: a quest from taking it to handing it in
    let mut quest = vec![
        (0.3, "ui.open", None, 1.0),
        (1.2, "ui.click", None, 1.0),
        (1.3, "ui.quest_accept", None, 1.0),
        (2.6, "ui.close", None, 1.0),
        (5.6, "ui.item_pick", None, 1.0),
        (5.7, "ui.quest_step", None, 1.0),
        (6.6, "ui.item_pick", None, 1.0),
        (6.7, "ui.quest_step", None, 1.0),
        (7.6, "ui.item_pick", None, 1.0),
        (7.7, "ui.quest_ready", None, 1.0),
        (11.4, "ui.open", None, 1.0),
        (12.2, "ui.click", None, 1.0),
        (12.3, "ui.quest_complete", None, 1.0),
        (12.4, "ui.coin", None, 1.0),
        (14.4, "ui.close", None, 1.0),
    ];
    steps(&mut quest, "step.grass", 3.2, 6);
    steps(&mut quest, "step.dirt", 9.0, 5);

    // 4: a fight with one foe: blows, a crit, a spell, a heal, the kill
    let mut fight = Vec::new();
    duel(&mut fight, ahead, 0.4, 7.0);
    fight.extend([
        (2.4, "combat.crit", Some(ahead), 1.0),
        (4.0, "combat.bolt", Some(ahead), 1.0),
        (5.2, "combat.heal", None, 1.0),
        (6.2, "combat.burst", Some(ahead), 1.0),
        (7.5, "combat.swing", None, 1.0),
        (7.68, "combat.hit", Some(ahead), 1.0),
        (7.7, "combat.crit", Some(ahead), 1.0),
        (7.9, "combat.kill", Some(ahead), 1.0),
        (9.0, "ui.open", None, 1.0),
        (9.6, "ui.click", None, 1.0),
        (9.7, "ui.loot", None, 1.0),
        (9.8, "ui.coin", None, 1.0),
        (10.4, "ui.close", None, 1.0),
    ]);

    // 5: the last blows of a level, and the level
    let mut level = Vec::new();
    duel(&mut level, ahead, 0.3, 2.0);
    level.extend([
        (2.3, "combat.swing", None, 1.0),
        (2.48, "combat.hit", Some(ahead), 1.0),
        (2.7, "combat.kill", Some(ahead), 1.0),
        (3.0, "ui.coin", None, 1.0),
        (3.2, "ui.level_up", None, 1.0),
        (5.6, "ui.ability_new", None, 1.0),
    ]);

    // 6: everything at once: three foes, another player beside you, rain,
    // the interface, a level-up over it all, and at last your death
    let mut every = Vec::new();
    duel(&mut every, ahead, 0.2, 9.0);
    duel(&mut every, right, 0.5, 9.0);
    duel(&mut every, left, 0.9, 9.0);
    steps(&mut every, "step.stone", 0.1, 22);
    for i in 0..10 {
        let t = i as f32;
        every.push((t + 0.1, "amb.rain", None, 1.0));
        every.push((t + 0.6, "amb.wind", None, 1.0));
        every.push((t + 0.35, "combat.shoot", Some(left), 1.0));
        every.push((t + 0.5, "combat.bolt", Some(right), 1.0));
        every.push((t + 0.75, "ui.click", None, 1.0));
    }
    every.extend([
        (1.0, "combat.crit", Some(ahead), 1.0),
        (1.5, "combat.burst", Some(right), 1.0),
        (2.0, "ui.error", None, 1.0),
        (2.2, "combat.heal", None, 1.0),
        (3.0, "combat.crit", Some(right), 1.0),
        (3.1, "combat.kill", Some(right), 1.0),
        (3.2, "ui.coin", None, 1.0),
        (3.3, "ui.level_up", None, 1.0),
        (3.4, "ui.quest_complete", None, 1.0),
        (3.5, "combat.burst", Some(ahead), 1.0),
        (4.0, "world.dusk", None, 1.0),
        (4.5, "combat.crit", Some(left), 1.0),
        (5.0, "ui.loot", None, 1.0),
        (5.7, "ui.ability_new", None, 1.0),
        (6.0, "combat.burst", Some(left), 1.0),
        (6.1, "combat.crit", Some(ahead), 1.0),
        (7.0, "ui.open", None, 1.0),
        (7.4, "ui.sell", None, 1.0),
        (8.0, "combat.heal", None, 1.0),
        (9.2, "combat.hurt", None, 1.0),
        (9.3, "combat.death", None, 1.0),
    ]);

    let mut scenes = vec![
        Scene {
            name: "1_footsteps",
            what: "grass, dirt, stone, wood, sand, water, snow: five running steps on each",
            secs: 16.5,
            moments: walk,
        },
        Scene {
            name: "2_interface",
            what: "bags open, an item moved, closed; a merchant: two bought, two sold, a refusal",
            secs: 10.8,
            moments: errand,
        },
        Scene {
            name: "3_quest",
            what: "a quest taken, a run, three things gathered, the run back, handed in",
            secs: 16.0,
            moments: quest,
        },
        Scene {
            name: "4_fight",
            what: "one foe ahead: blows both ways, a crit, a bolt, a heal, a burst, the kill, the loot",
            secs: 11.5,
            moments: fight,
        },
        Scene {
            name: "5_level_up",
            what: "the last blows, the kill, coin, the level, the new ability",
            secs: 8.5,
            moments: level,
        },
        Scene {
            name: "6_everything",
            what: "the worst case: three foes, rain, the interface, a level-up, your death",
            secs: 13.0,
            moments: every,
        },
    ];
    for s in &mut scenes {
        s.moments.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    scenes
}

/// Play a scene through `audio` (offline): what was heard, left and right
/// interleaved. `extra` is called each slice with the time (to lay
/// something under the scene).
#[allow(dead_code)]
pub fn render(
    audio: &mut Audio,
    scene: &Scene,
    mut extra: impl FnMut(&mut Audio, f32),
) -> Vec<f32> {
    const SLICE: f32 = 0.01;
    audio.listener(Vec3::ZERO, Vec3::X);
    let mut out = Vec::new();
    let mut next = 0;
    let mut t = 0.0;
    while t < scene.secs {
        while next < scene.moments.len() && scene.moments[next].0 <= t {
            let (_, key, at, gain) = scene.moments[next];
            match at {
                Some(at) => audio.play_at(key, at.into(), gain),
                None => audio.play(key, gain),
            }
            next += 1;
        }
        extra(audio, t);
        out.extend(audio.render(SLICE));
        t += SLICE;
    }
    out
}

/// Peak, in dB below full scale.
pub fn peak_db(x: &[f32]) -> f32 {
    20.0 * x.iter().fold(1e-9f32, |a, b| a.max(b.abs())).log10()
}

/// How loud a passage is: the level (RMS, dB below full scale) of its
/// loudest 3 seconds, in 0.4 s windows. Not a broadcast loudness meter
/// (no ear weighting), but steady enough to hold a mix to.
pub fn loudness_db(x: &[f32], rate: u32) -> f32 {
    let win = (rate as usize * 2) * 4 / 10;
    let mut power: Vec<f32> = x
        .chunks(win.max(2))
        .map(|w| w.iter().map(|v| v * v).sum::<f32>() / w.len() as f32)
        .collect();
    power.sort_by(|a, b| b.total_cmp(a));
    let n = power.len().clamp(1, 8);
    let mean = power[..n.min(power.len())].iter().sum::<f32>() / n as f32;
    10.0 * mean.max(1e-12).log10()
}
