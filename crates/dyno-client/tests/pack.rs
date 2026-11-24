//! Checks on the content pack in `content/` that the client depends on:
//! every block has its textures, every sound event its files (and every
//! event the client plays a description), every item icon is there, the
//! fonts and render settings load; and the client has no generators left
//! that paint textures or synthesise sounds in code.

use dyno_client::ambience::{AMBIENCE_JSON, AmbienceData, Env};
use dyno_client::audio::{AUDIO_JSON, Audio, AudioData, decode_wav};
use dyno_client::look::RenderSettings;
use dyno_client::music::{MUSIC_JSON, Music, MusicData};
use dyno_client::textures::{BlocksFile, TILE, decode_png};
use dyno_client::tracker::SongFile;
use dyno_world as w;
use std::path::{Path, PathBuf};

fn pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(pack().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn every_block_has_its_textures() {
    let file = BlocksFile::parse(&String::from_utf8(read("blocks.json")).unwrap()).unwrap();
    let mut listed = vec![false; w::BLOCKS.len()];
    for b in &file.blocks {
        assert!(
            (b.id as usize) < w::BLOCKS.len(),
            "blocks.json lists unknown block {}",
            b.id
        );
        assert!(!listed[b.id as usize], "block {} listed twice", b.id);
        listed[b.id as usize] = true;
        assert_eq!(b.key, w::block_key(b.id), "key of block {}", b.id);
        let Some(faces) = &b.textures else {
            assert_eq!(b.id, w::AIR, "block {} ({}) has no textures", b.id, b.key);
            continue;
        };
        for path in faces.all() {
            let (wd, ht, _) = decode_png(&read(path))
                .unwrap_or_else(|| panic!("{path} (block {}) is not a readable PNG", b.key));
            let frames = file
                .animated
                .get(path)
                .map(|a| a.frames as usize)
                .unwrap_or(1);
            assert_eq!(
                (wd, ht),
                (TILE, TILE * frames),
                "{path}: {TILE} px wide, {frames} frames tall"
            );
        }
    }
    for (id, l) in listed.iter().enumerate() {
        assert!(
            l,
            "block {id} ({}) is missing from blocks.json",
            w::block_key(id as u8)
        );
    }
    for (path, a) in &file.animated {
        assert!(a.frames > 1 && a.frame_time > 0.0, "{path}: animation");
    }
    let unknown = file.unknown.expect("a texture for unknown blocks");
    assert!(decode_png(&read(&unknown)).is_some());
}

/// The pack's description of its sounds.
fn audio_data() -> AudioData {
    AudioData::parse(&String::from_utf8(read(AUDIO_JSON)).unwrap()).unwrap()
}

/// The pack's description of the sounds of places (it streams).
fn ambience_data() -> AmbienceData {
    AmbienceData::parse(&String::from_utf8(read(AMBIENCE_JSON)).unwrap()).unwrap()
}

/// Every sound event of the pack: `audio.json`'s and the ambience file's.
fn all_events() -> std::collections::BTreeMap<String, dyno_client::audio::Event> {
    let mut all = audio_data().events;
    for (key, e) in ambience_data().events {
        assert!(
            all.insert(key.clone(), e).is_none(),
            "`{key}` is described twice"
        );
    }
    all
}

/// Every `.wav` under a directory of the pack, as pack paths.
fn wavs(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![pack().join(dir)];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "wav") {
                let rel = p.strip_prefix(pack()).unwrap();
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_sound_event_has_its_files() {
    let d = audio_data();
    let mut used = std::collections::HashSet::new();
    for (key, e) in &all_events() {
        assert!(e.variants >= 1, "{key}: no variants");
        // something heard again and again must not sound the same each time
        assert!(
            e.once || e.variants >= 3,
            "{key}: {} variants (three or more, or mark it `once`)",
            e.variants
        );
        assert!(e.gain > 0.0 && e.gain <= 1.0, "{key}: gain {}", e.gain);
        for path in e.paths() {
            let clip = decode_wav(&read(&path)).unwrap_or_else(|| panic!("{path}: not 16-bit PCM"));
            assert!(!clip.samples.is_empty(), "{path} is empty");
            let top = clip.samples.iter().fold(0.0f32, |a, b| a.max(b.abs()));
            assert!(top > 0.02, "{path} ({key}) is all but silent: {top}");
            assert!(top < 0.999, "{path} ({key}) is clipped");
            used.insert(path);
        }
    }
    // the sounds the rules name (an ability's `sound`): each has a first
    // variant, in the place and within the count `named` gives
    let named = d.named.as_ref().expect("how named sounds are played");
    let content = dyno_world::content::Content::default_pack();
    for a in content.abilities.iter().filter(|a| !a.sound.is_empty()) {
        for v in 0..named.variants {
            let path = format!("{}{}_{v}.wav", named.file, a.sound);
            if v > 0 && !pack().join(&path).exists() {
                break;
            }
            let clip = decode_wav(&read(&path))
                .unwrap_or_else(|| panic!("{path} (ability {}): not 16-bit PCM", a.key));
            assert!(!clip.samples.is_empty(), "{path} is empty");
            used.insert(path);
        }
    }
    // and no file is left that nothing plays
    for path in wavs("sounds") {
        assert!(
            used.contains(&path),
            "{path}: no event in {AUDIO_JSON} and no ability in the rules plays it"
        );
    }
}

/// The string literal that follows each `call("` in `text`.
fn literals_after<'a>(text: &'a str, call: &str) -> Vec<&'a str> {
    let pat = format!("{call}(\"");
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(&pat) {
        rest = &rest[i + pat.len()..];
        if let Some(end) = rest.find('"') {
            out.push(&rest[..end]);
        }
    }
    out
}

