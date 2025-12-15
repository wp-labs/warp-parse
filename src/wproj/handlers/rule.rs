use orion_error::ErrorOwe;
use wp_error::run_error::RunResult;
use wp_proj::wparse::samples::parse_wpl_samples;

use crate::args::{ParseArgs, RuleCmd};

pub fn dispatch_rule_cmd(sub: RuleCmd) -> RunResult<()> {
    match sub {
        RuleCmd::Parse(args) => run_rule_parse(args),
    }
}

fn run_rule_parse(_args: ParseArgs) -> RunResult<()> {
    wp_log::conf::log_init(&wp_log::conf::LogConf::log_to_console("error")).owe_conf()?;
    parse_wpl_samples("./")
}

#[cfg(test)]
mod tests {
    use crate::format::print_json;

    #[test]
    fn wproj_rule_handlers_json_ok_shape() {
        let obj = serde_json::json!({"ok": true, "action": "verify"});
        // ensure print_json handles simple objects
        let _ = print_json(&obj);
    }
}
