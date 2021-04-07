use bevy::prelude::*;

use crate::components::ROUND_SETUP;
use crate::game_field::GameField;
use crate::tank::{CurrentTank, Health, Tank};

pub struct StatusPanelPlugin;

impl Plugin for StatusPanelPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system_to_stage(ROUND_SETUP, setup_status_panel.system())
            .add_system(update_gun_angle_text.system())
            .add_system(update_gun_power_text.system())
            .add_system(update_wind_power_text.system())
            .add_system(update_player_number_text.system())
            .add_system(update_tank_health_text.system());
    }
}

pub struct GunAngleText;
pub struct GunPowerText;
pub struct WindPowerText;
pub struct PlayerNumberText;
pub struct TankHealthText;

pub fn setup_status_panel(
    commands: &mut Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    game_field: Res<GameField>,
    window: Res<WindowDescriptor>,
) {
    let panel_bottom = window.height - 30.;
    commands.spawn(NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Px(30.0)),
            position_type: PositionType::Absolute,
            position: Rect {
                left: Val::Px(0.0),
                bottom: Val::Px(panel_bottom),
                ..Default::default()
            },
            ..Default::default()
        },
        material: materials.add(Color::BLACK.into()),
        ..Default::default()
    });

    let text_bottom = panel_bottom + 4.;
    // Gun Angle
    commands
        .spawn(spawn_text(
            "Angle:",
            10.,
            text_bottom,
            game_field.font.clone(),
        ))
        .with(GunAngleText);

    // Gun Power
    commands
        .spawn(spawn_text(
            "Power:",
            110.,
            text_bottom,
            game_field.font.clone(),
        ))
        .with(GunPowerText);

    // Wind Power
    commands
        .spawn(spawn_text(
            "Wind:",
            220.,
            text_bottom,
            game_field.font.clone(),
        ))
        .with(WindPowerText);

    // Player number
    commands
        .spawn(spawn_text(
            "Player:",
            440.,
            text_bottom,
            game_field.font.clone(),
        ))
        .with(PlayerNumberText);

    // Tank health
    commands
        .spawn(spawn_text(
            "Health:",
            540.,
            text_bottom,
            game_field.font.clone(),
        ))
        .with(TankHealthText);
}

fn spawn_text(
    text_value: &str,
    left_position: f32,
    bottom_position: f32,
    font: Handle<Font>,
) -> TextBundle {
    TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            position: Rect {
                left: Val::Px(left_position),
                bottom: Val::Px(bottom_position),
                ..Default::default()
            },
            ..Default::default()
        },
        text: Text {
            value: text_value.to_string(),
            font,
            style: TextStyle {
                font_size: 20.0,
                color: Color::WHITE,
                ..Default::default()
            },
        },
        ..Default::default()
    }
}

pub fn update_gun_angle_text(
    current_tank_query: Query<&Tank, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<GunAngleText>>,
) {
    if let Some(tank) = current_tank_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.value = format!("Angle: {}", tank.gun_angle_deg());
        }
    }
}

pub fn update_gun_power_text(
    current_tank_query: Query<&Tank, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<GunPowerText>>,
) {
    if let Some(tank) = current_tank_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.value = format!("Power: {}", tank.power);
        }
    }
}

pub fn update_wind_power_text(
    game_filed: Res<GameField>,
    mut text_query: Query<&mut Text, With<WindPowerText>>,
) {
    if let Some(mut text) = text_query.iter_mut().next() {
        text.value = format!("Wind: {}", game_filed.wind_power * 10.0);
    }
}

pub fn update_player_number_text(
    current_tank_query: Query<&Tank, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<PlayerNumberText>>,
) {
    if let Some(tank) = current_tank_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.value = format!("Player: {}", tank.player_number);
        }
    }
}

pub fn update_tank_health_text(
    health_query: Query<&Health, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<TankHealthText>>,
) {
    if let Some(health) = health_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.value = format!("Health: {}", health.0);
        }
    }
}
