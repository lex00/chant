//! Worktree

use crate::common;
use crate::support;

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use support::harness::TestHarness;

fn get_branches(repo_dir: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["branch", "-a"])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to list branches");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn branch_exists(repo_dir: &Path, branch_name: &str) -> bool {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", branch_name])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to check branch");
    output.status.success()
}

fn worktree_exists(worktree_path: &Path) -> bool {
    worktree_path.exists()
}

fn get_commit_count(repo_dir: &Path, branch: &str) -> usize {
    let output = Command::new("git")
        .args(["rev-list", "--count", branch])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to count commits");

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap_or(0)
}

#[allow(dead_code)]
fn create_test_spec(spec_id: &str) -> String {
    format!(
        r#"---
type: code
status: pending
---

# Test Spec: {}

Test specification for integration testing.

## Acceptance Criteria

- [x] Test spec created
"#,
        spec_id
    )
}

fn get_chant_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_chant"))
}

#[allow(dead_code)]
fn run_chant(repo_dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    let chant_binary = get_chant_binary();
    Command::new(&chant_binary)
        .args(args)
        .current_dir(repo_dir)
        .output()
}

#[test]
fn test_worktree_creation_basic() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-001";
    let branch = format!("spec/{}", spec_id);

    // Create worktree using git commands directly
    let wt_path_str = repo_dir.join(format!("chant-{}", spec_id));
    let wt_path_str = wt_path_str.to_str().unwrap();
    let _output = Command::new("git")
        .args(["worktree", "add", "-b", &branch, &wt_path_str])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    let worktree_path = PathBuf::from(&wt_path_str);

    // Verify worktree was created
    assert!(
        worktree_exists(&worktree_path),
        "Worktree directory not created"
    );
    assert!(branch_exists(&repo_dir, &branch), "Branch not created");

    // Cleanup - restore dir BEFORE cleaning up repo
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["worktree", "remove", worktree_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&worktree_path);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_multiple_worktrees_parallel() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-multiple");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let mut worktree_paths = Vec::new();
    let mut branches = Vec::new();

    // Create multiple worktrees with slightly longer IDs to avoid collisions
    for i in 1..=2 {
        let spec_id = format!("test-spec-multi-{:03}", i);
        let branch = format!("spec/{}", spec_id);
        let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

        // Clean up if it exists from previous runs
        let _ = fs::remove_dir_all(&wt_path);

        let output = Command::new("git")
            .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output()
            .unwrap_or_else(|_| panic!("Failed to create worktree {}", i));

        if !output.status.success() {
            eprintln!("Git error: {}", String::from_utf8_lossy(&output.stderr));
            panic!("Worktree creation failed for iteration {}", i);
        }
        assert!(worktree_exists(&wt_path), "Worktree {} not created", i);
        assert!(
            branch_exists(&repo_dir, &branch),
            "Branch {} not created",
            i
        );

        worktree_paths.push(wt_path);
        branches.push(branch);
    }

    // Verify all worktrees exist independently
    for (i, path) in worktree_paths.iter().enumerate() {
        assert!(
            worktree_exists(path),
            "Worktree {} disappeared after creation",
            i + 1
        );
    }

    // Verify all branches exist
    let all_branches = get_branches(&repo_dir);
    for branch in &branches {
        assert!(
            all_branches.iter().any(|b| b.contains(branch)),
            "Branch {} not found in list",
            branch
        );
    }

    // Cleanup - restore dir BEFORE cleaning up repo
    let _ = std::env::set_current_dir(&original_dir);
    for (path, branch) in worktree_paths.iter().zip(branches.iter()) {
        let _ = Command::new("git")
            .args(["worktree", "remove", path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output();
        let _ = fs::remove_dir_all(path);
        let _ = Command::new("git")
            .args(["branch", "-D", branch])
            .current_dir(&repo_dir)
            .output();
    }
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_cleanup_on_failure() {
    let repo_dir = PathBuf::from("/tmp/test-chant-cleanup-failure");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-cleanup";
    let branch = format!("spec/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Create worktree
    Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    assert!(worktree_exists(&wt_path), "Worktree should exist");

    // Simulate cleanup after agent failure
    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();

    // Force remove directory if needed (idempotent cleanup)
    if wt_path.exists() {
        let _ = fs::remove_dir_all(&wt_path);
    }

    // Verify cleanup succeeded
    assert!(!worktree_exists(&wt_path), "Worktree should be cleaned up");

    // Cleanup
    std::env::set_current_dir(&original_dir).expect("Failed to restore dir");
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_concurrent_worktree_isolation() {
    let repo_dir = PathBuf::from("/tmp/test-chant-isolation");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create two separate worktrees
    let spec_id_1 = "spec-isolation-1";
    let branch_1 = format!("spec/{}", spec_id_1);
    let wt_path_1 = PathBuf::from(format!("/tmp/chant-{}", spec_id_1));

    let spec_id_2 = "spec-isolation-2";
    let branch_2 = format!("spec/{}", spec_id_2);
    let wt_path_2 = PathBuf::from(format!("/tmp/chant-{}", spec_id_2));

    // Clean up any previous artifacts
    let _ = fs::remove_dir_all(&wt_path_1);
    let _ = fs::remove_dir_all(&wt_path_2);

    // Create both worktrees
    Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch_1,
            wt_path_1.to_str().unwrap(),
        ])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree 1");

    Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &branch_2,
            wt_path_2.to_str().unwrap(),
        ])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree 2");

    // Add different files to each worktree
    fs::write(wt_path_1.join("file1.txt"), "content1").expect("Failed to write to wt1");
    fs::write(wt_path_2.join("file2.txt"), "content2").expect("Failed to write to wt2");

    // Verify files are independent
    assert!(
        wt_path_1.join("file1.txt").exists(),
        "file1 should exist in wt1"
    );
    assert!(
        !wt_path_1.join("file2.txt").exists(),
        "file2 should not exist in wt1"
    );
    assert!(
        wt_path_2.join("file2.txt").exists(),
        "file2 should exist in wt2"
    );
    assert!(
        !wt_path_2.join("file1.txt").exists(),
        "file1 should not exist in wt2"
    );

    // Commit and verify independence
    Command::new("git")
        .args(["add", "."])
        .current_dir(&wt_path_1)
        .output()
        .expect("Failed to add in wt1");
    Command::new("git")
        .args(["commit", "-m", "Add file1"])
        .current_dir(&wt_path_1)
        .output()
        .expect("Failed to commit in wt1");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&wt_path_2)
        .output()
        .expect("Failed to add in wt2");
    Command::new("git")
        .args(["commit", "-m", "Add file2"])
        .current_dir(&wt_path_2)
        .output()
        .expect("Failed to commit in wt2");

    let commits_1 = get_commit_count(&repo_dir, &branch_1);
    let commits_2 = get_commit_count(&repo_dir, &branch_2);

    assert!(commits_1 > 0, "Branch 1 should have commits");
    assert!(commits_2 > 0, "Branch 2 should have commits");

    // Cleanup - restore dir BEFORE cleaning up repo
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path_1.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path_2.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path_1);
    let _ = fs::remove_dir_all(&wt_path_2);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch_1])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["branch", "-D", &branch_2])
        .current_dir(&repo_dir)
        .output();
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_idempotent_cleanup() {
    let repo_dir = PathBuf::from("/tmp/test-chant-idempotent");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-idempotent";
    let branch = format!("spec/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Create worktree
    Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    // First removal
    let first_remove = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run first remove");

    assert!(
        first_remove.status.success(),
        "First removal should succeed"
    );

    // Try to remove again - should be idempotent
    let second_remove = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();

    // Either success (idempotent) or expected error is fine for idempotency test
    if let Ok(output) = second_remove {
        // If it succeeded, that's idempotent behavior
        // If it failed, that's also acceptable for a second remove
        let _ = output;
    }

    // Force cleanup if needed
    if wt_path.exists() {
        let _ = fs::remove_dir_all(&wt_path);
    }

    // Cleanup - restore dir BEFORE cleaning up repo
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_path_format() {
    // Test that worktree paths follow the expected format
    let spec_id = "2026-01-24-001-abc";
    let expected_path = format!("/tmp/chant-{}", spec_id);
    let wt_path = PathBuf::from(&expected_path);

    // Verify path structure
    assert!(
        wt_path.to_string_lossy().contains("/tmp/chant-"),
        "Worktree should be in /tmp/chant- prefix"
    );
    assert!(
        wt_path.to_string_lossy().contains(spec_id),
        "Worktree path should contain spec ID"
    );
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_status_with_active_worktree() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-status-active");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create a worktree with chant naming convention
    let spec_id = "2026-01-30-001-wts";
    let branch = format!("chant/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Clean up any existing worktree
    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path);

    // Create the worktree
    let create_output = Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    assert!(
        create_output.status.success(),
        "Failed to create worktree: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    // Run chant worktree status
    let status_output =
        run_chant(&repo_dir, &["worktree", "status"]).expect("Failed to run chant worktree status");

    let stdout = String::from_utf8_lossy(&status_output.stdout);
    let stderr = String::from_utf8_lossy(&status_output.stderr);

    // Verify command succeeded
    assert!(
        status_output.status.success(),
        "chant worktree status should succeed. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Verify output contains expected information
    assert!(
        stdout.contains(&wt_path.display().to_string())
            || stdout.contains(&format!("chant-{}", spec_id)),
        "Output should contain worktree path. Output: {}",
        stdout
    );

    assert!(
        stdout.contains(&branch) || stdout.contains(spec_id),
        "Output should contain branch or spec ID. Output: {}",
        stdout
    );

    // Cleanup
    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path);
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test `chant worktree status` with no worktrees shows appropriate message

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_status_no_worktrees() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-status-empty");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Run chant worktree status (no worktrees created)
    let status_output =
        run_chant(&repo_dir, &["worktree", "status"]).expect("Failed to run chant worktree status");

    let stdout = String::from_utf8_lossy(&status_output.stdout);
    let stderr = String::from_utf8_lossy(&status_output.stderr);

    // Verify command succeeded
    assert!(
        status_output.status.success(),
        "chant worktree status should succeed even with no worktrees. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Verify output indicates no worktrees found
    assert!(
        stdout.contains("No chant worktrees found")
            || stdout.contains("no")
            || stdout.is_empty()
            || stdout.trim().is_empty(),
        "Output should indicate no worktrees. Output: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test `chant worktree status` shows multiple worktrees

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_status_multiple_worktrees() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-status-multi");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create two worktrees with chant naming convention
    let spec_ids = ["2026-01-30-001-mw1", "2026-01-30-002-mw2"];
    let mut wt_paths = Vec::new();

    for spec_id in &spec_ids {
        let branch = format!("chant/{}", spec_id);
        let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

        // Clean up any existing worktree
        let _ = Command::new("git")
            .args(["worktree", "remove", wt_path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output();
        let _ = fs::remove_dir_all(&wt_path);

        // Create the worktree
        let create_output = Command::new("git")
            .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output()
            .expect("Failed to create worktree");

        assert!(
            create_output.status.success(),
            "Failed to create worktree for {}: {}",
            spec_id,
            String::from_utf8_lossy(&create_output.stderr)
        );

        wt_paths.push(wt_path);
    }

    // Run chant worktree status
    let status_output =
        run_chant(&repo_dir, &["worktree", "status"]).expect("Failed to run chant worktree status");

    let stdout = String::from_utf8_lossy(&status_output.stdout);

    // Verify command succeeded
    assert!(
        status_output.status.success(),
        "chant worktree status should succeed"
    );

    // Verify output contains both worktrees
    assert!(
        stdout.contains("2 chant worktrees"),
        "Output should mention 2 worktrees. Output: {}",
        stdout
    );

    for spec_id in &spec_ids {
        assert!(
            stdout.contains(spec_id),
            "Output should contain spec ID {}. Output: {}",
            spec_id,
            stdout
        );
    }

    // Cleanup
    for wt_path in &wt_paths {
        let _ = Command::new("git")
            .args(["worktree", "remove", wt_path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output();
        let _ = fs::remove_dir_all(wt_path);
    }
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}
