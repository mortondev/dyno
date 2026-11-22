//! Writes a preview sheet of every procedural texture as a PPM image.
//!
//! Usage: `cargo run -p dyno-client --example texture_sheet -- out.ppm [scale]`
//!
//! The sheet has five parts: every block face, 3×3 tiled patches of the key
//! materials (to check seams), the same materials as 8×8 walls at one screen
//! pixel per texel (to check that no grid shows from far away), the frames of
//! the animated textures, and every item sprite over a dark and a light
//! background. It also prints the checks that the eye does not catch: alpha
//! rules, determinism and build time.

use dyno_client::textures::{Rgba, TILE, Textures, item_sprite};
use dyno_world::BLOCKS;
use dyno_world::content::{
    ArmorType, Content, FIRST_ITEM, Gear, Icon, ItemView, Slot, Stats, Weapon, WeaponKind,
};
use std::io::Write;
use std::time::Instant;

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
        for p in px.as_chunks_mut::<3>().0 {
            *p = [34, 32, 40];
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
                        if ((x0 + x) / 8 + (y0 + y) / 8).is_multiple_of(2) {
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

/// The pixels of a tile, as RGBA.
fn pixels(t: &Rgba) -> &[[u8; 4]] {
    t.as_chunks::<4>().0
}

fn mean_rgb(t: &Rgba) -> [f32; 3] {
    let mut sum = [0f32; 3];
    let mut n = 0f32;
    for p in pixels(t).iter().filter(|p| p[3] > 0) {
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
        let clear = matches!(id, 0 | 8 | 13 | 22 | 54);
        for (f, layer) in faces.iter().enumerate() {
            let t = &tex.layers[*layer as usize];
            let holes = pixels(t).iter().filter(|p| p[3] == 0).count();
            let solid = pixels(t).iter().all(|p| p[3] == 255);
            if leafy && holes > TILE * TILE / 8 {
                fail(format!("{name} face {f}: {holes} holes"));
            }
            if leafy && pixels(t).iter().any(|p| p[3] != 0 && p[3] != 255) {
                fail(format!("{name} face {f}: partial alpha in leaves"));
            }
            if !leafy && !clear && !solid {
                fail(format!("{name} face {f}: not opaque"));
            }
            if id == 13 && pixels(t).iter().any(|p| !(160..=215).contains(&p[3])) {
                fail(format!("water face {f}: alpha out of range"));
            }
            // the leaded window: glass at about 0.6, lead lines opaque
            if id == 54 {
                let glass = pixels(t).iter().filter(|p| p[3] < 255).count();
                let lead = TILE * TILE - glass;
                if pixels(t).iter().any(|p| p[3] < 140) || glass < lead {
                    fail(format!("window face {f}: {glass} glass, {lead} lead"));
                }
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
        let far = TILE - 1;
        if corner(0, 0) != 0 || corner(far, 0) != 0 || corner(0, far) != 0 || corner(far, far) != 0
        {
            fail(format!("{}: sprite corner is not transparent", view.name));
        }
        if pixels(&s).iter().filter(|p| p[3] > 0).count() < 80 {
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
    let scale: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(4);

    let start = Instant::now();
    let tex = Textures::blocks();
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    let content = Content::default_pack();
    // the torch stick must sit in the strip that the renderer samples
    let torch = &tex.layers[tex.face(45, 1) as usize];
    let strip = |x: usize| {
        (0..TILE)
            .map(|y| torch[(y * TILE + x) * 4 + 3] as usize)
            .sum::<usize>()
    };
    if (TILE * 13 / 32..TILE * 19 / 32).any(|x| strip(x) != 255 * TILE) {
        println!("FAIL torch: the centre strip is not opaque");
    }
    let unknown = content.item(FIRST_ITEM + content.items.len() as u16);
    let mut items: Vec<ItemView> = content.items.iter().map(|i| content.item(i.id)).collect();
    items.push(content.item(3));
    items.push(unknown);
    // Equipment: every weapon kind, then every armour slot in each armour
    // type, then jewellery. Quality rises along each row.
    let mut gear: Vec<(Gear, [f32; 3], u8)> = Vec::new();
    let metals = [[0.80, 0.81, 0.85], [0.62, 0.45, 0.27], [0.45, 0.60, 0.85]];
    let kinds = [
        WeaponKind::Sword,
        WeaponKind::Axe,
        WeaponKind::Mace,
        WeaponKind::Dagger,
        WeaponKind::Staff,
        WeaponKind::Polearm,
        WeaponKind::Bow,
        WeaponKind::Wand,
        WeaponKind::Shield,
        WeaponKind::Offhand,
    ];
    for (i, kind) in kinds.into_iter().enumerate() {
        let weapon = Weapon {
            kind,
            min: 1.0,
            max: 2.0,
            speed: 2.0,
        };
        let g = Gear {
            slot: Slot::MainHand,
            armor_type: None,
            weapon: Some(weapon),
            armor: 0,
            stats: Stats::default(),
        };
        gear.push((g, metals[i % 3], (i % 5) as u8));
    }
    let slots = [
        Slot::Head,
        Slot::Shoulders,
        Slot::Back,
        Slot::Chest,
        Slot::Wrist,
        Slot::Hands,
        Slot::Waist,
        Slot::Legs,
        Slot::Feet,
    ];
    let types = [
        (ArmorType::Cloth, [0.45, 0.30, 0.70]),
        (ArmorType::Leather, [0.55, 0.38, 0.22]),
        (ArmorType::Mail, [0.55, 0.62, 0.70]),
        (ArmorType::Plate, [0.80, 0.70, 0.40]),
    ];
    for (t, (armor_type, color)) in types.into_iter().enumerate() {
        for slot in slots {
            let g = Gear {
                slot,
                armor_type: Some(armor_type),
                weapon: None,
                armor: 1,
                stats: Stats::default(),
            };
            gear.push((g, color, t as u8 + 1));
        }
    }
    for (slot, color) in [
        (Slot::Neck, [0.30, 0.75, 0.45]),
        (Slot::Ring, [0.85, 0.20, 0.25]),
        (Slot::Trinket, [0.35, 0.55, 0.95]),
    ] {
        let g = Gear {
            slot,
            armor_type: None,
            weapon: None,
            armor: 0,
            stats: Stats::default(),
        };
        gear.push((g, color, 3));
    }
    for (g, color, quality) in &gear {
        items.push(ItemView {
            id: 0,
            name: "gear",
            color: *color,
            icon: Icon::Lump,
            stack: 1,
            tool: None,
            food: 0,
            fuel: 0,
            block: None,
            light: 0,
            quality: *quality,
            value: 0,
            level: 1,
            gear: Some(g),
        });
    }
    let start = Instant::now();
    let sprites: Vec<Rgba> = items.iter().map(item_sprite).collect();
    let item_ms = start.elapsed().as_secs_f64() * 1000.0;
    for (s, view) in sprites.iter().zip(&items) {
        let far = TILE - 1;
        let clear = [(0, 0), (far, 0), (0, far), (far, far)]
            .iter()
            .all(|(x, y)| s[(y * TILE + x) * 4 + 3] == 0);
        if !clear || pixels(s).iter().filter(|p| p[3] > 0).count() < 80 {
            println!("FAIL {} sprite: corners or coverage", view.name);
        }
    }
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
    let patches: [(&str, u8, usize); 36] = [
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
        ("stone bricks", 47, 1),
        ("plaster", 48, 1),
        ("red roof", 49, 1),
        ("slate roof", 50, 1),
        ("thatch", 51, 1),
        ("timber", 52, 1),
        ("flagstones", 53, 0),
        ("window", 54, 1),
        ("door", 55, 1),
        ("banner", 56, 1),
        ("hay", 58, 1),
        ("flower bed", 64, 0),
    ];
    let patch_rows = patches.len().div_ceil(4);
    let per_row = (width - pad) / (t + pad / 2);
    let item_rows = sprites.len().div_ceil(per_row);
    let blocks_h = block_rows * (t + label + pad);
    let patches_h = patch_rows * (3 * t + label + pad);
    let far_tiles = 8;
    let far_cols = (width - pad) / (far_tiles * TILE + pad);
    let far_h = patches.len().div_ceil(far_cols) * (far_tiles * TILE + label + pad);
    let anim_h = tex.animated.len() * (t + label + pad);
    let items_h = 2 * item_rows * (t + pad / 2) + label + pad;
    let mut sheet = Sheet::new(width, pad + blocks_h + patches_h + far_h + anim_h + items_h);

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

    println!("section far 0 {y0} {width} {far_h}");
    for (i, (name, block, face)) in patches.iter().enumerate() {
        let x = pad + (i % far_cols) * (far_tiles * TILE + pad);
        let y = y0 + (i / far_cols) * (far_tiles * TILE + label + pad);
        sheet.text(name, x, y);
        let layer = &tex.layers[tex.face(*block, *face) as usize];
        for ty in 0..far_tiles {
            for tx in 0..far_tiles {
                sheet.tile(layer, x + tx * TILE, y + label + ty * TILE, 1, Bg::Checker);
            }
        }
    }
    y0 += far_h;

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
