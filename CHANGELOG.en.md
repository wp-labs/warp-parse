# Changelog

English | [中文](./CHANGELOG.md)

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.14.0] - 2025-01-16

### Added
- New `wproj rescue stat` command for statistics on rescue directory data:
  - Supports per-sink grouped statistics for file count, line count, and file size
  - Supports `--detail` flag to show file details
  - Supports `--json` and `--csv` output formats

### Changed
- Upgraded `wp-engine` core engine to v1.9.0-alpha.1 with dynamic rate control (SpeedProfile) support.

### Fixed
- Fixed `speed_profile` dynamic rate configuration not taking effect in wpgen config. Now correctly reads and applies sinusoidal, stepped, burst and other dynamic rate modes from configuration files.
- Fixed compilation error caused by missing `speed_profile` field in `GenGRA` after wp-engine upgrade.

[0.14.0]: https://github.com/wp-labs/warp-parse/releases/tag/v0.14.0

## [0.13.1] - 2026-01-14

### Changed
- Upgraded `wp-engine` core engine to v1.8.2-beta for latest runtime features and performance optimizations.
- Upgraded `wp-connectors` to v0.7.5-alpha to improve data source adapter stability.
- Enhanced CI workflows with integration testing steps based on wp-examples repository to ensure release quality.
- Cleaned up unused template files (`_gal/tpl/Cargo.toml`) and workflow configurations to simplify project structure.
- Updated README with revised performance testing documentation and examples.

[0.13.1]: https://github.com/wp-labs/warp-parse/releases/tag/v0.13.1

## [0.13.0] - 2024-05-09

> :information_source: This release follows the [wp-engine v1.8.0 changelog](https://github.com/wp-labs/wp-engine/releases/tag/v1.8.0). Changes on the CLI side primarily adapt to the core engine API updates. We recommend reading the engine release notes to understand runtime behavior differences.

### Added
- New **Field Pipe** design document (`docs/field-pipe-design.md`) explaining the execution model after splitting field collection pipes and single-field pipes, helping users understand how selectors like `take/last/@key` work with functions like `base64_decode`.
- `wproj` data, statistics, and validation subcommands now automatically load the security dictionary (`EnvDict`), providing access to secrets, variables, and other runtime configurations without manual setup.

### Changed
- Unified handling of `-q/--quiet` flags across `wproj`, `wparse`, and `wprescue` CLI tools using `wp_cli_core::split_quiet_args`, with consistent runtime feature registration for quiet mode and plugin loading.
- Migrated to `wp_cli_core` implementation for sink/source statistics and validation: `stat`/`validate` output now uses core library formatting, route/OML display aligns with the engine; `wpgen rule` direct execution also passes runtime variables to the engine layer.
- Updated dependencies in template `_gal/tpl/Cargo.toml` and main `Cargo.toml`, removing deprecated `wp-cli-utils` and directly referencing `wp-cli-core` for the latest CLI capabilities.

### Fixed
- Adapted to `wp-engine` v1.8.0 API changes where functions like `WarpProject::init/load`, `load_warp_engine_confs`, and `collect_oml_models` now require explicit `EnvDict` parameters. Resolved multiple compilation errors and improved runtime configuration consistency.
- Fixed statistics/validation commands crashing due to type mismatches with `wp-cli-core` in non-JSON mode. Now consistently converts to core library format for proper output.

[0.13.0]: https://github.com/wp-labs/warp-parse/releases/tag/v0.13.0
