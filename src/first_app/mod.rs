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

use itertools::Itertools;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use std::{f32::consts::PI, str::FromStr};
use std::rc::Rc;

extern crate nalgebra as na;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const NAME: &str = "Hello Vulkan!";

struct GravityPhysicsSystem {
    gravity_strength: f32,
}

impl GravityPhysicsSystem {
    pub fn new(gravity_strength: f32) -> Self {
        Self { gravity_strength }
    }

    pub fn update(&self, game_objects: &mut Vec<LveGameObject>, dt: f32, substeps: Option<isize>) {
        let substeps = match substeps {
            Some(step) => step,
            None => 1,
        };

        let step_delta = dt / substeps as f32;

        for _ in 0..substeps {
            self.step_simulation(game_objects, step_delta);
        }
    }

    pub fn compute_force(
        &self,
        from_obj: &LveGameObject,
        to_obj: &LveGameObject,
    ) -> na::Vector2<f32> {
        let offset = from_obj.transform.translation - to_obj.transform.translation;

        let distance_squred = offset.dot(&offset);

        // clown town - wtf is this Brendan
        if distance_squred.abs() < 1e-10f32 {
            return na::vector![0.0, 0.0];
        }

        let force = self.gravity_strength * to_obj.rigid_body.mass * from_obj.rigid_body.mass
            / distance_squred;

        force * offset / distance_squred.sqrt()
    }

    fn step_simulation(&self, physics_objects: &mut Vec<LveGameObject>, dt: f32) {

        for i in 0..physics_objects.len() {
            physics_objects.rotate_left(i);
            let (obj_a, obj_bs) = physics_objects.split_at_mut(1);
            obj_bs.iter().for_each(|obj_b| {
                let force = self.compute_force(&obj_a[0], obj_b);
                obj_a[0].rigid_body.velocity += dt * -force / obj_a[0].rigid_body.mass;
            })
        }

        // Rotate one more time to get to original array 
        physics_objects.rotate_left(1);

        for obj in physics_objects.iter_mut() {
            obj.transform.translation += dt * obj.rigid_body.velocity;
        }
    }
}

struct Vec2FieldSystem {}

impl Vec2FieldSystem {
    pub fn update(
        physics_system: &GravityPhysicsSystem,
        physics_objects: &Vec<LveGameObject>,
        vector_fields: &mut Vec<LveGameObject>,
    ) {
        for vf in vector_fields.iter_mut() {
            let mut direction = na::vector![0.0, 0.0];

            physics_objects.iter().for_each(|obj| {
                direction += physics_system.compute_force(obj, vf);
            });

            vf.transform.scale[0] =
                0.005 + 0.045 * ((direction.magnitude() + 1.0).log(3.0) / 3.0).clamp(0.0, 1.0);
            vf.transform.rotation = direction[1].atan2(direction[0]);
        }
    }
}

fn create_square_model(lve_device: &LveDevice, offset: na::Vector2<f32>) -> LveModel {
    let mut vertices = vec![
        Vertex {
            position: na::vector![-0.5, -0.5],
            color: na::vector![0.0, 0.0, 0.0],
        },
        Vertex {
            position: na::vector![0.5, 0.5],
            color: na::vector![0.0, 0.0, 0.0],
        },
        Vertex {
            position: na::vector![-0.5, 0.5],
            color: na::vector![0.0, 0.0, 0.0],
        },
        Vertex {
            position: na::vector![-0.5, -0.5],
            color: na::vector![0.0, 0.0, 0.0],
        },
        Vertex {
            position: na::vector![0.5, -0.5],
            color: na::vector![0.0, 0.0, 0.0],
        },
        Vertex {
            position: na::vector![0.5, 0.5],
            color: na::vector![0.0, 0.0, 0.0],
        },
    ];

    for v in vertices.iter_mut() {
        v.position += offset;
    }

    LveModel::new(lve_device, &vertices)
}

fn create_circle_model(lve_device: &LveDevice, num_sides: usize) -> LveModel {
    let mut unique_vertices = Vec::new();
    for i in 0..num_sides {
        let angle = (i as f32) * 2.0 * PI / (num_sides as f32);

        unique_vertices.push(Vertex {
            position: na::vector![angle.cos(), angle.sin()],
            color: na::vector![0.0, 0.0, 0.0],
        });
    }

    unique_vertices.push(Vertex {
        position: na::vector![0.0, 0.0],
        color: na::vector![0.0, 0.0, 0.0],
    });

    let mut vertices = Vec::new();

    for i in 0..num_sides {
        vertices.push(unique_vertices[i]);
        vertices.push(unique_vertices[(i + 1) % num_sides]);
        vertices.push(unique_vertices[num_sides])
    }

    LveModel::new(lve_device, &vertices)
}

