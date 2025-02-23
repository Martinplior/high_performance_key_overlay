#![deny(unsafe_op_in_unsafe_fn)]

use std::sync::Arc;

pub mod kps_dashboard_app;
pub mod main_app;
pub mod setting_app;

mod global_listener;
mod key;
mod key_bar;
mod key_draw_cache;
mod key_handler;
mod key_message;
mod key_overlay;
mod key_property;
mod key_shader;
mod message_dialog;
mod msg_hook;
mod setting;
mod ucolor32;
mod utils;
mod win_utils;

/// oh, blazing fast!
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// large enough to avoid jam
const CHANNEL_CAP: usize = u16::MAX as usize + 1;

const SETTING_FILE_NAME: &str = "setting.json";

fn common_eframe_native_options(vsync: bool) -> eframe::NativeOptions {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu::{Backends, InstanceDescriptor, PowerPreference, PresentMode};
    let WgpuSetup::CreateNew(default_setup) = WgpuConfiguration::default().wgpu_setup else {
        unreachable!();
    };
    eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: WgpuConfiguration {
            wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
                instance_descriptor: InstanceDescriptor {
                    backends: Backends::VULKAN | Backends::GL,
                    ..default_setup.instance_descriptor
                },
                power_preference: PowerPreference::HighPerformance,
                device_descriptor: Arc::new(move |adapter| {
                    let mut r = (default_setup.device_descriptor)(adapter);
                    r.required_limits = eframe::wgpu::Limits::downlevel_defaults()
                        .using_resolution(Default::default());
                    r
                }),
                ..default_setup
            }),
            present_mode: if vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

fn get_current_dir() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|err| {
        message_dialog::error(format!("未知错误：{}", err.to_string())).show();
        panic!()
    })
}

fn key_overlay_setting_path() -> std::path::PathBuf {
    get_current_dir().join(SETTING_FILE_NAME)
}

pub fn graceful_run<R>(
    f: impl FnOnce() -> R + std::panic::UnwindSafe,
) -> Result<R, Box<dyn std::any::Any + Send>> {
    std::panic::catch_unwind(f).map_err(|err| {
        let message = if let Some(err) = err.downcast_ref::<String>() {
            err.clone()
        } else if let Some(err) = err.downcast_ref::<&str>() {
            err.to_string()
        } else {
            format!("{:?}, type_id = {:?}", err, err.type_id())
        };
        #[cfg(debug_assertions)]
        eprintln!("{}", &message);
        message_dialog::error(message).show();
        err
    })
}

#[cfg(test)]
mod tests {

    use egui::{Color32, FontDefinitions};
    use font_kit::{family_name::FamilyName, properties::Properties};

    use crate::{setting::Setting, ucolor32::UColor32};

    #[test]
    fn query_all_families() {
        let sys_fonts = font_kit::source::SystemSource::new();
        let families = sys_fonts.all_families().unwrap();
        families.iter().enumerate().for_each(|(index, family)| {
            println!("{:^3}: {}", index, family);
        });
    }

    #[test]
    fn query_fonts() {
        let sys_fonts = font_kit::source::SystemSource::new();
        let families = sys_fonts.all_families().unwrap();
        families
            .iter()
            .enumerate()
            .take(50)
            .for_each(|(index, family)| {
                let family_handle = sys_fonts.select_family_by_name(family).unwrap();
                let fonts = family_handle.fonts();
                fonts.iter().for_each(|handle| {
                    let font = handle.load().unwrap();
                    let font_index = match handle {
                        font_kit::handle::Handle::Path { font_index, .. } => font_index,
                        _ => unreachable!(),
                    };
                    println!(
                        "{:^3}: {:^30} | {:^50} | font_index: {:^3}",
                        index,
                        family,
                        font.full_name(),
                        font_index
                    );
                });
            });
    }

    #[test]
    fn select_font() {
        let sys_fonts = font_kit::source::SystemSource::new();
        let font_family = "等距更纱黑体 SC";
        let Ok(family_handle) = sys_fonts.select_family_by_name(font_family) else {
            panic!("未找到字体：{}", font_family);
        };
        println!("family count: {}", family_handle.fonts().len());
        family_handle.fonts().iter().for_each(|handle| {
            let font = handle.load().unwrap();
            println!("font name: {}", font.full_name());
        });
    }

    #[test]
    fn select_font_single() {
        let sys_fonts = font_kit::source::SystemSource::new();
        let font_name = "等距更纱黑体 SC";
        let font_handle = sys_fonts
            .select_best_match(
                &[FamilyName::Title(font_name.to_string())],
                &Properties::new(),
            )
            .unwrap();
        let font = font_handle.load().unwrap();
        println!("font name: {}", font.full_name());
    }

    #[test]
    fn serialize() {
        let setting = Setting::default();
        let setting_json = serde_json::to_string_pretty(&setting).unwrap();
        let _setting_1 = serde_json::from_str::<Setting>(&setting_json).unwrap();
        println!("{}", setting_json);
    }

    #[test]
    fn builtin_font_names() {
        println!("{:?}", FontDefinitions::builtin_font_names());
    }

    #[test]
    fn tmp() {
        let ucolor = UColor32::WHITE.with_a(128);
        let color: Color32 = ucolor.into();
        println!("{:?}\n{:?}", ucolor, color);
    }
}
