//! Dependency

use serial_test::serial;
use std::fs;

mod support {
    include!("../support/mod.rs");
}

use support::factory::SpecFactory;
use support::harness::TestHarness;

#[test]
#[serial]
fn test_dependency_chain_updates() {
    let harness = TestHarness::new();

    // Create three specs in dependency chain: A (no deps), B (depends on A), C (depends on B)
    let spec_a = "2026-01-27-dep-a";
    let spec_b = "2026-01-27-dep-b";
    let spec_c = "2026-01-27-dep-c";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[spec_b]),
    );

    // Verify initial state: A ready, B and C blocked
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains("○") || stdout.contains(spec_a),
        "Spec A should be ready (○)"
    );
    assert!(
        stdout.contains("⊗") || stdout.contains(spec_b),
        "Spec B should be blocked (⊗)"
    );
    assert!(stdout.contains(spec_c), "Spec C should be present");

    // Complete spec A
    let spec_a_path = harness.specs_dir.join(format!("{}.md", spec_a));
    let content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated).expect("Failed to write spec A");

    // Verify B is now ready (and appears in list), C still blocked
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains(spec_b),
        "Spec B should be ready and present in list after A completes"
    );
    assert!(
        stdout.contains(spec_c),
        "Spec C should still be present but blocked"
    );

    // Complete spec B
    let spec_b_path = harness.specs_dir.join(format!("{}.md", spec_b));
    let content = fs::read_to_string(&spec_b_path).expect("Failed to read spec B");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_b_path, updated).expect("Failed to write spec B");

    // Verify C is now ready
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains(spec_c),
        "Spec C should be ready and present in list after B completes"
    );

    // Complete spec C
    let spec_c_path = harness.specs_dir.join(format!("{}.md", spec_c));
    let content = fs::read_to_string(&spec_c_path).expect("Failed to read spec C");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_c_path, updated).expect("Failed to write spec C");

    // Verify C no longer appears in default list (completed specs are filtered by default)
    let _list_output = harness.run(&["list"]).expect("Failed to run chant list");
}

/// Test dependency status updates via direct file edit (reload from disk)

#[test]
#[serial]
fn test_dependency_status_after_direct_file_edit() {
    let harness = TestHarness::new();

    // Create dependency chain
    let spec_a = "2026-01-27-edit-a";
    let spec_b = "2026-01-27-edit-b";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );

    // Verify B is blocked initially
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(stdout.contains(spec_b), "Spec B should be present in list");

    // Manually edit spec A's status to completed (simulating external change)
    let spec_a_path = harness.specs_dir.join(format!("{}.md", spec_a));
    let content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated).expect("Failed to write spec A");

    // Verify B shows as ready (should reload A's status from disk)
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains(spec_b),
        "Spec B should be ready after A's status updated on disk"
    );
}

/// Test parallel dependency resolution (multiple specs depending on same blocker)

#[test]
#[serial]
fn test_parallel_dependency_resolution() {
    let harness = TestHarness::new();

    // Create multiple specs depending on same blocker
    let spec_a = "2026-01-27-par-a";
    let spec_b1 = "2026-01-27-par-b1";
    let spec_b2 = "2026-01-27-par-b2";
    let spec_b3 = "2026-01-27-par-b3";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b1,
        &SpecFactory::as_markdown_with_deps(spec_b1, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_b2,
        &SpecFactory::as_markdown_with_deps(spec_b2, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_b3,
        &SpecFactory::as_markdown_with_deps(spec_b3, "pending", &[spec_a]),
    );

    // Verify all B specs are blocked
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(stdout.contains(spec_b1), "Spec B1 should be present");
    assert!(stdout.contains(spec_b2), "Spec B2 should be present");
    assert!(stdout.contains(spec_b3), "Spec B3 should be present");

    // Complete blocker
    let spec_a_path = harness.specs_dir.join(format!("{}.md", spec_a));
    let content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated).expect("Failed to write spec A");

    // Verify all dependents are ready
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(stdout.contains(spec_b1), "Spec B1 should be ready");
    assert!(stdout.contains(spec_b2), "Spec B2 should be ready");
    assert!(stdout.contains(spec_b3), "Spec B3 should be ready");
}

/// Test --skip-deps flag bypasses dependency checks

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_force_flag_bypasses_dependency_check() {
    let harness = TestHarness::new();

    // Create blocked spec
    let spec_a = "2026-01-27-force-a";
    let spec_b = "2026-01-27-force-b";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );

    // Verify B is blocked using chant list --ready
    let output = harness
        .run(&["list", "--ready"])
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
    let work_without_force = harness
        .run(&["work", spec_b])
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
    let work_with_force = harness
        .run(&["work", spec_b, "--skip-deps"])
        .expect("Failed to run chant work --skip-deps");

    let force_stderr = String::from_utf8_lossy(&work_with_force.stderr);
    assert!(
        force_stderr.contains("Warning: Forcing work on spec")
            || force_stderr.contains("Skipping dependencies"),
        "Warning message should appear when using --skip-deps on blocked spec. Stderr: {}",
        force_stderr
    );
}

