use std::{num::NonZeroU64, sync::Arc};

use eframe::wgpu::{
    self, include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
    BufferDescriptor, FragmentState, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipelineDescriptor, ShaderStages, VertexState,
};
use egui::Color32;
use parking_lot::{Mutex, MutexGuard};

use crate::key_property::KeyProperty;

#[repr(C)]
#[derive(Default, Clone, Copy, bytemuck::NoUninit)]
pub struct BarRect {
    pub property_index: u32,
    pub begin_duration_secs: f32,
    pub end_duration_secs: f32,
}

#[repr(C)]
#[derive(Clone)]
struct ScreenSize {
    width: f32,
    height: f32,
}

#[repr(C, align(16))]
#[derive(Default, Clone, Copy, bytemuck::NoUninit)]
struct Property {
    pressed_color: [f32; 4],
    key_position: [f32; 2],
    width: f32,
    height: f32,
    direction: u32,
    bar_speed: f32,
    max_distance: f32,
    fade_length: f32,
    has_max_distance: u32,
    has_fade: u32,
    _padding: [u32; 2],
}

#[repr(C)]
#[derive(Clone)]
struct Uniforms {
    screen_size: ScreenSize,
}

pub struct CustomCallbackInner {
    device: Arc<wgpu::Device>,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniforms_buffer: wgpu::Buffer,
    properties_buffer: wgpu::Buffer,
    bar_rects_buffer: wgpu::Buffer,
    pub bar_rects: Vec<BarRect>,
}

#[derive(Clone)]
pub struct CustomCallback {
    pub inner: Arc<Mutex<CustomCallbackInner>>,
}

impl CustomCallback {
    pub fn new(cc: &eframe::CreationContext, key_properties: &[KeyProperty]) -> Self {
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let device = &render_state.device;
        let shader = device.create_shader_module(include_wgsl!("key_shader.wgsl"));
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("custom_paint.bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<Uniforms>() as _),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<Property>() as _),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<BarRect>() as _),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("key_shader.pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("key_shader.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    ..render_state.target_format.into()
                })],
            }),
            multiview: None,
            cache: None,
        });
        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("key_shader.uniforms_buffer"),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            size: std::mem::size_of::<Uniforms>() as _,
            mapped_at_creation: false,
        });
        let properties_contents: Box<_> = key_properties
            .iter()
            .map(|key_property| Property {
                pressed_color: Into::<Color32>::into(key_property.pressed_color)
                    .to_normalized_gamma_f32(),
                key_position: key_property.position.into(),
                width: key_property.width,
                height: key_property.height,
                direction: key_property.key_direction as u32,
                bar_speed: key_property.bar_speed,
                max_distance: key_property.max_distance.1,
                fade_length: key_property.fade_length.1,
                has_max_distance: key_property.max_distance.0 as u32,
                has_fade: key_property.fade_length.0 as u32,
                _padding: Default::default(),
            })
            .chain([Default::default()])
            .collect();
        let properties_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("key_shader.properties_buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&properties_contents),
        });
        let bar_rects_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("key_shader.bar_rects_buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[BarRect::default()]),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("key_shader.bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: properties_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bar_rects_buffer.as_entire_binding(),
                },
            ],
        });
        let inner = Arc::new(Mutex::new(CustomCallbackInner {
            device: device.clone(),
            pipeline,
            bind_group_layout,
            bind_group,
            uniforms_buffer,
            properties_buffer,
            bar_rects_buffer,
            bar_rects: Vec::with_capacity(1024),
        }));
        Self { inner }
    }

    fn recreate_bind_group(inner: &mut MutexGuard<CustomCallbackInner>) {
        let new_bind_group = inner.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("key_shader.bind_group"),
            layout: &inner.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: inner.uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: inner.properties_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inner.bar_rects_buffer.as_entire_binding(),
                },
            ],
        });
        inner.bind_group = new_bind_group;
    }

    pub fn reload(&self, key_properties: &[KeyProperty]) {
        let mut inner = self.inner.lock();
        let properties_contents: Box<_> = key_properties
            .iter()
            .map(|key_property| Property {
                pressed_color: Into::<Color32>::into(key_property.pressed_color)
                    .to_normalized_gamma_f32(),
                key_position: key_property.position.into(),
                width: key_property.width,
                height: key_property.height,
                direction: key_property.key_direction as u32,
                bar_speed: key_property.bar_speed,
                max_distance: key_property.max_distance.1,
                fade_length: key_property.fade_length.1,
                has_max_distance: key_property.max_distance.0 as u32,
                has_fade: key_property.fade_length.0 as u32,
                _padding: Default::default(),
            })
            .chain([Default::default()])
            .collect();
        let new_properties_buffer = inner.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("key_shader.new_properties_buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&properties_contents),
        });
        inner.properties_buffer = new_properties_buffer;
        Self::recreate_bind_group(&mut inner);
    }

    fn update_bar_rects_buffer(
        inner: &mut MutexGuard<CustomCallbackInner>,
        queue: &eframe::wgpu::Queue,
    ) {
        if inner.bar_rects.len() * std::mem::size_of::<BarRect>()
            <= inner.bar_rects_buffer.size() as usize
        {
            queue.write_buffer(
                &inner.bar_rects_buffer,
                0,
                bytemuck::cast_slice(&inner.bar_rects),
            );
            return;
        }
        let new_len = inner.bar_rects.capacity();
        inner.bar_rects.resize(new_len, Default::default());
        let new_bar_rects_buffer = inner.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("key_shader.new_bar_rects_buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: &bytemuck::cast_slice(&inner.bar_rects),
        });
        inner.bar_rects_buffer = new_bar_rects_buffer;
        Self::recreate_bind_group(inner);
    }
}

impl eframe::egui_wgpu::CallbackTrait for CustomCallback {
    fn prepare(
        &self,
        _device: &eframe::wgpu::Device,
        queue: &eframe::wgpu::Queue,
        screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut eframe::wgpu::CommandEncoder,
        _callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<eframe::wgpu::CommandBuffer> {
        let screen_size = screen_descriptor.size_in_pixels.map(|x| x as f32);
        let mut inner = self.inner.lock();
        queue.write_buffer(&inner.uniforms_buffer, 0, bytemuck::bytes_of(&screen_size));
        Self::update_bar_rects_buffer(&mut inner, queue);
        Default::default()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut eframe::wgpu::RenderPass<'static>,
        _callback_resources: &eframe::egui_wgpu::CallbackResources,
    ) {
        let inner = self.inner.lock();
        render_pass.set_pipeline(&inner.pipeline);
        render_pass.set_bind_group(0, &inner.bind_group, &[]);
        render_pass.draw(0..4, 0..inner.bar_rects.len() as u32);
    }
}
