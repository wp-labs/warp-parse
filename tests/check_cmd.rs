//! Contract tests for `wparse check`.
//!
//! These exercise the offline config gate end-to-end through the real binary:
//! exit codes, human/JSON output, and the "no writes, no locks" property.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

const WPL: &str = r#"package /nginx/ {
   rule example {
        (ip:sip,2*_,time/clf:recv_time<[,]>,http/request",http/status,digit,chars",http/agent",_")
   }
}
"#;

const OML: &str = r#"name : nginx
rule : /nginx/*
---
* : auto = take() ;
"#;

const CONNECTOR: &str = r#"[[connectors]]
allow_override = ["base", "file", "sync"]
id = "file_json_sink"
type = "file"

[connectors.params]
base = "./data/out_dat"
file = "default.json"
fmt = "json"
sync = false
"#;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
    fs::write(path, body).expect("write fixture");
}

fn fixture() -> TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path();
    write(path, "conf/wparse.toml", "version = \"1.0\"\n");
    write(path, "models/wpl/parse.wpl", WPL);
    write(path, "models/oml/example.oml", OML);
    write(path, "connectors/sink.d/00-file_json_sink.toml", CONNECTOR);
    root
}

fn route(root: &Path, file: &str, group: &str, oml: &str) {
    write_route(root, file, group, &format!("oml = {oml}"));
}

fn route_rule(root: &Path, file: &str, group: &str, rule: &str) {
    write_route(root, file, group, &format!("rule = {rule}"));
}

fn write_route(root: &Path, file: &str, group: &str, matcher_line: &str) {
    let body = format!(
        r#"version = "2.0"

[sink_group]
name = "{group}"
{matcher_line}

[[sink_group.sinks]]
name = "json"
connect = "file_json_sink"

[sink_group.sinks.params]
file = "{group}.jsonl"
"#
    );
    write(root, &format!("topology/sinks/business.d/{file}"), &body);
}

fn check(work_root: &Path, extra: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_wparse"));
    cmd.arg("check").arg("--work-root").arg(work_root);
    cmd.args(extra);
    // Run from a different CWD to prove `--work-root` is authoritative.
    cmd.current_dir(std::env::temp_dir());
    cmd.output().expect("run wparse check")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn duplicate_concrete_oml_fails_and_names_both_files() {
    let f = fixture();
    route(f.path(), "a.toml", "a-metrics", r#"["nginx"]"#);
    route(f.path(), "b.toml", "b-metrics", r#"["nginx"]"#);

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));

    let err = stderr(&out);
    assert!(err.contains("a-metrics"), "{err}");
    assert!(err.contains("b-metrics"), "{err}");
    assert!(err.contains("a.toml"), "{err}");
    assert!(err.contains("b.toml"), "{err}");
    assert!(stdout(&out).is_empty(), "stdout: {}", stdout(&out));
}

#[test]
fn duplicate_star_fails() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["*"]"#);
    route(f.path(), "b.toml", "b", r#"["*"]"#);

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("'*'"), "{}", stderr(&out));
}

#[test]
fn star_plus_concrete_passes_silently() {
    let f = fixture();
    route(f.path(), "a.toml", "log_std", r#"["*"]"#);
    route(f.path(), "b.toml", "log_confused", r#"["nginx"]"#);

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).is_empty(), "stdout: {}", stdout(&out));
    assert!(stderr(&out).is_empty(), "stderr: {}", stderr(&out));
}

#[test]
fn single_group_distinct_matchers_passes_silently() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["nginx"]"#);

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).is_empty());
    assert!(stderr(&out).is_empty());
}

#[test]
fn broken_toml_exits_2_with_file() {
    let f = fixture();
    write(
        f.path(),
        "topology/sinks/business.d/broken.toml",
        "[sink_group\nname = \"x\"\n",
    );

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("broken.toml"), "{}", stderr(&out));
}

#[test]
fn json_is_machine_readable() {
    let f = fixture();
    route(f.path(), "a.toml", "a-metrics", r#"["nginx"]"#);
    route(f.path(), "b.toml", "b-metrics", r#"["nginx"]"#);

    let out = check(f.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(1));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&out)).expect("stdout must be valid JSON");
    assert_eq!(value["ok"], serde_json::json!(false));
    let issues = value["issues"].as_array().expect("issues array");
    let dup = issues
        .iter()
        .find(|i| i["kind"] == "duplicate_oml_matcher")
        .expect("duplicate issue");
    assert_eq!(dup["matcher"], "nginx");
    let groups = dup["groups"].as_array().expect("groups");
    assert_eq!(groups.len(), 2);
    let names: Vec<&str> = groups.iter().map(|g| g["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"a-metrics") && names.contains(&"b-metrics"));
    for group in groups {
        let file = group["file"].as_str().expect("file");
        assert!(file.contains("business.d/"), "{file}");
    }
}

#[test]
fn strict_promotes_warnings() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["does_not_exist"]"#);

    let lenient = check(f.path(), &[]);
    assert_eq!(lenient.status.code(), Some(0));

    let strict = check(f.path(), &["--strict"]);
    assert_eq!(strict.status.code(), Some(1), "stderr: {}", stderr(&strict));
    assert!(stderr(&strict).contains("warning:"), "{}", stderr(&strict));
}

fn snapshot(root: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64, std::time::SystemTime)>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            let meta = entry.metadata().expect("metadata");
            if meta.is_dir() {
                walk(&path, out);
            } else {
                out.push((path, meta.len(), meta.modified().expect("modified")));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

#[test]
fn check_performs_no_writes() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["nginx"]"#);

    let before = snapshot(f.path());
    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let after = snapshot(f.path());

    assert_eq!(before, after, "check must not write to the work root");
    assert!(
        !f.path().join(".run").exists(),
        "check must not create .run/"
    );
    assert!(
        !f.path().join("wparse.pid").exists(),
        "check must not create a pid file"
    );
    assert!(
        !f.path().join(".lock").exists(),
        "check must not create a lock file"
    );
}

#[cfg(unix)]
#[test]
fn check_passes_on_read_only_tree() {
    use std::os::unix::fs::PermissionsExt;

    fn set(root: &Path, dir_mode: u32, file_mode: u32) {
        for entry in fs::read_dir(root).expect("read_dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            let meta = entry.metadata().expect("metadata");
            if meta.is_dir() {
                set(&path, dir_mode, file_mode);
                fs::set_permissions(&path, fs::Permissions::from_mode(dir_mode))
                    .expect("chmod dir");
            } else {
                fs::set_permissions(&path, fs::Permissions::from_mode(file_mode))
                    .expect("chmod file");
            }
        }
    }

    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["nginx"]"#);

    set(f.path(), 0o555, 0o444);
    let out = check(f.path(), &[]);
    // Restore before TempDir cleanup.
    set(f.path(), 0o755, 0o644);

    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

#[test]
fn duplicate_rule_matcher_fails() {
    let f = fixture();
    route_rule(f.path(), "a.toml", "a", r#"["/nginx/*"]"#);
    route_rule(f.path(), "b.toml", "b", r#"["/nginx/*"]"#);

    let out = check(f.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    let kinds: Vec<&str> = value["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .filter_map(|i| i["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"duplicate_rule_matcher"), "{kinds:?}");
}

#[test]
fn overlapping_equal_length_oml_fails_as_binding_conflict() {
    let f = fixture();
    // Different matcher strings, same length, both match model `nginx`.
    route(f.path(), "a.toml", "a", r#"["ngin?"]"#);
    route(f.path(), "b.toml", "b", r#"["n?inx"]"#);

    let out = check(f.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    let kinds: Vec<&str> = value["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .filter_map(|i| i["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"oml_binding_conflict"), "{kinds:?}");
    assert!(
        !kinds.contains(&"duplicate_oml_matcher"),
        "distinct strings must not be reported as duplicates: {kinds:?}"
    );
}

#[test]
fn duplicate_group_name_fails() {
    let f = fixture();
    route(f.path(), "a.toml", "dup", r#"["nginx"]"#);
    route(f.path(), "b.toml", "dup", r#"["other"]"#);

    let out = check(f.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    let issue = value["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .find(|i| i["kind"] == "duplicate_sink_group")
        .expect("duplicate_sink_group issue");
    assert_eq!(issue["groups"].as_array().expect("groups").len(), 2);
}

#[test]
fn json_load_error_is_machine_readable() {
    let f = fixture();
    write(
        f.path(),
        "topology/sinks/business.d/broken.toml",
        "[sink_group\nname = \"x\"\n",
    );

    let out = check(f.path(), &["--json"]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    assert_eq!(value["ok"], serde_json::json!(false));
    let issue = &value["issues"][0];
    assert_eq!(issue["kind"], "config_load_error");
    let names_file = issue["message"]
        .as_str()
        .is_some_and(|m| m.contains("broken.toml"))
        || issue["file"]
            .as_str()
            .is_some_and(|f| f.contains("broken.toml"));
    assert!(names_file, "load error should name the file: {issue}");
}

#[test]
fn strict_json_sets_ok_false() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["does_not_exist"]"#);

    let out = check(f.path(), &["--json", "--strict"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    assert_eq!(value["ok"], serde_json::json!(false));
    assert_eq!(value["summary"]["errors"], serde_json::json!(0));
    assert_eq!(value["summary"]["warnings"], serde_json::json!(1));
}

#[test]
fn env_file_missing_is_json_error() {
    let f = fixture();
    let bad = f.path().join("does-not-exist.env");
    let out = check(
        f.path(),
        &["--json", "--env-file", bad.to_str().expect("utf8")],
    );
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("json");
    assert_eq!(value["issues"][0]["kind"], "config_load_error");
}

#[test]
fn malformed_site_overlay_is_reported() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["nginx"]"#);
    write(
        f.path(),
        "wparse.local.toml",
        "[validate]\ndeny_duplicate_oml = true\ntypo = 1\n",
    );

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("wparse.local.toml"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn missing_engine_config_exits_2() {
    let f = fixture();
    fs::remove_file(f.path().join("conf/wparse.toml")).expect("remove conf");

    let out = check(f.path(), &[]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("wparse.toml"), "{}", stderr(&out));
}

#[test]
fn site_local_overlay_fails_closed_on_duplicate_oml() {
    let f = fixture();
    route(f.path(), "a.toml", "a", r#"["nginx"]"#);
    route(f.path(), "b.toml", "b", r#"["nginx"]"#);
    write(f.path(), "topology/sources/wpsrc.toml", "sources = []\n");
    write(
        f.path(),
        "wparse.local.toml",
        "[validate]\ndeny_duplicate_oml = true\n",
    );

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_wparse"));
    cmd.args(["daemon", "--work-root"]).arg(f.path());
    cmd.current_dir(std::env::temp_dir());
    cmd.stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn wparse daemon");

    // Never let a regression turn this into a hanging test.
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for daemon") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("wparse daemon did not fail closed within 20s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let mut err = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        stderr.read_to_string(&mut err).expect("read stderr");
    }
    assert!(!status.success(), "daemon must fail closed: {err}");
    assert!(err.contains("duplicate oml matcher"), "{err}");
    assert!(err.contains("a.toml") && err.contains("b.toml"), "{err}");
}
