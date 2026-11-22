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
    /// Block IDs in the 9 hotbar slots.
    pub hotbar: Vec<u8>,
    pub volume: f32,
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
            hotbar: dyno_world::HOTBAR.to_vec(),
            volume: 0.8,
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
        path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
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
