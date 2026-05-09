use crate::{
    colour::ColourRgba,
    coords::{Length, Pos2, Rect, SCREEN_UNITS, Vec2},
    image::ImageSpec,
    interpolation::Interpable,
    shape::ShapeSpec,
    video::{VideoCompiledSpec, VideoSpec},
};
use ordered_float::OrderedFloat;
use std::rc::Rc;

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
    user: i64,
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
            user: id,
            draw_order_idx: vec![],
        });
        self
    }
    fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.interp_to_id = Some(InterpId {
            user: id,
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
    fill_rgba: ColourRgba,
}
impl Default for ShapeVisualOptions {
    fn default() -> Self {
        Self {
            fill_rgba: ColourRgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        }
    }
}
impl Interpable for ShapeVisualOptions {
    fn interp(x: &Self, y: &Self, f: f64) -> Self {
        Self {
            fill_rgba: ColourRgba::interp(&x.fill_rgba, &y.fill_rgba, f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemporalShapeOptions {
    interp: TemporalInterpOptions,
    visuals: ShapeVisualOptions,
    bounding_rect_sample_times: Vec<f64>,
}

impl Default for TemporalShapeOptions {
    fn default() -> Self {
        Self {
            interp: Default::default(),
            visuals: Default::default(),
            // sample the bounding rect between 6 and 12 seconds by default
            // first 6 seconds allow any entry animations and next 6 seconds allows for movement
            // user can customize it if this default doesnt work well in some case
            bounding_rect_sample_times: {
                let mut ts = vec![];
                let mut t = 6.0;
                while t <= 12.0 {
                    ts.push(t);
                    t += 0.1;
                }
                ts
            },
        }
    }
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
    pub fn fill_rgba(&mut self, fill_rgba: ColourRgba) -> &mut Self {
        self.visuals.fill_rgba = fill_rgba;
        self
    }
    pub fn bounding_rect_sample_times(
        &mut self,
        bounding_rect_sample_times: impl Into<Vec<f64>>,
    ) -> &mut Self {
        self.bounding_rect_sample_times = bounding_rect_sample_times.into();
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

    fn map<W>(self, f: impl Fn(f64, &V) -> W + 'static) -> Timeline<W>
    where
        V: 'static,
    {
        Timeline::Function(Rc::new(move |t| f(t, &self.at_time(t))))
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
}

impl<const W: u32, const H: u32> TemporalSlideElement<W, H> {
    fn interp_options(&self) -> &TemporalInterpOptions {
        match self {
            Self::Circle { options, .. } => &options.interp,
            Self::Line { options, .. } => &options.interp,
            Self::Shape { options, .. } => &options.interp,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct InstantaneousInterpOptions {
    interp_in_type: Option<InterpType>,
    interp_out_type: Option<InterpType>,
}
impl InstantaneousInterpOptions {
    fn interp_in_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.interp_in_type = Some(interp_type);
        self
    }

    fn interp_out_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.interp_out_type = Some(interp_type);
        self
    }
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
}

impl<const W: u32, const H: u32> InstantaneousSlideElement<W, H> {
    fn interp_options(&self) -> &InstantaneousInterpOptions {
        match self {
            InstantaneousSlideElement::Shape { interp, .. } => interp,
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
    background_colour: ColourRgba,
    states: Vec<SlideshowState<W, H>>,
}

impl<const W: u32, const H: u32> SlideshowBuilder<W, H> {
    pub fn new(background_colour: ColourRgba) -> Self {
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

        struct ElementInstant<const W: u32, const H: u32> {
            shape: ShapeSpec,
            fill: ColourRgba,
        }

        impl<const W: u32, const H: u32> MultiSlideElement<W, H> {
            fn get_at_instant(
                &self,
                (state_idx, state_t, state_frac): (usize, f64, OrderedFloat<f64>),
            ) -> Option<ElementInstant<W, H>> {
                if state_idx % 2 == 0 {
                    // on a slide
                    let slide_idx = state_idx / 2;

                    self.elements[slide_idx]
                        .as_ref()
                        .map(|element| match element {
                            TemporalSlideElement::Circle {
                                center,
                                radius,
                                options,
                            } => ElementInstant {
                                shape: ShapeSpec::Circle {
                                    center: center.at_time(state_t).pixels(),
                                    radius: radius.at_time(state_t).pixels(),
                                },
                                fill: options.visuals.fill_rgba,
                            },
                            TemporalSlideElement::Line {
                                start,
                                end,
                                radius,
                                options,
                            } => ElementInstant {
                                shape: ShapeSpec::Line {
                                    point1: start.at_time(state_t).pixels(),
                                    point2: end.at_time(state_t).pixels(),
                                    radius: radius.at_time(state_t).pixels(),
                                },
                                fill: options.visuals.fill_rgba,
                            },
                            TemporalSlideElement::Shape { shape, options } => ElementInstant {
                                shape: shape.at_time(state_t).clone(),
                                fill: options.visuals.fill_rgba,
                            },
                        })
                } else {
                    // on an interp
                    let from_slide_idx = (state_idx - 1) / 2;
                    #[allow(clippy::manual_div_ceil)]
                    let to_slide_idx = (state_idx + 1) / 2;

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
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    ElementInstant {
                                        shape: ShapeSpec::Circle {
                                            center: center.at_time(state_t).pixels(),
                                            radius: radius.at_time(state_t).pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * interp_frac,
                                        },
                                    }
                                }
                                TemporalSlideElement::Line {
                                    start,
                                    end,
                                    radius,
                                    options,
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    ElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: start.at_time(state_t).pixels(),
                                            point2: end.at_time(state_t).pixels(),
                                            radius: radius.at_time(state_t).pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * interp_frac,
                                        },
                                    }
                                }
                                TemporalSlideElement::Shape { shape, options } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    ElementInstant {
                                        shape: shape.at_time(state_t).clone(),
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * interp_frac,
                                        },
                                    }
                                }
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
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    ElementInstant {
                                        shape: ShapeSpec::Circle {
                                            center: center.at_time(state_t).pixels(),
                                            radius: radius.at_time(state_t).pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * (1.0 - interp_frac),
                                        },
                                    }
                                }
                                TemporalSlideElement::Line {
                                    start,
                                    end,
                                    radius,
                                    options,
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    ElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: start.at_time(state_t).pixels(),
                                            point2: end.at_time(state_t).pixels(),
                                            radius: radius.at_time(state_t).pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * (1.0 - interp_frac),
                                        },
                                    }
                                }
                                TemporalSlideElement::Shape { shape, options } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    ElementInstant {
                                        shape: shape.at_time(state_t).clone(),
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * (1.0 - interp_frac),
                                        },
                                    }
                                }
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
                                        &from_options.visuals,
                                        &to_options.visuals,
                                        interp_frac,
                                    );
                                    ElementInstant {
                                        shape: ShapeSpec::Circle {
                                            center: Pos2::interp(
                                                &from_center.at_time(state_t),
                                                &to_center.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(state_t),
                                                &to_radius.at_time(state_t),
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
                                        &from_options.visuals,
                                        &to_options.visuals,
                                        interp_frac,
                                    );
                                    ElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                &from_center.at_time(state_t),
                                                &to_start.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(
                                                &from_center.at_time(state_t),
                                                &to_end.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(state_t),
                                                &to_radius.at_time(state_t),
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
                                        &from_options.visuals,
                                        &to_options.visuals,
                                        interp_frac,
                                    );
                                    ElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                &from_start.at_time(state_t),
                                                &to_center.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(
                                                &from_end.at_time(state_t),
                                                &to_center.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(state_t),
                                                &to_radius.at_time(state_t),
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
                                        &from_options.visuals,
                                        &to_options.visuals,
                                        interp_frac,
                                    );
                                    ElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                &from_start.at_time(state_t),
                                                &to_start.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(
                                                &from_end.at_time(state_t),
                                                &to_end.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                &from_radius.at_time(state_t),
                                                &to_radius.at_time(state_t),
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                _ => {
                                    unimplemented!("Interpolation not implemented");
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
                top_left: (f64, f64),
                image: ImageSpec,
            }

            let mut layers = vec![];

            let mut instant_elements = multislide_elements
                .iter()
                .filter_map(|multislide_element| multislide_element.get_at_instant(state_pos))
                .chain({
                    let mut elements = vec![];
                    let (state_idx, state_t, state_frac) = state_pos;

                    if state_idx % 2 == 0 {
                        // on a slide
                        let slide_t = state_t;
                        match &self.states[state_idx] {
                            SlideshowState::Slide(slide) => {
                                for element in slide.elements.instantaneous.at_time(slide_t) {
                                    match element {
                                        InstantaneousSlideElement::Shape {
                                            shape, visuals, ..
                                        } => {
                                            elements.push(ElementInstant {
                                                shape,
                                                fill: visuals.fill_rgba,
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
                                            let ColourRgba { r, g, b, a } = visuals.fill_rgba;
                                            elements.push(ElementInstant {
                                                shape,
                                                fill: ColourRgba {
                                                    r,
                                                    g,
                                                    b,
                                                    a: interp_frac * a,
                                                },
                                            });
                                        }
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }

                        let from_slide_t = *(t - timings[from_slide_idx]);
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
                                            let ColourRgba { r, g, b, a } = visuals.fill_rgba;
                                            elements.push(ElementInstant {
                                                shape,
                                                fill: ColourRgba {
                                                    r,
                                                    g,
                                                    b,
                                                    a: (1.0 - interp_frac) * a,
                                                },
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
            // for now, sort by lightness
            instant_elements.sort_by_cached_key(|instant_element| {
                let ColourRgba { r, g, b, a: _ } = instant_element.fill;
                OrderedFloat(0.299 * r + 0.587 * g + 0.114 * b)
            });

            for instant_element in instant_elements {
                if let Some(br) = instant_element.shape.shape().bounding_rect() {
                    let min = br.min().x_y();
                    let min = (min.0.floor() as u32, min.1.floor() as u32);
                    let max = br.max().x_y();
                    let max = (max.0.ceil() as u32, max.1.ceil() as u32);
                    let width = max.0 - min.0;
                    let height = max.1 - min.1;
                    let ColourRgba { r, g, b, a } = instant_element.fill;
                    layers.push(Layer {
                        top_left: (min.0 as f64, min.1 as f64),
                        image: instant_element
                            .shape
                            .translate(-(min.0 as f64), -(min.1 as f64))
                            .image(
                                width,
                                height,
                                ColourRgba { r, g, b, a: 0.0 },
                                ColourRgba { r, g, b, a },
                            ),
                    });
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
    Picture(Picture),
    Canvas(Canvas),
}

impl SlideRegionElement {
    fn finish<const W: u32, const H: u32>(self, rect: &Rect<W, H>) -> SlideElements<W, H> {
        match self {
            SlideRegionElement::Picture(picture) => picture.finish(rect),
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
            _ => unreachable!(),
        }
    }

    pub fn picture(&mut self) -> &mut Picture {
        self.elements.push(SlideRegionElement::Picture(Picture {
            elements: vec![],
            current_interp_from_draw_ordering: None,
            current_interp_to_draw_ordering: None,
        }));
        match self.elements.last_mut().unwrap() {
            SlideRegionElement::Picture(element) => element,
            _ => unreachable!(),
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
                    CanvasElement::Circle(circle) => circle.interp_id.map(|id| InterpId {
                        user: id,
                        draw_order_idx: vec![],
                    }),
                    CanvasElement::Line(line) => line.interp_id.map(|id| InterpId {
                        user: id,
                        draw_order_idx: vec![],
                    }),
                };
                vec![CanvasElementWithInterpId { element, interp_id }]
            }
            CanvasElementOrGroup::Group(group) => group
                .flatten()
                .into_iter()
                .enumerate()
                .map(|(idx, mut element)| {
                    if let Some(interp_id) = element.interp_id.as_mut() {
                        interp_id.draw_order_idx.insert(0, idx);
                    }
                    element
                })
                .collect(),
        }
    }
}

enum CanvasElement {
    Circle(CanvasCircle),
    Line(CanvasLine),
}

pub struct CanvasCircle {
    center: (f64, f64),
    radius: f64,
    interp_id: Option<i64>,
}

impl CanvasCircle {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> InstantaneousSlideElement<W, H> {
        InstantaneousSlideElement::Shape {
            shape: ShapeSpec::Circle {
                center: embedding.map_point(self.center).pixels(),
                radius: embedding.map_length(self.radius).pixels(),
            },
            visuals: ShapeVisualOptions {
                fill_rgba: ColourRgba {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
            },
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

pub struct CanvasLine {
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
    interp_id: Option<i64>,
}

impl CanvasLine {
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
            visuals: ShapeVisualOptions {
                fill_rgba: ColourRgba {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
            },
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

    fn flatten(self) -> Vec<CanvasElementWithInterpId> {
        let mut elements = vec![];
        for element in self.elements {
            elements.append(&mut element.flatten());
        }
        elements
    }
}

pub struct Canvas {
    build_instant: Rc<dyn Fn(f64) -> CanvasInstantGroup>,
}

impl Canvas {
    fn new(build_instant: impl Fn(f64) -> CanvasInstantGroup + 'static) -> Self {
        Self {
            build_instant: Rc::new(build_instant),
        }
    }

    fn finish<const W: u32, const H: u32>(self, rect: &Rect<W, H>) -> SlideElements<W, H> {
        let bounding_rect = BoundingRect::new(-6.0, 6.0, -6.0, 6.0);
        let slide_embedding = SlideEmbedding::fit_within(rect, bounding_rect);

        println!("TODO");
        SlideElements {
            temporal: vec![],
            instantaneous: Timeline::from_fn({
                let build_instant = self.build_instant.clone();
                move |t| {
                    let mut elements = vec![];
                    for element in (build_instant)(t).flatten() {
                        if element.interp_id.is_none() {
                            match element.element {
                                CanvasElement::Circle(canvas_circle) => {
                                    elements.push(canvas_circle.slide_embed(&slide_embedding));
                                }
                                CanvasElement::Line(canvas_line) => {
                                    elements.push(canvas_line.slide_embed(&slide_embedding));
                                }
                            }
                        }
                    }
                    elements

                    /*
                    vec![InstantaneousSlideElement::Shape {
                        shape: ShapeSpec::Circle {
                            center: Pos2::<W, H>::from_units(
                                0.5 * SCREEN_UNITS,
                                0.5 * SCREEN_UNITS,
                            )
                            .pixels(),
                            radius: Length::<W, H>::from_units(0.1 * SCREEN_UNITS).pixels(),
                        },
                        visuals: ShapeVisualOptions {
                            fill_rgba: ColourRgba {
                                r: 1.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            },
                        },
                        interp: InstantaneousInterpOptions {
                            interp_in_type: None,
                            interp_out_type: None,
                        },
                    }]
                    */
                }
            }),
        }
    }
}

struct PictureInterpDrawOrdering {
    group: i64,
    idx: usize,
}

/*
A picture with coordinates in a space independent of the slide coordinates

The picture is fitted within the rect on the slide
*/
pub struct Picture {
    elements: Vec<PictureElement>,
    current_interp_from_draw_ordering: Option<PictureInterpDrawOrdering>,
    current_interp_to_draw_ordering: Option<PictureInterpDrawOrdering>,
}

impl Picture {
    fn current_initial_shape_options(&mut self) -> TemporalShapeOptions {
        let mut options = TemporalShapeOptions::default();
        if let Some(interp_draw_ordering) = &mut self.current_interp_from_draw_ordering {
            options.interp.interp_from_id = Some(InterpId {
                user: interp_draw_ordering.group,
                draw_order_idx: vec![interp_draw_ordering.idx],
            });
            interp_draw_ordering.idx += 1;
        }
        if let Some(interp_draw_ordering) = &mut self.current_interp_to_draw_ordering {
            options.interp.interp_to_id = Some(InterpId {
                user: interp_draw_ordering.group,
                draw_order_idx: vec![interp_draw_ordering.idx],
            });
            interp_draw_ordering.idx += 1;
        }
        options
    }

    pub fn start_interp_id_group(&mut self, group_id: i64) -> &mut Self {
        self.current_interp_from_draw_ordering = Some(PictureInterpDrawOrdering {
            group: group_id,
            idx: 0,
        });
        self.current_interp_to_draw_ordering = Some(PictureInterpDrawOrdering {
            group: group_id,
            idx: 0,
        });
        self
    }

    pub fn stop_interp_id_group(&mut self) -> &mut Self {
        self.current_interp_from_draw_ordering = None;
        self.current_interp_to_draw_ordering = None;
        self
    }

    pub fn circle(
        &mut self,
        center: impl Into<Timeline<(f64, f64)>>,
        radius: impl Into<Timeline<f64>>,
    ) -> &mut PictureCircle {
        let options = self.current_initial_shape_options();
        self.elements.push(PictureElement::Circle(PictureCircle {
            center: center.into(),
            radius: radius.into(),
            options,
        }));
        match self.elements.last_mut().unwrap() {
            PictureElement::Circle(x) => x,
            _ => unreachable!(),
        }
    }

    pub fn line(
        &mut self,
        start: impl Into<Timeline<(f64, f64)>>,
        end: impl Into<Timeline<(f64, f64)>>,
        radius: impl Into<Timeline<f64>>,
    ) -> &mut PictureLine {
        let options = self.current_initial_shape_options();
        self.elements.push(PictureElement::Line(PictureLine {
            start: start.into(),
            end: end.into(),
            radius: radius.into(),
            options,
        }));
        match self.elements.last_mut().unwrap() {
            PictureElement::Line(x) => x,
            _ => unreachable!(),
        }
    }

    pub fn latex(&mut self, expr: impl Into<String>) -> &mut PictureShape {
        let options = self.current_initial_shape_options();
        self.elements.push(PictureElement::Shape(PictureShape {
            shape: Timeline::Constant(ShapeSpec::latex(expr.into())),
            origin: (0.0, 0.0),
            position: Timeline::Constant((0.0, 0.0)),
            options,
        }));
        match self.elements.last_mut().unwrap() {
            PictureElement::Shape(x) => x,
            _ => unreachable!(),
        }
    }

    pub fn text(&mut self, text: impl Into<String>) -> &mut PictureShape {
        self.latex(format!("\\text{{{}}}", text.into()))
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        self.elements
            .iter()
            .filter_map(|shape_element| shape_element.bounding_rect())
            .reduce(|a, b| {
                let min_x = a.min_x.min(b.min_x);
                let min_y = a.min_y.min(b.min_y);
                let max_x = a.max_x.max(b.max_x);
                let max_y = a.max_y.max(b.max_y);
                BoundingRect::new(min_x, max_x, min_y, max_y)
            })
    }

    fn finish<const W: u32, const H: u32>(self, rect: &Rect<W, H>) -> SlideElements<W, H> {
        let mut temporal = vec![];
        if let Some(bounding_rect) = self.bounding_rect() {
            let slide_embedding = SlideEmbedding::fit_within(rect, bounding_rect);
            for element in self.elements {
                temporal.push(element.slide_embed(&slide_embedding));
            }
        }
        SlideElements {
            temporal,
            instantaneous: Timeline::Constant(vec![]),
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

    fn fit_within(rect: &Rect<W, H>, bounding_rect: BoundingRect) -> Self {
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

enum PictureElement {
    Circle(PictureCircle),
    Line(PictureLine),
    Shape(PictureShape),
}

impl PictureElement {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> TemporalSlideElement<W, H> {
        match self {
            PictureElement::Circle(x) => x.slide_embed(embedding),
            PictureElement::Line(x) => x.slide_embed(embedding),
            PictureElement::Shape(x) => x.slide_embed(embedding),
        }
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        match self {
            PictureElement::Circle(x) => x.bounding_rect(),
            PictureElement::Line(x) => x.bounding_rect(),
            PictureElement::Shape(x) => x.bounding_rect(),
        }
    }
}

pub struct PictureCircle {
    center: Timeline<(f64, f64)>,
    radius: Timeline<f64>,
    options: TemporalShapeOptions,
}

impl PictureCircle {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> TemporalSlideElement<W, H> {
        TemporalSlideElement::Circle {
            center: self.center.map({
                let embedding = embedding.clone();
                move |_t, x| embedding.map_point(*x)
            }),
            radius: self.radius.map({
                let embedding = embedding.clone();
                move |_t, x| embedding.map_length(*x)
            }),
            options: self.options,
        }
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        self.options
            .bounding_rect_sample_times
            .iter()
            .map(|t| {
                let center = self.center.at_time(*t);
                let radius = self.radius.at_time(*t);
                BoundingRect::new(
                    center.0 - radius,
                    center.0 + radius,
                    center.1 - radius,
                    center.1 + radius,
                )
            })
            .reduce(|x, y| BoundingRect::union(&x, &y))
    }

    pub fn interp_from_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_from_id(id);
        self
    }
    pub fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_to_id(id);
        self
    }
    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_id(id);
        self
    }
    pub fn interp_from_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.options.interp_from_type(interp_type);
        self
    }
    pub fn interp_to_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.options.interp_to_type(interp_type);
        self
    }
    pub fn fill_rgba(&mut self, fill_rgba: ColourRgba) -> &mut Self {
        self.options.fill_rgba(fill_rgba);
        self
    }
    pub fn bounding_rect_sample_times(
        &mut self,
        bounding_rect_sample_times: impl Into<Vec<f64>>,
    ) -> &mut Self {
        self.options
            .bounding_rect_sample_times(bounding_rect_sample_times);
        self
    }
}

pub struct PictureLine {
    start: Timeline<(f64, f64)>,
    end: Timeline<(f64, f64)>,
    radius: Timeline<f64>,
    options: TemporalShapeOptions,
}

impl PictureLine {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> TemporalSlideElement<W, H> {
        TemporalSlideElement::Line {
            start: self.start.map({
                let embedding = embedding.clone();
                move |_t, x| embedding.map_point(*x)
            }),
            end: self.end.map({
                let embedding = embedding.clone();
                move |_t, x| embedding.map_point(*x)
            }),
            radius: self.radius.map({
                let embedding = embedding.clone();
                move |_t, x| embedding.map_length(*x)
            }),
            options: self.options,
        }
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        self.options
            .bounding_rect_sample_times
            .iter()
            .map(|t| {
                let start = self.start.at_time(*t);
                let end = self.end.at_time(*t);
                let radius = self.radius.at_time(*t);
                BoundingRect::new(
                    start.0.min(end.0) - radius,
                    start.0.max(end.0) + radius,
                    start.1.min(end.1) - radius,
                    start.1.max(end.1) + radius,
                )
            })
            .reduce(|x, y| BoundingRect::union(&x, &y))
    }

    pub fn interp_from_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_from_id(id);
        self
    }
    pub fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_to_id(id);
        self
    }
    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_id(id);
        self
    }
    pub fn interp_from_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.options.interp_from_type(interp_type);
        self
    }
    pub fn interp_to_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.options.interp_to_type(interp_type);
        self
    }
    pub fn fill_rgba(&mut self, fill_rgba: ColourRgba) -> &mut Self {
        self.options.fill_rgba(fill_rgba);
        self
    }
    pub fn bounding_rect_sample_times(
        &mut self,
        bounding_rect_sample_times: impl Into<Vec<f64>>,
    ) -> &mut Self {
        self.options
            .bounding_rect_sample_times(bounding_rect_sample_times);
        self
    }
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

pub struct PictureShape {
    shape: Timeline<ShapeSpec>,
    origin: (f64, f64),
    position: Timeline<(f64, f64)>,
    options: TemporalShapeOptions,
}

impl Default for PictureShape {
    fn default() -> Self {
        Self {
            shape: Timeline::Constant(ShapeSpec::Empty),
            origin: (0.0, 0.0),
            position: Timeline::Constant((0.0, 0.0)),
            options: Default::default(),
        }
    }
}

impl PictureShape {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> TemporalSlideElement<W, H> {
        TemporalSlideElement::Shape {
            shape: self.shape.map({
                let embedding = embedding.clone();
                move |t, x| {
                    let position = self.position.at_time(t);
                    let origin = self.origin;
                    embedding.map_shape(x.translate(position.0 - origin.0, position.1 - origin.1))
                }
            }),
            options: self.options,
        }
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        self.options
            .bounding_rect_sample_times
            .iter()
            .filter_map(|t| {
                self.shape.at_time(*t).shape().bounding_rect().map(|rect| {
                    let min = rect.min().x_y();
                    let max = rect.max().x_y();
                    BoundingRect::new(min.0, max.0, min.1, max.1)
                })
            })
            .reduce(|x, y| BoundingRect::union(&x, &y))
    }

    pub fn position(&mut self, position: impl Into<Timeline<(f64, f64)>>) -> &mut Self {
        self.position = position.into();
        self
    }

    pub fn align(&mut self, align: AlignOptions) -> &mut Self {
        if let Some(br) = self.bounding_rect() {
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
        if let Some(br) = self.bounding_rect() {
            let origin = self.origin;
            self.shape = self.shape.clone().map(move |_t, shape| {
                debug_assert_ne!(br.width(), 0.0);
                shape
                    .translate(-origin.0, -origin.1)
                    .scale(width / br.width())
                    .translate(origin.0, origin.1)
            });
        }
        self
    }

    pub fn height(&mut self, height: f64) -> &mut Self {
        if let Some(br) = self.bounding_rect() {
            let origin = self.origin;
            self.shape = self.shape.clone().map(move |_t, shape| {
                debug_assert_ne!(br.height(), 0.0);
                shape
                    .translate(-origin.0, -origin.1)
                    .scale(height / br.height())
                    .translate(origin.0, origin.1)
            });
        }
        self
    }

    pub fn interp_from_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_from_id(id);
        self
    }
    pub fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_to_id(id);
        self
    }
    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.options.interp_id(id);
        self
    }
    pub fn interp_from_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.options.interp_from_type(interp_type);
        self
    }
    pub fn interp_to_type(&mut self, interp_type: InterpType) -> &mut Self {
        self.options.interp_to_type(interp_type);
        self
    }
    pub fn fill_rgba(&mut self, fill_rgba: ColourRgba) -> &mut Self {
        self.options.fill_rgba(fill_rgba);
        self
    }
    pub fn bounding_rect_sample_times(
        &mut self,
        bounding_rect_sample_times: impl Into<Vec<f64>>,
    ) -> &mut Self {
        self.options
            .bounding_rect_sample_times(bounding_rect_sample_times);
        self
    }
}
