#![deny(unsafe_op_in_unsafe_fn)]

pub mod kps_dashboard_app;
pub mod main_app;
pub mod main_app_vk;
pub mod setting_app;

mod key;
mod key_overlay_core;
mod msg_hook;
mod setting;
mod ucolor32;
mod utils;

use std::sync::Arc;

use eframe::wgpu::{MemoryHints, Trace, wgt::DeviceDescriptor};
use sak_rs::message_dialog;

/// oh, blazing fast!
#[cfg(not(feature = "save_memory"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// large enough to avoid jam
const CHANNEL_CAP: usize = u16::MAX as usize + 1;

const SETTING_FILE_NAME: &str = "setting.json";

const DEFAULT_FONT_NAMES: [&str; 3] = [
    Setting::DEFAULT_FONT_NAME,
    "Segoe UI emoji",
    "Segoe UI Symbol",
];

fn common_eframe_native_options(vsync: bool) -> eframe::NativeOptions {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu::{Backends, InstanceDescriptor, PowerPreference, PresentMode};
    eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: WgpuConfiguration {
            wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
                instance_descriptor: InstanceDescriptor {
                    backends: Backends::VULKAN | Backends::GL,
                    ..Default::default()
                },
                power_preference: PowerPreference::HighPerformance,
                device_descriptor: Arc::new(|adapter| {
                    let r = DeviceDescriptor {
                        label: None,
                        required_features: Features::empty(),
                        required_limits: adapter.limits(),
                        memory_hints: if cfg!(feature = "save_memory") {
                            MemoryHints::MemoryUsage
                        } else {
                            MemoryHints::Performance
                        },
                        trace: Trace::Off,
                    };
                    #[cfg(debug_assertions)]
                    println!("{r:?}");
                    r
                }),
                ..Default::default()
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
        message_dialog::error(format!("未知错误: {err}")).show();
        panic!()
    })
}

fn key_overlay_setting_path() -> std::path::PathBuf {
    get_current_dir().join(SETTING_FILE_NAME)
}

pub use sak_rs::graceful_run;
use wgpu::Features;

use crate::setting::Setting;

#[cfg(test)]
mod tests {

    use egui::{Color32, FontDefinitions};

    use crate::{setting::Setting, ucolor32::UColor32};

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
