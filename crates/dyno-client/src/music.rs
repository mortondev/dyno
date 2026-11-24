//! Which music plays: a theme for where you are, the title's theme, a
//! layer of drums over it while you fight, and stingers for moments.
//! Which place has which theme is the pack's (`music/music.json`, which
//! streams with the songs and their instruments); this only follows the
//! player from one to the next, crossing between them.

use crate::assets;
use crate::audio::Audio;
use crate::tracker::{Song, SongFile};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/// The pack file. It streams: it is asked for when music first runs.
pub const MUSIC_JSON: &str = "music/music.json";

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Track {
    /// the song's file
    pub song: String,
    pub gain: f32,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            song: String::new(),
            gain: 1.0,
        }
    }
}

/// A theme round a point, where the rules have no area to hang it on.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Spot {
    /// (x, z) in metres from the spawn hearth
    pub at: [f32; 2],
    pub radius: f32,
    pub theme: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct MusicData {
    pub tracks: BTreeMap<String, Track>,
    /// the theme of each area, by the area's key in the rules
    pub areas: BTreeMap<String, String>,
    /// themes round points; where you stand in one, it comes first
    pub spots: Vec<Spot>,
    /// the theme where no area has one
    pub wilds: String,
    /// the theme of the title screen
    pub title: String,
    /// the layer played over the theme in a fight
    pub combat: String,
    /// short pieces played over everything, by the moment's name
    pub stingers: BTreeMap<String, String>,
    /// seconds to cross from one theme to the next
    pub crossfade: f32,
    /// the theme's level under the combat layer (1: unchanged)
    pub combat_dip: f32,
    /// seconds after the last blow before the combat layer goes
    pub combat_linger: f32,
}

impl Default for MusicData {
    fn default() -> Self {
        MusicData {
            tracks: BTreeMap::new(),
            areas: BTreeMap::new(),
            spots: Vec::new(),
            wilds: String::new(),
            title: String::new(),
            combat: String::new(),
            stingers: BTreeMap::new(),
            crossfade: 4.0,
            combat_dip: 0.45,
            combat_linger: 5.0,
        }
    }
}

impl MusicData {
    pub fn parse(text: &str) -> Result<MusicData, String> {
        serde_json::from_str(text).map_err(|e| format!("{MUSIC_JSON}: {e}"))
    }

    /// The theme of a place, given where it is from the hearth (x, z) and
    /// the areas it is in, smallest first: that of a spot it stands in,
    /// or of the smallest area that has one (a camp inside a wood has the
    /// wood's), or the wilds'.
    pub fn theme(&self, at: [f32; 2], areas: &[&str]) -> &str {
        let spot = self
            .spots
            .iter()
            .filter(|s| (s.at[0] - at[0]).hypot(s.at[1] - at[1]) <= s.radius)
            .min_by(|a, b| a.radius.total_cmp(&b.radius));
        if let Some(s) = spot {
            return &s.theme;
        }
        areas
            .iter()
            .find_map(|a| self.areas.get(*a))
            .unwrap_or(&self.wilds)
    }
}

/// The data in the pack now (asked for if it has not come).
pub fn data() -> Option<Arc<MusicData>> {
    type Cache = Option<(Arc<[u8]>, Arc<MusicData>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let Some(bytes) = assets::get(MUSIC_JSON) else {
        assets::request(MUSIC_JSON);
        return None;
    };
    let mut c = CACHE.lock().unwrap();
    if let Some((b, d)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return Some(d.clone());
    }
    let d = Arc::new(
        std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(MusicData::parse)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                MusicData::default()
            }),
    );
    *c = Some((bytes, d.clone()));
    Some(d)
}

/// Where the player is, as far as music cares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scene<'a> {
    /// the title screen and its menus
    Title,
    /// in the world: where you are from the hearth (x, z), the areas you
    /// are in, smallest first, and whether you fight
    World {
        at: [f32; 2],
        areas: &'a [&'a str],
        combat: bool,
    },
    /// nothing (loading)
    Silence,
}

// the mixer's decks: three for themes (two crossing, and one to spare for
// a third begun before the first has gone), the combat layer, a stinger
const THEMES: [usize; 3] = [0, 1, 4];
const COMBAT: usize = 2;
const STINGER: usize = 3;

