use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use walkdir::WalkDir;
use wp_error::run_error::RunResult;

use super::{conf_err, BackupManifest, BACKUP_MANIFEST_PATH, BACKUP_PATH, MANAGED_DIRS};

pub(super) fn managed_dirs_differ(remote_root: &Path, current_root: &Path) -> RunResult<bool> {
    for dir in MANAGED_DIRS {
        if !paths_equal(&remote_root.join(dir), &current_root.join(dir))? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn backup_managed_dirs(work_root: &Path) -> RunResult<()> {
    let backup_root = work_root.join(BACKUP_PATH);
    remove_path(&backup_root)?;
    fs::create_dir_all(&backup_root)
        .map_err(|e| conf_err(format!("create {} failed: {}", backup_root.display(), e)))?;

    let mut existing_dirs = Vec::new();
    for dir in MANAGED_DIRS {
        let src = work_root.join(dir);
        if !src.exists() {
            continue;
        }
        existing_dirs.push((*dir).to_string());
        copy_path(&src, &backup_root.join(dir))?;
    }

    let manifest = BackupManifest { existing_dirs };
    let manifest_path = work_root.join(BACKUP_MANIFEST_PATH);
    let body = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| conf_err(format!("encode backup manifest failed: {}", e)))?;
    fs::write(&manifest_path, body)
        .map_err(|e| conf_err(format!("write {} failed: {}", manifest_path.display(), e)))?;
    Ok(())
}

pub(super) fn sync_managed_dirs(remote_root: &Path, work_root: &Path) -> RunResult<()> {
    for dir in MANAGED_DIRS {
        let src = remote_root.join(dir);
        let dst = work_root.join(dir);
        remove_path(&dst)?;
        if src.exists() {
            copy_path(&src, &dst)?;
        }
    }
    Ok(())
}

pub(super) fn restore_managed_dirs(work_root: &Path) -> RunResult<()> {
    let manifest_path = work_root.join(BACKUP_MANIFEST_PATH);
    let body = match fs::read(&manifest_path) {
        Ok(body) => body,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(conf_err(format!(
                "read {} failed: {}",
                manifest_path.display(),
                err
            )))
        }
    };
    let manifest: BackupManifest = serde_json::from_slice(&body)
        .map_err(|e| conf_err(format!("parse {} failed: {}", manifest_path.display(), e)))?;
    let backup_root = work_root.join(BACKUP_PATH);

    for dir in MANAGED_DIRS {
        remove_path(&work_root.join(dir))?;
    }
    for dir in &manifest.existing_dirs {
        copy_path(&backup_root.join(dir), &work_root.join(dir))?;
    }
    Ok(())
}

fn paths_equal(left: &Path, right: &Path) -> RunResult<bool> {
    let left_exists = left.exists();
    let right_exists = right.exists();
    if !left_exists && !right_exists {
        return Ok(true);
    }
    if left_exists != right_exists {
        return Ok(false);
    }

    let left_meta = fs::symlink_metadata(left)
        .map_err(|e| conf_err(format!("stat {} failed: {}", left.display(), e)))?;
    let right_meta = fs::symlink_metadata(right)
        .map_err(|e| conf_err(format!("stat {} failed: {}", right.display(), e)))?;
    if left_meta.file_type().is_dir() != right_meta.file_type().is_dir() {
        return Ok(false);
    }
    if left_meta.file_type().is_file() != right_meta.file_type().is_file() {
        return Ok(false);
    }
    if left_meta.file_type().is_symlink() != right_meta.file_type().is_symlink() {
        return Ok(false);
    }

    if left_meta.file_type().is_file() {
        return file_bytes_equal(left, right);
    }
    if left_meta.file_type().is_symlink() {
        let left_target = fs::read_link(left)
            .map_err(|e| conf_err(format!("read link {} failed: {}", left.display(), e)))?;
        let right_target = fs::read_link(right)
            .map_err(|e| conf_err(format!("read link {} failed: {}", right.display(), e)))?;
        return Ok(left_target == right_target);
    }

    let left_entries = read_dir_names(left)?;
    let right_entries = read_dir_names(right)?;
    if left_entries != right_entries {
        return Ok(false);
    }
    for entry in left_entries {
        if !paths_equal(&left.join(&entry), &right.join(&entry))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn read_dir_names(path: &Path) -> RunResult<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path)
        .map_err(|e| conf_err(format!("read dir {} failed: {}", path.display(), e)))?
    {
        let entry = entry.map_err(|e| conf_err(format!("read dir entry failed: {}", e)))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| conf_err(format!("non-utf8 path under {}", path.display())))?;
        names.insert(name);
    }
    Ok(names)
}

