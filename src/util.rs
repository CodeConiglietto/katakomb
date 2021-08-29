use ggez::nalgebra as na;
use na::*;
use ndarray::prelude::*;

use crate::{rendering::{tile::Tile, drawable::Drawable}, constants::*, geometry::util::*, world::util::*};

pub fn is_in_array(array: ArrayView3<Tile>, pos: Point3<usize>) -> bool {
    // pos.x >= 0
    //     && pos.y >= 0
    //     && pos.z >= 0
        // && 
        pos.x < array.dim().0
        && pos.y < array.dim().1
        && pos.z < array.dim().2
}

pub fn get_tile_at(pos: Point3<f32>, tile_array: &Array3<Tile>) -> Tile {
    let index = world_pos_to_index(pos);

    tile_array[[index.x, index.y, index.z]].clone()
}