use std::fs;
use std::path::Path;
use std::str::FromStr;

use orion_error::{ToStructError, UvsFrom};
use orion_variate::EnvDict;
use wp_error::run_error::{RunReason, RunResult};
use wp_proj::project::{checker, init::PrjScope, WarpProject};

use crate::args::{ProjectCheckArgs, ProjectInitArgs};

pub fn init_project(args: ProjectInitArgs, dict: &EnvDict) -> RunResult<()> {
    WarpProject::init(
        args.work_root.clone(),
        PrjScope::from_str(args.mode.as_str())?,
        dict,
    )?;
    ensure_admin_api_config_block(Path::new(&args.work_root))
}

pub fn check_project(args: ProjectCheckArgs, dict: &EnvDict) -> RunResult<()> {
    let comps = build_components(&args)?;
    check_project_components(
        &args.work_root,
        comps,
        args.console,
        args.fail_fast,
        args.json,
        args.only_fail,
        dict,
    )
}

pub fn check_project_components(
    work_root: &str,
    comps: checker::CheckComponents,
    console: bool,
    fail_fast: bool,
    json: bool,
    only_fail: bool,
    dict: &EnvDict,
) -> RunResult<()> {
    let project = WarpProject::load(work_root.to_string(), PrjScope::Normal, dict)?;
    let mut opts = checker::CheckOptions::new(work_root);
    opts.console = console;
    opts.fail_fast = fail_fast;
    opts.json = json;
    opts.only_fail = only_fail;
    checker::check_with(&project, &opts, &comps, dict)
}

fn build_components(args: &ProjectCheckArgs) -> RunResult<checker::CheckComponents> {
    let what = args.what.trim();
    if what.is_empty() || what.eq_ignore_ascii_case("all") {
        return Ok(checker::CheckComponents::default());
    }

    let selections: Vec<_> = what
        .split(',')
        .filter_map(|token| parse_component(token.trim()))
        .collect();

    if selections.is_empty() {
        return Err(RunReason::from_conf()
            .to_err()
            .with_detail(format!("unknown check target: '{}'", args.what)));
    }

    Ok(checker::CheckComponents::default().with_only(selections))
}

fn parse_component(token: &str) -> Option<checker::CheckComponent> {
    match token.to_ascii_lowercase().as_str() {
        "conf" | "config" | "engine" => Some(checker::CheckComponent::Engine),
        "sources" | "source" => Some(checker::CheckComponent::Sources),
        "connectors" | "connector" | "conn" => Some(checker::CheckComponent::Connectors),
        "sinks" | "sink" => Some(checker::CheckComponent::Sinks),
        "wpl" | "rules" | "rule" => Some(checker::CheckComponent::Wpl),
        "oml" => Some(checker::CheckComponent::Oml),
        "all" => None,
        _ => None,
    }
}

const DEFAULT_ADMIN_API_BLOCK: &str = r#"
[admin_api]
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
token_file = "runtime/admin_api.token"
"#;

fn ensure_admin_api_config_block(work_root: &Path) -> RunResult<()> {
    let conf_path = work_root.join("conf/wparse.toml");
    if !conf_path.exists() {
        return Ok(());
    }

    let mut conf = fs::read_to_string(&conf_path).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "read {} failed: {}",
            conf_path.display(),
            e
        ))
    })?;
    if conf.contains("[admin_api]") {
        return Ok(());
    }

    if !conf.ends_with('\n') {
        conf.push('\n');
    }
    conf.push_str(DEFAULT_ADMIN_API_BLOCK);

    fs::write(&conf_path, conf).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "write {} failed: {}",
            conf_path.display(),
            e
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::args::ProjectInitArgs;

    use super::*;
    use rand::{rng, RngCore};
    use serial_test::serial;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wp_config::test_support::ForTest;

    fn uniq_tmp_dir() -> String {
        let base = std::path::PathBuf::from("./tmp");
        let _ = std::fs::create_dir_all(&base);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let rnd: u64 = rng().next_u64();
        base.join(format!("wproj_project_{}_{}", ts, rnd))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    #[serial]
    fn wproj_project_init_full_ok() {
        let work = uniq_tmp_dir();
        // run project init (default full)
        println!("DEBUG: Attempting to initialize project at: {}", work);
        println!(
            "DEBUG: Parent directory exists: {}",
            std::path::Path::new(&work)
                .parent()
                .is_some_and(|p| p.exists())
        );

        match init_project(
            ProjectInitArgs {
                work_root: work.clone(),
                mode: "full".into(),
            },
            &orion_variate::EnvDict::test_default(),
        ) {
            Ok(_) => println!("DEBUG: Project init succeeded"),
            Err(e) => {
                println!("DEBUG: Project init failed with error: {:?}", e);
                println!(
                    "DEBUG: Work directory exists after attempt: {}",
                    std::path::Path::new(&work).exists()
                );
                if std::path::Path::new(&work).exists() {
                    println!("DEBUG: Directory contents:");
                    if let Ok(entries) = std::fs::read_dir(&work) {
                        for entry in entries.flatten() {
                            println!("  - {}", entry.path().display());
                        }
                    }
                }
                panic!("project init failed: {:?}", e);
            }
        }
        // verify key files/directories
        println!("DEBUG: Checking files in: {}", work);
        println!(
            "DEBUG: Directory exists: {}",
            std::path::Path::new(&work).exists()
        );

        // List all files in the directory
        if let Ok(entries) = std::fs::read_dir(&work) {
            println!("DEBUG: Directory contents:");
            for entry in entries.flatten() {
                println!("  {}", entry.path().display());
            }
        }

        // Check specific files with detailed info
        let files_to_check = vec![
            "conf/wparse.toml",
            "conf/wpgen.toml",
            "connectors/source.d/00-file_src.toml",
            "connectors/sink.d/01-file_json_sink.toml",
            "topology/sinks/business.d/demo.toml",
            "topology/sources/wpsrc.toml",
            "models/knowledge/knowdb.toml",
        ];

        for file in files_to_check {
            let full_path = format!("{}/{}", work, file);
            let exists = std::path::Path::new(&full_path).exists();
            println!("DEBUG: {} exists: {}", file, exists);
        }

        assert!(std::path::Path::new(&format!("{}/conf/wparse.toml", work)).exists());
        let wparse_conf =
            std::fs::read_to_string(format!("{}/conf/wparse.toml", work)).expect("read wparse");
        assert!(wparse_conf.contains("[admin_api]"));
        assert!(wparse_conf.contains("enabled = false"));
        assert!(wparse_conf.contains("token_file = \"runtime/admin_api.token\""));
        assert!(std::path::Path::new(&format!("{}/conf/wpgen.toml", work)).exists());
        assert!(
            std::path::Path::new(&format!("{}/connectors/source.d/00-file_src.toml", work))
                .exists()
        );
        println!("DEBUG: Test directory NOT cleaned for debugging: {}", work);
    }

    #[test]
    fn ensure_admin_api_block_appends_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        std::fs::create_dir_all(&conf_dir).expect("create conf dir");
        let conf_path = conf_dir.join("wparse.toml");
        std::fs::write(&conf_path, "version = \"1.0\"\n").expect("write conf");

        ensure_admin_api_config_block(temp.path()).expect("append admin_api block");
        ensure_admin_api_config_block(temp.path()).expect("skip duplicate admin_api block");

        let conf = std::fs::read_to_string(conf_path).expect("read conf");
        assert_eq!(conf.matches("[admin_api]").count(), 1);
        assert!(conf.contains("bind = \"127.0.0.1:19090\""));
    }
}
