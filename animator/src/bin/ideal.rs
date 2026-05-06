use std::path::Path;

use animator::{
    colour::ColourRgba,
    slideshow::{InterpType, SlideshowBuilder},
};

fn main() {
    let mut slideshow = SlideshowBuilder::<1920, 1080>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    slideshow.slide(|slide| {
        let region = slide.left_half().top_half();

        let picture = region.picture();

        picture
            .line((0.0, 0.0), (1.0, 1.0), 0.2)
            .fill_rgba(ColourRgba {
                r: 1.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            })
            .interp_id(0);
    });

    slideshow.interp().duration(1.0);

    slideshow.slide(|slide| {
        let region = slide.right_half().top_half();

        let picture = region.picture();

        picture.circle((0.0, 1.0), 0.5).interp_id(0);
    });

    slideshow.interp().duration(1.0);

    slideshow.slide(|slide| {
        let region = slide.right_half().bottom_half();

        let picture = region.picture();

        picture
            .line((1.0, 0.0), (0.0, 1.0), 0.1)
            .fill_rgba(ColourRgba {
                r: 0.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            })
            .interp_id(0);
    });

    let video = slideshow.video(30.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
}
