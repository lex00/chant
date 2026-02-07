//! Multi-repo integration tests
//!
//! Tests chant's ability to work with multiple repositories via global config.

mod common;

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

/// Create a temporary global config file with specified repos
fn create_global_config(config_path: &Path, repos: &[(&str, &str)]) -> std::io::Result<()> {
    let mut config_content = String::from(
        r#"---
project:
  name: global-config
repos:
"#,
    );

    for (name, path) in repos {
        config_content.push_str(&format!(
            r#"  - name: {}
    path: {}
"#,
            name, path
        ));
    }

    config_content.push_str(
        r#"
---

# Global Config
"#,
    );

    fs::create_dir_all(config_path.parent().unwrap())?;
    fs::write(config_path, config_content)?;
    Ok(())
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_multi_repo_listing() {
    let original_dir = std::env::current_dir().expect("Failed to get cwd");

    // Create two test repos
    let repo1_dir = PathBuf::from("/tmp/test-chant-multi-repo-1");
    let repo2_dir = PathBuf::from("/tmp/test-chant-multi-repo-2");
    let global_config_dir = PathBuf::from("/tmp/test-chant-global-config");

    // Clean up any previous test artifacts
    let _ = common::cleanup_test_repo(&repo1_dir);
    let _ = common::cleanup_test_repo(&repo2_dir);
    let _ = fs::remove_dir_all(&global_config_dir);

    // Set up both repos
    assert!(
        common::setup_test_repo(&repo1_dir).is_ok(),
        "Repo 1 setup failed"
    );
    assert!(
        common::setup_test_repo(&repo2_dir).is_ok(),
        "Repo 2 setup failed"
    );

    // Initialize chant in repo1 with project name "project-alpha"
    let init1_output = run_chant(
        &repo1_dir,
        &["init", "--minimal", "--name", "project-alpha"],
    )
    .expect("Failed to run chant init in repo1");
    if !init1_output.status.success() {
        let stderr = String::from_utf8_lossy(&init1_output.stderr);
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo1_dir);
        let _ = common::cleanup_test_repo(&repo2_dir);
        let _ = fs::remove_dir_all(&global_config_dir);
        panic!("Chant init in repo1 failed: {}", stderr);
    }

    // Initialize chant in repo2 with project name "project-beta"
    let init2_output = run_chant(&repo2_dir, &["init", "--minimal", "--name", "project-beta"])
        .expect("Failed to run chant init in repo2");
    if !init2_output.status.success() {
        let stderr = String::from_utf8_lossy(&init2_output.stderr);
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo1_dir);
        let _ = common::cleanup_test_repo(&repo2_dir);
        let _ = fs::remove_dir_all(&global_config_dir);
        panic!("Chant init in repo2 failed: {}", stderr);
    }

    // Add specs to each repo using the library API
    let specs_dir1 = repo1_dir.join(".chant/specs");
    let specs_dir2 = repo2_dir.join(".chant/specs");

    // Create spec in repo1
    let spec1_id = "2026-02-06-001-abc";
    let spec1_content = r#"---
type: code
status: pending
---

# Test Spec Repo 1

This is a test spec in repo 1.

## Acceptance Criteria

- [x] Test spec created
"#;
    fs::write(specs_dir1.join(format!("{}.md", spec1_id)), spec1_content)
        .expect("Failed to write spec1");

    // Create spec in repo2
    let spec2_id = "2026-02-06-002-xyz";
    let spec2_content = r#"---
type: code
status: pending
---

# Test Spec Repo 2

This is a test spec in repo 2.

## Acceptance Criteria

- [x] Test spec created
"#;
    fs::write(specs_dir2.join(format!("{}.md", spec2_id)), spec2_content)
        .expect("Failed to write spec2");

    // Commit specs in both repos
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo1_dir)
        .output()
        .expect("Failed to add spec1");
    Command::new("git")
        .args(["commit", "-m", "Add spec1"])
        .current_dir(&repo1_dir)
        .output()
        .expect("Failed to commit spec1");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo2_dir)
        .output()
        .expect("Failed to add spec2");
    Command::new("git")
        .args(["commit", "-m", "Add spec2"])
        .current_dir(&repo2_dir)
        .output()
        .expect("Failed to commit spec2");

    // Create global config pointing to both repos
    let global_config_path = global_config_dir.join(".config/chant/config.md");
    create_global_config(
        &global_config_path,
        &[
            ("alpha", repo1_dir.to_str().unwrap()),
            ("beta", repo2_dir.to_str().unwrap()),
        ],
    )
    .expect("Failed to create global config");

    // Set HOME to use our temp global config
    std::env::set_var("HOME", global_config_dir.to_str().unwrap());

    // Test global listing from repo1
    let list_output =
        run_chant(&repo1_dir, &["list", "--global"]).expect("Failed to run chant list --global");

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let list_stderr = String::from_utf8_lossy(&list_output.stderr);

    if !list_output.status.success() {
        eprintln!("List stdout: {}", list_stdout);
        eprintln!("List stderr: {}", list_stderr);
    }

    assert!(list_output.status.success(), "Global list should succeed");

    // Verify specs from both repos are listed with repo prefixes
    assert!(
        list_stdout.contains("alpha:") && list_stdout.contains(spec1_id),
        "Should show spec from repo1 with alpha prefix"
    );
    assert!(
        list_stdout.contains("beta:") && list_stdout.contains(spec2_id),
        "Should show spec from repo2 with beta prefix"
    );

    // Verify spec IDs are independent (no collision)
    // Both specs have similar IDs (001, 002) but should be prefixed differently
    assert!(
        list_stdout.contains(&format!("alpha:{}", spec1_id)),
        "Repo1 spec should have alpha prefix"
    );
    assert!(
        list_stdout.contains(&format!("beta:{}", spec2_id)),
        "Repo2 spec should have beta prefix"
    );

    // Verify worktree paths would be different due to project names
    // This is checked via the worktree_path_for_spec function
    let worktree1 =
        chant::worktree::git_ops::worktree_path_for_spec(spec1_id, Some("project-alpha"));
    let worktree2 =
        chant::worktree::git_ops::worktree_path_for_spec(spec2_id, Some("project-beta"));

    assert_ne!(
        worktree1, worktree2,
        "Worktree paths should be different due to different project names"
    );
    assert!(
        worktree1.to_string_lossy().contains("project-alpha"),
        "Worktree1 should contain project-alpha in path"
    );
    assert!(
        worktree2.to_string_lossy().contains("project-beta"),
        "Worktree2 should contain project-beta in path"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    std::env::remove_var("HOME");
    let _ = common::cleanup_test_repo(&repo1_dir);
    let _ = common::cleanup_test_repo(&repo2_dir);
    let _ = fs::remove_dir_all(&global_config_dir);
}
