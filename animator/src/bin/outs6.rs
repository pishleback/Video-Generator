use animator::{
    colour::{ColourBuilder, ColourRgb, ColourRgba},
    slideshow::{AlignOptions, SlideshowBuilder},
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
                let c = (0.2 * t) % 1.0;
                for x in 0..20 {
                    for y in 0..20 {
                        let x = x as f64;
                        let y = y as f64;

                        let a = x / 20.0;
                        let b = y / 20.0;

                        canvas.circle((x, y), 0.5).fill_rgba(
                            ColourBuilder::new()
                                .hue_rad(c * f64::consts::TAU)
                                .lightness(a)
                                .saturation(b)
                                .finish()
                                .alpha(1.0),
                        );
                    }
                }

                canvas
                    .latex(format!(
                        "\\texttt{{{:.0} {:.0}}}",
                        c * 360.0,
                        c * f64::consts::TAU
                    ))
                    .position((10.0, -2.0))
                    .height(2.0);
            });
        })
        .duration(5.0);

    let video = slideshow.video(60.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
    println!("Done :)");
}
