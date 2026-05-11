use super::SlideshowBuilder;
use crate::{
    colour::ColourWithAlpha,
    coords::{Length, Pos2},
    image::{ImageSpec, PixelsImage},
    interpolation::Interpable,
    shape::ShapeSpec,
    slideshow::{
        InterpId, InterpType, ShapeVisualOptions, SlideInterp, SlideshowState,
        instantaneous::InstantaneousSlideElement, temporal::TemporalSlideElement,
    },
    video::{VideoCompiledSpec, VideoSpec},
};
use core::f64;
use ordered_float::OrderedFloat;
use std::sync::Arc;

fn get_idx_and_frac(
    t: OrderedFloat<f64>,
    timings: &[OrderedFloat<f64>],
) -> (usize, OrderedFloat<f64>) {
    let state_idx = match timings.binary_search(&t) {
        Ok(i) => i,
        Err(i) => i - 1,
    };
    let state_frac = (t - timings[state_idx]) / (timings[state_idx + 1] - timings[state_idx]);
    (state_idx, state_frac)
}

pub enum DrawElement<const W: u32, const H: u32> {
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

#[derive(Clone)]
struct GluedTemporalElement<const W: u32, const H: u32> {
    // one entry for each slide
    // None if the element is not present on the slide, otherwise Some
    elements: Vec<Option<TemporalSlideElement<W, H>>>,
}

impl<const W: u32, const H: u32> GluedTemporalElement<W, H> {
    fn draw_at(
        &self,
        t: OrderedFloat<f64>,
        timings: &[OrderedFloat<f64>],
    ) -> Vec<DrawElement<W, H>> {
        // example: 3 states and 6 timings
        // t       t        t       t        t       t
        // - state - interp - state - interp - state -
        // i.e. a timing before and after each state
        assert_eq!(2 * self.elements.len(), timings.len());
        let (state_idx, state_frac) = get_idx_and_frac(t, timings);
        if state_idx % 2 == 0 {
            // on a slide
            let slide_idx = state_idx / 2;
            let slide_t = *(t - timings[slide_idx]);

            self.elements[slide_idx]
                .as_ref()
                .iter()
                .flat_map(|element| element.to_instantaneous(slide_t).draw())
                .collect()
        } else {
            // on an interp
            let from_slide_idx = (state_idx - 1) / 2;
            #[allow(clippy::manual_div_ceil)]
            let to_slide_idx = (state_idx + 1) / 2;

            let to_slide_t = *(t - timings[to_slide_idx]);
            let from_slide_t = *(t - timings[from_slide_idx]);

            match (&self.elements[from_slide_idx], &self.elements[to_slide_idx]) {
                (None, None) => vec![],
                (None, Some(to_element)) => to_element
                    .to_instantaneous(to_slide_t)
                    .draw_partial_in(*state_frac),
                (Some(from_element), None) => from_element
                    .to_instantaneous(from_slide_t)
                    .draw_partial_out(*state_frac),
                (Some(from_element), Some(to_element)) => {
                    let interp_frac = match (
                        from_element.interp_options().interp_to_type,
                        to_element.interp_options().interp_from_type,
                    ) {
                        (None, None) => InterpType::default().apply(*state_frac),
                        (None, Some(j)) => j.apply(*state_frac),
                        (Some(i), None) => i.apply(*state_frac),
                        (Some(i), Some(j)) => {
                            f64::interp(&i.apply(*state_frac), &j.apply(*state_frac), *state_frac)
                        }
                    };

                    vec![match (&from_element, &to_element) {
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
                            DrawElement::Shape {
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
                            DrawElement::Shape {
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
                            DrawElement::Shape {
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
                            DrawElement::Shape {
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
                        ) => DrawElement::Pixels {
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
                            unimplemented!("Interpolation not implemented for these element types");
                        }
                    }]
                }
            }
        }
    }
}

impl<const W: u32, const H: u32> InstantaneousSlideElement<W, H> {
    fn draw(self) -> Vec<DrawElement<W, H>> {
        match self {
            InstantaneousSlideElement::Shape { shape, visuals, .. } => {
                vec![DrawElement::Shape {
                    shape,
                    fill: visuals.fill_rgba,
                }]
            }
            InstantaneousSlideElement::Pixels {
                min, max, pixels, ..
            } => {
                vec![DrawElement::Pixels {
                    min: min.pixels(),
                    max: max.pixels(),
                    pixels,
                }]
            }
        }
    }

    fn draw_partial_impl(
        self,
        interp_frac: f64,
        interp_type: InterpType,
    ) -> Vec<DrawElement<W, H>> {
        let interp_frac_typed = interp_type.apply(interp_frac);
        match self {
            InstantaneousSlideElement::Shape { shape, visuals, .. } => {
                if true {
                    vec![
                        DrawElement::Shape {
                            shape: shape
                                .partial_boundary(
                                    2.0 * 1.0,
                                    (0.0, 1.0 - (1.0 - (2.0 * interp_frac).min(1.0)).powi(2)),
                                )
                                .intersect(&shape),
                            fill: visuals.fill_rgba,
                        },
                        DrawElement::Shape {
                            shape: shape.clone(),
                            fill: visuals.fill_rgba.mul_alpha(
                                interp_type.apply((2.0 * interp_frac - 1.0).max(0.0)) as f32,
                            ),
                        },
                    ]
                } else {
                    vec![DrawElement::Shape {
                        shape: shape.clone(),
                        fill: visuals.fill_rgba.mul_alpha(interp_frac_typed as f32),
                    }]
                }
            }
            InstantaneousSlideElement::Pixels {
                min, max, pixels, ..
            } => {
                vec![DrawElement::Pixels {
                    min: min.pixels(),
                    max: max.pixels(),
                    pixels: Arc::new(move |p| pixels(p).mul_alpha(interp_frac_typed as f32)),
                }]
            }
        }
    }

    fn draw_partial_in(self, interp_frac: f64) -> Vec<DrawElement<W, H>> {
        let interp_type = self.interp_options().interp_in_type.unwrap_or_default();
        self.draw_partial_impl(interp_frac, interp_type)
    }

    fn draw_partial_out(self, interp_frac: f64) -> Vec<DrawElement<W, H>> {
        let interp_type = self.interp_options().interp_out_type.unwrap_or_default();
        self.draw_partial_impl(1.0 - interp_frac, interp_type)
    }
}

impl<const W: u32, const H: u32> SlideshowBuilder<W, H> {
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
        struct GluingTemporalElement<const W: u32, const H: u32> {
            current_to_id: Option<InterpId>,
            // one entry for each slide
            // None if the element is not present on the slide, otherwise Some
            elements: Vec<Option<TemporalSlideElement<W, H>>>,
        }
        let multislide_elements = {
            let mut finished_multislide_elements: Vec<GluingTemporalElement<W, H>> = vec![];
            let mut active_multislide_elements: Vec<GluingTemporalElement<W, H>> = vec![];
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
                                next_active_multislide_elements.push(GluingTemporalElement {
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
                                finished_multislide_elements.push(GluingTemporalElement {
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
                                next_active_multislide_elements.push(GluingTemporalElement {
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
                .into_iter()
                .map(|x| GluedTemporalElement {
                    elements: x.elements,
                })
                .collect::<Vec<_>>()
        };

        // generate the video
        let image_at_time = |t: OrderedFloat<f64>| -> ImageSpec {
            struct Layer {
                top_left: (i64, i64),
                image: ImageSpec,
            }

            let mut layers = vec![];

            let mut instant_elements = multislide_elements
                .iter()
                .flat_map(|multislide_element| multislide_element.draw_at(t, &timings))
                .chain({
                    let (state_idx, state_frac) = get_idx_and_frac(t, &timings);

                    let mut elements = vec![];
                    if state_idx % 2 == 0 {
                        // on a slide
                        let slide_t = *(t - timings[state_idx]);
                        match &self.states[state_idx] {
                            SlideshowState::Slide(slide) => {
                                for element in slide.elements.instantaneous.at_time(slide_t) {
                                    for element_instant in element.draw() {
                                        elements.push(element_instant);
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

                        let from_slide_t = *(t - timings[from_slide_idx]);
                        let to_slide_t = *(t - timings[to_slide_idx]);

                        match &self.states[to_slide_idx] {
                            SlideshowState::Slide(to_slide) => {
                                for element in to_slide.elements.instantaneous.at_time(to_slide_t) {
                                    for element_instant in element.draw_partial_in(*state_frac) {
                                        elements.push(element_instant);
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
                                    for element_instant in element.draw_partial_out(*state_frac) {
                                        elements.push(element_instant);
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
                    DrawElement::Shape { fill, .. } => fill.to_relative_luminance(),
                    DrawElement::Pixels { .. } => -1.0,
                })
            });

            for instant_element in instant_elements {
                match instant_element {
                    DrawElement::Shape { shape, fill } => {
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
                    DrawElement::Pixels { min, max, pixels } => {
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
