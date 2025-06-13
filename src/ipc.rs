use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{env, io};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ControlMessage {
    SetFps(u32),
    EnableAi,
    NextImage,
}

pub fn socket_path() -> PathBuf {
    if let Some(path) = env::var_os("BONGO_SOCKET") {
        PathBuf::from(path)
    } else if let Some(dir) = env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(dir).join("bongo.sock")
    } else {
        env::temp_dir().join("bongo.sock")
    }
}

pub fn send_command(msg: ControlMessage) -> io::Result<Option<String>> {
    let path = socket_path();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match UnixStream::connect(&path) {
            Ok(mut stream) => {
                serde_json::to_writer(&mut stream, &msg)?;
                stream.flush()?;
                let _ = stream.shutdown(Shutdown::Write);

                if matches!(msg, ControlMessage::NextImage) {
                    let mut buf = String::new();
                    stream.read_to_string(&mut buf)?;
                    return Ok(Some(buf));
                } else {
                    return Ok(None);
                }
            }
            Err(e) => {
                if Instant::now() >= deadline {
                    return Err(e);
                }
                sleep(Duration::from_millis(10));
            }
        }
    }
}
