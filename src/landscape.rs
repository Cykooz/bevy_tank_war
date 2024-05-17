use std::time::Instant;

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use itertools::Itertools;
use noise::{self, Fbm, MultiFractal, NoiseFn, Seedable};
use rand::Rng;

use crate::explosion::{ExplosionMaxRadiusEvent, ExplosionsFinishedEvent};
use crate::game_field::{GameField, GameState};
use crate::missile;
use crate::missile::kill_missile;
use crate::G;

const TIME_SCALE: f32 = 3.0;

pub struct LandscapePlugin;

impl Plugin for LandscapePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SubsidenceFinishedEvent>()
            .add_systems(
                Update,
                (
                    check_missile_collides_with_landscape_system,
                    destroy_by_explosion_system,
                    run_subsidence_after_explosions_system,
                ),
            )
            .add_systems(
                PostUpdate,
                (update_landscape_system, update_landscape_texture_system).chain(),
            );
    }
}

#[derive(Event)]
pub struct SubsidenceFinishedEvent;

#[derive(Debug)]
pub struct Landscape {
    width: u16,
    height: u16,
    buffer: Vec<u8>,
    texture_handle: Handle<Image>,
    noise: Fbm,
    amplitude: f64,
    pub dx: f64,
    changed: bool,
    subsidence_started: Option<Instant>,
    // Last position of virtual pixel of landscape on the way of it falling.
    // Used for calculate speed of fall.
    subsidence_last_pos: u32,
    // "Skip" and "take" used for optimize process of landscape subsidence.
    subsidence_skip: usize,
    subsidence_take: usize,
}

#[derive(Component)]
pub struct LandscapeSprite;

impl Landscape {
    pub fn new(width: u16, height: u16, textures: &mut Assets<Image>) -> Result<Self, String> {
        if width.min(height) == 0 {
            return Err("'width' and 'height' must be greater than 0".into());
        }

        let stride = width as usize;
        let res_size = stride * height as usize;
        let texture = Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0u8; res_size * 4],
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );

        let mut rng = rand::thread_rng();
        let mut landscape = Self {
            width,
            height,
            buffer: vec![0; res_size],
            texture_handle: textures.add(texture),
            amplitude: f64::from(height) / 2.,
            dx: rng.gen_range(0.0..width as f64 / 2.),
            noise: Self::create_noise(width, rng.gen()),
            changed: true,
            subsidence_started: None,
            subsidence_last_pos: 0,
            subsidence_skip: 0,
            subsidence_take: stride,
        };
        landscape.generate();
        Ok(landscape)
    }

    fn create_noise(width: u16, seed: u32) -> Fbm {
        Fbm::new()
            .set_seed(seed)
            .set_octaves(4)
            .set_frequency(2. / f64::from(width))
    }

    #[inline]
    pub fn texture_handle(&self) -> Handle<Image> {
        self.texture_handle.clone()
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.noise = Self::create_noise(self.width, seed);
    }

    pub fn seed(&self) -> u32 {
        self.noise.seed()
    }

    #[inline]
    pub fn changed(&self) -> bool {
        self.changed
    }

    #[inline]
    pub fn set_changed(&mut self) {
        self.changed = true;
    }

    #[inline]
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn generate(&mut self) {
        let stride = self.width as usize;
        let y_center: f64 = f64::from(self.height) / 2.;

        for x in 0..self.width {
            let sx = f64::from(x) + self.dx;
            let value = self.noise.get([sx, 0.]) * self.amplitude;
            let y = (y_center + value).round().max(0.) as usize;
            let y = y.min(self.height as usize);
            let index = y * stride + (x as usize);

            if y > 0 {
                self.buffer
                    .iter_mut()
                    .skip(x as usize)
                    .step_by(stride)
                    .take(y)
                    .for_each(|v| *v = 0);
            }

            self.buffer
                .iter_mut()
                .skip(index)
                .step_by(stride)
                .for_each(|v| *v = 1);
        }
    }

    #[inline]
    fn index(&self, x: i32, y: i32) -> usize {
        // Point (0, 0) located in left bottom corner
        ((self.height as i32 - y - 1) * self.width as i32 + x) as usize
    }

    /// Get mutable slice with row of pixels given length
    pub fn get_pixels_line_mut(&mut self, point: (i32, i32), length: u16) -> Option<&mut [u8]> {
        let (x, y) = point;
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || length == 0 {
            return None;
        }
        let index = self.index(x, y);
        let length = (self.width as i32 - x).min(length as i32) as usize;
        Some(&mut self.buffer[index..index + length])
    }

    pub fn is_not_empty(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }
        let index = self.index(x, y);
        self.buffer[index] > 0
    }

    pub fn subsidence(&mut self) {
        if self.subsidence_started.is_none() {
            debug!("Start subsidence");
            self.subsidence_started = Some(Instant::now());
        }
        self.subsidence_last_pos = 0;
        self.subsidence_skip = 0;
        self.subsidence_take = self.width as usize;
    }

    pub fn is_subsidence(&self) -> bool {
        self.subsidence_started.is_some()
    }

    /// Returns `true` if current subsidence has finished.
    pub fn update(&mut self) -> bool {
        if let Some(subsidence_started) = self.subsidence_started {
            let time = subsidence_started.elapsed().as_secs_f32();
            let subsidence_cur_pos = (G * time * time * TIME_SCALE).round() as u32;
            let delta = subsidence_cur_pos - self.subsidence_last_pos;
            self.subsidence_last_pos = subsidence_cur_pos;
            let stride = self.width as usize;

            for _ in 0..delta {
                let mut changed = false;
                let mut cur_row_index = stride * self.height as usize;
                let mut left_changed_pos: usize = self.subsidence_take;
                let mut right_changed_pos = 0;

                for _ in 1..self.height {
                    cur_row_index -= stride;
                    let (top_rows, current_row) = self.buffer.split_at_mut(cur_row_index);
                    let (_, top_row) = top_rows.split_at_mut(cur_row_index - stride);
                    let pixels_for_change = top_row
                        .iter_mut()
                        .zip(current_row)
                        .skip(self.subsidence_skip)
                        .take(self.subsidence_take)
                        .enumerate()
                        .filter(|(_, (&mut top_pixel, &mut cur_pixel))| {
                            cur_pixel == 0 && top_pixel != 0
                        });
                    let min_max = pixels_for_change
                        .map(|(i, (top_pixel, cur_pixel))| {
                            *cur_pixel = *top_pixel;
                            *top_pixel = 0;
                            i
                        })
                        .minmax();

                    if let Some((min, max)) = min_max.into_option() {
                        changed = true;
                        left_changed_pos = left_changed_pos.min(min);
                        right_changed_pos = right_changed_pos.max(max);
                    };
                }

                self.subsidence_skip += left_changed_pos;
                self.subsidence_take = right_changed_pos + 1;

                if changed {
                    self.changed = true;
                } else {
                    debug!("Subsidence has end");
                    self.subsidence_started = None;
                    return true;
                }
            }
        }

        false
    }

    pub fn destroy_circle(&mut self, position: Vec2, radius: i32) {
        let mut landscape_changed = false;
        let circle =
            line_drawing::BresenhamCircle::new(position.x as i32, position.y as i32, radius - 1);
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
                if let Some(pixels) = self.get_pixels_line_mut((x, y), len) {
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
            self.set_changed();
        }
    }
}

