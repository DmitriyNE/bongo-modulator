use crate::config::{load_config, save_config};
use crate::daemon::run_daemon;
use crate::ipc::{send_command, ControlMessage};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};

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
