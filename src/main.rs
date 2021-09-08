use std::{
    cmp::Ordering,
    collections::BTreeSet,
    env,
    f32::consts::{FRAC_PI_4, PI},
    fs::File,
    io::BufReader,
    path::PathBuf,
    time::Duration,
    time::Instant,
};

use failure::Fallible;
use float_ord::FloatOrd;
use ggez::{
    // audio::{SoundData, Source, SoundSource},
    conf::WindowMode,
    event::{self, EventHandler, KeyCode},

    graphics::{self, spritebatch::SpriteBatch, Color, DrawParam, FilterMode, Image},
    input::{keyboard, mouse},
    timer,
    Context,
    ContextBuilder,
    GameResult,
};
use log::info;
use na::{
    Isometry3, Matrix4, Perspective3, Point2, Point3, Rotation3, Unit, UnitVector3, Vector2,
    Vector3,
};
use ndarray::arr2;
use ndarray::prelude::*;
use noise::{OpenSimplex, Perlin, Seedable, Value, Worley};
use rand::prelude::*;
use rayon::prelude::*;
use rodio::{OutputStream, Source};
use specs::prelude::*;
use structopt::StructOpt;

use crate::{
    components::{position::*, velocity::*},
    constants::*,
    generation::world::*,
    geometry::util::*,
    rendering::{drawable::Drawable, font::*, light::*, tile::*},
    systems::physics_system::*,
    util::*,
    world::util::*,
};

mod audio;
mod components;
mod constants;
mod editor;
mod generation;
mod geometry;
mod rendering;
mod systems;
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
}

struct Player {
    pos: Point3<f32>,
    vel: Vector3<f32>,
    facing: Point2<f32>,

    crouching: bool,

    equipped_item: Item,
}

impl Player {
    pub fn draw_equipped(
        &self,
        font: &KataFont,
        mvp: Matrix4<f32>,
        rotation: Rotation3<f32>,
        mut item_sprite_batch: &mut SpriteBatch,
    ) {
        match &self.equipped_item {
            Item::Weapon {
                gun_model,
                gun_timer,
                ads,
                gun_recoil,
                gun_rotation,
            } => {
                rendering::util::draw_player_weapon(
                    &mut item_sprite_batch,
                    &font,
                    mvp,
                    self.pos,
                    rotation,
                    &gun_model,
                    *ads,
                    *gun_recoil,
                    *gun_rotation,
                );
            }
        }
    }

    pub fn update_equipped(&mut self) {
        match &self.equipped_item {
            Item::Weapon {
                mut gun_rotation, ..
            } => {
                let gun_rotation =
                    Rotation3::from_euler_angles(-gun_rotation.y, gun_rotation.x, 0.0);

                let view_rotation = Rotation3::from_euler_angles(self.facing.y, self.facing.x, 0.0);

                let gun_facing = view_rotation
                    .transform_point(&gun_rotation.transform_point(&Point3::new(0.0, 0.0, 1.0)));
            }
        }
    }
}

enum Item {
    Weapon {
        gun_model: Array2<TileType>,
        gun_timer: u8,

        ads: f32,
        gun_recoil: f32,
        gun_rotation: Point2<f32>,
    },
}

impl Item {
    pub fn update(&mut self) {
        match self {
            Self::Weapon {
                ref mut gun_timer,
                ref mut ads,
                ref mut gun_recoil,
                ref mut gun_rotation,
                ..
            } => {
                *gun_recoil *= 0.95;
                gun_rotation.x *= 0.95;
                gun_rotation.y *= 0.95;
                *gun_timer -= 1;
                *ads *= 0.9; //(self.player.ads - 0.1).max(0.0);
            }
        }
    }

