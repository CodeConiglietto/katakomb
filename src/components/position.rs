use specs::prelude::*;
use specs_derive::Component;
use ggez::nalgebra::Point3;

#[derive(Component, Debug)]
pub struct PositionComponent
{
    pub value: Point3<f32>,
}