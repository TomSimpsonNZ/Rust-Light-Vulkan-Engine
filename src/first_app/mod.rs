mod lve_device;
mod lve_game_object;
mod lve_model;
mod lve_pipeline;
mod lve_renderer;
mod lve_swapchain;
mod simple_render_system;

use lve_device::*;
use lve_game_object::*;
use lve_model::*;
use lve_renderer::*;
use simple_render_system::*;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use std::rc::Rc;
use std::{f32::consts::PI, str::FromStr};

extern crate nalgebra as na;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";

pub struct VulkanApp {
    window: Window,
    lve_renderer: LveRenderer,
    simple_render_system: SimpleRenderSystem,
    game_objects: Vec<LveGameObject>,
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

        (
            Self {
                window,
                lve_renderer,
                simple_render_system,
                game_objects,
            },
            event_loop,
        )
    }

    pub fn run(&mut self) {
        match self.lve_renderer.begin_frame(&self.window) {
            Some(command_buffer) => {
                self.lve_renderer
                    .begin_swapchain_render_pass(command_buffer);
                self.simple_render_system
                    .render_game_objects(command_buffer, &mut self.game_objects);
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
        let lve_model = Self::create_cube_model(lve_device, na::vector![0.0, 0.0, 0.0]);
        
        let transform = Some(TransformComponent {
            translation: na::vector![0.0, 0.0, 0.5],
            scale: na::vector![0.5, 0.5, 0.5],
            rotation: na::vector![0.0, 0.0, 0.0],
        });

        vec![LveGameObject::new(lve_model, None, transform)]
    }

    fn create_cube_model(lve_device: &Rc<LveDevice>, offset: na::Vector3<f32>) -> Rc<LveModel> {
        let mut vertices = vec![
            // left face (white)
            Vertex {
                position: na::vector![-0.5, -0.5, -0.5],
                color: na::vector![0.9, 0.9, 0.9],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, 0.5],
                color: na::vector![0.9, 0.9, 0.9],
            },
            Vertex {
                position: na::vector![-0.5, -0.5, 0.5],
                color: na::vector![0.9, 0.9, 0.9],
            },
            Vertex {
                position: na::vector![-0.5, -0.5, -0.5],
                color: na::vector![0.9, 0.9, 0.9],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, -0.5],
                color: na::vector![0.9, 0.9, 0.9],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, 0.5],
                color: na::vector![0.9, 0.9, 0.9],
            },
            // left face (yellow)
            Vertex {
                position: na::vector![0.5, -0.5, -0.5],
                color: na::vector![0.8, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, 0.5],
                color: na::vector![0.8, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, -0.5, 0.5],
                color: na::vector![0.8, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, -0.5, -0.5],
                color: na::vector![0.8, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, -0.5],
                color: na::vector![0.8, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, 0.5],
                color: na::vector![0.8, 0.8, 0.1],
            },
            // top face (orange)
            Vertex {
                position: na::vector![-0.5, -0.5, -0.5],
                color: na::vector![0.9, 0.6, 0.1],
            },
            Vertex {
                position: na::vector![0.5, -0.5, 0.5],
                color: na::vector![0.9, 0.6, 0.1],
            },
            Vertex {
                position: na::vector![-0.5, -0.5, 0.5],
                color: na::vector![0.9, 0.6, 0.1],
            },
            Vertex {
                position: na::vector![-0.5, -0.5, -0.5],
                color: na::vector![0.9, 0.6, 0.1],
            },
            Vertex {
                position: na::vector![0.5, -0.5, -0.5],
                color: na::vector![0.9, 0.6, 0.1],
            },
            Vertex {
                position: na::vector![0.5, -0.5, 0.5],
                color: na::vector![0.9, 0.6, 0.1],
            },
            // bottom face (red)
            Vertex {
                position: na::vector![-0.5, 0.5, -0.5],
                color: na::vector![0.8, 0.1, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, 0.5],
                color: na::vector![0.8, 0.1, 0.1],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, 0.5],
                color: na::vector![0.8, 0.1, 0.1],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, -0.5],
                color: na::vector![0.8, 0.1, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, -0.5],
                color: na::vector![0.8, 0.1, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, 0.5],
                color: na::vector![0.8, 0.1, 0.1],
            },
            // front face (blue)
            Vertex {
                position: na::vector![-0.5, -0.5, 0.5],
                color: na::vector![0.1, 0.1, 0.8],
            },
            Vertex {
                position: na::vector![0.5, 0.5, 0.5],
                color: na::vector![0.1, 0.1, 0.8],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, 0.5],
                color: na::vector![0.1, 0.1, 0.8],
            },
            Vertex {
                position: na::vector![-0.5, -0.5, 0.5],
                color: na::vector![0.1, 0.1, 0.8],
            },
            Vertex {
                position: na::vector![0.5, -0.5, 0.5],
                color: na::vector![0.1, 0.1, 0.8],
            },
            Vertex {
                position: na::vector![0.5, 0.5, 0.5],
                color: na::vector![0.1, 0.1, 0.8],
            },
            // front face (blue)
            Vertex {
                position: na::vector![-0.5, -0.5, -0.5],
                color: na::vector![0.1, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, -0.5],
                color: na::vector![0.1, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![-0.5, 0.5, -0.5],
                color: na::vector![0.1, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![-0.5, -0.5, -0.5],
                color: na::vector![0.1, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, -0.5, -0.5],
                color: na::vector![0.1, 0.8, 0.1],
            },
            Vertex {
                position: na::vector![0.5, 0.5, -0.5],
                color: na::vector![0.1, 0.8, 0.1],
            },
        ];

        for v in vertices.iter_mut() {
            v.position += offset;
        }

        LveModel::new(Rc::clone(lve_device), &vertices, "cube")
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application");
    }
}
