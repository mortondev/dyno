//! Block textures from the pack: `blocks.json` names a PNG for each face of
//! each block (`textures/*.png`, 32 px a side; an animated texture is a
//! vertical strip of frames), and they are stacked here into the layers of
//! one array texture.
//!
//! Layer numbers depend only on `blocks.json`, never on which PNGs have
//! arrived, so a texture that arrives late only needs uploading again: the
//! meshes that point at its layer stay right. A face whose PNG is missing
//! shows the block's plain colour until it arrives.

use crate::assets;
use dyno_world as w;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

pub const TILE: usize = 32;
pub type Rgba = [u8; TILE * TILE * 4];

/// The pack file that names every block's textures.
pub const BLOCKS_JSON: &str = "blocks.json";

pub struct Textures {
    /// every texture, as an index into this list
    pub layers: Vec<Rgba>,
    /// per block ID: layer for [top, side, bottom]
    pub block_faces: Vec<[u16; 3]>,
    /// layers that animate: (first layer, frame count, seconds per frame)
    pub animated: Vec<(u16, u16, f32)>,
    /// how many of the texture files it names were in the pack
    pub loaded: usize,
    /// how many files it names
    pub wanted: usize,
    /// which bytes it was built from (see [`fingerprint`])
    source: Vec<usize>,
}

/// The files the textures are built from, as the addresses of their bytes
/// in the asset store (0 when missing): it changes exactly when one of them
/// is added or replaced.
fn fingerprint(file: &BlocksFile) -> Vec<usize> {
    let addr = |p: &str| assets::get(p).map(|b| b.as_ptr() as usize).unwrap_or(0);
    let mut v = vec![addr(BLOCKS_JSON)];
    for b in &file.blocks {
        if let Some(f) = &b.textures {
            v.extend(f.all().map(addr));
        }
    }
    v.extend(file.unknown.as_deref().map(addr));
    v
}

fn read_blocks_file() -> BlocksFile {
    assets::text(BLOCKS_JSON)
        .and_then(|t| BlocksFile::parse(&t).map_err(|e| eprintln!("{e}")).ok())
        .unwrap_or_default()
}

/// `blocks.json`: the textures of each block's faces.
#[derive(Deserialize, Default)]
pub struct BlocksFile {
    /// texture for block IDs the file does not list
    #[serde(default)]
    pub unknown: Option<String>,
    /// textures that animate, by path
    #[serde(default)]
    pub animated: HashMap<String, Anim>,
    pub blocks: Vec<BlockEntry>,
}

#[derive(Deserialize, Clone, Copy)]
pub struct Anim {
    pub frames: u16,
    /// seconds per frame
    pub frame_time: f32,
}

#[derive(Deserialize)]
pub struct BlockEntry {
    pub id: u8,
    #[serde(default)]
    pub key: String,
    /// none for blocks that are never drawn (air)
    #[serde(default)]
    pub textures: Option<Faces>,
}

#[derive(Deserialize)]
pub struct Faces {
    pub top: String,
    pub side: String,
    pub bottom: String,
}

impl Faces {
    pub fn all(&self) -> [&str; 3] {
        [&self.top, &self.side, &self.bottom]
    }
}

impl BlocksFile {
    pub fn parse(text: &str) -> Result<BlocksFile, String> {
        serde_json::from_str(text).map_err(|e| format!("{BLOCKS_JSON}: {e}"))
    }
}

/// Decode a PNG to 8-bit RGBA: (width, height, pixels).
pub fn decode_png(bytes: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    let mut d = png::Decoder::new(std::io::Cursor::new(bytes));
    d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == w * h * 4).then_some((w, h, rgba))
}

/// A tile of one flat colour (sRGB, 0..1).
fn flat(c: [f32; 3], alpha: f32) -> Rgba {
    let mut t = [0u8; TILE * TILE * 4];
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    for p in t.as_chunks_mut::<4>().0 {
        p.copy_from_slice(&[b(c[0]), b(c[1]), b(c[2]), b(alpha)]);
    }
    t
}

impl Textures {
    /// Build every block texture from the pack in the asset store.
    pub fn load() -> Textures {
        let file = read_blocks_file();
        let mut t = Self::from_file(&file);
        t.source = fingerprint(&file);
        t
    }

    fn from_file(file: &BlocksFile) -> Textures {
        let mut t = Textures {
            // layer 0 is clear: blocks that are never drawn
            layers: vec![[0; TILE * TILE * 4]],
            block_faces: vec![[0; 3]; w::BLOCKS.len()],
            animated: Vec::new(),
            loaded: 0,
            wanted: 0,
            source: Vec::new(),
        };
        let mut placed: HashMap<String, u16> = HashMap::new();
        // blocks the file gives textures, or says have none (air)
        let mut listed = vec![false; t.block_faces.len()];
        for b in &file.blocks {
            if b.id as usize >= t.block_faces.len() {
                continue;
            }
            let Some(faces) = &b.textures else {
                listed[b.id as usize] = true;
                continue;
            };
            let d = w::block(b.id);
            let mut f = [0u16; 3];
            for (k, path) in faces.all().into_iter().enumerate() {
                f[k] = match placed.get(path) {
                    Some(l) => *l,
                    None => {
                        let l = t.add(file, path, d.color, d.alpha);
                        placed.insert(path.to_string(), l);
                        l
                    }
                };
            }
            t.block_faces[b.id as usize] = f;
            listed[b.id as usize] = true;
        }
        // blocks without textures (no blocks.json yet): their plain colour
        for (id, done) in listed.iter().enumerate().skip(1) {
            if !done {
                let d = w::block(id as u8);
                let l = t.layers.len() as u16;
                t.layers.push(flat(d.color, d.alpha));
                t.block_faces[id] = [l; 3];
            }
        }
        // the last layer stands in for unknown block IDs
        let grey = [0.45, 0.42, 0.48];
        match file.unknown.as_deref() {
            Some(p) => match placed.get(p) {
                Some(&l) => {
                    let copy = t.layers[l as usize];
                    t.layers.push(copy);
                }
                None => {
                    t.add(file, p, grey, 1.0);
                }
            },
            None => t.layers.push(flat(grey, 1.0)),
        }
        t
    }