    pub fn primary_use(&mut self) {
        println!("primary item use");
        match self {
            Self::Weapon {
                mut gun_timer,
                mut gun_recoil,
                mut gun_rotation,
                ..
            } => {
                if gun_timer == 0 {
                    gun_recoil = (gun_recoil + 0.2).min(1.0);
                    gun_rotation.x = (gun_rotation.x + (thread_rng().gen::<f32>() - 0.5) * 0.05)
                        .min(1.0)
                        .max(-1.0);
                    gun_rotation.y = (gun_rotation.y + 0.05).min(1.0);

                    // // dbg!(std::env::current_dir().unwrap().to_str().unwrap());
                    // let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                    // // let device = rodio::default_output_device().unwrap();
                    // let file = File::open(r"resources/gunshot.wav").unwrap();
                    // let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

                    // let mut echo_distances = Vec::new();

                    // for cube_point in get_cube_points(Point3::new(-0.5, -0.5, -0.5)) {
                    //     let ray_target = self.player.pos + (cube_point.coords * MAX_SOUND_RANGE * 2.0);

                    //     if is_in_array(self.tile_array.view(), world_pos_to_index(ray_target)) {
                    //         let ray_hit = try_bresenham_hitscan(
                    //             self.tile_array.view(),
                    //             world_pos_to_int(self.player.pos),
                    //             world_pos_to_int(ray_target),
                    //         );

                    //         if ray_hit != world_pos_to_int(ray_target) {
                    //             // //TODO mess with this
                    //             let hit_distance = euclidean_distance_squared(
                    //                 self.player.pos,
                    //                 Point3::new(ray_hit.x as f32, ray_hit.y as f32, ray_hit.z as f32),
                    //             )
                    //             .sqrt();
                    //             let hit_distance_ratio = hit_distance / (MAX_SOUND_RANGE * 2.0);
                    //             let hit_distance_ratio_squared = hit_distance * hit_distance;
                    //             echo_distances.push(hit_distance_ratio);
                    //             // let mut source = Source::from_data(ctx, self.player_gun_sound.clone()).unwrap();
                    //             // source.set_pitch(0.5 + 0.5 * (1.0 - hit_distance_ratio));
                    //             // source.set_fade_in(Duration::from_millis((hit_distance_ratio_squared) as u64));
                    //             // //source.set_volume(1.0 - (hit_distance_ratio * 0.5));
                    //             // self.sound_queue.push((update_time + (hit_distance_ratio * 0.5) as f64, source));
                    //             // //TODO take average of hit distances and use that to change the non-ray sound's pitch
                    //         }
                    //     }
                    // }

                    // echo_distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    // let min_echo_distance = echo_distances.first().unwrap();
                    // // let med_echo_distance = echo_distances[echo_distances.len() /2 ];
                    // let max_echo_distance = echo_distances.last().unwrap();

                    // //warning: using more than 2 reverbs leads to very unpleasant results :<
                    // stream_handle
                    //     .play_raw(
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
                    //     )
                    //     .unwrap();

                    gun_timer = 12;
                }
            }
        }
    }

    pub fn secondary_use(&mut self) {
        println!("secondary item use");
        match self {
            Self::Weapon { ref mut ads, .. } => {
                *ads = (*ads + 0.1).min(1.0);
            }
        }
    }
}

struct Katakomb {
    // blank_texture: Image,
    // lighting_sphere: Vec<Point3<f32>>,
    font: KataFont,
    tile_array: Array3<Tile>,
    draw_tiles: BTreeSet<DrawTile>,

    player: Player,

    nuke_lighting: bool,

    lights: Vec<(Point3<usize>, Color)>,

    current_tic: u64,

    mouse_pos: ggez::mint::Point2<f32>,
    // lights: Vec<Light>,
    // light_noise: OpenSimplex,
    // player_gun_sound: SoundData,
    // sound_queue: Vec<(f64, Source)>,
}

