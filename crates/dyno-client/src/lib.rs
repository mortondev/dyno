//! Dyno game client (bootstrap).

pub mod app;
pub mod game;
pub mod net;
pub mod render;
pub mod settings;

/// Open the game window. `prefill` sets the server address field.
pub fn run(prefill: Option<String>, update_note: String) -> Result<(), String> {
    app::run(prefill, update_note)
}
