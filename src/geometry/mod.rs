use bevy::prelude::*;
use bevy::render::render_graph::base::MainPass;
use bevy_prototype_lyon::entity::{Processed, ShapeBundle};

pub use circle::Circle;
pub use ellipse::Ellipse;

pub mod circle;
pub mod ellipse;

pub fn clone_shape_bundle(shape_bundle: &ShapeBundle, transform: Transform) -> ShapeBundle {
    let mut sprite = Sprite::new(shape_bundle.sprite.size);
    sprite.resize_mode = shape_bundle.sprite.resize_mode;
    ShapeBundle {
        path: shape_bundle.path.clone(),
        mode: shape_bundle.mode,
        processed: Processed(shape_bundle.processed.0),
        sprite,
        mesh: shape_bundle.mesh.clone(),
        material: shape_bundle.material.clone(),
        main_pass: MainPass,
        draw: shape_bundle.draw.clone(),
        visible: shape_bundle.visible.clone(),
        render_pipelines: shape_bundle.render_pipelines.clone(),
        transform,
        global_transform: shape_bundle.global_transform,
    }
}
