//! The mix, heard through the real mixer with no device. The worst case
//! (a fight with three foes in the plaza in heavy rain, by the fire and
//! the anvil, the interface busy, a level-up, a death, with the theme, the
//! combat drums and a stinger playing) never clips, and each bus sits near
//! the level the pack sets for it. Themes cross from one to the next with
//! no click and no hole, and the music dips under a stinger and a fanfare.

#[path = "../examples/audio_montage/places.rs"]
mod places;
#[path = "../examples/audio_montage/scenes.rs"]
mod scenes;

use dyno_client::assets;
use dyno_client::audio::{self, Audio, Bus, Volumes};
use dyno_client::music::{self, Music, Scene};
use std::path::Path;

const RATE: u32 = 22_050;

fn load() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    assets::load_dir(&root).unwrap();
}

fn worst(audio: &mut Audio) -> Vec<f32> {
    let all = scenes::all();
    let fight = all.iter().find(|s| s.name == "6_everything").unwrap();
    places::render(audio, &mut places::Stage::new(), &places::worst(fight))
}

#[test]
fn the_worst_case_never_clips_and_each_bus_keeps_its_level() {
    load();
    let d = audio::data().unwrap();
    // sliders at full: the loudest the game can be
    let mut a = Audio::offline(RATE);
    let heard = worst(&mut a);
    let top = heard.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(
        top <= d.limiter.ceiling + 1e-4,
        "peak {top} over the ceiling {}",
        d.limiter.ceiling
    );
    let as_16_bit = |v: f32| (v * 32767.0).round() as i32;
    let clipped = heard
        .iter()
        .filter(|v| as_16_bit(**v).abs() >= 32767)
        .count();
    assert_eq!(clipped, 0, "clipped samples");
    assert!(top > 0.3, "and it is loud: {top}");
    // the limiter is a guard, not part of the sound: even here it takes
    // off little
    let off = (-20.0 * a.held().log10()).abs();
    assert!(off < 6.0, "the limiter took off {off:.1} dB");
    println!(
        "worst case, sliders full: peak {:.1} dBFS, loudness {:.1} dB, limiter took off {off:.1} dB at most, {clipped} clipped samples",
        scenes::peak_db(&heard),
        scenes::loudness_db(&heard, RATE)
    );

    // each bus alone, against what the pack says it should measure
    println!("| Bus | Peak (dBFS) | Loudness (dB) | Target |");
    println!("|---|---|---|---|");
    for bus in Bus::ALL {
        let name = format!("{bus:?}").to_lowercase();
        let target = d.targets[&name];
        let mut a = Audio::offline(RATE);
        a.set_volumes(Volumes::only(bus));
        let heard = worst(&mut a);
        let (peak, loud) = (scenes::peak_db(&heard), scenes::loudness_db(&heard, RATE));
        println!(
            "| {name} | {peak:.1} | {loud:.1} | {} |",
            target.loudness_db
        );
        // the busiest passage of a bus is within 4 dB of its target, and
        // never over full scale on its own
        assert!(
            (loud - target.loudness_db).abs() <= 4.0,
            "{name}: loudness {loud:.1} dB, target {}",
            target.loudness_db
        );
        assert!(peak < -0.9, "{name}: peak {peak:.1} dBFS");
    }

    // at the settings a new player has, nothing reaches the limiter
    let fresh = dyno_client::options::Sound::of(&dyno_client::settings::Settings::default());
    let mut a = Audio::offline(RATE);
    a.set_volumes(fresh.volumes());
    let heard = worst(&mut a);
    println!(
        "worst case, default sliders: peak {:.1} dBFS, loudness {:.1} dB",
        scenes::peak_db(&heard),
        scenes::loudness_db(&heard, RATE)
    );
    assert_eq!(a.held(), 1.0, "the limiter is idle at the default levels");

    // a slider at nothing silences its bus and only its bus
    let mut a = Audio::offline(RATE);
    a.set_volumes(Volumes {
        effects: 0.0,
        ambience: 0.0,
        music: 0.0,
        ..Volumes::FULL
    });
    let ui_only = worst(&mut a);
    let mut b = Audio::offline(RATE);
    b.set_volumes(Volumes::only(Bus::Ui));
    assert_eq!(ui_only, worst(&mut b));
}

