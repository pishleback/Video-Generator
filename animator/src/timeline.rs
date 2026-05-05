use ordered_float::OrderedFloat;

use crate::interpolation::{ImmediateInterp, Interp, InterpType, Interpable};

pub trait Timeline<V> {
    fn at_time(&self, t: f64) -> V;
}

#[derive(Debug)]
pub struct ConstantTimeline<V> {
    value: V,
}

impl<V: Clone> ConstantTimeline<V> {
    pub fn new(value: V) -> Self {
        Self { value }
    }
}

impl<V: Clone> Timeline<V> for ConstantTimeline<V> {
    fn at_time(&self, _t: f64) -> V {
        self.value.clone()
    }
}

#[derive(Debug)]
struct InterpTimelineKeyframe<V> {
    at_t: f64,
    from: V,
    to: V,
    interp: Box<dyn Interp<V>>,
}

#[derive(Debug)]
pub struct InterpTimeline<V> {
    default_value: V,
    keyframes: Vec<InterpTimelineKeyframe<V>>,
}

impl<V: Clone + 'static> InterpTimeline<V> {
    pub fn new(default_value: V) -> Self {
        Self {
            default_value,
            keyframes: vec![],
        }
    }

    pub fn set_raw(&mut self, t: f64, from: Option<V>, to: V, interp: Box<dyn Interp<V>>) {
        let from = from.unwrap_or_else(|| self.at_time(t));
        self.keyframes.push(InterpTimelineKeyframe {
            at_t: t,
            from,
            to,
            interp,
        });
        self.keyframes.sort_by_key(|k| OrderedFloat::from(k.at_t));
    }
}

impl<V: Clone + 'static> InterpTimeline<V> {
    pub fn set_immediate(&mut self, t: f64, from: Option<V>, to: V) {
        self.set_raw(t, from, to, Box::new(ImmediateInterp {}));
    }
}

impl<V: Interpable + Clone + 'static> InterpTimeline<V> {
    pub fn set(&mut self, t: f64, from: Option<V>, to: V, interp: InterpType) {
        self.set_raw(t, from, to, interp.interp());
    }
}

impl<V: Clone + 'static> Timeline<V> for InterpTimeline<V> {
    fn at_time(&self, t: f64) -> V {
        for keyframe in self.keyframes.iter().rev() {
            if keyframe.at_t <= t {
                let dt = t - keyframe.at_t;
                return keyframe.interp.interp(&keyframe.from, &keyframe.to, dt);
            }
        }
        if let Some(first) = self.keyframes.first() {
            first.from.clone()
        } else {
            self.default_value.clone()
        }
    }
}
