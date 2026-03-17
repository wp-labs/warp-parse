use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use hyper::http::StatusCode;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use orion_conf::{ToStructError, UvsConfFrom};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;
use wp_engine::facade::{
    RuntimeCommandResp, RuntimeCommandResult, RuntimeCommandSendError, RuntimeControlHandle,
};
use wp_error::run_error::{RunReason, RunResult};
use wp_log::{info_ctrl, warn_ctrl};

const ADMIN_CONF_PATH: &str = "conf/wparse.toml";
const DEFAULT_BIND: &str = "127.0.0.1:19090";
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_MAX_BODY_BYTES: usize = 4096;
const DEFAULT_AUTH_MODE: &str = "bearer_token";

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
    control_handle: RuntimeControlHandle,
) -> RunResult<Option<AdminApiRuntime>> {
    let config = load_config(work_root)?;
    let Some(config) = config else {
        return Ok(None);
    };

    let listener = TcpListener::bind(config.bind)
        .await
        .map_err(|e| conf_err(format!("bind admin api on {} failed: {}", config.bind, e)))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| conf_err(format!("read admin api local addr failed: {}", e)))?;
    let instance_id = format!("{}:{}", hostname_for_instance(), std::process::id());
    let state = Arc::new(AppState {
        control_handle,
        bearer_token: config.bearer_token,
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

#[derive(Debug, Deserialize, Default)]
struct HostConfigFile {
    #[serde(default)]
    admin_api: AdminApiConfigFile,
}

#[derive(Debug, Deserialize, Clone)]
struct AdminApiConfigFile {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_bind")]
    bind: String,
    #[serde(default = "default_request_timeout_ms")]
    request_timeout_ms: u64,
    #[serde(default = "default_max_body_bytes")]
    max_body_bytes: usize,
    #[serde(default)]
    tls: AdminTlsConfigFile,
    #[serde(default)]
    auth: AdminAuthConfigFile,
}

impl Default for AdminApiConfigFile {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_bind(),
            request_timeout_ms: default_request_timeout_ms(),
            max_body_bytes: default_max_body_bytes(),
            tls: AdminTlsConfigFile::default(),
            auth: AdminAuthConfigFile::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
struct AdminTlsConfigFile {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    cert_file: String,
    #[serde(default)]
    key_file: String,
}

#[derive(Debug, Deserialize, Clone)]
struct AdminAuthConfigFile {
    #[serde(default = "default_auth_mode")]
    mode: String,
    #[serde(default)]
    token_file: String,
}

impl Default for AdminAuthConfigFile {
    fn default() -> Self {
        Self {
            mode: default_auth_mode(),
            token_file: String::new(),
        }
    }
}

struct ResolvedAdminApiConfig {
    bind: SocketAddr,
    request_timeout: Duration,
    max_body_bytes: usize,
    bearer_token: String,
    tls: Option<ServerConfig>,
}

#[derive(Debug, Clone)]
pub struct AdminApiClientProfile {
    pub base_url: String,
    pub token_file: PathBuf,
    pub request_timeout: Duration,
}

fn load_config(work_root: &Path) -> RunResult<Option<ResolvedAdminApiConfig>> {
    let conf_path = work_root.join(ADMIN_CONF_PATH);
    let raw = fs::read_to_string(&conf_path)
        .map_err(|e| conf_err(format!("read {} failed: {}", conf_path.display(), e)))?;
    let parsed: HostConfigFile = toml::from_str(&raw)
        .map_err(|e| conf_err(format!("parse {} failed: {}", conf_path.display(), e)))?;
    if !parsed.admin_api.enabled {
        return Ok(None);
    }

    let bind: SocketAddr = parsed.admin_api.bind.parse().map_err(|e| {
        conf_err(format!(
            "invalid admin_api.bind '{}': {}",
            parsed.admin_api.bind, e
        ))
    })?;
    if parsed.admin_api.max_body_bytes == 0 {
        return Err(conf_err("admin_api.max_body_bytes must be > 0"));
    }

    let auth_mode = parsed.admin_api.auth.mode.trim().to_ascii_lowercase();
    if auth_mode != DEFAULT_AUTH_MODE {
        return Err(conf_err(format!(
            "unsupported admin_api.auth.mode '{}', expected '{}'",
            parsed.admin_api.auth.mode, DEFAULT_AUTH_MODE
        )));
    }
    if parsed.admin_api.auth.token_file.trim().is_empty() {
        return Err(conf_err(
            "admin_api.auth.token_file must be set when admin_api is enabled",
        ));
    }
    let token_path = resolve_path(work_root, &parsed.admin_api.auth.token_file);
    validate_token_file(&token_path)?;
    let bearer_token = fs::read_to_string(&token_path)
        .map_err(|e| {
            conf_err(format!(
                "read token file {} failed: {}",
                token_path.display(),
                e
            ))
        })?
        .trim()
        .to_string();
    if bearer_token.is_empty() {
        return Err(conf_err(format!(
            "token file {} is empty",
            token_path.display()
        )));
    }

    let tls = if parsed.admin_api.tls.enabled {
        Some(load_tls_config(work_root, &parsed.admin_api.tls)?)
    } else {
        None
    };

    if !bind.ip().is_loopback() && tls.is_none() {
        return Err(conf_err(format!(
            "non-loopback admin_api.bind '{}' requires admin_api.tls.enabled=true",
            bind
        )));
    }

    Ok(Some(ResolvedAdminApiConfig {
        bind,
        request_timeout: Duration::from_millis(parsed.admin_api.request_timeout_ms),
        max_body_bytes: parsed.admin_api.max_body_bytes,
        bearer_token,
        tls,
    }))
}

pub fn resolve_client_profile(work_root: &Path) -> RunResult<Option<AdminApiClientProfile>> {
    let conf_path = work_root.join(ADMIN_CONF_PATH);
    let raw = fs::read_to_string(&conf_path)
        .map_err(|e| conf_err(format!("read {} failed: {}", conf_path.display(), e)))?;
    let parsed: HostConfigFile = toml::from_str(&raw)
        .map_err(|e| conf_err(format!("parse {} failed: {}", conf_path.display(), e)))?;
    if !parsed.admin_api.enabled {
        return Ok(None);
    }

    let bind: SocketAddr = parsed.admin_api.bind.parse().map_err(|e| {
        conf_err(format!(
            "invalid admin_api.bind '{}': {}",
            parsed.admin_api.bind, e
        ))
    })?;
    let token_file = parsed.admin_api.auth.token_file.trim();
    if token_file.is_empty() {
        return Err(conf_err(
            "admin_api.auth.token_file must be set when admin_api is enabled",
        ));
    }

    let scheme = if parsed.admin_api.tls.enabled {
        "https"
    } else {
        "http"
    };
    Ok(Some(AdminApiClientProfile {
        base_url: format!("{}://{}", scheme, bind),
        token_file: resolve_path(work_root, token_file),
        request_timeout: Duration::from_millis(parsed.admin_api.request_timeout_ms),
    }))
}

fn validate_token_file(path: &Path) -> RunResult<()> {
    let meta = fs::metadata(path)
        .map_err(|e| conf_err(format!("stat token file {} failed: {}", path.display(), e)))?;
    if !meta.is_file() {
        return Err(conf_err(format!(
            "token file {} is not a regular file",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(conf_err(format!(
                "token file {} permissions {:o} are too permissive; require owner-only access",
                path.display(),
                mode
            )));
        }
    }
    Ok(())
}

fn load_tls_config(work_root: &Path, tls: &AdminTlsConfigFile) -> RunResult<ServerConfig> {
    if tls.cert_file.trim().is_empty() || tls.key_file.trim().is_empty() {
        return Err(conf_err(
            "admin_api.tls.cert_file and admin_api.tls.key_file must be set when TLS is enabled",
        ));
    }
    let cert_path = resolve_path(work_root, &tls.cert_file);
    let key_path = resolve_path(work_root, &tls.key_file);
    let cert_pem = fs::read(&cert_path).map_err(|e| {
        conf_err(format!(
            "read cert file {} failed: {}",
            cert_path.display(),
            e
        ))
    })?;
    let key_pem = fs::read(&key_path).map_err(|e| {
        conf_err(format!(
            "read key file {} failed: {}",
            key_path.display(),
            e
        ))
    })?;

    let certs = CertificateDer::pem_slice_iter(&cert_pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            conf_err(format!(
                "parse PEM certs from {} failed: {}",
                cert_path.display(),
                e
            ))
        })?;
    if certs.is_empty() {
        return Err(conf_err(format!(
            "no certificates found in {}",
            cert_path.display()
        )));
    }
    let key = PrivateKeyDer::from_pem_slice(&key_pem).map_err(|e| {
        conf_err(format!(
            "parse PEM key from {} failed: {}",
            key_path.display(),
            e
        ))
    })?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| conf_err(format!("build TLS server config failed: {}", e)))?;
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(server_config)
}

fn resolve_path(work_root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        work_root.join(path)
    }
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
    bearer_token: String,
    request_timeout: Duration,
    max_body_bytes: usize,
    instance_id: String,
    version: String,
}

#[derive(Debug, Deserialize, Default)]
struct ReloadRequest {
    #[serde(default = "default_wait")]
    wait: bool,
    timeout_ms: Option<u64>,
    reason: Option<String>,
}

#[derive(Serialize)]
struct ReloadResponse {
    request_id: String,
    accepted: bool,
    result: &'static str,
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

    if !authorized(req.headers(), &state.bearer_token) {
        warn_ctrl!(
            "admin api unauthorized request_id={} remote={} method={} path={}",
            request_id,
            remote_addr,
            method,
            path
        );
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
                return json_response(
                    StatusCode::ACCEPTED,
                    &ReloadResponse {
                        request_id: request_id.to_string(),
                        accepted: true,
                        result: "running",
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
            match timeout(wait_timeout, reply_rx).await {
                Ok(Ok(resp)) => map_runtime_response(resp, remote_addr, reason),
                Ok(Err(_)) => json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ReloadResponse {
                        request_id: request_id.to_string(),
                        accepted: true,
                        result: "reload_failed",
                        force_replaced: None,
                        warning: None,
                        error: Some("runtime response channel closed".to_string()),
                    },
                ),
                Err(_) => {
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
                            force_replaced: None,
                            warning: None,
                            error: None,
                        },
                    )
                }
            }
        }
        Err(err) => map_send_error(request_id, remote_addr, reason, err),
    }
}

