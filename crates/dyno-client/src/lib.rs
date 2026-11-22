//! Dyno game client (bootstrap).

pub mod app;
pub mod audio;
pub mod game;
pub mod light;
pub mod net;
pub mod render;
pub mod settings;
pub mod shot;

/// Open the game window. `prefill` sets the server address field.
pub fn run(prefill: Option<String>, update_note: String) -> Result<(), String> {
    app::run(prefill, update_note)
}
