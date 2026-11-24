//! Music as data: a song is a small file of patterns (which instrument
//! plays which note on which row) and its instruments are short samples
//! from the pack, pitched by playing them faster or slower, as a tracker
//! does. The engine only knows how to play a song ([`Deck`]); every tune
//! and every timbre is the pack's (`music/*.json`, `music/inst/*.wav`,
//! written by `tools/audio/compose.py` and `instruments.py`).
//!
//! A whole theme costs a few kilobytes, and its instruments are shared by
//! all the themes, which is why music is made this way here: the client
//! has no decoder for compressed sound and wants none (docs/design/audio.md).

use crate::audio::Clip;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Notes sounding at once on one deck; past this the oldest gives way.
const VOICES: usize = 32;

/// One recording of an instrument, and the note it was recorded at.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ZoneFile {
    pub sample: String,
    /// the note of the sample as recorded (MIDI numbers; 60 is middle C)
    pub root: f32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct InstrumentFile {
    /// the nearest recording to a note is the one played
    pub zones: Vec<ZoneFile>,
    /// the sample is one seamless loop, held as long as the note
    /// (strings, flutes); else it rings out by itself (plucks, drums)
    pub looped: bool,
    /// seconds to swell in, and to die away once the note is let go
    pub attack: f32,
    pub release: f32,
    pub gain: f32,
    /// -1 left .. 1 right
    pub pan: f32,
    /// how much of it goes to the room (the song's reverb)
    pub room: f32,
}

impl Default for InstrumentFile {
    fn default() -> Self {
        InstrumentFile {
            zones: Vec::new(),
            looped: false,
            attack: 0.004,
            release: 0.15,
            gain: 1.0,
            pan: 0.0,
            room: 1.0,
        }
    }
}

/// (row, instrument, note, rows it is held, how hard it is played 0..1)
pub type NoteFile = (f32, String, f32, f32, f32);

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct PatternFile {
    pub rows: u32,
    pub notes: Vec<NoteFile>,
}

/// A song as the pack writes it.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct SongFile {
    /// beats a minute, and rows to the beat
    pub bpm: f32,
    pub rows_per_beat: u32,
    pub gain: f32,
    /// how much room the song is played in, 0 to 1
    pub reverb: f32,
    /// it starts again when it ends (a theme; not a stinger)
    pub looped: bool,
    pub instruments: BTreeMap<String, InstrumentFile>,
    pub patterns: BTreeMap<String, PatternFile>,
    /// the patterns in the order they are played
    pub order: Vec<String>,
}

impl Default for SongFile {
    fn default() -> Self {
        SongFile {
            bpm: 90.0,
            rows_per_beat: 4,
            gain: 1.0,
            reverb: 0.25,
            looped: true,
            instruments: BTreeMap::new(),
            patterns: BTreeMap::new(),
            order: Vec::new(),
        }
    }
}

impl SongFile {
    pub fn parse(text: &str) -> Result<SongFile, String> {
        let s: SongFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.bpm <= 0.0 || s.rows_per_beat == 0 {
            return Err("a song needs a tempo".into());
        }
        for name in &s.order {
            if !s.patterns.contains_key(name) {
                return Err(format!("no pattern `{name}`"));
            }
        }
        for (name, p) in &s.patterns {
            for n in &p.notes {
                if !s.instruments.contains_key(&n.1) {
                    return Err(format!("pattern `{name}`: no instrument `{}`", n.1));
                }
                if n.0 < 0.0 || n.0 >= p.rows as f32 {
                    return Err(format!("pattern `{name}`: a note on row {}", n.0));
                }
            }
        }
        for (name, i) in &s.instruments {
            if i.zones.is_empty() {
                return Err(format!("instrument `{name}` has no sample"));
            }
        }
        Ok(s)
    }

