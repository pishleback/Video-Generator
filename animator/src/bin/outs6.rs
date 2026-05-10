use animator::{
    colour::{ColourBuilder, ColourWithAlpha},
    image::{ImageSpec, PixelsImage},
    slideshow::SlideshowBuilder,
};
use core::f64;
use std::path::Path;

fn main() {
    let mut slideshow =
        SlideshowBuilder::<1920, 1080>::new(ColourWithAlpha::from_srgb(0.0, 0.0, 0.0, 1.0));

    slideshow
        .slide(|slide| {
            slide.title_space().canvas(|canvas, t| {
                canvas.text("Title");
            });
        })
        .duration(3.0);

    let video = slideshow.video(20.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
}
