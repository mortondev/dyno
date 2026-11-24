//! Render the review montages: six scripted situations played through the
//! real mixer, written as small WAV files for a person to listen to, with
//! the measured level of each and of each bus.
//!
//! usage: cargo run -p dyno-client --example audio_montage -- [out-dir]
//! (tools/audio/montage.sh; the files in docs/audio/montages/ are its
//! output). Mono, 16-bit, 22050 Hz: the sounds' own rate, and small. The
//! game itself plays in stereo, with placed sounds to their side.

mod scenes;

use dyno_client::audio::{self, Audio, Bus, Volumes};
use std::path::{Path, PathBuf};

const RATE: u32 = 22_050;

fn write_wav(path: &Path, stereo: &[f32]) -> std::io::Result<usize> {
    let data: Vec<u8> = stereo
        .as_chunks::<2>()
        .0
        .iter()
        // (two ears to one: the sum of two equal-power halves, kept at the
        // level one ear heard)
        .map(|f| (f[0] + f[1]) * 0.5)
        .flat_map(|v| ((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes())
        .collect();
    let mut f = Vec::with_capacity(44 + data.len());
    f.extend_from_slice(b"RIFF");
    f.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    f.extend_from_slice(b"WAVEfmt ");
    f.extend_from_slice(&16u32.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&RATE.to_le_bytes());
    f.extend_from_slice(&(RATE * 2).to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&16u16.to_le_bytes());
    f.extend_from_slice(b"data");
    f.extend_from_slice(&(data.len() as u32).to_le_bytes());
    f.extend_from_slice(&data);
    std::fs::write(path, &f)?;
    Ok(f.len())
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("docs/audio/montages"));
    std::fs::create_dir_all(&out).expect("the output directory");
    dyno_client::assets::load_dir(&root.join("content")).expect("the pack in content/");
    let data = audio::data().expect("audio.json");
    let all = scenes::all();

    println!("| Montage | Seconds | Peak (dBFS) | Loudness (dB) | Limiter | Bytes |");
    println!("|---|---|---|---|---|---|");
    let mut heard_in = Vec::new();
    for s in &all {
        let mut a = Audio::offline(RATE);
        let heard = scenes::render(&mut a, s, |_, _| {});
        heard_in.push(format!("- `{}.wav`: {}.", s.name, s.what));
        let file = out.join(format!("{}.wav", s.name));
        let bytes = write_wav(&file, &heard).expect("writing the montage");
        let held = a.held();
        println!(
            "| `{}.wav` | {:.1} | {:.1} | {:.1} | {} | {} |",
            s.name,
            s.secs,
            scenes::peak_db(&heard),
            scenes::loudness_db(&heard, RATE),
            if held < 1.0 {
                format!("took off {:.1} dB at most", -20.0 * held.log10())
            } else {
                "idle".to_string()
            },
            bytes
        );
    }

    println!();
    println!("{}", heard_in.join("\n"));
    // each bus alone, in the scene where it is busiest (the peak target is
    // for one event alone: tests/pack.rs holds each event to it)
    println!();
    println!("| Bus | Busiest in | Peak there (dBFS) | Loudness (dB) | Target |");
    println!("|---|---|---|---|---|");
    for bus in Bus::ALL {
        let name = format!("{bus:?}").to_lowercase();
        let target = data.targets.get(&name).copied().unwrap_or_default();
        let mut best: Option<(f32, f32, &str)> = None;
        for s in &all {
            let mut a = Audio::offline(RATE);
            a.set_volumes(Volumes::only(bus));
            let heard = scenes::render(&mut a, s, |_, _| {});
            let (peak, loud) = (scenes::peak_db(&heard), scenes::loudness_db(&heard, RATE));
            if peak > -100.0 && best.is_none_or(|b| loud > b.1) {
                best = Some((peak, loud, s.name));
            }
        }
        match best {
            Some((peak, loud, scene)) => println!(
                "| {name} | `{scene}` | {peak:.1} | {loud:.1} | {:.0} |",
                target.loudness_db
            ),
            None => println!(
                "| {name} | (nothing plays on it yet) | | | {:.0} |",
                target.loudness_db
            ),
        }
    }
}
