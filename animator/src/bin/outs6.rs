use animator::{
    animation::{
        Animation, AnimationElement, BoundaryMode, ShapeElement, ShapeElementCollection,
        SubVideoElement, SubVideoSource, shape_group,
    },
    colour::{ColourRgb, ColourRgba},
    coords::{Length, Pos2, Rect, Vec2},
    image::{ImageSpec, LatexImage},
    interpolation::InterpType,
    shape::{FromImageShape, ShapeSpec},
    timeline::Timeline,
};
use animator::{audio::AudioSpec, video::VideoSpec};
use imageproc::drawing::Canvas;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

struct CompleteGraph<const WIDTH: u32, const HEIGHT: u32> {
    points: Vec<Rc<ShapeElement<WIDTH, HEIGHT>>>,
    lines: Vec<Rc<ShapeElement<WIDTH, HEIGHT>>>,
    union: Rc<ShapeElement<WIDTH, HEIGHT>>,
    all_points: ShapeElementCollection<WIDTH, HEIGHT>,
    all_lines: ShapeElementCollection<WIDTH, HEIGHT>,
    all_points_and_lines: ShapeElementCollection<WIDTH, HEIGHT>,
    all: ShapeElementCollection<WIDTH, HEIGHT>,
}

impl<const WIDTH: u32, const HEIGHT: u32> CompleteGraph<WIDTH, HEIGHT> {
    pub fn pair_to_line(mut a: usize, mut b: usize) -> usize {
        assert_ne!(a, b);
        if b < a {
            (a, b) = (b, a);
        }
        a + b * (b - 1) / 2
    }

    pub fn new(
        anim: &mut Animation<WIDTH, HEIGHT>,
        points: Vec<(f64, f64)>,
        point_radius: f64,
        line_radius: f64,
    ) -> Self {
        let n = points.len();

        let mut union = ShapeSpec::Empty;

        let line_elems = (0..n)
            .flat_map(|b| {
                (0..b)
                    .map(|a| {
                        let shape = ShapeSpec::Line {
                            point1: points[a],
                            point2: points[b],
                            radius: line_radius,
                        };
                        union = union.union(&shape);
                        anim.add_shape(shape)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let point_elems = (0..n)
            .map(|i| {
                let shape = ShapeSpec::Circle {
                    center: points[i],
                    radius: point_radius,
                };
                union = union.union(&shape);
                anim.add_shape(shape)
            })
            .collect::<Vec<_>>();

        let union = anim.add_shape(union);

        Self {
            points: point_elems.clone(),
            lines: line_elems.clone(),
            union: union.clone(),
            all_points: shape_group(point_elems.clone()),
            all_lines: shape_group(line_elems.clone()),
            all_points_and_lines: shape_group(
                line_elems.clone().into_iter().chain(point_elems.clone()),
            ),
            all: shape_group(
                line_elems
                    .clone()
                    .into_iter()
                    .chain(point_elems.clone())
                    .chain(vec![union]),
            ),
        }
    }

    pub fn complete_6_pentagon(
        anim: &mut Animation<WIDTH, HEIGHT>,
        point_radius: f64,
        line_radius: f64,
    ) -> Self {
        let mut points = vec![(0.0, 0.0)];
        for i in 0..5 {
            let a = std::f64::consts::TAU * (0.75 + i as f64) / 5.0;
            points.push((a.cos(), a.sin()));
        }
        Self::new(anim, points, point_radius, line_radius)
    }

    pub fn get_all(&self) -> &ShapeElementCollection<WIDTH, HEIGHT> {
        &self.all
    }

    pub fn get_point(&self, n: usize) -> Rc<ShapeElement<WIDTH, HEIGHT>> {
        self.points[n].clone()
    }

    pub fn get_line(&self, a: usize, b: usize) -> Rc<ShapeElement<WIDTH, HEIGHT>> {
        self.lines[Self::pair_to_line(a, b)].clone()
    }
}

fn main() {
    let red = ColourRgb {
        r: 1.0,
        g: 0.0,
        b: 0.0,
    };

    let yellow = ColourRgb {
        r: 1.0,
        g: 1.0,
        b: 0.0,
    };

    let green = ColourRgb {
        r: 0.0,
        g: 1.0,
        b: 0.0,
    };

    let cyan = ColourRgb {
        r: 0.0,
        g: 1.0,
        b: 1.0,
    };

    let blue = ColourRgb {
        r: 0.0,
        g: 0.0,
        b: 1.0,
    };

    let magenta = ColourRgb {
        r: 0.0,
        g: 1.0,
        b: 1.0,
    };

    let white = ColourRgb {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };

    // const WIDTH: u32 = 3840;
    // const HEIGHT: u32 = 2160;
    // const FPS: f64 = 60.0;

    const WIDTH: u32 = 1920;
    const HEIGHT: u32 = 1080;
    const FPS: f64 = 30.0;

    // const WIDTH: u32 = 1920;
    // const HEIGHT: u32 = 1080;
    // const FPS: f64 = 5.0;

    let change_duration = 1.0;
    let slide_duration = 3.0;

    let mut anim = Animation::<WIDTH, HEIGHT>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    let mut t = 0.0;

    {
        let k6 = CompleteGraph::complete_6_pentagon(&mut anim, 0.1, 0.03);

        {
            k6.all.set_within_rect(
                t,
                Rect::fullscreen().pad(Length::from_units(10.0)),
                InterpType::Immediate,
            );
            k6.union
                .set_boundary_frac(t, (0.0, 0.0), InterpType::Immediate);
            k6.union.set_boundary_rgba(
                t,
                ColourRgba {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                InterpType::Immediate,
            );

            for n in 0..6 {
                let p = k6.get_point(n);
                p.set_draw_ordering(t, 1.0.into());
                p.set_fill_rgb(
                    t,
                    ColourRgb {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                    },
                    InterpType::Immediate,
                );
            }

            for a in 0..6 {
                for b in 0..a {
                    k6.get_line(a, b)
                        .set_fill_rgb(t, cyan, InterpType::Immediate);
                }
            }

            t += 1.0;
        }

        {
            k6.all_points_and_lines.set_fill_alpha(
                t + 0.5,
                1.0,
                InterpType::Exp {
                    duration: change_duration,
                },
            );
            k6.union.set_boundary_frac(
                t,
                (0.0, 1.0),
                InterpType::FastStart {
                    duration: change_duration,
                },
            );
            k6.union.set_boundary_frac(
                t + 0.5 * change_duration,
                (1.0, 1.0),
                InterpType::FastStart {
                    duration: change_duration,
                },
            );

            t += slide_duration;
        }

        {
            k6.all.set_within_rect(
                t,
                Rect::fullscreen().left_half().pad(Length::from_units(20.0)),
                InterpType::Exp {
                    duration: change_duration,
                },
            );

            let expr = anim.add_shape(ShapeSpec::latex(
                r#"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#.to_string(),
            ));
            expr.set_within_rect(
                t,
                Rect::fullscreen()
                    .right_half()
                    .pad(Length::from_units(20.0)),
                InterpType::Immediate,
            );

            expr.set_fill_alpha(
                t + 0.1,
                1.0,
                InterpType::Exp {
                    duration: change_duration,
                },
            );

            t += slide_duration;
        }
    }

    let path = anim.video(0.0, t + 1.0, FPS).make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
}