#[test]
fn every_event_the_client_plays_is_described() {
    let d = audio_data();
    // the keys written in the client's code
    let mut keys = Vec::new();
    for (path, text) in client_sources() {
        for call in [
            ".play",
            ".play_at",
            ".want",
            "audio::emit",
            "audio::emit_in",
        ] {
            for key in literals_after(&text, call) {
                // (audio.rs's own tests use a pack of their own)
                if !key.starts_with("test.") && !key.contains(' ') {
                    keys.push((path.clone(), key.to_string()));
                }
            }
        }
    }
    assert!(
        keys.len() > 30,
        "found only {} keys: has the code moved?",
        keys.len()
    );
    for (path, key) in &keys {
        assert!(
            d.events.contains_key(key),
            "{}: plays `{key}`, which {AUDIO_JSON} does not describe",
            path.display()
        );
    }
    // the keys made from a block's material (app/sound.rs)
    for b in w::BLOCKS.iter() {
        let m = format!("{:?}", b.material).to_lowercase();
        for what in ["break", "place", "dig"] {
            let key = format!("{what}.{m}");
            assert!(d.events.contains_key(&key), "no `{key}` in {AUDIO_JSON}");
        }
    }
    // footsteps: every block gives a step that exists, and no block is
    // named that the world does not have
    assert!(d.events.contains_key(&d.footsteps.wade), "the wading step");
    for id in 0..w::BLOCKS.len() {
        let key = w::block_key(id as u8);
        let step = d.footstep(&key);
        assert!(d.events.contains_key(step), "block {key}: no step `{step}`");
    }
    for key in d.footsteps.blocks.keys() {
        assert!(
            w::block_by_key(key).is_some(),
            "footsteps: no block `{key}`"
        );
    }
    // what the first half hour needs (docs/design/quality-bar.md, 11)
    let needed = [
        "step.grass",
        "step.dirt",
        "step.stone",
        "step.wood",
        "step.sand",
        "step.water",
        "step.snow",
        "ui.open",
        "ui.close",
        "ui.click",
        "ui.item_pick",
        "ui.item_drop",
        "ui.buy",
        "ui.sell",
        "ui.quest_accept",
        "ui.quest_complete",
        "ui.level_up",
        "ui.ability_new",
        "ui.loot",
        "ui.coin",
        "ui.error",
        "combat.swing",
        "combat.hit",
        "combat.crit",
        "combat.hurt",
        "combat.heal",
        "combat.death",
        "combat.kill",
    ];
    for key in needed {
        assert!(d.events.contains_key(key), "no `{key}` in {AUDIO_JSON}");
        assert!(
            keys.iter().any(|(_, k)| k == key) || key.starts_with("step."),
            "`{key}` is described but nothing plays it"
        );
    }
    // the grounds the valley has give the steps they should
    for (block, step) in [
        ("grass", "step.grass"),
        ("dirt", "step.dirt"),
        ("cobble", "step.stone"),
        ("planks", "step.wood"),
        ("sand", "step.sand"),
        ("snow", "step.snow"),
        ("water", "step.water"),
    ] {
        assert_eq!(d.footstep(block), step);
    }
}

