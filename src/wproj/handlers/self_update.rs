use crate::args::{SelfCheckArgs, UpdateChannel};
use crate::format::print_json;
use orion_error::{ToStructError, UvsFrom};
use reqwest::StatusCode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use wp_error::run_error::{RunReason, RunResult};

const FETCH_CONNECT_TIMEOUT_SECS: u64 = 5;
const FETCH_REQUEST_TIMEOUT_SECS: u64 = 10;
const FETCH_RETRY_MAX_ATTEMPTS: usize = 3;

#[derive(Debug, Deserialize)]
struct UpdateManifestV2 {
    version: String,
    channel: String,
    assets: HashMap<String, UpdateAssetV2>,
}

#[derive(Debug, Deserialize)]
struct UpdateAssetV2 {
    url: String,
    sha256: String,
}

#[derive(Debug)]
struct ResolvedRelease {
    version: String,
    target: String,
    artifact: String,
    sha256: String,
}

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum VersionRelation {
    UpdateAvailable,
    UpToDate,
    AheadOfChannel,
}

pub async fn run_self_check(args: SelfCheckArgs) -> RunResult<()> {
    let branch = warp_parse::build::BRANCH.to_string();
    let channel = args
        .channel
        .unwrap_or_else(|| infer_channel_from_branch(&branch));
    let (release, source) = load_release(&args, channel).await?;
    validate_artifact_version_consistency(&release.version, &release.artifact)?;

    let current_version = parse_version(warp_parse::build::PKG_VERSION)?;
    let latest_version = parse_version(&release.version)?;
    let relation = compare_versions(&current_version, &latest_version);
    let update_available = relation == VersionRelation::UpdateAvailable;

    let report = SelfCheckReport {
        channel: channel.as_str().to_string(),
        branch,
        source,
        manifest_format: "v2".to_string(),
        current_version: warp_parse::build::PKG_VERSION.to_string(),
        latest_version: release.version.clone(),
        update_available,
        platform_key: release.target.clone(),
        artifact: release.artifact.clone(),
        sha256: release.sha256.clone(),
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

fn compare_versions(current: &Version, latest: &Version) -> VersionRelation {
    if latest > current {
        return VersionRelation::UpdateAvailable;
    }
    if latest == current {
        return VersionRelation::UpToDate;
    }
    VersionRelation::AheadOfChannel
}

fn relation_message(relation: VersionRelation) -> &'static str {
    match relation {
        VersionRelation::UpdateAvailable => "update available",
        VersionRelation::UpToDate => "up-to-date",
        VersionRelation::AheadOfChannel => "ahead of channel manifest",
    }
}

async fn load_release(
    args: &SelfCheckArgs,
    channel: UpdateChannel,
) -> RunResult<(ResolvedRelease, String)> {
    if let Some(root) = args.updates_root.as_deref() {
        let path = updates_manifest_path(Path::new(root), channel);
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            RunReason::from_conf().to_err().with_detail(format!(
                "failed to read manifest {}: {}",
                path.display(),
                e
            ))
        })?;
        let release = parse_v2_release(&raw, &path.display().to_string(), channel)?;
        return Ok((release, path.display().to_string()));
    }

    let url = updates_manifest_url(&args.updates_base_url, channel);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(FETCH_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(FETCH_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("failed to build HTTP client: {}", e))
        })?;

    let raw = fetch_manifest_text(&client, &url).await?;
    let release = parse_v2_release(&raw, &url, channel)?;
    Ok((release, url))
}

fn parse_v2_release(
    raw: &str,
    source: &str,
    expected_channel: UpdateChannel,
) -> RunResult<ResolvedRelease> {
    let manifest = serde_json::from_str::<UpdateManifestV2>(raw).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("invalid v2 manifest JSON {}: {}", source, e))
    })?;

    if manifest.channel != expected_channel.as_str() {
        return Err(RunReason::from_conf().to_err().with_detail(format!(
            "manifest channel mismatch: expected '{}', got '{}' ({})",
            expected_channel.as_str(),
            manifest.channel,
            source
        )));
    }

    let target = detect_target_triple_v2()?;
    let asset = manifest.assets.get(target).ok_or_else(|| {
        let mut keys: Vec<&str> = manifest.assets.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        RunReason::from_conf().to_err().with_detail(format!(
            "manifest missing asset for target '{}': {} (available: {})",
            target,
            source,
            keys.join(", ")
        ))
    })?;

    Ok(ResolvedRelease {
        version: manifest.version,
        target: target.to_string(),
        artifact: asset.url.clone(),
        sha256: validate_sha256_hex(&asset.sha256, source, target)?,
    })
}

