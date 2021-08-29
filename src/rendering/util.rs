use ggez::nalgebra as na;
use na::*;
use ndarray::prelude::*;

use ggez::graphics::{spritebatch::SpriteBatch, Color, DrawParam};

use crate::{
    constants::*,
    rendering::{drawable::*, font::*, tile::*},
};

pub fn draw_player_weapon(
    weapon_sprite_batch: &mut SpriteBatch,
    font: &KataFont,
    model_view_projection: Matrix4<f32>,
    camera_pos: Point3<f32>,
    rotation: Rotation3<f32>,
    player_gun_model: &Array2<TileType>,
    player_ads: f32,
    player_gun_recoil: f32,
    player_gun_rotation: Point2<f32>,
) {
    let player_gun_scale = 0.75;

    for x in 0..player_gun_model.dim().1 {
        for y in 0..player_gun_model.dim().0 {
            let gun_rotation =
                Rotation3::from_euler_angles(player_gun_rotation.y, player_gun_rotation.x, 0.0);

            let mut tile_offset =
                rotation.transform_point(&gun_rotation.transform_point(&Point3::new(
                    -player_ads,
                    y as f32 * player_gun_scale,
                    (player_gun_model.dim().1 - x) as f32 * player_gun_scale * 0.75
                        + (0.5 - player_gun_recoil),
                )));

            //No idea why this is necessary
            tile_offset.x -= 1.0;

            //this may explode
            if let Some(screen_pos) = Point3::from_homogeneous(
                model_view_projection * (camera_pos + tile_offset.coords).to_homogeneous(),
            ) {
                if screen_pos.z >= -1.0 && screen_pos.z <= 1.0 {
                    let tile_type = &player_gun_model[[y, x]];
                    let color = tile_type.get_color();
                    let color_darkness = (1.0 - screen_pos.z.min(1.0).max(0.0)).powf(1.1);

                    let screen_dest = [
                        screen_pos.x * WINDOW_WIDTH / 2.0 + WINDOW_WIDTH / 2.0,
                        screen_pos.y * WINDOW_HEIGHT / 2.0 + WINDOW_HEIGHT / 2.0, //We need to negate this, as 2d screen space is inverse of normalised device coords
                    ];

                    weapon_sprite_batch.add(DrawParam {
                        src: tile_type.get_char_offset(&font),
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
                        rotation: tile_type.rotation(),
                        offset: [0.5, 0.5].into(),
                        ..DrawParam::default()
                    });
                }
            }
        }
    }
}
