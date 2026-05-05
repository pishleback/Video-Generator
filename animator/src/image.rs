use crate::{
    colour::ColourRgba,
    data::{FileSpec, cache},
    shape::ShapeImage,
    video::VideoSpec,
};
use image::{DynamicImage, ImageBuffer, Rgba, imageops::FilterType};
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
        width: u32,
        height: u32,
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
            ImageSpec::BlitStack {
                width,
                height,
                layers,
            } => {
                let mut result = ImageBuffer::from_fn(*width, *height, |_x, _y| Rgba([0, 0, 0, 0]));
                for ((x, y), image_spec) in layers {
                    let img = image_spec.image();
                    image::imageops::overlay(&mut result, &img, *x as i64, *y as i64);
                }
                DynamicImage::from(result)
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
                "-scale-to",
                self.scale.to_string().as_str(),
                tmp_dir.path().join("out").to_str().unwrap(),
            ])
            .status()
            .unwrap();

        std::fs::copy(tmp_dir.path().join("out-1.png"), path).unwrap();
    }
}
