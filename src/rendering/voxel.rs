use std::{
    f32::consts::PI,
    ops::{Index, IndexMut},
};

use internship::IStr;
use ndarray::Array3;
use serde::{Deserialize, Serialize};

use crate::rendering::color::{self, Color};

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Voxel3 {
    pub x: Voxel2,
    pub y: Voxel2,
    pub z: Voxel2,
}

impl Voxel3 {
    pub fn new(x: Voxel2, y: Voxel2, z: Voxel2) -> Self {
        Self { x, y, z }
    }

    pub fn face(&self, face: VoxelFace) -> &Voxel2 {
        match face {
            VoxelFace::X => &self.x,
            VoxelFace::Y => &self.y,
            VoxelFace::Z => &self.z,
        }
    }

    pub fn face_mut(&mut self, face: VoxelFace) -> &mut Voxel2 {
        match face {
            VoxelFace::X => &mut self.x,
            VoxelFace::Y => &mut self.y,
            VoxelFace::Z => &mut self.z,
        }
    }
}

impl Index<VoxelFace> for Voxel3 {
    type Output = Voxel2;

    fn index(&self, face: VoxelFace) -> &Voxel2 {
        self.face(face)
    }
}

impl IndexMut<VoxelFace> for Voxel3 {
    fn index_mut(&mut self, face: VoxelFace) -> &mut Voxel2 {
        self.face_mut(face)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum VoxelFace {
    X,
    Y,
    Z,
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

impl Default for Voxel2 {
    fn default() -> Self {
        Self::new(0)
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
