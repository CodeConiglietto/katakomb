use na::*;
use ndarray::prelude::*;

use crate::rendering::{drawable::*, tile::*};

pub fn world_pos_to_index(pos: Point3<f32>) -> Point3<usize> {
    Point3::new(
        pos.x.floor() as usize,
        pos.y.floor() as usize,
        pos.z.floor() as usize,
    )
}

pub fn world_pos_to_int(pos: Point3<f32>) -> Point3<i32> {
    Point3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    )
}

pub fn any_neighbour_empty(array: &ArrayView3<Tile>, pos: Point3<i32>) -> bool {
    for x in -1..2 {
        for y in -1..2 {
            for z in -1..2 {
                let x_index = (pos.x + x) as usize;
                let y_index = (pos.y + y) as usize;
                let z_index = (pos.z + z) as usize;

                if x_index >= array.dim().0
                    || y_index >= array.dim().1
                    || z_index >= array.dim().2
                    || array[[x_index, y_index, z_index]]
                        .tile_type
                        .is_transparent()
                {
                    return true;
                }
            }
        }
    }

    false
}
