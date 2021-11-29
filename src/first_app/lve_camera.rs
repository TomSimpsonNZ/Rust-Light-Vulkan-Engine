extern crate nalgebra as na;

use std::f32::EPSILON;

pub struct LveCameraBuilder {
    pub projection_matrix: na::Matrix4<f32>,
    pub view_matrix: na::Matrix4<f32>,
}

impl LveCameraBuilder {
    pub fn new() -> LveCameraBuilder {
        LveCameraBuilder {
            projection_matrix: na::Matrix4::identity(),
            view_matrix: na::Matrix4::identity(),
        }
    }

    #[allow(dead_code)]
    pub fn set_orthographic_projection<'a>(
        &'a mut self,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    ) -> &'a mut LveCameraBuilder {
        self.projection_matrix = na::matrix![
            2.0 / (right - left), 0.0                 , 0.0               , -(right + left) / (right - left);
            0.0                 , 2.0 / (bottom - top), 0.0               , -(bottom + top) / (bottom - top);
            0.0                 , 0.0                 , 1.0 / (far - near), -near / (far - near);
            0.0                 , 0.0                 , 0.0               , 1.0;
        ];

        self
    }

    #[allow(dead_code)]
    pub fn set_perspective_projection<'a>(
        &'a mut self,
        fovy: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> &'a mut Self {
        assert!((aspect - EPSILON).abs() > 0.0);

        let tan_half_fovy = (fovy / 2.0).tan();

        self.projection_matrix = na::matrix![
            1.0 / (aspect * tan_half_fovy), 0.0                  , 0.0               , 0.0;
            0.0                           , 1.0 / (tan_half_fovy), 0.0               , 0.0;
            0.0                           , 0.0                  , far / (far - near), -(far * near) / (far - near);
            0.0                           , 0.0                  , 1.0               , 0.0;
        ];

        self
    }

    #[allow(dead_code)]
    pub fn set_view_direction<'a>(
        &'a mut self,
        position: na::Vector3<f32>,
        direction: na::Vector3<f32>,
        up: Option<na::Vector3<f32>>,
    ) -> &'a mut LveCameraBuilder {
        let up = match up {
            Some(v) => v,
            None => na::vector![0.0, -1.0, 0.0],
        };

        let w = na::UnitVector3::new_normalize(direction);
        let u = na::UnitVector3::new_normalize(w.cross(&up));
        let v = w.cross(&u);

        self.view_matrix = na::matrix![
            u[0], u[1], u[2], -u.dot(&position);
            v[0], v[1], v[2], -v.dot(&position);
            w[0], w[1], w[2], -w.dot(&position);
            0.0 , 0.0 , 0.0 , 1.0;
        ];

        self
    }

    #[allow(dead_code)]
    pub fn set_view_target<'a>(
        &'a mut self,
        position: na::Vector3<f32>,
        target: na::Vector3<f32>,
        up: Option<na::Vector3<f32>>,
    ) -> &'a mut LveCameraBuilder {
        self.set_view_direction(position, target - position, up)
    }

    #[allow(dead_code)]
    pub fn set_view_xyz<'a>(
        &'a mut self,
        position: na::Vector3<f32>,
        rotation: na::Vector3<f32>,
    ) -> &'a mut LveCameraBuilder {
        let c3 = rotation[2].cos();
        let s3 = rotation[2].sin();
        let c2 = rotation[0].cos();
        let s2 = rotation[0].sin();
        let c1 = rotation[1].cos();
        let s1 = rotation[1].sin();

        let u = na::vector![
            (c1 * c3 + s1 * s2 * s3),
            (c2 * s3),
            (c1 * s2 * s3 - c3 * s1)
        ];
        let v = na::vector![
            (c3 * s1 * s2 - c1 * s3),
            (c2 * c3),
            (c1 * c3 * s2 + s1 * s3)
        ];
        let w = na::vector![(c2 * s1), (-s2), (c1 * c2)];

        self.view_matrix = na::matrix![
            u[0], u[1], u[2], -u.dot(&position);
            v[0], v[1], v[2], -v.dot(&position);
            w[0], w[1], w[2], -w.dot(&position);
            0.0 , 0.0 , 0.0 , 1.0;
        ];

        self
    }

    pub fn build(&self) -> LveCamera {
        LveCamera {
            projection_matrix: self.projection_matrix,
            view_matrix: self.view_matrix,
        }
    }
}

pub struct LveCamera {
    pub projection_matrix: na::Matrix4<f32>,
    pub view_matrix: na::Matrix4<f32>,
}
