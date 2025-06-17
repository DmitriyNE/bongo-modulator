# Contribution Guidelines

- Use `cargo fmt` before committing.
- Commit messages should be in imperative mood and under 72 characters.
- Run `cargo clippy -- -D warnings` and `cargo nextest run` before commit.
- Prefer writing property tests with `proptest` when applicable.
- Use only the tools provided by `nix develop`. Either enter the dev shell
  interactively with `nix develop` or prefix commands with `nix develop -c` so
  tools like `cargo fmt`, `cargo clippy`, and `cargo nextest` are available.
- Ensure the Nix flake stays in sync with project dependencies:
  - After updating `Cargo.toml` or `Cargo.lock`, regenerate `Cargo.nix` in the dev shell:
    ```bash
    nix develop -c cargo2nix --overwrite
    ```
  - For faster local iteration, you can disable the lock-hash check by setting:
    ```bash
    export BONGO_IGNORE_LOCK_HASH=1
    ```
    Note: CI enforces lock-file consistency, so do not commit with this enabled.
- After building, push the resulting store paths to Cachix with
  `cachix push bongo-modulator`.
- Target POSIX-compliant systems only (Linux and macOS). Windows is unsupported.
- Camera capture should use the `nokhwa` crate for Linux and macOS.
- Building `nokhwa` requires `libclang`. Ensure it is available and that the
  `LIBCLANG_PATH` environment variable points to its library directory.
- On Linux, the build also needs Video4Linux headers. Provide them via your
  package manager or the Nix flake.
