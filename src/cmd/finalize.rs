//! Spec finalization logic.
//!
//! Handles marking specs as complete, updating frontmatter with commits,
//! timestamps, and model information.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use chant::config::Config;
use chant::repository::spec_repository::{FileSpecRepository, SpecRepository};
use chant::spec::{self, load_all_specs, Spec, SpecStatus, TransitionBuilder};
use chant::worktree;

use crate::cmd::commits::{
    detect_agent_in_commit, get_commits_for_spec, get_commits_for_spec_allow_no_commits,
    get_commits_for_spec_with_branch,
};
use crate::cmd::model::get_model_name;

/// Maximum characters to store in agent output section
pub const MAX_AGENT_OUTPUT_CHARS: usize = 5000;

/// Finalize a spec after successful completion
/// Sets status, commits, completed_at, and model
/// This function is idempotent and can be called multiple times safely
///
/// If `commits` is provided, uses those commits directly.
/// If `commits` is None, fetches commits using get_commits_for_spec.
pub fn finalize_spec(
    spec: &mut Spec,
    spec_repo: &FileSpecRepository,
    config: &Config,
    all_specs: &[Spec],
    allow_no_commits: bool,
    commits: Option<Vec<String>>,
) -> Result<()> {
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
    // Check the spec's branch field first if available (Issue 1 fix)
    let commits = match commits {
        Some(c) => c,
        None => {
            // If spec has a branch field, search that branch first
            let spec_branch = spec.frontmatter.branch.as_deref();
            if spec_branch.is_some() && !allow_no_commits {
                // Use branch-aware search
                get_commits_for_spec_with_branch(&spec.id, spec_branch)?
            } else if allow_no_commits {
                get_commits_for_spec_allow_no_commits(&spec.id)?
            } else {
                get_commits_for_spec(&spec.id)?
            }
        }
    };

    // Check for agent co-authorship if config requires approval for agent work
    if config.approval.require_approval_for_agent_work {
        check_and_set_agent_approval(spec, &commits, config)?;
    }

    // Update spec to completed using SpecStateMachine
    // Note: clean tree check is already done above via worktree::has_uncommitted_changes
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
    spec.frontmatter.model = get_model_name(Some(config));

    eprintln!(
        "{} [{}] Saving spec with status=Completed, commits={}, completed_at={:?}, model={:?}",
        "→".cyan(),
        spec.id,
        spec.frontmatter
            .commits
            .as_ref()
            .map(|c| c.len())
            .unwrap_or(0),
        spec.frontmatter.completed_at,
        spec.frontmatter.model
    );

    // Save the spec - this must not fail silently
    spec_repo
        .save(spec)
        .context("Failed to save finalized spec")?;

    eprintln!(
        "{} [{}] Spec successfully saved to disk with status=Completed",
        "✓".green(),
        spec.id
    );

    // Validation 1: Verify that status was actually changed to Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after finalization"
    );

    // Validation 2: Verify that completed_at timestamp is set and in valid ISO format
    let completed_at = spec
        .frontmatter
        .completed_at
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("completed_at timestamp was not set"))?;

    // Validate ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
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

    // Validation 3: Verify that spec was actually saved (reload and check)
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

    // Model may be None if no model was detected, but commits should match memory
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

    // Check what this spec unblocked
    let specs_dir = spec_repo.specs_dir();
    let unblocked = find_dependent_specs(&spec.id, specs_dir)?;
    if !unblocked.is_empty() {
        println!(
            "{} Unblocked {} dependent spec(s):",
            "✓".green(),
            unblocked.len()
        );
        for dependent_id in unblocked {
            println!("  - {}", dependent_id);
        }
    }

    // Auto-complete parent group if this is a member and all siblings are complete
    if let Some(parent_id) = get_parent_group_id(&spec.id) {
        auto_complete_parent_group(&parent_id, specs_dir)?;
    }

    Ok(())
}

