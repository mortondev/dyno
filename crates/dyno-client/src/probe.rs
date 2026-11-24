//! Where things are on screen, by name, for `dyno drive` scripts.
//!
//! The interface marks the rect of what a script may click ("bar.3",
//! "dialog.accept", "bag.0") as it draws it. Marks are kept only while a
//! script drives the client; in play `mark` does nothing. They last one
//! frame: a name that is present was drawn in the last frame.

use egui::Rect;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);
static NOW: Mutex<Option<HashMap<String, Rect>>> = Mutex::new(None);
static LAST: Mutex<Option<HashMap<String, Rect>>> = Mutex::new(None);

/// Start keeping marks (a script is driving).
pub fn enable() {
    ON.store(true, Ordering::Release);
}

pub fn on() -> bool {
    ON.load(Ordering::Acquire)
}

/// A frame begins: what the last one marked is what `get` answers from.
pub fn begin_frame() {
    if on() {
        let done = NOW.lock().unwrap().take().unwrap_or_default();
        *LAST.lock().unwrap() = Some(done);
    }
}

/// Mark where something was drawn. A name with spaces is kept with `_`.
pub fn mark(name: impl AsRef<str>, rect: Rect) {
    if on() {
        NOW.lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(name.as_ref().replace(' ', "_"), rect);
    }
}

/// Where the last frame drew `name`, if it did.
pub fn get(name: &str) -> Option<Rect> {
    LAST.lock().unwrap().as_ref()?.get(name).copied()
}

/// Every name the last frame marked, sorted (for a script's error message).
pub fn names() -> Vec<String> {
    let mut v: Vec<String> = LAST
        .lock()
        .unwrap()
        .as_ref()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    v.sort();
    v
}
