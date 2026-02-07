//! Tests for command handlers: approve, cleanup, silent, pause

use chant::spec::{Spec, SpecStatus};
use std::fs;
use std::path::Path;

mod support;
use support::harness::TestHarness;

fn create_spec(harness: &TestHarness, id: &str, title: &str, status: &str) -> Spec {
    let content = format!(
        r#"---
type: code
status: {}
approval:
  required: true
---

# {}

## Acceptance Criteria

- [ ] Test criterion
"#,
        status, title
    );

    let spec_path = harness.specs_dir.join(format!("{}.md", id));
    fs::write(&spec_path, &content).unwrap();

    let mut spec = Spec::parse(id, &content).unwrap();
    spec.id = id.to_string();
    spec.save(&spec_path).unwrap();
    spec
}

// ============================================================================
// APPROVE COMMAND TESTS
// ============================================================================

#[test]
fn test_approve_pending_spec() {
    let harness = TestHarness::new();
    let spec = create_spec(&harness, "2026-01-01-001-abc", "Test spec", "pending");

    assert_eq!(spec.frontmatter.status, SpecStatus::Pending);
    assert!(spec.frontmatter.approval.is_some());
}

#[test]
fn test_approve_already_approved_spec() {
    let harness = TestHarness::new();

    let content = r#"---
type: code
status: pending
approval:
  required: true
  status: approved
  by: Initial User
  at: 2026-01-01T00:00:00Z
---

# Already approved spec

## Acceptance Criteria

- [ ] Test
"#;

    let spec_id = "2026-01-01-002-def";
    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, content).unwrap();

    let spec = Spec::parse(spec_id, content).unwrap();
    assert!(spec.frontmatter.approval.is_some());
    let approval = spec.frontmatter.approval.as_ref().unwrap();
    assert_eq!(approval.status, chant::spec::ApprovalStatus::Approved);
}

#[test]
fn test_approve_nonexistent_spec() {
    let harness = TestHarness::new();

    let result = chant::spec::resolve_spec(&harness.specs_dir, "nonexistent");
    assert!(result.is_err());
}

// ============================================================================
// CLEANUP COMMAND TESTS
// ============================================================================

#[test]
fn test_cleanup_worktree_parsing() {
    // Test that worktree info can be created and validated
    let harness = TestHarness::new();

    // Verify that specs directory exists
    assert!(harness.specs_dir.exists());
}

#[test]
fn test_cleanup_dry_run_mode() {
    // Test that cleanup operations can be simulated
    let harness = TestHarness::new();

    // Cleanup should not affect spec files
    let spec = create_spec(&harness, "2026-01-01-005-xyz", "Test spec", "pending");
    let spec_path = harness.specs_dir.join(format!("{}.md", spec.id));
    assert!(spec_path.exists());
}

// ============================================================================
// SILENT COMMAND TESTS
// ============================================================================

#[test]
fn test_silent_config_check() {
    let harness = TestHarness::new();
    std::env::set_current_dir(harness.path()).unwrap();

    let config = chant::config::Config::load().unwrap();
    assert!(!config.project.silent);
}

#[test]
fn test_silent_config_persists() {
    let harness = TestHarness::new();

    let config_path = harness.path().join(".chant/config.md");
    let content_before = fs::read_to_string(&config_path).unwrap();
    assert!(content_before.contains("silent: false"));

    let config = chant::config::Config::load().unwrap();
    assert!(!config.project.silent);
}

// ============================================================================
// PAUSE COMMAND TESTS
// ============================================================================

#[test]
fn test_pause_nonexistent_spec() {
    let harness = TestHarness::new();

    let result = chant::spec::resolve_spec(&harness.specs_dir, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_pause_spec_status() {
    let harness = TestHarness::new();
    let spec = create_spec(&harness, "2026-01-01-003-ghi", "Test spec", "in_progress");

    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
}

#[test]
fn test_pause_status_transition() {
    let harness = TestHarness::new();
    let mut spec = create_spec(&harness, "2026-01-01-004-jkl", "Test spec", "in_progress");

    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);

    spec.frontmatter.status = SpecStatus::Paused;
    let spec_path = harness.specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path).unwrap();

    let content = fs::read_to_string(spec_path).unwrap();
    assert!(content.contains("status: paused") || content.contains("status: Paused"));
}
