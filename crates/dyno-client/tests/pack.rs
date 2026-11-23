//! Checks on the content pack in `content/` that the client depends on:
//! every block has its textures, every sound and item icon is there, the
//! fonts and render settings load; and the client has no generators left
//! that paint textures or synthesise sounds in code.

use dyno_client::audio::{Sfx, decode_wav};
use dyno_client::look::RenderSettings;
use dyno_client::textures::{BlocksFile, TILE, decode_png};
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

#[test]
fn every_sound_is_in_the_pack() {
    for s in Sfx::all() {
        let first = format!("sounds/{}_0.wav", s.name());
        let clip = decode_wav(&read(&first)).unwrap_or_else(|| panic!("{first}: not 16-bit PCM"));
        assert!(!clip.samples.is_empty(), "{first} is empty");
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
    let survival = std::fs::read_to_string(src.join("survival.rs")).unwrap();
    for kind in ["Stick", "Lump", "Ingot", "Gem", "Dust", "Berries", "Meat"] {
        let m = format!("Icon::{kind}");
        assert!(!survival.contains(&m), "survival.rs still draws `{m}`");
    }
    let tex = std::fs::read_to_string(src.join("textures.rs")).unwrap();
    assert!(
        tex.lines().count() < 400,
        "textures.rs should only load textures, not paint them"
    );
}
