use std::ops::{Add, Div, Mul, Sub};

use ordered_float::OrderedFloat;

use crate::interpolation::Interpable;

// Divide the width and height into this many units
// Also used for thinkness of lines based on the average of the width and height
pub const SCREEN_UNITS: f64 = 1000.0;

#[derive(Debug, Clone, Copy)]
pub struct Pos2<const WIDTH: u32, const HEIGHT: u32> {
    // in pixels
    x: f64,
    y: f64,
}

impl<const WIDTH: u32, const HEIGHT: u32> Pos2<WIDTH, HEIGHT> {
    // x is how much WIDTH / SCREEN_UNITS
    // y is how much HEIGHT / SCREEN_UNITS
    pub fn from_units(x: f64, y: f64) -> Self {
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
    pub fn from_units(x: f64, y: f64) -> Self {
        let avg = (WIDTH as f64 * HEIGHT as f64).sqrt();
        Self {
            x: x * avg / SCREEN_UNITS,
            y: y * avg / SCREEN_UNITS,
        }
    }

    pub fn from_lengths(x: Length<WIDTH, HEIGHT>, y: Length<WIDTH, HEIGHT>) -> Self {
        Self { x: x.v, y: y.v }
    }

    pub fn to_lengths(&self) -> (Length<WIDTH, HEIGHT>, Length<WIDTH, HEIGHT>) {
        (Length { v: self.x }, Length { v: self.y })
    }

    pub fn from_x_and_slope(
        x: Length<WIDTH, HEIGHT>,
        slope: (impl Into<f64>, impl Into<f64>),
    ) -> Self {
        Self::from_lengths(x, x * slope.1.into() / slope.0.into())
    }

    pub fn from_y_and_slope(
        y: Length<WIDTH, HEIGHT>,
        slope: (impl Into<f64>, impl Into<f64>),
    ) -> Self {
        Self::from_lengths(y * slope.0.into() / slope.1.into(), y)
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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Length<const WIDTH: u32, const HEIGHT: u32> {
    // in pixels
    v: f64,
}

impl<const WIDTH: u32, const HEIGHT: u32> Length<WIDTH, HEIGHT> {
    // x and y are how much (WIDTH * HEIGHT) / SCREEN_UNITS
    pub fn from_units(v: f64) -> Self {
        let avg = (WIDTH as f64 * HEIGHT as f64).sqrt();
        Self {
            v: v * avg / SCREEN_UNITS,
        }
    }

    pub fn pixels(&self) -> f64 {
        self.v
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> Interpable for Length<WIDTH, HEIGHT> {
    fn interp(a: &Self, b: &Self, f: f64) -> Self {
        Self {
            v: f64::interp(&a.v, &b.v, f),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rect<const WIDTH: u32, const HEIGHT: u32> {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl<const WIDTH: u32, const HEIGHT: u32> Rect<WIDTH, HEIGHT> {
    pub fn new(mut x1: f64, mut x2: f64, mut y1: f64, mut y2: f64) -> Self {
        if x2 < x1 {
            (x1, x2) = (x2, x1);
        }
        if y2 < y1 {
            (y1, y2) = (y2, y1);
        }
        Self {
            min_x: x1,
            max_x: x2,
            min_y: y1,
            max_y: y2,
        }
    }

    pub fn from_pos_and_vec(pos: Pos2<WIDTH, HEIGHT>, vec: Vec2<WIDTH, HEIGHT>) -> Self {
        Self::new(pos.x, pos.x + vec.x, pos.y, pos.y + vec.y)
    }

    pub fn fullscreen() -> Self {
        Self {
            min_x: 0.0,
            max_x: WIDTH as f64,
            min_y: 0.0,
            max_y: HEIGHT as f64,
        }
    }

    pub fn pad(&self, dist: Length<WIDTH, HEIGHT>) -> Self {
        let mid = self.center();
        Self {
            min_x: mid.x.min(self.min_x + dist.v),
            max_x: mid.x.max(self.max_x - dist.v),
            min_y: mid.y.min(self.min_y + dist.v),
            max_y: mid.y.max(self.max_y - dist.v),
        }
    }

    pub fn split_vertical(&self, f: f64) -> (Self, Self) {
        assert!(0.0 < f);
        assert!(f < 1.0);
        let m = self.min_x + f * (self.max_x - self.min_x);
        (
            Self {
                min_x: self.min_x,
                max_x: m,
                min_y: self.min_y,
                max_y: self.max_y,
            },
            Self {
                min_x: m,
                max_x: self.max_x,
                min_y: self.min_y,
                max_y: self.max_y,
            },
        )
    }

    pub fn split_horizontal(&self, f: f64) -> (Self, Self) {
        assert!(0.0 < f);
        assert!(f < 1.0);
        let m = self.min_y + f * (self.max_y - self.min_y);
        (
            Self {
                min_x: self.min_x,
                max_x: self.max_x,
                min_y: self.min_y,
                max_y: m,
            },
            Self {
                min_x: self.min_x,
                max_x: self.max_x,
                min_y: m,
                max_y: self.max_y,
            },
        )
    }

    pub fn top_left(&self) -> Pos2<WIDTH, HEIGHT> {
        Pos2 { x: 0.0, y: 0.0 }
    }

    pub fn left_half(&self) -> Self {
        self.split_vertical(0.5).0
    }

    pub fn right_half(&self) -> Self {
        self.split_vertical(0.5).1
    }

    pub fn top_half(&self) -> Self {
        self.split_horizontal(0.5).0
    }

    pub fn bottom_half(&self) -> Self {
        self.split_horizontal(0.5).1
    }

    pub fn center(&self) -> Pos2<WIDTH, HEIGHT> {
        Pos2 {
            x: 0.5 * (self.min_x + self.max_x),
            y: 0.5 * (self.min_y + self.max_y),
        }
    }

    pub fn width(&self) -> Length<WIDTH, HEIGHT> {
        Length {
            v: self.max_x - self.min_x,
        }
    }

    pub fn height(&self) -> Length<WIDTH, HEIGHT> {
        Length {
            v: self.max_y - self.min_y,
        }
    }

    pub fn size(&self) -> Vec2<WIDTH, HEIGHT> {
        Vec2 {
            x: self.max_x - self.min_x,
            y: self.max_y - self.min_y,
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min_x: *std::cmp::min(
                OrderedFloat::from(self.min_x),
                OrderedFloat::from(other.min_x),
            ),
            max_x: *std::cmp::max(
                OrderedFloat::from(self.max_x),
                OrderedFloat::from(other.max_x),
            ),
            min_y: *std::cmp::min(
                OrderedFloat::from(self.min_y),
                OrderedFloat::from(other.min_y),
            ),
            max_y: *std::cmp::max(
                OrderedFloat::from(self.max_y),
                OrderedFloat::from(other.max_y),
            ),
        }
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> Interpable for Rect<WIDTH, HEIGHT> {
    fn interp(a: &Self, b: &Self, f: f64) -> Self {
        Self {
            min_x: f64::interp(&a.min_x, &b.min_x, f),
            max_x: f64::interp(&a.max_x, &b.max_x, f),
            min_y: f64::interp(&a.min_y, &b.min_y, f),
            max_y: f64::interp(&a.max_y, &b.max_y, f),
        }
    }
}

// pos + vec
impl<const WIDTH: u32, const HEIGHT: u32> Add<Vec2<WIDTH, HEIGHT>> for Pos2<WIDTH, HEIGHT> {
    type Output = Pos2<WIDTH, HEIGHT>;
    fn add(self, other: Vec2<WIDTH, HEIGHT>) -> Self::Output {
        Pos2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

// pos - vec
impl<const WIDTH: u32, const HEIGHT: u32> Sub<Vec2<WIDTH, HEIGHT>> for Pos2<WIDTH, HEIGHT> {
    type Output = Pos2<WIDTH, HEIGHT>;
    fn sub(self, other: Vec2<WIDTH, HEIGHT>) -> Self::Output {
        Pos2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

// length * scalar
impl<const WIDTH: u32, const HEIGHT: u32> Mul<f64> for Length<WIDTH, HEIGHT> {
    type Output = Length<WIDTH, HEIGHT>;
    fn mul(self, other: f64) -> Self::Output {
        Length { v: self.v * other }
    }
}

// scalar * length
impl<const WIDTH: u32, const HEIGHT: u32> Mul<Length<WIDTH, HEIGHT>> for f64 {
    type Output = Length<WIDTH, HEIGHT>;
    fn mul(self, other: Length<WIDTH, HEIGHT>) -> Self::Output {
        Length { v: self * other.v }
    }
}

// length / scalar
impl<const WIDTH: u32, const HEIGHT: u32> Div<f64> for Length<WIDTH, HEIGHT> {
    type Output = Length<WIDTH, HEIGHT>;
    fn div(self, other: f64) -> Self::Output {
        Length { v: self.v / other }
    }
}