#[test]
fn sounds_for_the_first_minute_come_with_the_join_and_the_rest_stream() {
    let d = audio_data();
    let pack: serde_json::Value = serde_json::from_slice(&read("pack.json")).unwrap();
    let streamed: Vec<&str> = pack["stream"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s.as_str())
        .collect();
    let streams = |path: &str| streamed.iter().any(|p| path.starts_with(p));
    let mut join = 0;
    for (key, e) in &d.events {
        let early = ["step.", "ui.", "combat."]
            .iter()
            .any(|p| key.starts_with(p));
        for path in e.paths() {
            assert_eq!(
                streams(&path),
                !early,
                "{path} ({key}): steps, interface and blows come with the join; the rest streams"
            );
            if early {
                join += read(&path).len();
            }
        }
    }
    let named = d.named.as_ref().unwrap();
    assert!(streams(&named.file), "{}: named sounds stream", named.file);
    // the sounds of places and all the music stream, their descriptions
    // with them: none of it is in the join
    for (key, e) in &ambience_data().events {
        for path in e.paths() {
            assert!(streams(&path), "{path} ({key}) streams");
        }
    }
    assert!(streams(AMBIENCE_JSON) && streams(MUSIC_JSON));
    for path in files_under("music") {
        assert!(streams(&path), "{path} streams");
    }
    // (before compression; the join itself is held in tests/streaming.rs)
    assert!(join < 2_300_000, "{join} bytes of sound in the join");
}

#[test]
fn each_event_keeps_to_the_level_of_its_bus() {
    // through the real mixer, sliders at full, one event at a time: its
    // peak stays under its bus's target
    dyno_client::assets::load_dir(&pack()).unwrap();
    let d = dyno_client::audio::data().unwrap();
    let mut audio = Audio::offline(22_050);
    for (key, e) in &all_events() {
        let bus = format!("{:?}", e.bus).to_lowercase();
        let target = d
            .targets
            .get(&bus)
            .unwrap_or_else(|| panic!("no target for {bus}"));
        let mut top = 0.0f32;
        // (enough plays to hear every variant; each rings out first)
        for _ in 0..e.variants as usize * 3 {
            audio.play(key, 1.0);
            let out = audio.render(4.0);
            top = out.iter().fold(top, |a, b| a.max(b.abs()));
        }
        assert!(top > 0.0, "{key} was not heard");
        let db = 20.0 * top.log10();
        assert!(
            db <= target.peak_db + 0.05,
            "{key}: peaks at {db:.1} dB, over the {bus} bus's {} dB",
            target.peak_db
        );
    }
    // the sounds the rules name keep to the effects bus's level too
    let target = d.targets["effects"];
    let content = dyno_world::content::Content::default_pack();
    let mut names: Vec<&str> = content
        .abilities
        .iter()
        .map(|a| a.sound.as_str())
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names.dedup();
    assert!(!names.is_empty());
    audio.listener(glam::Vec3::ZERO, glam::Vec3::X);
    for name in names {
        let mut top = 0.0f32;
        for _ in 0..6 {
            assert!(
                audio.play_named_at(name, glam::Vec3::ZERO, 1.0),
                "ability sound `{name}` did not play"
            );
            let out = audio.render(2.0);
            top = out.iter().fold(top, |a, b| a.max(b.abs()));
        }
        let db = 20.0 * top.log10();
        assert!(
            top > 0.0 && db <= target.peak_db + 0.05,
            "ability sound `{name}`: peaks at {db:.1} dB (effects: {} dB)",
            target.peak_db
        );
    }
    assert_eq!(audio.held(), 1.0, "no single event reaches the limiter");
}

