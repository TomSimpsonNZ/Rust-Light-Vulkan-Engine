use super::lve_device::*;

use ash::extensions::khr::Swapchain;
use ash::version::DeviceV1_0;
use ash::{vk, Device};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct LveSwapchain {
    swapchain: Swapchain,
    pub swapchain_khr: vk::SwapchainKHR,
    _swapchain_image_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    pub swapchain_framebuffers: Vec<vk::Framebuffer>,
    pub render_pass: vk::RenderPass,
    depth_images: Vec<vk::Image>,
    depth_image_memories: Vec<vk::DeviceMemory>,
    depth_image_views: Vec<vk::ImageView>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    images_in_flight: Vec<vk::Fence>,
    current_frame: usize,
}

impl LveSwapchain {
    pub fn new(
        lve_device: &LveDevice,
        window_extent: vk::Extent2D,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Self {
        let old_swapchain = match old_swapchain {
            Some(swapchain) => swapchain,
            None => vk::SwapchainKHR::null(),
        };

        let (swapchain, swapchain_khr, swapchain_images, swapchain_image_format, swapchain_extent) =
            Self::create_swapchain(lve_device, window_extent, old_swapchain);

        let swapchain_image_views = Self::create_image_views(
            &lve_device.device,
            &swapchain_images,
            swapchain_image_format,
        );

        let render_pass = Self::create_render_pass(&lve_device, swapchain_image_format);

        let (depth_images, depth_image_memories, depth_image_views) =
            Self::create_depth_resources(lve_device, &swapchain_images, swapchain_extent);

        let swapchain_framebuffers = Self::create_framebuffers(
            &lve_device.device,
            swapchain_extent,
            &swapchain_image_views,
            &depth_image_views,
            render_pass,
        );

        let (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
        ) = Self::create_sync_objects(&lve_device.device, &swapchain_images);

        Self {
            swapchain,
            swapchain_khr,
            _swapchain_image_format: swapchain_image_format,
            swapchain_extent,
            swapchain_images,
            swapchain_image_views,
            swapchain_framebuffers,
            render_pass,
            depth_images,
            depth_image_memories,
            depth_image_views,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            current_frame: 0,
        }
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        self.swapchain_image_views
            .iter()
            .for_each(|iv| device.destroy_image_view(*iv, None));

        self.swapchain.destroy_swapchain(self.swapchain_khr, None);

        self.depth_image_views
            .iter()
            .for_each(|iv| device.destroy_image_view(*iv, None));

        self.depth_images
            .iter()
            .for_each(|i| device.destroy_image(*i, None));

        self.depth_image_memories
            .iter()
            .for_each(|m| device.free_memory(*m, None));

        self.swapchain_framebuffers
            .iter()
            .for_each(|f| device.destroy_framebuffer(*f, None));

        device.destroy_render_pass(self.render_pass, None);

        self.render_finished_semaphores
            .iter()
            .for_each(|s| device.destroy_semaphore(*s, None));

        self.image_available_semaphores
            .iter()
            .for_each(|s| device.destroy_semaphore(*s, None));

        self.in_flight_fences
            .iter()
            .for_each(|f| device.destroy_fence(*f, None));
    }

    pub fn image_count(&self) -> usize {
        self.swapchain_images.len()
    }

    pub fn width(&self) -> u32 {
        self.swapchain_extent.width
    }

    pub fn height(&self) -> u32 {
        self.swapchain_extent.height
    }

    pub fn extent_aspect_ratio(&self) -> f32 {
        self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32
    }

    pub fn find_depth_format(lve_device: &LveDevice) -> vk::Format {
        let candidates = vec![
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];
        lve_device.find_supported_format(
            &candidates,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }

    pub unsafe fn acquire_next_image(&self, device: &Device) -> Result<(u32, bool), vk::Result> {
        device
            .wait_for_fences(
                &[self.in_flight_fences[self.current_frame]],
                false,
                u64::MAX,
            )
            .map_err(|e| log::error!("Unable to wait for fences: {}", e))
            .unwrap();

        self.swapchain.acquire_next_image(
            self.swapchain_khr,
            u64::MAX,
            self.image_available_semaphores[self.current_frame],
            vk::Fence::null(),
        ) // Return the result of acquire next image
    }

    pub fn submit_command_buffers(
        &mut self,
        device: &Device,
        graphics_queue: &vk::Queue,
        present_queue: &vk::Queue,
        buffer: &vk::CommandBuffer,
        image_index: usize,
    ) -> Result<bool, vk::Result> {
        if self.images_in_flight[image_index] != vk::Fence::null() {
            unsafe {
                device
                    .wait_for_fences(&[self.images_in_flight[image_index]], true, u64::MAX)
                    .map_err(|e| log::error!("Unable to wait for fences: {}", e))
                    .unwrap()
            };
        }

        self.images_in_flight[image_index] = self.in_flight_fences[self.current_frame];

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];

        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&[*buffer])
            .signal_semaphores(&signal_semaphores)
            .build();

        unsafe {
            device
                .reset_fences(&[self.in_flight_fences[self.current_frame]])
                .map_err(|e| log::error!("Unable to reset fences: {}", e))
                .unwrap();

            device
                .queue_submit(
                    *graphics_queue,
                    &[submit_info],
                    self.in_flight_fences[self.current_frame],
                )
                .map_err(|e| log::error!("Unable to submit draw command buffer: {}", e))
                .unwrap();
        };

        let swapchains = [self.swapchain_khr];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&[image_index as u32])
            .build();

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        unsafe { self.swapchain.queue_present(*present_queue, &present_info) }
    }

    fn create_swapchain(
        lve_device: &LveDevice,
        window_extent: vk::Extent2D,
        old_swapchain: vk::SwapchainKHR,
    ) -> (
        Swapchain,
        vk::SwapchainKHR,
        Vec<vk::Image>,
        vk::Format,
        vk::Extent2D,
    ) {
        let swapchain_support = lve_device.get_swapchain_support();

        let surface_format = Self::choose_swap_surface_format(&swapchain_support.formats);

        let present_mode = Self::choose_swap_present_mode(&swapchain_support.present_modes);

        let extent = Self::choose_swap_extent(&swapchain_support.capabilities, window_extent);

        let mut image_count = swapchain_support.capabilities.min_image_count + 1;

        if swapchain_support.capabilities.max_image_count > 0
            && image_count > swapchain_support.capabilities.max_image_count
        {
            image_count = swapchain_support.capabilities.max_image_count;
        }

        let mut create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(lve_device.surface_khr)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

        let indices = lve_device.find_physical_queue_families();

        let queue_family_indices = [indices.graphics_family, indices.present_family];

        if indices.graphics_family != indices.present_family {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices);
        } else {
            create_info = create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        }

        let create_info = create_info
            .pre_transform(swapchain_support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain)
            .build();

        let swapchain = Swapchain::new(&lve_device.instance, &lve_device.device);

        let swapchain_khr = unsafe {
            swapchain
                .create_swapchain(&create_info, None)
                .map_err(|e| log::error!("Unable to create swapchain: {}", e))
                .unwrap()
        };

        // we only specified a minimum number of images in the swap chain, so the implementation is
        // allowed to create a swap chain with more. That's why we'll first query the final number of
        // images with vkGetSwapchainImagesKHR, then resize the container and finally call it again to
        // retrieve the handles.
        let swapchain_images = unsafe {
            swapchain
                .get_swapchain_images(swapchain_khr)
                .map_err(|e| log::error!("Unable to get swapchain images: {}", e))
                .unwrap()
        };

        let swapchain_image_format = surface_format.format;

        let swapchain_extent = extent;

        (
            swapchain,
            swapchain_khr,
            swapchain_images,
            swapchain_image_format,
            swapchain_extent,
        )
    }

    fn create_image_views(
        device: &Device,
        swapchain_images: &Vec<vk::Image>,
        swapchain_image_format: vk::Format,
    ) -> Vec<vk::ImageView> {
        swapchain_images
            .iter()
            .map(|image| {
                let view_info = vk::ImageViewCreateInfo::builder()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain_image_format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .build();

                unsafe {
                    device
                        .create_image_view(&view_info, None)
                        .map_err(|e| log::error!("Unable to create image view: {}", e))
                        .unwrap()
                }
            })
            .collect::<Vec<_>>()
    }

    fn create_depth_resources(
        lve_device: &LveDevice,
        swapchain_images: &Vec<vk::Image>,
        swapchain_extent: vk::Extent2D,
    ) -> (Vec<vk::Image>, Vec<vk::DeviceMemory>, Vec<vk::ImageView>) {
        let depth_format = Self::find_depth_format(lve_device);

        let (images, image_memories): (Vec<vk::Image>, Vec<vk::DeviceMemory>) = swapchain_images
            .iter()
            .map(|_| {
                let extent = vk::Extent3D {
                    width: swapchain_extent.width,
                    height: swapchain_extent.height,
                    depth: 1,
                };

                let image_info = vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .extent(extent)
                    .mip_levels(1)
                    .array_layers(1)
                    .format(depth_format)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .flags(vk::ImageCreateFlags::empty())
                    .build();

                lve_device
                    .create_image_with_info(&image_info, vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .unzip();

        let image_views = images
            .iter()
            .map(|image| {
                let view_info = vk::ImageViewCreateInfo::builder()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(depth_format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::DEPTH,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .build();

                unsafe {
                    lve_device
                        .device
                        .create_image_view(&view_info, None)
                        .map_err(|e| log::error!("Unable to create depth image view: {}", e))
                        .unwrap()
                }
            })
            .collect::<Vec<_>>();

        (images, image_memories, image_views)
    }

    fn create_render_pass(
        lve_device: &LveDevice,
        swapchain_image_format: vk::Format,
    ) -> vk::RenderPass {
        let depth_attachment = vk::AttachmentDescription::builder()
            .format(Self::find_depth_format(lve_device))
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let depth_attachment_ref = vk::AttachmentReference::builder()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let color_attachment = vk::AttachmentDescription::builder()
            .format(swapchain_image_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&[color_attachment_ref])
            .depth_stencil_attachment(&depth_attachment_ref)
            .build();

        let dependancy = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .src_access_mask(vk::AccessFlags::empty())
            .src_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .dst_subpass(0)
            .dst_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )
            .build();

        let attachments = [color_attachment, depth_attachment];

        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&[subpass])
            .dependencies(&[dependancy])
            .build();

        unsafe {
            lve_device
                .device
                .create_render_pass(&render_pass_info, None)
                .map_err(|e| log::error!("Unable to create render pass: {}", e))
                .unwrap()
        }
    }

    fn create_framebuffers(
        device: &Device,
        swapchain_extent: vk::Extent2D,
        swapchain_image_views: &Vec<vk::ImageView>,
        depth_image_views: &Vec<vk::ImageView>,
        render_pass: vk::RenderPass,
    ) -> Vec<vk::Framebuffer> {
        swapchain_image_views
            .iter()
            .zip(depth_image_views)
            .map(|view| [*view.0, *view.1])
            .map(|attachments| {
                let frame_buffer_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(swapchain_extent.width)
                    .height(swapchain_extent.height)
                    .layers(1)
                    .build();

                unsafe {
                    device
                        .create_framebuffer(&frame_buffer_info, None)
                        .map_err(|e| log::error!("Unable to create framebuffer: {}", e))
                        .unwrap()
                }
            })
            .collect::<Vec<_>>()
    }

    fn create_sync_objects(
        device: &Device,
        swapchain_images: &Vec<vk::Image>,
    ) -> (
        Vec<vk::Semaphore>,
        Vec<vk::Semaphore>,
        Vec<vk::Fence>,
        Vec<vk::Fence>,
    ) {
        let semaphore_info = vk::SemaphoreCreateInfo::builder().build();

        let fence_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();

        let mut image_available_semaphores = Vec::new();
        let mut render_finished_semaphore = Vec::new();
        let mut in_flight_fences = Vec::new();

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe {
                image_available_semaphores.push(
                    device
                        .create_semaphore(&semaphore_info, None)
                        .map_err(|e| {
                            log::error!("Unable to create image available semaphore: {}", e)
                        })
                        .unwrap(),
                );

                render_finished_semaphore.push(
                    device
                        .create_semaphore(&semaphore_info, None)
                        .map_err(|e| {
                            log::error!("Unable to create render finished semaphore: {}", e)
                        })
                        .unwrap(),
                );

                in_flight_fences.push(
                    device
                        .create_fence(&fence_info, None)
                        .map_err(|e| log::error!("Unable to create in flight fence: {}", e))
                        .unwrap(),
                );
            }
        }

        let images_in_flight = vec![vk::Fence::null(); swapchain_images.len()];

        (
            image_available_semaphores,
            render_finished_semaphore,
            in_flight_fences,
            images_in_flight,
        )
    }

    fn choose_swap_surface_format(
        available_formats: &Vec<vk::SurfaceFormatKHR>,
    ) -> vk::SurfaceFormatKHR {
        let format = available_formats
            .iter()
            .map(|f| *f)
            .find(|available_format| {
                available_format.format == vk::Format::B8G8R8A8_SRGB
                    && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| {
                log::warn!(
                    "Could not find appropriate surface format, returning first available format"
                );
                available_formats[0]
            });

        log::debug!("Surface format: {:?}", format);

        format
    }

    fn choose_swap_present_mode(
        available_present_modes: &Vec<vk::PresentModeKHR>,
    ) -> vk::PresentModeKHR {
        let present_mode = available_present_modes
            .iter()
            .map(|pm| *pm)
            .find(|available_present_mode| *available_present_mode == vk::PresentModeKHR::MAILBOX)
            // .find(|available_present_mode| {
            //     *available_present_mode == vk::PresentModeKHR::IMMEDIATE
            // })
            .unwrap_or_else(|| {
                log::warn!("Could not find desired present mode, defaulting to FIFO");
                vk::PresentModeKHR::FIFO
            });

        log::debug!("Present mode: {:?}", present_mode);

        present_mode
    }

    fn choose_swap_extent(
        capabilities: &vk::SurfaceCapabilitiesKHR,
        window_extent: vk::Extent2D,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        } else {
            return vk::Extent2D {
                width: std::cmp::max(
                    capabilities.min_image_extent.width,
                    std::cmp::min(capabilities.max_image_extent.width, window_extent.width),
                ),
                height: std::cmp::max(
                    capabilities.min_image_extent.height,
                    std::cmp::min(capabilities.max_image_extent.height, window_extent.height),
                ),
            };
        }
    }
}
