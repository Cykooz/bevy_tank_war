use std::f32::consts::PI;

use bevy::prelude::*;

use crate::ballistics::Ballistics;
use crate::components::Position;
use crate::explosion::spawn_explosion;
use crate::game_field::GameField;
use crate::tank::Tank;

const TIME_SCALE: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
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

    pub fn update<F>(&mut self, borders: (i32, i32), has_collision: F) -> Option<Vec2>
    where
        F: Fn(i32, i32) -> bool,
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
    commands
        .spawn(SpriteBundle {
            sprite: Sprite::new(game_field.missile_sprite_size),
            mesh: game_field.missile_mesh.clone(),
            material: game_field.missile_material.clone(),
            transform: Transform::from_translation(Vec3::new(position.x, position.y, 1.)),
            ..Default::default()
        })
        .with(missile)
        .with(Position(position))
        .with(Parent(game_field.parent_entity));
}

pub fn missile_moving_system(
    commands: &mut Commands,
    game_field: Res<GameField>,
    audio: Res<Audio>,
    mut missile_query: Query<(Entity, &mut Missile, &mut Position)>,
    tank_position_query: Query<(&Tank, &Position)>,
) {
    let landscape = &game_field.landscape;
    let size = landscape.size();
    let borders = (size.0 as i32, size.1 as i32);
    for (missile_entity, mut missile, mut missile_position) in missile_query.iter_mut() {
        let hit_point = missile.update(borders, |x, y| {
            landscape.is_not_empty(x, y)
                || tank_position_query
                    .iter()
                    .any(|(tank, position)| tank.has_collision(position.0, (x as f32, y as f32)))
        });
        let current_position = missile.cur_pos();
        missile_position.0 = current_position;

        if let Some(pos) = hit_point {
            commands.despawn(missile_entity);
            spawn_explosion(commands, &game_field, current_position);
            audio.play(game_field.explosion_sound.clone());
        }
    }
}
