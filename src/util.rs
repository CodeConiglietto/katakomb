use lazy_static::lazy_static;
use na::*;
use ndarray::prelude::*;

use crate::{constants::*, rendering::tile::Tile};

lazy_static! {
    pub static ref MAX_LOOKUP_RANGE: usize = (LIGHT_RANGE + 1).max(PLAYER_SIGHT_RANGE + 1);
    pub static ref ATAN_CASTING_LOOKUP: Array2<f32> =
        Array2::from_shape_fn((*MAX_LOOKUP_RANGE, *MAX_LOOKUP_RANGE), |(x, y)| (x as f32
            / (y + 1) as f32)
            .atan());
    pub static ref EUCLIDEAN_DISTANCE_LOOKUP: Array3<f32> = Array3::from_shape_fn(
        (*MAX_LOOKUP_RANGE, *MAX_LOOKUP_RANGE, *MAX_LOOKUP_RANGE),
        |(x, y, z)| ((x as f32).powf(2.0) + (y as f32).powf(2.0) + (z as f32).powf(2.0)).sqrt()
    );
}

pub fn is_in_array(array: ArrayView3<Tile>, pos: Point3<usize>) -> bool {
    // pos.x >= 0
    //     && pos.y >= 0
    //     && pos.z >= 0
    //     &&
    pos.x < array.dim().0 && pos.y < array.dim().1 && pos.z < array.dim().2
}

pub fn get_tile_at(pos: Point3<f32>, tile_array: &Array3<Tile>) -> Tile {
    let index = world_pos_to_index(pos);

    tile_array[[index.x, index.y, index.z]].clone()
}