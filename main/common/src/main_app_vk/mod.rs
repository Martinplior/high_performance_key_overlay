mod key_bar;
mod key_overlay;
mod numbers;
mod press_rect;
mod shaders;
mod static_overlay;

use std::{
    num::NonZero,
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
    time::Instant,
};

use eframe::WindowAttributes;
use egui::Color32;
use sak_rs::{
    graphics::renderer::vulkan::{Renderer, RendererCreateInfo},
    os::windows::input::GlobalListener,
    sync::mpmc,
};
use vulkano::{
    device::{DeviceExtensions, DeviceFeatures},
    format::Format,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop},
    window::{Icon, Window, WindowButtons},
};

use crate::{
    main_app_vk::key_overlay::KeyOverlay,
    msg_hook,
    setting::{Setting, WindowSetting},
};

pub struct MainAppVk;

impl MainAppVk {
    pub fn run() {
        let setting = Setting::load_from_local_setting();
        let WindowSetting {
            width,
            height,
            enable_vsync,
        } = setting.window_setting;

        let icon = {
            let img = image::load_from_memory(include_bytes!("../../../icons/main_icon.png"))
                .expect("unreachable");
            let width = img.width();
            let height = img.height();
            let data = img.into_bytes();
            Icon::from_rgba(data, width, height).expect("unreachable")
        };

        let event_loop = EventLoop::new().expect("unreachable");
        event_loop.listen_device_events(DeviceEvents::Never);
        event_loop.set_control_flow(ControlFlow::Wait);

        let window_attributes = WindowAttributes::default()
            .with_inner_size(PhysicalSize::new(width, height))
            .with_window_icon(Some(icon));
        let mut app = App::new(AppCreateInfo {
            setting,
            window_attributes,
            vsync: enable_vsync,
        });

        event_loop.run_app(&mut app).expect("unreachable");
    }
}

struct AppCreateInfo {
    setting: Setting,
    window_attributes: WindowAttributes,
    vsync: bool,
}

struct App {
    inner: Option<Inner>,
    create_info: Option<AppCreateInfo>,
}

struct Inner {
    window: Arc<Window>,
    redraw_requested: Arc<AtomicBool>,
    renderer: Renderer,
    key_overlay: KeyOverlay,
    _global_listener: GlobalListener,
}

impl App {
    fn new(create_info: AppCreateInfo) -> Self {
        Self {
            inner: None,
            create_info: Some(create_info),
        }
    }

    fn create_window(attributes: WindowAttributes, event_loop: &ActiveEventLoop) -> Arc<Window> {
        let attributes = attributes
            .with_title("HP KeyOverlay")
            .with_resizable(false)
            .with_min_inner_size(PhysicalSize::new(1, 1))
            .with_enabled_buttons(WindowButtons::CLOSE)
            .with_transparent(true)
            .with_visible(false);
        let window = event_loop.create_window(attributes).expect("unreachable");
        Arc::new(window)
    }
}

impl Inner {
    fn update(&mut self) {
        let instant_now = Instant::now();
        self.key_overlay.update(instant_now);
        self.key_overlay
            .need_redraw()
            .then(|| self.request_redraw());
        self.renderer
            .render(self.key_overlay.add_commands(instant_now));
    }

    fn request_redraw(&self) {
        let redraw_requested = self.redraw_requested.swap(true, atomic::Ordering::Relaxed);
        (!redraw_requested).then(|| self.window.request_redraw());
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.inner.is_some() {
            return;
        }
        let AppCreateInfo {
            setting,
            window_attributes,
            vsync,
        } = self.create_info.take().expect("unreachable");
        let window = Self::create_window(window_attributes, event_loop);
        let window_1 = window.clone();
        let mut renderer = Renderer::new(RendererCreateInfo {
            window: window.clone(),
            window_inner_size: move || window_1.inner_size().into(),
            desire_image_format: Some(Format::B8G8R8A8_UNORM),
            desire_image_count: NonZero::new(2),
            device_extensions: DeviceExtensions {
                ..Default::default()
            },
            device_features: DeviceFeatures {
                runtime_descriptor_array: true,
                descriptor_binding_partially_bound: true,
                ..Default::default()
            },
        });
        renderer.set_vsync(vsync);
        renderer.clear_color = Color32::from(setting.background_color).to_normalized_gamma_f32();

        let cap = crate::CHANNEL_CAP;
        let (keys_sender, keys_receiver) = mpmc::queue::bounded(cap);
        let window_1 = window.clone();
        let redraw_requested = Arc::new(AtomicBool::new(false));
        let redraw_requested_1 = redraw_requested.clone();
        let hook_shared = msg_hook::HookShared {
            request_redraw: Box::new(move || {
                let redraw_requested = redraw_requested_1.swap(true, atomic::Ordering::Relaxed);
                (!redraw_requested).then(|| window_1.request_redraw());
            }),
        };
        let _global_listener = GlobalListener::new(
            msg_hook::create_msg_hook(keys_sender, hook_shared),
            msg_hook::create_register_raw_input_hook(false),
        );
        let key_overlay = KeyOverlay::new(&renderer, setting, keys_receiver);
        window.set_visible(true);
        let inner = Inner {
            window,
            redraw_requested,
            renderer,
            key_overlay,
            _global_listener,
        };
        self.inner = Some(inner);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let inner = unsafe { self.inner.as_mut().unwrap_unchecked() };
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                inner
                    .redraw_requested
                    .store(false, atomic::Ordering::Relaxed);
                // inner.window.pre_present_notify();
                inner.update();
            }
            _ => (),
        }
    }
}
