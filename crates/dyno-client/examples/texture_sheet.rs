//! Writes a preview sheet of every procedural texture as a PPM image.
//!
//! Usage: `cargo run -p dyno-client --example texture_sheet -- out.ppm [scale]`
//!
//! The sheet has four parts: every block face, 3×3 tiled patches of the key
//! materials (to check seams), the frames of the animated textures, and every
//! item sprite over a dark and a light background. It also prints the checks
//! that the eye does not catch: alpha rules, determinism and build time.

#[path = "../src/textures.rs"]
mod textures;

use dyno_world::BLOCKS;
use dyno_world::content::{Content, FIRST_ITEM, ItemView};
use std::io::Write;
use std::time::Instant;
use textures::{Rgba, TILE, Textures, item_sprite};

/// 3×5 glyphs for A-Z then 0-9, one octal digit per row.
const FONT: [[u8; 5]; 36] = [
    [2, 5, 7, 5, 5],
    [6, 5, 6, 5, 6],
    [3, 4, 4, 4, 3],
    [6, 5, 5, 5, 6],
    [7, 4, 6, 4, 7],
    [7, 4, 6, 4, 4],
    [3, 4, 5, 5, 3],
    [5, 5, 7, 5, 5],
    [7, 2, 2, 2, 7],
    [1, 1, 1, 5, 2],
    [5, 5, 6, 5, 5],
    [4, 4, 4, 4, 7],
    [5, 7, 7, 5, 5],
    [6, 5, 5, 5, 5],
    [2, 5, 5, 5, 2],
    [6, 5, 6, 4, 4],
    [2, 5, 5, 6, 3],
    [6, 5, 6, 5, 5],
    [3, 4, 2, 1, 6],
    [7, 2, 2, 2, 2],
    [5, 5, 5, 5, 7],
    [5, 5, 5, 5, 2],
    [5, 5, 7, 7, 5],
    [5, 5, 2, 5, 5],
    [5, 5, 2, 2, 2],
    [7, 1, 2, 4, 7],
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [6, 1, 2, 4, 7],
    [6, 1, 2, 1, 6],
    [5, 5, 7, 1, 1],
    [7, 4, 6, 1, 6],
    [3, 4, 7, 5, 7],
    [7, 1, 2, 2, 2],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 6],
];

#[derive(Clone, Copy)]
enum Bg {
    Checker,
    Solid([u8; 3]),
}

struct Sheet {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Sheet {
    fn new(w: usize, h: usize) -> Sheet {
        let mut px = vec![0u8; w * h * 3];
        for p in px.chunks_exact_mut(3) {
            p.copy_from_slice(&[34, 32, 40]);
        }
        Sheet { w, h, px }
    }

    fn put(&mut self, x: usize, y: usize, c: [u8; 3]) {
        if x < self.w && y < self.h {
            let i = (y * self.w + x) * 3;
            self.px[i..i + 3].copy_from_slice(&c);
        }
    }

    /// Draw one tile scaled up with nearest neighbour, blended over `bg`.
    fn tile(&mut self, t: &Rgba, x0: usize, y0: usize, scale: usize, bg: Bg) {
        for y in 0..TILE * scale {
            for x in 0..TILE * scale {
                let i = ((y / scale) * TILE + x / scale) * 4;
                let back = match bg {
                    Bg::Solid(c) => c,
                    Bg::Checker => {
                        if ((x0 + x) / 8 + (y0 + y) / 8) % 2 == 0 {
                            [150, 150, 158]
                        } else {
                            [105, 105, 112]
                        }
                    }
                };
                let a = t[i + 3] as u32;
                let mut c = [0u8; 3];
                for k in 0..3 {
                    c[k] = ((t[i + k] as u32 * a + back[k] as u32 * (255 - a)) / 255) as u8;
                }
                self.put(x0 + x, y0 + y, c);
            }
        }
    }

