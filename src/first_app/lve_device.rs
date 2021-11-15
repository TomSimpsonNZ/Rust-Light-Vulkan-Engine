use ash::extensions::{
    ext::DebugUtils, // Read more about debugging here: https://www.lunarg.com/new-tutorial-for-vulkan-debug-utilities-extension/
    khr::{Surface, Swapchain},
};

#[cfg(target_os="linux")]
use ash::extensions::khr::XlibSurface;
#[cfg(target_os="windows")]
use ash::extensions::khr::Win32Surface;

use ash::{vk, Device, Entry, Instance};

use ash_window;

use winit::window::Window;

use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
    rc::Rc,
};

#[cfg(debug_assertions)]
pub const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
pub const ENABLE_VALIDATION_LAYERS: bool = false;

// What validation layers we want to use in out application
const VALIDATION_LAYERS: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

// A function that will print the error messages to the terminal depending on importance
unsafe extern "system" fn vulkan_debug_callback(
    flag: vk::DebugUtilsMessageSeverityFlagsEXT,
    typ: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    // Extract the message from the Callback Data
    let message = CStr::from_ptr((*p_callback_data).p_message);

    // Log the message depending on severity
    if flag == vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        log::error!("{:?} - {:?}", typ, message);
    } else if flag == vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        log::info!("{:?} - {:?}", typ, message);
    } else if flag == vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        log::warn!("{:?} - {:?}", typ, message);
    } else { // Any verbose logging goes here
         // log::info!("{:?} - {:?}", typ, message);
    }

    // Should we skip the call to the driver?
    vk::FALSE // No
}

///
/// Struct to store the swapchain details
///
/// # Fields
/// ```
/// capabilities: vk::SurfaceCapabilitiesKHR
/// formats: Vec<vk::SurfaceFormatKHR>
/// present_mode: Vec<vk::PresentModeKHR>
/// ```
pub struct SwapChainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

///
/// Struct to store the queue family indices
///
/// # Fields
/// ```
/// graphics_family: u32
/// present_family: u32
/// graphics_family_has_value: bool
/// present_family_has_value: bool
/// ```
pub struct QueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
    graphics_family_has_value: bool,
    present_family_has_value: bool,
}

impl QueueFamilyIndices {
    pub fn is_complete(self) -> bool {
        self.graphics_family_has_value && self.present_family_has_value
    }
}

pub struct LveDevice {
    _entry: Entry,
    pub instance: Instance,
    debug_messenger: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
    surface: Surface,
    pub surface_khr: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    _properties: vk::PhysicalDeviceProperties,
    pub device: Device,
    pub command_pool: vk::CommandPool,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl LveDevice {
    /// Will create a new instance of a vulkan device and all of it's associated functions
    pub fn new(window: &Window) -> Rc<Self> {
        let entry = unsafe {
            Entry::new()
                .map_err(|e| log::error!("Failed to create entry: {}", e))
                .unwrap()
        };
        let instance = Self::create_instance(&entry);
        let debug_messenger = Self::setup_debug_messenger(&entry, &instance);
        let (surface, surface_khr) = Self::create_surface(&entry, &instance, window);
        let (physical_device, properties) =
            Self::pick_physical_device(&instance, &surface, surface_khr);
        let (device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, &surface, surface_khr, physical_device);
        let command_pool =
            Self::create_command_pool(&instance, &surface, surface_khr, physical_device, &device);

        Rc::new(Self {
            _entry: entry,
            instance,
            debug_messenger,
            surface,
            surface_khr,
            physical_device,
            _properties: properties,
            device,
            graphics_queue,
            present_queue,
            command_pool,
        })
    }

    pub fn get_swapchain_support(&self) -> SwapChainSupportDetails {
        Self::query_swapchain_support(&self.surface, self.surface_khr, self.physical_device)
    }

    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        let mem_properties = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        let mut memory_type = None;

        for (index, m_type) in mem_properties.memory_types.iter().enumerate() {
            if (type_filter & (1 << index )) != 0 &&   // IDK if this is equivalent to c code
                (m_type.property_flags & properties) == properties
            {
                memory_type = Some(index as u32);
                break;
            }
        }

        memory_type
    }

    pub fn find_physical_queue_families(&self) -> QueueFamilyIndices {
        Self::find_queue_families(
            &self.instance,
            &self.surface,
            self.surface_khr,
            self.physical_device,
        )
    }

