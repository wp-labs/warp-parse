use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use serde_json::Value;
use serial_test::serial;
use tempfile::tempdir;

#[derive(Clone, Copy, Default)]
struct FixtureOptions {
    blackhole_sleep_ms: Option<u64>,
}

fn reserve_local_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);
    addr
}

fn write_file(work_root: &Path, rel_path: &str, content: &str) {
    let path = work_root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, content).expect("write file");
}

fn write_fixture(work_root: &Path, bind: SocketAddr, source_bind: SocketAddr) -> PathBuf {
    write_fixture_with_options(work_root, bind, source_bind, FixtureOptions::default())
}

fn write_fixture_with_options(
    work_root: &Path,
    bind: SocketAddr,
    source_bind: SocketAddr,
    options: FixtureOptions,
) -> PathBuf {
    let token_path = work_root.join("runtime/admin_api.token");
    if let Some(parent) = token_path.parent() {
        fs::create_dir_all(parent).expect("create token dir");
    }
    fs::write(&token_path, "test-token\n").expect("write token");
    let mut perms = fs::metadata(&token_path).expect("stat token").permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&token_path, perms).expect("chmod token");

    let sec_dir = work_root.join(".warp_parse");
    fs::create_dir_all(&sec_dir).expect("create sec dir");
    fs::write(sec_dir.join("sec_key.toml"), "X = \"hello\"\n").expect("write sec key");

    fs::create_dir_all(work_root.join("data/out_dat")).expect("create output dir");
    fs::create_dir_all(work_root.join("data/rescue")).expect("create rescue dir");
    fs::create_dir_all(work_root.join("data/logs")).expect("create log dir");

    let sample_wpl = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docker/default_setting/models/wpl/parse.wpl"),
    )
    .expect("read sample wpl");
    write_file(work_root, "models/wpl/parse.wpl", &sample_wpl);

    let sample_oml = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/models/oml/example.oml"),
    )
    .expect("read sample oml");
    write_file(work_root, "models/oml/example.oml", &sample_oml);

    write_file(
        work_root,
        "connectors/source.d/00-file_src.toml",
        r#"[[connectors]]
allow_override = ["base", "file", "encode"]
id = "file_src"
type = "file"

[connectors.params]
base = "./data/in_dat"
encode = "text"
file = "gen.dat"
"#,
    );
    write_file(
        work_root,
        "connectors/sink.d/01-file_json_sink.toml",
        r#"[[connectors]]
allow_override = ["base", "file"]
id = "file_json_sink"
type = "file"

[connectors.params]
base = "./data/out_dat"
file = "default.json"
fmt = "json"
"#,
    );
    write_file(
        work_root,
        "connectors/source.d/02-tcp_src.toml",
        r#"[[connectors]]
allow_override = ["addr", "port", "framing", "tcp_recv_bytes", "instances"]
id = "tcp_src"
type = "tcp"

[connectors.params]
addr = "0.0.0.0"
framing = "auto"
instances = 1
port = 9000
tcp_recv_bytes = 256000
"#,
    );
    if let Some(sleep_ms) = options.blackhole_sleep_ms {
        write_file(
            work_root,
            "connectors/sink.d/03-blackhole_sink.toml",
            &format!(
                r#"[[connectors]]
id = "blackhole_sink"
type = "blackhole"

[connectors.params]
sleep_ms = {sleep_ms}
"#
            ),
        );
    }
    write_file(
        work_root,
        "topology/sources/wpsrc.toml",
        &format!(
            r#"[[sources]]
key = "tcp_1"
enable = true
connect = "tcp_src"
tags = []

[sources.params]
addr = "127.0.0.1"
port = {port}
framing = "line"
instances = 1
"#,
            port = source_bind.port(),
        ),
    );
    write_file(
        work_root,
        "topology/sinks/defaults.toml",
        r#"version = "2.0"

[defaults]
tags = ["env:test"]

[defaults.expect]
basis = "total_input"
mode = "warn"
"#,
    );
    for (name, file) in [
        ("default", "default.json"),
        ("error", "error.json"),
        ("miss", "miss.json"),
        ("monitor", "monitor.json"),
        ("residue", "residue.json"),
    ] {
        write_file(
            work_root,
            &format!("topology/sinks/infra.d/{}.toml", name),
            &format!(
                r#"version = "2.0"

[sink_group]
name = "{name}"

[[sink_group.sinks]]
connect = "file_json_sink"

[sink_group.sinks.params]
file = "{file}"
"#
            ),
        );
    }
    write_file(
        work_root,
        "topology/sinks/business.d/demo.toml",
        &format!(
            r#"version = "2.0"

[sink_group]
name = "demo"
oml = ["*"]
tags = ["biz:demo"]

[[sink_group.sinks]]
name = "json"
connect = "{sink_id}"
tags = ["sink:json"]

[sink_group.sinks.params]
{sink_params}
"#,
            sink_id = if options.blackhole_sleep_ms.is_some() {
                "blackhole_sink"
            } else {
                "file_json_sink"
            },
            sink_params = if options.blackhole_sleep_ms.is_some() {
                ""
            } else {
                r#"file = "demo.json""#
            }
        ),
    );

    let conf = format!(
        r#"version = "1.0"
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
level = "warn,ctrl=info,launch=info,source=info,sink=info,stat=info,runtime=warn,oml=warn,wpl=warn,klib=warn,orion_error=error,orion_sens=warn"
output = "File"

[log_conf.file]
path = "./data/logs/"

[stat]
pick = []
parse = []
sink = []

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
token_file = "runtime/admin_api.token"
"#,
    );
    write_file(work_root, "conf/wparse.toml", &conf);

    token_path
}

