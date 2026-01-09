use anyhow::Result;
use orion_variate::EnvDict;
use wp_proj::wpgen::clean_wpgen_output_file;

/// 清理 wpgen 生成数据
pub async fn clean(work_root: &str, conf_name: &str, local: bool, dict: &EnvDict) -> Result<()> {
    let rep = clean_wpgen_output_file(work_root, conf_name, local, dict)?;
    if let Some(p) = rep.path {
        if rep.cleaned {
            println!("wpgen: cleaned {}", p);
        } else if rep.existed {
            eprintln!("wpgen: failed to clean {}", p);
        } else {
            println!("wpgen: nothing to clean (not found): {}", p);
        }
    }
    Ok(())
}
