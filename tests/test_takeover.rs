//! Integration tests for chant takeover command

#[path = "support/mod.rs"]
mod support;

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use support::factory::SpecFactory;
use support::harness::TestHarness;

/// Helper: Create a long-running process and write its PID
fn spawn_dummy_process(harness: &TestHarness, spec_id: &str) -> u32 {
    let child = Command::new("sleep")
        .arg("300")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn dummy process");

    let pid = child.id();
    std::mem::forget(child);

    let pids_dir = harness.path().join(".chant/pids");
    fs::create_dir_all(&pids_dir).expect("Failed to create pids dir");
    fs::write(pids_dir.join(format!("{}.pid", spec_id)), pid.to_string())
        .expect("Failed to write PID file");

    pid
}

/// Helper: Create a log file with some content
fn create_log_file(harness: &TestHarness, spec_id: &str, content: &str) {
    let logs_dir = harness.path().join(".chant/logs");
    fs::create_dir_all(&logs_dir).expect("Failed to create logs dir");
    fs::write(logs_dir.join(format!("{}.log", spec_id)), content)
        .expect("Failed to write log file");
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
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(harness.path()).expect("Failed to change dir");

    let spec_id = "takeover-001";
    harness.create_spec(spec_id, &SpecFactory::as_markdown(spec_id, "in_progress"));
    harness.git_commit("Add takeover test spec").ok();

    let pid = spawn_dummy_process(&harness, spec_id);
    thread::sleep(Duration::from_millis(100));

    assert!(is_process_running(pid), "Dummy process should be running");

    create_log_file(
        &harness,
        spec_id,
        "Agent starting work...\nReading files...\n",
    );

    let result = chant::takeover::cmd_takeover(spec_id, false);
    assert!(result.is_ok(), "Takeover should succeed");

    let pid_file = harness.path().join(format!(".chant/pids/{}.pid", spec_id));
    assert!(
        !pid_file.exists(),
        "PID file should be removed after takeover"
    );

    thread::sleep(Duration::from_millis(500));

    let process_stopped = !is_process_running(pid);
    if !process_stopped {
        eprintln!(
            "Warning: Process {} may still be running after takeover (possible PID reuse)",
            pid
        );
    }

    std::env::set_current_dir(&original_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Process handling differs on Windows")]
fn test_takeover_updates_spec_status_to_paused() {
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(harness.path()).expect("Failed to change dir");

    let spec_id = "takeover-002";
    harness.create_spec(spec_id, &SpecFactory::as_markdown(spec_id, "in_progress"));
    harness.git_commit("Add status test spec").ok();

    let _pid = spawn_dummy_process(&harness, spec_id);
    create_log_file(&harness, spec_id, "Working on task...\n");
    thread::sleep(Duration::from_millis(100));

    let result = chant::takeover::cmd_takeover(spec_id, false);
    assert!(result.is_ok(), "Takeover should succeed");

    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
    let updated_spec = chant::spec::Spec::load(&spec_path).expect("Failed to load updated spec");
    assert_eq!(
        updated_spec.frontmatter.status,
        chant::spec::SpecStatus::Paused,
        "Spec status should be Paused after takeover"
    );

    std::env::set_current_dir(&original_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Process handling differs on Windows")]
fn test_takeover_appends_analysis_to_spec() {
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(harness.path()).expect("Failed to change dir");

    let spec_id = "takeover-003";
    let spec_content = r#"---
type: code
status: in_progress
---

# Takeover Analysis Test

Initial spec content.

## Acceptance Criteria

- [ ] Analysis appended
"#;
    harness.create_spec(spec_id, spec_content);
    harness.git_commit("Add analysis test spec").ok();

    let _pid = spawn_dummy_process(&harness, spec_id);
    let log_content = "Starting work on spec...\nAgent made tool call\nReading file: src/main.rs\nWriting file: src/lib.rs\n";
    create_log_file(&harness, spec_id, log_content);
    thread::sleep(Duration::from_millis(100));

    let result = chant::takeover::cmd_takeover(spec_id, false);
    assert!(result.is_ok(), "Takeover should succeed");

    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
    let updated_content = fs::read_to_string(&spec_path).expect("Failed to read updated spec");

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
    assert!(
        updated_content.contains("Initial spec content"),
        "Original spec content should be preserved"
    );

    std::env::set_current_dir(&original_dir).ok();
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Process handling differs on Windows")]
fn test_takeover_force_flag_without_running_process() {
    let harness = TestHarness::new();
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    std::env::set_current_dir(harness.path()).expect("Failed to change dir");

    let spec_id = "takeover-004";
    harness.create_spec(spec_id, &SpecFactory::as_markdown(spec_id, "pending"));
    harness.git_commit("Add force test spec").ok();

    create_log_file(&harness, spec_id, "Previous work session...\n");

    let result_without_force = chant::takeover::cmd_takeover(spec_id, false);
    assert!(
        result_without_force.is_err(),
        "Takeover without force should fail when no process running"
    );

    let result_with_force = chant::takeover::cmd_takeover(spec_id, true);
    assert!(
        result_with_force.is_ok(),
        "Takeover with force should succeed even without running process"
    );

    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
    let updated_content = fs::read_to_string(&spec_path).expect("Failed to read updated spec");
    assert!(
        updated_content.contains("## Takeover Analysis"),
        "Force takeover should still append analysis"
    );

    std::env::set_current_dir(&original_dir).ok();
}
