use specs::prelude::*;
use specs_derive::Component;
use ggez::nalgebra::Vector3;

#[derive(Component, Debug)]
pub struct VelocityComponent
{
    pub value: Vector3<f32>,
}