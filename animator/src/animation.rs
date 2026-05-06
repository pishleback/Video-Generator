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
    shape: Rc<RefCell<dyn Timeline<ShapeSpec>>>,
    origin: RefCell<InterpTimeline<(f64, f64)>>, // in shape coordinates where is the center
    position: RefCell<InterpTimeline<Pos2<WIDTH, HEIGHT>>>, // as parts per thousand of the view
    scale: RefCell<InterpTimeline<Length<WIDTH, HEIGHT>>>,
    fill_rgb: RefCell<InterpTimeline<ColourRgb>>,
    fill_alpha: RefCell<InterpTimeline<f64>>,
    boundary_thickness: RefCell<InterpTimeline<Length<WIDTH, HEIGHT>>>,
    boundary_rgb: RefCell<InterpTimeline<ColourRgb>>,
    boundary_alpha: RefCell<InterpTimeline<f64>>,
    boundary_frac_1: RefCell<InterpTimeline<f64>>,
    boundary_frac_2: RefCell<InterpTimeline<f64>>,
    boundary_mode: RefCell<InterpTimeline<BoundaryMode>>,
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElement<WIDTH, HEIGHT> {
    pub fn set_draw_ordering(
        &self,
        t: f64,
        from: Option<OrderedFloat<f64>>,
        to: OrderedFloat<f64>,
    ) {
        self.draw_ordering.borrow_mut().set_immediate(t, from, to);
    }

    pub fn set_origin(&self, t: f64, from: Option<(f64, f64)>, to: (f64, f64), interp: InterpType) {
        self.origin.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_position(
        &self,
        t: f64,
        from: Option<Pos2<WIDTH, HEIGHT>>,
        to: Pos2<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.position.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_scale(
        &self,
        t: f64,
        from: Option<Length<WIDTH, HEIGHT>>,
        to: Length<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.scale.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_fill_rgb(&self, t: f64, from: Option<ColourRgb>, to: ColourRgb, interp: InterpType) {
        self.fill_rgb.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_fill_alpha(&self, t: f64, from: Option<f64>, to: f64, interp: InterpType) {
        self.fill_alpha.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_boundary_thickness(
        &self,
        t: f64,
        from: Option<Length<WIDTH, HEIGHT>>,
        to: Length<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.boundary_thickness
            .borrow_mut()
            .set(t, from, to, interp);
    }

    pub fn set_boundary_rgb(
        &self,
        t: f64,
        from: Option<ColourRgb>,
        to: ColourRgb,
        interp: InterpType,
    ) {
        self.boundary_rgb.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_boundary_alpha(&self, t: f64, from: Option<f64>, to: f64, interp: InterpType) {
        self.boundary_alpha.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_boundary_frac(
        &self,
        t: f64,
        from: Option<(f64, f64)>,
        to: (f64, f64),
        interp: InterpType,
    ) {
        self.boundary_frac_1
            .borrow_mut()
            .set(t, from.map(|from| from.0), to.0, interp);
        self.boundary_frac_2
            .borrow_mut()
            .set(t, from.map(|from| from.1), to.1, interp);
    }

    pub fn set_boundary_frac_1(&self, t: f64, from: Option<f64>, to: f64, interp: InterpType) {
        self.boundary_frac_1.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_boundary_frac_2(&self, t: f64, from: Option<f64>, to: f64, interp: InterpType) {
        self.boundary_frac_2.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_boundary_mode(&self, t: f64, from: Option<BoundaryMode>, to: BoundaryMode) {
        self.boundary_mode.borrow_mut().set_immediate(t, from, to);
    }

    fn bounding_rect(&self, t: f64) -> Option<geo::Rect> {
        let shape = self.shape.borrow().at_time(t).shape();
        shape.bounding_rect()
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for ShapeElement<WIDTH, HEIGHT>
{
    fn get_draw_ordering(&self, t: f64) -> (OrderedFloat<f64>, OrderedFloat<f64>) {
        // base depth on brightness
        let ColourRgb { r, g, b } = self.fill_rgb.borrow().at_time(t);
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
            .borrow()
            .at_time(t)
            .translate(-origin_x, -origin_y)
            .scale(scale.pixels())
            .translate(position_x, position_y);

        let boundary_thickness = self.boundary_thickness.borrow().at_time(t).pixels();
        let frac_interval = (
            self.boundary_frac_1.borrow().at_time(t),
            self.boundary_frac_2.borrow().at_time(t),
        );
        let shape_boundary = match self.boundary_mode.borrow().at_time(t) {
            BoundaryMode::Inner => shape
                .partial_boundary(2.0 * boundary_thickness, frac_interval)
                .intersect(&shape),
            BoundaryMode::Middle => shape.partial_boundary(boundary_thickness, frac_interval),
            BoundaryMode::Outer => shape
                .partial_boundary(2.0 * boundary_thickness, frac_interval)
                .subtract(&shape),
        };

        let fill_rgb = self.fill_rgb.borrow().at_time(t);
        let fill_alpha = self.fill_alpha.borrow().at_time(t);
        let boundary_rgb = self.boundary_rgb.borrow().at_time(t);
        let boundary_alpha = self.boundary_alpha.borrow().at_time(t);

        if fill_alpha == 0.0 && (boundary_alpha == 0.0 || boundary_thickness == 0.0) {
            return image_spec;
        }

        let mut layers = vec![];
        if fill_alpha != 0.0 {
            layers.push(((0.0, 0.0), {
                shape.image(
                    WIDTH,
                    HEIGHT,
                    ColourRgba {
                        r: fill_rgb.r,
                        g: fill_rgb.g,
                        b: fill_rgb.b,
                        a: 0.0,
                    },
                    ColourRgba {
                        r: fill_rgb.r,
                        g: fill_rgb.g,
                        b: fill_rgb.b,
                        a: fill_alpha,
                    },
                )
            }));
        }
        if boundary_alpha != 0.0 && boundary_thickness != 0.0 {
            layers.push(((0.0, 0.0), {
                shape_boundary.image(
                    WIDTH,
                    HEIGHT,
                    ColourRgba {
                        r: boundary_rgb.r,
                        g: boundary_rgb.g,
                        b: boundary_rgb.b,
                        a: 0.0,
                    },
                    ColourRgba {
                        r: boundary_rgb.r,
                        g: boundary_rgb.g,
                        b: boundary_rgb.b,
                        a: boundary_alpha,
                    },
                )
            }));
        }
        ImageSpec::BlitStack {
            base: Box::new(image_spec),
            layers,
        }
    }

    fn set_within_rect(&self, t: f64, rect: Rect<WIDTH, HEIGHT>, interp: InterpType) {
        let shape = self.shape.borrow().at_time(t).shape();
        if let Some(bounding_rect) = shape.bounding_rect() {
            let (br_min_x, br_min_y) = bounding_rect.min().x_y();
            let (br_w, br_h) = (bounding_rect.width(), bounding_rect.height());
            let (r_w, r_h) = rect.size().pixels();
            self.set_origin(
                t,
                None,
                (br_min_x + 0.5 * br_w, br_min_y + 0.5 * br_h),
                interp,
            );
            self.set_position(t, None, rect.center(), interp);
            self.set_scale(
                t,
                None,
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
    pub fn set_origin(&self, t: f64, from: Option<(f64, f64)>, to: (f64, f64), interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_origin(t, from, to, interp);
        }
    }

    pub fn set_position(
        &self,
        t: f64,
        from: Option<Pos2<WIDTH, HEIGHT>>,
        to: Pos2<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        for elem in &self.shape_elements {
            elem.set_position(t, from, to, interp);
        }
    }

    pub fn set_scale(
        &self,
        t: f64,
        from: Option<Length<WIDTH, HEIGHT>>,
        to: Length<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        for elem in &self.shape_elements {
            elem.set_scale(t, from, to, interp);
        }
    }

    pub fn set_fill_rgb(&self, t: f64, from: Option<ColourRgb>, to: ColourRgb, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_fill_rgb(t, from, to, interp);
        }
    }

    pub fn set_fill_alpha(&self, t: f64, from: Option<f64>, to: f64, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_fill_alpha(t, from, to, interp);
        }
    }

    pub fn set_boundary_thickness(
        &self,
        t: f64,
        from: Option<Length<WIDTH, HEIGHT>>,
        to: Length<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        for elem in &self.shape_elements {
            elem.set_boundary_thickness(t, from, to, interp);
        }
    }

    pub fn set_boundary_rgb(
        &self,
        t: f64,
        from: Option<ColourRgb>,
        to: ColourRgb,
        interp: InterpType,
    ) {
        for elem in &self.shape_elements {
            elem.set_boundary_rgb(t, from, to, interp);
        }
    }

    pub fn set_boundary_alpha(&self, t: f64, from: Option<f64>, to: f64, interp: InterpType) {
        for elem in &self.shape_elements {
            elem.set_boundary_alpha(t, from, to, interp);
        }
    }

    pub fn set_boundary_frac(
        &self,
        t: f64,
        from: Option<(f64, f64)>,
        to: (f64, f64),
        interp: InterpType,
    ) {
        for elem in &self.shape_elements {
            elem.set_boundary_frac(t, from, to, interp);
        }
    }

    pub fn set_boundary_mode(&self, t: f64, from: Option<BoundaryMode>, to: BoundaryMode) {
        for elem in &self.shape_elements {
            elem.set_boundary_mode(t, from, to);
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
            self.set_origin(
                t,
                None,
                (br_min_x + 0.5 * br_w, br_min_y + 0.5 * br_h),
                interp,
            );
            self.set_position(t, None, rect.center(), interp);
            self.set_scale(
                t,
                None,
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
    pub fn set_draw_ordering(
        &self,
        t: f64,
        from: Option<OrderedFloat<f64>>,
        to: OrderedFloat<f64>,
    ) {
        self.draw_ordering.borrow_mut().set_immediate(t, from, to);
    }

    pub fn set_origin(&self, t: f64, from: Option<(f64, f64)>, to: (f64, f64), interp: InterpType) {
        self.origin.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_position(
        &self,
        t: f64,
        from: Option<Pos2<WIDTH, HEIGHT>>,
        to: Pos2<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.position.borrow_mut().set(t, from, to, interp);
    }

    pub fn set_width(
        &self,
        t: f64,
        from: Option<Length<WIDTH, HEIGHT>>,
        to: Length<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.size.borrow_mut().set(
            t,
            from.map(|from| Vec2::from_x_and_slope(from, self.size_ratio)),
            Vec2::from_x_and_slope(to, self.size_ratio),
            interp,
        );
    }

    pub fn set_height(
        &self,
        t: f64,
        from: Option<Length<WIDTH, HEIGHT>>,
        to: Length<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.size.borrow_mut().set(
            t,
            from.map(|from| Vec2::from_y_and_slope(from, self.size_ratio)),
            Vec2::from_y_and_slope(to, self.size_ratio),
            interp,
        );
    }

    pub fn set_size(
        &self,
        t: f64,
        from: Option<Vec2<WIDTH, HEIGHT>>,
        to: Vec2<WIDTH, HEIGHT>,
        interp: InterpType,
    ) {
        self.size.borrow_mut().set(t, from, to, interp);
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
                base: Box::new(image_spec),
                layers: vec![(
                    (
                        position_x - origin_x * size_w,
                        position_y - origin_y * size_h,
                    ),
                    ImageSpec::Resize {
                        image: Box::new(img),
                        width: size_w as u32,
                        height: size_h as u32,
                    },
                )],
            }
        } else {
            image_spec
        }
    }

    fn set_within_rect(&self, t: f64, rect: Rect<WIDTH, HEIGHT>, interp: InterpType) {
        self.set_origin(t, None, (0.5, 0.5), interp);
        self.set_position(t, None, rect.center(), interp);
        let (rw, rh) = rect.size().to_lengths();
        let (sw, sh) = self.size_ratio;
        if rw * sh < rh * sw {
            self.set_width(t, None, rw, interp);
        } else {
            self.set_height(t, None, rh, interp);
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

    pub fn add_shape_timeline(
        &mut self,
        shape: Rc<RefCell<dyn Timeline<ShapeSpec>>>,
    ) -> Rc<ShapeElement<WIDTH, HEIGHT>> {
        self.add_visual(ShapeElement {
            draw_ordering: RefCell::new(InterpTimeline::new(0.0.into())),
            shape,
            origin: RefCell::new(InterpTimeline::new((0.0, 0.0))),
            position: RefCell::new(InterpTimeline::new(Rect::fullscreen().center())),
            scale: RefCell::new(InterpTimeline::new(Length::from_units(SCREEN_UNITS / 2.0))),
            fill_rgb: RefCell::new(InterpTimeline::new(ColourRgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            })),
            fill_alpha: RefCell::new(InterpTimeline::new(0.0)),
            boundary_thickness: RefCell::new(InterpTimeline::new(Length::from_units(
                0.001 * SCREEN_UNITS,
            ))),
            boundary_rgb: RefCell::new(InterpTimeline::new(ColourRgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            })),
            boundary_alpha: RefCell::new(InterpTimeline::new(0.0)),
            boundary_frac_1: RefCell::new(InterpTimeline::new(0.0)),
            boundary_frac_2: RefCell::new(InterpTimeline::new(1.0)),
            boundary_mode: RefCell::new(InterpTimeline::new(BoundaryMode::Middle)),
        })
    }

    pub fn add_shape(&mut self, shape: ShapeSpec) -> Rc<ShapeElement<WIDTH, HEIGHT>> {
        self.add_shape_timeline(Rc::new(RefCell::new(ConstantTimeline::new(shape))))
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
