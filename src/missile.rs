use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::ballistics::Ballistics;
use crate::components::Position;
use crate::explosion::spawn_explosion;
use crate::game_field::GameField;
use crate::tank::Tank;

const TIME_SCALE: f32 = 3.0;

pub struct MissilesPlugin;

impl Plugin for MissilesPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MissileMovedEvent>()
            .add_systems(Update, missile_moving_system2)
            .add_systems(PostUpdate, despawn_dead_missiles);
    }
}

#[derive(Event)]
pub struct MissileMovedEvent {
    pub missile: Entity,
    pub path: Vec<(i32, i32)>,
}

pub trait HasCollision {
    fn has_collision(&self, entity_position: Vec2, point: Vec2) -> bool;
}

#[derive(Debug, Default, Clone, Copy, Component)]
struct DeadPosition {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Missile {
    ballistics: Ballistics,
}

impl Missile {
    pub fn new(pos: Vec2, angle: f32, power: f32, acceleration: Vec2) -> Missile {
        let rad = angle * PI / 180.;
        let velocity: Vec2 = Vec2::new(rad.sin(), rad.cos()) * power;

        Missile {
            ballistics: Ballistics::new(pos, velocity, acceleration).time_scale(TIME_SCALE),
        }
    }

    #[inline]
    pub fn cur_pos(&self) -> Vec2 {
        self.ballistics.cur_pos()
    }

    pub fn update<F>(&mut self, borders: (i32, i32), mut has_collision: F) -> Option<Vec2>
    where
        F: FnMut(i32, i32) -> bool,
    {
        for (x, y) in self.ballistics.positions_iter(None, Some(borders)) {
            if has_collision(x, y) || y <= 0 {
                return Some(Vec2::new(x as f32, y as f32));
            }
        }

        None
    }
}

pub fn spawn_missile(commands: &mut Commands, game_field: &GameField, missile: Missile) {
    let position = missile.cur_pos();
    let missile_color = Color::rgb(1., 1., 1.);
    let missile_circle = shapes::Circle {
        radius: 1.5,
        ..shapes::Circle::default()
    };
    let missile_bundle = ShapeBundle {
        path: GeometryBuilder::build_as(&missile_circle),
        spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3::new(
            position.x, position.y, 1.,
        ))),
        ..Default::default()
    };
    let missile_entity = commands
        .spawn((
            missile_bundle,
            Fill::color(missile_color),
            missile,
            Position(position),
        ))
        .id();
    commands
        .entity(game_field.parent_entity)
        .add_child(missile_entity);
}

pub fn missile_moving_system(
    mut commands: Commands,
    game_field: Res<GameField>,
    tank_position_query: Query<(&Tank, &Position), Without<Missile>>,
    mut missile_query: Query<(Entity, &mut Missile, &mut Position)>,
) {
    let landscape = &game_field.landscape;
    let size = landscape.size();
    let borders = (size.0 as i32, size.1 as i32);
    for (missile_entity, mut missile, mut missile_position) in missile_query.iter_mut() {
        let is_hit = missile
            .update(borders, |x, y| {
                landscape.is_not_empty(x, y)
                    || tank_position_query.iter().any(|(tank, position)| {
                        tank.has_collision(position.0, (x as f32, y as f32))
                    })
            })
            .is_some();
        let current_position = missile.cur_pos();
        missile_position.0 = current_position;

        if is_hit {
            kill_missile(
                &mut commands,
                missile_entity,
                current_position.x as i32,
                current_position.y as i32,
            );
            // commands.entity(missile_entity).despawn();
            // spawn_explosion(&mut commands, &game_field, current_position);
        }
    }
}

pub fn missile_moving_system2(
    game_field: Res<GameField>,
    mut ev_missile_moved: EventWriter<MissileMovedEvent>,
    mut missile_query: Query<(Entity, &mut Missile, &mut Position)>,
) {
    let landscape = &game_field.landscape;
    let size = landscape.size();
    let borders = (size.0 as i32, size.1 as i32);

    for (missile_entity, mut missile, mut missile_position) in missile_query.iter_mut() {
        let mut path: Vec<(i32, i32)> = Vec::new();
        missile.update(borders, |x, y| {
            path.push((x, y));
            false
        });
        let current_position = missile.cur_pos();
        missile_position.0 = current_position;

        if !path.is_empty() {
            ev_missile_moved.send(MissileMovedEvent {
                missile: missile_entity,
                path,
            });
        }
    }
}

pub fn kill_missile(commands: &mut Commands, entity_id: Entity, x: i32, y: i32) {
    if let Some(mut entity) = commands.get_entity(entity_id) {
        entity.try_insert(DeadPosition { x, y });
    }
}

fn despawn_dead_missiles(
    mut commands: Commands,
    game_field: Res<GameField>,
    query: Query<(Entity, &DeadPosition), With<Missile>>,
) {
    for (entity, dead_pos) in query.iter() {
        commands.entity(entity).despawn_recursive();
        spawn_explosion(
            &mut commands,
            &game_field,
            Vec2::new(dead_pos.x as f32, dead_pos.y as f32),
        );
    }
}
