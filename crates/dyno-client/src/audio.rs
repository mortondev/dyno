//! Sound effects from the pack: each sound is a few variants,
//! `sounds/<name>_<n>.wav` (16-bit PCM), and one is picked at random each
//! time it plays. A sound loads the first time it plays after its files
//! arrive; until then it is silent. If no audio device exists, the game
//! stays silent too.

use crate::assets;
use dyno_world::Material;
use std::collections::HashMap;
use std::num::NonZero;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    Break(Material),
    Place(Material),
    Step(Material),
    Chat,
    Join,
    Click,
    Cricket,
    /// a weapon connects
    Hit,
    /// dusk is coming: a low two-note horn
    Horn,
    /// a new day: bright rising notes
    Chime,
    /// low health
    Heartbeat,
    /// a Shade comes apart
    Dissolve,
    /// an item picked up
    Pop,
    /// a pick bites while digging
    Tick,
    /// a second of steady rain
    Rain,
    /// a gust of wind
    Wind,
    /// a songbird
    Bird,
    /// a far-off smith's hammer (town ambience)
    Hammer,
}

impl Sfx {
    /// Every sound the client plays (for pack checks).
    pub fn all() -> Vec<Sfx> {
        use Material::*;
        let mut v = Vec::new();
        for m in [Stone, Earth, Wood, Foliage, Glass, Cloth, Water] {
            v.extend([Sfx::Break(m), Sfx::Place(m), Sfx::Step(m)]);
        }
        v.extend([
            Sfx::Chat,
            Sfx::Join,
            Sfx::Click,
            Sfx::Cricket,
            Sfx::Hit,
            Sfx::Horn,
            Sfx::Chime,
            Sfx::Heartbeat,
            Sfx::Dissolve,
            Sfx::Pop,
            Sfx::Tick,
            Sfx::Rain,
            Sfx::Wind,
            Sfx::Bird,
            Sfx::Hammer,
        ]);
        v
    }

    /// The sound's name in the pack: `sounds/<name>_<variant>.wav`.
    pub fn name(self) -> String {
        let m = |m: Material| format!("{m:?}").to_lowercase();
        match self {
            Sfx::Break(x) => format!("break_{}", m(x)),
            Sfx::Place(x) => format!("place_{}", m(x)),
            Sfx::Step(x) => format!("step_{}", m(x)),
            other => format!("{other:?}").to_lowercase(),
        }
    }
}

/// Decoded samples: channels, sample rate, interleaved samples in -1..1.
pub struct Clip {
    pub channels: u16,
    pub rate: u32,
    pub samples: Vec<f32>,
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

pub struct Audio {
    sink: Option<rodio::MixerDeviceSink>,
    /// the variants of each sound that has loaded
    bank: HashMap<Sfx, Vec<Clip>>,
    /// sounds the content names (abilities'), by name
    named: HashMap<String, Vec<Clip>>,
    rng: u32,
    pub volume: f32,
}

impl Audio {
    pub fn new() -> Self {
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut s) => {
                s.log_on_drop(false);
                Some(s)
            }
            Err(e) => {
                eprintln!("no audio: {e}");
                None
            }
        };
        Audio {
            sink,
            bank: HashMap::new(),
            named: HashMap::new(),
            rng: 1,
            volume: 0.8,
        }
    }

    /// No sound at all (a scripted client).
    pub fn silent() -> Self {
        Audio {
            sink: None,
            bank: HashMap::new(),
            named: HashMap::new(),
            rng: 1,
            volume: 0.0,
        }
    }

    /// The variants of a sound, loading them from the pack the first time.
    /// Empty while the files have not arrived.
    fn variants(&mut self, sfx: Sfx) -> &[Clip] {
        use std::collections::hash_map::Entry;
        match self.bank.entry(sfx) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let name = sfx.name();
                let clips: Vec<Clip> = (0..)
                    .map_while(|v| assets::get(&format!("sounds/{name}_{v}.wav")))
                    .filter_map(|b| decode_wav(&b))
                    .collect();
                if clips.is_empty() {
                    // not arrived yet: look again next time
                    return &[];
                }
                e.insert(clips)
            }
        }
    }

    /// Play a sound the content names (an ability's):
    /// `sounds/abilities/<name>_<n>.wav` in the pack. These are streamed: the
    /// first time one is wanted it is asked for, and this returns false
    /// until it has arrived (or if the pack has none), so the caller can
    /// fall back to a built-in sound.
    pub fn play_named(&mut self, name: &str, gain: f32) -> bool {
        if !self.named.contains_key(name) {
            let path = |v: usize| format!("sounds/abilities/{name}_{v}.wav");
            let clips: Vec<Clip> = (0..)
                .map_while(|v| assets::get(&path(v)))
                .filter_map(|b| decode_wav(&b))
                .collect();
            if clips.is_empty() {
                // up to three variants; asking for one the pack lacks is harmless
                for v in 0..3 {
                    assets::request(&path(v));
                }
                return false;
            }
            self.named.insert(name.to_string(), clips);
        }
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let pick = (self.rng >> 16) as usize;
        let vars = &self.named[name];
        let v = &vars[pick % vars.len()];
        Self::emit(&self.sink, v, gain * self.volume);
        true
    }

    /// Play a sound. `gain` scales the volume, for example by distance.
    pub fn play(&mut self, sfx: Sfx, gain: f32) {
        if self.sink.is_none() {
            return;
        }
        let g = gain * self.volume;
        if g < 0.01 {
            return;
        }
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let pick = (self.rng >> 16) as usize;
        let vars = self.variants(sfx);
        if vars.is_empty() {
            return;
        }
        let v = &vars[pick % vars.len()];
        let data: Vec<f32> = v.samples.iter().map(|s| (s * g).clamp(-1.0, 1.0)).collect();
        let (Some(ch), Some(rate)) = (NonZero::new(v.channels), NonZero::new(v.rate)) else {
            return;
        };
        if let Some(sink) = &self.sink {
            sink.mixer()
                .add(rodio::buffer::SamplesBuffer::new(ch, rate, data));
        }
    }

    fn emit(sink: &Option<rodio::MixerDeviceSink>, v: &Clip, g: f32) {
        let (Some(sink), Some(ch), Some(rate)) =
            (sink, NonZero::new(v.channels), NonZero::new(v.rate))
        else {
            return;
        };
        if g < 0.01 {
            return;
        }
        let data: Vec<f32> = v.samples.iter().map(|s| (s * g).clamp(-1.0, 1.0)).collect();
        sink.mixer()
            .add(rodio::buffer::SamplesBuffer::new(ch, rate, data));
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

    #[test]
    fn reads_16_bit_pcm() {
        let data: Vec<u8> = [0i16, 16384, -32767]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u16, 1] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&22050u32.to_le_bytes());
        f.extend_from_slice(&44100u32.to_le_bytes());
        for v in [2u16, 16] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(data.len() as u32).to_le_bytes());
        f.extend_from_slice(&data);
        let c = decode_wav(&f).unwrap();
        assert_eq!((c.channels, c.rate, c.samples.len()), (1, 22050, 3));
        assert!((c.samples[1] - 0.5).abs() < 0.001);
        assert_eq!(c.samples[2], -1.0);
        assert!(decode_wav(b"RIFF\0\0\0\0WAVE").is_none());
    }
}
