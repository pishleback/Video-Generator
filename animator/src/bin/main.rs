use animator::{
    animation::{
        Animation, AnimationElement, BoundaryMode, ShapeElement, SubVideoElement, SubVideoSource,
    },
    colour::{ColourRgb, ColourRgba},
    coords::{Length, Rect, Vec2},
    image::{ImageSpec, LatexImage},
    interpolation::InterpType,
    shape::{FromImageShape, ShapeSpec},
    timeline::Timeline,
};
use animator::{audio::AudioSpec, video::VideoSpec};
use imageproc::drawing::Canvas;
use std::path::{Path, PathBuf};

fn main() {
    let mut t = 0.0;

    // const WIDTH: u32 = 3840;
    // const HEIGHT: u32 = 2160;
    // const FPS: f64 = 60.0;

    // const WIDTH: u32 = 1920;
    // const HEIGHT: u32 = 1080;
    // const FPS: f64 = 30.0;

    const WIDTH: u32 = 1920;
    const HEIGHT: u32 = 1080;
    const FPS: f64 = 2.0;

    let mut anim = Animation::<WIDTH, HEIGHT>::new(ColourRgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    let elem1 = anim.add_shape(ShapeSpec::latex(
        r#"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"#.to_string(),
    ));
    elem1.set_boundary_mode(t, None, BoundaryMode::Outer);
    elem1.set_boundary_rgb(
        t,
        None,
        ColourRgb {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        },
        InterpType::Immediate,
    );
    elem1.set_boundary_alpha(t, None, 1.0, InterpType::Immediate);

    t += 3.0;

    elem1.set_boundary_frac(t, None, (0.0, 3.0), InterpType::Exp { duration: 2.5 });
    elem1.set_fill_alpha(t + 1.5, None, 1.0, InterpType::Linear { duration: 1.5 });
    elem1.set_within_rect(
        t,
        Rect::fullscreen().right_half(),
        InterpType::Exp { duration: 2.0 },
    );

    t += 3.0;

    anim.add_audio(
        t,
        AudioSpec::File {
            path: "\
/home/michael/Documents/GitHub/Animation-Generator/assets/soundscrate-dreaming-cello.mp3"
                .into(),
        },
    );

    let rec = VideoSpec::File {
        path: PathBuf::from(
            "\
/home/michael/Documents/GitHub/Animation-Generator/assets/2026-05-02 21-55-09.mp4",
        ),
    };

    let elem2 = anim.add_subanimation(t, SubVideoSource::Video(rec.clone()));
    elem2.set_within_rect(
        t,
        Rect::fullscreen().left_half().top_half(),
        InterpType::Immediate,
    );
    anim.add_audio(t, rec.audio());

    let path = anim.video(0.0, t + 1.0, FPS).make_path();
    std::fs::copy(path.clone(), Path::new("outputs/out.mp4")).unwrap();
}
