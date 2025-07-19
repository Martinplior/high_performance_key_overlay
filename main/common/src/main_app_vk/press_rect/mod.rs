use std::sync::Arc;

use sak_rs::graphics::renderer::vulkan::{
    Allocators, PREMUL_ALPHA, Renderer, command_builder::CommandBuilder,
};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::Device,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
        graphics::{
            GraphicsPipelineCreateInfo,
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
        },
        layout::{PipelineDescriptorSetLayoutCreateInfo, PipelineLayoutCreateInfo},
    },
    render_pass::{RenderPass, Subpass},
};

use crate::key_overlay_core::key_handler::KeyHandler;

use super::shaders;

#[repr(C)]
#[derive(Clone, BufferContents, Vertex)]
struct VertexInput {
    #[format(R32_UINT)]
    in_property_index: u32,
}

#[derive(Clone)]
struct Shared {
    memory_allocator: Arc<StandardMemoryAllocator>,
    pipeline: Arc<GraphicsPipeline>,

    descriptor_set: Arc<DescriptorSet>,
}

pub struct PressRectShader {
    shared: Shared,
    vertex_input_buf: Vec<VertexInput>,
}

impl PressRectShader {
    const DEFAULT_BUF_CAP: usize = 64;

    pub fn new(
        renderer: &Renderer,
        screen_size: [f32; 2],
        uniform_buffer: Subbuffer<shaders::ScreenSize>,
        properties_buffer: Subbuffer<[shaders::Property]>,
    ) -> Self {
        let device = renderer.device();
        let render_pass = renderer.render_pass();
        let allocators = renderer.allocators();
        let pipeline = Self::create_pipeline(device.clone(), render_pass.clone(), screen_size);
        let descriptor_set = Self::create_descriptor_set(
            allocators,
            uniform_buffer,
            properties_buffer,
            pipeline.layout(),
        );

        let shared = Shared {
            memory_allocator: allocators.memory().clone(),
            pipeline,
            descriptor_set,
        };
        Self {
            shared,
            vertex_input_buf: Vec::with_capacity(Self::DEFAULT_BUF_CAP),
        }
    }

    pub fn add_commands(
        &mut self,
        key_handler: &KeyHandler,
    ) -> Option<impl FnOnce(&mut CommandBuilder) + use<>> {
        let vertex_input_iter =
            key_handler
                .key_draw_caches()
                .iter()
                .enumerate()
                .filter_map(|(index, cache)| {
                    cache.begin_hold_instant.map(|_| VertexInput {
                        in_property_index: index as u32,
                    })
                });
        self.vertex_input_buf.extend(vertex_input_iter);
        if self.vertex_input_buf.is_empty() {
            return None;
        }
        let vertex_buffer = Buffer::from_iter(
            self.shared.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            self.vertex_input_buf.drain(..),
        )
        .expect("unreachable");

        let shared = self.shared.clone();

        Some(move |c: &mut CommandBuilder| Self::add_commands_main(shared, vertex_buffer, c))
    }
}

impl PressRectShader {
    fn create_pipeline(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        screen_size: [f32; 2],
    ) -> Arc<GraphicsPipeline> {
        let vertex_shader = shaders::press_rect::vs::load(device.clone()).expect("unreachable");
        let fragment_shader = shaders::press_rect::fs::load(device.clone()).expect("unreachable");
        let vertex_shader_entry_point = vertex_shader.entry_point("main").expect("unreachable");
        let fragment_shader_entry_point = fragment_shader.entry_point("main").expect("unreachable");
        let vertex_input_state = VertexInput::per_instance()
            .definition(&vertex_shader_entry_point)
            .expect("unreachable");
        let stages = [
            PipelineShaderStageCreateInfo::new(vertex_shader_entry_point),
            PipelineShaderStageCreateInfo::new(fragment_shader_entry_point),
        ];
        let pipeline_layout = PipelineLayout::new(
            device.clone(),
            PipelineLayoutCreateInfo {
                ..PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())
                    .expect("unreachable")
            },
        )
        .expect("unreachable");
        let subpass = Subpass::from(render_pass, 0).unwrap();
        let viewport = Viewport {
            extent: screen_size,
            ..Default::default()
        };
        let pipeline = GraphicsPipeline::new(
            device,
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState {
                    topology: PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                }),
                viewport_state: Some(ViewportState {
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(Default::default()),
                multisample_state: Some(Default::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState {
                        blend: Some(PREMUL_ALPHA),
                        ..Default::default()
                    },
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(pipeline_layout)
            },
        )
        .expect("unreachable");
        pipeline
    }

    fn create_descriptor_set(
        allocators: &Allocators,
        uniform_buffer: Subbuffer<shaders::ScreenSize>,
        properties_buffer: Subbuffer<[shaders::Property]>,
        pipeline_layout: &PipelineLayout,
    ) -> Arc<DescriptorSet> {
        let descriptor_set_layout = pipeline_layout.set_layouts().get(0).expect("unreachable");
        DescriptorSet::new(
            allocators.descriptor_set().clone(),
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, uniform_buffer.into()),
                WriteDescriptorSet::buffer(1, properties_buffer.into()),
            ],
            [],
        )
        .expect("unreachable")
    }

    #[inline]
    fn add_commands_main(
        shared: Shared,
        vertex_buffer: Subbuffer<[VertexInput]>,
        command_builder: &mut CommandBuilder,
    ) {
        let Shared {
            pipeline,
            descriptor_set,
            ..
        } = shared;
        let instantce_count = vertex_buffer.len() as u32;
        command_builder
            .builder
            .bind_pipeline_graphics(pipeline.clone())
            .expect("unreachable")
            .bind_vertex_buffers(0, vertex_buffer)
            .expect("unreachable")
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .expect("unreachable");
        unsafe { command_builder.builder.draw(4, instantce_count, 0, 0) }.expect("unreachable");
    }
}
