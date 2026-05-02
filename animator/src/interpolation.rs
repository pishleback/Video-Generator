use ordered_float::OrderedFloat;

use crate::colour::ColourRgba;
use std::fmt::Debug;

const DEFAULT_DURATION: f64 = 0.5;

pub trait Interpable {
    fn interp(x: &Self, y: &Self, f: f64) -> Self;
}

impl Interpable for f64 {
    fn interp(x: &f64, y: &f64, f: f64) -> f64 {
        if f <= 0.0 {
            *x
        } else if f >= 1.0 {
            *y
        } else {
            x + f * (y - x)
        }
    }
}

impl<V1: Interpable, V2: Interpable> Interpable for (V1, V2) {
    fn interp(x: &(V1, V2), y: &(V1, V2), f: f64) -> (V1, V2) {
        (V1::interp(&x.0, &y.0, f), V2::interp(&x.1, &y.1, f))
    }
}

impl Interpable for ColourRgba {
    fn interp(x: &ColourRgba, y: &ColourRgba, f: f64) -> ColourRgba {
        ColourRgba {
            r: f64::interp(&x.r, &y.r, f),
            g: f64::interp(&x.g, &y.g, f),
            b: f64::interp(&x.b, &y.b, f),
            a: f64::interp(&x.a, &y.a, f),
        }
    }
}

pub trait Interp<V>: Debug {
    // dt <= 0 should return `from`
    // dt >0 should from `from` towards `to`
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V;
}

#[derive(Debug)]
pub struct ImmediateInterp {}

impl<V: Clone> Interp<V> for ImmediateInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        if *dt < 0.0 { from.clone() } else { to.clone() }
    }
}

#[derive(Debug)]
pub struct LinearInterp {
    pub duration: f64,
}

impl Default for LinearInterp {
    fn default() -> Self {
        Self {
            duration: DEFAULT_DURATION,
        }
    }
}

impl<V: Interpable + Clone> Interp<V> for LinearInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        V::interp(from, to, *dt / self.duration)
    }
}

fn exp_smooth(f: f64) -> f64 {
    if f <= 0.0 {
        0.0
    } else if f >= 1.0 {
        1.0
    } else {
        0.5 * (1.0 - (std::f64::consts::PI * f).cos())
    }
}

#[derive(Debug)]
pub struct ExpInterp {
    pub duration: f64,
}

impl Default for ExpInterp {
    fn default() -> Self {
        Self {
            duration: DEFAULT_DURATION,
        }
    }
}

impl<V: Interpable + Clone> Interp<V> for ExpInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        V::interp(from, to, exp_smooth(*dt / self.duration))
    }
}

#[derive(Debug)]
pub struct Exp2Interp {
    pub duration: f64,
}

impl Default for Exp2Interp {
    fn default() -> Self {
        Self {
            duration: DEFAULT_DURATION,
        }
    }
}

impl<V: Interpable + Clone> Interp<V> for Exp2Interp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        V::interp(from, to, exp_smooth(exp_smooth(*dt / self.duration)))
    }
}

#[derive(Debug)]
pub struct FastStartInterp {
    pub duration: f64,
}

impl Default for FastStartInterp {
    fn default() -> Self {
        Self {
            duration: DEFAULT_DURATION,
        }
    }
}

impl<V: Interpable + Clone> Interp<V> for FastStartInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        fn f(x: f64) -> f64 {
            if x <= 0.0 {
                0.0
            } else if x >= 1.0 {
                1.0
            } else {
                1.0 - (1.0 - x) * (1.0 - x)
            }
        }
        V::interp(from, to, f(*dt / self.duration))
    }
}

#[derive(Debug)]
pub struct FastEndInterp {
    pub duration: f64,
}

impl Default for FastEndInterp {
    fn default() -> Self {
        Self {
            duration: DEFAULT_DURATION,
        }
    }
}

impl<V: Interpable + Clone> Interp<V> for FastEndInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        fn f(x: f64) -> f64 {
            if x <= 0.0 {
                0.0
            } else if x >= 1.0 {
                1.0
            } else {
                x * x
            }
        }
        V::interp(from, to, f(*dt / self.duration))
    }
}