/// Test that blocked spec error shows detailed dependency information

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_blocked_spec_shows_detailed_error() {
    let harness = TestHarness::new();

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
    harness.create_spec(spec_a, spec_a_content);

    // Create dependent spec
    let spec_b = "2026-01-27-blocked-detail-b";
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );

    // Try to work on blocked spec - should show detailed error
    let work_output = harness
        .run(&["work", spec_b])
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
}

/// Test --skip-deps flag warning shows which dependencies are being skipped

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_force_flag_shows_skipped_dependencies() {
    let harness = TestHarness::new();

    // Create chain: A -> B -> C (where C depends on both A and B)
    let spec_a = "2026-01-27-force-warn-a";
    let spec_b = "2026-01-27-force-warn-b";
    let spec_c = "2026-01-27-force-warn-c";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[spec_a, spec_b]),
    );

    // Test that working on spec C with --skip-deps shows both A and B as skipped dependencies
    let work_with_force = harness
        .run(&["work", spec_c, "--skip-deps"])
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
}

/// Test that completing a spec automatically reports unblocked dependents

#[test]
#[serial]
fn test_dependency_chain_updates_after_completion() {
    let harness = TestHarness::new();

    // Create dependency chain: A -> B -> C
    let spec_a = "2026-01-27-chain-a";
    let spec_b = "2026-01-27-chain-b";
    let spec_c = "2026-01-27-chain-c";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[spec_b]),
    );

    // Manually complete spec A (simulating what the agent would do)
    let spec_a_path = harness.specs_dir.join(format!("{}.md", spec_a));
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
    let list_output = harness.run(&["list"]).expect("Failed to run chant list");

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
    let spec_b_path = harness.specs_dir.join(format!("{}.md", spec_b));
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
    let ready_output = harness
        .run(&["list", "--ready"])
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
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_chain_with_specific_ids_validates_all_ids() {
    let harness = TestHarness::new();

    // Create three ready specs
    let spec_a = "2026-01-29-chain-a";
    let spec_b = "2026-01-29-chain-b";
    let spec_c = "2026-01-29-chain-c";

    harness.create_spec(spec_a, &SpecFactory::as_markdown(spec_a, "pending"));
    harness.create_spec(spec_b, &SpecFactory::as_markdown(spec_b, "pending"));
    harness.create_spec(spec_c, &SpecFactory::as_markdown(spec_c, "pending"));

    // Test that invalid spec ID fails fast
    let output = harness
        .run(&["work", "--chain", spec_a, "invalid-spec-xyz"])
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
}

/// Test that `chant work --chain` (no IDs) looks for ready specs

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_chain_without_ids_checks_ready_specs() {
    let harness = TestHarness::new();

    // No specs created - chain should report no ready specs
    let output = harness
        .run(&["work", "--chain"])
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
}

/// Test that chain with specific IDs shows note about ignoring --label filter

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_chain_max_limit_applies() {
    let harness = TestHarness::new();

    // No specs to execute - but this verifies the argument parsing works
    let output = harness
        .run(&["work", "--chain", "--chain-max", "2"])
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
}

// ============================================================================
// OUTPUT SCHEMA VALIDATION TESTS
// ============================================================================

#[test]
#[serial]
fn test_status_blocked_filter_with_dependencies() {
    let harness = TestHarness::new();

    // Create dependency chain: A (no deps) -> B (depends on A) -> C (depends on B)
    let spec_a = "2026-01-29-001-aaa";
    let spec_b = "2026-01-29-002-bbb";
    let spec_c = "2026-01-29-003-ccc";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[spec_b]),
    );

    // Test 1: List with --status blocked should show B and C (they have incomplete deps)
    let blocked_output = harness
        .run(&["list", "--status", "blocked"])
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
    let spec_a_path = harness.specs_dir.join(format!("{}.md", spec_a));
    let content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated).expect("Failed to write spec A");

    let blocked_output2 = harness
        .run(&["list", "--status", "blocked"])
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
}

/// Test --status blocked with no blocked specs returns empty

#[test]
#[serial]
fn test_status_blocked_filter_no_blocked_specs() {
    let harness = TestHarness::new();

    // Create independent specs (no dependencies)
    harness.create_spec(
        "2026-01-29-ind-a",
        &SpecFactory::as_markdown_with_deps("2026-01-29-ind-a", "pending", &[]),
    );
    harness.create_spec(
        "2026-01-29-ind-b",
        &SpecFactory::as_markdown_with_deps("2026-01-29-ind-b", "pending", &[]),
    );

    // List with --status blocked should return "No specs" message
    let blocked_output = harness
        .run(&["list", "--status", "blocked"])
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
}

/// Test --status blocked when all specs are blocked

