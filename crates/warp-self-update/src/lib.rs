use flate2::read::GzDecoder;
use orion_error::{ToStructError, UvsFrom};
use reqwest::StatusCode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Cursor, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tar::Archive;
use uuid::Uuid;
use wp_error::run_error::{RunReason, RunResult};

const FETCH_CONNECT_TIMEOUT_SECS: u64 = 5;
const FETCH_REQUEST_TIMEOUT_SECS: u64 = 10;
const FETCH_RETRY_MAX_ATTEMPTS: usize = 3;
const UPDATE_BINS: [&str; 4] = ["wparse", "wpgen", "wprescue", "wproj"];

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

pub async fn check(request: CheckRequest) -> RunResult<CheckReport> {
    let channel = request.source.channel;
    let (release, source) = load_release(&request.source, channel).await?;
    validate_artifact_version_consistency(&release.version, &release.artifact)?;

    let relation = compare_versions_str(&request.current_version, &release.version)?;
    Ok(CheckReport {
        channel: channel.as_str().to_string(),
        branch: request.branch,
        source,
        manifest_format: "v2".to_string(),
        current_version: request.current_version,
        latest_version: release.version.clone(),
        update_available: relation == VersionRelation::UpdateAvailable,
        platform_key: release.target,
        artifact: release.artifact,
        sha256: release.sha256,
    })
}

pub async fn update(request: UpdateRequest) -> RunResult<UpdateReport> {
    let channel = request.source.channel;
    let (release, source) = load_release(&request.source, channel).await?;
    validate_artifact_version_consistency(&release.version, &release.artifact)?;
    validate_download_url(&release.artifact, &request.source)?;

    let relation = compare_versions_str(&request.current_version, &release.version)?;
    let install_dir = resolve_install_dir(request.install_dir.as_deref())?;
    let install_dir_display = install_dir.display().to_string();

    if relation != VersionRelation::UpdateAvailable && !request.force {
        return Ok(UpdateReport {
            channel: channel.as_str().to_string(),
            source,
            current_version: request.current_version,
            latest_version: release.version,
            install_dir: install_dir_display,
            artifact: release.artifact,
            dry_run: request.dry_run,
            updated: false,
            status: relation_message(relation).to_string(),
        });
    }

    if is_probably_package_managed(&install_dir) && !request.force {
        return Err(RunReason::from_conf().to_err().with_detail(format!(
            "refusing to replace binaries under '{}'; looks like a package-managed install, rerun with --force if this is intentional",
            install_dir.display()
        )));
    }

    if request.dry_run {
        return Ok(UpdateReport {
            channel: channel.as_str().to_string(),
            source,
            current_version: request.current_version,
            latest_version: release.version,
            install_dir: install_dir_display,
            artifact: release.artifact,
            dry_run: true,
            updated: false,
            status: "dry-run".to_string(),
        });
    }

    if !request.yes
        && !confirm_update(
            &request.current_version,
            &release.version,
            &install_dir,
            &release.artifact,
        )?
    {
        return Ok(UpdateReport {
            channel: channel.as_str().to_string(),
            source,
            current_version: request.current_version,
            latest_version: release.version,
            install_dir: install_dir_display,
            artifact: release.artifact,
            dry_run: false,
            updated: false,
            status: "aborted".to_string(),
        });
    }

    let _lock = UpdateLock::acquire(&install_dir)?;
    let asset_bytes = fetch_asset_bytes(&release.artifact).await?;
    verify_asset_sha256(&asset_bytes, &release.sha256)?;

    let extract_root = create_temp_update_dir()?;
    let install_result = async {
        extract_artifact(&asset_bytes, &extract_root)?;
        let extracted = find_extracted_bins(&extract_root)?;
        let backup_dir = install_bins(&install_dir, &extracted)?;
        if let Err(err) = run_health_check(&install_dir, &release.version) {
            rollback_bins(&install_dir, &backup_dir)?;
            return Err(err);
        }
        Ok::<PathBuf, wp_error::RunError>(backup_dir)
    }
    .await;

    let _ = fs::remove_dir_all(&extract_root);
    let backup_dir = install_result?;

    Ok(UpdateReport {
        channel: channel.as_str().to_string(),
        source,
        current_version: request.current_version,
        latest_version: release.version,
        install_dir: install_dir_display,
        artifact: release.artifact,
        dry_run: false,
        updated: true,
        status: format!("installed (backup: {})", backup_dir.display()),
    })
}

