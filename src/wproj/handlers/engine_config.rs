use std::path::{Path, PathBuf};

use orion_error::{conversion::ToStructError, ErrorWrapAs, UvsFrom};
use orion_variate::{EnvDict, EnvEvaluable};
use wp_config::engine::EngineConfig;
use wp_error::run_error::{RunReason, RunResult};

pub fn load_resolved_engine_config(
    work_root: impl AsRef<Path>,
    dict: &EnvDict,
) -> RunResult<EngineConfig> {
    let work_root = resolve_work_root_path(work_root.as_ref())?;
    EngineConfig::load(&work_root, dict)
        .wrap_as(RunReason::from_conf(), "load engine config failed")
        .map(|conf| conf.env_eval(dict).conf_absolutize(&work_root))
}

fn resolve_work_root_path(path: &Path) -> RunResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail("resolve current dir failed")
                .with_source(e)
        })
}
