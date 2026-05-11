use ordered_float::OrderedFloat;

#[derive(Debug, Clone)]
pub struct BoundingRect {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl BoundingRect {
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

    pub fn min_x(&self) -> f64 {
        self.min_x
    }

    pub fn max_x(&self) -> f64 {
        self.max_x
    }

    pub fn min_y(&self) -> f64 {
        self.min_y
    }

    pub fn max_y(&self) -> f64 {
        self.max_y
    }

    pub fn center(&self) -> (f64, f64) {
        (
            0.5 * (self.min_x + self.max_x),
            0.5 * (self.min_y + self.max_y),
        )
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
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
