//! Lock file operations for spec execution tracking
//!
//! Provides functionality to create, read, remove, and check lock files
//! that track which processes are currently working on specs.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::paths::LOCKS_DIR;

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

/// Check if a spec has an active lock file
pub fn is_locked(spec_id: &str) -> bool {
    get_lock_path(spec_id).exists()
}

/// Get the path to a spec's lock file
fn get_lock_path(spec_id: &str) -> PathBuf {
    Path::new(LOCKS_DIR).join(format!("{}.lock", spec_id))
}
