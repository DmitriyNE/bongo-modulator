use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};
use tracing::error;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub fps: f32,
    pub ai_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fps: 5.0,
            ai_mode: false,
        }
    }
}

fn config_path() -> PathBuf {
    env::var_os("BONGO_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("state.json"))
}

pub fn load_config() -> Config {
    let path = config_path();
    if let Ok(data) = fs::read(&path) {
        if let Ok(cfg) = serde_json::from_slice(&data) {
            return cfg;
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_vec(cfg) {
        if let Err(e) = fs::write(&path, data) {
            error!("failed to write config: {e}");
        }
    } else {
        error!("failed to encode config");
    }
}

/// Returns the currently configured FPS.
pub fn current_fps() -> f32 {
    load_config().fps
}
