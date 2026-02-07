//! Integration test for in_progress status bug
//!
//! Tests that specs being worked are correctly marked as in_progress
//! when running `chant work` in parallel mode.

mod support;
use support::harness::TestHarness;

use std::fs;
use std::process::Command;

#[test]
fn test_spec_marked_in_progress_when_copied_to_worktree() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(repo_dir).expect("Failed to change dir");

    // Create a pending spec
    let spec_id = "test-001";
    let spec_content = r#"---
type: code
status: pending
---

# Test Spec

This spec tests that status is updated to in_progress.

## Acceptance Criteria

- [ ] Status is in_progress when spec is copied to worktree
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

    // Simulate what parallel.rs does:
    // 1. Load spec
    let mut spec = chant::spec::Spec::load(&spec_path).expect("Failed to load spec");
    assert_eq!(
        spec.frontmatter.status,
        chant::spec::SpecStatus::Pending,
        "Initial status should be pending"
    );

    // 2. Update status to in_progress
    spec.frontmatter.status = chant::spec::SpecStatus::InProgress;

    // 3. Save to main working dir
    spec.save(&spec_path).expect("Failed to save spec");

    // Verify the spec was saved with in_progress status
    let saved_spec = chant::spec::Spec::load(&spec_path).expect("Failed to load saved spec");
    assert_eq!(
        saved_spec.frontmatter.status,
        chant::spec::SpecStatus::InProgress,
        "Saved spec should have in_progress status"
    );

    // 4. Create worktree
    let branch_name = format!("chant/{}", spec_id);
    let worktree_path = chant::worktree::create_worktree(spec_id, &branch_name, None)
        .expect("Failed to create worktree");

    // 5. Copy spec to worktree (this is where the bug was)
    chant::worktree::copy_spec_to_worktree(spec_id, &worktree_path)
        .expect("Failed to copy spec to worktree");

    // 6. Verify spec in worktree has in_progress status
    let worktree_spec_path = worktree_path.join(format!(".chant/specs/{}.md", spec_id));
    let worktree_spec =
        chant::spec::Spec::load(&worktree_spec_path).expect("Failed to load worktree spec");
    assert_eq!(
        worktree_spec.frontmatter.status,
        chant::spec::SpecStatus::InProgress,
        "Worktree spec should have in_progress status after copy"
    );

    // Cleanup
    let _ = chant::worktree::remove_worktree(&worktree_path);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch_name])
        .current_dir(&repo_dir)
        .output();
    let _ = std::env::set_current_dir(&original_dir);
    // TempDir auto-cleans
}
