use crate::{
    audio::AudioSpec,
    image::ImageSpec,
    shape::ShapeSpec,
    video::{VideoCompiledSpec, VideoSpec},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileSpec {
    Image(ImageSpec),
    Video(VideoSpec),
    Shape(ShapeSpec),
    Audio(AudioSpec),
}

impl FileSpec {
    fn make_file(&self, path: &Path) {
        match self {
            FileSpec::Image(x) => x.make_image(path),
            FileSpec::Video(x) => x.make_video(path),
            FileSpec::Shape(x) => x.make_shape(path),
            FileSpec::Audio(x) => x.make_audio(path),
        }
    }

    fn extension(&self) -> &str {
        match self {
            FileSpec::Image(_) => "png",
            FileSpec::Video(_) => "mp4",
            FileSpec::Shape(_) => "txt",
            FileSpec::Audio(audio_spec) => audio_spec.extension(),
        }
    }
}

static CACHE: OnceLock<DataCache> = OnceLock::new();
pub fn cache() -> &'static DataCache {
    CACHE.get_or_init(|| DataCache::new(Path::new("data_cache").to_path_buf()))
}

pub struct DataCache {
    path: PathBuf,
    // None where cache entries fail to parse
    cache: Mutex<Vec<Option<DataCacheEntry>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataCacheEntry {
    spec: FileSpec,
    path: PathBuf,
}

impl DataCache {
    pub fn new(path: PathBuf) -> Self {
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
            std::fs::File::create_new(path.join("entries.txt")).unwrap();
        }
        assert!(path.exists());
        assert!(path.join("entries.txt").exists());

        let cache = Mutex::new(
            std::fs::read_to_string(path.join("entries.txt"))
                .unwrap()
                .lines()
                .map(|line| match serde_json::from_str::<DataCacheEntry>(line) {
                    Ok(x) => Some(x),
                    Err(e) => {
                        println!("Ignoring bad cache entry `{line}`: {}", e);
                        None
                    }
                })
                .collect(),
        );

        Self {
            path: path.clone(),
            cache,
        }
    }

    pub fn get_file(&self, file_spec: &FileSpec) -> PathBuf {
        let mut cache = self.cache.lock().unwrap();

        // Check if the file exists in the cache already
        for cache_entry in cache.iter().flatten() {
            if &cache_entry.spec == file_spec {
                let file_path = if cache_entry.path.exists() {
                    cache_entry.path.clone()
                } else {
                    let file_path = cache_entry.path.clone();
                    drop(cache);
                    println!("Regenerating file: {:?}", file_spec);
                    file_spec.make_file(&file_path);
                    assert!(file_path.exists());
                    file_path
                };
                return file_path;
            }
        }

        // The file does not exist in the cache so we'll generate it now
        let n = cache.len();
        let file_path = self.path.join(format!("{}.{}", n, file_spec.extension()));
        let cache_entry = DataCacheEntry {
            spec: file_spec.clone(),
            path: file_path.clone(),
        };
        let mut cache_file = std::fs::OpenOptions::new()
            .append(true)
            .open(self.path.join("entries.txt").clone())
            .unwrap();
        writeln!(
            cache_file,
            "{}",
            serde_json::to_string(&cache_entry).unwrap()
        )
        .unwrap();
        cache.push(Some(cache_entry));
        drop(cache);

        println!("Generating file: {:?}", file_spec);
        file_spec.make_file(&file_path);

        assert!(file_path.exists());

        file_path
    }
}
