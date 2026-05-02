use crate::audio::AudioSpec;
use crate::data::FileSpec;
use crate::data::cache;
use crate::image::ImageSpec;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoAudioClip {
    pub at_t: f64,
    pub spec: AudioSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSpec {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub images: Vec<ImageSpec>,
    pub audio: Vec<VideoAudioClip>,
}

impl VideoSpec {
    pub fn make_video(&self, path: &Path) {
        // Create a temp file to hold the list of image paths
        let list_file = tempfile::NamedTempFile::new().unwrap();
        let list_file_path = list_file.path().to_path_buf();

        let mut writer = std::io::BufWriter::new(list_file);

        let frame_duration = 1.0 / self.fps;

        for image in &self.images {
            let p = image.get_path();
            writeln!(
                writer,
                "file '{}'",
                std::fs::canonicalize(p).unwrap().display()
            )
            .unwrap();
            writeln!(writer, "duration {}", frame_duration).unwrap();
        }
        writer.flush().unwrap();

        struct AudioClip {
            path: PathBuf,
            start_ms: u64, // when it should start in the video
        }

        let audio_clips = self
            .audio
            .iter()
            .map(|VideoAudioClip { at_t, spec }| AudioClip {
                path: spec.get_path(),
                start_ms: (at_t * 1000.0) as u64,
            })
            .collect::<Vec<_>>();

        let mut args = vec![];

        args.append(&mut vec![
            "-f".to_string(),
            "concat".to_string(),
            "-safe".to_string(),
            "0".to_string(),
            "-r".to_string(),
            format!("{}", self.fps),
            "-i".to_string(),
            list_file_path.to_string_lossy().into(),
        ]);
        if !audio_clips.is_empty() {
            for clip in &audio_clips {
                args.append(&mut vec![
                    "-i".to_string(),
                    clip.path.to_string_lossy().into(),
                ]);
            }
            let filter = {
                let mut filter = String::new();
                for (i, clip) in audio_clips.iter().enumerate() {
                    let input_index = i + 1; // 0 is video
                    let delay = clip.start_ms;
                    filter.push_str(&format!(
                        "[{}:a]adelay={}|{}[a{}];",
                        input_index, delay, delay, i
                    ));
                }
                let inputs: Vec<String> = (0..audio_clips.len())
                    .map(|i| format!("[a{}]", i))
                    .collect();
                filter.push_str(&format!(
                    "{}amix=inputs={}[aout]",
                    inputs.join(""),
                    audio_clips.len()
                ));
                filter
            };
            args.append(&mut vec![
                "-filter_complex".to_string(),
                filter,
                "-map".to_string(),
                "0:v".to_string(),
                "-map".to_string(),
                "[aout]".to_string(),
                "-shortest".to_string(),
            ]);
        }
        args.append(&mut vec![
            "-vf".to_string(),
            format!("scale={}:{}", self.width, self.height),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-c:a".to_string(),
            "mp3".to_string(), // "aac" is a more modern alternative, but does not work in VS code player
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-y".to_string(), // overwrite
            path.to_string_lossy().into(),
        ]);

        #[allow(unused)]
        let output = std::process::Command::new("ffmpeg")
            .args(args)
            .output()
            .unwrap();

        println!("status: {}", output.status);
        println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    pub fn get_path(&self) -> PathBuf {
        cache().get_file(&FileSpec::Video(self.clone()))
    }
}
