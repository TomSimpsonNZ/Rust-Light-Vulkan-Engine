use super::lve_device::*;
use super::lve_game_object::*;
use super::lve_pipeline::*;

use ash::version::DeviceV1_0;
use ash::{vk, Device};

use std::f32::consts::PI;
use std::rc::Rc;

extern crate nalgebra as na;

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

pub struct SimpleRenderSystem {
    lve_device: Rc<LveDevice>,
    lve_pipeline: LvePipeline,
    pipeline_layout: vk::PipelineLayout, // I think this should be a part of the pipeline module
}

impl SimpleRenderSystem {
    pub fn new(lve_device: Rc<LveDevice>, render_pass: &vk::RenderPass) -> Self {
        let pipeline_layout = Self::create_pipeline_layout(&lve_device.device);

        let lve_pipeline = Self::create_pipeline(Rc::clone(&lve_device), render_pass, &pipeline_layout);

        Self {
            lve_device,
            lve_pipeline,
            pipeline_layout,
        }
    }

    fn create_pipeline(
        lve_device: Rc<LveDevice>,
        render_pass: &vk::RenderPass,
        pipeline_layout: &vk::PipelineLayout,
    ) -> LvePipeline {
        assert!(
            pipeline_layout != &vk::PipelineLayout::null(),
            "Cannot create pipeline before pipeline layout"
        );

        let pipeline_config = LvePipeline::default_pipline_config_info();

        LvePipeline::new(
            lve_device,
            "shaders/simple_shader.vert.spv",
            "shaders/simple_shader.frag.spv",
            pipeline_config,
            render_pass,
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

    pub fn render_game_objects(
        &mut self,
        command_buffer: vk::CommandBuffer,
        game_objects: &mut Vec<LveGameObject>,
    ) {
        unsafe { self.lve_pipeline.bind(&self.lve_device.device, command_buffer) };

        for game_obj in game_objects.iter_mut() {
            game_obj.transform.rotation = game_obj.transform.rotation + 0.01 % 2.0 * PI;

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

impl Drop for SimpleRenderSystem {
    fn drop(&mut self) {
        log::debug!("Dropping SimpleRenderSystem");

        unsafe {
            self.lve_device.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
