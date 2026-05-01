use core::f64;
use std::rc::Rc;

use crate::{colour::Colour, image::ImageSpec, shape::ShapeSpec, video::VideoSpec};
use ordered_float::OrderedFloat;
use std::fmt::Debug;

trait Interpable {
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

impl Interpable for Colour {
    fn interp(x: &Colour, y: &Colour, f: f64) -> Colour {
        Colour {
            r: f64::interp(&x.r, &y.r, f),
            g: f64::interp(&x.g, &y.g, f),
            b: f64::interp(&x.b, &y.b, f),
            a: f64::interp(&x.a, &y.a, f),
        }
    }
}

trait Interp<V>: Debug {
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
        0.5 * (1.0 - (f64::consts::PI * f).cos())
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
struct InterpTimeline<V> {
    initial_value: V,
    keyframes: Vec<InterpTimelineKeyframe<V>>,
}

impl<V: Clone> InterpTimeline<V> {
    fn new(initial_value: V) -> Self {
        Self {
            initial_value,
            keyframes: vec![],
        }
    }

    fn get(&self, t: OrderedFloat<f64>) -> V {
        for keyframe in self.keyframes.iter().rev() {
            if keyframe.at_t <= t {
                let dt = t - keyframe.at_t;
                return keyframe.interp.interp(&keyframe.from, &keyframe.to, dt);
            }
        }
        self.initial_value.clone()
    }

    fn set_raw(&mut self, t: OrderedFloat<f64>, to: V, interp: Box<dyn Interp<V>>) {
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

impl InterpTimeline<bool> {
    pub fn set(&mut self, t: OrderedFloat<f64>, to: bool) {
        self.set_raw(t, to, Box::new(ImmediateInterp {}));
    }
}

impl<V: Interpable + Clone> InterpTimeline<V> {
    pub fn set(&mut self, t: OrderedFloat<f64>, to: V, interp_type: InterpType) {
        self.set_raw(t, to, interp_type.interp());
    }
}

pub trait AnimationElement<const WIDTH: u32, const HEIGHT: u32> {
    fn apply(&self, t: OrderedFloat<f64>, image_spec: ImageSpec) -> ImageSpec;
}

pub struct ShapeElement<const WIDTH: u32, const HEIGHT: u32> {
    shape: ShapeSpec,
    fill_colour: InterpTimeline<Colour>,
    boundary_colour: InterpTimeline<Colour>,
    boundary_frac: InterpTimeline<(f64, f64)>,
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElement<WIDTH, HEIGHT> {
    pub fn new(
        shape: ShapeSpec,
        fill_colour: Colour,
        boundary_colour: Colour,
        boundary_frac: (f64, f64),
    ) -> Self {
        Self {
            shape,
            fill_colour: InterpTimeline::new(fill_colour),
            boundary_colour: InterpTimeline::new(boundary_colour),
            boundary_frac: InterpTimeline::new(boundary_frac),
        }
    }

    pub fn set_fill_colour(mut self, t: f64, v: Colour, interp_type: InterpType) -> Self {
        self.fill_colour.set(t.into(), v, interp_type);
        self
    }

    pub fn set_boundary_colour(mut self, t: f64, v: Colour, interp_type: InterpType) -> Self {
        self.boundary_colour.set(t.into(), v, interp_type);
        self
    }

    pub fn set_boundary_frac(mut self, t: f64, v: (f64, f64), interp_type: InterpType) -> Self {
        self.boundary_frac.set(t.into(), v, interp_type);
        self
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for ShapeElement<WIDTH, HEIGHT>
{
    fn apply(&self, t: OrderedFloat<f64>, image_spec: ImageSpec) -> ImageSpec {
        ImageSpec::BlitStack {
            width: WIDTH,
            height: HEIGHT,
            images: vec![
                ((0.0, 0.0), image_spec),
                ((0.0, 0.0), {
                    let Colour { r, g, b, a } = self.fill_colour.get(t);
                    self.shape
                        .clone()
                        .scale((WIDTH / 5) as f64)
                        .translate((2 * WIDTH / 5) as f64, (3 * HEIGHT / 5) as f64)
                        .image(
                            WIDTH,
                            HEIGHT,
                            Colour { r, g, b, a: 0.0 },
                            Colour { r, g, b, a },
                        )
                }),
                ((0.0, 0.0), {
                    let Colour { r, g, b, a } = self.boundary_colour.get(t);
                    self.shape
                        .clone()
                        .scale((WIDTH / 5) as f64)
                        .translate((2 * WIDTH / 5) as f64, (3 * HEIGHT / 5) as f64)
                        .partial_boundary(2.0, self.boundary_frac.get(t))
                        .intersection(
                            self.shape
                                .clone()
                                .scale((WIDTH / 5) as f64)
                                .translate((2 * WIDTH / 5) as f64, (3 * HEIGHT / 5) as f64),
                        )
                        .image(
                            WIDTH,
                            HEIGHT,
                            Colour { r, g, b, a: 0.0 },
                            Colour { r, g, b, a },
                        )
                }),
            ],
        }
    }
}

pub struct Animation<const WIDTH: u32, const HEIGHT: u32> {
    default_image: ImageSpec,
    elements: Vec<Rc<dyn AnimationElement<WIDTH, HEIGHT>>>,
}

impl<const WIDTH: u32, const HEIGHT: u32> Animation<WIDTH, HEIGHT> {
    pub fn new(default_bg: Colour) -> Self {
        Self {
            default_image: ImageSpec::Filled {
                width: WIDTH,
                height: HEIGHT,
                colour: default_bg,
            },
            elements: vec![],
        }
    }

    pub fn add(&mut self, element: impl AnimationElement<WIDTH, HEIGHT> + 'static) {
        let element = Rc::new(element);
        self.elements.push(element);
    }

    fn frame(&self, t: OrderedFloat<f64>) -> ImageSpec {
        let mut image_spec = self.default_image.clone();
        for element in &self.elements {
            image_spec = element.apply(t, image_spec);
        }
        image_spec
    }

    pub fn video(
        &self,
        from_t: impl Into<OrderedFloat<f64>>,
        to_t: impl Into<OrderedFloat<f64>>,
        fps: f64,
    ) -> VideoSpec {
        let from_t = from_t.into();
        let to_t = to_t.into();
        assert!(from_t <= to_t);
        let dt = 1.0 / fps;
        let mut images = vec![];
        let mut t = from_t;
        while t <= to_t {
            images.push(self.frame(t));
            t += dt;
        }
        VideoSpec {
            width: WIDTH,
            height: HEIGHT,
            fps,
            images,
        }
    }
}
