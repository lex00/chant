//! PID tracking for running work processes
//!
//! Provides functionality to track and manage running agent processes
//! associated with specs being worked on.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const PIDS_DIR: &str = ".chant/pids";
const PROCESSES_DIR: &str = ".chant/processes";

/// Ensure the PIDs directory exists
pub fn ensure_pids_dir() -> Result<PathBuf> {
    let pids_dir = PathBuf::from(PIDS_DIR);
    if !pids_dir.exists() {
        fs::create_dir_all(&pids_dir)?;
    }
    Ok(pids_dir)
}

/// Write a PID file for a spec
pub fn write_pid_file(spec_id: &str, pid: u32) -> Result<()> {
    let pids_dir = ensure_pids_dir()?;
    let pid_file = pids_dir.join(format!("{}.pid", spec_id));
    fs::write(&pid_file, pid.to_string())?;
    Ok(())
}

/// Read PID from a spec's PID file
pub fn read_pid_file(spec_id: &str) -> Result<Option<u32>> {
    let pids_dir = PathBuf::from(PIDS_DIR);
    let pid_file = pids_dir.join(format!("{}.pid", spec_id));

    if !pid_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&pid_file)
        .with_context(|| format!("Failed to read PID file: {}", pid_file.display()))?;

    let pid: u32 = content
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in file: {}", content))?;

    Ok(Some(pid))
}

/// Remove PID file for a spec
pub fn remove_pid_file(spec_id: &str) -> Result<()> {
    let pids_dir = PathBuf::from(PIDS_DIR);
    let pid_file = pids_dir.join(format!("{}.pid", spec_id));

    if pid_file.exists() {
        fs::remove_file(&pid_file)?;
    }

    Ok(())
}

/// Check if a process with the given PID is running
pub fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;

        // Use `kill -0` to check if process exists without actually killing it
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        // On Windows, we could use tasklist or similar
        // For now, assume it's not running if we can't check
        // This is a limitation on non-Unix platforms
        eprintln!("Warning: Process checking not implemented for this platform");
        false
    }
}

/// Stop a process with the given PID
pub fn stop_process(pid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;

        // Try graceful termination first (SIGTERM)
        let status = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .with_context(|| format!("Failed to send SIGTERM to process {}", pid))?;

        if !status.success() {
            anyhow::bail!("Failed to terminate process {}", pid);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        anyhow::bail!("Process termination not implemented for this platform");
    }
}

/// Stop the work process for a spec
pub fn stop_spec_work(spec_id: &str) -> Result<()> {
    let pid = read_pid_file(spec_id)?;

    if let Some(pid) = pid {
        if is_process_running(pid) {
            stop_process(pid)?;
            remove_pid_file(spec_id)?;
            Ok(())
        } else {
            // Process not running, clean up PID file
            remove_pid_file(spec_id)?;
            anyhow::bail!("Process {} is not running", pid)
        }
    } else {
        anyhow::bail!("No PID file found for spec {}", spec_id)
    }
}

/// List all specs with active PID files
pub fn list_active_pids() -> Result<Vec<(String, u32, bool)>> {
    let pids_dir = PathBuf::from(PIDS_DIR);

    if !pids_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for entry in fs::read_dir(&pids_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pid") {
            if let Some(spec_id) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(Some(pid)) = read_pid_file(spec_id) {
                    let is_running = is_process_running(pid);
                    results.push((spec_id.to_string(), pid, is_running));
                }
            }
        }
    }

    Ok(results)
}

/// Clean up stale PID files (where process is no longer running)
pub fn cleanup_stale_pids() -> Result<usize> {
    let active_pids = list_active_pids()?;
    let mut cleaned = 0;

    for (spec_id, _pid, is_running) in active_pids {
        if !is_running {
            remove_pid_file(&spec_id)?;
            cleaned += 1;
        }
    }

    Ok(cleaned)
}

/// Remove process JSON files for a spec
/// Matches files like `.chant/processes/{spec_id}-{pid}.json`
pub fn remove_process_files(spec_id: &str) -> Result<()> {
    let processes_dir = PathBuf::from(PROCESSES_DIR);
    if !processes_dir.exists() {
        return Ok(());
    }
    let prefix = format!("{}-", spec_id);
    for entry in fs::read_dir(&processes_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if let Some(name_str) = name.to_str() {
            if name_str.starts_with(&prefix) && name_str.ends_with(".json") {
                fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}
