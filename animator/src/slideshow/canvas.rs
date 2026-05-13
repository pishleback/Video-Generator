use super::InterpId;
use super::ShapeVisualOptions;
use super::SlideElements;
use super::SlideEmbedding;
use super::Timeline;
use super::bounding_rect::BoundingRect;
use crate::colour::ColourWithAlpha;
use crate::coords::Pos2;
use crate::coords::Rect;
use crate::shape::ShapeSpec;
use crate::slideshow::Align;
use crate::slideshow::ShapeInterpType;
use crate::slideshow::TemporalSlideElement;
use crate::slideshow::instantaneous::InstantaneousInterpOptions;
use crate::slideshow::instantaneous::InstantaneousSlideElement;
use crate::slideshow::temporal::TemporalInterpId;
use crate::slideshow::temporal::TemporalInterpType;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

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
    untransformed_shape: ShapeSpec,
    origin: (f64, f64),
    scale: f64,
    position: (f64, f64),
    visuals: ShapeVisualOptions,
    interp_id: Option<i64>,
    interp_type: Option<ShapeInterpType>,
}

impl CanvasShape {
    pub fn shape(&self) -> ShapeSpec {
        self.untransformed_shape
            .translate(-self.origin.0, -self.origin.1)
            .scale(self.scale)
            .translate(self.position.0, self.position.1)
    }

    fn untransformed_bounding_rect(&self) -> Option<BoundingRect> {
        self.untransformed_shape.shape().bounding_rect().map(|br| {
            let min = br.min().x_y();
            let max = br.max().x_y();
            BoundingRect::new(min.0, max.0, min.1, max.1)
        })
    }

    fn bounding_rect(&self) -> Option<BoundingRect> {
        self.untransformed_bounding_rect().map(|br| {
            BoundingRect::new(
                self.position.0 + self.scale * (br.min_x() - self.origin.0),
                self.position.0 + self.scale * (br.max_x() - self.origin.0),
                self.position.1 + self.scale * (br.min_y() - self.origin.1),
                self.position.1 + self.scale * (br.max_y() - self.origin.1),
            )
        })
    }

    fn slide_embed<const W: u32, const H: u32>(
        self,
        embedding: &SlideEmbedding<W, H>,
    ) -> InstantaneousSlideElement<W, H> {
        InstantaneousSlideElement::Shape {
            shape: embedding.map_shape(self.shape()),
            visuals: self.visuals,
            interp: InstantaneousInterpOptions {
                interp_in_type: self.interp_type,
                interp_out_type: self.interp_type,
            },
        }
    }

    pub fn interp_id(&mut self, id: i64) -> &mut Self {
        self.interp_id = Some(id);
        self
    }

    pub fn interp_type(&mut self, interp_type: ShapeInterpType) -> &mut Self {
        self.interp_type = Some(interp_type);
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

    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(br) = self.untransformed_bounding_rect() {
            let align = match align {
                Align::TopLeft => (0.0, 0.0),
                Align::TopCenter => (0.5, 0.0),
                Align::TopRight => (1.0, 0.0),
                Align::CenterLeft => (0.0, 0.5),
                Align::Center => (0.5, 0.5),
                Align::CenterRight => (1.0, 0.5),
                Align::BottomLeft => (0.0, 1.0),
                Align::BottomCenter => (0.5, 1.0),
                Align::BottomRight => (1.0, 1.0),
            };
            self.origin = (
                br.min_x() + align.0 * br.width(),
                br.min_y() + align.1 * br.height(),
            );
        }
        self
    }

    pub fn scale(&mut self, scale: f64) -> &mut Self {
        self.scale = scale;
        self
    }

    pub fn width(&mut self, width: f64) -> &mut Self {
        if let Some(br) = self.untransformed_bounding_rect() {
            self.scale = width / br.width();
        }
        self
    }