pub fn update_landscape_system(
    mut game_field: ResMut<GameField>,
    mut finished_event: EventWriter<SubsidenceFinishedEvent>,
) {
    let landscape = &mut game_field.landscape;
    if landscape.update() {
        finished_event.send(SubsidenceFinishedEvent);
    }
}

pub fn update_landscape_texture_system(
    mut textures: ResMut<Assets<Image>>,
    mut game_field: ResMut<GameField>,
) {
    let landscape = &mut game_field.landscape;
    if landscape.changed() {
        if let Some(texture) = textures.get_mut(&landscape.texture_handle) {
            let buf = unsafe { texture.data.align_to_mut::<u32>().1 };
            for (&v, d) in landscape.buffer.iter().zip(buf) {
                *d = if v == 0 { 0 } else { 0xff_40_71_9c } // 0xff_cf_bd_00
            }
            landscape.changed = false;
        }
    }
}

pub fn scroll_landscape(
    time: Res<Time>,
    keyboard_input: ResMut<ButtonInput<KeyCode>>,
    mut game_field: ResMut<GameField>,
) {
    let landscape = &mut game_field.landscape;

    let delta_seconds = f64::min(0.2, time.delta_seconds_f64());
    const SPEED: f64 = 1.;
    let mut changed = false;

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        landscape.dx -= SPEED;
        changed = true;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        landscape.dx += SPEED;
        changed = true;
    }

    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        let seed = landscape.seed().wrapping_add(1);
        landscape.set_seed(seed);
        changed = true;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        let seed = landscape.seed().wrapping_sub(1);
        landscape.set_seed(seed);
        changed = true;
    }

    if changed {
        landscape.generate();
        landscape.set_changed();
    }
}

pub fn check_missile_collides_with_landscape_system(
    mut commands: Commands,
    game_field: Res<GameField>,
    mut ev_missile_moved: EventReader<missile::MissileMovedEvent>,
) {
    let landscape = &game_field.landscape;
    for ev in ev_missile_moved.read() {
        for &(x, y) in ev.path.iter() {
            if landscape.is_not_empty(x, y) {
                debug!("Hit to landscape: {:?}", (x, y));
                kill_missile(&mut commands, ev.missile, x, y);
                break;
            }
        }
    }
}

fn destroy_by_explosion_system(
    mut game_field: ResMut<GameField>,
    mut radius_events: EventReader<ExplosionMaxRadiusEvent>,
) {
    let landscape = &mut game_field.landscape;
    for event in radius_events.read() {
        landscape.destroy_circle(event.position, event.max_radius as i32)
    }
}

fn run_subsidence_after_explosions_system(
    mut game_field: ResMut<GameField>,
    mut finish_events: EventReader<ExplosionsFinishedEvent>,
) {
    if finish_events.read().count() > 0 {
        game_field.landscape.subsidence();
    }
}
