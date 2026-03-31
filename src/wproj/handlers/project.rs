use std::fs;
use std::path::Path;
use std::str::FromStr;

use orion_error::{ToStructError, UvsFrom};
use orion_variate::EnvDict;
use wp_error::run_error::{RunReason, RunResult};
use wp_proj::project::{checker, init::PrjScope, WarpProject};

use crate::args::{ProjectCheckArgs, ProjectInitArgs};
use crate::handlers::conf::run_conf_update_from_repo;

pub async fn init_project(args: ProjectInitArgs, dict: &EnvDict) -> RunResult<()> {
    WarpProject::init(
        args.work_root.clone(),
        PrjScope::from_str(args.mode.as_deref().unwrap_or("normal"))?,
        dict,
    )?;
    let remote_repo = args
        .repo
        .as_deref()
        .map(str::trim)
        .filter(|repo| !repo.is_empty());
    if let Some(remote_repo) = remote_repo {
        return run_conf_update_from_repo(&args.work_root, remote_repo, args.version.as_deref())
            .await;
    }

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
    let project = WarpProject::load(work_root, PrjScope::Normal, dict)?;
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
token_file = "${HOME}/.warp_parse/admin_api.token"
"#;

const HOME_ADMIN_API_TOKEN_FILE_LINE: &str = "token_file = \"${HOME}/.warp_parse/admin_api.token\"";
const PROJECT_ADMIN_API_TOKEN_FILE_LINE: &str = "token_file = \"runtime/admin_api.token\"";

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
        if !conf.contains(HOME_ADMIN_API_TOKEN_FILE_LINE)
            && conf.contains(PROJECT_ADMIN_API_TOKEN_FILE_LINE)
        {
            conf = conf.replace(
                PROJECT_ADMIN_API_TOKEN_FILE_LINE,
                HOME_ADMIN_API_TOKEN_FILE_LINE,
            );
            fs::write(&conf_path, conf).map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "write {} failed: {}",
                    conf_path.display(),
                    e
                ))
            })?;
        }
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
    use git2::{Repository, Signature};
    use rand::{rng, RngCore};
    use serial_test::serial;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wp_config::test_support::ForTest;
    use wp_proj::project::init::PrjScope;

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

    struct RemoteFixture {
        _temp: tempfile::TempDir,
        remote_path: PathBuf,
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
            Some(parent) => repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[parent])
                .expect("commit with parent"),
            None => repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .expect("initial commit"),
        };
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

    fn rewrite_project_remote_conf(work_root: &Path, repo_url: &str, init_version: &str) {
        let conf_path = work_root.join("conf/wparse.toml");
        let conf = fs::read_to_string(&conf_path).expect("read wparse conf");
        let conf = conf.replace(
            "[project_remote]\nenabled = false\nrepo = \"\"\ninit_version = \"\"\n",
            &format!(
                "[project_remote]\nenabled = true\nrepo = \"{}\"\ninit_version = \"{}\"\n",
                repo_url, init_version
            ),
        );
        fs::write(&conf_path, conf).expect("write wparse conf");
    }

    fn create_remote_fixture(dict: &EnvDict) -> RemoteFixture {
        let temp = tempfile::tempdir().expect("tempdir");
        WarpProject::init(
            temp.path().to_string_lossy().to_string(),
            PrjScope::Full,
            dict,
        )
        .expect("init remote project");
        ensure_admin_api_config_block(temp.path()).expect("append admin block");

        let repo = Repository::init(temp.path()).expect("init remote repo");
        rewrite_project_remote_conf(
            temp.path(),
            temp.path().to_str().expect("repo path utf8"),
            "1.4.2",
        );
        fs::write(temp.path().join("models/version.txt"), "1.4.2\n").expect("write 1.4.2");
        commit_all(&repo, "release 1.4.2");
        tag_head(&repo, "v1.4.2");

        fs::write(temp.path().join("models/version.txt"), "1.4.3\n").expect("write 1.4.3");
        commit_all(&repo, "release 1.4.3");
        tag_head(&repo, "v1.4.3");
        let remote_path = temp.path().to_path_buf();

        RemoteFixture {
            _temp: temp,
            remote_path,
        }
    }

    fn create_remote_fixture_without_tags(dict: &EnvDict) -> RemoteFixture {
        let temp = tempfile::tempdir().expect("tempdir");
        WarpProject::init(
            temp.path().to_string_lossy().to_string(),
            PrjScope::Full,
            dict,
        )
        .expect("init remote project");
        ensure_admin_api_config_block(temp.path()).expect("append admin block");

        let repo = Repository::init(temp.path()).expect("init remote repo");
        rewrite_project_remote_conf(
            temp.path(),
            temp.path().to_str().expect("repo path utf8"),
            "",
        );
        fs::write(temp.path().join("models/version.txt"), "head\n").expect("write head marker");
        commit_all(&repo, "initial head");
        let remote_path = temp.path().to_path_buf();

        RemoteFixture {
            _temp: temp,
            remote_path,
        }
    }

    async fn run_remote_init_latest_release_case() {
        let dict = orion_variate::EnvDict::test_default();
        let fixture = create_remote_fixture(&dict);
        let work = uniq_tmp_dir();

        let result = init_project(
            ProjectInitArgs {
                work_root: work.clone(),
                mode: None,
                repo: Some(fixture.remote_path.to_string_lossy().to_string()),
                version: None,
            },
            &dict,
        )
        .await;
        result.expect("remote init");

        assert_eq!(
            fs::read_to_string(Path::new(&work).join("models/version.txt")).expect("read version"),
            "1.4.3\n"
        );
    }

    async fn run_remote_init_without_release_tags_case() {
        let dict = orion_variate::EnvDict::test_default();
        let fixture = create_remote_fixture_without_tags(&dict);
        let work = uniq_tmp_dir();

        init_project(
            ProjectInitArgs {
                work_root: work.clone(),
                mode: None,
                repo: Some(fixture.remote_path.to_string_lossy().to_string()),
                version: None,
            },
            &dict,
        )
        .await
        .expect("remote init without tags");

        assert_eq!(
            fs::read_to_string(Path::new(&work).join("models/version.txt")).expect("read version"),
            "head\n"
        );
    }

    #[tokio::test]
    #[serial]
    async fn wproj_project_init_full_ok() {
        let work = uniq_tmp_dir();
        init_project(
            ProjectInitArgs {
                work_root: work.clone(),
                mode: Some("full".into()),
                repo: None,
                version: None,
            },
            &orion_variate::EnvDict::test_default(),
        )
        .await
        .expect("project init");

        assert!(std::path::Path::new(&format!("{}/conf/wparse.toml", work)).exists());
        let wparse_conf =
            std::fs::read_to_string(format!("{}/conf/wparse.toml", work)).expect("read wparse");
        assert!(wparse_conf.contains("[admin_api]"));
        assert!(wparse_conf.contains("enabled = false"));
        assert!(wparse_conf.contains("token_file = \"${HOME}/.warp_parse/admin_api.token\""));
        assert!(std::path::Path::new(&format!("{}/conf/wpgen.toml", work)).exists());
        assert!(
            std::path::Path::new(&format!("{}/connectors/source.d/00-file_src.toml", work))
                .exists()
        );
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

    #[test]
    fn ensure_admin_api_block_normalizes_legacy_token_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let conf_dir = temp.path().join("conf");
        std::fs::create_dir_all(&conf_dir).expect("create conf dir");
        let conf_path = conf_dir.join("wparse.toml");
        std::fs::write(
            &conf_path,
            format!(
                "version = \"1.0\"\n\n[admin_api]\nenabled = false\n\n[admin_api.auth]\nmode = \"bearer_token\"\n{}\n",
                PROJECT_ADMIN_API_TOKEN_FILE_LINE
            ),
        )
        .expect("write conf");

        ensure_admin_api_config_block(temp.path()).expect("normalize admin_api token path");

        let conf = std::fs::read_to_string(conf_path).expect("read conf");
        assert!(conf.contains(HOME_ADMIN_API_TOKEN_FILE_LINE));
        assert!(!conf.contains(PROJECT_ADMIN_API_TOKEN_FILE_LINE));
    }

    #[tokio::test]
    #[serial]
    async fn wproj_project_init_remote_defaults_to_latest_release() {
        let current_exe = std::env::current_exe().expect("current exe");
        let status = std::process::Command::new(current_exe)
            .env("WP_REMOTE_INIT_SUBPROCESS", "1")
            .arg("--exact")
            .arg("handlers::project::tests::remote_init_latest_release_subprocess_helper")
            .arg("--nocapture")
            .status()
            .expect("spawn subprocess");
        assert!(
            status.success(),
            "remote init subprocess failed with status: {}",
            status
        );
    }

    #[tokio::test]
    async fn remote_init_latest_release_subprocess_helper() {
        if std::env::var_os("WP_REMOTE_INIT_SUBPROCESS").is_none() {
            return;
        }
        run_remote_init_latest_release_case().await;
    }

    #[tokio::test]
    #[serial]
    async fn wproj_project_init_remote_defaults_to_remote_head_when_release_tags_are_missing() {
        let current_exe = std::env::current_exe().expect("current exe");
        let status = std::process::Command::new(current_exe)
            .env("WP_REMOTE_INIT_SUBPROCESS", "1")
            .arg("--exact")
            .arg("handlers::project::tests::remote_init_without_release_tags_subprocess_helper")
            .arg("--nocapture")
            .status()
            .expect("spawn subprocess");
        assert!(
            status.success(),
            "remote init subprocess failed with status: {}",
            status
        );
    }

    #[tokio::test]
    async fn remote_init_without_release_tags_subprocess_helper() {
        if std::env::var_os("WP_REMOTE_INIT_SUBPROCESS").is_none() {
            return;
        }
        run_remote_init_without_release_tags_case().await;
    }
}