/// Every file under a directory of the pack, as pack paths.
fn files_under(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![pack().join(dir)];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                dirs.push(p);
            } else {
                let rel = p.strip_prefix(pack()).unwrap();
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

/// The cameras of the visual gauntlet: name, where each stands (metres
/// from the hearth), the hour of its picture.
fn gauntlet() -> Vec<(String, [f32; 3], f32)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/concepts/cameras.json");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    json["cameras"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let at = |i: usize| c["stand"][i].as_f64().unwrap() as f32;
            (
                c["name"].as_str().unwrap().to_string(),
                [at(0), at(1), at(2)],
                c["time"].as_f64().unwrap() as f32,
            )
        })
        .collect()
}

#[test]
fn every_gauntlet_place_has_a_bed_at_every_hour_and_a_theme() {
    let d = ambience_data();
    let m = MusicData::parse(&String::from_utf8(read(MUSIC_JSON)).unwrap()).unwrap();
    let content = dyno_world::content::Content::default_pack();
    let places = gauntlet();
    assert_eq!(places.len(), 18);
    // sunrise, morning, noon, afternoon, sunset, dusk, midnight, the small
    // hours, and each place's own hour
    let hours = [0.0, 0.1, 0.25, 0.4, 0.5, 0.55, 0.75, 0.9];
    let mut table = Vec::new();
    for (name, at, own) in &places {
        // (the crypt is under ground: no sky there. The mine's camera
        // stands at the mine head, outside, and outside the rules' area
        // for the mine itself, which is checked on its own below)
        let skies: &[f32] = match name.as_str() {
            "barrow_crypt" => &[0.0, 1.0],
            _ => &[1.0],
        };
        let mut by_day = Vec::new();
        for hour in hours.iter().chain([own]) {
            for sky in skies {
                let env = Env {
                    content: &content,
                    hearth: Some([0, 0, 0]),
                    pos: glam::Vec3::from(*at),
                    time: *hour,
                    weather: (0, 0.0),
                    sky: *sky,
                };
                let beds: Vec<&str> = d
                    .beds
                    .iter()
                    .filter(|b| env.bed(b) * b.gain > 0.2)
                    .map(|b| b.sound.as_str())
                    .collect();
                assert!(
                    !beds.is_empty(),
                    "{name} at hour {hour} (sky {sky}): no bed is heard"
                );
                if (*hour == 0.25 || *hour == 0.75) && *sky == 1.0 && by_day.len() < 2 {
                    by_day.push(beds.join(" + "));
                }
            }
        }
        let env = Env {
            content: &content,
            hearth: Some([0, 0, 0]),
            pos: glam::Vec3::from(*at),
            time: *own,
            weather: (0, 0.0),
            sky: 1.0,
        };
        let theme = m.theme([at[0], at[2]], &env.areas()).to_string();
        assert!(m.tracks.contains_key(&theme), "{name}: no track `{theme}`");
        table.push(format!(
            "| {name} | {} | {} | {theme} |",
            by_day[0], by_day[1]
        ));
    }
    println!("| Place | Beds at noon | Beds at midnight | Theme |\n|---|---|---|---|");
    println!("{}", table.join("\n"));
    // the moods the valley is known by
    let theme_at = |place: &str| {
        let (_, at, time) = places.iter().find(|p| p.0 == place).unwrap();
        let env = Env {
            content: &content,
            hearth: Some([0, 0, 0]),
            pos: glam::Vec3::from(*at),
            time: *time,
            weather: (0, 0.0),
            sky: 1.0,
        };
        m.theme([at[0], at[2]], &env.areas()).to_string()
    };
    for (place, theme) in [
        ("hearthstead_plaza", "hearthstead"),
        ("logging_camp", "birchrise"),
        ("old_barrow", "barrowwood"),
        ("barrow_crypt", "barrowwood"),
        ("mirrormere", "mirrormere"),
        ("miners_rest", "mines"),
        ("hollow_mines", "mines"),
    ] {
        assert_eq!(theme_at(place), theme, "{place}");
    }
    // down the mine itself (the middle of its area, no sky, any hour):
    // its own bed and its own theme
    let mine = content
        .areas
        .iter()
        .find(|a| a.key == "hollow_mines")
        .unwrap();
    for hour in hours {
        let env = Env {
            content: &content,
            hearth: Some([0, 0, 0]),
            pos: glam::Vec3::new(mine.at[0] as f32, -30.0, mine.at[1] as f32),
            time: hour,
            weather: (0, 0.0),
            sky: 0.0,
        };
        let beds: Vec<&str> = d
            .beds
            .iter()
            .filter(|b| env.bed(b) * b.gain > 0.2)
            .map(|b| b.sound.as_str())
            .collect();
        assert_eq!(beds, ["amb.mine"], "down the mine at hour {hour}");
        assert_eq!(m.theme([env.pos.x, env.pos.z], &env.areas()), "mines");
    }
    // every rule names a sound that is described and areas and blocks
    // that exist
    let events = all_events();
    let area = |key: &String| content.areas.iter().any(|a| &a.key == key);
    let mut places_named = Vec::new();
    for b in &d.beds {
        assert!(events.contains_key(&b.sound), "bed: no `{}`", b.sound);
        places_named.push(&b.place);
    }
    for s in &d.scatter {
        assert!(events.contains_key(&s.sound), "scatter: no `{}`", s.sound);
        assert!(s.every[0] > 0.5 && s.every[1] >= s.every[0], "{}", s.sound);
        places_named.push(&s.place);
    }
    for e in &d.emitters {
        assert!(events.contains_key(&e.sound), "emitter: no `{}`", e.sound);
        assert!(
            w::block_by_key(&e.block).is_some(),
            "no block `{}`",
            e.block
        );
        if let Some(above) = &e.above {
            assert!(w::block_by_key(above).is_some(), "no block `{above}`");
        }
        places_named.push(&e.place);
    }
    for p in places_named {
        for key in p.areas.iter().chain(&p.not_areas) {
            assert!(area(key), "no area `{key}` in the rules");
        }
    }
    for key in m.areas.keys() {
        assert!(area(key), "music: no area `{key}` in the rules");
    }
    for s in &m.spots {
        assert!(
            m.tracks.contains_key(&s.theme),
            "music: no track `{}`",
            s.theme
        );
    }
}

