//! Common test helpers for integration tests

use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to initialize a temporary git repo for testing.
pub fn setup_test_repo(repo_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(repo_dir)?;

    let output = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(repo_dir)
        .output()?;
    assert!(output.status.success(), "git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_dir)
        .output()?;

    // Create initial commit
    fs::write(repo_dir.join("README.md"), "# Test Repo")?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_dir)
        .output()?;

    Ok(())
}

/// Helper to clean up test repos.
pub fn cleanup_test_repo(repo_dir: &Path) -> std::io::Result<()> {
    if repo_dir.exists() {
        fs::remove_dir_all(repo_dir)?;
    }
    Ok(())
}
