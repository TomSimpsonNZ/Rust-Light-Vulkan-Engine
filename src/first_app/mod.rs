mod lve_device;
mod lve_game_object;
mod lve_model;
mod lve_pipeline;
mod lve_swapchain;

use lve_device::*;
use lve_game_object::*;
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

use std::f32::consts::PI;

extern crate nalgebra as na;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";

#[repr(align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Align16<T>(pub T);

type Pos = Align16<na::Vector2<f32>>;
type Color = Align16<na::Vector3<f32>>;
type Transform = Align16<na::Matrix2<f32>>;

#[derive(Debug)]
pub struct SimplePushConstantData {
    transform: Transform,
    offset: Pos,
    color: Color,
}

impl SimplePushConstantData {
    pub unsafe fn as_bytes(&self) -> &[u8] {
        let size_in_bytes = std::mem::size_of::<Self>();
        let size_in_u8 = size_in_bytes / std::mem::size_of::<u8>();
        let start_ptr = self as *const Self as *const u8;
        std::slice::from_raw_parts(start_ptr, size_in_u8)
    }

    /// This is for debugging, will print out the push constants as they are
    /// represented in memory. Will be useful for spotting alignment issues
    pub unsafe fn _print_buffer(&self) {
        let size_in_bytes = std::mem::size_of::<Self>();
        let size_in_u8 = size_in_bytes / std::mem::size_of::<f32>();
        let start_ptr = self as *const Self as *const f32;
        let buffer = std::slice::from_raw_parts(start_ptr, size_in_u8);
        log::debug!("{:?}", buffer);
    }
}

pub struct VulkanApp {
    window: Window,
    lve_device: LveDevice,
    lve_swapchain: LveSwapchain,
    lve_pipeline: LvePipeline,
    pipeline_layout: vk::PipelineLayout, // I think this should be a part of the pipeline module
    command_buffers: Vec<vk::CommandBuffer>,
    game_objects: Vec<LveGameObject>,
}

impl VulkanApp {
    pub fn new() -> (Self, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        let lve_device = LveDevice::new(&window);

        let window_extent = Self::get_window_extent(&window);
        let lve_swapchain = LveSwapchain::new(&lve_device, window_extent, None);

        let game_objects = Self::load_game_object(&lve_device);

        let pipeline_layout = Self::create_pipeline_layout(&lve_device.device);

        let lve_pipeline =
            Self::create_pipeline(&lve_device.device, &lve_swapchain, &pipeline_layout);

        let command_buffers = Self::create_command_buffers(
            &lve_device.device,
            lve_device.command_pool,
            &lve_swapchain,
        );

        (
            Self {
                window,
                lve_device,
                lve_swapchain,
                lve_pipeline,
                pipeline_layout,
                command_buffers,
                game_objects,
            },
            event_loop,
        )
    }

    pub fn draw_frame(&mut self) {
        let extent = Self::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return; // Don't do anything if the window is minimised
        }

