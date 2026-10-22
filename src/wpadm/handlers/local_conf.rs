//! Local overlay helpers.
//!
//! `conf/` is a `project_remote`-managed directory: every infra sync replaces
//! it wholesale. Fields that belong to the deployment site (admin API bind,
//! TLS, token path) must live outside it, otherwise a remote publish silently
//! resets the remote-control port and the node becomes unreachable.
//!
//! The overlay file is `<work_root>/wparse.local.toml`. It is intentionally a
//! sibling of `conf/`, `models/`, `topology/` and `connectors/`, none of which
//! include it.

use std::fs;
use std::path::{Path, PathBuf};

use orion_error::conversion::ToStructError;
use serde::Serialize;
use warp_parse::compat::UvsFrom;
use warp_parse::LOCAL_CONF_FILE;
use wp_config::engine::AdminApiConf;
use wp_error::run_error::{RunReason, RunResult};

const LOCAL_CONF_HEADER: &str = "\
# 本地覆盖层：不随 project_remote 发布，请勿放入 conf/
# Local overlay: not managed by project_remote sync.
";

const HOME_ADMIN_API_TOKEN_FILE: &str = "${HOME}/.warp_parse/admin_api.token";
const PROJECT_ADMIN_API_TOKEN_FILE: &str = "runtime/admin_api.token";

const DEFAULT_ADMIN_API_TEMPLATE: &str = r#"[admin_api]
enabled = false
bind = "127.0.0.1:19090"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.tls]
enabled = false
cert_file = ""
key_file = ""

[admin_api.auth]
mode = "bearer_token"
token_file = "{token}"
"#;

fn home_available() -> bool {
    std::env::var_os("HOME").is_some()
}

fn default_admin_api_body(home_available: bool) -> String {
    let token = if home_available {
        HOME_ADMIN_API_TOKEN_FILE
    } else {
        PROJECT_ADMIN_API_TOKEN_FILE
    };
    DEFAULT_ADMIN_API_TEMPLATE.replace("{token}", token)
}

pub(crate) fn local_conf_path(work_root: &Path) -> PathBuf {
    work_root.join(LOCAL_CONF_FILE)
}

/// Ensure the local overlay exists.
///
/// Used by `wproj init` and other bootstrap paths: migrate a legacy
/// `[admin_api]` block when present, otherwise write the default (disabled)
/// block.
pub(crate) fn ensure_local_admin_api_conf(work_root: &Path) -> RunResult<()> {
    if !work_root.join("conf/wparse.toml").exists() {
        return Ok(());
    }
    if !migrate_local_admin_api_conf(work_root)? {
        let path = local_conf_path(work_root);
        if !path.exists() {
            write_overlay(&path, &default_admin_api_body(home_available()))?;
        }
    }
    strip_legacy_admin_api(work_root)
}

/// Move `[admin_api]` from `conf/wparse.toml` into the local overlay.
///
/// Returns `true` when a new overlay file was created. Existing overlay files
/// are never overwritten.
///
/// Must run before any `project_remote` sync, because infra sync deletes and
/// recreates `conf/`.
pub(crate) fn migrate_local_admin_api_conf(work_root: &Path) -> RunResult<bool> {
    let overlay_path = local_conf_path(work_root);
    if overlay_path.exists() {
        return Ok(false);
    }

    let engine_path = work_root.join("conf/wparse.toml");
    let body = match fs::read_to_string(&engine_path) {
        Ok(body) => body,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(conf_err_source(
                format!("read {} failed", engine_path.display()),
                err,
            ))
        }
    };

    let raw: toml::Value = toml::from_str(&body).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "parse {} failed: {}",
            engine_path.display(),
            e
        ))
    })?;
    let Some(admin) = raw.get("admin_api").cloned() else {
        return Ok(false);
    };

    let mut admin: AdminApiConf = admin.try_into().map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "parse [admin_api] in {} failed: {}",
            engine_path.display(),
            e
        ))
    })?;
    normalize_token_path(&mut admin, home_available());
    let rendered = render_admin_api_section(&admin)?;
    write_overlay(&overlay_path, &rendered)?;
    Ok(true)
}

