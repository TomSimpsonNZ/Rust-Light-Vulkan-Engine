mod lve_device;
mod lve_pipeline;
mod lve_swapchain;

use lve_device::*;
use lve_pipeline::*;
use lve_swapchain::*;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use ash::{vk, Device};
use ash::version::{DeviceV1_0};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";

pub struct VulkanApp {
    window: Window,
    lve_device: LveDevice,
    lve_swapchain: LveSwapchain,
    lve_pipeline: LvePipeline,
    pipeline_layout: vk::PipelineLayout, // I think this should be a part of the pipeline module
                                         // command_buffers: Vec<vk::CommandBuffer>,
}

impl VulkanApp {
    pub fn new() -> (Self, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        let lve_device = LveDevice::new(&window, WIDTH, HEIGHT);

        let window_extent = Self::get_window_extent(&window);
        let lve_swapchain = LveSwapchain::new(&lve_device, window_extent);

        let pipeline_layout = Self::create_pipeline_layout(&lve_device.device);
        let lve_pipeline =
            Self::create_pipeline(&lve_device.device, &lve_swapchain, pipeline_layout);

        // let command_buffers = Self::create_command_buffers();

        (
            Self {
                window,
                lve_device,
                lve_swapchain,
                lve_pipeline,
                pipeline_layout,
            },
            event_loop,
        )
    }

    pub fn draw_frame() {}

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
        pipeline_layout: vk::PipelineLayout,
    ) -> LvePipeline {
        let mut pipeline_config =
            LvePipeline::default_pipline_config_info(lve_swapchain.width(), lve_swapchain.height());

        pipeline_config.render_pass = lve_swapchain.render_pass; // I don't like this
        pipeline_config.pipeline_layout = pipeline_layout;

        LvePipeline::new(
            device,
            "shaders/simple_shader.vert.spv",
            "shaders/simple_shader.frag.spv",
            &pipeline_config,
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

    // fn create_command_buffers() -> Vec<vk::CommandBuffer> {}
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application");

        unsafe {
            log::debug!("Destroying pipeline layout");
            self.lve_device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            log::debug!("Destroying Swapchain");
            self.lve_swapchain.destroy(&self.lve_device.device);

            log::debug!("Destroying pipeline");
            self.lve_pipeline.destroy(&self.lve_device.device);

            log::debug!("Destroying device");
            self.lve_device.destroy()
        }
    }
}
