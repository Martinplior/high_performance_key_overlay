use std::time::Instant;

use eframe::egui_wgpu;
use egui::{Color32, CornerRadius, FontId};

use crate::{
    key::Key, key_bar::KeyBar, key_draw_cache::KeyDrawCache, key_message::KeyMessage,
    key_overlay::KeyOverlay, key_property::KeyProperty, key_shader, setting::Setting,
};

#[derive(Debug)]
struct KeyMap {
    map: Box<[Option<Box<[usize]>>; Self::CAP]>,
}

impl KeyMap {
    const CAP: usize = Key::LAST_KEY as usize;

    fn new(key_properties: &[KeyProperty]) -> Self {
        let init = || -> Option<_> {
            let mut init_map: Box<[_; Self::CAP]> = Box::new(std::array::from_fn(|_| Some(vec![])));
            let iter = key_properties
                .iter()
                .filter(|key_property| key_property.key_bind != Key::Unknown)
                .enumerate();
            for (index, key_property) in iter {
                let indexes = init_map.get_mut(key_property.key_bind as usize)?.as_mut()?;
                indexes.push(index);
            }
            let map = Box::new(std::array::from_fn(|index| {
                let init_vec = init_map.get_mut(index).unwrap().take().unwrap();
                if init_vec.is_empty() {
                    None
                } else {
                    Some(init_vec.into_boxed_slice())
                }
            }));
            Some(map)
        };

        let map = init().unwrap();

        Self { map }
    }

    fn get(&self, key: Key) -> Option<&[usize]> {
        self.map.get(key as usize).unwrap().as_deref()
    }
}

pub struct KeyHandler {
    key_properties: Box<[KeyProperty]>,
    key_draw_caches: Box<[KeyDrawCache]>,
    key_shader: key_shader::CustomCallback,
    key_map: KeyMap,
    font_family: egui::FontFamily,
}

impl KeyHandler {
    pub fn new(cc: &eframe::CreationContext, setting: Setting) -> Self {
        let Setting {
            window_setting,
            key_properties,
            ..
        } = setting;
        let key_properties = key_properties.into_boxed_slice();
        let window_size = [window_setting.width, window_setting.height];
        let key_shader = key_shader::CustomCallback::new(cc, &key_properties, window_size);
        let key_map = KeyMap::new(&key_properties);
        let key_draw_caches = key_properties
            .iter()
            .map(|key_property| {
                KeyDrawCache::new(&window_setting, key_property.bar_speed, key_property)
            })
            .collect();
        let font_family = egui::FontFamily::Name(KeyOverlay::FONT_FAMILY_NAME.into());
        Self {
            key_properties,
            key_draw_caches,
            key_shader,
            key_map,
            font_family,
        }
    }

    pub fn reload(&mut self, setting: &Setting) {
        let Setting {
            window_setting,
            key_properties,
            ..
        } = setting;
        let new_key_properties = key_properties.clone().into_boxed_slice();
        let new_key_map = KeyMap::new(&new_key_properties);
        let new_key_draw_caches = new_key_properties
            .iter()
            .map(|key_property| {
                KeyDrawCache::new(window_setting, key_property.bar_speed, key_property)
            })
            .collect();

        let window_size = [window_setting.width, window_setting.height];
        self.key_shader.reload(&new_key_properties, window_size);
        self.key_properties = new_key_properties;
        self.key_map = new_key_map;
        self.key_draw_caches = new_key_draw_caches;
    }

    pub fn update(&mut self, key_message: KeyMessage) {
        debug_assert!(key_message.key != Key::Unknown);

        let mut inner_update = |indexes: &[usize]| -> Option<()> {
            let first_key_draw_cache = self.key_draw_caches.get_mut(*indexes.first()?)?;

            let now_pressed = key_message.is_pressed;
            let prev_pressed = first_key_draw_cache.begin_hold_instant.is_some();

            match (prev_pressed, now_pressed) {
                (false, true) => {
                    for index in indexes.iter() {
                        let key_property = self.key_properties.get_mut(*index)?;
                        let key_draw_cache = self.key_draw_caches.get_mut(*index)?;
                        if key_property.key_counter.0 {
                            key_draw_cache.increase_count();
                        }
                        key_draw_cache.begin_hold_instant = Some(key_message.instant);
                    }
                }
                (true, false) => {
                    for index in indexes.iter() {
                        let key_draw_cache = self.key_draw_caches.get_mut(*index)?;
                        let bar = KeyBar::new(
                            key_draw_cache.begin_hold_instant.take()?,
                            key_message.instant,
                        );
                        key_draw_cache.add_bar(bar);
                    }
                }
                _ => (),
            }
            Some(())
        };

        self.key_map
            .get(key_message.key)
            .map(|indexes| unsafe { inner_update(indexes).unwrap_unchecked() });
    }

