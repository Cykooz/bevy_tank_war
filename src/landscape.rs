use std::time::Instant;

use bevy::prelude::*;
use bevy::render::texture::{Extent3d, TextureDimension, TextureFormat};
use itertools::Itertools;
use noise::{self, Fbm, MultiFractal, NoiseFn, Seedable};
use rand::Rng;

use crate::explosion::spawn_explosion;
use crate::game_field::GameField;
use crate::missile;
use crate::G;

const TIME_SCALE: f32 = 3.0;

pub struct LandscapePlugin;

impl Plugin for LandscapePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<SubsidenceFinishedEvent>()
            .add_system(update_landscape.system())
            .add_system(update_landscape_texture.system())
            .add_system(
                missile_collides_with_landscape_system
                    .system()
                    .after(missile::MISSILE_MOVED_LABEL),
            );
    }
}

pub struct SubsidenceFinishedEvent;

#[derive(Debug)]
pub struct Landscape {
    width: u16,
    height: u16,
    buffer: Vec<u8>,
    texture_handle: Handle<Texture>,
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

pub struct LandscapeSprite;

impl Landscape {
    pub fn new(width: u16, height: u16, textures: &mut Assets<Texture>) -> Result<Self, String> {
        if width.min(height) == 0 {
            return Err("'width' and 'height' must be greater than 0".into());
        }

        let stride = width as usize;
        let res_size = stride * height as usize;
        let texture = Texture::new(
            Extent3d::new(width as u32, height as u32, 1),
            TextureDimension::D2,
            vec![0u8; res_size * 4],
            TextureFormat::Rgba8UnormSrgb,
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
    pub fn texture_handle(&self) -> Handle<Texture> {
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
}

pub fn update_landscape(
    mut game_field: ResMut<GameField>,
    mut finished_event: EventWriter<SubsidenceFinishedEvent>,
) {
    let landscape = &mut game_field.landscape;
    if landscape.update() {
        finished_event.send(SubsidenceFinishedEvent);
    }
}

pub fn update_landscape_texture(
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut game_field: ResMut<GameField>,
    mut material_query: Query<&Handle<ColorMaterial>, With<LandscapeSprite>>,
) {
    let landscape = &mut game_field.landscape;
    if landscape.changed() {
        if let Some(material_handle) = material_query.iter_mut().next() {
            if materials.get_mut(material_handle).is_some() {
                if let Some(texture) = textures.get_mut(&landscape.texture_handle) {
                    let buf = unsafe { texture.data.align_to_mut::<u32>().1 };
                    for (&v, d) in landscape.buffer.iter().zip(buf) {
                        *d = if v == 0 { 0 } else { 0xff_40_71_9c } // 0xff_cf_bd_00
                    }
                    landscape.changed = false;
                }
            }
        }
    }
}

pub fn scroll_landscape(
    time: Res<Time>,
    keyboard_input: ResMut<Input<KeyCode>>,
    mut game_field: ResMut<GameField>,
) {
    let landscape = &mut game_field.landscape;

    let delta_seconds = f64::min(0.2, time.delta_seconds_f64());
    const SPEED: f64 = 1.;
    let mut changed = false;

    if keyboard_input.pressed(KeyCode::Left) {
        landscape.dx -= SPEED;
        changed = true;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        landscape.dx += SPEED;
        changed = true;
    }

    if keyboard_input.just_pressed(KeyCode::Up) {
        let seed = landscape.seed().wrapping_add(1);
        landscape.set_seed(seed);
        changed = true;
    }
    if keyboard_input.just_pressed(KeyCode::Down) {
        let seed = landscape.seed().wrapping_sub(1);
        landscape.set_seed(seed);
        changed = true;
    }

    if changed {
        landscape.generate();
        landscape.set_changed();
    }
}

pub fn missile_collides_with_landscape_system(
    mut commands: Commands,
    game_field: Res<GameField>,
    audio: Res<Audio>,
    mut ev_missile_moved: EventReader<missile::MissileMovedEvent>,
) {
    let landscape = &game_field.landscape;
    for ev in ev_missile_moved.iter() {
        for &(x, y) in ev.path.iter() {
            if landscape.is_not_empty(x, y) {
                debug!("Hit to landscape: {:?}", (x, y));
                commands.entity(ev.missile).despawn();
                spawn_explosion(&mut commands, &game_field, Vec2::new(x as f32, y as f32));
                audio.play(game_field.explosion_sound.clone());
                break;
            }
        }
    }
}