    fn text(&mut self, s: &str, x0: usize, y0: usize) {
        let mut x = x0;
        for ch in s.chars() {
            let glyph = match ch.to_ascii_uppercase() {
                c @ 'A'..='Z' => Some(FONT[c as usize - 'A' as usize]),
                c @ '0'..='9' => Some(FONT[26 + c as usize - '0' as usize]),
                _ => None,
            };
            if let Some(rows) = glyph {
                for (gy, bits) in rows.iter().enumerate() {
                    for gx in 0..3 {
                        if bits & (4 >> gx) != 0 {
                            for d in 0..4 {
                                self.put(x + gx * 2 + d % 2, y0 + gy * 2 + d / 2, [230, 226, 210]);
                            }
                        }
                    }
                }
            }
            x += 8;
        }
    }
}

fn mean_rgb(t: &Rgba) -> [f32; 3] {
    let mut sum = [0f32; 3];
    let mut n = 0f32;
    for p in t.chunks_exact(4).filter(|p| p[3] > 0) {
        for k in 0..3 {
            sum[k] += p[k] as f32 / 255.0;
        }
        n += 1.0;
    }
    [
        sum[0] / n.max(1.0),
        sum[1] / n.max(1.0),
        sum[2] / n.max(1.0),
    ]
}

fn colors(t: &Rgba) -> usize {
    let mut seen: Vec<&[u8]> = t.chunks_exact(4).collect();
    seen.sort();
    seen.dedup();
    seen.len()
}

fn checks(tex: &Textures, content: &Content) {
    let mut bad = 0;
    let mut fail = |msg: String| {
        println!("FAIL {msg}");
        bad += 1;
    };
    let again = Textures::blocks();
    if again.layers != tex.layers || again.block_faces != tex.block_faces {
        fail("blocks() is not deterministic".into());
    }
    let mut worst: Vec<(f32, String)> = Vec::new();
    for (id, faces) in tex.block_faces.iter().enumerate() {
        let name = BLOCKS.get(id).map(|d| d.name).unwrap_or("Hearth");
        let leafy = matches!(id, 6 | 35 | 36);
        let clear = matches!(id, 0 | 8 | 13 | 22);
        for (f, layer) in faces.iter().enumerate() {
            let t = &tex.layers[*layer as usize];
            let holes = t.chunks_exact(4).filter(|p| p[3] == 0).count();
            let solid = t.chunks_exact(4).all(|p| p[3] == 255);
            if leafy && holes > 25 {
                fail(format!("{name} face {f}: {holes} holes"));
            }
            if leafy && t.chunks_exact(4).any(|p| p[3] != 0 && p[3] != 255) {
                fail(format!("{name} face {f}: partial alpha in leaves"));
            }
            if !leafy && !clear && !solid {
                fail(format!("{name} face {f}: not opaque"));
            }
            if id == 13 && t.chunks_exact(4).any(|p| !(160..=215).contains(&p[3])) {
                fail(format!("water face {f}: alpha out of range"));
            }
            if colors(t) > 20 {
                println!("note {name} face {f}: {} colours", colors(t));
            }
        }
        if let Some(def) = BLOCKS.get(id).filter(|_| id != 0) {
            let m = mean_rgb(&tex.layers[faces[1] as usize]);
            let d = (0..3).map(|k| (m[k] - def.color[k]).abs()).sum::<f32>();
            worst.push((d, format!("{name} side {m:.2?} vs {:.2?}", def.color)));
        }
    }
    for (first, count, secs) in &tex.animated {
        if (*first + *count) as usize > tex.layers.len() || *count < 2 || *secs <= 0.0 {
            fail(format!("animation {first}+{count} is out of range"));
        }
    }
    for i in 0..content.items.len() + 1 {
        let view = content.item(FIRST_ITEM + i as u16);
        let s = item_sprite(&view);
        let corner = |x: usize, y: usize| s[(y * TILE + x) * 4 + 3];
        if corner(0, 0) != 0 || corner(15, 0) != 0 || corner(0, 15) != 0 || corner(15, 15) != 0 {
            fail(format!("{}: sprite corner is not transparent", view.name));
        }
        if s.chunks_exact(4).filter(|p| p[3] > 0).count() < 20 {
            fail(format!("{}: sprite is nearly empty", view.name));
        }
    }
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("mean colour furthest from the BLOCKS colour:");
    for (d, line) in worst.iter().take(8) {
        println!("  {d:.2} {line}");
    }
    println!(
        "checks: {}",
        if bad == 0 {
            "all passed".into()
        } else {
            format!("{bad} failed")
        }
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: texture_sheet <out.ppm> [scale]");
        std::process::exit(2);
    };
    let scale: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(8);

    let start = Instant::now();
    let tex = Textures::blocks();
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    let content = Content::default_pack();
    let unknown = content.item(FIRST_ITEM + content.items.len() as u16);
    let mut items: Vec<ItemView> = content.items.iter().map(|i| content.item(i.id)).collect();
    items.push(content.item(3));
    items.push(unknown);
    let start = Instant::now();
    let sprites: Vec<Rgba> = items.iter().map(item_sprite).collect();
    let item_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{} layers, {} blocks, {} animations; blocks {build_ms:.2} ms, items {item_ms:.2} ms",
        tex.layers.len(),
        tex.block_faces.len(),
        tex.animated.len()
    );
    checks(&tex, &content);

