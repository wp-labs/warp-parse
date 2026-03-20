use orion_error::{ToStructError, UvsFrom};
use wp_error::run_error::{RunReason, RunResult};

pub(crate) fn detect_target_triple_v2() -> RunResult<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => Err(RunReason::from_conf()
            .to_err()
            .with_detail(format!("unsupported platform: {}-{}", os, arch))),
    }
}
