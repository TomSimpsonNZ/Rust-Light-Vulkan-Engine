use super::lve_model::*;

use std::rc::Rc;

extern crate nalgebra as na;

pub struct Transform2DComponent {
    pub translation: na::Vector2<f32>,
    pub scale: na::Vector2<f32>,
    pub rotation: f32,
}

impl Transform2DComponent {
    pub fn mat2(&self) -> na::Matrix2<f32> {

        let scale_matrix = na::matrix![self.scale[0], 0.0; 
                                                                0.0          , self.scale[1]];

        let s = self.rotation.sin();
        let c = self.rotation.cos();

        let rot_matrix = na::matrix![c, -s;
                                                              s,  c];

        rot_matrix * scale_matrix
    }
}

pub struct LveGameObject {
    pub model: Rc<LveModel>,
    pub color: na::Vector3<f32>,
    pub transform: Transform2DComponent,
}

impl LveGameObject {
    pub fn new(
        model: Rc<LveModel>,
        color: na::Vector3<f32>,
        transform: Transform2DComponent,
    ) -> Self {
        Self {
            model,
            color,
            transform,
        }
    }
}
