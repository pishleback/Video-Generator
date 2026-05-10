use crate::{
    colour::ColourWithAlpha,
    coords::{Length, Pos2, Rect, SCREEN_UNITS, Vec2},
    image::{ImageSpec, PixelsImage},
    interpolation::Interpable,
    shape::ShapeSpec,
    video::{VideoCompiledSpec, VideoSpec},
};
use core::f64;
use ordered_float::OrderedFloat;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, Clone)]
struct BoundingRect {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl BoundingRect {
    fn new(mut x1: f64, mut x2: f64, mut y1: f64, mut y2: f64) -> Self {
        if x2 < x1 {
            (x1, x2) = (x2, x1);
        }
        if y2 < y1 {
            (y1, y2) = (y2, y1);
        }
        Self {
            min_x: x1,
            max_x: x2,
            min_y: y1,
            max_y: y2,
        }
    }

    fn center(&self) -> (f64, f64) {
        (
            0.5 * (self.min_x + self.max_x),
            0.5 * (self.min_y + self.max_y),
        )
    }

    fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            min_x: *std::cmp::min(
                OrderedFloat::from(self.min_x),
                OrderedFloat::from(other.min_x),
            ),
            max_x: *std::cmp::max(
                OrderedFloat::from(self.max_x),
                OrderedFloat::from(other.max_x),
            ),
            min_y: *std::cmp::min(
                OrderedFloat::from(self.min_y),
                OrderedFloat::from(other.min_y),
            ),
            max_y: *std::cmp::max(
                OrderedFloat::from(self.max_y),
                OrderedFloat::from(other.max_y),
            ),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum InterpType {
    Linear,
    #[default]
    Exp,
}

impl InterpType {
    fn modify_interp(&self, f: f64) -> f64 {
        if f <= 0.0 {
            0.0
        } else if f >= 1.0 {
            1.0
        } else {
            match self {
                InterpType::Linear => f,
                InterpType::Exp => 0.5 * (1.0 - (std::f64::consts::PI * f).cos()),
            }
        }
    }
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
Options for how an object on a slide should be interpolated from the previous slide and to the next slide

`interp_from_id` and `interp_to_id` are matched against objects on the adjacent slides to decide which objects should be interpolated

`interp_from_type` and `interp_to_type` define the type of interpolation to use
 - if both are None then a default interpolation is used
 - if exactly one is Some, then that interpolation is used
 - if both are Some, then an interpolated interpolation is used
*/
#[derive(Debug, Clone, Default)]
struct TemporalInterpOptions {
    interp_from_id: Option<InterpId>,
    interp_to_id: Option<InterpId>,
    interp_from_type: Option<InterpType>,
    interp_to_type: Option<InterpType>,
}
impl TemporalInterpOptions {
    fn interp_from_id(&mut self, id: i64) -> &mut Self {
        self.interp_from_id = Some(InterpId {
            id,
            draw_order_idx: vec![],
        });
        self
    }
    fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.interp_to_id = Some(InterpId {
            id,
            draw_order_idx: vec![],
        });
        self
    }
    fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_from_id(id);
        self.interp_to_id(id);
        self
    }
    fn interp_from_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.interp_from_type = Some(interp_type);
        self
    }
    fn interp_to_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.interp_to_type = Some(interp_type);
        self
    }
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

#[derive(Clone)]
pub struct TemporalShapeOptions {
    interp: TemporalInterpOptions,
    visuals: Timeline<ShapeVisualOptions>,
}

impl TemporalShapeOptions {
    pub fn interp_from_id(&mut self, id: i64) -> &mut Self {
        self.interp.interp_from_id(id);
        self
    }

    pub fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.interp.interp_to_id(id);
        self
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp.interp_id(id);
        self
    }

    pub fn interp_from_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.interp.interp_from_type(interp_type);
        self
    }

    pub fn interp_to_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.interp.interp_to_type(interp_type);
        self
    }
}

#[derive(Clone)]
pub enum Timeline<V> {
    Constant(V),
    Function(Rc<dyn Fn(f64) -> V>),
}

impl<V> From<V> for Timeline<V> {
    fn from(v: V) -> Self {
        Timeline::Constant(v)
    }
}

impl<V> Timeline<V> {
    pub fn from_fn(f: impl Fn(f64) -> V + 'static) -> Self {
        Timeline::Function(Rc::new(f))
    }
}

impl<V: Clone> Timeline<V> {
    fn at_time(&self, t: f64) -> V {
        match self {
            Timeline::Constant(v) => v.clone(),
            Timeline::Function(g) => g(t),
        }
    }
}

