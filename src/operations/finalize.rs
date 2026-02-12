//! Spec finalization operation.
//!
//! Canonical implementation for finalizing specs with full validation.

use anyhow::{Context, Result};

use crate::config::Config;
use crate::operations::commits::{
    detect_agent_in_commit, get_commits_for_spec_allow_no_commits,
    get_commits_for_spec_with_branch, get_commits_for_spec_with_branch_allow_no_commits,
};
use crate::operations::model::get_model_name;
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
            // Auto-detect commits with branch awareness
            let spec_branch = spec.frontmatter.branch.as_deref();
            if spec_branch.is_some() && !options.allow_no_commits {
                get_commits_for_spec_with_branch(&spec.id, spec_branch)?
            } else if options.allow_no_commits {
                get_commits_for_spec_allow_no_commits(&spec.id)?
            } else if let Some(branch) = spec_branch {
                get_commits_for_spec_with_branch_allow_no_commits(&spec.id, Some(branch))?
            } else {
                get_commits_for_spec_allow_no_commits(&spec.id)?
            }
        }
    };

    // Check for agent co-authorship if config requires approval for agent work
    if config.approval.require_approval_for_agent_work {
        check_and_set_agent_approval(spec, &commits, config)?;
    }

    // Update spec to completed using state machine with force transition
    // This allows finalization from any state (e.g. failed specs whose agent completed work)
    TransitionBuilder::new(spec)
        .force()
        .to(SpecStatus::Completed)
        .context("Failed to transition spec to Completed status")?;

    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };
    spec.frontmatter.completed_at = Some(crate::utc_now_iso());
    spec.frontmatter.model = get_model_name(Some(config));

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

    // Parse timestamp to validate ISO 8601 format
    chrono::DateTime::parse_from_rfc3339(completed_at).with_context(|| {
        format!(
            "completed_at must be valid ISO 8601 format, got: {}",
            completed_at
        )
    })?;

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
        match detect_agent_in_commit(commit) {
            Ok(result) if result.has_agent => {
                // Agent detected - set approval requirement
                spec.frontmatter.approval = Some(Approval {
                    required: true,
                    status: ApprovalStatus::Pending,
                    by: None,
                    at: None,
                });
                return Ok(());
            }
            Ok(_) => {
                // No agent found in this commit, continue
            }
            Err(e) => {
                // Log warning but continue checking other commits
                eprintln!(
                    "Warning: Failed to check commit {} for agent: {}",
                    commit, e
                );
            }
        }
    }

    Ok(())
}
