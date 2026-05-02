pub mod animation;
pub mod colour;
pub mod data;
pub mod image;
pub mod interpolation;
pub mod shape;
pub mod timeline;
pub mod video;

use crate::{
    animation::{Animation, BoundaryMode, ShapeElement, ShapeElementParams},
    colour::ColourRgba,
    image::{ImageSpec, LatexImage},
    interpolation::{Exp2Interp, ExpInterp, FastEndInterp, FastStartInterp, LinearInterp},
    shape::{FromImageShape, ShapeSpec},
};
use std::path::Path;

fn main() {
    let mut t = 0.0;

    // const WIDTH: u32 = 3840;
    // const HEIGHT: u32 = 2160;
    // const FPS: f64 = 60.0;

    const WIDTH: u32 = 1920;
    const HEIGHT: u32 = 1080;
    const FPS: f64 = 30.0;

    // const WIDTH: u32 = 384;
    // const HEIGHT: u32 = 216;
    // const FPS: f64 = 5.0;

    let mut anim = Animation::<WIDTH, HEIGHT>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    let elem1 = ShapeElement::new(
        ShapeSpec::FromImage(FromImageShape {
            image: ImageSpec::Latex(LatexImage {
                scale: 8192,
                expr: r#"\frac{-b \pm \sqrt{b^2 - 4ac}}{8a}"#.to_string(),
            }),
        })
        .normalize(),
        ShapeElementParams::new(
            ColourRgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            },
            ColourRgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        )
        .boundary_mode(BoundaryMode::Inner)
        .boundary_frac((0.0, 0.0))
        .scale(40.0),
    );
    let elem1 = anim.add(elem1);

    elem1.set_boundary_frac(t, (2.0, 3.0), FastStartInterp { duration: 2.5 });
    elem1.set_fill_alpha(t + 1.5, 1.0, LinearInterp { duration: 1.5 });

    t += 4.0;

    let path = anim.video(0.0, t + 1.0, FPS).get_path();
    std::fs::copy(path, Path::new("out.mp4")).unwrap();
}
