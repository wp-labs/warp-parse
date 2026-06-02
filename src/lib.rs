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
