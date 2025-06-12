use bongo_modulator::{current_fps, execute, pick_frame, Cli, Commands, ModeSubcommand};
use clap::Parser;
use proptest::prelude::*;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use tempfile::tempdir;

#[derive(serde::Deserialize, Debug, PartialEq)]
enum ControlMessage {
    SetFps(u32),
    EnableAi,
    NextImage { dir: PathBuf },
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
    }
}

#[test]
fn next_image_uses_daemon() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("sock");
    std::env::set_var("BONGO_SOCKET", &socket);

    let listener = UnixListener::bind(&socket).unwrap();
    let img_path = dir.path().join("img.png");
    let return_path = img_path.clone();
    let server_dir = PathBuf::from("images");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let msg: ControlMessage = serde_json::from_reader(&mut stream).unwrap();
        assert_eq!(msg, ControlMessage::NextImage { dir: server_dir });
        stream
            .write_all(return_path.to_string_lossy().as_bytes())
            .unwrap();
    });

    let result = bongo_modulator::next_image_path(PathBuf::from("images"));
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
