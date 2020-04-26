use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
};

use ggez::{event::EventHandler, Context, GameResult};
use internship::IStr;
use na::Point3;
use ndarray::Array3;
use serde::{Deserialize, Serialize};

pub struct Editor {
    voxels: BTreeMap<IStr, Voxel>,
    objects: BTreeMap<IStr, Object>,

    current: Option<Editable>,
}

impl EventHandler for Editor {
    fn update(&mut self, context: &mut Context) -> GameResult<()> {
        match &mut self.current {
            Some(Editable::Voxel(voxel)) => {}

            Some(Editable::Object(object)) => {}

            None => {}
        }

        Ok(())
    }

    fn draw(&mut self, context: &mut Context) -> GameResult<()> {
        match &mut self.current {
            Some(Editable::Voxel(voxel)) => {}

            Some(Editable::Object(object)) => {}

            None => {}
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
pub struct Object {
    voxels: Array3<Option<IStr>>,
}

impl From<EditableObject> for Object {
    fn from(mut eo: EditableObject) -> Self {
        let mut w = 0;
        let mut h = 0;
        let mut d = 0;

        for pos in eo.voxels.keys() {
            w = w.max(pos.coords.x);
            h = h.max(pos.coords.y);
            d = d.max(pos.coords.z);
        }

        let voxels = Array3::from_shape_fn((w.into(), h.into(), d.into()), |(x, y, z)| {
            eo.voxels.remove(&Point3::new(x as u8, y as u8, z as u8))
        });

        assert!(eo.voxels.is_empty());

        Self { voxels }
    }
}

#[derive(Clone, Debug)]
struct EditableObject {
    voxels: HashMap<Point3<u8>, IStr>,
}

impl From<Object> for EditableObject {
    fn from(mut o: Object) -> Self {
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

enum Editable {
    Voxel(Voxel),
    Object(EditableObject),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}
