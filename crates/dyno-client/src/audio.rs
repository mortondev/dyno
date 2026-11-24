//! Sound. The engine knows how to mix; what there is to hear is the pack's
//! business (`audio.json`): each *event* (a key such as `ui.open` or
//! `step.grass`) names a few variant files `<file>_<n>.wav` (16-bit PCM),
//! its bus, its level and how much its pitch may wander. Game code says
//! what happened, by key: [`Audio::play`], [`Audio::play_at`] for a place
//! in the world, or [`emit`] from code that has no `Audio` to hand.
//!
//! A sound whose files have not arrived is asked for (`assets::request`:
//! most sounds stream when first needed) and is silent until they come;
//! [`Audio::want`] asks ahead of time.
//!
//! Everything is mixed here ([`Mixer`]) into four buses (music, effects,
//! ambience, interface) under a master level, then held under full scale
//! by a limiter. The mixer runs on the audio thread, fed by messages; it
//! also runs with no device at all ([`Audio::offline`]), which is how the
//! tests and the review montages hear exactly what a player would.

use crate::assets;
use glam::Vec3;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZero;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

/// The pack file.
pub const AUDIO_JSON: &str = "audio.json";
/// Frames a second the device is fed.
pub const RATE: u32 = 44_100;
/// Sounds at once; past this the quietest gives way.
const VOICES: usize = 48;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Bus {
    Music,
    #[default]
    Effects,
    Ambience,
    Ui,
}

impl Bus {
    pub const ALL: [Bus; 4] = [Bus::Music, Bus::Effects, Bus::Ambience, Bus::Ui];
}

/// The player's levels, each 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub effects: f32,
    pub ambience: f32,
    pub ui: f32,
}

impl Volumes {
    pub const FULL: Volumes = Volumes {
        master: 1.0,
        music: 1.0,
        effects: 1.0,
        ambience: 1.0,
        ui: 1.0,
    };

    /// One bus alone, at full.
    pub fn only(bus: Bus) -> Volumes {
        let on = |b: Bus| if b == bus { 1.0 } else { 0.0 };
        Volumes {
            master: 1.0,
            music: on(Bus::Music),
            effects: on(Bus::Effects),
            ambience: on(Bus::Ambience),
            ui: on(Bus::Ui),
        }
    }

    fn of(&self, bus: Bus) -> f32 {
        match bus {
            Bus::Music => self.music,
            Bus::Effects => self.effects,
            Bus::Ambience => self.ambience,
            Bus::Ui => self.ui,
        }
    }
}

// ---- the pack's description ----

/// One thing that can be heard.
#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Event {
    /// the files are `<file>_<n>.wav`, n from 0
    pub file: String,
    pub variants: u8,
    pub bus: Bus,
    pub gain: f32,
    /// the pitch wanders this far either way each time (0.05: 5%)
    pub pitch: f32,
    /// seconds before it may sound again (a volley of blows is one sound)
    pub gap: f32,
    /// heard rarely (a level-up, a death): one variant is enough
    pub once: bool,
    /// metres it carries when it has a place (else the pack's `reach`)
    pub reach: Option<f32>,
}

impl Default for Event {
    fn default() -> Self {
        Event {
            file: String::new(),
            variants: 1,
            bus: Bus::Effects,
            gain: 1.0,
            pitch: 0.0,
            gap: 0.03,
            once: false,
            reach: None,
        }
    }
}

impl Event {
    pub fn paths(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.variants).map(|v| format!("{}_{v}.wav", self.file))
    }
}

/// Which footstep each ground gives, by block key.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Footsteps {
    /// the event for a block not listed
    pub default: String,
    /// the event when your feet are in water
    pub wade: String,
    pub blocks: BTreeMap<String, String>,
}

/// How hard the last stage holds the mix under full scale.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Limit {
    /// nothing leaves louder than this (1 is full scale)
    pub ceiling: f32,
    /// seconds for the level to come back after a peak
    pub release: f32,
}

impl Default for Limit {
    fn default() -> Self {
        Limit {
            ceiling: 0.89,
            release: 0.12,
        }
    }
}

/// What one bus should measure: the loudest single event at full sliders
/// (dB below full scale), and the loudness of its busiest passage. The
/// montage tool prints what it measures beside these.
#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Target {
    pub peak_db: f32,
    pub loudness_db: f32,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct AudioData {
    /// metres a placed sound carries
    pub reach: f32,
    pub limiter: Limit,
    pub targets: BTreeMap<String, Target>,
    pub events: BTreeMap<String, Event>,
    pub footsteps: Footsteps,
    /// how the sounds the rules name are played (an ability's `sound`):
    /// the files are `<file><name>_<n>.wav`, up to `variants` of them
    pub named: Option<Event>,
}

