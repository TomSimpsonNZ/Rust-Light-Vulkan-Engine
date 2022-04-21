use super::lve_camera::LveCamera;

use ash::vk;

pub struct FrameInfo<'a> {
    pub frame_index: u64,
    pub frame_time: f32,
    pub command_buffer: vk::CommandBuffer,
    pub camera: &'a LveCamera,
    pub global_descriptor_set: vk::DescriptorSet,
}
