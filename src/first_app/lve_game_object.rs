use super::lve_model::*;

use std::rc::Rc;

extern crate nalgebra as na;

pub struct TransformComponent {
    pub translation: na::Vector3<f32>,
    pub scale: na::Vector3<f32>,
    pub rotation: na::Vector3<f32>,
}

impl TransformComponent {
    pub fn mat4(&self) -> na::Matrix4<f32> {

        let c3 = self.rotation[2].cos();
        let s3 = self.rotation[2].sin();
        let c2 = self.rotation[0].cos();
        let s2 = self.rotation[0].sin();
        let c1 = self.rotation[1].cos();
        let s1 = self.rotation[1].sin();

        na::matrix!(self.scale[0] * (c1 * c3 + s1 * s2 * s3), self.scale[1] * (c3 * s1 * s2 - c1 * s3), self.scale[2] * (c2 * s1), self.translation[0];
                    self.scale[0] * (c2 * s3)               , self.scale[1] * (c2 * c3)                , self.scale[2] * (-s2)    , self.translation[1];
                    self.scale[0] * (c1 * s2 * s3 - c3 * s1), self.scale[1] * (c1 * c3 * s2 + s1 * s3), self.scale[2] * (c1 * c2), self.translation[2];
                    0.0                                     , 0.0                                     , 0.0                      , 1.0;
                )
    }
}

pub struct LveGameObject {
    pub model: Rc<LveModel>,
    pub color: na::Vector3<f32>,
    pub transform: TransformComponent,
}

impl LveGameObject {
    pub fn new(
        model: Rc<LveModel>,
        color: Option<na::Vector3<f32>>,
        transform: Option<TransformComponent>,
    ) -> Self {
        let color = match color {
            Some(c) => c,
            None => na::vector![0.0, 0.0, 0.0],
        };

        let transform = match transform {
            Some(t) => t,
            None => TransformComponent {
                translation: na::vector![0.0, 0.0, 0.0],
                scale: na::vector![1.0, 1.0, 1.0],
                rotation: na::vector![0.0, 0.0, 0.0],
            }
        };

        Self {
            model,
            color,
            transform,
        }
    }
}
