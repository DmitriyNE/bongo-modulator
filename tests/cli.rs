use bongo_modulator::{Cli, Commands, ModeSubcommand};
use clap::Parser;
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_fps(value in 0u32..1000) {
        let args = ["bongo-modulator", "mode", "fps", &value.to_string()];
        let cli = Cli::parse_from(&args);
        match cli.command {
            Commands::Mode { mode: ModeSubcommand::Fps { fps } } => prop_assert_eq!(fps, value),
            _ => prop_assert!(false, "unexpected subcommand"),
        }
    }
}
