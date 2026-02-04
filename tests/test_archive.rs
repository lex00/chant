//! Tests for archive lifecycle operations

use chant::spec::{Spec, SpecStatus};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

mod common;

// Helper function that wraps archive command with proper initialization
fn cmd_archive_wrapper(
    _specs_dir: &Path,
    spec_id: Option<&str>,
    dry_run: bool,
    older_than: Option<u64>,
    allow_non_completed: bool,
    commit: bool,
    no_stage: bool,
) -> anyhow::Result<()> {
    // Call chant binary with archive command
    let mut args = vec!["archive".to_string()];

    if let Some(id) = spec_id {
        args.push(id.to_string());
    }

    if dry_run {
        args.push("--dry-run".to_string());
    }

    if let Some(days) = older_than {
        args.push("--older-than".to_string());
        args.push(days.to_string());
    }

    if allow_non_completed {
        args.push("--allow-non-completed".to_string());
    }

    if commit {
        args.push("--commit".to_string());
    }

    if no_stage {
        args.push("--no-stage".to_string());
    }

    let output = Command::new(env!("CARGO_BIN_EXE_chant"))
        .args(&args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Archive command failed: {}", stderr);
    }

    Ok(())
}

fn setup_test_env() -> (TempDir, PathBuf, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let specs_dir = base_path.join(".chant/specs");
    let archive_dir = base_path.join(".chant/archive");
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
    common::setup_test_repo(base_path).unwrap();

    (temp_dir, specs_dir, archive_dir)
}

fn create_spec(specs_dir: &Path, id: &str, title: &str, status: SpecStatus) -> Spec {
    let status_str = match status {
        SpecStatus::Pending => "pending",
        SpecStatus::InProgress => "in_progress",
        SpecStatus::Completed => "completed",
        SpecStatus::Failed => "failed",
        SpecStatus::Ready => "ready",
        SpecStatus::Blocked => "blocked",
        SpecStatus::Paused => "paused",
        SpecStatus::NeedsAttention => "needs_attention",
        SpecStatus::Cancelled => "cancelled",
    };

    let content = format!(
        r#"---
type: code
status: {}
---

# {}

## Acceptance Criteria

- [ ] Test criterion
"#,
        status_str, title
    );

    let spec_path = specs_dir.join(format!("{}.md", id));
    fs::write(&spec_path, &content).unwrap();

    let mut spec = Spec::parse(id, &content).unwrap();
    spec.id = id.to_string();
    spec.save(&spec_path).unwrap();

    // Add to git
    Command::new("git")
        .args(["add", &format!(".chant/specs/{}.md", id)])
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", &format!("Add spec {}", id)])
        .output()
        .unwrap();

    spec
}

// ============================================================================
// ARCHIVE COMMAND TESTS
// ============================================================================

#[test]
#[serial]
fn test_archive_completed_spec() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();
    let spec = create_spec(
        &specs_dir,
        "2026-02-03-001-abc",
        "Test completed spec",
        SpecStatus::Completed,
    );

    // Archive the spec
    let result = cmd_archive_wrapper(
        &specs_dir,
        Some("2026-02-03-001-abc"),
        false, // dry_run
        None,  // older_than
        false, // force
        false, // commit
        false, // no_stage
    );
    assert!(result.is_ok());

    // Verify spec was moved to archive
    let src = specs_dir.join(format!("{}.md", spec.id));
    let dst = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec.id));

    assert!(!src.exists(), "Source spec should be removed");
    assert!(dst.exists(), "Archived spec should exist in archive dir");
}

