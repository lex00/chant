//! Low-level git worktree operations.
//!
//! Provides utilities for creating, managing, and removing git worktrees.
//! These functions handle the mechanics of worktree lifecycle management.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: scale/isolation.md
//! - ignore: false

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the worktree path for a given spec ID.
///
/// This does not check whether the worktree exists.
pub fn worktree_path_for_spec(spec_id: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/chant-{}", spec_id))
}

/// Returns the worktree path for a spec if an active worktree exists.
///
/// Returns Some(path) if the worktree directory exists, None otherwise.
pub fn get_active_worktree(spec_id: &str) -> Option<PathBuf> {
    let path = worktree_path_for_spec(spec_id);
    if path.exists() && path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// Commits changes in a worktree.
///
/// # Arguments
///
/// * `worktree_path` - Path to the worktree
/// * `message` - Commit message
///
/// # Returns
///
/// Ok(commit_hash) if commit was successful, Err if failed.
pub fn commit_in_worktree(worktree_path: &Path, message: &str) -> Result<String> {
    // Stage all changes
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(worktree_path)
        .output()
        .context("Failed to stage changes in worktree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage changes: {}", stderr);
    }

    // Check if there are any changes to commit
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()
        .context("Failed to check git status in worktree")?;

    let status_output = String::from_utf8_lossy(&output.stdout);
    if status_output.trim().is_empty() {
        // No changes to commit, return the current HEAD
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(worktree_path)
            .output()
            .context("Failed to get HEAD commit")?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(hash);
    }

    // Commit the changes
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(worktree_path)
        .output()
        .context("Failed to commit changes in worktree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to commit: {}", stderr);
    }

    // Get the commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .context("Failed to get commit hash")?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(hash)
}

/// Creates a new git worktree for the given spec.
///
/// # Arguments
///
/// * `spec_id` - The specification ID (used to create unique worktree paths)
/// * `branch` - The branch name to create in the worktree
///
/// # Returns
///
/// The absolute path to the created worktree directory.
///
/// # Errors
///
/// Returns an error if:
/// - The branch already exists
/// - Git worktree creation fails (e.g., corrupted repo)
/// - Directory creation fails
pub fn create_worktree(spec_id: &str, branch: &str) -> Result<PathBuf> {
    let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Check if branch already exists
    let output = Command::new("git")
        .args(["rev-parse", "--verify", branch])
        .output()
        .context("Failed to check if branch exists")?;

    if output.status.success() {
        anyhow::bail!("Branch '{}' already exists", branch);
    }

    // Create the worktree with the new branch
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            branch,
            &worktree_path.to_string_lossy(),
        ])
        .output()
        .context("Failed to create git worktree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

    Ok(worktree_path)
}

/// Removes a git worktree and cleans up its directory.
///
/// This function is idempotent - it does not error if the worktree is already gone.
///
/// # Arguments
///
/// * `path` - The path to the worktree to remove
///
/// # Returns
///
/// Ok(()) if the worktree was successfully removed or didn't exist.
pub fn remove_worktree(path: &Path) -> Result<()> {
    // Try to remove the git worktree entry
    let _output = Command::new("git")
        .args(["worktree", "remove", &path.to_string_lossy()])
        .output()
        .context("Failed to run git worktree remove")?;

    // Even if git worktree remove fails, try to clean up the directory
    if path.exists() {
        std::fs::remove_dir_all(path)
            .context(format!("Failed to remove worktree directory at {:?}", path))?;
    }

    Ok(())
}

/// Result of a merge operation
#[derive(Debug, Clone)]
pub struct MergeCleanupResult {
    #[allow(dead_code)]
    pub success: bool,
    pub has_conflict: bool,
    pub error: Option<String>,
}

/// Merges a branch to main and cleans up.
///
/// # Arguments
///
/// * `branch` - The branch name to merge
///
/// # Returns
///
/// Returns a MergeCleanupResult indicating:
/// - success: true if merge succeeded and branch was deleted
/// - has_conflict: true if merge failed due to conflicts
/// - error: optional error message
///
/// If there are merge conflicts, the branch is preserved for manual resolution.
pub fn merge_and_cleanup(branch: &str) -> MergeCleanupResult {
    merge_and_cleanup_in_dir(branch, None)
}

