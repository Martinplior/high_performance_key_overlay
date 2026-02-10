use std::sync::Arc;

use sak_rs::{
    font::{
        FontFallbackList, LineLayout, SdfGenerator,
        layout::{CachedLineLayout, CachedLineLayoutLibrary, LineLayoutMetrics},
    },
    graphics::vulkan::{
        context::Allocators,
        renderer::{PREMUL_ALPHA, command_builder::CommandBuilder},
    },
};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryCommandBufferAbstract,
    },
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::Format,
    image::{
        Image, ImageCreateInfo, ImageUsage,
        sampler::{BorderColor, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
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
        layout::PipelineDescriptorSetLayoutCreateInfo,
    },
    render_pass::{RenderPass, Subpass},
    sync::GpuFuture,
};

use crate::key_overlay_core::key_handler::KeyHandler;

use super::shaders;

struct NumbersLayoutLibrary {
    map: ahash::HashMap<char, CachedLineLayout>,
}

impl NumbersLayoutLibrary {
    fn new(library: &FontFallbackList) -> Arc<Self> {
        let map = "0123456789"
            .chars()
            .filter_map(move |ch| {
                let font = library.font(ch)?;
                let scale_factor = font.px_scale_factor();
                let metrics = LineLayoutMetrics {
                    ascent: font.ascent_unscaled(),
                    descent: font.descent_unscaled(),
                    h_advance: font.h_advance_unscaled(ch),
                    h_side_bearing: font.h_side_bearing_unscaled(ch),
                };
                let bounds = font.outline(ch).map(|o| o.bounds).unwrap_or_default();
                let cached_layout = CachedLineLayout {
                    scale_factor,
                    metrics_unscaled: metrics,
                    outline_bounds_unscaled: bounds,
                };
                Some((ch, cached_layout))
            })
            .collect();
        Arc::new(Self { map })
    }
}

impl CachedLineLayoutLibrary for NumbersLayoutLibrary {
    fn get_cache(&self, ch: char) -> Option<&CachedLineLayout> {
        self.map.get(&ch)
    }
}

#[repr(C)]
#[derive(Debug, Clone, BufferContents, Vertex)]
struct VertexInput {
    #[format(R32_UINT)]
    in_property_index: u32,
    #[format(R32_UINT)]
    in_char_index: u32,
    #[format(R32G32_SFLOAT)]
    in_position: [f32; 2],
    #[format(R32G32_SFLOAT)]
    in_size: [f32; 2],
}

#[derive(Clone)]
struct Shared {
    allocators: Arc<Allocators>,
    pipeline: Arc<GraphicsPipeline>,

    descriptor_set: Arc<DescriptorSet>,
    numbers_descriptor_set: Arc<DescriptorSet>,
}

pub struct NumbersShader {
    shared: Shared,

    numbers_layout_library: Arc<NumbersLayoutLibrary>,
    screen_size: [f32; 2],

    vertex_input_buf: Vec<VertexInput>,
}

impl NumbersShader {
    const DEFAULT_BUF_CAP: usize = 64;

    pub fn new(
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        allocators: Arc<Allocators>,
        screen_size: [f32; 2],
        fonts: Arc<FontFallbackList>,
        uniform_buffer: Subbuffer<shaders::ScreenSize>,
        properties_buffer: Subbuffer<[shaders::Property]>,
    ) -> Self {
        let device = queue.device();
        let pipeline = Self::create_pipeline(device.clone(), render_pass, screen_size);
        let descriptor_set = Self::create_descriptor_set(
            &allocators,
            uniform_buffer,
            properties_buffer,
            pipeline.layout(),
        );
        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                address_mode: [SamplerAddressMode::ClampToBorder; 3],
                border_color: BorderColor::FloatTransparentBlack,
                ..SamplerCreateInfo::simple_repeat_linear()
            },
        )
        .expect("unreachable");
        let mut sdf_generator =
            SdfGenerator::new(crate::SDF_PADDING, crate::SDF_RADIUS, crate::SDF_CUTOFF);
        let numbers_descriptor_set = Self::create_numbers_descriptor_set(
            &allocators,
            &queue,
            pipeline.layout(),
            &sampler,
            &fonts,
            &mut sdf_generator,
        );

        let shared = Shared {
            allocators,
            pipeline,
            descriptor_set,
            numbers_descriptor_set,
        };

        let numbers_layout_library = NumbersLayoutLibrary::new(&fonts);

        Self {
            shared,
            numbers_layout_library,
            screen_size,
            vertex_input_buf: Vec::with_capacity(Self::DEFAULT_BUF_CAP),
        }
    }

    pub fn add_commands(
        &mut self,
        key_handler: &KeyHandler,
    ) -> Option<impl FnOnce(&mut CommandBuilder) + use<>> {
        let screen_size = self.screen_size;

        for (index, (property, cache)) in key_handler
            .key_properties()
            .iter()
            .zip(key_handler.key_draw_caches())
            .enumerate()
            .filter(|(_, (property, _))| property.key_counter.0)
        {
            let counter = &property.key_counter.1;
            let font_size = counter.font_size;
            let mut layout = LineLayout::new(font_size);
            layout.append(&*self.numbers_layout_library, cache.count.to_string());

            let frame_x_center = property.position.x + property.width / 2.0;
            let frame_y_center = property.position.y + property.height / 2.0;
            let [x_center, y_center] = layout.center();
            let dx = frame_x_center + counter.position.x - x_center;
            let dy = frame_y_center + counter.position.y - y_center;

            for char_layout in layout.into_layout().into_iter() {
                let Some(glyph_metrics) = self
                    .numbers_layout_library
                    .glyph_metrics(char_layout.ch, font_size)
                else {
                    continue;
                };
                let edge_padding = crate::sdf_edge_padding(font_size);
                let vertex = VertexInput {
                    in_property_index: index as u32,
                    in_char_index: char_layout.ch as u32 - b'0' as u32,
                    in_position: [
                        char_layout.x as f32 + dx - edge_padding,
                        glyph_metrics.y_offset as f32 + dy - edge_padding,
                    ],
                    in_size: [
                        glyph_metrics.width as f32 + 2.0 * edge_padding,
                        glyph_metrics.height as f32 + 2.0 * edge_padding,
                    ],
                };
                let invisible = glyph_metrics.width == 0
                    || glyph_metrics.height == 0
                    || vertex.in_position[0] > screen_size[0]
                    || vertex.in_position[1] > screen_size[1]
                    || vertex.in_position[0] + vertex.in_size[0] < 0.0
                    || vertex.in_position[1] + vertex.in_size[1] < 0.0;
                if invisible {
                    continue;
                }
                self.vertex_input_buf.push(vertex);
            }
        }
        let vertex_buffer = Buffer::from_iter(
            self.shared.allocators.memory().clone(),
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

        Some(move |b: &mut CommandBuilder| Self::add_commands_main(shared, vertex_buffer, b))
    }
}