pub fn compare_versions_str(current: &str, latest: &str) -> RunResult<VersionRelation> {
    let current_version = parse_version(current)?;
    let latest_version = parse_version(latest)?;
    Ok(compare_versions(&current_version, &latest_version))
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

pub fn relation_message(relation: VersionRelation) -> &'static str {
    match relation {
        VersionRelation::UpdateAvailable => "update available",
        VersionRelation::UpToDate => "up-to-date",
        VersionRelation::AheadOfChannel => "ahead of channel manifest",
    }
}

async fn load_release(
    source: &SourceConfig,
    channel: UpdateChannel,
) -> RunResult<(ResolvedRelease, String)> {
    if let Some(root) = source.updates_root.as_deref() {
        let path = updates_manifest_path(root, channel);
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

    let url = updates_manifest_url(&source.updates_base_url, channel);
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

fn resolve_install_dir(raw: Option<&Path>) -> RunResult<PathBuf> {
    let base = if let Some(raw) = raw {
        raw.to_path_buf()
    } else {
        let exe = std::env::current_exe().map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("failed to resolve current executable path: {}", e))
        })?;
        exe.parent().map(Path::to_path_buf).ok_or_else(|| {
            RunReason::from_conf().to_err().with_detail(format!(
                "failed to resolve install dir from {}",
                exe.display()
            ))
        })?
    };
    let canonical = base.canonicalize().map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "failed to access install dir {}: {}",
            base.display(),
            e
        ))
    })?;
    if !canonical.is_dir() {
        return Err(RunReason::from_conf().to_err().with_detail(format!(
            "install dir is not a directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn is_probably_package_managed(install_dir: &Path) -> bool {
    let path = install_dir.to_string_lossy();
    path.contains("/Cellar/")
        || path.contains("/Homebrew/")
        || path.starts_with("/usr/bin")
        || path.starts_with("/usr/local/bin")
        || path.starts_with("/opt/homebrew/bin")
}

fn confirm_update(
    current: &str,
    latest: &str,
    install_dir: &Path,
    artifact: &str,
) -> RunResult<bool> {
    println!("Self-update plan");
    println!("  Current  : {}", current);
    println!("  Latest   : {}", latest);
    println!("  Install  : {}", install_dir.display());
    println!("  Artifact : {}", artifact);
    print!("Proceed with installation? [y/N]: ");
    io::stdout()
        .flush()
        .map_err(|e| RunReason::from_conf().to_err().with_detail(e.to_string()))?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| RunReason::from_conf().to_err().with_detail(e.to_string()))?;
    let answer = line.trim().to_ascii_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

fn validate_download_url(raw: &str, source: &SourceConfig) -> RunResult<()> {
    let parsed = reqwest::Url::parse(raw).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("invalid artifact url '{}': {}", raw, e))
    })?;
    let host = parsed.host_str().unwrap_or_default();
    match parsed.scheme() {
        "https" => {
            if is_allowed_artifact_host(host, source) {
                return Ok(());
            }
            Err(RunReason::from_conf().to_err().with_detail(format!(
                "artifact host '{}' is not in the allowed release domain set",
                host
            )))
        }
        "http" => {
            if matches!(host, "127.0.0.1" | "localhost") {
                return Ok(());
            }
            Err(RunReason::from_conf().to_err().with_detail(format!(
                "insecure artifact url '{}' is not allowed; use https or loopback http for local testing",
                raw
            )))
        }
        other => Err(RunReason::from_conf().to_err().with_detail(format!(
            "unsupported artifact url scheme '{}': {}",
            other, raw
        ))),
    }
}

fn is_allowed_artifact_host(host: &str, source: &SourceConfig) -> bool {
    if matches!(
        host,
        "github.com"
            | "objects.githubusercontent.com"
            | "release-assets.githubusercontent.com"
            | "github-releases.githubusercontent.com"
            | "raw.githubusercontent.com"
            | "127.0.0.1"
            | "localhost"
    ) {
        return true;
    }

    if let Ok(url) = reqwest::Url::parse(&source.updates_base_url) {
        if url.host_str() == Some(host) {
            return true;
        }
    }
    false
}

