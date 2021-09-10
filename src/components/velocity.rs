use na::Vector3;
use specs::prelude::*;
use specs_derive::Component;

#[derive(Component, Debug)]
pub struct VelocityComponent {
    pub value: Vector3<f32>,
}