impl NumbersShader {
    fn create_pipeline(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        screen_size: [f32; 2],
    ) -> Arc<GraphicsPipeline> {
        let vertex_shader = shaders::numbers::vs::load(device.clone()).expect("unreachable");
        let fragment_shader = shaders::numbers::fs::load(device.clone()).expect("unreachable");
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
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .expect("unreachable"),
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

    fn create_numbers_descriptor_set(
        allocators: &Allocators,
        queue: &Arc<Queue>,
        pipeline_layout: &PipelineLayout,
        sampler: &Arc<Sampler>,
        fonts: &FontFallbackList,
        sdf_generator: &mut SdfGenerator,
    ) -> Arc<DescriptorSet> {
        let mut command_builder = AutoCommandBufferBuilder::primary(
            allocators.command_buffer().clone(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("unreachable");

        let image_view_vec: Vec<_> = "0123456789"
            .chars()
            .map(|ch| {
                let glyph = fonts
                    .font(ch)
                    .and_then(|font| font.rasterize(ch, crate::SDF_SIZE as f32))
                    .expect("unreachable");
                let sdf = sdf_generator.generate(&glyph.bitmap, glyph.metrics.width);
                let buffer = Buffer::from_iter(
                    allocators.memory().clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::TRANSFER_SRC,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    sdf.bitmap,
                )
                .expect("Failed to create buffer");
                let image = Image::new(
                    allocators.memory().clone(),
                    ImageCreateInfo {
                        extent: [sdf.width, sdf.height, 1],
                        format: Format::R8_UNORM,
                        usage: ImageUsage::SAMPLED
                            | ImageUsage::TRANSFER_DST
                            | ImageUsage::TRANSFER_SRC,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                        ..Default::default()
                    },
                )
                .expect("Failed to create sdf texture");
                command_builder
                    .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                        buffer,
                        image.clone(),
                    ))
                    .expect("unreachable");
                ImageView::new_default(image).expect("Failed to create image view")
            })
            .collect();

        command_builder
            .build()
            .expect("unreachable")
            .execute(queue.clone())
            .expect("unreachable")
            .then_signal_fence_and_flush()
            .expect("unreachable")
            .wait(None)
            .expect("unreachable");

        let descriptor_set_layout = pipeline_layout.set_layouts().get(1).expect("unreachable");
        DescriptorSet::new(
            allocators.descriptor_set().clone(),
            descriptor_set_layout.clone(),
            [WriteDescriptorSet::image_view_sampler_array(
                0,
                0,
                image_view_vec.into_iter().map(|iv| (iv, sampler.clone())),
            )],
            [],
        )
        .expect("unreachable")
    }
}

impl NumbersShader {
    #[inline]
    fn add_commands_main(
        shared: Shared,
        vertex_buffer: Subbuffer<[VertexInput]>,
        command_builder: &mut CommandBuilder,
    ) {
        let Shared {
            pipeline,
            descriptor_set,
            numbers_descriptor_set,
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
                (descriptor_set, numbers_descriptor_set),
            )
            .expect("unreachable");
        unsafe { command_builder.builder.draw(4, instantce_count, 0, 0) }.expect("unreachable");
    }
}
