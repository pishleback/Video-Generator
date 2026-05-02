use image::{Rgb, Rgba};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColourRgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl ColourRgba {
    pub fn to_rgba(&self) -> Rgba<u8> {
        Rgba([
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColourRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl ColourRgb {
    pub fn to_rgb(&self) -> Rgb<u8> {
        Rgb([
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        ])
    }
}
