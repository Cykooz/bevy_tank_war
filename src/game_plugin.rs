use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::components::{Angle, Position, Scale, POST_GAME_UPDATE, ROUND_SETUP};
use crate::explosion::Explosion;
use crate::game_field::{GameField, GameState};
use crate::input::InputWithRepeating;
use crate::landscape::LandscapeSprite;
use crate::missile;
use crate::tank::{AimingTank, CurrentTank, Health, TankThrowing};
use crate::{explosion, landscape, status_panel, tank};

pub struct TankWarGamePlugin;

impl Plugin for TankWarGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputWithRepeating<KeyCode>>()
            .add_startup_system(setup_camera)
            .add_startup_system(setup_game_field)
            // .add_system(set_texture_filtration)
            .add_startup_stage(ROUND_SETUP, SystemStage::parallel())
            .add_stage_after(CoreStage::Update, POST_GAME_UPDATE, SystemStage::parallel())
            .add_system_to_stage(POST_GAME_UPDATE, update_translation)
            .add_system_to_stage(POST_GAME_UPDATE, update_scale)
            .add_system_to_stage(POST_GAME_UPDATE, update_angle)
            .add_system(switch_current_tank_system.after("tanks_processing"))
            .add_plugin(ShapePlugin)
            .add_plugin(landscape::LandscapePlugin)
            .add_plugin(missile::MissilesPlugin)
            .add_plugin(tank::TanksPlugin)
            .add_plugin(explosion::ExplosionPlugin)
            .add_plugin(status_panel::StatusPanelPlugin);
    }
}

fn setup_camera(mut commands: Commands, window: Res<WindowDescriptor>) {
    let width = window.width as f32;
    let height = window.height as f32;

    let mut camera = Camera2dBundle::default();
    camera.transform.translation =
        Vec3::new(width / 2., height / 2., camera.transform.translation.z);
    commands.spawn_bundle(camera);
}

fn setup_game_field(
    mut commands: Commands,
    mut textures: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    window: Res<WindowDescriptor>,
) {
    let width = window.width;
    let height = window.height - 30.;

    let field_width = (width - 2.) as u16;
    let field_height = (height - 2.) as u16;

    // Game Field border
    let border = shapes::Rectangle {
        extents: Vec2::new(width - 1., height - 1.),
        origin: RectangleOrigin::BottomLeft,
    };
    let border_color = Color::rgb(1., 1., 1.);
    commands.spawn_bundle(GeometryBuilder::build_as(
        &border,
        DrawMode::Stroke(StrokeMode {
            options: StrokeOptions::default(),
            color: border_color,
        }),
        Transform::from_translation(Vec3::new(0.5, 0.5, 100.)),
    ));

    // let parent_entity = commands
    //     .spawn_bundle((
    //         Transform::from_translation(Vec3::new(0., 0., 0.)),
    //         GlobalTransform::identity(),
    //     ))
    //     .id();

    let parent_entity = commands
        .spawn_bundle(SpatialBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            visibility: Visibility { is_visible: true },
            ..default()
        })
        .id();

    // Landscape
    let game_landscape =
        landscape::Landscape::new(field_width, field_height, &mut textures).unwrap();
    let position = Vec3::new(field_width as f32 / 2., field_height as f32 / 2., 0.);
    let landscape_entity = commands
        .spawn_bundle(SpriteBundle {
            texture: game_landscape.texture_handle(),
            transform: Transform::from_translation(position),
            ..Default::default()
        })
        .insert(LandscapeSprite)
        .id();
    commands.entity(parent_entity).add_child(landscape_entity);

    let tank_texture = asset_server.load("sprites/tank.png");
    let gun_texture = asset_server.load("sprites/gun.png");

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
        tank_texture,
        gun_texture,
        tank_fire_sound: asset_server.load("sounds/tank_fire.ogg"),
        explosion_sound: asset_server.load("sounds/explosion1.ogg"),
    };
    commands.insert_resource(game_field);
}

// fn set_texture_filtration(
//     mut textures: ResMut<Assets<Image>>,
//     mut event_reader: EventReader<AssetEvent<Image>>,
// ) {
//     for event in event_reader.iter() {
//         if let AssetEvent::Created { handle } = event {
//             if let Some(texture) = textures.get_mut(handle) {
//                 texture.sampler_descriptor.mag_filter = FilterMode::Linear;
//             }
//         }
//     }
// }

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

fn switch_current_tank_system(
    mut commands: Commands,
    mut game_field: ResMut<GameField>,
    throwing_tanks: Query<(&TankThrowing,)>,
    health_query: Query<(&Health,), Changed<Health>>,
    cur_tank_query: Query<(&CurrentTank,)>,
    explosions: Query<(&Explosion,)>,
) {
    if let GameState::SwitchTank = game_field.state {
        if cur_tank_query.iter().next().is_some() {
            // Current tank already exists
            return;
        }
        if game_field.landscape.is_subsidence() {
            return;
        }
        if throwing_tanks.iter().next().is_some() {
            return;
        }
        if explosions.iter().next().is_some() {
            return;
        }
        if health_query.iter().any(|(h,)| h.0 == 0) {
            // Not all dead tanks has removed
            return;
        }
        debug!("Switch current tank");
        if let Some(new_current_entity) = game_field.switch_current_tank() {
            commands
                .entity(new_current_entity)
                .insert(CurrentTank)
                .insert(AimingTank);
            game_field.state = GameState::Playing;
        } else {
            // TODO: All tanks is dead
        }
    }
}
