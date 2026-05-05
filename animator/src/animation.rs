use ordered_float::OrderedFloat;

use crate::{
    audio::AudioSpec,
    colour::{ColourRgb, ColourRgba},
    coords::{Length, Pos2, Rect, SCREEN_UNITS, Vec2},
    image::ImageSpec,
    interpolation::InterpType,
    shape::ShapeSpec,
    timeline::{ConstantTimeline, InterpTimeline, Timeline},
    video::{VideoAudioClip, VideoCompiledSpec, VideoSpec},
};
use core::f64;
use std::fmt::Debug;
use std::{cell::RefCell, rc::Rc};

pub trait AnimationElement<const WIDTH: u32, const HEIGHT: u32>: 'static {
    fn get_draw_ordering(&self, t: f64) -> (OrderedFloat<f64>, OrderedFloat<f64>); // for draw ordering
    fn set_within_rect(&self, t: f64, rect: Rect<WIDTH, HEIGHT>, interp: InterpType);
    fn apply(&self, t: f64, image_spec: ImageSpec) -> ImageSpec;
}

#[derive(Debug, Clone, Copy)]
pub enum BoundaryMode {
    Inner,
    Middle,
    Outer,
}

pub struct ShapeElement<const WIDTH: u32, const HEIGHT: u32> {
    draw_ordering: RefCell<InterpTimeline<OrderedFloat<f64>>>,
    shape: Box<dyn Timeline<ShapeSpec>>,
    origin: RefCell<InterpTimeline<(f64, f64)>>, // in shape coordinates where is the center
    position: RefCell<InterpTimeline<Pos2<WIDTH, HEIGHT>>>, // as parts per thousand of the view
    scale: RefCell<InterpTimeline<Length<WIDTH, HEIGHT>>>,
    fill_colour: RefCell<InterpTimeline<ColourRgba>>,
    boundary_thickness: RefCell<InterpTimeline<Length<WIDTH, HEIGHT>>>,
    boundary_colour: RefCell<InterpTimeline<ColourRgba>>,
    boundary_frac: RefCell<InterpTimeline<(f64, f64)>>,
    boundary_mode: RefCell<InterpTimeline<BoundaryMode>>,
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElement<WIDTH, HEIGHT> {
    pub fn set_draw_ordering(&self, t: f64, draw_ordering: OrderedFloat<f64>) {
        self.draw_ordering
            .borrow_mut()
            .set_immediate(t, draw_ordering);
    }

    pub fn set_origin(&self, t: f64, v: (f64, f64), interp: InterpType) {
        self.origin.borrow_mut().set(t, v, interp);
    }

    pub fn set_position(&self, t: f64, v: Pos2<WIDTH, HEIGHT>, interp: InterpType) {
        self.position.borrow_mut().set(t, v, interp);
    }

    pub fn set_scale(&self, t: f64, v: Length<WIDTH, HEIGHT>, interp: InterpType) {
        self.scale.borrow_mut().set(t, v, interp);
    }

    pub fn set_fill_rgba(&self, t: f64, v: ColourRgba, interp: InterpType) {
        self.fill_colour.borrow_mut().set(t, v, interp);
    }

    pub fn set_fill_rgb(&self, t: f64, ColourRgb { r, g, b }: ColourRgb, interp: InterpType) {
        let a = self.fill_colour.borrow().at_time(t).a;
        self.fill_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a }, interp);
    }

    pub fn set_fill_alpha(&self, t: f64, alpha: f64, interp: InterpType) {
        let ColourRgba { r, g, b, .. } = self.fill_colour.borrow().at_time(t);
        self.fill_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a: alpha }, interp);
    }

    pub fn set_boundary_thickness(&self, t: f64, v: Length<WIDTH, HEIGHT>, interp: InterpType) {
        self.boundary_thickness.borrow_mut().set(t, v, interp);
    }

    pub fn set_boundary_rgba(&self, t: f64, v: ColourRgba, interp: InterpType) {
        self.boundary_colour.borrow_mut().set(t, v, interp);
    }

    pub fn set_boundary_rgb(&self, t: f64, ColourRgb { r, g, b }: ColourRgb, interp: InterpType) {
        let a = self.boundary_colour.borrow().at_time(t).a;
        self.boundary_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a }, interp);
    }

    pub fn set_boundary_alpha(&self, t: f64, alpha: f64, interp: InterpType) {
        let ColourRgba { r, g, b, .. } = self.boundary_colour.borrow().at_time(t);
        self.boundary_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a: alpha }, interp);
    }

    pub fn set_boundary_frac(&self, t: f64, v: (f64, f64), interp: InterpType) {
        self.boundary_frac.borrow_mut().set(t, v, interp);
    }

    pub fn set_boundary_mode(&self, t: f64, v: BoundaryMode) {
        self.boundary_mode.borrow_mut().set_immediate(t, v);
    }

    fn bounding_rect(&self, t: f64) -> Option<geo::Rect> {
        let shape = self.shape.at_time(t).shape();
        shape.bounding_rect()
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for ShapeElement<WIDTH, HEIGHT>
{
    fn get_draw_ordering(&self, t: f64) -> (OrderedFloat<f64>, OrderedFloat<f64>) {
        // base depth on brightness
        let ColourRgba { r, g, b, .. } = self.fill_colour.borrow().at_time(t);
        (
            self.draw_ordering.borrow().at_time(t),
            (0.299 * r + 0.587 * g + 0.114 * b).into(),
        )
    }

    fn apply(&self, t: f64, image_spec: ImageSpec) -> ImageSpec {
        let (origin_x, origin_y) = self.origin.borrow().at_time(t);
        let (position_x, position_y) = self.position.borrow().at_time(t).pixels();
        let scale = self.scale.borrow().at_time(t);

        let shape = self
            .shape
            .at_time(t)
            .translate(-origin_x, -origin_y)
            .scale(scale.pixels())
            .translate(position_x, position_y);

        let boundary_thickness = self.boundary_thickness.borrow().at_time(t).pixels();
        let shape_boundary = match self.boundary_mode.borrow().at_time(t) {
            BoundaryMode::Inner => shape
                .partial_boundary(
                    2.0 * boundary_thickness,
                    self.boundary_frac.borrow().at_time(t),
                )
                .intersect(&shape),
            BoundaryMode::Middle => {
                shape.partial_boundary(boundary_thickness, self.boundary_frac.borrow().at_time(t))
            }
            BoundaryMode::Outer => shape
                .partial_boundary(
                    2.0 * boundary_thickness,
                    self.boundary_frac.borrow().at_time(t),
                )
                .subtract(&shape),
        };

        let fill_colour = self.fill_colour.borrow().at_time(t);
        let boundary_colour = self.boundary_colour.borrow().at_time(t);

        if fill_colour.a == 0.0 && (boundary_colour.a == 0.0 || boundary_thickness == 0.0) {
            return image_spec;
        }

        let mut layers = vec![((0.0, 0.0), image_spec)];
        if fill_colour.a != 0.0 {
            layers.push(((0.0, 0.0), {
                let ColourRgba { r, g, b, a } = self.fill_colour.borrow().at_time(t);
                shape.image(
                    WIDTH,
                    HEIGHT,
                    ColourRgba { r, g, b, a: 0.0 },
                    ColourRgba { r, g, b, a },
                )
            }));
        }
        if boundary_colour.a != 0.0 && boundary_thickness != 0.0 {
            layers.push(((0.0, 0.0), {
                let ColourRgba { r, g, b, a } = self.boundary_colour.borrow().at_time(t);
                shape_boundary.image(
                    WIDTH,
                    HEIGHT,
                    ColourRgba { r, g, b, a: 0.0 },
                    ColourRgba { r, g, b, a },
                )
            }));
        }
        ImageSpec::BlitStack {
            width: WIDTH,
            height: HEIGHT,
            layers,
        }
    }

    fn set_within_rect(&self, t: f64, rect: Rect<WIDTH, HEIGHT>, interp: InterpType) {
        let shape = self.shape.at_time(t).shape();
        if let Some(bounding_rect) = shape.bounding_rect() {
            let (br_min_x, br_min_y) = bounding_rect.min().x_y();
            let (br_w, br_h) = (bounding_rect.width(), bounding_rect.height());
            let (r_w, r_h) = rect.size().pixels();
            self.set_origin(t, (br_min_x + 0.5 * br_w, br_min_y + 0.5 * br_h), interp);
            self.set_position(t, rect.center(), interp);
            self.set_scale(
                t,
                if br_w * r_h < br_h * r_w {
                    rect.height() / br_h
                } else {
                    rect.width() / br_w
                },
                interp,
            );
        }
    }
}

pub struct ShapeElementCollection<const WIDTH: u32, const HEIGHT: u32> {
    shape_elements: Vec<Rc<ShapeElement<WIDTH, HEIGHT>>>,
}

pub fn shape_group<const WIDTH: u32, const HEIGHT: u32>(
    shape_elements: impl IntoIterator<Item = Rc<ShapeElement<WIDTH, HEIGHT>>>,
) -> ShapeElementCollection<WIDTH, HEIGHT> {
    ShapeElementCollection {
        shape_elements: shape_elements.into_iter().collect(),
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElementCollection<WIDTH, HEIGHT> {
    pub fn set_origin(&self, t: f64, v: (f64, f64), interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_origin(t, v, interp);
        }
    }

    pub fn set_position(&self, t: f64, v: Pos2<WIDTH, HEIGHT>, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_position(t, v, interp);
        }
    }

    pub fn set_scale(&self, t: f64, v: Length<WIDTH, HEIGHT>, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_scale(t, v, interp);
        }
    }

    pub fn set_fill_rgba(&self, t: f64, v: ColourRgba, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_fill_rgba(t, v, interp);
        }
    }

    pub fn set_fill_rgb(&self, t: f64, v: ColourRgb, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_fill_rgb(t, v, interp);
        }
    }

    pub fn set_fill_alpha(&self, t: f64, alpha: f64, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_fill_alpha(t, alpha, interp);
        }
    }

    pub fn set_boundary_thickness(&self, t: f64, v: Length<WIDTH, HEIGHT>, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_boundary_thickness(t, v, interp);
        }
    }

    pub fn set_boundary_rgba(&self, t: f64, v: ColourRgba, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_boundary_rgba(t, v, interp);
        }
    }

    pub fn set_boundary_rgb(&self, t: f64, v: ColourRgb, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_boundary_rgb(t, v, interp);
        }
    }

    pub fn set_boundary_alpha(&self, t: f64, alpha: f64, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_boundary_alpha(t, alpha, interp);
        }
    }

    pub fn set_boundary_frac(&self, t: f64, v: (f64, f64), interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_boundary_frac(t, v, interp);
        }
    }

    pub fn set_boundary_mode(&self, t: f64, v: BoundaryMode) {
        for elem in &self.shape_elements {
            elem.set_boundary_mode(t, v);
        }
    }

    pub fn set_within_rect(&self, t: f64, rect: Rect<WIDTH, HEIGHT>, interp: InterpType) {
        if let Some(bounding_rect) = self
            .shape_elements
            .iter()
            .filter_map(|shape_element| shape_element.bounding_rect(t))
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
        {
            let (br_min_x, br_min_y) = bounding_rect.min().x_y();
            let (br_w, br_h) = (bounding_rect.width(), bounding_rect.height());
            let (r_w, r_h) = rect.size().pixels();
            self.set_origin(t, (br_min_x + 0.5 * br_w, br_min_y + 0.5 * br_h), interp);
            self.set_position(t, rect.center(), interp);
            self.set_scale(
                t,
                if br_w * r_h < br_h * r_w {
                    rect.height() / br_h
                } else {
                    rect.width() / br_w
                },
                interp,
            );
        }
    }
}

