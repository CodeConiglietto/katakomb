use std::{
    cmp::Ordering,
    env,
    f32::consts::{FRAC_PI_4, PI},
    fs::File,
    io::BufReader,
    path::PathBuf,
    time::Duration,
};

use failure::Fallible;
use ggez::{
    // audio::{SoundData, Source, SoundSource},
    conf::WindowMode,
    event::{self, EventHandler, KeyCode},

    graphics::{self, spritebatch::SpriteBatch, DrawParam, FilterMode, Image},
    input::{keyboard, mouse},
    timer,
    Context,
    ContextBuilder,
    GameResult,
};
use log::info;
use na::{Isometry3, Perspective3, Point2, Point3, Rotation3, Vector3};
use ndarray::arr2;
use ndarray::prelude::*;
use noise::{OpenSimplex, Perlin, Seedable, Value, Worley};
use rand::prelude::*;
use rayon::prelude::*;
use rodio::{OutputStream, Source};
use structopt::StructOpt;

use crate::{
    constants::*,
    generation::world::*,
    geometry::util::*,
    rendering::{drawable::Drawable, font::*, light::*, tile::*},
    util::*,
    world::util::*,
};

mod audio;
mod constants;
mod editor;
mod generation;
mod geometry;
mod rendering;
pub mod ui;
mod util;
mod world;

#[derive(StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    mode: Option<Mode>,
}

#[derive(StructOpt)]
enum Mode {
    Main,
    Editor,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Main
    }
}

fn main() -> Fallible<()> {
    let opts = Opts::from_args();

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S%.3f]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .level_for("gfx_device_gl", log::LevelFilter::Warn)
        .level_for("winit", log::LevelFilter::Info)
        .level_for("gilrs", log::LevelFilter::Warn)
        .level_for("ggez", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;

    let mut cb = ContextBuilder::new("Katakomb", "CodeBunny");

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest_dir);
        path.push("resources");
        info!("Adding cargo resource path: '{:?}'", path);
        cb = cb.add_resource_path(path);
    }

    let (mut ctx, mut event_loop) = cb
        .window_mode(WindowMode::default().dimensions(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build()
        .expect("Could not create ggez context!");

    match opts.mode.unwrap_or_default() {
        Mode::Main => {
            let mut handler = Katakomb::new(&mut ctx)?;
            event::run(ctx, event_loop, handler);
        }
        Mode::Editor => {
            let mut handler = editor::Editor::new(&mut ctx)?;
            event::run(ctx, event_loop, handler);
        }
    }

    Ok(())
}

struct Katakomb {
    blank_texture: Image,
    lighting_sphere: Vec<Point3<f32>>,
    font: KataFont,
    tile_array: Array3<Tile>,
    draw_tiles: Vec<Tile>,
    camera_pos: Point3<f32>,

    camera_rotation: Point2<f32>,

    nuke_lighting: bool,

    current_tic: u64,

    lights: Vec<Light>,
    light_noise: OpenSimplex,

    player_gun_model: Array2<TileType>,
    player_gun_timer: u8,
    // player_gun_sound: SoundData,
    player_ads: f32,
    player_gun_recoil: f32,
    player_gun_rotation: Point2<f32>,
    // sound_queue: Vec<(f64, Source)>,
}

impl Katakomb {
    pub fn new(ctx: &mut Context) -> Fallible<Self> {
        // Load/create resources such as images here.
        let noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());
        let meta_noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());

        let chunk_gen_package = ChunkGenPackage {
            simplex: OpenSimplex::new().set_seed(thread_rng().gen::<u32>()),
            simplex_weight: Value::new().set_seed(thread_rng().gen::<u32>()),
            perlin: Perlin::new().set_seed(thread_rng().gen::<u32>()),
            perlin_weight: Value::new().set_seed(thread_rng().gen::<u32>()),
            // worley: Worley::new().set_seed(thread_rng().gen::<u32>()),
            // worley_weight: Value::new().set_seed(thread_rng().gen::<u32>()),
            value: Value::new().set_seed(thread_rng().gen::<u32>()),
            value_weight: Value::new().set_seed(thread_rng().gen::<u32>()),
        };

        graphics::set_default_filter(ctx, FilterMode::Nearest);

        use crate::rendering::tile::TileType::*;

        Ok(Self {
            blank_texture: Image::solid(ctx, 1, graphics::Color::WHITE).unwrap(),
            lighting_sphere: calculate_sphere_surface(LIGHT_RANGE),
            font: KataFont::load(ctx)?,
            tile_array: generate_chunk(Point3::new(0, 0, 0), &chunk_gen_package),
            draw_tiles: Vec::new(),
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
        })
    }
}

