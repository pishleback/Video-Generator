use std::sync::Arc;

use crate::{
    colour::ColourWithAlpha,
    coords::{Length, Pos2},
    shape::ShapeSpec,
    slideshow::{
        ShapeVisualOptions,
        instantaneous::{InstantaneousInterpOptions, InstantaneousSlideElement},
        timeline::Timeline,
    },
};

use super::{InterpId, InterpType};

/*
Options for how an object on a slide should be interpolated from the previous slide and to the next slide

`interp_from_id` and `interp_to_id` are matched against objects on the adjacent slides to decide which objects should be interpolated

`interp_from_type` and `interp_to_type` define the type of interpolation to use
 - if both are None then a default interpolation is used
 - if exactly one is Some, then that interpolation is used
 - if both are Some, then an interpolated interpolation is used
*/
#[derive(Debug, Clone, Default)]
pub struct TemporalInterpOptions {
    pub interp_from_id: Option<InterpId>,
    pub interp_to_id: Option<InterpId>,
    pub interp_from_type: Option<InterpType>,
    pub interp_to_type: Option<InterpType>,
}

#[derive(Clone)]
pub struct TemporalShapeOptions {
    pub interp: TemporalInterpOptions,
    pub visuals: Timeline<ShapeVisualOptions>,
}

/*
A thing to be drawn on the slide with temporal continuity
These are the elements it's possible to interpolate between slides
This is in contrast to the instantaneous slide elements
*/
#[derive(Clone)]
pub enum TemporalSlideElement<const W: u32, const H: u32> {
    Circle {
        center: Timeline<Pos2<W, H>>,
        radius: Timeline<Length<W, H>>,
        options: TemporalShapeOptions,
    },
    Line {
        start: Timeline<Pos2<W, H>>,
        end: Timeline<Pos2<W, H>>,
        radius: Timeline<Length<W, H>>,
        options: TemporalShapeOptions,
    },
    Shape {
        shape: Timeline<ShapeSpec>, // in pixel coords
        options: TemporalShapeOptions,
    },
    Pixels {
        min: Pos2<W, H>,
        max: Pos2<W, H>,
        pixels: Timeline<Arc<dyn Fn(Pos2<W, H>) -> ColourWithAlpha + Send + Sync>>,
        interp: TemporalInterpOptions,
    },
}

impl<const W: u32, const H: u32> TemporalSlideElement<W, H> {
    pub fn interp_options(&self) -> &TemporalInterpOptions {
        match self {
            Self::Circle { options, .. } => &options.interp,
            Self::Line { options, .. } => &options.interp,
            Self::Shape { options, .. } => &options.interp,
            Self::Pixels { interp, .. } => interp,
        }
    }

    pub fn to_instantaneous(&self, t: f64) -> InstantaneousSlideElement<W, H> {
        match self {
            TemporalSlideElement::Circle {
                center,
                radius,
                options,
            } => InstantaneousSlideElement::Shape {
                shape: ShapeSpec::Circle {
                    center: center.at_time(t).pixels(),
                    radius: radius.at_time(t).pixels(),
                },
                visuals: options.visuals.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: options.interp.interp_from_type,
                    interp_out_type: options.interp.interp_to_type,
                },
            },
            TemporalSlideElement::Line {
                start,
                end,
                radius,
                options,
            } => InstantaneousSlideElement::Shape {
                shape: ShapeSpec::Line {
                    point1: start.at_time(t).pixels(),
                    point2: end.at_time(t).pixels(),
                    radius: radius.at_time(t).pixels(),
                },
                visuals: options.visuals.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: options.interp.interp_from_type,
                    interp_out_type: options.interp.interp_to_type,
                },
            },
            TemporalSlideElement::Shape { shape, options } => InstantaneousSlideElement::Shape {
                shape: shape.at_time(t),
                visuals: options.visuals.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: options.interp.interp_from_type,
                    interp_out_type: options.interp.interp_to_type,
                },
            },
            TemporalSlideElement::Pixels {
                min,
                max,
                pixels,
                interp,
            } => InstantaneousSlideElement::Pixels {
                min: *min,
                max: *max,
                pixels: pixels.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: interp.interp_from_type,
                    interp_out_type: interp.interp_to_type,
                },
            },
        }
    }
}
