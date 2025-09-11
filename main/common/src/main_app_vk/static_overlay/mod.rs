mod frame;
mod init;
mod text;

use std::sync::Arc;

use sak_rs::{
    font::FontFallbackList,
    graphics::renderer::vulkan::{Allocators, PREMUL_ALPHA, command_builder::CommandBuilder},
};
use vulkano::{
    buffer::Subbuffer,
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    image::view::ImageView,
    pipeline::{
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
        graphics::{
            GraphicsPipelineCreateInfo,
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            viewport::{Viewport, ViewportState},
        },
        layout::{PipelineDescriptorSetLayoutCreateInfo, PipelineLayoutCreateInfo},
    },
    render_pass::{RenderPass, Subpass},
};

use crate::key_overlay_core::key_property::KeyProperty;

use super::shaders;

#[derive(Clone)]
struct Shared {
    pipeline: Arc<GraphicsPipeline>,

    descriptor_set: Arc<DescriptorSet>,
}

pub struct StaticOverlayShader {
    shared: Shared,
}

impl StaticOverlayShader {
    pub fn new(
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        allocators: &Allocators,
        screen_size: [f32; 2],
        key_properties: &[KeyProperty],
        fonts: Arc<FontFallbackList>,
        max_font_size: f32,
        uniform_buffer: Subbuffer<shaders::ScreenSize>,
        properties_buffer: Subbuffer<[shaders::Property]>,
    ) -> Self {
        let device = queue.device().clone();

        let pipeline = Self::create_pipeline(device, render_pass, screen_size);
        let image_view = init::static_overlay_image_view(
            queue,
            allocators,
            screen_size,
            key_properties,
            fonts,
            max_font_size,
            uniform_buffer,
            properties_buffer,
        );
        let descriptor_set =
            Self::create_descriptor_set(&allocators, image_view, pipeline.layout());

        let shared = Shared {
            pipeline,
            descriptor_set,
        };
        Self { shared }
    }

    pub fn add_commands(&mut self) -> impl FnOnce(&mut CommandBuilder) + use<> {
        let shared = self.shared.clone();

        move |c| {
            let Shared {
                pipeline,
                descriptor_set,
            } = shared;
            c.builder
                .bind_pipeline_graphics(pipeline.clone())
                .expect("unreachable")
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .expect("unreachable");
            unsafe { c.builder.draw(3, 1, 0, 0) }.expect("unreachable");
        }
    }
}

impl StaticOverlayShader {
    fn create_pipeline(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        screen_size: [f32; 2],
    ) -> Arc<GraphicsPipeline> {
        let vertex_shader = shaders::static_overlay::vs::load(device.clone()).expect("unreachable");
        let fragment_shader =
            shaders::static_overlay::fs::load(device.clone()).expect("unreachable");
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
                vertex_input_state: Some(Default::default()),
                input_assembly_state: Some(Default::default()),
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
        image_view: Arc<ImageView>,
        pipeline_layout: &PipelineLayout,
    ) -> Arc<DescriptorSet> {
        let descriptor_set_layout = pipeline_layout.set_layouts().get(0).expect("unreachable");
        DescriptorSet::new(
            allocators.descriptor_set().clone(),
            descriptor_set_layout.clone(),
            [WriteDescriptorSet::image_view(0, image_view)],
            [],
        )
        .expect("unreachable")
    }
}
