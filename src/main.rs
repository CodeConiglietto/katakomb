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

use rodio::{Sink, Source};

use na::{Isometry3, Perspective3, Point2, Point3, Rotation3, Vector3};

use ndarray::arr2;
use ndarray::prelude::*;
use noise::{NoiseFn, OpenSimplex, Seedable};
use rand::prelude::*;
use rayon::prelude::*;

use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use std::{cmp::Ordering, env, path::PathBuf};

mod editor;

const WINDOW_WIDTH: f32 = 1600.0;
const WINDOW_HEIGHT: f32 = 900.0;
const CHUNK_SIZE: usize = 32;
const LIGHT_RANGE: i32 = 12;
const PLAYER_SIGHT_RANGE: f32 = 12.0;
const MAX_SOUND_RANGE: f32 = 12.0;

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

fn load_font(ctx: &mut Context) -> KataFont {
    let texture = image::open(r"C:\Users\admin\Documents\katakomb\resources\master8x8.png")
        .unwrap()
        .to_rgba();

    KataFont {
        texture: Image::from_rgba8(
            ctx,
            texture.width() as u16,
            texture.height() as u16,
            &texture.into_raw(),
        )
        .unwrap(),
        char_width: 8,
        char_height: 8,
    }
}

fn world_pos_to_index(pos: Point3<f32>) -> Point3<usize> {
    Point3::new(
        pos.x.floor() as usize,
        pos.y.floor() as usize,
        pos.z.floor() as usize,
    )
}

fn world_pos_to_int(pos: Point3<f32>) -> Point3<i32> {
    Point3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    )
}

fn is_in_array(array: ArrayView3<Voxel>, pos: Point3<usize>) -> bool {
    pos.x >= 0
        && pos.y >= 0
        && pos.z >= 0
        && pos.x < array.dim().0
        && pos.y < array.dim().1
        && pos.z < array.dim().2
}

fn calculate_bresenham(p1: Point3<i32>, p2: Point3<i32>) -> Vec<Point3<i32>> {
    let mut line = Vec::new();

    let mut p = Point3::new(p1.x, p1.y, p1.z);

    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let dz = p2.z - p1.z;
    let x_inc = if dx < 0 { -1 } else { 1 };
    let l = dx.abs();
    let y_inc = if dy < 0 { -1 } else { 1 };
    let m = dy.abs();
    let z_inc = if dz < 0 { -1 } else { 1 };
    let n = dz.abs();
    let dx2 = l << 1;
    let dy2 = m << 1;
    let dz2 = n << 1;

    if l >= m && l >= n {
        let mut err_1 = dy2 - l;
        let mut err_2 = dz2 - l;
        for _i in 0..l {
            line.push(p.clone());
            if err_1 > 0 {
                p.y += y_inc;
                err_1 -= dx2;
            }
            if err_2 > 0 {
                p.z += z_inc;
                err_2 -= dx2;
            }
            err_1 += dy2;
            err_2 += dz2;
            p.x += x_inc;
        }
    } else if m >= l && m >= n {
        let mut err_1 = dx2 - m;
        let mut err_2 = dz2 - m;
        for _i in 0..m {
            line.push(p.clone());
            if err_1 > 0 {
                p.x += x_inc;
                err_1 -= dy2;
            }
            if err_2 > 0 {
                p.z += z_inc;
                err_2 -= dy2;
            }
            err_1 += dx2;
            err_2 += dz2;
            p.y += y_inc;
        }
    } else {
        let mut err_1 = dy2 - n;
        let mut err_2 = dx2 - n;
        for _i in 0..n {
            line.push(p.clone());
            if err_1 > 0 {
                p.y += y_inc;
                err_1 -= dz2;
            }
            if err_2 > 0 {
                p.x += x_inc;
                err_2 -= dz2;
            }
            err_1 += dy2;
            err_2 += dx2;
            p.z += z_inc;
        }
    }
    line.push(p.clone());

    line
}

