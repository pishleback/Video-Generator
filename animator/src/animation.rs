use crate::{
    audio::AudioSpec,
    colour::{ColourRgb, ColourRgba},
    coords::{Pos2, SCREEN_UNITS, Vec2},
    image::ImageSpec,
    interpolation::Interp,
    shape::ShapeSpec,
    timeline::{ConstantTimeline, InterpTimeline, Timeline},
    video::{VideoAudioClip, VideoCompiledSpec, VideoSpec},
};
use core::f64;
use std::fmt::Debug;
use std::{cell::RefCell, rc::Rc};

pub trait AnimationElement<const WIDTH: u32, const HEIGHT: u32> {
    fn apply(&self, t: f64, image_spec: ImageSpec) -> ImageSpec;
}

#[derive(Debug)]
pub struct ShapeElementBuilder<const WIDTH: u32, const HEIGHT: u32> {
    origin: (f64, f64),
    position: Pos2<WIDTH, HEIGHT>,
    scale: f64,
    fill_colour: ColourRgba,
    boundary_thickness: f64,
    boundary_colour: ColourRgba,
    boundary_frac: (f64, f64),
    boundary_mode: BoundaryMode,
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElementBuilder<WIDTH, HEIGHT> {
    pub fn new(fill_colour: ColourRgba, boundary_colour: ColourRgba) -> Self {
        Self {
            origin: (0.0, 0.0),
            position: Pos2::new(0.5 * SCREEN_UNITS, 0.5 * SCREEN_UNITS),
            scale: 50.0,
            fill_colour,
            boundary_thickness: 0.05,
            boundary_colour,
            boundary_frac: (0.0, 1.0),
            boundary_mode: BoundaryMode::Middle,
        }
    }

    pub fn origin(mut self, origin: (f64, f64)) -> Self {
        self.origin = origin;
        self
    }

    pub fn position(mut self, position: Pos2<WIDTH, HEIGHT>) -> Self {
        self.position = position;
        self
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn boundary_thickness(mut self, thickness: f64) -> Self {
        self.boundary_thickness = thickness;
        self
    }

    pub fn boundary_frac(mut self, boundary_frac: (f64, f64)) -> Self {
        self.boundary_frac = boundary_frac;
        self
    }

    pub fn boundary_mode(mut self, boundary_mode: BoundaryMode) -> Self {
        self.boundary_mode = boundary_mode;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BoundaryMode {
    Inner,
    Middle,
    Outer,
}

pub struct ShapeElement<const WIDTH: u32, const HEIGHT: u32> {
    shape: Box<dyn Timeline<ShapeSpec>>,
    origin: RefCell<InterpTimeline<(f64, f64)>>, // in shape coordinates where is the center
    position: RefCell<InterpTimeline<Pos2<WIDTH, HEIGHT>>>, // as parts per thousand of the view
    scale: RefCell<InterpTimeline<f64>>,
    fill_colour: RefCell<InterpTimeline<ColourRgba>>,
    boundary_thickness: RefCell<InterpTimeline<f64>>,
    boundary_colour: RefCell<InterpTimeline<ColourRgba>>,
    boundary_frac: RefCell<InterpTimeline<(f64, f64)>>,
    boundary_mode: RefCell<InterpTimeline<BoundaryMode>>,
}

impl<const WIDTH: u32, const HEIGHT: u32> ShapeElement<WIDTH, HEIGHT> {
    pub fn new(shape: ShapeSpec, params: ShapeElementBuilder<WIDTH, HEIGHT>) -> Self {
        Self {
            shape: Box::new(ConstantTimeline::new(shape)),
            origin: RefCell::new(InterpTimeline::new(params.origin)),
            position: RefCell::new(InterpTimeline::new(params.position)),
            scale: RefCell::new(InterpTimeline::new(params.scale)),
            fill_colour: RefCell::new(InterpTimeline::new(params.fill_colour)),
            boundary_thickness: RefCell::new(InterpTimeline::new(params.boundary_thickness)),
            boundary_colour: RefCell::new(InterpTimeline::new(params.boundary_colour)),
            boundary_frac: RefCell::new(InterpTimeline::new(params.boundary_frac)),
            boundary_mode: RefCell::new(InterpTimeline::new(params.boundary_mode)),
        }
    }

    pub fn set_origin(&self, t: f64, v: (f64, f64), interp: impl Interp<(f64, f64)> + 'static) {
        self.origin.borrow_mut().set(t, v, interp);
    }

    pub fn set_position(
        &self,
        t: f64,
        v: Pos2<WIDTH, HEIGHT>,
        interp: impl Interp<Pos2<WIDTH, HEIGHT>> + 'static,
    ) {
        self.position.borrow_mut().set(t, v, interp);
    }

    pub fn set_scale(&self, t: f64, v: f64, interp: impl Interp<f64> + 'static) {
        self.scale.borrow_mut().set(t, v, interp);
    }

    pub fn set_fill_rgba(&self, t: f64, v: ColourRgba, interp: impl Interp<ColourRgba> + 'static) {
        self.fill_colour.borrow_mut().set(t, v, interp);
    }

    pub fn set_fill_rgb(
        &self,
        t: f64,
        ColourRgb { r, g, b }: ColourRgb,
        interp: impl Interp<ColourRgba> + 'static,
    ) {
        let a = self.fill_colour.borrow().at_time(t).a;
        self.fill_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a }, interp);
    }

    pub fn set_fill_alpha(&self, t: f64, alpha: f64, interp: impl Interp<ColourRgba> + 'static) {
        let ColourRgba { r, g, b, .. } = self.fill_colour.borrow().at_time(t);
        self.fill_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a: alpha }, interp);
    }

