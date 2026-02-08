//! Spec pause operation.
//!
//! Canonical implementation for pausing running work processes.

use anyhow::Result;

use crate::spec::{Spec, SpecStatus};
use std::path::Path;

/// Options for pausing a spec
#[derive(Debug, Clone, Default)]
pub struct PauseOptions {
    /// Force stop the process without confirmation (CLI only)
    pub force: bool,
}

/// Pause a running work process for a spec.
///
/// This is the canonical pause logic:
/// - Checks for PID file and running process
/// - Stops the process if running (respecting force flag)
/// - Updates spec status from in_progress to paused
/// - Cleans up PID file
///
/// Returns Ok(true) if a process was stopped, Ok(false) otherwise.
pub fn pause_spec(spec: &mut Spec, spec_path: &Path, options: PauseOptions) -> Result<bool> {
    let spec_id = &spec.id;

    // Check if there's a PID file
    let pid = crate::pid::read_pid_file(spec_id)?;

    let mut process_stopped = false;
    if let Some(pid) = pid {
        if crate::pid::is_process_running(pid) {
            // Process is running
            if options.force {
                // Stop the process
                crate::pid::stop_process(pid)?;
                crate::pid::remove_pid_file(spec_id)?;
                process_stopped = true;
            } else {
                // MCP handler: always stop without force flag check
                // CLI: this should have been checked earlier and returned error
                crate::pid::stop_process(pid)?;
                crate::pid::remove_pid_file(spec_id)?;
                process_stopped = true;
            }
        } else {
            // Process not running, clean up PID file
            crate::pid::remove_pid_file(spec_id)?;
        }
    }

    // Update spec status to paused if it was in_progress
    if spec.frontmatter.status == SpecStatus::InProgress {
        spec.set_status(SpecStatus::Paused)
            .map_err(|e| anyhow::anyhow!("Failed to transition spec to Paused: {}", e))?;
        spec.save(spec_path)?;
    }

    Ok(process_stopped)
}
