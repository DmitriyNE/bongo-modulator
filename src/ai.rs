use candle_core::Device;
use candle_onnx::read_file;
use hf_hub::api::sync::Api;
use image::DynamicImage;
use candle_core::{DType, Tensor};
use candle_transformers::object_detection::{non_maximum_suppression, Bbox};

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
use std::time::Duration;
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
        let graph = match &model.graph {
            Some(g) => g,
            None => {
                error!("invalid model graph");
                return;
            }
        };
        let input_name = graph.input[0].name.clone();
        let output_name = graph.output[0].name.clone();
        debug!("AI thread started");

        loop {
            if !enabled.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            let buffer = match cam.frame() {
                Ok(b) => b,
                Err(e) => {
                    error!("failed to capture frame: {e}");
                    continue;
                }
            };
            let rgb = match buffer.decode_image::<RgbFormat>() {
                Ok(img) => img,
                Err(e) => {
                    error!("failed to decode frame: {e}");
                    continue;
                }
            };

            let (orig_w, orig_h) = (rgb.width(), rgb.height());
            let (w, h) = if orig_w < orig_h {
                let w = orig_w as usize * 640 / orig_h as usize;
                (w / 32 * 32, 640)
            } else {
                let h = orig_h as usize * 640 / orig_w as usize;
                (640, h / 32 * 32)
            };
            let img = DynamicImage::ImageRgb8(rgb).resize_exact(
                w as u32,
                h as u32,
                image::imageops::FilterType::CatmullRom,
            );
            let data = img.to_rgb8().into_raw();
            let input = Tensor::from_vec(data, (h, w, 3), &device)
                .unwrap()
                .permute((2, 0, 1))
                .unwrap()
                .unsqueeze(0)
                .unwrap()
                .to_dtype(DType::F32)
                .unwrap()
                * (1f32 / 255f32);
            let mut inputs = std::collections::HashMap::new();
            inputs.insert(input_name.clone(), input);
            let outputs = match candle_onnx::simple_eval(&model, inputs) {
                Ok(o) => o,
                Err(e) => {
                    error!("failed to run model: {e}");
                    continue;
                }
            };
            let preds = outputs[&output_name].to_device(&Device::Cpu).unwrap();
            let preds = match preds.squeeze(0) {
                Ok(p) => p,
                Err(e) => {
                    error!("invalid predictions: {e}");
                    continue;
                }
            };
            let (pred_size, npreds) = match preds.dims2() {
                Ok(d) => d,
                Err(e) => {
                    error!("unexpected dims: {e}");
                    continue;
                }
            };
            let nclasses = pred_size - 4;
            let mut bboxes: Vec<Vec<Bbox<()>>> = (0..nclasses).map(|_| vec![]).collect();
            for index in 0..npreds {
                let pred = Vec::<f32>::try_from(preds.i((.., index)).unwrap()).unwrap();
                let confidence = pred[4..].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                if confidence > 0.25 {
                    let mut class_index = 0;
                    for i in 0..nclasses {
                        if pred[4 + i] > pred[4 + class_index] {
                            class_index = i;
                        }
                    }
                    if pred[class_index + 4] > 0.0 {
                        bboxes[class_index].push(Bbox {
                            xmin: pred[0] - pred[2] / 2.0,
                            ymin: pred[1] - pred[3] / 2.0,
                            xmax: pred[0] + pred[2] / 2.0,
                            ymax: pred[1] + pred[3] / 2.0,
                            confidence,
                            data: (),
                        });
                    }
                }
            }
            non_maximum_suppression(&mut bboxes, 0.45);
            let people = bboxes.get(0).cloned().unwrap_or_default();
            let closeness = people
                .iter()
                .map(|b| (b.xmax - b.xmin) * (b.ymax - b.ymin))
                .fold(0f32, f32::max)
                / ((w * h) as f32);

            let computed = compute_fps(people.len(), closeness);
            debug!(fps = computed, people = people.len(), closeness, "AI updated FPS");
            fps.store(computed, Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

fn compute_fps(num_people: usize, closeness: f32) -> u32 {
    let base = 1.0;
    let mut fps = base + num_people as f32 * 5.0 + closeness * 20.0;
    fps = fps.clamp(0.5, 30.0);
    fps.round() as u32
}
