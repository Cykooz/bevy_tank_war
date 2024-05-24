use angular_units::Deg;
use std::f32::consts::PI;

use bevy::prelude::*;

use crate::ballistics::Ballistics;
use crate::components::{Angle, HueOffset, Position};
use crate::explosion::{spawn_explosion, ExplosionHitEvent};
use crate::game_field::GameField;
use crate::game_plugin::AppState;
use crate::geometry::rect::MyRect;
use crate::geometry::Ellipse;
use crate::input::InputWithRepeating;
use crate::landscape;
use crate::missile::{kill_missile, spawn_missile, HasCollision, Missile, MissileMovedEvent};
use crate::{G, MAX_PLAYERS_COUNT};
use prisma::encoding::{EncodableColor, SrgbEncoding};
use prisma::{FromColor, Hsv, Rgb};

const TANK_SIZE: f32 = 41.;
const GUN_SIZE: f32 = 21.;
const POWER_SCALE: f32 = 300. / 100.;
const TIME_SCALE: f32 = 3.0;
/// Damage per one pixel of height with which tank was dropped.
const TANK_THROWING_DAMAGE_POWER: f32 = 0.1;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum TankSet {
    // Setup,
    Throwing,
    Aiming,
}

#[derive(Event)]
pub struct AllTanksPlacedEvent;

#[derive(Event)]
pub struct TankShotEvent {
    pub tank_entity: Entity,
}

pub struct TanksPlugin;

impl Plugin for TanksPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AllTanksPlacedEvent>()
            .add_event::<TankShotEvent>()
            .configure_sets(
                Update,
                (
                    TankSet::Throwing.run_if(in_state(AppState::TanksThrowing)),
                    TankSet::Aiming.run_if(in_state(AppState::Aiming)),
                ),
            )
            .add_systems(
                Update,
                (throw_down_tanks_system, tanks_throwing_system).chain(),
            )
            .add_systems(
                Update,
                (
                    gun_rotate_system,
                    gun_sprite_angle_system,
                    gun_power_system,
                    shoot_system,
                )
                    .in_set(TankSet::Aiming),
            )
            .add_systems(
                Update,
                (
                    check_missile_collides_with_tanks_system,
                    damage_tank_by_explosion_system,
                    set_texture_hue_system,
                ),
            )
            .add_systems(PostUpdate, remove_dead_tank_system);
    }
}

#[derive(Clone, Copy, Component)]
pub struct TankGun;

#[derive(Clone, Copy, Component)]
pub struct CurrentTank;

#[derive(Clone, Copy, Component)]
pub struct AimingTank;

#[derive(Clone, Copy, Component)]
pub struct Health {
    pub value: u8,
    pub invincible: bool,
}

impl Health {
    /// Returns value of health after damage has been applied.
    #[inline]
    pub fn damage(&mut self, v: u8) -> u8 {
        if !self.invincible {
            self.value = self.value.saturating_sub(v);
        }
        self.value
    }
}

#[derive(Debug, Clone, Component)]
pub struct TankThrowing {
    pub start_position: Vec2,
    pub tank_width: f32,
    pub ballistics: Ballistics,
}

#[derive(Debug, Clone, Component)]
pub struct Tank {
    pub player_number: u8,
    pub power: f32,
    pub dead: bool,
    body_bounds: Vec<Ellipse>,
    gun_bounds: Vec<Ellipse>,
    gun_angle_deg: f32,
}

impl Tank {
    #[inline]
    pub fn size() -> Vec2 {
        Vec2::new(TANK_SIZE, TANK_SIZE)
    }

    pub fn new(player_number: u8) -> Tank {
        let body_bounds = vec![
            Ellipse::new((0., -5.5), 9.5, 9.),    // top bound
            Ellipse::new((-9.5, -13.), 10., 6.5), // left bound
            Ellipse::new((9.5, -13.), 10., 6.5),  // right bound
            Ellipse::new((0., -13.), 19.5, 7.5),  // center bound
        ];
        let gun_bounds = vec![
            Ellipse::new((0., 14.), 2.5, 5.),
            Ellipse::new((0., 5.), 2., 8.),
        ];
        Tank {
            player_number,
            body_bounds,
            gun_bounds,
            gun_angle_deg: 0.0,
            power: 40.0,
            dead: false,
        }
    }

    pub fn gun_barrel_pos(&self, tank_position: Vec2) -> Vec2 {
        let rad = self.gun_angle_deg * PI / 180.0;
        let gun_vec = Vec2::new(GUN_SIZE * rad.sin(), GUN_SIZE * rad.cos());
        tank_position + gun_vec
    }