fn normalize_token_path(admin: &mut AdminApiConf, home_available: bool) {
    if home_available {
        if admin.auth.token_file == PROJECT_ADMIN_API_TOKEN_FILE {
            admin.auth.token_file = HOME_ADMIN_API_TOKEN_FILE.to_string();
        }
    } else if admin.auth.token_file == HOME_ADMIN_API_TOKEN_FILE {
        admin.auth.token_file = PROJECT_ADMIN_API_TOKEN_FILE.to_string();
    }
}

fn strip_legacy_admin_api(work_root: &Path) -> RunResult<()> {
    let engine_path = work_root.join("conf/wparse.toml");
    let body = match fs::read_to_string(&engine_path) {
        Ok(body) => body,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(conf_err_source(
                format!("read {} failed", engine_path.display()),
                err,
            ))
        }
    };
    // Only rewrite a valid TOML document that actually contains the legacy
    // section; otherwise leave user content untouched and let the normal
    // config loader report the problem.
    let parsed: toml::Value = match toml::from_str(&body) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(()),
    };
    if parsed.get("admin_api").is_none() {
        return Ok(());
    }
    let stripped = strip_admin_api_section(&body);
    if stripped != body {
        fs::write(&engine_path, stripped)
            .map_err(|e| conf_err_source(format!("write {} failed", engine_path.display()), e))?;
    }
    Ok(())
}

fn strip_admin_api_section(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_admin = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[admin_api]") || trimmed.starts_with("[admin_api.") {
            in_admin = true;
            continue;
        }
        if in_admin {
            if trimmed.starts_with('[') {
                in_admin = false;
            } else {
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

#[derive(Serialize)]
struct LocalOverlay<'a> {
    admin_api: &'a AdminApiConf,
}

fn render_admin_api_section(admin: &AdminApiConf) -> RunResult<String> {
    let body = toml::to_string_pretty(&LocalOverlay { admin_api: admin }).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("serialize [admin_api] failed: {}", e))
    })?;
    Ok(body)
}

fn write_overlay(path: &Path, body: &str) -> RunResult<()> {
    let mut content = String::from(LOCAL_CONF_HEADER);
    content.push('\n');
    content.push_str(body.trim_start_matches('\n'));
    if !content.ends_with('\n') {
        content.push('\n');
    }
    fs::write(path, content)
        .map_err(|e| conf_err_source(format!("write {} failed", path.display()), e))
}

