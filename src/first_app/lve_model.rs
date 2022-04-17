use super::lve_device::*;

use ash::{vk, Device};

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::rc::Rc;
use std::str::FromStr;

use ordered_float::OrderedFloat;

extern crate nalgebra as na;

type Hf32 = OrderedFloat<f32>;

type Pos = na::Vector3<Hf32>;
type Color = na::Vector3<Hf32>;
type Normal = na::Vector3<Hf32>;
type TextureCoord = na::Vector2<Hf32>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Vertex {
    pub position: Pos,
    pub color: Color,
    pub normal: Normal,
    pub uv: TextureCoord,
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
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(size_of::<Pos>() as u32) // Using size of the position field
                .build(),
        ]
    }
}

pub struct ModelData {
    pub vertices: Vec<Vertex>,
    pub indices: Option<Vec<u32>>,
}

impl ModelData {
    pub fn load_model(file_path: &str) -> (Self, Vec<String>) {
        let model_file = tobj::load_obj(file_path, &tobj::GPU_LOAD_OPTIONS);
        let (models, _materials) = model_file
            .map_err(|e| log::error!("Unable to load model: {}", e))
            .unwrap();

        // Stores the hash of the vertex as the key, and the index of the unique vertex
        let mut unique_vertices: HashMap<usize, u32> = HashMap::new();
        let mut unique_ind: u32 = 0;

        let mut indices: Vec<u32> = Vec::new();

        let vertices = models
            .iter()
            .map(|model| {
                let positions = &model.mesh.positions;
                let colors = &model.mesh.vertex_color;
                let normals = &model.mesh.normals;
                let uvs = &model.mesh.texcoords;
                model
                    .mesh
                    .indices
                    .iter()
                    .filter_map(|index| {
                        let vertex = Vertex {
                            position: na::vector![
                                OrderedFloat(positions[(3 * index + 0) as usize]),
                                OrderedFloat(positions[(3 * index + 1) as usize]),
                                OrderedFloat(positions[(3 * index + 2) as usize])
                            ],
                            color: match colors.as_slice() {
                                [] => {
                                    na::vector![
                                        OrderedFloat(1.0),
                                        OrderedFloat(1.0),
                                        OrderedFloat(1.0)
                                    ]
                                }
                                c => na::vector![
                                    OrderedFloat(c[(3 * index + 0) as usize]),
                                    OrderedFloat(c[(3 * index + 1) as usize]),
                                    OrderedFloat(c[(3 * index + 2) as usize])
                                ],
                            },
                            normal: na::vector![
                                OrderedFloat(normals[(3 * index + 0) as usize]),
                                OrderedFloat(normals[(3 * index + 1) as usize]),
                                OrderedFloat(normals[(3 * index + 2) as usize])
                            ],
                            uv: na::vector![
                                OrderedFloat(uvs[(2 * index + 0) as usize]),
                                OrderedFloat(uvs[(2 * index + 1) as usize])
                            ],
                        };

                        let mut hasher = DefaultHasher::new();

                        vertex.hash(&mut hasher);
                        let hash = hasher.finish() as usize;

                        if !unique_vertices.contains_key(&hash) {
                            unique_vertices.insert(hash, unique_ind);
                            unique_ind += 1;
                            // Will never panic as we have already checked that the hashmap contains the vertex
                            indices.push(*unique_vertices.get(&hash).unwrap());
                            return Some(vertex);
                        } else {
                            indices.push(*unique_vertices.get(&hash).unwrap());
                            return None;
                        }
                    })
                    .collect::<Vec<Vertex>>()
            })
            .flatten()
            .collect::<Vec<Vertex>>();

        let mut names = Vec::new();

        for model in models {
            names.push(model.name)
        }

        (
            Self {
                vertices,
                indices: Some(indices),
            },
            names,
        )
    }
}

pub struct LveModel {
    lve_device: Rc<LveDevice>,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    vertex_count: u32,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    index_count: u32,
    has_index_buffer: bool,
    name: String,
}

impl LveModel {
    pub fn new(lve_device: Rc<LveDevice>, model_data: &ModelData, name: &str) -> Rc<Self> {
        let (vertex_buffer, vertex_buffer_memory, vertex_count) =
            Self::create_vertex_buffers(&lve_device, &model_data.vertices);
        let (index_buffer, index_buffer_memory, index_count, has_index_buffer) =
            Self::create_index_buffer(&lve_device, &model_data.indices);
        Rc::new(Self {
            lve_device,
            vertex_buffer,
            vertex_buffer_memory,
            vertex_count,
            index_buffer,
            index_buffer_memory,
            index_count,
            has_index_buffer,
            name: String::from_str(name).unwrap(),
        })
    }

