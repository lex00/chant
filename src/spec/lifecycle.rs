//! Spec lifecycle operations.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::frontmatter::SpecStatus;
use super::parse::Spec;

/// Apply blocked status to specs with unmet dependencies.
/// For pending specs that have incomplete dependencies, updates their status to blocked.
/// This is a local-only version that only checks dependencies within the current repo.
fn apply_blocked_status(specs: &mut [Spec]) {
    apply_blocked_status_with_repos(specs, std::path::Path::new(".chant/specs"), &[]);
}

/// Apply blocked status considering both local and cross-repo dependencies.
/// This version supports cross-repo dependency checking when repos config is available.
pub fn apply_blocked_status_with_repos(
    specs: &mut [Spec],
    specs_dir: &std::path::Path,
    repos: &[crate::config::RepoConfig],
) {
    // Build a reference list of specs for dependency checking
    let specs_snapshot = specs.to_vec();

    for spec in specs.iter_mut() {
        // Handle both Pending and Blocked specs
        if spec.frontmatter.status != SpecStatus::Pending
            && spec.frontmatter.status != SpecStatus::Blocked
        {
            continue;
        }

        // Check if this spec has unmet dependencies (local only)
        let is_blocked_locally = spec.is_blocked(&specs_snapshot);

        // Check cross-repo dependencies if repos config is available
        let is_blocked_cross_repo = !repos.is_empty()
            && crate::deps::is_blocked_by_dependencies(spec, &specs_snapshot, specs_dir, repos);

        if is_blocked_locally || is_blocked_cross_repo {
            // Has unmet dependencies - mark as blocked
            spec.frontmatter.status = SpecStatus::Blocked;
        } else if spec.frontmatter.status == SpecStatus::Blocked {
            // No unmet dependencies and was previously blocked - revert to pending
            spec.frontmatter.status = SpecStatus::Pending;
        }
    }
}

pub fn load_all_specs(specs_dir: &Path) -> Result<Vec<Spec>> {
    load_all_specs_with_options(specs_dir, true)
}

/// Load all specs with optional branch resolution.
pub fn load_all_specs_with_options(
    specs_dir: &Path,
    use_branch_resolution: bool,
) -> Result<Vec<Spec>> {
    let mut specs = Vec::new();

    if !specs_dir.exists() {
        return Ok(specs);
    }

    load_specs_recursive(specs_dir, &mut specs, use_branch_resolution)?;

    // Apply blocked status to specs with unmet dependencies
    apply_blocked_status(&mut specs);

    Ok(specs)
}

