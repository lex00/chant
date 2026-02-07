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

mod support {
    pub use crate::support::*;
}

use support::harness::TestHarness;

/// Helper to create a test worktree with a status file
fn create_test_worktree_with_status(
    harness: &TestHarness,
    spec_id: &str,
    status: AgentStatusState,
    updated_at: &str,
) -> Result<PathBuf> {
    let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    let _ = fs::remove_dir_all(&worktree_path);

    let branch = format!("chant/{}", spec_id);
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch,
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(harness.path())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

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
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(harness.path())?;

    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(harness.path())
        .output()?;

    let spec_id = "2026-02-03-001-abc";
    harness.create_spec(
        spec_id,
        "---\ntype: code\nstatus: in_progress\n---\n# Test\n\n## Acceptance Criteria\n- [x] Done\n",
    );

    let worktree_path = create_test_worktree_with_status(
        &harness,
        spec_id,
        AgentStatusState::Done,
        "2026-02-03T10:00:00Z",
    )?;

    fs::write(worktree_path.join("test.txt"), "test")?;
    worktree::commit_in_worktree(&worktree_path, "Test commit")?;

    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once", "--dry-run"])
        .current_dir(harness.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    std::env::set_current_dir(&original_dir)?;
    let _ = fs::remove_dir_all(&worktree_path);

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
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(harness.path())?;

    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(harness.path())
        .output()?;

    let spec_id = "2026-02-03-002-def";
    harness.create_spec(
        spec_id,
        "---\ntype: code\nstatus: in_progress\n---\n# Test\n\n## Acceptance Criteria\n- [ ] Todo\n",
    );

    let stale_time = chrono::Utc::now() - chrono::Duration::hours(2);
    let worktree_path = create_test_worktree_with_status(
        &harness,
        spec_id,
        AgentStatusState::Working,
        &stale_time.to_rfc3339(),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once", "--dry-run"])
        .current_dir(harness.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    std::env::set_current_dir(&original_dir)?;
    let _ = fs::remove_dir_all(&worktree_path);

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
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(harness.path())?;

    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(harness.path())
        .output()?;

    let spec_id = "2026-02-03-003-ghi";
    harness.create_spec(spec_id, "---\ntype: code\nstatus: pending\n---\n# Test\n");

    let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));
    let _ = fs::remove_dir_all(&worktree_path);

    let branch = format!("chant/{}", spec_id);

    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch,
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(harness.path())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create worktree: {}", stderr);
    }

    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once", "--dry-run"])
        .current_dir(harness.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    std::env::set_current_dir(&original_dir)?;
    let _ = fs::remove_dir_all(&worktree_path);

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
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(harness.path())?;

    Command::new(env!("CARGO_BIN_EXE_chant"))
        .arg("init")
        .current_dir(harness.path())
        .output()?;

    let spec_id = "2026-02-03-004-jkl";
    harness.create_spec(spec_id, "---\ntype: code\nstatus: pending\n---\n# Test\n");

    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(["watch", "--once"])
        .current_dir(harness.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    std::env::set_current_dir(&original_dir)?;

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
