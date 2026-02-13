//! Git operations for branch management and merging.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/git.md
//! - ignore: false

use anyhow::{Context, Result};

// Re-export low-level git operations for backward compatibility
pub use crate::git_ops::{
    branch_exists, can_fast_forward_merge, checkout_branch, count_commits, delete_branch,
    get_commit_changed_files, get_commit_files_with_status, get_commits_for_path,
    get_commits_in_range, get_conflicting_files, get_current_branch, get_file_at_commit,
    get_file_at_parent, get_git_config, get_git_user_info, get_recent_commits, is_branch_behind,
    is_branch_merged, rebase_abort, rebase_branch, rebase_continue, stage_file, CommitInfo,
    ConflictType, MergeAttemptResult, RebaseResult,
};

/// Ensure the main repo is on the main branch.
///
/// Call this at command boundaries to prevent branch drift.
/// Uses config's main_branch setting (defaults to "main").
///
/// Warns but does not fail if checkout fails (e.g., dirty worktree).
pub fn ensure_on_main_branch(main_branch: &str) -> Result<()> {
    let current = get_current_branch()?;

    if current != main_branch {
        let output = std::process::Command::new("git")
            .args(["checkout", main_branch])
            .output()
            .context("Failed to checkout main branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail hard - just warn
            eprintln!("Warning: Could not return to {}: {}", main_branch, stderr);
        }
    }

    Ok(())
}

/// Merge a single spec's branch into the main branch.
///
/// This function:
/// 1. Saves the current branch
/// 2. Checks if main branch exists
/// 3. Checks out main branch
/// 4. Merges spec branch with fast-forward only
/// 5. Optionally deletes spec branch if requested
/// 6. Returns to original branch
///
/// In dry-run mode, no actual git commands are executed.
pub fn merge_single_spec(
    spec_id: &str,
    spec_branch: &str,
    main_branch: &str,
    should_delete_branch: bool,
    dry_run: bool,
) -> Result<MergeResult> {
    // In dry_run mode, try to get current branch but don't fail if we're not in a repo
    if dry_run {
        let original_branch = get_current_branch().unwrap_or_default();
        return Ok(MergeResult {
            spec_id: spec_id.to_string(),
            success: true,
            original_branch,
            merged_to: main_branch.to_string(),
            branch_deleted: should_delete_branch,
            branch_delete_warning: None,
            dry_run: true,
        });
    }

    // Save current branch
    let original_branch = get_current_branch()?;

    // Check if main branch exists
    if !dry_run && !branch_exists(main_branch)? {
        anyhow::bail!(
            "{}",
            crate::merge_errors::main_branch_not_found(main_branch)
        );
    }

    // Check if spec branch exists
    if !dry_run && !branch_exists(spec_branch)? {
        anyhow::bail!(
            "{}",
            crate::merge_errors::branch_not_found(spec_id, spec_branch)
        );
    }

    // Checkout main branch
    if let Err(e) = checkout_branch(main_branch, dry_run) {
        // Try to return to original branch before failing
        let _ = checkout_branch(&original_branch, false);
        return Err(e);
    }

    // Perform merge
    let merge_result = match crate::git_ops::merge_branch_ff_only(spec_branch, dry_run) {
        Ok(result) => result,
        Err(e) => {
            // Try to return to original branch before failing
            let _ = checkout_branch(&original_branch, false);
            return Err(e);
        }
    };

    if !merge_result.success && !dry_run {
        // Merge had conflicts - return to original branch
        let _ = checkout_branch(&original_branch, false);

        // Use detailed error message with conflict type and file list
        let conflict_type = merge_result.conflict_type.unwrap_or(ConflictType::Unknown);

        anyhow::bail!(
            "{}",
            crate::merge_errors::merge_conflict_detailed(
                spec_id,
                spec_branch,
                main_branch,
                conflict_type,
                &merge_result.conflicting_files
            )
        );
    }

    let merge_success = merge_result.success;

    // Delete branch if requested and merge was successful
    let mut branch_delete_warning: Option<String> = None;
    let mut branch_actually_deleted = false;
    if should_delete_branch && merge_success {
        if let Err(e) = delete_branch(spec_branch, dry_run) {
            // Log warning but don't fail overall
            branch_delete_warning = Some(format!("Warning: Failed to delete branch: {}", e));
        } else {
            branch_actually_deleted = true;
        }
    }

    // Clean up worktree after successful merge
    if merge_success && !dry_run {
        use crate::worktree::git_ops::{get_active_worktree, remove_worktree};

        // Load config to get project name
        if let Ok(config) = crate::config::Config::load() {
            let project_name = Some(config.project.name.as_str());
            if let Some(worktree_path) = get_active_worktree(spec_id, project_name) {
                if let Err(e) = remove_worktree(&worktree_path) {
                    // Log warning but don't fail the merge
                    eprintln!(
                        "Warning: Failed to clean up worktree at {:?}: {}",
                        worktree_path, e
                    );
                }
            }
        }
    }

    // Return to original branch, BUT not if:
    // 1. We're already on main (no need to switch)
    // 2. The original branch was the spec branch that we just deleted
    let should_checkout_original = original_branch != main_branch
        && !(branch_actually_deleted && original_branch == spec_branch);

    if should_checkout_original {
        if let Err(e) = checkout_branch(&original_branch, false) {
            // If we can't checkout the original branch, stay on main
            // This can happen if the original branch was deleted elsewhere
            eprintln!(
                "Warning: Could not return to original branch '{}': {}. Staying on {}.",
                original_branch, e, main_branch
            );
        }
    }

    Ok(MergeResult {
        spec_id: spec_id.to_string(),
        success: merge_success,
        original_branch,
        merged_to: main_branch.to_string(),
        branch_deleted: should_delete_branch && merge_success,
        branch_delete_warning,
        dry_run,
    })
}

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub spec_id: String,
    pub success: bool,
    pub original_branch: String,
    pub merged_to: String,
    pub branch_deleted: bool,
    pub branch_delete_warning: Option<String>,
    pub dry_run: bool,
}

