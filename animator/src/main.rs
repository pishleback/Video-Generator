pub mod animation;
pub mod colour;
pub mod data;
pub mod image;
pub mod interpolation;
pub mod shape;
pub mod timeline;
pub mod video;

use crate::{
    animation::{Animation, BoundaryMode, ShapeElement},
    colour::ColourRgba,
    image::{ImageSpec, LatexImage},
    interpolation::InterpType,
    shape::{FromImageShape, ShapeSpec},
};
use std::path::Path;

fn main() {
    let mut anim = Animation::<1920, 1080>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    anim.add(
        ShapeElement::new(
            ShapeSpec::FromImage(FromImageShape {
                image: ImageSpec::Latex(LatexImage {
                    scale: 8192,
                    expr: r#"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#.to_string(),
                }),
            })
            .normalize(),
            ColourRgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            ColourRgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            (0.0, 1.0),
            BoundaryMode::Inner,
        )
        .set_fill_alpha(0.0, 1.0, InterpType::Linear { duration: 4.0 })
        .set_boundary_mode(1.0, BoundaryMode::Middle)
        .set_boundary_mode(2.0, BoundaryMode::Outer),
    );

    let path = anim.video(0.0, 4.0, 10.0).get_path();
    std::fs::copy(path, Path::new("out.mp4")).unwrap();
}