    pub fn height(&mut self, height: f64) -> &mut Self {
        if let Some(br) = self.untransformed_bounding_rect() {
            self.scale = height / br.height();
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
            min: embedding.map_point((bounding_rect.min_x(), bounding_rect.min_y())),
            max: embedding.map_point((bounding_rect.max_x(), bounding_rect.max_y())),
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
    pub(super) fn new() -> Self {
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

    pub fn maths(&mut self, expr: impl Into<String>) -> &mut CanvasShape {
        let shape = CanvasShape {
            untransformed_shape: ShapeSpec::latex(format!("$\\displaystyle {}$", expr.into())),
            position: (0.0, 0.0),
            scale: 1.0,
            origin: (0.0, 0.0),
            visuals: ShapeVisualOptions::default(),
            interp_id: None,
            interp_type: None,
        };
        self.elements
            .push(CanvasElementOrGroup::Element(CanvasElement::Shape(shape)));
        if let CanvasElementOrGroup::Element(element) = self.elements.last_mut().unwrap()
            && let CanvasElement::Shape(element) = element
        {
            element
        } else {
            unreachable!()
        }
    }

    pub fn latex(&mut self, expr: impl Into<String>) -> &mut CanvasShape {
        let shape = CanvasShape {
            untransformed_shape: ShapeSpec::latex(expr.into()),
            position: (0.0, 0.0),
            scale: 1.0,
            origin: (0.0, 0.0),
            visuals: ShapeVisualOptions::default(),
            interp_id: None,
            interp_type: None,
        };
        self.elements
            .push(CanvasElementOrGroup::Element(CanvasElement::Shape(shape)));
        if let CanvasElementOrGroup::Element(element) = self.elements.last_mut().unwrap()
            && let CanvasElement::Shape(element) = element
        {
            element
        } else {
            unreachable!()
        }
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
    pub(super) fn new(build_instant: impl Fn(f64) -> CanvasInstantGroup + 'static) -> Self {
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

    pub(super) fn finish<const W: u32, const H: u32>(
        self,
        rect: &Rect<W, H>,
    ) -> SlideElements<W, H> {
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
                                        _ => panic!("Temporal element changed type"),
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
                                    visuals: Timeline::from_fn({
                                        let get_circle_t = get_circle_t.clone();
                                        move |t| get_circle_t(t).visuals
                                    }),
                                    interp_type: TemporalInterpType {
                                        from: None,
                                        to: None,
                                    },
                                    interp_id: TemporalInterpId {
                                        from: Some(id.clone()),
                                        to: Some(id.clone()),
                                    },
                                }
                            }
                            CanvasElement::Line(_) => {
                                let get_line_t = Rc::new({
                                    let id = id.clone();
                                    let at_t = at_t_check_matches.clone();
                                    move |t| match at_t(t).get(&id).unwrap().element.clone() {
                                        CanvasElement::Line(line) => line,
                                        _ => panic!("Temporal element changed type"),
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
                                    visuals: Timeline::from_fn({
                                        let get_line_t = get_line_t.clone();
                                        move |t| get_line_t(t).visuals
                                    }),
                                    interp_type: TemporalInterpType {
                                        from: None,
                                        to: None,
                                    },
                                    interp_id: TemporalInterpId {
                                        from: Some(id.clone()),
                                        to: Some(id.clone()),
                                    },
                                }
                            }
                            CanvasElement::Shape(_) => {
                                let get_shape_t = Rc::new({
                                    let id = id.clone();
                                    let at_t = at_t_check_matches.clone();
                                    move |t| match at_t(t).get(&id).unwrap().element.clone() {
                                        CanvasElement::Shape(shape) => shape,
                                        _ => panic!("Temporal element changed type"),
                                    }
                                });
                                TemporalSlideElement::Shape {
                                    shape: Timeline::from_fn({
                                        let slide_embedding = slide_embedding.clone();
                                        let get_shape_t = get_shape_t.clone();
                                        move |t| slide_embedding.map_shape(get_shape_t(t).shape())
                                    }),
                                    visuals: Timeline::from_fn({
                                        let get_circle_t = get_shape_t.clone();
                                        move |t| get_circle_t(t).visuals
                                    }),
                                    interp_type: TemporalInterpType {
                                        from: None,
                                        to: None,
                                    },
                                    interp_id: TemporalInterpId {
                                        from: Some(id.clone()),
                                        to: Some(id.clone()),
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
                                    .map_point((bounding_rect.min_x(), bounding_rect.min_y()));
                                let max = slide_embedding
                                    .map_point((bounding_rect.max_x(), bounding_rect.max_y()));

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
                                    interp_type: TemporalInterpType {
                                        from: None,
                                        to: None,
                                    },
                                    interp_id: TemporalInterpId {
                                        from: Some(id.clone()),
                                        to: Some(id.clone()),
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
