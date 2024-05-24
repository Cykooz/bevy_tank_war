pub use game_plugin::TankWarGamePlugin;
pub use materials::*;

mod ballistics;
mod collider;
mod components;
mod explosion;
mod game_field;
mod game_plugin;
mod geometry;
mod input;
mod landscape;
mod materials;
mod missile;
mod status_panel;
mod tank;
pub const G: f32 = 9.80665;
pub const MAX_PLAYERS_COUNT: u8 = 5;
