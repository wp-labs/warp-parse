use std::convert::Infallible;
use std::fs;
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::compat::UvsFrom;
use crate::token::{self, TokenAuth};
use crate::LOCAL_CONF_FILE as LOCAL_ADMIN_API_CONF;
use chrono::{DateTime, Utc};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use hyper::http::StatusCode;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use orion_error::conversion::{SourceErr, ToStructError};
use orion_variate::{EnvDict, EnvEvaluable};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;
use wp_config::engine::AdminApiConf;
use wp_engine::facade::{
    RuntimeCommandResp, RuntimeCommandResult, RuntimeCommandSendError, RuntimeControlHandle,
};
use wp_error::run_error::{RunReason, RunResult};
use wp_log::{info_ctrl, warn_ctrl};

#[derive(Debug)]
pub struct AdminApiRuntime {
    local_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl AdminApiRuntime {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }
}

pub async fn start_if_enabled(
    work_root: &Path,
    dict: &EnvDict,
    control_handle: RuntimeControlHandle,
) -> RunResult<Option<AdminApiRuntime>> {
    let config = load_config(work_root, dict)?;
    let Some(config) = config else {
        return Ok(None);
    };

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|e| conf_err_source(format!("bind admin api on {} failed", config.bind), e))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| conf_err_source("read admin api local addr failed", e))?;
    let instance_id = format!("{}:{}", hostname_for_instance(), std::process::id());
    info_ctrl!(
        "admin api auth source=file path={} token_fp={}",
        config.token_path.display(),
        config.token.fingerprint()
    );
    let state = Arc::new(AppState {
        control_handle,
        work_root: work_root.to_path_buf(),
        dict: dict.clone(),
        reload_gate: Mutex::new(()),
        token: config.token,
        auth_log: std::sync::Mutex::new(AuthFailureLog::default()),
        request_timeout: config.request_timeout,
        max_body_bytes: config.max_body_bytes,
        instance_id,
        version: crate::build::PKG_VERSION.to_string(),
    });

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let task = match config.tls {
        Some(server_config) => {
            info_ctrl!(
                "admin api listening on https://{} (request_timeout_ms={}, max_body_bytes={})",
                local_addr,
                config.request_timeout.as_millis(),
                config.max_body_bytes
            );
            tokio::spawn(run_tls(
                listener,
                TlsAcceptor::from(Arc::new(server_config)),
                state,
                shutdown_rx,
            ))
        }
        None => {
            info_ctrl!(
                "admin api listening on http://{} (request_timeout_ms={}, max_body_bytes={})",
                local_addr,
                config.request_timeout.as_millis(),
                config.max_body_bytes
            );
            tokio::spawn(run_plain(listener, state, shutdown_rx))
        }
    };

    Ok(Some(AdminApiRuntime {
        local_addr,
        shutdown_tx: Some(shutdown_tx),
        task,
    }))
}

struct ResolvedAdminApiConfig {
    bind: SocketAddr,
    request_timeout: Duration,
    max_body_bytes: usize,
    token: TokenAuth,
    token_path: PathBuf,
    tls: Option<ServerConfig>,
}

#[derive(Debug, Clone)]
pub struct AdminApiClientProfile {
    pub base_url: String,
    pub token_file: PathBuf,
    pub request_timeout: Duration,
}

fn load_config(work_root: &Path, dict: &EnvDict) -> RunResult<Option<ResolvedAdminApiConfig>> {
    let parsed = load_engine_config(work_root, dict)?;
    let admin_api = effective_admin_api(work_root, dict, &parsed)?;
    if !admin_api.enabled {
        return Ok(None);
    }

    let bind: SocketAddr = admin_api
        .bind
        .parse()
        .map_err(|e| conf_err_source(format!("invalid admin_api.bind '{}'", admin_api.bind), e))?;
    if admin_api.max_body_bytes == 0 {
        return Err(admin_api_validation_err(
            "admin_api.max_body_bytes must be > 0",
        ));
    }

    if admin_api.auth.token_file.trim().is_empty() {
        return Err(admin_api_validation_err(
            "admin_api.auth.token_file must be set when admin_api is enabled",
        ));
    }
    let token_path = PathBuf::from(&admin_api.auth.token_file);
    let token = load_token_file(&token_path)?;

    let tls = if admin_api.tls.enabled {
        Some(load_tls_config(
            Path::new(&admin_api.tls.cert_file),
            Path::new(&admin_api.tls.key_file),
        )?)
    } else {
        None
    };

    if !bind.ip().is_loopback() && tls.is_none() {
        warn_ctrl!(
            "非回环地址 admin_api.bind='{}' 未启用 TLS，建议设置 admin_api.tls.enabled=true 保障安全",
            bind
        );
    }

    Ok(Some(ResolvedAdminApiConfig {
        bind,
        request_timeout: Duration::from_millis(admin_api.request_timeout_ms),
        max_body_bytes: admin_api.max_body_bytes,
        token,
        token_path,
        tls,
    }))
}

pub fn resolve_client_profile(
    work_root: &Path,
    dict: &EnvDict,
) -> RunResult<Option<AdminApiClientProfile>> {
    let parsed = load_engine_config(work_root, dict)?;
    let admin_api = effective_admin_api(work_root, dict, &parsed)?;
    if !admin_api.enabled {
        return Ok(None);
    }

    let bind: SocketAddr = admin_api
        .bind
        .parse()
        .map_err(|e| conf_err_source(format!("invalid admin_api.bind '{}'", admin_api.bind), e))?;
    let token_file = admin_api.auth.token_file.trim();
    if token_file.is_empty() {
        return Err(admin_api_validation_err(
            "admin_api.auth.token_file must be set when admin_api is enabled",
        ));
    }

    let scheme = if admin_api.tls.enabled {
        "https"
    } else {
        "http"
    };
    Ok(Some(AdminApiClientProfile {
        base_url: format!("{}://{}", scheme, bind),
        token_file: PathBuf::from(token_file),
        request_timeout: Duration::from_millis(admin_api.request_timeout_ms),
    }))
}

fn load_engine_config(
    work_root: &Path,
    dict: &EnvDict,
) -> RunResult<wp_config::engine::EngineConfig> {
    wp_config::engine::EngineConfig::load(work_root, dict)
        .source_err(RunReason::from_conf(), "load engine config failed")
        .map(|conf| conf.env_eval(dict).conf_absolutize(work_root))
}

/// Resolve the effective admin API config.
///
/// `wparse.local.toml` is a site-local overlay outside `conf/`; when present it
/// fully replaces `[admin_api]` from `conf/wparse.toml`. This keeps the remote
/// control port out of the directories that `project_remote` rewrites.
fn effective_admin_api(
    work_root: &Path,
    dict: &EnvDict,
    parsed: &wp_config::engine::EngineConfig,
) -> RunResult<AdminApiConf> {
    let local_path = work_root.join(LOCAL_ADMIN_API_CONF);
    let overlay = crate::local_conf::load_site_overlay(work_root, dict)?;
    let overlay_present = overlay.is_some();
    if let Some(mut local) = overlay.and_then(|overlay| overlay.admin_api) {
        absolutize_admin_api_paths(&mut local, work_root);
        info_ctrl!("admin api config source={}", local_path.display());
        return Ok(local);
    }

    let engine_path = work_root.join("conf/wparse.toml");
    info_ctrl!("admin api config source={}", engine_path.display());
    let admin_api = parsed.admin_api();
    if admin_api.enabled {
        if overlay_present {
            warn_ctrl!(
                "[admin_api] in {} is used because {} has no [admin_api] section",
                engine_path.display(),
                local_path.display()
            );
        } else {
            warn_ctrl!(
                "[admin_api] in {} is used because {} is missing; create the local overlay so project_remote sync cannot overwrite it",
                engine_path.display(),
                local_path.display()
            );
        }
    }
    Ok(admin_api.clone())
}

fn absolutize_admin_api_paths(conf: &mut AdminApiConf, work_root: &Path) {
    conf.tls.cert_file = resolve_against(work_root, &conf.tls.cert_file);
    conf.tls.key_file = resolve_against(work_root, &conf.tls.key_file);
    conf.auth.token_file = resolve_against(work_root, &conf.auth.token_file);
}