pub enum SubVideoSource {
    Video(VideoSpec),
    ImageTimeline {
        size_ratio: (f64, f64),
        images: Box<dyn Timeline<Option<ImageSpec>>>,
    },
}

pub struct SubVideoElement<const WIDTH: u32, const HEIGHT: u32> {
    draw_ordering: RefCell<InterpTimeline<OrderedFloat<f64>>>,
    at_t: f64,
    images: Box<dyn Timeline<Option<ImageSpec>>>,
    size_ratio: (f64, f64),
    origin: RefCell<InterpTimeline<(f64, f64)>>,
    position: RefCell<InterpTimeline<Pos2<WIDTH, HEIGHT>>>,
    size: RefCell<InterpTimeline<Vec2<WIDTH, HEIGHT>>>,
}

impl<const WIDTH: u32, const HEIGHT: u32> SubVideoElement<WIDTH, HEIGHT> {
    pub fn set_draw_ordering(&self, t: f64, draw_ordering: OrderedFloat<f64>) {
        self.draw_ordering
            .borrow_mut()
            .set_immediate(t, draw_ordering);
    }

    pub fn set_origin(&self, t: f64, v: (f64, f64), interp: InterpType) {
        self.origin.borrow_mut().set(t, v, interp);
    }

    pub fn set_position(&self, t: f64, v: Pos2<WIDTH, HEIGHT>, interp: InterpType) {
        self.position.borrow_mut().set(t, v, interp);
    }