async fn fetch_manifest_text(client: &reqwest::Client, url: &str) -> RunResult<String> {
    let mut last_error: Option<String> = None;

    for attempt in 1..=FETCH_RETRY_MAX_ATTEMPTS {
        match client.get(url).send().await {
            Ok(rsp) => {
                let status = rsp.status();
                if status.is_success() {
                    return rsp.text().await.map_err(|e| {
                        RunReason::from_conf()
                            .to_err()
                            .with_detail(format!("failed to read manifest response {}: {}", url, e))
                    });
                }

                if status == StatusCode::NOT_FOUND {
                    return Err(RunReason::from_conf()
                        .to_err()
                        .with_detail(format!("manifest not found: {}", url)));
                }

                if is_retryable_status(status) && attempt < FETCH_RETRY_MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }

                return Err(RunReason::from_conf()
                    .to_err()
                    .with_detail(format!("manifest request failed {}: HTTP {}", url, status)));
            }
            Err(e) => {
                last_error = Some(e.to_string());
                if attempt < FETCH_RETRY_MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }
            }
        }
    }

    Err(RunReason::from_conf().to_err().with_detail(format!(
        "failed to fetch manifest {} after {} attempts: {}",
        url,
        FETCH_RETRY_MAX_ATTEMPTS,
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

fn updates_manifest_path(root: &Path, channel: UpdateChannel) -> PathBuf {
    root.join("updates")
        .join(channel.as_str())
        .join("manifest.json")
}

fn updates_manifest_url(base_url: &str, channel: UpdateChannel) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{}/updates/{}/manifest.json", base, channel.as_str())
}

fn detect_target_triple_v2() -> RunResult<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => Err(RunReason::from_conf()
            .to_err()
            .with_detail(format!("unsupported platform: {}-{}", os, arch))),
    }
}

fn parse_version(raw: &str) -> RunResult<Version> {
    let normalized = raw.trim().trim_start_matches('v');
    Version::parse(normalized).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("invalid semver '{}': {}", raw, e))
    })
}

fn validate_artifact_version_consistency(version: &str, artifact: &str) -> RunResult<()> {
    if artifact.contains(version) {
        return Ok(());
    }
    Err(RunReason::from_conf().to_err().with_detail(format!(
        "artifact/version mismatch: artifact '{}' does not contain version '{}'",
        artifact, version
    )))
}

fn validate_sha256_hex(raw: &str, source: &str, target: &str) -> RunResult<String> {
    let value = raw.trim().to_ascii_lowercase();
    let is_hex_64 = value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit());
    if is_hex_64 {
        return Ok(value);
    }
    Err(RunReason::from_conf().to_err().with_detail(format!(
        "invalid sha256 for target '{}' in {}: expected 64 hex chars, got '{}'",
        target, source, raw
    )))
}

