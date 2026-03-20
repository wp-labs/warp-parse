use crate::fetch::is_retryable_status;
use crate::types::{SourceConfig, UPDATE_BINS};
use flate2::read::GzDecoder;
use orion_error::{ToStructError, UvsFrom};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Cursor, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tar::Archive;
use uuid::Uuid;
use wp_error::run_error::{RunReason, RunResult};

pub(crate) fn resolve_install_dir(raw: Option<&Path>) -> RunResult<PathBuf> {
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

pub(crate) fn is_probably_package_managed(install_dir: &Path) -> bool {
    let path = install_dir.to_string_lossy();
    path.contains("/Cellar/")
        || path.contains("/Homebrew/")
        || path.starts_with("/usr/bin")
        || path.starts_with("/usr/local/bin")
        || path.starts_with("/opt/homebrew/bin")
}

pub(crate) fn confirm_update(
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

pub(crate) fn validate_download_url(raw: &str, source: &SourceConfig) -> RunResult<()> {
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

pub(crate) async fn fetch_asset_bytes(url: &str) -> RunResult<Vec<u8>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("failed to build HTTP client: {}", e))
        })?;

    let mut last_error: Option<String> = None;
    for attempt in 1..=3 {
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
                if is_retryable_status(status) && attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }
                return Err(RunReason::from_conf()
                    .to_err()
                    .with_detail(format!("artifact request failed {}: HTTP {}", url, status)));
            }
            Err(e) => {
                last_error = Some(e.to_string());
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }
            }
        }
    }
    Err(RunReason::from_conf().to_err().with_detail(format!(
        "failed to fetch artifact {} after {} attempts: {}",
        url,
        3,
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

pub(crate) fn verify_asset_sha256(bytes: &[u8], expected_hex: &str) -> RunResult<()> {
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

pub(crate) fn create_temp_update_dir() -> RunResult<PathBuf> {
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

pub(crate) fn extract_artifact(bytes: &[u8], extract_root: &Path) -> RunResult<()> {
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

pub(crate) fn find_extracted_bins(
    extract_root: &Path,
) -> RunResult<HashMap<&'static str, PathBuf>> {
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

pub(crate) fn install_bins(
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

pub(crate) fn rollback_bins(install_dir: &Path, backup_dir: &Path) -> RunResult<()> {
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

pub(crate) fn run_health_check(install_dir: &Path, version: &str) -> RunResult<()> {
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

pub(crate) struct InstalledBin {
    pub(crate) dst: PathBuf,
    pub(crate) backup: PathBuf,
    pub(crate) existed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UpdateChannel;

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
}
