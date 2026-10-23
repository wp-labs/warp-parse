//! Admin API token tooling: `wpadm token generate|show|check`.
//!
//! The engine only ever reads a token file. Callers may obtain the value from a
//! file, an environment variable, or the local `wparse.local.toml`, so the
//! resolution chain implemented here deliberately differs from the engine's.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use orion_error::conversion::ToStructError;
use orion_variate::EnvDict;
use warp_parse::admin_api::{self, AdminApiClientProfile};
use warp_parse::compat::UvsFrom;
use warp_parse::load_sec_dict;
use warp_parse::token;
use wp_error::run_error::{RunReason, RunResult};

use crate::args::{TokenCheckArgs, TokenGenerateArgs, TokenShowArgs};
use crate::handlers::engine::{resolve_override_path, resolve_work_root};

/// Default token path used when no other source is configured.
const DEFAULT_HOME_TOKEN_REL: &str = ".warp_parse/admin_api.token";

#[derive(Debug, Clone)]
pub enum TokenSource {
    File(PathBuf),
    Env(String),
}

impl std::fmt::Display for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenSource::File(path) => write!(f, "file:{}", path.display()),
            TokenSource::Env(name) => write!(f, "env:{}", name),
        }
    }
}

pub struct ResolvedToken {
    pub token: String,
    pub source: TokenSource,
}

impl ResolvedToken {
    pub fn fingerprint(&self) -> String {
        token::fingerprint(&self.token)
    }
}

/// Resolve the token used by `wpadm`.
///
/// Precedence: explicit `--token-file` > explicit `--token-env` >
/// `$WPARSE_ADMIN_TOKEN` > `token_file` from the local admin API profile.
pub fn resolve_token(
    token_file: Option<&Path>,
    token_env: Option<&str>,
    dict: &EnvDict,
    profile: Option<&AdminApiClientProfile>,
) -> RunResult<ResolvedToken> {
    if let Some(path) = token_file {
        let token = read_token_file(path)?;
        return Ok(ResolvedToken {
            token,
            source: TokenSource::File(path.to_path_buf()),
        });
    }

    if let Some(name) = token_env {
        let token = lookup_var(dict, name).ok_or_else(|| {
            token_err(format!(
                "environment variable '{name}' is not set or empty (use --token-file instead)"
            ))
        })?;
        return Ok(ResolvedToken {
            token,
            source: TokenSource::Env(name.to_string()),
        });
    }

    if let Some(token) = lookup_var(dict, token::TOKEN_ENV_VAR) {
        return Ok(ResolvedToken {
            token,
            source: TokenSource::Env(token::TOKEN_ENV_VAR.to_string()),
        });
    }

    if let Some(profile) = profile {
        let token = read_token_file(&profile.token_file)?;
        return Ok(ResolvedToken {
            token,
            source: TokenSource::File(profile.token_file.clone()),
        });
    }

    Err(token_err(format!(
        "no token source: pass --token-file, --token-env, set {}, or configure token_file in wparse.local.toml",
        token::TOKEN_ENV_VAR
    )))
}

fn lookup_var(dict: &EnvDict, name: &str) -> Option<String> {
    if let Some(value) = dict.get(name) {
        let text = value.to_string();
        if !text.trim().is_empty() {
            return Some(text.trim().to_string());
        }
    }
    match env::var(name) {
        Ok(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        _ => None,
    }
}

fn read_token_file(path: &Path) -> RunResult<String> {
    let body = fs::read_to_string(path)
        .map_err(|e| token_err_source(format!("read token file {} failed", path.display()), e))?;
    token::parse_single_line(&body)
        .map(str::to_string)
        .map_err(|err| token_err(format!("token file {}: {err}", path.display())))
}

/// Load the local admin API profile, tolerating config errors.
///
/// `token show` / `token check` are often used with `--token-env` in CI where no
/// project config exists; a broken local profile must not block them.
pub fn optional_local_profile(work_root: &Path, dict: &EnvDict) -> Option<AdminApiClientProfile> {
    match admin_api::resolve_client_profile(work_root, dict) {
        Ok(profile) => profile,
        Err(err) => {
            wp_log::warn_ctrl!("resolve local admin api profile failed: {}", err);
            None
        }
    }
}

pub fn run_token_generate(args: TokenGenerateArgs) -> RunResult<()> {
    let path = match args.out.as_deref() {
        Some(raw) if !raw.trim().is_empty() => PathBuf::from(raw.trim()),
        _ => default_home_token_path().ok_or_else(|| {
            token_err("HOME is not set; pass --out to choose a token path".to_string())
        })?,
    };

    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(token_err(format!(
                "refusing to write {}: path is a symlink; remove it or choose another --out",
                path.display()
            )));
        }
        Ok(meta) if meta.is_dir() => {
            return Err(token_err(format!(
                "refusing to write {}: path is a directory",
                path.display()
            )));
        }
        Ok(_) if !args.force => {
            return Err(token_err(format!(
                "{} already exists; pass --force to overwrite",
                path.display()
            )));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(token_err_source(
                format!("stat {} failed", path.display()),
                err,
            ));
        }
    }

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|e| token_err_source(format!("create {} failed", parent.display()), e))?;
    }

    atomic_write_private(&path, &token::generate_token())?;
    println!("{}", path.display());
    Ok(())
}

