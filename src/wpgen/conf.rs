use orion_variate::EnvDict;
use warp_parse::compat::ErrorConv;
use wp_error::RunResult;
use wp_proj::wpgen::{gen_conf_check, gen_conf_clean, gen_conf_init};

pub async fn init(work_root: &str) -> RunResult<()> {
    gen_conf_init(work_root).conv_err()?;
    Ok(())
}

pub async fn clean(work_root: &str) -> RunResult<()> {
    gen_conf_clean(work_root).conv_err()?;
    Ok(())
}

pub async fn check(work_root: &str, dict: &EnvDict) -> RunResult<()> {
    gen_conf_check(work_root, dict).conv_err()?;
    println!("config file check ok!");
    Ok(())
}
