use candle_core::{DType, Device, Tensor};
use candle_onnx::{onnx, read_file, simple_eval};
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
        let mut cam = None;
        for (w, h) in [(1280, 720), (640, 480)] {
            for fmt in [FrameFormat::RAWRGB, FrameFormat::MJPEG, FrameFormat::YUYV] {
                let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
                    CameraFormat::new_from(w, h, fmt, 30),
                ));
                match Camera::new(CameraIndex::Index(0), req) {
                    Ok(c) => {
                        cam = Some(c);
                        break;
                    }
                    Err(_) => continue,
                }
            }
            if cam.is_some() {
                break;
            }
        }
        let mut cam = match cam.or_else(|| Camera::new(CameraIndex::Index(0), format).ok()) {
            Some(c) => c,
            None => {
                error!("failed to open camera");
                return;
            }
        };
        if let Err(e) = cam.open_stream() {
            error!("failed to open camera stream: {e}");
            return;
        }
        debug!(format = ?cam.camera_format(), "camera stream opened");

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
        let mut model = match read_file(&model_path) {
            Ok(m) => m,
            Err(e) => {
                error!("failed to load model: {e}");
                return;
            }
        };
        patch_maxpool_padding(&mut model);
        patch_resize_identity(&mut model);
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

fn patch_maxpool_padding(model: &mut onnx::ModelProto) {
    let Some(graph) = model.graph.as_mut() else {
        return;
    };
    let mut new_nodes = Vec::with_capacity(graph.node.len());
    for mut node in std::mem::take(&mut graph.node) {
        if node.op_type == "MaxPool" {
            let mut pad_attr = None;
            for attr in node.attribute.iter_mut() {
                if attr.name == "pads" {
                    if attr.ints.iter().any(|&v| v != 0) {
                        pad_attr = Some(attr.ints.clone());
                        for v in &mut attr.ints {
                            *v = 0;
                        }
                    }
                    break;
                }
            }
            if let Some(pads) = pad_attr {
                let pad_init_name = format!("{}_pads", node.name);
                let full_pads = vec![0, 0, pads[0], pads[1], 0, 0, pads[2], pads[3]];
                let tensor = onnx::TensorProto {
                    name: pad_init_name.clone(),
                    dims: vec![full_pads.len() as i64],
                    data_type: onnx::tensor_proto::DataType::Int64 as i32,
                    int64_data: full_pads.clone(),
                    ..Default::default()
                };
                graph.initializer.push(tensor);

                let pad_output = format!("{}_pad_out", node.name);
                let mut pad_node = onnx::NodeProto {
                    input: vec![node.input[0].clone(), pad_init_name],
                    output: vec![pad_output.clone()],
                    name: format!("{}_pad", node.name),
                    op_type: "Pad".to_string(),
                    ..Default::default()
                };
                pad_node.attribute.push(onnx::AttributeProto {
                    name: "mode".to_string(),
                    r#type: onnx::attribute_proto::AttributeType::String as i32,
                    s: b"reflect".to_vec(),
                    ..Default::default()
                });
                new_nodes.push(pad_node);

                node.input[0] = pad_output;
            }
        }
        new_nodes.push(node);
    }
    graph.node = new_nodes;
}

fn patch_resize_identity(model: &mut onnx::ModelProto) {
    let Some(graph) = model.graph.as_mut() else {
        return;
    };
    for node in graph.node.iter_mut() {
        if node.op_type == "Resize" {
            if let Some(first) = node.input.first().cloned() {
                node.op_type = "Identity".to_string();
                node.input.clear();
                node.input.push(first);
            }
        }
    }
}
