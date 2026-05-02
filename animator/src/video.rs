use crate::audio::AudioSpec;
use crate::data::FileSpec;
use crate::data::cache;
use crate::image::ImageSpec;
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

    pub fn get_path(&self) -> PathBuf {
        cache().get_file(&FileSpec::Video(self.clone()))
    }

    pub fn get_size(&self) -> (u32, u32) {
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
                self.get_path().to_str().unwrap(),
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

    pub fn get_fps(&self) -> f32 {
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
                self.get_path().to_str().unwrap(),
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

        let parts: Vec<&str> = fps_str.split('/').collect();
        if parts.len() != 2 {
            panic!("invalid fps format");
        }

        let num: f32 = parts[0].parse().unwrap();
        let den: f32 = parts[1].parse().unwrap();

        num / den
    }

    pub fn get_images(&self) -> Vec<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
        let (width, height) = self.get_size();
        let frame_size = (width as usize) * (height as usize) * 3;

        let mut child = std::process::Command::new("ffmpeg")
            .args([
                "-i",
                self.get_path().to_str().unwrap(),
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-vsync",
                "0", // no duplication/drop
                "pipe:1",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().ok_or("No stdout").unwrap();
        let mut reader = std::io::BufReader::new(stdout);

        let mut images = vec![];
        loop {
            let mut buffer = vec![0u8; frame_size];

            match std::io::Read::read_exact(&mut reader, &mut buffer) {
                Ok(_) => {
                    let img: ::image::ImageBuffer<::image::Rgb<u8>, _> =
                        ::image::ImageBuffer::from_raw(width, height, buffer)
                            .ok_or("Invalid buffer size")
                            .unwrap();
                    images.push(img);
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break; // no more frames
                }
                Err(e) => {
                    panic!("{}", e);
                }
            }
        }
        let ecode = child.wait().expect("failed to wait on child");
        assert!(ecode.success());
        images
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
            .map(|VideoAudioClip { at_t, spec }| {
                if *at_t >= 0.0 {
                    AudioClip {
                        path: spec.get_path(),
                        start_ms: (at_t * 1000.0) as u64,
                    }
                } else {
                    let cut_clip = AudioSpec::Sub {
                        spec: Box::new(spec.clone()),
                        from: Some(-at_t),
                        to: None,
                    };
                    AudioClip {
                        path: cut_clip.get_path(),
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
