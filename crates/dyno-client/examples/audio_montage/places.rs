//! Scripted visits for the ear: a place of the visual gauntlet at an hour,
//! with its beds and voices, what stands near by, its theme, and what the
//! player does there. The montage tool and the mixing tests play them
//! through the real mixer with the real ambience and music (no world is
//! loaded: the fires and water near by are written into each scene).

use super::scenes::Moment;
use dyno_client::ambience::{self, Ambience, Env};
use dyno_client::audio::Audio;
use dyno_client::music::{Music, Scene};
use dyno_world::content::Content;
use glam::Vec3;
use std::collections::HashMap;
use std::path::Path;

/// Where and when the listener is, from some second of a scene on.
#[derive(Clone)]
pub struct Setting {
    /// a camera of the gauntlet (tools/concepts/cameras.json); "" is the
    /// title screen
    pub place: &'static str,
    /// time of day, 0 sunrise, 0.5 sunset
    pub time: f32,
    /// 1 under the sky, 0 indoors or underground
    pub sky: f32,
    /// rain, 0 to 1
    pub rain: f32,
    pub combat: bool,
    /// blocks that sound, by key, and where they are from the listener
    /// (x right, y up, z behind)
    pub near: Vec<(&'static str, [f32; 3])>,
    /// metres from the camera to where the listener really is
    pub moved: [f32; 3],
}

impl Setting {
    pub fn at(place: &'static str, time: f32) -> Setting {
        Setting {
            place,
            time,
            sky: 1.0,
            rain: 0.0,
            combat: false,
            near: Vec::new(),
            moved: [0.0; 3],
        }
    }
    pub fn moved(mut self, by: [f32; 3]) -> Setting {
        self.moved = by;
        self
    }
    pub fn near(mut self, block: &'static str, at: [f32; 3]) -> Setting {
        self.near.push((block, at));
        self
    }
    pub fn indoors(mut self) -> Setting {
        self.sky = 0.0;
        self
    }
    pub fn rain(mut self, rain: f32) -> Setting {
        self.rain = rain;
        self
    }
    pub fn fighting(mut self) -> Setting {
        self.combat = true;
        self
    }
}

pub struct Visit {
    pub name: &'static str,
    #[allow(dead_code)]
    pub what: &'static str,
    pub secs: f32,
    /// (from this second, the setting)
    pub settings: Vec<(f32, Setting)>,
    pub moments: Vec<Moment>,
    /// (at this second, a stinger by its moment's name)
    pub stingers: Vec<(f32, &'static str)>,
}

/// The gauntlet's places: where each camera stands, in metres from the
/// hearth, and the hour of its picture.
pub fn cameras() -> HashMap<String, ([f32; 3], f32)> {
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
                ([at(0), at(1), at(2)], c["time"].as_f64().unwrap() as f32),
            )
        })
        .collect()
}

fn steps(out: &mut Vec<Moment>, key: &'static str, t0: f32, n: usize) {
    for i in 0..n {
        out.push((t0 + i as f32 * 0.4, key, None, 1.0));
    }
}