impl AudioData {
    pub fn parse(text: &str) -> Result<AudioData, String> {
        serde_json::from_str(text).map_err(|e| format!("{AUDIO_JSON}: {e}"))
    }

    /// The footstep event for the block underfoot.
    pub fn footstep(&self, block_key: &str) -> &str {
        self.footsteps
            .blocks
            .get(block_key)
            .unwrap_or(&self.footsteps.default)
    }
}

/// The data in the pack now, parsed again only when the file changes.
pub fn data() -> Option<Arc<AudioData>> {
    type Cache = Option<(Arc<[u8]>, Arc<AudioData>)>;
    static CACHE: Mutex<Cache> = Mutex::new(None);
    let bytes = assets::get(AUDIO_JSON)?;
    let mut c = CACHE.lock().unwrap();
    if let Some((b, d)) = c.as_ref()
        && Arc::ptr_eq(b, &bytes)
    {
        return Some(d.clone());
    }
    let d = Arc::new(
        std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(AudioData::parse)
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                AudioData::default()
            }),
    );
    *c = Some((bytes, d.clone()));
    Some(d)
}

// ---- events from code with no `Audio` to hand ----

/// (event, share of its level, seconds from now)
static QUEUE: Mutex<Vec<(&'static str, f32, f32)>> = Mutex::new(Vec::new());

/// Something happened that has a sound (panels, the HUD): it plays on the
/// next frame. An event the pack does not describe is silent.
pub fn emit(key: &'static str) {
    emit_in(key, 0.0);
}

/// The same, `secs` from now (after a fanfare, say).
pub fn emit_in(key: &'static str, secs: f32) {
    let mut q = QUEUE.lock().unwrap();
    if q.len() < 64 {
        q.push((key, 1.0, secs));
    }
}

// ---- samples ----

/// Decoded samples: channels, sample rate, interleaved samples in -1..1.
pub struct Clip {
    pub channels: u16,
    pub rate: u32,
    pub samples: Vec<f32>,
}

impl Clip {
    /// The same sound on one channel.
    pub fn mono(mut self) -> Clip {
        let ch = self.channels.max(1) as usize;
        if ch > 1 {
            self.samples = self
                .samples
                .chunks_exact(ch)
                .map(|f| f.iter().sum::<f32>() / ch as f32)
                .collect();
            self.channels = 1;
        }
        self
    }
}

/// Read a 16-bit PCM WAV file. Other formats are refused.
pub fn decode_wav(b: &[u8]) -> Option<Clip> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let u16_at = |i: usize| Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?));
    let u32_at = |i: usize| Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?));
    let mut fmt = None;
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32_at(i + 4)? as usize;
        let body = i + 8;
        let end = body.checked_add(len)?.min(b.len());
        if id == b"fmt " {
            // PCM, channels, rate, .., .., bits
            fmt = Some((
                u16_at(body)?,
                u16_at(body + 2)?,
                u32_at(body + 4)?,
                u16_at(body + 14)?,
            ));
        } else if id == b"data" {
            let (format, channels, rate, bits) = fmt?;
            if format != 1 || bits != 16 || channels == 0 || rate == 0 {
                return None;
            }
            let samples = b[body..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|s| i16::from_le_bytes(*s) as f32 / 32767.0)
                .collect();
            return Some(Clip {
                channels,
                rate,
                samples,
            });
        }
        // chunks are padded to an even length
        i = end + (len & 1);
    }
    None
}

// ---- the mixer ----

/// What the game tells the mixer.
pub enum Cmd {
    Play {
        clip: Arc<Clip>,
        bus: Bus,
        /// level on the left and on the right
        gain: [f32; 2],
        /// 1 as recorded; higher is faster and higher-pitched
        pitch: f32,
    },
    Volumes(Volumes),
    Limit(Limit),
}

struct Voice {
    clip: Arc<Clip>,
    bus: Bus,
    gain: [f32; 2],
    /// where it is in the clip, in the clip's own frames, and how far a
    /// frame of output moves it
    at: f64,
    step: f64,
}

/// Every sound playing, summed per bus and held under full scale.
pub struct Mixer {
    rate: u32,
    voices: Vec<Voice>,
    volumes: Volumes,
    /// the bus levels as they are now (they glide to `volumes`)
    level: [f32; 4],
    limit: Limit,
    /// the limiter: the level it is holding down; the frames it is
    /// looking ahead of and the gain each asked for (rings)
    peak: f32,
    ahead: Vec<[f32; 2]>,
    wants: Vec<f32>,
    ahead_at: usize,
    /// the most the limiter has had to take off so far (1: nothing)
    pub held: f32,
}