#[test]
fn every_scene_plays_only_what_the_pack_describes() {
    load();
    let all = scenes::all();
    assert_eq!(all.len(), 6);
    for s in &all {
        for (t, key, _, _) in &s.moments {
            assert!(audio::event(key).is_some(), "{}: `{key}` at {t}", s.name);
            assert!(*t < s.secs, "{}: `{key}` after the end", s.name);
        }
    }
    let visits = places::all();
    assert_eq!(visits.len(), 6);
    let cameras = places::cameras();
    let m = music::data().unwrap();
    for v in &visits {
        for (_, s) in &v.settings {
            assert!(
                s.place.is_empty() || cameras.contains_key(s.place),
                "{}: no camera `{}`",
                v.name,
                s.place
            );
        }
        for (t, key, _, _) in &v.moments {
            assert!(audio::event(key).is_some(), "{}: `{key}` at {t}", v.name);
        }
        for (_, moment) in &v.stingers {
            assert!(m.stingers.contains_key(*moment), "{}: `{moment}`", v.name);
        }
    }
}

/// The left channel of the next `secs` of music alone.
fn left(audio: &mut Audio, music: &mut Music, areas: &[&str], combat: bool, secs: f32) -> Vec<f32> {
    let mut out = Vec::new();
    for _ in 0..(secs * 100.0) as usize {
        let scene = if areas == ["title"] {
            Scene::Title
        } else {
            Scene::World {
                at: [0.0, 0.0],
                areas,
                combat,
            }
        };
        music.update(audio, scene, 0.01);
        out.extend(audio.render(0.01).iter().step_by(2));
    }
    out
}