//Tries to fire a bresenham hitscan, returns dest if no collisions
fn try_bresenham_hitscan(
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
fn try_ray_hitscan(
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

fn get_light_hitscans(
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

fn get_tile_at(pos: Point3<f32>, tile_array: &Array3<Tile>) -> Tile {
    let index = world_pos_to_index(pos);

    tile_array[[index.x, index.y, index.z]].clone()
}

impl EventHandler<ggez::GameError> for Katakomb {
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
            // if mouse::button_pressed(ctx, mouse::MouseButton::Left) {
            //     self.player_gun_recoil = (self.player_gun_recoil + 0.2).min(1.0);
            //     self.player_gun_rotation.x = (self.player_gun_rotation.x
            //         + (thread_rng().gen::<f32>() - 0.5) * 0.05)
            //         .min(1.0)
            //         .max(-1.0);
            //     self.player_gun_rotation.y = (self.player_gun_rotation.y + 0.05).min(1.0);

            //     // dbg!(std::env::current_dir().unwrap().to_str().unwrap());
            //     let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            //     // let device = rodio::default_output_device().unwrap();
            //     let file = File::open(r"resources/gunshot.wav").unwrap();
            //     let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

            //     let mut echo_distances = Vec::new();

            //     for cube_point in get_cube_points(Point3::new(-0.5, -0.5, -0.5)) {
            //         let ray_target = self.camera_pos + (cube_point.coords * MAX_SOUND_RANGE * 2.0);

            //         if is_in_array(self.tile_array.view(), world_pos_to_index(ray_target)) {
            //             let ray_hit = try_bresenham_hitscan(
            //                 self.tile_array.view(),
            //                 world_pos_to_int(self.camera_pos),
            //                 world_pos_to_int(ray_target),
            //             );

            //             if ray_hit != world_pos_to_int(ray_target) {
            //                 // //TODO mess with this
            //                 let hit_distance = euclidean_distance_squared(
            //                     self.camera_pos,
            //                     Point3::new(ray_hit.x as f32, ray_hit.y as f32, ray_hit.z as f32),
            //                 )
            //                 .sqrt();
            //                 let hit_distance_ratio = hit_distance / (MAX_SOUND_RANGE * 2.0);
            //                 let hit_distance_ratio_squared = hit_distance * hit_distance;
            //                 echo_distances.push(hit_distance_ratio);
            //                 // let mut source = Source::from_data(ctx, self.player_gun_sound.clone()).unwrap();
            //                 // source.set_pitch(0.5 + 0.5 * (1.0 - hit_distance_ratio));
            //                 // source.set_fade_in(Duration::from_millis((hit_distance_ratio_squared) as u64));
            //                 // //source.set_volume(1.0 - (hit_distance_ratio * 0.5));
            //                 // self.sound_queue.push((update_time + (hit_distance_ratio * 0.5) as f64, source));
            //                 // //TODO take average of hit distances and use that to change the non-ray sound's pitch
            //             }
            //         }
            //     }

            //     echo_distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
            //     let min_echo_distance = echo_distances.first().unwrap();
            //     // let med_echo_distance = echo_distances[echo_distances.len() /2 ];
            //     let max_echo_distance = echo_distances.last().unwrap();

            //     //warning: using more than 2 reverbs leads to very unpleasant results :<
            //     stream_handle.play_raw(
            //         source
            //             .convert_samples::<i16>()
            //             .buffered()
            //             .reverb(
            //                 Duration::from_millis((min_echo_distance * 1000.0) as u64),
            //                 0.5 - min_echo_distance * 0.5,
            //             )
            //             // .reverb(
            //             //     Duration::from_millis((med_echo_distance * 750.0) as u64),
            //             //     0.5 - med_echo_distance * 0.5,
            //             // )
            //             .reverb(
            //                 Duration::from_millis((max_echo_distance * 1250.0) as u64),
            //                 0.25 - max_echo_distance * 0.25,
            //             )
            //             .convert_samples(),
            //     );

            //     muzzle_flash = true;
            //     self.player_gun_timer = 12;
            // }
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
            self.camera_rotation.x += 0.05;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Right) {
            self.camera_rotation.x -= 0.05;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Up) {
            self.camera_rotation.y -= 0.05;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Down) {
            self.camera_rotation.y += 0.05;
        }

        if keyboard::is_key_pressed(ctx, KeyCode::N) {
            self.nuke_lighting = true;
        }

        self.draw_tiles.clear();

        //let tile_points = self.tile_draw_points;
        let camera_pos = self.camera_pos;

        // let tile_array = &self.tile_array;
        // let zip_iter = ndarray::Zip::indexed(tile_array);

        // let int_camera_pos = Point3::new(
        //     camera_pos.x.floor() as i32,
        //     camera_pos.y.floor() as i32,
        //     camera_pos.z.floor() as i32,
        // );

        // let usize_camera_pos = Point3::new(
        //     camera_pos.x.floor() as usize,
        //     camera_pos.y.floor() as usize,
        //     camera_pos.z.floor() as usize,
        // );

        let light_sources = [
            Point3::new(CHUNK_SIZE / 2, CHUNK_SIZE / 2, CHUNK_SIZE / 2),
            Point3::new(
                camera_pos.x.floor() as usize,
                camera_pos.y.floor() as usize,
                camera_pos.z.floor() as usize,
            ),
        ];

        //NEW SHITTY IMPLEMENTATION
        self.tile_array
            .par_iter_mut()
            .for_each(|tile| tile.illumination = 0.0);

        for light_pos in light_sources.iter() {
            if is_in_array(self.tile_array.view(), world_pos_to_index(camera_pos)) {
                let light_distance = LIGHT_RANGE;

                let (tiles_width, tiles_height, tiles_depth) = self.tile_array.dim();

                let light_left = light_pos.x.saturating_sub(light_distance);
                let light_right = (light_pos.x + light_distance).min(tiles_width);
                let light_bottom = light_pos.y.saturating_sub(light_distance);
                let light_top = (light_pos.y + light_distance).min(tiles_height);
                let light_back = light_pos.z.saturating_sub(light_distance);
                let light_front = (light_pos.z + light_distance).min(tiles_depth);

                let light_cube = self.tile_array.slice_mut(s![
                    light_left..light_right,
                    light_bottom..light_top,
                    light_back..light_front,
                ]);

                let mid_x = light_pos.x - light_left;
                let mid_y = light_pos.y - light_bottom;
                let mid_z = light_pos.z - light_back;

                let (bx, tx) = light_cube.split_at(Axis(0), mid_x);

                let (bxby, bxty) = bx.split_at(Axis(1), mid_y);
                let (txby, txty) = tx.split_at(Axis(1), mid_y);

                let (bxbybz, bxbytz) = bxby.split_at(Axis(2), mid_z);
                let (bxtybz, bxtytz) = bxty.split_at(Axis(2), mid_z);
                let (txbybz, txbytz) = txby.split_at(Axis(2), mid_z);
                let (txtybz, txtytz) = txty.split_at(Axis(2), mid_z);

                let mut octs = [
                    (bxbybz, (false, false, false)),
                    (bxbytz, (false, false, true)),
                    (bxtybz, (false, true, false)),
                    (bxtytz, (false, true, true)),
                    (txbybz, (true, false, false)),
                    (txbytz, (true, false, true)),
                    (txtybz, (true, true, false)),
                    (txtytz, (true, true, true)),
                ];

                octs.par_iter_mut()
                    .for_each(|o| shadowcast_octant(o.0.view_mut(), o.1));
                // octs.iter_mut().for_each(|o| shadowcast_octant(o.0.view_mut(), o.1));
            }
        }
        self.draw_tiles.clear();

        self.draw_tiles.par_extend(
            self.tile_array
                .par_iter()
                .filter(|tile| tile.illumination > 0.0)
                .cloned(),
        );

        //OLD SHITTY IMPLEMENTATION
        // let nuke_lighting = self.nuke_lighting;

        // //populate tiles to draw

        // let tile_array_view = self.tile_array.view();

        // self.lights = self
        //     .lights
        //     .par_iter()
        //     .filter(|light| light.persistent)
        //     .cloned()
        //     .collect();

        // if is_in_array(self.tile_array.view(), world_pos_to_index(camera_pos)) {
        //     let player_light = Light {
        //         pos: camera_pos,
        //         facing: gun_facing,
        //         illumination: 0.5,
        //         range: 16.0,
        //         persistent: false,
        //     };

        //     self.lights = self
        //         .lights
        //         .par_iter()
        //         .filter(|light| light.persistent)
        //         .cloned()
        //         .collect();

        //     if is_in_array(self.tile_array.view(), world_pos_to_index(camera_pos)) {
        //         let player_light = Light {
        //             pos: camera_pos,
        //             facing: gun_facing,
        //             illumination: 0.5,
        //             range: 24.0,
        //             persistent: false,
        //         };

        //         self.lights.push(player_light);
        //         // dbg!(&lights);

        //         if muzzle_flash {
        //             let muzzle_light = Light {
        //                 pos: camera_pos,
        //                 facing: gun_facing,
        //                 illumination: 0.9,
        //                 range: 0.0,
        //                 persistent: false,
        //             };

        //             self.lights.push(muzzle_light);
        //         }
        //     }

        //     let light_iter = self.lights.par_iter();
        //     let mut draw_tiles: Vec<_> = light_iter
        //         .flat_map(|light| get_light_hitscans(light, &self.lighting_sphere, tile_array_view))
        //         .map(|pos| {
        //             let mut tile = get_tile_at(pos, &self.tile_array).clone();
        //             tile.illumination = 0.5; //euclidean_distance_squared(tile.pos, light.pos) / (LIGHT_RANGE * LIGHT_RANGE) as f32;
        //             tile
        //         })
        //         .collect();

        //     draw_tiles = draw_tiles
        //         .par_iter()
        //         .filter(|tile| {
        //             any_neighbour_empty(&tile_array.view(), world_pos_to_int(tile.pos))
        //                 && (nuke_lighting || world_pos_to_index(try_ray_hitscan(
        //                     tile_array.view(),
        //                     camera_pos,
        //                     tile.pos,
        //                 )) == world_pos_to_index(tile.pos))
        //         })
        //         .cloned()
        //         .collect();

        //     draw_tiles.sort_unstable_by(|a, b| {
        //         euclidean_distance_squared(b.pos, camera_pos)
        //             .partial_cmp(&euclidean_distance_squared(a.pos, camera_pos))
        //             .unwrap_or(Ordering::Equal)
        //     });

        //     draw_tiles.dedup_by(|a, b| {
        //         let equal = world_pos_to_index(a.pos) == world_pos_to_index(b.pos);
        //         if equal {
        //             b.illumination = (b.illumination + 0.01).min(1.0)
        //         };
        //         if b.illumination > 1.0 {
        //             panic!()
        //         };
        //         equal
        //     });

        //     std::mem::swap(&mut draw_tiles, &mut self.draw_tiles);

        //     // self.draw_tiles.clear();
        //     // self.draw_tiles.par_extend(
        //     //     zip_iter
        //     //         .into_par_iter()
        //     //         .filter(|((x, y, z), v)| {
        //     //             (!v.tile_type.is_transparent() || v.tile_type.illuminates())
        //     //                 && (v.illumination > 0.01
        //     //                     || v.tile_type.illuminates()
        //     //                     || euclidean_distance_squared(camera_pos, v.pos) < PLAYER_SIGHT_RANGE)
        //     //                 && {
        //     //                     let v_pos = Point3::new(*x as i32, *y as i32, *z as i32);
        //     //                     any_neighbour_empty(&tile_array.view(), v_pos)
        //     //                 }
        //     //                 && (world_pos_to_index(try_ray_hitscan(
        //     //                     tile_array.view(),
        //     //                     camera_pos,
        //     //                     v.pos,
        //     //                 )) == world_pos_to_index(v.pos)
        //     //                     || hitscan_tile(tile_array.view(), camera_pos, v.pos).len() != 8
        //     //                     || nuke_lighting)
        //     //         })
        //     //         .map(|((_x, _y, _z), v)| {
        //     //             let mut new_v = v.clone();
        //     //             if nuke_lighting {
        //     //                 new_v.illumination = 1.0
        //     //             }
        //     //             new_v.illumination *= 1.0 - (0.5 * new_v.illumination.min(0.99)).max(0.01);
        //     //             new_v.illumination = (new_v.illumination - 0.01).max(0.0);
        //     //             new_v
        //     //         }),
        //     // );

        //     // self.nuke_lighting = false;

        self.draw_tiles.sort_unstable_by(|a, b| {
            euclidean_distance_squared(b.pos, camera_pos)
                .partial_cmp(&euclidean_distance_squared(a.pos, camera_pos))
                .unwrap_or(Ordering::Equal)
        });

        //     // //Copy back to our world
        //     // for new_tile in &mut self.draw_tiles {
        //     //     self.tile_array[[
        //     //         new_tile.pos.x.floor() as usize,
        //     //         new_tile.pos.y.floor() as usize,
        //     //         new_tile.pos.z.floor() as usize,
        //     //     ]]
        //     //     .illumination = new_tile.illumination;
        //     // }

        //     // let mut lights: Vec<_> = self
        //     //     .draw_tiles
        //     //     .iter()
        //     //     .filter(|v| v.tile_type.illuminates())
        //     //     .map(|v| Light {
        //     //         pos: Point3::new(v.pos.x + 0.5, v.pos.y + 0.5, v.pos.z + 0.5),
        //     //         facing: Point3::new(0.0, 1.0, 0.0),
        //     //         illumination: 0.25,
        //     //         range: 0.0,
        //     //     })
        //     //     .collect();

        //     // if is_in_array(self.tile_array.view(), world_pos_to_index(camera_pos)) {
        //     //     let player_light = Light {
        //     //         pos: camera_pos,
        //     //         facing: gun_facing,
        //     //         illumination: 0.5,
        //     //         range: 24.0,
        //     //     };

        //     //     lights.push(player_light);
        //     //     // dbg!(&lights);

        //     //     if muzzle_flash {
        //     //         let muzzle_light = Light {
        //     //             pos: camera_pos,
        //     //             facing: gun_facing,
        //     //             illumination: 0.9,
        //     //             range: 0.0,
        //     //         };

        //     //         lights.push(muzzle_light);
        //     //     }
        //     // }

        //     // for light in lights {
        //     //     self.tile_array[[
        //     //         light.pos.x.floor() as usize,
        //     //         light.pos.y.floor() as usize,
        //     //         light.pos.z.floor() as usize,
        //     //     ]]
        //     //     .illumination = 0.9;

        //     //     let light_target: Point3<f32> = Point3::origin() + (light.facing * light.range).coords;

        //     //     let light_deviance = self.light_noise.get([
        //     //         light.pos.x as f64,
        //     //         light.pos.y as f64,
        //     //         light.pos.z as f64,
        //     //         self.current_tic as f64 * 0.5,
        //     //     ]);

        //     //     for target_point in &self.lighting_sphere {
        //     //         let target_point_offset = Point3::new(
        //     //             target_point.x + light.pos.x + light_target.x,
        //     //             target_point.y + light.pos.y + light_target.y,
        //     //             target_point.z + light.pos.z + light_target.z,
        //     //         );

        //     //         let ray_hits =
        //     //             hitscan_tile(self.tile_array.view(), light.pos, target_point_offset);

        //     //         for hit in ray_hits {
        //     //             if is_in_array(self.tile_array.view(), world_pos_to_index(hit))
        //     //                 && world_pos_to_index(hit) != world_pos_to_index(target_point_offset)
        //     //             {
        //     //                 let hit_index = world_pos_to_index(hit);

        //     //                 let ray_tile =
        //     //                     &mut self.tile_array[[hit_index.x, hit_index.y, hit_index.z]];

        //     //                 ray_tile.illumination = (ray_tile.illumination
        //     //                     + (light.illumination
        //     //                         / euclidean_distance_squared(
        //     //                             ray_tile.pos,
        //     //                             Point3::new(
        //     //                                 light.pos.x as f32,
        //     //                                 light.pos.y as f32,
        //     //                                 light.pos.z as f32,
        //     //                             ),
        //     //                         )
        //     //                         .max(1.0))
        //     //                     .powf(1.1)
        //     //                         * (light_deviance * 0.5 + 0.5) as f32)
        //     //                     .min(1.0);
        //     //             }
        //     //         }
        //     //     }
        //     // }
        // }
        self.current_tic += 1;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::Color::BLACK);

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

        let mut sprite_batch = SpriteBatch::new(self.font.texture().clone());

        for tile in self.draw_tiles.iter() {
            if let Some(screen_pos) =
                Point3::from_homogeneous(model_view_projection * tile.pos.to_homogeneous())
            {
                if screen_pos.z >= -1.0 && screen_pos.z <= 1.0 {
                    let color = tile.tile_type.get_color();
                    let color_darkness =
                        (1.0 - screen_pos.z.min(1.0).max(0.0)) * 0.25 + tile.illumination * 0.75;
                    let color_back_darkness = color_darkness * 0.75;

                    let screen_dest = [
                        screen_pos.x * WINDOW_WIDTH / 2.0 + WINDOW_WIDTH / 2.0,
                        -screen_pos.y * WINDOW_HEIGHT / 2.0 + WINDOW_HEIGHT / 2.0, //We need to negate this, as 2d screen space is inverse of normalised device coords
                    ];

                    if !tile.tile_type.is_transparent() {
                        sprite_batch.add(
                            DrawParam::new()
                                .src(self.font.get_src_rect(0x2CF))
                                .dest(screen_dest)
                                .scale([
                                    (1.0 - screen_pos.z) * PI * 10.0,
                                    (1.0 - screen_pos.z) * PI * 10.0,
                                ])
                                .color(graphics::Color {
                                    r: color.r * color_back_darkness,
                                    g: color.g * color_back_darkness,
                                    b: color.b * color_back_darkness,
                                    a: 1.0,
                                })
                                .offset([0.5, 0.5]), // ..DrawParam::default()
                        );
                    }

                    sprite_batch.add(
                        DrawParam::new()
                            .src(tile.tile_type.get_char_offset(&self.font))
                            .dest(screen_dest)
                            .scale([
                                (1.0 - screen_pos.z) * PI * 10.0,
                                (1.0 - screen_pos.z) * PI * 10.0,
                            ])
                            .color(graphics::Color {
                                r: color.r * color_darkness,
                                g: color.g * color_darkness,
                                b: color.b * color_darkness,
                                a: 1.0,
                            })
                            .offset([0.5, 0.5]), // ..DrawParam::default()
                    );
                }
            }
        }
        ggez::graphics::draw(ctx, &sprite_batch, DrawParam::default())?;

        let mut weapon_sprite_batch = SpriteBatch::new(self.font.texture().clone());

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

fn shadowcast_octant(mut slice: ArrayViewMut3<Tile>, (x_sign, y_sign, z_sign): (bool, bool, bool)) {
    if !x_sign {
        slice.invert_axis(Axis(0));
    }
    if !y_sign {
        slice.invert_axis(Axis(1));
    }
    if !z_sign {
        slice.invert_axis(Axis(2));
    }

    let total_len = slice.dim().0 + slice.dim().1 + slice.dim().2;

    for i in 0..3 {
        let mut permuted_slice = slice
            .view_mut()
            .permuted_axes((i, (i + 1) % 3, (i + 2) % 3));

        scan_recursive_shadowcast(permuted_slice);
        // iterate_recursive_shadowcast(permuted_slice, 0.0, FRAC_PI_4, 0.0, FRAC_PI_4, 0);

        // let pslice_width = permuted_slice.dim().0;
        // let pslice_height = permuted_slice.dim().1;

        // for (z, mut sub_slice) in permuted_slice.axis_iter_mut(Axis(2)).enumerate() {
        //     for ((x, y), tile) in sub_slice.slice_mut(s![..z.min(pslice_width), ..z.min(pslice_height)]).indexed_iter_mut() {
        //         // tile.illumination = 1.0 - ((x + y + z) as f32 / total_len as f32);
        //         tile.illumination = 1.0 / z as f32;
        //     }
        // }
    }
}

#[derive(Debug)]
struct Shadowcast {
    left_angle: f32,
    right_angle: f32,
    top_angle: f32,
    bottom_angle: f32,
    z: usize,
}

fn scan_recursive_shadowcast(mut slice: ArrayViewMut3<Tile>) {
    let mut frontier = Vec::new();

    frontier.push(Shadowcast {
        left_angle: 0.0,
        right_angle: FRAC_PI_4,
        top_angle: 0.0,
        bottom_angle: FRAC_PI_4,
        z: 0,
    });

    let (slice_width, slice_height, slice_depth) = slice.dim();

    while let Some(current) = frontier.pop() {
        let left = ((current.z + 1) as f32 * current.left_angle.tan()).floor() as usize;
        let right = (((current.z + 1) as f32 * current.right_angle.tan()).ceil() as usize)
            .min(slice_width.saturating_sub(1));
        let top = ((current.z + 1) as f32 * current.top_angle.tan()).floor() as usize;
        let bottom = (((current.z + 1) as f32 * current.bottom_angle.tan()).ceil() as usize)
            .min(slice_height.saturating_sub(1));

        let mut last_top = None;

        'y_loop: for y in top..=bottom {
            let mut last_left = None;

            for x in left..=right {
                let dist_from_center = EUCLIDEAN_DISTANCE_LOOKUP[[x, y, current.z]];
                let outside_range = dist_from_center >= LIGHT_RANGE as f32;

                if outside_range {
                    if current.z < slice_depth - 1 {
                        // At the end of each row, we check if there's any clear tiles
                        if let Some(last_left) = last_left {
                            frontier.push(Shadowcast {
                                left_angle: ATAN_CASTING_LOOKUP[[last_left, current.z]],
                                //(last_left as f32 / (current.z + 1) as f32).atan(),
                                right_angle: ATAN_CASTING_LOOKUP[[x, current.z]],
                                top_angle: ATAN_CASTING_LOOKUP[[y, current.z]],
                                //(y as f32 / (current.z + 1) as f32).atan(),
                                bottom_angle: ATAN_CASTING_LOOKUP[[y + 1usize, current.z]],
                                //((y + 1usize) as f32 / (current.z + 1) as f32).atan(),
                                z: current.z + 1,
                            });
                        }

                        if let Some(last_top) = last_top.take() {
                            frontier.push(Shadowcast {
                                left_angle: current.left_angle,
                                right_angle: current.right_angle,
                                top_angle: ATAN_CASTING_LOOKUP[[last_top, current.z]],
                                //(last_top as f32 / (current.z + 1) as f32).atan(),
                                bottom_angle: ATAN_CASTING_LOOKUP[[y, current.z]],

                                z: current.z + 1,
                            });
                        }
                    }   

                    continue 'y_loop;
                }

                let tile = &mut slice[[x, y, current.z]];

                tile.illumination = tile
                    .illumination
                    .max(1.0 - (dist_from_center / LIGHT_RANGE as f32).min(1.0));

                // If we're on the last layer, we don't worry about bookkeeping for recursion
                if current.z < slice_depth - 1 {
                    if tile.tile_type.is_transparent() {
                        last_left = Some(last_left.unwrap_or(x));
                    } else {
                        let tile_top_angle = ATAN_CASTING_LOOKUP[[y, current.z]];
                        //(y as f32 / (current.z + 1) as f32).atan();
                        let tile_bottom_angle = ATAN_CASTING_LOOKUP[[y + 1usize, current.z]];
                        //((y + 1usize) as f32 / (current.z + 1) as f32).atan();
                        let tile_left_angle = ATAN_CASTING_LOOKUP[[x, current.z]];
                        //(x as f32 / (current.z + 1) as f32).atan();

                        if let Some(last_top) = last_top.take() {
                            frontier.push(Shadowcast {
                                left_angle: current.left_angle,
                                right_angle: current.right_angle,
                                top_angle: ATAN_CASTING_LOOKUP[[last_top, current.z]],
                                //(last_top as f32 / (current.z + 1) as f32).atan(),
                                bottom_angle: tile_top_angle,

                                z: current.z + 1,
                            });
                        }

                        if let Some(last_left) = last_left.take() {
                            frontier.push(Shadowcast {
                                left_angle: ATAN_CASTING_LOOKUP[[last_left, current.z]],
                                //(last_left as f32 / (current.z + 1) as f32).atan(),
                                right_angle: tile_left_angle,
                                top_angle: tile_top_angle,
                                bottom_angle: tile_bottom_angle,
                                z: current.z + 1,
                            });
                        }
                    }
                }
            }

            // At the end of each row, we check if there's any clear tiles
            if let Some(last_left) = last_left {
                if current.z < slice_depth - 1 {
                    if last_left == left {
                        // The whole row is clear
                        last_top = Some(last_top.unwrap_or(y));
                    } else {
                        frontier.push(Shadowcast {
                            left_angle: ATAN_CASTING_LOOKUP[[last_left, current.z]],
                            //(last_left as f32 / (current.z + 1) as f32).atan(),
                            right_angle: current.right_angle,
                            top_angle: ATAN_CASTING_LOOKUP[[y, current.z]],
                            //(y as f32 / (current.z + 1) as f32).atan(),
                            bottom_angle: ATAN_CASTING_LOOKUP[[y + 1usize, current.z]],
                            //((y + 1usize) as f32 / (current.z + 1) as f32).atan(),
                            z: current.z + 1,
                        });
                    }
                }
            }
        }

        // At the end of each scan, we check if there's any clear rows
        if let Some(last_top) = last_top {
            if current.z < slice_depth - 1 {
                if last_top == top {
                    // The whole scan is clear
                    frontier.push(Shadowcast {
                        left_angle: current.left_angle,
                        right_angle: current.right_angle,
                        top_angle: current.top_angle,
                        bottom_angle: current.bottom_angle,

                        z: current.z + 1,
                    });
                } else {
                    frontier.push(Shadowcast {
                        left_angle: current.left_angle,
                        right_angle: current.right_angle,
                        top_angle: ATAN_CASTING_LOOKUP[[last_top, current.z]],
                        //(last_top as f32 / (current.z + 1) as f32).atan(),
                        bottom_angle: current.top_angle,

                        z: current.z + 1,
                    });
                }
            }
        }
    }
}

// fn iterate_recursive_shadowcast(mut slice: ArrayViewMut3<Tile>, top_angle: f32, bottom_angle: f32, left_angle: f32, right_angle: f32, z: usize) {
//     let slice_width = slice.dim().0;
//     let slice_height = slice.dim().1;

//     let left = ((z + 1) as f32 * left_angle.tan()).floor() as usize;
//     let right = ((z + 1) as f32 * right_angle.tan()).ceil() as usize;
//     let top = ((z + 1) as f32 * top_angle.tan()).floor() as usize;
//     let bottom = ((z + 1) as f32 * bottom_angle.tan()).ceil() as usize;

//     // dbg!(left, right, top, bottom);

//     if !slice.is_empty() {
//         let (mut selected, mut remainder) = slice.split_at(Axis(2), 1);

//         for ((x, y), tile) in selected.slice_mut(s![left..=right.min(slice_width.saturating_sub(1)), top..=bottom.min(slice_height.saturating_sub(1)), 0]).indexed_iter_mut() {
//             tile.illumination = 1.0 / z as f32;

//             if tile.tile_type.is_transparent() {
//                 iterate_recursive_shadowcast(
//                     remainder.view_mut(),
//                     (y as f32 / (z + 1) as f32).atan(),
//                     ((y + 1usize) as f32 / (z + 1) as f32).atan(),
//                     (x as f32 / (z + 1) as f32).atan(),
//                     ((x + 1usize) as f32 / (z + 1) as f32).atan(),
//                     z + 1
//                 );
//             }
//         }
//     }
// }
