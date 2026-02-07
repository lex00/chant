//! Config

use crate::common;
use crate::support;

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
fn test_lint_required_fields_missing() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-lint-required-missing");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);
    std::fs::create_dir_all(&repo_dir).expect("Failed to create temp dir");

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to init git repo");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create a config with required fields (using standard frontmatter fields)
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
enterprise:
  required:
    - branch
    - model
    - labels
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create a spec without required fields
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_path = specs_dir.join("2026-01-27-001-abc.md");
    let spec_content = r#"---
type: code
status: pending
---

# Test spec without required fields

This spec is missing branch, model, and labels fields.
"#;
    std::fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Run lint - should fail
    let lint_cmd = Command::new(&chant_binary)
        .args(["lint"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant lint");

    let stderr = String::from_utf8_lossy(&lint_cmd.stderr);
    let stdout = String::from_utf8_lossy(&lint_cmd.stdout);

    eprintln!("Lint stdout: {}", stdout);
    eprintln!("Lint stderr: {}", stderr);

    // Lint should fail (exit code 1)
    assert!(
        !lint_cmd.status.success(),
        "Lint should fail when required fields are missing"
    );

    // Should report missing required fields
    let output = format!("{}{}", stdout, stderr);
    assert!(
        output.contains("Missing required field 'branch'"),
        "Should report missing branch field"
    );
    assert!(
        output.contains("Missing required field 'model'"),
        "Should report missing model field"
    );
    assert!(
        output.contains("Missing required field 'labels'"),
        "Should report missing labels field"
    );

    // Should mention enterprise policy
    assert!(
        output.contains("Enterprise policy requires"),
        "Should mention enterprise policy"
    );

    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_lint_required_fields_present() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-lint-required-present");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);
    std::fs::create_dir_all(&repo_dir).expect("Failed to create temp dir");

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to init git repo");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create a config with required fields
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
enterprise:
  required:
    - branch
    - labels
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create a spec WITH required fields
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_path = specs_dir.join("2026-01-27-002-def.md");
    let spec_content = r#"---
type: code
status: pending
branch: chant/feature
labels:
  - important
  - feature
---

# Test spec with required fields

This spec has branch and labels fields.
"#;
    std::fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Run lint - should pass
    let lint_cmd = Command::new(&chant_binary)
        .args(["lint"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant lint");

    let stderr = String::from_utf8_lossy(&lint_cmd.stderr);
    let stdout = String::from_utf8_lossy(&lint_cmd.stdout);

    eprintln!("Lint stdout: {}", stdout);
    eprintln!("Lint stderr: {}", stderr);

    // Lint should pass (exit code 0)
    assert!(
        lint_cmd.status.success(),
        "Lint should pass when required fields are present"
    );

    let output = format!("{}{}", stdout, stderr);
    assert!(
        output.contains("All 1 specs valid"),
        "Should report all specs valid"
    );

    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_lint_no_required_fields_configured() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-lint-no-required");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);
    std::fs::create_dir_all(&repo_dir).expect("Failed to create temp dir");

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to init git repo");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create default config without enterprise required fields
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create a spec without any special fields
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    let spec_path = specs_dir.join("2026-01-27-003-ghi.md");
    let spec_content = r#"---
type: code
status: pending
---

# Test spec without required fields config

This spec should pass even without required fields since none are configured.
"#;
    std::fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Run lint - should pass (no required fields configured)
    let lint_cmd = Command::new(&chant_binary)
        .args(["lint"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant lint");

    let stderr = String::from_utf8_lossy(&lint_cmd.stderr);
    let stdout = String::from_utf8_lossy(&lint_cmd.stdout);

    eprintln!("Lint stdout: {}", stdout);
    eprintln!("Lint stderr: {}", stderr);

    // Lint should pass
    assert!(
        lint_cmd.status.success(),
        "Lint should pass when no required fields are configured"
    );

    let output = format!("{}{}", stdout, stderr);
    assert!(
        output.contains("All 1 specs valid"),
        "Should report all specs valid"
    );

    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_env_based_derivation_end_to_end() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-env-deriv");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);
    std::fs::create_dir_all(&repo_dir).expect("Failed to create temp dir");

    // Initialize test repo with setup_test_repo helper
    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    // Manually set up .chant directory (similar to init test)
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create enterprise config with env variable derivation
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

    // Run chant add with environment variables set
    let add_output = Command::new(&chant_binary)
        .args(["add", "Test spec with env derivation"])
        .env("TEAM_NAME", "platform")
        .env("DEPLOY_ENV", "production")
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

    // Verify spec contains values from environment variables in context field
    assert!(
        spec_content.contains("derived_team=platform"),
        "Spec should contain derived_team=platform in context. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("derived_environment=production"),
        "Spec should contain derived_environment=production in context. Got:\n{}",
        spec_content
    );

    // Verify derived_fields tracking
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
        spec_content.contains("- environment") || spec_content.contains("  - environment"),
        "Spec should list 'environment' in derived_fields. Got:\n{}",
        spec_content
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_no_derivation_when_config_empty() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-no-config");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create config WITHOUT enterprise section
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create specs directory
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Run chant add
    let add_output = Command::new(&chant_binary)
        .args(["add", "Test spec without config"])
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

    // Verify spec created normally without derived fields
    assert!(
        !spec_content.contains("derived_fields:"),
        "Spec should NOT contain derived_fields when no enterprise config. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("type: code"),
        "Spec should contain type: code. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("status: pending"),
        "Spec should contain status: pending. Got:\n{}",
        spec_content
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_no_derivation_when_enterprise_derived_empty() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-empty-derived");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    // Manually set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create config with enterprise section but empty derived
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project
enterprise:
  derived: {}
  required: []
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create specs directory
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Run chant add
    let add_output = Command::new(&chant_binary)
        .args(["add", "Test spec with empty derived"])
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

    // Verify no derivation occurred
    assert!(
        !spec_content.contains("derived_fields:"),
        "Spec should NOT contain derived_fields when enterprise.derived is empty. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("type: code"),
        "Spec should contain type: code. Got:\n{}",
        spec_content
    );
    assert!(
        spec_content.contains("status: pending"),
        "Spec should contain status: pending. Got:\n{}",
        spec_content
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test the `chant derive <SPEC_ID>` command re-derives fields for a single spec
/// This verifies:
/// 1. Creating a spec WITHOUT derived fields (no enterprise config initially)
/// 2. Adding enterprise config AFTER spec creation
/// 3. Running `chant derive <SPEC_ID>` to re-derive fields
/// 4. Verifying the spec file is updated with derived fields

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_chant_derive_single_spec() {
    let original_dir = std::env::current_dir().expect("Failed to get current dir");

    let repo_dir = PathBuf::from("/tmp/test-chant-derive-single");
    let chant_binary = get_chant_binary();

    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    std::env::set_current_dir(&repo_dir).expect("Failed to change dir");

    // Initialize chant with minimal config (no enterprise derivation)
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

    // Create specs directory
    let chant_dir = repo_dir.join(".chant");
    let specs_dir = chant_dir.join("specs");
    fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create a spec file manually (simulating spec creation without enterprise config)
    let spec_id = "2026-01-27-test-derive";
    let spec_content = r#"---
type: code
status: pending
---

# Test Spec for Derivation

This spec is created without derived fields.

## Acceptance Criteria

- [ ] Test completed
"#;
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Verify the spec has NO derived fields initially
    let initial_content = fs::read_to_string(&spec_path).expect("Failed to read spec");
    assert!(
        !initial_content.contains("derived_fields:"),
        "Spec should NOT contain derived_fields initially. Got:\n{}",
        initial_content
    );
    assert!(
        !initial_content.contains("component:"),
        "Spec should NOT contain component field initially. Got:\n{}",
        initial_content
    );

    // Now add enterprise config with derivation rules
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project

defaults:
  prompt: standard

enterprise:
  derived:
    component:
      from: path
      pattern: "/([^/]+)\\.md$"
---

# Chant Configuration

Enterprise config added after spec creation.
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    eprintln!("Config written:\n{}", config_content);

    // Run chant derive <SPEC_ID>
    let derive_output = Command::new(&chant_binary)
        .args(["derive", spec_id])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to run chant derive");

    let stdout = String::from_utf8_lossy(&derive_output.stdout);
    let stderr = String::from_utf8_lossy(&derive_output.stderr);

    eprintln!("chant derive stdout: {}", stdout);
    eprintln!("chant derive stderr: {}", stderr);

    assert!(
        derive_output.status.success(),
        "chant derive should succeed. stderr: {}",
        stderr
    );

    // Verify success message in stdout
    // The derive command prints "{spec_id}: updated with N derived field(s)"
    assert!(
        stdout.contains("updated with") || stdout.contains("derived field"),
        "Output should indicate fields were derived. Got:\n{}",
        stdout
    );

    // Verify the spec file now has derived fields
    let updated_content = fs::read_to_string(&spec_path).expect("Failed to read updated spec");

    eprintln!("Updated spec content:\n{}", updated_content);

    // The pattern "/([^/]+)\\.md$" should capture the spec filename
    // Derived fields that aren't standard frontmatter fields (like 'component')
    // are stored in the context field as "derived_{key}={value}"
    assert!(
        updated_content.contains("derived_component="),
        "Spec should contain derived_component in context after derivation. Got:\n{}",
        updated_content
    );

    // Verify derived_fields tracking is added
    assert!(
        updated_content.contains("derived_fields:"),
        "Spec should contain derived_fields tracking. Got:\n{}",
        updated_content
    );
    assert!(
        updated_content.contains("- component"),
        "derived_fields should include component. Got:\n{}",
        updated_content
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

/// Test that spec status is updated to 'completed' after finalization in parallel mode
/// This validates the fix for the issue where parallel execution didn't update spec status

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_output_schema_validation_valid_output() {
    use chant::validation;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.json");

    // Create a simple schema
    let schema = r#"{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["spec_id", "status"],
        "properties": {
            "spec_id": {"type": "string"},
            "status": {"type": "string", "enum": ["success", "failure"]}
        }
    }"#;
    fs::write(&schema_path, schema).unwrap();

    // Simulate agent output with valid JSON
    let agent_output = r#"
Here is my analysis:

```json
{"spec_id": "test-001", "status": "success"}
```

End of report.
"#;

    let result = validation::validate_agent_output("test-001", &schema_path, agent_output).unwrap();

    assert!(result.is_valid, "Expected validation to pass");
    assert!(result.errors.is_empty(), "Expected no errors");
    assert!(result.extracted_json.is_some(), "Expected extracted JSON");
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_output_schema_validation_missing_required_field() {
    use chant::validation;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.json");

    // Schema requires spec_id
    let schema = r#"{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["spec_id"],
        "properties": {
            "spec_id": {"type": "string"},
            "value": {"type": "number"}
        }
    }"#;
    fs::write(&schema_path, schema).unwrap();

    // Agent output missing required field
    let agent_output = r#"{"value": 42}"#;

    let result = validation::validate_agent_output("test-001", &schema_path, agent_output).unwrap();

    assert!(!result.is_valid, "Expected validation to fail");
    assert!(!result.errors.is_empty(), "Expected errors");
    // Check that error mentions missing field
    let error_text = result.errors.join(" ");
    assert!(
        error_text.contains("spec_id") || error_text.contains("required"),
        "Error should mention missing required field: {}",
        error_text
    );
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_output_schema_validation_no_json_in_output() {
    use chant::validation;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.json");

    let schema = r#"{
        "type": "object",
        "properties": {"x": {"type": "string"}}
    }"#;
    fs::write(&schema_path, schema).unwrap();

    let agent_output = "Just some plain text without any JSON.";

    let result = validation::validate_agent_output("test-001", &schema_path, agent_output).unwrap();

    assert!(!result.is_valid, "Expected validation to fail");
    assert!(
        result.extracted_json.is_none(),
        "Expected no extracted JSON"
    );
    assert!(
        result.errors[0].contains("No JSON found"),
        "Error should indicate no JSON found"
    );
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_finalize_validates_output_schema() {
    use chant::spec::{Spec, SpecFrontmatter, SpecStatus};

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let repo_dir = PathBuf::from("/tmp/test-chant-finalize-validation");
    let _ = common::cleanup_test_repo(&repo_dir);

    assert!(common::setup_test_repo(&repo_dir).is_ok(), "Setup failed");

    // Set working directory to repo
    let _ = std::env::set_current_dir(&repo_dir);

    // Set up .chant directory
    let chant_dir = repo_dir.join(".chant");
    std::fs::create_dir_all(&chant_dir).expect("Failed to create .chant dir");

    // Create config
    let config_path = chant_dir.join("config.md");
    let config_content = r#"---
project:
  name: test-project

validation:
  strict_output_validation: false
---

# Config
"#;
    std::fs::write(&config_path, config_content).expect("Failed to write config");

    // Create schemas directory and schema file
    let schemas_dir = chant_dir.join("schemas");
    std::fs::create_dir_all(&schemas_dir).expect("Failed to create schemas dir");

    let schema_content = r#"{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["spec_id", "status"],
        "properties": {
            "spec_id": {"type": "string"},
            "status": {"type": "string"}
        }
    }"#;
    let schema_path = schemas_dir.join("test-schema.json");
    std::fs::write(&schema_path, schema_content).expect("Failed to write schema");

    // Create specs directory
    let specs_dir = chant_dir.join("specs");
    std::fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");

    // Create a spec with output_schema
    let spec_id = "2026-01-29-finalize-validation-test";
    let spec = Spec {
        id: spec_id.to_string(),
        frontmatter: SpecFrontmatter {
            status: SpecStatus::InProgress,
            output_schema: Some(".chant/schemas/test-schema.json".to_string()),
            ..Default::default()
        },
        title: Some("Test Finalize Validation".to_string()),
        body: r#"# Test Finalize Validation

## Acceptance Criteria

- [x] Test criterion
"#
        .to_string(),
    };
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    spec.save(&spec_path).expect("Failed to save spec");

    // Create logs directory and log file with valid JSON
    let logs_dir = chant_dir.join("logs");
    std::fs::create_dir_all(&logs_dir).expect("Failed to create logs dir");

    let log_content = r#"
Agent working on spec...

```json
{"spec_id": "2026-01-29-finalize-validation-test", "status": "completed"}
```

Done.
"#;
    let log_path = logs_dir.join(format!("{}.log", spec_id));
    std::fs::write(&log_path, log_content).expect("Failed to write log file");

    // Create a git commit to associate with the spec
    let test_file = repo_dir.join("test_changes.txt");
    std::fs::write(&test_file, "Some changes").expect("Failed to write test file");

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output();

    let _ = Command::new("git")
        .args(["commit", "-m", &format!("chant({}): test commit", spec_id)])
        .current_dir(&repo_dir)
        .output();

    // Run chant finalize
    let chant_binary = get_chant_binary();
    let output = Command::new(&chant_binary)
        .args(["finalize", spec_id])
        .current_dir(&repo_dir)
        .env("CHANT_TEST_MODE", "1")
        .output()
        .expect("Failed to run chant finalize");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should indicate validation passed
    assert!(
        stdout.contains("Output validation passed") || stderr.contains("Output validation passed"),
        "Should show validation passed. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Clean up
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&repo_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_config_loading_global_and_project_merge() {
    use chant::config::Config;

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let tmp_dir = PathBuf::from("/tmp/test-chant-config-merge");
    let _ = common::cleanup_test_repo(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("Failed to create temp dir");

    // Set up mock global config location
    let global_config_dir = tmp_dir.join("global_config");
    fs::create_dir_all(&global_config_dir).expect("Failed to create global config dir");
    let global_config_path = global_config_dir.join("config.md");

    // Write global config
    let global_config_content = r#"---
project:
  prefix: global-prefix
defaults:
  prompt: global-prompt
  branch_prefix: global/
  model: claude-opus-4
parallel:
  agents:
    - name: global-agent
      command: global-claude
      max_concurrent: 5
---

# Global Config
"#;
    fs::write(&global_config_path, global_config_content).expect("Failed to write global config");

    // Set up project config
    let project_dir = tmp_dir.join("project");
    fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    let project_config_path = project_dir.join("config.md");

    let project_config_content = r#"---
project:
  name: test-project
defaults:
  prompt: project-prompt
---

# Project Config
"#;
    fs::write(&project_config_path, project_config_content)
        .expect("Failed to write project config");

    // Load merged config
    let config = Config::load_merged_from(Some(&global_config_path), &project_config_path, None)
        .expect("Failed to load merged config");

    // Verify merge behavior
    assert_eq!(config.project.name, "test-project"); // From project
    assert_eq!(config.project.prefix.as_deref(), Some("global-prefix")); // From global
    assert_eq!(config.defaults.prompt, "project-prompt"); // Project overrides global
    assert_eq!(config.defaults.branch_prefix, "global/"); // From global
    assert_eq!(config.defaults.model, Some("claude-opus-4".to_string())); // From global
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "global-agent"); // From global

    // Clean up
    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&tmp_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_config_loading_project_overrides_all_global_fields() {
    use chant::config::Config;

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let tmp_dir = PathBuf::from("/tmp/test-chant-config-override-all");
    let _ = common::cleanup_test_repo(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("Failed to create temp dir");

    let global_config_dir = tmp_dir.join("global_config");
    fs::create_dir_all(&global_config_dir).expect("Failed to create global config dir");
    let global_config_path = global_config_dir.join("config.md");

    let global_config_content = r#"---
defaults:
  prompt: global-prompt
  branch_prefix: global/
  model: claude-haiku-4
  main_branch: global-main
parallel:
  agents:
    - name: global-agent
      command: global-claude
enterprise:
  derived:
    team:
      from: env
      pattern: "GLOBAL_TEAM"
  required:
    - team
---
"#;
    fs::write(&global_config_path, global_config_content).expect("Failed to write global config");

    let project_dir = tmp_dir.join("project");
    fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    let project_config_path = project_dir.join("config.md");

    let project_config_content = r#"---
project:
  name: test-project
defaults:
  prompt: project-prompt
  branch_prefix: project/
  model: claude-sonnet-4
  main_branch: main
parallel:
  agents:
    - name: project-agent
      command: project-claude
enterprise:
  derived:
    component:
      from: path
      pattern: "([^/]+)\\.md$"
  required:
    - component
---
"#;
    fs::write(&project_config_path, project_config_content)
        .expect("Failed to write project config");

    let config = Config::load_merged_from(Some(&global_config_path), &project_config_path, None)
        .expect("Failed to load merged config");

    // All fields should be from project config
    assert_eq!(config.defaults.prompt, "project-prompt");
    assert_eq!(config.defaults.branch_prefix, "project/");
    assert_eq!(config.defaults.model, Some("claude-sonnet-4".to_string()));
    assert_eq!(config.defaults.main_branch, "main");
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "project-agent");
    assert!(config.enterprise.derived.contains_key("component"));
    assert!(!config.enterprise.derived.contains_key("team"));
    assert_eq!(config.enterprise.required.len(), 1);
    assert!(config
        .enterprise
        .required
        .contains(&"component".to_string()));

    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&tmp_dir);
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_config_loading_no_global_uses_project_only() {
    use chant::config::Config;

    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    let tmp_dir = PathBuf::from("/tmp/test-chant-config-no-global");
    let _ = common::cleanup_test_repo(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("Failed to create temp dir");

    let project_dir = tmp_dir.join("project");
    fs::create_dir_all(&project_dir).expect("Failed to create project dir");
    let project_config_path = project_dir.join("config.md");

    let project_config_content = r#"---
project:
  name: test-project
  prefix: project-prefix
defaults:
  prompt: project-prompt
  model: claude-sonnet-4
---
"#;
    fs::write(&project_config_path, project_config_content)
        .expect("Failed to write project config");

    // Load without global config
    let config =
        Config::load_merged_from(None, &project_config_path, None).expect("Failed to load config");

    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.project.prefix.as_deref(), Some("project-prefix"));
    assert_eq!(config.defaults.prompt, "project-prompt");
    assert_eq!(config.defaults.model, Some("claude-sonnet-4".to_string()));

    let _ = std::env::set_current_dir(&original_dir);
    let _ = common::cleanup_test_repo(&tmp_dir);
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_missing_env_var_graceful_failure() {
    use support::harness::TestHarness;

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

    let harness = TestHarness::with_config(config_content);

    // Run chant add WITHOUT setting environment variables
    let add_output = Command::new(&harness.chant_binary)
        .args(["add", "Test spec"])
        .current_dir(harness.path())
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
        panic!("chant add failed");
    }

    assert!(
        add_output.status.success(),
        "chant add should succeed even with missing env vars"
    );

    // Read the created spec
    let spec_files: Vec<_> = fs::read_dir(&harness.specs_dir)
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
}

#[test]
#[cfg_attr(target_os = "windows", ignore = "Uses Unix /tmp paths")]
fn test_partial_env_vars_available() {
    use support::harness::TestHarness;

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

    let harness = TestHarness::with_config(config_content);

    // Run chant add with only one env var set
    let add_output = Command::new(&harness.chant_binary)
        .args(["add", "Test spec"])
        .env("TEAM_NAME", "platform")
        .current_dir(harness.path())
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
        panic!("chant add failed");
    }

    assert!(
        add_output.status.success(),
        "chant add should succeed with partial env vars"
    );

    // Verify partial success
    let spec_files: Vec<_> = fs::read_dir(&harness.specs_dir)
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
}