async fn fetch_asset_bytes(url: &str) -> RunResult<Vec<u8>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(FETCH_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(FETCH_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("failed to build HTTP client: {}", e))
        })?;

    let mut last_error: Option<String> = None;
    for attempt in 1..=FETCH_RETRY_MAX_ATTEMPTS {
        match client.get(url).send().await {
            Ok(rsp) => {
                let status = rsp.status();
                if status.is_success() {
                    let bytes = rsp.bytes().await.map_err(|e| {
                        RunReason::from_conf()
                            .to_err()
                            .with_detail(format!("failed to read artifact response {}: {}", url, e))
                    })?;
                    return Ok(bytes.to_vec());
                }
                if is_retryable_status(status) && attempt < FETCH_RETRY_MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }
                return Err(RunReason::from_conf()
                    .to_err()
                    .with_detail(format!("artifact request failed {}: HTTP {}", url, status)));
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
        "failed to fetch artifact {} after {} attempts: {}",
        url,
        FETCH_RETRY_MAX_ATTEMPTS,
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn verify_asset_sha256(bytes: &[u8], expected_hex: &str) -> RunResult<()> {
    use sha2::{Digest, Sha256};
    let actual_hex = hex::encode(Sha256::digest(bytes));
    if actual_hex == expected_hex {
        return Ok(());
    }
    Err(RunReason::from_conf().to_err().with_detail(format!(
        "artifact sha256 mismatch: expected {}, got {}",
        expected_hex, actual_hex
    )))
}

fn create_temp_update_dir() -> RunResult<PathBuf> {
    let dir = std::env::temp_dir().join(format!("wproj-self-update-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "failed to create temp update dir {}: {}",
            dir.display(),
            e
        ))
    })?;
    Ok(dir)
}

fn extract_artifact(bytes: &[u8], extract_root: &Path) -> RunResult<()> {
    let cursor = Cursor::new(bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive.unpack(extract_root).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "failed to extract artifact into {}: {}",
            extract_root.display(),
            e
        ))
    })
}

fn find_extracted_bins(extract_root: &Path) -> RunResult<HashMap<&'static str, PathBuf>> {
    let mut found = HashMap::new();
    for entry in walkdir::WalkDir::new(extract_root) {
        let entry = entry.map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("failed to walk extracted artifact: {}", e))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str() else {
            continue;
        };
        if UPDATE_BINS.contains(&name) {
            found.insert(
                UPDATE_BINS
                    .iter()
                    .find(|candidate| **candidate == name)
                    .copied()
                    .unwrap(),
                entry.path().to_path_buf(),
            );
        }
    }

    let missing: Vec<&str> = UPDATE_BINS
        .iter()
        .copied()
        .filter(|name| !found.contains_key(name))
        .collect();
    if !missing.is_empty() {
        return Err(RunReason::from_conf().to_err().with_detail(format!(
            "artifact missing required binaries: {}",
            missing.join(", ")
        )));
    }
    Ok(found)
}

fn install_bins(
    install_dir: &Path,
    extracted: &HashMap<&'static str, PathBuf>,
) -> RunResult<PathBuf> {
    let update_root = install_dir.join(".warp_parse-update");
    let backup_dir = update_root
        .join("backups")
        .join(format!("{}", Uuid::new_v4()));
    fs::create_dir_all(&backup_dir).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "failed to create backup dir {}: {}",
            backup_dir.display(),
            e
        ))
    })?;

    let mut installed = Vec::new();
    for name in UPDATE_BINS {
        let src = extracted.get(name).ok_or_else(|| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("missing extracted binary '{}'", name))
        })?;
        let dst = install_dir.join(name);
        let backup = backup_dir.join(name);
        let existed = dst.exists();
        if existed {
            fs::copy(&dst, &backup).map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to back up {} to {}: {}",
                    dst.display(),
                    backup.display(),
                    e
                ))
            })?;
        }

        let staged = update_root.join(format!("{}.{}", name, Uuid::new_v4()));
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to create staging dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        fs::copy(src, &staged).map_err(|e| {
            RunReason::from_conf().to_err().with_detail(format!(
                "failed to stage {} into {}: {}",
                src.display(),
                staged.display(),
                e
            ))
        })?;
        set_exec_permission(&staged)?;
        if let Err(err) = fs::rename(&staged, &dst) {
            let _ = fs::remove_file(&staged);
            rollback_installed_bins(&installed)?;
            return Err(RunReason::from_conf().to_err().with_detail(format!(
                "failed to install {} into {}: {}",
                src.display(),
                dst.display(),
                err
            )));
        }
        installed.push(InstalledBin {
            dst,
            backup,
            existed,
        });
    }
    Ok(backup_dir)
}

fn rollback_bins(install_dir: &Path, backup_dir: &Path) -> RunResult<()> {
    let installed: Vec<InstalledBin> = UPDATE_BINS
        .iter()
        .map(|name| InstalledBin {
            dst: install_dir.join(name),
            backup: backup_dir.join(name),
            existed: backup_dir.join(name).exists(),
        })
        .collect();
    rollback_installed_bins(&installed)
}

