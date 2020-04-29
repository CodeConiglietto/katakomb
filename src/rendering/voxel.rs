use na::*;

use crate::rendering::{drawable::*, font::*};

use ggez::graphics::{Color, Rect};

#[derive(Debug, PartialEq, Clone)]
pub enum VoxelType {
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
pub struct Voxel {
    pub pos: Point3<f32>,
    pub illumination: f32,
    pub voxel_type: VoxelType,
}
