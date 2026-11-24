//! The content pack the server runs and streams to clients
//! (docs/design/data-and-streaming.md §1).
//!
//! A pack is a directory with `pack.json` (name, version, entry points) and
//! the files it names: the rules JSON, UI strings, the title stage, and later
//! textures, models, sounds and fonts. The server hashes every file into a
//! [`Manifest`]; clients ask for the hashes they lack.

use dyno_protocol::{self as proto, AssetRef, Manifest};
use dyno_world::content::Content;
use dyno_world::layout::Layout;
use dyno_world::stamp::Stamps;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/embedded_pack.rs"));

/// The rules path when `pack.json` names none.
pub const DEFAULT_RULES: &str = "rules/hearthblock.json";

/// `pack.json`.
#[derive(Debug, Deserialize)]
struct PackFile {
    name: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, String>,
    #[serde(default)]
    stream: Vec<String>,
}

pub struct Pack {
    pub manifest: Manifest,
    /// SHA-256 of the manifest: two packs with the same hash are the same
    pub hash: [u8; 32],
    /// the parsed rules (files[0])
    pub content: Content,
    /// the world's layout and stamps (`world/layout.json`, `stamps/`):
    /// what the server builds its world from. They stay on the server;
    /// clients are sent the world itself.
    pub layout: Option<Arc<Layout>>,
    pub stamps: Arc<Stamps>,
    files: HashMap<[u8; 32], Stored>,
}

/// The layout path when `pack.json` names none.
pub const DEFAULT_LAYOUT: &str = "world/layout.json";

/// A file as it is, and as it travels.
struct Stored {
    raw: Arc<[u8]>,
    codec: u8,
    wire: Arc<[u8]>,
}

/// The pack build: source files become what is streamed. Each
/// `anims/<skeleton>.json` (clips as artists edit them) becomes
/// `anims/<skeleton>.clips`, thinned and quantised
/// ([`dyno_protocol::clips`]); the source stays out of the manifest.
pub fn compile(files: BTreeMap<String, Vec<u8>>) -> Result<BTreeMap<String, Vec<u8>>, String> {
    Ok(compile_world(files)?.0)
}

type Compiled = (BTreeMap<String, Vec<u8>>, Option<Layout>, Stamps);

