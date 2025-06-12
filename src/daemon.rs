use crate::config::load_config;
use crate::frame::{image_dir, FrameCache};
use crate::ipc::ControlMessage;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use std::{env, fs};
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tracing::{debug, error, info, trace};

fn run_ai_controller(fps: Arc<Mutex<f32>>, enabled: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut cam = match videoio::VideoCapture::new(0, videoio::CAP_ANY) {
            Ok(c) => c,
            Err(e) => {
                error!("failed to open camera: {e}");
                return;
            }
        };
        if !cam.is_opened().unwrap_or(false) {
            error!("camera not opened");
            return;
        }

        let cfg = env::var("BONGO_YOLO_CONFIG").unwrap_or_else(|_| "yolov3-tiny.cfg".into());
        let weights =
            env::var("BONGO_YOLO_WEIGHTS").unwrap_or_else(|_| "yolov3-tiny.weights".into());
        let mut net = match dnn::read_net_from_darknet(&cfg, &weights) {
            Ok(n) => n,
            Err(e) => {
                error!("failed to load YOLO model: {e}");
                return;
            }
        };
        let _ = net.set_preferable_backend(dnn::DNN_BACKEND_OPENCV);
        let _ = net.set_preferable_target(dnn::DNN_TARGET_CPU);
        let mut frame = core::Mat::default();
        while enabled.load(Ordering::Relaxed) {
            if cam.read(&mut frame).is_ok() && !frame.empty() {
                if let Ok(blob) = dnn::blob_from_image(
                    &frame,
                    1.0 / 255.0,
                    core::Size::new(416, 416),
                    core::Scalar::default(),
                    true,
                    false,
                    core::CV_32F,
                ) {
                    let _ = net.set_input(&blob, "", 1.0, core::Scalar::default());
                    if let Ok(out_names) = net.get_unconnected_out_layers_names() {
                        let mut outs = core::Vector::<core::Mat>::new();
                        if net.forward(&mut outs, &out_names).is_ok() {
                            let mut closeness = 0.0f32;
                            for out in outs {
                                for j in 0..out.rows() {
                                    if let Ok(row) = out.at_row::<f32>(j) {
                                        let conf = row[4];
                                        if conf > 0.5 {
                                            let area = (row[2] * row[3]).abs().max(1.0);
                                            closeness += 500.0 / area;
                                        }
                                    }
                                }
                            }
                            let mut new_fps = 5.0 + closeness;
                            new_fps = new_fps.clamp(0.3, 30.0);
                            if let Ok(mut val) = fps.lock() {
                                *val = new_fps;
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}
use opencv::{core, dnn, prelude::*, videoio};

fn wait_for_process(name: &str, sys: &mut System) -> Vec<Pid> {
    loop {
        sys.refresh_processes(ProcessesToUpdate::All, true);
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
    let fps = Arc::new(Mutex::new(cfg.fps.clamp(0.3, 30.0)));
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
                                if let Ok(mut val) = fps_ctrl.lock() {
                                    *val = v.clamp(0.3, 30.0);
                                }
                            }
                            ControlMessage::EnableAi => {
                                debug!("enabling AI mode");
                                if !ai_ctrl.swap(true, Ordering::Relaxed) {
                                    run_ai_controller(fps_ctrl.clone(), ai_ctrl.clone());
                                }
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

    if ai_mode.load(Ordering::Relaxed) {
        run_ai_controller(fps.clone(), ai_mode.clone());
    }

    let mut sys = System::new();
    let mut pids = wait_for_process(&process, &mut sys);

    loop {
        sys.refresh_processes(ProcessesToUpdate::All, true);

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

        let delay = {
            if let Ok(val) = fps.lock() {
                *val
            } else {
                5.0
            }
        };
        trace!(fps = delay, "sleeping");
        std::thread::sleep(Duration::from_secs_f64(1.0 / delay as f64));
    }
}
