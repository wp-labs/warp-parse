use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use git2::{Repository, Signature};
use reqwest::StatusCode;
use serde_json::Value;
use serial_test::serial;
use tempfile::tempdir;

#[derive(Clone, Copy, Default)]
struct FixtureOptions {
    blackhole_sleep_ms: Option<u64>,
    reload_timeout_ms: Option<u64>,
    broken_runtime_release: bool,
    invalid_conf_release: bool,
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
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docker/default_setting/models/oml/example.oml"),
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
reload_timeout_ms = {reload_timeout_ms}

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
        reload_timeout_ms = options.reload_timeout_ms.unwrap_or(10_000),
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

fn run_wproj_from_cwd(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wproj"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run wproj from custom cwd")
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
            panic!(
                "timed out waiting for reloading state, last status={}",
                body
            );
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

fn append_project_remote_conf(work_root: &Path, repo_url: &str, init_version: &str) {
    let conf_path = work_root.join("conf/wparse.toml");
    let mut conf = fs::read_to_string(&conf_path).expect("read wparse.toml");
    conf.push_str(&format!(
        r#"

[project_remote]
enabled = true
repo = "{repo_url}"
init_version = "{init_version}"
"#
    ));
    fs::write(conf_path, conf).expect("write project_remote config");
}

fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().expect("open index");
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .expect("add all");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    let sig = Signature::now("warp-parse-test", "warp-parse@test.local").expect("signature");
    let parent = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    match parent.as_ref() {
        Some(parent) => {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[parent])
                .expect("commit with parent");
        }
        None => {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .expect("initial commit");
        }
    }
}

fn tag_head(repo: &Repository, tag: &str) {
    let obj = repo
        .head()
        .expect("head")
        .peel(git2::ObjectType::Commit)
        .expect("peel head");
    repo.tag_lightweight(tag, &obj, false)
        .expect("create lightweight tag");
}

fn create_remote_project_repo(bind: SocketAddr, source_bind: SocketAddr) -> tempfile::TempDir {
    create_remote_project_repo_with_options(bind, source_bind, FixtureOptions::default())
}

fn create_remote_project_repo_with_options(
    bind: SocketAddr,
    source_bind: SocketAddr,
    options: FixtureOptions,
) -> tempfile::TempDir {
    let temp = tempdir().expect("tempdir");
    write_fixture_with_options(temp.path(), bind, source_bind, options);
    append_project_remote_conf(
        temp.path(),
        temp.path().to_str().expect("repo path utf8"),
        "1.4.2",
    );
    write_file(temp.path(), "models/version.txt", "1.4.2\n");
    let repo = Repository::init(temp.path()).expect("init remote repo");
    commit_all(&repo, "release 1.4.2");
    tag_head(&repo, "v1.4.2");

    write_file(temp.path(), "models/version.txt", "1.4.3\n");
    if options.broken_runtime_release {
        write_file(
            temp.path(),
            "topology/sinks/business.d/demo.toml",
            "this is not valid toml\n",
        );
    }
    if options.invalid_conf_release {
        write_file(temp.path(), "conf/wparse.toml", "this is not valid toml\n");
    }
    commit_all(&repo, "release 1.4.3");
    tag_head(&repo, "v1.4.3");

    temp
}

fn clone_project_repo(remote_path: &Path) -> tempfile::TempDir {
    let temp = tempdir().expect("tempdir");
    Repository::clone(remote_path.to_str().expect("remote path utf8"), temp.path())
        .expect("clone remote repo");
    let token_path = temp.path().join("runtime/admin_api.token");
    let mut perms = fs::metadata(&token_path)
        .expect("stat cloned token")
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&token_path, perms).expect("chmod cloned token");
    temp
}

