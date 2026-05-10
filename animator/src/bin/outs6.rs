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
            slide.canvas(|canvas, t| {
                canvas
                    .circle((-6.0, -6.0), 0.5)
                    .fill_rgba(ColourBuilder::new().red().finish().alpha(1.0));

                canvas
                    .circle((6.0, -6.0), 0.5)
                    .fill_rgba(ColourBuilder::new().green().finish().alpha(1.0));

                canvas
                    .circle((-6.0, 6.0), 0.5)
                    .fill_rgba(ColourBuilder::new().purple().finish().alpha(1.0));

                canvas
                    .circle((6.0, 6.0), 0.5)
                    .fill_rgba(ColourBuilder::new().gold().finish().alpha(1.0));

                canvas.pixels(move |x, y| {
                    ColourBuilder::new()
                        .hue_rad(t)
                        .saturation((x + 6.0) / 12.0)
                        .lightness((y + 6.0) / 12.0)
                        .finish()
                        .alpha(1.0)
                });
            });
        })
        .duration(5.0);

    let video = slideshow.video(2.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();

    let path = ImageSpec::BlitStack {
        base: Box::new(ImageSpec::Filled {
            width: 100,
            height: 100,
            colour: ColourWithAlpha::from_linear(0.0, 0.0, 0.0, 1.0),
        }),
        layers: vec![(
            (0, 0),
            ImageSpec::Pixels(PixelsImage::new(100, 100, |x, y| {
                ColourBuilder::new()
                    .red()
                    .saturation(x as f64 / 100.0)
                    .lightness(y as f64 / 100.0)
                    .finish()
                    .alpha(1.0)
            })),
        )],
    }
    .make_path();
    std::fs::copy(path.clone(), Path::new("outputs/img.png")).unwrap();
}
