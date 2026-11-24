//! The mix, heard through the real mixer with no device: the worst case
//! (a fight with three foes, rain, the interface, a level-up, a death, and
//! a bed of music under it all) never clips, each bus sits near the level
//! the pack sets for it, and the sliders do what they say.

#[path = "../examples/audio_montage/scenes.rs"]
mod scenes;

use dyno_client::assets;
use dyno_client::audio::{self, AUDIO_JSON, Audio, Bus, Volumes};
use std::path::Path;

const RATE: u32 = 22_050;

/// The pack, plus a stand-in for music (there is none yet): a long horn
/// on the music bus, at the level music will have.
fn load() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    assets::load_dir(&root).unwrap();
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(AUDIO_JSON)).unwrap()).unwrap();
    json["events"]["review.music"] = serde_json::json!({
        "file": "sounds/world/horn", "variants": 4, "bus": "music", "gain": 0.55, "gap": 0.0
    });
    assets::insert(AUDIO_JSON, serde_json::to_vec(&json).unwrap());
}

/// The worst case with the music bed under it.
fn worst(audio: &mut Audio) -> Vec<f32> {
    let all = scenes::all();
    let scene = all.iter().find(|s| s.name == "6_everything").unwrap();
    let mut next = 0.0;
    scenes::render(audio, scene, |a, t| {
        if t >= next {
            a.play("review.music", 1.0);
            next += 0.7;
        }
    })
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
    assert!(
        heard.iter().all(|v| as_16_bit(*v).abs() < 32767),
        "a clipped sample"
    );
    assert!(top > 0.3, "and it is loud: {top}");
    // the limiter is a guard, not part of the sound: even here it takes
    // off little
    let off = -20.0 * a.held().log10();
    assert!(off < 6.0, "the limiter took off {off:.1} dB");
    println!(
        "worst case, sliders full: peak {:.1} dBFS, loudness {:.1} dB, limiter took off {off:.1} dB at most",
        scenes::peak_db(&heard),
        scenes::loudness_db(&heard, RATE)
    );

    // each bus alone, against what the pack says it should measure
    println!("| Bus | Peak (dBFS) | Target | Loudness (dB) | Target |");
    println!("|---|---|---|---|---|");
    for bus in Bus::ALL {
        let name = format!("{bus:?}").to_lowercase();
        let target = d.targets[&name];
        let mut a = Audio::offline(RATE);
        a.set_volumes(Volumes::only(bus));
        let heard = worst(&mut a);
        let (peak, loud) = (scenes::peak_db(&heard), scenes::loudness_db(&heard, RATE));
        println!(
            "| {name} | {peak:.1} | {} | {loud:.1} | {} |",
            target.peak_db, target.loudness_db
        );
        assert!(loud > -90.0, "{name}: silent");
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
    let d = audio::data().unwrap();
    let all = scenes::all();
    assert_eq!(all.len(), 6);
    for s in &all {
        for (t, key, _, _) in &s.moments {
            assert!(d.events.contains_key(*key), "{}: `{key}` at {t}", s.name);
            assert!(*t < s.secs, "{}: `{key}` after the end", s.name);
        }
    }
}