        let (image_index, is_subopt) = unsafe {
            self.lve_swapchain
                .acquire_next_image(&self.lve_device.device)
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        log::error!("Out of date KHR!");
                        self.recreate_swapchain();
                        return;
                    }
                    _ => {
                        log::error!("Unable to aquire next image");
                    }
                })
                .unwrap()
        };

        match is_subopt {
            true => {
                log::warn!("Swapchain is suboptimal for surface");
                self.recreate_swapchain();
            }
            false => {}
        }

        self.record_command_buffer(image_index as usize);

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

    pub fn recreate_swapchain(&mut self) {
        let extent = Self::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return; // Don't do anything if the window is minimised
        }

        log::debug!("Recreating swapchain");

        unsafe {
            self.lve_device
                .device
                .device_wait_idle()
                .map_err(|e| log::error!("Cannot wait: {}", e))
                .unwrap()
        };

        let new_lve_swapchain = LveSwapchain::new(
            &self.lve_device,
            extent,
            Some(self.lve_swapchain.swapchain_khr),
        );

        let new_command_buffers: Option<Vec<vk::CommandBuffer>> =
            if new_lve_swapchain.image_count() != self.command_buffers.len() {
                unsafe { self.free_command_buffers() };
                Some(Self::create_command_buffers(
                    &self.lve_device.device,
                    self.lve_device.command_pool,
                    &new_lve_swapchain,
                ))
            } else {
                None
            };

        let new_lve_pipeline = Self::create_pipeline(
            &self.lve_device.device,
            &new_lve_swapchain,
            &self.pipeline_layout,
        );

        unsafe {
            self.lve_swapchain.destroy(&self.lve_device.device);
            self.lve_pipeline.destroy(&self.lve_device.device)
        };

        self.lve_swapchain = new_lve_swapchain;
        self.lve_pipeline = new_lve_pipeline;

        match new_command_buffers {
            Some(cbs) => self.command_buffers = cbs,
            None => {}
        }
    }

    unsafe fn free_command_buffers(&mut self) {
        self.lve_device
            .device
            .free_command_buffers(self.lve_device.command_pool, &self.command_buffers);
        self.command_buffers.clear();
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
            .with_resizable(true)
            .build(&event_loop)
            .unwrap();

        (event_loop, winit_window)
    }

    fn create_pipeline(
        device: &Device,
        lve_swapchain: &LveSwapchain,
        pipeline_layout: &vk::PipelineLayout,
    ) -> LvePipeline {
        assert!(
            lve_swapchain.swapchain_khr != vk::SwapchainKHR::null(),
            "Cannot create pipeline before swapchain"
        );
        assert!(
            pipeline_layout != &vk::PipelineLayout::null(),
            "Cannot create pipeline before pipeline layout"
        );

        let pipeline_config = LvePipeline::default_pipline_config_info();

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
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<SimplePushConstantData>() as u32)
            .build();

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            // .set_layouts(&[vk::DescriptorSetLayout::null()])
            .push_constant_ranges(&[push_constant_range])
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
    }

    fn record_command_buffer(&mut self, image_index: usize) {
        let begin_info = vk::CommandBufferBeginInfo::builder().build();

        let command_buffer = self.command_buffers[image_index];

        unsafe {
            self.lve_device
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| log::error!("Unable to begin command buffer: {}", e))
                .unwrap()
        };

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.lve_swapchain.swapchain_extent,
        };

        let color_clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.01, 0.01, 0.01, 1.0],
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
            .render_pass(self.lve_swapchain.render_pass)
            .framebuffer(self.lve_swapchain.swapchain_framebuffers[image_index])
            .render_area(render_area)
            .clear_values(&clear_values)
            .build();

        unsafe {
            self.lve_device.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            let viewport = vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(self.lve_swapchain.width() as f32)
                .height(self.lve_swapchain.height() as f32)
                .min_depth(0.0)
                .max_depth(1.0)
                .build();

            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.lve_swapchain.swapchain_extent,
            };

            self.lve_device
                .device
                .cmd_set_viewport(command_buffer, 0, &[viewport]);
            self.lve_device
                .device
                .cmd_set_scissor(command_buffer, 0, &[scissor]);

            self.lve_pipeline
                .bind(&self.lve_device.device, command_buffer);

            for _ in 0..4 {
                self.render_game_objects(command_buffer);
            }

            self.lve_device.device.cmd_end_render_pass(command_buffer);

            self.lve_device
                .device
                .end_command_buffer(command_buffer)
                .map_err(|e| log::error!("Unable to end command buffer: {}", e))
                .unwrap()
        };
    }

    fn load_game_object(lve_device: &LveDevice) -> Vec<LveGameObject> {
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

        let colors = vec![
            na::vector![1.0, 0.49, 0.5],
            na::vector![1.0, 0.81, 0.5],
            na::vector![1.0, 1.0, 0.5],
            na::vector![0.5, 0.49, 0.64],
            na::vector![0.5, 0.81, 1.0]
        ];

        let mut game_objects = Vec::new();

        for i in 0..40 {
            let model = LveModel::new(lve_device, &vertices);
            let color = colors[i % colors.len()];
            let transform = Transform2DComponent {
                translation: na::vector![0.0, 0.0],
                scale: na::vector![0.5 + (i as f32) * 0.025, 0.5 + (i as f32) * 0.025],
                rotation: (i as f32) * 0.025 * PI,
            };

            game_objects.push(LveGameObject::new(model, color, transform));
        }

        game_objects
    }

    fn render_game_objects(&mut self, command_buffer: vk::CommandBuffer) {

        unsafe {
            self.lve_pipeline
                .bind(&self.lve_device.device, command_buffer)
        };

        let mut i = 0;

        for game_obj in self.game_objects.iter_mut() {
            game_obj.transform.rotation = (game_obj.transform.rotation + 0.0001 * (i as f32)) % (2.0 * PI);
            i += 1;

            let push = SimplePushConstantData {
                transform: Align16(game_obj.transform.mat2()),
                offset: Align16(game_obj.transform.translation),
                color: Align16(game_obj.color),
            };

            unsafe {
                let push_ptr = push.as_bytes();

                self.lve_device.device.cmd_push_constants(
                    command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    push_ptr,
                );

                game_obj.model.bind(&self.lve_device.device, command_buffer);
                game_obj.model.draw(&self.lve_device.device, command_buffer);
            }
        }
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

            log::debug!("Destroying game objects");
            for game_obj in self.game_objects.iter_mut() {
                game_obj.destroy(&self.lve_device.device);
            }

            log::debug!("Destroying swapchain");
            self.lve_swapchain.destroy(&self.lve_device.device);

            log::debug!("Destroying pipeline");
            self.lve_pipeline.destroy(&self.lve_device.device);

            log::debug!("Destroying device");
            self.lve_device.destroy()
        }
    }
}
