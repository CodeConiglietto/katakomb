use ggez::nalgebra as na;
use na::*;
use ndarray::prelude::*;

use crate::{constants::*, geometry::util::*, rendering::{drawable::*, tile::*, light::Light}, util::*};

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

pub fn any_neighbour_is<F>(array: ArrayView3<Tile>, pos: Point3<i32>, f: F) -> bool
where
    F: Fn(&Tile) -> bool,
{
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let x_index = (pos.x + x) as usize;
                let y_index = (pos.y + y) as usize;
                let z_index = (pos.z + z) as usize;

                if x_index < array.dim().0
                    && y_index < array.dim().1
                    && z_index < array.dim().2
                    && f(&array[[x_index, y_index, z_index]])
                {
                    return true;
                }
            }
        }
    }

    false
}

//Tries to fire a bresenham hitscan, returns dest if no collisions
pub fn try_bresenham_hitscan(
    tile_array: ArrayView3<Tile>,
    src: Point3<i32>,
    dest: Point3<i32>,
) -> Point3<i32> {
    if src.x >= 0
        && src.x < CHUNK_SIZE as i32
        && src.y >= 0
        && src.y < CHUNK_SIZE as i32
        && src.z >= 0
        && src.z < CHUNK_SIZE as i32
    {
        for ray_point in calculate_bresenham(src, dest) {
            let ray_tile = tile_array[[
                ray_point.x as usize,
                ray_point.y as usize,
                ray_point.z as usize,
            ]]
            .clone();

            if ray_point.x >= 0
                && ray_point.x < CHUNK_SIZE as i32
                && ray_point.y >= 0
                && ray_point.y < CHUNK_SIZE as i32
                && ray_point.z >= 0
                && ray_point.z < CHUNK_SIZE as i32
            {
                if !ray_tile.tile_type.is_transparent() {
                    return ray_point;
                }
            }
        }
    } else {
        return src;
    }

    return dest;
}

//Tries to fire a floating point hitscan, returns dest if no collisions
//This assumes that whatever is being scanned against is in an evenly spaced grid of tile size 1*1*1
pub fn try_ray_hitscan(
    tile_array: ArrayView3<Tile>,
    src: Point3<f32>,
    dest: Point3<f32>,
) -> Point3<f32> {
    if is_in_array(tile_array, world_pos_to_index(src)) {
        let distance = euclidean_distance_squared(src, dest).sqrt();
        let distance_ratios = Point3::new(
            (dest.x - src.x) / distance,
            (dest.y - src.y) / distance,
            (dest.z - src.z) / distance,
        );

        let mut ray_point = src.clone();

        ray_point.x += distance_ratios.x;
        ray_point.y += distance_ratios.y;
        ray_point.z += distance_ratios.z;

        for _i in 0..distance.floor() as i32 - 1 {
            let ray_int_point = Point3::new(
                ray_point.x as usize,
                ray_point.y as usize,
                ray_point.z as usize,
            );

            if is_in_array(tile_array, world_pos_to_index(ray_point)) {
                let ray_tile = &tile_array[[ray_int_point.x, ray_int_point.y, ray_int_point.z]];

                if !ray_tile.tile_type.is_transparent() {
                    return ray_point;
                }
            }

            ray_point.x += distance_ratios.x;
            ray_point.y += distance_ratios.y;
            ray_point.z += distance_ratios.z;
        }
    } else {
        return src;
    }

    return dest;
}

pub fn hitscan_tile(
    tile_array: ArrayView3<Tile>,
    src: Point3<f32>,
    dest: Point3<f32>,
) -> Vec<Point3<f32>> {
    let mut hits = Vec::new();

    for target in get_cube_points(dest) {
        let hit = try_ray_hitscan(tile_array, src, target);

        if world_pos_to_index(hit) != world_pos_to_index(target) {
            hits.push(hit);
        }
    }

    hits
}

// fn get_tile_from_point(tile_array: ArrayView3<Tile>, pos: Point3::<f32>) -> Tile{

// }

pub fn get_light_hitscans(
    light: &Light,
    lighting_sphere: &Vec<Point3<f32>>,
    tile_array: ArrayView3<Tile>,
) -> Vec<Point3<f32>> {
    let mut ray_hits = Vec::new();

    // tile_array[[
    //         light.pos.x.floor() as usize,
    //         light.pos.y.floor() as usize,
    //         light.pos.z.floor() as usize,
    //     ]]
    //     .illumination = 0.9;

    let light_target: Point3<f32> = Point3::origin() + (light.facing * light.range).coords;

    for target_point in lighting_sphere {
        let target_point_offset = Point3::new(
            target_point.x + light.pos.x + light_target.x,
            target_point.y + light.pos.y + light_target.y,
            target_point.z + light.pos.z + light_target.z,
        );

        ray_hits.append(&mut hitscan_tile(
            tile_array,
            light.pos,
            target_point_offset,
        ));
    }

    ray_hits
}
