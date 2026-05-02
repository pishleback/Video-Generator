use crate::data::{FileSpec, cache};
use crate::{colour::ColourRgba, image::ImageSpec};
use geo::algorithm::contains::Contains;
use geo::{
    AffineOps, AffineTransform, Area, BooleanOps, BoundingRect, Buffer, Coord, Distance, Euclidean,
    LineString, MakeValid, MultiLineString, MultiPolygon, Polygon, Scale, SimplifyVwPreserve,
    Translate, Validation,
};
use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use image::{GrayImage, Luma};
use imageproc::contours::{BorderType, Contour, find_contours};
use imageproc::drawing::draw_polygon_mut;
use serde::{Deserialize, Serialize};
use std::path::Path;

const AA_RESCALE: u32 = 4; // This is a bit bodge and slow... It would be better to draw with AA using a better library

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShapeSpec {
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
    pub fn make_shape(&self, path: &Path) {
        match self {
            ShapeSpec::FromImage(x) => x.make_shape(path),
            ShapeSpec::Scale {
                shape,
                scale_factor,
            } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape.get_shape().scale(*scale_factor)).unwrap(),
            )
            .unwrap(),
            ShapeSpec::Translate {
                shape,
                x_offset,
                y_offset,
            } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape.get_shape().translate(*x_offset, *y_offset))
                    .unwrap(),
            )
            .unwrap(),
            ShapeSpec::Normalize { shape } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape.get_shape().normalize()).unwrap(),
            )
            .unwrap(),
            ShapeSpec::PartialBoundary {
                shape,
                start_frac,
                end_frac,
                radius,
            } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape.get_shape().partial_boundary(
                    *radius,
                    *start_frac,
                    *end_frac,
                ))
                .unwrap(),
            )
            .unwrap(),
            ShapeSpec::Buffer { shape, radius } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape.get_shape().buffer(*radius)).unwrap(),
            )
            .unwrap(),
            ShapeSpec::Union { shape1, shape2 } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape1.get_shape().union(&shape2.get_shape()))
                    .unwrap(),
            )
            .unwrap(),
            ShapeSpec::Intersect { shape1, shape2 } => std::fs::write(
                path,
                serde_json::to_string_pretty(&shape1.get_shape().intersect(&shape2.get_shape()))
                    .unwrap(),
            )
            .unwrap(),
            Self::Subtract { target, tool } => std::fs::write(
                path,
                serde_json::to_string_pretty(&target.get_shape().subtract(&tool.get_shape()))
                    .unwrap(),
            )
            .unwrap(),
        }
    }

    pub fn get_shape(&self) -> ShapeData {
        let shape_path = cache().get_file(&FileSpec::Shape(self.clone()));
        serde_json::from_str(std::fs::read_to_string(shape_path).unwrap().as_str()).unwrap()
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
        bg_colour: ColourRgba,
        shape_colour: ColourRgba,
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
        fn contour_to_linestring(contour: &Contour<u32>) -> LineString<f64> {
            let coords: Vec<Coord<f64>> = contour
                .points
                .iter()
                .map(|p| Coord {
                    x: p.x as f64,
                    y: p.y as f64,
                })
                .collect();

            LineString::from(coords)
        }

        fn image_to_multipolygon(img: DynamicImage) -> MultiPolygon<f64> {
            let img = img.to_luma8();

            let mut binary = GrayImage::new(img.width(), img.height());
            for (x, y, p) in img.enumerate_pixels() {
                let v = if p[0] < 128 { 255 } else { 0 };
                binary.put_pixel(x, y, Luma([v]));
            }

            let contours = find_contours::<u32>(&binary);

            let mut outers: Vec<LineString<f64>> = Vec::new();
            let mut holes: Vec<LineString<f64>> = Vec::new();

            // Separate outer borders and holes
            for c in &contours {
                let ls = contour_to_linestring(c);
                if ls.0.len() < 3 {
                    continue;
                }
                match c.border_type {
                    BorderType::Outer => outers.push(ls),
                    BorderType::Hole => holes.push(ls),
                }
            }

            // Build polygons by assigning holes to the correct outer
            let mut polygons = Vec::new();

            for outer in outers {
                let outer_poly = Polygon::new(outer.clone(), vec![]);

                let mut inner_rings = Vec::new();

                for hole in &holes {
                    let hole_poly = Polygon::new(hole.clone(), vec![]);

                    // Check if hole belongs to this outer polygon
                    if outer_poly.contains(&hole_poly) {
                        inner_rings.push(hole.clone());
                    }
                }

                polygons.push(Polygon::new(outer, inner_rings));
            }

            MultiPolygon(polygons)
                .make_valid()
                .unwrap()
                .simplify_vw_preserve(32.0)
        }

        let shape = ShapeData::new(image_to_multipolygon(self.image.get_image()));

        std::fs::write(shape_path, serde_json::to_string_pretty(&shape).unwrap()).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeData {
    multipolygon: MultiPolygon,
}

impl ShapeData {
    pub fn new(multipolygon: MultiPolygon) -> Self {
        let es = multipolygon.validation_errors();
        for e in &es {
            println!("Invalid geometry: {}", e);
        }
        assert!(es.is_empty());

        Self { multipolygon }
    }

    pub fn scale(self, scale_factor: f64) -> Self {
        Self::new(self.multipolygon.affine_transform(&AffineTransform::scale(
            scale_factor,
            scale_factor,
            (0.0, 0.0),
        )))
    }

    pub fn translate(self, x_offset: f64, y_offset: f64) -> Self {
        Self::new(self.multipolygon.translate(x_offset, y_offset))
    }

    pub fn normalize(mut self) -> Self {
        if let Some(rect) = self.multipolygon.bounding_rect() {
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
        Self::new({
            let mut lines = Vec::new();
            for poly in &self.multipolygon.0 {
                lines.push(poly.exterior().clone());
                for inner in poly.interiors() {
                    lines.push(inner.clone());
                }
            }
            MultiLineString(lines).buffer(radius)
        })
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

        Self::new({
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
        })
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

    pub fn exmaple() -> Self {
        // --- Outer polygon ---
        let exterior = LineString::from(vec![
            Coord {
                x: 0.1000,
                y: 0.1000,
            },
            Coord {
                x: 0.9000,
                y: 0.1000,
            },
            Coord {
                x: 0.9000,
                y: 0.9000,
            },
            Coord {
                x: 0.1000,
                y: 0.9000,
            },
            Coord {
                x: 0.1000,
                y: 0.1000,
            },
        ]);

        // --- Two holes ---
        let hole1 = LineString::from(vec![
            Coord { x: 0.300, y: 0.300 },
            Coord { x: 0.450, y: 0.300 },
            Coord { x: 0.450, y: 0.450 },
            Coord { x: 0.300, y: 0.450 },
            Coord { x: 0.300, y: 0.300 },
        ]);

        let hole2 = LineString::from(vec![
            Coord { x: 0.600, y: 0.600 },
            Coord { x: 0.750, y: 0.600 },
            Coord { x: 0.750, y: 0.750 },
            Coord { x: 0.600, y: 0.750 },
            Coord { x: 0.600, y: 0.600 },
        ]);

        let polygon = Polygon::new(exterior.clone(), vec![hole1.clone(), hole2.clone()]);
        let multipolygon = MultiPolygon(vec![polygon]);
        Self::new(multipolygon)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeImage {
    pub width: u32,
    pub height: u32,
    pub bg_colour: ColourRgba,
    pub shape_colour: ColourRgba,
    pub shape: Box<ShapeSpec>,
}

impl ShapeImage {
    pub fn make_image(&self, path: &Path) {
        // convert geo coords to image points
        let to_points = |coords: &LineString<f64>| -> Vec<imageproc::point::Point<i32>> {
            coords
                .points()
                .map(|p| imageproc::point::Point::new(p.x() as i32, p.y() as i32))
                .collect()
        };

        let mut img = RgbaImage::new(AA_RESCALE * self.width, AA_RESCALE * self.height);

        // background
        for pixel in img.pixels_mut() {
            *pixel = self.bg_colour.to_rgba();
        }

        let shape = self.shape.get_shape();

        let mpoly =
            shape
                .multipolygon
                .scale_around_point(AA_RESCALE as f64, AA_RESCALE as f64, (0.0, 0.0));

        // outer polygon
        for poly in &mpoly {
            draw_polygon_mut(
                &mut img,
                &to_points(poly.exterior()),
                self.shape_colour.to_rgba(),
            );

            // cut out the holes
            for hole in poly.interiors() {
                draw_polygon_mut(&mut img, &to_points(hole), self.bg_colour.to_rgba());
            }
        }
        let img = DynamicImage::ImageRgba8(img)
            .resize_exact(self.width, self.height, FilterType::CatmullRom)
            .to_rgba8();

        img.save(path).unwrap();
    }
}
