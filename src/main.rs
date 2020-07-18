use ggez::event::{self, EventHandler, KeyCode};
use ggez::{
    // audio::{SoundData, Source, SoundSource},
    conf::WindowMode,
    graphics,
    graphics::{spritebatch::SpriteBatch, DrawParam, Image, *},
    input::{keyboard, mouse},
    timer,
    Context,
    ContextBuilder,
    GameResult,
};

use itertools::Itertools;
use na::{Isometry3, Perspective3, Point2, Point3, Rotation3, Vector3};
use ndarray::arr2;
use ndarray::prelude::*;
use noise::{NoiseFn, OpenSimplex, Seedable};
use rand::prelude::*;
use rayon::prelude::*;
use rodio::{buffer::SamplesBuffer, source, Sample, Source};

use std::{
    cmp::Ordering,
    collections::BTreeSet,
    env,
    fs::File,
    io::BufReader,
    iter::{self, Map},
    path::PathBuf,
    slice,
    time::Duration,
};

mod editor;
mod ui;

use crate::{
    constants::*,
    generation::world::*,
    geometry::util::*,
    rendering::{drawable::Drawable, font::*, light::*, voxel::*},
    util::*,
    world::util::*,
};

pub mod constants;
pub mod generation;
pub mod geometry;
pub mod rendering;
pub mod util;
pub mod world;

fn main() {
    let mut cb = ContextBuilder::new("Katakomb", "CodeBunny");

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest_dir);
        path.push("resources");
        println!("Adding path {:?}", path);
        cb = cb.add_resource_path(path);
    }

    let (mut ctx, mut event_loop) = cb
        .window_mode(WindowMode::default().dimensions(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build()
        .expect("Could not create ggez context!");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.
    let mut my_game = MyGame::new(&mut ctx);

    // Run!
    match event::run(&mut ctx, &mut event_loop, &mut my_game) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occurred: {}", e),
    }
}

struct MyGame {
    blank_texture: Image,
    lighting_sphere: Vec<Point3<f32>>,
    font: KataFont,
    voxel_array: Array3<Voxel>,
    draw_voxels: Vec<Voxel>,
    camera_pos: Point3<f32>,

    camera_rotation: Point2<f32>,

    nuke_lighting: bool,

    current_tic: u64,

    lights: Vec<Light>,
    light_noise: OpenSimplex,

    player_gun_model: Array2<VoxelType>,
    player_gun_timer: u8,
    // player_gun_sound: SoundData,
    player_ads: f32,
    player_gun_recoil: f32,
    player_gun_rotation: Point2<f32>,
    // sound_queue: Vec<(f64, Source)>,
}

impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        // Load/create resources such as images here.
        let noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());
        let meta_noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());

        set_default_filter(ctx, FilterMode::Nearest);

        use crate::rendering::voxel::VoxelType::*;

        MyGame {
            blank_texture: Image::solid(ctx, 1, WHITE).unwrap(),
            lighting_sphere: calculate_sphere_surface(LIGHT_RANGE),
            font: load_font(ctx),
            voxel_array: generate_chunk(Point3::new(0, 0, 0), noise, meta_noise),
            draw_voxels: Vec::new(),
            camera_pos: Point3::new(
                (CHUNK_SIZE / 2) as f32,
                (CHUNK_SIZE / 2) as f32,
                (CHUNK_SIZE / 2) as f32,
            ),
            camera_rotation: Point2::origin(),
            nuke_lighting: false,
            current_tic: 0,
            lights: Vec::new(),
            light_noise: OpenSimplex::new(),
            player_gun_recoil: 0.0,
            player_gun_rotation: Point2::origin(),
            player_gun_model: arr2(&[
                [
                    Air, Air, FrontSight, Air, Air, Air, Air, RearSight, Air, Air, Air,
                ],
                [
                    BarrelEnd, BarrelEnd, GasBlock, Barrel, Barrel, RecLower, RecLower, RecLower,
                    Air, StockUpper, StockUpper,
                ],
                [
                    Air, Air, Air, Air, Air, Air, Magazine, Grip, Stock, Stock, Stock,
                ],
            ]),
            player_gun_timer: 0,
            // player_gun_sound: SoundData::new(ctx, r"/gunshot.wav").unwrap(),
            player_ads: 0.0,
            // sound_queue: Vec::new(),
        }
    }
}

