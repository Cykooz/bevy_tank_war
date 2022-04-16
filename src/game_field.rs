use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::landscape::Landscape;

#[derive(Debug, Clone, Copy)]
pub enum GameState {
    Starting,
    Playing,
    SwitchTank,
    TanksThrowing,
    Aiming,
    Subsidence,
    Finish,
}

impl Default for GameState {
    fn default() -> Self {
        Self::Starting
    }
}

pub struct GameField {
    pub width: u16,
    pub height: u16,
    pub parent_entity: Entity,
    pub landscape: Landscape,
    pub wind_power: f32,
    pub player_numbers: Vec<u8>,
    pub tanks: Vec<Option<Entity>>,
    pub current_tank: usize,
    pub state: GameState,
    pub number_of_iteration: usize,
    pub font: Handle<Font>,
    pub tank_texture: Handle<Image>,
    pub gun_texture: Handle<Image>,
    pub tank_fire_sound: Handle<AudioSource>,
    pub explosion_sound: Handle<AudioSource>,
}

impl GameField {
    pub fn start_round(&mut self, count_of_tanks: u8) {
        let mut player_numbers: Vec<u8> = (1..=count_of_tanks).collect();
        player_numbers.shuffle(&mut rand::thread_rng());
        self.tanks.clear();
        self.player_numbers = player_numbers;
        self.state = GameState::Starting;
        self.number_of_iteration = 0;
        self.current_tank = 0;
        self.change_wind();
    }

    fn change_wind(&mut self) {
        self.wind_power = (rand::thread_rng().gen_range(-10.0_f32..10.0_f32) * 10.0).round() / 10.0;
    }

    pub fn switch_current_tank(&mut self) -> Option<Entity> {
        let mut current_tank = self.current_tank;
        for _ in 0..self.tanks.len() {
            current_tank += 1;
            if current_tank >= self.tanks.len() {
                current_tank = 0;
            }
            if let Some(entity) = self.tanks[current_tank] {
                self.current_tank = current_tank;
                return Some(entity);
            }
        }
        None
    }

    pub fn remove_tank_by_entity(&mut self, entity: Entity) {
        if let Some(tank_entity) = self
            .tanks
            .iter_mut()
            .find(|t| t.map(|e| e == entity).unwrap_or(false))
        {
            *tank_entity = None;
        }
    }
}