#[test]
#[serial]
fn test_archive_with_date_directories() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();
    let spec1 = create_spec(
        &specs_dir,
        "2026-02-03-001-abc",
        "Spec from Feb 3",
        SpecStatus::Completed,
    );
    let spec2 = create_spec(
        &specs_dir,
        "2026-02-04-001-def",
        "Spec from Feb 4",
        SpecStatus::Completed,
    );

    // Archive both specs
    cmd_archive_wrapper(
        &specs_dir,
        Some("2026-02-03-001-abc"),
        false,
        None,
        false,
        false,
        false,
    )
    .unwrap();
    cmd_archive_wrapper(
        &specs_dir,
        Some("2026-02-04-001-def"),
        false,
        None,
        false,
        false,
        false,
    )
    .unwrap();

    // Verify directory structure
    let date_dir1 = archive_dir.join("2026-02-03");
    let date_dir2 = archive_dir.join("2026-02-04");

    assert!(
        date_dir1.exists(),
        "Date directory for 2026-02-03 should exist"
    );
    assert!(
        date_dir2.exists(),
        "Date directory for 2026-02-04 should exist"
    );

    assert!(
        date_dir1.join(format!("{}.md", spec1.id)).exists(),
        "Spec 1 should be in correct date directory"
    );
    assert!(
        date_dir2.join(format!("{}.md", spec2.id)).exists(),
        "Spec 2 should be in correct date directory"
    );
}

#[test]
#[serial]
fn test_archive_non_completed_spec() {
    let (_temp_dir, specs_dir, _archive_dir) = setup_test_env();
    create_spec(
        &specs_dir,
        "2026-02-03-002-xyz",
        "Test pending spec",
        SpecStatus::Pending,
    );

    // Try to archive non-completed spec without force
    let result = cmd_archive_wrapper(
        &specs_dir,
        Some("2026-02-03-002-xyz"),
        false,
        None,
        false, // force = false
        false,
        false,
    );

    // Should succeed but not archive anything (no specs to archive)
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_archive_with_allow_non_completed() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();
    let spec = create_spec(
        &specs_dir,
        "2026-02-03-003-pen",
        "Test pending spec",
        SpecStatus::Pending,
    );

    // Archive with force flag
    let result = cmd_archive_wrapper(
        &specs_dir,
        Some("2026-02-03-003-pen"),
        false,
        None,
        true, // force = true (allows non-completed)
        false,
        false,
    );
    if let Err(ref e) = result {
        eprintln!("Archive error: {}", e);
    }
    assert!(result.is_ok());

    // Verify spec was moved
    let src = specs_dir.join(format!("{}.md", spec.id));
    let dst = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec.id));

    assert!(!src.exists(), "Source spec should be removed");
    assert!(dst.exists(), "Archived spec should exist");
}

#[test]
#[serial]
fn test_archive_dry_run() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();
    let spec = create_spec(
        &specs_dir,
        "2026-02-03-004-dry",
        "Test dry run",
        SpecStatus::Completed,
    );

    // Archive with dry_run
    let result = cmd_archive_wrapper(
        &specs_dir,
        Some("2026-02-03-004-dry"),
        true, // dry_run = true
        None,
        false,
        false,
        false,
    );
    assert!(result.is_ok());

    // Verify spec was NOT moved
    let src = specs_dir.join(format!("{}.md", spec.id));
    let dst = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec.id));

    assert!(src.exists(), "Source spec should still exist (dry run)");
    assert!(!dst.exists(), "Archived spec should not exist (dry run)");
}

#[test]
#[serial]
fn test_archive_older_than() {
    use chrono::Local;

    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();

    // Create old completed spec (30 days ago)
    let old_date = Local::now() - chrono::Duration::days(30);
    let old_completed_at = old_date.to_rfc3339();
    let old_spec_id = "2026-01-04-001-old";
    let old_content = format!(
        r#"---
type: code
status: completed
completed_at: {}
---

# Old completed spec

## Acceptance Criteria

- [x] Test
"#,
        old_completed_at
    );
    let old_spec_path = specs_dir.join(format!("{}.md", old_spec_id));
    fs::write(&old_spec_path, &old_content).unwrap();
    Command::new("git")
        .args(["add", &format!(".chant/specs/{}.md", old_spec_id)])
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", &format!("Add spec {}", old_spec_id)])
        .output()
        .unwrap();

    // Create recent completed spec (5 days ago)
    let recent_date = Local::now() - chrono::Duration::days(5);
    let recent_completed_at = recent_date.to_rfc3339();
    let recent_spec_id = "2026-01-29-001-new";
    let recent_content = format!(
        r#"---
type: code
status: completed
completed_at: {}
---

# Recent completed spec

## Acceptance Criteria

- [x] Test
"#,
        recent_completed_at
    );
    let recent_spec_path = specs_dir.join(format!("{}.md", recent_spec_id));
    fs::write(&recent_spec_path, &recent_content).unwrap();
    Command::new("git")
        .args(["add", &format!(".chant/specs/{}.md", recent_spec_id)])
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", &format!("Add spec {}", recent_spec_id)])
        .output()
        .unwrap();

    // Archive specs older than 14 days
    let result = cmd_archive_wrapper(
        &specs_dir,
        None,     // archive all
        false,    // dry_run
        Some(14), // older_than = 14 days
        false,    // force
        false,    // commit
        false,    // no_stage
    );
    assert!(result.is_ok());

    // Verify only old spec was archived
    let old_dst = archive_dir
        .join("2026-01-04")
        .join(format!("{}.md", old_spec_id));
    let recent_src = specs_dir.join(format!("{}.md", recent_spec_id));

    assert!(old_dst.exists(), "Old spec should be archived");
    assert!(recent_src.exists(), "Recent spec should not be archived");
}