/// The largest jump from one sample to the next.
fn step(x: &[f32]) -> f32 {
    x.windows(2).fold(0.0f32, |a, w| a.max((w[1] - w[0]).abs()))
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

#[test]
fn themes_cross_without_a_click_or_a_hole() {
    load();
    let m = music::data().unwrap();
    // an area for each mood, and the title
    let moods = [
        ("title", "title"),
        ("hearthstead", "hearthstead"),
        ("birchrise", "birchrise"),
        ("barrowwood", "barrowwood"),
        ("mirrormere", "mirrormere"),
        ("hollow_mines", "mines"),
    ];
    for (area, theme) in moods {
        if area != "title" {
            assert_eq!(m.theme([0.0, 0.0], &[area]), theme);
        }
    }
    // each theme alone: how big a step its own notes make, and how loud
    // it is
    let mut alone = Vec::new();
    for (area, theme) in moods {
        let mut a = Audio::offline(RATE);
        a.set_volumes(Volumes::only(Bus::Music));
        let mut music = Music::default();
        let x = left(&mut a, &mut music, &[area], false, 16.0);
        assert_eq!(music.playing(), Some(theme));
        let body = &x[RATE as usize * 4..];
        assert!(
            rms(body) > 0.003,
            "{theme} is all but silent: {}",
            rms(body)
        );
        alone.push((step(body), rms(body)));
    }
    println!(
        "| From | To | Largest step in the crossing | Largest in either theme alone | Quietest half second (of the quieter theme) |"
    );
    println!("|---|---|---|---|---|");
    let mut worst_ratio = 0.0f32;
    for (i, (from, a_name)) in moods.iter().enumerate() {
        for (j, (to, b_name)) in moods.iter().enumerate() {
            if i == j {
                continue;
            }
            let mut a = Audio::offline(RATE);
            a.set_volumes(Volumes::only(Bus::Music));
            let mut music = Music::default();
            left(&mut a, &mut music, &[from], false, 8.0);
            // the crossing: the pack's crossfade and a second either side
            let x = left(&mut a, &mut music, &[to], false, m.crossfade + 2.0);
            assert_eq!(music.playing(), Some(*b_name));
            let own = alone[i].0.max(alone[j].0);
            let ratio = step(&x) / own;
            worst_ratio = worst_ratio.max(ratio);
            // no click: nothing jumps further than the themes' own notes do
            assert!(
                step(&x) <= own * 1.25 + 1e-4,
                "{a_name} to {b_name}: a step of {} (their own: {own})",
                step(&x)
            );
            // no hole: no half second of the crossing is much quieter than
            // the quieter theme
            let quiet = alone[i].1.min(alone[j].1);
            let low = x
                .chunks(RATE as usize / 2)
                .filter(|c| c.len() == RATE as usize / 2)
                .map(rms)
                .fold(f32::MAX, f32::min);
            println!(
                "| {a_name} | {b_name} | {:.4} | {own:.4} | {:.0}% |",
                step(&x),
                100.0 * low / quiet
            );
            assert!(
                low > quiet * 0.25,
                "{a_name} to {b_name}: a hole ({low} of {quiet})"
            );
            // and afterwards only the new theme is left playing
            left(&mut a, &mut music, &[to], false, 1.0);
            assert_eq!(
                a.held_sounds().1,
                1,
                "{a_name} to {b_name}: decks left playing"
            );
        }
    }
    println!(
        "largest step in any crossing: {:.0}% of the themes' own",
        worst_ratio * 100.0
    );
}

#[test]
fn the_music_dips_under_a_stinger_and_a_fanfare_and_under_the_drums() {
    load();
    let m = music::data().unwrap();
    let d = audio::data().unwrap();
    let dip = d.duck.unwrap();
    // the theme's level, a half second at a time, with something over it
    let levels = |over: &dyn Fn(&mut Audio, &mut Music)| {
        let run = |with: bool| {
            let mut a = Audio::offline(RATE);
            // (the music alone: the fanfare itself is on the interface's bus)
            a.set_volumes(Volumes::only(Bus::Music));
            let mut music = Music::default();
            let mut x = left(&mut a, &mut music, &["hearthstead"], false, 8.0);
            if with {
                over(&mut a, &mut music);
            }
            x.extend(left(&mut a, &mut music, &["hearthstead"], false, 9.0));
            x
        };
        (run(false), run(true))
    };
    let at = |x: &[f32], secs: f32| {
        let i = (secs * RATE as f32) as usize;
        rms(&x[i..i + RATE as usize / 2])
    };
    // a fanfare that is not music (the level-up cue): for its `duck`
    // seconds the theme is at the pack's dip, then it comes back
    let (plain, ducked) = levels(&|a, _| a.play("ui.level_up", 1.0));
    let ratio = at(&ducked, 9.0) / at(&plain, 9.0);
    println!(
        "under the level-up fanfare the theme is at {:.0}% (the pack asks {:.0}%); 6 s later {:.0}%",
        ratio * 100.0,
        dip * 100.0,
        100.0 * at(&ducked, 15.0) / at(&plain, 15.0)
    );
    assert!((ratio - dip).abs() < 0.08, "dipped to {ratio}, not {dip}");
    assert!(
        (at(&ducked, 15.0) / at(&plain, 15.0) - 1.0).abs() < 0.05,
        "and back"
    );
    // the dip itself is a glide: no step larger than the theme's own
    assert!(step(&ducked) <= step(&plain) * 1.25);
    // a stinger: it is heard, and the theme under it dips the same way
    // (measured after it has rung out, on the way back up)
    let (plain, stung) = levels(&|a, music| music.stinger(a, "discovery"));
    assert!(
        at(&stung, 9.0) > at(&plain, 9.0) * 0.5,
        "the stinger sounds"
    );
    assert_eq!(m.stingers["discovery"], "discovery");
    // a fight: the drums come in over the theme, which steps back
    let mut a = Audio::offline(RATE);
    a.set_volumes(Volumes::only(Bus::Music));
    let mut music = Music::default();
    left(&mut a, &mut music, &["hearthstead"], false, 6.0);
    assert_eq!(a.held_sounds().1, 1);
    left(&mut a, &mut music, &["hearthstead"], true, 1.0);
    assert_eq!(a.held_sounds().1, 3, "the theme, the drums and the sting");
    left(&mut a, &mut music, &["hearthstead"], true, 6.0);
    assert_eq!(a.held_sounds().1, 2, "the theme and the drums");
    // ...and leave a while after the last blow
    left(
        &mut a,
        &mut music,
        &["hearthstead"],
        false,
        m.combat_linger + 5.0,
    );
    assert_eq!(a.held_sounds().1, 1, "the theme alone again");
}
