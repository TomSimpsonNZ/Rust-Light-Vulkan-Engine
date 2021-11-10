use std::mem::size_of;

use super::lve_device::*;

use ash::{vk, Device};

use ash::version::DeviceV1_0;

extern crate nalgebra as na;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: na::Vector2<f32>,
    pub color: na::Vector3<f32>,
}

impl Vertex {
    pub fn get_binding_descriptions() -> Vec<vk::VertexInputBindingDescription> {
        let vertex_size = size_of::<Vertex>() as u32;

        vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(vertex_size)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()]
    }

    pub fn get_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(size_of::<na::Vector2<f32>>() as u32) // Using size of the position field
                .build(),
        ]
    }
}

pub struct LveModel {
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    vertex_count: u32,
}

impl LveModel {
    pub fn new(lve_device: &LveDevice, vertices: &Vec<Vertex>) -> Self {
        let (vertex_buffer, vertex_buffer_memory, vertex_count) =
            Self::create_vertex_buffers(lve_device, vertices);

        Self {
            vertex_buffer,
            vertex_buffer_memory,
            vertex_count,
        }
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_buffer(self.vertex_buffer, None);
        device.free_memory(self.vertex_buffer_memory, None);
    }

    pub unsafe fn draw(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        device.cmd_draw(command_buffer, self.vertex_count, 1, 0, 0);
    }

    pub unsafe fn bind(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        let buffers = [self.vertex_buffer];
        let offsets = [0 as u64];

        device.cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);
    }

    fn create_vertex_buffers(
        lve_device: &LveDevice,
        vertices: &Vec<Vertex>,
    ) -> (vk::Buffer, vk::DeviceMemory, u32) {
        let vertex_count = vertices.len();
        assert!(vertex_count >= 3, "Vertex count must be at least 3");

        let buffer_size: vk::DeviceSize = (size_of::<Vertex>() * vertex_count) as u64;

        let (vertex_buffer, vertex_buffer_memory) = lve_device.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT, // Accessible from the host (cpu)
        );

        unsafe {
            let data_ptr = lve_device
                .device
                .map_memory(
                    vertex_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(|e| log::error!("Unable to map vertex buffer memory: {}", e))
                .unwrap();

            let mut align =
                ash::util::Align::new(data_ptr, std::mem::align_of::<u32>() as u64, buffer_size);

            align.copy_from_slice(vertices);

            lve_device.device.unmap_memory(vertex_buffer_memory);
        };

        (vertex_buffer, vertex_buffer_memory, vertex_count as u32)
    }
}
