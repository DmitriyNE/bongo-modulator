# Contribution Guidelines

- Use `cargo fmt` before committing.
- Commit messages should be in imperative mood and under 72 characters.
- Run `cargo clippy -- -D warnings` and `cargo nextest run` before commit.
- Prefer writing property tests with `proptest` when applicable.
- Ensure the Nix flake stays in sync with project dependencies.
