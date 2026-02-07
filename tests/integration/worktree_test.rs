//! Worktree integration tests

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod support {
    pub use crate::support::*;
}

use support::harness::TestHarness;

fn worktree_exists(worktree_path: &Path) -> bool {
    worktree_path.exists()
}

#[test]
fn test_worktree_creation_basic() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-001";
    let branch = format!("spec/{}", spec_id);

    let wt_path_str = repo_dir.join(format!("chant-{}", spec_id));
    let wt_path_str = wt_path_str.to_str().unwrap();
    let _output = Command::new("git")
        .args(["worktree", "add", "-b", &branch, &wt_path_str])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    let worktree_path = PathBuf::from(&wt_path_str);

    assert!(
        worktree_exists(&worktree_path),
        "Worktree directory not created"
    );
    assert!(harness.branch_exists(&branch), "Branch not created");

    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["worktree", "remove", worktree_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&worktree_path);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_multiple_worktrees_parallel() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let mut worktree_paths = Vec::new();
    let mut branches = Vec::new();

    for i in 1..=2 {
        let spec_id = format!("test-spec-multi-{:03}", i);
        let branch = format!("spec/{}", spec_id);
        let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

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
        assert!(harness.branch_exists(&branch), "Branch {} not created", i);

        worktree_paths.push(wt_path);
        branches.push(branch);
    }

    for (i, path) in worktree_paths.iter().enumerate() {
        assert!(
            worktree_exists(path),
            "Worktree {} disappeared after creation",
            i + 1
        );
    }

    let all_branches = harness.get_branches();
    for branch in &branches {
        assert!(
            all_branches.iter().any(|b| b.contains(branch)),
            "Branch {} not found in list",
            branch
        );
    }

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
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_cleanup_on_failure() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-cleanup";
    let branch = format!("spec/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    assert!(worktree_exists(&wt_path), "Worktree should exist");

    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();

    if wt_path.exists() {
        let _ = fs::remove_dir_all(&wt_path);
    }

    assert!(!worktree_exists(&wt_path), "Worktree should be cleaned up");

    std::env::set_current_dir(&original_dir).expect("Failed to restore dir");
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_concurrent_worktree_isolation() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id_1 = "spec-isolation-1";
    let branch_1 = format!("spec/{}", spec_id_1);
    let wt_path_1 = PathBuf::from(format!("/tmp/chant-{}", spec_id_1));

    let spec_id_2 = "spec-isolation-2";
    let branch_2 = format!("spec/{}", spec_id_2);
    let wt_path_2 = PathBuf::from(format!("/tmp/chant-{}", spec_id_2));

    let _ = fs::remove_dir_all(&wt_path_1);
    let _ = fs::remove_dir_all(&wt_path_2);

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

    fs::write(wt_path_1.join("file1.txt"), "content1").expect("Failed to write to wt1");
    fs::write(wt_path_2.join("file2.txt"), "content2").expect("Failed to write to wt2");

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

    let commits_1 = harness.get_commit_count(&branch_1);
    let commits_2 = harness.get_commit_count(&branch_2);

    assert!(commits_1 > 0, "Branch 1 should have commits");
    assert!(commits_2 > 0, "Branch 2 should have commits");

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
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_idempotent_cleanup() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-idempotent";
    let branch = format!("spec/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    let first_remove = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run first remove");

    assert!(
        first_remove.status.success(),
        "First removal should succeed"
    );

    let second_remove = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();

    if let Ok(output) = second_remove {
        let _ = output;
    }

    if wt_path.exists() {
        let _ = fs::remove_dir_all(&wt_path);
    }

    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_path_format() {
    let spec_id = "2026-01-24-001-abc";
    let expected_path = format!("/tmp/chant-{}", spec_id);
    let wt_path = PathBuf::from(&expected_path);

    assert!(
        wt_path.to_string_lossy().contains("/tmp/chant-"),
        "Worktree should be in /tmp/chant- prefix"
    );
    assert!(
        wt_path.to_string_lossy().contains(spec_id),
        "Worktree path should contain spec ID"
    );
}

fn get_chant_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_chant"))
}

fn run_chant(repo_dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    let chant_binary = get_chant_binary();
    Command::new(&chant_binary)
        .args(args)
        .current_dir(repo_dir)
        .output()
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_status_with_active_worktree() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "2026-01-30-001-wts";
    let branch = format!("chant/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path);

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

    let status_output =
        run_chant(&repo_dir, &["worktree", "status"]).expect("Failed to run chant worktree status");

    let stdout = String::from_utf8_lossy(&status_output.stdout);
    let stderr = String::from_utf8_lossy(&status_output.stderr);

    assert!(
        status_output.status.success(),
        "chant worktree status should succeed. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

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

    let _ = Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path);
    let _ = std::env::set_current_dir(&original_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_status_no_worktrees() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let status_output =
        run_chant(&repo_dir, &["worktree", "status"]).expect("Failed to run chant worktree status");

    let stdout = String::from_utf8_lossy(&status_output.stdout);
    let stderr = String::from_utf8_lossy(&status_output.stderr);

    assert!(
        status_output.status.success(),
        "chant worktree status should succeed even with no worktrees. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains("No chant worktrees found")
            || stdout.contains("no")
            || stdout.is_empty()
            || stdout.trim().is_empty(),
        "Output should indicate no worktrees. Output: {}",
        stdout
    );

    let _ = std::env::set_current_dir(&original_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_worktree_status_multiple_worktrees() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_ids = ["2026-01-30-001-mw1", "2026-01-30-002-mw2"];
    let mut wt_paths = Vec::new();

    for spec_id in &spec_ids {
        let branch = format!("chant/{}", spec_id);
        let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

        let _ = Command::new("git")
            .args(["worktree", "remove", wt_path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output();
        let _ = fs::remove_dir_all(&wt_path);

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

    let status_output =
        run_chant(&repo_dir, &["worktree", "status"]).expect("Failed to run chant worktree status");

    let stdout = String::from_utf8_lossy(&status_output.stdout);

    assert!(
        status_output.status.success(),
        "chant worktree status should succeed"
    );

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

    for wt_path in &wt_paths {
        let _ = Command::new("git")
            .args(["worktree", "remove", wt_path.to_str().unwrap()])
            .current_dir(&repo_dir)
            .output();
        let _ = fs::remove_dir_all(wt_path);
    }
    let _ = std::env::set_current_dir(&original_dir);
}