#[derive(Default)]
pub struct Music {
    /// songs built so far, by file, with the bytes each was built from
    songs: HashMap<String, (Arc<[u8]>, Arc<Song>)>,
    /// the theme playing: its track's name and its deck
    theme: Option<(String, usize)>,
    /// the deck the next theme takes
    turn: usize,
    /// the combat layer is on; seconds since the last blow
    combat: bool,
    quiet: f32,
}

impl Music {
    /// A song ready to play. None (and asked for) while its file or any of
    /// its instruments has not arrived.
    pub fn song(&mut self, audio: &mut Audio, path: &str) -> Option<Arc<Song>> {
        let Some(bytes) = assets::get(path) else {
            assets::request(path);
            return None;
        };
        if let Some((from, song)) = self.songs.get(path)
            && Arc::ptr_eq(from, &bytes)
        {
            return Some(song.clone());
        }
        let file = match std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(SongFile::parse)
        {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{path}: {e}");
                return None;
            }
        };
        // (every sample is looked at, so every missing one is asked for)
        let mut clips = HashMap::new();
        for s in file.samples() {
            if let Some(c) = audio.sample(s) {
                clips.insert(s, c);
            }
        }
        let song = Arc::new(Song::build(&file, |s| clips.get(s).cloned())?);
        self.songs.insert(path.to_string(), (bytes, song.clone()));
        Some(song)
    }

    fn track(&mut self, audio: &mut Audio, d: &MusicData, name: &str) -> Option<(Arc<Song>, f32)> {
        let t = d.tracks.get(name)?;
        Some((self.song(audio, &t.song)?, t.gain))
    }

    /// The theme playing (its track's name).
    pub fn playing(&self) -> Option<&str> {
        self.theme.as_ref().map(|t| t.0.as_str())
    }

    /// Play a stinger over everything; the themes dip under it.
    pub fn stinger(&mut self, audio: &mut Audio, moment: &str) {
        let Some(d) = data() else { return };
        let Some(name) = d.stingers.get(moment) else {
            return;
        };
        if let Some((song, gain)) = self.track(audio, &d, name) {
            audio.deck(STINGER, song, gain, 0.0, true);
        }
    }

    /// Once a frame: follow the player.
    pub fn update(&mut self, audio: &mut Audio, scene: Scene, dt: f32) {
        let Some(d) = data() else { return };
        let (want, fighting) = match scene {
            Scene::Title => (d.title.as_str(), false),
            Scene::World { at, areas, combat } => (d.theme(at, areas), combat),
            Scene::Silence => ("", false),
        };
        // the theme: cross to the one wanted, once it is ready to play
        if self.playing() != Some(want).filter(|w| !w.is_empty()) {
            let dip = if self.combat { d.combat_dip } else { 1.0 };
            if want.is_empty() {
                if let Some((_, deck)) = self.theme.take() {
                    audio.deck_stop(deck, d.crossfade);
                }
            } else if let Some((song, gain)) = self.track(audio, &d, want) {
                // (the first theme of all comes in gently too, but sooner)
                let fade = if self.theme.is_some() {
                    d.crossfade
                } else {
                    d.crossfade * 0.5
                };
                if let Some((_, old)) = self.theme.take() {
                    audio.deck_stop(old, d.crossfade);
                }
                let deck = THEMES[self.turn % THEMES.len()];
                self.turn += 1;
                audio.deck(deck, song, gain * dip, fade, false);
                self.theme = Some((want.to_string(), deck));
            }
        }
        // the fight: drums over the theme, which steps back; a sting as it
        // starts; and the drums linger a little after the last blow
        if fighting {
            self.quiet = 0.0;
        } else {
            self.quiet += dt;
        }
        let theme_gain = |name: &str| d.tracks.get(name).map_or(1.0, |t| t.gain);
        if fighting && !self.combat {
            // (asked for the first time a fight starts; it joins the
            // next one if it has not come by then)
            if let Some((song, gain)) = self.track(audio, &d, &d.combat) {
                audio.deck(COMBAT, song, gain, 1.2, false);
                self.combat = true;
                if let Some((name, deck)) = &self.theme {
                    audio.deck_level(*deck, theme_gain(name) * d.combat_dip, 1.2);
                }
                self.stinger(audio, "combat");
            }
        } else if self.combat && self.quiet > d.combat_linger {
            audio.deck_stop(COMBAT, 3.0);
            self.combat = false;
            if let Some((name, deck)) = &self.theme {
                audio.deck_level(*deck, theme_gain(name), 3.0);
            }
        }
    }
}
