use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::components::{Angle, Position, Scale, POST_GAME_UPDATE, ROUND_SETUP};
use crate::explosion::Explosion;
use crate::game_field::{GameField, GameState};
use crate::landscape::LandscapeSprite;
use crate::missile::missile_moving_system;
use crate::tank::TankThrowing;
use crate::{explosion, landscape, status_panel, tank};

pub struct TankWarGamePlugin;

impl Plugin for TankWarGamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_camera.system())
            .add_startup_system(setup_game_field.system())
            .add_startup_stage(ROUND_SETUP, SystemStage::parallel())
            .add_stage_after(stage::UPDATE, POST_GAME_UPDATE, SystemStage::parallel())
            .add_system(landscape::update_landscape.system())
            .add_system(landscape::update_landscape_texture.system())
            .add_system(missile_moving_system.system())
            .add_system_to_stage(POST_GAME_UPDATE, update_translation.system())
            .add_system_to_stage(POST_GAME_UPDATE, update_scale.system())
            .add_system_to_stage(POST_GAME_UPDATE, update_angle.system())
            .add_plugin(tank::TanksPlugin)
            .add_plugin(explosion::ExplosionPlugin)
            .add_plugin(status_panel::StatusPanelPlugin);
    }
}

fn setup_camera(commands: &mut Commands, window: Res<WindowDescriptor>) {
    let width = window.width as f32;
    let height = window.height as f32;

    let mut camera = Camera2dBundle::default();
    camera.transform.translation = Vec3::new(
        width / 2. + 0.5,
        height / 2. + 0.5,
        camera.transform.translation.z,
    );
    commands.spawn(camera);
    commands.spawn(CameraUiBundle::default());
}

fn setup_game_field(
    commands: &mut Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut textures: ResMut<Assets<Texture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    window: Res<WindowDescriptor>,
) {
    let width = window.width;
    let height = window.height - 30.;

    let field_width = (width - 2.) as u16;
    let field_height = (height - 2.) as u16;

    // Game Field border
    let border_material = materials.add(Color::rgb(1., 1., 1.).into());
    commands.spawn(primitive(
        border_material.clone(),
        &mut meshes,
        ShapeType::Rectangle {
            width: width - 1.,
            height: height - 1.,
        },
        TessellationMode::Stroke(&StrokeOptions::default()),
        Vec3::new(1., 1., 100.),
    ));

    let parent_entity = commands
        .spawn((
            Transform::from_translation(Vec3::new(0.5, 0.5, 0.)),
            GlobalTransform::identity(),
        ))
        .current_entity()
        .unwrap();

    // Landscape
    let game_landscape =
        landscape::Landscape::new(field_width, field_height, &mut textures).unwrap();
    let position = Vec3::new(field_width as f32 / 2., field_height as f32 / 2., 0.);
    commands
        .spawn(SpriteBundle {
            material: materials.add(game_landscape.texture_handle().into()),
            transform: Transform::from_translation(position),
            ..Default::default()
        })
        .with(LandscapeSprite)
        .with(Parent(parent_entity));

    let tank_texture_handle = asset_server.load("sprites/tank.png");
    let tank_material = materials.add(tank_texture_handle.into());
    let gun_texture_handle = asset_server.load("sprites/gun.png");
    let gun_material = materials.add(gun_texture_handle.into());

    // Missile
    let SpriteBundle {
        sprite: missile_sprite,
        mesh: missile_mesh,
        material: missile_material,
        ..
    } = primitive(
        materials.add(Color::rgb(1., 1., 1.).into()),
        &mut meshes,
        ShapeType::Circle(1.5),
        TessellationMode::Fill(&FillOptions::default()),
        Vec3::new(0., 0., 1.),
    );

    // Explosion
    let explosion_color = Color::rgba(242. / 255., 68. / 255., 15. / 255., 1.);
    let SpriteBundle {
        sprite: explosion_sprite,
        mesh: explosion_mesh,
        ..
    } = primitive(
        materials.add(explosion_color.into()),
        &mut meshes,
        ShapeType::Circle(1000.),
        TessellationMode::Fill(&FillOptions::default()),
        Vec3::new(0., 0., 2.),
    );

    // Game field
    let game_field = GameField {
        width: field_width,
        height: field_height,
        parent_entity,
        landscape: game_landscape,
        wind_power: 0.,
        player_numbers: vec![],
        tanks: vec![],
        current_tank: 0,
        state: GameState::Starting,
        number_of_iteration: 0,
        font: asset_server.load("fonts/DejaVuSerif.ttf"),
        border_material,
        tank_material,
        gun_material,
        missile_sprite_size: missile_sprite.size,
        missile_mesh,
        missile_material,
        explosion_sprite_size: explosion_sprite.size,
        explosion_mesh,
        explosion_color,
        tank_fire_sound: asset_server.load("sounds/tank_fire.ogg"),
        explosion_sound: asset_server.load("sounds/explosion1.ogg"),
    };
    commands.insert_resource(game_field);
}

fn update_translation(mut query: Query<(&Position, &mut Transform), (Changed<Position>,)>) {
    for (position, mut transform) in query.iter_mut() {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}

fn update_scale(mut query: Query<(&Scale, &mut Transform), (Changed<Scale>,)>) {
    for (scale, mut transform) in query.iter_mut() {
        transform.scale.x = scale.0;
        transform.scale.y = scale.0;
    }
}

fn update_angle(mut query: Query<(&Angle, &mut Transform), (Changed<Angle>,)>) {
    for (angle, mut transform) in query.iter_mut() {
        transform.rotation = Quat::from_rotation_z(angle.0 * PI / 180.);
    }
}

// fn switch_current_tank_system(
//     commands: &mut Commands,
//     game_field: Res<GameField>,
//     throwing_tanks: Query<(&TankThrowing,)>,
//     explosions: Query<(&Explosion,)>,
// ) {
//     if let GameState::Starting = game_field.state {
//         return;
//     }
//     if game_field.landscape.is_subsidence() {
//         return;
//     }
//     if throwing_tanks.iter().next().is_some() {
//         return;
//     }
//     if explosions.iter().next().is_some() {
//         return;
//     }
// }
