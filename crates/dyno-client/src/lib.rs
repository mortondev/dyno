//! Dyno game client (bootstrap).

pub mod anim;
pub mod app;
pub mod assets;
pub mod audio;
pub(crate) mod detail;
pub mod front;
pub mod game;
pub mod look;
pub mod minimap;
pub use dyno_world::light;
pub mod models;
pub mod net;
pub mod render;
pub mod rpg_ui;
pub mod screens;
pub mod settings;
pub mod sheet;
pub mod shot;
pub mod survival;
pub mod terrain_mesh;
pub mod textures;
pub mod theme;
pub mod voxmodel;

/// Open the game window. `prefill` sets the server address field.
pub fn run(prefill: Option<String>, update_note: String) -> Result<(), String> {
    assets::load_local_pack();
    app::run(prefill, update_note)
}
