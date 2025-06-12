use std::path::{Path, PathBuf};
use tracing::error;

pub fn image_dir() -> PathBuf {
    std::env::var_os("BONGO_IMAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("images"))
}

fn load_frames(dir: &Path) -> Vec<PathBuf> {
    match std::fs::read_dir(dir) {
        Ok(rd) => {
            let mut paths: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect();
            paths.sort();
            if paths.is_empty() {
                error!("no frames found in {}", dir.display());
            }
            paths
        }
        Err(_) => Vec::new(),
    }
}

pub struct FrameCache {
    frames: Vec<PathBuf>,
    index: usize,
}

impl FrameCache {
    pub fn new(dir: &Path) -> Self {
        Self {
            frames: load_frames(dir),
            index: 0,
        }
    }

    pub fn next_frame(&mut self) -> Option<PathBuf> {
        if self.frames.is_empty() {
            None
        } else {
            let path = self.frames[self.index % self.frames.len()].clone();
            self.index = (self.index + 1) % self.frames.len();
            Some(path)
        }
    }
}

pub fn pick_frame(dir: &Path, index: &mut usize) -> Option<PathBuf> {
    let frames = load_frames(dir);
    if frames.is_empty() {
        None
    } else {
        let path = frames[*index % frames.len()].clone();
        *index = (*index + 1) % frames.len();
        Some(path)
    }
}