fn checkout_tag(work_root: &Path, tag: &str) {
    let repo = Repository::open(work_root).expect("open repo");
    let obj = repo
        .revparse_single(&format!("refs/tags/{tag}"))
        .expect("find tag");
    let commit = obj.peel_to_commit().expect("peel commit");
    repo.checkout_tree(
        commit.as_object(),
        Some(git2::build::CheckoutBuilder::new().force()),
    )
    .expect("checkout tag");
    repo.set_head_detached(commit.id()).expect("detach head");
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

    let ready = wait_until_ready(
        &mut child,
        work_root,
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;
    assert!(ready["project_version"].is_null());
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
    assert!(after_body["project_version"].is_null());
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
        log_dump
            .contains("runtime command finished request_id=integration-reload-1 command=LoadModel"),
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

    wait_until_ready(
        &mut child,
        work_root,
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
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

    wait_until_ready(
        &mut child,
        work_root,
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
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
            blackhole_sleep_ms: Some(1_000),
            reload_timeout_ms: Some(2_000),
            ..FixtureOptions::default()
        },
    );
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(
        &mut child,
        work_root,
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
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
                "timeout_ms": 10000,
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
            "timeout_ms": 5000,
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
            blackhole_sleep_ms: Some(1_000),
            reload_timeout_ms: Some(300),
            ..FixtureOptions::default()
        },
    );
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(work_root, "daemon");

    wait_until_ready(
        &mut child,
        work_root,
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
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
            "timeout_ms": 5000,
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

    wait_until_ready(
        &mut child,
        work_root,
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;

    let status = run_wproj(
        work_root,
        &[
            "engine",
            "status",
            "--work-root",
            work_root.to_str().expect("work root utf8"),
            "--json",
        ],
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

    let final_status = wait_until_reload_finished(
        &base_url,
        "test-token",
        "cli-reload-1",
        Duration::from_secs(20),
    )
    .await;
    assert_eq!(final_status["last_reload_result"], "reload_done");

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_reload_with_update_moves_project_to_target_version() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo(bind, source_bind);
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(clone.path(), "daemon");

    wait_until_ready(
        &mut child,
        clone.path(),
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-update-1")
        .json(&serde_json::json!({
            "wait": true,
            "update": true,
            "version": "1.4.3",
            "timeout_ms": 15000,
            "reason": "integration reload with update"
        }))
        .send()
        .await
        .expect("send reload update request");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("decode reload update response");
    assert_eq!(body["accepted"], true);
    assert_eq!(body["result"], "reload_done");
    assert_eq!(body["update"], true);
    assert_eq!(body["requested_version"], "1.4.3");
    assert_eq!(body["current_version"], "1.4.3");
    assert_eq!(body["resolved_tag"], "v1.4.3");
    let status_after = client
        .get(format!("{}/admin/v1/runtime/status", base_url))
        .bearer_auth("test-token")
        .send()
        .await
        .expect("send status after update reload");
    assert_eq!(status_after.status(), StatusCode::OK);
    let status_body: Value = status_after.json().await.expect("decode status after update");
    assert_eq!(status_body["project_version"], "1.4.3");
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read updated version marker"),
        "1.4.3\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("runtime/admin_api.token"))
            .expect("read runtime token after update"),
        "test-token\n"
    );

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_reload_with_update_rolls_back_on_reload_failure() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo_with_options(
        bind,
        source_bind,
        FixtureOptions {
            broken_runtime_release: true,
            ..FixtureOptions::default()
        },
    );
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(clone.path(), "daemon");

    wait_until_ready(
        &mut child,
        clone.path(),
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;
    let original_rule_mapping = fs::read(clone.path().join(".run/rule_mapping.dat"))
        .expect("read original runtime rule mapping");

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-update-fail-1")
        .json(&serde_json::json!({
            "wait": true,
            "update": true,
            "version": "1.4.3",
            "timeout_ms": 15000,
            "reason": "integration reload rollback"
        }))
        .send()
        .await
        .expect("send reload update request");

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: Value = resp.json().await.expect("decode reload failure response");
    assert_eq!(body["accepted"], true);
    assert_eq!(body["result"], "reload_failed");
    assert_eq!(body["update"], true);
    assert_eq!(body["requested_version"], "1.4.3");
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read rolled back version marker"),
        "1.4.2\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("runtime/admin_api.token"))
            .expect("read runtime token after rollback"),
        "test-token\n"
    );
    assert_eq!(
        fs::read(clone.path().join(".run/rule_mapping.dat"))
            .expect("read restored runtime rule mapping"),
        original_rule_mapping
    );

    let log_dump = read_wparse_log(clone.path());
    assert!(
        log_dump.contains(
            "admin api project rollback done request_id=integration-reload-update-fail-1"
        ),
        "expected rollback log, got:\n{}",
        log_dump
    );

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn daemon_admin_api_rejects_busy_update_without_changing_project_version() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo_with_options(
        bind,
        source_bind,
        FixtureOptions {
            blackhole_sleep_ms: Some(1_000),
            reload_timeout_ms: Some(2_000),
            ..FixtureOptions::default()
        },
    );
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(clone.path(), "daemon");

    wait_until_ready(
        &mut child,
        clone.path(),
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;

    send_tcp_line(source_bind, "busy update test\n");
    thread::sleep(Duration::from_millis(200));

    let client = reqwest::Client::new();
    let first_client = client.clone();
    let first_base = base_url.clone();
    let first = tokio::spawn(async move {
        first_client
            .post(format!("{}/admin/v1/reloads/model", first_base))
            .bearer_auth("test-token")
            .header("X-Request-Id", "integration-reload-busy-update-1")
            .json(&serde_json::json!({
                "wait": true,
                "timeout_ms": 10000,
                "reason": "busy update first reload"
            }))
            .send()
            .await
            .expect("send first reload request")
    });

    wait_until_reloading(&base_url, "test-token", Duration::from_secs(5)).await;

    let second = client
        .post(format!("{}/admin/v1/reloads/model", base_url))
        .bearer_auth("test-token")
        .header("X-Request-Id", "integration-reload-busy-update-2")
        .json(&serde_json::json!({
            "wait": false,
            "update": true,
            "version": "1.4.3",
            "timeout_ms": 5000,
            "reason": "busy update second reload"
        }))
        .send()
        .await
        .expect("send second reload request");

    assert_eq!(second.status(), StatusCode::CONFLICT);
    let second_body: Value = second.json().await.expect("decode conflict response");
    assert_eq!(second_body["accepted"], false);
    assert_eq!(second_body["result"], "reload_in_progress");
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read version marker after conflict"),
        "1.4.2\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("runtime/admin_api.token"))
            .expect("read runtime token after conflict"),
        "test-token\n"
    );

    let first = first.await.expect("join first reload");
    assert_eq!(first.status(), StatusCode::OK);

    shutdown_child(&mut child);
}

#[tokio::test]
#[serial]
async fn wproj_conf_update_rejects_when_runtime_reload_holds_project_remote_lock() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo_with_options(
        bind,
        source_bind,
        FixtureOptions {
            blackhole_sleep_ms: Some(1_000),
            reload_timeout_ms: Some(2_000),
            ..FixtureOptions::default()
        },
    );
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(clone.path(), "daemon");

    wait_until_ready(
        &mut child,
        clone.path(),
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;

    send_tcp_line(source_bind, "lock during reload test\n");
    thread::sleep(Duration::from_millis(200));

    let client = reqwest::Client::new();
    let first_client = client.clone();
    let first_base = base_url.clone();
    let first = tokio::spawn(async move {
        first_client
            .post(format!("{}/admin/v1/reloads/model", first_base))
            .bearer_auth("test-token")
            .header("X-Request-Id", "integration-reload-lock-1")
            .json(&serde_json::json!({
                "wait": true,
                "timeout_ms": 10000,
                "reason": "hold project remote lock during reload"
            }))
            .send()
            .await
            .expect("send reload request")
    });

    wait_until_reloading(&base_url, "test-token", Duration::from_secs(5)).await;

    let conf_update = run_wproj(
        clone.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
        ],
    );
    assert!(
        !conf_update.status.success(),
        "wproj conf update unexpectedly succeeded during runtime reload: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    let stderr = String::from_utf8_lossy(&conf_update.stderr);
    assert!(
        stderr.contains("project remote update already in progress"),
        "expected project remote lock conflict, got stderr=\n{}",
        stderr
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read version marker after rejected update"),
        "1.4.2\n"
    );

    let first = first.await.expect("join first reload");
    assert_eq!(first.status(), StatusCode::OK);

    shutdown_child(&mut child);
}

#[tokio::test]
async fn wproj_conf_update_rolls_back_when_project_check_fails() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo_with_options(
        bind,
        source_bind,
        FixtureOptions {
            invalid_conf_release: true,
            ..FixtureOptions::default()
        },
    );
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let conf_update = run_wproj(
        clone.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
        ],
    );
    assert!(
        !conf_update.status.success(),
        "wproj conf update unexpectedly succeeded: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read rolled back version marker"),
        "1.4.2\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("runtime/admin_api.token"))
            .expect("read runtime token after rollback"),
        "test-token\n"
    );
}

