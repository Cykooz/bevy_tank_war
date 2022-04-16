use std::time::Instant;

use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::sprite::Mesh2dHandle;
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;

use crate::components::{Position, Scale, POST_GAME_UPDATE};
use crate::game_field::{GameField, GameState};
use crate::geometry::Circle;
use crate::landscape::Landscape;
use crate::tank::{Health, Tank};

const SPEED: f32 = 150.0;

pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_explosion_system)
            .add_system_to_stage(POST_GAME_UPDATE, update_explosion_alpha_system);
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct Explosion {
    created: Instant,
    max_radius: f32,
    pub cur_radius: f32,
    pub cur_opacity: f32,
    landscape_updated: bool,
}

impl Explosion {
    pub fn new(max_radius: f32) -> Self {
        Explosion {
            created: Instant::now(),
            max_radius,
            cur_radius: 0.0,
            cur_opacity: 1.0,
            landscape_updated: false,
        }
    }

    fn destroy_landscape(&mut self, position: Vec2, landscape: &mut Landscape) {
        let mut landscape_changed = false;
        let circle = line_drawing::BresenhamCircle::new(
            position.x as i32,
            position.y as i32,
            self.max_radius as i32 - 1,
        );
        for points_iter in &circle.chunks(4) {
            let points: Vec<(i32, i32)> = points_iter.step_by(2).collect();
            if points.len() != 2 {
                break;
            }
            let (x1, y1) = points[0];
            let (x2, y2) = points[1];
            let x = x1.min(x2).max(0);
            let len = (x1.max(x2).max(0) - x) as u16;
            if len == 0 {
                continue;
            }
            for &y in [y1, y2].iter() {
                if let Some(pixels) = landscape.get_pixels_line_mut((x, y), len) {
                    let changed_count: u32 = pixels
                        .iter_mut()
                        .map(|c| {
                            if *c == 0 {
                                0
                            } else {
                                *c = 0;
                                1
                            }
                        })
                        .sum();
                    if changed_count > 0 {
                        landscape_changed = true;
                    }
                }
            }
        }
        if landscape_changed {
            landscape.set_changed();
        }
        self.landscape_updated = true;
    }

    pub fn get_intersection_percents(&self, position: Vec2, bound: Rect<f32>) -> u8 {
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
    let explosion_bundle = GeometryBuilder::build_as(
        &explosion_circle,
        DrawMode::Fill(FillMode {
            options: FillOptions::default(),
            color,
        }),
        Transform::from_translation(Vec3::new(position.x, position.y, 2.)),
    );

    commands
        .spawn_bundle(explosion_bundle)
        .insert(explosion)
        .insert(Position(position))
        .insert(Scale(scale))
        .insert(Parent(game_field.parent_entity));
}

pub fn update_explosion_system(
    mut commands: Commands,
    mut game_field: ResMut<GameField>,
    mut explosions_query: Query<(&mut Explosion, &mut Scale, &Position, Entity)>,
    mut tanks_query: Query<(&Tank, &mut Health, &Position)>,
) {
    let mut total_explosions: usize = 0;
    let mut remove_explosions: usize = 0;

    for (mut explosion, mut scale, &Position(explosion_pos), entity) in explosions_query.iter_mut()
    {
        total_explosions += 1;
        let time = explosion.created.elapsed().as_secs_f32();
        let radius = time * SPEED;
        explosion.cur_opacity = if radius <= explosion.max_radius {
            1.0
        } else {
            0.0_f32.max((2.0 * explosion.max_radius - radius) / explosion.max_radius)
        };
        explosion.cur_radius = radius.min(explosion.max_radius);
        scale.0 = explosion.cur_radius / 1000.;

        if !explosion.landscape_updated && radius >= explosion.max_radius {
            explosion.destroy_landscape(explosion_pos, &mut game_field.landscape);
        }

        if explosion.cur_opacity == 0. {
            // Remove explosion entity
            commands.entity(entity).despawn();
            debug!("Explosion removed");
            remove_explosions += 1;

            // Check intersection of explosion with tanks and decrease its health.
            for (tank, mut health, &Position(tank_position)) in tanks_query.iter_mut() {
                let percents = explosion
                    .get_intersection_percents(explosion_pos, tank.body_rect(tank_position));
                if percents > 0 {
                    health.damage(percents);
                }
            }
        }
    }

    if total_explosions > 0 && total_explosions == remove_explosions {
        game_field.landscape.subsidence();
        game_field.state = GameState::Subsidence;
    }
}

pub fn update_explosion_alpha_system(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&Explosion, &Mesh2dHandle), (Changed<Scale>,)>,
) {
    for (explosion, mesh_handle) in query.iter() {
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            if let Some(VertexAttributeValues::Float32x4(colors)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
            {
                colors.iter_mut().for_each(|c| c[3] = explosion.cur_opacity);
            }
        }
    }
}
