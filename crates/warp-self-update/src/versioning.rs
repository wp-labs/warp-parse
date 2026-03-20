use crate::types::VersionRelation;
use orion_error::{ToStructError, UvsFrom};
use semver::Version;
use wp_error::run_error::{RunReason, RunResult};

pub(crate) fn parse_version(raw: &str) -> RunResult<Version> {
    let normalized = raw.trim().trim_start_matches('v');
    Version::parse(normalized).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("invalid semver '{}': {}", raw, e))
    })
}

pub(crate) fn compare_versions(current: &Version, latest: &Version) -> VersionRelation {
    if latest > current {
        return VersionRelation::UpdateAvailable;
    }
    if latest == current {
        return VersionRelation::UpToDate;
    }
    VersionRelation::AheadOfChannel
}

pub fn compare_versions_str(current: &str, latest: &str) -> RunResult<VersionRelation> {
    let current_version = parse_version(current)?;
    let latest_version = parse_version(latest)?;
    Ok(compare_versions(&current_version, &latest_version))
}

pub(crate) fn validate_artifact_version_consistency(
    version: &str,
    artifact: &str,
) -> RunResult<()> {
    if artifact.contains(version) {
        return Ok(());
    }
    Err(RunReason::from_conf().to_err().with_detail(format!(
        "artifact/version mismatch: artifact '{}' does not contain version '{}'",
        artifact, version
    )))
}

pub fn relation_message(relation: VersionRelation) -> &'static str {
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
