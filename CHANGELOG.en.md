# Changelog

English | [中文](./CHANGELOG.md)

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.16.1] - 2026-02-05

### Changed
- Upgraded `wp-motor` core engine to v1.14.1-alpha with the following key changes:
  - **WPL Pipe Processor**: Added `strip/bom` processor for removing BOM (Byte Order Mark) from data
    - Supports UTF-8, UTF-16 LE/BE, and UTF-32 LE/BE BOM detection and removal
    - Fast O(1) detection by checking only first 2-4 bytes
    - Preserves input container type (String → String, Bytes → Bytes, ArcBytes → ArcBytes)

## [0.16.0] - 2026-02-04

### Changed
- Upgraded `wp-motor` core engine to v1.14.0 with the following key changes:
  - **WPL Functions**: Added `starts_with` pipe function for efficient string prefix matching
  - **OML Pipe Functions**: Added `starts_with` function for prefix matching in OML query language
  - **OML Pipe Functions**: Added `map_to` function for type-aware conditional value assignment (supports string, integer, float, boolean)
  - **OML Match Expression**: Added function-based pattern matching support (`match read(field) { starts_with('prefix') => result }`)
    - String matching functions: `starts_with`, `ends_with`, `contains`, `regex_match`, `is_empty`, `iequals`
    - Numeric comparison functions: `gt`, `lt`, `eq`, `in_range`
  - **OML Parser**: Added quoted string support for `chars()` and other value constructors (single and double quotes)
  - **OML Transformer**: Added automatic temporary field filtering (fields starting with `__` are converted to ignore type)
  - **OML Syntax**: Made `pipe` keyword optional in pipe expressions (both `take(field) | func` and `pipe take(field) | func` supported)
  - **Bug Fixes**: Fixed `in_range` function parsing failure in OML match expressions
  - **Bug Fixes**: Fixed large integer precision loss in `map_to` parser
  - **Bug Fixes**: Fixed OML display output round-trip parsing compatibility

## [0.15.8] - 2026-02-03

### Changed
- Upgraded `wp-motor` core engine to v1.13.3 with the following key changes:
  - **WPL Parser**: Added support for `\t` (tab) and `\S` (non-whitespace) separators in parsing expressions
  - **WPL Parser**: Added support for quoted field names with special characters (e.g., `"field.name"`, `"field-name"`)
  - **WPL Functions**: Added `regex_match` function for regex pattern matching
  - **WPL Functions**: Added `digit_range` function for numeric range validation
  - **WPL Functions**: Added `chars_replace` function for character-level string replacement
  - **Logging Optimization**: High-frequency log paths now use `log_enabled!` guard to eliminate loop overhead when log level is filtered
  - **Bug Fixes**: Fixed compilation errors in WPL pattern parser implementations
  - **Bug Fixes**: Fixed data rescue functionality data loss issue
  - **Bug Fixes**: Removed base64 encoding from Miss Sink raw data display to show actual content
- Updated all dependencies to latest versions.
- **License Change**: Project license changed from Elastic License 2.0 to Apache 2.0.
- **Documentation**: Added CONTRIBUTING.md and updated README.md.

## [0.15.7] - 2026-01-30

### Changed
- Upgraded `wp-motor` core engine to v1.13.1.
- Upgraded `wp-connectors` to v0.7.5-beta.

## [0.15.6] - 2026-01-29

### Changed
- Upgraded `wp-motor` core engine to v1.13.0-alpha with the following key changes:
  - **WPL Parser Enhancement**: Added support for `\t` (tab) and `\S` (non-whitespace) separators in parsing expressions
  - **WPL Parser Enhancement**: Added support for quoted field names with special characters (e.g., `"field.name"`, `"field-name"`)
  - **New Function**: Added `chars_replace` function for character-level string replacement
  - **Logging Optimization**: High-frequency log paths now use `log_enabled!` guard to eliminate loop overhead when log level is filtered
  - **Removed Feature**: Removed `SO_REUSEPORT` multi-instance support from Syslog UDP Source (security risk and cross-platform inconsistency)

## [0.15.5] - 2026-01-28

### Changed
- Upgraded `wp-motor` core engine to v1.11.0-alpha.
- Updated project dependencies to latest versions.

## [0.15.4] - 2026-01-27

### Changed
- Updated all dependencies to latest versions for improved stability and performance.

## [0.15.3] - 2026-01-23

### Fixed
- Fixed wp-motor related issues to improve runtime stability.

## [0.15.2] - 2026-01-22

### Changed
- Migrated from `wp-engine` to `wp-motor` v1.10.2-beta:
  - wp-engine project has been renamed to wp-motor, all dependencies updated to point to new repository
  - Upgraded to v1.10.2-beta with latest runtime features and performance optimizations

## [0.15.1] - 2026-01-18

### Added
- Integrated shadow-rs for build-time information support (#100):
  - Added shadow-rs as build dependency to generate metadata at compile time
  - Version command now displays Git commit, build time, and Rust compiler version
  - Enhanced traceability for deployed binaries to facilitate troubleshooting

### Changed
- Updated project dependencies to latest versions.

## [0.15.0] - 2025-01-17

### Changed
- Upgraded `wp-engine` core engine to v1.10.0-alpha with the following key changes:
  - **New KvArr Parser**: Added key=value array format parser supporting flexible separators (comma, space, or mixed), automatic type inference, and automatic array indexing for duplicate keys
  - **Fixed Meta Fields Issue**: Fixed meta fields being ignored in sub-parser context
  - **API Improvements**: Fixed `validate_groups` function export in wp-cli-core, now exported from `wp_cli_core::utils::validate` module
- Upgraded `wp-model-core` to 0.7.1.

## [0.14.0] - 2025-01-16

### Added
- New `wproj rescue stat` command for statistics on rescue directory data:
  - Supports per-sink grouped statistics for file count, line count, and file size
  - Supports `--detail` flag to show file details
  - Supports `--json` and `--csv` output formats
- Added Doris connector support, enabling direct data writes to Apache Doris database.
- GitHub Release workflow now includes automatic CHANGELOG extraction:
  - Automatically extracts version-specific entries from CHANGELOG.md and CHANGELOG.en.md
  - English changelog shown by default, with Chinese content in collapsible section
  - Implemented via scripts/extract-changelog.sh script

### Changed
- Upgraded `wp-engine` core engine to v1.9.0-alpha.2 with the following key changes:
  - **Dynamic Speed Control Module**: Added `SpeedProfile` supporting multiple rate modes (constant, sinusoidal, stepped, burst, ramp, random walk, composite) for realistic traffic simulation
  - **Rescue Statistics Module**: New rescue data statistics functionality with per-sink grouping and multiple output formats (table, JSON, CSV)
  - **wpgen.toml Configuration Enhancement**: Support for defining `speed_profile` dynamic rate configuration in config files
  - **BlackHoleSink Enhancement**: Added `sink_sleep_ms` parameter to control delay per sink operation

### Fixed
- Fixed `speed_profile` dynamic rate configuration not taking effect in wpgen config. Now correctly reads and applies sinusoidal, stepped, burst and other dynamic rate modes from configuration files.
- Fixed compilation error caused by missing `speed_profile` field in `GenGRA` after wp-engine upgrade.
- Fixed YAML syntax error in dependabot-branch-filter workflow.
- Fixed issues related to adm.gxl configuration file.

### Documentation
- Removed outdated technical design and user guide documentation, cleaning up documentation structure.

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
