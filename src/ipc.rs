use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::{env, io};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ControlMessage {
    SetFps(f32),
    EnableAi,
    NextImage,
}

pub fn socket_path() -> PathBuf {
    env::var_os("BONGO_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/bongo.sock"))
}

pub fn send_command(msg: ControlMessage) -> io::Result<Option<String>> {
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
