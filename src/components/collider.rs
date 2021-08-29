use specs::prelude::*;
use specs_derive::Component;

#[derive(Component, Debug)]
pub struct ColliderComponent
{
    pub radius: f32,
}