# wproj check — Configuration Validation Reference

`wproj check` performs 8 validation categories in dependency order. The first 4 are required; the last 4 allow the target file to be absent (absence = pass).

## Validation Overview

| # | Component | Config Files | Required | Tolerates Absence |
|---|-----------|-------------|----------|-------------------|
| 1 | Engine | `conf/wparse.toml` | Yes | No |
| 2 | Sources | `conf/wpsrc.toml` | Yes | No |
| 3 | Connectors | `connectors/source.d/*.toml`, `connectors/sink.d/*.toml` | Yes | No |
| 4 | Sinks | `usecase/*/sink/**/*.toml`, `models/sinks/**/*.toml` | Yes | No |
| 5 | WPL | `{rule_root}/*.wpl` | No | Yes (Miss = pass) |
| 6 | OML | `{oml_root}/**/model.oml` | No | Yes (Miss = pass) |
| 7 | SemanticDict | `{knowledge_root}/semantic_dict.toml` | No | Yes (uses built-in) |
| 8 | Wpgen | `conf/wpgen.toml` | No | Yes (optional) |

---

## 1. Engine — `conf/wparse.toml`

| Check | Description | Level |
|-------|-------------|-------|
| File exists | Reads `{work_root}/conf/wparse.toml` | Error |
| TOML parse | Deserializes to `EngineConfig`; `#[serde(deny_unknown_fields)]` rejects unknown fields | Error |
| Env expansion | All `${VAR}` fields expanded via EnvDict | Error |
| Path absolutization | Relative paths for models.wpl, models.oml, topology.sources/sinks, rescue.path, admin_api TLS resolved to absolute | Error |
| CWD switch | Process working directory changed to work_root | Error |

## 2. Sources — `conf/wpsrc.toml`

| Check | Description | Level |
|-------|-------------|-------|
| File exists | Checks `{work_root}/conf/wpsrc.toml` | Error |
| TOML parse + env eval | `WarpSources::env_load_toml()` loads and expands environment variables | Error |
| Structure validation | Serializes back to TOML and calls `parse_and_validate_only()` | Error |
| Source instance build | `load_source_instances_from_file()` validates all source definitions | Error |
| **File path existence** | `base` directory exists + `file` path/glob has matching files | Warning |
| **Syslog/TCP port range** ★P0 | `port` ∈ [1, 65535] | Error |
| **Syslog/TCP protocol** ★P0 | `protocol` ∈ {tcp, udp} | Error |

## 3. Connectors — `connectors/source.d/`, `connectors/sink.d/`

| Check | Description | Level |
|-------|-------------|-------|
| Directory resolution | Reads dirs from Engine config, falls back to `{work_root}/connectors/source.d`, `sink.d` | Error |
| TOML parse | Loads each `*.toml` as `ConnectorDef` with env eval | Error |
| ID characters | IDs must match `[a-z0-9_]` | Error |
| Source ID suffix | Source connector IDs must end with `_src` | Error |
| Sink ID suffix | Non-file sink connector IDs must end with `_sink` | Error |
| File prefix suggestion | File source/sink IDs should start with `file_` | Warning |
| Kind consistency | Filename kind hint matches declared type | Warning |
| **Warning output** ★P2 | Warning-level lint issues now printed to console (previously silently discarded) | Warning |
| **Dead connector detection** ★P2 | Detects source connectors with `refs == 0` and sink connectors unused by any route | Warning |

## 4. Sinks — `usecase/*/sink/`, `models/sinks/`

| Check | Description | Level |
|-------|-------------|-------|
| Route discovery | Scans `usecase/*/{case}/sink/{business.d,infra.d}` and `models/sinks/{business.d,infra.d}` | Error |
| TOML parse + env eval | Each route file `env_load_toml` + `env_eval` | Error |
| Connector resolution | Loads sink connectors for each route directory | Error |
| Default merge | Loads and merges default sink config | Error |
| Route build | `build_route_conf_from()` parses route configuration | Error |
| FlexGroup OML/RULE mutual exclusion | Same FlexGroup cannot use both OML and RULE | Error |
| Rule pattern validation | Empty patterns rejected; patterns starting with `/` require additional validation | Error |
| **File sink output path** ★P1 | `base` directory and output parent dir existence for file/test_rescue sinks | Error |

## 5. WPL — `{rule_root}/*.wpl`

| Check | Description | Level |
|-------|-------------|-------|
| File discovery | `find_conf_files()` for `rule.wpl`, fallback `glob("*.wpl")` | — |
| File absence | No WPL files → `CheckStatus::Miss`, treated as pass | — |
| **File read errors** ★P2 | File read failures (e.g., UTF-8 encoding) now report specific errors instead of silently degrading to empty content | Error |
| Non-empty check | Each file must be non-empty | Error |
| Syntax parse | `WplCode::build()` parses syntax | Error |
| Package structure | `code.parse_pkg()` validates package structure | Error |
| **Duplicate package names** ★P2 | Multiple WPL files defining the same package name produce a Warning | Warning |
| **Duplicate rule names** ★P2 | Same rule name (`package::rule`) defined in multiple files produces a Warning | Warning |
| **Empty package** ★P2 | Package with no rules produces a Warning | Warning |
| **Empty rule body** ★P3 | Rule with no fields produces a Warning | Warning |

## 6. OML — `{oml_root}/**/model.oml`

