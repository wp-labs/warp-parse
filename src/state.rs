//! Runtime state directory resolution for the daemon and maintenance commands.
//!
//! The engine owns the actual directory logic ([`wp_config::state::StateDir`]);
//! this module only decides which configured value wins:
//!
//! 1. CLI `--state-dir`
//! 2. `wparse.local.toml` `[runtime].state_dir` (site-local, survives a
//!    `project_remote` publish)
//! 3. `conf/wparse.toml` `[runtime].state_dir`
//! 4. `<work_root>/.run`
//!
//! The resolved value is cached per work root for the process lifetime. A
//! `project_remote` update replaces `conf/` — possibly with a broken config —
//! before the rollback path runs, and rollback must still find its backup in the
//! directory that was resolved *before* the update. The engine itself also fixes
//! its state directory at startup, so a running process never moves it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::compat::UvsFrom;
use orion_error::conversion::{SourceErr, ToStructError};
use orion_variate::{EnvDict, EnvEvaluable};
use wp_config::engine::EngineConfig;
use wp_config::state::StateDir;
use wp_error::run_error::{RunReason, RunResult};

use crate::local_conf;

/// Directories replaced wholesale by a `project_remote` publish.
///
/// A state directory inside one of them would be backed up and published to the
/// remote repository.
pub const MANAGED_DIRS: &[&str] = &["conf", "models", "topology", "connectors"];

/// Resolve the effective state directory for a daemon/rescue start.
///
/// `cli_override` (`--state-dir`) wins over the configuration. The result is
/// primed into the process cache so maintenance paths
/// (`project_remote` cache/backup, cleanup) use the same directory even after
/// `conf/` is replaced.
pub fn resolve_effective_state_dir(
    work_root: &Path,
    dict: &EnvDict,
    cli_override: Option<&str>,
) -> RunResult<StateDir> {
    let dir = match cli_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(raw) => resolve_raw(work_root, raw)?,
        None => resolve_state_dir(work_root, dict)?,
    };
    prime_state_dir(work_root, dir.clone());
    Ok(dir)
}

/// Resolve the state directory from the configuration (overlay first).
///
/// An invalid configured value is an error; it is never silently replaced by the
/// default, because that would split engine state between two directories.
pub fn resolve_state_dir(work_root: &Path, dict: &EnvDict) -> RunResult<StateDir> {
    let overlay_state = local_conf::load_site_overlay(work_root, dict)?
        .and_then(|overlay| overlay.runtime)
        .map(|runtime| runtime.state_dir)
        .unwrap_or_default();

    let raw = if overlay_state.trim().is_empty() {
        EngineConfig::load(work_root, dict)
            .source_err(
                RunReason::from_conf(),
                format!(
                    "load {} failed",
                    work_root.join("conf/wparse.toml").display()
                ),
            )?
            .env_eval(dict)
            .conf_absolutize(work_root)
            .runtime_state_dir()
            .to_string()
    } else {
        overlay_state
    };

    resolve_raw(work_root, &raw)
}

/// Resolution for maintenance paths (`project_remote` cache, cleanup).
///
/// A missing engine config falls back to `<work_root>/.run` so maintenance
/// commands still work on bare directories; a present-but-invalid
/// `[runtime].state_dir` remains an error.
///
/// Only the process environment is available here (no secret dictionary), so a
/// `state_dir` that references `${SEC_*}` fails loudly instead of silently
/// resolving to a different directory than the daemon.
pub fn resolve_state_dir_maintenance(work_root: &Path) -> RunResult<StateDir> {
    let key = cache_key(work_root);
    if let Some(dir) = state_dir_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        return Ok(dir);
    }

    let dir = compute_maintenance_state_dir(work_root)?;
    if let Ok(mut cache) = state_dir_cache().lock() {
        cache.insert(key, dir.clone());
    }
    Ok(dir)
}

/// Remember the state directory resolved by the process entry point.
pub fn prime_state_dir(work_root: &Path, state_dir: StateDir) {
    if let Ok(mut cache) = state_dir_cache().lock() {
        cache.insert(cache_key(work_root), state_dir);
    }
}

fn compute_maintenance_state_dir(work_root: &Path) -> RunResult<StateDir> {
    if work_root.join("conf/wparse.toml").is_file() {
        return resolve_state_dir(work_root, &EnvDict::default());
    }

    if let Some(overlay) = local_conf::load_site_overlay(work_root, &EnvDict::default())? {
        if let Some(runtime) = overlay.runtime {
            return resolve_raw(work_root, &runtime.state_dir);
        }
    }
    Ok(StateDir::default_for(work_root))
}