#[tokio::test]
async fn wproj_conf_update_rolls_back_when_sec_dict_load_fails() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo(bind, source_bind);
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");
    write_file(clone.path(), ".warp_parse/sec_key.toml", "X = \n");
    write_file(clone.path(), ".run/rule_mapping.dat", "original mapping\n");

    let conf_update = run_wproj(
        clone.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
        ],
    );
    assert!(
        !conf_update.status.success(),
        "wproj conf update unexpectedly succeeded: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read rolled back version marker"),
        "1.4.2\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join(".run/rule_mapping.dat"))
            .expect("read restored runtime mapping"),
        "original mapping\n"
    );
}

#[tokio::test]
async fn wproj_conf_update_rolls_back_when_runtime_load_check_fails() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo_with_options(
        bind,
        source_bind,
        FixtureOptions {
            broken_runtime_release: true,
            ..FixtureOptions::default()
        },
    );
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let conf_update = run_wproj(
        clone.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
        ],
    );
    assert!(
        !conf_update.status.success(),
        "wproj conf update unexpectedly succeeded: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read rolled back version marker"),
        "1.4.2\n"
    );
}

#[tokio::test]
async fn wproj_conf_update_uses_work_root_for_sec_dict_lookup() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo(bind, source_bind);
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");
    let outside = tempdir().expect("tempdir");

    let conf_update = run_wproj_from_cwd(
        outside.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
            "--json",
        ],
    );
    assert!(
        conf_update.status.success(),
        "wproj conf update from external cwd failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    let conf_body: Value = serde_json::from_slice(&conf_update.stdout).unwrap_or_else(|err| {
        panic!(
            "decode conf update json failed: {} stdout=\n{}\nstderr=\n{}",
            err,
            String::from_utf8_lossy(&conf_update.stdout),
            String::from_utf8_lossy(&conf_update.stderr)
        )
    });
    assert_eq!(conf_body["current_version"], "1.4.3");
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read updated version marker"),
        "1.4.3\n"
    );
}

