use bevy::prelude::*;

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct Position(pub Vec2);

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct Scale(pub f32);

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct Angle(pub f32);

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct Opacity(pub f32);

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct HueOffset(pub u16);
