use animator::{
    colour::{ColourBuilder, ColourWithAlpha},
    image::ImageSpec,
    slideshow::SlideshowBuilder,
};
use core::f64;
use std::path::Path;

fn main() {
    let mut slideshow =
        SlideshowBuilder::<1920, 1080>::new(ColourWithAlpha::from_srgb(0.0, 0.0, 0.0, 1.0));

    slideshow
        .slide(|slide| {
            slide.canvas(|canvas, t| {
                println!("{:?}", t);

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
                        "\\texttt{{{:.0} {:.1}}}",
                        c * 360.0,
                        c * f64::consts::TAU
                    ))
                    .position((10.0, -2.0))
                    .height(2.0);
            });
        })
        .duration(5.0);

    let video = slideshow.video(20.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
    println!("Done :)");

    ImageSpec::Filled {
        width: 100,
        height: 100,
        colour: ColourBuilder::new()
            .gold()
            .saturation(1.0)
            .lightness(0.5)
            .finish()
            .alpha(1.0),
    }
    .make_image(Path::new("outputs/img.png"));
}
