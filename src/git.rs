//! Git provider abstraction for PR creation.
//!
//! Supports multiple git hosting providers (GitHub, GitLab, Bitbucket).
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/git.md
//! - ignore: false

use anyhow::{Context, Result};
use std::process::Command;

use crate::config::GitProvider;

/// Trait for git hosting providers that can create pull/merge requests.
pub trait PrProvider {
    /// Create a pull/merge request with the given title and body.
    /// Returns the URL of the created PR/MR.
    fn create_pr(&self, title: &str, body: &str) -> Result<String>;

    /// Returns the CLI tool name used by this provider.
    #[allow(dead_code)]
    fn cli_tool(&self) -> &'static str;

    /// Returns a human-readable name for this provider.
    fn name(&self) -> &'static str;
}

/// GitHub provider using the `gh` CLI.
pub struct GitHubProvider;

impl PrProvider for GitHubProvider {
    fn create_pr(&self, title: &str, body: &str) -> Result<String> {
        let output = Command::new("gh")
            .args(["pr", "create", "--title", title, "--body", body])
            .output()
            .context("Failed to run gh pr create. Is gh CLI installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create pull request: {}", stderr);
        }

        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(pr_url)
    }

    fn cli_tool(&self) -> &'static str {
        "gh"
    }

    fn name(&self) -> &'static str {
        "GitHub"
    }
}

/// GitLab provider using the `glab` CLI.
pub struct GitLabProvider;

impl PrProvider for GitLabProvider {
    fn create_pr(&self, title: &str, body: &str) -> Result<String> {
        let output = Command::new("glab")
            .args([
                "mr",
                "create",
                "--title",
                title,
                "--description",
                body,
                "--yes",
            ])
            .output()
            .context("Failed to run glab mr create. Is glab CLI installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create merge request: {}", stderr);
        }

        let mr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(mr_url)
    }

    fn cli_tool(&self) -> &'static str {
        "glab"
    }

    fn name(&self) -> &'static str {
        "GitLab"
    }
}

/// Bitbucket provider using the `bb` CLI.
pub struct BitbucketProvider;

impl PrProvider for BitbucketProvider {
    fn create_pr(&self, title: &str, body: &str) -> Result<String> {
        let output = Command::new("bb")
            .args(["pr", "create", "--title", title, "--body", body])
            .output()
            .context("Failed to run bb pr create. Is Bitbucket CLI installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create pull request: {}", stderr);
        }

        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(pr_url)
    }

    fn cli_tool(&self) -> &'static str {
        "bb"
    }

    fn name(&self) -> &'static str {
        "Bitbucket"
    }
}

/// Get the appropriate PR provider for the given config.
pub fn get_provider(provider: GitProvider) -> Box<dyn PrProvider> {
    match provider {
        GitProvider::Github => Box::new(GitHubProvider),
        GitProvider::Gitlab => Box::new(GitLabProvider),
        GitProvider::Bitbucket => Box::new(BitbucketProvider),
    }
}

/// Create a pull/merge request using the configured provider.
#[allow(dead_code)]
pub fn create_pull_request(provider: GitProvider, title: &str, body: &str) -> Result<String> {
    let pr_provider = get_provider(provider);
    pr_provider.create_pr(title, body)
}

/// Find a spec branch by constructing the full branch name from spec ID and prefix.
/// Verifies the branch exists using `git branch --list`.
#[allow(dead_code)]
pub fn find_spec_branch(spec_id: &str, branch_prefix: &str) -> Result<String> {
    let branch_name = format!("{}{}", branch_prefix, spec_id);

    let output = Command::new("git")
        .args(["branch", "--list", &branch_name])
        .output()
        .context("Failed to run git branch --list")?;

    if !output.status.success() {
        anyhow::bail!("Failed to check if branch exists");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        anyhow::bail!("Branch '{}' not found for spec {}", branch_name, spec_id);
    }

    Ok(branch_name)
}

/// Get the current branch name.
/// Returns the branch name for the current HEAD, including "HEAD" for detached HEAD state.
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to run git rev-parse")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get current branch");
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch)
}

/// Check if a branch exists in the repository.
pub fn branch_exists(branch_name: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["branch", "--list", branch_name])
        .output()
        .context("Failed to check if branch exists")?;

    if !output.status.success() {
        anyhow::bail!("Failed to check if branch exists");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

/// Checkout a specific branch or commit.
/// If branch is "HEAD", it's a detached HEAD checkout.
fn checkout_branch(branch: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    let output = Command::new("git")
        .args(["checkout", branch])
        .output()
        .context("Failed to run git checkout")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to checkout {}: {}", branch, stderr);
    }

    Ok(())
}

/// Merge a branch using fast-forward only.
/// Returns true if successful, false if there are conflicts.
fn merge_branch_ff_only(spec_branch: &str, dry_run: bool) -> Result<bool> {
    if dry_run {
        return Ok(true);
    }

    let output = Command::new("git")
        .args(["merge", "--ff-only", spec_branch])
        .output()
        .context("Failed to run git merge")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if this was a conflict (CONFLICT) or a non-ff situation
        if stderr.contains("CONFLICT") || stderr.contains("merge conflict") {
            // Abort the merge
            let _ = Command::new("git").args(["merge", "--abort"]).output();
            return Ok(false);
        }

        // Non-ff failure - branches have diverged
        anyhow::bail!(
            "{}",
            crate::merge_errors::fast_forward_conflict(
                spec_branch.trim_start_matches("chant/"),
                spec_branch,
                "main",
                &stderr
            )
        );
    }

    Ok(true)
}

