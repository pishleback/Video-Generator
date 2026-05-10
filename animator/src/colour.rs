use core::f64;

use image::{Rgb, Rgba};
use serde::{Deserialize, Serialize};

// linear RGBA space
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

// linear RGB space
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

    pub fn from_oklab(l: f64, a: f64, b: f64) -> Self {
        let l = l as f32;
        let a = a as f32;
        let b = b as f32;

        // encodes the saturation
        let chroma = (a * a + b * b).sqrt();

        // if the color is basically gray thrn hue direction is unstable so skip all gamut logic
        if chroma < 1e-6 {
            let rgb = oklab::oklab_to_linear_srgb(oklab::Oklab { l, a, b });
            return Self {
                r: rgb.r.clamp(0.0, 1.0) as f64,
                g: rgb.g.clamp(0.0, 1.0) as f64,
                b: rgb.b.clamp(0.0, 1.0) as f64,
            };
        }

        let dir_a = a / chroma;
        let dir_b = b / chroma;

        // binary search for the maximum chroma we can use along this hue direction before RGB goes out of gamut
        // this reduces the chroma so that the rgb clamping later does not destroy hue information in a bad way
        let mut lo = 0.0;
        let mut hi = chroma;
        let mut mid = 0.0;
        for _ in 0..12 {
            mid = 0.5 * (lo + hi);
            let rgb = oklab::oklab_to_linear_srgb(oklab::Oklab {
                l,
                a: dir_a * mid,
                b: dir_b * mid,
            });
            if rgb.r >= 0.0
                && rgb.r <= 1.0
                && rgb.g >= 0.0
                && rgb.g <= 1.0
                && rgb.b >= 0.0
                && rgb.b <= 1.0
            {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let rgb = oklab::oklab_to_linear_srgb(oklab::Oklab {
            l,
            a: dir_a * mid,
            b: dir_b * mid,
        });

        Self {
            r: rgb.r.clamp(0.0, 1.0) as f64,
            g: rgb.g.clamp(0.0, 1.0) as f64,
            b: rgb.b.clamp(0.0, 1.0) as f64,
        }
    }

    /*
    perceptually meaningful parameters
    l: lightness
        0.0     black
        1.0     bright
      between these the blow-out point depends on the hue
        1.5     blown-out white
    c: chroma (colour intensity)
        0.0     greyscale
        0.1     half saturated
        0.2     saturated
        0.3     very saturated
        0.4     over saturated
    h: hue in radians
     */
    pub fn from_oklch(l: f64, c: f64, h: f64) -> Self {
        Self::from_oklab(l, c * h.cos(), c * h.sin())
    }

    pub fn to_rgb(&self) -> Rgb<u8> {
        Rgb([
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        ])
    }

    pub fn alpha(&self, alpha: f64) -> ColourRgba {
        ColourRgba {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }
}

// build colours based on oklch
pub struct ColourBuilder {
    l: f64,
    c: f64,
    h: f64,
}

impl ColourBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            l: 0.5,
            c: 0.0,
            h: 0.0,
        }
    }

    /// Value guide:
    /// - 0.0 for greyscale
    /// - 0.5 for half-saturated
    /// - 1.0 for saturated
    /// - more than 1.0 for very/over saturated
    pub fn saturation(&mut self, saturation: f64) -> &mut Self {
        self.c = 0.2 * saturation;
        self
    }

    /// Value guide:
    /// - 0.0 for black
    /// - 0.5 for mid lightness
    /// - 1.0 for very light
    /// - 1.5 for blown-out white
    pub fn lightness(&mut self, lightness: f64) -> &mut Self {
        self.l = lightness;
        self
    }

    pub fn hue_rad(&mut self, hue_rad: f64) -> &mut Self {
        self.h = hue_rad;
        self
    }

    /// Value guide:
    ///  - 25 for red
    ///  - 45 for orange
    ///  - 65 for gold
    ///  - 85 for yellow
    ///  - 125 for lime
    ///  - 145 for green
    ///  - 185 for cyan
    ///  - 215 for aqua
    ///  - 245 for blue
    ///  - 285 for indigo
    ///  - 305 for violet
    ///  - 325 for pink
    ///  - 345 for purple
    pub fn hue_deg(&mut self, hue_deg: f64) -> &mut Self {
        self.h = f64::consts::TAU * hue_deg / 360.0;
        self
    }

    pub fn red(&mut self) -> &mut Self {
        self.hue_deg(25.0);
        self
    }

    pub fn orange(&mut self) -> &mut Self {
        self.hue_deg(45.0);
        self
    }

    pub fn gold(&mut self) -> &mut Self {
        self.hue_deg(65.0);
        self
    }

    pub fn yellow(&mut self) -> &mut Self {
        self.hue_deg(85.0);
        self
    }

    pub fn chartreuse(&mut self) -> &mut Self {
        self.hue_deg(105.0);
        self
    }

    pub fn lime(&mut self) -> &mut Self {
        self.hue_deg(125.0);
        self
    }

    pub fn green(&mut self) -> &mut Self {
        self.hue_deg(145.0);
        self
    }

    pub fn cyan(&mut self) -> &mut Self {
        self.hue_deg(185.0);
        self
    }

    pub fn aqua(&mut self) -> &mut Self {
        self.hue_deg(215.0);
        self
    }

    pub fn blue(&mut self) -> &mut Self {
        self.hue_deg(245.0);
        self
    }

    pub fn indigo(&mut self) -> &mut Self {
        self.hue_deg(285.0);
        self
    }

    pub fn violet(&mut self) -> &mut Self {
        self.hue_deg(305.0);
        self
    }

    pub fn purple(&mut self) -> &mut Self {
        self.hue_deg(325.0);
        self
    }

    pub fn pink(&mut self) -> &mut Self {
        self.hue_deg(345.0);
        self
    }

    pub fn finish(&self) -> ColourRgb {
        ColourRgb::from_oklch(self.l, self.c, self.h)
    }
}
