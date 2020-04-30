use na::*;

use crate::rendering::{drawable::*, font::*};

use ggez::graphics::{Color, Rect};

#[derive(Debug, PartialEq, Clone)]
pub enum TileType {
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

impl Drawable for TileType {
    fn get_char_offset(&self, font: &KataFont) -> Rect {
        match self {
            TileType::Air => get_font_offset(0, font),
            TileType::Rock => get_font_offset(0xB1, font),
            TileType::Mushroom => get_font_offset(0x2E1, font),
            TileType::Candle => get_font_offset(0x21A, font),
            TileType::FrontSight => get_font_offset(0x211, font),
            TileType::RearSight => get_font_offset(0x203, font),
            TileType::GasBlock => get_font_offset(0x7C, font),
            TileType::Barrel => get_font_offset(0x3A, font),
            TileType::BarrelEnd => get_font_offset(0x2E9, font),
            TileType::RecUpper => get_font_offset(0x2DD, font),
            TileType::RecLower => get_font_offset(0x319, font),
            TileType::RecLowerHalf => get_font_offset(0xDF, font),
            TileType::RecLowerBack => get_font_offset(0x2C5, font),
            TileType::Magazine => get_font_offset(0x1AB, font),
            TileType::Stock => get_font_offset(0x319, font),
            TileType::StockUpper => get_font_offset(0x2DD, font),
            TileType::Grip => get_font_offset(0x283, font),
        }
    }
    fn get_color(&self) -> Color {
        match self {
            TileType::Air => Color::new(0.0, 0.0, 0.0, 0.0),
            TileType::Rock => Color::new(0.5, 0.5, 0.5, 1.0),
            TileType::Mushroom => Color::new(0.75, 0.0, 0.75, 1.0),
            TileType::Candle => Color::new(0.9, 0.9, 0.0, 1.0),
            TileType::StockUpper => Color::new(0.75, 0.5, 0.25, 1.0),
            TileType::Stock => Color::new(0.75, 0.5, 0.25, 1.0),
            _ => Color::new(0.25, 0.25, 0.25, 1.0),
        }
    }
    fn is_transparent(&self) -> bool {
        match self {
            TileType::Air => true,
            TileType::Rock => false,
            TileType::Mushroom => true,
            TileType::Candle => true,
            TileType::FrontSight => true,
            TileType::RearSight => true,
            TileType::BarrelEnd => true,
            TileType::Barrel => true,
            TileType::GasBlock => true,
            TileType::RecUpper => true,
            TileType::RecLower => true,
            TileType::RecLowerHalf => true,
            TileType::RecLowerBack => true,
            TileType::Magazine => true,
            TileType::Stock => true,
            TileType::StockUpper => true,
            TileType::Grip => true,
        }
    }
    fn illuminates(&self) -> bool {
        match self {
            TileType::Mushroom => true,
            TileType::Candle => true,
            _ => false,
        }
    }
    fn rotation(&self) -> f32 {
        match self {
            TileType::RecLower => 3.14 / 2.0,
            TileType::Stock => 3.14 / 2.0,
            TileType::RearSight => 3.0 * (3.14 / 2.0),
            // TileType::Grip => 2.0 * (3.14 / 2.0),
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub pos: Point3<f32>,
    pub illumination: f32,
    pub tile_type: TileType,
}
