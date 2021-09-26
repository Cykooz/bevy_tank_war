use bevy::prelude::Vec2;

pub trait Collider {
    /// Returns `true` if given point locates inside of collider.
    fn has_collision<P: Into<Vec2>>(&self, point: P) -> bool;
}
