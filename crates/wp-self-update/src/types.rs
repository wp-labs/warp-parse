use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
pub struct SelfCheckRequest {
    pub branch: String,
    pub current_version: String,
    pub channel: Option<UpdateChannel>,
    pub updates_base_url: String,
    pub updates_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SelfCheckOutcome {
    pub channel: UpdateChannel,
    pub branch: String,
    pub source: String,
    pub current_version: String,
    pub latest_version: String,
    pub relation: VersionRelation,
    pub platform_key: String,
    pub artifact: String,
    pub sha256: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum VersionRelation {
    UpdateAvailable,
    UpToDate,
    AheadOfChannel,
}

impl SelfCheckOutcome {
    pub fn update_available(&self) -> bool {
        self.relation == VersionRelation::UpdateAvailable
    }
}
