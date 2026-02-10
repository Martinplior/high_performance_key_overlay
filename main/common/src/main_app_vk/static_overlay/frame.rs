use std::sync::Arc;

use sak_rs::graphics::vulkan::{context::Allocators, renderer::PREMUL_ALPHA};
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::Device,
    pipeline::{
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
        graphics::{
            GraphicsPipelineCreateInfo,
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            viewport::{Viewport, ViewportState},
        },
        layout::{PipelineDescriptorSetLayoutCreateInfo, PipelineLayoutCreateInfo},
    },
    render_pass::{RenderPass, Subpass},
};

use super::shaders;

#[derive(Clone)]
struct Shared {
    pipeline: Arc<GraphicsPipeline>,
    properties_count: u32,

    descriptor_set: Arc<DescriptorSet>,
}

pub struct FrameShader {
    shared: Shared,
}

impl FrameShader {
    pub fn new(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        allocators: &Allocators,
        screen_size: [f32; 2],
        uniform_buffer: Subbuffer<shaders::ScreenSize>,
        properties_buffer: Subbuffer<[shaders::Property]>,
    ) -> Self {
        let pipeline = Self::create_pipeline(device, render_pass, screen_size);
        let properties_count = properties_buffer.len() as u32 - 1;
        let descriptor_set = Self::create_descriptor_set(
            allocators,
            uniform_buffer,
            properties_buffer,
            pipeline.layout(),
        );

        let shared = Shared {
            pipeline,
            properties_count,
            descriptor_set,
        };
        Self { shared }
    }

    pub fn add_commands(
        &self,
    ) -> Option<impl FnOnce(&mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) + use<>> {
        if self.shared.properties_count == 0 {
            return None;
        }
        let shared = self.shared.clone();

        let f = move |b: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>| {
            let Shared {
                pipeline,
                properties_count,
                descriptor_set,
            } = shared;
            b.bind_pipeline_graphics(pipeline.clone())
                .expect("unreachable")
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .expect("unreachable");
            unsafe { b.draw(10, properties_count, 0, 0) }.expect("unreachable");
        };
        Some(f)
    }
}

impl FrameShader {
    fn create_pipeline(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        screen_size: [f32; 2],
    ) -> Arc<GraphicsPipeline> {
        let vertex_shader = shaders::frame::vs::load(device.clone()).expect("unreachable");
        let fragment_shader = shaders::frame::fs::load(device.clone()).expect("unreachable");
        let vertex_shader_entry_point = vertex_shader.entry_point("main").expect("unreachable");
        let fragment_shader_entry_point = fragment_shader.entry_point("main").expect("unreachable");
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
        let subpass = Subpass::from(render_pass, 0).expect("unreachable");
        let viewport = Viewport {
            extent: screen_size,
            ..Default::default()
        };
        GraphicsPipeline::new(
            device,
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(Default::default()),
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
        .expect("unreachable")
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
                WriteDescriptorSet::buffer(0, uniform_buffer),
                WriteDescriptorSet::buffer(1, properties_buffer),
            ],
            [],
        )
        .expect("unreachable")
    }
}