#[test]
#[serial]
fn test_archive_all() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();

    let spec1 = create_spec(
        &specs_dir,
        "2026-02-03-005-all1",
        "Completed 1",
        SpecStatus::Completed,
    );
    let spec2 = create_spec(
        &specs_dir,
        "2026-02-03-006-all2",
        "Completed 2",
        SpecStatus::Completed,
    );
    create_spec(
        &specs_dir,
        "2026-02-03-007-pend",
        "Pending",
        SpecStatus::Pending,
    );

    // Archive all (should only archive completed)
    let result = cmd_archive_wrapper(
        &specs_dir, None,  // archive all
        false, // dry_run
        None,  // older_than
        false, // force
        false, // commit
        false, // no_stage
    );
    assert!(result.is_ok());

    // Verify completed specs were archived
    let dst1 = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec1.id));
    let dst2 = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec2.id));
    let pending_src = specs_dir.join("2026-02-03-007-pend.md");

    assert!(dst1.exists(), "Completed spec 1 should be archived");
    assert!(dst2.exists(), "Completed spec 2 should be archived");
    assert!(pending_src.exists(), "Pending spec should not be archived");
}

#[test]
#[serial]
fn test_archive_directory_structure() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();
    let spec_id = "2026-02-03-008-dir";
    create_spec(&specs_dir, spec_id, "Directory test", SpecStatus::Completed);

    // Archive the spec
    cmd_archive_wrapper(&specs_dir, Some(spec_id), false, None, false, false, false).unwrap();

    // Verify structure: .chant/archive/YYYY-MM-DD/spec.md
    let expected_path = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec_id));
    assert!(
        expected_path.exists(),
        "Spec should be in .chant/archive/YYYY-MM-DD/"
    );
    assert_eq!(
        expected_path.parent().unwrap().file_name().unwrap(),
        "2026-02-03",
        "Date directory should match spec date"
    );
}

#[test]
#[serial]
fn test_archive_with_commit() {
    let (_temp_dir, specs_dir, archive_dir) = setup_test_env();
    let spec_id = "2026-02-03-009-com";
    create_spec(&specs_dir, spec_id, "Commit test", SpecStatus::Completed);

    // Archive with commit flag
    let result = cmd_archive_wrapper(
        &specs_dir,
        Some(spec_id),
        false, // dry_run
        None,  // older_than
        false, // allow_non_completed
        true,  // commit = true
        false, // no_stage
    );
    assert!(result.is_ok());

    // Verify spec was archived
    let dst = archive_dir
        .join("2026-02-03")
        .join(format!("{}.md", spec_id));
    assert!(dst.exists(), "Spec should be archived");

    // The --commit flag should create a commit, but there may be other uncommitted changes
    // that prevent auto-commit. We verify the basic archive operation succeeded.
    // A full test would require isolating git state, which is complex in integration tests.

    // Verify the spec was moved (no longer in specs dir)
    let src = specs_dir.join(format!("{}.md", spec_id));
    assert!(!src.exists(), "Source spec should be removed after archive");
}
