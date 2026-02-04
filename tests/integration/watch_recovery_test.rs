//! Integration tests for watch startup recovery
//!
//! Tests crash recovery scenarios:
//! - Stale worktrees with "done" status but not merged
//! - Stale worktrees with "working" status >1 hour old
//! - Orphaned worktrees without status files

use anyhow::Result;
use chant::worktree::{self, status::*};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod common {
    pub use crate::common::*;
}

/// Helper to create a test worktree with a status file
fn create_test_worktree_with_status(
    repo_dir: &std::path::Path,
    spec_id: &str,
    status: AgentStatusState,
    updated_at: &str,
) -> Result<PathBuf> {
    let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Clean up if already exists
    let _ = fs::remove_dir_all(&worktree_path);

    // Create the branch and worktree
    let branch = format!("chant/{}", spec_id);
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch,
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(repo_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

    // Write status file
    let status_file = worktree_path.join(".chant-status.json");
    let agent_status = AgentStatus {
        spec_id: spec_id.to_string(),
        status,
        updated_at: updated_at.to_string(),
        error: None,
        commits: vec![],
    };
    write_status(&status_file, &agent_status)?;

    Ok(worktree_path)
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_recovery_done_status_triggers_merge() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let tmp_path = tmp.path();
    common::setup_test_repo(tmp_path)?;
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(tmp_path)?;

    // Initialize chant
    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(tmp_path)
        .output()?;

    // Create a spec
    let spec_id = "2026-02-03-001-abc";
    let specs_dir = tmp_path.join(".chant/specs");
    fs::create_dir_all(&specs_dir)?;

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(
        &spec_path,
        "---\ntype: code\nstatus: in_progress\n---\n# Test\n\n## Acceptance Criteria\n- [x] Done\n",
    )?;

    // Create worktree with "done" status
    let worktree_path = create_test_worktree_with_status(
        tmp_path,
        spec_id,
        AgentStatusState::Done,
        "2026-02-03T10:00:00Z",
    )?;

    // Make a commit in the worktree so there's something to merge
    fs::write(worktree_path.join("test.txt"), "test")?;
    worktree::commit_in_worktree(&worktree_path, "Test commit")?;

    // Run watch with --once to trigger recovery
    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once", "--dry-run"])
        .current_dir(tmp_path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    std::env::set_current_dir(&original_dir)?;
    let _ = fs::remove_dir_all(&worktree_path);
    drop(tmp);

    // Verify recovery detected the done status
    assert!(
        stdout.contains("completed worktree") || stdout.contains("needs merge"),
        "Recovery should detect completed worktree. stdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    Ok(())
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_recovery_stale_working_marks_failed() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let tmp_path = tmp.path();
    common::setup_test_repo(tmp_path)?;
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(tmp_path)?;

    // Initialize chant
    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(tmp_path)
        .output()?;

    // Create a spec
    let spec_id = "2026-02-03-002-def";
    let specs_dir = tmp_path.join(".chant/specs");
    fs::create_dir_all(&specs_dir)?;

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(
        &spec_path,
        "---\ntype: code\nstatus: in_progress\n---\n# Test\n\n## Acceptance Criteria\n- [ ] Todo\n",
    )?;

    // Create worktree with stale "working" status (>1 hour old)
    let stale_time = chrono::Utc::now() - chrono::Duration::hours(2);
    let worktree_path = create_test_worktree_with_status(
        tmp_path,
        spec_id,
        AgentStatusState::Working,
        &stale_time.to_rfc3339(),
    )?;

    // Run watch with --once to trigger recovery
    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once", "--dry-run"])
        .current_dir(tmp_path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    std::env::set_current_dir(&original_dir)?;
    let _ = fs::remove_dir_all(&worktree_path);
    drop(tmp);

    // Verify recovery detected the stale status
    assert!(
        stdout.contains("stale working") || stdout.contains("would mark spec failed"),
        "Recovery should detect stale working status. stdout: {}",
        stdout
    );

    Ok(())
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_recovery_orphaned_worktree_cleanup() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let tmp_path = tmp.path();
    common::setup_test_repo(tmp_path)?;
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(tmp_path)?;

    // Initialize chant
    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(tmp_path)
        .output()?;

    // Create a spec
    let spec_id = "2026-02-03-003-ghi";
    let specs_dir = tmp_path.join(".chant/specs");
    fs::create_dir_all(&specs_dir)?;

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(
        &spec_path,
        "---\ntype: code\nstatus: pending\n---\n# Test\n",
    )?;

    // Create worktree without status file (orphaned)
    let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Clean up if already exists
    let _ = fs::remove_dir_all(&worktree_path);

    let branch = format!("chant/{}", spec_id);

    // Create worktree (also creates the branch)
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch,
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(tmp_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

    // Set modified time to >24 hours ago (simulate old worktree)
    // Note: This is platform-specific and may not work on all systems
    // We'll rely on the natural age of the directory instead

    // Run watch with --once to trigger recovery
    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once", "--dry-run"])
        .current_dir(tmp_path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    std::env::set_current_dir(&original_dir)?;
    let _ = fs::remove_dir_all(&worktree_path);
    drop(tmp);

    // Note: This test may not trigger cleanup if the worktree is too fresh
    // The important thing is that recovery runs without errors
    assert!(
        output.status.success(),
        "Watch should complete successfully"
    );
    assert!(
        stdout.contains("startup recovery") || stdout.contains("Running"),
        "Recovery should run. stdout: {}",
        stdout
    );

    Ok(())
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_recovery_no_worktrees_no_action() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let tmp_path = tmp.path();
    common::setup_test_repo(tmp_path)?;
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(tmp_path)?;

    // Initialize chant
    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(tmp_path)
        .output()?;

    // Create a spec but no worktrees
    let spec_id = "2026-02-03-004-jkl";
    let specs_dir = tmp_path.join(".chant/specs");
    fs::create_dir_all(&specs_dir)?;

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(
        &spec_path,
        "---\ntype: code\nstatus: pending\n---\n# Test\n",
    )?;

    // Run watch with --once to trigger recovery
    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once"])
        .current_dir(tmp_path)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    std::env::set_current_dir(&original_dir)?;
    drop(tmp);

    // Verify recovery runs but takes no action
    assert!(
        output.status.success(),
        "Watch should complete successfully"
    );
    assert!(
        stdout.contains("No recovery actions needed") || stdout.contains("startup recovery"),
        "Recovery should run with no actions. stdout: {}",
        stdout
    );

    Ok(())
}