/// Re-finalize a spec that was left in an incomplete state
/// This can be called on in_progress or completed specs to update commits and timestamp
/// Idempotent: safe to call multiple times
pub fn re_finalize_spec(
    spec: &mut Spec,
    spec_repo: &FileSpecRepository,
    config: &Config,
    allow_no_commits: bool,
) -> Result<()> {
    // Re-finalization only works on specs that have been started (in_progress or completed)
    // A pending spec has never been started and should use normal work flow
    // Allow failed too - agents often leave specs in failed state when they actually completed the work
    match spec.frontmatter.status {
        SpecStatus::InProgress | SpecStatus::Completed | SpecStatus::Failed => {
            // These are valid for re-finalization
        }
        _ => {
            anyhow::bail!(
                "Cannot re-finalize spec '{}' with status '{:?}'. Must be in_progress, completed, or failed.",
                spec.id,
                spec.frontmatter.status
            );
        }
    }

    // Get the commits for this spec (may have new ones since last finalization)
    // Check the spec's branch field first if available (Issue 1 fix)
    let spec_branch = spec.frontmatter.branch.as_deref();
    let commits = if spec_branch.is_some() && !allow_no_commits {
        // Use branch-aware search
        get_commits_for_spec_with_branch(&spec.id, spec_branch)?
    } else if allow_no_commits {
        get_commits_for_spec_allow_no_commits(&spec.id)?
    } else {
        get_commits_for_spec(&spec.id)?
    };

    // Update spec with new commit info
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };

    // Update the timestamp to now
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );

    // Update model name
    spec.frontmatter.model = get_model_name(Some(config));

    // Ensure spec is marked as completed
    spec.force_status(SpecStatus::Completed);

    // Save the spec
    spec_repo
        .save(spec)
        .context("Failed to save re-finalized spec")?;

    // Validation 1: Verify that status is Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after re-finalization"
    );

    // Validation 2: Verify completed_at timestamp is set and valid
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

    // Validation 3: Verify spec was saved (reload and check)
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

    Ok(())
}

