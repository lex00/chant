//! Workflow

mod common {
    pub use crate::common::*;
}

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
fn get_chant_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_chant"))
}

#[allow(dead_code)]
fn run_chant(repo_dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    let chant_binary = get_chant_binary();
    Command::new(&chant_binary)
        .args(args)
        .current_dir(repo_dir)
        .output()
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_missing_env_var_graceful_failure() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-missing-env");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);
    std::fs::create_dir_all(&repo_dir).expect("Failed to create temp dir");

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to init git repo");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to config git user.email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to config git user.name");

    // Create initial commit
    std::fs::write(repo_dir.join("README.md"), "# Test Repo").expect("Failed to write README");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to git commit");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create enterprise config with env variable and path derivation
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
enterprise:
  derived:
    team:
      from: env
      pattern: "TEAM_NAME"
    environment:
      from: env
      pattern: "DEPLOY_ENV"
    component:
      from: path
      pattern: "/([^/]+)\\.md$"
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create specs directory
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Run chant add WITHOUT setting environment variables
    let add_output = Command::new(&chant_binary)
        .args(["add", "Test spec"])
        .current_dir(&repo_dir)
        // Explicitly do NOT set TEAM_NAME or DEPLOY_ENV
        .output()
        .expect("Failed to run chant add");

    if !add_output.status.success() {
        eprintln!(
            "chant add stderr: {}",
            String::from_utf8_lossy(&add_output.stderr)
        );
        eprintln!(
            "chant add stdout: {}",
            String::from_utf8_lossy(&add_output.stdout)
        );
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo_dir);
        panic!("chant add failed");
    }

    // Command should succeed
    assert!(
        add_output.status.success(),
        "chant add should succeed even with missing env vars"
    );

    // Read the created spec
    let spec_files: Vec<_> = fs::read_dir(&specs_dir)
        .expect("Failed to read specs directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert!(!spec_files.is_empty(), "No spec file was created");

    let spec_file = spec_files[0].path();
    let spec_content = fs::read_to_string(&spec_file).expect("Failed to read spec file");

    eprintln!("Spec content:\n{}", spec_content);

    // team and environment should be missing (env vars not set)
    assert!(
        !spec_content.contains("derived_team"),
        "Spec should not contain derived_team when TEAM_NAME env var is missing. Got:\n{}",
        spec_content
    );
    assert!(
        !spec_content.contains("derived_environment"),
        "Spec should not contain derived_environment when DEPLOY_ENV env var is missing. Got:\n{}",
        spec_content
    );

    // component should work (derived from path, doesn't depend on env)
    assert!(
        spec_content.contains("derived_component"),
        "Spec should contain derived_component from path. Got:\n{}",
        spec_content
    );

    // derived_fields should only list component
    assert!(
        spec_content.contains("derived_fields:"),
        "Spec should track derived_fields. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("- component") || spec_content.contains("  - component"),
        "Spec should list 'component' in derived_fields. Got:\n{}",
        spec_content
    );
    assert!(
        !spec_content.contains("- team") && !spec_content.contains("  - team"),
        "Spec should NOT list 'team' in derived_fields when env var missing. Got:\n{}",
        spec_content
    );
    assert!(
        !spec_content.contains("- environment") && !spec_content.contains("  - environment"),
        "Spec should NOT list 'environment' in derived_fields when env var missing. Got:\n{}",
        spec_content
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_partial_env_vars_available() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-partial-env");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);
    std::fs::create_dir_all(&repo_dir).expect("Failed to create temp dir");

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to init git repo");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to config git user.email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to config git user.name");

    // Create initial commit
    std::fs::write(repo_dir.join("README.md"), "# Test Repo").expect("Failed to write README");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to git commit");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create enterprise config expecting multiple env vars
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
enterprise:
  derived:
    team:
      from: env
      pattern: "TEAM_NAME"
    environment:
      from: env
      pattern: "DEPLOY_ENV"
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create specs directory
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Run chant add with only one env var set
    let add_output = Command::new(&chant_binary)
        .args(["add", "Test spec"])
        .env("TEAM_NAME", "platform") // Set this one
        // DEPLOY_ENV not set
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant add");

    if !add_output.status.success() {
        eprintln!(
            "chant add stderr: {}",
            String::from_utf8_lossy(&add_output.stderr)
        );
        eprintln!(
            "chant add stdout: {}",
            String::from_utf8_lossy(&add_output.stdout)
        );
        let _ = std::env::set_current_dir(&original_dir);
        let _ = common::cleanup_test_repo(&repo_dir);
        panic!("chant add failed");
    }

    assert!(
        add_output.status.success(),
        "chant add should succeed with partial env vars"
    );

    // Verify partial success
    let spec_files: Vec<_> = fs::read_dir(&specs_dir)
        .expect("Failed to read specs directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert!(!spec_files.is_empty(), "No spec file was created");

    let spec_file = spec_files[0].path();
    let spec_content = fs::read_to_string(&spec_file).expect("Failed to read spec file");

    eprintln!("Spec content:\n{}", spec_content);

    // team should be present (env var was set)
    assert!(
        spec_content.contains("derived_team=platform"),
        "Spec should contain derived_team=platform when TEAM_NAME is set. Got:\n{}",
        spec_content
    );

    // environment should be missing (env var not set)
    assert!(
        !spec_content.contains("derived_environment"),
        "Spec should not contain derived_environment when DEPLOY_ENV is not set. Got:\n{}",
        spec_content
    );

    // derived_fields should only list team
    assert!(
        spec_content.contains("derived_fields:"),
        "Spec should track derived_fields. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("- team") || spec_content.contains("  - team"),
        "Spec should list 'team' in derived_fields. Got:\n{}",
        spec_content
    );
    assert!(
        !spec_content.contains("- environment") && !spec_content.contains("  - environment"),
        "Spec should NOT list 'environment' in derived_fields when env var missing. Got:\n{}",
        spec_content
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}