impl Katakomb {
    pub fn new(ctx: &mut Context) -> Fallible<Self> {
        // Load/create resources such as images here.
        // let noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());
        // let meta_noise = OpenSimplex::new().set_seed(thread_rng().gen::<u32>());

        ggez::input::mouse::set_cursor_grabbed(ctx, true);

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

        let tile_array = generate_chunk(Point3::new(0, 0, 0), &chunk_gen_package);

        let lights: Vec<_> = tile_array
            .iter()
            .filter(|tile| {
                thread_rng().gen_range(0, 5000) == 0
                    && world::util::any_neighbour_is(
                        tile_array.view(),
                        Point3::new(
                            tile.pos.x.floor() as i32,
                            tile.pos.y.floor() as i32,
                            tile.pos.z.floor() as i32,
                        ),
                        |t| t.tile_type.is_transparent(),
                    )
                    && world::util::any_neighbour_is(
                        tile_array.view(),
                        Point3::new(
                            tile.pos.x.floor() as i32,
                            tile.pos.y.floor() as i32,
                            tile.pos.z.floor() as i32,
                        ),
                        |t| t.tile_type.collides(),
                    )
            })
            .map(|tile| {
                (
                    Point3::new(
                        tile.pos.x.floor() as usize,
                        tile.pos.y.floor() as usize,
                        tile.pos.z.floor() as usize,
                    ),
                    Color {
                        r: thread_rng().gen_range(0.0, 1.0),
                        g: thread_rng().gen_range(0.0, 1.0),
                        b: thread_rng().gen_range(0.0, 1.0),
                        a: 1.0,
                    },
                )
            })
            .collect();

        Ok(Self {
            // blank_texture: Image::solid(ctx, 1, graphics::Color::WHITE).unwrap(),
            // lighting_sphere: calculate_sphere_surface(LIGHT_RANGE),
            font: KataFont::load(ctx)?,
            tile_array,
            draw_tiles: BTreeSet::new(),
            player: Player {
                pos: Point3::new(
                    (CHUNK_SIZE / 2) as f32,
                    (CHUNK_SIZE / 2) as f32,
                    (CHUNK_SIZE / 2) as f32,
                ),
                vel: Vector3::new(0.0, 0.0, 0.0),
                facing: Point2::origin(),
                equipped_item: Item::Weapon {
                    gun_recoil: 0.0,
                    gun_rotation: Point2::origin(),
                    gun_model: arr2(&[
                        [
                            Air, Air, FrontSight, Air, Air, Air, Air, RearSight, Air, Air, Air,
                        ],
                        [
                            BarrelEnd, BarrelEnd, GasBlock, Barrel, Barrel, RecLower, RecLower,
                            RecLower, Air, StockUpper, StockUpper,
                        ],
                        [
                            Air, Air, Air, Air, Air, Air, Magazine, Grip, Stock, Stock, Stock,
                        ],
                    ]),
                    gun_timer: 0,
                    ads: 0.0,
                },
                crouching: false,
            },
            nuke_lighting: false,
            lights,
            current_tic: 0,
            mouse_pos: [
                WINDOW_WIDTH / 2.0,
                WINDOW_HEIGHT / 2.0,
            ]
            .into()
            // lights: Vec::new(),
            // light_noise: OpenSimplex::new(),
            // player_gun_sound: SoundData::new(ctx, r"/gunshot.wav").unwrap(),
            // sound_queue: Vec::new(),
        })
    }
}

impl EventHandler<ggez::GameError> for Katakomb {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let start_t = Instant::now();

        // Update code here...
        // self.physics_system.run_now(&self.ecs_world);
        // self.ecs_world.maintain();

        let (screen_width, screen_height) = graphics::drawable_size(ctx);

        let screen_center: Point2<f32> = [
            screen_width / 2.0,
            screen_height / 2.0,
        ]
        .into();

        // let old_mouse_pos = self.mouse_pos;
        // self.mouse_pos = mouse::position(ctx);
        mouse::set_position(ctx, screen_center).unwrap();

        // let mouse_delta: Point2<f32> =
        //     Point2::new(old_mouse_pos.x - self.mouse_pos.x, old_mouse_pos.y - self.mouse_pos.y).into();

        // self.mouse_pos = mouse::position(ctx);

        let mouse_delta = mouse::delta(&ctx);

        self.player.facing =
            self.player.facing + Vector2::new(mouse_delta.x * -0.0025, mouse_delta.y * 0.0025);

        let mut muzzle_flash = false;

