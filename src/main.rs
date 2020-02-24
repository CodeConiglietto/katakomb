use ggez::event::{self, EventHandler, KeyCode};
use ggez::{
    conf::WindowMode,
    graphics,
    graphics::{spritebatch::SpriteBatch, DrawParam, Image, *},
    input::keyboard,
    Context, ContextBuilder, GameResult,
};

use na::*;
use rand::prelude::*;
use noise::{OpenSimplex, NoiseFn};
use ndarray::prelude::*;
use rayon::prelude::*;

use std::cmp::Ordering;

const WINDOW_WIDTH: f32 = 1600.0;
const WINDOW_HEIGHT: f32 = 900.0;
const CHUNK_SIZE: usize = 64;

fn main() {
    let (mut ctx, mut event_loop) = ContextBuilder::new("Katakomb", "CodeBunny")
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

fn any_neighbour_empty(array: Array3<Voxel>, pos: Point3<i32>) -> bool {
    for x in -1..1
    {
        for y in -1..1
        {
            for z in -1..1
            {
                if array[[
                    (pos.x + x) as usize, 
                    (pos.y + y) as usize, 
                    (pos.z + z) as usize]].voxel_type.is_transparent()
                {
                    return true
                }
            }
        }
    }

    false
}

pub trait Drawable {
    fn get_color(&self) -> Color;
    fn is_transparent(&self) -> bool;
}

#[derive(Debug)]
enum VoxelType {
    Air,
    Rock,
}

impl Drawable for VoxelType {
    fn get_color(&self) -> Color{
        match self{
            VoxelType::Air => Color::new(0.0, 0.0, 0.0, 0.0),
            VoxelType::Rock => Color::new(0.5, 0.5, 0.5, 1.0),
        }
    }
    fn is_transparent(&self) -> bool{
        match self{
            VoxelType::Air => true,
            VoxelType::Rock => false,
        }
    }
}

#[derive(Debug)]
struct Voxel{
    voxel_type: VoxelType,
}

struct MyGame {
    // Your state here...
    image: Image,
    voxel_array: Array3<Voxel>,
    voxel_draw_points: Vec<Point3<f32>>,
    camera_pos: Point3<f32>,
}

fn gen_voxel(noise: OpenSimplex, x: usize, y: usize, z: usize) -> Voxel
{
    Voxel{
        voxel_type: 
            if noise.get([
                x as f64 * 0.1, 
                y as f64 * 0.1, 
                z as f64 * 0.1]) > 0.0
            {
                VoxelType::Air
            } else {
                VoxelType::Rock
            },
    }
}

impl MyGame {
    pub fn new(ctx: &mut Context) -> MyGame {
        // Load/create resources such as images here.
        let noise = OpenSimplex::new();

        MyGame {
            image: Image::solid(ctx, 1, WHITE).unwrap(),
            voxel_array: Array3::from_shape_fn(
                (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                |(x, y, z)| gen_voxel(noise, x, y, z),
            ),
            voxel_draw_points: (0..1000)
                .map(|_| {
                    Point3::new(
                        thread_rng().gen_range(-5, 5) as f32,
                        thread_rng().gen_range(-5, 5) as f32,
                        thread_rng().gen_range(-5, 5) as f32,
                    )
                })
                .collect(),
            camera_pos: Point3::new(10.0, 1.0, 0.0),
        }
    }
}

fn euclidean_distance(a: Point3<f32>, b: Point3<f32>) -> f32
{
    ((a.x - b.x).powf(2.0) + (a.y - b.y).powf(2.0) + (a.z - b.z).powf(2.0))
}

impl EventHandler for MyGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // Update code here...
        if keyboard::is_key_pressed(ctx, KeyCode::Left) {
            self.camera_pos.x += 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Right) {
            self.camera_pos.x -= 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::PageUp) {
            self.camera_pos.y += 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::PageDown) {
            self.camera_pos.y -= 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Up) {
            self.camera_pos.z += 0.1;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::Down) {
            self.camera_pos.z -= 0.1;
        }

        self.voxel_draw_points.clear();

        //let voxel_points = self.voxel_draw_points;

        let camera_pos = self.camera_pos;

        let zip_iter = ndarray::Zip::indexed(&self.voxel_array);

        let mut new_points: Vec<_> = zip_iter.into_par_iter().filter(|((x, y, z), v)| {
            !v
            .voxel_type.is_transparent()})
            .map(|((x, y, z), v)| {
                Point3::new(x as f32, y as f32, z as f32)}).collect();

        new_points.sort_unstable_by(
            |a, b| 
            euclidean_distance(*b, camera_pos)
            .partial_cmp(&euclidean_distance(*a, camera_pos)).unwrap_or(Ordering::Equal));

        std::mem::swap(&mut self.voxel_draw_points, &mut new_points);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

        // Our object is translated along the x axis.
        let model = Isometry3::new(Vector3::x(), na::zero());

        // Our camera looks toward the point (1.0, 0.0, 0.0).
        // It is located at (0.0, 0.0, 1.0).
        let eye = self.camera_pos; //Point3::new(0.0, 0.0, 1.0);
        let target = Point3::new(self.camera_pos.x + 1.0, self.camera_pos.y + 0.0, self.camera_pos.z + 0.0);
        // let target = Point3::new(0.0, 0.0, 0.0);
        let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());

        // A perspective projection.
        let projection = Perspective3::new(16.0 / 9.0, 3.14 / 4.0, 1.0, 1000.0);

        // The combination of the model with the view is still an isometry.
        let model_view = view * model;

        // Convert everything to a `Matrix4` so that they can be combined.
        let mat_model_view = model_view.to_homogeneous();

        // Combine everything.
        let model_view_projection = projection.as_matrix() * mat_model_view;

        let mut sprite_batch = SpriteBatch::new(self.image.clone());

        for point in self.voxel_draw_points.iter() {
            let screen_pos =
                Point3::from_homogeneous(model_view_projection * point.to_homogeneous()).unwrap_or(Point3::origin()); //TODO this is broken af

            let color_value = (1.0 - screen_pos.z.min(1.0).max(0.0)) * 0.5;

            sprite_batch.add(DrawParam {
                dest: [
                    screen_pos.x * WINDOW_WIDTH / 2.0 + WINDOW_WIDTH / 2.0,
                    screen_pos.y * WINDOW_HEIGHT / 2.0 + WINDOW_HEIGHT / 2.0,
                ]
                .into(),
                scale: [(1.0 - screen_pos.z) * WINDOW_HEIGHT * 0.75, (1.0 - screen_pos.z) * WINDOW_HEIGHT * 0.75].into(),
                color: Color::new(color_value, color_value, color_value, 1.0),
                offset: [0.5, 0.5].into(),
                ..DrawParam::default()
            });
        }
        ggez::graphics::draw(ctx, &sprite_batch, DrawParam::default())?;

        graphics::present(ctx)
    }
}
