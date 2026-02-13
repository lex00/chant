//! Low-level git operations and wrappers.
//!
//! This module provides pure git command wrappers without dependencies on
//! spec, config, or operations modules. For high-level merge orchestration,
//! see the `git` module.

use anyhow::{Context, Result};
use std::fmt;
use std::process::Command;

/// Run a git command with arguments and return stdout on success.
///
/// # Errors
///
/// Returns an error if the command fails to execute or exits with non-zero status.
fn run_git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context(format!("Failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

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
    let branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    Ok(branch.trim().to_string())
}

/// Check if a branch exists in the repository.
pub fn branch_exists(branch_name: &str) -> Result<bool> {
    let stdout = run_git(&["branch", "--list", branch_name])?;
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
    let stdout = run_git(&["branch", "--merged", target_branch, "--list", branch_name])?;
    Ok(!stdout.trim().is_empty())
}

/// Checkout a specific branch or commit.
/// If branch is "HEAD", it's a detached HEAD checkout.
pub fn checkout_branch(branch: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    run_git(&["checkout", branch]).with_context(|| format!("Failed to checkout {}", branch))?;

    Ok(())
}

/// Check if branches have diverged (i.e., fast-forward is not possible).
///
/// Returns true if branches have diverged (fast-forward not possible).
/// Returns false if a fast-forward merge is possible.
///
/// Fast-forward is possible when HEAD is an ancestor of or equal to spec_branch.
/// Branches have diverged when HEAD has commits not in spec_branch.
pub fn branches_have_diverged(spec_branch: &str) -> Result<bool> {
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
    pub conflict_type: Option<ConflictType>,
    /// Files with conflicts if any
    pub conflicting_files: Vec<String>,
    /// Git stderr output
    pub stderr: String,
}

/// Type of merge conflict encountered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// Content conflict in file(s)
    Content,
    /// Fast-forward only merge failed due to diverged branches
    FastForward,
    /// Tree conflict (file vs directory, rename conflicts, etc)
    Tree,
    /// Unknown or unclassified conflict
    Unknown,
}

impl fmt::Display for ConflictType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictType::Content => write!(f, "content"),
            ConflictType::FastForward => write!(f, "fast-forward"),
            ConflictType::Tree => write!(f, "tree"),
            ConflictType::Unknown => write!(f, "unknown"),
        }
    }
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
pub fn merge_branch_ff_only(spec_branch: &str, dry_run: bool) -> Result<MergeAttemptResult> {
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

        let conflict_type = classify_conflict_type(&stderr, status_output.as_deref());

        let conflicting_files = status_output
            .as_deref()
            .map(parse_conflicting_files)
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

/// Classify the type of merge conflict from stderr and status output.
pub fn classify_conflict_type(stderr: &str, status_output: Option<&str>) -> ConflictType {
    let stderr_lower = stderr.to_lowercase();

    if stderr_lower.contains("not possible to fast-forward")
        || stderr_lower.contains("cannot fast-forward")
        || stderr_lower.contains("refusing to merge unrelated histories")
    {
        return ConflictType::FastForward;
    }

    if stderr_lower.contains("conflict (rename/delete)")
        || stderr_lower.contains("conflict (modify/delete)")
        || stderr_lower.contains("deleted in")
        || stderr_lower.contains("renamed in")
        || stderr_lower.contains("conflict (add/add)")
    {
        return ConflictType::Tree;
    }

    if let Some(status) = status_output {
        if status.lines().any(|line| {
            let prefix = line.get(..2).unwrap_or("");
            matches!(prefix, "DD" | "AU" | "UD" | "UA" | "DU")
        }) {
            return ConflictType::Tree;
        }

        if status.lines().any(|line| {
            let prefix = line.get(..2).unwrap_or("");
            matches!(prefix, "UU" | "AA")
        }) {
            return ConflictType::Content;
        }
    }

    if stderr_lower.contains("conflict") || stderr_lower.contains("merge conflict") {
        return ConflictType::Content;
    }

    ConflictType::Unknown
}

/// Parse conflicting files from git status --porcelain output.
pub fn parse_conflicting_files(status_output: &str) -> Vec<String> {
    let mut files = Vec::new();

    for line in status_output.lines() {
        if line.len() >= 3 {
            let status = &line[0..2];
            // Conflict markers: UU, AA, DD, AU, UD, UA, DU
            if status.contains('U') || status == "AA" || status == "DD" {
                let file = line[3..].trim();
                files.push(file.to_string());
            }
        }
    }

    files
}

/// Remove all worktrees associated with a branch.
/// This is idempotent and won't fail if no worktrees exist.
pub fn remove_worktrees_for_branch(branch_name: &str) -> Result<()> {
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

/// Delete a branch, removing associated worktrees first.
/// Returns Ok(()) on success, or an error if deletion fails.
pub fn delete_branch(branch_name: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    // Remove any worktrees associated with this branch before deleting it
    remove_worktrees_for_branch(branch_name)?;

    run_git(&["branch", "-d", branch_name])
        .with_context(|| format!("Failed to delete branch {}", branch_name))?;

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
    run_git(&["add", file_path]).with_context(|| format!("Failed to stage file {}", file_path))?;
    Ok(())
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
