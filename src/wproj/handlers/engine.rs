use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::args::{EngineReloadArgs, EngineStatusArgs, EngineTargetArgs};
use crate::format::print_json;
use orion_error::{ToStructError, UvsFrom};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use warp_parse::admin_api;
use wp_error::run_error::{RunReason, RunResult};

#[derive(Debug, Serialize, Deserialize)]
struct EngineStatusResponse {
    instance_id: String,
    version: String,
    project_version: Option<String>,
    accepting_commands: bool,
    reloading: bool,
    current_request_id: Option<String>,
    last_reload_request_id: Option<String>,
    last_reload_result: Option<String>,
    last_reload_started_at: Option<String>,
    last_reload_finished_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EngineReloadResponse {
    request_id: String,
    accepted: bool,
    result: String,
    update: Option<bool>,
    requested_version: Option<String>,
    current_version: Option<String>,
    resolved_tag: Option<String>,
    force_replaced: Option<bool>,
    warning: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EngineErrorResponse {
    request_id: String,
    accepted: bool,
    result: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct EngineReloadRequest<'a> {
    wait: bool,
    timeout_ms: u64,
    update: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
}

pub async fn run_engine_status(args: EngineStatusArgs) -> RunResult<()> {
    let profile = resolve_target(&args.target)?;
    let client = build_client(&profile, args.target.insecure)?;
    let url = format!(
        "{}/admin/v1/runtime/status",
        profile.base_url.trim_end_matches('/')
    );

    let response = client
        .get(&url)
        .headers(auth_headers(&profile.token))
        .send()
        .await
        .map_err(|e| conf_err(format!("request {} failed: {}", url, e)))?;

    if response.status().is_success() {
        let status: EngineStatusResponse = response
            .json()
            .await
            .map_err(|e| conf_err(format!("decode status response failed: {}", e)))?;
        if args.json {
            return print_json(&status);
        }

        println!("Engine status");
        println!("  Endpoint   : {}", profile.base_url);
        println!("  Instance   : {}", status.instance_id);
        println!("  Version    : {}", status.version);
        println!(
            "  Project V  : {}",
            status.project_version.as_deref().unwrap_or("-")
        );
        println!("  Accepting  : {}", status.accepting_commands);
        println!("  Reloading  : {}", status.reloading);
        println!(
            "  Current    : {}",
            status.current_request_id.as_deref().unwrap_or("-")
        );
        println!(
            "  Last ID    : {}",
            status.last_reload_request_id.as_deref().unwrap_or("-")
        );
        println!(
            "  Last Result: {}",
            status.last_reload_result.as_deref().unwrap_or("-")
        );
        println!(
            "  Started At : {}",
            status.last_reload_started_at.as_deref().unwrap_or("-")
        );
        println!(
            "  Finished At: {}",
            status.last_reload_finished_at.as_deref().unwrap_or("-")
        );
        return Ok(());
    }

    let err = decode_error_response(response).await?;
    Err(conf_err(format!(
        "status request rejected: {} ({})",
        err.error, err.result
    )))
}

pub async fn run_engine_reload(args: EngineReloadArgs) -> RunResult<()> {
    if !args.update && args.version.is_some() {
        return Err(conf_err("--version requires --update"));
    }

    let profile = resolve_target(&args.target)?;
    let client = build_client(&profile, args.target.insecure)?;
    let url = format!(
        "{}/admin/v1/reloads/model",
        profile.base_url.trim_end_matches('/')
    );
    let request_id = args
        .request_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let response = client
        .post(&url)
        .headers(auth_headers(&profile.token))
        .header("X-Request-Id", &request_id)
        .json(&EngineReloadRequest {
            wait: args.wait,
            timeout_ms: args.timeout_ms,
            update: args.update,
            version: args.version.as_deref(),
            reason: args.reason.as_deref(),
        })
        .send()
        .await
        .map_err(|e| conf_err(format!("request {} failed: {}", url, e)))?;

    match response.status() {
        status
            if status.is_success()
                || status == StatusCode::ACCEPTED
                || status == StatusCode::CONFLICT =>
        {
            let body: EngineReloadResponse = response
                .json()
                .await
                .map_err(|e| conf_err(format!("decode reload response failed: {}", e)))?;
            if args.json {
                return print_json(&body);
            }

            println!("Engine reload");
            println!("  Endpoint : {}", profile.base_url);
            println!("  Request  : {}", body.request_id);
            println!("  Accepted : {}", body.accepted);
            println!("  Result   : {}", body.result);
            if let Some(update) = body.update {
                println!("  Updated  : {}", update);
            }
            if let Some(version) = body.requested_version.as_deref() {
                println!("  Request V: {}", version);
            }
            if let Some(version) = body.current_version.as_deref() {
                println!("  Current V: {}", version);
            }
            if let Some(tag) = body.resolved_tag.as_deref() {
                println!("  Tag      : {}", tag);
            }
            if let Some(force_replaced) = body.force_replaced {
                println!("  Forced   : {}", force_replaced);
            }
            if let Some(warning) = body.warning.as_deref() {
                println!("  Warning  : {}", warning);
            }
            if let Some(error) = body.error.as_deref() {
                println!("  Error    : {}", error);
            }

            if status == StatusCode::CONFLICT {
                return Err(conf_err("reload already in progress"));
            }
            if status == StatusCode::ACCEPTED {
                return Ok(());
            }
            if body.result == "reload_failed" {
                return Err(conf_err(body.error.unwrap_or_else(|| {
                    "reload failed without error detail".to_string()
                })));
            }
            Ok(())
        }
        _ => {
            let err = decode_error_response(response).await?;
            Err(conf_err(format!(
                "reload request rejected: {} ({})",
                err.error, err.result
            )))
        }
    }
}

struct ResolvedTarget {
    base_url: String,
    token: String,
    request_timeout: Duration,
}

fn resolve_target(args: &EngineTargetArgs) -> RunResult<ResolvedTarget> {
    let work_root = resolve_work_root(&args.work_root)?;
    let need_local_profile = args.admin_url.is_none() || args.token_file.is_none();
    let local_profile = if need_local_profile {
        admin_api::resolve_client_profile(&work_root)?
    } else {
        None
    };
    let base_url = match (&args.admin_url, &local_profile) {
        (Some(url), _) => url.trim_end_matches('/').to_string(),
        (None, Some(profile)) => profile.base_url.trim_end_matches('/').to_string(),
        (None, None) => {
            return Err(conf_err(format!(
                "admin API is not enabled in {} and --admin-url was not provided",
                work_root.join("conf/wparse.toml").display()
            )));
        }
    };

    let token_path = match (&args.token_file, &local_profile) {
        (Some(path), _) => resolve_override_path(&work_root, path),
        (None, Some(profile)) => profile.token_file.clone(),
        (None, None) => {
            return Err(conf_err(
                "token file is not configured locally and --token-file was not provided",
            ));
        }
    };
    let token = std::fs::read_to_string(&token_path)
        .map_err(|e| {
            conf_err(format!(
                "read token file {} failed: {}",
                token_path.display(),
                e
            ))
        })?
        .trim()
        .to_string();
    if token.is_empty() {
        return Err(conf_err(format!(
            "token file {} is empty",
            token_path.display()
        )));
    }

    let request_timeout = local_profile
        .as_ref()
        .map(|profile| profile.request_timeout)
        .unwrap_or_else(|| Duration::from_millis(15_000));

    Ok(ResolvedTarget {
        base_url,
        token,
        request_timeout,
    })
}

fn build_client(target: &ResolvedTarget, insecure: bool) -> RunResult<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(target.request_timeout)
        .danger_accept_invalid_certs(insecure)
        .build()
        .map_err(|e| conf_err(format!("build HTTP client failed: {}", e)))
}

fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {}", token))
        .expect("token should be valid header value");
    headers.insert(AUTHORIZATION, value);
    headers
}

