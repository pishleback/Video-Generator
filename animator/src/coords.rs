use crate::interpolation::Interpable;

// Divide the width and height into this many units
// Also used for thinkness of lines based on the average of the width and height
pub const SCREEN_UNITS: f64 = 100.0;

#[derive(Debug, Clone, Copy)]
pub struct Pos2<const WIDTH: u32, const HEIGHT: u32> {
    // in pixels
    x: f64,
    y: f64,
}

impl<const WIDTH: u32, const HEIGHT: u32> Pos2<WIDTH, HEIGHT> {
    // x is how much WIDTH / SCREEN_UNITS
    // y is how much HEIGHT / SCREEN_UNITS
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x: x * (WIDTH as f64) / SCREEN_UNITS,
            y: y * (HEIGHT as f64) / SCREEN_UNITS,
        }
    }

    pub fn pixels(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> Interpable for Pos2<WIDTH, HEIGHT> {
    fn interp(a: &Self, b: &Self, f: f64) -> Self {
        Self {
            x: f64::interp(&a.x, &b.x, f),
            y: f64::interp(&a.y, &b.y, f),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vec2<const WIDTH: u32, const HEIGHT: u32> {
    // in pixels
    x: f64,
    y: f64,
}

impl<const WIDTH: u32, const HEIGHT: u32> Vec2<WIDTH, HEIGHT> {
    // x and y are how much (WIDTH * HEIGHT) / SCREEN_UNITS
    pub fn new(x: f64, y: f64) -> Self {
        let avg = (WIDTH as f64 * HEIGHT as f64).sqrt();
        Self {
            x: x * avg / SCREEN_UNITS,
            y: y * avg / SCREEN_UNITS,
        }
    }

    pub fn from_x_and_slope(x: f64, slope: (impl Into<f64>, impl Into<f64>)) -> Self {
        Self::new(x, x * slope.1.into() / slope.0.into())
    }

    pub fn from_y_and_slope(y: f64, slope: (impl Into<f64>, impl Into<f64>)) -> Self {
        Self::new(y * slope.0.into() / slope.1.into(), y)
    }

    pub fn pixels(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> Interpable for Vec2<WIDTH, HEIGHT> {
    fn interp(a: &Self, b: &Self, f: f64) -> Self {
        Self {
            x: f64::interp(&a.x, &b.x, f),
            y: f64::interp(&a.y, &b.y, f),
        }
    }
}
