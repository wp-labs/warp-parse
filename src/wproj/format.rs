use orion_error::{ToStructError, UvsFrom};
use serde::Serialize;
use wp_error::run_error::{RunReason, RunResult};

pub fn print_json<T: Serialize>(value: &T) -> RunResult<()> {
    let s = serde_json::to_string_pretty(value)
        .map_err(|e| RunReason::from_conf().to_err().with_detail(e.to_string()))?;
    println!("{}", s);
    Ok(())
}
