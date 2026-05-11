use animator::{
    colour::{Colour, ColourBuilder, ColourWithAlpha},
    slideshow::{Align, SlideshowBuilder},
};
use std::path::Path;

fn main() {
    let mut slideshow =
        SlideshowBuilder::<1920, 1080>::new(ColourWithAlpha::from_srgb(0.0, 0.0, 0.0, 1.0));

    slideshow.slide(|slide| {});

    slideshow
        .slide(|slide| {
            slide.title_space_split().0.canvas(|picture, t| {
                picture.latex("This is a Title");
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
                        .fill_rgba(ColourWithAlpha::from_srgb(1.0, 0.0, 0.5, 1.0));
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
                                Colour::from_hsl(10.0 * (a as f64 + 6.0 * b as f64), 1.0, 0.5)
                                    .alpha(1.0),
                            );
                        }
                    }
                }
            });
        })
        .duration(2.0);

    slideshow
        .slide(|slide| {
            slide.title_space_split().0.canvas(|picture, t| {
                picture.latex("This is another Title");
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
                                Colour::from_hsl(10.0 * (a as f64 + 6.0 * b as f64), 1.0, 0.5)
                                    .alpha(1.0),
                            );
                        }
                    }
                }
            });
        })
        .duration(2.0);

    slideshow
        .slide(|slide| {
            slide
                .canvas(|canvas, t| {
                    let c = 0.2 * t;
                    for x in 0..20 {
                        for y in 0..20 {
                            let x = x as f64;
                            let y = y as f64;

                            let a = x / 20.0;
                            let b = y / 20.0;

                            canvas.circle((x, y), 0.5).fill_rgba(
                                ColourBuilder::new()
                                    .hue_rad(c * std::f64::consts::TAU)
                                    .lightness(a)
                                    .saturation(b)
                                    .finish()
                                    .alpha(1.0),
                            );
                        }
                    }

                    canvas
                        .maths(format!(
                            "\\texttt{{{:.0} {:.1}}}",
                            c * 360.0,
                            c * std::f64::consts::TAU
                        ))
                        .align(Align::Center)
                        .position((10.0, -2.0))
                        .scale(2.0);
                })
                .sampled_bounding_rect(vec![6.0]);
        })
        .duration(5.0);

    slideshow
        .slide(|slide| {
            slide
                .canvas(|canvas, t| {
                    canvas
                        .pixels(move |x, y| {
                            ColourBuilder::new()
                                .hue_rad(t)
                                .saturation(x)
                                .lightness(y)
                                .finish()
                                .alpha(1.0)
                        })
                        .interp_id(0);
                })
                .fixed_bounding_rect(0.0, 2.0, 0.0, 1.0);
        })
        .duration(6.0);

    slideshow
        .slide(|slide| {
            slide
                .canvas(|canvas, t| {
                    canvas
                        .pixels(move |x, y| {
                            ColourBuilder::new()
                                .hue_rad(t)
                                .saturation(y)
                                .lightness(x)
                                .finish()
                                .alpha(1.0)
                        })
                        .interp_id(0);
                })
                .fixed_bounding_rect(0.0, 1.0, 0.0, 2.0);
        })
        .duration(6.0);

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.latex("Title");
        });

        body.canvas(|canvas, t| {
            canvas.maths(
                r#"
            \begin{aligned}
                \bullet & \; \text{Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii Hii} \\
                \bullet & \; \text{Byeeee \(a + \frac{b}{c}\)} \\
                \bullet & \; \text{AwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwooAwoo}
            \end{aligned}
            "#,
            );
        });
    });

    slideshow.slide(|slide| {
        let (title, body) = slide.title_space_split();
        title.canvas(|canvas, t| {
            canvas.latex("Title");
        });

        body.left_half().left_half().canvas(|canvas, t| {
            canvas
                .maths("\\bullet \\; \\text{Hiiiiiiiiiii}")
                .align(Align::CenterLeft)
                .position((0.0, 0.0))
                .scale(1.0);
            canvas
                .maths("\\bullet \\; \\text{weee \\(\\displaystyle a+\\frac{b}{c}\\)}")
                .align(Align::CenterLeft)
                .position((0.0, 1.0))
                .scale(1.0);
            canvas
                .maths("\\bullet \\; \\text{wooo}")
                .align(Align::CenterLeft)
                .position((0.0, 2.0))
                .scale(1.0);
            canvas
                .maths("\\bullet \\; \\text{Hiiiiiiiiiii}")
                .align(Align::CenterLeft)
                .position((0.0, 3.0))
                .scale(1.0);
            canvas
                .maths("\\bullet \\; \\text{weee \\(\\scriptstyle a+\\frac{b}{c}\\)}")
                .align(Align::CenterLeft)
                .position((0.0, 4.0))
                .scale(1.0);
            canvas
                .maths("\\bullet \\; \\text{wooo}")
                .align(Align::CenterLeft)
                .position((0.0, 5.0))
                .scale(1.0);
        });
    });

    let video = slideshow.video(60.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
    println!("Done :)");
}
