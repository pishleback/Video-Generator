use crate::colour::srgb_to_linear;
use crate::data::{FileSpec, cache};
use crate::image::LatexImage;
use crate::{colour::ColourWithAlpha, image::ImageSpec};
use geo::{
    AffineOps, AffineTransform, Area, BooleanOps, BoundingRect, Buffer, Coord, Distance, Euclidean,
    HasDimensions, Line, LineString, MakeValid, MultiLineString, MultiPolygon, Point, Polygon,
    SimplifyVwPreserve, Translate, Validation,
};
use image::Rgba32FImage;
use serde::{Deserialize, Serialize};
use std::path::Path;

const EPSILON: f64 = 10e-10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShapeSpec {
    Empty,
    Circle {
        center: (f64, f64),
        radius: f64,
    },
    Line {
        point1: (f64, f64),
        point2: (f64, f64),
        radius: f64,
    },
    Scale {
        shape: Box<ShapeSpec>,
        scale_factor: f64,
    },
    Translate {
        shape: Box<ShapeSpec>,
        x_offset: f64,
        y_offset: f64,
    },
    Normalize {
        shape: Box<ShapeSpec>,
    },
    PartialBoundary {
        shape: Box<ShapeSpec>,
        radius: f64,
        start_frac: f64,
        end_frac: f64,
    },
    Buffer {
        shape: Box<ShapeSpec>,
        radius: f64,
    },
    Union {
        shape1: Box<ShapeSpec>,
        shape2: Box<ShapeSpec>,
    },
    Intersect {
        shape1: Box<ShapeSpec>,
        shape2: Box<ShapeSpec>,
    },
    Subtract {
        target: Box<ShapeSpec>,
        tool: Box<ShapeSpec>,
    },
    FromImage(FromImageShape),
}

impl ShapeSpec {
    pub fn latex(expr: String) -> Self {
        let pt = 10.0; // default Tex text size in pt
        let dpi = 2048.0; // dpi used for rasterization
        let inch_in_pt = 72.27; // 1 inch = 72.27 pt in Tex https://en.wikipedia.org/wiki/Point_(typography)
        let scale_height_factor = inch_in_pt / (pt * dpi);
        ShapeSpec::FromImage(FromImageShape {
            image: ImageSpec::Latex(LatexImage { dpi, expr }),
        })
        .scale(scale_height_factor)
    }
}

impl ShapeSpec {
    pub fn make_shape(&self, path: &Path) {
        match self {
            ShapeSpec::FromImage(x) => x.make_shape(path),
            _ => {
                std::fs::write(path, serde_json::to_string_pretty(&self.shape()).unwrap()).unwrap()
            }
        }
    }

    pub(crate) fn shape(&self) -> ShapeData {
        match self {
            ShapeSpec::Empty => ShapeData {
                multipolygon: MultiPolygon(vec![]),
                invisible: MultiPolygon(vec![]),
            },
            ShapeSpec::Circle {
                center: (x, y),
                radius,
            } => ShapeData::new(Point::new(*x, *y).buffer(radius.max(EPSILON))),
            ShapeSpec::Line {
                point1: (x1, y1),
                point2: (x2, y2),
                radius,
            } => ShapeData::new(
                Line::new(Point::new(*x1, *y1), Point::new(*x2, *y2)).buffer(radius.max(EPSILON)),
            ),
            ShapeSpec::Scale {
                shape,
                scale_factor,
            } => {
                debug_assert_ne!(scale_factor, &0.0);
                shape.shape().scale(*scale_factor)
            }
            ShapeSpec::Translate {
                shape,
                x_offset,
                y_offset,
            } => shape.shape().translate(*x_offset, *y_offset),
            ShapeSpec::Normalize { shape } => shape.shape().normalize(),
            ShapeSpec::PartialBoundary {
                shape,
                start_frac,
                end_frac,
                radius,
            } => shape
                .shape()
                .partial_boundary(*radius, *start_frac, *end_frac),
            ShapeSpec::Buffer { shape, radius } => shape.shape().buffer(*radius),
            ShapeSpec::Union { shape1, shape2 } => shape1.shape().union(&shape2.shape()),
            ShapeSpec::Intersect { shape1, shape2 } => shape1.shape().intersect(&shape2.shape()),
            ShapeSpec::Subtract { target, tool } => target.shape().subtract(&tool.shape()),
            ShapeSpec::FromImage(_) => {
                let shape_path = cache().make_file(&FileSpec::Shape(self.clone()));
                serde_json::from_str(std::fs::read_to_string(shape_path).unwrap().as_str()).unwrap()
            }
        }
    }