/// Format the merge result as a human-readable summary.
pub fn format_merge_summary(result: &MergeResult) -> String {
    let mut output = String::new();

    if result.dry_run {
        output.push_str("[DRY RUN] ");
    }

    if result.success {
        output.push_str(&format!(
            "✓ Successfully merged {} to {}",
            result.spec_id, result.merged_to
        ));
        if result.branch_deleted {
            output.push_str(&format!(" and deleted branch {}", result.spec_id));
        }
    } else {
        output.push_str(&format!(
            "✗ Failed to merge {} to {}",
            result.spec_id, result.merged_to
        ));
    }

    if let Some(warning) = &result.branch_delete_warning {
        output.push_str(&format!("\n  {}", warning));
    }

    output.push_str(&format!("\nReturned to branch: {}", result.original_branch));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // Helper function to initialize a mock git repo for testing
    fn setup_test_repo() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()?;

        // Configure git
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;

        // Create initial commit
        let file_path = repo_path.join("test.txt");
        fs::write(&file_path, "test content")?;
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()?;

        // Create and checkout main branch
        Command::new("git")
            .args(["branch", "main"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo_path)
            .output()?;

        Ok(temp_dir)
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_single_spec_successful_dry_run() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch
        Command::new("git")
            .args(["checkout", "-b", "spec-001"])
            .output()?;

        // Make a change on spec branch
        let file_path = repo_path.join("spec-file.txt");
        fs::write(&file_path, "spec content")?;
        Command::new("git")
            .args(["add", "spec-file.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add spec-file"])
            .output()?;

        // Go back to main
        Command::new("git").args(["checkout", "main"]).output()?;

        // Test merge with dry-run
        let result = merge_single_spec("spec-001", "spec-001", "main", false, true)?;

        assert!(result.success);
        assert!(result.dry_run);
        assert_eq!(result.spec_id, "spec-001");
        assert_eq!(result.merged_to, "main");
        assert_eq!(result.original_branch, "main");

        // Verify we're still on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        // Verify spec branch still exists (because of dry-run)
        assert!(branch_exists("spec-001")?);

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_single_spec_successful_with_delete() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch
        Command::new("git")
            .args(["checkout", "-b", "spec-002"])
            .output()?;

        // Make a change on spec branch
        let file_path = repo_path.join("spec-file2.txt");
        fs::write(&file_path, "spec content 2")?;
        Command::new("git")
            .args(["add", "spec-file2.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add spec-file2"])
            .output()?;

        // Go back to main
        Command::new("git").args(["checkout", "main"]).output()?;

        // Test merge with delete
        let result = merge_single_spec("spec-002", "spec-002", "main", true, false)?;

        assert!(result.success);
        assert!(!result.dry_run);
        assert!(result.branch_deleted);

        // Verify branch was deleted
        assert!(!branch_exists("spec-002")?);

        // Verify we're back on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_single_spec_nonexistent_main_branch() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch
        Command::new("git")
            .args(["checkout", "-b", "spec-003"])
            .output()?;

        // Make a change on spec branch
        let file_path = repo_path.join("spec-file3.txt");
        fs::write(&file_path, "spec content 3")?;
        Command::new("git")
            .args(["add", "spec-file3.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add spec-file3"])
            .output()?;

        // Test merge with nonexistent main branch
        let result = merge_single_spec("spec-003", "spec-003", "nonexistent", false, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        // Verify we're still on spec-003
        let current = get_current_branch()?;
        assert_eq!(current, "spec-003");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_single_spec_nonexistent_spec_branch() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Test merge with nonexistent spec branch
        let result = merge_single_spec("nonexistent", "nonexistent", "main", false, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        // Verify we're still on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    fn test_format_merge_summary_success() {
        let result = MergeResult {
            spec_id: "spec-001".to_string(),
            success: true,
            original_branch: "main".to_string(),
            merged_to: "main".to_string(),
            branch_deleted: false,
            branch_delete_warning: None,
            dry_run: false,
        };

        let summary = format_merge_summary(&result);
        assert!(summary.contains("✓"));
        assert!(summary.contains("spec-001"));
        assert!(summary.contains("Returned to branch: main"));
    }

    #[test]
    fn test_format_merge_summary_with_delete() {
        let result = MergeResult {
            spec_id: "spec-002".to_string(),
            success: true,
            original_branch: "main".to_string(),
            merged_to: "main".to_string(),
            branch_deleted: true,
            branch_delete_warning: None,
            dry_run: false,
        };

        let summary = format_merge_summary(&result);
        assert!(summary.contains("✓"));
        assert!(summary.contains("deleted branch spec-002"));
    }

    #[test]
    fn test_format_merge_summary_dry_run() {
        let result = MergeResult {
            spec_id: "spec-003".to_string(),
            success: true,
            original_branch: "main".to_string(),
            merged_to: "main".to_string(),
            branch_deleted: false,
            branch_delete_warning: None,
            dry_run: true,
        };

        let summary = format_merge_summary(&result);
        assert!(summary.contains("[DRY RUN]"));
    }

    #[test]
    fn test_format_merge_summary_with_warning() {
        let result = MergeResult {
            spec_id: "spec-004".to_string(),
            success: true,
            original_branch: "main".to_string(),
            merged_to: "main".to_string(),
            branch_deleted: false,
            branch_delete_warning: Some("Warning: Could not delete branch".to_string()),
            dry_run: false,
        };

        let summary = format_merge_summary(&result);
        assert!(summary.contains("Warning"));
    }

    #[test]
    fn test_format_merge_summary_failure() {
        let result = MergeResult {
            spec_id: "spec-005".to_string(),
            success: false,
            original_branch: "main".to_string(),
            merged_to: "main".to_string(),
            branch_deleted: false,
            branch_delete_warning: None,
            dry_run: false,
        };

        let summary = format_merge_summary(&result);
        assert!(summary.contains("✗"));
        assert!(summary.contains("Failed to merge"));
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_single_spec_with_diverged_branches() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch from main
        Command::new("git")
            .args(["checkout", "-b", "spec-diverged"])
            .output()?;

        // Make a change on spec branch
        let file_path = repo_path.join("spec-change.txt");
        fs::write(&file_path, "spec content")?;
        Command::new("git")
            .args(["add", "spec-change.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add spec-change"])
            .output()?;

        // Go back to main and make a different change
        Command::new("git").args(["checkout", "main"]).output()?;
        let main_file = repo_path.join("main-change.txt");
        fs::write(&main_file, "main content")?;
        Command::new("git")
            .args(["add", "main-change.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add main-change"])
            .output()?;

        // Merge with diverged branches - should use --no-ff automatically
        let result = merge_single_spec("spec-diverged", "spec-diverged", "main", false, false)?;

        assert!(result.success, "Merge should succeed with --no-ff");
        assert_eq!(result.spec_id, "spec-diverged");
        assert_eq!(result.merged_to, "main");

        // Verify we're back on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_ensure_on_main_branch() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch
        Command::new("git")
            .args(["checkout", "-b", "spec-test"])
            .output()?;

        // Verify we're on spec-test
        let current = get_current_branch()?;
        assert_eq!(current, "spec-test");

        // Call ensure_on_main_branch - should switch back to main
        ensure_on_main_branch("main")?;

        // Verify we're back on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_ensure_on_main_branch_already_on_main() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Verify we're on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        // Call ensure_on_main_branch - should be a no-op
        ensure_on_main_branch("main")?;

        // Verify we're still on main
        let current = get_current_branch()?;
        assert_eq!(current, "main");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }
}