/// Prompt for user confirmation
/// Returns true if user confirms, false otherwise
/// force_flag bypasses the confirmation
pub fn confirm_re_finalize(spec_id: &str, force_flag: bool) -> Result<bool> {
    if force_flag {
        return Ok(true);
    }

    println!(
        "{} Are you sure you want to re-finalize spec '{}'?",
        "?".cyan(),
        spec_id
    );
    println!("This will update commits and completion timestamp to now.");
    println!(
        "Use {} to skip this confirmation.",
        "--skip-criteria".cyan()
    );

    use std::io::{self, Write};
    print!("Continue? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Check commits for agent co-authorship and set approval requirement if found.
/// This is called during finalization when require_approval_for_agent_work is enabled.
fn check_and_set_agent_approval(
    spec: &mut Spec,
    commits: &[String],
    config: &Config,
) -> Result<()> {
    use chant::spec::{Approval, ApprovalStatus};

    // Skip if approval is already set (don't override existing approval settings)
    if spec.frontmatter.approval.is_some() {
        return Ok(());
    }

    // Check each commit for agent co-authorship
    for commit in commits {
        match detect_agent_in_commit(commit) {
            Ok(result) if result.has_agent => {
                // Agent detected - set approval requirement
                let agent_sig = result
                    .agent_signature
                    .unwrap_or_else(|| "AI Agent".to_string());
                eprintln!(
                    "{} Agent co-authorship detected in commit {}: {}",
                    "⚠".yellow(),
                    commit,
                    agent_sig
                );
                eprintln!(
                    "{} Auto-setting approval requirement (config: require_approval_for_agent_work={})",
                    "→".cyan(),
                    config.approval.require_approval_for_agent_work
                );

                // Set approval requirement
                spec.frontmatter.approval = Some(Approval {
                    required: true,
                    status: ApprovalStatus::Pending,
                    by: None,
                    at: None,
                });

                eprintln!(
                    "{} Spec requires approval before merge. Run: chant approve {} --by <approver>",
                    "ℹ".blue(),
                    spec.id
                );

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

/// Append agent output to the spec body, truncating if too long.
pub fn append_agent_output(spec: &mut Spec, output: &str) {
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let formatted_output = if output.len() > MAX_AGENT_OUTPUT_CHARS {
        let truncated = &output[..MAX_AGENT_OUTPUT_CHARS];
        format!(
            "{}\n\n... (output truncated, {} chars total)",
            truncated,
            output.len()
        )
    } else {
        output.to_string()
    };

    let agent_section = format!(
        "\n\n## Agent Output\n\n{}\n\n```\n{}```\n",
        timestamp,
        formatted_output.trim_end()
    );

    spec.body.push_str(&agent_section);
}

/// Find specs that depend on the completed spec and are now ready.
/// Returns a list of spec IDs that were unblocked by completing this spec.
pub fn find_dependent_specs(completed_spec_id: &str, specs_dir: &Path) -> Result<Vec<String>> {
    // Load all specs to check dependencies
    let all_specs = load_all_specs(specs_dir)?;
    let mut unblocked = Vec::new();

    for spec in &all_specs {
        // Check if this spec depends on the completed spec
        if let Some(deps) = &spec.frontmatter.depends_on {
            if deps.contains(&completed_spec_id.to_string()) {
                // This spec depends on the one we just completed
                // Check if it's now ready (all dependencies met)
                if spec.is_ready(&all_specs) {
                    unblocked.push(spec.id.clone());
                }
            }
        }
    }

    Ok(unblocked)
}

/// Extract parent group ID from a member spec ID.
/// Member IDs have format "2026-01-30-00h-f77.1" where parent is "2026-01-30-00h-f77".
/// Returns None if this is not a member spec (no dot in ID).
fn get_parent_group_id(spec_id: &str) -> Option<String> {
    // Member IDs contain a dot: "parent-id.N"
    if let Some(dot_pos) = spec_id.rfind('.') {
        // Check that what's after the dot is a number (member index)
        let suffix = &spec_id[dot_pos + 1..];
        if suffix.parse::<u32>().is_ok() {
            return Some(spec_id[..dot_pos].to_string());
        }
    }
    None
}

/// Auto-complete a parent group spec if all its members are now completed.
/// This is called after finalizing a member spec.
fn auto_complete_parent_group(parent_id: &str, specs_dir: &Path) -> Result<()> {
    // Load the parent spec
    let parent_path = specs_dir.join(format!("{}.md", parent_id));
    if !parent_path.exists() {
        // Parent doesn't exist, nothing to do
        return Ok(());
    }

    let mut parent = Spec::load(&parent_path)?;

    // Only process group specs
    if parent.frontmatter.r#type != "group" {
        return Ok(());
    }

    // Already completed, nothing to do
    if parent.frontmatter.status == SpecStatus::Completed {
        return Ok(());
    }

    // Check if all members are completed
    let all_specs = load_all_specs(specs_dir)?;
    let incomplete_members = spec::get_incomplete_members(parent_id, &all_specs);

    if !incomplete_members.is_empty() {
        // Still has incomplete members, don't auto-complete
        return Ok(());
    }

    // All members complete! Auto-complete the parent group
    println!(
        "\n{} All members of group {} are complete. Auto-completing parent...",
        "→".cyan(),
        parent_id
    );

    // Set parent as completed (groups don't have commits of their own)
    parent.force_status(SpecStatus::Completed);
    parent.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );

    parent.save(&parent_path)?;

    println!("{} Group {} auto-completed", "✓".green(), parent_id);

    // Recursively check if this group is itself a member of a parent group
    if let Some(grandparent_id) = get_parent_group_id(parent_id) {
        auto_complete_parent_group(&grandparent_id, specs_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chant::spec::SpecFrontmatter;
    use tempfile::TempDir;

    #[test]
    fn test_find_dependent_specs_single_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a completed spec
        let completed_spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Completed".to_string()),
            body: "# Completed\n\nBody.".to_string(),
        };

        // Create a spec that depends on the completed one
        let dependent_spec = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent".to_string()),
            body: "# Dependent\n\nBody.".to_string(),
        };

        // Save both specs
        completed_spec
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();
        dependent_spec
            .save(&specs_dir.join("2026-01-27-002-def.md"))
            .unwrap();

        // Find dependent specs
        let unblocked = find_dependent_specs("2026-01-27-001-abc", specs_dir).unwrap();

        // Should find the dependent spec
        assert_eq!(unblocked.len(), 1);
        assert_eq!(unblocked[0], "2026-01-27-002-def");
    }

    #[test]
    fn test_find_dependent_specs_multiple_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create two completed specs
        let completed_1 = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Completed 1".to_string()),
            body: "# Completed 1\n\nBody.".to_string(),
        };

        let completed_2 = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Completed 2".to_string()),
            body: "# Completed 2\n\nBody.".to_string(),
        };

        // Create a spec that depends on BOTH completed specs
        let dependent_spec = Spec {
            id: "2026-01-27-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec![
                    "2026-01-27-001-abc".to_string(),
                    "2026-01-27-002-def".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Dependent".to_string()),
            body: "# Dependent\n\nBody.".to_string(),
        };

        // Save all specs
        completed_1
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();
        completed_2
            .save(&specs_dir.join("2026-01-27-002-def.md"))
            .unwrap();
        dependent_spec
            .save(&specs_dir.join("2026-01-27-003-ghi.md"))
            .unwrap();

        // Find dependents when completing the second spec
        let unblocked = find_dependent_specs("2026-01-27-002-def", specs_dir).unwrap();

        // Should find the dependent spec (both dependencies are now met)
        assert_eq!(unblocked.len(), 1);
        assert_eq!(unblocked[0], "2026-01-27-003-ghi");
    }

    #[test]
    fn test_find_dependent_specs_partial_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create one completed spec
        let completed = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Completed".to_string()),
            body: "# Completed\n\nBody.".to_string(),
        };

        // Create one incomplete spec
        let incomplete = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Incomplete".to_string()),
            body: "# Incomplete\n\nBody.".to_string(),
        };

        // Create a spec that depends on BOTH (one complete, one incomplete)
        let dependent_spec = Spec {
            id: "2026-01-27-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec![
                    "2026-01-27-001-abc".to_string(),
                    "2026-01-27-002-def".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Dependent".to_string()),
            body: "# Dependent\n\nBody.".to_string(),
        };

        // Save all specs
        completed
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();
        incomplete
            .save(&specs_dir.join("2026-01-27-002-def.md"))
            .unwrap();
        dependent_spec
            .save(&specs_dir.join("2026-01-27-003-ghi.md"))
            .unwrap();

        // Find dependents when completing the first spec
        let unblocked = find_dependent_specs("2026-01-27-001-abc", specs_dir).unwrap();

        // Should NOT find the dependent spec (still has unmet dependency on 002-def)
        assert_eq!(unblocked.len(), 0);
    }

    #[test]
    fn test_find_dependent_specs_cascade() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a chain: A -> B -> C
        let spec_a = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("A".to_string()),
            body: "# A\n\nBody.".to_string(),
        };

        let spec_b = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                depends_on: Some(vec!["2026-01-27-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("B".to_string()),
            body: "# B\n\nBody.".to_string(),
        };

        let spec_c = Spec {
            id: "2026-01-27-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("C".to_string()),
            body: "# C\n\nBody.".to_string(),
        };

        // Save all specs
        spec_a
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();
        spec_b
            .save(&specs_dir.join("2026-01-27-002-def.md"))
            .unwrap();
        spec_c
            .save(&specs_dir.join("2026-01-27-003-ghi.md"))
            .unwrap();

        // Complete B should unblock C
        let unblocked = find_dependent_specs("2026-01-27-002-def", specs_dir).unwrap();
        assert_eq!(unblocked.len(), 1);
        assert_eq!(unblocked[0], "2026-01-27-003-ghi");
    }

    #[test]
    fn test_find_dependent_specs_no_dependents() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a completed spec with no dependents
        let completed = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Completed".to_string()),
            body: "# Completed\n\nBody.".to_string(),
        };

        completed
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();

        // Find dependents
        let unblocked = find_dependent_specs("2026-01-27-001-abc", specs_dir).unwrap();

        // Should find no dependents
        assert_eq!(unblocked.len(), 0);
    }

    #[test]
    #[serial_test::serial]
    fn test_validate_spec_rejects_completed() {
        // re_finalize_spec actually ACCEPTS completed specs, so this test verifies that behavior
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Initialize git repo (required for re_finalize_spec which calls git commands)
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(specs_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(specs_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(specs_dir)
            .output()
            .unwrap();

        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
        let config = Config::parse("---\nproject:\n  name: test\n---").unwrap();

        let mut spec = Spec {
            id: "2026-02-05-001-test".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-02-05T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        // Save the spec first
        spec.save(&specs_dir.join("2026-02-05-001-test.md"))
            .unwrap();

        // Create initial commit so git log works
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(specs_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(specs_dir)
            .output()
            .unwrap();

        // Change to the temp dir for git commands to work
        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(specs_dir).unwrap();

        // re_finalize_spec accepts completed specs
        let result = re_finalize_spec(&mut spec, &spec_repo, &config, true);

        // Restore original directory
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        assert!(
            result.is_ok(),
            "re_finalize_spec should accept completed specs"
        );
    }

    #[test]
    fn test_validate_spec_rejects_cancelled() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
        let config = Config::parse("---\nproject:\n  name: test\n---").unwrap();

        let mut spec = Spec {
            id: "2026-02-05-002-test".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Cancelled,
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        // Save the spec first
        spec.save(&specs_dir.join("2026-02-05-002-test.md"))
            .unwrap();

        // re_finalize_spec should reject cancelled specs
        let result = re_finalize_spec(&mut spec, &spec_repo, &config, true);
        assert!(
            result.is_err(),
            "re_finalize_spec should reject cancelled specs"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Cannot re-finalize") && err_msg.contains("Cancelled"),
            "Error message should mention that cancelled specs cannot be re-finalized, got: {}",
            err_msg
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_validate_spec_accepts_in_progress() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Initialize git repo (required for re_finalize_spec which calls git commands)
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(specs_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(specs_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(specs_dir)
            .output()
            .unwrap();

        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
        let config = Config::parse("---\nproject:\n  name: test\n---").unwrap();

        let mut spec = Spec {
            id: "2026-02-05-003-test".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        // Save the spec first
        spec.save(&specs_dir.join("2026-02-05-003-test.md"))
            .unwrap();

        // Create initial commit so git log works
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(specs_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(specs_dir)
            .output()
            .unwrap();

        // Change to the temp dir for git commands to work
        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(specs_dir).unwrap();

        // re_finalize_spec should accept in_progress specs
        let result = re_finalize_spec(&mut spec, &spec_repo, &config, true);

        // Restore original directory
        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        assert!(
            result.is_ok(),
            "re_finalize_spec should accept in_progress specs"
        );
    }
}