fn resolve_against(work_root: &Path, value: &str) -> String {
    let path = Path::new(value);
    if value.is_empty() || path.is_absolute() {
        return value.to_string();
    }
    let mut resolved = PathBuf::new();
    for component in work_root.join(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other.as_os_str()),
        }
    }
    resolved.to_string_lossy().to_string()
}

/// Read and validate the bearer token file.
///
/// Security floor (no configuration knob):
///
/// * the path must resolve to a regular file (symlinks are followed so
///   Kubernetes `..data` secret links work);
/// * group/other must not be able to write it (`mode & 0o022 == 0`), which
///   accepts `0600/0640/0440/0444` and rejects `0660/0664/0777`;
/// * the file must contain exactly one non-empty line.
///
/// Validation runs on the opened file descriptor (`fstat`) and content is read
/// from that same descriptor, so the file cannot be swapped between the check
/// and the read.
fn load_token_file(path: &Path) -> RunResult<TokenAuth> {
    let mut file = fs::File::open(path)
        .map_err(|e| conf_err_source(format!("open token file {} failed", path.display()), e))?;
    let meta = file
        .metadata()
        .map_err(|e| conf_err_source(format!("stat token file {} failed", path.display()), e))?;
    validate_token_metadata(path, &meta)?;

    let mut body = String::new();
    file.read_to_string(&mut body)
        .map_err(|e| conf_err_source(format!("read token file {} failed", path.display()), e))?;
    let token = token::parse_single_line(&body).map_err(|err| {
        token_file_validation_err(format!("token file {}: {err}", path.display()))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            warn_ctrl!(
                "token file {} mode {:o} is readable by group/other; prefer 0600 for host files",
                path.display(),
                mode
            );
        }
    }

    Ok(TokenAuth::from_token(token))
}

fn validate_token_metadata(path: &Path, meta: &fs::Metadata) -> RunResult<()> {
    if !meta.is_file() {
        return Err(token_file_validation_err(format!(
            "token file {} is not a regular file",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o022 != 0 {
            return Err(token_file_validation_err(format!(
                "token file {} permissions {:o} are too permissive; group/other must not be able to write it (chmod 600 or 640)",
                path.display(),
                mode
            )));
        }
    }
    Ok(())
}

fn load_tls_config(cert_path: &Path, key_path: &Path) -> RunResult<ServerConfig> {
    if cert_path.as_os_str().is_empty() || key_path.as_os_str().is_empty() {
        return Err(tls_validation_err(
            "admin_api.tls.cert_file and admin_api.tls.key_file must be set when TLS is enabled",
        ));
    }
    let cert_pem = fs::read(cert_path).map_err(|e| {
        conf_err_source(format!("read cert file {} failed", cert_path.display()), e)
    })?;
    let key_pem = fs::read(key_path)
        .map_err(|e| conf_err_source(format!("read key file {} failed", key_path.display()), e))?;

    let certs = CertificateDer::pem_slice_iter(&cert_pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            conf_err_source(
                format!("parse PEM certs from {} failed", cert_path.display()),
                e,
            )
        })?;
    if certs.is_empty() {
        return Err(tls_validation_err(format!(
            "no certificates found in {}",
            cert_path.display()
        )));
    }
    let key = PrivateKeyDer::from_pem_slice(&key_pem).map_err(|e| {
        conf_err_source(
            format!("parse PEM key from {} failed", key_path.display()),
            e,
        )
    })?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| conf_err_source("build TLS server config failed", e))?;
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(server_config)
}

async fn run_plain(
    listener: TcpListener,
    state: Arc<AppState>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    run_accept_loop(listener, state, shutdown_rx, None).await;
}

async fn run_tls(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    state: Arc<AppState>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    run_accept_loop(listener, state, shutdown_rx, Some(acceptor)).await;
}

async fn run_accept_loop(
    listener: TcpListener,
    state: Arc<AppState>,
    mut shutdown_rx: oneshot::Receiver<()>,
    tls_acceptor: Option<TlsAcceptor>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                info_ctrl!("admin api shutdown requested");
                break;
            }
            accept_res = listener.accept() => {
                let (stream, remote_addr) = match accept_res {
                    Ok(pair) => pair,
                    Err(err) => {
                        warn_ctrl!("admin api accept failed: {}", err);
                        continue;
                    }
                };
                let state = state.clone();
                let tls_acceptor = tls_acceptor.clone();
                tokio::spawn(async move {
                    if let Some(acceptor) = tls_acceptor {
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => serve_connection(tls_stream, remote_addr, state).await,
                            Err(err) => warn_ctrl!("admin api TLS handshake failed from {}: {}", remote_addr, err),
                        }
                    } else {
                        serve_connection(stream, remote_addr, state).await;
                    }
                });
            }
        }
    }
}

async fn serve_connection<IO>(stream: IO, remote_addr: SocketAddr, state: Arc<AppState>)
where
    IO: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let io = TokioIo::new(stream);
    let svc = service_fn(move |req| handle_request(req, remote_addr, state.clone()));
    let builder = AutoBuilder::new(TokioExecutor::new());
    if let Err(err) = builder.serve_connection_with_upgrades(io, svc).await {
        warn_ctrl!("admin api connection error from {}: {}", remote_addr, err);
    }
}

struct AppState {
    control_handle: RuntimeControlHandle,
    work_root: PathBuf,
    dict: EnvDict,
    reload_gate: Mutex<()>,
    token: TokenAuth,
    auth_log: std::sync::Mutex<AuthFailureLog>,
    request_timeout: Duration,
    max_body_bytes: usize,
    instance_id: String,
    version: String,
}

/// Rate limit for unauthenticated-request logging.
///
/// Every 401 used to emit one log line, which lets an unauthenticated scanner
/// flood the control-plane log. At most one line per window is kept, with the
/// number of suppressed lines appended.
#[derive(Default)]
struct AuthFailureLog {
    last: Option<std::time::Instant>,
    suppressed: u64,
}

impl AuthFailureLog {
    const WINDOW: Duration = Duration::from_secs(10);

    /// Return the suffix for the next log line, or `None` if it must be suppressed.
    fn record(&mut self, now: std::time::Instant) -> Option<String> {
        let due = self
            .last
            .map(|last| now.duration_since(last) >= Self::WINDOW)
            .unwrap_or(true);
        if !due {
            self.suppressed += 1;
            return None;
        }
        let suffix = if self.suppressed > 0 {
            format!(" suppressed={}", self.suppressed)
        } else {
            String::new()
        };
        self.suppressed = 0;
        self.last = Some(now);
        Some(suffix)
    }
}

struct ProjectRemoteReloadContext {
    _lock_guard: crate::project_remote::ProjectRemoteLockGuard,
    snapshot: Option<crate::project_remote::ProjectRemoteSnapshot>,
    runtime_snapshot: Option<crate::project_remote::ProjectRuntimeArtifactSnapshot>,
    update_result: Option<crate::project_remote::ProjectRemoteUpdateResult>,
    #[allow(dead_code)]
    group: Option<crate::project_remote::RemoteGroup>,
}

#[derive(Debug, Deserialize, Default)]
struct ReloadRequest {
    #[serde(default = "default_wait")]
    wait: bool,
    #[serde(default)]
    update: bool,
    version: Option<String>,
    #[serde(default)]
    group: Option<String>,
    timeout_ms: Option<u64>,
    reason: Option<String>,
}

