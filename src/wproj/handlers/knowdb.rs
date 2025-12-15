use orion_conf::ToStructError;
use orion_error::UvsConfFrom;
use std::path::PathBuf;
use wp_error::run_error::{RunReason, RunResult};

use crate::args::{KnowdbCheckArgs, KnowdbCleanArgs, KnowdbInitArgs};

pub fn init_knowdb(a: &KnowdbInitArgs) -> RunResult<()> {
    wp_cli_core::knowdb::init(&a.work_root, a.full)
        .map_err(|e| RunReason::from_conf(e.to_string()).to_err())?;
    println!(
        "wprojknowdb skeleton created under '{}'",
        PathBuf::from(&a.work_root).display()
    );
    Ok(())
}

pub fn check_knowdb(a: &KnowdbCheckArgs) -> RunResult<()> {
    let rep = wp_cli_core::knowdb::check(&a.work_root)
        .map_err(|e| RunReason::from_conf(e.to_string()).to_err())?;
    println!("提示: 按配置顺序加载（[[tables]] 出现顺序）");
    for t in &rep.tables {
        if t.create_ok && t.insert_ok && t.data_ok && t.columns_ok {
            println!("OK   table '{}' -> {}", t.name, t.dir);
        } else {
            eprintln!(
                "FAIL table '{}': create={}, insert={}, data={}, columns={}",
                t.name, t.create_ok, t.insert_ok, t.data_ok, t.columns_ok
            );
        }
    }
    println!(
        "summary: total={}, ok={}, fail={}",
        rep.total, rep.ok, rep.fail
    );
    if rep.fail > 0 {
        return Err(RunReason::from_conf("knowdb check failed").to_err());
    }
    Ok(())
}

pub fn clean_knowdb(a: &KnowdbCleanArgs) -> RunResult<()> {
    let rep = wp_cli_core::knowdb::clean(&a.work_root)
        .map_err(|e| RunReason::from_conf(e.to_string()).to_err())?;

    let wr = PathBuf::from(&a.work_root);
    let models_dir = wr.join("models").join("knowledge");
    if rep.removed_models_dir {
        println!("wprojremoved '{}'", models_dir.display());
    } else if rep.not_found_models {
        println!("wproj'{}' not found (skip)", models_dir.display());
    }
    if rep.removed_authority_cache {
        let auth = wr.join(".run").join("authority.sqlite");
        println!("wproj removed '{}'", auth.display());
    }
    Ok(())
}
