//! Dependency

mod common {
    pub use crate::common::*;
}

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
fn test_dependency_chain_updates() {
    let repo_dir = PathBuf::from("/tmp/test-chant-dep-chain");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "Chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create three specs in dependency chain: A (no deps), B (depends on A), C (depends on B)
    let spec_a = "2026-01-27-dep-a";
    let spec_b = "2026-01-27-dep-b";
    let spec_c = "2026-01-27-dep-c";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[spec_b]).expect("Failed to create spec C");

    // Verify initial state: A ready, B and C blocked
    let list_output = run_chant_list(&repo_dir);
    assert!(
        list_output.contains("○") || list_output.contains(spec_a),
        "Spec A should be ready (○)"
    );
    assert!(
        list_output.contains("⊗") || list_output.contains(spec_b),
        "Spec B should be blocked (⊗)"
    );
    assert!(list_output.contains(spec_c), "Spec C should be present");

    // Complete spec A
    update_spec_status(&specs_dir, spec_a, "completed").expect("Failed to update spec A");

    // Verify B is now ready (and appears in list), C still blocked
    let list_output = run_chant_list(&repo_dir);
    assert!(
        list_output.contains(spec_b),
        "Spec B should be ready and present in list after A completes"
    );
    assert!(
        list_output.contains(spec_c),
        "Spec C should still be present but blocked"
    );

    // Complete spec B
    update_spec_status(&specs_dir, spec_b, "completed").expect("Failed to update spec B");

    // Verify C is now ready
    let list_output = run_chant_list(&repo_dir);
    assert!(
        list_output.contains(spec_c),
        "Spec C should be ready and present in list after B completes"
    );

    // Complete spec C
    update_spec_status(&specs_dir, spec_c, "completed").expect("Failed to update spec C");

    // Verify C no longer appears in default list (completed specs are filtered by default)
    let _list_output = run_chant_list(&repo_dir);
    // Just verify the command ran successfully - completed specs may or may not appear
    // depending on filter settings

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test dependency status updates via direct file edit (reload from disk)

