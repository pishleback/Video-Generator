use animator::{
    colour::{ColourBuilder, ColourWithAlpha},
    image::{ImageSpec, PixelsImage},
    slideshow::{Align, MorphInterpType, ShapeInterpType, SlideshowBuilder},
    timeline::Timeline,
};
use core::f64;
use std::path::Path;

fn main() {
    let mut slideshow =
        SlideshowBuilder::<1920, 1080>::new(ColourWithAlpha::from_srgb(0.0, 0.0, 0.0, 1.0));

    /*
    The outer automorphism of S6
     - Groups as ways in which things can be symmetric
       - Symmetries of a cube = the set of actions which preserves the cube
       - In general the invertible structure preserving maps on an object to itself is a group Aut(X)
     - Group automorphisms
       - If G is a finite group then Aut(G) is a finite group
       - Aut(V4) = S3
       - The conjugation action by any element is an automorphism
       - These are the inner automorphisms and they form a normal subgroup Inn(G) of Aut(G)
       - The quotient group Out(G) := Aut(G)/Inn(G) is the outer automorphism group
    - Examples
      - Aut(Cn) = Out(Cn) = (Z/nZ)^x    Inn(Cn) = {1}
      - Aut(V4) = S3
      - Symmetric groups:
        - Expect Aut(Sn) = Sn because of the inner automorphisms
          - Breaks at n=2 because it's abelian
          - Breaks at n=6 because there is an non-trivial outer automorphism
          - Works for all other n

     */

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.maths("\\text{The Graph \\(K_6\\)}");
        });
    });

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.maths("\\text{Definition: \\bf{Duad}}");
        });
    });

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.maths("\\text{Definition: \\bf{Syntheme}}");
        });
    });

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.maths("\\text{Definition: \\bf{Pentad}}");
        });
    });

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.maths("\\text{Duads and Synthemes as Permutations}");
        });
    });

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.maths("\\text{Duads and Synthemes as Permutations}");
        });
    });

    let video = slideshow.video(30.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
}
