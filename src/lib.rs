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
    Daemon,
    /// Print the path to the next image
    NextImage {
        /// Directory containing frames
        #[arg(short, long, default_value = "images")]
        dir: PathBuf,
    },
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
        Commands::Daemon => run_daemon(),
        Commands::NextImage { dir } => next_image(dir),
        Commands::Mode { mode } => match mode {
            ModeSubcommand::Ai => enable_ai(),
            ModeSubcommand::Fps { fps } => set_fps(fps),
        },
    }
}

fn run_daemon() {
    info!("daemon started");
    // TODO: detect hyprlock and send SIGUSR2 periodically
}

fn next_image(dir: PathBuf) {
    match std::fs::read_dir(&dir).and_then(|mut entries| {
        entries
            .find(|res| res.as_ref().map(|e| e.path().is_file()).unwrap_or(false))
            .map(|res| res.map(|e| e.path()))
            .transpose()
    }) {
        Ok(Some(path)) => println!("{}", path.display()),
        Ok(None) => error!("no images found in {}", dir.display()),
        Err(e) => error!("failed to read {dir:?}: {e}", dir = dir, e = e),
    }
}

fn enable_ai() {
    info!("AI mode enabled (stub)");
}

fn set_fps(fps: u32) {
    info!("manual fps set to {fps}");
}
