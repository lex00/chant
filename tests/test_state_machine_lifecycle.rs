//! Lifecycle integration tests for SpecStateMachine.
//!
//! Tests the state machine through full multi-transition sequences using
//! TransitionBuilder directly (not CLI commands).

use chant::spec::{Spec, SpecStatus, TransitionBuilder, TransitionError};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

fn setup_git_repo(repo_dir: &Path) -> std::io::Result<()> {
    common::setup_test_repo(repo_dir)
}

fn create_test_spec(repo_dir: &Path, spec_id: &str, content: &str) -> PathBuf {
    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, content).expect("Failed to write spec");

    // Commit the spec
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add test spec"])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to commit");

    spec_path
}

fn make_spec_commit(repo_dir: &Path, spec_id: &str) {
    // Create a file representing work done
    let work_file = repo_dir.join(format!("work_{}.txt", spec_id));
    fs::write(&work_file, "Work completed").expect("Failed to write work file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to add");

    Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("chant({}): Complete work", spec_id),
        ])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to commit work");
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_happy_path_pending_to_completed() {
    let repo_dir = PathBuf::from("/tmp/test-state-machine-happy-path");
    let _ = common::cleanup_test_repo(&repo_dir);
    setup_git_repo(&repo_dir).expect("Failed to setup repo");

    let spec_content = r#"---
type: code
status: pending
---
# Test Happy Path

## Acceptance Criteria

- [x] Task 1
- [x] Task 2
"#;

    let spec_id = "test-happy-001";
    let spec_path = create_test_spec(&repo_dir, spec_id, spec_content);

    // Create commits for the spec
    make_spec_commit(&repo_dir, spec_id);

    // Load spec and transition: Pending -> InProgress
    let mut spec = Spec::load(&spec_path).expect("Failed to load spec");
    assert_eq!(spec.frontmatter.status, SpecStatus::Pending);

    let result = TransitionBuilder::new(&mut spec).to(SpecStatus::InProgress);
    assert!(result.is_ok(), "Pending -> InProgress should succeed");
    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);

    // Save the updated spec
    spec.save(&spec_path).expect("Failed to save spec");

    // Transition: InProgress -> Completed (with all preconditions)
    let mut spec = Spec::load(&spec_path).expect("Failed to reload spec");
    let result = TransitionBuilder::new(&mut spec)
        .require_clean_tree()
        .require_all_criteria_checked()
        .require_commits_exist()
        .to(SpecStatus::Completed);

    assert!(
        result.is_ok(),
        "InProgress -> Completed should succeed with preconditions met"
    );
    assert_eq!(spec.frontmatter.status, SpecStatus::Completed);

    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_retry_flow_pending_to_failed_to_completed() {
    let repo_dir = PathBuf::from("/tmp/test-state-machine-retry");
    let _ = common::cleanup_test_repo(&repo_dir);
    setup_git_repo(&repo_dir).expect("Failed to setup repo");

    let spec_content = r#"---
type: code
status: pending
---
# Test Retry Flow

## Acceptance Criteria

- [x] Task 1
"#;

    let spec_id = "test-retry-001";
    let spec_path = create_test_spec(&repo_dir, spec_id, spec_content);
    make_spec_commit(&repo_dir, spec_id);

    // Pending -> InProgress
    let mut spec = Spec::load(&spec_path).expect("Failed to load spec");
    TransitionBuilder::new(&mut spec)
        .to(SpecStatus::InProgress)
        .expect("Pending -> InProgress failed");
    spec.save(&spec_path).expect("Failed to save spec");

    // InProgress -> Failed
    let mut spec = Spec::load(&spec_path).expect("Failed to reload spec");
    TransitionBuilder::new(&mut spec)
        .to(SpecStatus::Failed)
        .expect("InProgress -> Failed failed");
    assert_eq!(spec.frontmatter.status, SpecStatus::Failed);
    spec.save(&spec_path).expect("Failed to save spec");

    // Failed -> Pending (retry)
    let mut spec = Spec::load(&spec_path).expect("Failed to reload spec");
    TransitionBuilder::new(&mut spec)
        .to(SpecStatus::Pending)
        .expect("Failed -> Pending failed");
    assert_eq!(spec.frontmatter.status, SpecStatus::Pending);
    spec.save(&spec_path).expect("Failed to save spec");

    // Pending -> InProgress (retry)
    let mut spec = Spec::load(&spec_path).expect("Failed to reload spec");
    TransitionBuilder::new(&mut spec)
        .to(SpecStatus::InProgress)
        .expect("Pending -> InProgress (retry) failed");
    spec.save(&spec_path).expect("Failed to save spec");

    // InProgress -> Completed
    let mut spec = Spec::load(&spec_path).expect("Failed to reload spec");
    TransitionBuilder::new(&mut spec)
        .require_clean_tree()
        .require_all_criteria_checked()
        .require_commits_exist()
        .to(SpecStatus::Completed)
        .expect("InProgress -> Completed failed");
    assert_eq!(spec.frontmatter.status, SpecStatus::Completed);

    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_blocked_flow_with_dependencies() {
    let repo_dir = PathBuf::from("/tmp/test-state-machine-blocked");
    let _ = common::cleanup_test_repo(&repo_dir);
    setup_git_repo(&repo_dir).expect("Failed to setup repo");

    // Create dependency spec
    let dep_content = r#"---
type: code
status: pending
---
# Dependency Spec
"#;
    let dep_id = "test-dep-001";
    let dep_path = create_test_spec(&repo_dir, dep_id, dep_content);

    // Create blocked spec with dependency
    let blocked_content = format!(
        r#"---
type: code
status: pending
depends_on: {}
---
# Blocked Spec
"#,
        dep_id
    );
    let blocked_id = "test-blocked-001";
    let blocked_path = create_test_spec(&repo_dir, blocked_id, &blocked_content);

    // Load and check if blocked
    let mut blocked_spec = Spec::load(&blocked_path).expect("Failed to load blocked spec");
    let dep_spec = Spec::load(&dep_path).expect("Failed to load dep spec");
    let all_specs = vec![blocked_spec.clone(), dep_spec.clone()];

    // Verify spec is blocked
    assert!(
        blocked_spec.is_blocked(&all_specs),
        "Spec should be blocked with unmet dependency"
    );

    // Attempting transition to InProgress with dependency check should fail
    let result = TransitionBuilder::new(&mut blocked_spec)
        .require_dependencies_met()
        .to(SpecStatus::InProgress);

    match result {
        Err(TransitionError::UnmetDependencies(_)) => {
            // Expected error
        }
        _ => panic!("Expected UnmetDependencies error"),
    }

    // Complete the dependency
    let mut dep_spec = Spec::load(&dep_path).expect("Failed to reload dep");
    TransitionBuilder::new(&mut dep_spec)
        .force()
        .to(SpecStatus::Completed)
        .expect("Failed to complete dependency");
    dep_spec.save(&dep_path).expect("Failed to save dep");

    // Now blocked spec should transition successfully
    let all_specs_updated = vec![
        Spec::load(&blocked_path).expect("Failed to reload blocked"),
        Spec::load(&dep_path).expect("Failed to reload dep"),
    ];
    let mut blocked_spec = Spec::load(&blocked_path).expect("Failed to reload blocked spec");
    assert!(
        !blocked_spec.is_blocked(&all_specs_updated),
        "Spec should not be blocked after dependency completion"
    );

    let result = TransitionBuilder::new(&mut blocked_spec)
        .require_dependencies_met()
        .to(SpecStatus::InProgress);
    assert!(result.is_ok(), "Transition should succeed after dep met");
    assert_eq!(blocked_spec.frontmatter.status, SpecStatus::InProgress);

    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_precondition_errors() {
    let repo_dir = PathBuf::from("/tmp/test-state-machine-preconditions");
    let _ = common::cleanup_test_repo(&repo_dir);
    setup_git_repo(&repo_dir).expect("Failed to setup repo");

    // Test DirtyWorktree error
    let spec_content = r#"---
type: code
status: in_progress
---
# Test Dirty Tree

## Acceptance Criteria

- [x] Task 1
"#;
    let spec_id = "test-dirty-001";
    let spec_path = create_test_spec(&repo_dir, spec_id, spec_content);
    make_spec_commit(&repo_dir, spec_id);

    // Make worktree dirty
    fs::write(repo_dir.join("dirty_file.txt"), "uncommitted change")
        .expect("Failed to create dirty file");

    let mut spec = Spec::load(&spec_path).expect("Failed to load spec");
    let result = TransitionBuilder::new(&mut spec)
        .require_clean_tree()
        .to(SpecStatus::Completed);

    match result {
        Err(TransitionError::DirtyWorktree(_)) => {
            // Expected error
        }
        _ => panic!("Expected DirtyWorktree error, got: {:?}", result),
    }

    // Clean up dirty file
    fs::remove_file(repo_dir.join("dirty_file.txt")).ok();
    Command::new("git")
        .args(["checkout", "."])
        .current_dir(&repo_dir)
        .output()
        .ok();

    // Test UncheckedCriteria error
    let spec_content_unchecked = r#"---
type: code
status: in_progress
---
# Test Unchecked Criteria

## Acceptance Criteria

- [ ] Task 1
- [ ] Task 2
"#;
    let spec_id_unchecked = "test-unchecked-001";
    let spec_path_unchecked =
        create_test_spec(&repo_dir, spec_id_unchecked, spec_content_unchecked);
    make_spec_commit(&repo_dir, spec_id_unchecked);

    let mut spec = Spec::load(&spec_path_unchecked).expect("Failed to load spec");
    let result = TransitionBuilder::new(&mut spec)
        .require_all_criteria_checked()
        .to(SpecStatus::Completed);

    match result {
        Err(TransitionError::IncompleteCriteria) => {
            // Expected error
        }
        _ => panic!("Expected IncompleteCriteria error, got: {:?}", result),
    }

    // Test NoCommits error
    let spec_content_no_commits = r#"---
type: code
status: in_progress
---
# Test No Commits

## Acceptance Criteria

- [x] Task 1
"#;
    let spec_id_no_commits = "test-nocommits-001";
    let spec_path_no_commits =
        create_test_spec(&repo_dir, spec_id_no_commits, spec_content_no_commits);
    // Don't make a commit for this spec

    let mut spec = Spec::load(&spec_path_no_commits).expect("Failed to load spec");
    let result = TransitionBuilder::new(&mut spec)
        .require_commits_exist()
        .to(SpecStatus::Completed);

    match result {
        Err(TransitionError::NoCommits) => {
            // Expected error
        }
        _ => panic!("Expected NoCommits error, got: {:?}", result),
    }

    // Test UnmetDependencies error
    let dep_content = r#"---
type: code
status: pending
---
# Unmet Dependency
"#;
    let dep_id = "test-unmet-dep-001";
    create_test_spec(&repo_dir, dep_id, dep_content);

    let blocked_content = format!(
        r#"---
type: code
status: pending
depends_on: {}
---
# Has Unmet Dep

## Acceptance Criteria

- [x] Task 1
"#,
        dep_id
    );
    let blocked_id = "test-unmet-blocked-001";
    let blocked_path = create_test_spec(&repo_dir, blocked_id, &blocked_content);

    let mut spec = Spec::load(&blocked_path).expect("Failed to load spec");
    let result = TransitionBuilder::new(&mut spec)
        .require_dependencies_met()
        .to(SpecStatus::InProgress);

    match result {
        Err(TransitionError::UnmetDependencies(_)) => {
            // Expected error
        }
        _ => panic!("Expected UnmetDependencies error, got: {:?}", result),
    }

    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_force_bypass() {
    let repo_dir = PathBuf::from("/tmp/test-state-machine-force");
    let _ = common::cleanup_test_repo(&repo_dir);
    setup_git_repo(&repo_dir).expect("Failed to setup repo");

    let spec_content = r#"---
type: code
status: pending
---
# Test Force Bypass

## Acceptance Criteria

- [ ] Task 1
- [ ] Task 2
"#;

    let spec_id = "test-force-001";
    let spec_path = create_test_spec(&repo_dir, spec_id, spec_content);
    // Don't make commits - testing force bypass

    // Make worktree dirty to test all precondition bypasses
    fs::write(repo_dir.join("dirty.txt"), "dirty").expect("Failed to create dirty file");

    let mut spec = Spec::load(&spec_path).expect("Failed to load spec");

    // Invalid transition without force should fail
    let result = TransitionBuilder::new(&mut spec).to(SpecStatus::Completed);
    match result {
        Err(TransitionError::InvalidTransition { .. }) => {
            // Expected error
        }
        _ => panic!("Expected InvalidTransition error"),
    }

    // Force should allow invalid transition
    let mut spec = Spec::load(&spec_path).expect("Failed to reload spec");
    let result = TransitionBuilder::new(&mut spec)
        .force()
        .to(SpecStatus::Completed);
    assert!(result.is_ok(), "Force should allow invalid transition");
    assert_eq!(spec.frontmatter.status, SpecStatus::Completed);

    // Force should bypass all preconditions
    let spec_content_in_progress = r#"---
type: code
status: in_progress
---
# Test Force Preconditions

## Acceptance Criteria

- [ ] Unchecked task
"#;
    let spec_id_force = "test-force-002";
    let spec_path_force = create_test_spec(&repo_dir, spec_id_force, spec_content_in_progress);

    let mut spec = Spec::load(&spec_path_force).expect("Failed to load spec");

    // Without force, should fail preconditions
    let result = TransitionBuilder::new(&mut spec)
        .require_clean_tree()
        .require_all_criteria_checked()
        .require_commits_exist()
        .to(SpecStatus::Completed);
    assert!(result.is_err(), "Should fail preconditions without force");

    // With force, should succeed despite failed preconditions
    let mut spec = Spec::load(&spec_path_force).expect("Failed to reload spec");
    let result = TransitionBuilder::new(&mut spec)
        .require_clean_tree()
        .require_all_criteria_checked()
        .require_commits_exist()
        .force()
        .to(SpecStatus::Completed);
    assert!(
        result.is_ok(),
        "Force should bypass all preconditions: {:?}",
        result
    );
    assert_eq!(spec.frontmatter.status, SpecStatus::Completed);

    common::cleanup_test_repo(&repo_dir).ok();
}