    /// Increment angle of gun
    pub fn inc_gun_angle(&mut self, delta_degrees: f32) {
        self.gun_angle_deg = (self.gun_angle_deg + delta_degrees).min(90.).max(-90.);
    }

    pub fn gun_angle_deg(&self) -> f32 {
        self.gun_angle_deg
    }

    pub fn gun_angle_rad(&self) -> f32 {
        self.gun_angle_deg * PI / 180.
    }

    /// Increment power of gun of current tank
    pub fn inc_gun_power(&mut self, delta: f32) {
        self.power = (self.power + delta).min(100.).max(0.);
    }

    pub fn shoot(&self, tank_position: Vec2, acceleration: Vec2) -> Missile {
        Missile::new(
            self.gun_barrel_pos(tank_position),
            self.gun_angle_deg,
            self.power * POWER_SCALE,
            acceleration,
        )
    }

    pub fn throw_down(&self, start_position: Vec2) -> TankThrowing {
        let left_bottom = start_position - Self::size() / 2.;
        let start_height = left_bottom.y + 1.;
        TankThrowing {
            start_position,
            tank_width: TANK_SIZE,
            ballistics: Ballistics::new([left_bottom.x, start_height], [0., 0.], [0., -G])
                .time_scale(TIME_SCALE),
        }
    }

    #[inline]
    pub fn body_rect(&self, position: Vec2) -> MyRect {
        let half_size = TANK_SIZE / 2.;
        MyRect {
            left: position.x - half_size,
            right: position.x + half_size,
            top: position.y + half_size,
            bottom: position.y - half_size,
        }
    }

    #[inline]
    fn left_bottom(&self, tank_position: Vec2) -> Vec2 {
        Vec2::new(
            tank_position.x - TANK_SIZE / 2.,
            tank_position.y - TANK_SIZE / 2.,
        )
    }

    /// Returns `true` if given point locates inside of tank's body or gun.
    pub fn has_collision<P: Into<Vec2>>(&self, tank_position: Vec2, point: P) -> bool {
        let point = point.into();
        let local_point = point - tank_position;
        // If point outside of tank's rectangle
        if local_point.abs().max_element() > TANK_SIZE / 2. {
            return false;
        }

        // Check tank's body bounds
        if self
            .body_bounds
            .iter()
            .any(|b| b.point_position(local_point) <= 0.)
        {
            return true;
        }

        // Check the tank's gun bounds.
        // Rotate local_point into the coordinate system of tank's gun.
        let rotation = Quat::from_rotation_z(self.gun_angle_deg * PI / 180.);
        let rotated_point = rotation.mul_vec3(Vec3::new(local_point.x, local_point.y, 0.));
        let rotated_point = Vec2::new(rotated_point.x, rotated_point.y);
        self.gun_bounds
            .iter()
            .any(|b| b.point_position(rotated_point) <= 0.)
    }
}

struct TankCollider {
    body_bounds: Vec<Ellipse>,
    gun_bounds: Vec<Ellipse>,
    gun_angle_deg: f32,
}

impl TankCollider {
    pub fn new() -> Self {
        let body_bounds = vec![
            Ellipse::new((0., -5.5), 9.5, 9.),    // top bound
            Ellipse::new((-9.5, -13.), 10., 6.5), // left bound
            Ellipse::new((9.5, -13.), 10., 6.5),  // right bound
            Ellipse::new((0., -13.), 19.5, 7.5),  // center bound
        ];
        let gun_bounds = vec![
            Ellipse::new((0., 14.), 2.5, 5.),
            Ellipse::new((0., 5.), 2., 8.),
        ];
        Self {
            body_bounds,
            gun_bounds,
            gun_angle_deg: 0.,
        }
    }
}

impl HasCollision for TankCollider {
    fn has_collision(&self, entity_position: Vec2, point: Vec2) -> bool {
        let local_point = point - entity_position;
        // If point outside inside of tank's rectangle
        if local_point.abs().max_element() > TANK_SIZE / 2. {
            return false;
        }

        // Check tank's body bounds
        if self
            .body_bounds
            .iter()
            .any(|b| b.point_position(local_point) <= 0.)
        {
            return true;
        }

        // Check tank's gun bounds.
        // Rotate local_point into coordinate system of tank's gun.
        let rotation = Quat::from_rotation_z(self.gun_angle_deg * PI / 180.);
        let rotated_point = rotation.mul_vec3(Vec3::new(local_point.x, local_point.y, 0.));
        let rotated_point = Vec2::new(rotated_point.x, rotated_point.y);
        self.gun_bounds
            .iter()
            .any(|b| b.point_position(rotated_point) <= 0.)
    }
}

