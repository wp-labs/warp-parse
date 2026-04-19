use orion_error::ErrorOweSource;
use serde::Serialize;
use wp_error::run_error::RunResult;

pub fn print_json<T: Serialize>(value: &T) -> RunResult<()> {
    let s = serde_json::to_string_pretty(value).owe_conf_source()?;
    println!("{}", s);
    Ok(())
}
