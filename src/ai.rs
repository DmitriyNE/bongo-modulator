use candle_core::Device;
use candle_onnx::read_file;
use opencv::{
    prelude::*,
    videoio::{VideoCapture, CAP_ANY},
};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::time::Duration;
use tracing::{debug, error};

pub fn spawn_ai_thread(fps: Arc<AtomicU32>, enabled: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut cam = match VideoCapture::new(0, CAP_ANY) {
            Ok(c) => c,
            Err(e) => {
                error!("failed to open camera: {e}");
                return;
            }
        };
        if !cam.is_opened().unwrap_or(false) {
            error!("failed to open camera");
            return;
        }

        let model_path =
            std::env::var("BONGO_YOLO_MODEL").unwrap_or_else(|_| "yolov8.onnx".to_string());
        let model = match read_file(&model_path) {
            Ok(m) => m,
            Err(e) => {
                error!("failed to load model: {e}");
                return;
            }
        };
        let device = Device::Cpu;
        debug!("AI thread started");

        loop {
            if !enabled.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            let mut frame = opencv::core::Mat::default();
            if cam.read(&mut frame).is_err() || frame.empty() {
                error!("failed to capture frame");
                continue;
            }
            let _device = &device;
            let _ = &model; // placeholder for actual inference
            let computed = compute_fps(1, 0.5);
            fps.store(computed, Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

fn compute_fps(count: usize, ratio: f32) -> u32 {
    let base = 5.0;
    let weight = 20.0;
    let fps = base + weight * ratio * count as f32;
    fps.clamp(1.0, 60.0) as u32
}
