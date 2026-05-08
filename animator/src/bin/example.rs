use std::path::Path;

use animator::{
    colour::{ColourRgb, ColourRgba},
    slideshow::{SlideshowBuilder, Timeline},
};

fn main() {
    let mut slideshow = SlideshowBuilder::<1920, 1080>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    slideshow
        .slide(|slide| {
            slide.title_space().picture().text("This is a Title");

            let region = slide.left_half().top_half();

            let picture = region.picture();

            // for i in 0..10 {
            //     let a_rad = std::f64::consts::TAU * (i as f64) / 10.0;
            //     let a_deg = 360.0 * (i as f64) / 10.0;
            //     picture
            //         .circle((a_rad.cos(), a_rad.sin()), 0.05)
            //         .fill_rgba(ColourRgb::from_hsl(a_deg, 1.0, 0.5).to_rgba(1.0));
            // }

            let mut pts = vec![(0.0, 0.0)];
            for i in 0..5 {
                let a = std::f64::consts::TAU * (i as f64) / 5.0;
                pts.push((a.cos(), a.sin()));
            }

            picture.circle((2.0, 2.0), Timeline::from_fn(|t| t % 1.0));
            picture.circle((-2.0, 2.0), Timeline::from_fn(|t| t % 1.0));
            picture.circle((2.0, -2.0), Timeline::from_fn(|t| t % 1.0));
            picture.circle((-2.0, -2.0), Timeline::from_fn(|t| t % 1.0));

            picture.start_interp_id_group(0);

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

            picture.stop_interp_id_group();
        })
        .duration(4.0);

    slideshow.interp().duration(1.0);

    slideshow.slide(|slide| {
        let picture = slide.right_half().top_half().picture();

        picture.circle((2.0, 2.0), 0.5).interp_id(0);

        picture.start_interp_id_group(1);

        let mut pts = vec![(0.0, 0.0)];
        for i in 0..5 {
            let a = std::f64::consts::TAU * ((i + 2) as f64) / 5.0;
            pts.push((a.cos(), a.sin()));
        }

        for pt in &pts {
            picture.circle(*pt, 0.2);
        }
        for a in 0..6 {
            for b in 0..a {
                picture.line(pts[a], pts[b], 0.1).fill_rgba(
                    ColourRgb::from_hsl(10.0 * (a as f64 + 6.0 * b as f64), 1.0, 0.5).to_rgba(1.0),
                );
            }
        }

        picture.stop_interp_id_group();

        let picture = slide.right_half().bottom_half().picture();

        picture.start_interp_id_group(0);

        let mut pts = vec![(0.0, 0.0)];
        for i in 0..5 {
            let a = std::f64::consts::TAU * ((i + 2) as f64) / 5.0;
            pts.push((a.cos(), a.sin()));
        }

        for pt in &pts {
            picture.circle(*pt, 0.2);
        }
        for a in 0..6 {
            for b in 0..a {
                picture.line(pts[a], pts[b], 0.1).fill_rgba(
                    ColourRgb::from_hsl(10.0 * (a as f64 + 6.0 * b as f64), 1.0, 0.5).to_rgba(1.0),
                );
            }
        }

        picture.stop_interp_id_group();
    });

    slideshow.interp().duration(1.0);

    slideshow.slide(|slide| {
        let picture = slide.right_half().bottom_half().picture();

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

    let video = slideshow.video(20.0);
    let path = video.make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
}
