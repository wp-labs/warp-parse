use crate::platform::detect_target_triple_v2;
use crate::types::{ResolvedRelease, UpdateChannel};
use orion_error::{ToStructError, UvsFrom};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wp_error::run_error::{RunReason, RunResult};

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

pub fn updates_manifest_url(base_url: &str, channel: UpdateChannel) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{}/updates/{}/manifest.json", base, channel.as_str())
}

pub(crate) fn updates_manifest_path(root: &Path, channel: UpdateChannel) -> PathBuf {
    root.join("updates")
        .join(channel.as_str())
        .join("manifest.json")
}

pub(crate) fn parse_v2_release(
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let release = parse_v2_release(raw, "test", UpdateChannel::Alpha).unwrap();
        assert_eq!(release.version, "0.12.2-alpha");
    }

    #[test]
    fn parse_v2_release_channel_mismatch_err() {
        let raw = r#"{
  "version": "0.12.2-alpha",
  "channel": "beta",
  "assets": {"aarch64-apple-darwin": { "url": "https://example.com/a.tar.gz", "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" }}
}"#;
        let err = parse_v2_release(raw, "test", UpdateChannel::Alpha).unwrap_err();
        assert!(format!("{}", err).contains("channel mismatch"));
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
        assert!(format!("{}", err).contains("invalid sha256"));
    }
}
