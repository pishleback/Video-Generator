pub mod animation;
pub mod colour;
pub mod data;
pub mod image;
pub mod shape;
pub mod video;

use crate::{
    animation::{Animation, InterpType, ShapeElement},
    colour::Colour,
    image::{ImageSpec, LatexImage},
    shape::FromImageShape,
};
use std::path::Path;

fn main() {
    let mut anim = Animation::<1920, 1080>::new(Colour {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    anim.add(
        ShapeElement::new(
            shape::ShapeSpec::FromImage(FromImageShape {
                image: ImageSpec::Latex(LatexImage {
                    scale: 8192,
                    expr: r#"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#.to_string(),
                }),
            })
            .normalize(),
            Colour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            Colour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            (0.0, 0.0),
        )
        .set_boundary_frac(1.0, (0.0, 1.0), InterpType::Exp { duration: 2.0 })
        .set_fill_colour(
            2.5,
            Colour {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            InterpType::Exp { duration: 0.5 },
        ),
    );

    let path = anim.video(0.0, 5.0, 10.0).get_path();
    std::fs::copy(path, Path::new("out.mp4")).unwrap();
}
