//! Dyno game client (bootstrap).

pub mod anim;
pub mod app;
pub mod assets;
pub mod atmos;
pub mod audio;
pub mod cache;
pub mod clip;
pub mod cover;
pub mod expr;
pub mod farview;
pub mod figure;
pub mod foliage;
pub mod front;
pub mod game;
pub mod look;
pub mod minimap;
pub use dyno_world::light;
pub mod models;
pub mod mood;
pub mod net;
pub mod post;
pub mod render;
pub mod rpg_ui;
pub mod scout;
pub mod screens;
pub mod settings;
pub mod shadows;
pub(crate) mod shapes;
pub mod sheet;
pub mod shot;
pub mod sky;
pub mod survival;
pub mod terrain_mesh;
pub mod textures;
pub mod theme;
pub mod vox;
pub mod voxmodel;
pub mod water;
pub mod worldwork;

/// Open the game window. `prefill` sets the server address field.
pub fn run(prefill: Option<String>, update_note: String) -> Result<(), String> {
    assets::load_local_pack();
    app::run(prefill, update_note)
}

/// Put the pack built into the local server in the asset store (dev
/// builds and tools that host or render without joining). The player
/// client has no pack of its own: the server streams it.
#[cfg(feature = "host")]
pub fn load_builtin_pack() -> usize {
    let pack = dyno_server::pack::Pack::builtin();
    let files = pack.files();
    for (path, bytes) in &files {
        assets::insert(path, bytes.clone());
    }
    files.len()
}