impl Mixer {
    pub fn new(rate: u32) -> Mixer {
        Mixer {
            rate,
            voices: Vec::with_capacity(VOICES),
            volumes: Volumes::FULL,
            level: [1.0; 4],
            limit: Limit::default(),
            peak: 0.0,
            // (1.5 ms: long enough to turn down smoothly, far too short
            // to hear as a delay)
            ahead: vec![[0.0; 2]; (rate as usize * 3 / 2000).max(8)],
            wants: vec![1.0; (rate as usize * 3 / 2000).max(8)],
            ahead_at: 0,
            held: 1.0,
        }
    }

    pub fn apply(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play {
                clip,
                bus,
                gain,
                pitch,
            } => {
                if clip.samples.is_empty() {
                    return;
                }
                let v = Voice {
                    step: clip.rate as f64 / self.rate as f64 * pitch.clamp(0.25, 4.0) as f64,
                    clip,
                    bus,
                    gain,
                    at: 0.0,
                };
                if self.voices.len() < VOICES {
                    self.voices.push(v);
                } else if let Some(quiet) = self
                    .voices
                    .iter_mut()
                    .min_by(|a, b| (a.gain[0] + a.gain[1]).total_cmp(&(b.gain[0] + b.gain[1])))
                    && quiet.gain[0] + quiet.gain[1] < v.gain[0] + v.gain[1]
                {
                    *quiet = v;
                }
            }
            Cmd::Volumes(v) => self.volumes = v,
            Cmd::Limit(l) => self.limit = l,
        }
    }

    pub fn playing(&self) -> usize {
        self.voices.len()
    }

    /// Fill `out` (left, right, left, right…) with the next frames.
    pub fn render(&mut self, out: &mut [f32]) {
        const CHUNK: usize = 64;
        for block in out.chunks_mut(CHUNK * 2) {
            block.fill(0.0);
            // the sliders glide: no click when one is dragged
            for (i, b) in Bus::ALL.iter().enumerate() {
                let want = (self.volumes.master * self.volumes.of(*b)).clamp(0.0, 1.0);
                self.level[i] += (want - self.level[i]) * 0.2;
                if (want - self.level[i]).abs() < 1e-4 {
                    self.level[i] = want;
                }
            }
            for v in &mut self.voices {
                let lvl = self.level[v.bus as usize];
                let (l, r) = (v.gain[0] * lvl, v.gain[1] * lvl);
                let s = &v.clip.samples;
                let last = s.len() - 1;
                for f in block.as_chunks_mut::<2>().0 {
                    let i = v.at as usize;
                    if i >= last {
                        break;
                    }
                    let t = (v.at - i as f64) as f32;
                    let x = s[i] + (s[i + 1] - s[i]) * t;
                    f[0] += x * l;
                    f[1] += x * r;
                    v.at += v.step;
                }
            }
            self.voices
                .retain(|v| (v.at as usize) < v.clip.samples.len() - 1);
            // the limiter: it sees each peak a moment before it plays (the
            // sound is held back by `ahead`), turns down smoothly in that
            // moment, and lets go slowly: no peak passes the ceiling and
            // none is cut flat
            let ceiling = self.limit.ceiling.clamp(0.1, 1.0);
            let fall = (-1.0 / (self.limit.release.max(0.005) * self.rate as f32)).exp();
            let span = self.ahead.len();
            for f in block.as_chunks_mut::<2>().0 {
                let late = std::mem::replace(&mut self.ahead[self.ahead_at], *f);
                // the level held down is never under any frame still
                // waiting to play, and falls away slowly after them
                let waiting = self
                    .ahead
                    .iter()
                    .fold(0.0f32, |m, w| m.max(w[0].abs()).max(w[1].abs()));
                self.peak = (self.peak * fall)
                    .max(waiting)
                    .max(late[0].abs().max(late[1].abs()));
                self.wants[self.ahead_at] = if self.peak > ceiling {
                    ceiling / self.peak
                } else {
                    1.0
                };
                self.ahead_at = (self.ahead_at + 1) % span;
                // the gain is the mean of what the frames since asked for:
                // every one of them asked for enough for this frame, so it
                // is enough, and it moves without a step
                let mut g = self.wants.iter().sum::<f32>() / span as f32;
                // (a last guard against rounding)
                let out = late[0].abs().max(late[1].abs()) * g;
                if out > ceiling {
                    g *= ceiling / out;
                }
                *f = [late[0] * g, late[1] * g];
                if g < 0.9999 {
                    self.held = self.held.min(g);
                }
            }
        }
    }
}