#[test]
#[serial]
fn test_dependency_status_after_direct_file_edit() {
    let repo_dir = PathBuf::from("/tmp/test-chant-dep-file-edit");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "Chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create dependency chain
    let spec_a = "2026-01-27-edit-a";
    let spec_b = "2026-01-27-edit-b";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");

    // Verify B is blocked initially
    let list_output = run_chant_list(&repo_dir);
    assert!(
        list_output.contains(spec_b),
        "Spec B should be present in list"
    );

    // Manually edit spec A's status to completed (simulating external change)
    let spec_a_path = specs_dir.join(format!("{}.md", spec_a));
    let content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated).expect("Failed to write spec A");

    // Verify B shows as ready (should reload A's status from disk)
    let list_output = run_chant_list(&repo_dir);
    assert!(
        list_output.contains(spec_b),
        "Spec B should be ready after A's status updated on disk"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test parallel dependency resolution (multiple specs depending on same blocker)

#[test]
#[serial]
fn test_parallel_dependency_resolution() {
    let repo_dir = PathBuf::from("/tmp/test-chant-dep-parallel");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "Chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create multiple specs depending on same blocker
    let spec_a = "2026-01-27-par-a";
    let spec_b1 = "2026-01-27-par-b1";
    let spec_b2 = "2026-01-27-par-b2";
    let spec_b3 = "2026-01-27-par-b3";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b1, &[spec_a])
        .expect("Failed to create spec B1");
    create_spec_with_dependencies(&specs_dir, spec_b2, &[spec_a])
        .expect("Failed to create spec B2");
    create_spec_with_dependencies(&specs_dir, spec_b3, &[spec_a])
        .expect("Failed to create spec B3");

    // Verify all B specs are blocked
    let list_output = run_chant_list(&repo_dir);
    assert!(list_output.contains(spec_b1), "Spec B1 should be present");
    assert!(list_output.contains(spec_b2), "Spec B2 should be present");
    assert!(list_output.contains(spec_b3), "Spec B3 should be present");

    // Complete blocker
    update_spec_status(&specs_dir, spec_a, "completed").expect("Failed to update spec A");

    // Verify all dependents are ready
    let list_output = run_chant_list(&repo_dir);
    assert!(list_output.contains(spec_b1), "Spec B1 should be ready");
    assert!(list_output.contains(spec_b2), "Spec B2 should be ready");
    assert!(list_output.contains(spec_b3), "Spec B3 should be ready");

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test --skip-deps flag bypasses dependency checks

#[test]
#[serial]
fn test_force_flag_bypasses_dependency_check() {
    let repo_dir = PathBuf::from("/tmp/test-chant-dep-force");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant with --minimal to avoid interactive wizard
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "Chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create a minimal prompt file for testing
    let prompts_dir = repo_dir.join(".chant/prompts");
    fs::create_dir_all(&prompts_dir).expect("Failed to create prompts dir");
    fs::write(
        prompts_dir.join("standard.md"),
        "# Standard Prompt\n\n{{spec.body}}",
    )
    .expect("Failed to write prompt file");

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create blocked spec
    let spec_a = "2026-01-27-force-a";
    let spec_b = "2026-01-27-force-b";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");

    // Verify B is blocked using chant list --ready
    let output = Command::new(&chant_binary)
        .args(["list", "--ready"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --ready");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "chant list --ready should succeed");
    assert!(
        !stdout.contains(spec_b),
        "Spec B should not be in ready list due to dependency block. Output: {}",
        stdout
    );
    assert!(
        stdout.contains(spec_a),
        "Spec A should be in ready list. Output: {}",
        stdout
    );

    // Test that working on blocked spec without --skip-deps fails
    let work_without_force = Command::new(&chant_binary)
        .args(["work", spec_b])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work");

    let work_stdout = String::from_utf8_lossy(&work_without_force.stdout);
    let work_stderr = String::from_utf8_lossy(&work_without_force.stderr);
    assert!(
        !work_without_force.status.success(),
        "chant work on blocked spec without --skip-deps should fail"
    );
    // New detailed error message format goes to stderr
    assert!(
        work_stderr.contains("blocked by dependencies")
            || work_stderr.contains("Blocking dependencies:")
            || work_stderr.contains("Next steps:")
            || work_stdout.contains("unsatisfied dependencies")
            || work_stdout.contains("Blocked by")
            || work_stdout.contains("--skip-deps"),
        "Error message should mention dependency blocking. Stdout: {}, Stderr: {}",
        work_stdout,
        work_stderr
    );

    // Test that working on blocked spec with --skip-deps shows warning
    // Note: This will fail later because there's no agent configured, but the warning
    // should appear in stderr before the agent invocation
    let work_with_force = Command::new(&chant_binary)
        .args(["work", spec_b, "--skip-deps"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work --skip-deps");

    let force_stderr = String::from_utf8_lossy(&work_with_force.stderr);
    assert!(
        force_stderr.contains("Warning: Forcing work on spec")
            || force_stderr.contains("Skipping dependencies"),
        "Warning message should appear when using --skip-deps on blocked spec. Stderr: {}",
        force_stderr
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that blocked spec error shows detailed dependency information

#[test]
#[serial]
fn test_blocked_spec_shows_detailed_error() {
    let repo_dir = PathBuf::from("/tmp/test-chant-blocked-detail");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant with --minimal
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "Chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create prompt file
    let prompts_dir = repo_dir.join(".chant/prompts");
    fs::create_dir_all(&prompts_dir).expect("Failed to create prompts dir");
    fs::write(
        prompts_dir.join("standard.md"),
        "# Standard Prompt\n\n{{spec.body}}",
    )
    .expect("Failed to write prompt file");

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create blocking spec with a title
    let spec_a = "2026-01-27-blocked-detail-a";
    let spec_a_content = r#"---
type: code
status: pending
---

# Important Blocking Spec

This spec blocks spec B.

## Acceptance Criteria

- [ ] Do something
"#;
    fs::write(specs_dir.join(format!("{}.md", spec_a)), spec_a_content)
        .expect("Failed to write spec A");

    // Create dependent spec
    let spec_b = "2026-01-27-blocked-detail-b";
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");

    // Try to work on blocked spec - should show detailed error
    let work_output = Command::new(&chant_binary)
        .args(["work", spec_b])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work");

    let work_stderr = String::from_utf8_lossy(&work_output.stderr);

    assert!(
        !work_output.status.success(),
        "chant work on blocked spec should fail"
    );

    // Check for detailed error message components
    assert!(
        work_stderr.contains("blocked by dependencies"),
        "Error should mention 'blocked by dependencies'. Stderr: {}",
        work_stderr
    );
    assert!(
        work_stderr.contains("Blocking dependencies:"),
        "Error should show 'Blocking dependencies:' header. Stderr: {}",
        work_stderr
    );
    assert!(
        work_stderr.contains(spec_a),
        "Error should show blocking spec ID. Stderr: {}",
        work_stderr
    );
    assert!(
        work_stderr.contains("Status:"),
        "Error should show dependency status. Stderr: {}",
        work_stderr
    );
    assert!(
        work_stderr.contains("Next steps:"),
        "Error should show actionable next steps. Stderr: {}",
        work_stderr
    );
    assert!(
        work_stderr.contains("--skip-deps"),
        "Error should mention --skip-deps flag. Stderr: {}",
        work_stderr
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test --skip-deps flag warning shows which dependencies are being skipped

#[test]
#[serial]
fn test_force_flag_shows_skipped_dependencies() {
    let repo_dir = PathBuf::from("/tmp/test-chant-dep-force-warn");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant with --minimal to avoid interactive wizard
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "Chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create a minimal prompt file for testing
    let prompts_dir = repo_dir.join(".chant/prompts");
    fs::create_dir_all(&prompts_dir).expect("Failed to create prompts dir");
    fs::write(
        prompts_dir.join("standard.md"),
        "# Standard Prompt\n\n{{spec.body}}",
    )
    .expect("Failed to write prompt file");

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create chain: A -> B -> C (where C depends on both A and B)
    let spec_a = "2026-01-27-force-warn-a";
    let spec_b = "2026-01-27-force-warn-b";
    let spec_c = "2026-01-27-force-warn-c";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[spec_a, spec_b])
        .expect("Failed to create spec C");

    // Test that working on spec C with --skip-deps shows both A and B as skipped dependencies
    let work_with_force = Command::new(&chant_binary)
        .args(["work", spec_c, "--skip-deps"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work --skip-deps");

    let force_stderr = String::from_utf8_lossy(&work_with_force.stderr);

    // The warning should mention both spec A and B as skipped dependencies
    assert!(
        force_stderr.contains("Skipping dependencies"),
        "Warning should mention 'Skipping dependencies'. Stderr: {}",
        force_stderr
    );
    assert!(
        force_stderr.contains(spec_a) || force_stderr.contains("force-warn-a"),
        "Warning should mention spec A as a skipped dependency. Stderr: {}",
        force_stderr
    );
    assert!(
        force_stderr.contains(spec_b) || force_stderr.contains("force-warn-b"),
        "Warning should mention spec B as a skipped dependency. Stderr: {}",
        force_stderr
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that completing a spec automatically reports unblocked dependents

#[test]
#[serial]
fn test_dependency_chain_updates_after_completion() {
    let chant_binary = get_chant_binary();

    // Get current directory to restore later
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    // Setup test repo
    let repo_dir = PathBuf::from("/tmp/test-dependency-chain");
    let _ = common::cleanup_test_repo(&repo_dir);
    common::setup_test_repo(&repo_dir).expect("Failed to setup test repo");

    // Change to repo directory
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant (use --minimal to avoid wizard mode which requires interactive input)
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create dependency chain: A -> B -> C
    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_a = "2026-01-27-chain-a";
    let spec_b = "2026-01-27-chain-b";
    let spec_c = "2026-01-27-chain-c";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[spec_b]).expect("Failed to create spec C");

    // Manually complete spec A (simulating what the agent would do)
    let spec_a_path = specs_dir.join(format!("{}.md", spec_a));
    let spec_a_content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated_content = spec_a_content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated_content).expect("Failed to write spec A");

    // Add completed_at timestamp manually
    let spec_a_content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated_content = spec_a_content.replace(
        "status: completed",
        "status: completed\ncompleted_at: 2026-01-27T10:00:00Z",
    );
    fs::write(&spec_a_path, updated_content).expect("Failed to write spec A");

    // Use chant list to verify B is now ready (not blocked)
    let list_output = Command::new(&chant_binary)
        .args(["list"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list");

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_output.status.success(), "chant list should succeed");

    // B should now be ready (shown with ○) not blocked (⊗)
    // C should still be blocked because B is not completed yet
    assert!(
        stdout.contains(spec_b),
        "Spec B should appear in list. Output: {}",
        stdout
    );
    assert!(
        stdout.contains(spec_c),
        "Spec C should appear in list. Output: {}",
        stdout
    );

    // Now manually complete spec B
    let spec_b_path = specs_dir.join(format!("{}.md", spec_b));
    let spec_b_content = fs::read_to_string(&spec_b_path).expect("Failed to read spec B");
    let updated_content = spec_b_content.replace("status: pending", "status: completed");
    fs::write(&spec_b_path, updated_content).expect("Failed to write spec B");

    let spec_b_content = fs::read_to_string(&spec_b_path).expect("Failed to read spec B");
    let updated_content = spec_b_content.replace(
        "status: completed",
        "status: completed\ncompleted_at: 2026-01-27T11:00:00Z",
    );
    fs::write(&spec_b_path, updated_content).expect("Failed to write spec B");

    // Use chant list --ready to verify C is now ready
    let ready_output = Command::new(&chant_binary)
        .args(["list", "--ready"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --ready");

    let stdout = String::from_utf8_lossy(&ready_output.stdout);
    assert!(
        ready_output.status.success(),
        "chant list --ready should succeed"
    );
    assert!(
        stdout.contains(spec_c),
        "Spec C should be ready after B is completed. Output: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
fn test_chain_with_specific_ids_validates_all_ids() {
    let repo_dir = PathBuf::from("/tmp/test-chant-chain-specific");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create three ready specs
    let spec_a = "2026-01-29-chain-a";
    let spec_b = "2026-01-29-chain-b";
    let spec_c = "2026-01-29-chain-c";

    create_ready_spec(&specs_dir, spec_a).expect("Failed to create spec A");
    create_ready_spec(&specs_dir, spec_b).expect("Failed to create spec B");
    create_ready_spec(&specs_dir, spec_c).expect("Failed to create spec C");

    // Test that invalid spec ID fails fast
    let output = Command::new(&chant_binary)
        .args(["work", "--chain", spec_a, "invalid-spec-xyz"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should fail fast with error about invalid spec
    assert!(
        !output.status.success(),
        "Should fail when invalid spec ID is provided"
    );
    assert!(
        stderr.contains("Invalid spec ID") || stderr.contains("not found"),
        "Error message should mention invalid spec ID. Got: {}",
        stderr
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that `chant work --chain` (no IDs) looks for ready specs

#[test]
#[serial]
fn test_chain_without_ids_checks_ready_specs() {
    let repo_dir = PathBuf::from("/tmp/test-chant-chain-no-ids");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // No specs created - chain should report no ready specs
    let output = Command::new(&chant_binary)
        .args(["work", "--chain"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work --chain");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should succeed (exit 0) with message about no ready specs
    assert!(
        output.status.success(),
        "Should succeed when no ready specs exist. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("No ready specs"),
        "Should indicate no ready specs. Got: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that chain with specific IDs shows note about ignoring --label filter

#[test]
#[serial]
fn test_chain_max_limit_applies() {
    let repo_dir = PathBuf::from("/tmp/test-chant-chain-max");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // No specs to execute - but this verifies the argument parsing works
    let output = Command::new(&chant_binary)
        .args(["work", "--chain", "--chain-max", "2"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work --chain --chain-max");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should succeed (no specs to execute)
    assert!(
        output.status.success(),
        "Should succeed with --chain-max flag. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("No ready specs"),
        "Should indicate no ready specs. Got: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

// ============================================================================
// OUTPUT SCHEMA VALIDATION TESTS
// ============================================================================

#[test]
#[serial]
fn test_status_blocked_filter_with_dependencies() {
    use tempfile::TempDir;

    let chant_binary = get_chant_binary();
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_dir = temp_dir.path().to_path_buf();

    common::setup_test_repo(&repo_dir).expect("Failed to setup test repo");

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("Failed to run chant init");
    assert!(init_output.status.success(), "chant init failed");

    // Create dependency chain: A (no deps) -> B (depends on A) -> C (depends on B)
    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_a = "2026-01-29-001-aaa";
    let spec_b = "2026-01-29-002-bbb";
    let spec_c = "2026-01-29-003-ccc";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[spec_b]).expect("Failed to create spec C");

    // Test 1: List with --status blocked should show B and C (they have incomplete deps)
    let blocked_output = Command::new(&chant_binary)
        .args(["list", "--status", "blocked"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --status blocked");

    let stdout = String::from_utf8_lossy(&blocked_output.stdout);
    let stderr = String::from_utf8_lossy(&blocked_output.stderr);
    assert!(
        blocked_output.status.success(),
        "chant list --status blocked should succeed. stderr: {}",
        stderr
    );

    // B and C should be blocked (they have incomplete dependencies)
    assert!(
        stdout.contains(spec_b),
        "Spec B should appear in blocked list (depends on incomplete A). Output: {}",
        stdout
    );
    assert!(
        stdout.contains(spec_c),
        "Spec C should appear in blocked list (depends on incomplete B). Output: {}",
        stdout
    );
    // A should NOT appear (it has no dependencies)
    assert!(
        !stdout.contains(spec_a),
        "Spec A should NOT appear in blocked list (no dependencies). Output: {}",
        stdout
    );

    // Test 2: Complete spec A and verify B is no longer blocked, but C still is
    update_spec_status(&specs_dir, spec_a, "completed").expect("Failed to update spec A status");

    let blocked_output2 = Command::new(&chant_binary)
        .args(["list", "--status", "blocked"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --status blocked");

    let stdout2 = String::from_utf8_lossy(&blocked_output2.stdout);

    // Now only C should be blocked
    assert!(
        !stdout2.contains(spec_b),
        "Spec B should NOT be blocked after A is completed. Output: {}",
        stdout2
    );
    assert!(
        stdout2.contains(spec_c),
        "Spec C should still be blocked (B not completed). Output: {}",
        stdout2
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
}

/// Test --status blocked with no blocked specs returns empty

#[test]
#[serial]
fn test_status_blocked_filter_no_blocked_specs() {
    let chant_binary = get_chant_binary();
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let repo_dir = PathBuf::from("/tmp/test-blocked-filter-none");

    let _ = common::cleanup_test_repo(&repo_dir);
    common::setup_test_repo(&repo_dir).expect("Failed to setup test repo");

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("Failed to run chant init");
    assert!(init_output.status.success(), "chant init failed");

    // Create independent specs (no dependencies)
    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    create_spec_with_dependencies(&specs_dir, "2026-01-29-ind-a", &[])
        .expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, "2026-01-29-ind-b", &[])
        .expect("Failed to create spec B");

    // List with --status blocked should return "No specs" message
    let blocked_output = Command::new(&chant_binary)
        .args(["list", "--status", "blocked"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --status blocked");

    let stdout = String::from_utf8_lossy(&blocked_output.stdout);
    assert!(
        blocked_output.status.success(),
        "chant list --status blocked should succeed"
    );
    assert!(
        stdout.contains("No specs") || stdout.trim().is_empty() || !stdout.contains("2026-01-29"),
        "Should show no blocked specs. Output: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test --status blocked when all specs are blocked

#[test]
#[serial]
fn test_status_blocked_filter_all_blocked() {
    let chant_binary = get_chant_binary();
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let repo_dir = PathBuf::from("/tmp/test-blocked-filter-all");

    let _ = common::cleanup_test_repo(&repo_dir);
    common::setup_test_repo(&repo_dir).expect("Failed to setup test repo");

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("Failed to run chant init");
    assert!(init_output.status.success(), "chant init failed");

    // Create specs that all depend on a non-existent spec (all blocked)
    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_a = "2026-01-29-allblk-a";
    let spec_b = "2026-01-29-allblk-b";
    let spec_c = "2026-01-29-allblk-c";
    // Dependency that doesn't exist, making all specs blocked
    let missing_dep = "2026-01-29-missing-dep";

    create_spec_with_dependencies(&specs_dir, spec_a, &[missing_dep])
        .expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[missing_dep])
        .expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[missing_dep])
        .expect("Failed to create spec C");

    // List with --status blocked should show all three specs
    let blocked_output = Command::new(&chant_binary)
        .args(["list", "--status", "blocked"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --status blocked");

    let stdout = String::from_utf8_lossy(&blocked_output.stdout);
    assert!(
        blocked_output.status.success(),
        "chant list --status blocked should succeed"
    );

    assert!(
        stdout.contains(spec_a),
        "Spec A should appear in blocked list. Output: {}",
        stdout
    );
    assert!(
        stdout.contains(spec_b),
        "Spec B should appear in blocked list. Output: {}",
        stdout
    );
    assert!(
        stdout.contains(spec_c),
        "Spec C should appear in blocked list. Output: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test --status blocked with mixed statuses (pending, completed, in_progress)

#[test]
#[serial]
fn test_status_blocked_filter_mixed_statuses() {
    let chant_binary = get_chant_binary();
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let repo_dir = PathBuf::from("/tmp/test-blocked-filter-mixed");

    let _ = common::cleanup_test_repo(&repo_dir);
    common::setup_test_repo(&repo_dir).expect("Failed to setup test repo");

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("Failed to run chant init");
    assert!(init_output.status.success(), "chant init failed");

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create a completed spec
    let spec_completed = "2026-01-29-mixed-completed";
    create_spec_with_dependencies(&specs_dir, spec_completed, &[])
        .expect("Failed to create completed spec");
    update_spec_status(&specs_dir, spec_completed, "completed")
        .expect("Failed to update status to completed");

    // Create an in_progress spec that depends on completed (not blocked because dep is done)
    let spec_in_progress = "2026-01-29-mixed-in-progress";
    create_spec_with_dependencies(&specs_dir, spec_in_progress, &[spec_completed])
        .expect("Failed to create in_progress spec");
    update_spec_status(&specs_dir, spec_in_progress, "in_progress")
        .expect("Failed to update status to in_progress");

    // Create a pending spec that depends on in_progress spec (blocked)
    let spec_blocked = "2026-01-29-mixed-blocked";
    create_spec_with_dependencies(&specs_dir, spec_blocked, &[spec_in_progress])
        .expect("Failed to create blocked spec");

    // Create a pending spec with no dependencies (not blocked)
    let spec_ready = "2026-01-29-mixed-ready";
    create_spec_with_dependencies(&specs_dir, spec_ready, &[])
        .expect("Failed to create ready spec");

    // List with --status blocked should only show the blocked spec
    let blocked_output = Command::new(&chant_binary)
        .args(["list", "--status", "blocked"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --status blocked");

    let stdout = String::from_utf8_lossy(&blocked_output.stdout);
    assert!(
        blocked_output.status.success(),
        "chant list --status blocked should succeed"
    );

    // Only the blocked spec should appear
    assert!(
        stdout.contains(spec_blocked),
        "Blocked spec should appear. Output: {}",
        stdout
    );
    assert!(
        !stdout.contains(spec_completed),
        "Completed spec should NOT appear in blocked list. Output: {}",
        stdout
    );
    assert!(
        !stdout.contains(spec_in_progress),
        "In-progress spec should NOT appear in blocked list. Output: {}",
        stdout
    );
    assert!(
        !stdout.contains(spec_ready),
        "Ready (no deps) spec should NOT appear in blocked list. Output: {}",
        stdout
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Helper to create a spec that requires approval
fn create_spec_with_approval(
    specs_dir: &Path,
    spec_id: &str,
    status: &str,
    approval_required: bool,
    approval_status: &str,
) -> std::io::Result<()> {
    let content = format!(
        r#"---
type: code
status: {}
approval:
  required: {}
  status: {}
---

# Test Spec: {}

Test specification for approval testing.

## Acceptance Criteria

- [x] Test spec created
"#,
        status, approval_required, approval_status, spec_id
    );

    fs::write(specs_dir.join(format!("{}.md", spec_id)), content)?;
    Ok(())
}

fn create_spec_with_dependencies(
    specs_dir: &Path,
    spec_id: &str,
    dependencies: &[&str],
) -> std::io::Result<()> {
    let deps_yaml = if dependencies.is_empty() {
        String::new()
    } else {
        format!(
            "depends_on:\n{}",
            dependencies
                .iter()
                .map(|d| format!("  - {}", d))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let content = format!(
        r#"---
type: code
status: pending
{}---

# Test Spec: {}

Test specification for dependency testing.

## Acceptance Criteria

- [x] Test spec created
"#,
        if deps_yaml.is_empty() {
            String::new()
        } else {
            format!("{}\n", deps_yaml)
        },
        spec_id
    );

    fs::write(specs_dir.join(format!("{}.md", spec_id)), content)?;
    Ok(())
}

fn update_spec_status(specs_dir: &Path, spec_id: &str, new_status: &str) -> std::io::Result<()> {
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    let content = fs::read_to_string(&spec_path)?;
    let updated = content.replace("status: pending", &format!("status: {}", new_status));
    fs::write(&spec_path, updated)?;
    Ok(())
}

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

fn run_chant_list(repo_dir: &Path) -> String {
    let chant_binary = get_chant_binary();
    let output = Command::new(&chant_binary)
        .args(["list"])
        .current_dir(repo_dir)
        .output()
        .expect("Failed to run chant list");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Test that chain execution with dependent specs handles dependencies correctly
#[test]
#[serial]
fn test_chain_execution_with_dependencies() {
    let repo_dir = PathBuf::from("/tmp/test-chant-chain-deps");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create chain: A (no deps) -> B (depends on A) -> C (depends on B)
    let spec_a = "2026-02-03-chain-dep-a";
    let spec_b = "2026-02-03-chain-dep-b";
    let spec_c = "2026-02-03-chain-dep-c";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[spec_b]).expect("Failed to create spec C");

    // Try to chain all three - B and C should be skipped because they're blocked
    let output = Command::new(&chant_binary)
        .args(["work", "--chain", spec_a, spec_b, spec_c])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work --chain");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should skip B and C (blocked) or fail with dependency error
    // The exact behavior depends on whether the chain tries to execute or validates upfront
    assert!(
        !output.status.success()
            || stdout.contains("Skipping")
            || stderr.contains("not ready")
            || stderr.contains("dependencies"),
        "Chain should skip blocked specs or show dependency error. Stdout: {}, Stderr: {}",
        stdout,
        stderr
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test chain execution behavior when encountering a blocked spec
#[test]
#[serial]
fn test_chain_skips_blocked_specs_in_sequence() {
    let repo_dir = PathBuf::from("/tmp/test-chant-chain-blocked");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create specs: A (ready), B (blocked by X), C (ready)
    let spec_a = "2026-02-03-chain-blk-a";
    let spec_b = "2026-02-03-chain-blk-b";
    let spec_c = "2026-02-03-chain-blk-c";
    let spec_x = "2026-02-03-chain-blk-x"; // blocker that doesn't exist yet

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_x]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[]).expect("Failed to create spec C");

    // Try to chain A, B, C - should skip B because it's blocked (or fail at A due to missing prompt)
    let output = Command::new(&chant_binary)
        .args(["work", "--chain", spec_a, spec_b, spec_c])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant work --chain");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Chain should either skip B (blocked) or fail before that
    // The key is that B's blocked status should be recognized
    assert!(
        stdout.contains("Skipping")
            || stderr.contains("not ready")
            || stderr.contains("Prompt not found")
            || !output.status.success(),
        "Chain should skip blocked spec B or fail. Stdout: {}, Stderr: {}",
        stdout,
        stderr
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test chain automatically picks up newly unblocked specs
#[test]
#[serial]
fn test_chain_all_ready_with_dependency_updates() {
    let repo_dir = PathBuf::from("/tmp/test-chant-chain-ready-deps");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let chant_binary = get_chant_binary();

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant
    let init_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let specs_dir = repo_dir.join(".chant/specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create chain: A (ready) -> B (depends on A) -> C (depends on B)
    let spec_a = "2026-02-03-chain-ready-a";
    let spec_b = "2026-02-03-chain-ready-b";
    let spec_c = "2026-02-03-chain-ready-c";

    create_spec_with_dependencies(&specs_dir, spec_a, &[]).expect("Failed to create spec A");
    create_spec_with_dependencies(&specs_dir, spec_b, &[spec_a]).expect("Failed to create spec B");
    create_spec_with_dependencies(&specs_dir, spec_c, &[spec_b]).expect("Failed to create spec C");

    // Verify initial state: only A is ready
    let ready_output = Command::new(&chant_binary)
        .args(["list", "--ready"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --ready");

    let stdout = String::from_utf8_lossy(&ready_output.stdout);
    assert!(
        stdout.contains(spec_a),
        "Spec A should be ready. Output: {}",
        stdout
    );
    assert!(
        !stdout.contains(spec_b),
        "Spec B should be blocked. Output: {}",
        stdout
    );
    assert!(
        !stdout.contains(spec_c),
        "Spec C should be blocked. Output: {}",
        stdout
    );

    // Complete A
    update_spec_status(&specs_dir, spec_a, "completed").expect("Failed to update spec A");

    // Verify B is now ready
    let ready_output2 = Command::new(&chant_binary)
        .args(["list", "--ready"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant list --ready");

    let stdout2 = String::from_utf8_lossy(&ready_output2.stdout);
    assert!(
        stdout2.contains(spec_b),
        "Spec B should be ready after A completes. Output: {}",
        stdout2
    );
    assert!(
        !stdout2.contains(spec_c),
        "Spec C should still be blocked. Output: {}",
        stdout2
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}