#[test]
fn beds_and_loops_go_round_without_a_seam() {
    let d = ambience_data();
    let mut looped: Vec<&str> = d.beds.iter().map(|b| b.sound.as_str()).collect();
    looped.extend(
        d.emitters
            .iter()
            .filter(|e| e.every.is_none())
            .map(|e| e.sound.as_str()),
    );
    looped.sort();
    looped.dedup();
    assert!(looped.len() >= 8);
    for key in looped {
        let path = format!("{}_0.wav", d.events[key].file);
        let s = decode_wav(&read(&path)).unwrap().samples;
        // from the last sample back to the first is no further than the
        // sound steps anywhere else
        let inside = s.windows(2).fold(0.0f32, |a, w| a.max((w[1] - w[0]).abs()));
        let seam = (s[0] - s[s.len() - 1]).abs();
        assert!(
            seam <= inside,
            "{path}: a step of {seam} at the seam ({inside} inside)"
        );
        assert!(s.len() > 4 * 22_050, "{path}: too short to hide its round");
    }
}

#[test]
fn every_song_plays_on_instruments_the_pack_has() {
    dyno_client::assets::load_dir(&pack()).unwrap();
    let m = MusicData::parse(&String::from_utf8(read(MUSIC_JSON)).unwrap()).unwrap();
    for name in [&m.wilds, &m.title, &m.combat]
        .into_iter()
        .chain(m.areas.values())
        .chain(m.stingers.values())
    {
        assert!(m.tracks.contains_key(name), "no track `{name}`");
    }
    for moment in ["combat", "discovery"] {
        assert!(m.stingers.contains_key(moment), "no stinger for `{moment}`");
    }
    let mut used = std::collections::HashSet::new();
    used.insert(MUSIC_JSON.to_string());
    let mut audio = Audio::offline(22_050);
    let mut music = Music::default();
    for (name, t) in &m.tracks {
        let file = SongFile::parse(&String::from_utf8(read(&t.song)).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", t.song));
        used.insert(t.song.clone());
        for s in file.samples() {
            let clip = decode_wav(&read(s)).unwrap_or_else(|| panic!("{s}: not 16-bit PCM"));
            assert!(clip.samples.len() > 100, "{s}");
            used.insert(s.to_string());
        }
        // a theme goes round; a stinger ends
        let stinger = m.stingers.values().any(|s| s == name);
        assert_eq!(file.looped, !stinger, "{name}");
        assert!(file.seconds() > if stinger { 1.0 } else { 12.0 }, "{name}");
        // a held instrument's sample is one seamless loop
        for (inst, i) in &file.instruments {
            for z in i.zones.iter().filter(|_| i.looped) {
                let s = decode_wav(&read(&z.sample)).unwrap().samples;
                let inside = s.windows(2).fold(0.0f32, |a, w| a.max((w[1] - w[0]).abs()));
                let seam = (s[0] - s[s.len() - 1]).abs();
                assert!(seam <= inside, "{}: {inst}'s loop has a seam", z.sample);
            }
        }
        assert!(
            music.song(&mut audio, &t.song).is_some(),
            "{name} is ready to play"
        );
    }
    // and nothing under music/ is left that no song uses
    for path in files_under("music") {
        assert!(used.contains(&path), "{path}: no song uses it");
    }
}

#[test]
fn every_item_has_an_icon() {
    let c = dyno_world::content::Content::default_pack();
    for (i, it) in c.items.iter().enumerate() {
        if it.block.is_some() {
            continue;
        }
        let path = it
            .image
            .as_deref()
            .unwrap_or_else(|| panic!("item {} has no image", it.key));
        let (wd, ht, _) = decode_png(&read(path)).unwrap_or_else(|| panic!("{path}: not a PNG"));
        assert!(wd > 0 && ht > 0, "{path} (item {i})");
    }
}

#[test]
fn fonts_and_render_settings_load() {
    let fonts: serde_json::Value = serde_json::from_slice(&read("fonts/fonts.json")).unwrap();
    for (_, face) in fonts["faces"].as_object().unwrap() {
        let f = read(face["file"].as_str().unwrap());
        assert!(f.len() > 1000);
    }
    for family in ["proportional", "body", "body-bold", "title"] {
        assert!(fonts["families"][family].is_array(), "font family {family}");
    }
    let s = RenderSettings::parse(&String::from_utf8(read("render.json")).unwrap()).unwrap();
    assert!(s != RenderSettings::default());
}

/// Every source file of the client, as (path, text).
fn client_sources() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                out.push((p, text));
            }
        }
    }
    out
}

