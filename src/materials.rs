use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin};

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<GlowMaterial>::default(),
            Material2dPlugin::<HueOffsetMaterial>::default(),
        ));
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GlowMaterial {
    #[uniform(0)]
    pub color: Color,
    #[uniform(1)]
    pub intensity: f32,
    #[texture(2)]
    #[sampler(3)]
    pub texture: Handle<Image>,
}

// All functions on `Material2d` have default impls. You only need to implement the
// functions that are relevant for your material.
impl Material2d for GlowMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/glow_material.wgsl".into()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HueOffsetMaterial {
    #[uniform(0)]
    pub offset: f32,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

// All functions on `Material2d` have default impls. You only need to implement the
// functions that are relevant for your material.
impl Material2d for HueOffsetMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/hue_material.wgsl".into()
    }
}
