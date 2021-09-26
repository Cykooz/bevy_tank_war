use bevy::prelude::*;
use bevy::render::render_graph::base::MainPass;
use bevy_prototype_lyon::entity::{ShapeBundle, ShapeColors};

pub use circle::Circle;
pub use ellipse::Ellipse;

pub mod circle;
pub mod ellipse;
pub mod rect;

pub fn clone_shape_bundle(shape_bundle: &ShapeBundle, transform: Transform) -> ShapeBundle {
    let colors = ShapeColors {
        main: shape_bundle.colors.main,
        outline: shape_bundle.colors.outline,
    };
    ShapeBundle {
        path: shape_bundle.path.clone(),
        mode: shape_bundle.mode,
        mesh: shape_bundle.mesh.clone(),
        colors,
        main_pass: MainPass,
        draw: shape_bundle.draw.clone(),
        visible: shape_bundle.visible.clone(),
        render_pipelines: shape_bundle.render_pipelines.clone(),
        transform,
        global_transform: shape_bundle.global_transform,
    }
}
