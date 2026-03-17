use orion_error::{ToStructError, UvsFrom};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use wp_error::run_error::{RunReason, RunResult};

pub(crate) struct UpdateLock {
    path: PathBuf,
}

impl UpdateLock {
    pub(crate) fn acquire(install_dir: &Path) -> RunResult<Self> {
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
    use tempfile::tempdir;

    #[test]
    fn stale_lock_is_cleared_when_pid_is_dead() {
        let dir = tempdir().expect("tempdir");
        let lock_path = dir.path().join("lock");
        fs::write(&lock_path, "pid=999999\n").expect("write stale lock");
        clear_stale_lock_if_present(&lock_path).expect("clear stale lock");
        assert!(!lock_path.exists());
    }
}
