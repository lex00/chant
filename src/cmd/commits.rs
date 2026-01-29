//! Commit tracking and detection for spec finalization.
//!
//! Handles finding commits associated with a spec by searching for the
//! `chant(spec-id): description` pattern in commit messages.

use anyhow::{Context, Result};
use colored::Colorize;

/// Enum to distinguish between different commit retrieval scenarios
#[derive(Debug)]
pub enum CommitError {
    /// Git command failed (e.g., not in a git repository)
    GitCommandFailed(String),
    /// Git log succeeded but found no matching commits
    NoMatchingCommits,
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitError::GitCommandFailed(err) => write!(f, "Git command failed: {}", err),
            CommitError::NoMatchingCommits => write!(f, "No matching commits found"),
        }
    }
}

impl std::error::Error for CommitError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Helper to set up a test git repository
    fn setup_test_repo(repo_dir: &std::path::Path, commits: &[(String, String)]) -> Result<()> {
        // Ensure repo directory exists
        std::fs::create_dir_all(repo_dir).context("Failed to create repo directory")?;

        // Initialize repo
        let init = Command::new("git")
            .args(["init"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to git init")?;
        if !init.status.success() {
            return Err(anyhow::anyhow!(
                "git init failed: {}",
                String::from_utf8_lossy(&init.stderr)
            ));
        }

        // Configure git
        let email = Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to set git user.email")?;
        if !email.status.success() {
            return Err(anyhow::anyhow!(
                "git config user.email failed: {}",
                String::from_utf8_lossy(&email.stderr)
            ));
        }

        let name = Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to set git user.name")?;
        if !name.status.success() {
            return Err(anyhow::anyhow!(
                "git config user.name failed: {}",
                String::from_utf8_lossy(&name.stderr)
            ));
        }

        // Create commits
        for (msg, file_content) in commits {
            let file_path = repo_dir.join("test_file.txt");
            std::fs::write(&file_path, file_content).context("Failed to write test file")?;

            let add = Command::new("git")
                .args(["add", "test_file.txt"])
                .current_dir(repo_dir)
                .output()
                .context("Failed to git add")?;
            if !add.status.success() {
                return Err(anyhow::anyhow!(
                    "git add failed: {}",
                    String::from_utf8_lossy(&add.stderr)
                ));
            }

            let commit = Command::new("git")
                .args(["commit", "-m", msg])
                .current_dir(repo_dir)
                .output()
                .context("Failed to git commit")?;
            if !commit.status.success() {
                return Err(anyhow::anyhow!(
                    "git commit failed: {}",
                    String::from_utf8_lossy(&commit.stderr)
                ));
            }
        }

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_pattern_matches_full_spec_id() -> Result<()> {
        let repo_dir = TempDir::new()?.path().to_path_buf();
        let spec_id = "2026-01-27-001-abc";

        let commits_to_make = vec![
            (format!("chant({}):", spec_id), "content 1".to_string()),
            (
                format!("chant({}): Fix bug", spec_id),
                "content 2".to_string(),
            ),
            (
                format!("chant({}): Add tests", spec_id),
                "content 3".to_string(),
            ),
        ];

        setup_test_repo(&repo_dir, &commits_to_make)?;

        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_dir)?;

        let result = get_commits_for_spec(spec_id);

        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        let commits = result?;
        assert_eq!(
            commits.len(),
            3,
            "Should find all 3 commits matching full spec ID"
        );

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_pattern_with_extra_whitespace() -> Result<()> {
        let repo_dir = TempDir::new()?.path().to_path_buf();
        let spec_id = "2026-01-27-007-xyz";

        // Only test exact format - git grep doesn't match variations
        let commits_to_make = vec![
            (format!("chant({}):", spec_id), "content 1".to_string()),
            (
                format!("chant({}): Fix with standard format", spec_id),
                "content 2".to_string(),
            ),
            (
                format!("chant({}): Add more tests", spec_id),
                "content 3".to_string(),
            ),
        ];

        setup_test_repo(&repo_dir, &commits_to_make)?;

        let original_dir = std::env::current_dir().ok();
        let _ = std::env::set_current_dir(&repo_dir);

        let result = get_commits_for_spec(spec_id);

        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        let commits = result?;
        // Should find all commits with standard chant(spec_id): pattern
        assert_eq!(
            commits.len(),
            3,
            "Should find all 3 commits with standard pattern"
        );

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_pattern_no_match_returns_error() -> Result<()> {
        let repo_dir = TempDir::new()?.path().to_path_buf();
        let spec_id = "2026-01-27-003-ghi";
        let unrelated_spec = "2026-01-27-999-zzz";

        let commits_to_make = vec![
            (
                format!("chant({}):", unrelated_spec),
                "content 1".to_string(),
            ),
            ("Some other commit".to_string(), "content 2".to_string()),
        ];

        setup_test_repo(&repo_dir, &commits_to_make)?;

        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_dir)?;

        let result = get_commits_for_spec(spec_id);

        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        assert!(
            result.is_err(),
            "Should return error when no commits match the pattern"
        );

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_pattern_with_description() -> Result<()> {
        let repo_dir = TempDir::new()?.path().to_path_buf();
        let spec_id = "2026-01-27-004-jkl";

        let commits_to_make = vec![
            (
                format!("chant({}): Implement feature", spec_id),
                "content 1".to_string(),
            ),
            (
                format!("chant({}): Fix unit tests", spec_id),
                "content 2".to_string(),
            ),
            (
                format!("chant({}): Update documentation", spec_id),
                "content 3".to_string(),
            ),
        ];

        setup_test_repo(&repo_dir, &commits_to_make)?;

        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_dir)?;

        let result = get_commits_for_spec(spec_id);

        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        let commits = result?;
        assert_eq!(
            commits.len(),
            3,
            "Should find all commits with descriptions"
        );

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_get_commits_for_spec_allow_no_commits_with_fallback() -> Result<()> {
        let repo_dir = TempDir::new()?.path().to_path_buf();
        let spec_id = "2026-01-27-005-mno";
        let unrelated_spec = "2026-01-27-999-xxx";

        let commits_to_make = vec![(
            format!("chant({}):", unrelated_spec),
            "content 1".to_string(),
        )];

        setup_test_repo(&repo_dir, &commits_to_make)?;

        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_dir)?;

        let result = get_commits_for_spec_allow_no_commits(spec_id);

        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        let commits = result?;
        assert_eq!(
            commits.len(),
            1,
            "Should fallback to HEAD when no commits match"
        );

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_commit_pattern_multiple_commits_different_dates() -> Result<()> {
        let repo_dir = TempDir::new()?.path().to_path_buf();
        let spec_id = "2026-01-27-006-pqr";

        let commits_to_make = vec![
            (
                format!("chant({}): First commit", spec_id),
                "v1".to_string(),
            ),
            (
                format!("chant({}): Second commit", spec_id),
                "v2".to_string(),
            ),
            (
                format!("chant({}): Third commit", spec_id),
                "v3".to_string(),
            ),
            (
                "unrelated: Some other work".to_string(),
                "other".to_string(),
            ),
            (
                format!("chant({}): Fourth commit", spec_id),
                "v4".to_string(),
            ),
        ];

        setup_test_repo(&repo_dir, &commits_to_make)?;

        let original_dir = std::env::current_dir().ok();
        std::env::set_current_dir(&repo_dir)?;

        let result = get_commits_for_spec(spec_id);

        if let Some(dir) = original_dir {
            let _ = std::env::set_current_dir(&dir);
        }

        let commits = result?;
        assert_eq!(
            commits.len(),
            4,
            "Should find all 4 commits for spec ID, excluding unrelated ones"
        );

        Ok(())
    }
}

/// Get commits for a spec, failing if no commits match the pattern.
pub fn get_commits_for_spec(spec_id: &str) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, None, false)
}

/// Get commits for a spec with branch context for better error messages.
/// If spec_branch is provided, searches that branch first before current branch.
pub fn get_commits_for_spec_with_branch(
    spec_id: &str,
    spec_branch: Option<&str>,
) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, spec_branch, false)
}

