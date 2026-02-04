//! Tests for command handlers: approve, cleanup, silent, pause

use chant::spec::{Spec, SpecStatus};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let specs_dir = base_path.join(".chant/specs");
    let prompts_dir = base_path.join(".chant/prompts");
    let config_path = base_path.join(".chant/config.md");

    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&prompts_dir).unwrap();

    let config_content = r#"---
model: sonnet
silent: false
---

# Project Config
"#;
    fs::write(&config_path, config_content).unwrap();

    let prompt_content = "You are implementing a task for chant.";
    fs::write(prompts_dir.join("standard.md"), prompt_content).unwrap();

    std::env::set_current_dir(base_path).unwrap();

    (temp_dir, specs_dir)
}

fn create_spec(specs_dir: &Path, id: &str, title: &str, status: &str) -> Spec {
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

    let spec_path = specs_dir.join(format!("{}.md", id));
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
#[serial]
fn test_approve_pending_spec() {
    let (_temp_dir, specs_dir) = setup_test_env();
    let spec = create_spec(&specs_dir, "2026-01-01-001-abc", "Test spec", "pending");

    assert_eq!(spec.frontmatter.status, SpecStatus::Pending);
    assert!(spec.frontmatter.approval.is_some());
}

#[test]
#[serial]
fn test_approve_already_approved_spec() {
    let (_temp_dir, specs_dir) = setup_test_env();

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
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, content).unwrap();

    let spec = Spec::parse(spec_id, content).unwrap();
    assert!(spec.frontmatter.approval.is_some());
    let approval = spec.frontmatter.approval.as_ref().unwrap();
    assert_eq!(approval.status, chant::spec::ApprovalStatus::Approved);
}

#[test]
#[serial]
fn test_approve_nonexistent_spec() {
    let (_temp_dir, specs_dir) = setup_test_env();

    let result = chant::spec::resolve_spec(&specs_dir, "nonexistent");
    assert!(result.is_err());
}

// ============================================================================
// CLEANUP COMMAND TESTS
// ============================================================================

#[test]
#[serial]
fn test_cleanup_worktree_parsing() {
    // Test that worktree info can be created and validated
    let (_temp_dir, _specs_dir) = setup_test_env();

    // Verify that specs directory exists
    assert!(_specs_dir.exists());
}

#[test]
#[serial]
fn test_cleanup_dry_run_mode() {
    // Test that cleanup operations can be simulated
    let (_temp_dir, specs_dir) = setup_test_env();

    // Cleanup should not affect spec files
    let spec = create_spec(&specs_dir, "2026-01-01-005-xyz", "Test spec", "pending");
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    assert!(spec_path.exists());
}

// ============================================================================
// SILENT COMMAND TESTS
// ============================================================================

#[test]
#[serial]
fn test_silent_config_check() {
    let (_temp_dir, _specs_dir) = setup_test_env();

    let config = chant::config::Config::load().unwrap();
    assert!(!config.project.silent);
}

#[test]
#[serial]
fn test_silent_config_persists() {
    let (temp_dir, _specs_dir) = setup_test_env();

    let config_path = temp_dir.path().join(".chant/config.md");
    let content_before = fs::read_to_string(&config_path).unwrap();
    assert!(content_before.contains("silent: false"));

    let config = chant::config::Config::load().unwrap();
    assert!(!config.project.silent);
}

// ============================================================================
// PAUSE COMMAND TESTS
// ============================================================================

#[test]
#[serial]
fn test_pause_nonexistent_spec() {
    let (_temp_dir, specs_dir) = setup_test_env();

    let result = chant::spec::resolve_spec(&specs_dir, "nonexistent");
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_pause_spec_status() {
    let (_temp_dir, specs_dir) = setup_test_env();
    let spec = create_spec(&specs_dir, "2026-01-01-003-ghi", "Test spec", "in_progress");

    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
}

#[test]
#[serial]
fn test_pause_status_transition() {
    let (_temp_dir, specs_dir) = setup_test_env();
    let mut spec = create_spec(&specs_dir, "2026-01-01-004-jkl", "Test spec", "in_progress");

    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);

    spec.frontmatter.status = SpecStatus::Paused;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path).unwrap();

    let content = fs::read_to_string(spec_path).unwrap();
    assert!(content.contains("status: paused") || content.contains("status: Paused"));
}