/// Internal function that merges a branch to main with optional working directory.
fn merge_and_cleanup_in_dir(branch: &str, work_dir: Option<&Path>) -> MergeCleanupResult {
    // Checkout main branch
    let mut cmd = Command::new("git");
    cmd.args(["checkout", "main"]);
    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return MergeCleanupResult {
                success: false,
                has_conflict: false,
                error: Some(format!("Failed to checkout main: {}", e)),
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return MergeCleanupResult {
            success: false,
            has_conflict: false,
            error: Some(format!("Failed to checkout main: {}", stderr)),
        };
    }

    // Perform fast-forward merge
    let mut cmd = Command::new("git");
    cmd.args(["merge", "--ff-only", branch]);
    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return MergeCleanupResult {
                success: false,
                has_conflict: false,
                error: Some(format!("Failed to perform merge: {}", e)),
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if this was a conflict
        let has_conflict = stderr.contains("CONFLICT") || stderr.contains("merge conflict");

        // Abort merge if there was a conflict to preserve the branch
        if has_conflict {
            let mut cmd = Command::new("git");
            cmd.args(["merge", "--abort"]);
            if let Some(dir) = work_dir {
                cmd.current_dir(dir);
            }
            let _ = cmd.output();
        }

        // Extract spec_id from branch name (strip "chant/" prefix if present)
        let spec_id = branch.trim_start_matches("chant/");
        let error_msg = if has_conflict {
            crate::merge_errors::merge_conflict(spec_id, branch, "main")
        } else {
            crate::merge_errors::fast_forward_conflict(spec_id, branch, "main", &stderr)
        };
        return MergeCleanupResult {
            success: false,
            has_conflict,
            error: Some(error_msg),
        };
    }

    // Delete the local branch after successful merge
    let mut cmd = Command::new("git");
    cmd.args(["branch", "-d", branch]);
    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return MergeCleanupResult {
                success: false,
                has_conflict: false,
                error: Some(format!("Failed to delete branch: {}", e)),
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return MergeCleanupResult {
            success: false,
            has_conflict: false,
            error: Some(format!("Failed to delete branch '{}': {}", branch, stderr)),
        };
    }

    // Delete the remote branch (best-effort, don't fail if it doesn't exist)
    let mut cmd = Command::new("git");
    cmd.args(["push", "origin", "--delete", branch]);
    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }
    // Ignore errors - remote branch may not exist or remote may be unavailable
    let _ = cmd.output();

    MergeCleanupResult {
        success: true,
        has_conflict: false,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;

    /// Helper to initialize a temporary git repo for testing.
    fn setup_test_repo(repo_dir: &Path) -> Result<()> {
        fs::create_dir_all(repo_dir)?;

        let output = StdCommand::new("git")
            .args(["init", "-b", "main"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to run git init")?;
        anyhow::ensure!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = StdCommand::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to run git config")?;
        anyhow::ensure!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = StdCommand::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to run git config")?;
        anyhow::ensure!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create an initial commit
        fs::write(repo_dir.join("README.md"), "# Test")?;

        let output = StdCommand::new("git")
            .args(["add", "."])
            .current_dir(repo_dir)
            .output()
            .context("Failed to run git add")?;
        anyhow::ensure!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = StdCommand::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to run git commit")?;
        anyhow::ensure!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(())
    }

    /// Helper to clean up test repos.
    fn cleanup_test_repo(repo_dir: &Path) -> Result<()> {
        if repo_dir.exists() {
            fs::remove_dir_all(repo_dir)?;
        }
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_create_worktree_branch_already_exists() -> Result<()> {
        let repo_dir = PathBuf::from("/tmp/test-chant-repo-branch-exists");
        cleanup_test_repo(&repo_dir)?;
        setup_test_repo(&repo_dir)?;

        let original_dir = std::env::current_dir()?;

        let result = {
            std::env::set_current_dir(&repo_dir).context("Failed to change to repo directory")?;

            let spec_id = "test-spec-branch-exists";
            let branch = "spec/test-spec-branch-exists";

            // Create the branch first
            let output = StdCommand::new("git")
                .args(["branch", branch])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git branch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            create_worktree(spec_id, branch)
        };

        // Always restore original directory
        std::env::set_current_dir(&original_dir).context("Failed to restore original directory")?;
        cleanup_test_repo(&repo_dir)?;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_and_cleanup_with_conflict_preserves_branch() -> Result<()> {
        let repo_dir = PathBuf::from("/tmp/test-chant-repo-conflict-preserve");
        cleanup_test_repo(&repo_dir)?;
        setup_test_repo(&repo_dir)?;

        let original_dir = std::env::current_dir()?;

        let result = {
            std::env::set_current_dir(&repo_dir).context("Failed to change to repo directory")?;

            let branch = "feature/conflict-test";

            // Create a feature branch that conflicts with main
            let output = StdCommand::new("git")
                .args(["branch", branch])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git branch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let output = StdCommand::new("git")
                .args(["checkout", branch])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git checkout branch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            fs::write(repo_dir.join("README.md"), "feature version")?;

            let output = StdCommand::new("git")
                .args(["add", "."])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let output = StdCommand::new("git")
                .args(["commit", "-m", "Modify README on feature"])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git commit feature failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            // Modify README on main differently
            let output = StdCommand::new("git")
                .args(["checkout", "main"])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git checkout main failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            fs::write(repo_dir.join("README.md"), "main version")?;

            let output = StdCommand::new("git")
                .args(["add", "."])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git add main failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let output = StdCommand::new("git")
                .args(["commit", "-m", "Modify README on main"])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git commit main failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            // Now call merge_and_cleanup with explicit repo directory
            merge_and_cleanup_in_dir(branch, Some(&repo_dir))
        };

        // Always restore original directory
        std::env::set_current_dir(&original_dir).context("Failed to restore original directory")?;

        // Check that branch still exists (wasn't deleted)
        let branch_check = StdCommand::new("git")
            .args(["rev-parse", "--verify", "feature/conflict-test"])
            .current_dir(&repo_dir)
            .output()?;

        cleanup_test_repo(&repo_dir)?;

        // Merge should fail (either due to conflict or non-ff situation)
        assert!(!result.success);
        // Branch should still exist
        assert!(branch_check.status.success());
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_and_cleanup_successful_merge() -> Result<()> {
        let repo_dir = PathBuf::from("/tmp/test-chant-repo-merge-success");
        cleanup_test_repo(&repo_dir)?;
        setup_test_repo(&repo_dir)?;

        let original_dir = std::env::current_dir()?;

        let result = {
            std::env::set_current_dir(&repo_dir).context("Failed to change to repo directory")?;

            let branch = "feature/new-feature";

            // Create a fast-forwardable feature branch
            let output = StdCommand::new("git")
                .args(["branch", branch])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git branch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let output = StdCommand::new("git")
                .args(["checkout", branch])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            fs::write(repo_dir.join("feature.txt"), "feature content")?;

            let output = StdCommand::new("git")
                .args(["add", "."])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let output = StdCommand::new("git")
                .args(["commit", "-m", "Add feature"])
                .current_dir(&repo_dir)
                .output()?;
            anyhow::ensure!(
                output.status.success(),
                "git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            // Merge the branch with explicit repo directory
            merge_and_cleanup_in_dir(branch, Some(&repo_dir))
        };

        // Always restore original directory
        std::env::set_current_dir(&original_dir).context("Failed to restore original directory")?;

        // Check that branch no longer exists
        let branch_check = StdCommand::new("git")
            .args(["rev-parse", "--verify", "feature/new-feature"])
            .current_dir(&repo_dir)
            .output()?;

        cleanup_test_repo(&repo_dir)?;

        assert!(
            result.success && result.error.is_none(),
            "Merge result: {:?}",
            result
        );
        // Branch should be deleted after merge
        assert!(!branch_check.status.success());
        Ok(())
    }

    #[test]
    fn test_remove_worktree_idempotent() -> Result<()> {
        let path = PathBuf::from("/tmp/nonexistent-worktree-12345");

        // Try to remove a non-existent worktree - should succeed
        let result = remove_worktree(&path);

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_worktree_path_for_spec() {
        let path = worktree_path_for_spec("2026-01-27-001-abc");
        assert_eq!(path, PathBuf::from("/tmp/chant-2026-01-27-001-abc"));
    }

    #[test]
    fn test_get_active_worktree_nonexistent() {
        // Test with a spec ID that definitely doesn't have a worktree
        let result = get_active_worktree("nonexistent-spec-12345");
        assert!(result.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_in_worktree() -> Result<()> {
        let repo_dir = PathBuf::from("/tmp/test-chant-commit-in-worktree");
        cleanup_test_repo(&repo_dir)?;
        setup_test_repo(&repo_dir)?;

        // Create a new file
        fs::write(repo_dir.join("new_file.txt"), "content")?;

        // Commit the changes
        let result = commit_in_worktree(&repo_dir, "Test commit message");

        cleanup_test_repo(&repo_dir)?;

        assert!(result.is_ok());
        let hash = result.unwrap();
        // Commit hash should be a 40-character hex string
        assert_eq!(hash.len(), 40);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_in_worktree_no_changes() -> Result<()> {
        let repo_dir = PathBuf::from("/tmp/test-chant-commit-no-changes");
        cleanup_test_repo(&repo_dir)?;
        setup_test_repo(&repo_dir)?;

        // Don't make any changes, just try to commit
        let result = commit_in_worktree(&repo_dir, "Empty commit");

        cleanup_test_repo(&repo_dir)?;

        // Should still succeed (returns HEAD)
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 40);

        Ok(())
    }
}
