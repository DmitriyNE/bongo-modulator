use candle_core::Device;
use candle_onnx::read_file;
use hf_hub::api::sync::Api;
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType},
    Camera,
};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tracing::{debug, error};

pub fn spawn_ai_thread(fps: Arc<AtomicU32>, enabled: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::None);
        #[cfg(target_os = "macos")]
        let fallback = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
            CameraFormat::new_from(1280, 720, FrameFormat::NV12, 30),
        ));
        #[cfg(not(target_os = "macos"))]
        let fallback = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
            CameraFormat::new_from(1280, 720, FrameFormat::MJPEG, 30),
        ));
        let mut cam = match Camera::new(CameraIndex::Index(0), format)
            .or_else(|_| Camera::new(CameraIndex::Index(0), fallback))
        {
            Ok(c) => c,
            Err(e) => {
                error!("failed to open camera: {e}");
                return;
            }
        };
        if let Err(e) = cam.open_stream() {
            error!("failed to open camera stream: {e}");
            return;
        }

        let filename = std::env::var("BONGO_YOLO_MODEL")
            .unwrap_or_else(|_| "yolov8n-onnx-web/yolov8n.onnx".to_string());
        let repo = std::env::var("BONGO_YOLO_REPO")
            .unwrap_or_else(|_| "salim4n/yolov8n-detect-onnx".to_string());
        let model_path = if Path::new(&filename).exists() {
            filename.clone()
        } else {
            match Api::new().and_then(|api| api.model(repo).get(&filename)) {
                Ok(p) => p.to_string_lossy().into(),
                Err(e) => {
                    error!("failed to download model: {e}");
                    return;
                }
            }
        };
        let model = match read_file(&model_path) {
            Ok(m) => m,
            Err(e) => {
                error!("failed to load model: {e}");
                return;
            }
        };
        let device = Device::Cpu;
        debug!("AI thread started");

        let start = Instant::now();
        let mut count = 0usize;

        loop {
            if !enabled.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            if let Err(e) = cam.frame() {
                error!("failed to capture frame: {e}");
                continue;
            }
            count += 1;
            let _device = &device;
            let _ = &model; // placeholder for actual inference
            let ratio = (start.elapsed().as_millis() % 1000) as f32 / 1000.0;
            let computed = compute_fps(count, ratio);
            debug!(fps = computed, count, ratio = ratio, "AI updated FPS");
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
