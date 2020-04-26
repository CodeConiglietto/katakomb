use na::*;

pub struct Light {
    pub pos: Point3<f32>,
    pub facing: Point3<f32>,
    pub illumination: f32,
    pub range: f32,
}