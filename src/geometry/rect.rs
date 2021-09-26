use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub center: Vec2,
    width: f32,
    height: f32,
}

impl Rect {
    pub fn new<P: Into<Vec2>>(center: P, width: f32, height: f32) -> Self {
        assert!(width >= 0.0, "'width' is negative number");
        assert!(height >= 0.0, "'height' is negative number");
        Self {
            center: center.into(),
            width,
            height,
        }
    }
}
