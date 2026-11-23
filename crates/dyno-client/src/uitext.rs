//! The interface's words. They are data: `ui/strings.json` in the pack
//! (docs/design/ui.md). A key is a dotted path, `"hud.release"`; a missing
//! key reads as empty text, so a pack can leave a line out.
//!
//! The file is parsed once and again only when the asset store changes.

use serde_json::Value;
use std::sync::{Arc, Mutex};

/// The parsed strings and the asset generation they were read at.
static CACHE: Mutex<Option<(u64, Arc<Value>)>> = Mutex::new(None);

fn all() -> Arc<Value> {
    let generation = crate::assets::generation();
    let mut cache = CACHE.lock().unwrap();
    if let Some((at, v)) = cache.as_ref()
        && *at == generation
    {
        return v.clone();
    }
    let path = crate::assets::text("pack.json")
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| {
            v.get("entries")?
                .get("strings")?
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_else(|| "ui/strings.json".into());
    let v = Arc::new(
        crate::assets::text(&path)
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null),
    );
    *cache = Some((generation, v.clone()));
    v
}

/// The value at a dotted path.
pub fn value(key: &str) -> Option<Value> {
    let all = all();
    let mut v: &Value = &all;
    for part in key.split('.') {
        v = v.get(part)?;
    }
    Some(v.clone())
}

/// A line of text; empty when the pack has none.
pub fn t(key: &str) -> String {
    let all = all();
    let mut v: &Value = &all;
    for part in key.split('.') {
        match v.get(part) {
            Some(x) => v = x,
            None => return String::new(),
        }
    }
    v.as_str().unwrap_or_default().to_string()
}

/// A line with `{name}` places filled in.
pub fn f(key: &str, with: &[(&str, &str)]) -> String {
    fill(&t(key), with)
}

/// Fill `{name}` places in a line.
pub fn fill(line: &str, with: &[(&str, &str)]) -> String {
    let mut s = line.to_string();
    for (k, v) in with {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// A list of lines.
pub fn lines(key: &str) -> Vec<String> {
    value(key)
        .and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_filled() {
        assert_eq!(
            fill("Level {n}: {what}", &[("n", "3"), ("what", "done")]),
            "Level 3: done"
        );
        assert_eq!(fill("plain", &[("n", "3")]), "plain");
    }
}