/// Write a single-line token atomically with `0600`.
///
/// The temporary file is created with the final permissions (`create_new` +
/// `mode`), so there is no window where the token is world-readable, and
/// `rename` makes the replacement atomic.
fn atomic_write_private(path: &Path, token_value: &str) -> RunResult<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| token_err(format!("invalid token path {}", path.display())))?;
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or_default();
    let tmp = parent.join(format!(".{file_name}.tmp-{}-{suffix}", std::process::id()));
    let _ = fs::remove_file(&tmp);

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(&tmp)
        .map_err(|e| token_err_source(format!("create {} failed", tmp.display()), e))?;
    file.write_all(token_value.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.sync_all())
        .map_err(|e| {
            let _ = fs::remove_file(&tmp);
            token_err_source(format!("write {} failed", tmp.display()), e)
        })?;
    drop(file);

    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        token_err_source(
            format!("rename {} -> {} failed", tmp.display(), path.display()),
            e,
        )
    })
}

pub fn run_token_show(args: TokenShowArgs) -> RunResult<()> {
    let work_root = resolve_work_root(&args.work_root)?;
    let dict = load_sec_dict()?;
    let profile = optional_local_profile(&work_root, &dict);
    let override_path = args
        .token_file
        .as_deref()
        .map(|raw| resolve_override_path(&work_root, raw));
    let resolved = resolve_token(
        override_path.as_deref(),
        args.token_env.as_deref(),
        &dict,
        profile.as_ref(),
    )?;

    // Value on stdout (pipeable); diagnostics on stderr.
    println!("{}", resolved.token);
    eprintln!("token_fp: {}", resolved.fingerprint());
    eprintln!("source: {}", resolved.source);
    Ok(())
}

pub fn run_token_check(args: TokenCheckArgs) -> RunResult<()> {
    let work_root = resolve_work_root(&args.work_root)?;
    let dict = load_sec_dict()?;
    let profile = optional_local_profile(&work_root, &dict);

    let path = match args.file.as_deref() {
        Some(raw) if !raw.trim().is_empty() => resolve_override_path(&work_root, raw.trim()),
        _ => profile
            .as_ref()
            .map(|profile| profile.token_file.clone())
            .or_else(default_home_token_path)
            .ok_or_else(|| {
                token_err(
                    "no token file to check: pass --file, or configure token_file in wparse.local.toml"
                        .to_string(),
                )
            })?,
    };

    inspect_token_file(&path, args.require_owner_only)
}