    pub fn set_boundary_thickness(&self, t: f64, v: f64, interp: impl Interp<f64> + 'static) {
        self.boundary_thickness.borrow_mut().set(t, v, interp);
    }

    pub fn set_boundary_rgba(
        &self,
        t: f64,
        v: ColourRgba,
        interp: impl Interp<ColourRgba> + 'static,
    ) {
        self.boundary_colour.borrow_mut().set(t, v, interp);
    }

    pub fn set_boundary_rgb(
        &self,
        t: f64,
        ColourRgb { r, g, b }: ColourRgb,
        interp: impl Interp<ColourRgba> + 'static,
    ) {
        let a = self.boundary_colour.borrow().at_time(t).a;
        self.boundary_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a }, interp);
    }

    pub fn set_boundary_alpha(
        &self,
        t: f64,
        alpha: f64,
        interp: impl Interp<ColourRgba> + 'static,
    ) {
        let ColourRgba { r, g, b, .. } = self.boundary_colour.borrow().at_time(t);
        self.boundary_colour
            .borrow_mut()
            .set(t, ColourRgba { r, g, b, a: alpha }, interp);
    }

    pub fn set_boundary_frac(
        &self,
        t: f64,
        v: (f64, f64),
        interp: impl Interp<(f64, f64)> + 'static,
    ) {
        self.boundary_frac.borrow_mut().set(t, v, interp);
    }

    pub fn set_boundary_mode(&self, t: f64, v: BoundaryMode) {
        self.boundary_mode.borrow_mut().set_immediate(t, v);
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for ShapeElement<WIDTH, HEIGHT>
{
    fn apply(&self, t: f64, image_spec: ImageSpec) -> ImageSpec {
        let (origin_x, origin_y) = self.origin.borrow().at_time(t);
        let (position_x, position_y) = self.position.borrow().at_time(t).pixels();
        let scale = self.scale.borrow().at_time(t);
        let px_mul = (((WIDTH as u64) * (HEIGHT as u64)) as f64).sqrt();

        let shape = self
            .shape
            .at_time(t)
            .translate(-origin_x, -origin_y)
            .scale(scale * px_mul / SCREEN_UNITS)
            .translate(position_x, position_y);

        let boundary_thickness =
            px_mul * self.boundary_thickness.borrow().at_time(t) / SCREEN_UNITS;
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
}

pub enum SizeConstraint {
    // set width to this and scale height to preserve aspect ratio
    Width(f64),
    // set height to this and scale width to preserve aspect ratio
    Height(f64),
    // set width and height to these, not preserving aspect ratio
    Size(f64, f64),
}

pub enum SubVideoSource {
    Video(VideoSpec),
    ImageTimeline {
        size_ratio: (f64, f64),
        images: Box<dyn Timeline<Option<ImageSpec>>>,
    },
}

pub struct SubVideoElementBuilder<const WIDTH: u32, const HEIGHT: u32> {
    origin: (f64, f64),
    position: Pos2<WIDTH, HEIGHT>,
    source: SubVideoSource,
    size: SizeConstraint,
}

impl<const WIDTH: u32, const HEIGHT: u32> SubVideoElementBuilder<WIDTH, HEIGHT> {
    pub fn new(source: SubVideoSource, size: SizeConstraint) -> Self {
        Self {
            origin: (0.5, 0.5),
            position: Pos2::new(0.5 * SCREEN_UNITS, 0.5 * SCREEN_UNITS),
            source,
            size,
        }
    }

    pub fn origin(mut self, origin: (f64, f64)) -> Self {
        self.origin = origin;
        self
    }

    pub fn position(mut self, position: Pos2<WIDTH, HEIGHT>) -> Self {
        self.position = position;
        self
    }

    pub fn size(mut self, size: SizeConstraint) -> Self {
        self.size = size;
        self
    }
}

pub struct SubVideoElement<const WIDTH: u32, const HEIGHT: u32> {
    at_t: f64,
    images: Box<dyn Timeline<Option<ImageSpec>>>,
    size_ratio: (f64, f64),
    origin: RefCell<InterpTimeline<(f64, f64)>>,
    position: RefCell<InterpTimeline<Pos2<WIDTH, HEIGHT>>>,
    size: RefCell<InterpTimeline<Vec2<WIDTH, HEIGHT>>>,
}

impl<const WIDTH: u32, const HEIGHT: u32> SubVideoElement<WIDTH, HEIGHT> {
    pub fn new(at_t: f64, builder: SubVideoElementBuilder<WIDTH, HEIGHT>) -> Self {
        let (images, size_ratio): (Box<dyn Timeline<Option<ImageSpec>>>, _) = match builder.source {
            SubVideoSource::Video(video_spec) => {
                let (w, h) = video_spec.size();
                (Box::new(video_spec.image_timeline()), (w as f64, h as f64))
            }
            SubVideoSource::ImageTimeline { size_ratio, images } => (images, size_ratio),
        };
        Self {
            at_t,
            images,
            size_ratio,
            origin: RefCell::new(InterpTimeline::new(builder.origin)),
            position: RefCell::new(InterpTimeline::new(builder.position)),
            size: RefCell::new(InterpTimeline::new(match builder.size {
                SizeConstraint::Width(width) => Vec2::from_x_and_slope(width, size_ratio),
                SizeConstraint::Height(height) => Vec2::from_y_and_slope(height, size_ratio),
                SizeConstraint::Size(width, height) => Vec2::new(width, height),
            })),
        }
    }

    pub fn set_origin(&self, t: f64, v: (f64, f64), interp: impl Interp<(f64, f64)> + 'static) {
        self.origin.borrow_mut().set(t, v, interp);
    }

    pub fn set_position(
        &self,
        t: f64,
        v: Pos2<WIDTH, HEIGHT>,
        interp: impl Interp<Pos2<WIDTH, HEIGHT>> + 'static,
    ) {
        self.position.borrow_mut().set(t, v, interp);
    }

    pub fn set_width(
        &self,
        t: f64,
        width: f64,
        interp: impl Interp<Vec2<WIDTH, HEIGHT>> + 'static,
    ) {
        self.size
            .borrow_mut()
            .set(t, Vec2::from_x_and_slope(width, self.size_ratio), interp);
    }

    pub fn set_height(
        &self,
        t: f64,
        height: f64,
        interp: impl Interp<Vec2<WIDTH, HEIGHT>> + 'static,
    ) {
        self.size
            .borrow_mut()
            .set(t, Vec2::from_y_and_slope(height, self.size_ratio), interp);
    }

    pub fn set_size(
        &self,
        t: f64,
        v: Vec2<WIDTH, HEIGHT>,
        interp: impl Interp<Vec2<WIDTH, HEIGHT>> + 'static,
    ) {
        self.size.borrow_mut().set(t, v, interp);
    }
}

impl<const WIDTH: u32, const HEIGHT: u32> AnimationElement<WIDTH, HEIGHT>
    for SubVideoElement<WIDTH, HEIGHT>
{
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

    pub fn add_visual<E: AnimationElement<WIDTH, HEIGHT> + 'static>(
        &mut self,
        element: E,
    ) -> Rc<E> {
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
        for element in &self.elements {
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