#[derive(Serialize)]
struct ReloadResponse {
    request_id: String,
    accepted: bool,
    result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_replaced: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct RuntimeStatusResponse {
    instance_id: String,
    version: String,
    /// Public fingerprint of the loaded bearer token; never the value itself.
    token_fp: String,
    project_version: Option<serde_json::Value>,
    accepting_commands: bool,
    reloading: bool,
    current_request_id: Option<String>,
    last_reload_request_id: Option<String>,
    last_reload_result: Option<&'static str>,
    last_reload_started_at: Option<String>,
    last_reload_finished_at: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    request_id: String,
    accepted: bool,
    result: &'static str,
    error: String,
}

async fn handle_request(
    req: Request<Incoming>,
    remote_addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let request_id = request_id(req.headers());
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    if !authorized(req.headers(), &state.token) {
        let presented_fp = bearer_token(req.headers())
            .map(token::fingerprint)
            .unwrap_or_else(|| "-".to_string());
        let log_line = state
            .auth_log
            .lock()
            .ok()
            .and_then(|mut log| log.record(std::time::Instant::now()));
        if let Some(suffix) = log_line {
            warn_ctrl!(
                "admin api unauthorized request_id={} remote={} method={} path={} token_fp={}{}",
                request_id,
                remote_addr,
                method,
                path,
                presented_fp,
                suffix
            );
        }
        return Ok(json_response(
            StatusCode::UNAUTHORIZED,
            &ErrorResponse {
                request_id,
                accepted: false,
                result: "unauthorized",
                error: "invalid bearer token".to_string(),
            },
        ));
    }

    let response = match (method, path.as_str()) {
        (Method::GET, "/admin/v1/runtime/status") => {
            status_response(&request_id, remote_addr, &state)
        }
        (Method::POST, "/admin/v1/reloads/model") => {
            reload_response(req, &request_id, remote_addr, state).await
        }
        _ => json_response(
            StatusCode::NOT_FOUND,
            &ErrorResponse {
                request_id,
                accepted: false,
                result: "not_found",
                error: format!("unsupported route {}", path),
            },
        ),
    };

    Ok(response)
}

fn status_response(
    request_id: &str,
    remote_addr: SocketAddr,
    state: &AppState,
) -> Response<Full<Bytes>> {
    let snapshot = state.control_handle.status_snapshot();
    let project_version = match read_project_version(&state.work_root) {
        Ok(version) => version,
        Err(err) => {
            warn_ctrl!(
                "admin api status project version read failed request_id={} remote={} error={}",
                request_id,
                remote_addr,
                err
            );
            None
        }
    };
    info_ctrl!(
        "admin api status request_id={} remote={} accepting={} reloading={}",
        request_id,
        remote_addr,
        snapshot.accepting_commands,
        snapshot.reloading
    );
    json_response(
        StatusCode::OK,
        &RuntimeStatusResponse {
            instance_id: state.instance_id.clone(),
            version: state.version.clone(),
            token_fp: state.token.fingerprint().to_string(),
            project_version,
            accepting_commands: snapshot.accepting_commands,
            reloading: snapshot.reloading,
            current_request_id: snapshot.current_request_id,
            last_reload_request_id: snapshot.last_reload_request_id,
            last_reload_result: snapshot.last_reload_result.as_ref().map(result_code),
            last_reload_started_at: snapshot.last_reload_started_at.map(system_time_to_rfc3339),
            last_reload_finished_at: snapshot.last_reload_finished_at.map(system_time_to_rfc3339),
        },
    )
}

async fn reload_response(
    req: Request<Incoming>,
    request_id: &str,
    remote_addr: SocketAddr,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    let _reload_guard = match state.reload_gate.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return json_response(
                StatusCode::CONFLICT,
                &ReloadResponse {
                    request_id: request_id.to_string(),
                    accepted: false,
                    result: "reload_in_progress",
                    update: None,
                    requested_version: None,
                    current_version: None,
                    resolved_tag: None,
                    group: None,
                    force_replaced: None,
                    warning: None,
                    error: None,
                },
            )
        }
    };
    let reload_req =
        match read_json_body::<ReloadRequest>(req.into_body(), state.max_body_bytes).await {
            Ok(payload) => payload,
            Err(ReadBodyError::TooLarge(limit)) => {
                return json_response(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "payload_too_large",
                        error: format!("request body exceeds {} bytes", limit),
                    },
                );
            }
            Err(ReadBodyError::InvalidJson(err)) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "invalid_request",
                        error: err,
                    },
                );
            }
            Err(ReadBodyError::Read(err)) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "invalid_request",
                        error: err,
                    },
                );
            }
        };

    let reason = reload_req.reason.as_deref().unwrap_or("");
    if !reload_req.update && reload_req.version.is_some() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                request_id: request_id.to_string(),
                accepted: false,
                result: "invalid_request",
                error: "version requires update=true".to_string(),
            },
        );
    }
    if !reload_req.update && reload_req.group.as_deref().is_some_and(|g| !g.is_empty()) {
        return json_response(
            StatusCode::BAD_REQUEST,
            &ErrorResponse {
                request_id: request_id.to_string(),
                accepted: false,
                result: "invalid_request",
                error: "group requires update=true".to_string(),
            },
        );
    }

    // In dual-repo mode, update requires --group
    if reload_req.update && reload_req.group.as_deref().map_or(true, |g| g.is_empty()) {
        if let Ok(config) = load_engine_config(&state.work_root, &state.dict) {
            let remote_conf = config.project_remote();
            if remote_conf.enabled
                && matches!(
                    crate::project_remote::resolve_project_remote_mode(remote_conf),
                    Ok(crate::project_remote::ProjectRemoteMode::Dual { .. })
                )
            {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "invalid_request",
                        error: "dual-repo mode requires group (models|infra) with update=true"
                            .to_string(),
                    },
                );
            }
        }
    }

    let runtime_status = state.control_handle.status_snapshot();
    if !runtime_status.accepting_commands {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &ErrorResponse {
                request_id: request_id.to_string(),
                accepted: false,
                result: "runtime_not_ready",
                error: "runtime command receiver not ready".to_string(),
            },
        );
    }
    if runtime_status.reloading {
        return json_response(
            StatusCode::CONFLICT,
            &ReloadResponse {
                request_id: request_id.to_string(),
                accepted: false,
                result: "reload_in_progress",
                update: None,
                requested_version: None,
                current_version: None,
                resolved_tag: None,
                group: None,
                force_replaced: None,
                warning: None,
                error: None,
            },
        );
    }

    let reload_lock = match crate::project_remote::acquire_project_remote_lock(&state.work_root) {
        Ok(lock) => lock,
        Err(err) => {
            return json_response(
                StatusCode::CONFLICT,
                &ReloadResponse {
                    request_id: request_id.to_string(),
                    accepted: false,
                    result: "update_in_progress",
                    update: Some(reload_req.update),
                    requested_version: reload_req.version.clone(),
                    current_version: None,
                    resolved_tag: None,
                    group: None,
                    force_replaced: None,
                    warning: None,
                    error: Some(err.to_string()),
                },
            );
        }
    };

    let update_group = match reload_req.group.as_deref() {
        None | Some("") => None,
        Some(raw) => match raw.parse::<crate::project_remote::RemoteGroup>() {
            Ok(group) => Some(group),
            Err(err) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "invalid_request",
                        error: err,
                    },
                );
            }
        },
    };

    let rollback_snapshot = if reload_req.update {
        match crate::project_remote::capture_project_remote_snapshot_with_group(
            &state.work_root,
            update_group,
        ) {
            Ok(snapshot) => Some(snapshot),
            Err(err) => {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "update_failed",
                        error: err.to_string(),
                    },
                );
            }
        }
    } else {
        None
    };
    let runtime_snapshot = if reload_req.update {
        match crate::project_remote::capture_runtime_artifact_snapshot(&state.work_root) {
            Ok(snapshot) => Some(snapshot),
            Err(err) => {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "update_failed",
                        error: err.to_string(),
                    },
                );
            }
        }
    } else {
        None
    };

    let update_result = if reload_req.update {
        info_ctrl!(
            "admin api project update start request_id={} remote={} requested_version={} group={}",
            request_id,
            remote_addr,
            reload_req.version.as_deref().unwrap_or("(auto)"),
            reload_req.group.as_deref().unwrap_or("-")
        );
        let sync_result = match update_group {
            Some(group) => crate::project_remote::sync_project_remote_group_with_dict(
                &state.work_root,
                group,
                reload_req.version.as_deref(),
                &state.dict,
            ),
            None => crate::project_remote::sync_project_remote_with_dict(
                &state.work_root,
                reload_req.version.as_deref(),
                &state.dict,
            ),
        };
        match sync_result {
            Ok(result) => {
                info_ctrl!(
                    "admin api project update done request_id={} remote={} requested_version={} current_version={} resolved_tag={} from_revision={} to_revision={} changed={}",
                    request_id,
                    remote_addr,
                    reload_req.version.as_deref().unwrap_or("(auto)"),
                    result.current_version,
                    result.resolved_tag,
                    result.from_revision.as_deref().unwrap_or("-"),
                    result.to_revision,
                    result.changed
                );
                Some(result)
            }
            Err(err) => {
                warn_ctrl!(
                    "admin api project update failed request_id={} remote={} requested_version={} error={}",
                    request_id,
                    remote_addr,
                    reload_req.version.as_deref().unwrap_or("(auto)"),
                    err
                );
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ErrorResponse {
                        request_id: request_id.to_string(),
                        accepted: false,
                        result: "update_failed",
                        error: err.to_string(),
                    },
                );
            }
        }
    } else {
        None
    };
    let mut reload_ctx = Some(ProjectRemoteReloadContext {
        _lock_guard: reload_lock,
        snapshot: rollback_snapshot,
        runtime_snapshot,
        update_result: update_result.clone(),
        group: update_group,
    });

    match state
        .control_handle
        .request_load_model(request_id.to_string())
        .await
    {
        Ok(reply_rx) => {
            info_ctrl!(
                "admin api reload accepted request_id={} remote={} wait={} reason={}",
                request_id,
                remote_addr,
                reload_req.wait,
                reason
            );
            if !reload_req.wait {
                if let Some(ctx) = reload_ctx.take() {
                    tokio::spawn(monitor_reload_result(
                        reply_rx,
                        state.work_root.clone(),
                        ctx,
                        remote_addr,
                        reason.to_string(),
                    ));
                }
                return json_response(
                    StatusCode::ACCEPTED,
                    &ReloadResponse {
                        request_id: request_id.to_string(),
                        accepted: true,
                        result: "running",
                        update: Some(reload_req.update),
                        requested_version: update_result
                            .as_ref()
                            .and_then(|result| result.requested_version.clone()),
                        current_version: update_result
                            .as_ref()
                            .map(|result| result.current_version.clone()),
                        resolved_tag: update_result
                            .as_ref()
                            .map(|result| result.resolved_tag.clone()),
                        group: update_result.as_ref().and_then(|r| r.group.clone()),
                        force_replaced: None,
                        warning: None,
                        error: None,
                    },
                );
            }

            let wait_timeout = Duration::from_millis(
                reload_req
                    .timeout_ms
                    .unwrap_or(state.request_timeout.as_millis() as u64),
            );
            let mut reply_rx = reply_rx;
            match timeout(wait_timeout, &mut reply_rx).await {
                Ok(Ok(resp)) => {
                    let rollback_warning =
                        if matches!(resp.result, RuntimeCommandResult::ReloadFailed { .. }) {
                            rollback_updated_project(
                                &state.work_root,
                                reload_ctx.as_ref(),
                                request_id,
                                remote_addr,
                                "reload_failed",
                            )
                        } else {
                            None
                        };
                    map_runtime_response(
                        resp,
                        remote_addr,
                        reason,
                        update_result.as_ref(),
                        rollback_warning,
                    )
                }
                Ok(Err(_)) => json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ReloadResponse {
                        request_id: request_id.to_string(),
                        accepted: true,
                        result: "reload_failed",
                        update: Some(reload_req.update),
                        requested_version: update_result
                            .as_ref()
                            .and_then(|result| result.requested_version.clone()),
                        current_version: update_result
                            .as_ref()
                            .map(|result| result.current_version.clone()),
                        resolved_tag: update_result
                            .as_ref()
                            .map(|result| result.resolved_tag.clone()),
                        group: update_result.as_ref().and_then(|r| r.group.clone()),
                        force_replaced: None,
                        warning: rollback_updated_project(
                            &state.work_root,
                            reload_ctx.as_ref(),
                            request_id,
                            remote_addr,
                            "response_channel_closed",
                        ),
                        error: Some("runtime response channel closed".to_string()),
                    },
                ),
                Err(_) => {
                    if let Some(ctx) = reload_ctx.take() {
                        tokio::spawn(monitor_reload_result(
                            reply_rx,
                            state.work_root.clone(),
                            ctx,
                            remote_addr,
                            reason.to_string(),
                        ));
                    }
                    info_ctrl!(
                        "admin api reload still running request_id={} remote={} timeout_ms={} reason={}",
                        request_id,
                        remote_addr,
                        wait_timeout.as_millis(),
                        reason
                    );
                    json_response(
                        StatusCode::ACCEPTED,
                        &ReloadResponse {
                            request_id: request_id.to_string(),
                            accepted: true,
                            result: "running",
                            update: Some(reload_req.update),
                            requested_version: update_result
                                .as_ref()
                                .and_then(|result| result.requested_version.clone()),
                            current_version: update_result
                                .as_ref()
                                .map(|result| result.current_version.clone()),
                            resolved_tag: update_result
                                .as_ref()
                                .map(|result| result.resolved_tag.clone()),
                            group: update_result.as_ref().and_then(|r| r.group.clone()),
                            force_replaced: None,
                            warning: None,
                            error: None,
                        },
                    )
                }
            }
        }
        Err(err) => {
            let _ = rollback_updated_project(
                &state.work_root,
                reload_ctx.as_ref(),
                request_id,
                remote_addr,
                "send_error",
            );
            map_send_error(request_id, remote_addr, reason, err)
        }
    }
}

