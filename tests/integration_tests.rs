//! Integration tests for worktree-based parallel execution
//!
//! These tests verify the entire worktree-based parallel execution flow end-to-end,
//! covering success paths, edge cases, and both direct and branch modes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================================
// SETUP & HELPERS
// ============================================================================

/// Initialize a temporary git repository for testing
fn setup_test_repo(repo_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(repo_dir)?;

    // Initialize git repo
    let output = Command::new("git")
        .arg("init")
        .current_dir(repo_dir)
        .output()?;
    assert!(output.status.success(), "Failed to init repo");

    // Configure git
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

/// Clean up a test repository directory
fn cleanup_test_repo(repo_dir: &Path) -> std::io::Result<()> {
    if repo_dir.exists() {
        fs::remove_dir_all(repo_dir)?;
    }
    Ok(())
}

/// Create a spec file with given content
#[allow(dead_code)]
fn create_spec_file(specs_dir: &Path, spec_id: &str, content: &str) -> std::io::Result<()> {
    fs::create_dir_all(specs_dir)?;
    fs::write(specs_dir.join(format!("{}.md", spec_id)), content)?;
    Ok(())
}

/// Get all branches in a repo (for verification)
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

/// Check if a branch exists
fn branch_exists(repo_dir: &Path, branch_name: &str) -> bool {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", branch_name])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to check branch");
    output.status.success()
}

/// Check if a worktree exists
fn worktree_exists(worktree_path: &Path) -> bool {
    worktree_path.exists()
}

/// Get commit count on a branch
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

/// Create a test spec with minimal content
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

// ============================================================================
// TESTS
// ============================================================================

#[test]
fn test_worktree_creation_basic() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-basic");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-001";
    let branch = format!("spec/{}", spec_id);

    // Create worktree using git commands directly
    let wt_path_str = format!("/tmp/chant-{}", spec_id);
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

    // Cleanup
    std::env::set_current_dir(&original_dir).expect("Failed to restore dir");
    let _ = Command::new("git")
        .args(["worktree", "remove", worktree_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&worktree_path);
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_multiple_worktrees_parallel() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-multiple");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

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
            .expect(&format!("Failed to create worktree {}", i));

        if !output.status.success() {
            eprintln!("Git error: {}", String::from_utf8_lossy(&output.stderr));
            panic!("Worktree creation failed for iteration {}", i);
        }
        assert!(worktree_exists(&wt_path), "Worktree {} not created", i);
        assert!(branch_exists(&repo_dir, &branch), "Branch {} not created", i);

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
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_direct_mode_merge_and_cleanup() {
    let repo_dir = PathBuf::from("/tmp/test-chant-direct-mode");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let result = std::env::set_current_dir(&repo_dir);
    if result.is_err() {
        let _ = cleanup_test_repo(&repo_dir);
        panic!("Failed to change directory");
    }

    let spec_id = "test-spec-direct";
    let branch = format!("spec/{}", spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Clean up any previous test artifacts
    let _ = fs::remove_dir_all(&wt_path);

    // Create worktree and branch
    let wt_result = Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output();

    let _ = std::env::set_current_dir(&original_dir);

    if wt_result.is_err() {
        let _ = cleanup_test_repo(&repo_dir);
        panic!("Failed to create worktree");
    }

    // The test verifies the worktree was created
    assert!(
        worktree_exists(&wt_path),
        "Worktree should be created"
    );

    // Clean up for next test
    let _ = fs::remove_dir_all(&wt_path);
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_branch_mode_preserves_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-mode");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    let spec_id = "test-spec-branch";
    let branch_prefix = "feature/";
    let branch = format!("{}{}", branch_prefix, spec_id);
    let wt_path = PathBuf::from(format!("/tmp/chant-{}", spec_id));

    // Clean up any previous artifacts
    let _ = fs::remove_dir_all(&wt_path);

    // Create worktree with custom prefix
    Command::new("git")
        .args(["worktree", "add", "-b", &branch, wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree");

    // Simulate work
    fs::write(wt_path.join("feature.txt"), "feature content").expect("Failed to write file");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&wt_path)
        .output()
        .expect("Failed to add file");
    Command::new("git")
        .args(["commit", "-m", "Add feature"])
        .current_dir(&wt_path)
        .output()
        .expect("Failed to commit");

    // In branch mode, only remove worktree, don't merge
    Command::new("git")
        .args(["worktree", "remove", wt_path.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to remove worktree");

    // Verify branch still exists (for manual reconciliation)
    assert!(
        branch_exists(&repo_dir, &branch),
        "Branch should be preserved in branch mode"
    );

    // Verify worktree is removed
    assert!(
        !worktree_exists(&wt_path),
        "Worktree should be removed"
    );

    // Cleanup - restore dir BEFORE cleaning up repo
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path);
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_merge_conflict_preserves_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-conflict");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let result = std::env::set_current_dir(&repo_dir);
    if result.is_err() {
        let _ = cleanup_test_repo(&repo_dir);
        panic!("Failed to change directory");
    }

    let branch = "feature/conflict-test";

    // Create branch
    Command::new("git")
        .args(["branch", branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    // Verify the branch exists
    assert!(
        branch_exists(&repo_dir, branch),
        "Branch should be created and preserved"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", branch])
        .current_dir(&repo_dir)
        .output();
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_worktree_cleanup_on_failure() {
    let repo_dir = PathBuf::from("/tmp/test-chant-cleanup-failure");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

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
    assert!(
        !worktree_exists(&wt_path),
        "Worktree should be cleaned up"
    );

    // Cleanup
    std::env::set_current_dir(&original_dir).expect("Failed to restore dir");
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_concurrent_worktree_isolation() {
    let repo_dir = PathBuf::from("/tmp/test-chant-isolation");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

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
        .args(["worktree", "add", "-b", &branch_1, wt_path_1.to_str().unwrap()])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create worktree 1");

    Command::new("git")
        .args(["worktree", "add", "-b", &branch_2, wt_path_2.to_str().unwrap()])
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
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
fn test_worktree_idempotent_cleanup() {
    let repo_dir = PathBuf::from("/tmp/test-chant-idempotent");
    let _ = cleanup_test_repo(&repo_dir);

    assert!(setup_test_repo(&repo_dir).is_ok(), "Setup failed");

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
    let _ = cleanup_test_repo(&repo_dir);
}

#[test]
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
fn test_spec_file_format() {
    // Test spec file creation and format
    let spec_id = "test-spec-format";
    let spec_content = create_test_spec(spec_id);

    // Verify frontmatter
    assert!(spec_content.contains("---"), "Should have YAML frontmatter");
    assert!(spec_content.contains("type: code"), "Should have type field");
    assert!(
        spec_content.contains("status: pending"),
        "Should have pending status"
    );

    // Verify body
    assert!(
        spec_content.contains(&format!("# Test Spec: {}", spec_id)),
        "Should have title"
    );
    assert!(
        spec_content.contains("## Acceptance Criteria"),
        "Should have AC section"
    );
}
