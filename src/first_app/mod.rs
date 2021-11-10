mod lve_device;
mod lve_model;
mod lve_pipeline;
mod lve_swapchain;

use lve_device::*;
use lve_model::*;
use lve_pipeline::*;
use lve_swapchain::*;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use ash::version::DeviceV1_0;
use ash::{vk, Device};

extern crate nalgebra as na;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";

pub struct VulkanApp {
    window: Window,
    lve_device: LveDevice,
    lve_swapchain: LveSwapchain,
    lve_pipeline: LvePipeline,
    pipeline_layout: vk::PipelineLayout, // I think this should be a part of the pipeline module
    command_buffers: Vec<vk::CommandBuffer>,
    lve_model: LveModel,
}

impl VulkanApp {
    pub fn new() -> (Self, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        let lve_device = LveDevice::new(&window, WIDTH, HEIGHT);

        let window_extent = Self::get_window_extent(&window);
        let lve_swapchain = LveSwapchain::new(&lve_device, window_extent);

        let lve_model = Self::load_models(&lve_device);

        let pipeline_layout = Self::create_pipeline_layout(&lve_device.device);
        let lve_pipeline =
            Self::create_pipeline(&lve_device.device, &lve_swapchain, &pipeline_layout);

        let command_buffers = Self::create_command_buffers(
            &lve_device.device,
            lve_device.command_pool,
            &lve_swapchain,
            &lve_pipeline,
            &lve_model,
        );

        (
            Self {
                window,
                lve_device,
                lve_swapchain,
                lve_pipeline,
                pipeline_layout,
                command_buffers,
                lve_model,
            },
            event_loop,
        )
    }

    pub fn draw_frame(&mut self) {
        let (image_index, result) = unsafe {
            self.lve_swapchain
                .acquire_next_image(&self.lve_device.device)
                .map_err(|e| log::error!("Unable to acquire next image: {}", e))
                .unwrap()
        };

        match result {
            true => {
                log::error!("Swapchain is suboptimal for surface");
                panic!("Will handle this better later");
            }

            false => {}
        }

        let _result = self
            .lve_swapchain
            .submit_command_buffers(
                &self.lve_device.device,
                &self.lve_device.graphics_queue,
                &self.lve_device.present_queue,
                &self.command_buffers[image_index as usize],
                image_index as usize,
            )
            .map_err(|e| log::error!("Unable to present swapchain image: {}", e))
            .unwrap();

        unsafe {
            self.lve_device
                .device
                .device_wait_idle()
                .map_err(|e| log::error!("Cannot wait: {}", e))
                .unwrap()
        };
    }

    fn get_window_extent(window: &Window) -> vk::Extent2D {
        let window_inner_size = window.inner_size();
        vk::Extent2D {
            width: window_inner_size.width,
            height: window_inner_size.height,
        }
    }

    fn new_window(w: u32, h: u32, name: &str) -> (EventLoop<()>, Window) {
        log::debug!("Starting event loop");
        let event_loop = EventLoop::new();

        log::debug!("Creating window");
        let winit_window = WindowBuilder::new()
            .with_title(name)
            .with_inner_size(LogicalSize::new(w, h))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();

        (event_loop, winit_window)
    }

    fn create_pipeline(
        device: &Device,
        lve_swapchain: &LveSwapchain,
        pipeline_layout: &vk::PipelineLayout,
    ) -> LvePipeline {
        let pipeline_config =
            LvePipeline::default_pipline_config_info(lve_swapchain.width(), lve_swapchain.height());

        LvePipeline::new(
            device,
            "shaders/simple_shader.vert.spv",
            "shaders/simple_shader.frag.spv",
            pipeline_config,
            &lve_swapchain.render_pass,
            pipeline_layout,
        )
    }

    fn create_pipeline_layout(device: &Device) -> vk::PipelineLayout {
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            // .set_layouts(&[vk::DescriptorSetLayout::null()])
            // .push_constant_ranges(&[vk::PushConstantRange::null()])
            .build();

        unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| log::error!("Unable to create pipeline layout: {}", e))
                .unwrap()
        }
    }

    fn create_command_buffers(
        device: &Device,
        command_pool: vk::CommandPool,
        lve_swapchain: &LveSwapchain,
        lve_pipeline: &LvePipeline,
        lve_model: &LveModel,
    ) -> Vec<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(lve_swapchain.image_count() as u32)
            .build();

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| log::error!("Unable to allocate command buffer: {}", e))
                .unwrap()
        };

        command_buffers
            .iter()
            .zip(lve_swapchain.swapchain_framebuffers.iter())
            .for_each(|(command_buffer, frame_buffer)| {
                let begin_info = vk::CommandBufferBeginInfo::builder().build();

                unsafe {
                    device
                        .begin_command_buffer(*command_buffer, &begin_info)
                        .map_err(|e| log::error!("Unable to begin command buffer: {}", e))
                        .unwrap()
                };

                let render_area = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: lve_swapchain.swapchain_extent,
                };

                let color_clear = vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.1, 0.1, 0.1, 1.0],
                    },
                };

                let depth_clear = vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                };

                let clear_values = [color_clear, depth_clear];

                let render_pass_info = vk::RenderPassBeginInfo::builder()
                    .render_pass(lve_swapchain.render_pass)
                    .framebuffer(*frame_buffer)
                    .render_area(render_area)
                    .clear_values(&clear_values)
                    .build();

                unsafe {
                    device.cmd_begin_render_pass(
                        *command_buffer,
                        &render_pass_info,
                        vk::SubpassContents::INLINE,
                    );

                    lve_pipeline.bind(device, *command_buffer);

                    lve_model.bind(device, *command_buffer);
                    lve_model.draw(device, *command_buffer);

                    device.cmd_end_render_pass(*command_buffer);

                    device
                        .end_command_buffer(*command_buffer)
                        .map_err(|e| log::error!("Unable to end command buffer: {}", e))
                        .unwrap()
                };
            });

        command_buffers
    }

    fn load_models(lve_device: &LveDevice) -> LveModel {
        let vertices = vec![
            Vertex {
                position: na::vector![0.0, -0.5],
                color: na::vector![1.0, 0.0, 0.0],
            },
            Vertex {
                position: na::vector![0.5, 0.5],
                color: na::vector![0.0, 1.0, 0.0],
            },
            Vertex {
                position: na::vector![-0.5, 0.5],
                color: na::vector![0.0, 0.0, 1.0],
            },
        ];

        LveModel::new(lve_device, &vertices)
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application");

        unsafe {
            log::debug!("Destroying pipeline layout");
            self.lve_device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            log::debug!("Destroying vertex buffers");
            self.lve_model.destroy(&self.lve_device.device);

            log::debug!("Destroying swapchain");
            self.lve_swapchain.destroy(&self.lve_device.device);

            log::debug!("Destroying pipeline");
            self.lve_pipeline.destroy(&self.lve_device.device);

            log::debug!("Destroying device");
            self.lve_device.destroy()
        }
    }
}