    pub fn find_supported_format(
        &self,
        candidates: &Vec<vk::Format>,
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> vk::Format {
        *candidates
            .iter()
            .find(|format| {
                let props = unsafe {
                    self.instance
                        .get_physical_device_format_properties(self.physical_device, **format)
                };

                if tiling == vk::ImageTiling::LINEAR {
                    return (props.linear_tiling_features & features) == features;
                } else if tiling == vk::ImageTiling::OPTIMAL {
                    return (props.optimal_tiling_features & features) == features;
                }
                false
            })
            .expect("failed to find supported format!")
    }

    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            self.device
                .create_buffer(&create_info, None)
                .map_err(|e| log::error!("Unable to create buffer: {}", e))
                .unwrap()
        };

        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                self.find_memory_type(mem_requirements.memory_type_bits, properties)
                    .unwrap(),
            );

        let buffer_memory = unsafe {
            self.device
                .allocate_memory(&alloc_info, None)
                .map_err(|e| log::error!("Unable to allocate memory: {}", e))
                .unwrap()
        };

        // Bind the memory to the buffer
        unsafe {
            self.device
                .bind_buffer_memory(buffer, buffer_memory, 0)
                .map_err(|e| log::error!("Unable to bind memory to buffer: {}", e))
                .unwrap()
        };

        (buffer, buffer_memory)
    }

    pub fn _begin_single_time_commands(&self) -> vk::CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| log::error!("Unable to allocate command buffer: {}", e))
                .unwrap()[0] // There is only 1 command buffer in the vec, so use that one
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        // Start the first (and only) command buffer
        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| log::error!("Unable to begin command buffer: {}", e))
                .unwrap()
        };

        command_buffer
    }

    pub fn _end_single_time_commands(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device
                .end_command_buffer(command_buffer)
                .map_err(|e| log::error!("Unable to end command buffer: {}", e))
                .unwrap()
        };

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(std::slice::from_ref(&command_buffer));

        unsafe {
            self.device
                .queue_submit(self.graphics_queue, std::slice::from_ref(&submit_info), vk::Fence::null())
                .map_err(|e| log::error!("Unable to submit queue: {}", e))
                .unwrap()
        };

        unsafe {
            self.device
                .queue_wait_idle(self.graphics_queue)
                .map_err(|e| log::error!("Unable to idle queue: {}", e))
                .unwrap()
        };

        unsafe {
            self.device
                .free_command_buffers(self.command_pool, &[command_buffer])
        };
    }

    pub fn _copy_buffer(
        &self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        let command_buffer = self._begin_single_time_commands();

        let copy_region = vk::BufferCopy::builder()
            .src_offset(0)
            .dst_offset(0)
            .size(size);

        unsafe {
            self.device
                .cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, std::slice::from_ref(&copy_region))
        };

        self._end_single_time_commands(command_buffer);
    }

    pub fn _copy_buffer_to_image(
        &self,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
        layer_count: u32,
    ) {
        let command_buffer = self._begin_single_time_commands();

        let image_subresource_info = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(layer_count)
            .build();

        let offset = vk::Offset3D::builder().x(0).y(0).z(0).build();

        let extent = vk::Extent3D::builder()
            .width(width)
            .height(height)
            .depth(1)
            .build();

        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(image_subresource_info)
            .image_offset(offset)
            .image_extent(extent);

        unsafe {
            self.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&region),
            )
        };

        self._end_single_time_commands(command_buffer);
    }

    pub fn create_image_with_info(
        &self,
        image_info: &vk::ImageCreateInfo,
        properties: vk::MemoryPropertyFlags,
    ) -> (vk::Image, vk::DeviceMemory) {
        let image = unsafe {
            self.device
                .create_image(image_info, None)
                .map_err(|e| log::error!("Unable to create image: {}", e))
                .unwrap()
        };

        let mem_requirements = unsafe { self.device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                self.find_memory_type(mem_requirements.memory_type_bits, properties)
                    .unwrap(),
            );

        let image_memory = unsafe {
            self.device
                .allocate_memory(&alloc_info, None)
                .map_err(|e| log::error!("Unable to allocate image memory: {}", e))
                .unwrap()
        };

        unsafe {
            self.device
                .bind_image_memory(image, image_memory, 0)
                .map_err(|e| log::error!("Unable to bind image memory: {}", e))
                .unwrap()
        };

        (image, image_memory)
    }

    fn create_instance(entry: &Entry) -> Instance {
        let app_name = CString::new("LittleVulkanEngine App").unwrap();
        let engine_name = CString::new("No Engine").unwrap();

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 2, 176));

        let extensions = Self::get_required_extensions();

        let mut create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_extension_names(&extensions);
        
        let (_layer_names, layer_name_ptrs) = Self::get_enabled_layers();

        if ENABLE_VALIDATION_LAYERS {
            Self::check_validation_layer_support(entry);
            create_info = create_info.enabled_layer_names(&layer_name_ptrs);
        }

        unsafe {
            entry
                .create_instance(&create_info, None)
                .map_err(|e| log::error!("Unable to create instance: {}", e))
                .unwrap()
        }
    }

    fn setup_debug_messenger(
        entry: &Entry,
        instance: &Instance,
    ) -> Option<(DebugUtils, vk::DebugUtilsMessengerEXT)> {
        if !ENABLE_VALIDATION_LAYERS {
            return None;
        }

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .flags(vk::DebugUtilsMessengerCreateFlagsEXT::all())
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_report = DebugUtils::new(entry, instance);
        let debug_report_callback = unsafe {
            debug_report
                .create_debug_utils_messenger(&create_info, None)
                .unwrap()
        };

        Some((debug_report, debug_report_callback))
    }

    fn create_surface(
        entry: &Entry,
        instance: &Instance,
        window: &Window,
    ) -> (Surface, vk::SurfaceKHR) {
        let surface = Surface::new(entry, instance);
        // Get window handler
        let surface_khr = unsafe {
            ash_window::create_surface(entry, instance, window, None)
                .map_err(|e| log::error!("Unable to create surface: {}", e))
                .unwrap()
        };

        (surface, surface_khr)
    }

    fn pick_physical_device(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
    ) -> (vk::PhysicalDevice, vk::PhysicalDeviceProperties) {
        // Get all of the GPUs connected to the PC
        let devices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(|e| log::error!("Failed to find GPUs with Vulkan Support: {}", e))
                .unwrap()
        };

        log::info!("Device Count: {}", devices.len());

        let device = devices
            .into_iter()
            .find(|device| Self::is_device_suitable(instance, surface, surface_khr, *device))
            .expect("No suitable physical device");

        let device_properties = unsafe { instance.get_physical_device_properties(device) };

        // Tell the user the name of the device
        log::info!("Selected physical device: {:?}", unsafe {
            CStr::from_ptr(device_properties.device_name.as_ptr())
        });

        (device, device_properties)
    }

    fn is_device_suitable(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> bool {
        let indices = Self::find_queue_families(instance, surface, surface_khr, device);

        let extensions_supported = Self::check_device_extension_support(instance, device);

        let mut swap_chain_adequate = false;

        if extensions_supported {
            let swap_chain_support = Self::query_swapchain_support(surface, surface_khr, device);
            swap_chain_adequate = {
                !swap_chain_support.formats.is_empty()
                    && !swap_chain_support.present_modes.is_empty()
            };
        }

        let supported_features = unsafe { instance.get_physical_device_features(device) };

        {
            indices.is_complete()
                && extensions_supported
                && swap_chain_adequate
                && supported_features.sampler_anisotropy != 0
        }
    }

    fn create_logical_device(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> (Device, vk::Queue, vk::Queue) {
        // Get the indices of the valid queue families
        let queue_indices =
            Self::find_queue_families(instance, surface, surface_khr, physical_device);

        // Give the queue a priority (only want one queue so we shall set it to 1.0)
        let queue_priorities = [1.0f32];

        // Set up all the information about the queues
        let queue_create_infos = {
            // Vulkan specs does not allow passing an array containing duplicated family indices.
            // And since the family for graphics and presentation could be the same we need to
            // deduplicate it.
            let mut indices = vec![queue_indices.graphics_family, queue_indices.present_family];
            indices.dedup();

            // Now we build an array of `DeviceQueueCreateInfo`.
            // One for each different family index.
            indices
                .iter()
                .map(|index| {
                    vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(*index)
                        .queue_priorities(&queue_priorities)
                        .build()
                })
                .collect::<Vec<_>>()
        };

        // Get the physical device features
        let physical_device_features = vk::PhysicalDeviceFeatures::builder().build();

        let (_, device_extensions_ptrs) = Self::get_device_extensions();

        let mut create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&device_extensions_ptrs);

        let (_layer_names, layer_name_ptrs) = Self::get_enabled_layers();

        if ENABLE_VALIDATION_LAYERS {
            create_info = create_info.enabled_layer_names(&layer_name_ptrs);
        }

        let device = unsafe {
            instance
                .create_device(physical_device, &create_info, None)
                .map_err(|e| log::error!("Unable to create logical device: {}", e))
                .unwrap()
        };

        // Allocate the queues
        let graphics_queue = unsafe { device.get_device_queue(queue_indices.graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(queue_indices.present_family, 0) };

        (device, graphics_queue, present_queue)
    }

    fn create_command_pool(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        device: &Device,
    ) -> vk::CommandPool {
        let queue_family_indices =
            Self::find_queue_families(instance, surface, surface_khr, physical_device);

        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_indices.graphics_family)
            .flags(
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT,
            );

        unsafe {
            device
                .create_command_pool(&create_info, None)
                .map_err(|e| log::error!("Unable to create command pool: {}", e))
                .unwrap()
        }
    }

    fn get_required_extensions() -> Vec<*const i8> {
        let mut extensions: Vec<*const i8> = Vec::new();

        extensions.push(Surface::name().as_ptr());

        #[cfg(target_os="windows")]
        extensions.push(Win32Surface::name().as_ptr());
        #[cfg(target_os="linux")]
        extensions.push(XlibSurface::name().as_ptr());

        if ENABLE_VALIDATION_LAYERS {
            extensions.push(DebugUtils::name().as_ptr());
        }

        log::info!("Number of required extensions: {}", extensions.len());

        extensions
    }

    fn check_validation_layer_support(entry: &Entry) {
        // Iterate through all the requested validation layers
        for required in VALIDATION_LAYERS.iter() {
            // Check if this required layer is in the Vulkan entry
            let found = entry
                .enumerate_instance_layer_properties()
                .unwrap()
                .iter()
                .any(|layer| {
                    let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                    let name = name
                        .to_str()
                        .map_err(|e| log::error!("Failed to get layer name pointer: {}", e))
                        .unwrap();
                    required == &name
                });

            // Throw an error if it is not found
            if !found {
                panic!("Validation layer not supported: {}", required);
            } else {
                log::debug!("Found required validation layers");
            }
        }
    }

    fn get_enabled_layers() -> (Vec<CString>, Vec<*const i8>) {
        // Store a list of all the validation layer names
        let layer_names = VALIDATION_LAYERS
            .iter()
            .map(|name| CString::new(*name).expect("Failed to Build CString"))
            .collect::<Vec<_>>();

        // Also store their pointers
        let layer_names_ptrs = layer_names
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();

        (layer_names, layer_names_ptrs)
    }

    fn get_device_extensions() -> ([&'static CStr; 1], Vec<*const i8>) {
        let device_extensions: [&'static CStr; 1] = [Swapchain::name()];

        // Store a list of all the device extensions pointers
        let ext_names_pts = device_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        (device_extensions, ext_names_pts)
    }

    fn find_queue_families(
        instance: &Instance,
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> QueueFamilyIndices {
        let mut graphics_family: u32 = 0;
        let mut present_family: u32 = 0;
        let mut graphics_family_has_value = false;
        let mut present_family_has_value = false;

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        for (index, queue_family) in queue_families
            .iter()
            .filter(|f| f.queue_count > 0)
            .enumerate()
        {
            let index = index as u32;

            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics_family = index;
                graphics_family_has_value = true;
            }

            let present_support = unsafe {
                surface
                    .get_physical_device_surface_support(device, index, surface_khr)
                    .unwrap()
            };

            if present_support {
                present_family = index;
                present_family_has_value = true;
            }

            if graphics_family_has_value && present_family_has_value {
                break;
            }
        }

        QueueFamilyIndices {
            graphics_family,
            present_family,
            graphics_family_has_value,
            present_family_has_value,
        }
    }

    fn check_device_extension_support(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
        };

        let (required_extensions, _) = Self::get_device_extensions();

        for extension in required_extensions.iter() {
            let found = available_extensions.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
                extension == &name
            });

            if !found {
                log::error!(
                    "Device does not support the following extension: {:?}",
                    extension
                );
                return false;
            }
        }

        true
    }

    fn query_swapchain_support(
        surface: &Surface,
        surface_khr: vk::SurfaceKHR,
        device: vk::PhysicalDevice,
    ) -> SwapChainSupportDetails {
        let capabilities = unsafe {
            surface
                .get_physical_device_surface_capabilities(device, surface_khr)
                .unwrap()
        };

        let formats = unsafe {
            surface
                .get_physical_device_surface_formats(device, surface_khr)
                .unwrap()
        };

        let present_modes = unsafe {
            surface
                .get_physical_device_surface_present_modes(device, surface_khr)
                .unwrap()
        };

        SwapChainSupportDetails {
            capabilities,
            formats,
            present_modes,
        }
    }
}

impl Drop for LveDevice {
    fn drop(&mut self) {
        log::debug!("Dropping device");
        unsafe {
            // log::debug!("Destroying command pool");
            self.device.destroy_command_pool(self.command_pool, None);
    
            // log::debug!("Destroying device");
            self.device.destroy_device(None);
    
            // log::debug!("Destroying surface");
            self.surface.destroy_surface(self.surface_khr, None);
    
            // log::debug!("Destroying debug messenger");
            // Destroy the Debug messenger
            if let Some((report, callback)) = self.debug_messenger.take() {
                report.destroy_debug_utils_messenger(callback, None);
            }
    
            // log::debug!("Destroying instance");
            self.instance.destroy_instance(None);
        }
    }
}
