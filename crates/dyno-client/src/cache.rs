//! Pack files the server streamed, stored by SHA-256 under the platform
//! cache directory, so a second join with nothing changed downloads nothing
//! (docs/design/data-and-streaming.md §1). Every failure here is soft: the
//! file is fetched again.

use dyno_protocol::{self as proto, AssetRef, Manifest};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Where pack files live on this machine.
#[derive(Debug, Clone)]
pub struct Cache {
    dir: Option<PathBuf>,
}

impl Default for Cache {
    /// `DYNO_CACHE_DIR`, else the platform cache directory.
    fn default() -> Cache {
        let dir = std::env::var_os("DYNO_CACHE_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                directories::ProjectDirs::from("io", "mortondev", "dyno")
                    .map(|d| d.cache_dir().to_path_buf())
            });
        Cache { dir }
    }
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

impl Cache {
    /// A cache in a chosen directory (tests, tools).
    pub fn at(dir: impl Into<PathBuf>) -> Cache {
        Cache {
            dir: Some(dir.into()),
        }
    }

    /// No cache: everything is fetched.
    pub fn none() -> Cache {
        Cache { dir: None }
    }

    fn file(&self, hash: &[u8; 32]) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join("assets").join(proto::hex(hash)))
    }

    /// A cached file, if it is there and intact.
    pub fn read(&self, f: &AssetRef) -> Option<Vec<u8>> {
        let bytes = std::fs::read(self.file(&f.hash)?).ok()?;
        (bytes.len() == f.size as usize && sha256(&bytes) == f.hash).then_some(bytes)
    }

    /// Keep a file (already checked against its hash).
    pub fn write(&self, hash: &[u8; 32], bytes: &[u8]) {
        let Some(path) = self.file(hash) else { return };
        let write = || -> std::io::Result<()> {
            if let Some(d) = path.parent() {
                std::fs::create_dir_all(d)?;
            }
            let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
            std::fs::write(&tmp, bytes)?;
            std::fs::rename(&tmp, &path)
        };
        if let Err(e) = write() {
            eprintln!("cache: cannot keep {}: {e}", path.display());
        }
    }

    fn last_path(&self) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join("last.manifest"))
    }

    /// Remember the pack of the last world joined (for the title screen).
    pub fn save_manifest(&self, m: &Manifest) {
        if let Some(p) = self.last_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let tmp = p.with_extension("tmp");
            if std::fs::write(&tmp, proto::encode(m)).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }

    /// The pack of the last world joined.
    pub fn last_manifest(&self) -> Option<Manifest> {
        let bytes = std::fs::read(self.last_path()?).ok()?;
        proto::decode(&bytes).ok()
    }

    /// Put every cached file of the last pack in the asset store, so the
    /// title screen can show its stage before any join. Returns how many.
    pub fn load_last(&self) -> usize {
        let Some(m) = self.last_manifest() else {
            return 0;
        };
        let mut n = 0;
        for f in &m.files {
            if let Some(b) = self.read(f) {
                crate::assets::insert(&f.path, b);
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_files_by_hash_and_refuses_damaged_ones() {
        let dir = std::env::temp_dir().join(format!("dyno-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let c = Cache::at(&dir);
        let bytes = b"hello pack".to_vec();
        let f = AssetRef {
            path: "cache-test/cache.txt".into(),
            hash: sha256(&bytes),
            size: bytes.len() as u32,
            codec: dyno_protocol::wire::RAW,
            wire: bytes.len() as u32,
        };
        assert!(c.read(&f).is_none());
        c.write(&f.hash, &bytes);
        assert_eq!(c.read(&f).as_deref(), Some(&bytes[..]));
        // a damaged file is not used
        std::fs::write(c.file(&f.hash).unwrap(), b"hello pacK").unwrap();
        assert!(c.read(&f).is_none());
        c.write(&f.hash, &bytes);
        let m = Manifest {
            pack: "t".into(),
            version: 1,
            files: vec![f],
            stream: Vec::new(),
        };
        c.save_manifest(&m);
        assert_eq!(c.last_manifest(), Some(m));
        assert_eq!(c.load_last(), 1);
        assert_eq!(
            crate::assets::text("cache-test/cache.txt").as_deref(),
            Some("hello pack")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