fn map_runtime_response(
    resp: RuntimeCommandResp,
    remote_addr: SocketAddr,
    reason: &str,
    update_result: Option<&crate::project_remote::ProjectRemoteUpdateResult>,
    rollback_warning: Option<String>,
) -> Response<Full<Bytes>> {
    match resp.result {
        RuntimeCommandResult::ReloadDone => {
            info_ctrl!(
                "admin api reload done request_id={} remote={} force_replaced=false reason={}",
                resp.request_id,
                remote_addr,
                reason
            );
            json_response(
                StatusCode::OK,
                &ReloadResponse {
                    request_id: resp.request_id,
                    accepted: resp.accepted,
                    result: "reload_done",
                    update: update_result.map(|_| true),
                    requested_version: update_result
                        .and_then(|result| result.requested_version.clone()),
                    current_version: update_result.map(|result| result.current_version.clone()),
                    resolved_tag: update_result.map(|result| result.resolved_tag.clone()),
                    group: update_result.and_then(|r| r.group.clone()),
                    force_replaced: Some(false),
                    warning: rollback_warning,
                    error: None,
                },
            )
        }
        RuntimeCommandResult::ReloadDoneWithForceReplace => {
            warn_ctrl!(
                "admin api reload force-replaced request_id={} remote={} reason={}",
                resp.request_id,
                remote_addr,
                reason
            );
            json_response(
                StatusCode::OK,
                &ReloadResponse {
                    request_id: resp.request_id,
                    accepted: resp.accepted,
                    result: "reload_done",
                    update: update_result.map(|_| true),
                    requested_version: update_result
                        .and_then(|result| result.requested_version.clone()),
                    current_version: update_result.map(|result| result.current_version.clone()),
                    resolved_tag: update_result.map(|result| result.resolved_tag.clone()),
                    group: update_result.and_then(|r| r.group.clone()),
                    force_replaced: Some(true),
                    warning: rollback_warning.or_else(|| {
                        Some("graceful drain timed out, fallback to force replace".to_string())
                    }),
                    error: None,
                },
            )
        }
        RuntimeCommandResult::ReloadFailed { reason: err } => {
            warn_ctrl!(
                "admin api reload failed request_id={} remote={} reason={} error={}",
                resp.request_id,
                remote_addr,
                reason,
                err
            );
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ReloadResponse {
                    request_id: resp.request_id,
                    accepted: resp.accepted,
                    result: "reload_failed",
                    update: update_result.map(|_| true),
                    requested_version: update_result
                        .and_then(|result| result.requested_version.clone()),
                    current_version: update_result.map(|result| result.current_version.clone()),
                    resolved_tag: update_result.map(|result| result.resolved_tag.clone()),
                    group: update_result.and_then(|r| r.group.clone()),
                    force_replaced: None,
                    warning: rollback_warning,
                    error: Some(err),
                },
            )
        }
    }
}

