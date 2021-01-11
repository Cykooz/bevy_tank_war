pub use game_plugin::TankWarGamePlugin;

mod ballistics;
mod components;
mod explosion;
mod game_field;
mod game_plugin;
mod geometry;
mod landscape;
mod missile;
//mod shaders;
mod status_panel;
mod tank;

pub const G: f32 = 9.80665;
pub const MAX_PLAYERS_COUNT: u8 = 5;
