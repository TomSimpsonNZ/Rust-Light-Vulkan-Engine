use super::lve_device::*;
use super::lve_frameinfo::FrameInfo;
use super::lve_pipeline::*;

use ash::{vk, Device};

use std::rc::Rc;

extern crate nalgebra as na;

#[repr(align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Align16<T>(pub T);

type Mat4 = Align16<na::Matrix4<f32>>;

#[derive(Debug)]
pub struct SimplePushConstantData {
    _model_matrix: Mat4,
    _normal_matrix: Mat4,
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
    pub fn new(
        lve_device: Rc<LveDevice>,
        render_pass: &vk::RenderPass,
        global_set_layout: vk::DescriptorSetLayout
    ) -> Self {
        let pipeline_layout = Self::create_pipeline_layout(&lve_device.device, global_set_layout);

        let lve_pipeline =
            Self::create_pipeline(Rc::clone(&lve_device), render_pass, &pipeline_layout);

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

    fn create_pipeline_layout(
        device: &Device,
        global_set_layout: vk::DescriptorSetLayout
    ) -> vk::PipelineLayout {
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<SimplePushConstantData>() as u32)
            .build();

        let descriptor_set_layouts = vec![global_set_layout];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(descriptor_set_layouts.as_slice())
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
        frame_info: &mut FrameInfo,
    ) {
        unsafe {
            self.lve_pipeline
                .bind(&self.lve_device.device, frame_info.command_buffer);

            self.lve_device.device.cmd_bind_descriptor_sets(
                frame_info.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[frame_info.global_descriptor_set],
                &[],
            );
        };

        for (_, game_obj) in frame_info.game_objects.iter_mut() {
            let push = SimplePushConstantData {
                _model_matrix: Align16(game_obj.transform.mat4()),
                _normal_matrix: Align16(game_obj.transform.normal_matrix()),
            };

            unsafe {
                let push_ptr = push.as_bytes();

                self.lve_device.device.cmd_push_constants(
                    frame_info.command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    push_ptr,
                );

                game_obj
                    .model
                    .bind(&self.lve_device.device, frame_info.command_buffer);
                game_obj
                    .model
                    .draw(&self.lve_device.device, frame_info.command_buffer);
            }
        }
    }
}

impl Drop for SimpleRenderSystem {
    fn drop(&mut self) {
        log::debug!("Dropping SimpleRenderSystem");

        unsafe {
            self.lve_device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
