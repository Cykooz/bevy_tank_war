use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};
use bevy::window::{PresentMode, PrimaryWindow};

//use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy_tank_war::TankWarGamePlugin;

fn main() {
    // env_logger::init();

    App::new()
        // .insert_resource(Msaa { samples: 4 })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Tank War - Rust edition".to_string(),
                        resolution: (1024., 768.).into(),
                        present_mode: PresentMode::AutoNoVsync,
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // Tell the asset server to watch for asset changes on disk:
                    watch_for_changes_override: Some(true),
                    ..default()
                }),
        )
        .add_systems(Startup, (setup_camera, setup_mesh))
        // // Adds frame time diagnostics
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // Adds a system that prints diagnostics to the console
        // .add_plugin(LogDiagnosticsPlugin {
        //     debug: true,
        //     ..Default::default()
        // })
        .add_plugins((
            //TankWarGamePlugin,
            Material2dPlugin::<GlowMaterial>::default(),
            Material2dPlugin::<HueOffsetMaterial>::default(),
        ))
        .run();
}

fn setup_camera(mut commands: Commands, primary_windows: Query<&Window, With<PrimaryWindow>>) {
    let Ok(window) = primary_windows.get_single() else {
        return;
    };
    let width = window.width();
    let height = window.height();
    let mut camera = Camera2dBundle::default();
    camera.transform.translation =
        Vec3::new(width / 2., height / 2., camera.transform.translation.z);
    commands.spawn(camera);
}

pub fn setup_mesh(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut glow_materials: ResMut<Assets<GlowMaterial>>,
    mut hue_materials: ResMut<Assets<HueOffsetMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(41., 41.)).into(),
        transform: Transform::from_translation(Vec3::new(100.5, 600.5, 0.)),
        material: glow_materials.add(GlowMaterial {
            color: Color::WHITE,
            intensity: 2.0,
            texture: asset_server.load("sprites/tank.png"),
        }),
        ..Default::default()
    });

    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(41., 41.)).into(),
        transform: Transform::from_translation(Vec3::new(200.5, 600.5, 0.)),
        material: hue_materials.add(HueOffsetMaterial {
            offset: 0.5,
            texture: asset_server.load("sprites/tank.png"),
        }),
        ..Default::default()
    });
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GlowMaterial {
    #[uniform(0)]
    color: Color,
    #[uniform(1)]
    intensity: f32,
    #[texture(2)]
    #[sampler(3)]
    texture: Handle<Image>,
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
    offset: f32,
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
}

// All functions on `Material2d` have default impls. You only need to implement the
// functions that are relevant for your material.
impl Material2d for HueOffsetMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/hue_material.wgsl".into()
    }
}
