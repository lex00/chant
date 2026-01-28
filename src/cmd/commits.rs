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

/// Get commits for a spec, failing if no commits match the pattern.
pub fn get_commits_for_spec(spec_id: &str) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, false)
}

/// Get commits for a spec, using HEAD as fallback if no commits match.
pub fn get_commits_for_spec_allow_no_commits(spec_id: &str) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, true)
}

fn get_commits_for_spec_internal(spec_id: &str, allow_no_commits: bool) -> Result<Vec<String>> {
    use std::process::Command;

    // Look for all commits with the chant(spec_id) pattern
    let pattern = format!("chant({})", spec_id);

    let output = Command::new("git")
        .args(["log", "--oneline", "--grep", &pattern, "--reverse"])
        .output()
        .context("Failed to execute git log command")?;

    // Check if git command itself failed
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!(
            "git log command failed for pattern 'chant({})': {}",
            spec_id, stderr
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

    // If no matching commits found, decide what to do based on flag
    if commits.is_empty() {
        if allow_no_commits {
            // Fallback behavior: use HEAD with warning
            eprintln!(
                "{} No commits found with pattern 'chant({})'. Attempting to use HEAD as fallback.",
                "⚠".yellow(),
                spec_id
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
            let error_msg =
                chant::merge_errors::no_commits_found(spec_id, &format!("chant/{}", spec_id));
            eprintln!("{} {}", "✗".red(), error_msg);
            return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
        }
    }

    Ok(commits)
}
