use std::sync::Arc;

use sak_rs::graphics::vulkan::context::Allocators;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, BlitImageInfo, ClearColorImageInfo, CommandBufferUsage,
        PrimaryCommandBufferAbstract, RenderPassBeginInfo,
        allocator::StandardCommandBufferAllocator,
    },
    device::{Device, Queue},
    format::Format,
    image::{Image, ImageCreateInfo, ImageType, ImageUsage, view::ImageView},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    render_pass::{Framebuffer, FramebufferCreateInfo},
    sync::GpuFuture,
};

use crate::app_main_vk::static_overlay::{
    ShaderInitResources, frame::FrameShader, text::TextShader,
};

pub fn static_overlay_image_view(r: &ShaderInitResources) -> Arc<ImageView> {
    let max_batch_size = r
        .queue
        .device()
        .physical_device()
        .properties()
        .max_per_stage_descriptor_samplers;
    let batch_size = max_batch_size.min(1024) as usize;
    Init::new(r, batch_size as u32).draw_static_overlay_image_view(batch_size)
}

struct Init {
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    queue: Arc<Queue>,
    frame_buffer: Arc<Framebuffer>,
    storage_image_view: Arc<ImageView>,

    frame: FrameShader,
    text: TextShader,
}

impl Init {
    fn new(r: &ShaderInitResources, batch_size: u32) -> Self {
        let ShaderInitResources {
            queue,
            allocators,
            screen_size,
            uniform_buffer,
            properties_buffer,
            ..
        } = *r;
        let device = queue.device();

        let (frame_buffer, storage_image_view) =
            Self::create_frame_buffer(allocators, device, screen_size);

        let render_pass = frame_buffer.render_pass();

        let frame = FrameShader::new(
            device.clone(),
            render_pass.clone(),
            allocators,
            screen_size,
            uniform_buffer.clone(),
            properties_buffer.clone(),
        );
        let text = {
            let r = ShaderInitResources { render_pass, ..*r };
            TextShader::new(&r, batch_size)
        };

        Self {
            command_buffer_allocator: allocators.command_buffer().clone(),
            queue: queue.clone(),
            frame_buffer,
            storage_image_view,
            frame,
            text,
        }
    }

    /// returns `(frame_buffer, storage_image_view)`
    fn create_frame_buffer(
        allocators: &Allocators,
        device: &Arc<Device>,
        screen_size: [f32; 2],
    ) -> (Arc<Framebuffer>, Arc<ImageView>) {
        let image = Image::new(
            allocators.memory().clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [screen_size[0] as u32, screen_size[1] as u32, 1],
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_DST
                    | ImageUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .expect("unreachable");
        let image_view = ImageView::new_default(image.clone()).expect("unreachable");
        let storage_image = Image::new(
            allocators.memory().clone(),
            ImageCreateInfo {
                image_type: image.image_type(),
                format: Format::R32G32B32A32_SFLOAT,
                extent: image.extent(),
                usage: ImageUsage::STORAGE | ImageUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .expect("unreachable");
        let storage_image_view =
            ImageView::new_default(storage_image.clone()).expect("unreachable");
        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: image.format(),
                    samples: 1,
                    load_op: Load,
                    store_op: Store,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .expect("unreachable");
        let frame_buffer = Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![image_view.clone()],
                extent: screen_size.map(|x| x as u32),
                ..Default::default()
            },
        )
        .expect("unreachable");
        (frame_buffer, storage_image_view)
    }

    fn draw_static_overlay_image_view(self, batch_size: usize) -> Arc<ImageView> {
        let Self {
            command_buffer_allocator,
            queue,
            frame_buffer,
            storage_image_view,
            frame,
            mut text,
        } = self;

        let frame_image = frame_buffer
            .attachments()
            .get(0)
            .expect("unreachable")
            .image();

        let frame = {
            let frame = frame;
            frame.add_commands()
        };
        let first_text = text.add_commands(batch_size);

        let mut first_command_builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("unreachable");
        first_command_builder
            .clear_color_image(ClearColorImageInfo::image(frame_image.clone()))
            .expect("unrachable")
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![None],
                    ..RenderPassBeginInfo::framebuffer(frame_buffer.clone())
                },
                Default::default(),
            )
            .expect("unreachable");
        frame.map(|f| f(&mut first_command_builder));
        first_text.map(|f| f(&mut first_command_builder));
        first_command_builder
            .end_render_pass(Default::default())
            .expect("unreachable");

        first_command_builder
            .build()
            .expect("unreachable")
            .execute(queue.clone())
            .expect("unreachable")
            .then_signal_fence_and_flush()
            .expect("unreachable")
            .wait(None)
            .expect("unreachable");

        while let Some(next_text) = text.add_commands(batch_size) {
            let mut command_builder = AutoCommandBufferBuilder::primary(
                command_buffer_allocator.clone(),
                queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .expect("unreachable");
            command_builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![None],
                        ..RenderPassBeginInfo::framebuffer(frame_buffer.clone())
                    },
                    Default::default(),
                )
                .expect("unreachable");
            next_text(&mut command_builder);
            command_builder
                .end_render_pass(Default::default())
                .expect("unreachable");

            command_builder
                .build()
                .expect("unreachable")
                .execute(queue.clone())
                .expect("unreachable")
                .then_signal_fence_and_flush()
                .expect("unreachable")
                .wait(None)
                .expect("unreachable");
        }

        let mut storage_command_builder = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("unrachable");
        storage_command_builder
            .blit_image(BlitImageInfo::images(
                frame_image.clone(),
                storage_image_view.image().clone(),
            ))
            .expect("unreachable");
        storage_command_builder
            .build()
            .expect("unreachable")
            .execute(queue)
            .expect("unreachable")
            .then_signal_fence_and_flush()
            .expect("unreachable")
            .wait(None)
            .expect("unreachable");

        storage_image_view
    }
}
