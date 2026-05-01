use crate::{
    colour::Colour,
    data::{FileSpec, cache},
    shape::ShapeImage,
};
use image::{ImageBuffer, Rgba};
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
        colour: Colour,
    },
    BlitStack {
        width: u32,
        height: u32,
        images: Vec<((f64, f64), ImageSpec)>,
    },
    Latex(LatexImage),
    Shape(ShapeImage),
}

impl ImageSpec {
    pub fn make_image(&self, path: &Path) {
        match self {
            ImageSpec::Latex(x) => x.make_image(path),
            ImageSpec::Shape(x) => x.make_image(path),
            ImageSpec::Filled {
                width,
                height,
                colour,
            } => {
                let img = ImageBuffer::from_fn(*width, *height, |_x, _y| colour.to_rgba());
                img.save(path).unwrap();
            }
            ImageSpec::BlitStack {
                width,
                height,
                images,
            } => {
                let mut result = ImageBuffer::from_fn(*width, *height, |_x, _y| Rgba([0, 0, 0, 0]));
                for ((x, y), image_spec) in images {
                    let img = image_spec.get_image();
                    image::imageops::overlay(&mut result, &img, *x as i64, *y as i64);
                }
                result.save(path).unwrap()
            }
        }
    }

    pub fn get_path(&self) -> PathBuf {
        cache().get_file(&FileSpec::Image(self.clone()))
    }

    pub fn get_image(&self) -> image::DynamicImage {
        image::open(self.get_path()).unwrap()
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
