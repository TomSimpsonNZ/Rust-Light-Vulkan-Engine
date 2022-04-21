mod fps_counter;
mod keyboard_movement_controller;
mod lve_buffer;
mod lve_camera;
mod lve_descriptors;
mod lve_device;
mod lve_frameinfo;
mod lve_game_object;
mod lve_model;
mod lve_pipeline;
mod lve_renderer;
mod lve_swapchain;
mod simple_render_system;

use fps_counter::FPSCounter;

use keyboard_movement_controller::*;
use lve_buffer::*;
use lve_camera::*;
use lve_descriptors::*;
use lve_device::*;
use lve_frameinfo::FrameInfo;
use lve_game_object::*;
use lve_model::*;
use lve_renderer::*;
use simple_render_system::*;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};

use std::time::Instant;

use std::{mem::size_of, rc::Rc};

use ash::vk;

extern crate nalgebra as na;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";

#[derive(Clone, Copy)]
struct GlobalUBO {
    _projection_view: na::Matrix4<f32>,
    _light_direction: na::Vector3<f32>,
}

pub struct VulkanApp {
    window: Window,
    lve_device: Rc<LveDevice>,
    lve_renderer: LveRenderer,
    global_pool: Rc<LveDescriptorPool>,
    game_objects: Vec<LveGameObject>,
    viewer_object: LveGameObject,
    camera_controller: KeyboardMovementController,
}

impl VulkanApp {
    pub fn new() -> (VulkanApp, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        let lve_device = LveDevice::new(&window);

        let lve_renderer = LveRenderer::new(Rc::clone(&lve_device), &window);

        let global_pool = LveDescriptorPoolBuilder::new(Rc::clone(&lve_device))
            .set_max_sets(lve_swapchain::MAX_FRAMES_IN_FLIGHT as u32)
            .add_pool_size(
                ash::vk::DescriptorType::UNIFORM_BUFFER,
                lve_swapchain::MAX_FRAMES_IN_FLIGHT as u32,
            )
            .build();

        let game_objects = Self::load_game_objects(&lve_device);

        let viewer_object = LveGameObject::new(LveModel::new_null("camera"), None, None);

        let camera_controller = KeyboardMovementController::new(None, None);

        (
            Self {
                window,
                lve_device,
                lve_renderer,
                global_pool,
                game_objects,
                viewer_object,
                camera_controller,
            },
            event_loop,
        )
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        let mut ubo_buffers: Vec<Rc<LveBuffer>> = Vec::new();

        for _ in 0..lve_swapchain::MAX_FRAMES_IN_FLIGHT {
            let mut ubo = lve_buffer::LveBuffer::new(
                Rc::clone(&self.lve_device),
                size_of::<GlobalUBO>() as u64,
                1,
                ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
                self.lve_device
                    .properties
                    .limits
                    .min_uniform_buffer_offset_alignment,
                BufferType::Uniform,
            );

            unsafe { ubo.map(ash::vk::WHOLE_SIZE, 0) };

            ubo_buffers.push(Rc::new(ubo));
        }

        let global_set_layout = LveDescriptorSetLayoutBuilder::new(Rc::clone(&self.lve_device))
            .add_binding(
                0,
                ash::vk::DescriptorType::UNIFORM_BUFFER,
                ash::vk::ShaderStageFlags::VERTEX,
                1,
            )
            .build();

        let mut global_descriptor_sets: Vec<vk::DescriptorSet> = Vec::new();

        for i in 0..lve_swapchain::MAX_FRAMES_IN_FLIGHT {
            let buffer_info = ubo_buffers[i].descriptor_info(vk::WHOLE_SIZE, 0);
            global_descriptor_sets.push(
                LveDescriptorWriter::new(
                    Rc::clone(&global_set_layout),
                    Rc::clone(&self.global_pool),
                )
                .write_buffer(0, &[*buffer_info])
                .build()
                .map_err(|_| log::error!("Unable to create a descriptor set!"))
                .unwrap(),
            )
        }

        let mut simple_render_system = SimpleRenderSystem::new(
            Rc::clone(&self.lve_device),
            &self.lve_renderer.get_swapchain_render_pass(),
            global_set_layout.descriptor_set_layout,
        );

        let mut current_time = Instant::now();

        let mut keys_pressed: Vec<VirtualKeyCode> = Vec::new();

        let mut fps_counter = FPSCounter::new(100);

        // Begin the events loop
        event_loop.run(move |event, _, control_flow| {
            // Set the behavior to poll the window for user events
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    log::debug!("Closing window");
                    *control_flow = ControlFlow::Exit
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(PhysicalSize { width, height }),
                    ..
                } => {
                    log::debug!("Resizing window");
                    log::info!("New window size: {}x{}", width, height);
                    self.resize();
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { input, .. },
                    ..
                } => {
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Escape) => {
                            log::debug!("Closing window");
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                        Some(input_key) => {
                            match input.state {
                                ElementState::Pressed => {
                                    if !keys_pressed.contains(&input_key) {
                                        keys_pressed.push(input_key);
                                    }
                                }
                                ElementState::Released => {
                                    let index = keys_pressed
                                        .iter()
                                        .position(|key| *key == input_key)
                                        .unwrap();
                                    keys_pressed.remove(index);
                                }
                            };
                        }
                        None => {}
                    };
                }
                Event::MainEventsCleared => {
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {
                    let time_since_last_frame = current_time.elapsed().as_secs_f32();
                    current_time = Instant::now();

                    // Code to run each frame goes here

                    self.camera_controller.move_in_plane_xz(
                        keys_pressed.as_slice(),
                        time_since_last_frame,
                        &mut self.viewer_object,
                    );

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

                            let frame_info = FrameInfo {
                                frame_index,
                                frame_time: time_since_last_frame,
                                command_buffer,
                                camera: &camera,
                                global_descriptor_set: global_descriptor_sets[frame_index as usize],
                            };

                            // Update
                            let ubo = GlobalUBO {
                                _projection_view: camera.projection_matrix * camera.view_matrix,
                                _light_direction: na::vector![1.0, -3.0, -1.0].normalize(),
                            };

                            unsafe {
                                ubo_buffers[frame_index as usize].write_to_buffer(
                                    &[ubo],
                                    ash::vk::WHOLE_SIZE,
                                    0,
                                );
                                ubo_buffers[frame_index as usize]
                                    .flush(ash::vk::WHOLE_SIZE, 0)
                                    .map_err(|e| log::error!("Unable to flush memory: {}", e))
                                    .unwrap();
                            }

                            // Render
                            self.lve_renderer
                                .begin_swapchain_render_pass(command_buffer);
                            simple_render_system
                                .render_game_objects(&frame_info, &mut self.game_objects);
                            self.lve_renderer.end_swapchain_render_pass(command_buffer);
                        }
                        None => {}
                    }

                    self.lve_renderer.end_frame();

                    let window_title = format!(
                        "HELLO VULAKN | fps: {}",
                        fps_counter.tick(time_since_last_frame)
                    );
                    self.window.set_title(&window_title);
                }
                _ => (),
            };
        });
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
