use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use egui::Color32;

use crate::{
    key_overlay_core::{
        key_bar::KeyBar,
        key_property::{KeyDirection, KeyProperty},
    },
    setting::WindowSetting,
};

#[derive(Debug)]
pub struct KeyDrawCache {
    pub key_text_color: Color32,
    pub frame_color: Color32,
    pub pressed_color: Color32,
    pub key_counter_color: Color32,
    pub max_bar_duration: Duration,
    pub count: u32,
    pub bar_queue: VecDeque<KeyBar>,
    pub begin_hold_instant: Option<Instant>,
}

impl KeyDrawCache {
    pub fn new(window_setting: &WindowSetting, bar_speed: f32, key_property: &KeyProperty) -> Self {
        let max_distance = if let (true, max_distantce) = key_property.max_distance {
            max_distantce
        } else {
            match key_property.key_direction {
                KeyDirection::Up => key_property.position.y,
                KeyDirection::Down => {
                    window_setting.height - key_property.position.y - key_property.height
                }
                KeyDirection::Left => key_property.position.x,
                KeyDirection::Right => {
                    window_setting.width - key_property.position.x - key_property.width
                }
            }
            .max(0.0)
        };
        let max_bar_duration = Duration::from_secs_f32(max_distance / bar_speed);
        Self {
            bar_queue: VecDeque::with_capacity(64),
            key_text_color: key_property.text_color.into(),
            frame_color: key_property.frame_color.into(),
            pressed_color: key_property.pressed_color.into(),
            key_counter_color: key_property.key_counter.1.text_color.into(),
            max_bar_duration,
            count: 0,
            begin_hold_instant: None,
        }
    }

    pub fn increase_count(&mut self) {
        self.count = self.count.wrapping_add(1);
    }

    pub fn need_repaint(&self) -> bool {
        !self.bar_queue.is_empty() || self.begin_hold_instant.is_some()
    }

    pub fn add_bar(&mut self, bar: KeyBar) {
        self.bar_queue.push_back(bar);
    }

    pub fn remove_outer_bar(&mut self, instant_now: Instant) {
        let dead_line = instant_now - self.max_bar_duration;
        while let Some(bar) = self.bar_queue.front() {
            if bar.release_instant < dead_line {
                self.bar_queue.pop_front();
            } else {
                break;
            }
        }
    }
}