fn rollback_updated_project(
    work_root: &Path,
    reload_ctx: Option<&ProjectRemoteReloadContext>,
    request_id: &str,
    remote_addr: SocketAddr,
    stage: &str,
) -> Option<String> {
    let ctx = match reload_ctx {
        Some(ctx) => ctx,
        None => {
            warn_ctrl!(
                "admin api project rollback skipped (no context) request_id={} remote={} stage={}",
                request_id,
                remote_addr,
                stage
            );
            return None;
        }
    };
    let (snapshot, runtime_snapshot, changed, version) = match (
        ctx.snapshot.as_ref(),
        ctx.runtime_snapshot.as_ref(),
        ctx.update_result.as_ref(),
    ) {
        (Some(s), Some(r), Some(u)) => (s, r, u.changed, u.current_version.as_str()),
        (snap, rt, upd) => {
            let mut missing = Vec::new();
            if snap.is_none() {
                missing.push("snapshot");
            }
            if rt.is_none() {
                missing.push("runtime_snapshot");
            }
            if upd.is_none() {
                missing.push("update_result");
            }
            warn_ctrl!(
                "admin api project rollback missing components request_id={} remote={} stage={} missing={}",
                request_id,
                remote_addr,
                stage,
                missing.join(",")
            );
            // Attempt partial rollback with what we have
            let mut warnings = Vec::new();
            if let (Some(snapshot), Some(upd)) = (snap, upd) {
                if let Err(err) = crate::project_remote::restore_project_remote_update(
                    work_root,
                    snapshot,
                    upd.changed,
                ) {
                    warnings.push(format!("restore project failed: {}", err));
                }
            }
            if let Some(rt) = rt {
                if let Err(err) =
                    crate::project_remote::restore_runtime_artifact_snapshot(work_root, rt)
                {
                    warnings.push(format!("restore runtime artifacts failed: {}", err));
                }
            }
            if warnings.is_empty() {
                return None;
            }
            return Some(warnings.join("; "));
        }
    };
    match rollback_project_and_runtime(work_root, snapshot, changed, runtime_snapshot) {
        Ok(()) => {
            info_ctrl!(
                "admin api project rollback done request_id={} remote={} stage={} target_version={} changed={}",
                request_id,
                remote_addr,
                stage,
                version,
                changed
            );
            None
        }
        Err(err) => {
            warn_ctrl!(
                "admin api project rollback failed request_id={} remote={} stage={} error={}",
                request_id,
                remote_addr,
                stage,
                err
            );
            Some(format!("project rollback failed: {}", err))
        }
    }
}

fn rollback_project_and_runtime(
    work_root: &Path,
    snapshot: &crate::project_remote::ProjectRemoteSnapshot,
    changed: bool,
    runtime_snapshot: &crate::project_remote::ProjectRuntimeArtifactSnapshot,
) -> wp_error::run_error::RunResult<()> {
    let mut errs = Vec::new();
    if let Err(err) =
        crate::project_remote::restore_project_remote_update(work_root, snapshot, changed)
    {
        errs.push(format!("restore project failed: {}", err));
    }
    if let Err(err) =
        crate::project_remote::restore_runtime_artifact_snapshot(work_root, runtime_snapshot)
    {
        errs.push(format!("restore runtime artifacts failed: {}", err));
    }
    if errs.is_empty() {
        return Ok(());
    }
    Err(wp_error::run_error::RunReason::from_conf()
        .to_err()
        .with_detail(errs.join("; ")))
}

async fn monitor_reload_result(
    reply_rx: oneshot::Receiver<RuntimeCommandResp>,
    work_root: PathBuf,
    reload_ctx: ProjectRemoteReloadContext,
    remote_addr: SocketAddr,
    reason: String,
) {
    match reply_rx.await {
        Ok(resp) => {
            if matches!(resp.result, RuntimeCommandResult::ReloadFailed { .. }) {
                let _ = rollback_updated_project(
                    &work_root,
                    Some(&reload_ctx),
                    &resp.request_id,
                    remote_addr,
                    "background_reload_failed",
                );
            }
            info_ctrl!(
                "admin api background reload finished request_id={} remote={} result={} reason={}",
                resp.request_id,
                remote_addr,
                result_code(&resp.result),
                reason
            );
        }
        Err(_) => {
            let _ = rollback_updated_project(
                &work_root,
                Some(&reload_ctx),
                "<unknown>",
                remote_addr,
                "background_channel_closed",
            );
            warn_ctrl!(
                "admin api background reload response channel closed remote={} reason={}",
                remote_addr,
                reason
            );
        }
    }
}

fn read_project_version(work_root: &Path) -> RunResult<Option<serde_json::Value>> {
    match crate::project_remote::current_project_group_versions(work_root)? {
        Some(group_versions) => Ok(Some(group_versions)),
        None => Ok(crate::project_remote::current_project_version(work_root)?
            .map(serde_json::Value::String)),
    }
}

fn map_send_error(
    request_id: &str,
    remote_addr: SocketAddr,
    reason: &str,
    err: RuntimeCommandSendError,
) -> Response<Full<Bytes>> {
    match err {
        RuntimeCommandSendError::ReloadBusy => {
            warn_ctrl!(
                "admin api reload busy request_id={} remote={} reason={}",
                request_id,
                remote_addr,
                reason
            );
            json_response(
                StatusCode::CONFLICT,
                &ReloadResponse {
                    request_id: request_id.to_string(),
                    accepted: false,
                    result: "reload_in_progress",
                    update: None,
                    requested_version: None,
                    current_version: None,
                    resolved_tag: None,
                    group: None,
                    force_replaced: None,
                    warning: None,
                    error: None,
                },
            )
        }
        RuntimeCommandSendError::RuntimeNotReady => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &ErrorResponse {
                request_id: request_id.to_string(),
                accepted: false,
                result: "runtime_not_ready",
                error: "runtime command receiver not ready".to_string(),
            },
        ),
        RuntimeCommandSendError::ChannelClosed => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &ErrorResponse {
                request_id: request_id.to_string(),
                accepted: false,
                result: "runtime_unavailable",
                error: "runtime command channel closed".to_string(),
            },
        ),
    }
}

#[derive(Debug)]
enum ReadBodyError {
    TooLarge(usize),
    InvalidJson(String),
    Read(String),
}

async fn read_json_body<T>(mut body: Incoming, max_body_bytes: usize) -> Result<T, ReadBodyError>
where
    T: for<'de> Deserialize<'de>,
{
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame =
            frame.map_err(|e| ReadBodyError::Read(format!("read request body failed: {}", e)))?;
        if let Ok(data) = frame.into_data() {
            if bytes.len() + data.len() > max_body_bytes {
                return Err(ReadBodyError::TooLarge(max_body_bytes));
            }
            bytes.extend_from_slice(&data);
        }
    }

    serde_json::from_slice(&bytes)
        .map_err(|e| ReadBodyError::InvalidJson(format!("invalid JSON body: {}", e)))
}

fn bearer_token(headers: &hyper::HeaderMap<HeaderValue>) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ")
}

fn authorized(headers: &hyper::HeaderMap<HeaderValue>, token: &TokenAuth) -> bool {
    bearer_token(headers).is_some_and(|presented| token.verify(presented))
}

