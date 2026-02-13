//! Lock file operations for spec execution tracking
//!
//! Provides functionality to create, read, remove, and check lock files
//! that track which processes are currently working on specs.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::paths::LOCKS_DIR;

/// RAII guard that automatically removes lock file on drop
pub struct LockGuard {
    spec_id: String,
}

impl LockGuard {
    /// Create a new lock guard and lock file
    pub fn new(spec_id: &str) -> Result<Self> {
        create_lock(spec_id)?;
        Ok(Self {
            spec_id: spec_id.to_string(),
        })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Err(e) = remove_lock(&self.spec_id) {
            eprintln!(
                "Warning: Failed to remove lock file for spec {}: {}",
                self.spec_id, e
            );
        }
    }
}

/// Create a lock file for a spec with the current process ID
pub fn create_lock(spec_id: &str) -> Result<PathBuf> {
    let lock_path = get_lock_path(spec_id);
    fs::create_dir_all(LOCKS_DIR)?;
    fs::write(&lock_path, format!("{}", std::process::id()))?;
    Ok(lock_path)
}

/// Remove a lock file for a spec
pub fn remove_lock(spec_id: &str) -> Result<()> {
    let lock_path = get_lock_path(spec_id);
    if lock_path.exists() {
        fs::remove_file(&lock_path)?;
    }
    Ok(())
}

/// Read the PID from a lock file
pub fn read_lock(spec_id: &str) -> Result<Option<u32>> {
    let lock_path = get_lock_path(spec_id);

    if !lock_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&lock_path)?;
    let pid: u32 = content.trim().parse()?;
    Ok(Some(pid))
}

/// Check if a spec has an active lock file with a running process
pub fn is_locked(spec_id: &str) -> bool {
    let lock_path = get_lock_path(spec_id);
    if !lock_path.exists() {
        return false;
    }

    // Verify PID is actually running
    match read_lock(spec_id) {
        Ok(Some(pid)) => is_process_alive(pid),
        _ => false,
    }
}

/// Check if a process with the given PID is alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Signal 0 checks if process exists without sending a signal
        kill(Pid::from_raw(pid as i32), Signal::try_from(0).ok()).is_ok()
    }
    #[cfg(not(unix))]
    {
        // On Windows, use a basic check via std::process
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| !String::from_utf8_lossy(&o.stdout).contains("No tasks"))
            .unwrap_or(false)
    }
}

/// Get the path to a spec's lock file
fn get_lock_path(spec_id: &str) -> PathBuf {
    Path::new(LOCKS_DIR).join(format!("{}.lock", spec_id))
}
