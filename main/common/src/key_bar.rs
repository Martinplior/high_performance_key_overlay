#![allow(dead_code)]

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct KeyBar {
    pub press_instant: Instant,
    pub release_instant: Instant,
}

impl KeyBar {
    pub fn new(press_instant: Instant, release_instant: Instant) -> Self {
        Self {
            press_instant,
            release_instant,
        }
    }

    /// time * speed = distance
    pub fn compute_pos(duration: Duration, bar_speed: f32) -> f32 {
        duration.as_secs_f32() * bar_speed
    }

    /// head corresponds to `press_instant`
    pub fn get_head_pos(&self, instant_now: Instant, bar_speed: f32) -> f32 {
        Self::compute_pos(instant_now - self.press_instant, bar_speed)
    }

    /// tail corresponds to `release_instant`
    pub fn get_tail_pos(&self, instant_now: Instant, bar_speed: f32) -> f32 {
        Self::compute_pos(instant_now - self.release_instant, bar_speed)
    }

    /// bar_speed: unit: pixel/s
    ///
    /// returns (head, tail)
    ///
    /// head corresponds to `press_instant`, tail corresponds to `release_instant`
    pub fn into_range(&self, instant_now: Instant, bar_speed: f32) -> (f32, f32) {
        (
            self.get_head_pos(instant_now, bar_speed),
            self.get_tail_pos(instant_now, bar_speed),
        )
    }
}
