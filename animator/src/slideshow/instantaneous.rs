use crate::{
    colour::ColourWithAlpha,
    coords::Pos2,
    shape::ShapeSpec,
    slideshow::{MorphInterpType, ShapeInterpType, ShapeVisualOptions},
};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InOrOut {
    In,
    Out,
}

#[derive(Debug, Clone, Default)]
pub struct InstantaneousInterpOptions<InterpType> {
    pub interp_in_type: Option<InterpType>,
    pub interp_out_type: Option<InterpType>,
}

impl<InterpType> InstantaneousInterpOptions<InterpType> {
    pub(crate) fn select(self, in_or_out: InOrOut) -> Option<InterpType> {
        match in_or_out {
            InOrOut::In => self.interp_in_type,
            InOrOut::Out => self.interp_out_type,
        }
    }
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
        interp: InstantaneousInterpOptions<ShapeInterpType>,
    },
    Pixels {
        min: Pos2<W, H>,
        max: Pos2<W, H>,
        // screen pixel coords -> colour
        pixels: Arc<dyn Fn(Pos2<W, H>) -> ColourWithAlpha + Send + Sync>,
        interp: InstantaneousInterpOptions<MorphInterpType>,
    },
}
