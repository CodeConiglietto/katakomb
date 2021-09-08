use na::*;
use ndarray::prelude::*;
use noise::{NoiseFn, OpenSimplex, Perlin, Value, Worley};
use rand::prelude::*;

use crate::{
    constants::*,
    rendering::tile::{Tile, *},
    util::*,
};

pub struct ChunkGenPackage {
    pub simplex: OpenSimplex,
    pub simplex_weight: Value,
    pub perlin: Perlin,
    pub perlin_weight: Value,
    // pub worley: Worley,
    // pub worley_weight: Value,
    pub value: Value,
    pub value_weight: Value,
}

pub fn gen_tile(gen_package: &ChunkGenPackage, x: usize, y: usize, z: usize) -> Tile {
    let simplex_raw = gen_package
        .simplex
        .get([
            x as f64 * NOISE_SCALE,
            y as f64 * NOISE_SCALE, // * 0.05,
            z as f64 * NOISE_SCALE,
        ])
        .abs();
    let simplex_raw_weight = gen_package
        .simplex_weight
        .get([
            x as f64 * NOISE_WEIGHT_SCALE,
            y as f64 * NOISE_WEIGHT_SCALE,
            z as f64 * NOISE_WEIGHT_SCALE,
        ])
        .abs();

    let perlin_raw = gen_package
        .perlin
        .get([
            x as f64 * NOISE_SCALE,
            y as f64 * NOISE_SCALE, // * 0.05,
            z as f64 * NOISE_SCALE,
        ])
        .abs();
    let perlin_raw_weight = gen_package
        .perlin_weight
        .get([
            x as f64 * NOISE_WEIGHT_SCALE,
            y as f64 * NOISE_WEIGHT_SCALE,
            z as f64 * NOISE_WEIGHT_SCALE,
        ])
        .abs();

    // let worley_raw = gen_package.worley.get([
    //     x as f64 * NOISE_SCALE,
    //     y as f64 * NOISE_SCALE * 0.05,
    //     z as f64 * NOISE_SCALE,
    // ]).abs();
    // let worley_raw_weight = gen_package.worley_weight.get([
    //     x as f64 * NOISE_WEIGHT_SCALE,
    //     y as f64 * NOISE_WEIGHT_SCALE,
    //     z as f64 * NOISE_WEIGHT_SCALE,
    // ]).abs();

    let value_raw = gen_package
        .value
        .get([
            x as f64 * NOISE_SCALE,
            y as f64 * NOISE_SCALE, // * 0.05,
            z as f64 * NOISE_SCALE,
        ])
        .abs();
    let value_raw_weight = gen_package
        .value_weight
        .get([
            x as f64 * NOISE_WEIGHT_SCALE,
            y as f64 * NOISE_WEIGHT_SCALE,
            z as f64 * NOISE_WEIGHT_SCALE,
        ])
        .abs();

    let weights_total = simplex_raw_weight +
        perlin_raw_weight +
        // worley_raw_weight + 
        value_raw_weight;

    let final_value = (simplex_raw * (simplex_raw_weight / weights_total))
        + (perlin_raw * (perlin_raw_weight / weights_total))
        // + (worley_raw * (worley_raw_weight / weights_total))
        + (value_raw * (value_raw_weight / weights_total));
    // let noise_value = noise
    //     .get([x as f64 * 0.25, y as f64 * 0.025, z as f64 * 0.25])
    //     .abs()
    //     .powf(2.0)
    //     .max(
    //         meta_noise
    //             .get([x as f64 * 0.05, y as f64 * 0.005, z as f64 * 0.05])
    //             .abs(),
    //     );

    let cave_threshold =
        ((y as f64 - (CHUNK_SIZE / 2) as f64).abs() / (CHUNK_SIZE / 2) as f64).max(0.0) + 0.15;

    Tile {
        pos: Point3::new(x as f32, y as f32, z as f32),
        illumination_color: ggez::graphics::Color::BLACK,
        tile_type: if final_value > cave_threshold {
            TileType::Air
        } else {
            TileType::Rock
        },
    }
}

pub fn generate_chunk(offset: Point3<i32>, gen_package: &ChunkGenPackage) -> Array3<Tile> {
    let mut chunk = Array3::from_shape_fn((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), |(x, y, z)| {
        gen_tile(gen_package, x, y, z)
    });

    for x in 0..chunk.dim().0 {
        for y in 0..chunk.dim().1 {
            for z in 0..chunk.dim().2 {
                let pos = Point3::new(x, y, z);
                // BUGGY AF CANDLE CODE THAT'S A SHIT
                // let pos_under = Point3::new(x, y - 1, z);
                // if thread_rng().gen_range(0, 500) == 0
                //     && is_in_array(chunk.view(), pos)
                //     && is_in_array(chunk.view(), pos_under)
                //     && chunk[[x, y, z]].tile_type == TileType::Air
                //     && chunk[[x, y - 1, z]].tile_type == TileType::Rock
                // {
                //     chunk[[x, y, z]] = Tile {
                //         pos: Point3::new(x as f32, y as f32, z as f32),
                //         illumination: 0.5,
                //         tile_type: TileType::Candle,
                //     }
                // }
            }
        }
    }

    chunk
}
