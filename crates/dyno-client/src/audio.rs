//! Procedural sound effects. Every sound is synthesised at start, so the game
//! ships no audio files. If no audio device exists, the game stays silent.

use dyno_world::Material;
use std::num::NonZero;

const RATE: u32 = 44_100;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    Break(Material),
    Place(Material),
    Step(Material),
    Chat,
    Join,
    Click,
    Cricket,
}

pub struct Audio {
    sink: Option<rodio::MixerDeviceSink>,
    bank: Vec<(Sfx, Vec<Vec<f32>>)>,
    rng: u32,
    pub volume: f32,
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Noise burst through a one-pole low-pass filter, with an exponential decay.
fn noise_burst(rng: &mut Rng, ms: f32, cutoff: f32, decay: f32, gain: f32) -> Vec<f32> {
    let n = (RATE as f32 * ms / 1000.0) as usize;
    let a = (-2.0 * std::f32::consts::PI * cutoff / RATE as f32).exp();
    let mut y = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            y = a * y + (1.0 - a) * rng.next();
            y * gain * (-decay * t).exp() * (1.0 - t).min(1.0) * 3.0
        })
        .collect()
}

fn tone(ms: f32, freq0: f32, freq1: f32, decay: f32, gain: f32) -> Vec<f32> {
    let n = (RATE as f32 * ms / 1000.0) as usize;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = freq0 + (freq1 - freq0) * t;
            phase += f / RATE as f32;
            (phase * std::f32::consts::TAU).sin() * gain * (-decay * t).exp() * (t * 40.0).min(1.0)
        })
        .collect()
}

fn mix(a: Vec<f32>, b: Vec<f32>) -> Vec<f32> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
        .collect()
}

fn material_params(m: Material) -> (f32, f32) {
    // (cutoff Hz, length ms)
    match m {
        Material::Stone => (2600.0, 150.0),
        Material::Earth => (900.0, 130.0),
        Material::Wood => (1500.0, 120.0),
        Material::Foliage => (3500.0, 110.0),
        Material::Glass => (6000.0, 90.0),
        Material::Cloth => (700.0, 100.0),
        Material::Water => (1200.0, 200.0),
    }
}

fn synth(sfx: Sfx, rng: &mut Rng, variant: usize) -> Vec<f32> {
    let pitch = 1.0 + (variant as f32 - 1.5) * 0.06;
    match sfx {
        Sfx::Break(m) => {
            let (c, ms) = material_params(m);
            let body = noise_burst(rng, ms * 1.2, c * pitch, 5.0, 0.55);
            match m {
                Material::Wood => mix(body, tone(90.0, 220.0 * pitch, 160.0 * pitch, 8.0, 0.25)),
                Material::Glass => mix(
                    body,
                    mix(
                        tone(160.0, 2400.0 * pitch, 2200.0, 6.0, 0.12),
                        tone(120.0, 3300.0 * pitch, 3100.0, 7.0, 0.08),
                    ),
                ),
                Material::Stone => mix(body, tone(70.0, 140.0 * pitch, 90.0, 9.0, 0.2)),
                _ => body,
            }
        }
        Sfx::Place(m) => {
            let (c, _) = material_params(m);
            mix(
                noise_burst(rng, 70.0, c * 0.6 * pitch, 9.0, 0.45),
                tone(80.0, 130.0 * pitch, 80.0, 10.0, 0.35),
            )
        }
        Sfx::Step(m) => {
            let (c, _) = material_params(m);
            noise_burst(rng, 55.0, c * 0.8 * pitch, 10.0, 0.18)
        }
        Sfx::Chat => tone(90.0, 880.0, 990.0, 6.0, 0.12),
        Sfx::Join => mix(tone(140.0, 523.0, 523.0, 4.0, 0.12), {
            let mut v = vec![0.0; (RATE as f32 * 0.09) as usize];
            v.extend(tone(180.0, 784.0, 784.0, 4.0, 0.12));
            v
        }),
        Sfx::Click => tone(30.0, 1200.0, 900.0, 12.0, 0.08),
        Sfx::Cricket => {
            // three quick chirps
            let mut v = Vec::new();
            let f = 4200.0 * pitch;
            for _ in 0..3 {
                v.extend(tone(38.0, f, f * 0.98, 3.0, 0.05));
                v.extend(vec![0.0; (RATE as f32 * 0.035) as usize]);
            }
            v
        }
    }
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
        let mut rng = Rng(0x1234_5678);
        let mut bank = Vec::new();
        let mats = [
            Material::Stone,
            Material::Earth,
            Material::Wood,
            Material::Foliage,
            Material::Glass,
            Material::Cloth,
            Material::Water,
        ];
        let mut kinds: Vec<Sfx> = Vec::new();
        for m in mats {
            kinds.extend([Sfx::Break(m), Sfx::Place(m), Sfx::Step(m)]);
        }
        kinds.extend([Sfx::Chat, Sfx::Join, Sfx::Click, Sfx::Cricket]);
        for k in kinds {
            let variants = (0..4).map(|v| synth(k, &mut rng, v)).collect();
            bank.push((k, variants));
        }
        Audio {
            sink,
            bank,
            rng: 1,
            volume: 0.8,
        }
    }

    /// Play a sound. `gain` scales the volume, for example by distance.
    pub fn play(&mut self, sfx: Sfx, gain: f32) {
        let Some(sink) = &self.sink else { return };
        let g = gain * self.volume;
        if g < 0.01 {
            return;
        }
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let Some((_, vars)) = self.bank.iter().find(|(k, _)| *k == sfx) else {
            return;
        };
        let v = &vars[(self.rng >> 16) as usize % vars.len()];
        let data: Vec<f32> = v.iter().map(|s| (s * g).clamp(-1.0, 1.0)).collect();
        sink.mixer().add(rodio::buffer::SamplesBuffer::new(
            NonZero::new(1).unwrap(),
            NonZero::new(RATE).unwrap(),
            data,
        ));
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}
