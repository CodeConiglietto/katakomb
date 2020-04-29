use na::*;
use ndarray::prelude::*;
use noise::{NoiseFn, OpenSimplex};
use rand::prelude::*;

use crate::{
    constants::*,
    rendering::voxel::{Voxel, *},
    util::*,
};

pub fn gen_voxel(
    noise: OpenSimplex,
    meta_noise: OpenSimplex,
    x: usize,
    y: usize,
    z: usize,
) -> Voxel {
    let noise_value = noise
        .get([x as f64 * 0.1, y as f64 * 0.025, z as f64 * 0.1])
        .abs()
        .powf(2.0)
        .max(
            meta_noise
                .get([x as f64 * 0.05, y as f64 * 0.005, z as f64 * 0.05])
                .abs(),
        );

    let cave_threshold =
        ((y as f64 - (CHUNK_SIZE / 2) as f64).abs() / (CHUNK_SIZE / 2) as f64).max(0.0) + 0.05;

    Voxel {
        pos: Point3::new(x as f32, y as f32, z as f32),
        illumination: 0.5,
        voxel_type: if noise_value > cave_threshold {
            VoxelType::Air
        } else {
            VoxelType::Rock
        },
    }
}

pub fn generate_chunk(
    offset: Point3<i32>,
    noise: OpenSimplex,
    meta_noise: OpenSimplex,
) -> Array3<Voxel> {
    let mut chunk = Array3::from_shape_fn((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), |(x, y, z)| {
        gen_voxel(noise, meta_noise, x, y, z)
    });

    for x in 0..chunk.dim().0 {
        for y in 0..chunk.dim().1 {
            for z in 0..chunk.dim().2 {
                let pos = Point3::new(x, y, z);
                let pos_under = Point3::new(x, y - 1, z);
                if thread_rng().gen_range(0, 500) == 0
                    && is_in_array(chunk.view(), pos)
                    && is_in_array(chunk.view(), pos_under)
                    && chunk[[x, y, z]].voxel_type == VoxelType::Air
                    && chunk[[x, y - 1, z]].voxel_type == VoxelType::Rock
                {
                    chunk[[x, y, z]] = Voxel {
                        pos: Point3::new(x as f32, y as f32, z as f32),
                        illumination: 0.5,
                        voxel_type: VoxelType::Candle,
                    }
                }
            }
        }
    }

    chunk
}
