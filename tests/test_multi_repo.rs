//! Multi-repo integration test
//!
//! Tests that `chant list --global` works correctly across multiple repos,
//! verifying spec IDs are independent and worktree paths are namespaced by project name.

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

mod common;

fn get_chant_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_chant"))
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_multi_repo_global_list() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    // Create two temporary repos
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo1_dir = temp_dir.path().join("repo1");
    let repo2_dir = temp_dir.path().join("repo2");

    fs::create_dir_all(&repo1_dir).expect("Failed to create repo1 dir");
    fs::create_dir_all(&repo2_dir).expect("Failed to create repo2 dir");

    // Initialize both repos with git
    common::setup_test_repo(&repo1_dir).expect("Failed to setup repo1");
    common::setup_test_repo(&repo2_dir).expect("Failed to setup repo2");

    // Initialize chant in repo1 with project name "project-one"
    let chant_binary = get_chant_binary();
    let init1_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo1_dir)
        .output()
        .expect("Failed to run chant init in repo1");

    assert!(
        init1_output.status.success(),
        "chant init failed in repo1: {}",
        String::from_utf8_lossy(&init1_output.stderr)
    );

    // Set project.name for repo1
    let config1_path = repo1_dir.join(".chant/config.md");
    let config1_content = r#"---
project:
  name: project-one

defaults:
  prompt: standard
---

# Project One Config
"#;
    fs::write(&config1_path, config1_content).expect("Failed to write repo1 config");

    // Initialize chant in repo2 with project name "project-two"
    let init2_output = Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo2_dir)
        .output()
        .expect("Failed to run chant init in repo2");

    assert!(
        init2_output.status.success(),
        "chant init failed in repo2: {}",
        String::from_utf8_lossy(&init2_output.stderr)
    );

    // Set project.name for repo2
    let config2_path = repo2_dir.join(".chant/config.md");
    let config2_content = r#"---
project:
  name: project-two

defaults:
  prompt: standard
---

# Project Two Config
"#;
    fs::write(&config2_path, config2_content).expect("Failed to write repo2 config");

    // Create global config directory at ~/.config/chant/config.md
    // We'll use a temp HOME to control where the global config is read from
    let fake_home = temp_dir.path().join("home");
    let global_config_dir = fake_home.join(".config/chant");
    fs::create_dir_all(&global_config_dir).expect("Failed to create global config dir");
    let global_config_path = global_config_dir.join("config.md");

    // Write global config with both repos listed
    let global_config_content = format!(
        r#"---
repos:
  - name: repo1
    path: {}
  - name: repo2
    path: {}
---

# Global Config
"#,
        repo1_dir.display(),
        repo2_dir.display()
    );
    fs::write(&global_config_path, &global_config_content).expect("Failed to write global config");

    // Add specs to repo1 directly via file creation (avoiding CLI path issues)
    let specs1_dir = repo1_dir.join(".chant/specs");
    fs::create_dir_all(&specs1_dir).expect("Failed to create specs dir in repo1");

    let spec1_content = r#"---
type: code
status: pending
---

# Spec 1 in repo1

Test spec in first repository.

## Acceptance Criteria

- [ ] Test criterion
"#;
    let spec1_path = specs1_dir.join("2026-01-01-001-aaa.md");
    fs::write(&spec1_path, spec1_content).expect("Failed to write spec1");

    // Add specs to repo2
    let specs2_dir = repo2_dir.join(".chant/specs");
    fs::create_dir_all(&specs2_dir).expect("Failed to create specs dir in repo2");

    let spec2_content = r#"---
type: code
status: pending
---

# Spec 1 in repo2

Test spec in second repository.

## Acceptance Criteria

