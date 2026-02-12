use std::{sync::Arc, time::Instant};

use crate::{
    app_main_vk::{
        key_bar::KeyBarShader, numbers::NumbersShader, press_rect::PressRectShader, shaders,
        static_overlay::StaticOverlayShader,
    },
    key_overlay_core::{
        KeyOverlayCore, key_handler::KeyHandler, key_message::KeyMessage, key_property::KeyProperty,
    },
    setting::Setting,
};

use egui::Color32;
use sak_rs::{
    font::{Font, FontFallbackList, SystemFontsLoader},
    graphics::vulkan::{
        context::Allocators,
        renderer::{Renderer, command_builder::CommandBuilder},
    },
    message_dialog,
    sync::mpmc::queue::BoundedReceiver as MpscReceiver,
};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    device::Queue,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    render_pass::RenderPass,
};

pub struct KeyOverlay {
    core: KeyOverlayCore,
    instant_now: Instant,
    shaders: Shaders,
}

impl KeyOverlay {
    pub fn new(
        renderer: &Renderer,
        setting: Setting,
        keys_receiver: MpscReceiver<KeyMessage>,
    ) -> Self {
        let shaders = Shaders::new(renderer, &setting);
        let core = KeyOverlayCore::new(setting, keys_receiver);

        Self {
            core,
            instant_now: Instant::now(),
            shaders,
        }
    }

    pub fn update(&mut self, instant_now: Instant) {
        self.instant_now = instant_now;
        self.core.update(instant_now);
    }

    #[inline]
    pub fn add_commands(
        &mut self,
        instant_now: Instant,
    ) -> impl FnOnce(&mut CommandBuilder) + use<> {
        self.shaders
            .add_commands(instant_now, self.core.key_handler())
    }

    pub fn need_redraw(&self) -> bool {
        self.core.need_repaint()
    }
}

struct Shaders {
    key_bar: KeyBarShader,
    press_rect: PressRectShader,
    static_overlay: StaticOverlayShader,
    numbers: NumbersShader,
}

#[derive(Clone)]
pub(super) struct ShaderInitResources<'a> {
    pub queue: &'a Arc<Queue>,
    pub render_pass: &'a Arc<RenderPass>,
    pub allocators: &'a Arc<Allocators>,
    pub screen_size: [f32; 2],
    pub key_properties: &'a [KeyProperty],
    pub fonts: &'a Arc<FontFallbackList>,
    pub uniform_buffer: &'a Subbuffer<shaders::ScreenSize>,
    pub properties_buffer: &'a Subbuffer<[shaders::Property]>,
}

impl Shaders {
    fn new(renderer: &Renderer, setting: &Setting) -> Self {
        let Setting {
            window_setting,
            font_name,
            key_properties,
            ..
        } = setting;

        let fonts_loader = SystemFontsLoader::new();
        let font_data = [&**font_name]
            .into_iter()
            .chain(crate::DEFAULT_FONT_NAMES)
            .filter_map(|name| {
                fonts_loader
                    .load_by_family_name(name)
                    .map(|data| Font::try_from_vec(data).expect("unreachable"))
                    .map_err(|e| {
                        message_dialog::warning(format!("Failed to load font: {e:?}")).show()
                    })
                    .ok()
            })
            .collect();
        let fonts = Arc::new(FontFallbackList::new(font_data));
        let screen_size = [window_setting.width, window_setting.height];
        let uniform_buffer = Self::create_uniform_buffer(renderer.allocators(), screen_size);
        let properties_buffer =
            Self::create_properties_buffer(renderer.allocators(), key_properties);
        let (key_bar, press_rect, static_overlay, numbers) = std::thread::scope(|s| {
            let resources = ShaderInitResources {
                queue: renderer.queue(),
                render_pass: renderer.render_pass(),
                allocators: renderer.allocators(),
                screen_size,
                key_properties,
                fonts: &fonts,
                uniform_buffer: &uniform_buffer,
                properties_buffer: &properties_buffer,
            };
            let static_overlay_create_thread = {
                let resources_1 = resources.clone();
                s.spawn(move || StaticOverlayShader::new(&resources_1))
            };
            let key_bar = KeyBarShader::new(&resources);
            let press_rect = PressRectShader::new(&resources);
            let numbers = NumbersShader::new(&resources);
            let static_overlay = static_overlay_create_thread.join().expect("unreachable");
            (key_bar, press_rect, static_overlay, numbers)
        });

        Self {
            key_bar,
            press_rect,
            static_overlay,
            numbers,
        }
    }
}

impl Shaders {
    fn create_uniform_buffer(
        allocators: &Allocators,
        screen_size: [f32; 2],
    ) -> Subbuffer<shaders::ScreenSize> {
        Buffer::from_data(
            allocators.memory().clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            shaders::ScreenSize {
                width: screen_size[0],
                height: screen_size[1],
            },
        )
        .expect("unreachable")
    }

    fn create_properties_buffer(
        allocators: &Allocators,
        key_properties: &[KeyProperty],
    ) -> Subbuffer<[shaders::Property]> {
        let properties: Vec<_> = key_properties
            .iter()
            .map(|key_property| shaders::Property {
                pressed_color: Color32::from(key_property.pressed_color).to_normalized_gamma_f32(),
                frame_color: Color32::from(key_property.frame_color).to_normalized_gamma_f32(),
                text_color: Color32::from(key_property.text_color).to_normalized_gamma_f32(),
                key_position: key_property.position.into(),
                width: key_property.width,
                height: key_property.height,
                thickness: key_property.thickness,
                bar_speed: key_property.bar_speed,
                has_max_distance: key_property.max_distance.0 as u32,
                max_distance: key_property.max_distance.1,
                has_fade: key_property.fade_length.0 as u32,
                fade_length: key_property.fade_length.1,
                direction: shaders::Direction {
                    v: key_property.key_direction as u32,
                },
                font_size: key_property.font_size,
                counter_text_color: Color32::from(key_property.key_counter.1.text_color)
                    .to_normalized_gamma_f32(),
                counter_font_size: key_property.key_counter.1.font_size,
                _padding: Default::default(),
            })
            .chain([unsafe { core::mem::zeroed() }])
            .collect();
        Buffer::from_iter(
            allocators.memory().clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            properties,
        )
        .expect("unreachable")
    }

    fn add_commands(
        &mut self,
        instant_now: Instant,
        key_handler: &KeyHandler,
    ) -> impl FnOnce(&mut CommandBuilder) + use<> {
        let key_bar = self.key_bar.add_commands(instant_now, key_handler);
        let press_rect = self.press_rect.add_commands(key_handler);
        let static_overlay = self.static_overlay.add_commands();
        let numbers = self.numbers.add_commands(key_handler);
        move |c| {
            key_bar.map(|f| f(c));
            press_rect.map(|f| f(c));
            static_overlay(c);
            numbers.map(|f| f(c));
        }
    }
}
