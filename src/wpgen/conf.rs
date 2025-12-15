use anyhow::Result;
use wp_proj::wpgen::{gen_conf_check, gen_conf_clean, gen_conf_init};

pub async fn init(work_root: &str) -> Result<()> {
    gen_conf_init(work_root)?;
    Ok(())
}

pub async fn clean(work_root: &str) -> Result<()> {
    gen_conf_clean(work_root)?;
    Ok(())
}

pub async fn check(work_root: &str) -> Result<()> {
    match gen_conf_check(work_root) {
        Ok(_) => {
            println!("config file check ok!");
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(e.to_string())),
    }
}
