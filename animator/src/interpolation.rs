use ordered_float::OrderedFloat;

use crate::colour::ColourRgba;
use std::fmt::Debug;

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
struct LinearInterp {
    duration: f64,
}

impl<V: Interpable + Clone> Interp<V> for LinearInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        V::interp(from, to, *dt / self.duration)
    }
}

#[derive(Debug)]
struct ExpInterp {
    duration: f64,
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

impl<V: Interpable + Clone> Interp<V> for ExpInterp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        V::interp(from, to, exp_smooth(*dt / self.duration))
    }
}

#[derive(Debug)]
struct Exp2Interp {
    duration: f64,
}

impl<V: Interpable + Clone> Interp<V> for Exp2Interp {
    fn interp(&self, from: &V, to: &V, dt: OrderedFloat<f64>) -> V {
        V::interp(from, to, exp_smooth(exp_smooth(*dt / self.duration)))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterpType {
    Immediate,
    Linear { duration: f64 },
    Exp { duration: f64 },
    Exp2 { duration: f64 },
}

impl InterpType {
    pub fn interp<V: Interpable + Clone>(self) -> Box<dyn Interp<V>> {
        match self {
            InterpType::Immediate => Box::new(ImmediateInterp {}),
            InterpType::Linear { duration } => Box::new(LinearInterp { duration }),
            InterpType::Exp { duration } => Box::new(ExpInterp { duration }),
            InterpType::Exp2 { duration } => Box::new(Exp2Interp { duration }),
        }
    }
}
