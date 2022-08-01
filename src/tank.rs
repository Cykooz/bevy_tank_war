use std::f32::consts::PI;

use bevy::prelude::*;

use crate::ballistics::Ballistics;
use crate::components::{Angle, Position, ROUND_SETUP};
use crate::explosion::spawn_explosion;
use crate::game_field::{GameField, GameState};
use crate::geometry::Ellipse;
use crate::landscape;
use crate::missile::{
    spawn_missile, HasCollision, Missile, MissileMovedEvent, MISSILE_MOVED_LABEL,
};
use crate::{G, MAX_PLAYERS_COUNT};

const TANK_SIZE: f32 = 41.;
const GUN_SIZE: f32 = 21.;
const POWER_SCALE: f32 = 300. / 100.;
const TIME_SCALE: f32 = 3.0;
/// Damage per one pixel of height with which tank was dropped.
const TANK_THROWING_DAMAGE_POWER: f32 = 0.1;

pub struct TanksPlugin;

impl Plugin for TanksPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(ROUND_SETUP, setup_tanks)
            .add_system(gun_rotate_system)
            .add_system(gun_sprite_angle_system)
            .add_system(gun_power_system)
            .add_system(shoot_system)
            .add_system_set(
                SystemSet::new()
                    .label("tanks_processing")
                    .with_system(throw_down_tanks_system.label("throw_down_tanks"))
                    .with_system(
                        tanks_throwing_system
                            .label("tanks_throwing")
                            .after("throw_down_tanks"),
                    )
                    .with_system(remove_dead_tank_system.after("tanks_throwing")),
            )
            .add_system(missile_collides_with_tanks_system.after(MISSILE_MOVED_LABEL));
    }
}

#[derive(Clone, Copy, Component)]
pub struct TankGun;

#[derive(Clone, Copy, Component)]
pub struct CurrentTank;

#[derive(Clone, Copy, Component)]
pub struct AimingTank;

#[derive(Clone, Copy, Component)]
pub struct Health(pub u8);

impl Health {
    /// Returns value of health after damage has been applied.
    #[inline]
    pub fn damage(&mut self, v: u8) -> u8 {
        self.0 = self.0.saturating_sub(v);
        self.0
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
    hue_offset: u16,
    gun_angle_deg: f32,
}

impl Tank {
    #[inline]
    pub fn size() -> Vec2 {
        Vec2::new(TANK_SIZE, TANK_SIZE)
    }

