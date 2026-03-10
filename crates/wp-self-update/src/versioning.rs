use crate::error::{err, Result};
use crate::types::VersionRelation;
use semver::Version;

pub(crate) fn parse_version(raw: &str) -> Result<Version> {
    let normalized = raw.trim().trim_start_matches('v');
    Version::parse(normalized).map_err(|e| err(format!("invalid semver '{}': {}", raw, e)))
}

pub fn compare_versions(current: &Version, latest: &Version) -> VersionRelation {
    if latest > current {
        return VersionRelation::UpdateAvailable;
    }
    if latest == current {
        return VersionRelation::UpToDate;
    }
    VersionRelation::AheadOfChannel
}

pub(crate) fn validate_artifact_version_consistency(version: &str, artifact: &str) -> Result<()> {
    if artifact.contains(version) {
        return Ok(());
    }
    Err(err(format!(
        "artifact/version mismatch: artifact '{}' does not contain version '{}'",
        artifact, version
    )))
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
}
