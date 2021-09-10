use na::Point3;
use specs::prelude::*;
use specs_derive::Component;

#[derive(Component, Debug)]
pub struct PositionComponent {
    pub value: Point3<f32>,
}