fn file_bytes_equal(left: &Path, right: &Path) -> RunResult<bool> {
    let left_bytes =
        fs::read(left).map_err(|e| conf_err(format!("read {} failed: {}", left.display(), e)))?;
    let right_bytes =
        fs::read(right).map_err(|e| conf_err(format!("read {} failed: {}", right.display(), e)))?;
    Ok(left_bytes == right_bytes)
}

fn copy_path(src: &Path, dst: &Path) -> RunResult<()> {
    let meta = fs::symlink_metadata(src)
        .map_err(|e| conf_err(format!("stat {} failed: {}", src.display(), e)))?;
    if meta.file_type().is_dir() {
        fs::create_dir_all(dst)
            .map_err(|e| conf_err(format!("create {} failed: {}", dst.display(), e)))?;
        for entry in WalkDir::new(src).min_depth(1) {
            let entry =
                entry.map_err(|e| conf_err(format!("walk {} failed: {}", src.display(), e)))?;
            let rel = entry
                .path()
                .strip_prefix(src)
                .map_err(|e| conf_err(format!("strip prefix failed: {}", e)))?;
            let target = dst.join(rel);
            let file_type = entry.file_type();
            if file_type.is_dir() {
                fs::create_dir_all(&target)
                    .map_err(|e| conf_err(format!("create {} failed: {}", target.display(), e)))?;
            } else if file_type.is_file() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        conf_err(format!("create {} failed: {}", parent.display(), e))
                    })?;
                }
                fs::copy(entry.path(), &target).map_err(|e| {
                    conf_err(format!(
                        "copy {} -> {} failed: {}",
                        entry.path().display(),
                        target.display(),
                        e
                    ))
                })?;
                let permissions = fs::metadata(entry.path())
                    .map_err(|e| {
                        conf_err(format!("stat {} failed: {}", entry.path().display(), e))
                    })?
                    .permissions();
                fs::set_permissions(&target, permissions).map_err(|e| {
                    conf_err(format!(
                        "set permissions on {} failed: {}",
                        target.display(),
                        e
                    ))
                })?;
            } else if file_type.is_symlink() {
                copy_symlink(entry.path(), &target)?;
            } else {
                return Err(conf_err(format!(
                    "unsupported file type under {}",
                    entry.path().display()
                )));
            }
        }
        return Ok(());
    }

    if meta.file_type().is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
        }
        fs::copy(src, dst).map_err(|e| {
            conf_err(format!(
                "copy {} -> {} failed: {}",
                src.display(),
                dst.display(),
                e
            ))
        })?;
        fs::set_permissions(dst, meta.permissions()).map_err(|e| {
            conf_err(format!(
                "set permissions on {} failed: {}",
                dst.display(),
                e
            ))
        })?;
        return Ok(());
    }

    if meta.file_type().is_symlink() {
        return copy_symlink(src, dst);
    }

    Err(conf_err(format!("unsupported file type {}", src.display())))
}

fn copy_symlink(src: &Path, dst: &Path) -> RunResult<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
    }
    remove_path(dst)?;
    let target = fs::read_link(src)
        .map_err(|e| conf_err(format!("read link {} failed: {}", src.display(), e)))?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, dst).map_err(|e| {
            conf_err(format!(
                "create symlink {} -> {} failed: {}",
                dst.display(),
                target.display(),
                e
            ))
        })?;
        return Ok(());
    }
    #[cfg(windows)]
    {
        let meta = fs::metadata(src)
            .map_err(|e| conf_err(format!("stat {} failed: {}", src.display(), e)))?;
        if meta.is_dir() {
            std::os::windows::fs::symlink_dir(&target, dst).map_err(|e| {
                conf_err(format!(
                    "create symlink {} -> {} failed: {}",
                    dst.display(),
                    target.display(),
                    e
                ))
            })?;
        } else {
            std::os::windows::fs::symlink_file(&target, dst).map_err(|e| {
                conf_err(format!(
                    "create symlink {} -> {} failed: {}",
                    dst.display(),
                    target.display(),
                    e
                ))
            })?;
        }
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err(conf_err(format!(
        "symlink copy is not supported on this platform for {}",
        src.display()
    )))
}

pub(super) fn remove_path(path: &Path) -> RunResult<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(conf_err(format!("stat {} failed: {}", path.display(), err))),
    };
    if meta.file_type().is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| conf_err(format!("remove {} failed: {}", path.display(), e)))?;
    } else {
        fs::remove_file(path)
            .map_err(|e| conf_err(format!("remove {} failed: {}", path.display(), e)))?;
    }
    Ok(())
}