fn infer_channel_from_branch(branch: &str) -> UpdateChannel {
    let name = branch
        .rsplit('/')
        .next()
        .unwrap_or(branch)
        .to_ascii_lowercase();
    match name.as_str() {
        "alpha" => UpdateChannel::Alpha,
        "beta" => UpdateChannel::Beta,
        _ => UpdateChannel::Stable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_accepts_v_prefix() {
        let parsed = parse_version("v0.19.0-alpha.3").unwrap();
        assert_eq!(parsed.to_string(), "0.19.0-alpha.3");
    }

    #[test]
    fn compare_versions_update_available() {
        let current = Version::parse("0.18.0").unwrap();
        let latest = Version::parse("0.19.0").unwrap();
        assert_eq!(
            compare_versions(&current, &latest),
            VersionRelation::UpdateAvailable
        );
    }

    #[test]
    fn compare_versions_up_to_date() {
        let current = Version::parse("0.19.0").unwrap();
        let latest = Version::parse("0.19.0").unwrap();
        assert_eq!(
            compare_versions(&current, &latest),
            VersionRelation::UpToDate
        );
    }

    #[test]
    fn compare_versions_ahead_of_channel() {
        let current = Version::parse("0.19.0").unwrap();
        let latest = Version::parse("0.15.3").unwrap();
        assert_eq!(
            compare_versions(&current, &latest),
            VersionRelation::AheadOfChannel
        );
    }

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
    fn infer_channel_from_branch_ok() {
        assert_eq!(infer_channel_from_branch("main"), UpdateChannel::Stable);
        assert_eq!(infer_channel_from_branch("beta"), UpdateChannel::Beta);
        assert_eq!(
            infer_channel_from_branch("feature/alpha-hotfix"),
            UpdateChannel::Stable
        );
    }

    #[test]
    fn updates_manifest_path_mapping_ok() {
        let root = Path::new("./repo");
        assert_eq!(
            updates_manifest_path(root, UpdateChannel::Stable),
            PathBuf::from("./repo/updates/stable/manifest.json")
        );
        assert_eq!(
            updates_manifest_path(root, UpdateChannel::Beta),
            PathBuf::from("./repo/updates/beta/manifest.json")
        );
        assert_eq!(
            updates_manifest_path(root, UpdateChannel::Alpha),
            PathBuf::from("./repo/updates/alpha/manifest.json")
        );
    }

    #[test]
    fn updates_manifest_url_mapping_ok() {
        let base = "https://raw.githubusercontent.com/wp-labs/wp-install/main";
        assert_eq!(
            updates_manifest_url(base, UpdateChannel::Stable),
            "https://raw.githubusercontent.com/wp-labs/wp-install/main/updates/stable/manifest.json"
        );
        assert_eq!(
            updates_manifest_url(base, UpdateChannel::Beta),
            "https://raw.githubusercontent.com/wp-labs/wp-install/main/updates/beta/manifest.json"
        );
        assert_eq!(
            updates_manifest_url(base, UpdateChannel::Alpha),
            "https://raw.githubusercontent.com/wp-labs/wp-install/main/updates/alpha/manifest.json"
        );
    }

    #[test]
    fn parse_v2_release_ok() {
        let raw = r#"{
  "version": "0.12.2-alpha",
  "channel": "alpha",
  "assets": {
    "aarch64-apple-darwin": { "url": "https://example.com/app-v0.12.2-alpha-aarch64-apple-darwin.tar.gz", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" },
    "aarch64-unknown-linux-gnu": { "url": "https://example.com/app-v0.12.2-alpha-aarch64-unknown-linux-gnu.tar.gz", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" },
    "x86_64-unknown-linux-gnu": { "url": "https://example.com/app-v0.12.2-alpha-x86_64-unknown-linux-gnu.tar.gz", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" }
  }
}"#;
        let r = parse_v2_release(raw, "test", UpdateChannel::Alpha).unwrap();
        assert_eq!(r.version, "0.12.2-alpha");
    }

    #[test]
    fn parse_v2_release_channel_mismatch_err() {
        let raw = r#"{
  "version": "0.12.2-alpha",
  "channel": "beta",
  "assets": {"aarch64-apple-darwin": { "url": "https://example.com/a.tar.gz", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" }}
}"#;
        let err = parse_v2_release(raw, "test", UpdateChannel::Alpha).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("channel mismatch"));
    }

    #[test]
    fn parse_v2_release_invalid_sha256_err() {
        let raw = r#"{
  "version": "0.12.2-alpha",
  "channel": "alpha",
  "assets": {
    "aarch64-apple-darwin": { "url": "https://example.com/a.tar.gz", "sha256": "" },
    "aarch64-unknown-linux-gnu": { "url": "https://example.com/b.tar.gz", "sha256": "" },
    "x86_64-unknown-linux-gnu": { "url": "https://example.com/c.tar.gz", "sha256": "" }
  }
}"#;
        let err = parse_v2_release(raw, "test", UpdateChannel::Alpha).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("invalid sha256"));
    }

    #[test]
    fn retryable_status_rules_ok() {
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }
}
