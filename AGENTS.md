# Repository Guidelines

## Project Structure & Module Organization
- Root: `Cargo.toml` (package and bins), `build.rs` (build metadata via shadow-rs), `docs/` (mdBook).
- CLI binaries: `wparse/`, `wpgen/`,  `wprescue/` (each has `main.rs`; features wired in `feats.rs` or `plugins.rs`).
- Config and samples: `connectors/source.d/` and `connectors/sink.d/` TOML; `examples/` contains runnable cases and configs.
- Upstream crates are path deps in `Cargo.toml` (e.g., `../wp-engine`, optional `../wp-connectors`); ensure these sibling repos exist when building features like `kafka`.

## Build, Test, and Development Commands
- Build: `cargo build` (add `--release` for optimized binaries).
- Features: default is `community`. Examples: `cargo build --features kafka`; minimal: `cargo build --no-default-features --features runtime-core`.
- Run a CLI: `cargo run --bin wpkit -- --help` (swap `wpkit` for `wparse`, `wpgen`, `wprescue`).
- Lint/format: `cargo fmt --all`; `cargo clippy --all-targets --all-features -- -D warnings`.
- Docs: `make -C docs build` or `make -C docs serve`.

## Coding Style & Naming Conventions
- Rust 2021; rustfmt defaults (4-space indent; soft limit ~100–120 cols).
- Names: types `PascalCase`; modules/files `snake_case`; constants `SCREAMING_SNAKE_CASE`.
- Errors: use `thiserror` in libs; `anyhow` at binary boundaries.
- Logging via `log`/`env_logger`; avoid `unwrap()`/`expect()` in production.
- Keep connector registration behind features in app-layer files (e.g., `wparse/feats.rs`).

## Testing Guidelines
- Unit tests live next to code (`mod tests`); integration tests under `tests/`.
- Run all: `cargo test`; gated: `cargo test --features sink_test`.
- Use `serial_test` for stateful tests; `httpmock` for HTTP-facing units.
- Benchmarks: Criterion via `cargo bench`.

## Commit & Pull Request Guidelines
- Conventional Commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`; scope optional (e.g., `feat(wpkit): ...`).
- PRs must describe intent and behavior changes, link issues, include tests for new logic, and update `docs/` for CLI or behavior changes.
- Before submit: run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, and build key feature sets (`community`, `kafka`).

## Security & Configuration Tips
- Do not commit secrets or env-specific files; prefer local TOML/env.
- Initialize and validate generator config: `cargo run --bin wpgen -- conf init` then `cargo run --bin wpgen -- conf check`.
- For minimal/debug builds, use `--no-default-features` and enable only what you need.
