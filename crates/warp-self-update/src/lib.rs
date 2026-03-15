use orion_error::{ToStructError, UvsFrom};
mod fetch;
mod install;
mod lock;
mod manifest;
mod platform;
mod types;
mod versioning;

pub use manifest::updates_manifest_url;
pub use types::{
    CheckReport, CheckRequest, SourceConfig, UpdateChannel, UpdateReport, UpdateRequest,
    VersionRelation,
};
pub use versioning::{compare_versions_str, relation_message};

use fetch::load_release;
use install::{
    confirm_update, create_temp_update_dir, extract_artifact, fetch_asset_bytes,
    find_extracted_bins, install_bins, is_probably_package_managed, resolve_install_dir,
    rollback_bins, run_health_check, validate_download_url, verify_asset_sha256,
};
use lock::UpdateLock;
use std::path::PathBuf;
use versioning::validate_artifact_version_consistency;
use wp_error::run_error::RunResult;

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
        return Err(wp_error::run_error::RunReason::from_conf()
            .to_err()
            .with_detail(format!(
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

    let _ = std::fs::remove_dir_all(&extract_root);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UPDATE_BINS;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;
    use tar::Builder;
    use tempfile::tempdir;

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

    fn apply_artifact(install_dir: &Path, artifact: &[u8], version: &str) -> RunResult<PathBuf> {
        let extract_root = create_temp_update_dir()?;
        let install_result = (|| {
            extract_artifact(artifact, &extract_root)?;
            let extracted = find_extracted_bins(&extract_root)?;
            let backup_dir = install_bins(install_dir, &extracted)?;
            if let Err(err) = run_health_check(install_dir, version) {
                rollback_bins(install_dir, &backup_dir)?;
                return Err(err);
            }
            Ok(backup_dir)
        })();
        let _ = std::fs::remove_dir_all(&extract_root);
        install_result
    }

    #[test]
    fn installs_release_artifact() {
        let artifact = build_artifact_tar_gz("0.30.0", true);
        let install_dir = tempdir().expect("install tempdir");
        write_existing_bins(install_dir.path(), "0.21.0");

        let backup_dir =
            apply_artifact(install_dir.path(), &artifact, "0.30.0").expect("install artifact");
        assert!(backup_dir.exists());

        let out = Command::new(install_dir.path().join("wproj"))
            .arg("--version")
            .output()
            .expect("run installed wproj");
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("0.30.0"));
    }

    #[test]
    fn rolls_back_on_health_check_failure() {
        let artifact = build_artifact_tar_gz("0.30.0", false);
        let install_dir = tempdir().expect("install tempdir");
        write_existing_bins(install_dir.path(), "0.21.0");

        let err =
            apply_artifact(install_dir.path(), &artifact, "0.30.0").expect_err("expected failure");
        assert!(format!("{}", err).contains("health check failed"));

        let out = Command::new(install_dir.path().join("wproj"))
            .arg("--version")
            .output()
            .expect("run rolled back wproj");
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("0.21.0"));
    }
}