    pub fn scale(&self, scale_factor: f64) -> Self {
        Self::Scale {
            shape: Box::new(self.clone()),
            scale_factor,
        }
    }

    pub fn translate(&self, x_offset: f64, y_offset: f64) -> Self {
        Self::Translate {
            shape: Box::new(self.clone()),
            x_offset,
            y_offset,
        }
    }

    pub fn normalize(&self) -> Self {
        Self::Normalize {
            shape: Box::new(self.clone()),
        }
    }

    pub fn boundary(&self, radius: f64) -> Self {
        Self::PartialBoundary {
            shape: Box::new(self.clone()),
            radius,
            start_frac: 0.0,
            end_frac: 1.0,
        }
    }

    pub fn partial_boundary(&self, radius: f64, frac_interval: (f64, f64)) -> Self {
        Self::PartialBoundary {
            shape: Box::new(self.clone()),
            radius,
            start_frac: frac_interval.0,
            end_frac: frac_interval.1,
        }
    }

    pub fn buffer(&self, radius: f64) -> Self {
        Self::Buffer {
            shape: Box::new(self.clone()),
            radius,
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        Self::Union {
            shape1: Box::new(self.clone()),
            shape2: Box::new(other.clone()),
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        Self::Intersect {
            shape1: Box::new(self.clone()),
            shape2: Box::new(other.clone()),
        }
    }

    pub fn subtract(&self, other: &Self) -> Self {
        Self::Subtract {
            target: Box::new(self.clone()),
            tool: Box::new(other.clone()),
        }
    }

    pub fn image(
        &self,
        width: u32,
        height: u32,
        bg_colour: ColourWithAlpha,
        shape_colour: ColourWithAlpha,
    ) -> ImageSpec {
        ImageSpec::Shape(ShapeImage {
            width,
            height,
            bg_colour,
            shape_colour,
            shape: Box::new(self.clone()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromImageShape {
    pub image: ImageSpec,
}

impl FromImageShape {
    pub fn make_shape(&self, shape_path: &Path) {
        fn image_to_multipolygon(img: image::DynamicImage) -> MultiPolygon<f64> {
            use marching_squares::{Field, march};

            #[derive(Debug)]
            struct HeightMap {
                img: image::GrayImage,
                invert: bool,
            }

            impl Field for HeightMap {
                fn dimensions(&self) -> (usize, usize) {
                    let (w, h) = self.img.dimensions();
                    (w as usize, h as usize)
                }

                fn z_at(&self, x: usize, y: usize) -> f64 {
                    let mut z = self.img.get_pixel(x as u32, y as u32).0[0] as f64 / 255.0;
                    if self.invert {
                        z = 1.0 - z;
                    }
                    z
                }
            }

            let heightmap = HeightMap {
                img: img.to_luma8(),
                invert: true,
            };

            let contours = march(&heightmap.framed(0.0), srgb_to_linear(0.5) as f64);

            let mut contour_linestrings: Vec<LineString<f64>> = Vec::new();
            for c in &contours {
                let ls = LineString(c.iter().map(|p| Coord { x: p.0, y: p.1 }).collect());
                if ls.0.len() < 3 {
                    continue;
                }
                contour_linestrings.push(ls);
            }

            // Build polygons by assigning holes to the correct outer
            let mut polygon = MultiPolygon::empty();
            for linestring in contour_linestrings {
                polygon = polygon.xor(
                    &Polygon::new(linestring.clone(), vec![])
                        .make_valid()
                        .unwrap(),
                );
            }
            polygon.make_valid().unwrap().simplify_vw_preserve(1.0)
        }

        let image = self.image.image();
        let (w, h) = image.dimensions();
        let (w, h) = (w as f64, h as f64);
        let invisible = MultiPolygon(vec![Polygon::new(
            LineString::from(vec![(0.0, 0.0), (w, 0.0), (w, h), (0.0, h), (0.0, 0.0)]),
            vec![],
        )]);
        assert!(invisible.is_valid());

        let shape = ShapeData {
            multipolygon: image_to_multipolygon(image.into()),
            invisible,
        };

        std::fs::write(shape_path, serde_json::to_string_pretty(&shape).unwrap()).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeData {
    multipolygon: MultiPolygon,
    invisible: MultiPolygon, // not part of the shape but is used to determine the size of the shape
}

impl ShapeData {
    pub fn new(multipolygon: MultiPolygon) -> Self {
        for poly in &multipolygon.0 {
            for line in vec![poly.exterior()].into_iter().chain(poly.interiors()) {
                for point in line {
                    debug_assert!(!point.x.is_nan());
                    debug_assert!(!point.y.is_nan());
                }
            }
        }

        let es = multipolygon.validation_errors();
        for e in &es {
            println!("Invalid geometry: {}", e);
        }
        assert!(es.is_empty());
        assert!(multipolygon.is_valid());

        Self {
            multipolygon,
            invisible: MultiPolygon(vec![]),
        }
    }

    pub fn bounding_rect(&self) -> Option<geo::Rect> {
        vec![
            self.multipolygon.bounding_rect(),
            self.invisible.bounding_rect(),
        ]
        .into_iter()
        .flatten()
        .reduce(|a, b| {
            geo::Rect::new(
                (a.min().x.min(b.min().x), a.min().y.min(b.min().y)),
                (a.max().x.max(b.max().x), a.max().y.max(b.max().y)),
            )
        })
    }

    pub fn scale(self, scale_factor: f64) -> Self {
        debug_assert!(!scale_factor.is_nan());
        debug_assert_ne!(scale_factor, 0.0);
        debug_assert!(scale_factor.is_finite());
        Self {
            multipolygon: self.multipolygon.affine_transform(&AffineTransform::scale(
                scale_factor,
                scale_factor,
                (0.0, 0.0),
            )),
            invisible: self.invisible.affine_transform(&AffineTransform::scale(
                scale_factor,
                scale_factor,
                (0.0, 0.0),
            )),
        }
    }

    pub fn translate(self, x_offset: f64, y_offset: f64) -> Self {
        debug_assert!(!x_offset.is_nan());
        debug_assert!(!y_offset.is_nan());
        Self {
            multipolygon: self.multipolygon.translate(x_offset, y_offset),
            invisible: self.invisible.translate(x_offset, y_offset),
        }
    }

    pub fn normalize(mut self) -> Self {
        if let Some(rect) = self.bounding_rect() {
            let Coord { x, y } = rect.center();
            self = self.translate(-x, -y);
            let avg_side = rect.unsigned_area().sqrt();
            self = self.scale(1.0 / avg_side);
        } else {
            //empty
        }
        self
    }

    pub fn boundary(self, radius: f64) -> Self {
        Self {
            multipolygon: {
                let mut lines = Vec::new();
                for poly in &self.multipolygon.0 {
                    lines.push(poly.exterior().clone());
                    for inner in poly.interiors() {
                        lines.push(inner.clone());
                    }
                }
                MultiLineString(lines).buffer(radius)
            },
            invisible: self.invisible,
        }
    }

    pub fn partial_boundary(self, radius: f64, start_frac: f64, end_frac: f64) -> Self {
        fn partial_line(line: &LineString<f64>, frac_1: f64, frac_2: f64) -> Vec<LineString<f64>> {
            // we want the shortest path between frac_1 and frac_2 mod 2, then reduce the shortest path mod 1
            let frac_1 = frac_1.rem_euclid(2.0);
            let frac_2 = frac_2.rem_euclid(2.0);

            // sort frac_1 and frac_2
            let (frac_1, frac_2) = if frac_1 <= frac_2 {
                (frac_1, frac_2)
            } else {
                (frac_2, frac_1)
            };

            // sort frac_1 and frac_2 so that the sortest path between them mod 2 goes from frac_1 to frac_2
            let (frac_1, frac_2) = if frac_2 - frac_1 <= 1.0 {
                (frac_1, frac_2)
            } else {
                (frac_2, frac_1)
            };

            // reduce frac_1 and frac_2 mod 1 and set flip so we have either [   1---2   ] [---1   2---] as is required
            let (frac_1, frac_2, flip) = match (frac_1 <= 1.0, frac_2 <= 1.0) {
                (true, true) => (frac_1, frac_2, false),
                (true, false) => (frac_1, frac_2 - 1.0, true),
                (false, true) => (frac_1 - 1.0, frac_2, true),
                (false, false) => (frac_1 - 1.0, frac_2 - 1.0, false),
            };

            let coords = &line.0;

            if coords.len() <= 1 {
                return vec![line.clone()];
            }

            let mut total_len = 0.0;
            for i in 0..coords.len() - 1 {
                total_len += Euclidean.distance(coords[i], coords[i + 1]);
            }
            if total_len == 0.0 {
                return vec![line.clone()];
            }

            let target_1 = frac_1 * total_len;
            let target_2 = frac_2 * total_len;
            let (first_target, second_target) = if target_1 <= target_2 {
                (target_1, target_2)
            } else {
                (target_2, target_1)
            };

            let mut points_1 = vec![];
            let mut points_2 = vec![];
            let mut points_3 = vec![];

            let mut i = 0;
            let mut len_acc = 0.0;
            let mut pts_acc = vec![coords[0]];
            while i < coords.len() - 1 {
                let p = coords[i];
                let q = coords[i + 1];
                let seg_len = Euclidean.distance(p, q);
                if len_acc + seg_len >= first_target {
                    let remaining = first_target - len_acc;
                    let t = if seg_len == 0.0 {
                        0.0
                    } else {
                        remaining / seg_len
                    };
                    let x = p.x + t * (q.x - p.x);
                    let y = p.y + t * (q.y - p.y);
                    let pt = Coord { x, y };
                    if &pt != pts_acc.last().unwrap() {
                        pts_acc.push(pt);
                    }
                    points_1 = pts_acc;
                    pts_acc = vec![pt];
                    break;
                }
                if &q != pts_acc.last().unwrap() {
                    pts_acc.push(q);
                }
                len_acc += seg_len;
                i += 1;
            }

            while i < coords.len() - 1 {
                let p = coords[i];
                let q = coords[i + 1];
                let seg_len = Euclidean.distance(p, q);
                if len_acc + seg_len >= second_target {
                    let remaining = second_target - len_acc;
                    let t = if seg_len == 0.0 {
                        0.0
                    } else {
                        remaining / seg_len
                    };
                    let x = p.x + t * (q.x - p.x);
                    let y = p.y + t * (q.y - p.y);
                    let pt = Coord { x, y };
                    if &pt != pts_acc.last().unwrap() {
                        pts_acc.push(pt);
                    }
                    points_2 = pts_acc;
                    pts_acc = vec![pt];
                    break;
                }
                if &q != pts_acc.last().unwrap() {
                    pts_acc.push(q);
                }
                len_acc += seg_len;
                i += 1;
            }

            while i < coords.len() - 1 {
                let q = coords[i + 1];
                if &q != pts_acc.last().unwrap() {
                    pts_acc.push(q);
                }
                i += 1;
            }
            points_3 = pts_acc;

            let mut line_strings = vec![];
            if !flip {
                if points_2.len() >= 2 {
                    line_strings.push(LineString(points_2));
                }
            } else {
                if !line.is_closed() {
                    for pt in &points_1[1..] {
                        if pt != points_3.last().unwrap() {
                            points_3.push(*pt);
                        }
                    }
                    if points_3.len() >= 2 {
                        line_strings.push(LineString(points_3));
                    }
                } else {
                    if points_1.len() >= 2 {
                        line_strings.push(LineString(points_1));
                    }
                    if points_3.len() >= 2 {
                        line_strings.push(LineString(points_3));
                    }
                }
            }
            for line in &line_strings {
                if !line.validation_errors().is_empty() {
                    panic!();
                }
            }

            line_strings
        }

        Self {
            multipolygon: {
                let mut lines = Vec::new();
                for poly in &self.multipolygon.0 {
                    for partial_line in partial_line(poly.exterior(), start_frac, end_frac) {
                        assert!(partial_line.validation_errors().is_empty());
                        lines.push(partial_line);
                    }
                    for inner in poly.interiors() {
                        for partial_line in partial_line(inner, start_frac, end_frac) {
                            assert!(partial_line.validation_errors().is_empty());
                            lines.push(partial_line);
                        }
                    }
                }
                MultiLineString(lines)
                    .simplify_vw_preserve(0.00000000001)
                    .buffer(radius)
            },
            invisible: self.invisible,
        }
    }

    pub fn buffer(self, distance: f64) -> Self {
        Self::new(self.multipolygon.buffer(distance))
    }

    pub fn union(self, other: &Self) -> Self {
        Self::new(self.multipolygon.union(&other.multipolygon))
    }

    pub fn intersect(self, other: &Self) -> Self {
        Self::new(self.multipolygon.intersection(&other.multipolygon))
    }

    pub fn subtract(self, other: &Self) -> Self {
        Self::new(self.multipolygon.difference(&other.multipolygon))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeImage {
    pub width: u32,
    pub height: u32,
    pub bg_colour: ColourWithAlpha,
    pub shape_colour: ColourWithAlpha,
    pub shape: Box<ShapeSpec>,
}

impl ShapeImage {
    pub fn image(&self) -> Rgba32FImage {
        use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

        let width = self.width;
        let height = self.height;

        let mut pixmap = Pixmap::new(width, height).expect("failed to create pixmap");

        let to_tiny_skia_colour = |colour: ColourWithAlpha| {
            let colour = colour.to_srgb_f32();
            Color::from_rgba(colour[0], colour[1], colour[2], colour[3]).unwrap()
        };

        // background fill
        pixmap.fill(to_tiny_skia_colour(self.bg_colour));

        let shape = self.shape.shape();

        let mut paint = Paint::default();
        paint.set_color(to_tiny_skia_colour(self.shape_colour));

        if !shape.multipolygon.is_empty() {
            let mut pb = PathBuilder::new();

            for poly in &shape.multipolygon {
                // exterior ring
                if let Some((first, rest)) = poly
                    .exterior()
                    .points()
                    .collect::<Vec<_>>()
                    .as_slice()
                    .split_first()
                {
                    pb.move_to(first.x() as f32, first.y() as f32);
                    for p in rest {
                        pb.line_to(p.x() as f32, p.y() as f32);
                    }
                    pb.close();
                }

                // holes
                for hole in poly.interiors() {
                    if let Some((first, rest)) =
                        hole.points().collect::<Vec<_>>().as_slice().split_first()
                    {
                        pb.move_to(first.x() as f32, first.y() as f32);
                        for p in rest {
                            pb.line_to(p.x() as f32, p.y() as f32);
                        }
                        pb.close();
                    }
                }
            }

            let path = pb.finish().expect("invalid path");

            pixmap.fill_path(
                &path,
                &paint,
                FillRule::EvenOdd,
                Transform::identity(),
                None,
            );
        }

        // convert to DynamicImage
        let mut img = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(width, height, pixmap.data().to_vec())
                .expect("conversion failed"),
        )
        .to_rgba32f();

        // tiny-skia outputs pre-multipled alpha
        // our render pipeline assumes straight alpha, so we correct for that now
        for x in 0..img.width() {
            for y in 0..img.height() {
                let p = img.get_pixel_mut(x, y);
                let a = p[3];
                if a > 0.0 {
                    let a_inv = 1.0 / a;
                    p[0] *= a_inv;
                    p[1] *= a_inv;
                    p[2] *= a_inv;
                } else {
                    p[0] = 0.0;
                    p[1] = 0.0;
                    p[2] = 0.0;
                }
            }
        }
        img
    }
}
