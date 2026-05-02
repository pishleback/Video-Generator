use core::f64;
use std::rc::Rc;

use crate::{
    colour::{ColourRgb, ColourRgba},
    image::ImageSpec,
    interpolation::{InterpTimeline, InterpType},
    shape::ShapeSpec,
    video::VideoSpec,
};
use ordered_float::OrderedFloat;
use std::fmt::Debug;

pub trait AnimationElement<const WIDTH: u32, const HEIGHT: u32> {
    fn apply(&self, t: OrderedFloat<f64>, image_spec: ImageSpec) -> ImageSpec;
}

#[derive(Debug, Clone, Copy)]
pub enum BoundaryMode {
    Inner,
    Middle,
    Outer,
}

pub struct ShapeElement<const WIDTH: u32, const HEIGHT: u32> {
    shape: ShapeSpec,
    fill_colour: InterpTimeline<ColourRgba>,
    boundary_colour: InterpTimeline<ColourRgba>,
    boundary_frac: InterpTimeline<(f64, f64)>,
    boundary_mode: InterpTimeline<BoundaryMode>,
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElement<WIDTH, HEIGHT> {
    pub fn new(
        shape: ShapeSpec,
        fill_colour: ColourRgba,
        boundary_colour: ColourRgba,
        boundary_frac: (f64, f64),
        boundary_mode: BoundaryMode,
    ) -> Self {
        Self {
            shape,
            fill_colour: InterpTimeline::new(fill_colour),
            boundary_colour: InterpTimeline::new(boundary_colour),
            boundary_frac: InterpTimeline::new(boundary_frac),
            boundary_mode: InterpTimeline::new(boundary_mode),
        }
    }

    pub fn set_fill_rgba(mut self, t: f64, v: ColourRgba, interp_type: InterpType) -> Self {
        self.fill_colour.set(t.into(), v, interp_type);
        self
    }

    pub fn set_fill_rgb(
        mut self,
        t: f64,
        ColourRgb { r, g, b }: ColourRgb,
        interp_type: InterpType,
    ) -> Self {
        let a = self.fill_colour.get(t.into()).a;
        self.fill_colour
            .set(t.into(), ColourRgba { r, g, b, a }, interp_type);
        self
    }

    pub fn set_fill_alpha(mut self, t: f64, alpha: f64, interp_type: InterpType) -> Self {
        let ColourRgba { r, g, b, .. } = self.fill_colour.get(t.into());
        self.fill_colour
            .set(t.into(), ColourRgba { r, g, b, a: alpha }, interp_type);
        self
    }

    pub fn set_boundary_rgba(mut self, t: f64, v: ColourRgba, interp_type: InterpType) -> Self {
        self.boundary_colour.set(t.into(), v, interp_type);
        self
    }

    pub fn set_boundary_rgb(
        mut self,
        t: f64,
        ColourRgb { r, g, b }: ColourRgb,
        interp_type: InterpType,
    ) -> Self {
        let a = self.boundary_colour.get(t.into()).a;
        self.boundary_colour
            .set(t.into(), ColourRgba { r, g, b, a }, interp_type);
        self
    }

    pub fn set_boundary_alpha(mut self, t: f64, alpha: f64, interp_type: InterpType) -> Self {
        let ColourRgba { r, g, b, .. } = self.boundary_colour.get(t.into());
        self.boundary_colour
            .set(t.into(), ColourRgba { r, g, b, a: alpha }, interp_type);
        self
    }

    pub fn set_boundary_frac(mut self, t: f64, v: (f64, f64), interp_type: InterpType) -> Self {
        self.boundary_frac.set(t.into(), v, interp_type);
        self
    }

    pub fn set_boundary_mode(mut self, t: f64, v: BoundaryMode) -> Self {
        self.boundary_mode.set_immediate(t.into(), v);
        self
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for ShapeElement<WIDTH, HEIGHT>
{
    fn apply(&self, t: OrderedFloat<f64>, image_spec: ImageSpec) -> ImageSpec {
        let shape = self
            .shape
            .scale((WIDTH / 2) as f64)
            .translate((WIDTH / 2) as f64, (HEIGHT / 2) as f64);

        let shape_boundary = match self.boundary_mode.get(t) {
            BoundaryMode::Inner => shape
                .partial_boundary(6.0, self.boundary_frac.get(t))
                .intersect(&shape),
            BoundaryMode::Middle => shape.partial_boundary(3.0, self.boundary_frac.get(t)),
            BoundaryMode::Outer => shape
                .partial_boundary(6.0, self.boundary_frac.get(t))
                .subtract(&shape),
        };

        ImageSpec::BlitStack {
            width: WIDTH,
            height: HEIGHT,
            images: vec![
                ((0.0, 0.0), image_spec),
                ((0.0, 0.0), {
                    let ColourRgba { r, g, b, a } = self.fill_colour.get(t);
                    shape.image(
                        WIDTH,
                        HEIGHT,
                        ColourRgba { r, g, b, a: 0.0 },
                        ColourRgba { r, g, b, a },
                    )
                }),
                ((0.0, 0.0), {
                    let ColourRgba { r, g, b, a } = self.boundary_colour.get(t);
                    shape_boundary.image(
                        WIDTH,
                        HEIGHT,
                        ColourRgba { r, g, b, a: 0.0 },
                        ColourRgba { r, g, b, a },
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
    pub fn new(default_bg: ColourRgba) -> Self {
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
