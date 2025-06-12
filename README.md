# Bongo modulator

This project modulates bongo cat intensity on the Hyprlock lockscreen.

The daemon sends `SIGUSR2` to Hyprlock so it refreshes its image element.
Hyprlock retrieves frames by running `bongo-modulator next-image` which outputs
the path to a frame from the images directory.

## Usage

```bash
bongo-modulator daemon       # start the signalling service
bongo-modulator next-image   # print path to next frame
bongo-modulator mode ai      # enable AI mode (stub)
bongo-modulator mode fps 10  # set manual FPS
```

See `AGENTS.md` for contribution guidelines and `CHANGELOG.md` for release
notes.
