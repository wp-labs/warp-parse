use orion_error::{ToStructError, UvsConfFrom};
use wp_error::run_error::{RunReason, RunResult};
use wp_proj::{
    connectors::{types::SilentErrKind, LintSeverity},
    project::{Connectors, ProjectPaths},
};

pub fn lint_connectors_silent(work_root: &str) -> RunResult<()> {
    let paths = ProjectPaths::from_root(work_root);
    let connectors = Connectors::new(paths.connectors);
    let mut errors = Vec::new();

    for row in connectors.lint_rows_from_root(work_root) {
        if matches!(row.sev, LintSeverity::Error) {
            let line = match row.silent_err {
                Some(SilentErrKind::BadIdChars) => {
                    format!("{}: bad id chars: {} in {}", row.scope, row.id, row.file)
                }
                Some(SilentErrKind::SourcesIdMustEndSrc) => format!(
                    "{}: id must end with _src: {} in {}",
                    row.scope, row.id, row.file
                ),
                Some(SilentErrKind::SinksIdMustEndSink) => format!(
                    "{}: id must end with _sink: {} in {}",
                    row.scope, row.id, row.file
                ),
                None => format!(
                    "{}: parse failed for {}: {}",
                    row.scope,
                    row.file,
                    row.msg.replace("parse failed: ", ""),
                ),
            };
            errors.push(line);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(RunReason::from_conf(format!(
            "connectors lint failed: {} error(s)\n{}",
            errors.len(),
            errors.join("\n")
        ))
        .to_err())
    }
}