    /// Add the layers of one texture file (or plain tiles of `color` while
    /// it is missing). Returns its first layer.
    fn add(&mut self, file: &BlocksFile, path: &str, color: [f32; 3], alpha: f32) -> u16 {
        let anim = file.animated.get(path).copied();
        let frames = anim.map(|a| a.frames.max(1)).unwrap_or(1) as usize;
        let first = self.layers.len() as u16;
        self.wanted += 1;
        let img = assets::get(path)
            .and_then(|b| decode_png(&b))
            .filter(|(w, h, _)| *w == TILE && *h == TILE * frames);
        match img {
            Some((_, _, px)) => {
                self.loaded += 1;
                for f in px.as_chunks::<{ TILE * TILE * 4 }>().0 {
                    self.layers.push(*f);
                }
            }
            None => self.layers.extend((0..frames).map(|_| flat(color, alpha))),
        }
        if let Some(a) = anim.filter(|_| frames > 1) {
            self.animated.push((first, frames as u16, a.frame_time));
        }
        first
    }

    /// Layer for a block face: face 0 = top, 1 = side, 2 = bottom.
    ///
    /// An unknown block ID gets the last layer, which is a neutral tile.
    pub fn face(&self, block: u8, face: usize) -> u16 {
        match self.block_faces.get(block as usize) {
            Some(faces) => faces[face.min(2)],
            None => (self.layers.len() - 1) as u16,
        }
    }

    /// Same layer numbers for every block and frame: only pixels differ.
    pub fn same_layout(&self, other: &Textures) -> bool {
        self.layers.len() == other.layers.len()
            && self.block_faces == other.block_faces
            && self.animated == other.animated
    }
}

/// The textures in use. Mesh building reads this for every face on many
/// threads, so it is a plain pointer load; a replaced set is leaked, which
/// happens only a few times a session, when textures arrive.
static CURRENT: AtomicPtr<Textures> = AtomicPtr::new(std::ptr::null_mut());
static SWAP: Mutex<()> = Mutex::new(());
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Goes up each time the textures change, so caches built from them (the
/// GPU array, the icon atlas) know to build again.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

/// The block textures now in use, built from the pack on first use.
pub fn current() -> &'static Textures {
    let p = CURRENT.load(Ordering::Acquire);
    if !p.is_null() {
        // SAFETY: only ever set from a leaked Box, never freed
        return unsafe { &*p };
    }
    let _g = SWAP.lock().unwrap();
    let p = CURRENT.load(Ordering::Acquire);
    if !p.is_null() {
        // SAFETY: as above
        return unsafe { &*p };
    }
    let t: &'static Textures = Box::leak(Box::new(Textures::load()));
    CURRENT.store(t as *const Textures as *mut Textures, Ordering::Release);
    t
}

/// Build the textures again from the pack (more of it has arrived).
/// Returns true when any pixel or layer changed.
pub fn reload() -> bool {
    let _g = SWAP.lock().unwrap();
    let p = CURRENT.load(Ordering::Acquire);
    // SAFETY: as in `current`
    if !p.is_null() && unsafe { &*p }.source == fingerprint(&read_blocks_file()) {
        return false;
    }
    let new = Textures::load();
    // SAFETY: as in `current`
    let changed = p.is_null() || {
        let old = unsafe { &*p };
        !old.same_layout(&new) || old.layers != new.layers
    };
    if changed {
        let t: &'static Textures = Box::leak(Box::new(new));
        CURRENT.store(t as *const Textures as *mut Textures, Ordering::Release);
        GENERATION.fetch_add(1, Ordering::AcqRel);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_files_fall_back_to_plain_colour() {
        let file = BlocksFile::parse(
            r#"{"animated": {"textures/nope_anim.png": {"frames": 3, "frame_time": 0.2}},
                "blocks": [{"id": 0}, {"id": 1, "textures": {"top": "textures/nope_a.png",
                "side": "textures/nope_anim.png", "bottom": "textures/nope_a.png"}}]}"#,
        )
        .unwrap();
        let t = Textures::from_file(&file);
        // clear, one still, three frames, a plain tile for each block the
        // file leaves out, the unknown tile
        assert_eq!(t.layers.len(), 1 + 1 + 3 + (w::BLOCKS.len() - 2) + 1);
        assert_eq!(t.block_faces[0], [0; 3]);
        assert_eq!(t.block_faces[1], [1, 2, 1]);
        assert_eq!(t.block_faces[2], [5; 3]);
        let dirt = w::block(2).color;
        assert_eq!(t.layers[5][0], (dirt[0] * 255.0 + 0.5) as u8);
        assert_eq!(t.animated, vec![(2, 3, 0.2)]);
        assert_eq!(t.loaded, 0);
        let c = w::block(1).color;
        assert_eq!(t.layers[1][0], (c[0] * 255.0 + 0.5) as u8);
        assert_eq!(t.layers[1][3], 255);
    }
}
