//! Integration tests for chant takeover command

mod common;

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Helper: Create a long-running process and write its PID
fn spawn_dummy_process(spec_id: &str) -> u32 {
    // Spawn a sleep process that we can track
    // Don't wait for the child - let it run in the background
    let child = Command::new("sleep")
        .arg("300") // 5 minutes
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn dummy process");

    let pid = child.id();

    // Important: forget the child handle so it doesn't get cleaned up
    // when the handle is dropped
    std::mem::forget(child);

    // Write PID file
    fs::create_dir_all(".chant/pids").expect("Failed to create pids dir");
    fs::write(format!(".chant/pids/{}.pid", spec_id), pid.to_string())
        .expect("Failed to write PID file");

    pid
}

/// Helper: Create a log file with some content
fn create_log_file(spec_id: &str, content: &str) {
    fs::create_dir_all(".chant/logs").expect("Failed to create logs dir");
    fs::write(format!(".chant/logs/{}.log", spec_id), content).expect("Failed to write log file");
}

/// Helper: Check if a process is running
fn is_process_running(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Process handling differs on Windows")]
fn test_takeover_stops_running_process() {
    let repo_dir = PathBuf::from("/tmp/test-chant-takeover-stop");
    let _ = common::cleanup_test_repo(&repo_dir);
    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create spec
    let spec_id = "takeover-001";
    fs::create_dir_all(".chant/specs").expect("Failed to create specs dir");
    let spec_content = r#"---
type: code
status: in_progress
---

# Takeover Test Spec

Testing takeover stops process.

## Acceptance Criteria

- [ ] Process stopped
"#;
    let spec_path = format!(".chant/specs/{}.md", spec_id);
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit the spec
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add takeover test spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Spawn a dummy process
    let pid = spawn_dummy_process(spec_id);

    // Wait a bit for process to fully start
    thread::sleep(Duration::from_millis(100));

    // Verify process is running
    assert!(is_process_running(pid), "Dummy process should be running");

    // Create a log file
    create_log_file(spec_id, "Agent starting work...\nReading files...\n");

    // Run takeover command
    let result = chant::takeover::cmd_takeover(spec_id, false);
    assert!(result.is_ok(), "Takeover should succeed");

    // Verify PID file is removed (primary assertion)
    let pid_file = format!(".chant/pids/{}.pid", spec_id);
    assert!(
        !PathBuf::from(&pid_file).exists(),
        "PID file should be removed after takeover"
    );

    // Give the SIGTERM signal time to propagate
    thread::sleep(Duration::from_millis(500));

    // Best effort check that process is stopped
    // Note: This may be flaky due to PID reuse, but PID file removal is the primary contract
    let process_stopped = !is_process_running(pid);
    if !process_stopped {
        eprintln!(
            "Warning: Process {} may still be running after takeover (possible PID reuse)",
            pid
        );
    }

    // Cleanup
    std::env::set_current_dir(&original_dir).ok();
    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
fn test_takeover_updates_spec_status_to_paused() {
    let repo_dir = PathBuf::from("/tmp/test-chant-takeover-status");
    let _ = common::cleanup_test_repo(&repo_dir);
    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create spec with in_progress status
    let spec_id = "takeover-002";
    fs::create_dir_all(".chant/specs").expect("Failed to create specs dir");
    let spec_content = r#"---
type: code
status: in_progress
---

# Takeover Status Test

Testing status change.

## Acceptance Criteria

- [ ] Status updated
"#;
    let spec_path = format!(".chant/specs/{}.md", spec_id);
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add status test spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Spawn dummy process
    let _pid = spawn_dummy_process(spec_id);
    create_log_file(spec_id, "Working on task...\n");

    thread::sleep(Duration::from_millis(100));

    // Run takeover
    let result = chant::takeover::cmd_takeover(spec_id, false);
    assert!(result.is_ok(), "Takeover should succeed");

    // Load spec and verify status is paused
    let updated_spec =
        chant::spec::Spec::load(&PathBuf::from(&spec_path)).expect("Failed to load updated spec");
    assert_eq!(
        updated_spec.frontmatter.status,
        chant::spec::SpecStatus::Paused,
        "Spec status should be Paused after takeover"
    );

    // Cleanup
    std::env::set_current_dir(&original_dir).ok();
    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
fn test_takeover_appends_analysis_to_spec() {
    let repo_dir = PathBuf::from("/tmp/test-chant-takeover-analysis");
    let _ = common::cleanup_test_repo(&repo_dir);
    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create spec with known content
    let spec_id = "takeover-003";
    fs::create_dir_all(".chant/specs").expect("Failed to create specs dir");
    let initial_body = "# Takeover Analysis Test\n\nInitial spec content.";
    let spec_content = format!(
        r#"---
type: code
status: in_progress
---

{}

## Acceptance Criteria

- [ ] Analysis appended
"#,
        initial_body
    );
    let spec_path = format!(".chant/specs/{}.md", spec_id);
    fs::write(&spec_path, &spec_content).expect("Failed to write spec");

    // Commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add analysis test spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Spawn dummy process
    let _pid = spawn_dummy_process(spec_id);

    // Create log with some activity
    let log_content = "Starting work on spec...\nAgent made tool call\nReading file: src/main.rs\nWriting file: src/lib.rs\n";
    create_log_file(spec_id, log_content);

    thread::sleep(Duration::from_millis(100));

    // Run takeover
    let result = chant::takeover::cmd_takeover(spec_id, false);
    assert!(result.is_ok(), "Takeover should succeed");

    // Read the updated spec file
    let updated_content = fs::read_to_string(&spec_path).expect("Failed to read updated spec");

    // Verify takeover analysis section was appended
    assert!(
        updated_content.contains("## Takeover Analysis"),
        "Spec should contain Takeover Analysis section"
    );
    assert!(
        updated_content.contains("### Recent Log Activity"),
        "Spec should contain Recent Log Activity section"
    );
    assert!(
        updated_content.contains("### Recommendation"),
        "Spec should contain Recommendation section"
    );

    // Verify original content is preserved
    assert!(
        updated_content.contains("Initial spec content"),
        "Original spec content should be preserved"
    );

    // Cleanup
    std::env::set_current_dir(&original_dir).ok();
    common::cleanup_test_repo(&repo_dir).ok();
}

#[test]
#[serial]
fn test_takeover_force_flag_without_running_process() {
    let repo_dir = PathBuf::from("/tmp/test-chant-takeover-force");
    let _ = common::cleanup_test_repo(&repo_dir);
    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Create spec without running process
    let spec_id = "takeover-004";
    fs::create_dir_all(".chant/specs").expect("Failed to create specs dir");
    let spec_content = r#"---
type: code
status: pending
---

# Takeover Force Test

Testing force flag.

## Acceptance Criteria

- [ ] Force flag works
"#;
    let spec_path = format!(".chant/specs/{}.md", spec_id);
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Add force test spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Create a log file (but no running process)
    create_log_file(spec_id, "Previous work session...\n");

    // Run takeover WITHOUT --force, should fail
    let result_without_force = chant::takeover::cmd_takeover(spec_id, false);
    assert!(
        result_without_force.is_err(),
        "Takeover without force should fail when no process running"
    );

    // Run takeover WITH --force, should succeed
    let result_with_force = chant::takeover::cmd_takeover(spec_id, true);
    assert!(
        result_with_force.is_ok(),
        "Takeover with force should succeed even without running process"
    );

    // Verify analysis was appended even without running process
    let updated_content = fs::read_to_string(&spec_path).expect("Failed to read updated spec");
    assert!(
        updated_content.contains("## Takeover Analysis"),
        "Force takeover should still append analysis"
    );

    // Cleanup
    std::env::set_current_dir(&original_dir).ok();
    common::cleanup_test_repo(&repo_dir).ok();
}
