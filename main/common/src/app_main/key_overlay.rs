use std::time::Instant;

use crate::{
    app_main::key_shader,
    key_overlay_core::{KeyOverlayCore, key_handler::KeyHandler, key_message::KeyMessage},
    setting::Setting,
};
use eframe::egui_wgpu;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId};

use sak_rs::font::SystemFontsLoader;
use sak_rs::sync::mpmc::queue::BoundedReceiver as MpscReceiver;

pub struct KeyOverlay {
    core: KeyOverlayCore,
    instant_now: Instant,
    egui_ctx: egui::Context,
    background_color: Color32,
    key_shader: key_shader::CustomCallback,
    font_family: FontFamily,
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

        Self::init_fonts(egui_ctx, [&**font_name]);

        let instant_now = Instant::now();
        let window_size = [window_setting.width, window_setting.height];
        let key_shader = key_shader::CustomCallback::new(cc, key_properties, window_size);
        let font_family = egui::FontFamily::Name(Self::FONT_FAMILY_NAME.into());
        let core = KeyOverlayCore::new(setting, keys_receiver);
        Self {
            core,
            instant_now,
            egui_ctx: egui_ctx.clone(),
            background_color,
            key_shader,
            font_family,
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

        reload_font.then(|| Self::init_fonts(&self.egui_ctx, [&**font_name]));

        self.background_color = new_background_color;
        self.core.reload(setting);
        let window_size = [window_setting.width, window_setting.height];
        self.key_shader
            .reload(self.core.key_handler().key_properties(), window_size);
    }

    fn init_fonts<'a>(
        egui_ctx: &egui::Context,
        custom_font_names: impl IntoIterator<Item = &'a str> + 'a,
    ) {
        let fonts_loader = SystemFontsLoader::new();
        let mut font_definitions = FontDefinitions::default();

        let font_list: Vec<_> = custom_font_names
            .into_iter()
            .chain(crate::DEFAULT_FONT_NAMES)
            .map(|x| x.to_string())
            .collect();
        let font_list_iter = font_list.iter().filter_map(|font_family| {
            let font_data = fonts_loader.load_by_family_name(font_family).ok()?;
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
        self.core.update(instant_now);
    }

    pub fn keys_receiver(&self) -> &MpscReceiver<KeyMessage> {
        self.core.keys_receiver()
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
            key_handler: self.core.key_handler(),
            font_family: &self.font_family,
            instant_now: self.instant_now,
            painter,
        }
        .draw();
    }

    pub fn need_repaint(&self) -> bool {
        self.core.need_repaint()
    }
}

struct KeyDrawingPipeline<'a> {
    key_shader: &'a key_shader::CustomCallback,
    key_handler: &'a KeyHandler,
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
        let bar_rect_iter = self.key_handler.key_draw_caches_flat_map_iter(
            instant_now,
            &|index, begin_duration_secs| key_shader::BarRect {
                property_index: index as u32,
                begin_duration_secs,
                end_duration_secs: 0.0,
            },
            &|index, begin_duration_secs, end_duration_secs| key_shader::BarRect {
                property_index: index as u32,
                begin_duration_secs,
                end_duration_secs,
            },
        );
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
        self.key_handler
            .key_properties()
            .iter()
            .zip(self.key_handler.key_draw_caches().iter())
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
        self.key_handler
            .key_properties()
            .iter()
            .zip(self.key_handler.key_draw_caches().iter())
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
        self.key_handler
            .key_properties()
            .iter()
            .zip(self.key_handler.key_draw_caches().iter())
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
