//! Site-local overlay (`wparse.local.toml`).
//!
//! `conf/` is a `project_remote`-managed directory: every infra sync replaces
//! it wholesale. Settings that belong to the deployment site (admin API bind,
//! TLS, token path, runtime state directory, startup validation policy) must
//! live outside it, otherwise a remote publish silently resets them.
//!
//! The overlay is a sibling of `conf/`, `models/`, `topology/` and
//! `connectors/`, none of which include it.

use std::fs;
use std::path::Path;

use crate::compat::UvsFrom;
use orion_conf::EnvTomlLoad;
use orion_error::conversion::ToStructError;
use orion_variate::EnvDict;
use wp_config::engine::{AdminApiConf, RuntimeConf, ValidateConf};
use wp_error::run_error::{RunReason, RunResult};

use crate::LOCAL_CONF_FILE;

#[derive(Debug, Default)]
pub struct SiteOverlay {
    pub admin_api: Option<AdminApiConf>,
    pub runtime: Option<RuntimeConf>,
    pub validate: Option<ValidateConf>,
}

impl SiteOverlay {
    /// Explicit site-local `[validate].deny_duplicate_oml`, if configured.
    ///
    /// `None` means "inherit from `conf/wparse.toml`"; `Some(false)` is an
    /// explicit site override that turns the engine config value off.
    pub fn deny_duplicate_oml(&self) -> Option<bool> {
        self.validate
            .as_ref()
            .and_then(|validate| validate.deny_duplicate_oml)
    }
}

/// Load `wparse.local.toml` when present.
///
/// Returns `Ok(None)` when the file does not exist. A file that contains none
/// of `[admin_api]`, `[runtime]` or `[validate]` is an error, because it
/// usually means the section name is misspelled and the site would silently
/// fall back to the remote-managed config.
pub fn load_site_overlay(work_root: &Path, dict: &EnvDict) -> RunResult<Option<SiteOverlay>> {
    let path = work_root.join(LOCAL_CONF_FILE);
    if !path.is_file() {
        return Ok(None);
    }

    let body = fs::read_to_string(&path).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!("read {} failed", path.display()))
    })?;
    let raw: toml::Value = toml::from_str(&body).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!("parse {} failed", path.display()))
    })?;

    let admin_api = match raw.get("admin_api") {
        Some(section) => Some(parse_admin_api_section(section, &path, dict)?),
        None => None,
    };
    let runtime = match raw.get("runtime") {
        Some(section) => Some(parse_runtime_section(section, &path, dict)?),
        None => None,
    };
    let validate = match raw.get("validate") {
        Some(section) => Some(parse_validate_section(section, &path, dict)?),
        None => None,
    };

    if admin_api.is_none() && runtime.is_none() && validate.is_none() {
        return Err(conf_err(format!(
            "{} exists but has none of [admin_api], [runtime] or [validate]; add one or remove the file",
            path.display()
        )));
    }

    Ok(Some(SiteOverlay {
        admin_api,
        runtime,
        validate,
    }))
}

fn parse_admin_api_section(
    section: &toml::Value,
    path: &Path,
    dict: &EnvDict,
) -> RunResult<AdminApiConf> {
    let body = toml::to_string(section).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!(
                "serialize [admin_api] from {} failed",
                path.display()
            ))
    })?;
    AdminApiConf::env_parse_toml(&body, dict).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!("parse [admin_api] in {} failed", path.display()))
    })
}

fn parse_runtime_section(
    section: &toml::Value,
    path: &Path,
    dict: &EnvDict,
) -> RunResult<RuntimeConf> {
    let body = toml::to_string(section).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!(
                "serialize [runtime] from {} failed",
                path.display()
            ))
    })?;
    RuntimeConf::env_parse_toml(&body, dict).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!("parse [runtime] in {} failed", path.display()))
    })
}

fn parse_validate_section(
    section: &toml::Value,
    path: &Path,
    dict: &EnvDict,
) -> RunResult<ValidateConf> {
    let body = toml::to_string(section).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!(
                "serialize [validate] from {} failed",
                path.display()
            ))
    })?;
    ValidateConf::env_parse_toml(&body, dict).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_source(err)
            .with_detail(format!("parse [validate] in {} failed", path.display()))
    })
}

fn conf_err(detail: String) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_overlay(dir: &Path, body: &str) {
        fs::write(dir.join(LOCAL_CONF_FILE), body).expect("write overlay");
    }

    #[test]
    fn missing_overlay_returns_none() {
        let dir = tempdir().expect("tempdir");
        assert!(load_site_overlay(dir.path(), &EnvDict::new())
            .expect("load")
            .is_none());
    }

    #[test]
    fn runtime_only_overlay_is_accepted() {
        let dir = tempdir().expect("tempdir");
        write_overlay(dir.path(), "[runtime]\nstate_dir = \"/state\"\n");

        let overlay = load_site_overlay(dir.path(), &EnvDict::new())
            .expect("load")
            .expect("overlay");
        assert!(overlay.admin_api.is_none());
        assert_eq!(
            overlay.runtime.as_ref().map(|r| r.state_dir.as_str()),
            Some("/state")
        );
    }

    #[test]
    fn admin_api_only_overlay_is_accepted() {
        let dir = tempdir().expect("tempdir");
        write_overlay(
            dir.path(),
            "[admin_api]\nenabled = true\nbind = \"127.0.0.1:19090\"\n",
        );

        let overlay = load_site_overlay(dir.path(), &EnvDict::new())
            .expect("load")
            .expect("overlay");
        assert!(overlay.admin_api.as_ref().is_some_and(|c| c.enabled));
        assert!(overlay.runtime.is_none());
    }

    #[test]
    fn empty_overlay_is_rejected() {
        let dir = tempdir().expect("tempdir");
        write_overlay(dir.path(), "version = \"1.0\"\n");

        let err = load_site_overlay(dir.path(), &EnvDict::new()).unwrap_err();
        assert!(err.to_string().contains("none of [admin_api]"), "{err}");
    }

    #[test]
    fn validate_only_overlay_is_accepted() {
        let dir = tempdir().expect("tempdir");
        write_overlay(dir.path(), "[validate]\ndeny_duplicate_oml = true\n");

        let overlay = load_site_overlay(dir.path(), &EnvDict::new())
            .expect("load")
            .expect("overlay");
        assert_eq!(overlay.deny_duplicate_oml(), Some(true));
    }

    #[test]
    fn validate_explicit_false_is_kept() {
        let dir = tempdir().expect("tempdir");
        write_overlay(dir.path(), "[validate]\ndeny_duplicate_oml = false\n");

        let overlay = load_site_overlay(dir.path(), &EnvDict::new())
            .expect("load")
            .expect("overlay");
        assert_eq!(overlay.deny_duplicate_oml(), Some(false));
    }

    #[test]
    fn validate_unknown_field_is_rejected() {
        let dir = tempdir().expect("tempdir");
        write_overlay(
            dir.path(),
            "[validate]\ndeny_duplicate_oml = true\ntypo = 1\n",
        );

        assert!(load_site_overlay(dir.path(), &EnvDict::new()).is_err());
    }

    #[test]
    fn unknown_runtime_field_is_rejected() {
        let dir = tempdir().expect("tempdir");
        write_overlay(dir.path(), "[runtime]\nstate = \"/state\"\n");

        assert!(load_site_overlay(dir.path(), &EnvDict::new()).is_err());
    }
}
