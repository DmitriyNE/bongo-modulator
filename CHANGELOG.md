# Changelog

## [Unreleased]
- Initial project scaffolding
- Daemon periodically signals Hyprlock with SIGUSR2
- FPS can be adjusted with `mode fps`
- State shared via `state.json` between CLI and daemon
- CLI updates are sent to the daemon over a Unix socket
- `next-image` retrieves frames from the daemon
- Hyprlock process detection via `--process`
- Default YOLOv8 model now downloaded from `salim4n/yolov8n-detect-onnx`
