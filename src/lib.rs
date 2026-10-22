use std::path::Path;

use crate::compat::UvsFrom;
use orion_error::conversion::ToStructError;
use orion_sec::load_sec_dict_by;
use orion_variate::EnvDict;
use std::sync::Once;
use wp_error::RunResult;

shadow_rs::shadow!(build);

// Shared library module for warp-parse
pub mod admin_api;
pub mod compat;
pub mod feats;
pub mod project_remote;
pub const SEK_KEY_FILE: &str = "sec_key.toml";
pub const WP_DOT_DIR: &str = ".warp_parse";
/// Site-local overlay at the project root. It stays outside the directories
/// managed by `project_remote`, so remote publishes cannot overwrite it.
pub const LOCAL_CONF_FILE: &str = "wparse.local.toml";

pub fn init_rustls_crypto_provider() {
    static RUSTLS_PROVIDER_ONCE: Once = Once::new();
    RUSTLS_PROVIDER_ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

pub fn load_sec_dict() -> RunResult<EnvDict> {
    load_sec_dict_by(WP_DOT_DIR, SEK_KEY_FILE, orion_sec::SecFileFmt::Toml).map_err(|e| {
        wp_log::warn_ctrl!("load sec dict failed: {}", e);
        wp_error::RunReason::from_conf()
            .to_err()
            .with_detail("load sec dict failed")
            .with_source(e)
    })
}

/// 加载实例级 `.env`（若存在）。
///
/// 查找顺序：`<work_root>/.env` → `<work_root>/.warp_parse/.env`。
/// 已存在的进程环境变量不会被覆盖，保证 `docker -e` / systemd 优先。
pub fn load_instance_env(work_root: &Path) -> RunResult<()> {
    for candidate in [
        work_root.join(".env"),
        work_root.join(WP_DOT_DIR).join(".env"),
    ] {
        if candidate.is_file() {
            dotenvy::from_path(&candidate).map_err(|e| {
                wp_error::RunReason::core_conf()
                    .to_err()
                    .with_detail(format!("load instance env failed: {}", candidate.display()))
                    .with_source(e)
            })?;
            wp_log::info_ctrl!("loaded instance env: {}", candidate.display());
            return Ok(());
        }
    }
    Ok(())
}

pub fn log_build_info_once() {
    static BUILD_INFO_ONCE: Once = Once::new();
    BUILD_INFO_ONCE.call_once(|| {
        wp_log::info_ctrl!(
            "version {} (branch {}, commit {}, built {} via {})",
            build::PKG_VERSION,
            build::BRANCH,
            build::SHORT_COMMIT,
            build::BUILD_TIME_3339,
            build::RUST_VERSION,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_instance_env_reads_dotenv_without_override() {
        let temp = tempfile::tempdir().expect("tempdir");
        let key = "WPARSE_TEST_DOTENV_OWNER";
        fs::write(temp.path().join(".env"), format!("{key}=team-a\n")).expect("write .env");
        unsafe { std::env::remove_var(key) };

        load_instance_env(temp.path()).expect("load instance env");
        assert_eq!(std::env::var(key).as_deref(), Ok("team-a"));

        // 已存在的进程环境变量优先，.env 不覆盖
        unsafe { std::env::set_var(key, "from-process") };
        load_instance_env(temp.path()).expect("load instance env again");
        assert_eq!(std::env::var(key).as_deref(), Ok("from-process"));

        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn load_instance_env_falls_back_to_warp_parse_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path().join(WP_DOT_DIR);
        fs::create_dir_all(&dir).expect("create .warp_parse dir");
        let key = "WPARSE_TEST_DOTENV_FALLBACK";
        fs::write(dir.join(".env"), format!("{key}=fallback\n")).expect("write .env");
        unsafe { std::env::remove_var(key) };

        load_instance_env(temp.path()).expect("load fallback env");
        assert_eq!(std::env::var(key).as_deref(), Ok("fallback"));

        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn load_instance_env_prefers_work_root_dotenv() {
        let temp = tempfile::tempdir().expect("tempdir");
        let key = "WPARSE_TEST_DOTENV_PRECEDENCE";
        fs::write(temp.path().join(".env"), format!("{key}=root\n")).expect("write root .env");
        let dir = temp.path().join(WP_DOT_DIR);
        fs::create_dir_all(&dir).expect("create .warp_parse dir");
        fs::write(dir.join(".env"), format!("{key}=fallback\n")).expect("write fallback .env");
        unsafe { std::env::remove_var(key) };

        load_instance_env(temp.path()).expect("load instance env");
        assert_eq!(std::env::var(key).as_deref(), Ok("root"));

        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn load_instance_env_handles_quoted_values() {
        let temp = tempfile::tempdir().expect("tempdir");
        let key = "WPARSE_TEST_DOTENV_QUOTED";
        fs::write(temp.path().join(".env"), format!("{key}=\"a b\"\n")).expect("write .env");
        unsafe { std::env::remove_var(key) };

        load_instance_env(temp.path()).expect("load instance env");
        assert_eq!(std::env::var(key).as_deref(), Ok("a b"));

        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn load_instance_env_is_noop_when_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        load_instance_env(temp.path()).expect("missing .env is not an error");
    }
}
