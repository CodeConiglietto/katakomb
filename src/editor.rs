use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
};

use ggez::{event::EventHandler, graphics::Rect, Context, GameResult};
use internship::IStr;
use na::Point3;
use ndarray::Array3;
use serde::{Deserialize, Serialize};

pub struct Editor {
    voxels: BTreeMap<IStr, Voxel>,
    objects: BTreeMap<IStr, Model>,

    mode: EditorMode,
}

enum EditorMode {
    Voxel { current: Option<Voxel> },
    Model { current: Option<EditableModel> },
}

impl EventHandler for Editor {
    fn update(&mut self, context: &mut Context) -> GameResult<()> {
        match &mut self.mode {
            EditorMode::Voxel { current } => {}

            EditorMode::Model { current } => {}
        }

        Ok(())
    }

    fn draw(&mut self, context: &mut Context) -> GameResult<()> {
        match &mut self.mode {
            EditorMode::Voxel { current } => {}

            EditorMode::Model { current } => {}
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Voxel {
    pub x: VoxelFace,
    pub y: VoxelFace,
    pub z: VoxelFace,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VoxelFace {
    pub char_offset: u16,
    pub color: Color,
    pub transparent: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Model {
    voxels: Array3<Option<IStr>>,
}

impl From<EditableModel> for Model {
    fn from(mut eo: EditableModel) -> Self {
        if eo.voxels.is_empty() {
            return Self {
                voxels: Array3::from_shape_simple_fn((0, 0, 0), || unreachable!()),
            };
        }

        let mut keys = eo.voxels.keys();
        let first = keys.by_ref().next().unwrap();

        let mut min_x = first.coords.x;
        let mut min_y = first.coords.y;
        let mut min_z = first.coords.z;

        let mut max_x = first.coords.x;
        let mut max_y = first.coords.y;
        let mut max_z = first.coords.z;

        for pos in keys {
            min_x = min_x.min(pos.coords.x);
            min_y = min_y.min(pos.coords.y);
            min_z = min_z.min(pos.coords.z);

            max_x = max_x.max(pos.coords.x);
            max_y = max_y.max(pos.coords.y);
            max_z = max_z.max(pos.coords.z);
        }

        let w = (max_x - min_x) as usize;
        let h = (max_y - min_y) as usize;
        let d = (max_z - min_z) as usize;

        let voxels = Array3::from_shape_fn((w, h, d), |(x, y, z)| {
            eo.voxels.remove(&Point3::new(
                (x as i16) - min_x,
                (y as i16) - min_y,
                (z as i16) - min_z,
            ))
        });

        assert!(eo.voxels.is_empty());

        Self { voxels }
    }
}

#[derive(Clone, Debug)]
struct EditableModel {
    voxels: HashMap<Point3<i16>, IStr>,
}

impl From<Model> for EditableModel {
    fn from(mut o: Model) -> Self {
        Self {
            voxels: o
                .voxels
                .indexed_iter_mut()
                .filter_map(|((x, y, z), v)| {
                    v.take().map(|v| {
                        (
                            Point3::new(
                                x.try_into().unwrap(),
                                y.try_into().unwrap(),
                                z.try_into().unwrap(),
                            ),
                            v,
                        )
                    })
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}
