use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_prototype_lyon::prelude::*;

use crate::components::{Angle, Position, Scale};
use crate::game_field::{GameField, GameState};
use crate::input::InputWithRepeating;
use crate::missile;
use crate::status_panel::setup_status_panel;
use crate::tank::{setup_tanks, AimingTank, AllTanksPlacedEvent, CurrentTank, TankShotEvent};
use crate::{explosion, landscape, status_panel, tank};

#[derive(States, PartialEq, Eq, Debug, Clone, Hash, Default)]
pub enum AppState {
    #[default]
    RoundSetup,
    TanksThrowing,
    Aiming,
    MainAction,
}

pub struct TankWarGamePlugin;

impl Plugin for TankWarGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputWithRepeating<KeyCode>>()
            .init_state::<AppState>()
            .add_systems(Startup, setup_camera)
            .add_systems(PostUpdate, (update_translation, update_scale, update_angle))
            .add_systems(PostUpdate, switch_to_aiming_system)
            .add_systems(
                OnEnter(AppState::RoundSetup),
                (
                    setup_game_field,
                    (setup_tanks, setup_status_panel),
                    switch_to_tanks_throwing_system,
                )
                    .chain(),
            )
            .add_systems(OnEnter(AppState::Aiming), switch_current_tank_system)
            .add_systems(
                Update,
                after_tank_shot_system.run_if(in_state(AppState::Aiming)),
            )
            .add_plugins((
                ShapePlugin,
                landscape::LandscapePlugin,
                missile::MissilesPlugin,
                tank::TanksPlugin,
                explosion::ExplosionPlugin,
                status_panel::StatusPanelPlugin,
            ));
    }
}

fn switch_to_tanks_throwing_system(mut next_state: ResMut<NextState<AppState>>) {
    debug!("Switch to TanksThrowing");
    next_state.set(AppState::TanksThrowing);
}

fn switch_to_aiming_system(
    cur_state_res: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut ev_tanks_placed: EventReader<AllTanksPlacedEvent>,
) {
    let cur_state = cur_state_res.get();
    if matches!(cur_state, AppState::TanksThrowing | AppState::MainAction)
        && ev_tanks_placed.read().count() > 0
    {
        debug!("Switch to Aiming from {:?}", cur_state);
        next_state.set(AppState::Aiming);
    }
}

fn switch_current_tank_system(
    mut commands: Commands,
    mut game_field: ResMut<GameField>,
    cur_tank_query: Query<Entity, With<CurrentTank>>,
) {
    for cur_tank_entity in cur_tank_query.iter() {
        commands.entity(cur_tank_entity).remove::<CurrentTank>();
        commands.entity(cur_tank_entity).remove::<AimingTank>();
    }

    debug!("Switch current tank");
    if let Some(new_current_entity) = game_field.switch_current_tank() {
        commands
            .entity(new_current_entity)
            .insert(CurrentTank)
            .insert(AimingTank);
    } else {
        // TODO: All tanks are dead
    }
}

fn after_tank_shot_system(
    mut commands: Commands,
    mut shot_events: EventReader<TankShotEvent>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let mut has_shoots = false;
    for event in shot_events.read() {
        if let Some(mut entity) = commands.get_entity(event.tank_entity) {
            entity.remove::<AimingTank>();
        }
        has_shoots = true;
    }
    if has_shoots {
        debug!("Switch to MainAction");
        next_state.set(AppState::MainAction);
    }
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

pub fn setup_game_field(
    mut commands: Commands,
    mut textures: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = primary_windows.get_single() else {
        return;
    };
    let width = window.width();
    let height = window.height() - 30.;

    let field_width = (width - 2.) as u16;
    let field_height = (height - 2.) as u16;

    let parent_entity = commands
        .spawn(SpatialBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            visibility: Visibility::Visible,
            ..default()
        })
        .id();

    // Game Field border
    let border = shapes::Rectangle {
        extents: Vec2::new(width - 1., height - 1.),
        origin: RectangleOrigin::BottomLeft,
    };
    let border_color = Color::rgb(1., 1., 1.);
    commands
        .spawn((
            GeometryBuilder::build_as(&border),
            Stroke {
                options: StrokeOptions::default(),
                color: border_color,
            },
            Transform::from_translation(Vec3::new(0.5, 0.5, 100.)),
        ))
        .set_parent(parent_entity);

    // Landscape
    let game_landscape =
        landscape::Landscape::new(field_width, field_height, &mut textures).unwrap();
    let position = Vec3::new(field_width as f32 / 2., field_height as f32 / 2., 0.);
    commands
        .spawn((
            SpriteBundle {
                texture: game_landscape.texture_handle(),
                transform: Transform::from_translation(position),
                ..Default::default()
            },
            landscape::LandscapeSprite,
        ))
        .set_parent(parent_entity);

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
        current_tank: None,
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