/*
A thing to be drawn on the slide with temporal continuity
These are the elements it's possible to interpolate between slides
This is in contrast to the instantaneous slide elements
*/
#[derive(Clone)]
enum TemporalSlideElement<const W: u32, const H: u32> {
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
    fn interp_options(&self) -> &TemporalInterpOptions {
        match self {
            Self::Circle { options, .. } => &options.interp,
            Self::Line { options, .. } => &options.interp,
            Self::Shape { options, .. } => &options.interp,
            Self::Pixels { interp, .. } => interp,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct InstantaneousInterpOptions {
    interp_in_type: Option<InterpType>,
    interp_out_type: Option<InterpType>,
}

/*
A thing to be drawn on the slide without temporal continuity i.e. it only exist for a singular frame
This is in contrast to the temporal slide elements which exist through time
*/
#[derive(Clone)]
enum InstantaneousSlideElement<const W: u32, const H: u32> {
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
    fn interp_options(&self) -> &InstantaneousInterpOptions {
        match self {
            InstantaneousSlideElement::Shape { interp, .. } => interp,
            InstantaneousSlideElement::Pixels { interp, .. } => interp,
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

    /// Convert this slideshow into a video
    pub fn video(mut self, fps: f64) -> VideoSpec {
        // fiddle with self.states so that it
        // - has only slides at even indexes
        // - has only interps at odd intexes

        // ensure self.states is non-empty
        if self.states.is_empty() {
            return VideoSpec::Compiled(VideoCompiledSpec {
                width: W,
                height: H,
                fps,
                images: vec![],
                audio: vec![],
            });
        }

        // ensure self.states does not contain two adjacent interps
        for i in 0..(self.states.len() - 1) {
            let state_1 = &self.states[i];
            let state_2 = &self.states[i + 1];
            if let (SlideshowState::Interp(_), SlideshowState::Interp(_)) = (state_1, state_2) {
                panic!("Cannot have two adjacent interp states");
            }
        }

        // ensure self.states starts and ends with a slide
        match self.states.first().unwrap() {
            SlideshowState::Slide(_) => {}
            SlideshowState::Interp(_) => {
                panic!("Must start with a slide");
            }
        }
        match self.states.last().unwrap() {
            SlideshowState::Slide(_) => {}
            SlideshowState::Interp(_) => {
                panic!("Must end with a slide");
            }
        }

        // insert interp states between slides where one is missing
        let mut i = 0;
        while i < self.states.len() - 1 {
            match &self.states[i] {
                SlideshowState::Slide(_) => {}
                SlideshowState::Interp(_) => unreachable!(),
            }
            match &self.states[i + 1] {
                SlideshowState::Slide(_) => {
                    self.states
                        .insert(i + 1, SlideshowState::Interp(SlideInterp::default()));
                }
                SlideshowState::Interp(_) => {}
            }
            i += 2;
        }

        // check
        debug_assert!(self.states.len() % 2 == 1);
        for i in 0..self.states.len() {
            let state = &self.states[i];
            match (i % 2 == 0, state) {
                (true, SlideshowState::Slide(_)) | (false, SlideshowState::Interp(_)) => {}
                _ => {
                    unreachable!()
                }
            }
        }

        // resolve timings at the start and end and between slides
        // always start at t=0.0 so we record here the times at the end of each state
        let mut timings: Vec<OrderedFloat<f64>> = vec![];
        let mut t_acc = OrderedFloat(0.0);
        for i in 0..self.states.len() {
            let state_1 = &self.states[i].timing_options();
            let state_2 = self.states.get(i + 1).map(|s| s.timing_options());

            // the first state cannot have a start time
            if i == 0 && state_1.start_at.is_some() {
                panic!("The first state cannot have a start time");
            }

            // find the time between state_1 and state_2
            match (
                state_1.end_at,
                state_1.duration,
                state_2.and_then(|s| s.start_at),
            ) {
                (Some(t), None, None) | (None, None, Some(t)) => {
                    if t <= *t_acc {
                        panic!("Invalid timing; cannot go back in time");
                    }
                    t_acc = t.into();
                }
                (None, Some(dt), None) => {
                    if dt <= 0.0 {
                        panic!("Invalid timing; duration must be positive");
                    }
                    t_acc += dt;
                }
                (None, None, None) => {
                    // default last for 1 second
                    t_acc += 1.0;
                }
                _ => {
                    panic!("Timing overspecified");
                }
            }
            timings.push(t_acc);
        }
        timings.insert(0, 0.0.into());
        let n = self.states.len();
        debug_assert_eq!(n % 2, 1);
        debug_assert_eq!(timings.len(), n + 1);
        #[allow(clippy::manual_div_ceil)]
        let num_slides = (n + 1) / 2;

        // generate the animation objects spanning multiple frames if interp ids match
        #[derive(Clone)]
        struct MultiSlideElement<const W: u32, const H: u32> {
            current_to_id: Option<InterpId>,
            // one entry for each slide
            // None if the element is not present on the slide, otherwise Some
            elements: Vec<Option<TemporalSlideElement<W, H>>>,
        }
        let multislide_elements = {
            let mut finished_multislide_elements: Vec<MultiSlideElement<W, H>> = vec![];
            let mut active_multislide_elements: Vec<MultiSlideElement<W, H>> = vec![];
            for i in 0..num_slides {
                if let SlideshowState::Slide(slide) = &self.states[2 * i] {
                    let mut matching_pairs = vec![];
                    for multislide_element in &active_multislide_elements {
                        for element in &slide.elements.temporal {
                            let interp = element.interp_options();
                            if let (Some(from), Some(to)) =
                                (&interp.interp_from_id, &multislide_element.current_to_id)
                                && from == to
                            {
                                matching_pairs.push((multislide_element, element));
                            }
                        }
                    }

                    for multislide_element in &mut finished_multislide_elements {
                        multislide_element.elements.push(None);
                    }

                    let mut next_active_multislide_elements = vec![];

                    for element in &slide.elements.temporal {
                        let mut matches = vec![];
                        for (matched_multislide_element, matched_element) in &matching_pairs {
                            if std::ptr::eq(element, *matched_element) {
                                matches.push(matched_multislide_element);
                            }
                        }
                        match matches.len() {
                            0 => {
                                next_active_multislide_elements.push(MultiSlideElement {
                                    current_to_id: element.interp_options().interp_to_id.clone(),
                                    elements: vec![None; i]
                                        .into_iter()
                                        .chain(vec![Some(element.clone())])
                                        .collect(),
                                });
                            }
                            1 => {
                                // handled below when the multi slide element matches this element
                            }
                            _ => {
                                panic!("Too many interp id matches: {:?}", matches.len());
                            }
                        }
                    }

                    for multislide_element in &active_multislide_elements {
                        let mut matches = vec![];
                        for (matched_multislide_element, matched_element) in &matching_pairs {
                            if std::ptr::eq(multislide_element, *matched_multislide_element) {
                                matches.push(matched_element);
                            }
                        }
                        match matches.len() {
                            0 => {
                                finished_multislide_elements.push(MultiSlideElement {
                                    current_to_id: None,
                                    elements: multislide_element
                                        .elements
                                        .clone()
                                        .into_iter()
                                        .chain(vec![None])
                                        .collect(),
                                });
                            }
                            1 => {
                                next_active_multislide_elements.push(MultiSlideElement {
                                    current_to_id: matches[0].interp_options().interp_to_id.clone(),
                                    elements: multislide_element
                                        .elements
                                        .clone()
                                        .into_iter()
                                        .chain(vec![Some((*matches[0]).clone())])
                                        .collect(),
                                });
                            }
                            _ => {
                                panic!("Too many interp id matches: {:?}", matches.len());
                            }
                        }
                    }

                    active_multislide_elements = next_active_multislide_elements;
                } else {
                    unreachable!()
                }
            }

            finished_multislide_elements.append(&mut active_multislide_elements);
            let multislide_elements = finished_multislide_elements;
            for multislide_element in &multislide_elements {
                debug_assert_eq!(multislide_element.elements.len(), num_slides);
            }

            multislide_elements
        };

        let t_to_state_idx_and_time_and_frac =
            |t: OrderedFloat<f64>| -> (usize, f64, OrderedFloat<f64>) {
                let state_idx = match timings.binary_search(&t) {
                    Ok(i) => i,
                    Err(i) => i - 1,
                };
                let state_time = t - timings[state_idx];
                let state_frac =
                    (t - timings[state_idx]) / (timings[state_idx + 1] - timings[state_idx]);
                (state_idx, *state_time, state_frac)
            };

        enum ElementInstant<const W: u32, const H: u32> {
            Shape {
                shape: ShapeSpec,
                fill: ColourWithAlpha,
            },
            Pixels {
                min: (f64, f64),
                max: (f64, f64),
                pixels: Arc<dyn Fn(Pos2<W, H>) -> ColourWithAlpha + Send + Sync>,
            },
        }

        impl<const W: u32, const H: u32> MultiSlideElement<W, H> {
            fn get_at_instant(
                &self,
                t: &OrderedFloat<f64>,
                timings: &[OrderedFloat<f64>],
                (state_idx, _, state_frac): (usize, f64, OrderedFloat<f64>),
            ) -> Option<ElementInstant<W, H>> {
                if state_idx % 2 == 0 {
                    // on a slide
                    let slide_idx = state_idx / 2;
                    let slide_t = *(t - timings[slide_idx]);

                    self.elements[slide_idx]
                        .as_ref()
                        .map(|element| match element {
                            TemporalSlideElement::Circle {
                                center,
                                radius,
                                options,
                            } => ElementInstant::Shape {
                                shape: ShapeSpec::Circle {
                                    center: center.at_time(slide_t).pixels(),
                                    radius: radius.at_time(slide_t).pixels(),
                                },
                                fill: options.visuals.at_time(slide_t).fill_rgba,
                            },
                            TemporalSlideElement::Line {
                                start,
                                end,
                                radius,
                                options,
                            } => ElementInstant::Shape {
                                shape: ShapeSpec::Line {
                                    point1: start.at_time(slide_t).pixels(),
                                    point2: end.at_time(slide_t).pixels(),
                                    radius: radius.at_time(slide_t).pixels(),
                                },
                                fill: options.visuals.at_time(slide_t).fill_rgba,
                            },
                            TemporalSlideElement::Shape { shape, options } => {
                                ElementInstant::Shape {
                                    shape: shape.at_time(slide_t).clone(),
                                    fill: options.visuals.at_time(slide_t).fill_rgba,
                                }
                            }
                            TemporalSlideElement::Pixels {
                                min, max, pixels, ..
                            } => ElementInstant::Pixels {
                                min: min.pixels(),
                                max: max.pixels(),
                                pixels: pixels.at_time(slide_t),
                            },
                        })
                } else {
                    // on an interp
                    let from_slide_idx = (state_idx - 1) / 2;
                    #[allow(clippy::manual_div_ceil)]
                    let to_slide_idx = (state_idx + 1) / 2;

                    let to_slide_t = *(t - timings[to_slide_idx]);
                    let from_slide_t = *(t - timings[from_slide_idx]);

                    match (&self.elements[from_slide_idx], &self.elements[to_slide_idx]) {
                        (None, None) => None,
                        (None, Some(to_element)) => {
                            let interp_frac = to_element
                                .interp_options()
                                .interp_from_type
                                .unwrap_or_default()
                                .modify_interp(*state_frac);

                            Some(match to_element {
                                TemporalSlideElement::Circle {
                                    center,
                                    radius,
                                    options,
                                } => ElementInstant::Shape {
                                    shape: ShapeSpec::Circle {
                                        center: center.at_time(to_slide_t).pixels(),
                                        radius: radius.at_time(to_slide_t).pixels(),
                                    },
                                    fill: options
                                        .visuals
                                        .at_time(to_slide_t)
                                        .fill_rgba
                                        .mul_alpha(interp_frac as f32),
                                },
                                TemporalSlideElement::Line {
                                    start,
                                    end,
                                    radius,
                                    options,
                                } => ElementInstant::Shape {
                                    shape: ShapeSpec::Line {
                                        point1: start.at_time(to_slide_t).pixels(),
                                        point2: end.at_time(to_slide_t).pixels(),
                                        radius: radius.at_time(to_slide_t).pixels(),
                                    },
                                    fill: options
                                        .visuals
                                        .at_time(to_slide_t)
                                        .fill_rgba
                                        .mul_alpha(interp_frac as f32),
                                },
                                TemporalSlideElement::Shape { shape, options } => {
                                    ElementInstant::Shape {
                                        shape: shape.at_time(to_slide_t).clone(),
                                        fill: options
                                            .visuals
                                            .at_time(to_slide_t)
                                            .fill_rgba
                                            .mul_alpha(interp_frac as f32),
                                    }
                                }
                                TemporalSlideElement::Pixels {
                                    min, max, pixels, ..
                                } => ElementInstant::Pixels {
                                    min: min.pixels(),
                                    max: max.pixels(),
                                    pixels: {
                                        let pixels_t = pixels.at_time(to_slide_t);
                                        Arc::new(move |p| pixels_t(p).mul_alpha(interp_frac as f32))
                                    },
                                },
                            })
                        }
                        (Some(from_element), None) => {
                            let interp_frac = from_element
                                .interp_options()
                                .interp_to_type
                                .unwrap_or_default()
                                .modify_interp(*state_frac);

                            Some(match from_element {
                                TemporalSlideElement::Circle {
                                    center,
                                    radius,
                                    options,
                                } => ElementInstant::Shape {
                                    shape: ShapeSpec::Circle {
                                        center: center.at_time(from_slide_t).pixels(),
                                        radius: radius.at_time(from_slide_t).pixels(),
                                    },
                                    fill: options
                                        .visuals
                                        .at_time(from_slide_t)
                                        .fill_rgba
                                        .mul_alpha(1.0 - interp_frac as f32),
                                },
                                TemporalSlideElement::Line {
                                    start,
                                    end,
                                    radius,
                                    options,
                                } => ElementInstant::Shape {
                                    shape: ShapeSpec::Line {
                                        point1: start.at_time(from_slide_t).pixels(),
                                        point2: end.at_time(from_slide_t).pixels(),
                                        radius: radius.at_time(from_slide_t).pixels(),
                                    },
                                    fill: options
                                        .visuals
                                        .at_time(from_slide_t)
                                        .fill_rgba
                                        .mul_alpha(1.0 - interp_frac as f32),
                                },
                                TemporalSlideElement::Shape { shape, options } => {
                                    ElementInstant::Shape {
                                        shape: shape.at_time(from_slide_t).clone(),
                                        fill: options
                                            .visuals
                                            .at_time(from_slide_t)
                                            .fill_rgba
                                            .mul_alpha(1.0 - interp_frac as f32),
                                    }
                                }
                                TemporalSlideElement::Pixels {
                                    min, max, pixels, ..
                                } => ElementInstant::Pixels {
                                    min: min.pixels(),
                                    max: max.pixels(),
                                    pixels: {
                                        let pixels_t = pixels.at_time(from_slide_t);
                                        Arc::new(move |p| {
                                            pixels_t(p).mul_alpha(1.0 - interp_frac as f32)
                                        })
                                    },
                                },
                            })
                        }
                        (Some(from_element), Some(to_element)) => {
                            let interp_frac = match (
                                from_element.interp_options().interp_to_type,
                                to_element.interp_options().interp_from_type,
                            ) {
                                (None, None) => InterpType::default().modify_interp(*state_frac),
                                (None, Some(j)) => j.modify_interp(*state_frac),
                                (Some(i), None) => i.modify_interp(*state_frac),
                                (Some(i), Some(j)) => f64::interp(
                                    &i.modify_interp(*state_frac),
                                    &j.modify_interp(*state_frac),
                                    *state_frac,
                                ),
                            };

                            Some(match (&from_element, &to_element) {
                                (
                                    TemporalSlideElement::Circle {
                                        center: from_center,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    TemporalSlideElement::Circle {
                                        center: to_center,
                                        radius: to_radius,
                                        options: to_options,
                                    },
                                ) => {
                                    let visuals = ShapeVisualOptions::interp(
                                        &from_options.visuals.at_time(from_slide_t),
                                        &to_options.visuals.at_time(to_slide_t),
                                        interp_frac,
                                    );
                                    ElementInstant::Shape {
                                        shape: ShapeSpec::Circle {
                                            center: Pos2::interp(
                                                &from_center.at_time(from_slide_t),
                                                &to_center.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(from_slide_t),
                                                &to_radius.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    TemporalSlideElement::Circle {
                                        center: from_center,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    TemporalSlideElement::Line {
                                        start: to_start,
                                        end: to_end,
                                        radius: to_radius,
                                        options: to_options,
                                    },
                                ) => {
                                    let visuals = ShapeVisualOptions::interp(
                                        &from_options.visuals.at_time(from_slide_t),
                                        &to_options.visuals.at_time(to_slide_t),
                                        interp_frac,
                                    );
                                    ElementInstant::Shape {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                &from_center.at_time(from_slide_t),
                                                &to_start.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(
                                                &from_center.at_time(from_slide_t),
                                                &to_end.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(from_slide_t),
                                                &to_radius.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    TemporalSlideElement::Line {
                                        start: from_start,
                                        end: from_end,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    TemporalSlideElement::Circle {
                                        center: to_center,
                                        radius: to_radius,
                                        options: to_options,
                                    },
                                ) => {
                                    let visuals = ShapeVisualOptions::interp(
                                        &from_options.visuals.at_time(from_slide_t),
                                        &to_options.visuals.at_time(to_slide_t),
                                        interp_frac,
                                    );
                                    ElementInstant::Shape {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                &from_start.at_time(from_slide_t),
                                                &to_center.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(
                                                &from_end.at_time(from_slide_t),
                                                &to_center.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(from_slide_t),
                                                &to_radius.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    TemporalSlideElement::Line {
                                        start: from_start,
                                        end: from_end,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    TemporalSlideElement::Line {
                                        start: to_start,
                                        end: to_end,
                                        radius: to_radius,
                                        options: to_options,
                                    },
                                ) => {
                                    let visuals = ShapeVisualOptions::interp(
                                        &from_options.visuals.at_time(from_slide_t),
                                        &to_options.visuals.at_time(to_slide_t),
                                        interp_frac,
                                    );
                                    ElementInstant::Shape {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                &from_start.at_time(from_slide_t),
                                                &to_start.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(
                                                &from_end.at_time(from_slide_t),
                                                &to_end.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(from_slide_t),
                                                &to_radius.at_time(to_slide_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    TemporalSlideElement::Pixels {
                                        min: from_min,
                                        max: from_max,
                                        pixels: from_pixels,
                                        ..
                                    },
                                    TemporalSlideElement::Pixels {
                                        min: to_min,
                                        max: to_max,
                                        pixels: to_pixels,
                                        ..
                                    },
                                ) => ElementInstant::Pixels {
                                    min: Pos2::interp(from_min, to_min, interp_frac).pixels(),
                                    max: Pos2::interp(from_max, to_max, interp_frac).pixels(),
                                    pixels: {
                                        let from_pixels_t = from_pixels.at_time(from_slide_t);
                                        let to_pixels_t = to_pixels.at_time(to_slide_t);
                                        Arc::new(move |p| {
                                            ColourWithAlpha::interp(
                                                &from_pixels_t(p),
                                                &to_pixels_t(p),
                                                interp_frac,
                                            )
                                        })
                                    },
                                },
                                _ => {
                                    unimplemented!(
                                        "Interpolation not implemented for these element types"
                                    );
                                }
                            })
                        }
                    }
                }
            }
        }

        // generate the video
        let image_at_time = |t: OrderedFloat<f64>| -> ImageSpec {
            let state_pos = t_to_state_idx_and_time_and_frac(t);

            struct Layer {
                top_left: (i64, i64),
                image: ImageSpec,
            }

            let mut layers = vec![];

            let mut instant_elements = multislide_elements
                .iter()
                .filter_map(|multislide_element| {
                    multislide_element.get_at_instant(&t, &timings, state_pos)
                })
                .chain({
                    let mut elements = vec![];
                    let (state_idx, _, state_frac) = state_pos;

                    if state_idx % 2 == 0 {
                        // on a slide
                        let slide_t = *(t - timings[state_idx]);
                        match &self.states[state_idx] {
                            SlideshowState::Slide(slide) => {
                                for element in slide.elements.instantaneous.at_time(slide_t) {
                                    match element {
                                        InstantaneousSlideElement::Shape {
                                            shape, visuals, ..
                                        } => {
                                            elements.push(ElementInstant::Shape {
                                                shape,
                                                fill: visuals.fill_rgba,
                                            });
                                        }
                                        InstantaneousSlideElement::Pixels {
                                            min,
                                            max,
                                            pixels,
                                            ..
                                        } => {
                                            elements.push(ElementInstant::Pixels {
                                                min: min.pixels(),
                                                max: max.pixels(),
                                                pixels,
                                            });
                                        }
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        // on an interp
                        let from_slide_idx = state_idx - 1;
                        #[allow(clippy::manual_div_ceil)]
                        let to_slide_idx = state_idx + 1;

                        let to_slide_t = *(t - timings[to_slide_idx]);
                        let from_slide_t = *(t - timings[from_slide_idx]);

                        match &self.states[to_slide_idx] {
                            SlideshowState::Slide(to_slide) => {
                                for element in to_slide.elements.instantaneous.at_time(to_slide_t) {
                                    let interp_frac = element
                                        .interp_options()
                                        .interp_out_type
                                        .unwrap_or_default()
                                        .modify_interp(*state_frac);
                                    match element {
                                        InstantaneousSlideElement::Shape {
                                            shape, visuals, ..
                                        } => {
                                            elements.push(ElementInstant::Shape {
                                                shape,
                                                fill: visuals
                                                    .fill_rgba
                                                    .mul_alpha(interp_frac as f32),
                                            });
                                        }
                                        InstantaneousSlideElement::Pixels {
                                            min,
                                            max,
                                            pixels,
                                            ..
                                        } => {
                                            elements.push(ElementInstant::Pixels {
                                                min: min.pixels(),
                                                max: max.pixels(),
                                                pixels: Arc::new(move |p| {
                                                    pixels(p).mul_alpha(interp_frac as f32)
                                                }),
                                            });
                                        }
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }

                        match &self.states[from_slide_idx] {
                            SlideshowState::Slide(from_slide) => {
                                for element in
                                    from_slide.elements.instantaneous.at_time(from_slide_t)
                                {
                                    let interp_frac = element
                                        .interp_options()
                                        .interp_in_type
                                        .unwrap_or_default()
                                        .modify_interp(*state_frac);
                                    match element {
                                        InstantaneousSlideElement::Shape {
                                            shape, visuals, ..
                                        } => {
                                            elements.push(ElementInstant::Shape {
                                                shape,
                                                fill: visuals
                                                    .fill_rgba
                                                    .mul_alpha(1.0 - interp_frac as f32),
                                            });
                                        }
                                        InstantaneousSlideElement::Pixels {
                                            min,
                                            max,
                                            pixels,
                                            ..
                                        } => {
                                            elements.push(ElementInstant::Pixels {
                                                min: min.pixels(),
                                                max: max.pixels(),
                                                pixels: Arc::new(move |p| {
                                                    pixels(p).mul_alpha(1.0 - interp_frac as f32)
                                                }),
                                            });
                                        }
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                    elements
                })
                .collect::<Vec<_>>();

            // TODO: custom options for how to sort
            // for now, sort by relative luminance
            instant_elements.sort_by_cached_key(|instant_element| {
                OrderedFloat(match instant_element {
                    ElementInstant::Shape { fill, .. } => fill.to_relative_luminance(),
                    ElementInstant::Pixels { .. } => -1.0,
                })
            });

            for instant_element in instant_elements {
                match instant_element {
                    ElementInstant::Shape { shape, fill } => {
                        if let Some(br) = shape.shape().bounding_rect() {
                            let min = br.min().x_y();
                            let min = (min.0.floor() as i64, min.1.floor() as i64);
                            let max = br.max().x_y();
                            let max = (max.0.ceil() as i64, max.1.ceil() as i64);
                            let width = (max.0 - min.0) as u32;
                            let height = (max.1 - min.1) as u32;
                            layers.push(Layer {
                                top_left: (min.0, min.1),
                                image: shape.translate(-(min.0 as f64), -(min.1 as f64)).image(
                                    width,
                                    height,
                                    fill.mul_alpha(0.0),
                                    fill,
                                ),
                            });
                        }
                    }
                    ElementInstant::Pixels { min, max, pixels } => {
                        let min_rounded = (min.0.round() as i64, min.1.round() as i64);
                        let max_rounded = (max.0.round() as i64, max.1.round() as i64);
                        layers.push(Layer {
                            top_left: min_rounded,
                            image: ImageSpec::Pixels(PixelsImage::new(
                                (max_rounded.0 - min_rounded.0) as u32,
                                (max_rounded.1 - min_rounded.1) as u32,
                                move |x, y| {
                                    pixels(Pos2::from_pixels(
                                        x as f64 + min_rounded.0 as f64,
                                        y as f64 + min_rounded.1 as f64,
                                    ))
                                },
                            )),
                        });
                    }
                }
            }

            ImageSpec::BlitStack {
                base: Box::new(ImageSpec::Filled {
                    width: W,
                    height: H,
                    colour: self.background_colour,
                }),
                layers: layers
                    .into_iter()
                    .map(|layer| (layer.top_left, layer.image))
                    .collect(),
            }
        };

        let mut images = vec![];
        let dt = 1.0 / fps;
        let mut t = OrderedFloat(0.0);
        while t < *timings.last().unwrap() {
            images.push(image_at_time(t));
            t += dt;
        }

        VideoSpec::Compiled(VideoCompiledSpec {
            width: W,
            height: H,
            fps,
            images,
            audio: vec![],
        })
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

    pub fn title_space(&mut self) -> &mut Self {
        self.subregions.push(SlideRegionBuilder::new(
            self.rect
                .split_horizontal(0.1)
                .0
                .pad(Length::from_units(0.01 * SCREEN_UNITS)),
        ));
        self.subregions.last_mut().unwrap()
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

enum CanvasElementOrGroup {
    Element(CanvasElement),
    Group(CanvasInstantGroup),
}
impl CanvasElementOrGroup {
    fn flatten(self) -> Vec<CanvasElementWithInterpId> {
        match self {
            CanvasElementOrGroup::Element(element) => {
                let interp_id = match &element {
                    CanvasElement::Circle(circle) => circle.interp_id,
                    CanvasElement::Line(line) => line.interp_id,
                    CanvasElement::Shape(shape) => shape.interp_id,
                    CanvasElement::Pixels(pixels) => pixels.interp_id,
                }
                .map(|id| InterpId {
                    id,
                    draw_order_idx: vec![],
                });
                vec![CanvasElementWithInterpId { element, interp_id }]
            }
            CanvasElementOrGroup::Group(group) => {
                let group_interp_id = group.interp_id;
                group
                    .flatten()
                    .into_iter()
                    .enumerate()
                    .map({
                        move |(idx, mut element)| {
                            // elements inherit their interp ID from the group they are in
                            if let Some(group_interp_id) = group_interp_id {
                                if let Some(element_interp_id) = element.interp_id {
                                    panic!(
                                        "Multiple interp ID definitions: `{:?}` and `{:?}`",
                                        element_interp_id.id, group_interp_id
                                    );
                                }
                                element.interp_id = Some(InterpId {
                                    id: group_interp_id,
                                    draw_order_idx: vec![],
                                });
                            }
                            // set the interp id draw idx according to the position in the group
                            if let Some(interp_id) = element.interp_id.as_mut() {
                                interp_id.draw_order_idx.insert(0, idx);
                            }
                            element
                        }
                    })
                    .collect()
            }
        }
    }
}

#[derive(Clone)]
enum CanvasElement {
    Circle(CanvasCircle),
    Line(CanvasLine),
    Shape(CanvasShape),
    Pixels(CanvasPixels),
}

impl CanvasElement {
    fn bounding_rect(&self) -> Option<BoundingRect> {
        match self {
            CanvasElement::Circle(circle) => circle.bounding_rect(),
            CanvasElement::Line(line) => line.bounding_rect(),
            CanvasElement::Shape(shape) => shape.bounding_rect(),
            CanvasElement::Pixels(_) => None,
        }
    }
}

#[derive(Clone)]
pub struct CanvasCircle {
    center: (f64, f64),
    radius: f64,
    visuals: ShapeVisualOptions,
    interp_id: Option<i64>,
}

impl CanvasCircle {
    fn bounding_rect(&self) -> Option<BoundingRect> {
        Some(BoundingRect::new(
            self.center.0 - self.radius,
            self.center.0 + self.radius,
            self.center.1 - self.radius,
            self.center.1 + self.radius,
        ))
    }

    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> InstantaneousSlideElement<W, H> {
        InstantaneousSlideElement::Shape {
            shape: ShapeSpec::Circle {
                center: embedding.map_point(self.center).pixels(),
                radius: embedding.map_length(self.radius).pixels(),
            },
            visuals: self.visuals,
            interp: InstantaneousInterpOptions {
                interp_in_type: None,
                interp_out_type: None,
            },
        }
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_id = Some(id);
        self
    }

    pub fn fill_rgba(&mut self, fill_rgba: ColourWithAlpha) -> &mut Self {
        self.visuals.fill_rgba = fill_rgba;
        self
    }
}

#[derive(Clone)]
pub struct CanvasLine {
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
    visuals: ShapeVisualOptions,
    interp_id: Option<i64>,
}

impl CanvasLine {
    fn bounding_rect(&self) -> Option<BoundingRect> {
        Some(BoundingRect::new(
            self.start.0.min(self.end.0) - self.radius,
            self.start.0.max(self.end.0) + self.radius,
            self.start.1.min(self.end.1) - self.radius,
            self.start.1.max(self.end.1) + self.radius,
        ))
    }

    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> InstantaneousSlideElement<W, H> {
        InstantaneousSlideElement::Shape {
            shape: ShapeSpec::Line {
                point1: embedding.map_point(self.start).pixels(),
                point2: embedding.map_point(self.end).pixels(),
                radius: embedding.map_length(self.radius).pixels(),
            },
            visuals: self.visuals,
            interp: InstantaneousInterpOptions {
                interp_in_type: None,
                interp_out_type: None,
            },
        }
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_id = Some(id);
        self
    }

    pub fn fill_rgba(&mut self, fill_rgba: ColourWithAlpha) -> &mut Self {
        self.visuals.fill_rgba = fill_rgba;
        self
    }
}

#[derive(Clone)]
pub struct CanvasShape {
    shape: ShapeSpec,
    origin: (f64, f64),
    position: (f64, f64),
    visuals: ShapeVisualOptions,
    interp_id: Option<i64>,
}

pub enum AlignOptions {
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

impl CanvasShape {
    fn untranslated_bounding_rect(&self) -> Option<BoundingRect> {
        self.shape.shape().bounding_rect().map(|br| {
            let min = br.min().x_y();
            let max = br.max().x_y();
            BoundingRect::new(min.0, max.0, min.1, max.1)
        })
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        self.shape.shape().bounding_rect().map(|br| {
            let min = br.min().x_y();
            let max = br.max().x_y();
            BoundingRect::new(
                min.0 + self.position.0 - self.origin.0,
                max.0 + self.position.0 - self.origin.0,
                min.1 + self.position.1 - self.origin.1,
                max.1 + self.position.1 - self.origin.1,
            )
        })
    }

    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> InstantaneousSlideElement<W, H> {
        InstantaneousSlideElement::Shape {
            shape: embedding.map_shape(self.shape.translate(
                self.position.0 - self.origin.0,
                self.position.1 - self.origin.1,
            )),
            visuals: self.visuals,
            interp: InstantaneousInterpOptions {
                interp_in_type: None,
                interp_out_type: None,
            },
        }
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_id = Some(id);
        self
    }

    pub fn fill_rgba(&mut self, fill_rgba: ColourWithAlpha) -> &mut Self {
        self.visuals.fill_rgba = fill_rgba;
        self
    }

    pub fn position(&mut self, position: (f64, f64)) -> &mut Self {
        self.position = position;
        self
    }

    pub fn align(&mut self, align: AlignOptions) -> &mut Self {
        if let Some(br) = self.untranslated_bounding_rect() {
            let align = match align {
                AlignOptions::TopLeft => (0.0, 0.0),
                AlignOptions::TopCenter => (0.5, 0.0),
                AlignOptions::TopRight => (1.0, 0.0),
                AlignOptions::CenterLeft => (0.0, 0.5),
                AlignOptions::Center => (0.5, 0.5),
                AlignOptions::CenterRight => (1.0, 0.5),
                AlignOptions::BottomLeft => (0.0, 1.0),
                AlignOptions::BottomCenter => (0.5, 1.0),
                AlignOptions::BottomRight => (1.0, 1.0),
            };
            self.origin = (
                br.min_x + align.0 * br.width(),
                br.min_y + align.1 * br.height(),
            );
        }
        self
    }

    pub fn width(&mut self, width: f64) -> &mut Self {
        if let Some(br) = self.untranslated_bounding_rect() {
            let origin = self.origin;
            self.shape = self
                .shape
                .translate(-origin.0, -origin.1)
                .scale(width / br.width())
                .translate(origin.0, origin.1);
        }
        self
    }

    pub fn height(&mut self, height: f64) -> &mut Self {
        if let Some(br) = self.untranslated_bounding_rect() {
            let origin = self.origin;
            self.shape = self
                .shape
                .translate(-origin.0, -origin.1)
                .scale(height / br.height())
                .translate(origin.0, origin.1);
        }
        self
    }
}

#[derive(Clone)]
pub struct CanvasPixels {
    pixels: Arc<dyn Fn(f64, f64) -> ColourWithAlpha + Send + Sync>,
    interp_id: Option<i64>,
}

impl CanvasPixels {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
        bounding_rect: &BoundingRect,
    ) -> InstantaneousSlideElement<W, H> {
        InstantaneousSlideElement::Pixels {
            min: embedding.map_point((bounding_rect.min_x, bounding_rect.min_y)),
            max: embedding.map_point((bounding_rect.max_x, bounding_rect.max_y)),
            pixels: Arc::new({
                let pixels = self.pixels.clone();
                let embedding = embedding.clone();
                move |pt| {
                    let (x, y) = embedding.unmap_point(pt);
                    pixels(x, y)
                }
            }),
            interp: InstantaneousInterpOptions {
                interp_in_type: None,
                interp_out_type: None,
            },
        }
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_id = Some(id);
        self
    }
}

#[derive(Clone)]
struct CanvasElementWithInterpId {
    element: CanvasElement,
    interp_id: Option<InterpId>,
}

pub struct CanvasInstantGroup {
    interp_id: Option<i64>,
    elements: Vec<CanvasElementOrGroup>,
}

impl CanvasInstantGroup {
    fn new() -> Self {
        Self {
            elements: vec![],
            interp_id: None,
        }
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_id = Some(id);
        self
    }

    pub fn group(&mut self) -> &mut CanvasInstantGroup {
        self.elements
            .push(CanvasElementOrGroup::Group(CanvasInstantGroup::new()));
        match self.elements.last_mut().unwrap() {
            CanvasElementOrGroup::Group(group) => group,
            _ => {
                unreachable!()
            }
        }
    }

    pub fn circle(&mut self, center: (f64, f64), radius: f64) -> &mut CanvasCircle {
        self.elements
            .push(CanvasElementOrGroup::Element(CanvasElement::Circle(
                CanvasCircle {
                    center,
                    radius,
                    visuals: ShapeVisualOptions::default(),
                    interp_id: None,
                },
            )));
        if let CanvasElementOrGroup::Element(element) = self.elements.last_mut().unwrap()
            && let CanvasElement::Circle(element) = element
        {
            element
        } else {
            unreachable!()
        }
    }

    pub fn line(&mut self, start: (f64, f64), end: (f64, f64), radius: f64) -> &mut CanvasLine {
        self.elements
            .push(CanvasElementOrGroup::Element(CanvasElement::Line(
                CanvasLine {
                    start,
                    end,
                    radius,
                    visuals: ShapeVisualOptions::default(),
                    interp_id: None,
                },
            )));
        if let CanvasElementOrGroup::Element(element) = self.elements.last_mut().unwrap()
            && let CanvasElement::Line(element) = element
        {
            element
        } else {
            unreachable!()
        }
    }

    pub fn latex(&mut self, expr: impl Into<String>) -> &mut CanvasShape {
        self.elements
            .push(CanvasElementOrGroup::Element(CanvasElement::Shape(
                CanvasShape {
                    shape: ShapeSpec::latex(expr.into()),
                    position: (0.0, 0.0),
                    origin: (0.0, 0.0),
                    visuals: ShapeVisualOptions::default(),
                    interp_id: None,
                },
            )));
        if let CanvasElementOrGroup::Element(element) = self.elements.last_mut().unwrap()
            && let CanvasElement::Shape(element) = element
        {
            element
        } else {
            unreachable!()
        }
    }

    pub fn text(&mut self, expr: impl Into<String>) -> &mut CanvasShape {
        self.latex(format!(r#"\text{{{}}}"#, expr.into()))
    }

    pub fn pixels(
        &mut self,
        pixels: impl Fn(f64, f64) -> ColourWithAlpha + Send + Sync + 'static,
    ) -> &mut CanvasPixels {
        self.elements
            .push(CanvasElementOrGroup::Element(CanvasElement::Pixels(
                CanvasPixels {
                    pixels: Arc::new(pixels),
                    interp_id: None,
                },
            )));
        if let CanvasElementOrGroup::Element(element) = self.elements.last_mut().unwrap()
            && let CanvasElement::Pixels(element) = element
        {
            element
        } else {
            unreachable!()
        }
    }

    fn flatten(self) -> Vec<CanvasElementWithInterpId> {
        let mut elements = vec![];
        for element in self.elements {
            elements.append(&mut element.flatten());
        }
        elements
    }
}

#[derive(Default)]
enum CanvasBoundingRectMethod {
    #[default]
    Unset,
    // A fixed bounding box
    Fixed(BoundingRect),
    // Times at which to sample the shapes and take their bounding box
    ShapeSamples(Vec<f64>),
}

pub struct Canvas {
    build_instant: Rc<dyn Fn(f64) -> CanvasInstantGroup>,
    bounding_rect_method: CanvasBoundingRectMethod,
}

impl Canvas {
    fn new(build_instant: impl Fn(f64) -> CanvasInstantGroup + 'static) -> Self {
        Self {
            build_instant: Rc::new(build_instant),
            bounding_rect_method: CanvasBoundingRectMethod::default(),
        }
    }

    fn get_bounding_rect_at(&self, t: f64) -> Option<BoundingRect> {
        (self.build_instant)(t)
            .flatten()
            .into_iter()
            .filter_map(|elem| elem.element.bounding_rect())
            .reduce(|a, b| BoundingRect::union(&a, &b))
    }

    fn get_bounding_rect(&self) -> Option<BoundingRect> {
        match &self.bounding_rect_method {
            // sample between 6 and 12 seconds at 0.1 second intervals as reasonable guess at good default behavour
            CanvasBoundingRectMethod::Unset => (60..120)
                .map(|i| (i as f64) / 10.0)
                .filter_map(|t| self.get_bounding_rect_at(t))
                .reduce(|a, b| BoundingRect::union(&a, &b)),
            CanvasBoundingRectMethod::Fixed(br) => Some(br.clone()),
            CanvasBoundingRectMethod::ShapeSamples(times) => times
                .iter()
                .filter_map(|t| self.get_bounding_rect_at(*t))
                .reduce(|a, b| BoundingRect::union(&a, &b)),
        }
    }

    pub fn fixed_bounding_rect(&mut self, x1: f64, x2: f64, y1: f64, y2: f64) -> &mut Self {
        self.bounding_rect_method =
            CanvasBoundingRectMethod::Fixed(BoundingRect::new(x1, x2, y1, y2));
        self
    }

    pub fn sampled_bounding_rect(&mut self, times: impl Into<Vec<f64>>) -> &mut Self {
        self.bounding_rect_method = CanvasBoundingRectMethod::ShapeSamples(times.into());
        self
    }

    fn finish<const W: u32, const H: u32>(self, rect: &Rect<W, H>) -> SlideElements<W, H> {
        if let Some(bounding_rect) = self.get_bounding_rect() {
            let slide_embedding = SlideEmbedding::fit_within(rect, &bounding_rect);
            SlideElements {
                temporal: {
                    // list of all the temporal elements at a given time by ID
                    // these lists at different times are required to be compatible, meaning
                    //  - they have the same length
                    //  - elements at the same key have the same type
                    // this allows us to glue them toegher through time to construct a temporal element
                    let at_t = Rc::new({
                        let build_instant = self.build_instant.clone();
                        move |t: f64| {
                            let mut elements_by_id = HashMap::new();
                            for (id, elem) in
                                (build_instant)(t)
                                    .flatten()
                                    .into_iter()
                                    .filter_map(|element| {
                                        element
                                            .interp_id
                                            .clone()
                                            .map(|interp_id| (interp_id, element))
                                    })
                            {
                                if elements_by_id.insert(id.clone(), elem).is_some() {
                                    panic!("Interp ID `{:?}` used by multiple elements", id);
                                }
                            }
                            elements_by_id
                        }
                    });

                    // use t=0 as the somewhat arbitrary template for the thing all other times should match
                    let zero_instant = at_t(0.0);

                    let keys = zero_instant.keys().cloned().collect::<Vec<_>>();
                    let at_t_check_matches = Rc::new(move |t: f64| {
                        let at_t = at_t(t);
                        if at_t.keys().collect::<HashSet<_>>()
                            != keys.iter().collect::<HashSet<_>>()
                        {
                            panic!("Different temporal elements returned at different times");
                        }
                        at_t
                    });

                    zero_instant
                        .into_iter()
                        .map(|(id, elem)| match elem.element {
                            CanvasElement::Circle(_) => {
                                let get_circle_t = Rc::new({
                                    let id = id.clone();
                                    let at_t = at_t_check_matches.clone();
                                    move |t| match at_t(t).get(&id).unwrap().element.clone() {
                                        CanvasElement::Circle(circle) => circle,
                                        _ => {
                                            panic!("Temporal element changed type")
                                        }
                                    }
                                });
                                TemporalSlideElement::Circle {
                                    center: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_circle_t = get_circle_t.clone();
                                        move |t| slide_embedding.map_point(get_circle_t(t).center)
                                    }),
                                    radius: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_circle_t = get_circle_t.clone();
                                        move |t| slide_embedding.map_length(get_circle_t(t).radius)
                                    }),
                                    options: TemporalShapeOptions {
                                        interp: TemporalInterpOptions {
                                            interp_from_id: Some(id.clone()),
                                            interp_to_id: Some(id.clone()),
                                            interp_from_type: None,
                                            interp_to_type: None,
                                        },
                                        visuals: Timeline::from_fn({
                                            let get_circle_t = get_circle_t.clone();
                                            move |t| get_circle_t(t).visuals
                                        }),
                                    },
                                }
                            }
                            CanvasElement::Line(_) => {
                                let get_line_t = Rc::new({
                                    let id = id.clone();
                                    let at_t = at_t_check_matches.clone();
                                    move |t| match at_t(t).get(&id).unwrap().element.clone() {
                                        CanvasElement::Line(line) => line,
                                        _ => {
                                            panic!("Temporal element changed type")
                                        }
                                    }
                                });
                                TemporalSlideElement::Line {
                                    start: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_line_t = get_line_t.clone();
                                        move |t| slide_embedding.map_point(get_line_t(t).start)
                                    }),
                                    end: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_line_t = get_line_t.clone();
                                        move |t| slide_embedding.map_point(get_line_t(t).end)
                                    }),
                                    radius: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_line_t = get_line_t.clone();
                                        move |t| slide_embedding.map_length(get_line_t(t).radius)
                                    }),
                                    options: TemporalShapeOptions {
                                        interp: TemporalInterpOptions {
                                            interp_from_id: Some(id.clone()),
                                            interp_to_id: Some(id.clone()),
                                            interp_from_type: None,
                                            interp_to_type: None,
                                        },
                                        visuals: Timeline::from_fn({
                                            let get_line_t = get_line_t.clone();
                                            move |t| get_line_t(t).visuals
                                        }),
                                    },
                                }
                            }
                            CanvasElement::Shape(_) => {
                                let get_shape_t = Rc::new({
                                    let id = id.clone();
                                    let at_t = at_t_check_matches.clone();
                                    move |t| match at_t(t).get(&id).unwrap().element.clone() {
                                        CanvasElement::Shape(shape) => shape,
                                        _ => {
                                            panic!("Temporal element changed type")
                                        }
                                    }
                                });
                                TemporalSlideElement::Shape {
                                    shape: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_shape_t = get_shape_t.clone();
                                        move |t| slide_embedding.map_shape(get_shape_t(t).shape)
                                    }),
                                    options: TemporalShapeOptions {
                                        interp: TemporalInterpOptions {
                                            interp_from_id: Some(id.clone()),
                                            interp_to_id: Some(id.clone()),
                                            interp_from_type: None,
                                            interp_to_type: None,
                                        },
                                        visuals: Timeline::from_fn({
                                            let get_circle_t = get_shape_t.clone();
                                            move |t| get_circle_t(t).visuals
                                        }),
                                    },
                                }
                            }
                            CanvasElement::Pixels(_) => {
                                let get_pixels_t = Rc::new({
                                    let id = id.clone();
                                    let at_t = at_t_check_matches.clone();
                                    move |t| match at_t(t).get(&id).unwrap().element.clone() {
                                        CanvasElement::Pixels(pixels) => pixels,
                                        _ => {
                                            panic!("Temporal element changed type")
                                        }
                                    }
                                });

                                let min = slide_embedding
                                    .map_point((bounding_rect.min_x, bounding_rect.min_y));
                                let max = slide_embedding
                                    .map_point((bounding_rect.max_x, bounding_rect.max_y));

                                TemporalSlideElement::Pixels {
                                    min,
                                    max,
                                    pixels: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        move |t| {
                                            let pixels: Arc<
                                                dyn Fn(Pos2<W, H>) -> ColourWithAlpha + Send + Sync,
                                            > = Arc::new({
                                                let pixels = get_pixels_t(t).clone().pixels;
                                                let slide_embedding = slide_embedding.clone();
                                                move |p| {
                                                    let (x, y) = slide_embedding.unmap_point(p);
                                                    pixels(x, y)
                                                }
                                            });
                                            pixels
                                        }
                                    }),
                                    interp: TemporalInterpOptions {
                                        interp_from_id: Some(id.clone()),
                                        interp_to_id: Some(id.clone()),
                                        interp_from_type: None,
                                        interp_to_type: None,
                                    },
                                }
                            }
                        })
                        .collect()
                },
                instantaneous: Timeline::from_fn({
                    let build_instant = self.build_instant.clone();
                    let bounding_rect = bounding_rect.clone();
                    move |t| {
                        let mut elements = vec![];
                        for element in (build_instant)(t).flatten() {
                            if element.interp_id.is_none() {
                                match element.element {
                                    CanvasElement::Circle(circle) => {
                                        elements.push(circle.slide_embed(&slide_embedding));
                                    }
                                    CanvasElement::Line(line) => {
                                        elements.push(line.slide_embed(&slide_embedding));
                                    }
                                    CanvasElement::Shape(shape) => {
                                        elements.push(shape.slide_embed(&slide_embedding));
                                    }
                                    CanvasElement::Pixels(pixels) => {
                                        elements.push(
                                            pixels.slide_embed(&slide_embedding, &bounding_rect),
                                        );
                                    }
                                }
                            }
                        }
                        elements
                    }
                }),
            }
        } else {
            panic!("Canvas has no bounding rect");
        }
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