/// [`compile`], with the world data taken out: the layout and the stamps
/// are parsed and checked against each other and stay on the server, and
/// the rules' places (`"place": "miners_rest"`) become the offsets from the
/// spawn hearth that the rules' readers work with.
fn compile_world(mut files: BTreeMap<String, Vec<u8>>) -> Result<Compiled, String> {
    let meta: Option<PackFile> = files
        .get("pack.json")
        .and_then(|b| serde_json::from_slice(b).ok());
    let entry = |k: &str, default: &str| {
        meta.as_ref()
            .and_then(|m| m.entries.get(k).cloned())
            .unwrap_or_else(|| default.into())
    };
    let stamps = Stamps::from_files(files.iter().map(|(p, b)| (p.as_str(), b.as_slice())))?;
    let layout_path = entry("layout", DEFAULT_LAYOUT);
    let layout = match files.get(&layout_path) {
        Some(b) => {
            let text = std::str::from_utf8(b).map_err(|e| format!("{layout_path}: {e}"))?;
            let l = Layout::from_json(text).map_err(|e| format!("{layout_path}: {e}"))?;
            l.check_stamps(&stamps)
                .map_err(|e| format!("{layout_path}: {e}"))?;
            Some(l)
        }
        None => None,
    };
    let rules_path = entry("rules", DEFAULT_RULES);
    if layout.is_none()
        && files
            .get(&rules_path)
            .is_some_and(|r| r.windows(8).any(|w| w == b"\"place\":"))
    {
        return Err(format!(
            "{rules_path} places things at places of a layout, but the pack has no {layout_path}"
        ));
    }
    if let (Some(l), Some(rules)) = (&layout, files.get_mut(&rules_path)) {
        let mut v: serde_json::Value =
            serde_json::from_slice(rules).map_err(|e| format!("{rules_path}: {e}"))?;
        l.resolve_rules(&mut v)
            .map_err(|e| format!("{rules_path}: {e}"))?;
        *rules = serde_json::to_vec(&v).map_err(|e| format!("{rules_path}: {e}"))?;
    }
    // the water is where the layout has it
    let render_path = entry("render", "render.json");
    if let (Some(l), Some(render)) = (&layout, files.get_mut(&render_path)) {
        let mut v: serde_json::Value =
            serde_json::from_slice(render).map_err(|e| format!("{render_path}: {e}"))?;
        l.place_water(&mut v)
            .map_err(|e| format!("{render_path}: {e}"))?;
        *render = serde_json::to_vec(&v).map_err(|e| format!("{render_path}: {e}"))?;
    }
    let mut out = BTreeMap::new();
    for (path, bytes) in files {
        if path.starts_with("stamps/") || path.starts_with("world/") {
            continue;
        }
        match path
            .strip_prefix("anims/")
            .and_then(|p| p.strip_suffix(".json"))
        {
            Some(name) if !name.contains('/') => {
                let baked = proto::clips::encode(&bytes).map_err(|e| format!("{path}: {e}"))?;
                out.insert(format!("anims/{name}.clips"), baked);
            }
            _ => {
                out.insert(path, bytes);
            }
        }
    }
    Ok((out, layout, stamps))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

type Encoded = (u8, Arc<[u8]>);

/// Every file's wire encoding, by hash, made once per process: packs are
/// built again on `/reload` and in tests, and most files stay the same.
fn encode_all(files: &[([u8; 32], &[u8])]) -> HashMap<[u8; 32], Encoded> {
    type Memo = std::sync::Mutex<HashMap<[u8; 32], Encoded>>;
    static MEMO: std::sync::OnceLock<Memo> = std::sync::OnceLock::new();
    let memo = MEMO.get_or_init(Default::default);
    let mut todo: Vec<([u8; 32], &[u8])> = {
        let m = memo.lock().unwrap_or_else(|e| e.into_inner());
        files
            .iter()
            .filter(|(h, _)| !m.contains_key(h))
            .copied()
            .collect()
    };
    todo.sort_by_key(|(h, _)| *h);
    todo.dedup_by_key(|(h, _)| *h);
    // the largest first, spread over the cores
    todo.sort_by_key(|(_, b)| std::cmp::Reverse(b.len()));
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 16);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: Vec<([u8; 32], Encoded)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((h, b)) = todo.get(i) else { break };
                        let (codec, wire) = proto::wire::encode(b);
                        out.push((*h, (codec, Arc::from(wire))));
                    }
                    out
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("encoding a pack file"))
            .collect()
    });
    let mut m = memo.lock().unwrap_or_else(|e| e.into_inner());
    m.extend(done);
    files
        .iter()
        .filter_map(|(h, _)| Some((*h, m.get(h)?.clone())))
        .collect()
}

impl Pack {
    /// Build a pack from its files (pack path to bytes). Sources are
    /// compiled first ([`compile`]).
    pub fn from_files(files: BTreeMap<String, Vec<u8>>) -> Result<Pack, String> {
        let (files, layout, stamps) = compile_world(files)?;
        let meta: PackFile = match files.get("pack.json") {
            Some(b) => serde_json::from_slice(b).map_err(|e| format!("pack.json: {e}"))?,
            None => return Err("the pack has no pack.json".into()),
        };
        let rules_path = meta
            .entries
            .get("rules")
            .cloned()
            .unwrap_or_else(|| DEFAULT_RULES.into());
        let rules = files
            .get(&rules_path)
            .ok_or_else(|| format!("the pack has no rules file {rules_path}"))?;
        let text = std::str::from_utf8(rules).map_err(|e| format!("{rules_path}: {e}"))?;
        let content = Content::from_json(text).map_err(|e| format!("{rules_path}: {e}"))?;
        let mut refs = Vec::with_capacity(files.len());
        let mut stored = HashMap::with_capacity(files.len());
        // the rules first; the rest in path order
        let order: Vec<&String> = std::iter::once(&rules_path)
            .chain(files.keys().filter(|p| **p != rules_path))
            .collect();
        let hashes: Vec<([u8; 32], &[u8])> = order
            .iter()
            .map(|p| (sha256(&files[*p]), files[*p].as_slice()))
            .collect();
        let encoded = encode_all(&hashes);
        for (path, (hash, bytes)) in order.into_iter().zip(&hashes) {
            let size = u32::try_from(bytes.len()).map_err(|_| format!("{path} is too large"))?;
            let (codec, wire) = encoded[hash].clone();
            refs.push(AssetRef {
                path: path.clone(),
                hash: *hash,
                size,
                codec,
                wire: wire.len() as u32,
            });
            stored.insert(
                *hash,
                Stored {
                    raw: Arc::from(*bytes),
                    codec,
                    wire,
                },
            );
        }
        let manifest = Manifest {
            pack: meta.name,
            version: meta.version,
            files: refs,
            stream: meta.stream,
        };
        let hash = sha256(&proto::encode(&manifest));
        Ok(Pack {
            manifest,
            hash,
            content,
            layout: layout.map(Arc::new),
            stamps: Arc::new(stamps),
            files: stored,
        })
    }

