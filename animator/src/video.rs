use crate::data::FileSpec;
use crate::data::cache;
use crate::image::ImageSpec;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSpec {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub images: Vec<ImageSpec>,
}

impl VideoSpec {
    pub fn make_video(&self, path: &Path) {
        // Create a temp file to hold the list of image paths
        let list_file = tempfile::NamedTempFile::new().unwrap();
        let list_file_path = list_file.path().to_path_buf();

        let mut writer = std::io::BufWriter::new(list_file);

        for image in &self.images {
            let p = image.get_path();
            writeln!(
                writer,
                "file '{}'",
                std::fs::canonicalize(p).unwrap().display()
            )
            .unwrap();
        }
        writer.flush().unwrap();

        #[allow(unused)]
        let output = std::process::Command::new("ffmpeg")
            .args([
                "-f",
                "concat",
                "-safe",
                "0",
                "-r",
                self.fps.to_string().as_str(),
                "-i",
                list_file_path.to_str().unwrap(),
                "-vf",
                format!("scale={}:{}", self.width, self.height).as_str(),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-y", // overwrite
                path.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        // println!("status: {}", output.status);
        // println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        // println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    pub fn get_path(&self) -> PathBuf {
        cache().get_file(&FileSpec::Video(self.clone()))
    }
}
