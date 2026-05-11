use std::sync::Arc;

use crate::{
    colour::ColourWithAlpha,
    coords::Pos2,
    shape::ShapeSpec,
    slideshow::{InterpType, ShapeVisualOptions},
};

#[derive(Debug, Clone, Default)]
pub struct InstantaneousInterpOptions {
    pub interp_in_type: Option<InterpType>,
    pub interp_out_type: Option<InterpType>,
}

/*
A thing to be drawn on the slide without temporal continuity i.e. it only exist for a singular frame
This is in contrast to the temporal slide elements which exist through time
*/
#[derive(Clone)]
pub enum InstantaneousSlideElement<const W: u32, const H: u32> {
    Shape {
        shape: ShapeSpec, // in pixel coords
        visuals: ShapeVisualOptions,
        interp: InstantaneousInterpOptions,
    },
    Pixels {
        min: Pos2<W, H>,
        max: Pos2<W, H>,
        // screen pixel coords -> colour
        pixels: Arc<dyn Fn(Pos2<W, H>) -> ColourWithAlpha + Send + Sync>,
        interp: InstantaneousInterpOptions,
    },
}

impl<const W: u32, const H: u32> InstantaneousSlideElement<W, H> {
    pub fn interp_options(&self) -> &InstantaneousInterpOptions {
        match self {
            InstantaneousSlideElement::Shape { interp, .. } => interp,
            InstantaneousSlideElement::Pixels { interp, .. } => interp,
        }
    }
}
