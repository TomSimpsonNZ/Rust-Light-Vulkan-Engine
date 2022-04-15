use super::lve_game_object::*;

use std::f32::consts::PI;
use std::f32::EPSILON;
use winit::event::VirtualKeyCode;

extern crate nalgebra as na;

pub struct KeyboardMovementController {
    move_speed: f32,
    look_speed: f32,
}

impl KeyboardMovementController {
    pub fn new(move_speed: Option<f32>, look_speed: Option<f32>) -> Self {
        let move_speed = match move_speed {
            Some(speed) => speed,
            None => 3.0,
        };

        let look_speed = match look_speed {
            Some(speed) => speed,
            None => 3.0,
        };
        Self {
            move_speed,
            look_speed,
        }
    }

    pub fn move_in_plane_xz(
        &self,
        key_codes: &[VirtualKeyCode],
        dt: f32,
        game_object: &mut LveGameObject,
    ) {
        let mut rotate = na::Vector3::<f32>::zeros();

        if key_codes.contains(&VirtualKeyCode::Right) {
            rotate[1] += 1.0
        } // look right
        if key_codes.contains(&VirtualKeyCode::Left) {
            rotate[1] -= 1.0
        } // look left
        if key_codes.contains(&VirtualKeyCode::Up) {
            rotate[0] += 1.0
        } // look up
        if key_codes.contains(&VirtualKeyCode::Down) {
            rotate[0] -= 1.0
        } // look down

        if rotate.dot(&rotate) > EPSILON {
            game_object.transform.rotation += self.look_speed * dt * rotate.normalize();
        }

        game_object.transform.rotation[0] = game_object.transform.rotation[0].clamp(-1.5, 1.5);
        game_object.transform.rotation[1] = game_object.transform.rotation[1] % (2.0 * PI);

        let yaw = game_object.transform.rotation[1];
        let forward_dir = na::vector![yaw.sin(), 0.0, yaw.cos()];
        let right_dir = na::vector![forward_dir[2], 0.0, -forward_dir[0]];
        let up_dir = na::vector![0.0, -1.0, 0.0];

        let mut move_dir = na::Vector3::<f32>::zeros();

        if key_codes.contains(&VirtualKeyCode::W) {
            move_dir += forward_dir
        } // move forward
        if key_codes.contains(&VirtualKeyCode::S) {
            move_dir -= forward_dir
        } // move backwards
        if key_codes.contains(&VirtualKeyCode::D) {
            move_dir += right_dir
        } // move right
        if key_codes.contains(&VirtualKeyCode::A) {
            move_dir -= right_dir
        } // move left
        if key_codes.contains(&VirtualKeyCode::E) {
            move_dir += up_dir
        } // move up
        if key_codes.contains(&VirtualKeyCode::Q) {
            move_dir -= up_dir
        } // move down

        if move_dir.dot(&move_dir) > EPSILON {
            game_object.transform.translation += self.move_speed * dt * move_dir.normalize();
        }
    }
}
