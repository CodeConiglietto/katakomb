use ggez::graphics::*;

use crate::rendering::font::*;

pub trait Drawable {
    fn get_char_offset(&self, font: &KataFont) -> Rect;
    fn get_color(&self) -> Color;
    fn is_transparent(&self) -> bool;
    fn illuminates(&self) -> bool;
    fn rotation(&self) -> f32;
}
