//! Render the review montages: six scripted situations played through the
//! real mixer, written as small WAV files for a person to listen to, with
//! the measured level of each and of each bus.
//!
//! usage: cargo run -p dyno-client --example audio_montage -- [out-dir]
//! (tools/audio/montage.sh; the files in docs/audio/montages/ are its
//! output). Mono, 16-bit, 22050 Hz: the sounds' own rate, and small. The
//! game itself plays in stereo, with placed sounds to their side.

mod places;
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

fn limiter(held: f32) -> String {
    if held < 1.0 {
        format!("took off {:.1} dB at most", -20.0 * held.log10())
    } else {
        "idle".to_string()
    }
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
    let visits = places::all();

    println!("| Montage | Seconds | Peak (dBFS) | Loudness (dB) | Limiter | Bytes |");
    println!("|---|---|---|---|---|---|");
    let mut heard_in = Vec::new();
    let mut row = |name: &str, what: &str, secs: f32, heard: &[f32], held: f32| {
        heard_in.push(format!("- `{name}.wav`: {what}."));
        let bytes = write_wav(&out.join(format!("{name}.wav")), heard).expect("writing");
        println!(
            "| `{name}.wav` | {secs:.1} | {:.1} | {:.1} | {} | {bytes} |",
            scenes::peak_db(heard),
            scenes::loudness_db(heard, RATE),
            limiter(held),
        );
    };
    for s in &all {
        let mut a = Audio::offline(RATE);
        let heard = scenes::render(&mut a, s, |_, _| {});
        row(s.name, s.what, s.secs, &heard, a.held());
    }
    for v in &visits {
        let mut a = Audio::offline(RATE);
        let heard = places::render(&mut a, &mut places::Stage::new(), v);
        row(v.name, v.what, v.secs, &heard, a.held());
    }
    println!();
    println!("{}", heard_in.join("\n"));

    // each bus alone, in the worst case: the fight of `6_everything` in
    // the plaza in heavy rain, with the theme, the drums and a stinger
    // (the peak target is for one event alone: tests/pack.rs holds each
    // event to it)
    let fight = all.iter().find(|s| s.name == "6_everything").unwrap();
    let worst = places::worst(fight);
    println!();
    println!("| Bus | Peak in the worst case (dBFS) | Loudness (dB) | Target |");
    println!("|---|---|---|---|");
    for bus in Bus::ALL {
        let name = format!("{bus:?}").to_lowercase();
        let target = data.targets.get(&name).copied().unwrap_or_default();
        let mut a = Audio::offline(RATE);
        a.set_volumes(Volumes::only(bus));
        let heard = places::render(&mut a, &mut places::Stage::new(), &worst);
        println!(
            "| {name} | {:.1} | {:.1} | {:.0} |",
            scenes::peak_db(&heard),
            scenes::loudness_db(&heard, RATE),
            target.loudness_db
        );
    }
    for (what, volumes) in [
        ("all at full", Volumes::FULL),
        (
            "a new player's",
            dyno_client::options::Sound::of(&dyno_client::settings::Settings::default()).volumes(),
        ),
    ] {
        let mut a = Audio::offline(RATE);
        a.set_volumes(volumes);
        let heard = places::render(&mut a, &mut places::Stage::new(), &worst);
        println!(
            "worst case, sliders {what}: peak {:.1} dBFS, loudness {:.1} dB, limiter {}",
            scenes::peak_db(&heard),
            scenes::loudness_db(&heard, RATE),
            limiter(a.held())
        );
    }
}
