//! Git operations for branch management and merging.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/git.md
//! - ignore: false

use anyhow::{Context, Result};
use std::process::Command;

/// Get a git config value by key.
///
/// Returns `Some(value)` if the config key exists and has a non-empty value,
/// `None` otherwise.
pub fn get_git_config(key: &str) -> Option<String> {
    let output = Command::new("git").args(["config", key]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Get git user name and email from config.
///
/// Returns a tuple of (user.name, user.email), where each is `Some` if configured.
pub fn get_git_user_info() -> (Option<String>, Option<String>) {
    (get_git_config("user.name"), get_git_config("user.email"))
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

/// Ensure the main repo is on the main branch.
///
/// Call this at command boundaries to prevent branch drift.
/// Uses config's main_branch setting (defaults to "main").
///
/// Warns but does not fail if checkout fails (e.g., dirty worktree).
pub fn ensure_on_main_branch(main_branch: &str) -> Result<()> {
    let current = get_current_branch()?;

    if current != main_branch {
        let output = Command::new("git")
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

/// Check if a branch has been merged into a target branch.
///
/// # Arguments
/// * `branch_name` - The branch to check
/// * `target_branch` - The target branch to check against (e.g., "main")
///
/// # Returns
/// * `Ok(true)` if the branch has been merged into the target
/// * `Ok(false)` if the branch exists but hasn't been merged
/// * `Err(_)` if git operations fail
pub fn is_branch_merged(branch_name: &str, target_branch: &str) -> Result<bool> {
    // Use git branch --merged to check if the branch is in the list of merged branches
    let output = Command::new("git")
        .args(["branch", "--merged", target_branch, "--list", branch_name])
        .output()
        .context("Failed to check if branch is merged")?;

    if !output.status.success() {
        anyhow::bail!("Failed to check if branch is merged");
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

/// Check if branches have diverged (i.e., fast-forward is not possible).
///
/// Returns true if branches have diverged (fast-forward not possible).
/// Returns false if a fast-forward merge is possible.
///
/// Fast-forward is possible when HEAD is an ancestor of or equal to spec_branch.
/// Branches have diverged when HEAD has commits not in spec_branch.
fn branches_have_diverged(spec_branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", "HEAD", spec_branch])
        .output()
        .context("Failed to check if branches have diverged")?;

    // merge-base --is-ancestor returns 0 if HEAD is ancestor of spec_branch (fast-forward possible)
    // Returns non-zero if HEAD is not ancestor of spec_branch (branches have diverged)
    Ok(!output.status.success())
}

/// Result of a merge attempt with conflict details.
#[derive(Debug)]
pub struct MergeAttemptResult {
    /// Whether merge succeeded
    pub success: bool,
    /// Type of conflict if any
    pub conflict_type: Option<crate::merge_errors::ConflictType>,
    /// Files with conflicts if any
    pub conflicting_files: Vec<String>,
    /// Git stderr output
    pub stderr: String,
}

/// Merge a branch using appropriate strategy based on divergence.
///
/// Strategy:
/// 1. Check if branches have diverged
/// 2. If diverged: Use --no-ff to create a merge commit
/// 3. If clean fast-forward possible: Use fast-forward merge
/// 4. If conflicts exist: Return details about the conflict
///
/// Returns MergeAttemptResult with success status and conflict details.
fn merge_branch_ff_only(spec_branch: &str, dry_run: bool) -> Result<MergeAttemptResult> {
    if dry_run {
        return Ok(MergeAttemptResult {
            success: true,
            conflict_type: None,
            conflicting_files: vec![],
            stderr: String::new(),
        });
    }

    // Check if branches have diverged
    let diverged = branches_have_diverged(spec_branch)?;

    let merge_message = format!("Merge {}", spec_branch);

    let mut cmd = Command::new("git");
    if diverged {
        // Branches have diverged - use --no-ff to create a merge commit
        cmd.args(["merge", "--no-ff", spec_branch, "-m", &merge_message]);
    } else {
        // Can do fast-forward merge
        cmd.args(["merge", "--ff-only", spec_branch]);
    }

    let output = cmd.output().context("Failed to run git merge")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Get git status to classify conflict and find files
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

        let conflict_type =
            crate::merge_errors::classify_conflict_type(&stderr, status_output.as_deref());

        let conflicting_files = status_output
            .as_deref()
            .map(crate::merge_errors::parse_conflicting_files)
            .unwrap_or_default();

        // Abort the merge to restore clean state
        let _ = Command::new("git").args(["merge", "--abort"]).output();

        return Ok(MergeAttemptResult {
            success: false,
            conflict_type: Some(conflict_type),
            conflicting_files,
            stderr,
        });
    }

    Ok(MergeAttemptResult {
        success: true,
        conflict_type: None,
        conflicting_files: vec![],
        stderr: String::new(),
    })
}

/// Delete a branch, removing associated worktrees first.
/// Returns Ok(()) on success, or an error if deletion fails.
pub fn delete_branch(branch_name: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    // Remove any worktrees associated with this branch before deleting it
    remove_worktrees_for_branch(branch_name)?;

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

/// Remove all worktrees associated with a branch.
/// This is idempotent and won't fail if no worktrees exist.
fn remove_worktrees_for_branch(branch_name: &str) -> Result<()> {
    // List all worktrees
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("Failed to list worktrees")?;

    if !output.status.success() {
        // If git worktree list fails, just continue (maybe not a worktree-enabled repo)
        return Ok(());
    }

    let worktree_list = String::from_utf8_lossy(&output.stdout);
    let mut current_path: Option<String> = None;
    let mut worktrees_to_remove = Vec::new();

    // Parse the porcelain output to find worktrees for this branch
    for line in worktree_list.lines() {
        if line.starts_with("worktree ") {
            current_path = Some(line.trim_start_matches("worktree ").to_string());
        } else if line.starts_with("branch ") {
            let branch = line
                .trim_start_matches("branch ")
                .trim_start_matches("refs/heads/");
            if branch == branch_name {
                if let Some(path) = current_path.take() {
                    worktrees_to_remove.push(path);
                }
            }
        }
    }

    // Remove each worktree associated with this branch
    for path in worktrees_to_remove {
        // Try with --force to handle any uncommitted changes
        let _ = Command::new("git")
            .args(["worktree", "remove", &path, "--force"])
            .output();

        // Also try to remove the directory if it still exists (in case git worktree remove failed)
        let _ = std::fs::remove_dir_all(&path);
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
    let merge_result = match merge_branch_ff_only(spec_branch, dry_run) {
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
        let conflict_type = merge_result
            .conflict_type
            .unwrap_or(crate::merge_errors::ConflictType::Unknown);

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

/// Check if branch can be fast-forward merged into target branch.
/// Returns true if the merge can be done as a fast-forward (no divergence).
pub fn can_fast_forward_merge(branch: &str, target: &str) -> Result<bool> {
    // Get merge base between branch and target
    let output = Command::new("git")
        .args(["merge-base", target, branch])
        .output()
        .context("Failed to find merge base")?;

    if !output.status.success() {
        return Ok(false);
    }

    let merge_base = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Get the commit hash of target
    let output = Command::new("git")
        .args(["rev-parse", target])
        .output()
        .context("Failed to get target commit")?;

    if !output.status.success() {
        return Ok(false);
    }

    let target_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // If merge base equals target, then branch is ahead and can ff-merge
    Ok(merge_base == target_commit)
}

/// Check if branch is behind target branch.
/// Returns true if target has commits that branch doesn't have.
pub fn is_branch_behind(branch: &str, target: &str) -> Result<bool> {
    // Get merge base
    let output = Command::new("git")
        .args(["merge-base", branch, target])
        .output()
        .context("Failed to find merge base")?;

    if !output.status.success() {
        return Ok(false);
    }

    let merge_base = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Get branch commit
    let output = Command::new("git")
        .args(["rev-parse", branch])
        .output()
        .context("Failed to get branch commit")?;

    if !output.status.success() {
        return Ok(false);
    }

    let branch_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // If merge base equals branch commit, then branch is behind target
    Ok(merge_base == branch_commit)
}

/// Count number of commits in branch.
pub fn count_commits(branch: &str) -> Result<usize> {
    let output = Command::new("git")
        .args(["rev-list", "--count", branch])
        .output()
        .context("Failed to count commits")?;

    if !output.status.success() {
        return Ok(0);
    }

    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(count_str.parse().unwrap_or(0))
}

/// Information about a single git commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

/// Get commits in a range between two refs.
///
/// Returns commits between `from_ref` and `to_ref` (inclusive of `to_ref`, exclusive of `from_ref`).
/// Uses `git log from_ref..to_ref` format.
///
/// # Errors
/// Returns error if refs are invalid or git command fails.
pub fn get_commits_in_range(from_ref: &str, to_ref: &str) -> Result<Vec<CommitInfo>> {
    let range = format!("{}..{}", from_ref, to_ref);

    let output = Command::new("git")
        .args(["log", &range, "--format=%H|%an|%at|%s", "--reverse"])
        .output()
        .context("Failed to execute git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Invalid git refs {}: {}", range, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() != 4 {
            continue;
        }

        commits.push(CommitInfo {
            hash: parts[0].to_string(),
            author: parts[1].to_string(),
            timestamp: parts[2].parse().unwrap_or(0),
            message: parts[3].to_string(),
        });
    }

    Ok(commits)
}

/// Get files changed in a specific commit.
///
/// Returns a list of file paths that were modified in the commit.
///
/// # Errors
/// Returns error if commit hash is invalid or git command fails.
pub fn get_commit_changed_files(hash: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", hash])
        .output()
        .context("Failed to execute git diff-tree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Invalid commit hash {}: {}", hash, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(files)
}

/// Get files changed in a commit with their status (A/M/D).
///
/// Returns a list of strings in the format "STATUS:filename" (e.g., "A:file.txt", "M:file.txt").
///
/// # Errors
/// Returns error if commit hash is invalid or git command fails.
pub fn get_commit_files_with_status(hash: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-status", "-r", hash])
        .output()
        .context("Failed to execute git diff-tree")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            // parts[0] is status (A, M, D), parts[1] is filename
            files.push(format!("{}:{}", parts[0], parts[1]));
        }
    }

    Ok(files)
}

/// Get file content at a specific commit.
///
/// Returns the file content as a string, or an empty string if the file doesn't exist at that commit.
///
/// # Errors
/// Returns error if git command fails.
pub fn get_file_at_commit(commit: &str, file: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit, file)])
        .output()
        .context("Failed to get file at commit")?;

    if !output.status.success() {
        return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get file content at parent commit.
///
/// Returns the file content as a string, or an empty string if the file doesn't exist at parent commit.
///
/// # Errors
/// Returns error if git command fails.
pub fn get_file_at_parent(commit: &str, file: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["show", &format!("{}^:{}", commit, file)])
        .output()
        .context("Failed to get file at parent")?;

    if !output.status.success() {
        return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get the N most recent commits.
///
/// # Errors
/// Returns error if git command fails.
pub fn get_recent_commits(count: usize) -> Result<Vec<CommitInfo>> {
    let count_str = count.to_string();

    let output = Command::new("git")
        .args(["log", "-n", &count_str, "--format=%H|%an|%at|%s"])
        .output()
        .context("Failed to execute git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to get recent commits: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() != 4 {
            continue;
        }

        commits.push(CommitInfo {
            hash: parts[0].to_string(),
            author: parts[1].to_string(),
            timestamp: parts[2].parse().unwrap_or(0),
            message: parts[3].to_string(),
        });
    }

    Ok(commits)
}

/// Get commits that modified a specific path.
///
/// # Arguments
/// * `path` - File or directory path to filter by
///
/// # Errors
/// Returns error if git command fails.
pub fn get_commits_for_path(path: &str) -> Result<Vec<CommitInfo>> {
    let output = Command::new("git")
        .args(["log", "--all", "--format=%H|%an|%at|%s", "--", path])
        .output()
        .context("Failed to execute git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() != 4 {
            continue;
        }

        commits.push(CommitInfo {
            hash: parts[0].to_string(),
            author: parts[1].to_string(),
            timestamp: parts[2].parse().unwrap_or(0),
            message: parts[3].to_string(),
        });
    }

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_current_branch_returns_string() {
        // This should work in any git repo - gets the current branch
        let result = get_current_branch();
        // In a properly initialized git repo, this should succeed
        if let Ok(branch) = result {
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

    #[test]
    #[serial_test::serial]
    fn test_branches_have_diverged_no_divergence() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch that's ahead of main
        Command::new("git")
            .args(["checkout", "-b", "spec-no-diverge"])
            .output()?;

        // Make a change on spec branch
        let file_path = repo_path.join("diverge-test.txt");
        fs::write(&file_path, "spec content")?;
        Command::new("git")
            .args(["add", "diverge-test.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add diverge-test"])
            .output()?;

        // Go back to main
        Command::new("git").args(["checkout", "main"]).output()?;

        // Test divergence check - spec branch is ancestor of main, so no divergence
        let diverged = branches_have_diverged("spec-no-diverge")?;
        assert!(!diverged, "Fast-forward merge should be possible");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_branches_have_diverged_with_divergence() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a spec branch from main
        Command::new("git")
            .args(["checkout", "-b", "spec-diverge"])
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

        // Go back to main and make a different change
        Command::new("git").args(["checkout", "main"]).output()?;
        let main_file = repo_path.join("main-file.txt");
        fs::write(&main_file, "main content")?;
        Command::new("git")
            .args(["add", "main-file.txt"])
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Add main-file"])
            .output()?;

        // Test divergence check - branches have diverged
        let diverged = branches_have_diverged("spec-diverge")?;
        assert!(diverged, "Branches should have diverged");

        std::env::set_current_dir(original_dir)?;
        Ok(())
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

    #[test]
    #[serial_test::serial]
    fn test_get_commits_in_range() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create additional commits
        for i in 1..=5 {
            let file_path = repo_path.join(format!("test{}.txt", i));
            fs::write(&file_path, format!("content {}", i))?;
            Command::new("git").args(["add", "."]).output()?;
            Command::new("git")
                .args(["commit", "-m", &format!("Commit {}", i)])
                .output()?;
        }

        // Get commits in range
        let commits = get_commits_in_range("HEAD~5", "HEAD")?;

        assert_eq!(commits.len(), 5);
        assert_eq!(commits[0].message, "Commit 1");
        assert_eq!(commits[4].message, "Commit 5");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_commits_in_range_invalid_refs() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        let result = get_commits_in_range("invalid", "HEAD");
        assert!(result.is_err());

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_commits_in_range_empty() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Same ref should return empty
        let commits = get_commits_in_range("HEAD", "HEAD")?;
        assert_eq!(commits.len(), 0);

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_commit_changed_files() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create a commit with multiple files
        let file1 = repo_path.join("file1.txt");
        let file2 = repo_path.join("file2.txt");
        fs::write(&file1, "content1")?;
        fs::write(&file2, "content2")?;
        Command::new("git").args(["add", "."]).output()?;
        Command::new("git")
            .args(["commit", "-m", "Add files"])
            .output()?;

        let hash_output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
        let hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        let files = get_commit_changed_files(&hash)?;
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"file2.txt".to_string()));

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_commit_changed_files_invalid_hash() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        let result = get_commit_changed_files("invalid_hash");
        assert!(result.is_err());

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_commit_changed_files_empty() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create an empty commit
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Empty commit"])
            .output()?;

        let hash_output = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
        let hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        let files = get_commit_changed_files(&hash)?;
        assert_eq!(files.len(), 0);

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_recent_commits() -> Result<()> {
        let temp_dir = setup_test_repo()?;
        let repo_path = temp_dir.path();
        let original_dir = std::env::current_dir()?;

        std::env::set_current_dir(repo_path)?;

        // Create additional commits
        for i in 1..=5 {
            let file_path = repo_path.join(format!("test{}.txt", i));
            fs::write(&file_path, format!("content {}", i))?;
            Command::new("git").args(["add", "."]).output()?;
            Command::new("git")
                .args(["commit", "-m", &format!("Recent {}", i)])
                .output()?;
        }

        // Get 3 recent commits
        let commits = get_recent_commits(3)?;
        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0].message, "Recent 5");
        assert_eq!(commits[1].message, "Recent 4");
        assert_eq!(commits[2].message, "Recent 3");

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }
}
