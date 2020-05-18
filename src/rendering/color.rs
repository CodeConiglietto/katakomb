use ggez::graphics::Color as GGColor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl From<Color> for GGColor {
    fn from(c: Color) -> Self {
        GGColor::new(
            c.r as f32 / u8::max_value() as f32,
            c.g as f32 / u8::max_value() as f32,
            c.b as f32 / u8::max_value() as f32,
            1.0,
        )
    }
}

pub const WHITE: Color = Color::new(255, 255, 255);
pub const LIGHT_GRAY: Color = Color::new(64, 64, 64);
pub const GRAY: Color = Color::new(127, 127, 127);
pub const DARK_GRAY: Color = Color::new(191, 191, 191);
pub const BLACK: Color = Color::new(0, 0, 0);

pub const RED: Color = Color::new(255, 0, 0);
pub const GREEN: Color = Color::new(0, 255, 0);
pub const BLUE: Color = Color::new(0, 0, 255);