| Check | Description | Level |
|-------|-------------|-------|
| Directory exists | Checks if `{oml_root}/` exists | — |
| File discovery | `find_conf_files()` for `model.oml` | — |
| Dir/file absence | No OML dir or no `.oml` files → "OML files missing", treated as pass | — |
| Non-empty check | `ErrorHandler::check_file_not_empty()` | Error |
| Structure parse | `fetch_oml_data()` parses all OML files | Error |
| **Model name uniqueness** ★P2 | Detects duplicate `name : <model-name>` across OML files | Warning |
| **Empty rule pattern** ★P3 | Detects empty `rule :` declarations | Warning |

## 7. SemanticDict — `{knowledge_root}/semantic_dict.toml`

| Check | Description | Level |
|-------|-------------|-------|
| Engine config load | Loads Engine config first to get `knowledge_root()` path | Error |
| Primary path probe | Checks `{knowledge_root}/semantic_dict.toml` | — |
| Fallback path probe | If primary missing, checks `{work_root}/knowledge/semantic_dict.toml` | — |
| File absence | Both paths missing → "using built-in dictionary", treated as pass | — |
| Enabled state | `conf.enabled == false` → treated as unconfigured, pass | — |
| TOML parse | `load_semantic_dict()` deserialization | Error |
| Mode report | Reports `ADD` (extends built-in) or `REPLACE` (replaces built-in) | Info |
| Word count | Counts total entries across stop_words, domain_words, status_words, action_verbs, entity_nouns | Info |
| **Empty string detection** ★P1 | Detects empty string entries in word lists, reports count | Warning |
| **Duplicate word detection** ★P1 | Detects duplicate words within the same category, reports count | Warning |
| **Empty category detection** ★P1 | Detects `chinese = []` / `english = []` empty list categories | Warning |

## 8. Wpgen — `conf/wpgen.toml`

| Check | Description | Level |
|-------|-------------|-------|
| File exists | Checks `{work_root}/conf/wpgen.toml` | — |
| File absence | Not present → `"wpgen.toml not found (optional)"`, treated as pass | — |
| TOML parse + env eval | `WpGenConfig::load_from_path()` calls `env_load_toml` | Error |
| Unknown field rejection | `WpGenConfig`, `GeneratorConfig`, `OutputConfig` all use `#[serde(deny_unknown_fields)]` | Error |
| **version non-empty** ★P0 | `version` must not be empty | Error |
| **generator.count > 0** ★P0 | If `count` is set, must be > 0 | Error |
| **generator.speed = 0** ★P0 | `0` means unlimited, valid value | Info |
| **generator.parallel > 0** ★P0 | `parallel` must be > 0 | Error |
| **speed_profile params** ★P0 | Each variant: rate/base/amplitude > 0, period_secs/duration_secs > 0, burst_probability/variance ∈ [0,1], steps non-empty | Error |
| **logging.level** ★P0 | Each comma-separated segment's level part must be trace/debug/info/warn/error; supports `module=level` format (e.g., `info,ctrl=info`) | Error |
| **logging.output** ★P0 | Must be stdout/console/file/both | Error |
| **rule_root path** ★P0 | Directory specified by `rule_root` must exist (resolved relative to work_root) | Error |
| **sample_pattern glob** ★P0 | `sample_pattern` must be a valid glob pattern | Error |
| **output.connect reference** ★P0 | `output.connect` must reference an existing sink connector in `connectors/sink.d/` | Error |
| **logging.file_path parent dir** | Parent directory existence (created at runtime; warning only) | Warning |

## Fail-Fast Mechanism

When `--fail-fast` is enabled, any component check failure immediately stops subsequent checks for that target. Checks run in order 1→8; Engine failure skips all remaining checks.

---

## P4 High-Risk Items (Planned, not yet implemented)

The following items carry higher technical risk and are documented for future reference:

| # | Item | Risk |
|---|------|------|
| 1 | **WPL field type semantic validation** | `WplField::validate()` uses `panic!` instead of returning errors; calling it from the check path could crash the process |
| 2 | **WPL Sample data validation** | Requires the runtime engine (`WplEngine`) to load pipeline context, involving tokio async runtime and full pipeline initialization |
| 3 | **OML structure parsing (oml_parse_raw)** | Core parser `oml_parse_raw` is async; the check path is sync. Requires introducing a tokio runtime or refactoring the parser layer |
| 4 | **WPL-OML rule cross-references** | OML `rule : /path/*` matching against WPL rule paths happens at runtime (`SinkDispatcher::get_match_oml`); check would need both full rule sets loaded and match logic executed |
| 5 | **Connectors reverse cross-references** | Requires modifying `Connectors::check()` signature to accept Engine config, and loading source config + sink route config in the check path to verify connector reference validity. Involves cross-component dependency injection |
| 6 | **Sink OML pattern existence validation** | Sink route `oml = ["pattern"]` entries must match loaded OML model names; current matching logic lives in the runtime dispatcher. Migrating to check requires loading full OML models and route configs |

### Priority Recommendations

- **P2 (recommended next phase)**: #5 Connectors reverse cross-references — risk is manageable; build layer already has parsing logic, only needs integration into the check flow
- **P3 (design review needed)**: #3 OML structure parsing — consider making `oml_parse_raw` synchronous or providing a sync wrapper
- **P4 (architecture change needed)**: #1 #2 #4 #6 — require introducing runtime components into the check path or refactoring core parsing APIs
