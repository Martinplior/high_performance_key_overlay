pub mod key_overlay;
mod key_shader;

use egui::ViewportBuilder;
use sak_rs::os::windows::input::GlobalListener;

use crate::{
    main_app::key_overlay::KeyOverlay,
    msg_hook,
    setting::{Setting, WindowSetting},
};

pub struct MainApp;

impl MainApp {
    pub fn run() {
        let setting = Setting::load_from_local_setting();
        let WindowSetting {
            width,
            height,
            enable_vsync,
        } = setting.window_setting;

        let icon_data = {
            let img = image::load_from_memory(include_bytes!("../../../icons/main_icon.png"))
                .expect("unreachable");
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
                .with_icon(icon_data)
                .with_transparent(true),
            ..crate::common_eframe_native_options(enable_vsync)
        };
        eframe::run_native(
            "HP KeyOverlay",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc, setting)))),
        )
        .expect("unreachable");
    }
}

struct App {
    key_overlay: KeyOverlay,
    _global_listener: GlobalListener,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, setting: Setting) -> Self {
        cc.egui_ctx.request_repaint();
        let cap = crate::CHANNEL_CAP;
        let (keys_sender, keys_receiver) = crossbeam_channel::bounded(cap);
        let hook_shared = msg_hook::HookShared {
            egui_ctx: cc.egui_ctx.clone(),
        };
        let global_listener = GlobalListener::new(
            msg_hook::create_msg_hook(keys_sender, hook_shared),
            msg_hook::create_register_raw_input_hook(),
        );
        let key_overlay = KeyOverlay::new(cc, &cc.egui_ctx, setting, keys_receiver);
        Self {
            key_overlay,
            _global_listener: global_listener,
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let instant_now = std::time::Instant::now();
        self.key_overlay.update(instant_now);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| self.key_overlay.show(ui));
        self.key_overlay
            .need_repaint()
            .then(|| ctx.request_repaint());
    }
}
