use crate::args::{KnowdbCmd, ModelCmd, SelfCmd, StatCmd, ValidateCmd, WProj, WProjCli};
use crate::handlers::rescue::dispatch_rescue_cmd;
use crate::handlers::rule::dispatch_rule_cmd;
use crate::handlers::self_update::{run_self_check, run_self_update};
use crate::handlers::sinks::{list_sinks, show_sink_routes};
use crate::handlers::sources::list_sources_for_cli;
use crate::handlers::stat::{run_combined_stat, run_sink_stat, run_src_stat};
use crate::handlers::validate::run_sink_validation;
use crate::handlers::{data, knowdb, project};
use orion_variate::EnvDict;
use warp_parse::load_sec_dict;
use wp_error::run_error::RunResult;

pub async fn dispatch_cli(cli: WProjCli) -> RunResult<()> {
    match cli.cmd {
        WProj::SelfUpdate(sub) => dispatch_self_cmd(sub).await?,
        other => {
            let dict = load_sec_dict()?;
            match other {
                WProj::Rule(sub) => dispatch_rule_cmd(sub, &dict)?,
                WProj::Init(args) => project::init_project(args, &dict)?,
                WProj::Check(args) => project::check_project(args, &dict)?,
                WProj::Data(sub) => data::dispatch_data_cmd(sub, &dict).await?,
                WProj::Model(sub) => dispatch_model_cmd(sub, &dict)?,
                WProj::Rescue(sub) => dispatch_rescue_cmd(sub)?,
                WProj::SelfUpdate(_) => unreachable!("self command handled above"),
            }
        }
    }
    Ok(())
}

async fn dispatch_self_cmd(cmd: SelfCmd) -> RunResult<()> {
    match cmd {
        SelfCmd::Check(args) => run_self_check(args).await,
        SelfCmd::Update(args) => run_self_update(args).await,
    }
}

fn dispatch_model_cmd(cmd: ModelCmd, dict: &EnvDict) -> RunResult<()> {
    match cmd {
        ModelCmd::Sources(args) => list_sources_for_cli(&args, dict),
        ModelCmd::Sinks(args) => list_sinks(args, dict),
        ModelCmd::Route(args) => show_sink_routes(args, dict),
        ModelCmd::Knowdb(sub) => dispatch_knowdb_cmd(sub, dict),
    }
}

fn dispatch_knowdb_cmd(cmd: KnowdbCmd, dict: &EnvDict) -> RunResult<()> {
    match cmd {
        KnowdbCmd::Init(args) => knowdb::init_knowdb(&args),
        KnowdbCmd::Check(args) => knowdb::check_knowdb(&args, dict),
        KnowdbCmd::Clean(args) => knowdb::clean_knowdb(&args),
    }
}

pub fn dispatch_stat_cmd(sub: StatCmd, dict: &EnvDict) -> RunResult<()> {
    match sub {
        StatCmd::File(a) => run_combined_stat(&a.common, dict),
        StatCmd::SrcFile(a) => run_src_stat(&a.common, dict),
        StatCmd::SinkFile(a) => run_sink_stat(&a.common, dict),
    }
}

pub fn dispatch_validate_cmd(sub: ValidateCmd, dict: &EnvDict) -> RunResult<()> {
    match sub {
        ValidateCmd::SinkFile(args) => run_sink_validation(&args, dict),
    }
}

// No dedicated sink-init command exposed via CLI currently

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{SelfCheckArgs, SelfSourceArgs, WProjCli};
    use serial_test::serial;
    use std::path::PathBuf;

    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn enter(path: &std::path::Path) -> Self {
            let original = std::env::current_dir().expect("read current dir");
            std::env::set_current_dir(path).expect("set current dir");
            Self { original }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    fn platform_key_for_test() -> Option<&'static str> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
            ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
            ("macos", "aarch64") => Some("aarch64-apple-darwin"),
            _ => None,
        }
    }

    #[tokio::test]
    #[serial]
    async fn self_check_does_not_require_sec_key() {
        let Some(platform_key) = platform_key_for_test() else {
            return;
        };

        let temp = tempfile::tempdir().expect("create temp dir");
        let _guard = CwdGuard::enter(temp.path());

        let updates_dir = temp.path().join("updates").join("stable");
        std::fs::create_dir_all(&updates_dir).expect("create updates dir");

        let manifest_path = updates_dir.join("manifest.json");
        let body = format!(
            r#"{{
  "version": "0.19.0",
  "channel": "stable",
  "assets": {{
    "{}": {{
      "url": "https://example.com/warp-parse-v0.19.0-test.tar.gz",
      "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }}
  }}
}}"#,
            platform_key
        );
        std::fs::write(&manifest_path, body).expect("write manifest");

        // intentionally do not create .warp_parse/sec_key.toml
        let cli = WProjCli {
            quiet: true,
            cmd: WProj::SelfUpdate(SelfCmd::Check(SelfCheckArgs {
                source: SelfSourceArgs {
                    channel: crate::args::UpdateChannel::Stable,
                    updates_base_url: "https://raw.githubusercontent.com/wp-labs/wp-install/main"
                        .to_string(),
                    updates_root: Some(".".to_string()),
                    json: true,
                },
            })),
        };

        let result = dispatch_cli(cli).await;
        assert!(result.is_ok(), "self check should not require sec key");
    }
}
