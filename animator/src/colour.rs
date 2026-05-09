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

    /*
    `h` = hue angle in degrees
        0	    red?
        30	    orange?
        90	    yellow?
        140	    green?
        220	    blue?
        300	    magenta?
    `c` = chroma (color intensity)
        0.00	grayscale
        0.02	barely tinted?
        0.05	muted?
        0.10	normal UI color?
        0.15	strong?
        0.25	vivid?
        0.35+	extreme / often clips
    `l` = perceptual lightness, sensible values between
        0       black
      values less than ~0.1 don't produce true perceptual lightness very well. Greens are darker and purples are lightner.
        0.1     quite dark
        1       light
     */
    pub fn oklch_to_rgb(h: f64, c: f64, l: f64) -> Self {
        let h = h.to_radians();

        // OKLCH -> OKLab
        let a = c * h.cos();
        let b = c * h.sin();

        // OKLab -> LMS
        let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
        let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
        let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

        // Cube
        let l3 = l_ * l_ * l_;
        let m3 = m_ * m_ * m_;
        let s3 = s_ * s_ * s_;

        // LMS -> linear RGB
        let r_lin = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
        let g_lin = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
        let b_lin = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;

        // linear RGB -> sRGB
        fn gamma(x: f64) -> f64 {
            if x <= 0.0031308 {
                12.92 * x
            } else {
                1.055 * x.powf(1.0 / 2.4) - 0.055
            }
        }

        let r = gamma(r_lin).clamp(0.0, 1.0);
        let g = gamma(g_lin).clamp(0.0, 1.0);
        let b = gamma(b_lin).clamp(0.0, 1.0);

        Self { r, g, b }
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
