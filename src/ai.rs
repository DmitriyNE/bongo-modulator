use candle_core::{DType, Device, Tensor};
use candle_onnx::{read_file, simple_eval};
use hf_hub::api::sync::Api;
use image::imageops::FilterType;
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType},
    Camera,
};
use std::collections::HashMap;
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
        let graph = match &model.graph {
            Some(g) => g,
            None => {
                error!("model graph missing");
                return;
            }
        };
        let input_name = graph.input[0].name.clone();
        let output_name = graph.output[0].name.clone();
        let device = Device::Cpu;
        debug!("AI thread started");

        let start = Instant::now();

        loop {
            if !enabled.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            let frame = match cam.frame() {
                Ok(f) => f,
                Err(e) => {
                    error!("failed to capture frame: {e}");
                    continue;
                }
            };
            let img = match frame.decode_image::<RgbFormat>() {
                Ok(i) => image::DynamicImage::ImageRgb8(i),
                Err(e) => {
                    error!("failed to decode frame: {e}");
                    continue;
                }
            };
            let img = img.resize_exact(640, 640, FilterType::CatmullRom);
            let data = img.into_rgb8().into_raw();
            let tensor = match Tensor::from_vec(data, (640, 640, 3), &device) {
                Ok(t) => match t
                    .permute((2, 0, 1))
                    .and_then(|t| t.to_dtype(DType::F32))
                    .and_then(|t| t.affine(1.0 / 255.0, 0.0))
                {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to prepare tensor: {e}");
                        continue;
                    }
                },
                Err(e) => {
                    error!("failed to create tensor: {e}");
                    continue;
                }
            };
            let mut inputs = HashMap::new();
            let tensor = match tensor.unsqueeze(0) {
                Ok(t) => t,
                Err(e) => {
                    error!("failed to unsqueeze tensor: {e}");
                    continue;
                }
            };
            inputs.insert(input_name.clone(), tensor);
            let mut outputs = match simple_eval(&model, inputs) {
                Ok(o) => o,
                Err(e) => {
                    error!("failed to run model: {e}");
                    continue;
                }
            };
            let output = match outputs.remove(&output_name) {
                Some(o) => o,
                None => {
                    error!("model output missing");
                    continue;
                }
            };
            let dims = output.dims();
            let count = dims.get(1).copied().unwrap_or(0);
            let ratio = (start.elapsed().as_millis() % 1000) as f32 / 1000.0;
            let computed = compute_fps(count, ratio);
            debug!(fps = computed, count, ratio = ratio, "AI updated FPS");
            fps.store(computed.to_bits(), Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

fn compute_fps(count: usize, ratio: f32) -> f32 {
    let base = 5.0;
    let weight = 20.0;
    let fps = base + weight * ratio * count as f32;
    fps.clamp(0.5, 30.0)
}
