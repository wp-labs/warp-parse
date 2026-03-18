use crate::args::{SelfCheckArgs, SelfSourceArgs, SelfUpdateArgs, UpdateChannel};
use crate::format::print_json;
use orion_error::{ToStructError, UvsFrom};
use std::env;
use wp_self_update::{
    check, compare_versions_str, relation_message, CheckReport, CheckRequest, SourceConfig,
    UpdateChannel as CoreChannel, UpdateProduct, UpdateReport, UpdateRequest, VersionRelation,
};
use wp_error::run_error::{RunReason, RunResult};

const DEFAULT_SELF_UPDATE_BASE_URL: &str =
    "https://raw.githubusercontent.com/wp-labs/wp-install/refs/heads/main/updates";
const SELF_UPDATE_BASE_URL_ENV: &str = "UPDATE_BASE_URL";
const SELF_UPDATE_ROOT_ENV: &str = "WPROJ_SELF_UPDATE_ROOT";

pub async fn run_self_check(args: SelfCheckArgs) -> RunResult<()> {
    let report = check(CheckRequest {
        product: UpdateProduct::Suite,
        source: to_core_source(&args.source)?,
        current_version: warp_parse::build::PKG_VERSION.to_string(),
        branch: warp_parse::build::BRANCH.to_string(),
    })
    .await?;

    if args.source.json {
        return print_json(&report);
    }

    let relation = compare_versions_str(&report.current_version, &report.latest_version)?;
    print_check_report(&report, relation);
    Ok(())
}

pub async fn run_self_update(args: SelfUpdateArgs) -> RunResult<()> {
    let report = wp_self_update::update(UpdateRequest {
        product: UpdateProduct::Suite,
        source: to_core_source(&args.source)?,
        current_version: warp_parse::build::PKG_VERSION.to_string(),
        install_dir: args.install_dir.as_deref().map(std::path::PathBuf::from),
        yes: args.yes,
        dry_run: args.dry_run,
        force: args.force,
    })
    .await?;

    if args.source.json {
        return print_json(&report);
    }

    print_update_report(&report);
    Ok(())
}

fn to_core_source(source: &SelfSourceArgs) -> RunResult<SourceConfig> {
    let updates_root = source
        .updates_root
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(|| env::var_os(SELF_UPDATE_ROOT_ENV).map(std::path::PathBuf::from));
    let updates_base_url = source
        .updates_base_url
        .clone()
        .or_else(|| env::var(SELF_UPDATE_BASE_URL_ENV).ok())
        .or_else(|| Some(DEFAULT_SELF_UPDATE_BASE_URL.to_string()));

    if updates_root.is_none() && updates_base_url.is_none() {
        return Err(RunReason::from_conf().to_err().with_detail(format!(
            "self-update manifest source is required: provide --updates-base-url, --updates-root, or set {} / {}",
            SELF_UPDATE_BASE_URL_ENV, SELF_UPDATE_ROOT_ENV
        )));
    }

    Ok(SourceConfig {
        channel: to_core_channel(source.channel),
        updates_base_url: updates_base_url.unwrap_or_default(),
        updates_root,
    })
}

fn to_core_channel(channel: UpdateChannel) -> CoreChannel {
    match channel {
        UpdateChannel::Stable => CoreChannel::Stable,
        UpdateChannel::Beta => CoreChannel::Beta,
        UpdateChannel::Alpha => CoreChannel::Alpha,
    }
}

fn print_check_report(report: &CheckReport, relation: VersionRelation) {
    println!("Self-check result");
    println!("  Product  : {}", report.product);
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
}

fn print_update_report(report: &UpdateReport) {
    if report.updated {
        println!("Self-update complete");
        println!("  Product  : {}", report.product);
        println!("  Channel  : {}", report.channel);
        println!("  Install  : {}", report.install_dir);
        println!("  Current  : {}", report.current_version);
        println!("  Latest   : {}", report.latest_version);
        println!("  Artifact : {}", report.artifact);
        println!("  Status   : {}", report.status);
        return;
    }

    if report.status == "dry-run" {
        println!("Self-update dry run");
        println!("  Product  : {}", report.product);
        println!("  Channel  : {}", report.channel);
        println!("  Install  : {}", report.install_dir);
        println!("  Current  : {}", report.current_version);
        println!("  Latest   : {}", report.latest_version);
        println!("  Artifact : {}", report.artifact);
        println!("  Source   : {}", report.source);
        return;
    }

    if report.status == "aborted" {
        println!("Self-update aborted");
        return;
    }

    println!("Self-update skipped");
    println!("  Product  : {}", report.product);
    println!("  Channel  : {}", report.channel);
    println!("  Install  : {}", report.install_dir);
    println!("  Current  : {}", report.current_version);
    println!("  Latest   : {}", report.latest_version);
    println!("  Status   : {}", report.status);
}

fn render_channel(channel: &str) -> String {
    if !should_use_color() {
        return channel.to_string();
    }
    let code = match channel {
        "stable" => "32",
        "beta" => "33",
        "alpha" => "35",
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

    #[test]
    fn semver_compare_bridge_ok() {
        let relation = compare_versions_str("0.19.0", "0.20.0").unwrap();
        assert_eq!(relation, VersionRelation::UpdateAvailable);
        let parsed = semver::Version::parse("0.20.0").unwrap();
        assert_eq!(parsed.to_string(), "0.20.0");
    }

    #[test]
    fn to_core_source_rejects_missing_manifest_source() {
        let err = to_core_source(&SelfSourceArgs {
            channel: UpdateChannel::Stable,
            updates_base_url: None,
            updates_root: None,
            json: false,
        })
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("self-update manifest source is required"));
    }
}
