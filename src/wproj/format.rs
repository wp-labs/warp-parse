use orion_error::conversion::ToStructError;
use serde::Serialize;
use warp_parse::compat::UvsFrom;
use wp_error::run_error::RunResult;
use wp_error::RunReason;

pub fn print_json<T: Serialize>(value: &T) -> RunResult<()> {
    let s = serde_json::to_string_pretty(value).map_err(|err| {
        RunReason::from_conf()
            .to_err()
            .with_detail("serialize json failed")
            .with_source(err)
    })?;
    println!("{}", s);
    Ok(())
}
