use std::str::FromStr;

use orion_error::{ToStructError, UvsConfFrom};
use orion_variate::EnvDict;
use wp_error::run_error::{RunReason, RunResult};
use wp_proj::project::{checker, init::PrjScope, WarpProject};

use crate::args::{ProjectCheckArgs, ProjectInitArgs};

pub fn init_project(args: ProjectInitArgs, dict: &EnvDict) -> RunResult<()> {
    WarpProject::init(
        args.work_root.clone(),
        PrjScope::from_str(args.mode.as_str())?,
        dict,
    )
    .map(|_| ())
}

pub fn check_project(args: ProjectCheckArgs, dict: &EnvDict) -> RunResult<()> {
    let project = WarpProject::load(args.work_root.clone(), PrjScope::Normal, dict)?;
    let mut opts = checker::CheckOptions::new(&args.work_root);
    opts.what = args.what.clone();
    opts.console = args.console;
    opts.fail_fast = args.fail_fast;
    opts.json = args.json;
    opts.only_fail = args.only_fail;
    let comps = build_components(&args)?;
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
        return Err(
            RunReason::from_conf(format!("unknown check target: '{}'", args.what)).to_err(),
        );
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

#[cfg(test)]
mod tests {
    use crate::args::ProjectInitArgs;

    use super::*;
    use rand::{rng, RngCore};
    use serial_test::serial;
    use std::time::{SystemTime, UNIX_EPOCH};

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
            &orion_variate::EnvDict::default(),
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
        assert!(std::path::Path::new(&format!("{}/conf/wpgen.toml", work)).exists());
        assert!(
            std::path::Path::new(&format!("{}/connectors/source.d/00-file_src.toml", work))
                .exists()
        );
        println!("DEBUG: Test directory NOT cleaned for debugging: {}", work);
    }
}