        let update_time = timer::duration_to_f64(timer::time_since_start(ctx));

        self.player.equipped_item.update();
        self.player.update_equipped();

        let movement_rotation =
            Rotation3::from_axis_angle(&Vector3::y_axis(), self.player.facing.x);

        if mouse::button_pressed(ctx, mouse::MouseButton::Left) {
            self.player.equipped_item.primary_use();
        }

        // if keyboard::is_key_pressed(ctx, KeyCode::Left) {
        //     self.player.facing.x += 0.025;
        // }
        // if keyboard::is_key_pressed(ctx, KeyCode::Right) {
        //     self.player.facing.x -= 0.025;
        // }
        // if keyboard::is_key_pressed(ctx, KeyCode::Up) {
        //     self.player.facing.y -= 0.025;
        // }
        // if keyboard::is_key_pressed(ctx, KeyCode::Down) {
        //     self.player.facing.y += 0.025;
        // }

        if mouse::button_pressed(ctx, mouse::MouseButton::Right) {
            self.player.equipped_item.secondary_use();
        }

        if keyboard::is_key_pressed(ctx, KeyCode::A) {
            self.player.vel.x += 0.01;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::D) {
            self.player.vel.x -= 0.01;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::W) {
            self.player.vel.z += 0.01;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::S) {
            self.player.vel.z -= 0.01;
        }

        if get_tile_at(
            self.player.pos + Point3::new(0.0f32, -0.1f32, 0.0f32).coords,
            &self.tile_array,
        )
        .tile_type
        .collides()
        {
            if keyboard::is_key_pressed(ctx, KeyCode::Space) {
                self.player.vel.y += 0.3;
            }
        } else {
            self.player.vel.y -= 0.01;
        }
        if keyboard::is_key_pressed(ctx, KeyCode::LControl) {
            self.player.crouching = true;
        } else {
            self.player.crouching = false;
        }

        if get_tile_at(
            self.player.pos + Point3::new(self.player.vel.x, 0.0f32, 0.0f32).coords,
            &self.tile_array,
        )
        .tile_type
        .collides()
        {
            self.player.vel.x = 0.0;
        }
        if get_tile_at(
            self.player.pos + Point3::new(0.0f32, self.player.vel.y, 0.0f32).coords,
            &self.tile_array,
        )
        .tile_type
        .collides()
        {
            self.player.vel.y = 0.0;
        }
        if get_tile_at(
            self.player.pos + Point3::new(0.0f32, 0.0f32, self.player.vel.z).coords,
            &self.tile_array,
        )
        .tile_type
        .collides()
        {
            self.player.vel.z = 0.0;
        }

        let vel_normalised = Unit::new_and_get(self.player.vel);
        if vel_normalised.1 > 1.0 {
            self.player.vel = vel_normalised.0.into_inner();
        }

        let movement_offset = movement_rotation.transform_vector(&self.player.vel);

        let new_pos = self.player.pos + movement_offset;

        if !get_tile_at(new_pos, &self.tile_array).tile_type.collides() {
            self.player.pos = new_pos;
        }

        self.player.vel *= 0.9;

        if keyboard::is_key_pressed(ctx, KeyCode::N) {
            self.nuke_lighting = true;
        }

        self.draw_tiles.clear();

        //let tile_points = self.tile_draw_points;
        let camera_pos = self.player.pos;

        // let tile_array = &self.tile_array;
        // let zip_iter = ndarray::Zip::indexed(tile_array);

        // let int_camera_pos = Point3::new(
        //     camera_pos.x.floor() as i32,
        //     camera_pos.y.floor() as i32,
        //     camera_pos.z.floor() as i32,
        // );

        let usize_camera_pos = Point3::new(
            camera_pos.x.floor() as usize,
            camera_pos.y.floor() as usize + 1,
            camera_pos.z.floor() as usize,
        );

        let mut light_sources = Vec::new();

        light_sources.push((usize_camera_pos, Color::GREEN));

