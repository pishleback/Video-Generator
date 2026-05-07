use crate::{
    colour::ColourRgba,
    coords::{Length, Pos2, Rect, Vec2},
    image::ImageSpec,
    interpolation::Interpable,
    shape::{ShapeData, ShapeSpec},
    video::{VideoCompiledSpec, VideoSpec},
};
use geo::Coord;
use ordered_float::OrderedFloat;

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
Options for how an object on a slide should be interpolated from the previous slide and to the next slide

`interp_from_id` and `interp_to_id` are matched against objects on the adjacent slides to decide which objects should be interpolated

`interp_from_type` and `interp_to_type` define the type of interpolation to use
 - if both are None then a default interpolation is used
 - if exactly one is Some, then that interpolation is used
 - if both are Some, then an interpolated interpolation is used
*/
#[derive(Debug, Clone, Default)]
struct InterpOptions {
    interp_from_id: Option<i64>,
    interp_to_id: Option<i64>,
    interp_from_type: Option<InterpType>,
    interp_to_type: Option<InterpType>,
}
impl InterpOptions {
    fn interp_from_id(&mut self, id: i64) -> &mut Self {
        self.interp_from_id = Some(id);
        self
    }
    fn interp_to_id(&mut self, id: i64) -> &mut Self {
        self.interp_to_id = Some(id);
        self
    }
    fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_from_id = Some(id);
        self.interp_to_id = Some(id);
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

#[derive(Default, Debug, Clone)]
pub struct ShapeOptions {
    interp: InterpOptions,
    visuals: ShapeVisualOptions,
}

impl ShapeOptions {
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
}

/*
An element on the slide with its positional information worked out
*/
#[derive(Debug, Clone)]
enum SlideElement<const W: u32, const H: u32> {
    Circle {
        center: Pos2<W, H>,
        radius: Length<W, H>,
        options: ShapeOptions,
    },
    Line {
        start: Pos2<W, H>,
        end: Pos2<W, H>,
        radius: Length<W, H>,
        options: ShapeOptions,
    },
    Shape {
        shape: ShapeSpec, // in pixel coords
        options: ShapeOptions,
    },
}

impl<const W: u32, const H: u32> SlideElement<W, H> {
    fn interp_options(&self) -> &InterpOptions {
        match self {
            Self::Circle { options, .. } => &options.interp,
            Self::Line { options, .. } => &options.interp,
            Self::Shape { options, .. } => &options.interp,
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

/*
A slide and its elements
*/
#[derive(Debug)]
pub struct Slide<const W: u32, const H: u32> {
    timing: SlideshowStateTimingOptions,
    elements: Vec<SlideElement<W, H>>,
}
impl<const W: u32, const H: u32> Slide<W, H> {
    fn new(elements: Vec<SlideElement<W, H>>) -> Self {
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
#[derive(Debug)]
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
#[derive(Debug)]
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
    pub fn slide(&mut self, f: impl FnOnce(&mut SlideRegionBuilder<W, H>)) -> &mut Slide<W, H> {
        let mut slide_builder = SlideRegionBuilder::new(Rect::fullscreen());
        f(&mut slide_builder);
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
        #[derive(Debug, Clone)]
        struct MultiSlideElement<const W: u32, const H: u32> {
            current_to_id: Option<i64>,
            elements: Vec<Option<SlideElement<W, H>>>,
        }
        let multislide_elements = {
            let mut finished_multislide_elements: Vec<MultiSlideElement<W, H>> = vec![];
            let mut active_multislide_elements: Vec<MultiSlideElement<W, H>> = vec![];
            for i in 0..num_slides {
                if let SlideshowState::Slide(slide) = &self.states[2 * i] {
                    let mut matching_pairs = vec![];
                    for multislide_element in &active_multislide_elements {
                        for element in &slide.elements {
                            let interp = element.interp_options();
                            if let Some(i) = interp.interp_from_id
                                && let Some(j) = multislide_element.current_to_id
                                && i == j
                            {
                                matching_pairs.push((multislide_element, element));
                            }
                        }
                    }

                    for multislide_element in &mut finished_multislide_elements {
                        multislide_element.elements.push(None);
                    }

                    let mut next_active_multislide_elements = vec![];

                    for element in &slide.elements {
                        let mut matches = vec![];
                        for (matched_multislide_element, matched_element) in &matching_pairs {
                            if std::ptr::eq(element, *matched_element) {
                                matches.push(matched_multislide_element);
                            }
                        }
                        match matches.len() {
                            0 => {
                                next_active_multislide_elements.push(MultiSlideElement {
                                    current_to_id: element.interp_options().interp_to_id,
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
                                panic!("too many interp id matches");
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
                                    current_to_id: matches[0].interp_options().interp_to_id,
                                    elements: multislide_element
                                        .elements
                                        .clone()
                                        .into_iter()
                                        .chain(vec![Some((*matches[0]).clone())])
                                        .collect(),
                                });
                            }
                            _ => {
                                panic!("too many interp id matches");
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

        let t_to_state_idx_and_frac = |t: OrderedFloat<f64>| -> (usize, OrderedFloat<f64>) {
            let state_idx = match timings.binary_search(&t) {
                Ok(i) => i,
                Err(i) => i - 1,
            };
            let state_frac =
                (t - timings[state_idx]) / (timings[state_idx + 1] - timings[state_idx]);
            (state_idx, state_frac)
        };

        struct MultiSlideElementInstant<const W: u32, const H: u32> {
            shape: ShapeSpec,
            fill: ColourRgba,
        }

        impl<const W: u32, const H: u32> MultiSlideElement<W, H> {
            fn get_at_instant(
                &self,
                (state_idx, state_frac): (usize, OrderedFloat<f64>),
            ) -> Option<MultiSlideElementInstant<W, H>> {
                if state_idx % 2 == 0 {
                    // on a slide
                    let slide_idx = state_idx / 2;

                    self.elements[slide_idx]
                        .as_ref()
                        .map(|element| match element {
                            SlideElement::Circle {
                                center,
                                radius,
                                options,
                            } => MultiSlideElementInstant {
                                shape: ShapeSpec::Circle {
                                    center: center.pixels(),
                                    radius: radius.pixels(),
                                },
                                fill: options.visuals.fill_rgba,
                            },
                            SlideElement::Line {
                                start,
                                end,
                                radius,
                                options,
                            } => MultiSlideElementInstant {
                                shape: ShapeSpec::Line {
                                    point1: start.pixels(),
                                    point2: end.pixels(),
                                    radius: radius.pixels(),
                                },
                                fill: options.visuals.fill_rgba,
                            },
                            SlideElement::Shape { shape, options } => MultiSlideElementInstant {
                                shape: shape.clone(),
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
                                SlideElement::Circle {
                                    center,
                                    radius,
                                    options,
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Circle {
                                            center: center.pixels(),
                                            radius: radius.pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * interp_frac,
                                        },
                                    }
                                }
                                SlideElement::Line {
                                    start,
                                    end,
                                    radius,
                                    options,
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: start.pixels(),
                                            point2: end.pixels(),
                                            radius: radius.pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * interp_frac,
                                        },
                                    }
                                }
                                SlideElement::Shape { shape, options } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    MultiSlideElementInstant {
                                        shape: shape.clone(),
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
                                SlideElement::Circle {
                                    center,
                                    radius,
                                    options,
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Circle {
                                            center: center.pixels(),
                                            radius: radius.pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * (1.0 - interp_frac),
                                        },
                                    }
                                }
                                SlideElement::Line {
                                    start,
                                    end,
                                    radius,
                                    options,
                                } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: start.pixels(),
                                            point2: end.pixels(),
                                            radius: radius.pixels(),
                                        },
                                        fill: ColourRgba {
                                            r,
                                            g,
                                            b,
                                            a: a * (1.0 - interp_frac),
                                        },
                                    }
                                }
                                SlideElement::Shape { shape, options } => {
                                    let ColourRgba { r, g, b, a } = options.visuals.fill_rgba;
                                    MultiSlideElementInstant {
                                        shape: shape.clone(),
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
                                    SlideElement::Circle {
                                        center: from_center,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    SlideElement::Circle {
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
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Circle {
                                            center: Pos2::interp(
                                                from_center,
                                                to_center,
                                                interp_frac,
                                            )
                                            .pixels(),
                                            radius: Length::interp(
                                                from_radius,
                                                to_radius,
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    SlideElement::Circle {
                                        center: from_center,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    SlideElement::Line {
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
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                from_center,
                                                to_start,
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(from_center, to_end, interp_frac)
                                                .pixels(),
                                            radius: Length::interp(
                                                from_radius,
                                                to_radius,
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    SlideElement::Line {
                                        start: from_start,
                                        end: from_end,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    SlideElement::Circle {
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
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(
                                                from_start,
                                                to_center,
                                                interp_frac,
                                            )
                                            .pixels(),
                                            point2: Pos2::interp(from_end, to_center, interp_frac)
                                                .pixels(),
                                            radius: Length::interp(
                                                from_radius,
                                                to_radius,
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                (
                                    SlideElement::Line {
                                        start: from_start,
                                        end: from_end,
                                        radius: from_radius,
                                        options: from_options,
                                    },
                                    SlideElement::Line {
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
                                    MultiSlideElementInstant {
                                        shape: ShapeSpec::Line {
                                            point1: Pos2::interp(from_start, to_start, interp_frac)
                                                .pixels(),
                                            point2: Pos2::interp(from_end, to_end, interp_frac)
                                                .pixels(),
                                            radius: Length::interp(
                                                from_radius,
                                                to_radius,
                                                interp_frac,
                                            )
                                            .pixels(),
                                        },
                                        fill: visuals.fill_rgba,
                                    }
                                }
                                _ => {
                                    unimplemented!(
                                        "Interpolation not implemented from {:?} to {:?}",
                                        from_element,
                                        to_element
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
            let state_pos = t_to_state_idx_and_frac(t);

            struct Layer {
                top_left: (f64, f64),
                image: ImageSpec,
            }

            let mut layers = vec![];

            let mut multislide_instant_elements = multislide_elements
                .iter()
                .filter_map(|multislide_element| multislide_element.get_at_instant(state_pos))
                .collect::<Vec<_>>();

            // TODO: custom options for how to sort
            // for now, sort by lightness
            multislide_instant_elements.sort_by_cached_key(|multislide_instant_element| {
                let ColourRgba { r, g, b, a: _ } = multislide_instant_element.fill;
                OrderedFloat(0.299 * r + 0.587 * g + 0.114 * b)
            });

            for multislide_instant_element in multislide_instant_elements {
                let ColourRgba { r, g, b, a } = multislide_instant_element.fill;
                layers.push(Layer {
                    top_left: (0.0, 0.0),
                    image: multislide_instant_element.shape.image(
                        W,
                        H,
                        ColourRgba { r, g, b, a: 0.0 },
                        ColourRgba { r, g, b, a },
                    ),
                });
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

/*
For placing visual elements on a region of the slide
*/
pub struct SlideRegionBuilder<const W: u32, const H: u32> {
    rect: Rect<W, H>,
    subregions: Vec<SlideRegionBuilder<W, H>>,
    pictures: Vec<Picture<W, H>>,
}

impl<const W: u32, const H: u32> SlideRegionBuilder<W, H> {
    fn new(rect: Rect<W, H>) -> Self {
        Self {
            rect,
            subregions: vec![],
            pictures: vec![],
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

    pub fn picture(&mut self) -> &mut Picture<W, H> {
        self.pictures.push(Picture { elements: vec![] });
        self.pictures.last_mut().unwrap()
    }

    fn finish(self) -> Vec<SlideElement<W, H>> {
        let mut elements = vec![];
        for subregion in self.subregions {
            elements.append(&mut subregion.finish());
        }
        for picture in self.pictures {
            if let Some(bounding_rect) = picture.bounding_rect() {
                let slide_embedding = SlideEmbedding::fit_within(self.rect, bounding_rect);
                for element in picture.elements {
                    elements.push(element.slide_embed(&slide_embedding));
                }
            }
        }
        elements
    }
}

/*
A picture with coordinates in a space independent of the slide coordinates

The picture is fitted within the rect on the slide
*/
pub struct Picture<const W: u32, const H: u32> {
    elements: Vec<PictureElement>,
}

impl<const W: u32, const H: u32> Picture<W, H> {
    pub fn circle(&mut self, center: (f64, f64), radius: f64) -> &mut PictureCircle {
        self.elements.push(PictureElement::Circle(PictureCircle {
            center,
            radius,
            options: Default::default(),
        }));
        match self.elements.last_mut().unwrap() {
            PictureElement::Circle(x) => x,
            _ => unreachable!(),
        }
    }

    pub fn line(&mut self, start: (f64, f64), end: (f64, f64), radius: f64) -> &mut PictureLine {
        self.elements.push(PictureElement::Line(PictureLine {
            start,
            end,
            radius,
            options: Default::default(),
        }));
        match self.elements.last_mut().unwrap() {
            PictureElement::Line(x) => x,
            _ => unreachable!(),
        }
    }

    pub fn latex(&mut self, expr: impl Into<String>) -> &mut PictureShape {
        self.elements.push(PictureElement::Shape(PictureShape {
            shape: ShapeSpec::latex(expr.into()),
            ..Default::default()
        }));
        match self.elements.last_mut().unwrap() {
            PictureElement::Shape(x) => x,
            _ => unreachable!(),
        }
    }

    fn bounding_rect(&self) -> Option<geo::Rect> {
        self.elements
            .iter()
            .filter_map(|shape_element| shape_element.bounding_rect())
            .reduce(|a, b| {
                let min_x = a.min().x.min(b.min().x);
                let min_y = a.min().y.min(b.min().y);
                let max_x = a.max().x.max(b.max().x);
                let max_y = a.max().y.max(b.max().y);
                geo::Rect::new(
                    geo::Coord { x: min_x, y: min_y },
                    geo::Coord { x: max_x, y: max_y },
                )
            })
    }
}

/*
A mapping from mathematical coordinates into slide coordinates
*/
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

    fn fit_within(rect: Rect<W, H>, bounding_rect: geo::Rect) -> Self {
        Self {
            origin: bounding_rect.center().x_y(),
            scale: if bounding_rect.width() * rect.height() < bounding_rect.height() * rect.width()
            {
                rect.height() / bounding_rect.height()
            } else {
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
    ) -> SlideElement<W, H> {
        match self {
            PictureElement::Circle(x) => x.slide_embed(embedding),
            PictureElement::Line(x) => x.slide_embed(embedding),
            PictureElement::Shape(x) => x.slide_embed(embedding),
        }
    }

    fn bounding_rect(&self) -> Option<geo::Rect> {
        match self {
            PictureElement::Circle(x) => x.bounding_rect(),
            PictureElement::Line(x) => x.bounding_rect(),
            PictureElement::Shape(x) => x.bounding_rect(),
        }
    }
}

pub struct PictureCircle {
    center: (f64, f64),
    radius: f64,
    options: ShapeOptions,
}

impl PictureCircle {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> SlideElement<W, H> {
        SlideElement::Circle {
            center: embedding.map_point(self.center),
            radius: embedding.map_length(self.radius),
            options: self.options,
        }
    }

    fn bounding_rect(&self) -> Option<geo::Rect> {
        Some(geo::Rect::new(
            Coord {
                x: self.center.0 - self.radius,
                y: self.center.1 - self.radius,
            },
            Coord {
                x: self.center.0 + self.radius,
                y: self.center.1 + self.radius,
            },
        ))
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
}

pub struct PictureLine {
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
    options: ShapeOptions,
}

impl PictureLine {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> SlideElement<W, H> {
        SlideElement::Line {
            start: embedding.map_point(self.start),
            end: embedding.map_point(self.end),
            radius: embedding.map_length(self.radius),
            options: self.options,
        }
    }

    fn bounding_rect(&self) -> Option<geo::Rect> {
        Some(geo::Rect::new(
            Coord {
                x: self.start.0.min(self.end.0) - self.radius,
                y: self.start.1.min(self.end.1) - self.radius,
            },
            Coord {
                x: self.start.0.max(self.end.0) + self.radius,
                y: self.start.1.max(self.end.1) + self.radius,
            },
        ))
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
    shape: ShapeSpec,
    origin: (f64, f64),
    position: (f64, f64),
    options: ShapeOptions,
}

impl Default for PictureShape {
    fn default() -> Self {
        Self {
            shape: ShapeSpec::Empty,
            origin: (0.0, 0.0),
            position: (0.0, 0.0),
            options: Default::default(),
        }
    }
}

impl PictureShape {
    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> SlideElement<W, H> {
        SlideElement::Shape {
            shape: embedding.map_shape(self.shape),
            options: self.options,
        }
    }

    fn bounding_rect(&self) -> Option<geo::Rect> {
        self.shape.shape().bounding_rect()
    }

    pub fn position(&mut self, position: (f64, f64)) -> &mut Self {
        self.position = position;
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

            self.origin = ();
        }
        self
    }

    pub fn width(&mut self, width: f64) -> &mut Self {
        if let Some(br) = self.bounding_rect() {
            self.shape = self.shape.scale(width / br.width());
        }
        self
    }

    pub fn height(&mut self, height: f64) -> &mut Self {
        if let Some(br) = self.bounding_rect() {
            self.shape = self.shape.scale(height / br.height());
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
}
