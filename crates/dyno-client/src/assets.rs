//! The client's asset store: files from the content pack, by pack path
//! ("textures/grass.png", "models/person.vox"). The server streams them
//! (docs/design/data-and-streaming.md); until that lands, and for tools and
//! tests, they load from a local pack directory.
//!
//! Every piece of content the client shows comes through here. The client
//! never carries content of its own.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

static STORE: OnceLock<RwLock<HashMap<String, Arc<[u8]>>>> = OnceLock::new();

fn store() -> &'static RwLock<HashMap<String, Arc<[u8]>>> {
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// A file from the pack, if it has arrived.
pub fn get(path: &str) -> Option<Arc<[u8]>> {
    store().read().unwrap().get(path).cloned()
}

/// A file as text (JSON).
pub fn text(path: &str) -> Option<String> {
    get(path).and_then(|b| String::from_utf8(b.to_vec()).ok())
}

/// Add or replace a file (from the network, or a local pack).
pub fn insert(path: &str, bytes: impl Into<Arc<[u8]>>) {
    store()
        .write()
        .unwrap()
        .insert(path.to_string(), bytes.into());
}

/// Every path under a directory prefix ("models/").
pub fn list(prefix: &str) -> Vec<String> {
    let mut v: Vec<String> = store()
        .read()
        .unwrap()
        .keys()
        .filter(|k| k.starts_with(prefix))
        .cloned()
        .collect();
    v.sort();
    v
}

static REQUESTS: OnceLock<std::sync::Mutex<Vec<String>>> = OnceLock::new();

/// Ask the server for a streamed file (one under a `stream` prefix of the
/// pack) that `get` does not have yet. It arrives through `insert`; until
/// then, draw a neutral placeholder. Asking twice is harmless.
pub fn request(path: &str) {
    let mut q = REQUESTS.get_or_init(Default::default).lock().unwrap();
    if !q.iter().any(|p| p == path) {
        q.push(path.to_string());
    }
}

/// The paths asked for that a connection can serve (`known`); the rest wait
/// for another connection.
pub(crate) fn take_requests(known: impl Fn(&str) -> bool) -> Vec<String> {
    let Some(q) = REQUESTS.get() else {
        return Vec::new();
    };
    let mut q = q.lock().unwrap();
    if q.is_empty() {
        return Vec::new();
    }
    let (mine, rest): (Vec<String>, Vec<String>) = q.drain(..).partition(|p| known(p));
    *q = rest;
    mine
}

/// Load every file under a local pack directory (tools, tests, dev builds).
pub fn load_dir(root: &Path) -> std::io::Result<usize> {
    fn walk(root: &Path, dir: &Path, n: &mut usize) -> std::io::Result<()> {
        for e in std::fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_dir() {
                walk(root, &p, n)?;
            } else if let Ok(rel) = p.strip_prefix(root) {
                let key = rel.to_string_lossy().replace('\\', "/");
                insert(&key, std::fs::read(&p)?);
                *n += 1;
            }
        }
        Ok(())
    }
    let mut n = 0;
    walk(root, root, &mut n)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_lists_files() {
        insert("tests/a.json", b"{}".to_vec());
        insert("tests/b.png", vec![1u8, 2, 3]);
        assert_eq!(text("tests/a.json").as_deref(), Some("{}"));
        assert_eq!(list("tests/"), vec!["tests/a.json", "tests/b.png"]);
        assert!(get("tests/missing").is_none());
    }
}