fn conf_err_source<E>(detail: impl Into<String>, source: E) -> wp_error::RunError
where
    E: std::error::Error + Send + Sync + 'static,
{
    RunReason::from_conf()
        .to_err()
        .with_source(source)
        .with_detail(detail.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn read(path: &Path) -> String {
        fs::read_to_string(path).expect("read file")
    }

    fn expected_token_path() -> &'static str {
        if home_available() {
            HOME_ADMIN_API_TOKEN_FILE
        } else {
            PROJECT_ADMIN_API_TOKEN_FILE
        }
    }

    #[test]
    fn ensure_creates_default_overlay_once() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(conf_dir.join("wparse.toml"), "version = \"1.0\"\n").expect("write engine");

        ensure_local_admin_api_conf(temp.path()).expect("ensure overlay");
        ensure_local_admin_api_conf(temp.path()).expect("ensure overlay twice");

        let body = read(&local_conf_path(temp.path()));
        assert_eq!(body.matches("[admin_api]").count(), 1);
        assert!(body.contains("bind = \"127.0.0.1:19090\""));
        assert!(body.contains(expected_token_path()));
    }

    #[test]
    fn default_body_uses_home_token_when_available() {
        let body = default_admin_api_body(true);
        assert!(body.contains(HOME_ADMIN_API_TOKEN_FILE));
        assert!(!body.contains(&format!(
            "token_file = \"{}\"",
            PROJECT_ADMIN_API_TOKEN_FILE
        )));
    }

    #[test]
    fn default_body_falls_back_to_project_token() {
        let body = default_admin_api_body(false);
        assert!(body.contains(&format!(
            "token_file = \"{}\"",
            PROJECT_ADMIN_API_TOKEN_FILE
        )));
        assert!(!body.contains("${HOME}"));
    }

    #[test]
    fn normalize_token_path_switches_both_ways() {
        let mut home = AdminApiConf::default();
        home.auth.token_file = PROJECT_ADMIN_API_TOKEN_FILE.to_string();
        normalize_token_path(&mut home, true);
        assert_eq!(home.auth.token_file, HOME_ADMIN_API_TOKEN_FILE);

        let mut project = AdminApiConf::default();
        project.auth.token_file = HOME_ADMIN_API_TOKEN_FILE.to_string();
        normalize_token_path(&mut project, false);
        assert_eq!(project.auth.token_file, PROJECT_ADMIN_API_TOKEN_FILE);

        let mut custom = AdminApiConf::default();
        custom.auth.token_file = "custom.token".to_string();
        normalize_token_path(&mut custom, true);
        assert_eq!(custom.auth.token_file, "custom.token");
    }

    #[test]
    fn ensure_is_noop_without_engine_config() {
        let temp = tempdir().expect("tempdir");
        ensure_local_admin_api_conf(temp.path()).expect("ensure overlay");
        assert!(!local_conf_path(temp.path()).exists());
    }

    #[test]
    fn ensure_strips_legacy_section_after_migration() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            "version = \"1.0\"\n\n[admin_api]\nenabled = true\nbind = \"0.0.0.0:19999\"\n",
        )
        .expect("write engine");

        ensure_local_admin_api_conf(temp.path()).expect("ensure overlay");

        let engine = read(&conf_dir.join("wparse.toml"));
        assert!(!engine.contains("[admin_api]"));
        assert!(engine.contains("version = \"1.0\""));
        assert!(read(&local_conf_path(temp.path())).contains("0.0.0.0:19999"));
    }

    #[test]
    fn ensure_does_not_rewrite_invalid_engine_config() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        let invalid = "version = \"1.0\n[admin_api]\nenabled = true\n";
        fs::write(conf_dir.join("wparse.toml"), invalid).expect("write invalid engine");
        fs::write(
            local_conf_path(temp.path()),
            "[admin_api]\nenabled = false\n",
        )
        .expect("write overlay");

        ensure_local_admin_api_conf(temp.path()).expect("ensure overlay");
        assert_eq!(read(&conf_dir.join("wparse.toml")), invalid);
    }

    #[test]
    fn strip_removes_admin_section_at_end_of_file() {
        let body = "version = \"1.0\"\n\n[admin_api]\nenabled = true\n";
        assert_eq!(strip_admin_api_section(body), "version = \"1.0\"\n");
    }

    #[test]
    fn strip_removes_admin_section_and_keeps_others() {
        let body = "version = \"1.0\"\n\n[admin_api]\nenabled = true\n\n[admin_api.auth]\ntoken_file = \"x\"\n\n[topology]\nsources = \"./s\"\n";
        let out = strip_admin_api_section(body);
        assert!(!out.contains("admin_api"));
        assert!(out.contains("version = \"1.0\""));
        assert!(out.contains("[topology]"));
    }

    #[test]
    fn migrate_moves_legacy_admin_api_values() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            r#"version = "1.0"

[admin_api]
enabled = true
bind = "0.0.0.0:19999"

[admin_api.auth]
mode = "bearer_token"
token_file = "runtime/admin_api.token"
"#,
        )
        .expect("write engine");

        assert!(migrate_local_admin_api_conf(temp.path()).expect("migrate"));
        assert!(!migrate_local_admin_api_conf(temp.path()).expect("migrate twice"));

        let body = read(&local_conf_path(temp.path()));
        assert!(body.contains("enabled = true"));
        assert!(body.contains("bind = \"0.0.0.0:19999\""));
        assert!(body.contains(expected_token_path()));
    }

    #[test]
    fn migrate_does_not_overwrite_existing_overlay() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            "[admin_api]\nenabled = true\nbind = \"0.0.0.0:19999\"\n",
        )
        .expect("write engine");

        let overlay = local_conf_path(temp.path());
        fs::write(
            &overlay,
            "[admin_api]\nenabled = false\nbind = \"127.0.0.1:19090\"\n",
        )
        .expect("write overlay");

        assert!(!migrate_local_admin_api_conf(temp.path()).expect("migrate"));
        assert!(read(&overlay).contains("127.0.0.1:19090"));
    }
}
