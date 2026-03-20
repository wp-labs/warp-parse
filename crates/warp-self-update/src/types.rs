use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const UPDATE_BINS: [&str; 4] = ["wparse", "wpgen", "wprescue", "wproj"];

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
    Alpha,
}

impl UpdateChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Alpha => "alpha",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceConfig {
    pub channel: UpdateChannel,
    pub updates_base_url: String,
    pub updates_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CheckRequest {
    pub source: SourceConfig,
    pub current_version: String,
    pub branch: String,
}

#[derive(Debug, Clone)]
pub struct UpdateRequest {
    pub source: SourceConfig,
    pub current_version: String,
    pub install_dir: Option<PathBuf>,
    pub yes: bool,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct CheckReport {
    pub channel: String,
    pub branch: String,
    pub source: String,
    pub manifest_format: String,
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub platform_key: String,
    pub artifact: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateReport {
    pub channel: String,
    pub source: String,
    pub current_version: String,
    pub latest_version: String,
    pub install_dir: String,
    pub artifact: String,
    pub dry_run: bool,
    pub updated: bool,
    pub status: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum VersionRelation {
    UpdateAvailable,
    UpToDate,
    AheadOfChannel,
}

#[derive(Debug)]
pub(crate) struct ResolvedRelease {
    pub(crate) version: String,
    pub(crate) target: String,
    pub(crate) artifact: String,
    pub(crate) sha256: String,
}
