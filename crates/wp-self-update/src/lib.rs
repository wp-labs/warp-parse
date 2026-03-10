mod error;
mod fetch;
mod manifest;
mod platform;
mod types;
mod versioning;

pub use error::SelfUpdateError;
pub use manifest::updates_manifest_url;
pub use types::{SelfCheckOutcome, SelfCheckRequest, UpdateChannel, VersionRelation};
pub use versioning::compare_versions;

use error::Result;
use fetch::load_release;
use versioning::{parse_version, validate_artifact_version_consistency};

pub async fn check_updates(req: SelfCheckRequest) -> Result<SelfCheckOutcome> {
    let channel = req
        .channel
        .unwrap_or_else(|| infer_channel_from_branch(&req.branch));

    let (release, source) = load_release(&req, channel).await?;
    validate_artifact_version_consistency(&release.version, &release.artifact)?;

    let current_version = parse_version(&req.current_version)?;
    let latest_version = parse_version(&release.version)?;
    let relation = compare_versions(&current_version, &latest_version);

    Ok(SelfCheckOutcome {
        channel,
        branch: req.branch,
        source,
        current_version: req.current_version,
        latest_version: release.version,
        relation,
        platform_key: release.target,
        artifact: release.artifact,
        sha256: release.sha256,
    })
}

pub fn infer_channel_from_branch(branch: &str) -> UpdateChannel {
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
    fn infer_channel_from_branch_ok() {
        assert_eq!(infer_channel_from_branch("main"), UpdateChannel::Stable);
        assert_eq!(infer_channel_from_branch("beta"), UpdateChannel::Beta);
        assert_eq!(
            infer_channel_from_branch("feature/alpha-hotfix"),
            UpdateChannel::Stable
        );
    }
}
