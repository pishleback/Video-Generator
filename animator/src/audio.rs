use crate::data::{FileSpec, cache};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioSpec {
    Null {
        extension: String,
    },
    File {
        path: PathBuf,
    },
    Sub {
        spec: Box<AudioSpec>,
        from: Option<f64>,
        to: Option<f64>,
    },
}

impl AudioSpec {
    pub fn extension(&self) -> &str {
        match self {
            AudioSpec::Null { extension } => extension.as_str(),
            AudioSpec::File { path } => path.extension().unwrap().to_str().unwrap(),
            AudioSpec::Sub { spec, .. } => spec.extension(),
        }
    }

    pub fn make_audio(&self, path: &Path) {
        match self {
            AudioSpec::Null { .. } => {
                // generate very short silence
                let status = std::process::Command::new("ffmpeg")
                    .args([
                        "-f".to_string(),
                        "lavfi".to_string(),
                        "-i".to_string(),
                        "anullsrc=r=44100:cl=mono".to_string(),
                        "-t".to_string(),
                        "0.10".to_string(),
                        "-q:a".to_string(),
                        "9".to_string(),
                        "-acodec".to_string(),
                        "libmp3lame".to_string(),
                        path.to_string_lossy().to_string(),
                    ])
                    .status()
                    .unwrap();
                if !status.success() {
                    panic!();
                }
            }
            AudioSpec::File { path: audio_path } => {
                std::fs::copy(audio_path, path).unwrap();
            }
            AudioSpec::Sub { spec, from, to } => {
                if from.is_none() && to.is_none() {
                    spec.make_audio(path);
                } else {
                    let duration = spec.duration();

                    let mut args = vec![];
                    args.append(&mut vec![
                        "-i".to_string(),
                        spec.make_path().to_string_lossy().into(),
                    ]);
                    if let Some(from) = from {
                        if *from >= duration {
                            AudioSpec::Null {
                                extension: self.extension().to_string(),
                            }
                            .make_audio(path);
                            return;
                        }
                        args.append(&mut vec!["-ss".to_string(), format!("{}", from)]);
                    }
                    if let Some(to) = to {
                        if *to <= 0.0 {
                            AudioSpec::Null {
                                extension: self.extension().to_string(),
                            }
                            .make_audio(path);
                            return;
                        }
                        args.append(&mut vec!["-to".to_string(), format!("{}", to)]);
                    }
                    if let Some(from) = from
                        && let Some(to) = to
                        && to <= from
                    {
                        AudioSpec::Null {
                            extension: self.extension().to_string(),
                        }
                        .make_audio(path);
                        return;
                    }
                    args.append(&mut vec![
                        // "-c".to_string(),
                        // "copy".to_string(),
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
        }
    }

    pub fn make_path(&self) -> PathBuf {
        cache().make_file(&FileSpec::Audio(self.clone()))
    }

    pub fn duration(&self) -> f64 {
        let path = self.make_path();

        let output = std::process::Command::new("ffprobe")
            .arg("-i")
            .arg(&path)
            .args([
                "-show_entries",
                "format=duration",
                "-v",
                "quiet",
                "-of",
                "json",
            ])
            .output()
            .expect("failed to execute ffprobe");

        if !output.status.success() {
            return 0.0;
        }

        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("failed to parse ffprobe output");

        json["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0)
    }
}