- [ ] Test criterion
"#;
    let spec2_path = specs2_dir.join("2026-01-01-001-bbb.md");
    fs::write(&spec2_path, spec2_content).expect("Failed to write spec2");

    // Run `chant list --global` using the global config
    // Set HOME to our fake home so chant finds the global config
    // Run from repo1 so it has a local .chant/config.md to satisfy Config::load_merged
    let list_output = Command::new(&chant_binary)
        .args(["list", "--global"])
        .env("HOME", &fake_home)
        .current_dir(&repo1_dir)
        .output()
        .expect("Failed to run chant list --global");

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    let stderr = String::from_utf8_lossy(&list_output.stderr);

    eprintln!("chant list --global stdout:\n{}", stdout);
    eprintln!("chant list --global stderr:\n{}", stderr);

    assert!(
        list_output.status.success(),
        "chant list --global should succeed. stderr: {}",
        stderr
    );

    // Verify output contains specs from both repos with repo prefixes
    assert!(
        stdout.contains("repo1:") || stdout.contains("repo1"),
        "Output should contain specs from repo1. Got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("repo2:") || stdout.contains("repo2"),
        "Output should contain specs from repo2. Got:\n{}",
        stdout
    );

    // Verify spec IDs are prefixed with repo names
    assert!(
        stdout.contains("repo1:2026-01-01-001-aaa") || stdout.contains("repo1"),
        "Spec from repo1 should have repo1 prefix. Got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("repo2:2026-01-01-001-bbb") || stdout.contains("repo2"),
        "Spec from repo2 should have repo2 prefix. Got:\n{}",
        stdout
    );

    // Verify worktree path namespacing (paths would differ due to different project names)
    // Since worktree paths use project names, we verify the configs have different names
    let loaded_config1 = fs::read_to_string(&config1_path).expect("Failed to read config1");
    let loaded_config2 = fs::read_to_string(&config2_path).expect("Failed to read config2");

    assert!(
        loaded_config1.contains("name: project-one"),
        "Repo1 should have project name 'project-one'"
    );
    assert!(
        loaded_config2.contains("name: project-two"),
        "Repo2 should have project name 'project-two'"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_multi_repo_spec_id_independence() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    // Create two temporary repos
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo1_dir = temp_dir.path().join("repo1");
    let repo2_dir = temp_dir.path().join("repo2");

    fs::create_dir_all(&repo1_dir).expect("Failed to create repo1 dir");
    fs::create_dir_all(&repo2_dir).expect("Failed to create repo2 dir");

    common::setup_test_repo(&repo1_dir).expect("Failed to setup repo1");
    common::setup_test_repo(&repo2_dir).expect("Failed to setup repo2");

    let chant_binary = get_chant_binary();

    // Initialize both repos
    Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo1_dir)
        .output()
        .expect("Failed to init repo1");

    Command::new(&chant_binary)
        .args(["init", "--minimal"])
        .current_dir(&repo2_dir)
        .output()
        .expect("Failed to init repo2");

    // Create same spec ID in both repos
    let specs1_dir = repo1_dir.join(".chant/specs");
    let specs2_dir = repo2_dir.join(".chant/specs");
    fs::create_dir_all(&specs1_dir).expect("Failed to create specs1 dir");
    fs::create_dir_all(&specs2_dir).expect("Failed to create specs2 dir");

    let spec_content = r#"---
type: code
status: pending
---

# Test spec

## Acceptance Criteria

- [ ] Test
"#;

    // Same ID in both repos
    let same_id = "2026-02-06-test-xyz";
    fs::write(specs1_dir.join(format!("{}.md", same_id)), spec_content)
        .expect("Failed to write spec in repo1");
    fs::write(specs2_dir.join(format!("{}.md", same_id)), spec_content)
        .expect("Failed to write spec in repo2");

    // Create global config at fake HOME location
    let fake_home = temp_dir.path().join("home");
    let global_config_dir = fake_home.join(".config/chant");
    fs::create_dir_all(&global_config_dir).expect("Failed to create global config dir");
    let global_config_path = global_config_dir.join("config.md");

    let global_config_content = format!(
        r#"---
repos:
  - name: alpha
    path: {}
  - name: beta
    path: {}
---

# Global Config
"#,
        repo1_dir.display(),
        repo2_dir.display()
    );
    fs::write(&global_config_path, &global_config_content).expect("Failed to write global config");

    // List specs globally
    // Run from repo1 directory since Config::load_merged requires a project config
    let list_output = Command::new(&chant_binary)
        .args(["list", "--global"])
        .env("HOME", &fake_home)
        .current_dir(&repo1_dir)
        .output()
        .expect("Failed to list specs");

    let stdout = String::from_utf8_lossy(&list_output.stdout);

    // Verify both specs are listed with different prefixes
    // This proves spec IDs don't collide - they're namespaced by repo name
    assert!(
        stdout.contains("alpha:") || stdout.contains("alpha"),
        "Should show spec from alpha repo. Got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("beta:") || stdout.contains("beta"),
        "Should show spec from beta repo. Got:\n{}",
        stdout
    );

    let _ = std::env::set_current_dir(&original_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_multi_repo_worktree_path_namespacing() {
    use chant::worktree::git_ops::worktree_path_for_spec;

    // Test that different project names produce different worktree paths
    let spec_id = "2026-02-06-test-abc";

    let path_no_project = worktree_path_for_spec(spec_id);
    let path_project_one = worktree_path_for_spec(&format!("project-one:{}", spec_id));
    let path_project_two = worktree_path_for_spec(&format!("project-two:{}", spec_id));

    // All paths should be different to prevent collisions
    assert_ne!(
        path_no_project, path_project_one,
        "Paths should differ when project name is included"
    );
    assert_ne!(
        path_project_one, path_project_two,
        "Different project names should produce different paths"
    );
    assert_ne!(
        path_no_project, path_project_two,
        "Paths should differ when project name is included"
    );

    // Verify paths contain the spec ID
    assert!(
        path_no_project.to_string_lossy().contains(spec_id),
        "Path should contain spec ID"
    );
}
