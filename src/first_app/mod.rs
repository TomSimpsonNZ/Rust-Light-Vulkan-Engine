mod keyboard_movement_controller;
mod lve_buffer;
mod lve_camera;
mod lve_device;
mod lve_game_object;
mod lve_model;
mod lve_pipeline;
mod lve_renderer;
mod lve_swapchain;
mod lve_frameinfo;
mod simple_render_system;

use keyboard_movement_controller::*;
use lve_buffer::*;
use lve_camera::*;
use lve_device::*;
use lve_game_object::*;
use lve_model::*;
use lve_renderer::*;
use lve_frameinfo::FrameInfo;
use simple_render_system::*;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use winit::event::VirtualKeyCode;

use std::{mem::size_of, rc::Rc};

extern crate nalgebra as na;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";

#[derive(Clone, Copy)]
struct GlobalUBO {
    projection_view: na::Matrix4<f32>,
    light_direction: na::Vector3<f32>,
}

pub struct VulkanApp {
    pub window: Window,
    lve_renderer: LveRenderer,
    simple_render_system: SimpleRenderSystem,
    game_objects: Vec<LveGameObject>,
    viewer_object: LveGameObject,
    camera_controller: KeyboardMovementController,
    global_ubo_buffer: Rc<LveBuffer>,
}

impl VulkanApp {
    pub fn new() -> (Self, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        let lve_device = LveDevice::new(&window);

        let lve_renderer = LveRenderer::new(Rc::clone(&lve_device), &window);

        let simple_render_system = SimpleRenderSystem::new(
            Rc::clone(&lve_device),
            &lve_renderer.get_swapchain_render_pass(),
        );

        let game_objects = Self::load_game_objects(&lve_device);

        let viewer_object = LveGameObject::new(LveModel::new_null("camera"), None, None);

        let camera_controller = KeyboardMovementController::new(None, None);

        let mut global_ubo_buffer = lve_buffer::LveBuffer::new(
            Rc::clone(&lve_device),
            size_of::<GlobalUBO>() as u64,
            lve_swapchain::MAX_FRAMES_IN_FLIGHT as u32,
            ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
            lve_device
                .properties
                .limits
                .min_uniform_buffer_offset_alignment,
            BufferType::Uniform,
        );
    
        unsafe { global_ubo_buffer.map(ash::vk::WHOLE_SIZE, 0) };

        (
            Self {
                window,
                lve_renderer,
                simple_render_system,
                game_objects,
                viewer_object,
                camera_controller,
                global_ubo_buffer: Rc::new(global_ubo_buffer),
            },
            event_loop,
        )
    }

    pub fn run(&mut self, keys_pressed: &[VirtualKeyCode], frame_time: f32) {
        // log::debug!("frame time: {}s", frame_time);
        // log::debug!("Keys pressed: {:?}", keys_pressed);
        // log::debug!("fps: {:?}", 1.0/frame_time); // This is a bit shit :)

        self.camera_controller
            .move_in_plane_xz(keys_pressed, frame_time, &mut self.viewer_object);

        let aspect = self.lve_renderer.get_aspect_ratio();
        // self.camera = LveCamera::set_orthographic_projection(-aspect, aspect, -1.0, 1.0, -1.0, 1.0);
        let camera = LveCameraBuilder::new()
            .set_view_xyz(
                self.viewer_object.transform.translation,
                self.viewer_object.transform.rotation,
            )
            .set_perspective_projection(50_f32.to_radians(), aspect, 0.1, 10.0)
            // .set_view_direction(na::Vector3::zeros(), na::vector![0.5, 0.0, 1.0], None)
            // .set_view_target(
            //     na::vector![-1.0, -2.0, 2.0],
            //     na::vector![0.0, 0.0, 2.5],
            //     None,
            // )
            .build();

        let extent = LveRenderer::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return; // Don't do anything if the window is minimised
        }

        match self.lve_renderer.begin_frame(&self.window) {
            Some(command_buffer) => {
                let frame_index = self.lve_renderer.get_frame_index() as u64;

                let frame_info = FrameInfo{
                    frame_index,
                    frame_time,
                    command_buffer,
                    camera: &camera,
                };

                // Update
                let ubo = GlobalUBO {
                    projection_view: camera.projection_matrix * camera.view_matrix,
                    light_direction: na::vector![1.0, -3.0, -1.0].normalize(),
                };

                unsafe {
                    self.global_ubo_buffer.write_to_index(&[ubo], frame_index);
                    self.global_ubo_buffer
                        .flush_index(frame_index)
                        .map_err(|e| log::error!("Unable to flush memory: {}", e))
                        .unwrap();
                }

                // Render
                self.lve_renderer
                    .begin_swapchain_render_pass(command_buffer);
                self.simple_render_system.render_game_objects(
                    &frame_info,
                    &mut self.game_objects,
                );
                self.lve_renderer.end_swapchain_render_pass(command_buffer);
            }
            None => {}
        }

        self.lve_renderer.end_frame();
    }

    pub fn resize(&mut self) {
        self.lve_renderer.recreate_swapchain(&self.window)
    }

    fn new_window(w: u32, h: u32, name: &str) -> (EventLoop<()>, Window) {
        log::debug!("Starting event loop");
        let event_loop = EventLoop::new();

        log::debug!("Creating window");
        let winit_window = WindowBuilder::new()
            .with_title(name)
            .with_inner_size(LogicalSize::new(w, h))
            .with_resizable(true)
            .build(&event_loop)
            .unwrap();

        (event_loop, winit_window)
    }

    fn load_game_objects(lve_device: &Rc<LveDevice>) -> Vec<LveGameObject> {
        let mut game_objects: Vec<LveGameObject> = Vec::new();

        let smooth_vase =
            LveModel::create_model_from_file(Rc::clone(lve_device), "models/smooth_vase.obj");

        let transform = Some(TransformComponent {
            translation: na::vector![-0.5, 0.5, 2.5],
            scale: na::vector![3.0, 1.5, 3.0],
            rotation: na::vector![0.0, 0.0, 0.0],
        });

        game_objects.push(LveGameObject::new(smooth_vase, None, transform));

        let flat_vase =
            LveModel::create_model_from_file(Rc::clone(lve_device), "models/flat_vase.obj");

        let transform = Some(TransformComponent {
            translation: na::vector![0.5, 0.5, 2.5],
            scale: na::vector![3.0, 3.0, 3.0],
            rotation: na::vector![0.0, 0.0, 0.0],
        });

        game_objects.push(LveGameObject::new(flat_vase, None, transform));

        game_objects
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application");
    }
}