/// The mixer as the device hears it.
struct Output {
    rx: Receiver<Cmd>,
    mixer: Mixer,
    buf: [f32; 512],
    at: usize,
}

impl Iterator for Output {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.at == self.buf.len() {
            while let Ok(c) = self.rx.try_recv() {
                self.mixer.apply(c);
            }
            self.mixer.render(&mut self.buf);
            self.at = 0;
        }
        let v = self.buf[self.at];
        self.at += 1;
        Some(v)
    }
}

impl rodio::Source for Output {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).unwrap()
    }
    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(RATE).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

enum Out {
    /// no device: silent (but sounds still load, so tests can watch them)
    None,
    Device {
        _sink: rodio::MixerDeviceSink,
        tx: Sender<Cmd>,
    },
    /// mixed here, on request
    Offline(Box<Mixer>),
}

/// Where a decoded file came from, to notice when the pack replaces it.
type Loaded = (Arc<[u8]>, Arc<Clip>);

pub struct Audio {
    out: Out,
    /// decoded files by path
    clips: HashMap<String, Loaded>,
    /// per event: the variant played last, and when
    last: HashMap<String, (u8, f64)>,
    rng: u32,
    /// seconds, as the game counts them (`tick`)
    clock: f64,
    /// emitted for later: when, what, at what share of its level
    later: Vec<(f64, &'static str, f32)>,
    /// where the listener is and which way is their right
    ear: (Vec3, Vec3),
    volumes: Volumes,
    limit: Limit,
}

impl Audio {
    pub fn new() -> Self {
        let out = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                let (tx, rx) = channel();
                sink.mixer().add(Output {
                    rx,
                    mixer: Mixer::new(RATE),
                    buf: [0.0; 512],
                    at: 512,
                });
                Out::Device { _sink: sink, tx }
            }
            Err(e) => {
                eprintln!("no audio: {e}");
                Out::None
            }
        };
        Audio::with(out)
    }

    /// No device and no mixing: sounds load and nothing is heard.
    pub fn silent() -> Self {
        Audio::with(Out::None)
    }

    /// No device: [`Audio::render`] returns what would have been heard, at
    /// `rate` frames a second.
    pub fn offline(rate: u32) -> Self {
        Audio::with(Out::Offline(Box::new(Mixer::new(rate))))
    }

    fn with(out: Out) -> Self {
        Audio {
            out,
            clips: HashMap::new(),
            last: HashMap::new(),
            rng: 1,
            clock: 0.0,
            later: Vec::new(),
            ear: (Vec3::ZERO, Vec3::X),
            volumes: Volumes::FULL,
            limit: Limit::default(),
        }
    }

    fn send(&mut self, cmd: Cmd) {
        match &mut self.out {
            Out::None => {}
            Out::Device { tx, .. } => {
                let _ = tx.send(cmd);
            }
            Out::Offline(m) => m.apply(cmd),
        }
    }

    /// The player's levels: they take hold at once.
    pub fn set_volumes(&mut self, v: Volumes) {
        if v != self.volumes {
            self.volumes = v;
            self.send(Cmd::Volumes(v));
        }
    }

    pub fn volumes(&self) -> Volumes {
        self.volumes
    }

    /// Where the listener stands and which way their right ear points.
    pub fn listener(&mut self, pos: Vec3, right: Vec3) {
        self.ear = (pos, right.normalize_or(Vec3::X));
    }

    /// Once a frame: time passes, and what was `emit`ted plays.
    pub fn tick(&mut self, dt: f32) {
        self.clock += dt as f64;
        if let Some(d) = data()
            && d.limiter != self.limit
        {
            self.limit = d.limiter;
            self.send(Cmd::Limit(d.limiter));
        }
        let queued = std::mem::take(&mut *QUEUE.lock().unwrap());
        for (key, gain, secs) in queued {
            self.later.push((self.clock + secs as f64, key, gain));
        }
        let mut i = 0;
        while i < self.later.len() {
            if self.later[i].0 <= self.clock {
                let (_, key, gain) = self.later.remove(i);
                self.play(key, gain);
            } else {
                i += 1;
            }
        }
    }

    /// A file of the pack, decoded. None (and asked for) until it arrives.
    fn clip(&mut self, path: &str) -> Option<Arc<Clip>> {
        let clip = self.arrived(path);
        if clip.is_none() {
            assets::request(path);
        }
        clip
    }

    /// A file of the pack, decoded, if it has arrived.
    fn arrived(&mut self, path: &str) -> Option<Arc<Clip>> {
        let bytes = assets::get(path)?;
        if let Some((from, clip)) = self.clips.get(path)
            && Arc::ptr_eq(from, &bytes)
        {
            return Some(clip.clone());
        }
        let clip = Arc::new(decode_wav(&bytes)?.mono());
        self.clips.insert(path.to_string(), (bytes, clip.clone()));
        Some(clip)
    }

    /// Ask for an event's files before it is needed. How many variants of
    /// it are ready to play.
    pub fn want(&mut self, key: &str) -> usize {
        let Some(d) = data() else { return 0 };
        let Some(e) = d.events.get(key) else { return 0 };
        e.paths().filter(|p| self.clip(p).is_some()).count()
    }

    /// Play an event. `gain` scales its level.
    pub fn play(&mut self, key: &str, gain: f32) {
        self.start(key, gain, 0.0);
    }

    /// Play an event at a place in the world: quieter with distance, and
    /// to the side it is on.
    pub fn play_at(&mut self, key: &str, at: Vec3, gain: f32) {
        let Some(d) = data() else { return };
        let reach = d.events.get(key).and_then(|e| e.reach).unwrap_or(d.reach);
        let (near, side) = self.placed(at, reach);
        self.start(key, gain * near, side);
    }

    /// The footstep of the block with this key.
    pub fn footstep(&mut self, block_key: &str, wading: bool, gain: f32) {
        let Some(d) = data() else { return };
        let key = if wading && !d.footsteps.wade.is_empty() {
            &d.footsteps.wade
        } else {
            d.footstep(block_key)
        };
        self.start(key, gain, 0.0);
    }

    /// Play a sound the rules name (an ability's `sound`), at a place.
    /// These stream: the first time one is wanted it is asked for, and
    /// this is false until it has arrived (or if the pack has none), so
    /// the caller can fall back to a plain sound.
    pub fn play_named_at(&mut self, name: &str, at: Vec3, gain: f32) -> bool {
        let Some(d) = data() else { return false };
        let Some(e) = &d.named else { return false };
        // (how many variants a name has is not written down: those that
        // are there play, and none is asked for twice once one has come)
        let ready: Vec<(u8, Arc<Clip>)> = (0..e.variants)
            .filter_map(|v| Some((v, self.arrived(&format!("{}{name}_{v}.wav", e.file))?)))
            .collect();
        if ready.is_empty() {
            for v in 0..e.variants {
                assets::request(&format!("{}{name}_{v}.wav", e.file));
            }
            return false;
        }
        let (near, side) = self.placed(at, e.reach.unwrap_or(d.reach));
        self.voice(name, e, ready, gain * near, side);
        true
    }

    /// How much of a sound at `at` is heard (0 to 1), and how far to the
    /// right it is (-1 to 1).
    fn placed(&self, at: Vec3, reach: f32) -> (f32, f32) {
        let to = at - self.ear.0;
        let dist = to.length();
        // full within arm's reach, then falling away to nothing at `reach`
        let near = (1.0 - (dist - 1.5).max(0.0) / reach.max(1.0)).clamp(0.0, 1.0);
        let side = if dist > 0.3 {
            (to / dist).dot(self.ear.1)
        } else {
            0.0
        };
        (near * near, side)
    }

    fn start(&mut self, key: &str, gain: f32, side: f32) {
        let Some(d) = data() else { return };
        let Some(e) = d.events.get(key) else { return };
        // (every variant is looked at, so the missing ones are asked for)
        let ready: Vec<(u8, Arc<Clip>)> = e
            .paths()
            .enumerate()
            .filter_map(|(i, p)| Some((i as u8, self.clip(&p)?)))
            .collect();
        self.voice(key, e, ready, gain, side);
    }

    /// Start one of the variants that are `ready`.
    fn voice(&mut self, key: &str, e: &Event, ready: Vec<(u8, Arc<Clip>)>, gain: f32, side: f32) {
        let g = gain * e.gain;
        if g < 0.004 || ready.is_empty() {
            return;
        }
        let (last, when) = self.last.get(key).copied().unwrap_or((u8::MAX, -1e9));
        if self.clock - when < e.gap as f64 {
            return;
        }
        // never the same variant twice running
        let fresh: Vec<&(u8, Arc<Clip>)> = ready.iter().filter(|(i, _)| *i != last).collect();
        let (variant, clip) = if fresh.is_empty() {
            &ready[0]
        } else {
            fresh[self.random() as usize % fresh.len()]
        };
        match self.last.get_mut(key) {
            Some(l) => *l = (*variant, self.clock),
            None => {
                self.last.insert(key.to_string(), (*variant, self.clock));
            }
        }
        let pitch = 1.0 + e.pitch * (self.random() as f32 / 32768.0 - 1.0);
        // equal power from left to right; a placed sound never leaves one
        // ear entirely
        let angle = (side.clamp(-1.0, 1.0) * 0.8 + 1.0) * std::f32::consts::FRAC_PI_4;
        let cmd = Cmd::Play {
            clip: clip.clone(),
            bus: e.bus,
            gain: [g * angle.cos(), g * angle.sin()],
            pitch,
        };
        self.send(cmd);
    }

    /// 0..65535
    fn random(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        (self.rng >> 8) & 0xFFFF
    }

    /// Offline only: the next `secs` of sound (left, right, left…), with
    /// the clock moved on. Empty when there is a device, or none.
    pub fn render(&mut self, secs: f32) -> Vec<f32> {
        self.tick(0.0);
        self.clock += secs as f64;
        match &mut self.out {
            Out::Offline(m) => {
                let mut out = vec![0.0; (secs * m.rate as f32).round() as usize * 2];
                m.render(&mut out);
                out
            }
            _ => Vec::new(),
        }
    }

    /// Offline only: the most the limiter has had to take off (1: nothing).
    pub fn held(&self) -> f32 {
        match &self.out {
            Out::Offline(m) => m.held,
            _ => 1.0,
        }
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u16, 1] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * 2).to_le_bytes());
        for v in [2u16, 16] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(data.len() as u32).to_le_bytes());
        f.extend_from_slice(&data);
        f
    }

    #[test]
    fn reads_16_bit_pcm() {
        let c = decode_wav(&wav(&[0, 16384, -32767], 22050)).unwrap();
        assert_eq!((c.channels, c.rate, c.samples.len()), (1, 22050, 3));
        assert!((c.samples[1] - 0.5).abs() < 0.001);
        assert_eq!(c.samples[2], -1.0);
        assert!(decode_wav(b"RIFF\0\0\0\0WAVE").is_none());
    }

    fn clip(level: f32, frames: usize) -> Arc<Clip> {
        Arc::new(Clip {
            channels: 1,
            rate: RATE,
            samples: vec![level; frames],
        })
    }

    fn play(m: &mut Mixer, bus: Bus, level: f32, frames: usize) {
        m.apply(Cmd::Play {
            clip: clip(level, frames),
            bus,
            gain: [1.0, 1.0],
            pitch: 1.0,
        });
    }

    #[test]
    fn the_limiter_lets_nothing_past_its_ceiling() {
        let mut m = Mixer::new(RATE);
        // ten full-scale sounds at once, on every bus
        for i in 0..10 {
            play(
                &mut m,
                Bus::ALL[i % 4],
                if i % 2 == 0 { 1.0 } else { 0.9 },
                4000,
            );
        }
        let mut out = vec![0.0; 8000];
        m.render(&mut out);
        let top = out.iter().fold(0.0f32, |a, b| a.max(b.abs()));
        assert!(top <= Limit::default().ceiling + 1e-6, "peak {top}");
        assert!(top > 0.5, "and it is not simply silent: {top}");
        assert!(m.held < 1.0);
        // a quiet sound alone is left as it is
        let mut m = Mixer::new(RATE);
        play(&mut m, Bus::Ui, 0.25, 4000);
        m.render(&mut out);
        assert!((out[2000] - 0.25).abs() < 1e-6);
        assert_eq!(m.held, 1.0);
    }

    #[test]
    fn a_rising_peak_is_turned_down_not_cut_flat() {
        // a tone that swells to twice full scale: under the ceiling
        // throughout, and still a tone (no run of samples pinned there)
        let frames = 20_000;
        let samples: Vec<f32> = (0..frames)
            .map(|i| {
                let swell = (i as f32 / 4000.0).min(2.0);
                swell * (i as f32 * 0.05).sin()
            })
            .collect();
        let mut m = Mixer::new(RATE);
        m.apply(Cmd::Play {
            clip: Arc::new(Clip {
                channels: 1,
                rate: RATE,
                samples,
            }),
            bus: Bus::Effects,
            gain: [1.0, 1.0],
            pitch: 1.0,
        });
        let mut out = vec![0.0; frames * 2];
        m.render(&mut out);
        let ceiling = Limit::default().ceiling;
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        let top = left.iter().fold(0.0f32, |a, b| a.max(b.abs()));
        assert!(top <= ceiling + 1e-6 && top > ceiling * 0.9, "peak {top}");
        let pinned = left
            .windows(3)
            .filter(|w| w.iter().all(|s| s.abs() > ceiling - 1e-4))
            .count();
        assert_eq!(pinned, 0, "flat tops");
        assert!(m.held < 0.6, "it had to halve the level: {}", m.held);
    }

    #[test]
    fn the_device_is_fed_what_the_mixer_makes() {
        // the path a player hears: messages in, samples out
        let (tx, rx) = channel();
        let mut device = Output {
            rx,
            mixer: Mixer::new(RATE),
            buf: [0.0; 512],
            at: 512,
        };
        use rodio::Source;
        assert_eq!(
            (device.channels().get(), device.sample_rate().get()),
            (2, RATE)
        );
        assert!(
            device.by_ref().take(2000).all(|s| s == 0.0),
            "silence first"
        );
        tx.send(Cmd::Play {
            clip: clip(0.5, 4000),
            bus: Bus::Ui,
            gain: [1.0, 0.25],
            pitch: 1.0,
        })
        .unwrap();
        tx.send(Cmd::Volumes(Volumes {
            master: 0.5,
            ..Volumes::FULL
        }))
        .unwrap();
        let heard: Vec<f32> = device.by_ref().take(6000).collect();
        let (left, right) = (heard[5000], heard[5001]);
        assert!((left - 0.25).abs() < 0.01, "left {left}");
        assert!((right - 0.0625).abs() < 0.01, "right {right}");
        // and it never ends, with or without the game
        drop(tx);
        assert!(device.next().is_some());
    }

    #[test]
    fn each_slider_moves_its_own_bus() {
        for bus in Bus::ALL {
            let mut m = Mixer::new(RATE);
            m.apply(Cmd::Volumes(Volumes::only(bus)));
            for b in Bus::ALL {
                // each bus sounds at a level of its own, to tell them apart
                play(&mut m, b, 0.1 * (b as usize + 1) as f32, 40_000);
            }
            let mut out = vec![0.0; 40_000];
            m.render(&mut out);
            let want = 0.1 * (bus as usize + 1) as f32;
            assert!(
                (out[30_000] - want).abs() < 1e-3,
                "{bus:?}: {}",
                out[30_000]
            );
            // and the master scales what is left
            m.apply(Cmd::Volumes(Volumes {
                master: 0.5,
                ..Volumes::only(bus)
            }));
            m.render(&mut out);
            assert!((out[30_000] - want * 0.5).abs() < 1e-3);
        }
    }

    #[test]
    fn a_sound_ends_and_a_lower_rate_is_stretched() {
        let mut m = Mixer::new(RATE);
        m.apply(Cmd::Play {
            clip: Arc::new(Clip {
                channels: 1,
                rate: RATE / 2,
                samples: vec![0.5; 1000],
            }),
            bus: Bus::Effects,
            gain: [1.0, 0.0],
            pitch: 1.0,
        });
        let mut out = vec![0.0; 6000];
        m.render(&mut out);
        // 1000 frames at half the rate last 2000 frames; left only
        assert!((out[2 * 1900] - 0.5).abs() < 1e-6);
        assert_eq!(out[2 * 1900 + 1], 0.0);
        assert_eq!(out[2 * 2100], 0.0);
        assert_eq!(m.playing(), 0);
    }

    /// A pack of this test's own in the shared store (other tests use it
    /// too, so the names here are used nowhere else).
    fn test_pack() -> std::sync::MutexGuard<'static, ()> {
        // (one at a time: they share the queue of emitted events)
        static LOCK: Mutex<()> = Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let json = r#"{ "reach": 20, "events": {
            "test.streamed": { "file": "sounds/test/streamed", "variants": 3, "bus": "ui", "gap": 0 },
            "test.placed": { "file": "sounds/test/placed", "variants": 1, "once": true, "gap": 0 }
        }, "footsteps": { "default": "test.placed", "blocks": { "grass": "test.streamed" } },
           "named": { "file": "sounds/test/named/", "variants": 3, "gain": 0.5, "gap": 0 } }"#;
        assets::insert(AUDIO_JSON, json.as_bytes().to_vec());
        guard
    }

    #[test]
    fn streamed_variants_are_asked_for_and_play_as_they_arrive() {
        let _pack = test_pack();
        let mut a = Audio::offline(RATE);
        // nothing has arrived: the event is silent, and every variant is
        // asked for
        a.play("test.streamed", 1.0);
        assert!(a.render(0.01).iter().all(|s| *s == 0.0));
        let asked = assets::take_requests(|p| p.starts_with("sounds/test/streamed"));
        let mut asked_sorted = asked.clone();
        asked_sorted.sort();
        assert_eq!(
            asked_sorted,
            [
                "sounds/test/streamed_0.wav",
                "sounds/test/streamed_1.wav",
                "sounds/test/streamed_2.wav"
            ]
        );
        // one arrives: it plays (and the others are asked for again)
        assets::insert("sounds/test/streamed_1.wav", wav(&[8000; 400], RATE));
        assert_eq!(a.want("test.streamed"), 1);
        a.play("test.streamed", 1.0);
        assert!(a.render(0.01).iter().any(|s| *s != 0.0));
        let again = assets::take_requests(|p| p.starts_with("sounds/test/streamed"));
        assert_eq!(again.len(), 2, "{again:?}");
        // the rest arrive: all three take turns, never one twice running
        assets::insert("sounds/test/streamed_0.wav", wav(&[4000; 400], RATE));
        assets::insert("sounds/test/streamed_2.wav", wav(&[12000; 400], RATE));
        assert_eq!(a.want("test.streamed"), 3);
        let mut heard = Vec::new();
        for _ in 0..12 {
            a.play("test.streamed", 1.0);
            let out = a.render(0.02);
            heard.push((out[400] * 32767.0 * std::f32::consts::SQRT_2).round() as i32 / 1000);
        }
        for w in heard.windows(2) {
            assert_ne!(w[0], w[1], "{heard:?}");
        }
        for v in [4, 8, 12] {
            assert!(heard.contains(&v), "{heard:?}");
        }
        // a file the pack replaces is decoded again
        assets::insert("sounds/test/streamed_0.wav", wav(&[2000; 400], RATE));
        assets::insert("sounds/test/streamed_1.wav", wav(&[2000; 400], RATE));
        assets::insert("sounds/test/streamed_2.wav", wav(&[2000; 400], RATE));
        a.play("test.streamed", 1.0);
        let out = a.render(0.02);
        assert_eq!(
            (out[400] * 32767.0 * std::f32::consts::SQRT_2).round() as i32,
            2000
        );
    }

    #[test]
    fn a_placed_sound_fades_with_distance_and_leans_to_its_side() {
        let _pack = test_pack();
        assets::insert("sounds/test/placed_0.wav", wav(&[16000; 2000], RATE));
        let mut a = Audio::offline(RATE);
        a.listener(Vec3::ZERO, Vec3::X);
        let mut hear = |at: Vec3| {
            a.play_at("test.placed", at, 1.0);
            let out = a.render(0.05);
            (out[200], out[201])
        };
        let (l, r) = hear(Vec3::new(0.0, 0.0, -1.0));
        assert!((l - r).abs() < 1e-4 && l > 0.3, "ahead: {l} {r}");
        let (l, r) = hear(Vec3::new(5.0, 0.0, 0.0));
        assert!(r > l * 2.0 && l > 0.0, "to the right: {l} {r}");
        let (l2, _) = hear(Vec3::new(0.0, 0.0, -12.0));
        assert!(l2 > 0.0 && l2 < 0.3 * 0.5, "far: {l2}");
        let (l3, r3) = hear(Vec3::new(0.0, 0.0, -30.0));
        assert_eq!((l3, r3), (0.0, 0.0), "out of reach");
    }

    #[test]
    fn a_named_sound_is_asked_for_once_and_then_plays() {
        let _pack = test_pack();
        let mut a = Audio::offline(RATE);
        let here = Vec3::new(0.0, 0.0, -1.0);
        // not arrived: the caller is told to fall back, and it is asked for
        assert!(!a.play_named_at("spark", here, 1.0));
        let asked = assets::take_requests(|p| p.starts_with("sounds/test/named/"));
        assert_eq!(asked.len(), 3, "{asked:?}");
        // it has one variant: that plays, and the others are not asked for
        // again every time
        assets::insert("sounds/test/named/spark_0.wav", wav(&[16000; 2000], RATE));
        assert!(a.play_named_at("spark", here, 1.0));
        let out = a.render(0.03);
        let want = 16000.0 / 32767.0 * 0.5 * std::f32::consts::FRAC_1_SQRT_2;
        assert!((out[400] - want).abs() < 1e-3, "{} {want}", out[400]);
        assert!(assets::take_requests(|p| p.starts_with("sounds/test/named/")).is_empty());
    }

    #[test]
    fn footsteps_follow_the_ground_and_events_queue_from_anywhere() {
        let _pack = test_pack();
        let d = data().unwrap();
        assert_eq!(d.footstep("grass"), "test.streamed");
        assert_eq!(d.footstep("anything else"), "test.placed");
        assets::insert("sounds/test/placed_0.wav", wav(&[16000; 2000], RATE));
        let mut a = Audio::offline(RATE);
        emit("test.placed");
        emit("no such event");
        assert!(a.render(0.01).iter().any(|s| *s != 0.0));
    }
}