async fn decode_error_response(response: reqwest::Response) -> RunResult<EngineErrorResponse> {
    let status = response.status();
    response.json::<EngineErrorResponse>().await.map_err(|e| {
        conf_err(format!(
            "decode error response failed (HTTP {}): {}",
            status, e
        ))
    })
}

fn resolve_work_root(raw: &str) -> RunResult<PathBuf> {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return Ok(path);
    }
    env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|e| conf_err(format!("resolve current dir failed: {}", e)))
}

fn resolve_override_path(work_root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        work_root.join(path)
    }
}

fn conf_err(detail: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::OnceLock;
    use tempfile::tempdir;
    use wp_engine::facade::args::ParseArgs;
    use wp_engine::facade::WpApp;

    fn shared_control_handle() -> wp_engine::facade::RuntimeControlHandle {
        static HANDLE: OnceLock<wp_engine::facade::RuntimeControlHandle> = OnceLock::new();
        HANDLE
            .get_or_init(|| {
                let work_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
                let args = ParseArgs {
                    work_root: Some(work_root.to_string_lossy().to_string()),
                    ..Default::default()
                };
                WpApp::try_from(args, orion_variate::EnvDict::default())
                    .expect("build wp app")
                    .control_handle()
            })
            .clone()
    }

    fn write_token(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create token dir");
        }
        fs::write(path, "test-token\n").expect("write token");
        let mut perms = fs::metadata(path).expect("stat token").permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms).expect("chmod token");
    }

    fn write_conf(work_root: &Path, bind: &str, token_file: &str) {
        let conf_dir = work_root.join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            format!(
                r#"
version = "1.0"

[admin_api]
enabled = true
bind = "{bind}"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.tls]
enabled = false
cert_file = ""
key_file = ""

[admin_api.auth]
mode = "bearer_token"
token_file = "{token_file}"
"#
            ),
        )
        .expect("write conf");
    }

    #[tokio::test]
    #[serial]
    async fn status_uses_local_profile_without_sec_key() {
        let temp = tempdir().expect("tempdir");
        let token_path = temp.path().join("runtime/admin_api.token");
        write_token(&token_path);
        write_conf(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");

        let runtime = warp_parse::admin_api::start_if_enabled(temp.path(), shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        write_conf(
            temp.path(),
            &runtime.local_addr().to_string(),
            "runtime/admin_api.token",
        );

        let result = run_engine_status(EngineStatusArgs {
            target: EngineTargetArgs {
                work_root: temp.path().to_string_lossy().to_string(),
                admin_url: None,
                token_file: None,
                insecure: false,
            },
            json: true,
        })
        .await;

        runtime.shutdown().await;
        assert!(result.is_ok(), "status should work from local profile");
    }

    #[tokio::test]
    async fn reload_rejects_version_without_update() {
        let err = run_engine_reload(EngineReloadArgs {
            target: EngineTargetArgs {
                work_root: ".".to_string(),
                admin_url: None,
                token_file: None,
                insecure: false,
            },
            wait: true,
            timeout_ms: 15_000,
            reason: None,
            update: false,
            version: Some("1.4.3".to_string()),
            request_id: None,
            json: false,
        })
        .await
        .expect_err("version without update should be rejected");

        assert!(
            err.to_string().contains("--version requires --update"),
            "unexpected error: {}",
            err
        );
    }
}
