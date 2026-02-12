#![deny(unsafe_op_in_unsafe_fn)]

pub mod app_kps_dashboard;
pub mod app_main;
pub mod app_main_vk;
pub mod app_setting;

mod key;
mod key_overlay_core;
mod msg_hook;
mod setting;
mod ucolor32;
mod utils;

pub use sak_rs::graceful_run;

use std::sync::Arc;

use eframe::wgpu::{MemoryHints, Trace, wgt::DeviceDescriptor};
use sak_rs::message_dialog;
use setting::Setting;

/// oh, blazing fast!
#[global_allocator]
static _GLOBAL_ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// large enough to avoid jam
const CHANNEL_CAP: usize = u16::MAX as usize + 1;

const SETTING_FILE_NAME: &str = "setting.json";

const DEFAULT_FONT_NAMES: [&str; 3] = [
    Setting::DEFAULT_FONT_NAME,
    "Segoe UI emoji",
    "Segoe UI Symbol",
];

const SDF_SIZE: u32 = 64;
const SDF_RADIUS: f32 = SDF_SIZE as f32 / 4.0;
const SDF_CUTOFF: f32 = 0.1;
const SDF_PADDING: u32 = (SDF_RADIUS * (1.0 - SDF_CUTOFF)).ceil() as u32;

#[inline]
fn sdf_edge_padding(pt: f32) -> f32 {
    const PADDING_PER_PT: f32 = SDF_PADDING as f32 / SDF_SIZE as f32;
    pt * PADDING_PER_PT
}

fn common_eframe_native_options(vsync: bool) -> eframe::NativeOptions {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu::{
        Backends, ExperimentalFeatures, Features, InstanceDescriptor, PowerPreference, PresentMode,
    };
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
                        memory_hints: MemoryHints::MemoryUsage,
                        trace: Trace::Off,
                        experimental_features: ExperimentalFeatures::disabled(),
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