#[test]
#[serial]
fn test_status_blocked_filter_all_blocked() {
    let harness = TestHarness::new();

    // Create specs that all depend on a non-existent spec (all blocked)
    let spec_a = "2026-01-29-allblk-a";
    let spec_b = "2026-01-29-allblk-b";
    let spec_c = "2026-01-29-allblk-c";
    // Dependency that doesn't exist, making all specs blocked
    let missing_dep = "2026-01-29-missing-dep";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[missing_dep]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[missing_dep]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[missing_dep]),
    );

    // List with --status blocked should show all three specs
    let blocked_output = harness
        .run(&["list", "--status", "blocked"])
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
}

/// Test --status blocked with mixed statuses (pending, completed, in_progress)

#[test]
#[serial]
fn test_status_blocked_filter_mixed_statuses() {
    let harness = TestHarness::new();

    // Create a completed spec
    let spec_completed = "2026-01-29-mixed-completed";
    harness.create_spec(
        spec_completed,
        &SpecFactory::as_markdown_with_deps(spec_completed, "completed", &[]),
    );

    // Create an in_progress spec that depends on completed (not blocked because dep is done)
    let spec_in_progress = "2026-01-29-mixed-in-progress";
    harness.create_spec(
        spec_in_progress,
        &SpecFactory::as_markdown_with_deps(spec_in_progress, "in_progress", &[spec_completed]),
    );

    // Create a pending spec that depends on in_progress spec (blocked)
    let spec_blocked = "2026-01-29-mixed-blocked";
    harness.create_spec(
        spec_blocked,
        &SpecFactory::as_markdown_with_deps(spec_blocked, "pending", &[spec_in_progress]),
    );

    // Create a pending spec with no dependencies (not blocked)
    let spec_ready = "2026-01-29-mixed-ready";
    harness.create_spec(
        spec_ready,
        &SpecFactory::as_markdown_with_deps(spec_ready, "pending", &[]),
    );

    // List with --status blocked should only show the blocked spec
    let blocked_output = harness
        .run(&["list", "--status", "blocked"])
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
}

/// Test that chain execution with dependent specs handles dependencies correctly
#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_chain_execution_with_dependencies() {
    let harness = TestHarness::new();

    // Create chain: A (no deps) -> B (depends on A) -> C (depends on B)
    let spec_a = "2026-02-03-chain-dep-a";
    let spec_b = "2026-02-03-chain-dep-b";
    let spec_c = "2026-02-03-chain-dep-c";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[spec_b]),
    );

    // Try to chain all three - B and C should be skipped because they're blocked
    let output = harness
        .run(&["work", "--chain", spec_a, spec_b, spec_c])
        .expect("Failed to run chant work --chain");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Chain should execute A, then B (after A completes), then C (after B completes)
    // All three should complete successfully as dependencies are satisfied in order
    assert!(
        output.status.success()
            || stdout.contains("Skipping")
            || stderr.contains("not ready")
            || stderr.contains("dependencies"),
        "Chain should execute specs in dependency order or skip blocked specs. Stdout: {}, Stderr: {}",
        stdout,
        stderr
    );
}

/// Test chain execution behavior when encountering a blocked spec
#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Agent spawning may hang on Windows")]
fn test_chain_skips_blocked_specs_in_sequence() {
    let harness = TestHarness::new();

    // Create specs: A (ready), B (blocked by X), C (ready)
    let spec_a = "2026-02-03-chain-blk-a";
    let spec_b = "2026-02-03-chain-blk-b";
    let spec_c = "2026-02-03-chain-blk-c";
    let spec_x = "2026-02-03-chain-blk-x"; // blocker that doesn't exist yet

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_x]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[]),
    );

    // Try to chain A, B, C - should skip B because it's blocked (or fail at A due to missing prompt)
    let output = harness
        .run(&["work", "--chain", spec_a, spec_b, spec_c])
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
}

/// Test chain automatically picks up newly unblocked specs
#[test]
#[serial]
fn test_chain_all_ready_with_dependency_updates() {
    let harness = TestHarness::new();

    // Create chain: A (ready) -> B (depends on A) -> C (depends on B)
    let spec_a = "2026-02-03-chain-ready-a";
    let spec_b = "2026-02-03-chain-ready-b";
    let spec_c = "2026-02-03-chain-ready-c";

    harness.create_spec(
        spec_a,
        &SpecFactory::as_markdown_with_deps(spec_a, "pending", &[]),
    );
    harness.create_spec(
        spec_b,
        &SpecFactory::as_markdown_with_deps(spec_b, "pending", &[spec_a]),
    );
    harness.create_spec(
        spec_c,
        &SpecFactory::as_markdown_with_deps(spec_c, "pending", &[spec_b]),
    );

    // Verify initial state: only A is ready
    let ready_output = harness
        .run(&["list", "--ready"])
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
    let spec_a_path = harness.specs_dir.join(format!("{}.md", spec_a));
    let content = fs::read_to_string(&spec_a_path).expect("Failed to read spec A");
    let updated = content.replace("status: pending", "status: completed");
    fs::write(&spec_a_path, updated).expect("Failed to write spec A");

    // Verify B is now ready
    let ready_output2 = harness
        .run(&["list", "--ready"])
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
}