#[tokio::test]
async fn wproj_conf_update_preserves_runtime_rule_mapping_after_success() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo(bind, source_bind);
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");
    write_file(clone.path(), ".run/rule_mapping.dat", "original mapping\n");

    let conf_update = run_wproj(
        clone.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
        ],
    );
    assert!(
        conf_update.status.success(),
        "wproj conf update failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read updated version marker"),
        "1.4.3\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join(".run/rule_mapping.dat"))
            .expect("read preserved runtime mapping"),
        "original mapping\n"
    );
}

#[tokio::test]
async fn wproj_conf_update_and_reload_update_flow_work_against_local_project_remote() {
    let bind = reserve_local_addr();
    let source_bind = reserve_local_addr();
    let remote = create_remote_project_repo(bind, source_bind);
    let clone = clone_project_repo(remote.path());
    checkout_tag(clone.path(), "v1.4.2");

    let conf_update = run_wproj(
        clone.path(),
        &[
            "conf",
            "update",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--version",
            "1.4.3",
            "--json",
        ],
    );
    assert!(
        conf_update.status.success(),
        "wproj conf update failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&conf_update.stdout),
        String::from_utf8_lossy(&conf_update.stderr)
    );
    let conf_body: Value = serde_json::from_slice(&conf_update.stdout).unwrap_or_else(|err| {
        panic!(
            "decode conf update json failed: {} stdout=\n{}\nstderr=\n{}",
            err,
            String::from_utf8_lossy(&conf_update.stdout),
            String::from_utf8_lossy(&conf_update.stderr)
        )
    });
    assert_eq!(conf_body["requested_version"], "1.4.3");
    assert_eq!(conf_body["current_version"], "1.4.3");
    assert_eq!(conf_body["resolved_tag"], "v1.4.3");
    assert_eq!(
        fs::read_to_string(clone.path().join("models/version.txt"))
            .expect("read updated version marker"),
        "1.4.3\n"
    );
    assert_eq!(
        fs::read_to_string(clone.path().join("runtime/admin_api.token"))
            .expect("read runtime token after reload"),
        "test-token\n"
    );

    checkout_tag(clone.path(), "v1.4.2");
    let base_url = format!("http://{}", bind);
    let mut child = spawn_wparse(clone.path(), "daemon");

    wait_until_ready(
        &mut child,
        clone.path(),
        &base_url,
        "test-token",
        Duration::from_secs(20),
    )
    .await;

    let reload = run_wproj(
        clone.path(),
        &[
            "engine",
            "reload",
            "--work-root",
            clone.path().to_str().expect("work root utf8"),
            "--request-id",
            "cli-reload-update-1",
            "--update",
            "--version",
            "1.4.3",
            "--json",
        ],
    );
    assert!(
        reload.status.success(),
        "wproj engine reload update failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&reload.stdout),
        String::from_utf8_lossy(&reload.stderr)
    );
    let reload_body: Value =
        serde_json::from_slice(&reload.stdout).expect("decode reload update json");
    assert_eq!(reload_body["accepted"], true);
    assert_eq!(reload_body["result"], "reload_done");
    assert_eq!(reload_body["update"], true);
    assert_eq!(reload_body["requested_version"], "1.4.3");
    assert_eq!(reload_body["current_version"], "1.4.3");
    assert_eq!(reload_body["resolved_tag"], "v1.4.3");

    shutdown_child(&mut child);
}

#[tokio::test]
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
