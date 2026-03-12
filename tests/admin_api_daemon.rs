use std::fs;
use std::io::Read;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use serde_json::Value;
use serial_test::serial;
use tempfile::tempdir;
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
        r#"version = "2.0"

[sink_group]
name = "demo"
oml = ["*"]
tags = ["biz:demo"]

[[sink_group.sinks]]
name = "json"
connect = "file_json_sink"
tags = ["sink:json"]

[sink_group.sinks.params]
file = "demo.json"
"#,
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

    let _ = child.kill();
    let _ = child.wait();
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

    let _ = child.kill();
    let _ = child.wait();

    assert!(resp.is_err(), "batch mode should not expose admin HTTP");
}
