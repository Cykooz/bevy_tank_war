use bevy::prelude::*;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct Ballistics {
    created: Instant,
    start_pos: Vec2,
    start_velocity: Vec2,
    acceleration: Vec2,
    cur_pos: Vec2,
    last_updated: f32,
    time_scale: f32,
    rebound_efficiency: f32,
}

impl Ballistics {
    pub fn new<V>(start_pos: V, start_velocity: V, acceleration: V) -> Ballistics
    where
        V: Into<Vec2>,
    {
        let start_pos = start_pos.into();
        Ballistics {
            created: Instant::now(),
            start_pos,
            start_velocity: start_velocity.into(),
            acceleration: acceleration.into(),
            cur_pos: start_pos,
            last_updated: 0.0,
            time_scale: 1.0,
            rebound_efficiency: 1.0,
        }
    }

    pub fn time_scale(self, value: f32) -> Self {
        Self {
            time_scale: value,
            last_updated: 0.0,
            ..self
        }
    }

    pub fn rebound_efficiency(self, value: f32) -> Self {
        Self {
            rebound_efficiency: value,
            ..self
        }
    }

    #[inline]
    fn velocity(&self, time: f32) -> Vec2 {
        self.start_velocity + self.acceleration * time * 2.0
    }

    #[inline]
    fn pos(&self, time: f32) -> Vec2 {
        self.start_pos + (self.start_velocity + self.acceleration * time) * time
    }

    #[inline]
    pub fn cur_pos(&self) -> Vec2 {
        self.cur_pos
    }

    /// Returns current position and velocity
    #[inline]
    pub fn pos_and_velocity(&self) -> (Vec2, Vec2) {
        (
            self.pos(self.last_updated),
            self.velocity(self.last_updated),
        )
    }

    fn apply_rebound(&mut self, horizontal: bool, vertical: bool) {
        let (pos, mut velocity) = self.pos_and_velocity();
        if horizontal {
            velocity.x = -velocity.x;
        }
        if vertical {
            velocity.y = -velocity.y;
        }

        self.start_pos = pos;
        self.start_velocity = velocity * self.rebound_efficiency;
        self.cur_pos = pos;
        self.created = Instant::now();
        self.last_updated = 0.0;
    }

    pub fn positions_iter(
        &mut self,
        end_time: Option<f32>,
        borders: Option<(i32, i32)>,
    ) -> BallisticsPosIterator {
        let start_time = self.last_updated;
        let end_time =
            end_time.unwrap_or_else(|| self.created.elapsed().as_secs_f32()) * self.time_scale;

        let start_velocity = self.velocity(start_time);
        let end_velocity = self.velocity(end_time);
        let max_velocity = start_velocity
            .x
            .abs()
            .max(start_velocity.y.abs())
            .max(end_velocity.x.abs())
            .max(end_velocity.y.abs());

        let time_period = end_time - start_time;

        let time_step = if max_velocity == 0.0 {
            time_period
        } else {
            1.0 / (2.0 * max_velocity)
        };
        let last_pos = (self.cur_pos.x.floor() as i32, self.cur_pos.y.floor() as i32);

        BallisticsPosIterator {
            ballistics: self,
            end_time,
            time_step,
            last_time: start_time,
            last_pos,
            borders,
        }
    }
}

pub struct BallisticsPosIterator<'a> {
    ballistics: &'a mut Ballistics,
    end_time: f32,
    time_step: f32,
    last_time: f32,
    last_pos: (i32, i32),
    borders: Option<(i32, i32)>,
}

impl<'a> Iterator for BallisticsPosIterator<'a> {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        let mut next_time = self.last_time;
        let mut rebound_on_prev_step = false;

        while next_time <= self.end_time {
            next_time += self.time_step;
            let clamped_time = self.end_time.min(next_time);

            let pos = self.ballistics.pos(clamped_time);
            let pos_i32 = (pos.x.floor() as i32, pos.y.floor() as i32);
            if self.last_pos == pos_i32 {
                continue;
            }

            if let Some((width, height)) = self.borders {
                let (x, y) = pos_i32;
                let horizontal_rebound = x < 0 || x >= width;
                let vertical_rebound = y < 0 || y > height;
                if horizontal_rebound || vertical_rebound {
                    self.ballistics
                        .apply_rebound(horizontal_rebound, vertical_rebound);
                    if rebound_on_prev_step {
                        self.ballistics.start_velocity.x = 0.0;
                        self.ballistics.acceleration.x = 0.0;
                    }
                    self.end_time -= self.last_time;
                    self.last_time = 0.0;
                    next_time = 0.0;
                    rebound_on_prev_step = true;
                    continue;
                }
            }

            self.last_time = next_time;
            self.last_pos = pos_i32;
            self.ballistics.last_updated = clamped_time;
            self.ballistics.cur_pos = pos;
            return Some(pos_i32);
        }

        self.ballistics.cur_pos = self.ballistics.pos(self.end_time);
        self.ballistics.last_updated = self.end_time;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positions_iter_horizontal() {
        const TIME_SCALE: f32 = 1.0;
        let pos = [0., 0.];
        let acceleration = [0.0, 0.0];
        let velocity = [100.0, 0.0];
        let mut ballistics = Ballistics::new(pos, velocity, acceleration).time_scale(TIME_SCALE);

        //assert_eq!(ballistics.pos_i32(10.0), (3000, 0));

        let mut pos_iterator = ballistics.positions_iter(Some(10.0), None);
        for x in 1..=1000 {
            assert_eq!(pos_iterator.next(), Some((x, 0)));
        }
        assert_eq!(pos_iterator.next(), None);
        assert!((ballistics.last_updated - 10.0).abs() < f32::EPSILON);
        assert!((ballistics.cur_pos.x - 1000.0).abs() < f32::EPSILON);

        let mut pos_iterator = ballistics.positions_iter(Some(20.0), None);
        for x in 1001..=2000 {
            assert_eq!(pos_iterator.next(), Some((x, 0)));
        }
        assert_eq!(pos_iterator.next(), None);
        assert!((ballistics.last_updated - 20.0).abs() < f32::EPSILON);
        assert!((ballistics.cur_pos.x - 2000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_positions_iter_vertical() {
        const TIME_SCALE: f32 = 1.0;
        let pos = [0., 0.];
        let acceleration = [0., 0.];
        let velocity = [0., 100.];
        let mut ballistics = Ballistics::new(pos, velocity, acceleration).time_scale(TIME_SCALE);

        //assert_eq!(missile.pos(10.0 * TIME_SCALE).y, -3000.0);

        let mut pos_iterator = ballistics.positions_iter(Some(10.0), None);
        for y in 1..=999 {
            assert_eq!(pos_iterator.next(), Some((0, y)));
        }
        assert_eq!(pos_iterator.next(), Some((0, 1000)));
        assert_eq!(pos_iterator.next(), None);
        assert!((ballistics.last_updated - 10.0).abs() < f32::EPSILON);
        assert!((ballistics.cur_pos.y - 1000.0) < f32::EPSILON);
    }
}
