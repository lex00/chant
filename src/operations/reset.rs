//! Spec reset operation.
//!
//! Canonical implementation for resetting specs to pending status.

use anyhow::Result;
use std::path::Path;

use crate::spec::{Spec, SpecStatus};

/// Options for spec reset
#[derive(Debug, Clone, Default)]
pub struct ResetOptions {
    /// Whether to re-execute the spec after reset
    pub re_execute: bool,
    /// Optional prompt template for re-execution
    pub prompt: Option<String>,
    /// Optional branch to use for re-execution
    pub branch: Option<String>,
}

/// Reset a spec to pending status.
///
/// This is the canonical reset logic used by both CLI and MCP.
/// Only failed or in_progress specs can be reset.
pub fn reset_spec(spec: &mut Spec, spec_path: &Path, _options: ResetOptions) -> Result<()> {
    // Check if spec is in failed or in_progress state
    if spec.frontmatter.status != SpecStatus::Failed
        && spec.frontmatter.status != SpecStatus::InProgress
    {
        anyhow::bail!(
            "Spec '{}' is not in failed or in_progress state (current: {:?}). \
             Only failed or in_progress specs can be reset.",
            spec.id,
            spec.frontmatter.status
        );
    }

    let spec_id = &spec.id;

    // Clean up resources: lock file, PID file, worktree, branch
    // Use best-effort cleanup - don't fail if resources don't exist

    // Remove lock file
    let _ = crate::lock::remove_lock(spec_id);

    // Remove PID file
    let _ = crate::pid::remove_pid_file(spec_id);

    // Remove process files
    let _ = crate::pid::remove_process_files(spec_id);

    // Remove worktree if it exists
    if let Ok(config) = crate::config::Config::load() {
        let project_name = Some(config.project.name.as_str());
        if let Some(worktree_path) = crate::worktree::get_active_worktree(spec_id, project_name) {
            let _ = crate::worktree::remove_worktree(&worktree_path);
        }
    }

    // Remove branch if it exists
    if let Ok(config) = crate::config::Config::load() {
        let branch_prefix = &config.defaults.branch_prefix;
        let branch = format!("{}{}", branch_prefix, spec_id);
        let _ = std::process::Command::new("git")
            .args(["branch", "-D", &branch])
            .output();
    }

    // Reset to pending using state machine
    spec.set_status(SpecStatus::Pending)
        .map_err(|e| anyhow::anyhow!("Failed to transition spec to Pending: {}", e))?;

    // Save the spec
    spec.save(spec_path)?;

    Ok(())
}