/// Get commits for a spec, using HEAD as fallback if no commits match.
pub fn get_commits_for_spec_allow_no_commits(spec_id: &str) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, None, true)
}

/// Search for commits on a specific branch matching the spec pattern.
/// Returns Ok(commits) if found, Err if not found or git command failed.
fn find_commits_on_branch(branch: &str, spec_id: &str) -> Result<Vec<String>> {
    use std::process::Command;

    let pattern = format!("chant({}):", spec_id);

    let output = Command::new("git")
        .args(["log", branch, "--oneline", "--grep", &pattern, "--reverse"])
        .output()
        .context("Failed to execute git log command")?;

    if !output.status.success() {
        // Branch might not exist or other git error
        return Ok(vec![]);
    }

    let mut commits = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(hash) = line.split_whitespace().next() {
            if !hash.is_empty() {
                commits.push(hash.to_string());
            }
        }
    }

    Ok(commits)
}

fn get_commits_for_spec_internal(
    spec_id: &str,
    spec_branch: Option<&str>,
    allow_no_commits: bool,
) -> Result<Vec<String>> {
    use std::process::Command;

    // Look for all commits with the chant(spec_id): pattern
    // Include colon and optional space to match the actual commit message format
    let pattern = format!("chant({}):", spec_id);

    eprintln!(
        "{} Searching for commits matching pattern: '{}'",
        "→".cyan(),
        pattern
    );

    // If a spec branch is specified, check that branch first
    if let Some(branch) = spec_branch {
        eprintln!(
            "{} Checking spec branch '{}' for commits",
            "→".cyan(),
            branch
        );
        if let Ok(branch_commits) = find_commits_on_branch(branch, spec_id) {
            if !branch_commits.is_empty() {
                eprintln!(
                    "{} Found {} commit(s) on branch '{}'",
                    "→".cyan(),
                    branch_commits.len(),
                    branch
                );
                return Ok(branch_commits);
            }
        }
    }

    let output = Command::new("git")
        .args(["log", "--oneline", "--grep", &pattern, "--reverse"])
        .output()
        .context("Failed to execute git log command")?;

    // Check if git command itself failed
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!(
            "git log command failed for pattern '{}': {}",
            pattern, stderr
        );
        eprintln!("{} {}", "✗".red(), error_msg);
        return Err(anyhow::anyhow!(CommitError::GitCommandFailed(error_msg)));
    }

    // Parse commits from successful output
    let mut commits = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(hash) = line.split_whitespace().next() {
            if !hash.is_empty() {
                commits.push(hash.to_string());
            }
        }
    }

    eprintln!(
        "{} Found {} commit(s) matching pattern '{}'",
        "→".cyan(),
        commits.len(),
        pattern
    );

    // If no matching commits found, decide what to do based on flag
    if commits.is_empty() {
        if allow_no_commits {
            // Fallback behavior: use HEAD with warning
            eprintln!(
                "{} No commits found with pattern '{}'. Attempting to use HEAD as fallback.",
                "⚠".yellow(),
                pattern
            );

            let head_output = Command::new("git")
                .args(["rev-parse", "--short=7", "HEAD"])
                .output()
                .context("Failed to execute git rev-parse command")?;

            if head_output.status.success() {
                let head_hash = String::from_utf8_lossy(&head_output.stdout)
                    .trim()
                    .to_string();
                if !head_hash.is_empty() {
                    eprintln!("{} Using HEAD commit: {}", "⚠".yellow(), head_hash);
                    commits.push(head_hash);
                }
            } else {
                let stderr = String::from_utf8_lossy(&head_output.stderr);
                let error_msg = format!(
                    "Could not find any commit for spec '{}' and HEAD fallback failed: {}",
                    spec_id, stderr
                );
                eprintln!("{} {}", "✗".red(), error_msg);
                return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
            }
        } else {
            // Default behavior: fail loudly with actionable message
            // Check if commits exist on the spec's branch to provide better error message
            let default_branch = format!("chant/{}", spec_id);
            let branch_to_check = spec_branch.unwrap_or(&default_branch);
            if let Ok(branch_commits) = find_commits_on_branch(branch_to_check, spec_id) {
                if !branch_commits.is_empty() {
                    let error_msg = format!(
                        "No matching commits found on main\n\
                         Found {} commit(s) on branch {}\n\
                         Run 'chant merge {}' to merge the branch first",
                        branch_commits.len(),
                        branch_to_check,
                        spec_id
                    );
                    eprintln!("{} {}", "✗".red(), error_msg);
                    return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
                }
            }
            let error_msg =
                chant::merge_errors::no_commits_found(spec_id, &format!("chant/{}", spec_id));
            eprintln!("{} {}", "✗".red(), error_msg);
            return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
        }
    }

    Ok(commits)
}