#[derive(Bundle, Clone)]
struct TankBundle {
    tank: Tank,
    health: Health,
    position: Position,
    tank_throwing: TankThrowing,
    sprite: SpriteBundle,
}

impl TankBundle {
    pub fn new(player_number: u8, position: Vec2, texture: Handle<Image>) -> Self {
        let tank = Tank::new(player_number);
        let tank_throwing = tank.throw_down(position);
        let mut transform = Transform::default();
        transform.translation.z = 0.1;
        let sprite = SpriteBundle {
            texture,
            transform,
            ..Default::default()
        };
        Self {
            tank,
            health: Health {
                value: 100,
                invincible: true,
            },
            position: Position(position),
            tank_throwing,
            sprite,
        }
    }
}

#[derive(Bundle, Clone)]
struct TankGunBundle {
    gun: TankGun,
    angle: Angle,
    sprite: SpriteBundle,
}

impl TankGunBundle {
    pub fn new(texture: Handle<Image>) -> Self {
        let mut transform = Transform::default();
        transform.translation.z = -0.1;
        let sprite = SpriteBundle {
            texture,
            transform,
            ..Default::default()
        };
        Self {
            gun: TankGun,
            angle: Angle(0.),
            sprite,
        }
    }
}

pub fn setup_tanks(mut commands: Commands, mut game_field: ResMut<GameField>) {
    let tank_material = game_field.tank_texture.clone();
    let gun_material = game_field.gun_texture.clone();

    let count_of_tanks = 5u8;
    game_field.start_round(count_of_tanks);

    let tank_size = Tank::size();
    let padding: f32 = 100.5;
    let size_between_tanks =
        ((game_field.width as f32 - 2. * padding) / (count_of_tanks - 1) as f32).round();
    let tank_y = (game_field.height - 50) as f32 + tank_size.y / 2.;
    let start_position = Vec2::new(padding, tank_y);

    let parent_entity = game_field.parent_entity;
    let player_numbers = game_field.player_numbers.clone();
    for (i, &player_number) in player_numbers.iter().enumerate() {
        let tank_position = start_position + Vec2::new(size_between_tanks * i as f32, 0.);

        let hue_offset = (player_number as u16 - 1) * (360 / MAX_PLAYERS_COUNT as u16);
        let tank_entity = commands
            .spawn((
                TankBundle::new(player_number, tank_position, tank_material.clone()),
                HueOffset(hue_offset),
            ))
            .with_children(|parent| {
                parent.spawn((
                    TankGunBundle::new(gun_material.clone()),
                    HueOffset(hue_offset),
                ));
            })
            .id();
        if i == 0 {
            commands
                .entity(tank_entity)
                .insert(CurrentTank)
                .insert(AimingTank);
        }

        commands.entity(parent_entity).add_child(tank_entity);
        game_field.tanks.push(Some(tank_entity));
    }
}

pub fn gun_rotate_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut repeated_input: ResMut<InputWithRepeating<KeyCode>>,
    mut aiming_tanks: Query<&mut Tank, With<AimingTank>>,
) {
    let mut delta: f32 = 0.;

    if repeated_input.pressed(&keyboard_input, KeyCode::ArrowLeft) {
        delta = -1.;
    }
    if repeated_input.pressed(&keyboard_input, KeyCode::ArrowRight) {
        delta = 1.;
    }
    if delta == 0. {
        return;
    }

    for mut tank in aiming_tanks.iter_mut() {
        tank.inc_gun_angle(delta);
    }
}

pub fn gun_sprite_angle_system(
    tank_query: Query<(&Tank, &Children), Changed<Tank>>,
    mut gun_angle_query: Query<&mut Angle, With<TankGun>>,
) {
    for (tank, children) in tank_query.iter() {
        for child in children.iter() {
            if let Ok(mut gun_angle) = gun_angle_query.get_mut(*child) {
                gun_angle.0 = -tank.gun_angle_deg();
            }
        }
    }
}