//Tries to fire a bresenham hitscan, returns dest if no collisions
fn try_bresenham_hitscan(
    voxel_array: ArrayView3<Voxel>,
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
            let ray_voxel = voxel_array[[
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
                if !ray_voxel.voxel_type.is_transparent() {
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
fn try_ray_hitscan(
    voxel_array: ArrayView3<Voxel>,
    src: Point3<f32>,
    dest: Point3<f32>,
) -> Point3<f32> {
    if is_in_array(voxel_array, world_pos_to_index(src)) {
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

            if is_in_array(voxel_array, world_pos_to_index(ray_point)) {
                let ray_voxel = &voxel_array[[ray_int_point.x, ray_int_point.y, ray_int_point.z]];

                if !ray_voxel.voxel_type.is_transparent() {
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

fn get_cube_points(pos: Point3<f32>) -> Vec<Point3<f32>> {
    vec![
        Point3::new(pos.x - 0.0, pos.y - 0.0, pos.z - 0.0),
        Point3::new(pos.x - 0.0, pos.y - 0.0, pos.z + 0.9),
        Point3::new(pos.x - 0.0, pos.y + 0.9, pos.z - 0.0),
        Point3::new(pos.x - 0.0, pos.y + 0.9, pos.z + 0.9),
        Point3::new(pos.x + 0.9, pos.y - 0.0, pos.z - 0.0),
        Point3::new(pos.x + 0.9, pos.y - 0.0, pos.z + 0.9),
        Point3::new(pos.x + 0.9, pos.y + 0.9, pos.z - 0.0),
        Point3::new(pos.x + 0.9, pos.y + 0.9, pos.z + 0.9),
    ]
}

fn hitscan_tile(
    voxel_array: ArrayView3<Voxel>,
    src: Point3<f32>,
    dest: Point3<f32>,
) -> Vec<Point3<f32>> {
    let mut hits = Vec::new();

    for target in get_cube_points(dest) {
        let hit = try_ray_hitscan(voxel_array, src, target);

        if world_pos_to_index(hit) != world_pos_to_index(target) {
            hits.push(hit);
        }
    }

    hits
}

// fn get_voxel_from_point(voxel_array: ArrayView3<Voxel>, pos: Point3::<f32>) -> Voxel{

// }

fn get_light_hitscans(
    light: &Light,
    lighting_sphere: &Vec<Point3<f32>>,
    voxel_array: ArrayView3<Voxel>,
) -> Vec<Point3<f32>> {
    let mut ray_hits = Vec::new();

    // voxel_array[[
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
            voxel_array,
            light.pos,
            target_point_offset,
        ));
    }

    ray_hits
}

fn get_voxel_at(pos: Point3<f32>, voxel_array: &Array3<Voxel>) -> Voxel {
    let index = world_pos_to_index(pos);

    voxel_array[[index.x, index.y, index.z]].clone()
}

/*
trait IteratorSourceExt: Sized + Source
where
    Self::Item: Sample,
{
    fn resample<F, U>(self, f: F) -> Box<dyn Source<Item = Self::Item> + Send + Sync>
    where
        Self::Item: Send + Sync,
        F: FnMut(iter::StepBy<slice::Iter<Self::Item>>) -> U,
        U: ExactSizeIterator,
        U::Item: Sample;
}

impl<T> IteratorSourceExt for T
where
    T: Sized + Source,
    T::Item: Sample,
{
    fn resample<F, U>(self, mut f: F) -> Box<dyn Source<Item = Self::Item> + Send + Sync>
    where
        Self::Item: Send + Sync,
        F: FnMut(iter::StepBy<slice::Iter<Self::Item>>) -> U,
        U: ExactSizeIterator,
        U::Item: Sample,
    {
        let mut max_chunk_size = MAX_RESAMPLE_CHUNK_SIZE * self.channels() as usize;

        let mut chunk_size = max_chunk_size;
        let _self = &mut self;
        let mut new_frame = true;
        let mut chunk = Vec::new();

        Box::new(source::from_iter(iter::repeat_with(|| {
            if new_frame {
                if let Some(frame_len) = _self.current_frame_len() {
                    chunk_size = max_chunk_size.min(frame_len * _self.channels() as usize);
                }

                new_frame = false;
            } else {
                if let Some(frame_len) = _self.current_frame_len() {
                    if frame_len < chunk_size {
                        new_frame = true;
                    }
                }
            };

            chunk.clear();
            chunk.extend(_self.take(chunk_size));

            let out: Vec<_> = (0.._self.channels())
                .map(|channel_idx| {
                    let out = f(chunk
                        .iter()
                        .dropping(channel_idx as usize)
                        .step_by(_self.channels() as usize));
                })
                .interleave()
                .collect();

            let new_sample_rate = if chunk.len() == out.len() {
                _self.sample_rate()
            } else {
                (_self.sample_rate() as f64 * (out.len() as f64 / chunk.len() as f64)) as u32
            };

            SamplesBuffer::new(self.channels(), new_sample_rate, chunk)
        })))
    }
}

const MAX_RESAMPLE_CHUNK_SIZE: usize = 1024 * 100;
*/

impl EventHandler for MyGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // Update code here...

        let mut muzzle_flash = false;

        self.player_gun_recoil *= 0.95;
        self.player_gun_rotation.x *= 0.95;
        self.player_gun_rotation.y *= 0.95;

        let update_time = timer::duration_to_f64(timer::time_since_start(ctx));

        let movement_rotation =
            Rotation3::from_axis_angle(&Vector3::y_axis(), self.camera_rotation.x);

        let gun_rotation = Rotation3::from_euler_angles(
            -self.player_gun_rotation.y,
            self.player_gun_rotation.x,
            0.0,
        );

        let view_rotation =
            Rotation3::from_euler_angles(self.camera_rotation.y, self.camera_rotation.x, 0.0);

        let gun_facing = view_rotation
            .transform_point(&gun_rotation.transform_point(&Point3::new(0.0, 0.0, 1.0)));

        if self.player_gun_timer == 0 {
            if mouse::button_pressed(ctx, mouse::MouseButton::Left) {
                self.player_gun_recoil = (self.player_gun_recoil + 0.2).min(1.0);
                self.player_gun_rotation.x = (self.player_gun_rotation.x
                    + (thread_rng().gen::<f32>() - 0.5) * 0.05)
                    .min(1.0)
                    .max(-1.0);
                self.player_gun_rotation.y = (self.player_gun_rotation.y + 0.05).min(1.0);

                // dbg!(std::env::current_dir().unwrap().to_str().unwrap());

                let device = rodio::default_output_device().unwrap();
                let file = File::open(r"resources/gunshot.wav").unwrap();
                let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

                let mut echo_distances = Vec::new();

                for cube_point in get_cube_points(Point3::new(-0.5, -0.5, -0.5)) {
                    let ray_target = self.camera_pos + (cube_point.coords * MAX_SOUND_RANGE * 2.0);

                    if is_in_array(self.voxel_array.view(), world_pos_to_index(ray_target)) {
                        let ray_hit = try_bresenham_hitscan(
                            self.voxel_array.view(),
                            world_pos_to_int(self.camera_pos),
                            world_pos_to_int(ray_target),
                        );

                        if ray_hit != world_pos_to_int(ray_target) {
                            // //TODO mess with this
                            let hit_distance = euclidean_distance_squared(
                                self.camera_pos,
                                Point3::new(ray_hit.x as f32, ray_hit.y as f32, ray_hit.z as f32),
                            )
                            .sqrt();
                            let hit_distance_ratio = hit_distance / (MAX_SOUND_RANGE * 2.0);
                            let hit_distance_ratio_squared = hit_distance * hit_distance;
                            echo_distances.push(hit_distance_ratio);
                            // let mut source = Source::from_data(ctx, self.player_gun_sound.clone()).unwrap();
                            // source.set_pitch(0.5 + 0.5 * (1.0 - hit_distance_ratio));
                            // source.set_fade_in(Duration::from_millis((hit_distance_ratio_squared) as u64));
                            // //source.set_volume(1.0 - (hit_distance_ratio * 0.5));
                            // self.sound_queue.push((update_time + (hit_distance_ratio * 0.5) as f64, source));
                            // //TODO take average of hit distances and use that to change the non-ray sound's pitch
                        }
                    }
                }

                echo_distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let min_echo_distance = echo_distances.first().unwrap();
                // let med_echo_distance = echo_distances[echo_distances.len() /2 ];
                let max_echo_distance = echo_distances.last().unwrap();

                //warning: using more than 2 reverbs leads to very unpleasant results :<
                rodio::play_raw(
                    &device,
                    source
                        .convert_samples::<i16>()
                        .buffered()
                        .reverb(
                            Duration::from_millis((min_echo_distance * 750.0) as u64),
                            0.5 - min_echo_distance * 0.5,
                        )
                        // .reverb(
                        //     Duration::from_millis((med_echo_distance * 750.0) as u64),
                        //     0.5 - med_echo_distance * 0.5,
                        // )
                        .reverb(
                            Duration::from_millis((max_echo_distance * 750.0) as u64),
                            0.25 - max_echo_distance * 0.25,
                        )
                        .convert_samples(),
                );

                muzzle_flash = true;
                self.player_gun_timer = 4;
            }
        } else {
            self.player_gun_timer -= 1;
        }

        if mouse::button_pressed(ctx, mouse::MouseButton::Right) {
            self.player_ads *= 0.9; //(self.player_ads - 0.1).max(0.0);
        } else {
            self.player_ads = (self.player_ads + 0.1).min(1.0);
        }

        let mut movement_offset: Point3<f32> = Point3::origin();

        if keyboard::is_key_pressed(ctx, KeyCode::A) {
            movement_offset.x += 0.25;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::D) {
            movement_offset.x -= 0.25;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Space) {
            movement_offset.y += 0.25;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::LControl) {
            movement_offset.y -= 0.25;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::W) {
            movement_offset.z += 0.25;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::S) {
            movement_offset.z -= 0.25;
        }

        movement_offset = movement_rotation.transform_point(&movement_offset);

        self.camera_pos = self.camera_pos + movement_offset.coords;

        if keyboard::is_key_pressed(ctx, KeyCode::Left) {
            self.camera_rotation.x += 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Right) {
            self.camera_rotation.x -= 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Up) {
            self.camera_rotation.y -= 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Down) {
            self.camera_rotation.y += 0.1;
        }

        if keyboard::is_key_pressed(ctx, KeyCode::N) {
            self.nuke_lighting = true;
        }

        self.draw_voxels.clear();

        //let voxel_points = self.voxel_draw_points;
        let camera_pos = self.camera_pos;

        let voxel_array = &self.voxel_array;
        let zip_iter = ndarray::Zip::indexed(voxel_array);

        let _int_camera_pos = Point3::new(
            camera_pos.x.floor() as i32,
            camera_pos.y.floor() as i32,
            camera_pos.z.floor() as i32,
        );

        let nuke_lighting = self.nuke_lighting;

        //populate voxels to draw

        let voxel_array_view = self.voxel_array.view();

        self.lights = self
            .lights
            .par_iter()
            .filter(|light| light.persistent)
            .cloned()
            .collect();

        if is_in_array(self.voxel_array.view(), world_pos_to_index(camera_pos)) {
            let player_light = Light {
                pos: camera_pos,
                facing: gun_facing,
                illumination: 0.5,
                range: 24.0,
                persistent: false,
            };

            self.lights.push(player_light);
            // dbg!(&lights);

            if muzzle_flash {
                let muzzle_light = Light {
                    pos: camera_pos,
                    facing: gun_facing,
                    illumination: 0.9,
                    range: 0.0,
                    persistent: false,
                };

                self.lights.push(muzzle_light);
            }
        }

        let light_iter = self.lights.par_iter();
        let mut draw_voxels: Vec<_> = light_iter
            .flat_map(|light| get_light_hitscans(light, &self.lighting_sphere, voxel_array_view))
            .map(|pos| {
                let mut vox = get_voxel_at(pos, &self.voxel_array).clone();
                vox.illumination = 0.5; //euclidean_distance_squared(vox.pos, light.pos) / (LIGHT_RANGE * LIGHT_RANGE) as f32;
                vox
            })
            .collect();

        draw_voxels = draw_voxels
            .par_iter()
            .filter(|voxel| {
                any_neighbour_empty(&voxel_array.view(), world_pos_to_int(voxel.pos))
                    && world_pos_to_index(try_ray_hitscan(
                        voxel_array.view(),
                        camera_pos,
                        voxel.pos,
                    )) == world_pos_to_index(voxel.pos)
            })
            .cloned()
            .collect();

        draw_voxels.sort_unstable_by(|a, b| {
            euclidean_distance_squared(b.pos, camera_pos)
                .partial_cmp(&euclidean_distance_squared(a.pos, camera_pos))
                .unwrap_or(Ordering::Equal)
        });

        draw_voxels.dedup_by(|a, b| {
            let equal = world_pos_to_index(a.pos) == world_pos_to_index(b.pos);
            if equal {
                b.illumination = (b.illumination + 0.01).min(1.0)
            };
            if b.illumination > 1.0 {
                panic!()
            };
            equal
        });

        std::mem::swap(&mut draw_voxels, &mut self.draw_voxels);

        // self.draw_voxels.clear();
        // self.draw_voxels.par_extend(
        //     zip_iter
        //         .into_par_iter()
        //         .filter(|((x, y, z), v)| {
        //             (!v.voxel_type.is_transparent() || v.voxel_type.illuminates())
        //                 && (v.illumination > 0.01
        //                     || v.voxel_type.illuminates()
        //                     || euclidean_distance_squared(camera_pos, v.pos) < PLAYER_SIGHT_RANGE)
        //                 && {
        //                     let v_pos = Point3::new(*x as i32, *y as i32, *z as i32);
        //                     any_neighbour_empty(&voxel_array.view(), v_pos)
        //                 }
        //                 && (world_pos_to_index(try_ray_hitscan(
        //                     voxel_array.view(),
        //                     camera_pos,
        //                     v.pos,
        //                 )) == world_pos_to_index(v.pos)
        //                     || hitscan_tile(voxel_array.view(), camera_pos, v.pos).len() != 8
        //                     || nuke_lighting)
        //         })
        //         .map(|((_x, _y, _z), v)| {
        //             let mut new_v = v.clone();
        //             if nuke_lighting {
        //                 new_v.illumination = 1.0
        //             }
        //             new_v.illumination *= 1.0 - (0.5 * new_v.illumination.min(0.99)).max(0.01);
        //             new_v.illumination = (new_v.illumination - 0.01).max(0.0);
        //             new_v
        //         }),
        // );

        // self.nuke_lighting = false;

        // self.draw_voxels.sort_unstable_by(|a, b| {
        //     euclidean_distance_squared(b.pos, camera_pos)
        //         .partial_cmp(&euclidean_distance_squared(a.pos, camera_pos))
        //         .unwrap_or(Ordering::Equal)
        // });

        // //Copy back to our world
        // for new_vox in &mut self.draw_voxels {
        //     self.voxel_array[[
        //         new_vox.pos.x.floor() as usize,
        //         new_vox.pos.y.floor() as usize,
        //         new_vox.pos.z.floor() as usize,
        //     ]]
        //     .illumination = new_vox.illumination;
        // }

        // let mut lights: Vec<_> = self
        //     .draw_voxels
        //     .iter()
        //     .filter(|v| v.voxel_type.illuminates())
        //     .map(|v| Light {
        //         pos: Point3::new(v.pos.x + 0.5, v.pos.y + 0.5, v.pos.z + 0.5),
        //         facing: Point3::new(0.0, 1.0, 0.0),
        //         illumination: 0.25,
        //         range: 0.0,
        //     })
        //     .collect();

        // if is_in_array(self.voxel_array.view(), world_pos_to_index(camera_pos)) {
        //     let player_light = Light {
        //         pos: camera_pos,
        //         facing: gun_facing,
        //         illumination: 0.5,
        //         range: 24.0,
        //     };

        //     lights.push(player_light);
        //     // dbg!(&lights);

        //     if muzzle_flash {
        //         let muzzle_light = Light {
        //             pos: camera_pos,
        //             facing: gun_facing,
        //             illumination: 0.9,
        //             range: 0.0,
        //         };

        //         lights.push(muzzle_light);
        //     }
        // }

        // for light in lights {
        //     self.voxel_array[[
        //         light.pos.x.floor() as usize,
        //         light.pos.y.floor() as usize,
        //         light.pos.z.floor() as usize,
        //     ]]
        //     .illumination = 0.9;

        //     let light_target: Point3<f32> = Point3::origin() + (light.facing * light.range).coords;

        //     let light_deviance = self.light_noise.get([
        //         light.pos.x as f64,
        //         light.pos.y as f64,
        //         light.pos.z as f64,
        //         self.current_tic as f64 * 0.5,
        //     ]);

        //     for target_point in &self.lighting_sphere {
        //         let target_point_offset = Point3::new(
        //             target_point.x + light.pos.x + light_target.x,
        //             target_point.y + light.pos.y + light_target.y,
        //             target_point.z + light.pos.z + light_target.z,
        //         );

        //         let ray_hits =
        //             hitscan_tile(self.voxel_array.view(), light.pos, target_point_offset);

        //         for hit in ray_hits {
        //             if is_in_array(self.voxel_array.view(), world_pos_to_index(hit))
        //                 && world_pos_to_index(hit) != world_pos_to_index(target_point_offset)
        //             {
        //                 let hit_index = world_pos_to_index(hit);

        //                 let ray_voxel =
        //                     &mut self.voxel_array[[hit_index.x, hit_index.y, hit_index.z]];

        //                 ray_voxel.illumination = (ray_voxel.illumination
        //                     + (light.illumination
        //                         / euclidean_distance_squared(
        //                             ray_voxel.pos,
        //                             Point3::new(
        //                                 light.pos.x as f32,
        //                                 light.pos.y as f32,
        //                                 light.pos.z as f32,
        //                             ),
        //                         )
        //                         .max(1.0))
        //                     .powf(1.1)
        //                         * (light_deviance * 0.5 + 0.5) as f32)
        //                     .min(1.0);
        //             }
        //         }
        //     }
        // }

        self.current_tic += 1;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

        // Our object is translated along the x axis.
        let model = Isometry3::new(Vector3::x(), na::zero());

        // Our camera looks toward the point (1.0, 0.0, 0.0).
        // It is located at (0.0, 0.0, 1.0).
        let eye = self.camera_pos; //Point3::new(0.0, 0.0, 1.0);

        let rotation =
            Rotation3::from_euler_angles(self.camera_rotation.y, self.camera_rotation.x, 0.0);

        let rotation_offset = rotation.transform_point(&Point3::new(0.0, 0.0, 1.0));

        let target = Point3::new(
            self.camera_pos.x + rotation_offset.x,
            self.camera_pos.y + rotation_offset.y,
            self.camera_pos.z + rotation_offset.z,
        );
        // let target = Point3::new(0.0, 0.0, 0.0);
        let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());

        // A perspective projection.
        let projection = Perspective3::new(16.0 / 9.0, 3.14 / 2.0, 1.0, 1000.0);

        // The combination of the model with the view is still an isometry.
        let model_view = view * model;

        // Convert everything to a `Matrix4` so that they can be combined.
        let mat_model_view = model_view.to_homogeneous();

        // Combine everything.
        let model_view_projection = projection.as_matrix() * mat_model_view;

        let mut sprite_batch = SpriteBatch::new(self.font.texture.clone());

        for voxel in self.draw_voxels.iter() {
            if let Some(screen_pos) =
                Point3::from_homogeneous(model_view_projection * voxel.pos.to_homogeneous())
            {
                if screen_pos.z >= -1.0 && screen_pos.z <= 1.0 {
                    let color = voxel.voxel_type.get_color();
                    let color_darkness =
                        (1.0 - screen_pos.z.min(1.0).max(0.0)) * 0.25 + voxel.illumination * 0.75;
                    let color_back_darkness = color_darkness * 0.75;

                    let screen_dest = [
                        screen_pos.x * WINDOW_WIDTH / 2.0 + WINDOW_WIDTH / 2.0,
                        -screen_pos.y * WINDOW_HEIGHT / 2.0 + WINDOW_HEIGHT / 2.0, //We need to negate this, as 2d screen space is inverse of normalised device coords
                    ];

                    if !voxel.voxel_type.is_transparent() {
                        sprite_batch.add(DrawParam {
                            src: get_font_offset(0x2CF, &self.font),
                            dest: screen_dest.into(),
                            scale: [(1.0 - screen_pos.z) * 31.4, (1.0 - screen_pos.z) * 31.4]
                                .into(),
                            color: Color {
                                r: color.r * color_back_darkness,
                                g: color.g * color_back_darkness,
                                b: color.b * color_back_darkness,
                                a: 1.0,
                            },
                            offset: [0.5, 0.5].into(),
                            ..DrawParam::default()
                        });
                    }

                    sprite_batch.add(DrawParam {
                        src: voxel.voxel_type.get_char_offset(&self.font),
                        dest: screen_dest.into(),
                        scale: [(1.0 - screen_pos.z) * 31.4, (1.0 - screen_pos.z) * 31.4].into(),
                        color: Color {
                            r: color.r * color_darkness,
                            g: color.g * color_darkness,
                            b: color.b * color_darkness,
                            a: 1.0,
                        },
                        offset: [0.5, 0.5].into(),
                        ..DrawParam::default()
                    });
                }
            }
        }
        ggez::graphics::draw(ctx, &sprite_batch, DrawParam::default())?;

        let mut weapon_sprite_batch = SpriteBatch::new(self.font.texture.clone());

        rendering::util::draw_player_weapon(
            &mut weapon_sprite_batch,
            &self.font,
            model_view_projection,
            self.camera_pos,
            rotation,
            &self.player_gun_model,
            self.player_ads,
            self.player_gun_recoil,
            self.player_gun_rotation,
        );

        ggez::graphics::draw(ctx, &weapon_sprite_batch, DrawParam::default())?;

        graphics::present(ctx)
    }
}