fn spawn_wparse(work_root: &Path, subcmd: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_wparse"))
        .current_dir(work_root)
        .arg(subcmd)
        .arg("--work-root")
        .arg(work_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn wparse")
}

fn run_wproj(work_root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wproj"))
        .current_dir(work_root)
        .args(args)
        .output()
        .expect("run wproj")
}

async fn wait_until_ready(
    child: &mut Child,
    work_root: &Path,
    base_url: &str,
    token: &str,
    timeout: Duration,
) -> Value {
    let client = reqwest::Client::new();
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("query child status") {
            let output = collect_output(child);
            let log_dump = fs::read_to_string(work_root.join("data/logs/wparse.log"))
                .unwrap_or_else(|_| "<missing wparse.log>".to_string());
            panic!(
                "daemon exited before admin API became ready: status={} output=\n{}\nlog=\n{}",
                status, output, log_dump
            );
        }

        match client
            .get(format!("{}/admin/v1/runtime/status", base_url))
            .bearer_auth(token)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await.expect("decode status response");
                if body["accepting_commands"] == true {
                    return body;
                }
            }
            Ok(_) | Err(_) => {}
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let output = collect_output(child);
            let log_dump = fs::read_to_string(work_root.join("data/logs/wparse.log"))
                .unwrap_or_else(|_| "<missing wparse.log>".to_string());
            panic!(
                "timed out waiting for daemon admin API readiness; child output=\n{}\nlog=\n{}",
                output, log_dump
            );
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn wait_until_reload_finished(
    base_url: &str,
    token: &str,
    request_id: &str,
    timeout: Duration,
) -> Value {
    let client = reqwest::Client::new();
    let deadline = Instant::now() + timeout;
    loop {
        let resp = client
            .get(format!("{}/admin/v1/runtime/status", base_url))
            .bearer_auth(token)
            .send()
            .await
            .expect("send status request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.expect("decode status response");
        if body["last_reload_request_id"] == request_id && body["reloading"] == false {
            return body;
        }

        if Instant::now() >= deadline {
            panic!(
                "timed out waiting for reload {} to finish, last status={}",
                request_id, body
            );
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn wait_until_reloading(base_url: &str, token: &str, timeout: Duration) -> Value {
    let client = reqwest::Client::new();
    let deadline = Instant::now() + timeout;
    loop {
        let resp = client
            .get(format!("{}/admin/v1/runtime/status", base_url))
            .bearer_auth(token)
            .send()
            .await
            .expect("send status request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.expect("decode status response");
        if body["reloading"] == true {
            return body;
        }

        if Instant::now() >= deadline {
            panic!("timed out waiting for reloading state, last status={}", body);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn send_tcp_line(addr: SocketAddr, line: &str) {
    let mut stream = std::net::TcpStream::connect(addr).expect("connect tcp source");
    stream
        .write_all(line.as_bytes())
        .expect("write tcp source payload");
    stream.flush().expect("flush tcp source payload");
}

fn shutdown_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn collect_output(child: &mut Child) -> String {
    let mut output = String::new();
    if let Some(stdout) = child.stdout.as_mut() {
        let _ = stdout.read_to_string(&mut output);
    }
    if let Some(stderr) = child.stderr.as_mut() {
        let _ = stderr.read_to_string(&mut output);
    }
    output
}

fn read_wparse_log(work_root: &Path) -> String {
    fs::read_to_string(work_root.join("data/logs/wparse.log"))
        .unwrap_or_else(|_| "<missing wparse.log>".to_string())
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_status_and_reload_work_end_to_end() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture(work_root, bind, source_bind);
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    let ready =
        wait_until_ready(&mut child, work_root, &base_url, "test-token", Duration::from_secs(20))
            .await;
    assert_eq!(ready["reloading"], false);

    let client = reqwest::Client::new();
    let reload = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-1")
        .json(&serde_json::json!({
            "wait": true,
            "timeout_ms": 15000,
            "reason": "integration test reload"
        }))
        .send()
        .await
        .expect("send reload request");

    let status = reload.status();
    let body: Value = reload.json().await.expect("decode reload response");
    assert_eq!(status, StatusCode::OK, "reload response body: {}", body);
    assert_eq!(body["accepted"], true);
    assert_eq!(body["result"], "reload_done");

    let after = client
        .get(format!("{}/admin/v1/runtime/status", base_url))
        .bearer_auth("test-token")
        .send()
        .await
        .expect("send status after reload");
    assert_eq!(after.status(), StatusCode::OK);
    let after_body: Value = after.json().await.expect("decode status after reload");
    assert_eq!(after_body["last_reload_request_id"], "integration-reload-1");
    assert_eq!(after_body["last_reload_result"], "reload_done");
    let log_dump = read_wparse_log(work_root);
    assert!(
        log_dump.contains("admin api reload accepted request_id=integration-reload-1"),
        "expected accepted reload log, got:\n{}",
        log_dump
    );
    assert!(
        log_dump.contains("runtime reload P0 start request_id=integration-reload-1"),
        "expected runtime reload start log, got:\n{}",
        log_dump
    );
    assert!(
        log_dump.contains("runtime reload P0 done request_id=integration-reload-1"),
        "expected runtime reload done log, got:\n{}",
        log_dump
    );
    assert!(
        log_dump.contains("runtime command finished request_id=integration-reload-1 command=LoadModel"),
        "expected runtime command completion log, got:\n{}",
        log_dump
    );

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_rejects_wrong_bearer_token() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture(work_root, bind, source_bind);
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(&mut child, work_root, &base_url, "test-token", Duration::from_secs(20))
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/admin/v1/runtime/status", base_url))
        .bearer_auth("wrong-token")
        .send()
        .await
        .expect("send unauthorized request");

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.expect("decode unauthorized response");
    assert_eq!(body["accepted"], false);
    assert_eq!(body["result"], "unauthorized");

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_reload_wait_false_returns_accepted_and_finishes_async() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture(work_root, bind, source_bind);
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(&mut child, work_root, &base_url, "test-token", Duration::from_secs(20))
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-async")
        .json(&serde_json::json!({
            "wait": false,
            "timeout_ms": 15000,
            "reason": "async reload integration test"
        }))
        .send()
        .await
        .expect("send async reload request");

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.expect("decode async reload response");
    assert_eq!(body["accepted"], true);
    assert_eq!(body["result"], "running");

    let final_status = wait_until_reload_finished(
        &base_url,
        "test-token",
        "integration-reload-async",
        Duration::from_secs(20),
    )
    .await;
    assert_eq!(final_status["last_reload_result"], "reload_done");

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_rejects_parallel_reload_with_conflict() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture_with_options(
        work_root,
        bind,
        source_bind,
        FixtureOptions {
            blackhole_sleep_ms: Some(3_000),
        },
    );
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(&mut child, work_root, &base_url, "test-token", Duration::from_secs(20))
        .await;

    send_tcp_line(source_bind, "parallel reload test\n");
    thread::sleep(Duration::from_millis(200));

    let client = reqwest::Client::new();
    let first_client = client.clone();
    let first_base = base_url.clone();
    let first = tokio::spawn(async move {
        first_client
            .post(format!("{}/admin/v1/reloads/model", first_base))
            .bearer_auth("test-token")
            .header("X-Request-Id", "integration-reload-busy-1")
            .json(&serde_json::json!({
                "wait": true,
                "timeout_ms": 15000,
                "reason": "busy reload integration test"
            }))
            .send()
            .await
            .expect("send first reload request")
    });

    wait_until_reloading(&base_url, "test-token", Duration::from_secs(5)).await;

    let second = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-busy-2")
        .json(&serde_json::json!({
            "wait": false,
            "timeout_ms": 15000,
            "reason": "parallel reload integration test"
        }))
        .send()
        .await
        .expect("send second reload request");

    assert_eq!(second.status(), StatusCode::CONFLICT);
    let second_body: Value = second.json().await.expect("decode conflict response");
    assert_eq!(second_body["accepted"], false);
    assert_eq!(second_body["result"], "reload_in_progress");

    let first = first.await.expect("join first reload");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body: Value = first.json().await.expect("decode first reload response");
    assert_eq!(first_body["result"], "reload_done");

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_reports_force_replace_when_drain_times_out() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture_with_options(
        work_root,
        bind,
        source_bind,
        FixtureOptions {
            blackhole_sleep_ms: Some(15_000),
        },
    );
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(&mut child, work_root, &base_url, "test-token", Duration::from_secs(20))
        .await;

    send_tcp_line(source_bind, "force replace test\n");
    thread::sleep(Duration::from_millis(200));

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-force")
        .json(&serde_json::json!({
            "wait": true,
            "timeout_ms": 20000,
            "reason": "force replace integration test"
        }))
        .send()
        .await
        .expect("send force replace reload request");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("decode force replace response");
    assert_eq!(body["accepted"], true);
    assert_eq!(body["result"], "reload_done");
    assert_eq!(body["force_replaced"], true);
    assert_eq!(
        body["warning"],
        "graceful drain timed out, fallback to force replace"
    );
    let log_dump = read_wparse_log(work_root);
    assert!(
        log_dump.contains(
            "runtime reload P0 graceful drain timed out or failed request_id=integration-reload-force fallback_to_force_replace"
        ),
        "expected runtime force-replace fallback log, got:\n{}",
        log_dump
    );
    assert!(
        log_dump.contains(
            "runtime reload P0 done request_id=integration-reload-force force_replaced=true"
        ),
        "expected runtime force-replace completion log, got:\n{}",
        log_dump
    );

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn wproj_engine_status_and_reload_work_against_admin_api() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture(work_root, bind, source_bind);
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(&mut child, work_root, &base_url, "test-token", Duration::from_secs(20))
        .await;

    let status = run_wproj(
        work_root,
        &["engine", "status", "--work-root", work_root.to_str().expect("work root utf8"), "--json"],
    );
    assert!(
        status.status.success(),
        "wproj engine status failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    let status_body: Value =
        serde_json::from_slice(&status.stdout).expect("decode wproj status JSON");
    assert_eq!(status_body["accepting_commands"], true);

    let reload = run_wproj(
        work_root,
        &[
            "engine",
            "reload",
            "--work-root",
            work_root.to_str().expect("work root utf8"),
            "--request-id",
            "cli-reload-1",
            "--reason",
            "cli reload integration test",
            "--json",
        ],
    );
    assert!(
        reload.status.success(),
        "wproj engine reload failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&reload.stdout),
        String::from_utf8_lossy(&reload.stderr)
    );
    let reload_body: Value =
        serde_json::from_slice(&reload.stdout).expect("decode wproj reload JSON");
    assert_eq!(reload_body["accepted"], true);
    assert_eq!(reload_body["result"], "reload_done");

    let final_status =
        wait_until_reload_finished(&base_url, "test-token", "cli-reload-1", Duration::from_secs(20))
            .await;
    assert_eq!(final_status["last_reload_result"], "reload_done");

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn batch_mode_does_not_expose_admin_http_service() {
    let temp = tempdir().expect("tempdir");
    let work_root = temp.path();

    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let _token_path = write_fixture(work_root, bind, source_bind);
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "batch");

    thread::sleep(Duration::from_secs(1));

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/admin/v1/runtime/status", base_url))
        .bearer_auth("test-token")
        .send()
        .await;

    shutdown_child(&mut child);

    assert!(resp.is_err(), "batch mode should not expose admin HTTP");
}