fn request_id(headers: &hyper::HeaderMap<HeaderValue>) -> String {
    headers
        .get("X-Request-Id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn json_response<T: Serialize>(status: StatusCode, value: &T) -> Response<Full<Bytes>> {
    let body = match serde_json::to_vec(value) {
        Ok(body) => body,
        Err(err) => {
            let fallback = format!(
                r#"{{"accepted":false,"result":"internal_error","error":"{}"}}"#,
                err
            );
            fallback.into_bytes()
        }
    };
    let mut resp = Response::new(Full::new(Bytes::from(body)));
    *resp.status_mut() = status;
    resp.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    resp
}

fn result_code(result: &RuntimeCommandResult) -> &'static str {
    match result {
        RuntimeCommandResult::ReloadDone | RuntimeCommandResult::ReloadDoneWithForceReplace => {
            "reload_done"
        }
        RuntimeCommandResult::ReloadFailed { .. } => "reload_failed",
    }
}

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let dt: DateTime<Utc> = time.into();
    dt.to_rfc3339()
}

fn hostname_for_instance() -> String {
    System::host_name().unwrap_or_else(|| "unknown-host".to_string())
}

fn admin_api_validation_err(detail: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail.into())
}

fn token_file_validation_err(detail: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail.into())
}

fn tls_validation_err(detail: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail.into())
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

fn default_wait() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, OnceLock};

    use reqwest::Client;
    use tempfile::tempdir;
    use wp_engine::facade::args::ParseArgs;
    use wp_engine::facade::WpApp;

    const BASE_TEST_WPARSE_CONF: &str = r#"version = "1.0"
robust = "normal"
skip_parse = false
skip_sink = false

[models]
wpl = "./models/wpl"
oml = "./models/oml"

[topology]
sources = "./topology/sources"
sinks = "./topology/sinks"

[performance]
rate_limit_rps = 10000
parse_workers = 2

[rescue]
path = "./data/rescue"

[log_conf]
level = "warn,ctrl=info,data=error,matrc=error,dfx=warn,kdb=warn"
output = "File"

[log_conf.file]
path = "./data/logs/"

[[stat.pick]]
key = "pick_stat"
target = "*"

[[stat.parse]]
key = "parse_stat"
target = "*"