/// Delete a branch.
/// Returns Ok(()) on success, or an error if deletion fails.
pub fn delete_branch(branch_name: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    let output = Command::new("git")
        .args(["branch", "-d", branch_name])
        .output()
        .context("Failed to run git branch -d")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to delete branch {}: {}", branch_name, stderr);
    }

    Ok(())
}

/// Result of a rebase operation
#[derive(Debug)]
pub struct RebaseResult {
    /// Whether rebase succeeded
    pub success: bool,
    /// Files with conflicts (if any)
    pub conflicting_files: Vec<String>,
}

/// Rebase a branch onto another branch.
/// Returns RebaseResult with success status and any conflicting files.
pub fn rebase_branch(spec_branch: &str, onto_branch: &str) -> Result<RebaseResult> {
    // First checkout the spec branch
    checkout_branch(spec_branch, false)?;

    // Attempt rebase
    let output = Command::new("git")
        .args(["rebase", onto_branch])
        .output()
        .context("Failed to run git rebase")?;

    if output.status.success() {
        return Ok(RebaseResult {
            success: true,
            conflicting_files: vec![],
        });
    }

    // Rebase failed - check for conflicts
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("CONFLICT") || stderr.contains("conflict") {
        // Get list of conflicting files
        let conflicting_files = get_conflicting_files()?;

        // Abort rebase to restore clean state
        let _ = Command::new("git").args(["rebase", "--abort"]).output();

        return Ok(RebaseResult {
            success: false,
            conflicting_files,
        });
    }

    // Other rebase error
    let _ = Command::new("git").args(["rebase", "--abort"]).output();
    anyhow::bail!("Rebase failed: {}", stderr);
}

/// Get list of files with conflicts from git status
pub fn get_conflicting_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        // Conflict markers: UU, AA, DD, AU, UD, UA, DU
        if line.len() >= 3 {
            let status = &line[0..2];
            if status.contains('U') || status == "AA" || status == "DD" {
                let file = line[3..].trim();
                files.push(file.to_string());
            }
        }
    }

    Ok(files)
}

/// Continue a rebase after conflicts have been resolved
pub fn rebase_continue() -> Result<bool> {
    let output = Command::new("git")
        .args(["rebase", "--continue"])
        .env("GIT_EDITOR", "true") // Skip editor for commit message
        .output()
        .context("Failed to run git rebase --continue")?;

    Ok(output.status.success())
}

/// Abort an in-progress rebase
pub fn rebase_abort() -> Result<()> {
    let _ = Command::new("git").args(["rebase", "--abort"]).output();
    Ok(())
}

/// Stage a file for commit
pub fn stage_file(file_path: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["add", file_path])
        .output()
        .context("Failed to run git add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage file {}: {}", file_path, stderr);
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
    let merge_success = match merge_branch_ff_only(spec_branch, dry_run) {
        Ok(success) => success,
        Err(e) => {
            // Try to return to original branch before failing
            let _ = checkout_branch(&original_branch, false);
            return Err(e);
        }
    };

    if !merge_success && !dry_run {
        // Merge had conflicts - return to original branch
        let _ = checkout_branch(&original_branch, false);
        anyhow::bail!(
            "{}",
            crate::merge_errors::merge_conflict(spec_id, spec_branch, main_branch)
        );
    }

    // Delete branch if requested and merge was successful
    let mut branch_delete_warning: Option<String> = None;
    if should_delete_branch && merge_success {
        if let Err(e) = delete_branch(spec_branch, dry_run) {
            // Log warning but don't fail overall
            branch_delete_warning = Some(format!("Warning: Failed to delete branch: {}", e));
        }
    }

    // Return to original branch (always do this, even in dry_run)
    if let Err(e) = checkout_branch(&original_branch, false) {
        anyhow::bail!(
            "Failed to return to original branch '{}': {}",
            original_branch,
            e
        );
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
#[allow(dead_code)]
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
    use tempfile::TempDir;

    #[test]
    fn test_provider_names() {
        assert_eq!(GitHubProvider.name(), "GitHub");
        assert_eq!(GitLabProvider.name(), "GitLab");
        assert_eq!(BitbucketProvider.name(), "Bitbucket");
    }

    #[test]
    fn test_provider_cli_tools() {
        assert_eq!(GitHubProvider.cli_tool(), "gh");
        assert_eq!(GitLabProvider.cli_tool(), "glab");
        assert_eq!(BitbucketProvider.cli_tool(), "bb");
    }

    #[test]
    fn test_get_provider() {
        let github = get_provider(GitProvider::Github);
        assert_eq!(github.name(), "GitHub");

        let gitlab = get_provider(GitProvider::Gitlab);
        assert_eq!(gitlab.name(), "GitLab");

        let bitbucket = get_provider(GitProvider::Bitbucket);
        assert_eq!(bitbucket.name(), "Bitbucket");
    }

    #[test]
    fn test_find_spec_branch_constructs_name() {
        // This test verifies the branch name construction logic
        // Real branch existence would be tested in integration tests
        let result = find_spec_branch("nonexistent-spec-123", "chant/");
        // We expect this to fail because the branch doesn't exist
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("chant/nonexistent-spec-123") || err_msg.contains("not found"));
    }

    #[test]
    fn test_get_current_branch_returns_string() {
        // This should work in any git repo - gets the current branch
        let result = get_current_branch();
        // In a properly initialized git repo, this should succeed
        if result.is_ok() {
            let branch = result.unwrap();
            // Should have a branch name (not empty)
            assert!(!branch.is_empty());
        }
    }

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
}
