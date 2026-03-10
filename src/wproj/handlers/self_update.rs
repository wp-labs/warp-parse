use crate::args::{SelfCheckArgs, UpdateChannel};
use crate::format::print_json;
use orion_error::{ToStructError, UvsFrom};
use serde::Serialize;
use std::path::PathBuf;
use wp_error::run_error::{RunReason, RunResult};
use wp_self_update::{check_updates, SelfCheckRequest, VersionRelation};

#[derive(Debug, Serialize)]
struct SelfCheckReport {
    channel: String,
    branch: String,
    source: String,
    manifest_format: String,
    current_version: String,
    latest_version: String,
    update_available: bool,
    platform_key: String,
    artifact: String,
    sha256: String,
}

pub async fn run_self_check(args: SelfCheckArgs) -> RunResult<()> {
    let request = SelfCheckRequest {
        branch: warp_parse::build::BRANCH.to_string(),
        current_version: warp_parse::build::PKG_VERSION.to_string(),
        channel: args.channel.map(convert_channel),
        updates_base_url: args.updates_base_url,
        updates_root: args.updates_root.map(PathBuf::from),
    };

    let outcome = check_updates(request)
        .await
        .map_err(|e| RunReason::from_conf().to_err().with_detail(e.to_string()))?;
    let relation = outcome.relation;
    let update_available = outcome.update_available();

    let report = SelfCheckReport {
        channel: outcome.channel.as_str().to_string(),
        branch: outcome.branch,
        source: outcome.source,
        manifest_format: "v2".to_string(),
        current_version: outcome.current_version,
        latest_version: outcome.latest_version,
        update_available,
        platform_key: outcome.platform_key,
        artifact: outcome.artifact,
        sha256: outcome.sha256,
    };

    if args.json {
        return print_json(&report);
    }

    println!("Self-check result");
    println!("  Channel  : {}", render_channel(&report.channel));
    println!("  Branch   : {}", report.branch);
    println!("  Manifest : {}", report.source);
    println!("  Target   : {}", report.platform_key);
    println!("  Artifact : {}", report.artifact);
    println!("  SHA256   : {}", report.sha256);
    println!("  Current  : {}", report.current_version);
    println!(
        "  Latest   : {}",
        render_latest_version(&report.latest_version, relation)
    );
    println!("  Status   : {}", relation_message(relation));

    Ok(())
}

fn convert_channel(channel: UpdateChannel) -> wp_self_update::UpdateChannel {
    match channel.as_str() {
        "stable" => wp_self_update::UpdateChannel::Stable,
        "beta" => wp_self_update::UpdateChannel::Beta,
        "alpha" => wp_self_update::UpdateChannel::Alpha,
        _ => wp_self_update::UpdateChannel::Stable,
    }
}

fn render_channel(channel: &str) -> String {
    if !should_use_color() {
        return channel.to_string();
    }
    let code = match channel {
        "stable" => "32", // green
        "beta" => "33",   // yellow
        "alpha" => "35",  // magenta
        _ => return channel.to_string(),
    };
    format!("\x1b[{}m{}\x1b[0m", code, channel)
}

fn should_use_color() -> bool {
    match std::env::var("TERM") {
        Ok(term) => term != "dumb",
        Err(_) => true,
    }
}

fn render_latest_version(latest: &str, relation: VersionRelation) -> String {
    render_latest_version_with_color(latest, relation, should_use_color())
}

fn render_latest_version_with_color(
    latest: &str,
    relation: VersionRelation,
    use_color: bool,
) -> String {
    if use_color {
        if relation == VersionRelation::UpdateAvailable {
            return format!("\x1b[1;92m{}\x1b[0m", latest);
        }
        if relation == VersionRelation::AheadOfChannel {
            return format!("\x1b[90m{}\x1b[0m", latest);
        }
    }
    latest.to_string()
}

fn relation_message(relation: VersionRelation) -> &'static str {
    match relation {
        VersionRelation::UpdateAvailable => "update available",
        VersionRelation::UpToDate => "up-to-date",
        VersionRelation::AheadOfChannel => "ahead of channel manifest",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_channel_unknown_passthrough() {
        assert_eq!(render_channel("dev"), "dev");
    }

    #[test]
    fn render_latest_version_ahead_is_dimmed() {
        assert_eq!(
            render_latest_version_with_color("0.15.3", VersionRelation::AheadOfChannel, true),
            "\u{1b}[90m0.15.3\u{1b}[0m"
        );
    }

    #[test]
    fn render_latest_version_not_ahead_no_dim() {
        assert_eq!(
            render_latest_version_with_color("0.19.0", VersionRelation::UpToDate, true),
            "0.19.0"
        );
    }

    #[test]
    fn render_latest_version_update_available_is_bright() {
        assert_eq!(
            render_latest_version_with_color("0.20.0", VersionRelation::UpdateAvailable, true),
            "\u{1b}[1;92m0.20.0\u{1b}[0m"
        );
    }
}