    pub fn remove_outer_bar(&mut self, instant_now: Instant) {
        self.key_draw_caches.iter_mut().for_each(|key_draw_cache| {
            key_draw_cache.remove_outer_bar(instant_now);
        });
    }

    pub fn draw_on(&self, painter: &egui::Painter, instant_now: Instant) {
        let key_drawing_pipeline = KeyDrawingPipeline {
            key_shader: &self.key_shader,
            key_properties: &self.key_properties,
            key_draw_caches: &self.key_draw_caches,
            font_family: &self.font_family,
            instant_now,
            painter,
        };
        key_drawing_pipeline.draw_bars();
        key_drawing_pipeline.draw_frames();
        key_drawing_pipeline.draw_key_texts();
        key_drawing_pipeline.draw_counter_texts();
    }

    pub fn need_repaint(&self) -> bool {
        self.key_draw_caches
            .iter()
            .any(|key_draw_cache| key_draw_cache.need_repaint())
    }
}

struct KeyDrawingPipeline<'a> {
    key_shader: &'a key_shader::CustomCallback,
    key_properties: &'a [KeyProperty],
    key_draw_caches: &'a [KeyDrawCache],
    font_family: &'a egui::FontFamily,
    instant_now: Instant,
    painter: &'a egui::Painter,
}

impl<'a> KeyDrawingPipeline<'a> {
    fn draw_bars(&self) {
        let instant_now = self.instant_now;
        let painter = self.painter;

        let mut key_shader_inner = self.key_shader.inner.lock();
        key_shader_inner.bar_rects.clear();
        let bar_rect_iter =
            self.key_draw_caches
                .iter()
                .enumerate()
                .flat_map(|(index, key_drawer)| {
                    key_drawer
                        .begin_hold_instant
                        .iter()
                        .map(move |begin_hold_instant| {
                            let begin_duration_secs = instant_now
                                .duration_since(*begin_hold_instant)
                                .as_secs_f32();
                            key_shader::BarRect {
                                property_index: index as u32,
                                begin_duration_secs,
                                end_duration_secs: 0.0,
                            }
                        })
                        .chain(key_drawer.bar_queue.iter().map(move |key_bar| {
                            let begin_duration_secs = instant_now
                                .duration_since(key_bar.press_instant)
                                .as_secs_f32();
                            let end_duration_secs = instant_now
                                .duration_since(key_bar.release_instant)
                                .as_secs_f32();
                            key_shader::BarRect {
                                property_index: index as u32,
                                begin_duration_secs,
                                end_duration_secs,
                            }
                        }))
                });
        key_shader_inner.bar_rects.extend(bar_rect_iter);
        if key_shader_inner.bar_rects.is_empty() {
            return;
        }
        drop(key_shader_inner);
        painter.add(egui_wgpu::Callback::new_paint_callback(
            painter.clip_rect(),
            self.key_shader.clone(),
        ));
    }

    fn draw_frames(&self) {
        self.key_properties
            .iter()
            .zip(self.key_draw_caches.iter())
            .for_each(|(key_property, key_draw_cache)| {
                let rect = egui::Rect::from_min_size(
                    key_property.position,
                    [key_property.width, key_property.height].into(),
                )
                .shrink(key_property.thickness);
                self.painter.rect(
                    rect,
                    CornerRadius::ZERO,
                    if key_draw_cache.begin_hold_instant.is_some() {
                        key_draw_cache.pressed_color
                    } else {
                        Color32::TRANSPARENT
                    },
                    egui::Stroke::new(key_property.thickness, key_draw_cache.frame_color),
                    egui::StrokeKind::Outside,
                );
            });
    }

    fn draw_key_texts(&self) {
        self.key_properties
            .iter()
            .zip(self.key_draw_caches.iter())
            .for_each(|(key_property, key_draw_cache)| {
                self.painter.text(
                    key_property.position
                        + egui::vec2(key_property.width / 2.0, key_property.height / 2.0),
                    egui::Align2::CENTER_CENTER,
                    &key_property.key_text,
                    FontId::new(key_property.font_size, self.font_family.clone()),
                    key_draw_cache.key_text_color,
                );
            });
    }

    fn draw_counter_texts(&self) {
        self.key_properties
            .iter()
            .zip(self.key_draw_caches.iter())
            .filter(|(key_property, _)| key_property.key_counter.0)
            .for_each(|(key_property, key_draw_cache)| {
                let counter = &key_property.key_counter.1;
                self.painter.text(
                    key_property.position
                        + egui::vec2(key_property.width / 2.0, key_property.height / 2.0)
                        + counter.position.to_vec2(),
                    egui::Align2::CENTER_CENTER,
                    key_draw_cache.count,
                    FontId::new(counter.font_size, self.font_family.clone()),
                    key_draw_cache.key_counter_color,
                );
            });
    }
}
