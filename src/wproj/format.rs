use orion_error::{ToStructError, UvsConfFrom};
use serde::Serialize;
use wp_error::run_error::{RunReason, RunResult};

pub fn print_json<T: Serialize>(value: &T) -> RunResult<()> {
    let s = serde_json::to_string_pretty(value)
        .map_err(|e| RunReason::from_conf(e.to_string()).to_err())?;
    println!("{}", s);
    Ok(())
}
