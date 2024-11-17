use std::rc::Rc;

use eframe::{
    egui_wgpu::WgpuConfiguration,
    wgpu::{Backends, PowerPreference, PresentMode},
};
use egui::ViewportBuilder;
use winit::platform::windows::EventLoopBuilderExtWindows;

use crate::{
    key_message::KeyMessage,
    key_overlay::KeyOverlay,
    msg_hook::{self, create_msg_hook},
    setting::{Setting, WindowSetting},
};

use crossbeam::channel::Receiver as MpscReceiver;

pub struct MainApp {
    setting: Option<Setting>,
}

impl MainApp {
    pub fn new() -> Self {
        Self {
            setting: Some(Setting::load_from_local_setting()),
        }
    }

    pub fn run(mut self) {
        let setting = self.setting.take().unwrap();
        let WindowSetting {
            width,
            height,
            enable_vsync,
        } = setting.window_setting;
        // large enough to avoid jam
        const CAP: usize = u16::MAX as usize + 1;
        let (keys_sender, keys_receiver) = crossbeam::channel::bounded(CAP);
        let hook_shared = msg_hook::HookShared::new();
        let hook_shared_1 = hook_shared.clone();
        let icon_data = {
            let img = image::load_from_memory(include_bytes!("../../icons/main_icon.png")).unwrap();
            let width = img.width();
            let height = img.height();
            let data = img.into_bytes();
            egui::IconData {
                rgba: data,
                width,
                height,
            }
        };
        let native_options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size([width, height])
                .with_resizable(false)
                .with_maximize_button(false)
                .with_minimize_button(false)
                .with_icon(icon_data),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: WgpuConfiguration {
                supported_backends: Backends::VULKAN,
                present_mode: if enable_vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            },
            event_loop_builder: Some(Box::new(|event_loop_builder| {
                event_loop_builder
                    .with_msg_hook(create_msg_hook::<true>(keys_sender, hook_shared_1));
            })),
            ..Default::default()
        };
        eframe::run_native(
            "HP KeyOverlay",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc, keys_receiver, setting, hook_shared)))),
        )
        .unwrap();
    }
}

struct App {
    key_overlay: KeyOverlay,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        keys_receiver: MpscReceiver<KeyMessage>,
        setting: Setting,
        hook_shared: Rc<msg_hook::HookShared>,
    ) -> Self {
        cc.egui_ctx.request_repaint();
        let key_overlay = KeyOverlay::new(&cc.egui_ctx, setting, keys_receiver);
        hook_shared.egui_ctx.set(cc.egui_ctx.clone()).unwrap();
        Self { key_overlay }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let instant_now = std::time::Instant::now();
        self.key_overlay.update(instant_now);
        egui::CentralPanel::default().show(ctx, |ui| self.key_overlay.show(ui));
        self.key_overlay
            .need_repaint()
            .then(|| ctx.request_repaint());
    }
}
