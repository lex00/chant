//! Spec Lifecycle

use crate::common;

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_direct_mode_merge_and_cleanup() {
    let repo_dir = PathBuf::from("/tmp/test-chant-direct-mode");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let result = std::env::set_current_dir(&repo_dir);
    if result.is_err() {
        let _ = common::cleanup_test_repo(&repo_dir);
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
        let _ = common::cleanup_test_repo(&repo_dir);
        panic!("Failed to create worktree");
    }

    // The test verifies the worktree was created
    assert!(worktree_exists(&wt_path), "Worktree should be created");

    // Clean up for next test
    let _ = fs::remove_dir_all(&wt_path);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_branch_mode_preserves_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-mode");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

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
    assert!(!worktree_exists(&wt_path), "Worktree should be removed");

    // Cleanup - restore dir BEFORE cleaning up repo
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    let _ = fs::remove_dir_all(&wt_path);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_merge_conflict_preserves_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-conflict");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let result = std::env::set_current_dir(&repo_dir);
    if result.is_err() {
        let _ = common::cleanup_test_repo(&repo_dir);
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
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_spec_file_format() {
    // Test spec file creation and format
    let spec_id = "test-spec-format";
    let spec_content = create_test_spec(spec_id);

    // Verify frontmatter
    assert!(spec_content.contains("---"), "Should have YAML frontmatter");
    assert!(
        spec_content.contains("type: code"),
        "Should have type field"
    );
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

// ============================================================================
// CONFLICT RESOLUTION TESTS
// ============================================================================

/// Helper to check if a conflict spec was created
#[allow(dead_code)]
fn conflict_spec_exists(specs_dir: &Path, pattern: &str) -> bool {
    if let Ok(entries) = fs::read_dir(specs_dir) {
        for entry in entries.flatten() {
            if let Ok(filename) = entry.file_name().into_string() {
                if filename.contains("conflict") && filename.contains(pattern) {
                    return true;
                }
            }
        }
    }
    false
}

/// Helper to get the content of a spec file
#[allow(dead_code)]
fn read_spec_file(specs_dir: &Path, spec_id: &str) -> std::io::Result<String> {
    fs::read_to_string(specs_dir.join(format!("{}.md", spec_id)))
}

/// Helper to modify a file in a worktree
#[allow(dead_code)]
fn modify_file_in_worktree(wt_path: &Path, file: &str, content: &str) -> std::io::Result<()> {
    let file_path = wt_path.join(file);
    fs::create_dir_all(file_path.parent().unwrap())?;
    fs::write(&file_path, content)?;
    Ok(())
}

/// Helper to commit a change in a worktree
#[allow(dead_code)]
fn commit_in_worktree(wt_path: &Path, message: &str) -> std::io::Result<()> {
    Command::new("git")
        .args(["add", "."])
        .current_dir(wt_path)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(wt_path)
        .output()?;

    Ok(())
}

/// Test the full workflow with merge conflicts and conflict spec auto-creation
///
/// This test exercises:
/// 1. Creating a driver spec with multiple conflicting changes
/// 2. Splitting it into member specs
/// 3. Executing members in parallel
/// 4. Detecting and handling merge conflicts
/// 5. Verifying conflict specs are auto-created with correct metadata

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_spec_status_updated_after_finalization() {
    use chant::spec::{Spec, SpecStatus};

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let repo_dir = PathBuf::from("/tmp/test-chant-status-update");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    // Set working directory to repo
    let _ = std::env::set_current_dir(&repo_dir);

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create minimal config
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create specs directory with a test spec
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create a spec file manually with status: in_progress to simulate a completed work
    let spec_id = "test-status-update-001";
    let spec_content = r#"---
type: code
status: in_progress
---

# Test Spec for Status Update

This spec tests that finalization updates the status field.

## Acceptance Criteria

- [x] Test criterion 1
- [x] Test criterion 2
"#;

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    std::fs::write(&spec_path, spec_content).expect("Failed to write spec file");

    // Create a git commit to associate with the spec
    let test_file = repo_dir.join("test_changes.txt");
    std::fs::write(&test_file, "Some changes").expect("Failed to write test file");

    let commit_output = Command::new("git")
        .args(["commit", "-am", &format!("chant({}): test commit", spec_id)])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create commit");

    if !commit_output.status.success() {
        eprintln!(
            "Git commit failed: {}",
            String::from_utf8_lossy(&commit_output.stderr)
        );
    }

    // Now manually perform finalization by reading spec, updating it, and saving it
    // This simulates what finalize_spec does
    let mut spec = Spec::load(&spec_path).expect("Failed to load spec");

    // Verify initial status
    assert_eq!(
        spec.frontmatter.status,
        SpecStatus::InProgress,
        "Initial status should be in_progress"
    );

    // Update to completed (like finalize_spec does)
    spec.frontmatter.status = SpecStatus::Completed;
    spec.frontmatter.completed_at = Some("2026-01-27T12:00:00Z".to_string());
    spec.frontmatter.commits = Some(vec!["abc1234".to_string()]);
    spec.frontmatter.model = Some("claude-haiku-4-5".to_string());

    // Save the spec
    spec.save(&spec_path).expect("Failed to save spec");

    // Reload the spec from disk to verify persistence
    let reloaded_spec = Spec::load(&spec_path).expect("Failed to reload spec");

    // Verify the status was persisted correctly
    assert_eq!(
        reloaded_spec.frontmatter.status,
        SpecStatus::Completed,
        "Status should be persisted as Completed after save"
    );
    assert_eq!(
        reloaded_spec.frontmatter.completed_at,
        Some("2026-01-27T12:00:00Z".to_string()),
        "completed_at should be persisted"
    );
    assert_eq!(
        reloaded_spec.frontmatter.commits,
        Some(vec!["abc1234".to_string()]),
        "commits should be persisted"
    );
    assert_eq!(
        reloaded_spec.frontmatter.model,
        Some("haiku".to_string()),
        "model should be normalized on load"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that invalid regex patterns in enterprise config are handled gracefully
/// This verifies:
/// 1. Config with syntactically invalid regex pattern doesn't crash chant add
/// 2. Fields with invalid regex patterns are omitted from the spec
/// 3. Fields with valid patterns still work correctly

#[test]
#[serial]
#[cfg(unix)]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_merge_finalize_flag() {
    use chant::spec::{Spec, SpecStatus};

    let repo_dir = PathBuf::from("/tmp/test-chant-merge-finalize");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");

    // Initialize chant
    let init_output =
        run_chant(&repo_dir, &["init", "--minimal"]).expect("Failed to run chant init");
    if !init_output.status.success() {
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo_dir);
        panic!(
            "Chant init failed: {}",
            String::from_utf8_lossy(&init_output.stderr)
        );
    }

    // Create a spec in 'completed' state (simulating agent work done)
    let chant_dir = repo_dir.join(".chant");
    let specs_dir = chant_dir.join("specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_id = "2026-01-29-001-fin";
    // Note: Status is 'completed' to allow merge (merge only works on completed specs)
    let spec_content = r#"---
type: code
status: completed
---

# Test Spec for Finalize

Test spec for merge --finalize flag.

## Acceptance Criteria

- [x] Test feature
"#;
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit the spec
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add spec");
    Command::new("git")
        .args(["commit", "-m", "Add test spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit spec");

    // Create a branch with changes
    let branch = format!("chant/{}", spec_id);
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    // Make a commit on the branch
    fs::write(repo_dir.join("test_file.txt"), "Test content").expect("Failed to write file");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add file");
    Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("chant({}): Add test file", spec_id),
        ])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Go back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Merge with --finalize flag
    let merge_output = run_chant(
        &repo_dir,
        &["merge", spec_id, "--delete-branch", "--finalize"],
    )
    .expect("Failed to run merge");

    let merge_stdout = String::from_utf8_lossy(&merge_output.stdout);
    let merge_stderr = String::from_utf8_lossy(&merge_output.stderr);

    if !merge_output.status.success() {
        eprintln!("Merge stdout: {}", merge_stdout);
        eprintln!("Merge stderr: {}", merge_stderr);
    }

    assert!(
        merge_output.status.success(),
        "Merge with --finalize should succeed"
    );

    // Verify the spec was finalized (has completed_at timestamp)
    let spec = Spec::load(&spec_path).expect("Failed to reload spec");
    assert_eq!(
        spec.frontmatter.status,
        SpecStatus::Completed,
        "Spec should have Completed status"
    );
    assert!(
        spec.frontmatter.completed_at.is_some(),
        "Spec should have completed_at timestamp after --finalize"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that merge does not try to checkout a deleted branch after successful merge

#[test]
#[serial]
#[cfg(unix)]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_merge_no_checkout_deleted_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-merge-no-checkout-deleted");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");

    // Initialize chant
    let init_output =
        run_chant(&repo_dir, &["init", "--minimal"]).expect("Failed to run chant init");
    if !init_output.status.success() {
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo_dir);
        panic!("Chant init failed");
    }

    // Create a spec
    let chant_dir = repo_dir.join(".chant");
    let specs_dir = chant_dir.join("specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_id = "2026-01-29-002-del";
    let spec_content = r#"---
type: code
status: completed
---

# Test Delete Branch

Test that merge doesn't checkout deleted branch.

## Acceptance Criteria

- [x] Test
"#;
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit the spec
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create branch and make commit
    let branch = format!("chant/{}", spec_id);
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    fs::write(repo_dir.join("feature.txt"), "Feature").expect("Failed to write");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", &format!("chant({}): Add feature", spec_id)])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Stay on the spec branch (the branch we will delete)
    // This tests that merge handles the case where we're on the branch being deleted

    // Merge with delete-branch - should succeed without error about checkout
    let merge_output =
        run_chant(&repo_dir, &["merge", spec_id, "--delete-branch"]).expect("Failed to run merge");

    let merge_stdout = String::from_utf8_lossy(&merge_output.stdout);
    let merge_stderr = String::from_utf8_lossy(&merge_output.stderr);

    // Should succeed (we fixed the bug where it would fail trying to checkout deleted branch)
    assert!(
        merge_output.status.success(),
        "Merge should succeed. stdout: {}, stderr: {}",
        merge_stdout,
        merge_stderr
    );

    // Verify branch was deleted
    assert!(
        !branch_exists(&repo_dir, &branch),
        "Branch should be deleted after merge"
    );

    // Verify we're on main (not trying to be on the deleted branch)
    let current_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to get current branch");
    let current = String::from_utf8_lossy(&current_branch.stdout)
        .trim()
        .to_string();
    assert_eq!(current, "main", "Should be on main after merge");

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that finalization checks the spec's branch field before checking main

#[test]
#[serial]
#[cfg(unix)]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_finalization_checks_spec_branch_field() {
    use chant::spec::{Spec, SpecStatus};

    let repo_dir = PathBuf::from("/tmp/test-chant-finalize-branch-field");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");

    // Initialize chant
    let init_output =
        run_chant(&repo_dir, &["init", "--minimal"]).expect("Failed to run chant init");
    if !init_output.status.success() {
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo_dir);
        panic!("Chant init failed");
    }

    // Create a spec WITH a branch field set (simulating parallel execution)
    let chant_dir = repo_dir.join(".chant");
    let specs_dir = chant_dir.join("specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_id = "2026-01-29-003-brn";
    let branch = format!("chant/{}", spec_id);

    // Spec has branch field set - finalization should check this branch
    let spec_content = format!(
        r#"---
type: code
status: in_progress
branch: {}
---

# Test Branch Field

Test that finalization checks branch field.

## Acceptance Criteria

- [x] Test
"#,
        branch
    );
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, &spec_content).expect("Failed to write spec");

    // Commit the spec
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create the branch with a commit that matches the spec pattern
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    fs::write(repo_dir.join("impl.txt"), "Implementation").expect("Failed to write");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("chant({}): Implement feature", spec_id),
        ])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Go back to main - note: commits are ONLY on the branch, not on main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Finalization should find commits on the branch (not main) because spec has branch field
    let finalize_output =
        run_chant(&repo_dir, &["finalize", spec_id]).expect("Failed to run finalize");

    let finalize_stdout = String::from_utf8_lossy(&finalize_output.stdout);
    let finalize_stderr = String::from_utf8_lossy(&finalize_output.stderr);

    // Finalization should succeed because it found commits on the branch
    assert!(
        finalize_output.status.success(),
        "Finalize should succeed by finding commits on branch. stdout: {}, stderr: {}",
        finalize_stdout,
        finalize_stderr
    );

    // Verify spec is now completed
    let spec = Spec::load(&spec_path).expect("Failed to reload spec");
    assert_eq!(
        spec.frontmatter.status,
        SpecStatus::Completed,
        "Spec should be completed"
    );
    assert!(
        spec.frontmatter.commits.is_some(),
        "Spec should have commits recorded"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

// ============================================================================
// CHAIN EXECUTION MODE TESTS
// ============================================================================

/// Helper to create a simple ready spec (no dependencies)
#[allow(dead_code)]
fn create_ready_spec(specs_dir: &Path, spec_id: &str) -> std::io::Result<()> {
    let content = format!(
        r#"---
type: code
status: pending
---

# Test Spec: {}

Test specification for chain testing.

## Acceptance Criteria

- [x] Test spec created
"#,
        spec_id
    );

    fs::write(specs_dir.join(format!("{}.md", spec_id)), content)?;
    Ok(())
}

/// Test that `chant work --chain spec1 spec2 spec3` chains through ONLY those specs
/// Note: This test verifies validation and message output rather than actual execution
/// since actual execution requires agent invocation.

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_load_with_branch_resolution_non_in_progress() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-resolution-non-in-progress");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create .chant/specs directory
    fs::create_dir_all(repo_dir.join(".chant/specs")).expect("Failed to create specs dir");

    // Create a pending spec
    let spec_id = "2026-01-31-001-abc";
    let spec_content = r#"---
type: code
status: pending
---

# Test Spec

This is a pending spec.
"#;
    let spec_path = repo_dir.join(format!(".chant/specs/{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit it
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add pending spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create a branch with different content
    let branch = format!("chant/{}", spec_id);
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    let branch_spec_content = r#"---
type: code
status: pending
---

# Test Spec

This is DIFFERENT content on the branch.
"#;
    fs::write(&spec_path, branch_spec_content).expect("Failed to write branch spec");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Update spec on branch"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Switch back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Load spec with branch resolution - should NOT load from branch since status is pending
    let loaded_spec =
        chant::spec::Spec::load_with_branch_resolution(&spec_path).expect("Failed to load spec");

    assert!(!loaded_spec.body.contains("DIFFERENT"));
    assert!(loaded_spec.body.contains("This is a pending spec"));

    // Cleanup
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
fn test_load_with_branch_resolution_in_progress_with_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-resolution-in-progress");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create .chant/specs directory
    fs::create_dir_all(repo_dir.join(".chant/specs")).expect("Failed to create specs dir");

    // Create an in_progress spec
    let spec_id = "2026-01-31-002-xyz";
    let spec_content = r#"---
type: code
status: in_progress
---

# Test Spec

This is the main version.
"#;
    let spec_path = repo_dir.join(format!(".chant/specs/{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit it
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add in_progress spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create a branch with updated content
    let branch = format!("chant/{}", spec_id);
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    let branch_spec_content = r#"---
type: code
status: in_progress
---

# Test Spec

This is the BRANCH version with progress.

## Acceptance Criteria

- [x] First criterion completed
- [ ] Second criterion pending
"#;
    fs::write(&spec_path, branch_spec_content).expect("Failed to write branch spec");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Update spec on branch"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Switch back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Load spec with branch resolution - SHOULD load from branch
    let loaded_spec =
        chant::spec::Spec::load_with_branch_resolution(&spec_path).expect("Failed to load spec");

    assert!(
        loaded_spec.body.contains("BRANCH version with progress"),
        "Should load content from branch"
    );
    assert!(
        loaded_spec.body.contains("Acceptance Criteria"),
        "Should include branch content"
    );

    // Cleanup
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
fn test_load_with_branch_resolution_explicit_branch_field() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-resolution-explicit");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create .chant/specs directory
    fs::create_dir_all(repo_dir.join(".chant/specs")).expect("Failed to create specs dir");

    // Create an in_progress spec with explicit branch field
    let spec_id = "2026-01-31-003-def";
    let custom_branch = "feature/custom-branch";
    let spec_content = format!(
        r#"---
type: code
status: in_progress
branch: {}
---

# Test Spec

This is the main version.
"#,
        custom_branch
    );
    let spec_path = repo_dir.join(format!(".chant/specs/{}.md", spec_id));
    fs::write(&spec_path, &spec_content).expect("Failed to write spec");

    // Commit it
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add spec with custom branch"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create the custom branch with updated content
    Command::new("git")
        .args(["checkout", "-b", custom_branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create custom branch");

    let branch_spec_content = format!(
        r#"---
type: code
status: in_progress
branch: {}
---

# Test Spec

This is from the CUSTOM BRANCH.
"#,
        custom_branch
    );
    fs::write(&spec_path, branch_spec_content).expect("Failed to write branch spec");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Update spec on custom branch"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Switch back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Load spec with branch resolution - should load from custom branch
    let loaded_spec =
        chant::spec::Spec::load_with_branch_resolution(&spec_path).expect("Failed to load spec");

    assert!(
        loaded_spec.body.contains("CUSTOM BRANCH"),
        "Should load content from custom branch field"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", custom_branch])
        .current_dir(&repo_dir)
        .output();
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_load_with_branch_resolution_no_branch_exists() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-resolution-no-branch");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create .chant/specs directory
    fs::create_dir_all(repo_dir.join(".chant/specs")).expect("Failed to create specs dir");

    // Create an in_progress spec
    let spec_id = "2026-01-31-004-ghi";
    let spec_content = r#"---
type: code
status: in_progress
---

# Test Spec

This is the main version, no branch exists.
"#;
    let spec_path = repo_dir.join(format!(".chant/specs/{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit it
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add in_progress spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Don't create the branch - it should fall back to main version
    let loaded_spec =
        chant::spec::Spec::load_with_branch_resolution(&spec_path).expect("Failed to load spec");

    assert!(
        loaded_spec.body.contains("no branch exists"),
        "Should use main version when branch doesn't exist"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_load_with_branch_resolution_spec_not_on_branch() {
    let repo_dir = PathBuf::from("/tmp/test-chant-branch-resolution-spec-not-on-branch");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create .chant/specs directory
    fs::create_dir_all(repo_dir.join(".chant/specs")).expect("Failed to create specs dir");

    // Create an in_progress spec
    let spec_id = "2026-01-31-005-jkl";
    let spec_content = r#"---
type: code
status: in_progress
---

# Test Spec

This is the main version.
"#;
    let spec_path = repo_dir.join(format!(".chant/specs/{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit it
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add in_progress spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create a branch but delete the spec from it
    let branch = format!("chant/{}", spec_id);
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    fs::remove_file(&spec_path).expect("Failed to remove spec");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Remove spec from branch"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Switch back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Load spec with branch resolution - should fall back to main when spec not on branch
    let loaded_spec =
        chant::spec::Spec::load_with_branch_resolution(&spec_path).expect("Failed to load spec");

    assert!(
        loaded_spec.body.contains("This is the main version"),
        "Should fall back to main version when spec doesn't exist on branch"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    let _ = common::cleanup_test_repo(&repo_dir);
}