[[stat.sink]]
key = "sink_stat"
target = "*"
"#;

    fn generate_self_signed_cert(dir: &Path) -> (PathBuf, PathBuf) {
        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        let status = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-keyout",
                key_path.to_str().expect("key path is valid utf-8"),
                "-out",
                cert_path.to_str().expect("cert path is valid utf-8"),
                "-days",
                "365",
                "-nodes",
                "-subj",
                "/CN=localhost",
            ])
            .status()
            .expect("run openssl to generate self-signed cert");
        assert!(
            status.success(),
            "openssl failed to generate self-signed cert"
        );
        (cert_path, key_path)
    }

    fn write_test_work_root(dir: &Path, bind: &str, token_file: &str) {
        let conf_dir = dir.join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        let mut base = BASE_TEST_WPARSE_CONF.to_string();
        base.push_str(&format!(
            r#"

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
token_file = "{token_file}"
"#
        ));
        fs::write(conf_dir.join("wparse.toml"), base).expect("write config");
    }

    fn write_token(dir: &Path, rel_path: &str, mode: u32) {
        write_token_body(dir, rel_path, "test-token\n", mode);
    }

    fn write_token_body(dir: &Path, rel_path: &str, body: &str, mode: u32) {
        let token_path = dir.join(rel_path);
        if let Some(parent) = token_path.parent() {
            fs::create_dir_all(parent).expect("create token dir");
        }
        fs::write(&token_path, body).expect("write token");
        set_mode(&token_path, mode);
    }

    fn set_mode(path: &Path, mode: u32) {
        let mut perms = fs::metadata(path).expect("stat token").permissions();
        perms.set_mode(mode);
        fs::set_permissions(path, perms).expect("chmod token");
    }

    fn load_enabled_config(dir: &Path) -> RunResult<ResolvedAdminApiConfig> {
        match load_config(dir, &EnvDict::default()) {
            Ok(Some(conf)) => Ok(conf),
            Ok(None) => panic!("admin api should be enabled"),
            Err(err) => Err(err),
        }
    }

    #[test]
    fn admin_api_local_overlay_overrides_engine_config() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:19090", "runtime/admin_api.token");
        fs::write(
            temp.path().join(LOCAL_ADMIN_API_CONF),
            r#"
[admin_api]
enabled = true
bind = "127.0.0.1:19191"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.auth]
token_file = "runtime/admin_api.token"
"#,
        )
        .expect("write local overlay");

        let profile = resolve_client_profile(temp.path(), &EnvDict::default())
            .expect("resolve profile")
            .expect("admin api enabled");
        assert_eq!(profile.base_url, "http://127.0.0.1:19191");
        assert_eq!(
            profile.token_file,
            temp.path().join("runtime/admin_api.token")
        );
    }

    #[test]
    fn resolve_against_normalizes_relative_paths() {
        let root = Path::new("/work");
        assert_eq!(
            resolve_against(root, "runtime/token"),
            "/work/runtime/token"
        );
        assert_eq!(
            resolve_against(root, "./tls/cert.pem"),
            "/work/tls/cert.pem"
        );
        assert_eq!(
            resolve_against(root, "../shared/cert.pem"),
            "/shared/cert.pem"
        );
        assert_eq!(resolve_against(root, "/abs/cert.pem"), "/abs/cert.pem");
        assert_eq!(resolve_against(root, ""), "");
    }

    #[test]
    fn admin_api_local_overlay_without_known_section_errors() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:19092", "runtime/admin_api.token");
        fs::write(
            temp.path().join(LOCAL_ADMIN_API_CONF),
            "[other]\nkey = \"value\"\n",
        )
        .expect("write local overlay");

        let err = resolve_client_profile(temp.path(), &EnvDict::default())
            .expect_err("an overlay without a known section must not be ignored");
        assert!(
            err.to_string()
                .contains("neither [admin_api] nor [runtime]"),
            "{err}"
        );
    }

    #[test]
    fn admin_api_falls_back_when_overlay_has_only_runtime() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:19192", "runtime/admin_api.token");
        fs::write(
            temp.path().join(LOCAL_ADMIN_API_CONF),
            "[runtime]\nstate_dir = \"state\"\n",
        )
        .expect("write local overlay");

        let profile = resolve_client_profile(temp.path(), &EnvDict::default())
            .expect("resolve profile")
            .expect("admin api enabled");
        assert_eq!(profile.base_url, "http://127.0.0.1:19192");
    }

    #[test]
    fn admin_api_local_overlay_malformed_toml_errors() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:19093", "runtime/admin_api.token");
        fs::write(
            temp.path().join(LOCAL_ADMIN_API_CONF),
            "[admin_api]\nenabled = true\nbind = \n",
        )
        .expect("write malformed overlay");

        let err = resolve_client_profile(temp.path(), &EnvDict::default())
            .expect_err("malformed overlay must fail");
        assert!(err.to_string().contains("parse"));
    }

    #[test]
    fn admin_api_local_overlay_disables_legacy_config() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:19094", "runtime/admin_api.token");
        fs::write(
            temp.path().join(LOCAL_ADMIN_API_CONF),
            "[admin_api]\nenabled = false\n",
        )
        .expect("write local overlay");

        let profile =
            resolve_client_profile(temp.path(), &EnvDict::default()).expect("resolve profile");
        assert!(
            profile.is_none(),
            "local overlay must win over legacy config"
        );
    }

    #[test]
    fn admin_api_local_overlay_drives_server_bind() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:19090", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);
        fs::write(
            temp.path().join(LOCAL_ADMIN_API_CONF),
            r#"
[admin_api]
enabled = true
bind = "127.0.0.1:0"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.auth]
token_file = "runtime/admin_api.token"
"#,
        )
        .expect("write local overlay");

        let conf = load_config(temp.path(), &EnvDict::default())
            .expect("load server config")
            .expect("admin api enabled");
        assert_eq!(conf.bind.port(), 0);
        assert!(conf.token.verify("test-token"));
    }

    fn shared_control_handle() -> RuntimeControlHandle {
        fn shared_control_work_root() -> &'static PathBuf {
            static WORK_ROOT: OnceLock<PathBuf> = OnceLock::new();
            WORK_ROOT.get_or_init(|| {
                let root = std::env::temp_dir().join("warp-parse-admin-api-tests");
                let conf_dir = root.join("conf");
                fs::create_dir_all(&conf_dir).expect("create shared control conf dir");
                fs::write(conf_dir.join("wparse.toml"), BASE_TEST_WPARSE_CONF)
                    .expect("write shared control config");
                root
            })
        }

        static HANDLE: OnceLock<RuntimeControlHandle> = OnceLock::new();
        HANDLE
            .get_or_init(|| {
                let args = ParseArgs {
                    work_root: Some(shared_control_work_root().to_string_lossy().to_string()),
                    ..Default::default()
                };
                WpApp::try_from(args, orion_variate::EnvDict::default())
                    .expect("build wp app")
                    .control_handle()
            })
            .clone()
    }

    fn home_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct HomeOverride {
        original: Option<std::ffi::OsString>,
    }

    impl HomeOverride {
        fn new(home: &Path) -> Self {
            let original = std::env::var_os("HOME");
            unsafe {
                std::env::set_var("HOME", home);
            }
            Self { original }
        }
    }

    impl Drop for HomeOverride {
        fn drop(&mut self) {
            match &self.original {
                Some(home) => unsafe {
                    std::env::set_var("HOME", home);
                },
                None => unsafe {
                    std::env::remove_var("HOME");
                },
            }
        }
    }

    #[test]
    fn token_file_permission_matrix_accepts_read_only_secrets() {
        for mode in [0o600u32, 0o400, 0o640, 0o440, 0o444] {
            let temp = tempdir().expect("tempdir");
            write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
            write_token(temp.path(), "runtime/admin_api.token", mode);

            let conf = load_enabled_config(temp.path())
                .unwrap_or_else(|err| panic!("mode {mode:o} should be accepted: {err}"));
            assert!(conf.token.verify("test-token"), "mode {mode:o}");
        }
    }

    #[test]
    fn token_file_permission_matrix_rejects_group_or_other_write() {
        for mode in [0o660u32, 0o664, 0o777, 0o622] {
            let temp = tempdir().expect("tempdir");
            write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
            write_token(temp.path(), "runtime/admin_api.token", mode);

            let err = match load_config(temp.path(), &EnvDict::default()) {
                Ok(_) => panic!("mode {mode:o} should be rejected"),
                Err(err) => err,
            };
            assert!(
                err.to_string().contains("too permissive"),
                "mode {mode:o}: {err}"
            );
        }
    }

    #[test]
    fn token_file_must_contain_one_non_empty_line() {
        let cases = [
            ("", "token file is empty"),
            ("\n\r\n", "token file is empty"),
            ("one\ntwo\n", "exactly one token"),
        ];
        for (body, expected) in cases {
            let temp = tempdir().expect("tempdir");
            write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
            write_token_body(temp.path(), "runtime/admin_api.token", body, 0o600);

            let err = match load_config(temp.path(), &EnvDict::default()) {
                Ok(_) => panic!("body {body:?} should be rejected"),
                Err(err) => err,
            };
            assert!(err.to_string().contains(expected), "body {body:?}: {err}");
        }
    }

    #[test]
    fn token_file_directory_is_rejected() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        fs::create_dir_all(temp.path().join("runtime/admin_api.token")).expect("create dir");

        let err = match load_config(temp.path(), &EnvDict::default()) {
            Ok(_) => panic!("directory should be rejected"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("not a regular file"),
            "unexpected error: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn token_file_symlink_to_secret_target_is_accepted() {
        use std::os::unix::fs::symlink;

        // Mirrors the Kubernetes projected-secret layout: `token` is a symlink
        // into a versioned data directory.
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        let data_dir = temp.path().join("runtime/..2026_10_23_data");
        fs::create_dir_all(&data_dir).expect("create data dir");
        fs::write(data_dir.join("token"), "test-token\n").expect("write token");
        set_mode(&data_dir.join("token"), 0o444);
        symlink(
            data_dir.join("token"),
            temp.path().join("runtime/admin_api.token"),
        )
        .expect("create symlink");

        let conf = load_enabled_config(temp.path()).expect("symlinked secret should be accepted");
        assert!(conf.token.verify("test-token"));
    }

    #[cfg(unix)]
    #[test]
    fn token_file_symlink_to_writable_target_is_rejected() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        fs::create_dir_all(temp.path().join("runtime")).expect("create runtime dir");
        let target = temp.path().join("runtime/real.token");
        fs::write(&target, "test-token\n").expect("write token");
        set_mode(&target, 0o666);
        symlink(&target, temp.path().join("runtime/admin_api.token")).expect("create symlink");

        let err = match load_config(temp.path(), &EnvDict::default()) {
            Ok(_) => panic!("writable symlink target should be rejected"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("too permissive"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_failure_log_is_rate_limited() {
        let mut log = AuthFailureLog::default();
        let t0 = std::time::Instant::now();
        assert_eq!(log.record(t0), Some(String::new()));
        assert_eq!(log.record(t0 + Duration::from_secs(1)), None);
        assert_eq!(log.record(t0 + Duration::from_secs(2)), None);
        assert_eq!(
            log.record(t0 + Duration::from_secs(11)),
            Some(" suppressed=2".to_string())
        );
    }

    #[test]
    fn admin_api_loads_config_without_auth_mode() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            "version = \"1.0\"\n\n[admin_api]\nenabled = true\nbind = \"127.0.0.1:0\"\n\n[admin_api.auth]\ntoken_file = \"runtime/admin_api.token\"\n",
        )
        .expect("write config");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let conf =
            load_enabled_config(temp.path()).expect("config without mode should be accepted");
        assert!(conf.token.verify("test-token"));
    }

    #[test]
    fn admin_api_rejects_removed_auth_mode_field() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            "version = \"1.0\"\n\n[admin_api]\nenabled = true\nbind = \"127.0.0.1:0\"\n\n[admin_api.auth]\nmode = \"bearer_token\"\ntoken_file = \"runtime/admin_api.token\"\n",
        )
        .expect("write config");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        // `mode` is gone from the schema entirely; `deny_unknown_fields` in
        // wp-config rejects legacy configs that still carry it.
        assert!(
            load_config(temp.path(), &EnvDict::default()).is_err(),
            "configs with the removed admin_api.auth.mode field must be rejected"
        );
    }

    #[tokio::test]
    async fn admin_api_accepts_read_only_secret_permissions() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o444);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("read-only secret must be accepted")
            .expect("admin api enabled");
        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_rejects_writable_token_permissions() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o666);

        let dict = EnvDict::default();
        let err = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect_err("should reject writable token file");
        assert!(
            err.to_string().contains("too permissive"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn admin_api_expands_token_file_env_in_daemon_path() {
        let _guard = home_lock().lock().expect("lock HOME override");
        let temp = tempdir().expect("tempdir");
        let home = temp.path().join("fake-home");
        fs::create_dir_all(&home).expect("create fake home");
        let _home = HomeOverride::new(&home);

        write_test_work_root(
            temp.path(),
            "127.0.0.1:0",
            "${HOME}/.warp_parse/admin_api.token",
        );
        write_token(temp.path(), "fake-home/.warp_parse/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let loaded = load_config(temp.path(), &dict)
            .expect("load admin api config with env token path")
            .expect("enabled");
        assert_eq!(
            loaded.bind,
            "127.0.0.1:0".parse().expect("parse socket addr")
        );
        assert!(loaded.token.verify("test-token"));
    }

    #[tokio::test]
    async fn admin_api_status_requires_bearer_and_reports_runtime_state() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("build reqwest client without proxy");
        let base = format!("http://{}", runtime.local_addr());

        let unauthorized = client
            .get(format!("{}/admin/v1/runtime/status", base))
            .send()
            .await
            .expect("send unauthorized request");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let authorized = client
            .get(format!("{}/admin/v1/runtime/status", base))
            .bearer_auth("test-token")
            .send()
            .await
            .expect("send authorized request");
        assert_eq!(authorized.status(), StatusCode::OK);
        let body: serde_json::Value = authorized.json().await.expect("parse json");
        assert!(body["project_version"].is_null());
        assert_eq!(body["accepting_commands"], false);
        assert_eq!(body["reloading"], false);
        assert_eq!(
            body["token_fp"],
            serde_json::Value::String(token::fingerprint("test-token")),
            "status must expose the token fingerprint, never the token"
        );

        let reload = client
            .post(format!("{}/admin/v1/reloads/model", base))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"wait": false, "reason": "test"}))
            .send()
            .await
            .expect("send reload request");
        assert_eq!(reload.status(), StatusCode::SERVICE_UNAVAILABLE);

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_allows_non_loopback_without_tls() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "0.0.0.0:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("non-loopback without TLS should start with a warning")
            .expect("enabled");

        runtime.shutdown().await;
    }

    fn write_test_work_root_with_tls(
        dir: &Path,
        bind: &str,
        token_file: &str,
        cert_file: &str,
        key_file: &str,
    ) {
        let conf_dir = dir.join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        let mut base = BASE_TEST_WPARSE_CONF.to_string();
        base.push_str(&format!(
            r#"

[admin_api]
enabled = true
bind = "{bind}"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.tls]
enabled = true
cert_file = "{cert_file}"
key_file = "{key_file}"

[admin_api.auth]
token_file = "{token_file}"
"#
        ));
        fs::write(conf_dir.join("wparse.toml"), base).expect("write config");
    }

    fn init_tls_crypto() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("install rustls ring crypto provider");
        });
    }

    #[tokio::test]
    async fn admin_api_tls_accepts_https_requests() {
        init_tls_crypto();
        let temp = tempdir().expect("tempdir");
        let (cert_path, key_path) = generate_self_signed_cert(temp.path());
        write_test_work_root_with_tls(
            temp.path(),
            "127.0.0.1:0",
            "runtime/admin_api.token",
            &cert_path.to_string_lossy(),
            &key_path.to_string_lossy(),
        );
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("start admin api with TLS")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build reqwest client with unsafe TLS");
        let base = format!("https://{}", runtime.local_addr());

        // Without bearer token -> 401
        let unauthorized = client
            .get(format!("{}/admin/v1/runtime/status", base))
            .send()
            .await
            .expect("send unauthorized HTTPS request");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        // With bearer token -> 200
        let authorized = client
            .get(format!("{}/admin/v1/runtime/status", base))
            .bearer_auth("test-token")
            .send()
            .await
            .expect("send authorized HTTPS request");
        assert_eq!(authorized.status(), StatusCode::OK);
        let body: serde_json::Value = authorized.json().await.expect("parse json");
        assert_eq!(body["accepting_commands"], false);

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_tls_works_with_non_loopback() {
        init_tls_crypto();
        let temp = tempdir().expect("tempdir");
        let (cert_path, key_path) = generate_self_signed_cert(temp.path());
        write_test_work_root_with_tls(
            temp.path(),
            "0.0.0.0:0",
            "runtime/admin_api.token",
            &cert_path.to_string_lossy(),
            &key_path.to_string_lossy(),
        );
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("non-loopback with TLS should start successfully")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("build reqwest client with unsafe TLS");
        let base = format!("https://{}", runtime.local_addr());

        let response = client
            .get(format!("{}/admin/v1/runtime/status", base))
            .bearer_auth("test-token")
            .send()
            .await
            .expect("send HTTPS request to non-loopback TLS server");
        assert_eq!(response.status(), StatusCode::OK);

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_rejects_version_without_update() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("build reqwest client without proxy");
        let base = format!("http://{}", runtime.local_addr());
        let response = client
            .post(format!("{}/admin/v1/reloads/model", base))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"wait": false, "version": "1.4.3"}))
            .send()
            .await
            .expect("send reload request");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = response.json().await.expect("parse json");
        assert_eq!(body["result"], "invalid_request");
        assert_eq!(body["error"], "version requires update=true");

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_rejects_group_without_update() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("build reqwest client without proxy");
        let base = format!("http://{}", runtime.local_addr());
        let response = client
            .post(format!("{}/admin/v1/reloads/model", base))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"wait": false, "group": "models"}))
            .send()
            .await
            .expect("send reload request");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = response.json().await.expect("parse json");
        assert_eq!(body["result"], "invalid_request");
        assert_eq!(body["error"], "group requires update=true");

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_rejects_dual_update_without_group() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        let mut base = BASE_TEST_WPARSE_CONF.to_string();
        base.push_str(
            r#"

[admin_api]
enabled = true
bind = "127.0.0.1:0"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.tls]
enabled = false
cert_file = ""
key_file = ""

