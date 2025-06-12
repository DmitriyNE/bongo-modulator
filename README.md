# Bongo modulator

This project modulates bongo cat intensity on the Hyprlock lockscreen.

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
bongo-modulator mode ai      # enable AI mode (uses YOLOv3)
bongo-modulator mode fps 10  # set manual FPS
```

AI detection expects OpenCV to be installed on the system. Model files can be
specified with the `BONGO_YOLO_CONFIG` and `BONGO_YOLO_WEIGHTS` environment
variables. When not set the daemon falls back to `yolov3-tiny.cfg` and
`yolov3-tiny.weights` in the working directory.

See `AGENTS.md` for contribution guidelines and `CHANGELOG.md` for release
notes.