        if muzzle_flash {
            light_sources.push((
                Point3::new(
                    camera_pos.x.floor() as usize,
                    camera_pos.y.floor() as usize,
                    camera_pos.z.floor() as usize,
                ),
                Color::YELLOW,
            ));
        }

        // light_sources.extend(
        //     [
        //         (Point3::new(
        //             CHUNK_SIZE / 2 - 2,
        //             CHUNK_SIZE / 2 - 2,
        //             CHUNK_SIZE / 2
        //         ), Color::BLUE),
        //         (Point3::new(
        //             CHUNK_SIZE / 2 + 2,
        //             CHUNK_SIZE / 2 + 2,
        //             CHUNK_SIZE / 2
        //         ), Color::RED),
        //     ].iter()
        // );

        light_sources.extend(self.lights.iter().cloned());

        //NEW SHITTY IMPLEMENTATION
        self.tile_array
            .par_iter_mut()
            .for_each(|tile| tile.illumination_color = Color::BLACK);

        for light in light_sources.iter() {
            let light_pos: &Point3<usize> = &light.0.into();
            let light_color = light.1;

            if is_in_array(self.tile_array.view(), world_pos_to_index(camera_pos)) {
                let mut octs =
                    split_shadowcast_octants(self.tile_array.view_mut(), *light_pos, LIGHT_RANGE);

                //TODO: clean up euclidean distance cleanup by storing a usize position in a tile instead of a f32 one
                octs.iter_mut().for_each(|o| {
                    shadowcast_octant(
                        o.0.view_mut(),
                        o.1,
                        LIGHT_RANGE,
                        LightShape::Sphere,
                        Point3::new(light.0.x as f32, light.0.y as f32, light.0.z as f32),
                        |t, (x, y, z)| {
                            t.illumination_color = combine_light_colors(
                                scale_color(
                                    light_color,
                                    1.0 - (EUCLIDEAN_DISTANCE_LOOKUP[[x, y, z]]
                                        / LIGHT_RANGE as f32)
                                        .min(1.0),
                                ),
                                t.illumination_color,
                            );
                        },
                    )
                });
                // octs.iter_mut().for_each(|o| shadowcast_octant(o.0.view_mut(), o.1));
            }
        }
        self.draw_tiles.clear();

        let dt = &mut self.draw_tiles;

        let mut fov_octs = split_shadowcast_octants(
            self.tile_array.view_mut(),
            usize_camera_pos,
            PLAYER_SIGHT_RANGE,
        );

        fov_octs.iter_mut().for_each(|o| {
            shadowcast_octant(
                o.0.view_mut(),
                o.1,
                PLAYER_SIGHT_RANGE,
                LightShape::Sphere,
                camera_pos,
                |t, (x, y, z)| {
                    if !t.tile_type.is_transparent() && t.illuminated() {
                        dt.insert(DrawTile {
                            tile: t.clone(),
                            dist_from_eye: EUCLIDEAN_DISTANCE_LOOKUP[[x, y, z]],
                        });
                    }
                },
            )
        });

        println!("Draw tiles len: {}", self.draw_tiles.len());
        println!("Light sources len: {}", light_sources.len());
        println!(
            "Frame time: {} ms",
            Instant::now().duration_since(start_t).as_micros() as f64 / 1000.0
        );

        // self.draw_tiles.sort_unstable_by(|a, b| {
        //     euclidean_distance_squared(b.pos, camera_pos)
        //         .partial_cmp(&euclidean_distance_squared(a.pos, camera_pos))
        //         .unwrap_or(Ordering::Equal)
        // });

        // self.draw_tiles.par_extend(
        //     self.tile_array
        //         .par_iter()
        //         .filter(|tile| tile.illumination > 0.0)
        //         .cloned(),
        // );

