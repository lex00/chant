//! Tests for command handlers: approve, cleanup, silent, pause

use chant::spec::SpecStatus;
use std::fs;

mod support;
use serial_test::serial;
use support::{factory::SpecFactory, harness::TestHarness};

// ============================================================================
// APPROVE COMMAND TESTS
// ============================================================================

#[test]
fn test_approve_already_approved_spec() {
    let harness = TestHarness::new();

    let spec_id = "2026-01-01-002-def";
    let content = SpecFactory::as_markdown_with_approval(spec_id, "pending", "approved");
    harness.create_spec(spec_id, &content);

    let spec = chant::spec::Spec::parse(spec_id, &content).unwrap();
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
// SILENT COMMAND TESTS
// ============================================================================

#[test]
#[serial]
fn test_silent_config_check() {
    let harness = TestHarness::new();
    std::env::set_current_dir(harness.path()).unwrap();

    let config = chant::config::Config::load().unwrap();
    assert!(!config.project.silent);
}

#[test]
#[serial]
fn test_silent_config_persists() {
    let harness = TestHarness::new();
    std::env::set_current_dir(harness.path()).unwrap();

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
fn test_pause_status_transition() {
    let harness = TestHarness::new();

    let spec_id = "2026-01-01-004-jkl";
    harness.create_spec(spec_id, &SpecFactory::as_markdown(spec_id, "in_progress"));

    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
    let mut spec = harness.load_spec(spec_id);

    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);

    spec.frontmatter.status = SpecStatus::Paused;
    spec.save(&spec_path).unwrap();

    let content = fs::read_to_string(spec_path).unwrap();
    assert!(content.contains("status: paused") || content.contains("status: Paused"));
}
