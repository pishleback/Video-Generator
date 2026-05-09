use animator::{
    colour::{ColourRgb, ColourRgba},
    slideshow::SlideshowBuilder,
};
use core::f64;
use std::path::Path;

fn main() {
    let mut slideshow = SlideshowBuilder::<1920, 1080>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    slideshow
        .slide(|slide| {
            slide.canvas(|canvas, t| {
                for a in 0..100 {
                    let a_rad = f64::consts::TAU * (a as f64) / 100.0;
                    let a_deg = 360.0 * (a as f64) / 100.0;
                    canvas
                        .circle((a_rad.cos(), a_rad.sin()), 0.03)
                        .fill_rgba(ColourRgb::oklch_to_rgb(a_deg, 0.2, 0.1).to_rgba(1.0));
                }
            });
        })
        .duration(10.0);

    let video = slideshow.video(20.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
    println!("Done :)");
}
