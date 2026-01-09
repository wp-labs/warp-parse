use orion_conf::{ToStructError, UvsConfFrom};
use orion_sec::load_sec_dict_by;
use orion_variate::EnvDict;
use wp_error::{RunReason, RunResult};

// Shared library module for warp-parse
pub mod feats;
pub const SEK_KEY_FILE: &str = "sec_key.toml";
pub const WP_DOT_DIR: &str = ".warp_parse";

pub fn load_sec_dict() -> RunResult<EnvDict> {
    load_sec_dict_by(WP_DOT_DIR, SEK_KEY_FILE, orion_sec::SecFileFmt::Toml)
        .map_err(|e| RunReason::from_conf(format!("{}", e)).to_err())
}