[admin_api.auth]
token_file = "runtime/admin_api.token"

[project_remote]
enabled = true
repo = ""

[project_remote.models]
repo = "https://github.com/wp-labs/wp-rule.git"
init_version = "0.1.0"

[project_remote.infra]
repo = "https://github.com/wp-labs/editor-monitor-conf.git"
init_version = "0.1.7"
"#,
        );
        fs::write(conf_dir.join("wparse.toml"), base).expect("write dual config");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("build reqwest client without proxy");
        let base = format!("http://{}", runtime.local_addr());

        let response = client
            .post(format!("{}/admin/v1/reloads/model", base))
            .bearer_auth("test-token")
            .json(&serde_json::json!({"wait": false, "update": true}))
            .send()
            .await
            .expect("send reload request");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body: serde_json::Value = response.json().await.expect("parse json");
        assert_eq!(body["result"], "invalid_request");
        assert!(
            body["error"]
                .as_str()
                .expect("error string")
                .contains("group"),
            "error should mention group: {}",
            body["error"]
        );

        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn admin_api_status_reports_per_group_versions_in_dual_mode() {
        let temp = tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        let mut base = BASE_TEST_WPARSE_CONF.to_string();
        base.push_str(
            r#"

[admin_api]
enabled = true
bind = "127.0.0.1:0"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.tls]
enabled = false
cert_file = ""
key_file = ""

[admin_api.auth]
token_file = "runtime/admin_api.token"

[project_remote]
enabled = true
repo = ""

[project_remote.models]
repo = "https://github.com/wp-labs/wp-rule.git"
init_version = "0.1.0"

[project_remote.infra]
repo = "https://github.com/wp-labs/editor-monitor-conf.git"
init_version = "0.1.7"
"#,
        );
        fs::write(conf_dir.join("wparse.toml"), base).expect("write dual config");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        // Write dual state file
        let run_dir = temp.path().join(".run");
        fs::create_dir_all(&run_dir).expect("create .run dir");
        let state_json = serde_json::json!({
            "models": {
                "version": "1.4.2",
                "tag": "v1.4.2",
                "revision": "abc123def456"
            },
            "infra": {
                "version": "0.1.7",
                "tag": "v0.1.7",
                "revision": "def456abc123"
            }
        });
        fs::write(
            run_dir.join("project_remote_state.json"),
            serde_json::to_vec(&state_json).expect("serialize state"),
        )
        .expect("write state");

        let dict = EnvDict::default();
        let runtime = start_if_enabled(temp.path(), &dict, shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        let client = Client::builder()
            .no_proxy()
            .build()
            .expect("build reqwest client without proxy");
        let base = format!("http://{}", runtime.local_addr());

        let authorized = client
            .get(format!("{}/admin/v1/runtime/status", base))
            .bearer_auth("test-token")
            .send()
            .await
            .expect("send authorized request");
        assert_eq!(authorized.status(), StatusCode::OK);
        let body: serde_json::Value = authorized.json().await.expect("parse json");
        let pv = &body["project_version"];
        assert!(
            pv.is_object(),
            "project_version should be object, got: {}",
            pv
        );
        assert_eq!(pv["models"]["version"], "1.4.2");
        assert_eq!(pv["models"]["tag"], "v1.4.2");
        assert_eq!(pv["infra"]["version"], "0.1.7");
        assert_eq!(pv["infra"]["tag"], "v0.1.7");

        runtime.shutdown().await;
    }
}