fn calculate_sphere_surface(radius: i32) -> Vec<Point3<f32>> {
    let mut points = Vec::new();

    let origin = Point3::origin();

    for x in -radius..radius {
        for y in -radius..radius {
            for z in -radius..radius {
                let point = Point3::new(x as f32, y as f32, z as f32);

                //DISGOSDANG
                if euclidean_distance_squared(origin, point).sqrt().floor() as i32 == radius {
                    points.push(point);
                }
            }
        }
    }

    points
}

fn calculate_sphere(radius: i32) -> Vec<Point3<f32>> {
    let mut points = Vec::new();

    let origin = Point3::origin();

    for x in -radius..radius {
        for y in -radius..radius {
            for z in -radius..radius {
                let point = Point3::new(x as f32, y as f32, z as f32);

                //DISGOSDANG
                if euclidean_distance_squared(origin, point).sqrt().floor() as i32 <= radius {
                    points.push(point);
                }
            }
        }
    }

    points
}

fn any_neighbour_empty(array: &ArrayView3<Voxel>, pos: Point3<i32>) -> bool {
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
                        .voxel_type
                        .is_transparent()
                {
                    return true;
                }
            }
        }
    }

    false
}

pub trait Drawable {
    fn get_char_offset(&self, font: &KataFont) -> Rect;
    fn get_color(&self) -> Color;
    fn is_transparent(&self) -> bool;
    fn illuminates(&self) -> bool;
    fn rotation(&self) -> f32;
}

pub struct Light {
    pos: Point3<f32>,
    facing: Point3<f32>,
    illumination: f32,
    range: f32,
}

#[derive(Debug, PartialEq, Clone)]
enum VoxelType {
    Air,
    Rock,
    Mushroom,
    Candle,
    FrontSight,
    RearSight,
    Barrel,
    BarrelEnd,
    GasBlock,
    RecUpper,
    RecLower,
    RecLowerHalf,
    RecLowerBack,
    Magazine,
    Stock,
    StockUpper,
    Grip,
}

