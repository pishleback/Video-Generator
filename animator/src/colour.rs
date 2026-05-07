use image::{Rgb, Rgba};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColourRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl ColourRgb {
    /// h: 0.0..360.0
    /// s: 0.0..1.0
    /// l: 0.0..1.0
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let h_prime = h / 60.0;
        let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

        let (r1, g1, b1) = match h_prime {
            hp if (0.0..1.0).contains(&hp) => (c, x, 0.0),
            hp if (1.0..2.0).contains(&hp) => (x, c, 0.0),
            hp if (2.0..3.0).contains(&hp) => (0.0, c, x),
            hp if (3.0..4.0).contains(&hp) => (0.0, x, c),
            hp if (4.0..5.0).contains(&hp) => (x, 0.0, c),
            hp if (5.0..6.0).contains(&hp) => (c, 0.0, x),
            _ => (0.0, 0.0, 0.0),
        };

        let m = l - c / 2.0;

        Self {
            r: r1 + m,
            g: g1 + m,
            b: b1 + m,
        }
    }

    pub fn to_rgb(&self) -> Rgb<u8> {
        Rgb([
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        ])
    }

    pub fn to_rgba(&self, alpha: f64) -> ColourRgba {
        ColourRgba {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }
}