#[test]
fn no_texture_painting_or_sound_synthesis_left() {
    // the toolkit of the old generators (textures.rs, audio.rs), and the
    // fonts that were built into the binary
    let marks = [
        ["struct ", "Surf"].concat(),
        ["fn ", "item_sprite"].concat(),
        ["fn ", "build_blocks"].concat(),
        ["fn ", "noise_burst"].concat(),
        ["fn ", "synth("].concat(),
        ["include_bytes!(", "\"../assets"].concat(),
        [".ttf", "\")"].concat(),
    ];
    for (path, text) in client_sources() {
        for m in &marks {
            assert!(
                !text.contains(m.as_str()),
                "{} still has `{m}`",
                path.display()
            );
        }
    }
    // item icons are pictures from the pack, not shapes drawn per kind
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let item_ui = std::fs::read_to_string(src.join("item_ui.rs")).unwrap();
    for kind in ["Stick", "Lump", "Ingot", "Gem", "Dust", "Berries", "Meat"] {
        let m = format!("Icon::{kind}");
        assert!(!item_ui.contains(&m), "item_ui.rs still draws `{m}`");
    }
    let tex = std::fs::read_to_string(src.join("textures.rs")).unwrap();
    assert!(
        tex.lines().count() < 400,
        "textures.rs should only load textures, not paint them"
    );
}
