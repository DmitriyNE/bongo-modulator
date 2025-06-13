# Contribution Guidelines

- Use `cargo fmt` before committing.
- Commit messages should be in imperative mood and under 72 characters.
- Run `cargo clippy -- -D warnings` and `cargo nextest run` before commit.
- Prefer writing property tests with `proptest` when applicable.
- Ensure the Nix flake stays in sync with project dependencies.
- Target POSIX-compliant systems only (Linux and macOS). Windows is unsupported.
- Camera capture should use the `nokhwa` crate for Linux and macOS.
- Building `nokhwa` requires `libclang`. Ensure it is available and that the
  `LIBCLANG_PATH` environment variable points to its library directory.
- On Linux, the build also needs Video4Linux headers. Provide them via your
  package manager or the Nix flake.