    pub fn new_null(lve_device: Rc<LveDevice>, name: &str) -> Rc<Self> {
        Rc::new(Self {
            lve_device,
            vertex_buffer: vk::Buffer::null(),
            vertex_buffer_memory: vk::DeviceMemory::null(),
            vertex_count: 0,
            index_buffer: vk::Buffer::null(),
            index_buffer_memory: vk::DeviceMemory::null(),
            index_count: 0,
            has_index_buffer: false,
            name: String::from_str(name).unwrap(),
        })
    }

    pub fn create_model_from_file(lve_device: Rc<LveDevice>, file_path: &str) -> Rc<Self> {
        let (model_data, names) = ModelData::load_model(file_path);
        log::info!("Model Name: {}", names[0]);
        log::info!("Vertex count: {}", model_data.vertices.len());
        Self::new(lve_device, &model_data, &names[0])
    }

    pub unsafe fn draw(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        if self.has_index_buffer {
            device.cmd_draw_indexed(command_buffer, self.index_count, 1, 0, 0, 0)
        } else {
            device.cmd_draw(command_buffer, self.vertex_count, 1, 0, 0);
        }
    }

    pub unsafe fn bind(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        let buffers = [self.vertex_buffer];
        let offsets = [0 as u64];

        device.cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);

        if self.has_index_buffer {
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer,
                0,
                vk::IndexType::UINT32,
            )
        }
    }

    fn create_vertex_buffers(
        lve_device: &Rc<LveDevice>,
        vertices: &Vec<Vertex>,
    ) -> (vk::Buffer, vk::DeviceMemory, u32) {
        let vertex_count = vertices.len();
        assert!(vertex_count >= 3, "Vertex count must be at least 3");

        let buffer_size: vk::DeviceSize = (size_of::<Vertex>() * vertex_count) as u64;

        let (staging_buffer, staging_buffer_memory) = lve_device.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT, // Accessible from the host (cpu)
        );

        unsafe {
            let data_ptr = lve_device
                .device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(|e| log::error!("Unable to map vertex buffer memory: {}", e))
                .unwrap();

            let mut align =
                ash::util::Align::new(data_ptr, std::mem::align_of::<u32>() as u64, buffer_size);

            align.copy_from_slice(vertices);

            lve_device.device.unmap_memory(staging_buffer_memory);
        };

        let (vertex_buffer, vertex_buffer_memory) = lve_device.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        // Copy vertex data from the staging buffer to the local device memory
        lve_device.copy_buffer(staging_buffer, vertex_buffer, buffer_size);

        // Destroy the staging buffer as it is no longer needed
        unsafe {
            lve_device.device.destroy_buffer(staging_buffer, None);
            lve_device.device.free_memory(staging_buffer_memory, None);
        }

        (vertex_buffer, vertex_buffer_memory, vertex_count as u32)
    }

    fn create_index_buffer(
        lve_device: &Rc<LveDevice>,
        indices: &Option<Vec<u32>>,
    ) -> (vk::Buffer, vk::DeviceMemory, u32, bool) {
        let indices = match indices {
            Some(i) => i,
            None => return (vk::Buffer::null(), vk::DeviceMemory::null(), 0, false),
        };

        let index_count = indices.len();

        let buffer_size: vk::DeviceSize = (size_of::<u32>() * index_count) as u64;

        let (staging_buffer, staging_buffer_memory) = lve_device.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT, // Accessible from the host (cpu)
        );

        unsafe {
            let data_ptr = lve_device
                .device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(|e| log::error!("Unable to map vertex buffer memory: {}", e))
                .unwrap();

            let mut align =
                ash::util::Align::new(data_ptr, std::mem::align_of::<u32>() as u64, buffer_size);

            align.copy_from_slice(indices);

            lve_device.device.unmap_memory(staging_buffer_memory);
        };

        let (index_buffer, index_buffer_memory) = lve_device.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        // Copy vertex data from the staging buffer to the local device memory
        lve_device.copy_buffer(staging_buffer, index_buffer, buffer_size);

        // Destroy the staging buffer as it is no longer needed
        unsafe {
            lve_device.device.destroy_buffer(staging_buffer, None);
            lve_device.device.free_memory(staging_buffer_memory, None);
        }

        (index_buffer, index_buffer_memory, index_count as u32, true)
    }
}

impl Drop for LveModel {
    fn drop(&mut self) {
        log::debug!("Dropping Model: {}", self.name);
        unsafe {
            self.lve_device
                .device
                .destroy_buffer(self.vertex_buffer, None);
            self.lve_device
                .device
                .free_memory(self.vertex_buffer_memory, None);

            if self.has_index_buffer {
                self.lve_device
                    .device
                    .destroy_buffer(self.index_buffer, None);
                self.lve_device
                    .device
                    .free_memory(self.index_buffer_memory, None);
            }
        }
    }
}