    pub fn set_width(&self, t: f64, width: Length<WIDTH, HEIGHT>, interp: InterpType) {
        self.size
            .borrow_mut()
            .set(t, Vec2::from_x_and_slope(width, self.size_ratio), interp);
    }

    pub fn set_height(&self, t: f64, height: Length<WIDTH, HEIGHT>, interp: InterpType) {
        self.size
            .borrow_mut()
            .set(t, Vec2::from_y_and_slope(height, self.size_ratio), interp);
    }

    pub fn set_size(&self, t: f64, v: Vec2<WIDTH, HEIGHT>, interp: InterpType) {
        self.size.borrow_mut().set(t, v, interp);
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for SubVideoElement<WIDTH, HEIGHT>
{
    fn get_draw_ordering(&self, t: f64) -> (OrderedFloat<f64>, OrderedFloat<f64>) {
        (self.draw_ordering.borrow().at_time(t), 0.0.into())
    }

    fn apply(&self, t: f64, image_spec: ImageSpec) -> ImageSpec {
        if let Some(img) = self.images.at_time(t - self.at_t) {
            let (origin_x, origin_y) = self.origin.borrow().at_time(t);
            let (position_x, position_y) = self.position.borrow().at_time(t).pixels();
            let (size_w, size_h) = self.size.borrow().at_time(t).pixels();
            ImageSpec::BlitStack {
                width: WIDTH,
                height: HEIGHT,
                layers: vec![
                    ((0.0, 0.0), image_spec),
                    (
                        (
                            position_x - origin_x * size_w,
                            position_y - origin_y * size_h,
                        ),
                        ImageSpec::Resize {
                            image: Box::new(img),
                            width: size_w as u32,
                            height: size_h as u32,
                        },
                    ),
                ],
            }
        } else {
            image_spec
        }
    }

    fn set_within_rect(&self, t: f64, rect: Rect<WIDTH, HEIGHT>, interp: InterpType) {
        self.set_origin(t, (0.5, 0.5), interp);
        self.set_position(t, rect.center(), interp);
        let (rw, rh) = rect.size().to_lengths();
        let (sw, sh) = self.size_ratio;
        if rw * sh < rh * sw {
            self.set_width(t, rw, interp);
        } else {
            self.set_height(t, rh, interp);
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnimationAudioClip {
    pub at_t: f64,
    pub audio: AudioSpec,
}

pub struct Animation<const WIDTH: u32, const HEIGHT: u32> {
    default_image: ImageSpec,
    elements: Vec<Rc<dyn AnimationElement<WIDTH, HEIGHT>>>,
    audio: Vec<AnimationAudioClip>,
}

impl<const WIDTH: u32, const HEIGHT: u32> Animation<WIDTH, HEIGHT> {
    pub fn new(default_bg: ColourRgba) -> Self {
        Self {
            default_image: ImageSpec::Filled {
                width: WIDTH,
                height: HEIGHT,
                colour: default_bg,
            },
            elements: vec![],
            audio: vec![],
        }
    }

    pub fn add_shape(&mut self, shape: ShapeSpec) -> Rc<ShapeElement<WIDTH, HEIGHT>> {
        self.add_visual(ShapeElement {
            draw_ordering: RefCell::new(InterpTimeline::new(0.0.into())),
            shape: Box::new(ConstantTimeline::new(shape)),
            origin: RefCell::new(InterpTimeline::new((0.0, 0.0))),
            position: RefCell::new(InterpTimeline::new(Rect::fullscreen().center())),
            scale: RefCell::new(InterpTimeline::new(Length::from_units(0.5 * SCREEN_UNITS))),
            fill_colour: RefCell::new(InterpTimeline::new(ColourRgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            })),
            boundary_thickness: RefCell::new(InterpTimeline::new(Length::from_units(
                0.001 * SCREEN_UNITS,
            ))),
            boundary_colour: RefCell::new(InterpTimeline::new(ColourRgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            })),
            boundary_frac: RefCell::new(InterpTimeline::new((0.0, 1.0))),
            boundary_mode: RefCell::new(InterpTimeline::new(BoundaryMode::Middle)),
        })
    }

    pub fn add_subanimation(
        &mut self,
        at_t: f64,
        source: SubVideoSource,
    ) -> Rc<SubVideoElement<WIDTH, HEIGHT>> {
        let (images, size_ratio): (Box<dyn Timeline<Option<ImageSpec>>>, _) = match source {
            SubVideoSource::Video(video_spec) => {
                let (w, h) = video_spec.size();
                (Box::new(video_spec.image_timeline()), (w as f64, h as f64))
            }
            SubVideoSource::ImageTimeline { size_ratio, images } => (images, size_ratio),
        };
        self.add_visual(SubVideoElement {
            at_t,
            draw_ordering: RefCell::new(InterpTimeline::new(0.0.into())),
            images,
            size_ratio,
            origin: RefCell::new(InterpTimeline::new((0.5, 0.5))),
            position: RefCell::new(InterpTimeline::new(Rect::fullscreen().center())),
            size: RefCell::new(InterpTimeline::new(Vec2::from_units(
                0.5 * SCREEN_UNITS,
                0.5 * SCREEN_UNITS,
            ))),
        })
    }

    fn add_visual<E: AnimationElement<WIDTH, HEIGHT>>(&mut self, element: E) -> Rc<E> {
        let element = Rc::new(element);
        self.elements.push(element.clone());
        element
    }

    pub fn add_audio(&mut self, t: f64, audio: AudioSpec) {
        self.audio.push(AnimationAudioClip { at_t: t, audio });
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> Timeline<ImageSpec> for Animation<WIDTH, HEIGHT> {
    fn at_time(&self, t: f64) -> ImageSpec {
        let mut image_spec = self.default_image.clone();
        let mut elements = self.elements.clone();
        elements.sort_by_cached_key(|elem| elem.get_draw_ordering(t));
        for element in elements {
            image_spec = element.apply(t, image_spec);
        }
        image_spec
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> Animation<WIDTH, HEIGHT> {
    pub fn video(&self, from_t: f64, to_t: f64, fps: f64) -> VideoSpec {
        assert!(from_t <= to_t);
        let dt = 1.0 / fps;
        let mut images = vec![];
        let mut t = from_t;
        while t <= to_t {
            images.push(self.at_time(t));
            t += dt;
        }
        VideoSpec::Compiled(VideoCompiledSpec {
            width: WIDTH,
            height: HEIGHT,
            fps,
            images,
            audio: self
                .audio
                .iter()
                .map(|AnimationAudioClip { at_t, audio }| VideoAudioClip {
                    at_t: at_t - from_t,
                    spec: audio.clone(),
                })
                .collect(),
        })
    }
}
