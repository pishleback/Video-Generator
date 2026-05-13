use crate::{
    colour::ColourWithAlpha,
    coords::{Length, Pos2, Rect, Vec2},
    interpolation::Interpable,
    shape::ShapeSpec,
    slideshow::{
        canvas::{Canvas, CanvasInstantGroup},
        instantaneous::InstantaneousSlideElement,
        temporal::TemporalSlideElement,
        timeline::Timeline,
    },
};
use core::f64;

mod bounding_rect;
mod canvas;
mod instantaneous;
mod temporal;
mod timeline;
mod to_video;

use bounding_rect::BoundingRect;

pub enum Align {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MorphInterpType {
    Linear,
    #[default]
    Exp,
}

impl MorphInterpType {
    fn apply(&self, f: f64) -> f64 {
        if f <= 0.0 {
            0.0
        } else if f >= 1.0 {
            1.0
        } else {
            match self {
                MorphInterpType::Linear => f,
                MorphInterpType::Exp => 0.5 * (1.0 - (std::f64::consts::PI * f).cos()),
            }
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ShapeInterpType {
    Morph(MorphInterpType),
    #[default]
    Writing,
}

/*
An ID associated with temporal elements used to match up elements between slides
*/
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
struct InterpId {
    id: i64,
    // a list of indicies describing the path of the element in terms of its position in nested draw groups
    draw_order_idx: Vec<usize>,
}

/*
Options describing how shapes should look
*/
#[derive(Debug, Clone)]
pub struct ShapeVisualOptions {
    fill_rgba: ColourWithAlpha,
}
impl Default for ShapeVisualOptions {
    fn default() -> Self {
        Self {
            fill_rgba: ColourWithAlpha::from_linear(1.0, 1.0, 1.0, 1.0),
        }
    }
}
impl Interpable for ShapeVisualOptions {
    fn interp(x: &Self, y: &Self, f: f64) -> Self {
        Self {
            fill_rgba: ColourWithAlpha::interp(&x.fill_rgba, &y.fill_rgba, f),
        }
    }
}

/*
Timing options for a slide or an interpolation between slides
*/
#[derive(Debug)]
struct SlideshowStateTimingOptions {
    start_at: Option<f64>,
    end_at: Option<f64>,
    duration: Option<f64>,
}

struct SlideElements<const W: u32, const H: u32> {
    temporal: Vec<TemporalSlideElement<W, H>>,
    instantaneous: Timeline<Vec<InstantaneousSlideElement<W, H>>>,
}

impl<const W: u32, const H: u32> SlideElements<W, H> {
    fn new() -> Self {
        Self {
            temporal: vec![],
            instantaneous: Timeline::Constant(vec![]),
        }
    }

    fn append(&mut self, other: &mut Self) {
        self.temporal.append(&mut other.temporal);
        self.instantaneous = Timeline::from_fn({
            let self_instantaneous = self.instantaneous.clone();
            let other_instantaneous = other.instantaneous.clone();
            move |t| {
                let mut instantaneous = self_instantaneous.at_time(t);
                instantaneous.append(&mut other_instantaneous.at_time(t));
                instantaneous
            }
        })
    }
}

/*
A slide and its elements
*/
pub struct Slide<const W: u32, const H: u32> {
    timing: SlideshowStateTimingOptions,
    elements: SlideElements<W, H>,
}

impl<const W: u32, const H: u32> Slide<W, H> {
    fn new(elements: SlideElements<W, H>) -> Self {
        Self {
            timing: SlideshowStateTimingOptions {
                start_at: None,
                end_at: None,
                duration: None,
            },
            elements,
        }
    }

    pub fn start_at(&mut self, start_at: f64) -> &mut Self {
        self.timing.start_at = Some(start_at);
        self
    }

    pub fn end_at(&mut self, end_at: f64) -> &mut Self {
        self.timing.end_at = Some(end_at);
        self
    }

    pub fn duration(&mut self, duration: f64) -> &mut Self {
        self.timing.duration = Some(duration);
        self
    }
}

/*
The space between slides
*/
#[derive(Debug)]
pub struct SlideInterp {
    timing: SlideshowStateTimingOptions,
}
impl Default for SlideInterp {
    fn default() -> Self {
        Self {
            timing: SlideshowStateTimingOptions {
                start_at: None,
                end_at: None,
                duration: None,
            },
        }
    }
}
impl SlideInterp {
    pub fn start_at(&mut self, start_at: f64) -> &mut Self {
        self.timing.start_at = Some(start_at);
        self
    }

    pub fn end_at(&mut self, end_at: f64) -> &mut Self {
        self.timing.end_at = Some(end_at);
        self
    }

    pub fn duration(&mut self, duration: f64) -> &mut Self {
        self.timing.duration = Some(duration);
        self
    }
}

/*
A time interval of the slideshow. Either a slide or a gap between slides
*/
enum SlideshowState<const W: u32, const H: u32> {
    Slide(Slide<W, H>),
    Interp(SlideInterp),
}

impl<const W: u32, const H: u32> SlideshowState<W, H> {
    fn timing_options(&self) -> &SlideshowStateTimingOptions {
        match self {
            SlideshowState::Slide(slide) => &slide.timing,
            SlideshowState::Interp(slide_interp) => &slide_interp.timing,
        }
    }
}

/*
For building a slideshow animation object
*/
pub struct SlideshowBuilder<const W: u32, const H: u32> {
    background_colour: ColourWithAlpha,
    states: Vec<SlideshowState<W, H>>,
}

impl<const W: u32, const H: u32> SlideshowBuilder<W, H> {
    pub fn new(background_colour: ColourWithAlpha) -> Self {
        Self {
            background_colour,
            states: vec![],
        }
    }

    /// Add a slide
    pub fn slide(&mut self, build: impl FnOnce(&mut SlideRegionBuilder<W, H>)) -> &mut Slide<W, H> {
        let mut slide_builder = SlideRegionBuilder::new(Rect::fullscreen());
        build(&mut slide_builder);
        self.states
            .push(SlideshowState::Slide(Slide::new(slide_builder.finish())));
        match self.states.last_mut().unwrap() {
            SlideshowState::Slide(slide) => slide,
            _ => unreachable!(),
        }
    }

    /// Interpolation options between slides
    pub fn interp(&mut self) -> &mut SlideInterp {
        self.states
            .push(SlideshowState::Interp(SlideInterp::default()));
        match self.states.last_mut().unwrap() {
            SlideshowState::Interp(interp) => interp,
            _ => unreachable!(),
        }
    }
}

enum SlideRegionElement {
    Canvas(Canvas),
}

impl SlideRegionElement {
    fn finish<const W: u32, const H: u32>(self, rect: &Rect<W, H>) -> SlideElements<W, H> {
        match self {
            SlideRegionElement::Canvas(canvas) => canvas.finish(rect),
        }
    }
}

/*
For placing visual elements on a region of the slide
*/
pub struct SlideRegionBuilder<const W: u32, const H: u32> {
    rect: Rect<W, H>,
    subregions: Vec<SlideRegionBuilder<W, H>>,
    elements: Vec<SlideRegionElement>,
}

impl<const W: u32, const H: u32> SlideRegionBuilder<W, H> {
    fn new(rect: Rect<W, H>) -> Self {
        Self {
            rect,
            subregions: vec![],
            elements: vec![],
        }
    }

    pub fn left_half(&mut self) -> &mut Self {
        self.subregions
            .push(SlideRegionBuilder::new(self.rect.left_half()));
        self.subregions.last_mut().unwrap()
    }

    pub fn right_half(&mut self) -> &mut Self {
        self.subregions
            .push(SlideRegionBuilder::new(self.rect.right_half()));
        self.subregions.last_mut().unwrap()
    }

    pub fn top_half(&mut self) -> &mut Self {
        self.subregions
            .push(SlideRegionBuilder::new(self.rect.top_half()));
        self.subregions.last_mut().unwrap()
    }

    pub fn bottom_half(&mut self) -> &mut Self {
        self.subregions
            .push(SlideRegionBuilder::new(self.rect.bottom_half()));
        self.subregions.last_mut().unwrap()
    }

    pub fn title_space_split(&mut self) -> (&mut Self, &mut Self) {
        let (top, bottom) = self.rect.split_horizontal(0.1);
        let n = self.subregions.len();
        self.subregions.push(SlideRegionBuilder::new(top));
        self.subregions.push(SlideRegionBuilder::new(bottom));
        let top_and_bottom = self.subregions.split_at_mut(n).1.split_at_mut(1);
        (&mut top_and_bottom.0[0], &mut top_and_bottom.1[0])
    }

    pub fn canvas(
        &mut self,
        build: impl Fn(&mut CanvasInstantGroup, f64) + 'static,
    ) -> &mut Canvas {
        self.elements
            .push(SlideRegionElement::Canvas(Canvas::new(move |t| {
                let mut canvas_instant = CanvasInstantGroup::new();
                build(&mut canvas_instant, t);
                canvas_instant
            })));
        match self.elements.last_mut().unwrap() {
            SlideRegionElement::Canvas(element) => element,
            // _ => unreachable!(),
        }
    }

    fn finish(self) -> SlideElements<W, H> {
        let mut elements = SlideElements::new();
        for subregion in self.subregions {
            elements.append(&mut subregion.finish());
        }
        for element in self.elements {
            elements.append(&mut element.finish(&self.rect));
        }
        elements
    }
}

/*
A mapping from mathematical coordinates into slide coordinates
*/
#[derive(Clone)]
struct SlideEmbedding<const W: u32, const H: u32> {
    origin: (f64, f64),
    scale: Length<W, H>,
    position: Pos2<W, H>,
}

impl<const W: u32, const H: u32> SlideEmbedding<W, H> {
    fn map_point(&self, point: (f64, f64)) -> Pos2<W, H> {
        let offset_point = (point.0 - self.origin.0, point.1 - self.origin.1);
        self.position + Vec2::from_lengths(self.scale * offset_point.0, self.scale * offset_point.1)
    }

    fn map_length(&self, length: f64) -> Length<W, H> {
        length * self.scale
    }

    fn map_shape(&self, shape: ShapeSpec) -> ShapeSpec {
        shape
            .translate(-self.origin.0, -self.origin.1)
            .scale(self.scale.pixels())
            .translate(self.position.pixels().0, self.position.pixels().1)
    }

    fn unmap_point(&self, point: Pos2<W, H>) -> (f64, f64) {
        let offset_point = (point - self.position) / self.scale;
        (
            offset_point.0 + self.origin.0,
            offset_point.1 + self.origin.1,
        )
    }

    fn fit_within(rect: &Rect<W, H>, bounding_rect: &BoundingRect) -> Self {
        Self {
            origin: bounding_rect.center(),
            scale: if bounding_rect.width() * rect.height() < bounding_rect.height() * rect.width()
            {
                debug_assert_ne!(bounding_rect.height(), 0.0);
                rect.height() / bounding_rect.height()
            } else {
                debug_assert_ne!(bounding_rect.width(), 0.0);
                rect.width() / bounding_rect.width()
            },
            position: rect.center(),
        }
    }
}
