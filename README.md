# Bongo modulator

This project modulates bongo cat intensity on the Hyprlock lockscreen.

The daemon periodically sends `SIGUSR2` to Hyprlock so it refreshes its image
element. Hyprlock retrieves frames by running `bongo-modulator next-image`
which requests the next frame from the daemon over the Unix socket. The daemon
maintains its own list of frames, read from a directory named `images/` by
default. You can point `next-image` at a different folder with `--dir` and the
daemon will use that directory instead. If the chosen directory is empty or
missing the daemon returns no path and `next-image` reports an error.
Configuration is persisted in `state.json` and updates are sent to the daemon
so changes take effect immediately.

## Usage

```bash
bongo-modulator daemon       # start the signalling service
bongo-modulator next-image   # print path to next frame
bongo-modulator mode ai      # enable AI mode (stub)
bongo-modulator mode fps 10  # set manual FPS
```

See `AGENTS.md` for contribution guidelines and `CHANGELOG.md` for release
notes.
