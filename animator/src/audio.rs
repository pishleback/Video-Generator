use crate::data::{FileSpec, cache};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioSpec {
    File { path: PathBuf },
}

impl AudioSpec {
    pub fn extension(&self) -> &str {
        match self {
            AudioSpec::File { path } => path.extension().unwrap().to_str().unwrap(),
        }
    }

    pub fn make_audio(&self, path: &Path) {
        match self {
            AudioSpec::File { path: audio_path } => {
                std::fs::copy(audio_path, path).unwrap();
            }
        }
    }

    pub fn get_path(&self) -> PathBuf {
        cache().get_file(&FileSpec::Audio(self.clone()))
    }
}
