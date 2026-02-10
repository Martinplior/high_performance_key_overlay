use std::sync::Arc;

use ahash::HashMapExt;
use sak_rs::{
    font::{
        FontFallbackList, LineLayout, SdfGenerator,
        layout::{CachedLineLayout, CachedLineLayoutLibrary, LineLayoutMetrics},
    },
    graphics::vulkan::{context::Allocators, renderer::PREMUL_ALPHA},
};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract,
    },
    descriptor_set::{
        DescriptorSet, WriteDescriptorSet,
        layout::{
            DescriptorBindingFlags, DescriptorSetLayout, DescriptorSetLayoutBinding,
            DescriptorSetLayoutCreateInfo,
        },
    },
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

use crate::key_overlay_core::key_property::KeyProperty;

use super::shaders;

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
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,

    sampler: Arc<Sampler>,
    descriptor_set: Arc<DescriptorSet>,
}

struct CharLayoutLibrary {
    map: ahash::HashMap<char, CachedLineLayout>,
}

impl CharLayoutLibrary {
    fn new(library: &FontFallbackList, properties: &[KeyProperty]) -> Arc<Self> {
        let set: ahash::HashSet<_> = properties
            .iter()
            .flat_map(|property| property.key_text.chars())
            .collect();
        let map = set
            .into_iter()
            .filter_map(|ch| {
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

impl CachedLineLayoutLibrary for CharLayoutLibrary {
    fn get_cache(&self, ch: char) -> Option<&CachedLineLayout> {
        self.map.get(&ch)
    }
}

pub struct TextShader {
    shared: Shared,

    char_layout_map: Vec<(char, Vec<VertexInput>)>,
    char_buf: Vec<char>,
    fonts: Arc<FontFallbackList>,

    vertex_input_buf: Vec<VertexInput>,
}

impl TextShader {
    const DEFAULT_BUF_CAP: usize = 64;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        allocators: Arc<Allocators>,
        screen_size: [f32; 2],
        key_properties: &[KeyProperty],
        fonts: Arc<FontFallbackList>,
        batch_size: u32,
        uniform_buffer: Subbuffer<shaders::ScreenSize>,
        properties_buffer: Subbuffer<[shaders::Property]>,
    ) -> Self {
        let device = queue.device();
        let pipeline = Self::create_pipeline(device.clone(), render_pass, screen_size, batch_size);
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

        let shared = Shared {
            allocators,
            queue,
            pipeline,
            sampler,
            descriptor_set,
        };

        let char_layout_library = CharLayoutLibrary::new(&fonts, key_properties);

        let char_layout_map = {
            let mut map = ahash::HashMap::with_capacity(Self::DEFAULT_BUF_CAP);
            for (index, property) in key_properties.iter().enumerate() {
                let font_size = property.font_size;
                let mut layout = LineLayout::new(font_size);
                layout.append(&*char_layout_library, &property.key_text);
                let frame_x_center = property.position.x + property.width / 2.0;
                let frame_y_center = property.position.y + property.height / 2.0;
                let [x_center, y_center] = layout.center();
                let dx = frame_x_center - x_center;
                let dy = frame_y_center - y_center;
                for char_layout in layout.into_layout() {
                    let ch = char_layout.ch;
                    let Some(glyph_metrics) = char_layout_library.glyph_metrics(ch, font_size)
                    else {
                        continue;
                    };
                    let edge_padding = crate::sdf_edge_padding(font_size);
                    let vertex = VertexInput {
                        in_property_index: index as u32,
                        in_char_index: 0,
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
                    map.entry(ch)
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(vertex);
                }
            }
            map.into_iter().collect()
        };

        Self {
            shared,
            char_layout_map,
            fonts,
            char_buf: Vec::new(),
            vertex_input_buf: Vec::with_capacity(Self::DEFAULT_BUF_CAP),
        }
    }

    pub fn add_commands(
        &mut self,
        batch_size: usize,
    ) -> Option<impl FnOnce(&mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) + use<>> {
        if self.char_layout_map.is_empty() {
            return None;
        }
        let start = self
            .char_layout_map
            .len()
            .checked_sub(batch_size)
            .unwrap_or_default();

        self.char_buf.reserve(batch_size);
        let vertex_iter =
            self.char_layout_map
                .drain(start..)
                .enumerate()
                .flat_map(|(index, (ch, vertices))| {
                    self.char_buf.push(ch);
                    vertices.into_iter().map(move |mut vertex| {
                        vertex.in_char_index = index as u32;
                        vertex
                    })
                });
        self.vertex_input_buf.extend(vertex_iter);
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

        let mut sdf_generator =
            SdfGenerator::new(crate::SDF_PADDING, crate::SDF_RADIUS, crate::SDF_CUTOFF);

        let char_descriptor_set = Self::create_char_descriptor_set(
            &self.shared,
            self.char_buf.drain(..),
            &self.fonts,
            &mut sdf_generator,
        );

        let shared = self.shared.clone();

        Some(
            move |b: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>| {
                Self::add_commands_main(shared, vertex_buffer, char_descriptor_set, b)
            },
        )
    }
}

impl TextShader {
    fn create_pipeline(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        screen_size: [f32; 2],
        batch_size: u32,
    ) -> Arc<GraphicsPipeline> {
        let vertex_shader = shaders::text::vs::load(device.clone()).expect("unreachable");
        let fragment_shader = shaders::text::fs::load(device.clone()).expect("unreachable");
        let vertex_shader_entry_point = vertex_shader.entry_point("main").expect("unreachable");
        let fragment_shader_entry_point = fragment_shader.entry_point("main").expect("unreachable");
        let vertex_input_state = VertexInput::per_instance()
            .definition(&vertex_shader_entry_point)
            .expect("unreachable");
        let stages = [
            PipelineShaderStageCreateInfo::new(vertex_shader_entry_point),
            PipelineShaderStageCreateInfo::new(fragment_shader_entry_point),
        ];

        let pipeline_layout = {
            let mut create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .expect("unreachable");
            let set_1 = create_info.set_layouts.get_mut(1).expect("unreachable");
            let default_binding = set_1.bindings().get(&0).expect("unreachable");
            let binding = DescriptorSetLayoutBinding {
                binding_flags: DescriptorBindingFlags::PARTIALLY_BOUND,
                descriptor_count: batch_size,
                ..default_binding.clone()
            };
            let new_set_1 = DescriptorSetLayout::new(
                device.clone(),
                DescriptorSetLayoutCreateInfo {
                    bindings: [(0, binding)].into_iter().collect(),
                    ..Default::default()
                },
            )
            .expect("unreachable");
            *set_1 = new_set_1;
            PipelineLayout::new(device.clone(), create_info).expect("unreachable")
        };
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

    fn create_char_descriptor_set(
        shared: &Shared,
        chars: impl IntoIterator<Item = char>,
        fonts: &FontFallbackList,
        sdf_generator: &mut SdfGenerator,
    ) -> Arc<DescriptorSet> {
        let allocators = &shared.allocators;
        let queue = &shared.queue;
        let pipeline_layout = shared.pipeline.layout();

        let mut command_builder = AutoCommandBufferBuilder::primary(
            allocators.command_buffer().clone(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("unreachable");

        let image_view_vec: Vec<_> = chars
            .into_iter()
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
                image_view_vec
                    .into_iter()
                    .map(|iv| (iv, shared.sampler.clone())),
            )],
            [],
        )
        .expect("unreachable")
    }
}

impl TextShader {
    #[inline]
    fn add_commands_main(
        shared: Shared,
        vertex_buffer: Subbuffer<[VertexInput]>,
        char_descriptor_set: Arc<DescriptorSet>,
        command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        let Shared {
            pipeline,
            descriptor_set,
            ..
        } = shared;
        let instantce_count = vertex_buffer.len() as u32;
        command_builder
            .bind_pipeline_graphics(pipeline.clone())
            .expect("unreachable")
            .bind_vertex_buffers(0, vertex_buffer)
            .expect("unreachable")
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                (descriptor_set, char_descriptor_set),
            )
            .expect("unreachable");
        unsafe { command_builder.draw(4, instantce_count, 0, 0) }.expect("unreachable");
    }
}
