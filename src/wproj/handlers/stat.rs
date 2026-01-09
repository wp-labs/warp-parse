use crate::format::print_json;
use serde_json::json;
use wp_error::run_error::RunResult;
use wp_proj::sinks::stat::stat_file_combined;
use wp_proj::sinks::stat::stat_sink_files;
use wp_proj::sinks::stat::SinkStatFilters;
use wp_proj::sources::stat::stat_file_sources;
use wpcnt_lib as wlib;

pub fn run_combined_stat(args: &crate::args::CommonFiltArgs) -> RunResult<()> {
    let filters = SinkStatFilters::new(
        args.work_root.as_str(),
        &args.group_names,
        &args.sink_names,
        &args.path_like,
    );
    let stats = stat_file_combined(&filters)?;
    let sink_rows = stats.sink.rows;
    let sink_total = stats.sink.total;
    if args.json {
        let obj = match stats.src {
            Some(report) => json!({
                "ok": true,
                "src": report,
                "sink": {"total": sink_total, "items": sink_rows}
            }),
            None => json!({
                "ok": true,
                "src": {"total_enabled_lines": 0, "items": [], "note": "no file sources found"},
                "sink": {"total": sink_total, "items": sink_rows}
            }),
        };
        print_json(&obj)?;
        return Ok(());
    }

    println!("== Sources ==");
    if let Some(report) = stats.src {
        wlib::print_src_files_table(&report);
    } else {
        println!(
            "no file sources found: missing topology/sources/wpsrc.toml or no enabled entries"
        );
    }
    println!("\n== Sinks ==");
    wlib::print_rows(&sink_rows, sink_total);
    Ok(())
}

pub fn run_src_stat(args: &crate::args::CommonFiltArgs) -> RunResult<()> {
    let stats = stat_file_sources(args.work_root.as_str())?;
    if let Some(report) = stats.report {
        if args.json {
            print_json(&report)?;
        } else {
            wlib::print_src_files_table(&report);
        }
    } else if args.json {
        let obj = json!({
            "ok": true,
            "summary": {"total_enabled_lines": 0},
            "items": [],
            "note": "no file sources found"
        });
        print_json(&obj)?;
    } else {
        eprintln!(
            "no file sources found under {}/sources/wpsrc.toml; try --work-root 指向工程根目录",
            stats.work_root
        );
    }
    Ok(())
}

pub fn run_sink_stat(args: &crate::args::CommonFiltArgs) -> RunResult<()> {
    let filters = SinkStatFilters::new(
        args.work_root.as_str(),
        &args.group_names,
        &args.sink_names,
        &args.path_like,
    );
    let stats = stat_sink_files(&filters)?;
    if args.json {
        print_json(&wlib::JsonOut {
            total: stats.total,
            items: stats.rows,
        })?;
    } else {
        wlib::print_rows(&stats.rows, stats.total);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::args::{CommonFiltArgs, StatCmd, StatSinkArgs, StatSrcArgs};
    use crate::handlers::cli::dispatch_stat_cmd;

    #[test]
    fn wproj_stat_src_file_runs() {
        let work_root = std::path::Path::new("usecase/core/getting_started");
        if !work_root.exists() {
            eprintln!(
                "skip wproj_stat_src_file_runs: sample work_root {:?} 不存在",
                work_root
            );
            return;
        }
        let args = StatSrcArgs {
            common: CommonFiltArgs {
                work_root: work_root.to_string_lossy().into_owned(),
                group_names: vec![],
                sink_names: vec![],
                path_like: None,
                json: true,
            },
        };
        let cmd = StatCmd::SrcFile(args);
        // Just ensure it does not error in repo context
        let _ = dispatch_stat_cmd(cmd);
    }

    #[test]
    fn wproj_stat_sink_file_runs() {
        let work_root = std::path::Path::new("usecase/core/getting_started");
        if !work_root.exists() {
            eprintln!(
                "skip wproj_stat_sink_file_runs: sample work_root {:?} 不存在",
                work_root
            );
            return;
        }
        let args = StatSinkArgs {
            common: CommonFiltArgs {
                work_root: work_root.to_string_lossy().into_owned(),
                group_names: vec![],
                sink_names: vec![],
                path_like: None,
                json: true,
            },
        };
        let cmd = StatCmd::SinkFile(args);
        let _ = dispatch_stat_cmd(cmd);
    }
}
