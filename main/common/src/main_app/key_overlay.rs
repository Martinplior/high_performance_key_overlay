use std::time::Instant;

use crate::{
    key_overlay_core::{
        key_draw_cache::KeyDrawCache, key_handler::KeyHandler, key_message::KeyMessage,
        key_property::KeyProperty,
    },
    main_app::key_shader,
    setting::Setting,
};
use eframe::egui_wgpu;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId};

use crossbeam_channel::Receiver as MpscReceiver;

pub struct KeyOverlay {
    instant_now: Instant,
    key_messages_buffer: Vec<KeyMessage>,
    keys_receiver: MpscReceiver<KeyMessage>,
    egui_ctx: egui::Context,
    background_color: Color32,
    key_shader: key_shader::CustomCallback,
    font_family: FontFamily,
    key_handler: KeyHandler,
}

impl KeyOverlay {
    pub const FONT_FAMILY_NAME: &str = "key_overlay_font";

    pub fn new(
        cc: &eframe::CreationContext,
        egui_ctx: &egui::Context,
        setting: Setting,
        keys_receiver: MpscReceiver<KeyMessage>,
    ) -> Self {
        let Setting {
            window_setting,
            font_name,
            background_color,
            key_properties,
            ..
        } = &setting;

        let background_color = Color32::from(*background_color);

        Self::init_fonts(egui_ctx, font_name);

        let instant_now = Instant::now();
        let window_size = [window_setting.width, window_setting.height];
        let key_shader = key_shader::CustomCallback::new(cc, key_properties, window_size);
        let font_family = egui::FontFamily::Name(Self::FONT_FAMILY_NAME.into());
        let key_handler = KeyHandler::new(setting);
        Self {
            instant_now,
            key_messages_buffer: Vec::with_capacity(64),
            keys_receiver,
            egui_ctx: egui_ctx.clone(),
            background_color,
            key_shader,
            font_family,
            key_handler,
        }
    }

    pub fn reload(&mut self, setting: &Setting, reload_font: bool) {
        let Setting {
            window_setting,
            font_name,
            background_color,
            ..
        } = setting;

        let new_background_color = Color32::from(*background_color);

        reload_font.then(|| Self::init_fonts(&self.egui_ctx, font_name));

        self.background_color = new_background_color;
        self.key_handler.reload(setting);
        let window_size = [window_setting.width, window_setting.height];
        self.key_shader
            .reload(self.key_handler.key_properties(), window_size);
    }

    fn init_fonts(egui_ctx: &egui::Context, font_family: &str) {
        let sys_fonts = font_kit::source::SystemSource::new();
        let mut font_definitions = FontDefinitions::default();
        let font_list = [font_family, Setting::DEFAULT_FONT_NAME, "Segoe UI emoji"];
        let font_list_iter = font_list.iter().filter_map(|font_family| {
            let Ok(family_handle) = sys_fonts.select_family_by_name(font_family) else {
                return None;
            };
            let first_font_handle = family_handle.fonts().first()?;
            let is_ttc = match first_font_handle {
                font_kit::handle::Handle::Path { path, .. } => {
                    matches!(
                        font_kit::font::Font::analyze_path(path).ok()?,
                        font_kit::file_type::FileType::Collection(_)
                    )
                }
                _ => return None,
            };
            let font_data = first_font_handle.load().ok()?.copy_font_data()?;
            let font_data = if is_ttc {
                owned_ttf_parser::OwnedFace::from_vec((*font_data).clone(), 0)
                    .ok()?
                    .into_vec()
            } else {
                (*font_data).clone()
            };
            Some((font_data, font_family.to_string()))
        });

        let font_family_name = egui::FontFamily::Name(Self::FONT_FAMILY_NAME.into());
        let default_proportional = font_definitions
            .families
            .get(&FontFamily::Proportional)
            .expect("unreachable")
            .clone();

        let mut custom_font_names: Vec<_> = font_list_iter
            .map(|(font_data, font_name)| {
                font_definitions
                    .font_data
                    .insert(font_name.clone(), FontData::from_owned(font_data).into());
                font_name
            })
            .collect();
        custom_font_names.extend(default_proportional.clone());
        font_definitions
            .families
            .insert(font_family_name.clone(), custom_font_names);

        let mut font_names: Vec<_> = font_list[1..].iter().map(|x| x.to_string()).collect();
        font_names.extend(default_proportional);
        font_definitions
            .families
            .insert(egui::FontFamily::Proportional, font_names);

        egui_ctx.set_fonts(font_definitions);
    }

    pub fn update(&mut self, instant_now: Instant) {
        self.instant_now = instant_now;
        self.key_messages_buffer
            .extend(self.keys_receiver.try_iter());
        self.key_messages_buffer.drain(..).for_each(|key_message| {
            self.key_handler.update(key_message);
        });
        self.key_handler.remove_outer_bar(instant_now);
    }

    pub fn keys_receiver(&mut self) -> &mut MpscReceiver<KeyMessage> {
        &mut self.keys_receiver
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        painter.rect_filled(
            painter.clip_rect(),
            CornerRadius::ZERO,
            self.background_color,
        );
        KeyDrawingPipeline {
            key_shader: &self.key_shader,
            key_properties: self.key_handler.key_properties(),
            key_draw_caches: self.key_handler.key_draw_caches(),
            font_family: &self.font_family,
            instant_now: self.instant_now,
            painter,
        }
        .draw();
    }

    pub fn need_repaint(&self) -> bool {
        self.key_handler.need_repaint()
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
    fn draw(&self) {
        self.draw_bars();
        self.draw_frames();
        self.draw_key_texts();
        self.draw_counter_texts();
    }

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
                );
                self.painter.rect(
                    rect,
                    CornerRadius::ZERO,
                    if key_draw_cache.begin_hold_instant.is_some() {
                        key_draw_cache.pressed_color
                    } else {
                        Color32::TRANSPARENT
                    },
                    egui::Stroke::new(key_property.thickness, key_draw_cache.frame_color),
                    egui::StrokeKind::Inside,
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
