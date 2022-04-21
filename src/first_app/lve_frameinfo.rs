use super::lve_camera::LveCamera;
use super::lve_game_object::LveGameObject;

use std::collections::HashMap;

use ash::vk;

pub struct FrameInfo<'a> {
    pub frame_index: u64,
    pub frame_time: f32,
    pub command_buffer: vk::CommandBuffer,
    pub camera: &'a LveCamera,
    pub global_descriptor_set: vk::DescriptorSet,
    pub game_objects: &'a mut HashMap<u64, LveGameObject>
}
