# Bongo modulator

This project modulates bongo cat intensity on the Hyprlock lockscreen.
It targets POSIX-compliant systems (Linux and macOS) only.

The daemon periodically sends `SIGUSR2` to Hyprlock so it refreshes its image
element. Hyprlock retrieves frames by running `bongo-modulator next-image`
which requests the next frame from the daemon over the Unix socket. The daemon
maintains its own list of frames, read from a directory named `images/` by
default. You can point the daemon at a different folder with `--dir` or by
setting `BONGO_IMAGE_DIR` in the environment. If the chosen directory is empty or
missing the daemon returns no path and `next-image` reports an error.
The daemon assumes the Hyprlock process name is `hyprlock`; override it with
`--process` when needed.
Configuration is persisted in `state.json` and updates are sent to the daemon
so changes take effect immediately.

## Usage

```bash
bongo-modulator daemon       # start the signalling service
bongo-modulator daemon --process hyprlock  # custom process name
bongo-modulator next-image   # print path to next frame
bongo-modulator mode ai      # enable AI mode (YOLOv8)
bongo-modulator mode fps 10  # set manual FPS
```

A `bongo-modulator.service` unit is included for running the daemon under
systemd. Enable it with `systemctl enable --now bongo-modulator.service`.

See `AGENTS.md` for contribution guidelines and `CHANGELOG.md` for release
notes.

## AI mode

To use AI-based FPS modulation, download a YOLOv8 ONNX model and set the
`BONGO_YOLO_MODEL` environment variable to its path. The daemon captures frames
with the `nokhwa` crate (camera index `0`) and uses the model via the pure-Rust
`candle` runtime to estimate how many people are in front of the camera. The FPS
value is updated based on the detection results.

Building `nokhwa` requires libclang. When not using the provided Nix flake,
set the `LIBCLANG_PATH` environment variable to the directory containing
`libclang.so`.
On Linux, you'll also need the Video4Linux headers (e.g. via `libv4l-dev`).
