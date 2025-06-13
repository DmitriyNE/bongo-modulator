use candle_core::Device;
use candle_onnx::read_file;
use rscam::{Camera, Config};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::time::Duration;
use tracing::{debug, error};

pub fn spawn_ai_thread(fps: Arc<AtomicU32>, enabled: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut cam = match Camera::new("/dev/video0") {
            Ok(c) => c,
            Err(e) => {
                error!("failed to open camera: {e}");
                return;
            }
        };

        let _ = cam.start(&Config {
            interval: (1, 30),
            resolution: (640, 480),
            format: b"RGB3",
            ..Default::default()
        });

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
            if cam.capture().is_err() {
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
