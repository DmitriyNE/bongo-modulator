use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use std::{env, fs};
use tracing::{error, info};

fn config_path() -> PathBuf {
    env::var_os("BONGO_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("state.json"))
}

fn load_config() -> Config {
    let path = config_path();
    if let Ok(data) = fs::read(&path) {
        if let Ok(cfg) = serde_json::from_slice(&data) {
            return cfg;
        }
    }
    Config::default()
}

fn save_config(cfg: &Config) {
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

fn socket_path() -> PathBuf {
    env::var_os("BONGO_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/bongo.sock"))
}

fn image_dir() -> PathBuf {
    env::var_os("BONGO_IMAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("images"))
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum ControlMessage {
    SetFps(u32),
    EnableAi,
    NextImage,
}
fn send_command(msg: ControlMessage) -> std::io::Result<Option<String>> {
    let path = socket_path();
    match UnixStream::connect(&path) {
        Ok(mut stream) => {
            serde_json::to_writer(&mut stream, &msg)?;
            stream.flush()?;
            let _ = stream.shutdown(Shutdown::Write);

            if matches!(msg, ControlMessage::NextImage) {
                let mut buf = String::new();
                stream.read_to_string(&mut buf)?;
                Ok(Some(buf))
            } else {
                Ok(None)
            }
        }
        Err(e) => Err(e),
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    fps: u32,
    ai_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fps: 5,
            ai_mode: false,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "bongo-modulator",
    version,
    about = "Hyprlock bongo cat modulator"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the signalling service
    Daemon {
        /// Directory containing frames
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Print the path to the next image
    NextImage,
    /// Configure operation mode
    Mode {
        #[command(subcommand)]
        mode: ModeSubcommand,
    },
}

#[derive(Subcommand)]
pub enum ModeSubcommand {
    /// Enable AI mode (stub)
    Ai,
    /// Set manual FPS
    Fps { fps: u32 },
}

pub fn run_cli() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    execute(cli);
}

pub fn execute(cli: Cli) {
    match cli.command {
        Commands::Daemon { dir } => run_daemon(dir),
        Commands::NextImage => next_image(),
        Commands::Mode { mode } => match mode {
            ModeSubcommand::Ai => enable_ai(),
            ModeSubcommand::Fps { fps } => set_fps(fps),
        },
    }
}

fn run_daemon(dir: Option<PathBuf>) {
    if let Some(d) = dir {
        env::set_var("BONGO_IMAGE_DIR", &d);
    }
    info!("daemon started");

    let cfg = load_config();
    let fps = Arc::new(AtomicU32::new(cfg.fps.max(1)));
    let ai_mode = Arc::new(AtomicBool::new(cfg.ai_mode));

    let sock_path = socket_path();
    let _ = fs::remove_file(&sock_path);
    let listener = match UnixListener::bind(&sock_path) {
        Ok(l) => l,
        Err(e) => {
            error!("failed to bind socket: {e}");
            return;
        }
    };

    let fps_ctrl = fps.clone();
    let ai_ctrl = ai_mode.clone();
    let indexes: Arc<Mutex<HashMap<PathBuf, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let idx_ctrl = indexes.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => {
                    if let Ok(msg) = serde_json::from_reader::<_, ControlMessage>(&mut s) {
                        match msg {
                            ControlMessage::SetFps(v) => {
                                fps_ctrl.store(v.max(1), Ordering::Relaxed)
                            }
                            ControlMessage::EnableAi => ai_ctrl.store(true, Ordering::Relaxed),
                            ControlMessage::NextImage => {
                                let dir = image_dir();
                                let reply = {
                                    let mut idx = idx_ctrl.lock().unwrap();
                                    let entry = idx.entry(dir.clone()).or_default();
                                    pick_frame(&dir, entry)
                                };
                                if let Some(p) = reply {
                                    let _ = s.write_all(p.to_string_lossy().as_bytes());
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("failed to accept connection: {e}"),
            }
        }
    });

    loop {
        if let Err(e) = Command::new("pkill")
            .args(["-SIGUSR2", "hyprlock"])
            .status()
        {
            error!("failed to signal hyprlock: {e}");
        }
        let delay = fps.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_secs_f64(1.0 / delay as f64));
    }
}

pub fn pick_frame(dir: &Path, index: &mut usize) -> Option<PathBuf> {
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
                None
            } else {
                let path = paths[*index % paths.len()].clone();
                *index = (*index + 1) % paths.len();
                Some(path)
            }
        }
        Err(_) => None,
    }
}

pub fn next_image_path() -> Option<PathBuf> {
    match send_command(ControlMessage::NextImage) {
        Ok(Some(p)) if !p.is_empty() => Some(PathBuf::from(p)),
        _ => None,
    }
}

fn next_image() {
    match next_image_path() {
        Some(path) => println!("{}", path.display()),
        None => error!("daemon did not return an image"),
    }
}

fn enable_ai() {
    let mut cfg = load_config();
    cfg.ai_mode = true;
    save_config(&cfg);
    let _ = send_command(ControlMessage::EnableAi);
    info!("AI mode enabled (stub)");
}

fn set_fps(fps: u32) {
    let mut cfg = load_config();
    cfg.fps = fps;
    save_config(&cfg);
    let _ = send_command(ControlMessage::SetFps(fps));
    info!("manual fps set to {fps}");
}

/// Returns the currently configured FPS.
pub fn current_fps() -> u32 {
    load_config().fps
}
