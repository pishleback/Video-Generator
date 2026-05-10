use crate::{
    colour::{ColourWithAlpha, linear_to_srgb, srgb_to_linear},
    data::{FileSpec, cache},
    shape::ShapeImage,
    video::VideoSpec,
};
use image::{DynamicImage, ImageBuffer, Rgba32FImage, imageops::FilterType};
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageSpec {
    Filled {
        width: u32,
        height: u32,
        colour: ColourWithAlpha,
    },
    Pixels(PixelsImage),
    Resize {
        image: Box<ImageSpec>,
        width: u32,
        height: u32,
    },
    BlitStack {
        base: Box<ImageSpec>,
        layers: Vec<((i64, i64), ImageSpec)>,
    },
    VideoFrame {
        video: VideoSpec,
        frame: usize,
    },
    Latex(LatexImage),
    Shape(ShapeImage),
}

impl ImageSpec {
    pub fn make_image(&self, path: &Path) {
        match self {
            ImageSpec::Latex(x) => x.make_image(path),
            _ => DynamicImage::ImageRgba32F(self.image())
                .to_rgba8()
                .save(path)
                .unwrap(),
        }
    }

    pub fn make_path(&self) -> PathBuf {
        cache().make_file(&FileSpec::Image(self.clone()))
    }

    pub fn image(&self) -> Rgba32FImage {
        match self {
            ImageSpec::Shape(x) => x.image(),
            ImageSpec::VideoFrame { video, frame } => video.frame(*frame).unwrap(),
            ImageSpec::Resize {
                image,
                width,
                height,
            } => DynamicImage::ImageRgba32F(image.image())
                .resize_exact(*width, *height, FilterType::CatmullRom)
                .into(),
            ImageSpec::Pixels(pixels) => pixels.image(),
            ImageSpec::Filled {
                width,
                height,
                colour,
            } => ImageBuffer::from_fn(*width, *height, |_x, _y| colour.to_srgb_f32()),
            ImageSpec::BlitStack { base, layers } => {
                fn blit_linear(dst: &mut Rgba32FImage, src: &Rgba32FImage, ox: i64, oy: i64) {
                    let (dw, dh) = dst.dimensions();

                    for sy in 0..src.height() {
                        let dy = sy as i64 + oy;
                        if dy < 0 || dy >= dh as i64 {
                            continue;
                        }

                        for sx in 0..src.width() {
                            let dx = sx as i64 + ox;
                            if dx < 0 || dx >= dw as i64 {
                                continue;
                            }

                            let sp = src.get_pixel(sx, sy);
                            let dp = dst.get_pixel_mut(dx as u32, dy as u32);

                            let sa = sp[3];
                            let sr = srgb_to_linear(sp[0]);
                            let sg = srgb_to_linear(sp[1]);
                            let sb = srgb_to_linear(sp[2]);

                            let da = dp[3];
                            let dr = srgb_to_linear(dp[0]);
                            let dg = srgb_to_linear(dp[1]);
                            let db = srgb_to_linear(dp[2]);

                            let out_a = sa + da * (1.0 - sa);
                            let out_r = sr * sa + dr * da * (1.0 - sa);
                            let out_g = sg * sa + dg * da * (1.0 - sa);
                            let out_b = sb * sa + db * da * (1.0 - sa);

                            if out_a > 0.0 {
                                let inv_a = 1.0 / out_a;
                                dp[0] = linear_to_srgb(out_r * inv_a);
                                dp[1] = linear_to_srgb(out_g * inv_a);
                                dp[2] = linear_to_srgb(out_b * inv_a);
                            } else {
                                dp[0] = 0.0;
                                dp[1] = 0.0;
                                dp[2] = 0.0;
                            }
                            dp[3] = out_a;
                        }
                    }
                }

                let mut result = base.image();
                for ((x, y), image_spec) in layers {
                    let img = image_spec.image();
                    blit_linear(&mut result, &img, *x, *y);
                }
                result
            }
            _ => image::open(self.make_path()).unwrap().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatexImage {
    pub scale: u32,
    pub expr: String,
}

impl LatexImage {
    pub fn make_image(&self, path: &Path) {
        let latex = format!(
            "{}{}{}",
            r#"
\documentclass[border=1pt]{standalone}
\usepackage{amsmath}
\usepackage{xcolor}
\begin{document}
$"#,
            self.expr,
            r#"$
\end{document}
"#
        );

        let pdf = tectonic::latex_to_pdf(latex).unwrap();
        let mut tmp_pdf = tempfile::NamedTempFile::new().unwrap();
        tmp_pdf.write_all(&pdf).unwrap();
        let pdf_path = tmp_pdf.path();

        let tmp_dir = tempfile::TempDir::new().unwrap();
        std::process::Command::new("pdftoppm")
            .args([
                "-png",
                pdf_path.to_str().unwrap(),
                "-r",
                self.scale.to_string().as_str(),
                tmp_dir.path().join("out").to_str().unwrap(),
            ])
            .status()
            .unwrap();

        std::fs::copy(tmp_dir.path().join("out-1.png"), path).unwrap();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PixelsImage {
    width: u32,
    height: u32,
    pixels_hash: Vec<ColourWithAlpha>,
    #[serde(skip)]
    pixels: Option<Arc<dyn Fn(u32, u32) -> ColourWithAlpha + Send + Sync + 'static>>,
}

impl std::fmt::Debug for PixelsImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelsImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl PartialEq for PixelsImage {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.pixels_hash == other.pixels_hash
    }
}

impl Eq for PixelsImage {}

impl PixelsImage {
    pub fn new(
        width: u32,
        height: u32,
        pixels: impl Fn(u32, u32) -> ColourWithAlpha + Send + Sync + 'static,
    ) -> Self {
        let pixels = Arc::new(pixels);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let pixels_hash = (0..60)
            .map({
                let pixels = pixels.clone();
                move |_| {
                    let x = rng.random_range(0..width);
                    let y = rng.random_range(0..height);
                    pixels(x, y)
                }
            })
            .collect();
        Self {
            width,
            height,
            pixels_hash,
            pixels: Some(pixels),
        }
    }

    fn image(&self) -> Rgba32FImage {
        let pixels = self.pixels.clone().unwrap();
        ImageBuffer::from_fn(self.width, self.height, move |x, y| {
            pixels(x, y).to_srgb_f32()
        })
    }
}