fn resolve_raw(work_root: &Path, raw: &str) -> RunResult<StateDir> {
    let dir =
        StateDir::resolve(work_root, raw).map_err(|err| invalid_state_dir_err(err.to_string()))?;
    dir.reject_inside(work_root, MANAGED_DIRS)
        .map_err(|err| invalid_state_dir_err(err.to_string()))?;
    Ok(dir)
}

fn invalid_state_dir_err(detail: String) -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail(format!("[runtime].state_dir is invalid: {detail}"))
}

fn state_dir_cache() -> &'static Mutex<HashMap<PathBuf, StateDir>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, StateDir>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Canonical key so `/tmp/x` and `/private/tmp/x` (macOS) share one entry.
fn cache_key(work_root: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(work_root) {
        return canonical;
    }
    if work_root.is_absolute() {
        return work_root.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(work_root))
        .unwrap_or_else(|_| work_root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_engine_conf(dir: &Path, state_dir: &str) {
        fs::create_dir_all(dir.join("conf")).expect("create conf dir");
        fs::write(
            dir.join("conf/wparse.toml"),
            format!("version = \"1.0\"\n\n[runtime]\nstate_dir = \"{state_dir}\"\n"),
        )
        .expect("write engine conf");
    }

    #[test]
    fn engine_config_state_dir_is_used() {
        let dir = tempdir().expect("tempdir");
        write_engine_conf(dir.path(), "state");

        let resolved = resolve_state_dir(dir.path(), &EnvDict::new()).expect("resolve");
        assert_eq!(resolved.path(), dir.path().join("state"));
    }

    #[test]
    fn cli_override_wins() {
        let dir = tempdir().expect("tempdir");
        write_engine_conf(dir.path(), "from-conf");

        let resolved = resolve_effective_state_dir(dir.path(), &EnvDict::new(), Some("from-cli"))
            .expect("resolve");
        assert_eq!(resolved.path(), dir.path().join("from-cli"));
    }

    #[test]
    fn site_overlay_wins_over_engine_config() {
        let dir = tempdir().expect("tempdir");
        write_engine_conf(dir.path(), "from-conf");
        fs::write(
            dir.path().join(crate::LOCAL_CONF_FILE),
            "[runtime]\nstate_dir = \"from-overlay\"\n",
        )
        .expect("write overlay");

        let resolved = resolve_state_dir(dir.path(), &EnvDict::new()).expect("resolve");
        assert_eq!(resolved.path(), dir.path().join("from-overlay"));
    }

    #[test]
    fn default_is_dot_run() {
        let dir = tempdir().expect("tempdir");
        write_engine_conf(dir.path(), "");

        let resolved = resolve_state_dir(dir.path(), &EnvDict::new()).expect("resolve");
        assert_eq!(resolved.path(), dir.path().join(".run"));
    }

    #[test]
    fn managed_directory_is_rejected() {
        let dir = tempdir().expect("tempdir");
        write_engine_conf(dir.path(), "models/state");

        let err = resolve_state_dir(dir.path(), &EnvDict::new()).unwrap_err();
        assert!(err.to_string().contains("project_remote-managed"), "{err}");
    }

    #[test]
    fn maintenance_falls_back_without_engine_config() {
        let dir = tempdir().expect("tempdir");
        let resolved = resolve_state_dir_maintenance(dir.path()).expect("resolve");
        assert_eq!(resolved.path(), dir.path().join(".run"));
    }

    #[test]
    fn maintenance_honours_overlay_without_engine_config() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join(crate::LOCAL_CONF_FILE),
            "[runtime]\nstate_dir = \"state\"\n",
        )
        .expect("write overlay");

        let resolved = resolve_state_dir_maintenance(dir.path()).expect("resolve");
        assert_eq!(resolved.path(), dir.path().join("state"));
    }

    #[test]
    fn maintenance_rejects_unresolved_env() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join(crate::LOCAL_CONF_FILE),
            "[runtime]\nstate_dir = \"${WPARSE_STATE_DIR_NOT_SET}\"\n",
        )
        .expect("write overlay");

        assert!(resolve_state_dir_maintenance(dir.path()).is_err());
    }

    #[test]
    fn maintenance_survives_a_broken_config_after_priming() {
        let dir = tempdir().expect("tempdir");
        write_engine_conf(dir.path(), "state");

        let primed = resolve_state_dir_maintenance(dir.path()).expect("resolve");
        assert_eq!(primed.path(), dir.path().join("state"));

        // A project_remote publish may replace conf/ with a broken config; the
        // rollback path must keep using the primed directory.
        fs::write(dir.path().join("conf/wparse.toml"), "not toml [[[").expect("break config");
        let again = resolve_state_dir_maintenance(dir.path()).expect("cached resolve");
        assert_eq!(again, primed);
    }
}
