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

To use AI-based FPS modulation, set the `BONGO_YOLO_MODEL` environment variable
to the desired model filename. If the file does not exist locally the daemon
automatically downloads it from the Hugging Face hub (defaults to
`yolov8n-onnx-web/yolov8n.onnx` from `salim4n/yolov8n-detect-onnx`). The repository can
be overridden with `BONGO_YOLO_REPO`. The
daemon captures frames with the `nokhwa` crate (camera index `0`) and uses the
model via the pure-Rust `candle` runtime to estimate how many people are in
front of the camera. The FPS value is updated based on the detection results.

Building `nokhwa` requires libclang. When not using the provided Nix flake,
set the `LIBCLANG_PATH` environment variable to the directory containing
`libclang.so`.
On Linux, you'll also need the Video4Linux headers (e.g. via `libv4l-dev`).

## Nix build

The project uses [cargo2nix](https://github.com/cargo2nix/cargo2nix) for
reproducible builds. Regenerate `Cargo.nix` whenever `Cargo.toml` or
`Cargo.lock` changes:

The Nix flake provides a Rust nightly toolchain (version 1.79 or newer) to
support crates that require bleeding-edge features.

```bash
cachix watch-exec bongo-modulator -- \
  cargo2nix --overwrite
```

Before building ensure your Cachix credentials are configured:

```bash
cachix authtoken <token>
```

Build with Nix and upload the artifacts:

```bash
nix build
cachix push bongo-modulator result
```

This keeps the binary cache current for all contributors.