pub fn all() -> Vec<Visit> {
    let ahead = Some([0.0, 0.0, -2.5]);

    // 7: Hearthstead's plaza soon after sunrise
    let mut town = Vec::new();
    steps(&mut town, "step.stone", 13.0, 8);
    let town = Visit {
        name: "7_town_morning",
        what: "Hearthstead's plaza in the morning: the town's murmur, birds, the hearth fire to the right, the smith's anvil off to the left, a few steps on the flagstones, the town's theme coming in",
        secs: 24.0,
        settings: vec![(
            0.0,
            Setting::at("hearthstead_plaza", 0.08)
                .near("hearth", [4.0, 0.0, -3.0])
                .near("anvil", [-12.0, 0.0, -6.0]),
        )],
        moments: town,
        stingers: Vec::new(),
    };

    // 8: the barrow at night
    let barrow = Visit {
        name: "8_barrowwood_night",
        what: "the old barrow at midnight: the wood's low moan, crickets and an owl, a brazier ahead, the barrow's theme (voices on a drone, a bell out of key)",
        secs: 26.0,
        settings: vec![(
            0.0,
            Setting::at("old_barrow", 0.75).near("brazier", [-2.0, 0.0, -5.0]),
        )],
        moments: Vec::new(),
        stingers: Vec::new(),
    };

    // 9: Mirrormere as the sun goes down
    let mut lake = Vec::new();
    steps(&mut lake, "step.sand", 6.0, 5);
    steps(&mut lake, "step.water", 9.0, 4);
    let shore = |time: f32| Setting::at("mirrormere", time).near("water", [5.0, -1.0, -4.0]);
    let lake = Visit {
        name: "9_lakeside_sunset",
        what: "Mirrormere's shore from late afternoon into dusk: the lake lapping to the right, gulls and then frogs, day's air giving way to the night's crickets, steps on sand and into the shallows, the lake's theme (slow harp)",
        secs: 26.0,
        settings: vec![(0.0, shore(0.455)), (8.0, shore(0.5)), (12.0, shore(0.54))],
        moments: lake,
        stingers: Vec::new(),
    };

    // 10: down the mine
    let mut mine = Vec::new();
    steps(&mut mine, "step.stone", 10.0, 6);
    let mine = Visit {
        name: "10_mines",
        what: "the Hollow Mines: the weight of the hill, drips with long echoes, a brazier behind and to the left, steps on stone, the mines' theme (picks, a low horn)",
        secs: 24.0,
        settings: vec![(
            0.0,
            // (the camera stands at the mine head; the galleries are
            // some 80 m on and below)
            Setting::at("hollow_mines", 0.3)
                .moved([41.5, -15.0, -70.5])
                .indoors()
                .near("brazier", [-3.0, 0.0, 3.0]),
        )],
        moments: mine,
        stingers: Vec::new(),
    };

    // 11: a fight on the east road, and the level it earns
    let mut fight: Vec<Moment> = Vec::new();
    let mut t = 6.0;
    while t < 13.5 {
        fight.push((t, "combat.swing", None, 1.0));
        fight.push((t + 0.18, "combat.hit", ahead, 1.0));
        fight.push((t + 0.9, "combat.swing", ahead, 0.8));
        t += 1.8;
    }
    fight.extend([
        (9.2, "combat.hurt", None, 0.8),
        (11.4, "combat.crit", ahead, 1.0),
        (13.7, "combat.kill", ahead, 1.0),
        (14.4, "ui.coin", None, 1.0),
        (15.0, "ui.level_up", None, 1.0),
    ]);
    let road = |fighting: bool| {
        let s = Setting::at("wrecked_caravan", 0.3);
        if fighting { s.fighting() } else { s }
    };
    let fight = Visit {
        name: "11_fight_ducking",
        what: "the east road by day, the valley's theme playing; at 6 s a fight starts (a sting, the theme steps back, drums come in over it); the kill at 13.7 s; at 15 s the level-up fanfare, under which the music dips; then the drums leave and the theme comes back",
        secs: 28.0,
        settings: vec![(0.0, road(false)), (6.0, road(true)), (13.8, road(false))],
        moments: fight,
        stingers: Vec::new(),
    };

    // 12: from the title screen into the world
    let mut arrive = Vec::new();
    arrive.push((10.0, "ui.join", None, 1.0));
    steps(&mut arrive, "step.stone", 15.0, 6);
    let arrive = Visit {
        name: "12_title_to_world",
        what: "the title theme (the valley's tune with horns), then at 10 s you arrive in Hearthstead's plaza: the title crosses into the town's theme over four seconds as the town's sounds come up; at 20 s a place is discovered (the discovery stinger, the theme dipping under it)",
        secs: 28.0,
        settings: vec![
            (0.0, Setting::at("", 0.0)),
            (
                10.0,
                Setting::at("hearthstead_plaza", 0.3).near("hearth", [4.0, 0.0, -3.0]),
            ),
        ],
        moments: arrive,
        stingers: vec![(20.0, "discovery")],
    };

    let mut visits = vec![town, barrow, lake, mine, fight, arrive];
    for v in &mut visits {
        v.moments.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    visits
}

/// The loudest the game can be in one place: the fight of
/// `scenes::all()`'s worst case, in the plaza in heavy rain, by the fire
/// and the anvil, with the theme, the combat drums and both stingers.
pub fn worst(fight: &super::scenes::Scene) -> Visit {
    let here = |fighting: bool| {
        let s = Setting::at("hearthstead_plaza", 0.3)
            .rain(1.0)
            .near("hearth", [3.0, 0.0, -2.0])
            .near("anvil", [-6.0, 0.0, -3.0]);
        if fighting { s.fighting() } else { s }
    };
    Visit {
        name: "worst",
        what: "everything at once",
        secs: fight.secs + 6.0,
        settings: vec![(0.0, here(false)), (6.0, here(true))],
        // (the fight starts once the theme and the beds are up)
        moments: fight
            .moments
            .iter()
            .map(|(t, k, at, g)| (t + 6.0, *k, *at, *g))
            .collect(),
        stingers: vec![(9.0, "discovery")],
    }
}

/// What a visit needs besides the mixer.
pub struct Stage {
    pub content: Content,
    pub cameras: HashMap<String, ([f32; 3], f32)>,
    pub ambience: Ambience,
    pub music: Music,
}

impl Stage {
    pub fn new() -> Stage {
        Stage {
            content: Content::default_pack(),
            cameras: cameras(),
            ambience: Ambience::default(),
            music: Music::default(),
        }
    }
}

/// Play a visit through `audio` (offline): left and right interleaved.
pub fn render(audio: &mut Audio, stage: &mut Stage, visit: &Visit) -> Vec<f32> {
    const SLICE: f32 = 0.01;
    audio.listener(Vec3::ZERO, Vec3::X);
    let d = ambience::data().expect("the ambience file");
    let mut out = Vec::new();
    let (mut moment, mut stinger) = (0, 0);
    let mut t = 0.0;
    while t < visit.secs {
        let s = &visit
            .settings
            .iter()
            .rev()
            .find(|(from, _)| *from <= t)
            .expect("a setting from the start")
            .1;
        if s.place.is_empty() {
            stage.ambience.hush(audio);
            stage.music.update(audio, Scene::Title, SLICE);
        } else {
            let (camera, _) = stage.cameras[s.place];
            let at = [
                camera[0] + s.moved[0],
                camera[1] + s.moved[1],
                camera[2] + s.moved[2],
            ];
            // the listener is the mixer's origin, facing -z; the place is
            // only where the rules are asked
            let env = Env {
                content: &stage.content,
                hearth: Some([0, 0, 0]),
                pos: Vec3::from(at),
                time: s.time,
                weather: if s.rain > 0.0 { (2, s.rain) } else { (0, 0.0) },
                sky: s.sky,
            };
            // (each rule hears the first block of its kind written near)
            let found: Vec<Option<Vec3>> = d
                .emitters
                .iter()
                .map(|e| {
                    s.near
                        .iter()
                        .find(|(block, _)| *block == e.block)
                        .map(|(_, rel)| Vec3::from(at) + Vec3::from(*rel))
                })
                .collect();
            audio.listener(Vec3::from(at), Vec3::X);
            stage.ambience.update(audio, &env, &found, SLICE);
            let areas = env.areas();
            let scene = Scene::World {
                at: [at[0], at[2]],
                areas: &areas,
                combat: s.combat,
            };
            stage.music.update(audio, scene, SLICE);
            audio.listener(Vec3::ZERO, Vec3::X);
        }
        while moment < visit.moments.len() && visit.moments[moment].0 <= t {
            let (_, key, at, gain) = visit.moments[moment];
            match at {
                Some(at) => audio.play_at(key, at.into(), gain),
                None => audio.play(key, gain),
            }
            moment += 1;
        }
        while stinger < visit.stingers.len() && visit.stingers[stinger].0 <= t {
            stage.music.stinger(audio, visit.stingers[stinger].1);
            stinger += 1;
        }
        out.extend(audio.render(SLICE));
        t += SLICE;
    }
    out
}