fn rollback_installed_bins(installed: &[InstalledBin]) -> RunResult<()> {
    for item in installed.iter().rev() {
        if item.existed {
            fs::copy(&item.backup, &item.dst).map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to restore backup {} to {}: {}",
                    item.backup.display(),
                    item.dst.display(),
                    e
                ))
            })?;
            set_exec_permission(&item.dst)?;
        } else if item.dst.exists() {
            fs::remove_file(&item.dst).map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to remove partially installed {}: {}",
                    item.dst.display(),
                    e
                ))
            })?;
        }
    }
    Ok(())
}

fn run_health_check(install_dir: &Path, version: &str) -> RunResult<()> {
    let expected = version.trim().trim_start_matches('v');
    for name in UPDATE_BINS {
        let output = Command::new(install_dir.join(name))
            .arg("--version")
            .output()
            .map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "health check failed to start {} --version: {}",
                    name, e
                ))
            })?;
        if !output.status.success() {
            return Err(RunReason::from_conf().to_err().with_detail(format!(
                "health check failed for {} --version with status {}",
                name, output.status
            )));
        }
        let merged = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !merged.contains(expected) {
            return Err(RunReason::from_conf().to_err().with_detail(format!(
                "health check version mismatch for {}: expected output to contain '{}', got '{}'",
                name,
                expected,
                merged.trim()
            )));
        }
    }
    Ok(())
}

fn set_exec_permission(path: &Path) -> RunResult<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)
            .map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to stat {}: {}",
                    path.display(),
                    e
                ))
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).map_err(|e| {
            RunReason::from_conf().to_err().with_detail(format!(
                "failed to set executable permission on {}: {}",
                path.display(),
                e
            ))
        })?;
    }
    Ok(())
}

struct InstalledBin {
    dst: PathBuf,
    backup: PathBuf,
    existed: bool,
}

struct UpdateLock {
    path: PathBuf,
}

impl UpdateLock {
    fn acquire(install_dir: &Path) -> RunResult<Self> {
        let path = install_dir.join(".warp_parse-update").join("lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to create update lock dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        clear_stale_lock_if_present(&path)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                RunReason::from_conf().to_err().with_detail(format!(
                    "failed to acquire update lock {}: {}",
                    path.display(),
                    e
                ))
            })?;
        let _ = writeln!(file, "pid={}", std::process::id());
        Ok(Self { path })
    }
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn clear_stale_lock_if_present(path: &Path) -> RunResult<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path).unwrap_or_default();
    let pid = parse_lock_pid(&content);
    if pid.is_some_and(process_is_alive) {
        return Ok(());
    }

    fs::remove_file(path).map_err(|e| {
        RunReason::from_conf().to_err().with_detail(format!(
            "failed to clear stale update lock {}: {}",
            path.display(),
            e
        ))
    })
}

fn parse_lock_pid(content: &str) -> Option<u32> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|value| value.trim().parse::<u32>().ok())
}

fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid as i32, 0) };
        if rc == 0 {
            return true;
        }
        let errno = std::io::Error::last_os_error().raw_os_error();
        return !matches!(errno, Some(libc::ESRCH));
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use httpmock::prelude::*;
    use tar::Builder;
    use tempfile::tempdir;

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

    #[test]
    fn retryable_status_rules_ok() {
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn package_managed_dir_detects_usr_local_bin() {
        assert!(is_probably_package_managed(Path::new("/usr/local/bin")));
    }

    #[test]
    fn download_url_rejects_untrusted_https_host() {
        let err = validate_download_url(
            "https://evil.example.com/warp-parse-v0.30.0.tar.gz",
            &SourceConfig {
                channel: UpdateChannel::Stable,
                updates_base_url: "https://raw.githubusercontent.com/wp-labs/wp-install/main"
                    .to_string(),
                updates_root: None,
            },
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("allowed release domain"));
    }

    #[test]
    fn stale_lock_is_cleared_when_pid_is_dead() {
        let dir = tempdir().expect("tempdir");
        let lock_path = dir.path().join("lock");
        fs::write(&lock_path, "pid=999999\n").expect("write stale lock");
        clear_stale_lock_if_present(&lock_path).expect("clear stale lock");
        assert!(!lock_path.exists());
    }

    fn platform_key_for_test() -> Option<&'static str> {
        detect_target_triple_v2().ok()
    }

    fn build_artifact_tar_gz(version: &str, healthy: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let encoder = GzEncoder::new(&mut out, Compression::default());
        let mut builder = Builder::new(encoder);
        for bin in UPDATE_BINS {
            let body = if healthy || bin != "wproj" {
                format!("#!/bin/sh\necho \"{} {}\"\n", bin, version)
            } else {
                "#!/bin/sh\nexit 1\n".to_string()
            };
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("artifacts/{}", bin), body.as_bytes())
                .expect("append tar entry");
        }
        let encoder = builder.into_inner().expect("finish tar builder");
        encoder.finish().expect("finish gzip");
        out
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(bytes))
    }

    fn write_existing_bins(dir: &Path, version: &str) {
        for bin in UPDATE_BINS {
            let path = dir.join(bin);
            fs::write(&path, format!("#!/bin/sh\necho \"{} {}\"\n", bin, version))
                .expect("write existing bin");
            #[cfg(unix)]
            {
                let mut perms = fs::metadata(&path)
                    .expect("stat existing bin")
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&path, perms).expect("chmod existing bin");
            }
        }
    }

    #[tokio::test]
    async fn self_update_downloads_and_installs_release() {
        let Some(platform_key) = platform_key_for_test() else {
            return;
        };

        let server = MockServer::start_async().await;
        let artifact = build_artifact_tar_gz("0.30.0", true);
        let sha256 = sha256_hex(&artifact);
        let artifact_mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/artifacts/warp-parse-v0.30.0.tar.gz");
                then.status(200)
                    .header("content-type", "application/gzip")
                    .body(artifact.clone());
            })
            .await;

        let manifest = format!(
            r#"{{
  "version": "0.30.0",
  "channel": "stable",
  "assets": {{
    "{platform_key}": {{
      "url": "{base}/artifacts/warp-parse-v0.30.0.tar.gz",
      "sha256": "{sha256}"
    }}
  }}
}}"#,
            base = server.base_url()
        );
        let manifest_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/updates/stable/manifest.json");
                then.status(200)
                    .header("content-type", "application/json")
                    .body(manifest);
            })
            .await;

        let install_dir = tempdir().expect("install tempdir");
        write_existing_bins(install_dir.path(), "0.21.0");

        let report = update(UpdateRequest {
            source: SourceConfig {
                channel: UpdateChannel::Stable,
                updates_base_url: server.base_url(),
                updates_root: None,
            },
            current_version: "0.21.0".to_string(),
            install_dir: Some(install_dir.path().to_path_buf()),
            yes: true,
            dry_run: false,
            force: false,
        })
        .await
        .expect("run self update");

        artifact_mock.assert_async().await;
        manifest_mock.assert_async().await;
        assert!(report.updated);

        let out = Command::new(install_dir.path().join("wproj"))
            .arg("--version")
            .output()
            .expect("run installed wproj");
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("0.30.0"));
    }

    #[tokio::test]
    async fn self_update_rolls_back_on_health_check_failure() {
        let Some(platform_key) = platform_key_for_test() else {
            return;
        };

        let server = MockServer::start_async().await;
        let artifact = build_artifact_tar_gz("0.30.0", false);
        let sha256 = sha256_hex(&artifact);
        let manifest = format!(
            r#"{{
  "version": "0.30.0",
  "channel": "stable",
  "assets": {{
    "{platform_key}": {{
      "url": "{base}/artifacts/warp-parse-v0.30.0.tar.gz",
      "sha256": "{sha256}"
    }}
  }}
}}"#,
            base = server.base_url()
        );

        server
            .mock_async(|when, then| {
                when.method(GET).path("/updates/stable/manifest.json");
                then.status(200)
                    .header("content-type", "application/json")
                    .body(manifest);
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/artifacts/warp-parse-v0.30.0.tar.gz");
                then.status(200)
                    .header("content-type", "application/gzip")
                    .body(artifact.clone());
            })
            .await;

        let install_dir = tempdir().expect("install tempdir");
        write_existing_bins(install_dir.path(), "0.21.0");

        let err = update(UpdateRequest {
            source: SourceConfig {
                channel: UpdateChannel::Stable,
                updates_base_url: server.base_url(),
                updates_root: None,
            },
            current_version: "0.21.0".to_string(),
            install_dir: Some(install_dir.path().to_path_buf()),
            yes: true,
            dry_run: false,
            force: false,
        })
        .await
        .expect_err("self update should fail");
        assert!(format!("{}", err).contains("health check failed"));

        let out = Command::new(install_dir.path().join("wproj"))
            .arg("--version")
            .output()
            .expect("run rolled back wproj");
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("0.21.0"));
    }
}