    fn builtin_files() -> BTreeMap<String, Vec<u8>> {
        EMBEDDED
            .iter()
            .map(|(p, b)| (p.to_string(), b.to_vec()))
            .collect()
    }

    /// The built-in pack as it is authored: every file of `content/`, with
    /// the world's layout and stamps (which clients are never sent). What a
    /// pack directory holds; tests write it out to make packs of their own.
    pub fn builtin_source() -> BTreeMap<String, Vec<u8>> {
        Self::builtin_files()
    }

    /// The pack built into this program (the workspace's `content/`).
    pub fn builtin() -> Pack {
        Pack::from_files(Self::builtin_files()).expect("the built-in pack is valid")
    }

    /// Every file under a pack directory.
    pub fn read_dir(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
        fn walk(
            root: &Path,
            dir: &Path,
            out: &mut BTreeMap<String, Vec<u8>>,
        ) -> std::io::Result<()> {
            for e in std::fs::read_dir(dir)? {
                let e = e?;
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name.ends_with(".tmp") {
                    continue;
                }
                if p.is_dir() {
                    walk(root, &p, out)?;
                } else if let Ok(rel) = p.strip_prefix(root) {
                    out.insert(rel.to_string_lossy().replace('\\', "/"), std::fs::read(&p)?);
                }
            }
            Ok(())
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out).map_err(|e| format!("{}: {e}", root.display()))?;
        Ok(out)
    }

    /// The pack for `--content`:
    /// - nothing: the built-in pack;
    /// - a directory: that pack;
    /// - a JSON file: the built-in pack with these rules (tests, custom rules);
    /// - the old `content/hearthblock.json`, now moved: the pack around it.
    pub fn load(path: Option<&Path>) -> Result<Pack, String> {
        let Some(p) = path else {
            return Ok(Pack::builtin());
        };
        if p.is_dir() {
            return Pack::from_files(Pack::read_dir(p)?);
        }
        if p.is_file() {
            let rules = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            return Pack::with_rules(rules);
        }
        // the rules used to sit at the top of the pack; they now live under rules/
        let dir = p.parent().unwrap_or(Path::new("."));
        if let Some(name) = p.file_name() {
            if dir.join("pack.json").is_file() {
                return Pack::from_files(Pack::read_dir(dir)?);
            }
            let moved = dir.join("rules").join(name);
            if moved.is_file() {
                let rules =
                    std::fs::read(&moved).map_err(|e| format!("{}: {e}", moved.display()))?;
                return Pack::with_rules(rules);
            }
        }
        Err(format!("{}: no such file or pack", p.display()))
    }

    /// The built-in pack with other rules.
    pub fn with_rules(rules: Vec<u8>) -> Result<Pack, String> {
        let mut files = Self::builtin_files();
        let meta: PackFile = serde_json::from_slice(&files["pack.json"])
            .map_err(|e| format!("built-in pack.json: {e}"))?;
        let path = meta
            .entries
            .get("rules")
            .cloned()
            .unwrap_or_else(|| DEFAULT_RULES.into());
        files.insert(path, rules);
        Pack::from_files(files)
    }

    /// A file by hash.
    pub fn get(&self, hash: &[u8; 32]) -> Option<Arc<[u8]>> {
        self.files.get(hash).map(|f| f.raw.clone())
    }

    /// A file by hash as it travels: its codec and encoded bytes.
    pub fn wire(&self, hash: &[u8; 32]) -> Option<(u8, Arc<[u8]>)> {
        self.files.get(hash).map(|f| (f.codec, f.wire.clone()))
    }

    /// A short, printable identity (the first 16 hex digits of the hash).
    pub fn id(&self) -> String {
        proto::hex(&self.hash[..8])
    }

    /// Every file, by pack path (for a local client that hosts its own world).
    pub fn files(&self) -> Vec<(String, Arc<[u8]>)> {
        self.manifest
            .files
            .iter()
            .filter_map(|f| Some((f.path.clone(), self.get(&f.hash)?)))
            .collect()
    }

