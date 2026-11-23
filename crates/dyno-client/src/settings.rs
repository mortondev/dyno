//! Player settings, saved as JSON in the user config folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub name: String,
    /// The address used last time. The connect screen starts with it.
    pub last_server: String,
    /// Recently used addresses, newest first.
    pub recent: Vec<String>,
    pub mouse_sensitivity: f32,
    pub fov_degrees: f32,
    pub view_distance: f32,
    /// How much ground cover (grass, flowers, stones) is drawn: 0 none,
    /// 1 the pack's density and range, up to 2.
    pub ground_cover: f32,
    /// How far leaf crowns are drawn in full detail: 0 nowhere, 1 the
    /// pack's distance, up to 4.
    pub leaf_detail: f32,
    /// Block IDs in the 9 hotbar slots.
    pub hotbar: Vec<u8>,
    pub volume: f32,
    /// 0 plain water; 1 the bed seen through it and the far land mirrored;
    /// 2 everything on screen mirrored (water.rs)
    pub water_quality: u8,
    /// random ID of this install; servers bind your name to it
    pub token: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            name: String::new(),
            last_server: String::new(),
            recent: Vec::new(),
            mouse_sensitivity: 1.0,
            fov_degrees: 75.0,
            view_distance: 160.0,
            ground_cover: 1.0,
            leaf_detail: 1.0,
            hotbar: dyno_world::HOTBAR.to_vec(),
            volume: 0.8,
            water_quality: 2,
            token: 0,
        }
    }
}

/// Path of the settings file, for example `~/.config/dyno/settings.json`.
pub fn path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("DYNO_CONFIG_DIR") {
        return Some(PathBuf::from(p).join("settings.json"));
    }
    directories::ProjectDirs::from("io", "mortondev", "dyno")
        .map(|d| d.config_dir().join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        let mut s: Settings = path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if s.token == 0 {
            s.token = random_token();
            let _ = s.save();
        }
        s
    }

    pub fn save(&self) -> Result<(), String> {
        let p = path().ok_or("no config folder on this system")?;
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = p.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap())
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
    }

    /// Remember a server address as the last and most recent one.
    pub fn remember_server(&mut self, addr: &str) {
        let addr = addr.trim().to_string();
        self.last_server = addr.clone();
        self.recent.retain(|a| a != &addr);
        self.recent.insert(0, addr);
        self.recent.truncate(8);
    }
}

/// A random 64-bit number from the OS-seeded hasher.
pub fn random_token() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    h.write_u32(std::process::id());
    h.finish() | 1
}

/// A stable token for command-line tools, derived from their name.
pub fn tool_token(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("dyno-tool:{name}").bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h | 1
}
