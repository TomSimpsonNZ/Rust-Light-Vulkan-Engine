use super::lve_device::*;
use super::lve_swapchain::*;

use winit::window::Window;

use ash::version::DeviceV1_0;
use ash::{vk, Device};
use std::rc::Rc;

pub struct LveRenderer {
    lve_device: Rc<LveDevice>,
    pub lve_swapchain: LveSwapchain,
    command_buffers: Vec<vk::CommandBuffer>,
    current_image_index: usize,
    current_frame_index: usize,
    pub is_frame_started: bool,
}

impl LveRenderer {
    pub fn new(lve_device: Rc<LveDevice>, window: &Window) -> Self {
        let window_extent = Self::get_window_extent(window);

        let lve_swapchain = LveSwapchain::new(Rc::clone(&lve_device), window_extent, None);

        let command_buffers =
            Self::create_command_buffers(&lve_device.device, lve_device.command_pool);

        Self {
            lve_device,
            lve_swapchain,
            command_buffers,
            current_image_index: 0,
            current_frame_index: 0,
            is_frame_started: false,
        }
    }

    pub fn get_frame_index(&self) -> usize {
        assert!(
            self.is_frame_started,
            "Cannot get frame index when frame is not in progress"
        );
        self.current_frame_index
    }

    pub fn get_current_command_buffer(&self) -> vk::CommandBuffer {
        assert!(
            self.is_frame_started,
            "Cannot get command buffer when frame not in progress"
        );
        self.command_buffers[self.current_frame_index]
    }

    pub fn get_swapchain_render_pass(&self) -> vk::RenderPass {
        self.lve_swapchain.render_pass
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        self.lve_swapchain.extent_aspect_ratio()
    }

    pub fn begin_frame(&mut self, window: &Window) -> Option<vk::CommandBuffer> {
        assert!(
            !self.is_frame_started,
            "Can't call begin_frame while already in progress"
        );

        let extent = Self::get_window_extent(window);

        if extent.width == 0 || extent.height == 0 {
            return None; // Don't do anything if the window is minimised
        }

        let result = unsafe {
            self.lve_swapchain
                .acquire_next_image(&self.lve_device.device)
        };

        match result {
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::error!("Out of date KHR!");
                self.recreate_swapchain(window);
                return None;
            }
            Err(_) => {
                log::error!("Unable to acquire next image");
                panic!("Unable to handle this error")
            }
            Ok((current_image_index, is_subopt)) => {
                match is_subopt {
                    true => {
                        log::warn!("Swapchain is suboptimal for surface");
                        self.recreate_swapchain(window);
                    }
                    false => {}
                }

                self.is_frame_started = true;
                self.current_image_index = current_image_index as usize;
            }
        }

        let command_buffer = self.get_current_command_buffer();

        let begin_info = vk::CommandBufferBeginInfo::builder().build();

        unsafe {
            self.lve_device
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| log::error!("Unable to begin command buffer: {}", e))
                .unwrap()
        };

        return Some(command_buffer);
    }

    pub fn end_frame(&mut self) {
        assert!(
            self.is_frame_started,
            "Can't call end_frame while frame is not in progress"
        );
        let command_buffer = self.get_current_command_buffer();

        unsafe {
            self.lve_device
                .device
                .end_command_buffer(command_buffer)
                .map_err(|e| log::error!("Unable to end command buffer: {}", e))
                .unwrap()
        };

        let _result = self
            .lve_swapchain
            .submit_command_buffers(
                &self.lve_device.device,
                &self.lve_device.graphics_queue,
                &self.lve_device.present_queue,
                &command_buffer,
                self.current_image_index,
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

        self.is_frame_started = false;
        self.current_frame_index = (self.current_frame_index + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn begin_swapchain_render_pass(&self, command_buffer: vk::CommandBuffer) {
        assert!(
            self.is_frame_started,
            "Can't call begin_swpachain_render_pass while frame is not in progress"
        );

        assert_eq!(
            command_buffer,
            self.get_current_command_buffer(),
            "Can't begin render pass on a command buffer from a different frame"
        );

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
            .framebuffer(self.lve_swapchain.swapchain_framebuffers[self.current_image_index])
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
        };
    }

    pub fn end_swapchain_render_pass(&self, command_buffer: vk::CommandBuffer) {
        assert!(
            self.is_frame_started,
            "Can't call end_swpachain_render_pass while frame is not in progress"
        );

        assert_eq!(
            command_buffer,
            self.get_current_command_buffer(),
            "Can't end render pass on a command buffer from a different frame"
        );

        unsafe {
            self.lve_device.device.cmd_end_render_pass(command_buffer);
        }
    }

    pub fn recreate_swapchain(&mut self, window: &Window) {
        let extent = Self::get_window_extent(window);

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

        let new_lve_swapchain =
            LveSwapchain::new(Rc::clone(&self.lve_device), extent, Some(self.lve_swapchain.swapchain_khr));

        self.lve_swapchain
            .compare_swap_formats(&new_lve_swapchain)
            .map_err(|e| log::error!("Swapchain image (or depth) format has changed"))
            .unwrap();

        self.lve_swapchain = new_lve_swapchain;

        // We'll come back to this
    }

    fn get_window_extent(window: &Window) -> vk::Extent2D {
        let window_inner_size = window.inner_size();
        vk::Extent2D {
            width: window_inner_size.width,
            height: window_inner_size.height,
        }
    }

    fn create_command_buffers(
        device: &Device,
        command_pool: vk::CommandPool,
    ) -> Vec<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32)
            .build();

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| log::error!("Unable to allocate command buffer: {}", e))
                .unwrap()
        };

        command_buffers
    }
}

impl Drop for LveRenderer {
    fn drop(&mut self) {
        log::debug!("Dropping renderer");
        unsafe {
            self.lve_device.device.free_command_buffers(self.lve_device.command_pool, &self.command_buffers);
            self.command_buffers.clear();
        }
    }
}
