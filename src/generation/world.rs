use na::*;
use ndarray::prelude::*;
use noise::{NoiseFn, OpenSimplex};
use rand::prelude::*;

use crate::{
    constants::*,
    rendering::tile::{Tile, *},
    util::*,
};

pub fn gen_tile(noise: OpenSimplex, meta_noise: OpenSimplex, x: usize, y: usize, z: usize) -> Tile {
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

    Tile {
        pos: Point3::new(x as f32, y as f32, z as f32),
        illumination: 0.5,
        tile_type: if noise_value > cave_threshold {
            TileType::Air
        } else {
            TileType::Rock
        },
    }
}

pub fn generate_chunk(
    offset: Point3<i32>,
    noise: OpenSimplex,
    meta_noise: OpenSimplex,
) -> Array3<Tile> {
    let mut chunk = Array3::from_shape_fn((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), |(x, y, z)| {
        gen_tile(noise, meta_noise, x, y, z)
    });

    for x in 0..chunk.dim().0 {
        for y in 0..chunk.dim().1 {
            for z in 0..chunk.dim().2 {
                let pos = Point3::new(x, y, z);
                let pos_under = Point3::new(x, y - 1, z);
                if thread_rng().gen_range(0, 500) == 0
                    && is_in_array(chunk.view(), pos)
                    && is_in_array(chunk.view(), pos_under)
                    && chunk[[x, y, z]].tile_type == TileType::Air
                    && chunk[[x, y - 1, z]].tile_type == TileType::Rock
                {
                    chunk[[x, y, z]] = Tile {
                        pos: Point3::new(x as f32, y as f32, z as f32),
                        illumination: 0.5,
                        tile_type: TileType::Candle,
                    }
                }
            }
        }
    }

    chunk
}