    /// Every sample file the song needs.
    pub fn samples(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .instruments
            .values()
            .flat_map(|i| i.zones.iter().map(|z| z.sample.as_str()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Seconds from its first row to its last.
    pub fn seconds(&self) -> f32 {
        let rows: u32 = self.order.iter().map(|p| self.patterns[p].rows).sum();
        rows as f32 * 60.0 / (self.bpm * self.rows_per_beat as f32)
    }
}

struct Instrument {
    zones: Vec<(Arc<Clip>, f32)>,
    looped: bool,
    attack: f32,
    release: f32,
    /// level on the left and right
    gain: [f32; 2],
    room: f32,
}

struct Note {
    /// seconds from the start of the song
    at: f32,
    instrument: u16,
    note: f32,
    held: f32,
    level: f32,
}

/// A song ready to play: its notes laid end to end, its samples decoded.
pub struct Song {
    instruments: Vec<Instrument>,
    notes: Vec<Note>,
    seconds: f32,
    looped: bool,
    pub gain: f32,
    pub reverb: f32,
}

impl Song {
    /// None while a sample is missing (`clip` says what has arrived).
    pub fn build(file: &SongFile, mut clip: impl FnMut(&str) -> Option<Arc<Clip>>) -> Option<Song> {
        let mut index = BTreeMap::new();
        let mut instruments = Vec::new();
        for (name, i) in &file.instruments {
            let zones = i
                .zones
                .iter()
                .map(|z| Some((clip(&z.sample)?, z.root)))
                .collect::<Option<Vec<_>>>()?;
            if zones.iter().any(|(c, _)| c.samples.len() < 2) {
                return None;
            }
            let angle = (i.pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
            index.insert(name.as_str(), instruments.len() as u16);
            instruments.push(Instrument {
                zones,
                looped: i.looped,
                attack: i.attack.max(0.001),
                release: i.release.max(0.005),
                gain: [i.gain * angle.cos(), i.gain * angle.sin()],
                room: i.room,
            });
        }
        let row = 60.0 / (file.bpm * file.rows_per_beat as f32);
        let mut notes = Vec::new();
        let mut start = 0.0;
        for name in &file.order {
            let p = &file.patterns[name];
            for (r, inst, note, held, level) in &p.notes {
                notes.push(Note {
                    at: start + r * row,
                    instrument: *index.get(inst.as_str())?,
                    note: *note,
                    held: held.max(0.05) * row,
                    level: level.clamp(0.0, 1.0),
                });
            }
            start += p.rows as f32 * row;
        }
        notes.sort_by(|a, b| a.at.total_cmp(&b.at));
        Some(Song {
            instruments,
            notes,
            seconds: start,
            looped: file.looped,
            gain: file.gain,
            reverb: file.reverb,
        })
    }

    pub fn seconds(&self) -> f32 {
        self.seconds
    }
}

struct Voice {
    clip: Arc<Clip>,
    looped: bool,
    at: f64,
    step: f64,
    gain: [f32; 2],
    room: f32,
    /// the envelope: its level now, how fast it swells, how many frames
    /// until the note is let go, and how fast it then dies away
    level: f32,
    swell: f32,
    held: i64,
    fall: f32,
}

/// One song playing.
pub struct Deck {
    song: Arc<Song>,
    /// played over the rest (a stinger): the mixer dips the other decks
    /// under it
    pub over: bool,
    /// seconds into the song, and the next note to start
    clock: f64,
    next: usize,
    voices: Vec<Voice>,
    /// the deck's level now, where it is going, and how far it moves a frame
    level: f32,
    level_to: f32,
    level_step: f32,
    /// it ends when its level reaches nothing
    stopping: bool,
}

impl Deck {
    /// Start a song, its level rising from nothing to `level` over `fade`
    /// seconds.
    pub fn new(song: Arc<Song>, level: f32, fade: f32, over: bool, rate: u32) -> Deck {
        let mut d = Deck {
            song,
            over,
            clock: 0.0,
            next: 0,
            voices: Vec::with_capacity(VOICES),
            level: 0.0,
            level_to: 0.0,
            level_step: 0.0,
            stopping: false,
        };
        d.fade_to(level, fade, rate);
        if fade <= 0.0 {
            d.level = level;
        }
        d
    }

    /// Glide to `level` over `secs`.
    pub fn fade_to(&mut self, level: f32, secs: f32, rate: u32) {
        self.level_to = level.max(0.0);
        // (never at once: a step in level is a click)
        let frames = (secs.max(0.02) * rate as f32).max(1.0);
        self.level_step = (self.level_to - self.level).abs() / frames;
    }

    /// Fade out over `secs`, then end.
    pub fn stop(&mut self, secs: f32, rate: u32) {
        self.fade_to(0.0, secs, rate);
        self.stopping = true;
    }

    pub fn finished(&self) -> bool {
        // (a stinger is over when its last note has died away, not at
        // the end of its last bar: the music waits under it no longer)
        (self.stopping && self.level <= 0.0)
            || (!self.song.looped && self.next >= self.song.notes.len() && self.voices.is_empty())
    }

    /// The same song (so starting it again would change nothing).
    pub fn plays(&self, song: &Arc<Song>) -> bool {
        Arc::ptr_eq(&self.song, song) && !self.stopping
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    fn start(&mut self, n: usize, rate: u32) {
        let note = &self.song.notes[n];
        let inst = &self.song.instruments[note.instrument as usize];
        // the recording nearest the note
        let Some((clip, root)) = inst
            .zones
            .iter()
            .min_by(|a, b| (a.1 - note.note).abs().total_cmp(&(b.1 - note.note).abs()))
        else {
            return;
        };
        let pitch = 2f64.powf(((note.note - root) / 12.0) as f64);
        let v = Voice {
            clip: clip.clone(),
            looped: inst.looped,
            at: 0.0,
            step: clip.rate as f64 / rate as f64 * pitch,
            gain: [inst.gain[0] * note.level, inst.gain[1] * note.level],
            room: inst.room,
            level: 0.0,
            swell: 1.0 / (inst.attack * rate as f32),
            held: (note.held * rate as f32) as i64,
            fall: 1.0 / (inst.release * rate as f32),
        };
        if self.voices.len() == VOICES {
            self.voices.remove(0);
        }
        self.voices.push(v);
    }

    /// Start the notes that begin before `until` seconds.
    fn begin(&mut self, until: f64, rate: u32) {
        while self.next < self.song.notes.len() && (self.song.notes[self.next].at as f64) < until {
            self.start(self.next, rate);
            self.next += 1;
        }
    }

    /// Add the next frames to `out` (left, right…) and what goes to the
    /// room to `room` (one value a frame), both scaled by `bus`.
    pub fn render(&mut self, rate: u32, bus: f32, out: &mut [f32], room: &mut [f32]) {
        let frames = out.len() / 2;
        // the notes that begin in this stretch (to the nearest stretch:
        // a millisecond or so)
        let span = frames as f64 / rate as f64;
        self.begin(self.clock + span, rate);
        if self.song.looped
            && self.song.seconds > 0.0
            && self.clock + span >= self.song.seconds as f64
        {
            // round again
            self.clock -= self.song.seconds as f64;
            self.next = 0;
            self.begin(self.clock + span, rate);
        }
        self.clock += span;
        let song_gain = self.song.gain * bus;
        let reverb = self.song.reverb;
        for f in 0..frames {
            // the deck's own level glides
            if self.level < self.level_to {
                self.level = (self.level + self.level_step).min(self.level_to);
            } else if self.level > self.level_to {
                self.level = (self.level - self.level_step).max(self.level_to);
            }
            let g = self.level * song_gain;
            let (mut l, mut r, mut w) = (0.0f32, 0.0f32, 0.0f32);
            for v in &mut self.voices {
                let s = &v.clip.samples;
                let n = s.len();
                let i = v.at as usize;
                let x = if v.looped {
                    let i = i.min(n - 1);
                    let t = (v.at - i as f64) as f32;
                    s[i] + (s[(i + 1) % n] - s[i]) * t
                } else if i + 1 < n {
                    let t = (v.at - i as f64) as f32;
                    s[i] + (s[i + 1] - s[i]) * t
                } else {
                    v.level = -1.0;
                    0.0
                };
                if v.held > 0 {
                    v.held -= 1;
                    v.level = (v.level + v.swell).min(1.0);
                } else if v.level >= 0.0 {
                    v.level -= v.fall;
                    if v.level <= 0.0 {
                        v.level = -1.0;
                    }
                }
                let e = v.level.max(0.0);
                l += x * v.gain[0] * e;
                r += x * v.gain[1] * e;
                w += x * (v.gain[0] + v.gain[1]) * 0.5 * e * v.room;
                v.at += v.step;
                if v.looped && v.at >= n as f64 {
                    v.at -= n as f64;
                }
            }
            out[f * 2] += l * g;
            out[f * 2 + 1] += r * g;
            room[f] += w * g * reverb;
        }
        self.voices.retain(|v| v.level >= 0.0);
    }
}

/// A small room: four combs and two all-passes, as large halls have been
/// faked since the 1960s. One channel in, one out.
pub struct Room {
    combs: Vec<(Vec<f32>, usize, f32)>,
    passes: Vec<(Vec<f32>, usize)>,
}

impl Room {
    pub fn new(rate: u32) -> Room {
        let scale = |n: usize| (n * rate as usize / 44_100).max(8);
        Room {
            combs: [1557, 1617, 1491, 1422]
                .iter()
                .map(|n| (vec![0.0; scale(*n)], 0, 0.0))
                .collect(),
            passes: [225, 556]
                .iter()
                .map(|n| (vec![0.0; scale(*n)], 0))
                .collect(),
        }
    }

    /// Replace each value of `x` with the room's answer to it.
    pub fn render(&mut self, x: &mut [f32]) {
        const FEEDBACK: f32 = 0.82;
        const DAMP: f32 = 0.3;
        for v in x.iter_mut() {
            let dry = *v;
            let mut wet = 0.0;
            for (buf, at, store) in &mut self.combs {
                let o = buf[*at];
                *store = o * (1.0 - DAMP) + *store * DAMP;
                buf[*at] = dry + *store * FEEDBACK;
                *at = (*at + 1) % buf.len();
                wet += o;
            }
            for (buf, at) in &mut self.passes {
                let b = buf[*at];
                buf[*at] = wet + b * 0.5;
                wet = b - wet * 0.5;
                *at = (*at + 1) % buf.len();
            }
            *v = wet * 0.25;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 22_050;

    fn tone(frames: usize) -> Arc<Clip> {
        // 100 cycles in the clip: a seamless loop at 441 Hz
        Arc::new(Clip {
            channels: 1,
            rate: RATE,
            samples: (0..frames)
                .map(|i| (i as f32 * std::f32::consts::TAU * 100.0 / frames as f32).sin() * 0.5)
                .collect(),
        })
    }

    fn song(looped: bool) -> Arc<Song> {
        let json = format!(
            r#"{{ "bpm": 120, "rows_per_beat": 4, "reverb": 0, "looped": {looped},
            "instruments": {{ "pad": {{ "zones": [{{ "sample": "pad.wav", "root": 69 }}],
                "looped": true, "attack": 0.01, "release": 0.02 }} }},
            "patterns": {{ "a": {{ "rows": 8, "notes": [[0, "pad", 69, 4, 1.0], [4, "pad", 81, 2, 0.5]] }} }},
            "order": ["a"] }}"#
        );
        let file = SongFile::parse(&json).unwrap();
        assert_eq!(file.samples(), ["pad.wav"]);
        assert!((file.seconds() - 1.0).abs() < 1e-6);
        // nothing plays until every sample is there
        assert!(Song::build(&file, |_| None).is_none());
        Arc::new(Song::build(&file, |_| Some(tone(5000))).unwrap())
    }

    fn play(d: &mut Deck, secs: f32) -> Vec<f32> {
        let mut out = Vec::new();
        let frames = (secs * RATE as f32) as usize;
        for _ in 0..frames / 64 {
            let mut block = [0.0; 128];
            let mut room = [0.0; 64];
            d.render(RATE, 1.0, &mut block, &mut room);
            out.extend_from_slice(&block);
        }
        out
    }

    /// Times the wave crosses zero going up, in the left channel.
    fn cycles(x: &[f32]) -> usize {
        let left: Vec<f32> = x.iter().step_by(2).copied().collect();
        left.windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count()
    }

    #[test]
    fn notes_play_at_their_rows_at_their_pitch_and_end() {
        let mut d = Deck::new(song(false), 1.0, 0.0, false, RATE);
        let out = play(&mut d, 1.2);
        let at = |secs: f32| (secs * RATE as f32) as usize * 2;
        // rows are an eighth of a second: the first note sounds for four
        // of them at 441 Hz, the second for two an octave up and half as
        // loud, then silence
        let first = &out[at(0.1)..at(0.4)];
        assert!(
            (cycles(first) as f32 - 0.3 * 441.0).abs() <= 2.0,
            "{}",
            cycles(first)
        );
        let second = &out[at(0.52)..at(0.72)];
        assert!(
            (cycles(second) as f32 - 0.2 * 882.0).abs() <= 2.0,
            "{}",
            cycles(second)
        );
        let top = |x: &[f32]| x.iter().fold(0.0f32, |a, b| a.max(b.abs()));
        let centre = std::f32::consts::FRAC_1_SQRT_2;
        assert!((top(first) - 0.5 * centre).abs() < 0.01, "{}", top(first));
        assert!(
            (top(second) - 0.25 * centre).abs() < 0.01,
            "{}",
            top(second)
        );
        assert_eq!(top(&out[at(0.85)..]), 0.0);
        assert!(d.finished());
    }

    #[test]
    fn a_theme_goes_round_and_a_fade_has_no_step() {
        let mut d = Deck::new(song(true), 1.0, 0.5, false, RATE);
        let out = play(&mut d, 2.6);
        assert!(!d.finished());
        // the first note is there again a second later, and a second after
        let at = |secs: f32| (secs * RATE as f32) as usize * 2;
        for start in [1.1, 2.1] {
            let again = &out[at(start)..at(start + 0.3)];
            assert!((cycles(again) as f32 - 0.3 * 441.0).abs() <= 2.0);
        }
        // fading in over half a second: quiet at first, full after
        let top = |x: &[f32]| x.iter().fold(0.0f32, |a, b| a.max(b.abs()));
        assert!(top(&out[at(0.1)..at(0.2)]) < 0.5 * top(&out[at(1.1)..at(1.2)]));
        // fading out: no jump between one sample and the next is larger
        // than the tone itself makes, and then it ends
        d.stop(0.3, RATE);
        let tail = play(&mut d, 0.5);
        let left: Vec<f32> = tail.iter().step_by(2).copied().collect();
        let step = left
            .windows(2)
            .fold(0.0f32, |a, w| a.max((w[1] - w[0]).abs()));
        assert!(step < 0.06, "a step of {step}");
        assert!(d.finished());
        assert_eq!(*left.last().unwrap(), 0.0);
    }

    #[test]
    fn a_song_that_names_what_it_lacks_is_refused() {
        assert!(SongFile::parse(r#"{ "order": ["x"] }"#).is_err());
        let bad = r#"{ "patterns": { "a": { "rows": 4, "notes": [[0, "lute", 60, 1, 1]] } } }"#;
        assert!(SongFile::parse(bad).is_err());
        let late = r#"{ "instruments": { "p": { "zones": [{ "sample": "p", "root": 60 }] } },
            "patterns": { "a": { "rows": 4, "notes": [[4, "p", 60, 1, 1]] } } }"#;
        assert!(SongFile::parse(late).is_err());
    }

    #[test]
    fn the_room_rings_on_and_dies_away() {
        let mut room = Room::new(RATE);
        let mut x = vec![0.0; RATE as usize * 3];
        x[0] = 1.0;
        room.render(&mut x);
        let energy = |a: usize, b: usize| x[a..b].iter().map(|v| v * v).sum::<f32>();
        let r = RATE as usize;
        assert!(energy(r / 10, r / 2) > 0.0);
        assert!(energy(2 * r, 3 * r) < energy(r / 10, r / 2) * 0.01);
        assert!(x.iter().all(|v| v.abs() < 1.0));
    }
}