    /// Total bytes of every file.
    pub fn bytes(&self) -> u64 {
        self.manifest.files.iter().map(|f| f.size as u64).sum()
    }

    /// Total bytes of every file on the wire.
    pub fn wire_bytes(&self) -> u64 {
        self.manifest.files.iter().map(|f| f.wire as u64).sum()
    }
}

/// One session's queue of pack files to send, and its bandwidth budget.
/// The session writes game messages first and then asks for the chunks the
/// budget allows, so play never waits behind a download.
pub struct Outbox {
    queue: std::collections::VecDeque<([u8; 32], usize)>,
    budget: f64,
    rate: f64,
    last: std::time::Instant,
    /// asset bytes sent so far
    pub sent: u64,
}

/// Smallest chunk worth a message (unless the file ends sooner).
const MIN_CHUNK: usize = 16 * 1024;

impl Outbox {
    /// `rate` in bytes per second.
    pub fn new(rate: usize) -> Outbox {
        Outbox {
            queue: Default::default(),
            budget: proto::MAX_ASSET_CHUNK as f64,
            rate: rate.max(1024) as f64,
            last: std::time::Instant::now(),
            sent: 0,
        }
    }

    /// Queue files the client asked for (unknown hashes are ignored).
    pub fn want(&mut self, hashes: &[[u8; 32]], pack: &Pack) {
        for h in hashes.iter().take(proto::MAX_WANT) {
            if self.queue.len() >= proto::MAX_WANT {
                break;
            }
            if pack.wire(h).is_some() && !self.queue.iter().any(|(q, _)| q == h) {
                self.queue.push_back((*h, 0));
            }
        }
    }

