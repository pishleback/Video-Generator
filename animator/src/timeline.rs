use crate::interpolation::{ImmediateInterp, Interp, InterpType, Interpable};
use ordered_float::OrderedFloat;

pub trait Timeline<V> {
    fn at_time(&self, t: OrderedFloat<f64>) -> V;
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
    fn at_time(&self, _t: OrderedFloat<f64>) -> V {
        self.value.clone()
    }
}

#[derive(Debug)]
struct InterpTimelineKeyframe<V> {
    at_t: OrderedFloat<f64>,
    from: V,
    to: V,
    interp: Box<dyn Interp<V>>,
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

    pub fn set_raw(&mut self, t: OrderedFloat<f64>, to: V, interp: Box<dyn Interp<V>>) {
        let from = self.at_time(t);
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

impl<V: Clone> Timeline<V> for InterpTimeline<V> {
    fn at_time(&self, t: OrderedFloat<f64>) -> V {
        for keyframe in self.keyframes.iter().rev() {
            if keyframe.at_t <= t {
                let dt = t - keyframe.at_t;
                return keyframe.interp.interp(&keyframe.from, &keyframe.to, dt);
            }
        }
        self.initial_value.clone()
    }
}
