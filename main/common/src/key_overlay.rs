use std::time::Instant;

use crate::{key_handler::KeyHandler, key_message::KeyMessage, setting::Setting};
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily};

use crossbeam::channel::Receiver as MpscReceiver;

pub struct KeyOverlay {
    instant_now: Instant,
    key_messages_buffer: Vec<KeyMessage>,
    keys_receiver: MpscReceiver<KeyMessage>,
    egui_ctx: egui::Context,
    background_color: Color32,
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
            font_name,
            background_color,
            ..
        } = &setting;

        let background_color = background_color.clone().into();

        Self::init_fonts(egui_ctx, &font_name);

        let instant_now = Instant::now();
        let key_handler = KeyHandler::new(cc, setting);
        Self {
            instant_now,
            key_messages_buffer: Vec::with_capacity(64),
            keys_receiver,
            egui_ctx: egui_ctx.clone(),
            background_color,
            key_handler,
        }
    }

    pub fn load_setting(&mut self, setting: Setting, reload_font: bool) {
        let Setting {
            font_name,
            background_color,
            ..
        } = &setting;

        let new_background_color = background_color.clone().into();

        reload_font.then(|| Self::init_fonts(&self.egui_ctx, &font_name));

        self.background_color = new_background_color;
        self.key_handler.reload(&setting);
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
            .unwrap()
            .clone();

        let mut custom_font_names: Vec<_> = font_list_iter
            .map(|(font_data, font_name)| {
                font_definitions
                    .font_data
                    .insert(font_name.clone(), FontData::from_owned(font_data).into());
                font_name
            })
            .collect();
        custom_font_names.extend(default_proportional.clone().into_iter());
        font_definitions
            .families
            .insert(font_family_name.clone(), custom_font_names);

        let mut font_names: Vec<_> = font_list[1..].iter().map(|x| x.to_string()).collect();
        font_names.extend(default_proportional.into_iter());
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
        self.key_handler.draw_on(painter, self.instant_now);
    }

    pub fn need_repaint(&self) -> bool {
        self.key_handler.need_repaint()
    }
}
