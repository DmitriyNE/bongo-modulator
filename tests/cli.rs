use bongo_modulator::config::load_config;
use bongo_modulator::{current_fps, execute, pick_frame, Cli, Commands, ModeSubcommand};
use clap::Parser;
use proptest::prelude::*;
use serial_test::serial;
use std::io::Write;
use std::os::unix::net::UnixListener;
use tempfile::tempdir;

#[derive(serde::Deserialize, Debug, PartialEq)]
enum ControlMessage {
    SetFps(u32),
    EnableAi,
    NextImage,
}

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


    #[test]
    fn parse_daemon_dir(path in "[a-zA-Z0-9][a-zA-Z0-9/_\\.-]*") {
        let args = ["bongo-modulator", "daemon", "--dir", &path];
        let cli = Cli::parse_from(&args);
        match cli.command {
            Commands::Daemon { dir, process } => {
                prop_assert_eq!(dir, Some(std::path::PathBuf::from(path)));
                prop_assert_eq!(process, String::from("hyprlock"));
            }
            _ => prop_assert!(false, "unexpected subcommand"),
        }
    }

    #[test]
    fn parse_daemon_process(name in "[a-zA-Z0-9][a-zA-Z0-9_-]*") {
        let args = ["bongo-modulator", "daemon", "--process", &name];
        let cli = Cli::parse_from(&args);
        match cli.command {
            Commands::Daemon { dir, process } => {
                prop_assert!(dir.is_none());
                prop_assert_eq!(process, name);
            }
            _ => prop_assert!(false, "unexpected subcommand"),
        }
    }

    #[test]
    #[serial]
    fn execute_sets_fps(value in 1u32..30) {
        let dir = tempdir().unwrap();
        std::env::set_var("BONGO_STATE_PATH", dir.path().join("state.json"));
        let socket = dir.path().join("sock");
        std::env::set_var("BONGO_SOCKET", &socket);

        let listener = UnixListener::bind(&socket).unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            serde_json::from_reader::<_, ControlMessage>(&mut stream).unwrap()
        });

        let cli = Cli {
            command: Commands::Mode {
                mode: ModeSubcommand::Fps { fps: value },
            },
        };
        execute(cli);

        let received = handle.join().unwrap();
        prop_assert_eq!(received, ControlMessage::SetFps(value));
        prop_assert_eq!(current_fps(), value);
        let cfg = load_config();
        prop_assert!(!cfg.ai_mode);
    }

    #[test]
    #[serial]
    fn execute_enables_ai(value in Just(())) {
        let _ = value; // suppress unused param
        let dir = tempdir().unwrap();
        std::env::set_var("BONGO_STATE_PATH", dir.path().join("state.json"));
        let socket = dir.path().join("sock");
        std::env::set_var("BONGO_SOCKET", &socket);

        let listener = UnixListener::bind(&socket).unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            serde_json::from_reader::<_, ControlMessage>(&mut stream).unwrap()
        });

        let cli = Cli { command: Commands::Mode { mode: ModeSubcommand::Ai } };
        execute(cli);

        let received = handle.join().unwrap();
        prop_assert_eq!(received, ControlMessage::EnableAi);
    }
}

#[test]
fn parse_ai_mode() {
    let args = ["bongo-modulator", "mode", "ai"];
    let cli = Cli::parse_from(&args);
    match cli.command {
        Commands::Mode {
            mode: ModeSubcommand::Ai,
        } => {}
        _ => panic!("unexpected subcommand"),
    }
}

#[test]
#[serial]
fn next_image_uses_daemon() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("sock");
    std::env::set_var("BONGO_SOCKET", &socket);

    let listener = UnixListener::bind(&socket).unwrap();
    let img_path = dir.path().join("img.png");
    let return_path = img_path.clone();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let msg: ControlMessage = serde_json::from_reader(&mut stream).unwrap();
        assert_eq!(msg, ControlMessage::NextImage);
        stream
            .write_all(return_path.to_string_lossy().as_bytes())
            .unwrap();
    });

    let result = bongo_modulator::next_image_path();
    handle.join().unwrap();
    assert_eq!(result.unwrap(), img_path);
}

#[test]
fn pick_frame_empty_directory() {
    let dir = tempdir().unwrap();
    let mut index = 0usize;
    let result = pick_frame(dir.path(), &mut index);
    assert!(result.is_none());
}
