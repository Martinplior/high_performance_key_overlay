use egui::Pos2;
use serde::{Deserialize, Serialize};

use crate::{key::Key, message_dialog, ucolor32::UColor32};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowSetting {
    pub width: f32,
    pub height: f32,
    pub enable_vsync: bool,
}

impl WindowSetting {
    pub const DEFAULT_WIDTH: f32 = 600.;
    pub const DEFAULT_HEIGHT: f32 = 600.;
    pub const DEFAULT_ENABLE_VSYNC: bool = true;
}

impl Default for WindowSetting {
    fn default() -> Self {
        Self {
            width: Self::DEFAULT_WIDTH,
            height: Self::DEFAULT_HEIGHT,
            enable_vsync: Self::DEFAULT_ENABLE_VSYNC,
        }
    }
}

impl WindowSetting {
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_vsync(mut self, enable_vsync: bool) -> Self {
        self.enable_vsync = enable_vsync;
        self
    }
}

pub use v2::Setting;

pub mod v2 {
    use std::io::Seek;

    use crate::key_overlay_core::key_property::{KeyCounterProperty, KeyDirection, KeyProperty};

    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Setting {
        pub window_setting: WindowSetting,
        pub font_name: Box<str>,
        pub background_color: UColor32,
        pub key_properties: Vec<KeyProperty>,
    }

    impl Default for Setting {
        fn default() -> Self {
            Self::default_zxc()
        }
    }

    impl PartialEq for Setting {
        fn eq(&self, other: &Self) -> bool {
            self.window_setting == other.window_setting
                && self.font_name == other.font_name
                && self.background_color == other.background_color
                && self.key_properties.len() == other.key_properties.len()
                && self
                    .key_properties
                    .iter()
                    .zip(other.key_properties.iter())
                    .all(|(l, r)| l == r)
        }
    }

    impl Setting {
        pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, &'static str> {
            let file = std::fs::File::options()
                .read(true)
                .open(path)
                .map_err(|_| "无法读取文件")?;
            let mut reader = std::io::BufReader::new(&file);
            let setting = if let Ok(setting) = serde_json::de::from_reader(&mut reader) {
                setting
            } else {
                reader
                    .seek(std::io::SeekFrom::Start(0))
                    .map_err(|_| "无法读取文件")?;
                let setting_v1: v1::Setting =
                    serde_json::de::from_reader(reader).map_err(|_| "格式错误")?;
                Self {
                    window_setting: setting_v1.window_setting,
                    font_name: setting_v1.font_name,
                    background_color: UColor32::TRANSPARENT,
                    key_properties: setting_v1.key_properties,
                }
            };

            Ok(setting)
        }

