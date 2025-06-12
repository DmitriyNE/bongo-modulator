use crate::config::load_config;
use crate::frame::{image_dir, FrameCache};
use crate::ipc::ControlMessage;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use std::{env, fs};
use tracing::{error, info};

pub fn run_daemon(dir: Option<PathBuf>) {
    if let Some(d) = dir {
        env::set_var("BONGO_IMAGE_DIR", &d);
    }
    info!("daemon started");

    let cfg = load_config();
    let fps = Arc::new(AtomicU32::new(cfg.fps.max(1)));
    let ai_mode = Arc::new(AtomicBool::new(cfg.ai_mode));

    let sock_path = crate::ipc::socket_path();
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
    let caches: Arc<Mutex<HashMap<PathBuf, FrameCache>>> = Arc::new(Mutex::new(HashMap::new()));
    let cache_ctrl = caches.clone();
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
                                    let mut caches = cache_ctrl.lock().unwrap();
                                    let cache = caches
                                        .entry(dir.clone())
                                        .or_insert_with(|| FrameCache::new(&dir));
                                    cache.next_frame()
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
