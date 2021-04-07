use bevy::prelude::Vec2;

pub const POST_GAME_UPDATE: &str = "post_game_update";
pub const ROUND_SETUP: &str = "round_setup";

#[derive(Debug, Default, Clone, Copy)]
pub struct Position(pub Vec2);

#[derive(Debug, Default, Clone, Copy)]
pub struct Scale(pub f32);

#[derive(Debug, Default, Clone, Copy)]
pub struct Angle(pub f32);
