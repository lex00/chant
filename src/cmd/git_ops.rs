//! Git operations for spec execution.
//!
//! Contains git-related helper functions used by both single and parallel execution modes.

use anyhow::{Context, Result};
use std::path::Path;

/// Check if a path is gitignored.
fn is_gitignored(path: &Path) -> bool {
    use std::process::Command;

    Command::new("git")
        .args(["check-ignore", "-q", &path.to_string_lossy()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Create a new branch or switch to an existing one.
#[allow(dead_code)]
pub fn create_or_switch_branch(branch_name: &str) -> Result<()> {
    use std::process::Command;

    // Try to create a new branch
    let create_output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .output()
        .context("Failed to run git checkout")?;

    if create_output.status.success() {
        return Ok(());
    }

    // Branch might already exist, try to switch to it
    let switch_output = Command::new("git")
        .args(["checkout", branch_name])
        .output()
        .context("Failed to run git checkout")?;

    if switch_output.status.success() {
        return Ok(());
    }

    // Both failed, return error
    let stderr = String::from_utf8_lossy(&switch_output.stderr);
    anyhow::bail!(
        "Failed to create or switch to branch '{}': {}",
        branch_name,
        stderr
    )
}

/// Commit the spec file as a transcript record.
///
/// Handles the case where there's nothing to commit (returns Ok).
pub fn commit_transcript(spec_id: &str, spec_path: &Path) -> Result<()> {
    use std::process::Command;

    // Skip if the spec file is gitignored
    if is_gitignored(spec_path) {
        return Ok(());
    }

    // Stage the spec file
    let output = Command::new("git")
        .args(["add", &spec_path.to_string_lossy()])
        .output()
        .context("Failed to run git add for transcript commit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to stage spec file for transcript commit: {}",
            stderr
        );
    }

    // Create commit for transcript
    let commit_message = format!("chant: Record agent transcript for {}", spec_id);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to run git commit for transcript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // It's ok if there's nothing to commit (no changes after finalization)
        if stderr.contains("nothing to commit") || stderr.contains("no changes added") {
            return Ok(());
        }
        anyhow::bail!("Failed to commit transcript: {}", stderr);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_gitignored() {
        // Test with a file that should not be ignored
        let not_ignored = PathBuf::from("Cargo.toml");
        assert!(!is_gitignored(&not_ignored));

        // Test with a file that should be ignored (if .chant is in .gitignore)
        let possibly_ignored = PathBuf::from(".chant/specs/test.md");
        // This will be true if .chant is in .gitignore, false otherwise
        // The test passes either way, just verifying the function executes
        let _ = is_gitignored(&possibly_ignored);
    }
}
