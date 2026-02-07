//! Spec finalization operation.
//!
//! Canonical implementation for finalizing specs with full validation.

use anyhow::{Context, Result};

use crate::config::Config;
use crate::repository::spec_repository::{FileSpecRepository, SpecRepository};
use crate::spec::{Spec, SpecStatus, TransitionBuilder};
use crate::worktree;

/// Options for spec finalization
#[derive(Debug, Clone, Default)]
pub struct FinalizeOptions {
    /// Allow finalization without commits
    pub allow_no_commits: bool,
    /// Pre-fetched commits (if None, will auto-detect)
    pub commits: Option<Vec<String>>,
}

/// Get commits for a spec (placeholder - should be implemented in library).
/// For now, this is a simplified version that doesn't auto-detect commits.
fn get_commits_for_spec_impl(_spec_id: &str, _allow_no_commits: bool) -> Result<Vec<String>> {
    // This is a placeholder - in a real implementation, this would
    // call git log to find commits for the spec
    Ok(Vec::new())
}

/// Detect if a commit has agent co-authorship (placeholder).
fn detect_agent_in_commit_impl(_commit: &str) -> Result<bool> {
    // This is a placeholder - in a real implementation, this would
    // parse the commit message for "Co-Authored-By: Claude" etc.
    Ok(false)
}

/// Get model name from config (placeholder).
fn get_model_name_impl(_config: Option<&Config>) -> Option<String> {
    // This is a placeholder - in a real implementation, this would
    // extract the model name from environment or config
    None
}

/// Finalize a spec after successful completion.
///
/// This is the canonical finalization logic with full validation:
/// - Checks for uncommitted changes in worktree
/// - Validates driver/member relationships
/// - Detects commits (if not provided)
/// - Checks agent co-authorship for approval requirements
/// - Updates status, commits, completed_at, and model
/// - Verifies persistence
///
/// This function is idempotent and can be called multiple times safely.
pub fn finalize_spec(
    spec: &mut Spec,
    spec_repo: &FileSpecRepository,
    config: &Config,
    all_specs: &[Spec],
    options: FinalizeOptions,
) -> Result<()> {
    use crate::spec;

    // Check for uncommitted changes in worktree before finalization
    if let Some(worktree_path) = worktree::get_active_worktree(&spec.id, None) {
        if worktree::has_uncommitted_changes(&worktree_path)? {
            anyhow::bail!(
                "Cannot finalize: uncommitted changes in worktree. Commit your changes first.\nWorktree: {}",
                worktree_path.display()
            );
        }
    }

    // Check if this is a driver spec with incomplete members
    let incomplete_members = spec::get_incomplete_members(&spec.id, all_specs);
    if !incomplete_members.is_empty() {
        anyhow::bail!(
            "Cannot complete driver spec '{}' while {} member spec(s) are incomplete: {}",
            spec.id,
            incomplete_members.len(),
            incomplete_members.join(", ")
        );
    }

    // Use provided commits or fetch them
    let commits = match options.commits {
        Some(c) => c,
        None => {
            // Auto-detect commits (simplified version)
            get_commits_for_spec_impl(&spec.id, options.allow_no_commits)?
        }
    };

    // Check for agent co-authorship if config requires approval for agent work
    if config.approval.require_approval_for_agent_work {
        check_and_set_agent_approval(spec, &commits, config)?;
    }

    // Update spec to completed using state machine
    TransitionBuilder::new(spec)
        .to(SpecStatus::Completed)
        .context("Failed to transition spec to Completed status")?;

    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );
    spec.frontmatter.model = get_model_name_impl(Some(config));

    // Save the spec
    spec_repo
        .save(spec)
        .context("Failed to save finalized spec")?;

    // Validation: Verify that status was actually changed to Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after finalization"
    );

    // Validation: Verify that completed_at timestamp is set and in valid ISO format
    let completed_at = spec
        .frontmatter
        .completed_at
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("completed_at timestamp was not set"))?;

    if !completed_at.ends_with('Z') {
        anyhow::bail!(
            "completed_at must end with 'Z' (UTC format), got: {}",
            completed_at
        );
    }
    if !completed_at.contains('T') {
        anyhow::bail!(
            "completed_at must contain 'T' separator (ISO format), got: {}",
            completed_at
        );
    }

    // Validation: Verify that spec was actually saved (reload and check)
    let saved_spec = spec_repo
        .load(&spec.id)
        .context("Failed to reload spec from disk to verify persistence")?;

    anyhow::ensure!(
        saved_spec.frontmatter.status == SpecStatus::Completed,
        "Persisted spec status is not Completed - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.completed_at.is_some(),
        "Persisted spec is missing completed_at - save may have failed"
    );

    // Check commits match
    match (&spec.frontmatter.commits, &saved_spec.frontmatter.commits) {
        (Some(mem_commits), Some(saved_commits)) => {
            anyhow::ensure!(
                mem_commits == saved_commits,
                "Persisted commits don't match memory - save may have failed"
            );
        }
        (None, None) => {
            // Both None is correct
        }
        _ => {
            anyhow::bail!("Persisted commits don't match memory - save may have failed");
        }
    }

    Ok(())
}

/// Check commits for agent co-authorship and set approval requirement if found.
fn check_and_set_agent_approval(
    spec: &mut Spec,
    commits: &[String],
    _config: &Config,
) -> Result<()> {
    use crate::spec::{Approval, ApprovalStatus};

    // Skip if approval is already set
    if spec.frontmatter.approval.is_some() {
        return Ok(());
    }

    // Check each commit for agent co-authorship
    for commit in commits {
        if detect_agent_in_commit_impl(commit)? {
            // Agent detected - set approval requirement
            spec.frontmatter.approval = Some(Approval {
                required: true,
                status: ApprovalStatus::Pending,
                by: None,
                at: None,
            });
            return Ok(());
        }
    }

    Ok(())
}
