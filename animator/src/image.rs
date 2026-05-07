use crate::{
    colour::ColourRgba,
    data::{FileSpec, cache},
    shape::ShapeImage,
    video::VideoSpec,
};
use image::{DynamicImage, ImageBuffer, RgbaImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageSpec {
    Filled {
        width: u32,
        height: u32,
        colour: ColourRgba,
    },
    Resize {
        image: Box<ImageSpec>,
        width: u32,
        height: u32,
    },
    BlitStack {
        base: Box<ImageSpec>,
        layers: Vec<((f64, f64), ImageSpec)>,
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
            ImageSpec::Filled {
                width,
                height,
                colour,
            } => {
                let img = ImageBuffer::from_fn(*width, *height, |_x, _y| colour.to_rgba());
                img.save(path).unwrap();
            }
            _ => self.image().save(path).unwrap(),
        }
    }

    pub fn make_path(&self) -> PathBuf {
        cache().make_file(&FileSpec::Image(self.clone()))
    }

    pub fn image(&self) -> image::DynamicImage {
        match self {
            ImageSpec::Shape(x) => x.image(),
            ImageSpec::VideoFrame { video, frame } => {
                DynamicImage::from(video.frame(*frame).unwrap())
            }
            ImageSpec::Resize {
                image,
                width,
                height,
            } => image
                .image()
                .resize_exact(*width, *height, FilterType::CatmullRom),
            ImageSpec::BlitStack { base, layers } => {
                fn srgb_to_linear(c: f32) -> f32 {
                    if c <= 0.04045 {
                        c / 12.92
                    } else {
                        ((c + 0.055) / 1.055).powf(2.4)
                    }
                }

                fn linear_to_srgb(c: f32) -> f32 {
                    if c <= 0.0031308 {
                        c * 12.92
                    } else {
                        1.055 * c.powf(1.0 / 2.4) - 0.055
                    }
                }

                fn blit_linear(dst: &mut RgbaImage, src: &RgbaImage, ox: i64, oy: i64) {
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

                            // source in linear premultiplied
                            let (sr, sg, sb, sa) = (
                                sp[0] as f32 / 255.0,
                                sp[1] as f32 / 255.0,
                                sp[2] as f32 / 255.0,
                                sp[3] as f32 / 255.0,
                            );

                            // destination in linear premultiplied
                            let da = dp[3] as f32 / 255.0;
                            let dr = srgb_to_linear(dp[0] as f32 / 255.0) * da;
                            let dg = srgb_to_linear(dp[1] as f32 / 255.0) * da;
                            let db = srgb_to_linear(dp[2] as f32 / 255.0) * da;

                            // Porter-Duff "over" (premultiplied alpha)
                            let out_a = sa + da * (1.0 - sa);
                            let out_r = sr + dr * (1.0 - sa);
                            let out_g = sg + dg * (1.0 - sa);
                            let out_b = sb + db * (1.0 - sa);

                            if out_a > 0.0 {
                                let inv_a = 1.0 / out_a;
                                dp[0] =
                                    (linear_to_srgb((out_r * inv_a).clamp(0.0, 1.0)) * 255.0) as u8;
                                dp[1] =
                                    (linear_to_srgb((out_g * inv_a).clamp(0.0, 1.0)) * 255.0) as u8;
                                dp[2] =
                                    (linear_to_srgb((out_b * inv_a).clamp(0.0, 1.0)) * 255.0) as u8;
                                dp[3] = (out_a * 255.0) as u8;
                            } else {
                                dp[0] = 0;
                                dp[1] = 0;
                                dp[2] = 0;
                                dp[3] = 0;
                            }
                        }
                    }
                }

                let mut result = base.image().to_rgba8();
                for ((x, y), image_spec) in layers {
                    let img = image_spec.image().to_rgba8();
                    blit_linear(&mut result, &img, *x as i64, *y as i64);
                }
                result.into()
            }
            _ => image::open(self.make_path()).unwrap(),
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