pub fn gun_power_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut repeated_input: ResMut<InputWithRepeating<KeyCode>>,
    mut aiming_tanks: Query<&mut Tank, With<AimingTank>>,
) {
    let mut delta: f32 = 0.;

    if repeated_input.pressed(&keyboard_input, KeyCode::ArrowUp) {
        delta = 1.;
    }
    if repeated_input.pressed(&keyboard_input, KeyCode::ArrowDown) {
        delta = -1.;
    }

    if delta == 0. {
        return;
    }

    for mut tank in aiming_tanks.iter_mut() {
        tank.inc_gun_power(delta);
    }
}

pub fn shoot_system(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    game_field: Res<GameField>,
    mut aiming_tanks: Query<(&Tank, &Position, Entity), With<AimingTank>>,
    mut shot_events: EventWriter<TankShotEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for (tank, tank_position, entity) in aiming_tanks.iter_mut() {
            let acceleration = Vec2::new(game_field.wind_power, -G);
            let missile = tank.shoot(tank_position.0, acceleration);
            spawn_missile(&mut commands, &game_field, missile);
            commands.spawn(AudioBundle {
                source: game_field.tank_fire_sound.clone(),
                ..Default::default()
            });
            shot_events.send(TankShotEvent {
                tank_entity: entity,
            });
        }
    }
}

fn throw_down_tanks_system(
    mut commands: Commands,
    tanks_query: Query<(Entity, &Tank, &Position), (Without<TankThrowing>,)>,
    mut finished_event: EventReader<landscape::SubsidenceFinishedEvent>,
) {
    if finished_event.read().count() > 0 {
        debug!("Throw down all tanks");
        for (entity, tank, position) in tanks_query.iter() {
            commands.entity(entity).insert(tank.throw_down(position.0));
        }
        debug!("Tanks has thrown");
    }
}

fn tanks_throwing_system(
    mut commands: Commands,
    mut game_field: ResMut<GameField>,
    mut tanks_query: Query<(Entity, &mut TankThrowing, &mut Position, &mut Health)>,
    mut all_placed_event: EventWriter<AllTanksPlacedEvent>,
) {
    let mut tanks_count: usize = 0;
    let mut placed_tanks_count: usize = 0;

    for (entity, mut throwing, mut tank_position, mut health) in tanks_query.iter_mut() {
        tanks_count += 1;
        let tank_width = throwing.tank_width;
        let max_empty_count = (0.3 * tank_width).round() as usize;
        let mut offset: f32 = 0.0;
        let mut stop_throwing = false;

        for (x, y) in throwing.ballistics.positions_iter(None, None) {
            if y <= 0 {
                stop_throwing = true;
                break;
            }

            let landscape = &mut game_field.landscape;
            let pixels_under_tank = landscape.get_pixels_line_mut((x, y), tank_width as u16);
            if let Some(pixels) = pixels_under_tank {
                let empty_count = bytecount::count(pixels, 0);
                if empty_count > max_empty_count {
                    if empty_count < tank_width as usize {
                        // Landscape under tank is not empty - clear it
                        pixels.iter_mut().for_each(|c| *c = 0);
                        landscape.set_changed();
                    }
                    // Get down tank
                    offset += 1.0;
                } else {
                    stop_throwing = true;
                    break;
                }
            }
        }

        if offset > 0. {
            let new_y = tank_position.0.y - offset;
            tank_position.0.y = new_y;
        }

        if stop_throwing {
            placed_tanks_count += 1;
            commands.entity(entity).remove::<TankThrowing>();
            if health.invincible {
                health.invincible = false;
            } else {
                let cur_height = tank_position.0.y;
                let path_len = throwing.start_position.y - cur_height;
                let damage_value: u8 =
                    (path_len * TANK_THROWING_DAMAGE_POWER).min(255.).round() as u8;
                if damage_value > 0 {
                    health.damage(damage_value);
                }
            }
        }
    }

    if tanks_count > 0 && tanks_count == placed_tanks_count {
        // All tanks placed
        all_placed_event.send(AllTanksPlacedEvent);
    }
}

pub fn check_missile_collides_with_tanks_system(
    mut commands: Commands,
    mut ev_missile_moved: EventReader<MissileMovedEvent>,
    tank_position_query: Query<(&Tank, &Position)>,
) {
    for ev in ev_missile_moved.read() {
        for &(x, y) in ev.path.iter() {
            let is_hit = tank_position_query
                .iter()
                .any(|(tank, position)| tank.has_collision(position.0, (x as f32, y as f32)));
            if is_hit {
                debug!("Missile hit a tank in point {:?}", (x, y));
                kill_missile(&mut commands, ev.missile, x, y);
                break;
            }
        }
    }
}