    /// Change the rate (the player started playing), keeping no more saved
    /// budget than the new rate allows.
    pub fn set_rate(&mut self, rate: usize) {
        self.rate = rate.max(1024) as f64;
        self.budget = self.budget.min(proto::MAX_ASSET_CHUNK as f64);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// The chunks the budget allows now.
    pub fn chunks(&mut self, pack: &Pack) -> Vec<proto::ServerMsg> {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        let cap = (proto::MAX_ASSET_CHUNK as f64).max(self.rate / 10.0);
        self.budget = (self.budget + self.rate * dt).min(cap);
        let mut out = Vec::new();
        while let Some(&(hash, offset)) = self.queue.front() {
            let Some((_, bytes)) = pack.wire(&hash) else {
                // gone after a reload
                self.queue.pop_front();
                continue;
            };
            let left = bytes.len().saturating_sub(offset);
            let min = left.min(MIN_CHUNK);
            if self.budget < min as f64 {
                break;
            }
            let n = left
                .min(proto::MAX_ASSET_CHUNK)
                .min((self.budget as usize).max(min));
            out.push(proto::ServerMsg::Asset {
                hash,
                offset: offset as u32,
                total: bytes.len() as u32,
                data: bytes[offset..offset + n].to_vec(),
            });
            self.budget -= n as f64;
            self.sent += n as u64;
            if offset + n >= bytes.len() {
                self.queue.pop_front();
            } else if let Some(f) = self.queue.front_mut() {
                f.1 = offset + n;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbox_keeps_to_its_budget() {
        let mut files = Pack::builtin_files();
        // noise, which travels as it is
        let big: Vec<u8> = (0..600_000u32)
            .scan(0x9E37_79B9u32, |x, _| {
                *x ^= *x << 13;
                *x ^= *x >> 17;
                *x ^= *x << 5;
                Some((*x >> 24) as u8)
            })
            .collect();
        files.insert("textures/big.bin".into(), big);
        let p = Pack::from_files(files).unwrap();
        let rules = p.manifest.file("textures/big.bin").unwrap().clone();
        let mut o = Outbox::new(256 * 1024);
        o.want(&[rules.hash, rules.hash, [7; 32]], &p);
        // one burst, then nothing until time passes
        let first = o.chunks(&p);
        assert_eq!(first.len(), 1);
        assert!(o.chunks(&p).is_empty());
        let mut got = Vec::new();
        for m in first {
            if let proto::ServerMsg::Asset { data, offset, .. } = m {
                assert_eq!(offset, 0);
                assert!(data.len() <= proto::MAX_ASSET_CHUNK);
                got.extend(data);
            }
        }
        let t0 = std::time::Instant::now();
        while !o.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(15));
            for m in o.chunks(&p) {
                if let proto::ServerMsg::Asset {
                    data,
                    offset,
                    total,
                    ..
                } = m
                {
                    assert_eq!(offset as usize, got.len());
                    assert_eq!(total, rules.size);
                    got.extend(data);
                }
            }
        }
        assert_eq!(rules.codec, proto::wire::RAW);
        assert_eq!(rules.wire, rules.size);
        assert_eq!(sha256(&got), rules.hash);
        // about 256 KB/s after the first 64 KB burst
        let secs = t0.elapsed().as_secs_f64();
        let expect = (rules.size as f64 - proto::MAX_ASSET_CHUNK as f64) / (256.0 * 1024.0);
        assert!(secs > expect * 0.8, "{secs} s for {expect} s of budget");
        assert_eq!(o.sent, rules.size as u64);
    }

    #[test]
    fn builtin_pack_has_rules_first_and_hashes() {
        let p = Pack::builtin();
        assert_eq!(p.manifest.pack, "hearthblock");
        assert_eq!(p.manifest.files[0].path, DEFAULT_RULES);
        assert!(p.manifest.file("pack.json").is_some());
        for f in &p.manifest.files {
            let b = p.get(&f.hash).unwrap();
            assert_eq!(b.len() as u32, f.size);
            assert_eq!(sha256(&b), f.hash);
            // what travels decodes to the file
            let (codec, wire) = p.wire(&f.hash).unwrap();
            assert_eq!((codec, wire.len() as u32), (f.codec, f.wire));
            assert!(f.wire <= f.size);
            let back = proto::wire::decode(codec, &wire, f.size as usize).unwrap();
            assert_eq!(&back[..], &b[..], "{}", f.path);
        }
        // the same files give the same hash
        assert_eq!(Pack::builtin().hash, p.hash);
        // a first join stays small (docs/design/data-and-streaming.md §1)
        let first = p.manifest.download_bytes();
        eprintln!(
            "built-in pack: {} files, {} bytes, {} on the wire, a first join downloads {first}",
            p.manifest.files.len(),
            p.bytes(),
            p.wire_bytes()
        );
        assert!(first < 4_000_000, "a first join downloads {first} bytes");
        // the clips are compiled, and their source is not streamed
        assert!(p.manifest.file("anims/biped.clips").is_some());
        assert!(p.manifest.file("anims/biped.json").is_none());
    }

    #[test]
    fn other_rules_change_the_hash() {
        let a = Pack::builtin();
        let text = dyno_world::content::DEFAULT_JSON.replacen("\"version\": ", "\"version\": 1", 1);
        let b = Pack::with_rules(text.into_bytes()).unwrap();
        assert_ne!(a.hash, b.hash);
        assert_ne!(a.manifest.files[0].hash, b.manifest.files[0].hash);
        assert_eq!(a.manifest.files.len(), b.manifest.files.len());
    }

    #[test]
    fn loads_a_directory_and_the_old_rules_path() {
        let dir = std::env::temp_dir().join(format!("dyno-pack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("rules")).unwrap();
        std::fs::write(
            dir.join("pack.json"),
            r#"{"name":"t","version":3,"entries":{"rules":"rules/hearthblock.json"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("rules/hearthblock.json"),
            dyno_world::content::DEFAULT_JSON,
        )
        .unwrap();
        std::fs::write(dir.join("extra.txt"), b"hello").unwrap();
        // the rules stand on the layout's places: the world's data comes along
        // (and stays out of the manifest)
        for (path, bytes) in EMBEDDED {
            if path.starts_with("world/") || path.starts_with("stamps/") {
                let to = dir.join(path);
                std::fs::create_dir_all(to.parent().unwrap()).unwrap();
                std::fs::write(to, bytes).unwrap();
            }
        }
        let p = Pack::load(Some(&dir)).unwrap();
        assert!(p.layout.is_some() && p.stamps.get("inn").is_some());
        assert_eq!(p.manifest.pack, "t");
        assert_eq!(p.manifest.version, 3);
        assert_eq!(p.manifest.files.len(), 3);
        // `--content <dir>/hearthblock.json`, as servers were started before the move
        let old = Pack::load(Some(&dir.join("hearthblock.json"))).unwrap();
        assert_eq!(old.hash, p.hash);
        // a file is the built-in pack with those rules
        let rules = Pack::load(Some(&dir.join("rules/hearthblock.json"))).unwrap();
        assert_eq!(rules.manifest.pack, "hearthblock");
        assert!(Pack::load(Some(&dir.join("missing/x.json"))).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
