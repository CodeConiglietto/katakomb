use std::f32::consts::PI;

use internship::IStr;
use ndarray::Array3;
use serde::{Deserialize, Serialize};

use crate::rendering::color::{self, Color};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Voxel3 {
    pub x: Voxel2,
    pub y: Voxel2,
    pub z: Voxel2,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Voxel2 {
    pub char_offset: u16,
    pub foreground: Color,
    pub background: Option<Color>,
    pub rotation: VoxelRotation,
    pub mirror: VoxelMirror,
}

impl Voxel2 {
    pub fn new(char_offset: u16) -> Self {
        Self {
            char_offset,
            foreground: color::WHITE,
            background: None,
            rotation: VoxelRotation::None,
            mirror: VoxelMirror::None,
        }
    }

    pub fn foreground(self, foreground: Color) -> Self {
        Self { foreground, ..self }
    }

    pub fn background(self, background: Option<Color>) -> Self {
        Self { background, ..self }
    }

    pub fn rotation(self, rotation: VoxelRotation) -> Self {
        Self { rotation, ..self }
    }

    pub fn mirror(self, mirror: VoxelMirror) -> Self {
        Self { mirror, ..self }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub voxels: Array3<Option<IStr>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum VoxelRotation {
    None,
    Rotation90,
    Rotation180,
    Rotation270,
}

impl VoxelRotation {
    pub fn into_rotation(&self) -> f32 {
        match self {
            VoxelRotation::None => 0.0,
            VoxelRotation::Rotation90 => 0.5 * PI,
            VoxelRotation::Rotation180 => PI,
            VoxelRotation::Rotation270 => 0.75 * PI,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum VoxelMirror {
    None,
    MirrorX,
    MirrorY,
    MirrorBoth,
}

impl VoxelMirror {
    pub fn into_scale(&self) -> mint::Vector2<f32> {
        match self {
            VoxelMirror::None => [1.0, 1.0],
            VoxelMirror::MirrorX => [-1.0, 1.0],
            VoxelMirror::MirrorY => [1.0, -1.0],
            VoxelMirror::MirrorBoth => [-1.0, -1.0],
        }
        .into()
    }
}