    pub fn new(player_number: u8, hue_offset: u16) -> Tank {
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
            hue_offset,
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
    pub fn body_rect(&self, position: Vec2) -> Rect<f32> {
        let half_size = TANK_SIZE / 2.;
        Rect {
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
    #[bundle]
    sprite: SpriteBundle,
}

impl TankBundle {
    pub fn new(player_number: u8, position: Vec2, texture: Handle<Image>) -> Self {
        let hue_offset = (player_number as u16 - 1) * (360 / MAX_PLAYERS_COUNT as u16);
        let tank = Tank::new(player_number, hue_offset);
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
            health: Health(100),
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
    #[bundle]
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

fn setup_tanks(mut commands: Commands, mut game_field: ResMut<GameField>) {
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

        let mut entity_commands = commands.spawn_bundle(TankBundle::new(
            player_number,
            tank_position,
            tank_material.clone(),
        ));
        entity_commands
            .insert(Parent(parent_entity))
            .with_children(|parent| {
                parent.spawn_bundle(TankGunBundle::new(gun_material.clone()));
            });
        if i == 0 {
            entity_commands.insert(CurrentTank).insert(AimingTank);
        }

        let tank_entity = entity_commands.id();
        game_field.tanks.push(Some(tank_entity));
    }
}

pub fn gun_rotate_system(
    keyboard_input: ResMut<Input<KeyCode>>,
    mut aiming_tanks: Query<&mut Tank, With<AimingTank>>,
) {
    let mut delta: f32 = 0.;

    if keyboard_input.pressed(KeyCode::Left) {
        delta = -1.;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        delta = 1.;
    }
    if delta == 0. {
        return;
    }

    for mut tank in aiming_tanks.iter_mut() {
        tank.inc_gun_angle(delta);
    }
}

fn gun_sprite_angle_system(
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
    keyboard_input: ResMut<Input<KeyCode>>,
    mut aiming_tanks: Query<&mut Tank, With<AimingTank>>,
) {
    let mut delta: f32 = 0.;

    if keyboard_input.pressed(KeyCode::Up) {
        delta = 1.;
    }
    if keyboard_input.pressed(KeyCode::Down) {
        delta = -1.;
    }

    if delta == 0. {
        return;
    }

    for mut tank in aiming_tanks.iter_mut() {
        tank.inc_gun_power(delta);
    }
}

fn shoot_system(
    mut commands: Commands,
    keyboard_input: ResMut<Input<KeyCode>>,
    audio: Res<Audio>,
    game_field: Res<GameField>,
    mut aiming_tanks: Query<(&Tank, &Position, Entity), With<AimingTank>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for (tank, tank_position, entity) in aiming_tanks.iter_mut() {
            let acceleration = Vec2::new(game_field.wind_power, -G);
            let missile = tank.shoot(tank_position.0, acceleration);
            spawn_missile(&mut commands, &game_field, missile);
            audio.play(game_field.tank_fire_sound.clone());
            commands.entity(entity).remove::<AimingTank>();
        }
    }
}

fn throw_down_tanks_system(
    mut commands: Commands,
    tanks_query: Query<(Entity, &Tank, &Position), (Without<TankThrowing>,)>,
    mut finished_event: EventReader<landscape::SubsidenceFinishedEvent>,
) {
    if finished_event.iter().next().is_some() {
        debug!("Throw down all tanks");
        for (entity, tank, position) in tanks_query.iter() {
            commands.entity(entity).insert(tank.throw_down(position.0));
        }
        debug!("Tanks has thrown");
        finished_event.iter().for_each(|_| ());
    }
}

fn tanks_throwing_system(
    mut commands: Commands,
    mut game_field: ResMut<GameField>,
    mut tanks_query: Query<(Entity, &mut TankThrowing, &mut Position, &mut Health)>,
    cur_tank_query: Query<(Entity, &CurrentTank)>,
) {
    let mut tanks_count: usize = 0;
    let mut placed_tanks_count: usize = 0;

    let game_state = game_field.state;

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
            match game_state {
                GameState::Starting => {}
                _ => {
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
    }

    if tanks_count > 0 && tanks_count == placed_tanks_count {
        // All tanks placed
        match game_field.state {
            GameState::Starting => game_field.state = GameState::Playing,
            GameState::SwitchTank => (),
            _ => {
                // Remove current tank markers
                for (cur_tank_entity, _) in cur_tank_query.iter() {
                    commands.entity(cur_tank_entity).remove::<CurrentTank>();
                    commands.entity(cur_tank_entity).remove::<AimingTank>();
                }
                debug!("Change game status into SwitchTank");
                game_field.state = GameState::SwitchTank;
            }
        }
        // if let GameState::Starting = game_field.state {
        //     game_field.state = GameState::Playing;
        // } else if
    }
}

pub fn missile_collides_with_tanks_system(
    mut commands: Commands,
    game_field: Res<GameField>,
    audio: Res<Audio>,
    mut ev_missile_moved: EventReader<MissileMovedEvent>,
    tank_position_query: Query<(&Tank, &Position), Without<Missile>>,
) {
    for ev in ev_missile_moved.iter() {
        for &(x, y) in ev.path.iter() {
            let is_hit = tank_position_query
                .iter()
                .any(|(tank, position)| tank.has_collision(position.0, (x as f32, y as f32)));
            if is_hit {
                debug!("Hit to tank: {:?}", (x, y));
                commands.entity(ev.missile).despawn();
                spawn_explosion(&mut commands, &game_field, Vec2::new(x as f32, y as f32));
                audio.play(game_field.explosion_sound.clone());
                break;
            }
        }
    }
}

fn remove_dead_tank_system(
    mut commands: Commands,
    audio: Res<Audio>,
    mut game_field: ResMut<GameField>,
    health_query: Query<(&Health, &Position, Entity), Changed<Health>>,
) {
    for (health, position, entity) in health_query.iter() {
        if health.0 == 0 {
            debug!("Explode tank");
            spawn_explosion(&mut commands, &game_field, position.0);
            audio.play(game_field.explosion_sound.clone());
            game_field.remove_tank_by_entity(entity);
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_collision() {
        let tank_position = Vec2::new(10.0 + TANK_SIZE / 2., 20.0 - TANK_SIZE / 2.);
        let mut tank = Tank::new(1, 0);

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
