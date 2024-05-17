use std::time::Instant;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::components::{Opacity, Position, Scale};
use crate::game_field::GameField;
use crate::geometry::rect::MyRect;
use crate::geometry::Circle;

const SPEED: f32 = 150.0;

pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ExplosionHitEvent>()
            .add_event::<ExplosionMaxRadiusEvent>()
            .add_event::<ExplosionsFinishedEvent>()
            .add_systems(Update, update_explosion_system)
            .add_systems(PostUpdate, update_explosion_alpha_system);
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Explosion {
    created: Instant,
    max_radius: f32,
    pub cur_radius: f32,
    max_radius_passed: bool,
}

#[derive(Event)]
pub struct ExplosionHitEvent {
    pub explosion: Explosion,
    pub position: Vec2,
}

#[derive(Event)]
pub struct ExplosionMaxRadiusEvent {
    pub position: Vec2,
    pub max_radius: f32,
}

#[derive(Event)]
pub struct ExplosionsFinishedEvent;

impl Explosion {
    pub fn new(max_radius: f32) -> Self {
        Explosion {
            created: Instant::now(),
            max_radius,
            cur_radius: 0.0,
            max_radius_passed: false,
        }
    }

    pub fn get_intersection_percents(&self, position: Vec2, bound: MyRect) -> u8 {
        let bound_area = ((bound.right - bound.left) * (bound.top - bound.bottom)).abs();
        if bound_area > 0.0 {
            let circle = Circle::new(position, self.max_radius);
            let intersection_area = circle.area_of_rect_intersection(bound);
            if intersection_area > 0.0 {
                let percents = 100.0 * intersection_area / bound_area;
                return percents.min(100.0).max(0.0) as u8;
            }
        }
        0
    }
}

pub fn spawn_explosion(commands: &mut Commands, game_field: &GameField, position: Vec2) {
    debug!("Spawn explosion");
    let explosion = Explosion::new(50.0);
    let scale = explosion.cur_radius / 1000.0;

    let color = Color::rgba(242. / 255., 68. / 255., 15. / 255., 1.);
    let explosion_circle = shapes::Circle {
        radius: 1000.,
        ..shapes::Circle::default()
    };
    let explosion_bundle = ShapeBundle {
        path: GeometryBuilder::build_as(&explosion_circle),
        spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3::new(
            position.x, position.y, 2.,
        ))),
        ..default()
    };

    let explosion_entity = commands
        .spawn((
            explosion_bundle,
            Fill::color(color),
            explosion,
            Position(position),
            Scale(scale),
            Opacity(1.),
        ))
        .id();
    commands
        .entity(game_field.parent_entity)
        .add_child(explosion_entity);
    commands.spawn(AudioBundle {
        source: game_field.explosion_sound.clone(),
        ..Default::default()
    });
}

pub fn update_explosion_system(
    mut commands: Commands,
    mut explosions_query: Query<(&mut Explosion, &mut Scale, &Position, &mut Opacity, Entity)>,
    mut hit_events: EventWriter<ExplosionHitEvent>,
    mut radius_events: EventWriter<ExplosionMaxRadiusEvent>,
    mut finish_events: EventWriter<ExplosionsFinishedEvent>,
) {
    let mut total_explosions: usize = 0;
    let mut remove_explosions: usize = 0;

    for (mut explosion, mut scale, &Position(explosion_pos), mut opacity, entity) in
        explosions_query.iter_mut()
    {
        total_explosions += 1;
        let time = explosion.created.elapsed().as_secs_f32();
        let radius = time * SPEED;
        explosion.cur_radius = radius.min(explosion.max_radius);
        scale.0 = explosion.cur_radius / 1000.;

        let cur_opacity = if radius <= explosion.max_radius {
            1.0
        } else {
            0.0_f32.max((2.0 * explosion.max_radius - radius) / explosion.max_radius)
        };
        if cur_opacity != opacity.0 {
            opacity.0 = cur_opacity;
        }

        if !explosion.max_radius_passed && radius >= explosion.max_radius {
            radius_events.send(ExplosionMaxRadiusEvent {
                position: explosion_pos,
                max_radius: explosion.max_radius,
            });
            explosion.max_radius_passed = true;
        }

        if opacity.0 == 0. {
            // Remove explosion entity
            commands.entity(entity).despawn();
            remove_explosions += 1;
            hit_events.send(ExplosionHitEvent {
                explosion: *explosion,
                position: explosion_pos,
            });
            debug!("Explosion removed");
        }
    }

    if total_explosions > 0 && total_explosions == remove_explosions {
        finish_events.send(ExplosionsFinishedEvent);
    }
}

pub fn update_explosion_alpha_system(mut query: Query<(&Opacity, &mut Fill), Changed<Opacity>>) {
    for (opacity, mut fill) in query.iter_mut() {
        fill.color.set_a(opacity.0);
    }
}