pub struct GravityVecFieldApp {
    window: Window,
    lve_renderer: LveRenderer,
    simple_render_system: SimpleRenderSystem,
    physics_objects: Vec<LveGameObject>,
    vector_field: Vec<LveGameObject>,
    gravity_system: GravityPhysicsSystem,
}

impl GravityVecFieldApp {
    pub fn new() -> (Self, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        let lve_device = LveDevice::new(&window);

        let lve_renderer = LveRenderer::new(Rc::clone(&lve_device), &window);

        let simple_render_system =
            SimpleRenderSystem::new(Rc::clone(&lve_device), &lve_renderer.get_swapchain_render_pass());

        let physics_objects = Self::load_physics_objects(&lve_device);

        let vector_field = Self::load_vec_field(&lve_device, 40);

        let gravity_system = GravityPhysicsSystem::new(0.81);

        (
            Self {
                window,
                lve_renderer,
                simple_render_system,
                physics_objects,
                vector_field,
                gravity_system,
            },
            event_loop,
        )
    }

    pub fn run(&mut self) {

        match self.lve_renderer
            .begin_frame(&self.window)
        {
            Some(command_buffer) => {

                // Update system
                self.gravity_system.update(&mut self.physics_objects, 1.0 / 120.0, Some(5));
                Vec2FieldSystem::update(&self.gravity_system, &self.physics_objects, &mut self.vector_field);

                // Render System
                self.lve_renderer
                    .begin_swapchain_render_pass(command_buffer);
                self.simple_render_system
                    .render_game_objects(
                    command_buffer,
                    &mut self.physics_objects,
                );
                self.simple_render_system.render_game_objects(
                    &self.lve_device.device,
                    command_buffer,
                    &mut self.vector_field,
                );
                self.lve_renderer
                    .end_swapchain_render_pass(command_buffer);
            }
            None => {}
        }

        self.lve_renderer
            .end_frame();
    }

    pub fn resize(&mut self) {
        self.lve_renderer
            .recreate_swapchain(&self.window)
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

    fn load_physics_objects(lve_device: &LveDevice) -> Vec<LveGameObject> {
        let circ_model = create_circle_model(lve_device, 64);

        let color = na::vector![1.0, 0.0, 0.0];
        let transform = Transform2DComponent {
            translation: na::vector![0.5, 0.5],
            scale: na::vector![0.05, 0.05],
            rotation: 0.0,
        };
        let rigid_body = RigidBody2DComponent {
            velocity: na::vector![-0.5, 0.0],
            mass: 1.0,
        };

        let red = LveGameObject::new(circ_model, color, transform, rigid_body);

        let color = na::vector![0.0, 0.0, 1.0];
        let transform = Transform2DComponent {
            translation: na::vector![-0.45, -0.25],
            scale: na::vector![0.05, 0.05],
            rotation: 0.0,
        };
        let rigid_body = RigidBody2DComponent {
            velocity: na::vector![0.5, 0.0],
            mass: 1.0,
        };

        let blue = LveGameObject::new(circ_model, color, transform, rigid_body);

        vec![red, blue]
    }

    fn load_vec_field(lve_device: &LveDevice, grid_count: u32) -> Vec<LveGameObject> {
        let mut vector_field = Vec::new();

        for i in 0..grid_count {
            for j in 0..grid_count {
                let square_model = create_square_model(lve_device, na::vector![0.5, 0.0]);
                let color = na::vector![0.9, 0.9, 0.9];
                let transform = Transform2DComponent {
                    translation: na::vector![-1.0 + (i as f32 + 0.5) * 2.0 / (grid_count as f32), -1.0 + (j as f32 + 0.5) * 2.0 / (grid_count as f32)],
                    scale: na::vector![0.005, 0.005],
                    rotation: 0.0,
                };
                let rigid_body = RigidBody2DComponent {
                    velocity: na::vector![0.0, 0.0],
                    mass: 1.0,
                };
                vector_field.push(LveGameObject::new(square_model, color, transform, rigid_body));
            }
        }

        vector_field
    }
}

impl Drop for GravityVecFieldApp {
    fn drop(&mut self) {
        log::debug!("Dropping application");
    }
}