fn map_runtime_response(
    resp: RuntimeCommandResp,
    remote_addr: SocketAddr,
    reason: &str,
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
                    force_replaced: Some(false),
                    warning: None,
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
                    force_replaced: Some(true),
                    warning: Some(
                        "graceful drain timed out, fallback to force replace".to_string(),
                    ),
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
                    force_replaced: None,
                    warning: None,
                    error: Some(err),
                },
            )
        }
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

fn authorized(headers: &hyper::HeaderMap<HeaderValue>, token: &str) -> bool {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    let Some(token_part) = value.strip_prefix("Bearer ") else {
        return false;
    };
    token_part == token
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

fn conf_err(detail: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(detail.into())
}

fn default_bind() -> String {
    DEFAULT_BIND.to_string()
}

fn default_request_timeout_ms() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_MS
}

fn default_max_body_bytes() -> usize {
    DEFAULT_MAX_BODY_BYTES
}

fn default_auth_mode() -> String {
    DEFAULT_AUTH_MODE.to_string()
}

fn default_wait() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::OnceLock;

    use reqwest::Client;
    use tempfile::tempdir;
    use wp_engine::facade::args::ParseArgs;
    use wp_engine::facade::WpApp;

    fn write_test_work_root(dir: &Path, bind: &str, token_file: &str) {
        let conf_dir = dir.join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        let mut base = include_str!("../tests/conf/wparse.toml").to_string();
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
mode = "bearer_token"
token_file = "{token_file}"
"#
        ));
        fs::write(conf_dir.join("wparse.toml"), base).expect("write config");
    }

    fn write_token(dir: &Path, rel_path: &str, mode: u32) {
        let token_path = dir.join(rel_path);
        if let Some(parent) = token_path.parent() {
            fs::create_dir_all(parent).expect("create token dir");
        }
        fs::write(&token_path, "test-token\n").expect("write token");
        let mut perms = fs::metadata(&token_path).expect("stat token").permissions();
        perms.set_mode(mode);
        fs::set_permissions(&token_path, perms).expect("chmod token");
    }

    fn shared_control_handle() -> RuntimeControlHandle {
        static HANDLE: OnceLock<RuntimeControlHandle> = OnceLock::new();
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

    #[tokio::test]
    async fn admin_api_requires_safe_token_permissions() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o644);

        let err = start_if_enabled(temp.path(), shared_control_handle())
            .await
            .expect_err("should reject unsafe token file");
        assert!(
            err.to_string().contains("too permissive"),
            "unexpected error: {}",
            err
        );
    }

    #[tokio::test]
    async fn admin_api_status_requires_bearer_and_reports_runtime_state() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "127.0.0.1:0", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let runtime = start_if_enabled(temp.path(), shared_control_handle())
            .await
            .expect("start admin api")
            .expect("enabled");

        let client = Client::new();
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
        assert_eq!(body["accepting_commands"], false);
        assert_eq!(body["reloading"], false);

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
    async fn admin_api_rejects_non_loopback_without_tls() {
        let temp = tempdir().expect("tempdir");
        write_test_work_root(temp.path(), "0.0.0.0:19090", "runtime/admin_api.token");
        write_token(temp.path(), "runtime/admin_api.token", 0o600);

        let err = start_if_enabled(temp.path(), shared_control_handle())
            .await
            .expect_err("should reject non-loopback without tls");
        assert!(
            err.to_string()
                .contains("requires admin_api.tls.enabled=true"),
            "unexpected error: {}",
            err
        );
    }
}