        pub fn to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
            let file = std::fs::File::create(path).map_err(|_| "无法写入文件")?;
            let writer = std::io::BufWriter::new(&file);
            serde_json::ser::to_writer_pretty(writer, &self)
                .map_err(|err| format!("serde_json::ser::to_writer_pretty错误: {err}"))?;
            Ok(())
        }

        pub fn load_from_local_setting() -> Self {
            let path = crate::key_overlay_setting_path();
            Self::from_file(&path).unwrap_or_else(|_| {
                let setting = Self::default_zxc();
                let _ = setting
                    .to_file(path)
                    .map(|_| {
                        message_dialog::warning("读取配置文件失败，已生成默认配置").show();
                    })
                    .map_err(|_| {
                        message_dialog::warning(
                            "读取配置文件失败，且无法生成配置文件，使用默认配置",
                        )
                        .show();
                    });
                setting
            })
        }
    }

    impl Setting {
        pub fn default_zxc() -> Self {
            Self {
                window_setting: Default::default(),
                font_name: Self::DEFAULT_FONT_NAME.into(),
                background_color: UColor32::TRANSPARENT,
                key_properties: Self::property_zxc(),
            }
        }

        pub fn default_mouse() -> Self {
            Self {
                window_setting: Default::default(),
                font_name: Self::DEFAULT_FONT_NAME.into(),
                background_color: UColor32::TRANSPARENT,
                key_properties: Self::property_mouse(),
            }
        }

        pub fn default_four_directions() -> Self {
            Self {
                window_setting: Default::default(),
                font_name: Self::DEFAULT_FONT_NAME.into(),
                background_color: UColor32::TRANSPARENT,
                key_properties: Self::property_four_directions(),
            }
        }

        pub fn default_4k() -> Self {
            serde_json::de::from_str(include_str!("../../default_settings/4K.json"))
                .expect("load default setting failed")
        }

        pub fn default_7k() -> Self {
            serde_json::de::from_str(include_str!("../../default_settings/7K.json"))
                .expect("load default setting failed")
        }

        pub fn default_26k() -> Self {
            serde_json::de::from_str(include_str!("../../default_settings/26K.json"))
                .expect("load default setting failed")
        }

        pub fn default_hello_world() -> Self {
            serde_json::de::from_str(include_str!("../../default_settings/HelloWorld.json"))
                .expect("load default setting failed")
        }

        pub fn default_single_counter() -> Self {
            serde_json::de::from_str(include_str!("../../default_settings/单个计数器.json"))
                .expect("load default setting failed")
        }
    }

    impl Setting {
        pub const DEFAULT_FONT_NAME: &str = "Microsoft Yahei";

        fn property_zxc() -> Vec<KeyProperty> {
            let width = 100.0;
            let height = 100.0;
            let thickness = 3.0;
            let font_size = 30.0;
            let pos_x = WindowSetting::DEFAULT_WIDTH / 4.0;
            let pos_x_diff = width / 2.0;
            let pos_y = WindowSetting::DEFAULT_HEIGHT - height - 100.0;
            let fade_effect = Some(50.0);
            let key_counter = Some(
                KeyCounterProperty::default()
                    .with_position(Pos2::new(0.0, height))
                    .with_font_size(font_size),
            );
            let key_1 = KeyProperty::default()
                .with_key_bind(Key::KeyZ)
                .with_key_text("Z".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(pos_x - pos_x_diff, pos_y))
                .with_frame_color(UColor32::RED)
                .with_pressed_color(UColor32::RED.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_counter(key_counter.clone());
            let key_2 = KeyProperty::default()
                .with_key_bind(Key::KeyX)
                .with_key_text("X".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(pos_x * 2.0 - pos_x_diff, pos_y))
                .with_frame_color(UColor32::GREEN)
                .with_pressed_color(UColor32::GREEN.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_counter(key_counter.clone());
            let key_3 = KeyProperty::default()
                .with_key_bind(Key::KeyC)
                .with_key_text("C".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(pos_x * 3.0 - pos_x_diff, pos_y))
                .with_frame_color(UColor32::BLUE)
                .with_pressed_color(UColor32::BLUE.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_counter(key_counter);
            vec![key_1, key_2, key_3]
        }

        fn property_mouse() -> Vec<KeyProperty> {
            Self::property_zxc()
                .into_iter()
                .enumerate()
                .map(|(i, key_property)| {
                    let bind_text = [
                        (Key::MouseLeft, "左"),
                        (Key::MouseMiddle, "中"),
                        (Key::MouseRight, "右"),
                    ];
                    key_property
                        .with_key_bind(bind_text[i].0)
                        .with_key_text(bind_text[i].1.into())
                })
                .collect()
        }

        fn property_four_directions() -> Vec<KeyProperty> {
            let window_width = WindowSetting::DEFAULT_WIDTH;
            let width = 100.0;
            let height = 100.0;
            let thickness = 3.0;
            let font_size = 30.0;
            let pos_diff = 150.0;
            let key_counter = Some(
                KeyCounterProperty::default()
                    .with_position(Pos2::new(0.0, height))
                    .with_font_size(font_size),
            );
            let fade_effect = Some(50.0);
            let key_left = KeyProperty::default()
                .with_key_bind(Key::Left)
                .with_key_text("←".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(pos_diff, pos_diff))
                .with_frame_color(UColor32::RED)
                .with_pressed_color(UColor32::RED.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_direction(KeyDirection::Left)
                .with_key_counter(key_counter.clone());
            let key_up = KeyProperty::default()
                .with_key_bind(Key::Up)
                .with_key_text("↑".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(window_width - pos_diff - width, pos_diff))
                .with_frame_color(UColor32::YELLOW)
                .with_pressed_color(UColor32::YELLOW.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_direction(KeyDirection::Up)
                .with_key_counter(key_counter.clone());
            let key_right = KeyProperty::default()
                .with_key_bind(Key::Right)
                .with_key_text("→".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(
                    window_width - pos_diff - width,
                    window_width - pos_diff - width,
                ))
                .with_frame_color(UColor32::BLUE)
                .with_pressed_color(UColor32::BLUE.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_direction(KeyDirection::Right)
                .with_key_counter(key_counter.clone());
            let key_down = KeyProperty::default()
                .with_key_bind(Key::Down)
                .with_key_text("↓".into())
                .with_width(width)
                .with_height(height)
                .with_thickness(thickness)
                .with_font_size(font_size)
                .with_position(Pos2::new(pos_diff, window_width - pos_diff - width))
                .with_frame_color(UColor32::GREEN)
                .with_pressed_color(UColor32::GREEN.with_a(128))
                .with_fade_length(fade_effect)
                .with_key_direction(KeyDirection::Down)
                .with_key_counter(key_counter.clone());
            vec![key_left, key_up, key_right, key_down]
        }
    }
}

mod v1 {
    use crate::key_overlay_core::key_property::KeyProperty;

    use super::*;

    #[derive(Serialize, Deserialize)]
    pub struct BackgroundColor {
        pub r: bool,
        pub g: bool,
        pub b: bool,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Setting {
        pub window_setting: WindowSetting,
        pub font_name: Box<str>,
        pub background_color: BackgroundColor,
        pub key_properties: Vec<KeyProperty>,
    }
}
