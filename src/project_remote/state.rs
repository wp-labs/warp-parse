use std::fs;
use std::path::Path;

use orion_variate::EnvDict;
use wp_config::engine::EngineConfig;
use wp_error::run_error::RunResult;
use wp_log::info_ctrl;

use super::managed::restore_managed_dirs;
use super::{
    conf_err, ProjectRemoteLockGuard, ProjectRemoteSnapshot, ProjectRemoteState,
    ProjectRemoteUpdateResult, ProjectRuntimeArtifactSnapshot, AUTHORITY_DB_PATH, ENGINE_CONF_PATH,
    LOCK_PATH, RULE_MAPPING_PATH, STATE_PATH,
};

pub fn acquire_project_remote_lock<P: AsRef<Path>>(
    work_root: P,
) -> RunResult<ProjectRemoteLockGuard> {
    let work_root = work_root.as_ref();
    let lock_path = work_root.join(LOCK_PATH);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|e| conf_err(format!("open {} failed: {}", lock_path.display(), e)))?;
    try_lock_file(&file, &lock_path)?;
    info_ctrl!(
        "project remote lock acquired work_root={} lock_path={}",
        work_root.display(),
        lock_path.display()
    );
    Ok(ProjectRemoteLockGuard { file })
}

pub fn capture_project_remote_snapshot<P: AsRef<Path>>(
    work_root: P,
) -> RunResult<ProjectRemoteSnapshot> {
    let work_root = work_root.as_ref();
    let state_path = work_root.join(STATE_PATH);
    let state_file = match fs::read(&state_path) {
        Ok(bytes) => Some(bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(conf_err(format!(
                "read {} failed: {}",
                state_path.display(),
                err
            )))
        }
    };
    Ok(ProjectRemoteSnapshot { state_file })
}

pub fn restore_project_remote_snapshot<P: AsRef<Path>>(
    work_root: P,
    snapshot: &ProjectRemoteSnapshot,
) -> RunResult<()> {
    restore_project_remote_update(work_root, snapshot, true)
}

pub fn restore_project_remote_update<P: AsRef<Path>>(
    work_root: P,
    snapshot: &ProjectRemoteSnapshot,
    changed: bool,
) -> RunResult<()> {
    let work_root = work_root.as_ref();
    if changed {
        restore_managed_dirs(work_root)?;
    }
    restore_state_file_bytes(work_root, snapshot.state_file.as_deref())?;
    Ok(())
}

pub fn capture_runtime_artifact_snapshot<P: AsRef<Path>>(
    work_root: P,
) -> RunResult<ProjectRuntimeArtifactSnapshot> {
    let work_root = work_root.as_ref();
    Ok(ProjectRuntimeArtifactSnapshot {
        rule_mapping: read_optional_file(&work_root.join(RULE_MAPPING_PATH))?,
        authority_db: read_optional_file(&work_root.join(AUTHORITY_DB_PATH))?,
    })
}

pub fn restore_runtime_artifact_snapshot<P: AsRef<Path>>(
    work_root: P,
    snapshot: &ProjectRuntimeArtifactSnapshot,
) -> RunResult<()> {
    let work_root = work_root.as_ref();
    restore_optional_file(
        &work_root.join(RULE_MAPPING_PATH),
        snapshot.rule_mapping.as_deref(),
    )?;
    restore_optional_file(
        &work_root.join(AUTHORITY_DB_PATH),
        snapshot.authority_db.as_deref(),
    )?;
    Ok(())
}

pub(super) fn load_engine_config(work_root: &Path) -> RunResult<EngineConfig> {
    let dict = crate::load_sec_dict().unwrap_or_else(|_| EnvDict::new());
    EngineConfig::load(work_root, &dict).map_err(|e| {
        conf_err(format!(
            "load {} failed: {}",
            work_root.join(ENGINE_CONF_PATH).display(),
            e
        ))
    })
}

pub(super) fn load_state(work_root: &Path) -> RunResult<Option<ProjectRemoteState>> {
    let path = work_root.join(STATE_PATH);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(conf_err(format!("read {} failed: {}", path.display(), err))),
    };
    let state = serde_json::from_slice(&bytes)
        .map_err(|e| conf_err(format!("parse {} failed: {}", path.display(), e)))?;
    Ok(Some(state))
}

pub(super) fn restore_project_remote_state(
    work_root: &Path,
    previous_state: Option<&ProjectRemoteState>,
) -> RunResult<()> {
    match previous_state {
        Some(state) => {
            let body = serde_json::to_vec_pretty(state)
                .map_err(|e| conf_err(format!("encode project remote state failed: {}", e)))?;
            restore_state_file_bytes(work_root, Some(body.as_slice()))
        }
        None => restore_state_file_bytes(work_root, None),
    }
}

fn restore_state_file_bytes(work_root: &Path, bytes: Option<&[u8]>) -> RunResult<()> {
    let state_path = work_root.join(STATE_PATH);
    restore_optional_file(&state_path, bytes)
}

fn restore_optional_file(path: &Path, bytes: Option<&[u8]>) -> RunResult<()> {
    match bytes {
        Some(bytes) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
            }
            fs::write(path, bytes)
                .map_err(|e| conf_err(format!("write {} failed: {}", path.display(), e)))?;
        }
        None => {
            if let Err(err) = fs::remove_file(path) {
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(conf_err(format!(
                        "remove {} failed: {}",
                        path.display(),
                        err
                    )));
                }
            }
        }
    }
    Ok(())
}

fn read_optional_file(path: &Path) -> RunResult<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(conf_err(format!("read {} failed: {}", path.display(), err))),
    }
}

fn try_lock_file(file: &fs::File, lock_path: &Path) -> RunResult<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        let detail = match err.kind() {
            std::io::ErrorKind::WouldBlock => {
                format!(
                    "project remote update already in progress ({})",
                    lock_path.display()
                )
            }
            _ => format!("lock {} failed: {}", lock_path.display(), err),
        };
        Err(conf_err(detail))
    }
    #[cfg(not(unix))]
    {
        let _ = (file, lock_path);
        Ok(())
    }
}

impl Drop for ProjectRemoteLockGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;

            let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

pub(super) fn persist_state(work_root: &Path, result: &ProjectRemoteUpdateResult) -> RunResult<()> {
    let state = ProjectRemoteState {
        current_version: result.current_version.clone(),
        resolved_tag: result.resolved_tag.clone(),
        revision: result.to_revision.clone(),
    };
    let path = work_root.join(STATE_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
    }
    let body = serde_json::to_vec_pretty(&state)
        .map_err(|e| conf_err(format!("encode project remote state failed: {}", e)))?;
    fs::write(&path, body)
        .map_err(|e| conf_err(format!("write {} failed: {}", path.display(), e)))?;
    Ok(())
}
