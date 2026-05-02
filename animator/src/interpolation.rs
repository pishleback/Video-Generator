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
struct ImmediateInterp {}

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

#[derive(Debug)]
struct InterpTimelineKeyframe<V> {
    at_t: OrderedFloat<f64>,
    from: V,
    to: V,
    interp: Box<dyn Interp<V>>,
}

#[derive(Debug, Clone, Copy)]
pub enum InterpType {
    Immediate,
    Linear { duration: f64 },
    Exp { duration: f64 },
    Exp2 { duration: f64 },
}

impl InterpType {
    fn interp<V: Interpable + Clone>(self) -> Box<dyn Interp<V>> {
        match self {
            InterpType::Immediate => Box::new(ImmediateInterp {}),
            InterpType::Linear { duration } => Box::new(LinearInterp { duration }),
            InterpType::Exp { duration } => Box::new(ExpInterp { duration }),
            InterpType::Exp2 { duration } => Box::new(Exp2Interp { duration }),
        }
    }
}

#[derive(Debug)]
pub struct InterpTimeline<V> {
    initial_value: V,
    keyframes: Vec<InterpTimelineKeyframe<V>>,
}

impl<V: Clone> InterpTimeline<V> {
    pub fn new(initial_value: V) -> Self {
        Self {
            initial_value,
            keyframes: vec![],
        }
    }

    pub fn get(&self, t: OrderedFloat<f64>) -> V {
        for keyframe in self.keyframes.iter().rev() {
            if keyframe.at_t <= t {
                let dt = t - keyframe.at_t;
                return keyframe.interp.interp(&keyframe.from, &keyframe.to, dt);
            }
        }
        self.initial_value.clone()
    }

    pub fn set_raw(&mut self, t: OrderedFloat<f64>, to: V, interp: Box<dyn Interp<V>>) {
        let from = self.get(t);
        self.keyframes.push(InterpTimelineKeyframe {
            at_t: t,
            from,
            to,
            interp,
        });
        self.keyframes.sort_by_key(|k| k.at_t);
    }
}

impl<V: Clone> InterpTimeline<V> {
    pub fn set_immediate(&mut self, t: OrderedFloat<f64>, to: V) {
        self.set_raw(t, to, Box::new(ImmediateInterp {}));
    }
}

impl<V: Interpable + Clone> InterpTimeline<V> {
    pub fn set(&mut self, t: OrderedFloat<f64>, to: V, interp_type: InterpType) {
        self.set_raw(t, to, interp_type.interp());
    }
}
