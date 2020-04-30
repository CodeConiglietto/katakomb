use na::*;
use ndarray::prelude::*;

use crate::rendering::tile::Tile;

pub fn is_in_array(array: ArrayView3<Tile>, pos: Point3<usize>) -> bool {
    pos.x >= 0
        && pos.y >= 0
        && pos.z >= 0
        && pos.x < array.dim().0
        && pos.y < array.dim().1
        && pos.z < array.dim().2
}
