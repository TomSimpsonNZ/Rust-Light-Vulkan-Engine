extern crate nalgebra as na;

use std::f32::EPSILON;

pub struct LveCamera {
    pub projection_matrix: na::Matrix4<f32>,
}

impl LveCamera {
    pub fn _set_orthographic_projection(
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            projection_matrix: na::matrix![
            2.0 / (right - left), 0.0                 , 0.0               , -(right + left) / (right - left);
            0.0                 , 2.0 / (bottom - top), 0.0               , -(bottom + top) / (bottom - top);
            0.0                 , 0.0                 , 1.0 / (far - near), -near / (far - near);
            0.0                 , 0.0                 , 0.0               , 1.0;
        ]}
    }

    pub fn set_perspective_projection(fovy: f32, aspect: f32, near: f32, far: f32) -> Self {
        assert!((aspect - EPSILON).abs() > 0.0);

        let tan_half_fovy = (fovy / 2.0).tan();

        Self{ 
            projection_matrix: na::matrix![
            1.0 / (aspect * tan_half_fovy), 0.0                  , 0.0               , 0.0;
            0.0                           , 1.0 / (tan_half_fovy), 0.0               , 0.0;
            0.0                           , 0.0                  , far / (far - near), -(far * near) / (far - near);
            0.0                           , 0.0                  , 1.0               , 0.0;
        ]}
    }
}
