use crate::audio::AudioSpec;
use crate::data::FileSpec;
use crate::data::cache;
use crate::image::ImageSpec;
use crate::timeline::Timeline;
use image::Rgba32FImage;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoSpec {
    File { path: PathBuf },
    Compiled(VideoCompiledSpec),
}

impl VideoSpec {
    pub fn make_video(&self, path: &Path) {
        match self {
            VideoSpec::File { path: video_path } => {
                std::fs::copy(video_path, path).unwrap();
            }
            VideoSpec::Compiled(x) => x.make_video(path),
        }
    }

    pub fn make_path(&self) -> PathBuf {
        cache().make_file(&FileSpec::Video(self.clone()))
    }

    pub fn size(&self) -> (u32, u32) {
        let output = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height",
                "-of",
                "json",
                self.make_path().to_str().unwrap(),
            ])
            .output()
            .unwrap();

        if !output.status.success() {
            panic!();
        }

        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

        let stream = v["streams"].get(0).ok_or("no video stream").unwrap();
        let width = stream["width"].as_u64().ok_or("missing width").unwrap() as u32;
        let height = stream["height"].as_u64().ok_or("missing height").unwrap() as u32;

        (width, height)
    }

    pub fn fps(&self) -> f64 {
        let output = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=avg_frame_rate",
                "-of",
                "json",
                self.make_path().to_str().unwrap(),
            ])
            .output()
            .unwrap();

        if !output.status.success() {
            panic!();
        }

        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

        let stream = v["streams"].get(0).ok_or("no video stream").unwrap();

        let fps_str = stream["avg_frame_rate"]
            .as_str()
            .ok_or("missing fps")
            .unwrap();

        // the output is of the form "num/den"
        let parts: Vec<&str> = fps_str.split('/').collect();
        if parts.len() != 2 {
            panic!("invalid fps format");
        }
        let num: f64 = parts[0].parse().unwrap();
        let den: f64 = parts[1].parse().unwrap();
        num / den
    }

    pub fn num_frames(&self) -> usize {
        let output = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-count_frames",
                "-show_entries",
                "stream=nb_read_frames",
                "-of",
                "json",
                self.make_path().to_str().unwrap(),
            ])
            .output()
            .unwrap();

        if !output.status.success() {
            panic!();
        }

        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

        let stream = v["streams"].get(0).expect("no video stream");

        let frames_str = stream["nb_read_frames"]
            .as_str()
            .expect("missing frame count");

        frames_str.parse::<usize>().expect("invalid frame count")
    }

    pub fn frame(&self, frame: usize) -> Option<Rgba32FImage> {
        let (width, height) = self.size();
        let frame_size = (width as usize) * (height as usize) * 3;

        let output = std::process::Command::new("ffmpeg")
            .args([
                "-i",
                self.make_path().to_str().unwrap(),
                "-vf",
                &format!("select=eq(n\\,{})", frame),
                "-vframes",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "pipe:1",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        if output.stdout.len() != frame_size {
            return None;
        }

        let image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::from_raw(width, height, output.stdout).unwrap();
        Some(image::DynamicImage::from(image).into())
    }

    pub fn image_timeline(&self) -> VideoImageTimeline {
        VideoImageTimeline {
            video: self.clone(),
            frame_count: self.num_frames(),
            fps: self.fps(),
        }
    }

    pub fn audio(&self) -> AudioSpec {
        AudioSpec::File {
            path: self.make_path(),
        }
    }
}

pub struct VideoImageTimeline {
    video: VideoSpec,
    frame_count: usize,
    fps: f64,
}

impl Timeline<Option<ImageSpec>> for VideoImageTimeline {
    fn at_time(&self, t: f64) -> Option<ImageSpec> {
        let i = (t * self.fps).floor() as i64;
        if i < 0 {
            return None;
        }
        let i = i as usize;
        if i >= self.frame_count {
            return None;
        }
        Some(ImageSpec::VideoFrame {
            video: self.video.clone(),
            frame: i,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoAudioClip {
    pub at_t: f64,
    pub spec: AudioSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoCompiledSpec {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub images: Vec<ImageSpec>,
    pub audio: Vec<VideoAudioClip>,
}

impl VideoCompiledSpec {
    pub fn make_video(&self, path: &Path) {
        // Create a temp file to hold the list of image paths
        let list_file = tempfile::NamedTempFile::new().unwrap();
        let list_file_path = list_file.path().to_path_buf();

        let mut writer = std::io::BufWriter::new(list_file);

        let frame_duration = 1.0 / self.fps;

        for image in &self.images {
            let p = image.make_path();
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
            .map(|VideoAudioClip { at_t, spec }| {
                if *at_t >= 0.0 {
                    AudioClip {
                        path: spec.make_path(),
                        start_ms: (at_t * 1000.0) as u64,
                    }
                } else {
                    let cut_clip = AudioSpec::Sub {
                        spec: Box::new(spec.clone()),
                        from: Some(-at_t),
                        to: None,
                    };
                    AudioClip {
                        path: cut_clip.make_path(),
                        start_ms: 0,
                    }
                }
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
                    "{}amix=inputs={}[aout];[aout]apad[aout2]",
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
                "[aout2]".to_string(),
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

        let output = std::process::Command::new("ffmpeg")
            .args(args)
            .output()
            .unwrap();
        if !output.status.success() {
            println!("status: {}", output.status);
            println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
            println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
            panic!("ffmpeg failed");
        }
    }
}
