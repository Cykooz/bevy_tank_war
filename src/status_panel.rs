use bevy::prelude::*;

use crate::components::ROUND_SETUP;
use crate::game_field::GameField;
use crate::tank::{CurrentTank, Health, Tank};

pub struct StatusPanelPlugin;

impl Plugin for StatusPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(ROUND_SETUP, setup_status_panel)
            .add_system(update_gun_angle_text)
            .add_system(update_gun_power_text)
            .add_system(update_wind_power_text)
            .add_system(update_player_number_text)
            .add_system(update_tank_health_text);
    }
}

#[derive(Component)]
pub struct GunAngleText;
#[derive(Component)]
pub struct GunPowerText;
#[derive(Component)]
pub struct WindPowerText;
#[derive(Component)]
pub struct PlayerNumberText;
#[derive(Component)]
pub struct TankHealthText;

pub fn setup_status_panel(
    mut commands: Commands,
    game_field: Res<GameField>,
    window: Res<WindowDescriptor>,
) {
    let panel_bottom = window.height - 30.;
    let mut panel = commands.spawn_bundle(NodeBundle {
        style: Style {
            size: Size::new(Val::Percent(100.0), Val::Px(30.0)),
            position_type: PositionType::Absolute,
            position: UiRect {
                left: Val::Px(0.0),
                bottom: Val::Px(panel_bottom),
                ..default()
            },
            padding: UiRect {
                left: Val::Px(10.),
                right: Val::Px(10.),
                ..default()
            },
            align_items: AlignItems::Center,
            ..default()
        },
        color: Color::BLACK.into(),
        ..default()
    });

    panel.with_children(|parent| {
        // Gun Angle
        parent
            .spawn_bundle(spawn_text("Angle:", game_field.font.clone(), 110.0))
            .insert(GunAngleText);

        // Gun Power
        parent
            .spawn_bundle(spawn_text("Power:", game_field.font.clone(), 110.0))
            .insert(GunPowerText);

        // Wind Power
        parent
            .spawn_bundle(spawn_text("Wind:", game_field.font.clone(), 110.0))
            .insert(WindPowerText);

        // Player number
        parent
            .spawn_bundle(spawn_text("Player:", game_field.font.clone(), 110.0))
            .insert(PlayerNumberText);

        // Tank health
        parent
            .spawn_bundle(spawn_text("Health:", game_field.font.clone(), 120.0))
            .insert(TankHealthText);
    });
}

fn spawn_text(text_value: &str, font: Handle<Font>, width: f32) -> TextBundle {
    TextBundle {
        style: Style {
            size: Size::new(Val::Px(width), Val::Px(20.)),
            ..default()
        },
        text: Text::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            text_value.to_string(),
            TextStyle {
                font,
                font_size: 20.0,
                color: Color::WHITE,
            },
        ),
        ..default()
    }
}

pub fn update_gun_angle_text(
    current_tank_query: Query<&Tank, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<GunAngleText>>,
) {
    if let Some(tank) = current_tank_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.sections[0].value = format!("Angle: {}", tank.gun_angle_deg());
        }
    }
}

pub fn update_gun_power_text(
    current_tank_query: Query<&Tank, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<GunPowerText>>,
) {
    if let Some(tank) = current_tank_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.sections[0].value = format!("Power: {}", tank.power);
        }
    }
}

pub fn update_wind_power_text(
    game_filed: Res<GameField>,
    mut text_query: Query<&mut Text, With<WindPowerText>>,
) {
    if let Some(mut text) = text_query.iter_mut().next() {
        text.sections[0].value = format!("Wind: {}", game_filed.wind_power * 10.0);
    }
}

pub fn update_player_number_text(
    current_tank_query: Query<&Tank, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<PlayerNumberText>>,
) {
    if let Some(tank) = current_tank_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.sections[0].value = format!("Player: {}", tank.player_number);
        }
    }
}

pub fn update_tank_health_text(
    health_query: Query<&Health, With<CurrentTank>>,
    mut text_query: Query<&mut Text, With<TankHealthText>>,
) {
    if let Some(health) = health_query.iter().next() {
        if let Some(mut text) = text_query.iter_mut().next() {
            text.sections[0].value = format!("Health: {}", health.0);
        }
    }
}