        self.current_tic += 1;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::Color::BLACK);

        // Our object is translated along the x axis.
        let model = Isometry3::new(Vector3::x(), na::zero());

        // Our camera looks toward the point (1.0, 0.0, 0.0).
        // It is located at (0.0, 0.0, 1.0).
        let eye = self.player.pos + Point3::new(0.0, 1.0, 0.0).coords; //Point3::new(0.0, 0.0, 1.0);

        let rotation =
            Rotation3::from_euler_angles(self.player.facing.y, self.player.facing.x, 0.0);

        let rotation_offset = rotation.transform_point(&Point3::new(0.0, 0.0, 1.0));

        let target = Point3::new(
            self.player.pos.x + rotation_offset.x,
            self.player.pos.y + rotation_offset.y + 1.0,
            self.player.pos.z + rotation_offset.z,
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
            let tile = &tile.tile;
            if let Some(screen_pos) =
                Point3::from_homogeneous(model_view_projection * tile.pos.to_homogeneous())
            {
                if screen_pos.z >= -1.0 && screen_pos.z <= 1.0 {
                    let tile_color = tile.tile_type.get_color();
                    let illumination_color = tile.illumination_color;
                    // let color = tile.illumination_color;
                    let color = average_colors(tile_color, illumination_color);
                    let color_darkness = color_max(&color);
                    // tile.illumination;
                    // let color_darkness =
                    //     (1.0 - screen_pos.z.min(1.0).max(0.0)) * 0.25 + tile.illumination * 0.75;
                    let color_back_darkness = color_darkness * 0.75;

                    let screen_dest = [
                        screen_pos.x * WINDOW_WIDTH / 2.0 + WINDOW_WIDTH / 2.0,
                        -screen_pos.y * WINDOW_HEIGHT / 2.0 + WINDOW_HEIGHT / 2.0, //We need to negate this, as 2d screen space is inverse of normalised device coords
                    ];

                    let color_value = 1.0; //color_value(&color).sqrt();

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
                                    a: color_value,
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
                                a: color_value,
                            })
                            .offset([0.5, 0.5]), // ..DrawParam::default()
                    );
                }
            }
        }
        ggez::graphics::draw(ctx, &sprite_batch, DrawParam::default())?;

        let mut item_sprite_batch = SpriteBatch::new(self.font.texture().clone());

        self.player.draw_equipped(
            &self.font,
            model_view_projection,
            rotation,
            &mut item_sprite_batch,
        );

        ggez::graphics::draw(ctx, &item_sprite_batch, DrawParam::default())?;

        graphics::present(ctx)
    }
}

struct DrawTile {
    tile: Tile,
    dist_from_eye: f32,
}

impl Eq for DrawTile {}

impl PartialEq for DrawTile {
    fn eq(&self, other: &Self) -> bool {
        self.dist_from_eye == other.dist_from_eye && self.tile.pos == other.tile.pos
    }
}

impl PartialOrd for DrawTile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DrawTile {
    fn cmp(&self, other: &Self) -> Ordering {
        FloatOrd(self.dist_from_eye)
            .cmp(&FloatOrd(other.dist_from_eye))
            .then_with(|| FloatOrd(self.tile.pos.x).cmp(&FloatOrd(other.tile.pos.x)))
            .then_with(|| FloatOrd(self.tile.pos.y).cmp(&FloatOrd(other.tile.pos.y)))
            .then_with(|| FloatOrd(self.tile.pos.z).cmp(&FloatOrd(other.tile.pos.z)))
            .reverse()
    }
}