fn remove_dead_tank_system(
    mut commands: Commands,
    mut game_field: ResMut<GameField>,
    health_query: Query<(&Health, &Position, Entity), Changed<Health>>,
) {
    for (health, position, entity) in health_query.iter() {
        if health.value == 0 {
            debug!("Explode tank");
            spawn_explosion(&mut commands, &game_field, position.0);
            game_field.remove_tank_by_entity(entity);
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn damage_tank_by_explosion_system(
    mut tanks_query: Query<(&Tank, &mut Health, &Position)>,
    mut explosion_events: EventReader<ExplosionHitEvent>,
) {
    for event in explosion_events.read() {
        let explosion = event.explosion;
        let explosion_pos = event.position;
        // Check the intersection of explosion with tanks and decrease their health.
        for (tank, mut health, &Position(tank_position)) in tanks_query.iter_mut() {
            let percents =
                explosion.get_intersection_percents(explosion_pos, tank.body_rect(tank_position));
            if percents > 0 {
                debug!(
                    "Damage tank #{} by explosion on {} points",
                    tank.player_number, percents
                );
                health.damage(percents);
            }
        }
    }
}

fn set_texture_hue_system(
    mut commands: Commands,
    textures: ResMut<Assets<Image>>,
    mut asset_events: EventReader<AssetEvent<Image>>,
    images_query: Query<(Entity, &Handle<Image>, &HueOffset)>,
    asset_server: Res<AssetServer>,
) {
    for event in asset_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            if let Some(image) = textures.get(*id) {
                for (entity, image_handle, hue_offset) in images_query.iter() {
                    if image_handle.id() != *id {
                        continue;
                    }
                    let new_image = rotate_hue(image, hue_offset.0);
                    commands
                        .entity(entity)
                        .remove::<HueOffset>()
                        .remove::<Handle<Image>>()
                        .insert(asset_server.add(new_image));
                }
            }
        }
    }
}

fn rotate_hue(image: &Image, hue_offset: u16) -> Image {
    let mut new_image = image.clone();
    for pixel in new_image.data.chunks_exact_mut(4) {
        let mut rgb = *Rgb::new(
            pixel[0] as f32 / 255.,
            pixel[1] as f32 / 255.,
            pixel[2] as f32 / 255.,
        )
        .srgb_encoded()
        .decode()
        .color();
        let mut hsv: Hsv<f32, Deg<f32>> = Hsv::from_color(&rgb);
        let hue = (hsv.hue() + Deg(hue_offset as f32)).0 as u32 % 360;
        hsv.set_hue(Deg(hue as f32));
        rgb = *Rgb::from_color(&hsv)
            .linear()
            .encode(SrgbEncoding::new())
            .color();
        pixel[0] = (rgb.red() * 255.0).min(255.).round() as u8;
        pixel[1] = (rgb.green() * 255.0).min(255.).round() as u8;
        pixel[2] = (rgb.blue() * 255.0).min(255.).round() as u8;
    }
    new_image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_collision() {
        let tank_position = Vec2::new(10.0 + TANK_SIZE / 2., 20.0 - TANK_SIZE / 2.);
        let mut tank = Tank::new(1);

        let inner_points = [
            (20., 27.), // body center
            (4., 32.),  // body left
            (37., 32.), // body right
            (20., 40.), // body bottom
            (20., 18.), // body top
            (20., 2.),  // gun top
            (20., 13.), // gun middle
        ];
        for point in inner_points.iter() {
            assert!(
                tank.has_collision(tank_position, (10. + point.0, 20. - point.1)),
                "point=({}, {})",
                point.0,
                point.1
            );
        }

        // Rotated gun
        tank.gun_angle_deg = 60.;
        let inner_points = [
            (34., 11.), // gun top
            (24., 18.), // gun middle
        ];
        for point in inner_points.iter() {
            assert!(
                tank.has_collision(tank_position, (10. + point.0, 20. - point.1)),
                "point=({}, {})",
                point.0,
                point.1
            );
        }

        tank.gun_angle_deg = -45.;
        let inner_points = [
            (8., 8.),   // gun top
            (15., 15.), // gun middle
        ];
        for point in inner_points.iter() {
            assert!(
                tank.has_collision(tank_position, (10. + point.0, 20. - point.1)),
                "point=({}, {})",
                point.0,
                point.1
            );
        }
    }
}
