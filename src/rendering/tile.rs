use na::*;

use crate::rendering::{drawable::*, font::*};

use ggez::graphics::{Color, Rect};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TileType {
    Air,
    Rock0,
    Rock1,
    Rock2,
    Rock3,
    Rock4,
    Rock5,
    Rock6,
    Rock7,
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

impl TileType {
    pub fn collides(&self) -> bool {
        match self {
            TileType::Air => false,
            TileType::Rock0 | TileType::Rock1 | TileType::Rock2 | TileType::Rock3 | TileType::Rock4 | TileType::Rock5 | TileType::Rock6 | TileType::Rock7 => true,
            TileType::Mushroom => false,
            TileType::Candle => false,
            TileType::StockUpper => false,
            TileType::Stock => false,
            _ => todo!(),
        }
    }
}

impl Drawable for TileType {
    fn get_char_offset(&self, font: &KataFont) -> Rect {
        match self {
            TileType::Air => font.get_src_rect(0),
            TileType::Rock0 => font.get_src_rect(0x2B0),
            TileType::Rock1 => font.get_src_rect(0x2B1),
            TileType::Rock2 => font.get_src_rect(0x2B2),
            TileType::Rock3 => font.get_src_rect(0x2B3),
            TileType::Rock4 => font.get_src_rect(0x2B4),
            TileType::Rock5 => font.get_src_rect(0x2B5),
            TileType::Rock6 => font.get_src_rect(0x2B6),
            TileType::Rock7 => font.get_src_rect(0x2B7),
            TileType::Mushroom => font.get_src_rect(0x2E1),
            TileType::Candle => font.get_src_rect(0x21A),
            TileType::FrontSight => font.get_src_rect(0x211),
            TileType::RearSight => font.get_src_rect(0x203),
            TileType::GasBlock => font.get_src_rect(0x7C),
            TileType::Barrel => font.get_src_rect(0x3A),
            TileType::BarrelEnd => font.get_src_rect(0x2E9),
            TileType::RecUpper => font.get_src_rect(0x2DD),
            TileType::RecLower => font.get_src_rect(0x319),
            TileType::RecLowerHalf => font.get_src_rect(0xDF),
            TileType::RecLowerBack => font.get_src_rect(0x2C5),
            TileType::Magazine => font.get_src_rect(0x1AB),
            TileType::Stock => font.get_src_rect(0x319),
            TileType::StockUpper => font.get_src_rect(0x2DD),
            TileType::Grip => font.get_src_rect(0x283),
        }
    }
    fn get_color(&self) -> Color {
        match self {
            TileType::Air => Color::new(0.0, 0.0, 0.0, 0.0),
            TileType::Rock0 | TileType::Rock1 | TileType::Rock2 | TileType::Rock3 | TileType::Rock4 | TileType::Rock5 | TileType::Rock6 | TileType::Rock7 => Color::new(0.5, 0.5, 0.5, 1.0),
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
            TileType::Rock0 | TileType::Rock1 | TileType::Rock2 | TileType::Rock3 | TileType::Rock4 | TileType::Rock5 | TileType::Rock6 | TileType::Rock7 => false,
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
    pub illumination_color: Color,
    pub tile_type: TileType,
}

impl Tile {
    pub fn illuminated(&self) -> bool {
        self.illumination_color.r > 0.0
            || self.illumination_color.g > 0.0
            || self.illumination_color.b > 0.0
    }
}
