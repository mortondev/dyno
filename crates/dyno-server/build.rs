//! Embed the content pack (the workspace's `content/` directory) in the
//! server, so a release binary on its own serves the full pack.

use std::path::{Path, PathBuf};

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.ends_with(".tmp") {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push((rel.to_string_lossy().replace('\\', "/"), p.clone()));
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../../content");
    println!("cargo:rerun-if-changed={}", root.display());
    println!("cargo:rerun-if-changed=build.rs");
    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    files.sort();
    let mut code = String::from("/// Every file of the built-in pack: (pack path, bytes).\n");
    code.push_str("pub static EMBEDDED: &[(&str, &[u8])] = &[\n");
    for (rel, abs) in files {
        code.push_str(&format!(
            "    ({rel:?}, include_bytes!({:?})),\n",
            abs.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded_pack.rs");
    std::fs::write(out, code).unwrap();
}