fn inspect_token_file(path: &Path, require_owner_only: bool) -> RunResult<()> {
    let mut problems: Vec<String> = Vec::new();

    println!("path: {}", path.display());

    let link_meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("type: missing");
            println!("verdict: FAILED");
            return Err(token_err(format!("{} does not exist", path.display())));
        }
        Err(err) => {
            return Err(token_err_source(
                format!("stat {} failed", path.display()),
                err,
            ));
        }
    };

    if link_meta.file_type().is_symlink() {
        println!("symlink: yes");
        match fs::canonicalize(path) {
            Ok(target) => println!("resolved: {}", target.display()),
            Err(err) => {
                println!("resolved: <unresolved: {err}>");
                problems.push("symlink target cannot be resolved".to_string());
            }
        }
    } else {
        println!("symlink: no");
    }

    let meta = fs::metadata(path)
        .map_err(|e| token_err_source(format!("stat token file {} failed", path.display()), e))?;

    let kind = if meta.is_file() {
        "regular file"
    } else if meta.is_dir() {
        "directory"
    } else {
        "other"
    };
    println!("type: {kind}");
    if !meta.is_file() {
        problems.push(format!("not a regular file ({kind})"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let mode = meta.permissions().mode() & 0o777;
        println!("mode: {:04o}", mode);
        println!("owner: uid={} gid={}", meta.uid(), meta.gid());
        if mode & 0o022 != 0 {
            problems.push(format!("mode {mode:04o} is writable by group/other"));
        }
        if require_owner_only && mode & 0o077 != 0 {
            problems.push(format!("mode {mode:04o} is not owner-only"));
        }
    }

    match fs::read_to_string(path) {
        Ok(body) => match token::parse_single_line(&body) {
            Ok(value) => {
                println!("content: ok (1 token)");
                println!("token_fp: {}", token::fingerprint(value));
            }
            Err(err) => {
                println!("content: {err}");
                problems.push(err.to_string());
            }
        },
        Err(err) => {
            println!("content: <unreadable: {err}>");
            problems.push(format!("cannot read file: {err}"));
        }
    }

    if problems.is_empty() {
        println!("verdict: ok");
        Ok(())
    } else {
        println!("verdict: FAILED");
        for problem in &problems {
            println!("  - {problem}");
        }
        Err(token_err(format!(
            "token file {} failed checks: {}",
            path.display(),
            problems.join("; ")
        )))
    }
}

fn default_home_token_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(DEFAULT_HOME_TOKEN_REL))
}

fn token_err(detail: String) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail)
}

fn token_err_source<E>(detail: String, source: E) -> wp_error::RunError
where
    E: std::error::Error + Send + Sync + 'static,
{
    RunReason::from_conf()
        .to_err()
        .with_source(source)
        .with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generate_creates_private_single_line_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");

        atomic_write_private(&path, "generated-token").expect("write token");

        let body = fs::read_to_string(&path).expect("read token");
        assert_eq!(body, "generated-token\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");
        fs::write(&path, "old\n").expect("seed");

        atomic_write_private(&path, "new").expect("replace");

        assert_eq!(fs::read_to_string(&path).expect("read"), "new\n");
    }

    #[test]
    fn resolve_token_prefers_explicit_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");
        fs::write(&path, "from-file\n").expect("seed");
        let mut dict = EnvDict::new();
        dict.insert(
            token::TOKEN_ENV_VAR,
            orion_variate::ValueType::from("from-env"),
        );

        let resolved =
            resolve_token(Some(path.as_path()), None, &dict, None).expect("resolve token");

        assert_eq!(resolved.token, "from-file");
        assert!(matches!(resolved.source, TokenSource::File(_)));
    }

    #[test]
    fn resolve_token_falls_back_to_env() {
        let dict = EnvDict::new();
        // Deliberately unlikely to be set in the test environment.
        let resolved = resolve_token(None, Some("WPARSE_TEST_TOKEN_NOT_SET_4F2A"), &dict, None);
        assert!(resolved.is_err());
    }

    #[test]
    fn resolve_token_uses_local_profile_as_last_resort() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");
        fs::write(&path, "profile-token\n").expect("seed");
        let profile = AdminApiClientProfile {
            base_url: "http://127.0.0.1:19090".to_string(),
            token_file: path.clone(),
            request_timeout: std::time::Duration::from_secs(1),
        };

        let resolved =
            resolve_token(None, None, &EnvDict::new(), Some(&profile)).expect("resolve token");

        assert_eq!(resolved.token, "profile-token");
        assert_eq!(
            resolved.source.to_string(),
            format!("file:{}", path.display())
        );
    }

    #[test]
    fn check_reports_readable_secret_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");
        fs::write(&path, "checked-token\n").expect("seed");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).expect("stat").permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&path, perms).expect("chmod");
        }

        inspect_token_file(&path, false).expect("check should pass");
    }

    #[test]
    fn check_rejects_group_writable_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");
        fs::write(&path, "checked-token\n").expect("seed");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).expect("stat").permissions();
            perms.set_mode(0o666);
            fs::set_permissions(&path, perms).expect("chmod");
            assert!(inspect_token_file(&path, false).is_err());
        }
    }

    #[test]
    fn check_require_owner_only_rejects_group_readable_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("admin.token");
        fs::write(&path, "checked-token\n").expect("seed");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).expect("stat").permissions();
            perms.set_mode(0o640);
            fs::set_permissions(&path, perms).expect("chmod");
            assert!(inspect_token_file(&path, false).is_ok());
            assert!(inspect_token_file(&path, true).is_err());
        }
    }

    #[test]
    fn check_reports_missing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("missing.token");
        assert!(inspect_token_file(&path, false).is_err());
    }
}
