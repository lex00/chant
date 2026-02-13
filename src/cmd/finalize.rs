//! Spec finalization logic.
//!
//! Handles marking specs as complete, updating frontmatter with commits,
//! timestamps, and model information.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use crate::cmd::ui::{Output, OutputMode};
use chant::config::Config;
use chant::lock::LockGuard;
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, load_all_specs, Spec, SpecStatus, SpecType};

use chant::operations::{
    get_commits_for_spec, get_commits_for_spec_allow_no_commits, get_commits_for_spec_with_branch,
};

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
    let out = Output::new(OutputMode::Human);

    // Acquire spec-level lock to prevent concurrent finalization
    let _lock = LockGuard::new(&spec.id).context("Failed to acquire lock for spec finalization")?;

    // Call the operations layer with appropriate options
    let options = chant::operations::finalize::FinalizeOptions {
        allow_no_commits,
        commits,
        force: true, // CLI always bypasses agent log gate
    };

    // Delegate core finalization logic to operations layer
    chant::operations::finalize::finalize_spec(spec, spec_repo, config, all_specs, options)?;

    eprintln!(
        "{} [{}] Spec successfully finalized with status=Completed",
        "✓".green(),
        spec.id
    );

    // Check what this spec unblocked
    let specs_dir = spec_repo.specs_dir();
    let unblocked = find_dependent_specs(&spec.id, specs_dir)?;
    if !unblocked.is_empty() {
        out.success(&format!("Unblocked {} dependent spec(s):", unblocked.len()));
        for dependent_id in unblocked {
            out.detail(&format!("- {}", dependent_id));
        }
    }

    // Auto-complete parent group if this is a member and all siblings are complete
    if let Some(parent_id) = get_parent_group_id(&spec.id) {
        auto_complete_parent_group(&parent_id, specs_dir, &out)?;
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
    // Acquire spec-level lock to prevent concurrent finalization
    let _lock =
        LockGuard::new(&spec.id).context("Failed to acquire lock for spec re-finalization")?;

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

    // Call operations layer for re-finalization
    // Load all specs for driver/member validation
    let specs_dir = spec_repo.specs_dir();
    let all_specs = spec::load_all_specs(specs_dir)?;

    let options = chant::operations::finalize::FinalizeOptions {
        allow_no_commits,
        commits: Some(commits),
        force: true, // Bypass agent log gate for re-finalization
    };

    chant::operations::finalize::finalize_spec(spec, spec_repo, config, &all_specs, options)?;
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

/// Append agent output to the spec body, truncating if too long.
pub fn append_agent_output(spec: &mut Spec, output: &str) {
    let timestamp = chant::utc_now_iso();

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
fn auto_complete_parent_group(parent_id: &str, specs_dir: &Path, out: &Output) -> Result<()> {
    // Load the parent spec
    let parent_path = specs_dir.join(format!("{}.md", parent_id));
    if !parent_path.exists() {
        // Parent doesn't exist, nothing to do
        return Ok(());
    }

    let mut parent = Spec::load(&parent_path)?;

    // Only process group specs
    if parent.frontmatter.r#type != SpecType::Group {
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
    out.step(&format!(
        "\nAll members of group {} are complete. Auto-completing parent...",
        parent_id
    ));

    // Set parent as completed (groups don't have commits of their own)
    if let Err(e) = spec::TransitionBuilder::new(&mut parent)
        .force()
        .to(SpecStatus::Completed)
    {
        eprintln!(
            "Warning: Failed to transition parent group {} to Completed: {}",
            parent_id, e
        );
    }
    parent.frontmatter.completed_at = Some(chant::utc_now_iso());

    parent.save(&parent_path)?;

    out.success(&format!("Group {} auto-completed", parent_id));

    // Recursively check if this group is itself a member of a parent group
    if let Some(grandparent_id) = get_parent_group_id(parent_id) {
        auto_complete_parent_group(&grandparent_id, specs_dir, out)?;
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
