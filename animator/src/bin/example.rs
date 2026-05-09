use animator::{
    colour::{ColourRgb, ColourRgba},
    slideshow::SlideshowBuilder,
};
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
            slide.title_space().canvas(|picture, t| {
                picture.text("This is a Title");
            });

            slide.right_half().top_half().canvas(|canvas, t| {
                canvas.circle((1.0, 1.0), 0.2);
                if t % 1.0 < 0.5 {
                    canvas.circle((-1.0, 1.0), 0.2);
                }

                {
                    let a = 2.0 * t;
                    canvas.circle((2.0 * a.cos(), 2.0 * a.sin()), 0.2);
                }

                {
                    let group = canvas.group().interp_id(1);
                    group.circle((2.0, 2.0), 0.2);
                    group
                        .circle((-2.0, 2.0 * (5.0 * t).sin()), 0.2)
                        .fill_rgba(ColourRgba {
                            r: 1.0,
                            g: 0.0,
                            b: 0.5,
                            a: 1.0,
                        });
                    group.circle((2.0, -2.0), 0.2);
                    group.circle((-2.0, -2.0), 0.2);
                }

                if t % 1.0 < 0.5 {
                    canvas.circle((1.0, -1.0), 0.2);
                }
                canvas.circle((-1.0, -1.0), 0.2);
            });

            slide.left_half().top_half().canvas(|picture, t| {
                let mut pts = vec![(0.0, 0.0)];
                for i in 0..5 {
                    let a = std::f64::consts::TAU * (i as f64) / 5.0;
                    pts.push((a.cos(), a.sin()));
                }

                picture.circle((2.0, 2.0), t % 1.0);
                picture.circle((-2.0, 2.0), t % 1.0);
                picture.circle((2.0, -2.0), t % 1.0);
                picture.circle((-2.0, -2.0), t % 1.0);

                {
                    let picture = picture.group().interp_id(0);
                    for pt in &pts {
                        picture.circle(*pt, 0.3);
                    }
                    for a in 0..6 {
                        for b in 0..a {
                            picture.line(pts[a], pts[b], 0.1).fill_rgba(
                                ColourRgb::from_hsl(10.0 * (a as f64 + 6.0 * b as f64), 1.0, 0.5)
                                    .to_rgba(1.0),
                            );
                        }
                    }
                }
            });
        })
        .duration(2.0);

    slideshow
        .slide(|slide| {
            slide.title_space().canvas(|picture, t| {
                picture.text("This is another Title");
            });

            slide.left_half().bottom_half().canvas(|canvas, t| {
                let group = canvas.group().interp_id(1);
                group.circle((2.0, 2.0), 0.2);
                group.circle((-2.0, 2.0 * (10.0 * t).sin()), 0.2);
                group.circle((2.0, -2.0), 0.2);
                group.circle((-2.0, -2.0), 0.2);
            });

            slide.right_half().bottom_half().canvas(|picture, t| {
                let mut pts = vec![(0.0, 0.0)];
                for i in 0..5 {
                    let a = std::f64::consts::TAU * (i as f64) / 5.0;
                    pts.push((a.cos(), a.sin()));
                }

                picture.circle((2.0, 2.0), t % 1.0);
                picture.circle((-2.0, 2.0), t % 1.0);
                picture.circle((2.0, -2.0), t % 1.0);
                picture.circle((-2.0, -2.0), t % 1.0);

                {
                    let picture = picture.group().interp_id(0);
                    for pt in &pts {
                        picture.circle(*pt, 0.3);
                    }
                    for a in 0..6 {
                        for b in 0..a {
                            picture.line(pts[a], pts[b], 0.1).fill_rgba(
                                ColourRgb::from_hsl(10.0 * (a as f64 + 6.0 * b as f64), 1.0, 0.5)
                                    .to_rgba(1.0),
                            );
                        }
                    }
                }
            });
        })
        .duration(2.0);

    let video = slideshow.video(20.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
    println!("Done :)");
}