fn shadowcast_octant<F>(
    mut slice: ArrayViewMut3<Tile>,
    (x_sign, y_sign, z_sign): (bool, bool, bool),
    cast_range: usize,
    shape: LightShape,
    source_pos: Point3<f32>,
    mut f: F,
) where
    F: FnMut(&mut Tile, (usize, usize, usize)),
{
    if !slice.is_empty() {
        if !x_sign {
            slice.invert_axis(Axis(0));
        }
        if !y_sign {
            slice.invert_axis(Axis(1));
        }
        if !z_sign {
            slice.invert_axis(Axis(2));
        }

        for i in 0..3 {
            let permuted_slice = slice
                .view_mut()
                .permuted_axes((i, (i + 1) % 3, (i + 2) % 3));

            scan_recursive_shadowcast(permuted_slice, cast_range, shape, source_pos, &mut f);
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
}

#[derive(Clone, Copy, Debug)]
pub enum LightShape {
    Sphere,
    Cone {
        facing: UnitVector3<f32>,
        width_angle: f32,
    },
}

impl LightShape {
    fn contains(&self, pos: Point3<f32>) -> bool {
        match self {
            Self::Sphere => true,
            Self::Cone {
                facing,
                width_angle,
            } => facing.into_inner().angle(&pos.coords) < *width_angle,
        }
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

fn scan_recursive_shadowcast<F>(
    mut slice: ArrayViewMut3<Tile>,
    cast_range: usize,
    shape: LightShape,
    source_pos: Point3<f32>,
    mut f: F,
) where
    F: FnMut(&mut Tile, (usize, usize, usize)),
{
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
                let outside_range = dist_from_center >= cast_range as f32;

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

                let in_shape = shape.contains(Point3::from(tile.pos - source_pos));

                if in_shape {
                    f(tile, (x, y, current.z));
                }

                // If we're on the last layer, we don't worry about bookkeeping for recursion
                if current.z < slice_depth - 1 {
                    if tile.tile_type.is_transparent() && in_shape {
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

fn split_shadowcast_octants<'a>(
    mut tile_array: ArrayViewMut3<'a, Tile>,
    origin: Point3<usize>,
    cast_range: usize,
) -> [(ArrayViewMut3<'a, Tile>, (bool, bool, bool)); 8] {
    let (tiles_width, tiles_height, tiles_depth) = tile_array.dim();

    let light_left = origin.x.saturating_sub(cast_range);
    let light_right = (origin.x + cast_range).min(tiles_width);
    let light_bottom = origin.y.saturating_sub(cast_range);
    let light_top = (origin.y + cast_range).min(tiles_height);
    let light_back = origin.z.saturating_sub(cast_range);
    let light_front = (origin.z + cast_range).min(tiles_depth);

    let light_cube = tile_array.slice_move(s![
        light_left..light_right,
        light_bottom..light_top,
        light_back..light_front,
    ]);

    let mid_x = origin.x - light_left;
    let mid_y = origin.y - light_bottom;
    let mid_z = origin.z - light_back;

    let (bx, tx) = light_cube.split_at(Axis(0), mid_x);

    let (bxby, bxty) = bx.split_at(Axis(1), mid_y);
    let (txby, txty) = tx.split_at(Axis(1), mid_y);

    let (bxbybz, bxbytz) = bxby.split_at(Axis(2), mid_z);
    let (bxtybz, bxtytz) = bxty.split_at(Axis(2), mid_z);
    let (txbybz, txbytz) = txby.split_at(Axis(2), mid_z);
    let (txtybz, txtytz) = txty.split_at(Axis(2), mid_z);

    let octs = [
        (bxbybz, (false, false, false)),
        (bxbytz, (false, false, true)),
        (bxtybz, (false, true, false)),
        (bxtytz, (false, true, true)),
        (txbybz, (true, false, false)),
        (txbytz, (true, false, true)),
        (txtybz, (true, true, false)),
        (txtytz, (true, true, true)),
    ];

    octs
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

fn combine_light_colors(a: Color, b: Color) -> Color {
    Color {
        r: a.r.max(b.r).min(1.0),
        g: a.g.max(b.g).min(1.0),
        b: a.b.max(b.b).min(1.0),
        a: 1.0,
    }
}

fn average_colors(a: Color, b: Color) -> Color {
    Color {
        r: (a.r + b.r) / 2.0,
        g: (a.g + b.g) / 2.0,
        b: (a.b + b.b) / 2.0,
        a: 1.0,
    }
}

fn scale_color(color: Color, alpha: f32) -> Color {
    Color {
        r: color.r * alpha,
        g: color.g * alpha,
        b: color.b * alpha,
        a: 1.0,
    }
}

fn color_value(color: &Color) -> f32 {
    (color.r + color.g + color.b) / 3.0
}

fn color_max(color: &Color) -> f32 {
    color.r.max(color.g).max(color.b)
}
