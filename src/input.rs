use std::hash::Hash;
use std::ops::Add;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Debug, Clone)]
pub struct InputWithRepeating<T: Eq + Hash> {
    next_tick: HashMap<T, Instant>,
}

impl<T: Eq + Hash> Default for InputWithRepeating<T> {
    fn default() -> Self {
        Self {
            next_tick: Default::default(),
        }
    }
}

impl<T> InputWithRepeating<T>
where
    T: Copy + Eq + Hash,
{
    pub fn pressed(&mut self, input: &Input<T>, key_code: T) -> bool {
        if input.pressed(key_code) {
            let now = Instant::now();
            if let Some(next_tick) = self.next_tick.get_mut(&key_code) {
                if *next_tick <= now {
                    *next_tick = now.add(Duration::from_millis(25));
                    true
                } else {
                    false
                }
            } else {
                self.next_tick
                    .insert(key_code, now.add(Duration::from_millis(500)));
                true
            }
        } else {
            self.next_tick.remove(&key_code);
            false
        }
    }
}
