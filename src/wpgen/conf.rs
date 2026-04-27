use orion_error::ErrorConv;
use orion_variate::EnvDict;
use wp_error::RunResult;
use wp_proj::wpgen::{gen_conf_check, gen_conf_clean, gen_conf_init};

pub async fn init(work_root: &str) -> RunResult<()> {
    gen_conf_init(work_root).err_conv()?;
    Ok(())
}

pub async fn clean(work_root: &str) -> RunResult<()> {
    gen_conf_clean(work_root).err_conv()?;
    Ok(())
}

pub async fn check(work_root: &str, dict: &EnvDict) -> RunResult<()> {
    gen_conf_check(work_root, dict).err_conv()?;
    println!("config file check ok!");
    Ok(())
}