impl Drawable for VoxelType {
    fn get_char_offset(&self, font: &KataFont) -> Rect {
        match self {
            VoxelType::Air => get_font_offset(0, font),
            VoxelType::Rock => get_font_offset(0xB1, font),
            VoxelType::Mushroom => get_font_offset(0x2E1, font),
            VoxelType::Candle => get_font_offset(0x21A, font),
            VoxelType::FrontSight => get_font_offset(0x211, font),
            VoxelType::RearSight => get_font_offset(0x203, font),
            VoxelType::GasBlock => get_font_offset(0x7C, font),
            VoxelType::Barrel => get_font_offset(0x3A, font),
            VoxelType::BarrelEnd => get_font_offset(0x2E9, font),
            VoxelType::RecUpper => get_font_offset(0x2DD, font),
            VoxelType::RecLower => get_font_offset(0x319, font),
            VoxelType::RecLowerHalf => get_font_offset(0xDF, font),
            VoxelType::RecLowerBack => get_font_offset(0x2C5, font),
            VoxelType::Magazine => get_font_offset(0x1AB, font),
            VoxelType::Stock => get_font_offset(0x319, font),
            VoxelType::StockUpper => get_font_offset(0x2DD, font),
            VoxelType::Grip => get_font_offset(0x283, font),
        }
    }
    fn get_color(&self) -> Color {
        match self {
            VoxelType::Air => Color::new(0.0, 0.0, 0.0, 0.0),
            VoxelType::Rock => Color::new(0.5, 0.5, 0.5, 1.0),
            VoxelType::Mushroom => Color::new(0.75, 0.0, 0.75, 1.0),
            VoxelType::Candle => Color::new(0.9, 0.9, 0.0, 1.0),
            VoxelType::StockUpper => Color::new(0.75, 0.5, 0.25, 1.0),
            VoxelType::Stock => Color::new(0.75, 0.5, 0.25, 1.0),
            _ => Color::new(0.25, 0.25, 0.25, 1.0),
        }
    }
    fn is_transparent(&self) -> bool {
        match self {
            VoxelType::Air => true,
            VoxelType::Rock => false,
            VoxelType::Mushroom => true,
            VoxelType::Candle => true,
            VoxelType::FrontSight => true,
            VoxelType::RearSight => true,
            VoxelType::BarrelEnd => true,
            VoxelType::Barrel => true,
            VoxelType::GasBlock => true,
            VoxelType::RecUpper => true,
            VoxelType::RecLower => true,
            VoxelType::RecLowerHalf => true,
            VoxelType::RecLowerBack => true,
            VoxelType::Magazine => true,
            VoxelType::Stock => true,
            VoxelType::StockUpper => true,
            VoxelType::Grip => true,
        }
    }
    fn illuminates(&self) -> bool {
        match self {
            VoxelType::Mushroom => true,
            VoxelType::Candle => true,
            _ => false,
        }
    }
    fn rotation(&self) -> f32 {
        match self {
            VoxelType::RecLower => 3.14 / 2.0,
            VoxelType::Stock => 3.14 / 2.0,
            VoxelType::RearSight => 3.0 * (3.14 / 2.0),
            // VoxelType::Grip => 2.0 * (3.14 / 2.0),
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Voxel {
    pos: Point3<f32>,
    illumination: f32,
    voxel_type: VoxelType,
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
    light_noise: OpenSimplex,

    player_gun_model: Array2<VoxelType>,
    player_gun_timer: u8,
    // player_gun_sound: SoundData,
    player_ads: f32,
    player_gun_recoil: f32,
    player_gun_rotation: Point2<f32>,
    // sound_queue: Vec<(f64, Source)>,
}

fn gen_voxel(noise: OpenSimplex, meta_noise: OpenSimplex, x: usize, y: usize, z: usize) -> Voxel {
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

fn generate_map(noise: OpenSimplex, meta_noise: OpenSimplex) -> Array3<Voxel> {
    let mut map = Array3::from_shape_fn((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), |(x, y, z)| {
        gen_voxel(noise, meta_noise, x, y, z)
    });

    for x in 0..map.dim().0 {
        for y in 0..map.dim().1 {
            for z in 0..map.dim().2 {
                let pos = Point3::new(x, y, z);
                let pos_under = Point3::new(x, y - 1, z);
                if thread_rng().gen_range(0, 500) == 0
                    && is_in_array(map.view(), pos)
                    && is_in_array(map.view(), pos_under)
                    && map[[x, y, z]].voxel_type == VoxelType::Air
                    && map[[x, y - 1, z]].voxel_type == VoxelType::Rock
                {
                    map[[x, y, z]] = Voxel {
                        pos: Point3::new(x as f32, y as f32, z as f32),
                        illumination: 0.5,
                        voxel_type: VoxelType::Candle,
                    }
                }
            }
        }
    }

    map
}

impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        // Load/create resources such as images here.
        let noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());
        let meta_noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());

        set_default_filter(ctx, FilterMode::Nearest);

        use VoxelType::*;

        MyGame {
            blank_texture: Image::solid(ctx, 1, WHITE).unwrap(),
            lighting_sphere: calculate_sphere_surface(LIGHT_RANGE),
            font: load_font(ctx),
            voxel_array: generate_map(noise, meta_noise),
            draw_voxels: Vec::new(),
            camera_pos: Point3::new(
                (CHUNK_SIZE / 2) as f32,
                (CHUNK_SIZE / 2) as f32,
                (CHUNK_SIZE / 2) as f32,
            ),
            camera_rotation: Point2::origin(),
            nuke_lighting: false,
            current_tic: 0,
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

fn euclidean_distance_squared(a: Point3<f32>, b: Point3<f32>) -> f32 {
    let x_diff = a.x - b.x;
    let y_diff = a.y - b.y;
    let z_diff = a.z - b.z;

    (x_diff * x_diff + y_diff * y_diff + z_diff * z_diff)
}

pub struct KataFont {
    texture: Image,
    char_width: u8,
    char_height: u8,
}

fn get_font_offset(index: u16, font: &KataFont) -> Rect {
    let font_width = font.texture.width();
    let font_height = font.texture.height();
    let float_char_width = font.char_width as f32 / font_width as f32;
    let float_char_height = font.char_height as f32 / font_height as f32;

    let chars_width = 16;
    // let chars_height = 64;

    let x_index = index % chars_width;
    let y_index = index / chars_width;

    Rect::new(
        x_index as f32 * float_char_width,
        y_index as f32 * float_char_height,
        float_char_width,
        float_char_height,
    )
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
                let ray_voxel =
                    voxel_array[[ray_int_point.x, ray_int_point.y, ray_int_point.z]].clone();

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
                let max_echo_distance = echo_distances.last().unwrap();

                rodio::play_raw(
                    &device,
                    source
                        .buffered()
                        .reverb(
                            Duration::from_millis((min_echo_distance * 1000.0) as u64),
                            0.5 - min_echo_distance * 0.5,
                        )
                        .reverb(
                            Duration::from_millis((max_echo_distance * 1000.0) as u64),
                            0.5 - max_echo_distance * 0.5,
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

        // let sound_queue = &mut self.sound_queue;

        // let mut removal_indices = Vec::new();

        // //this order is very important for removing items properly
        // for i in (0..sound_queue.len()).rev()
        // {
        //     if (sound_queue[i].0 as f64) < update_time
        //     {
        //         sound_queue[i].1.play_detached().unwrap();
        //         removal_indices.push(i);
        //         assert_eq!(removal_indices.len() > 0, true);
        //     }
        // }

        // for index in removal_indices
        // {
        //     self.sound_queue.remove(index);
        // }

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

        self.draw_voxels.clear();
        self.draw_voxels.par_extend(
            zip_iter
                .into_par_iter()
                .filter(|((x, y, z), v)| {
                    (!v.voxel_type.is_transparent() || v.voxel_type.illuminates())
                        && (v.illumination > 0.01
                            || v.voxel_type.illuminates()
                            || euclidean_distance_squared(camera_pos, v.pos) < PLAYER_SIGHT_RANGE)
                        && {
                            let v_pos = Point3::new(*x as i32, *y as i32, *z as i32);
                            any_neighbour_empty(&voxel_array.view(), v_pos)
                        }
                        && (world_pos_to_index(try_ray_hitscan(
                            voxel_array.view(),
                            camera_pos,
                            v.pos,
                        )) == world_pos_to_index(v.pos)
                            || hitscan_tile(voxel_array.view(), camera_pos, v.pos).len() != 8
                            || nuke_lighting)
                })
                .map(|((_x, _y, _z), v)| {
                    let mut new_v = v.clone();
                    if nuke_lighting {
                        new_v.illumination = 1.0
                    }
                    new_v.illumination *= 1.0 - (0.5 * new_v.illumination.min(0.99)).max(0.01);
                    new_v.illumination = (new_v.illumination - 0.01).max(0.0);
                    new_v
                }),
        );

        self.nuke_lighting = false;

        self.draw_voxels.sort_unstable_by(|a, b| {
            euclidean_distance_squared(b.pos, camera_pos)
                .partial_cmp(&euclidean_distance_squared(a.pos, camera_pos))
                .unwrap_or(Ordering::Equal)
        });

        //Copy back to our world
        for new_vox in &mut self.draw_voxels {
            self.voxel_array[[
                new_vox.pos.x.floor() as usize,
                new_vox.pos.y.floor() as usize,
                new_vox.pos.z.floor() as usize,
            ]]
            .illumination = new_vox.illumination;
        }

        let mut lights: Vec<_> = self
            .draw_voxels
            .iter()
            .filter(|v| v.voxel_type.illuminates())
            .map(|v| Light {
                pos: Point3::new(v.pos.x + 0.5, v.pos.y + 0.5, v.pos.z + 0.5),
                facing: Point3::new(0.0, 1.0, 0.0),
                illumination: 0.25,
                range: 0.0,
            })
            .collect();

        if is_in_array(self.voxel_array.view(), world_pos_to_index(camera_pos)) {
            let player_light = Light {
                pos: camera_pos,
                facing: gun_facing,
                illumination: 0.5,
                range: 24.0,
            };

            lights.push(player_light);
            // dbg!(&lights);

            if muzzle_flash {
                let muzzle_light = Light {
                    pos: camera_pos,
                    facing: gun_facing,
                    illumination: 0.9,
                    range: 0.0,
                };

                lights.push(muzzle_light);
            }
        }

        for light in lights {
            self.voxel_array[[
                light.pos.x.floor() as usize,
                light.pos.y.floor() as usize,
                light.pos.z.floor() as usize,
            ]]
            .illumination = 0.9;

            let light_target: Point3<f32> = Point3::origin() + (light.facing * light.range).coords;

            let light_deviance = self.light_noise.get([
                light.pos.x as f64,
                light.pos.y as f64,
                light.pos.z as f64,
                self.current_tic as f64 * 0.5,
            ]);

            for target_point in &self.lighting_sphere {
                let target_point_offset = Point3::new(
                    target_point.x + light.pos.x + light_target.x,
                    target_point.y + light.pos.y + light_target.y,
                    target_point.z + light.pos.z + light_target.z,
                );

                let ray_hits =
                    hitscan_tile(self.voxel_array.view(), light.pos, target_point_offset);

                for hit in ray_hits {
                    if is_in_array(self.voxel_array.view(), world_pos_to_index(hit))
                        && world_pos_to_index(hit) != world_pos_to_index(target_point_offset)
                    {
                        let hit_index = world_pos_to_index(hit);

                        let ray_voxel =
                            &mut self.voxel_array[[hit_index.x, hit_index.y, hit_index.z]];

                        ray_voxel.illumination = (ray_voxel.illumination
                            + (light.illumination
                                / euclidean_distance_squared(
                                    ray_voxel.pos,
                                    Point3::new(
                                        light.pos.x as f32,
                                        light.pos.y as f32,
                                        light.pos.z as f32,
                                    ),
                                )
                                .max(1.0))
                            .powf(1.1)
                                * (light_deviance * 0.5 + 0.5) as f32)
                            .min(1.0);
                    }
                }
            }
        }

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

        let player_gun_scale = 0.75;

        for x in 0..self.player_gun_model.dim().1 {
            for y in 0..self.player_gun_model.dim().0 {
                let camera_pos = self.camera_pos;

                //|
                //V
                let gun_rotation = Rotation3::from_euler_angles(
                    self.player_gun_rotation.y,
                    self.player_gun_rotation.x,
                    0.0,
                );

                let mut voxel_offset =
                    rotation.transform_point(&gun_rotation.transform_point(&Point3::new(
                        -self.player_ads,
                        y as f32 * player_gun_scale,
                        (self.player_gun_model.dim().1 - x) as f32 * player_gun_scale * 0.75
                            + (0.5 - self.player_gun_recoil),
                    )));

                //No idea why this is necessary
                voxel_offset.x -= 1.0;

                //this may explode
                if let Some(screen_pos) = Point3::from_homogeneous(
                    model_view_projection * (camera_pos + voxel_offset.coords).to_homogeneous(),
                ) {
                    if screen_pos.z >= -1.0 && screen_pos.z <= 1.0 {
                        let voxel_type = &self.player_gun_model[[y, x]];
                        let color = voxel_type.get_color();
                        let color_darkness = (1.0 - screen_pos.z.min(1.0).max(0.0)).powf(1.1);

                        let screen_dest = [
                            screen_pos.x * WINDOW_WIDTH / 2.0 + WINDOW_WIDTH / 2.0,
                            screen_pos.y * WINDOW_HEIGHT / 2.0 + WINDOW_HEIGHT / 2.0, //We need to negate this, as 2d screen space is inverse of normalised device coords
                        ];

                        weapon_sprite_batch.add(DrawParam {
                            src: voxel_type.get_char_offset(&self.font),
                            dest: screen_dest.into(),
                            scale: [
                                (1.0 - screen_pos.z) * 31.4 * player_gun_scale,
                                (1.0 - screen_pos.z) * 31.4 * player_gun_scale,
                            ]
                            .into(),
                            color: Color {
                                r: color.r * color_darkness,
                                g: color.g * color_darkness,
                                b: color.b * color_darkness,
                                a: 1.0,
                            },
                            rotation: voxel_type.rotation(),
                            offset: [0.5, 0.5].into(),
                            ..DrawParam::default()
                        });
                    }
                }
            }
        }

        ggez::graphics::draw(ctx, &weapon_sprite_batch, DrawParam::default())?;

        graphics::present(ctx)
    }
}
