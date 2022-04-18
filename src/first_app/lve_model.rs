use super::lve_buffer::*;
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
        let mut attribute_descriptions: Vec<vk::VertexInputAttributeDescription> = Vec::new();

        attribute_descriptions.push(vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: 0,
        });
        attribute_descriptions.push(vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: size_of::<Pos>() as u32,
        });
        attribute_descriptions.push(vk::VertexInputAttributeDescription {
            location: 2,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: (size_of::<Pos>() + size_of::<Color>()) as u32,
        });
        attribute_descriptions.push(vk::VertexInputAttributeDescription {
            location: 3,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: (size_of::<Pos>() + size_of::<Color>() + size_of::<Normal>()) as u32,
        });

        attribute_descriptions
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
                let colors = match &model.mesh.vertex_color.as_slice() {
                    [] => vec![1_f32; positions.len()],
                    v => v.to_vec(),
                };
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
                            color: na::vector![
                                OrderedFloat(colors[(3 * index + 0) as usize]),
                                OrderedFloat(colors[(3 * index + 1) as usize]),
                                OrderedFloat(colors[(3 * index + 2) as usize])
                            ],
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
    vertex_buffer: Option<Rc<LveBuffer>>,
    vertex_count: u32,
    index_buffer: Option<Rc<LveBuffer>>,
    index_count: u32,
    name: String,
}

impl LveModel {
    pub fn new(lve_device: Rc<LveDevice>, model_data: &ModelData, name: &str) -> Rc<Self> {
        let (vertex_buffer, vertex_count) =
            Self::create_vertex_buffers(&lve_device, &model_data.vertices);
        let (index_buffer, index_count) =
            Self::create_index_buffer(&lve_device, &model_data.indices);
        Rc::new(Self {
            vertex_buffer,
            vertex_count,
            index_buffer,
            index_count,
            name: String::from_str(name).unwrap(),
        })
    }

    pub fn new_null(name: &str) -> Rc<Self> {
        Rc::new(Self {
            vertex_buffer: None,
            vertex_count: 0,
            index_buffer: None,
            index_count: 0,
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
        match &self.index_buffer {
            Some(_) => device.cmd_draw_indexed(command_buffer, self.index_count, 1, 0, 0, 0),
            None => device.cmd_draw(command_buffer, self.vertex_count, 1, 0, 0),
        }
    }

    pub unsafe fn bind(&self, device: &Device, command_buffer: vk::CommandBuffer) {
        match &self.vertex_buffer {
            Some(vert_buff) => {
                let buffers = [vert_buff.buffer];
                let offsets = [0 as u64];
                device.cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);
            }
            None => {}
        }

        match &self.index_buffer {
            Some(ind_buff) => device.cmd_bind_index_buffer(
                command_buffer,
                ind_buff.buffer,
                0,
                vk::IndexType::UINT32,
            ),
            None => {}
        }
    }

    fn create_vertex_buffers(
        lve_device: &Rc<LveDevice>,
        vertices: &Vec<Vertex>,
    ) -> (Option<Rc<LveBuffer>>, u32) {
        let vertex_count = vertices.len();
        assert!(vertex_count >= 3, "Vertex count must be at least 3");

        let buffer_size: vk::DeviceSize = (size_of::<Vertex>() * vertex_count) as u64;

        let vertex_size: vk::DeviceSize = size_of::<Vertex>() as u64;

        let mut staging_buffer = LveBuffer::new(
            Rc::clone(lve_device),
            vertex_size,
            vertex_count as u32,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            1,
            BufferType::Staging,
        );

        unsafe {
            staging_buffer.map(vk::WHOLE_SIZE, 0);
            staging_buffer.write_to_buffer(vertices.as_slice(), vk::WHOLE_SIZE, 0);
        }

        let vertex_buffer = LveBuffer::new(
            Rc::clone(lve_device),
            vertex_size,
            vertex_count as u32,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            1,
            BufferType::Vertex
        );

        // Copy vertex data from the staging buffer to the local device memory
        lve_device.copy_buffer(staging_buffer.buffer, vertex_buffer.buffer, buffer_size);

        (Some(Rc::new(vertex_buffer)), vertex_count as u32)
    }

    fn create_index_buffer(
        lve_device: &Rc<LveDevice>,
        indices: &Option<Vec<u32>>,
    ) -> (Option<Rc<LveBuffer>>, u32) {
        let indices = match indices {
            Some(i) => i,
            None => return (None, 0),
        };

        let index_count = indices.len();

        let buffer_size: vk::DeviceSize = (size_of::<u32>() * index_count) as u64;

        let index_size: vk::DeviceSize = size_of::<u32>() as u64;

        let mut staging_buffer = LveBuffer::new(
            Rc::clone(lve_device),
            index_size,
            index_count as u32,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            1,
            BufferType::Staging,
        );

        unsafe {
            staging_buffer.map(vk::WHOLE_SIZE, 0);
            staging_buffer.write_to_buffer(indices.as_slice(), vk::WHOLE_SIZE, 0);
        }

        let index_buffer = LveBuffer::new(
            Rc::clone(lve_device),
            index_size,
            index_count as u32,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            1,
            BufferType::Index,
        );

        // Copy vertex data from the staging buffer to the local device memory
        lve_device.copy_buffer(staging_buffer.buffer, index_buffer.buffer, buffer_size);

        (Some(Rc::new(index_buffer)), index_count as u32)
    }
}

impl Drop for LveModel {
    fn drop(&mut self) {
        log::debug!("Dropping Model: {}", self.name);
    }
}
