use crate::{
    colour::ColourWithAlpha,
    coords::{Length, Pos2},
    shape::ShapeSpec,
    slideshow::{
        ShapeInterpType, ShapeVisualOptions,
        instantaneous::{InstantaneousInterpOptions, InstantaneousSlideElement},
        timeline::Timeline,
    },
};
use std::sync::Arc;

use super::{InterpId, MorphInterpType};

/*
`interp_from_id` and `interp_to_id` are matched against objects on the adjacent slides to decide which objects should be interpolated
*/
#[derive(Debug, Clone, Default)]
pub struct TemporalInterpId {
    pub from: Option<InterpId>,
    pub to: Option<InterpId>,
}

/*
`interp_from_type` and `interp_to_type` define the type of interpolation to use
 - if both are None then a default interpolation is used
 - if exactly one is Some, then that interpolation is used
 - if both are Some they must match
*/
#[derive(Debug, Clone, Default)]
pub struct TemporalInterpType<InterpType> {
    pub from: Option<InterpType>,
    pub to: Option<InterpType>,
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
        visuals: Timeline<ShapeVisualOptions>,
        interp_type: TemporalInterpType<ShapeInterpType>,
        interp_id: TemporalInterpId,
    },
    Line {
        start: Timeline<Pos2<W, H>>,
        end: Timeline<Pos2<W, H>>,
        radius: Timeline<Length<W, H>>,
        visuals: Timeline<ShapeVisualOptions>,
        interp_type: TemporalInterpType<ShapeInterpType>,
        interp_id: TemporalInterpId,
    },
    Shape {
        shape: Timeline<ShapeSpec>, // in pixel coords
        visuals: Timeline<ShapeVisualOptions>,
        interp_type: TemporalInterpType<ShapeInterpType>,
        interp_id: TemporalInterpId,
    },
    Pixels {
        min: Pos2<W, H>,
        max: Pos2<W, H>,
        pixels: Timeline<Arc<dyn Fn(Pos2<W, H>) -> ColourWithAlpha + Send + Sync>>,
        interp_type: TemporalInterpType<MorphInterpType>,
        interp_id: TemporalInterpId,
    },
}

impl<const W: u32, const H: u32> TemporalSlideElement<W, H> {
    pub fn interp_id(&self) -> &TemporalInterpId {
        match self {
            TemporalSlideElement::Circle { interp_id, .. } => interp_id,
            TemporalSlideElement::Line { interp_id, .. } => interp_id,
            TemporalSlideElement::Shape { interp_id, .. } => interp_id,
            TemporalSlideElement::Pixels { interp_id, .. } => interp_id,
        }
    }

    pub fn to_instantaneous(&self, t: f64) -> InstantaneousSlideElement<W, H> {
        match self {
            TemporalSlideElement::Circle {
                center,
                radius,
                visuals,
                interp_type,
                ..
            } => InstantaneousSlideElement::Shape {
                shape: ShapeSpec::Circle {
                    center: center.at_time(t).pixels(),
                    radius: radius.at_time(t).pixels(),
                },
                visuals: visuals.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: interp_type.from,
                    interp_out_type: interp_type.to,
                },
            },
            TemporalSlideElement::Line {
                start,
                end,
                radius,
                visuals,
                interp_type,
                ..
            } => InstantaneousSlideElement::Shape {
                shape: ShapeSpec::Line {
                    point1: start.at_time(t).pixels(),
                    point2: end.at_time(t).pixels(),
                    radius: radius.at_time(t).pixels(),
                },
                visuals: visuals.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: interp_type.from,
                    interp_out_type: interp_type.to,
                },
            },
            TemporalSlideElement::Shape {
                shape,
                visuals,
                interp_type,
                ..
            } => InstantaneousSlideElement::Shape {
                shape: shape.at_time(t),
                visuals: visuals.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: interp_type.from,
                    interp_out_type: interp_type.to,
                },
            },
            TemporalSlideElement::Pixels {
                min,
                max,
                pixels,
                interp_type,
                ..
            } => InstantaneousSlideElement::Pixels {
                min: *min,
                max: *max,
                pixels: pixels.at_time(t),
                interp: InstantaneousInterpOptions {
                    interp_in_type: interp_type.from,
                    interp_out_type: interp_type.to,
                },
            },
        }
    }
}