/// Recursively load specs from a directory and its subdirectories.
fn load_specs_recursive(
    dir: &Path,
    specs: &mut Vec<Spec>,
    use_branch_resolution: bool,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            // Recursively load from subdirectories
            load_specs_recursive(&path, specs, use_branch_resolution)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            let load_result = if use_branch_resolution {
                Spec::load_with_branch_resolution(&path)
            } else {
                Spec::load(&path)
            };

            match load_result {
                Ok(spec) => specs.push(spec),
                Err(e) => {
                    eprintln!("Warning: Failed to load spec {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(())
}

/// Resolve a partial spec ID to a full spec.
/// Only searches active specs (in .chant/specs/), not archived specs.
pub fn resolve_spec(specs_dir: &Path, partial_id: &str) -> Result<Spec> {
    let specs = load_all_specs(specs_dir)?;

    // Exact match
    if let Some(spec) = specs.iter().find(|s| s.id == partial_id) {
        return Ok(spec.clone());
    }

    // Suffix match (random suffix)
    let suffix_matches: Vec<_> = specs
        .iter()
        .filter(|s| s.id.ends_with(partial_id))
        .collect();
    if suffix_matches.len() == 1 {
        return Ok(suffix_matches[0].clone());
    }

    // Sequence match for today (e.g., "001")
    if partial_id.len() == 3 {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_pattern = format!("{}-{}-", today, partial_id);
        let today_matches: Vec<_> = specs
            .iter()
            .filter(|s| s.id.starts_with(&today_pattern))
            .collect();
        if today_matches.len() == 1 {
            return Ok(today_matches[0].clone());
        }
    }

    // Partial date match (e.g., "22-001" or "01-22-001")
    let partial_matches: Vec<_> = specs.iter().filter(|s| s.id.contains(partial_id)).collect();
    if partial_matches.len() == 1 {
        return Ok(partial_matches[0].clone());
    }

    if partial_matches.len() > 1 {
        anyhow::bail!(
            "Ambiguous spec ID '{}'. Matches: {}",
            partial_id,
            partial_matches
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    anyhow::bail!("Spec not found: {}", partial_id)
}

/// Load a spec from its worktree if it exists, otherwise from the main repo.
///
/// This ensures that watch sees the current state of the spec including
/// any changes made by the agent in the worktree.
///
/// # Errors
///
/// Returns an error if the spec file is unreadable from both locations.
fn load_spec_from_worktree_or_main(spec_id: &str) -> Result<Spec> {
    // Check if a worktree exists for this spec
    if let Some(worktree_path) = crate::worktree::get_active_worktree(spec_id) {
        let worktree_spec_path = worktree_path
            .join(".chant/specs")
            .join(format!("{}.md", spec_id));

        // Try to load from worktree first
        if worktree_spec_path.exists() {
            return Spec::load(&worktree_spec_path).with_context(|| {
                format!(
                    "Failed to read spec file from worktree: {}",
                    worktree_spec_path.display()
                )
            });
        }
    }

    // Fall back to main repo
    let spec_path = Path::new(".chant/specs").join(format!("{}.md", spec_id));
    Spec::load(&spec_path)
        .with_context(|| format!("Failed to read spec file: {}", spec_path.display()))
}

/// Check if a spec is completed (ready for finalization).
///
/// A spec is considered completed if:
/// - Status is `in_progress`
/// - All acceptance criteria checkboxes are checked (`[x]`)
/// - Worktree is clean (no uncommitted changes including untracked files)
///
/// Edge cases:
/// - Spec with no acceptance criteria: Treated as completed if worktree clean
/// - Spec already finalized: Returns false (status not `in_progress`)
///
/// # Errors
///
/// Returns an error if:
/// - Spec file is unreadable
/// - Worktree is inaccessible (git status fails)
pub fn is_completed(spec_id: &str) -> Result<bool> {
    // Load the spec from worktree if it exists, otherwise from main repo
    let spec = load_spec_from_worktree_or_main(spec_id)?;

    // Only in_progress specs can be completed
    if spec.frontmatter.status != SpecStatus::InProgress {
        return Ok(false);
    }

    // Check if all criteria are checked
    let unchecked_count = spec.count_unchecked_checkboxes();
    if unchecked_count > 0 {
        return Ok(false);
    }

    // Fix G: verify commit exists before reporting completion
    // This prevents watch from reporting false completions
    if !has_success_signals(spec_id)? {
        return Ok(false);
    }

    // Check if worktree is clean
    is_worktree_clean(spec_id)
}

/// Check if a spec has success signals indicating work was completed.
///
/// Success signals include:
/// - Commits matching the `chant(spec_id):` pattern
///
/// # Errors
///
/// Returns an error if git command fails
pub(crate) fn has_success_signals(spec_id: &str) -> Result<bool> {
    // Check for commits with the chant(spec_id): pattern
    let pattern = format!("chant({}):", spec_id);
    let output = Command::new("git")
        .args(["log", "--all", "--grep", &pattern, "--format=%H"])
        .output()
        .context("Failed to check git log for spec commits")?;

    if !output.status.success() {
        return Ok(false);
    }

    let commits_output = String::from_utf8_lossy(&output.stdout);
    let has_commits = !commits_output.trim().is_empty();

    Ok(has_commits)
}

/// Check if a spec has failed.
///
/// A spec is considered failed if:
/// - Status is `in_progress`
/// - Agent has exited (no lock file present)
/// - Some acceptance criteria are still incomplete
/// - No success signals present (commits matching `chant(spec_id):` pattern)
///
/// Edge cases:
/// - Agent still running: Returns false
/// - Spec already finalized/failed: Returns false (status not `in_progress`)
/// - Has commits matching chant(spec_id) pattern: Returns false (agent completed work)
///
/// # Errors
///
/// Returns an error if:
/// - Spec file is unreadable
/// - Git commands fail
pub fn is_failed(spec_id: &str) -> Result<bool> {
    // Load the spec from worktree if it exists, otherwise from main repo
    let spec = load_spec_from_worktree_or_main(spec_id)?;

    // Only in_progress specs can fail
    if spec.frontmatter.status != SpecStatus::InProgress {
        return Ok(false);
    }

    // Check if agent is still running (lock file exists)
    let lock_file = Path::new(crate::paths::LOCKS_DIR).join(format!("{}.lock", spec_id));
    if lock_file.exists() {
        return Ok(false);
    }

    // Check if criteria are incomplete
    let unchecked_count = spec.count_unchecked_checkboxes();
    if unchecked_count == 0 {
        // All criteria checked - not failed
        return Ok(false);
    }

    // Check for success signals before flagging as failed
    // If work was committed, don't mark as failed even if criteria unchecked
    if has_success_signals(spec_id)? {
        return Ok(false);
    }

    // No lock, incomplete criteria, no success signals - failed
    Ok(true)
}

/// Check if worktree for a spec is clean (no uncommitted changes).
///
/// Uses `git status --porcelain` to check for uncommitted changes.
/// Untracked files count as dirty for safety.
///
/// # Errors
///
/// Returns an error if git status command fails or worktree is inaccessible.
fn is_worktree_clean(spec_id: &str) -> Result<bool> {
    let worktree_path = Path::new("/tmp").join(format!("chant-{}", spec_id));

    // If worktree doesn't exist, check in current directory
    let check_path = if worktree_path.exists() {
        &worktree_path
    } else {
        Path::new(".")
    };

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(check_path)
        .output()
        .with_context(|| format!("Failed to check git status in {:?}", check_path))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git status failed: {}", stderr);
    }

    let status_output = String::from_utf8_lossy(&output.stdout);
    Ok(status_output.trim().is_empty())
}
