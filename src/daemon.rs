use crate::config::load_config;
use crate::frame::{image_dir, FrameCache};
use crate::ipc::ControlMessage;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use std::{env, fs};
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tracing::{debug, error, info, trace};

fn wait_for_process(name: &str, sys: &mut System) -> Vec<Pid> {
    loop {
        sys.refresh_processes(ProcessesToUpdate::All, false);
        let pids: Vec<Pid> = sys
            .processes_by_name(std::ffi::OsStr::new(name))
            .map(|p| p.pid())
            .collect();
        if !pids.is_empty() {
            return pids;
        }
        info!(proc = %name, "process not found; waiting");
        std::thread::sleep(Duration::from_secs(1));
    }
}

pub fn run_daemon(dir: Option<PathBuf>, process: String) {
    if let Some(d) = dir {
        env::set_var("BONGO_IMAGE_DIR", &d);
        debug!(dir = %d.display(), "using custom image directory");
    }
    info!("daemon started");

    let cfg = load_config();
    debug!(fps = cfg.fps, ai_mode = cfg.ai_mode, "loaded configuration");
    let fps = Arc::new(AtomicU32::new(cfg.fps.max(1)));
    let ai_mode = Arc::new(AtomicBool::new(cfg.ai_mode));

    let sock_path = crate::ipc::socket_path();
    if fs::remove_file(&sock_path).is_ok() {
        trace!(path = %sock_path.display(), "removed stale socket");
    }
    let listener = match UnixListener::bind(&sock_path) {
        Ok(l) => {
            debug!(path = %sock_path.display(), "socket bound");
            l
        }
        Err(e) => {
            error!("failed to bind socket: {e}");
            return;
        }
    };

    let fps_ctrl = fps.clone();
    let ai_ctrl = ai_mode.clone();
    let caches: Arc<Mutex<HashMap<PathBuf, FrameCache>>> = Arc::new(Mutex::new(HashMap::new()));
    let cache_ctrl = caches.clone();
    debug!("starting IPC thread");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => {
                    debug!("connection accepted");
                    if let Ok(msg) = serde_json::from_reader::<_, ControlMessage>(&mut s) {
                        debug!(?msg, "received message");
                        match msg {
                            ControlMessage::SetFps(v) => {
                                debug!(fps = v, "updating fps");
                                fps_ctrl.store(v.max(1), Ordering::Relaxed)
                            }
                            ControlMessage::EnableAi => {
                                debug!("enabling AI mode");
                                ai_ctrl.store(true, Ordering::Relaxed)
                            }
                            ControlMessage::NextImage => {
                                trace!("next image requested");
                                let dir = image_dir();
                                let reply = {
                                    let mut caches = cache_ctrl.lock().unwrap();
                                    let cache = caches
                                        .entry(dir.clone())
                                        .or_insert_with(|| FrameCache::new(&dir));
                                    cache.next_frame()
                                };
                                if let Some(p) = &reply {
                                    trace!(path = %p.display(), "sending frame path");
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

    let mut sys = System::new();
    let mut pids = wait_for_process(&process, &mut sys);

    loop {
        sys.refresh_processes(ProcessesToUpdate::All, false);

        pids.retain(|pid| {
            if let Some(proc_) = sys.process(*pid) {
                if proc_.name() != std::ffi::OsStr::new(&process) {
                    return false;
                }
                trace!(pid = pid.as_u32(), "signalling");
                match proc_.kill_with(Signal::User2) {
                    Some(true) => true,
                    Some(false) => {
                        error!(pid = pid.as_u32(), "failed to send signal");
                        false
                    }
                    None => {
                        error!("signal not supported");
                        false
                    }
                }
            } else {
                false
            }
        });

        if pids.is_empty() {
            pids = wait_for_process(&process, &mut sys);
        }

        let delay = fps.load(Ordering::Relaxed);
        trace!(fps = delay, "sleeping");
        std::thread::sleep(Duration::from_secs_f64(1.0 / delay as f64));
    }
}