    let t = TILE * scale;
    let pad = 2 * scale;
    let label = 14;
    let group = 3 * t + pad;
    let width = pad + 4 * group;
    let block_rows = tex.block_faces.len().div_ceil(4);
    // 3×3 patches: (label, block, face)
    let patches: [(&str, u8, usize); 24] = [
        ("stone", 3, 1),
        ("cobble", 12, 1),
        ("grass top", 1, 0),
        ("grass side", 1, 1),
        ("dirt", 2, 1),
        ("planks", 7, 1),
        ("log", 5, 1),
        ("brick", 10, 1),
        ("leaves", 6, 1),
        ("water", 13, 0),
        ("sand", 4, 1),
        ("snow", 11, 1),
        ("mossy cobble", 19, 1),
        ("birch log", 34, 1),
        ("birch leaves", 35, 1),
        ("pine leaves", 36, 1),
        ("basalt", 15, 1),
        ("marble", 16, 1),
        ("wool", 25, 1),
        ("ice", 22, 1),
        ("bedrock", 37, 1),
        ("emberstone", 9, 1),
        ("clay", 17, 1),
        ("terracotta", 18, 1),
    ];
    let patch_rows = patches.len().div_ceil(4);
    let per_row = (width - pad) / (t + pad / 2);
    let item_rows = sprites.len().div_ceil(per_row);
    let blocks_h = block_rows * (t + label + pad);
    let patches_h = patch_rows * (3 * t + label + pad);
    let anim_h = tex.animated.len() * (t + label + pad);
    let items_h = 2 * item_rows * (t + pad / 2) + label + pad;
    let mut sheet = Sheet::new(width, pad + blocks_h + patches_h + anim_h + items_h);

    let mut y0 = pad;
    println!("section blocks 0 {y0} {width} {blocks_h}");
    for (id, faces) in tex.block_faces.iter().enumerate() {
        let x = pad + (id % 4) * group;
        let y = y0 + (id / 4) * (t + label + pad);
        let name = BLOCKS.get(id).map(|d| d.name).unwrap_or("Hearth");
        sheet.text(&format!("{id} {name}"), x, y);
        for (f, layer) in faces.iter().enumerate() {
            sheet.tile(
                &tex.layers[*layer as usize],
                x + f * t,
                y + label,
                scale,
                Bg::Checker,
            );
        }
    }
    y0 += blocks_h;

    println!("section patches 0 {y0} {width} {patches_h}");
    for (i, (name, block, face)) in patches.iter().enumerate() {
        let x = pad + (i % 4) * group;
        let y = y0 + (i / 4) * (3 * t + label + pad);
        sheet.text(name, x, y);
        let layer = &tex.layers[tex.face(*block, *face) as usize];
        for ty in 0..3 {
            for tx in 0..3 {
                sheet.tile(layer, x + tx * t, y + label + ty * t, scale, Bg::Checker);
            }
        }
    }
    y0 += patches_h;

    println!("section animated 0 {y0} {width} {anim_h}");
    for (first, count, secs) in &tex.animated {
        sheet.text(
            &format!("layer {first} {count} frames {} ms", (secs * 1000.0) as u32),
            pad,
            y0,
        );
        for f in 0..*count as usize {
            let layer = &tex.layers[*first as usize + f];
            sheet.tile(
                layer,
                pad + f * (t + pad / 2),
                y0 + label,
                scale,
                Bg::Checker,
            );
        }
        y0 += t + label + pad;
    }

    println!("section items 0 {y0} {width} {items_h}");
    sheet.text("items", pad, y0);
    y0 += label;
    for (half, bg) in [[26, 24, 30], [214, 208, 190]].into_iter().enumerate() {
        for (i, s) in sprites.iter().enumerate() {
            let x = pad + (i % per_row) * (t + pad / 2);
            let y = y0 + (half * item_rows + i / per_row) * (t + pad / 2);
            sheet.tile(s, x, y, scale, Bg::Solid(bg));
        }
    }

    let mut out = Vec::with_capacity(sheet.px.len() + 32);
    write!(out, "P6\n{} {}\n255\n", sheet.w, sheet.h).unwrap();
    out.extend_from_slice(&sheet.px);
    if let Err(e) = std::fs::write(&path, out) {
        eprintln!("cannot write {path}: {e}");
        std::process::exit(1);
    }
    println!("wrote {path} ({}×{})", sheet.w, sheet.h);
}
