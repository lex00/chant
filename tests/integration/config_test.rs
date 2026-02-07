//! Config

use crate::support::harness::TestHarness;

use serial_test::serial;
use std::fs;
use std::process::Command;

#[test]
fn test_lint_required_fields_missing() {
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

    let harness = TestHarness::with_config(config_content);

    let spec_content = r#"---
type: code
status: pending
---

# Test spec without required fields

This spec is missing branch, model, and labels fields.
"#;
    harness.create_spec("2026-01-27-001-abc", spec_content);

    let lint_cmd = harness.run(&["lint"]).expect("Failed to run chant lint");

    let stderr = String::from_utf8_lossy(&lint_cmd.stderr);
    let stdout = String::from_utf8_lossy(&lint_cmd.stdout);

    eprintln!("Lint stdout: {}", stdout);
    eprintln!("Lint stderr: {}", stderr);

    assert!(
        !lint_cmd.status.success(),
        "Lint should fail when required fields are missing"
    );

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
    assert!(
        output.contains("Enterprise policy requires"),
        "Should mention enterprise policy"
    );
}

#[test]
fn test_lint_required_fields_present() {
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

    let harness = TestHarness::with_config(config_content);

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
    harness.create_spec("2026-01-27-002-def", spec_content);

    let lint_cmd = harness.run(&["lint"]).expect("Failed to run chant lint");

    let stderr = String::from_utf8_lossy(&lint_cmd.stderr);
    let stdout = String::from_utf8_lossy(&lint_cmd.stdout);

    eprintln!("Lint stdout: {}", stdout);
    eprintln!("Lint stderr: {}", stderr);

    assert!(
        lint_cmd.status.success(),
        "Lint should pass when required fields are present"
    );

    let output = format!("{}{}", stdout, stderr);
    assert!(
        output.contains("All 1 specs valid"),
        "Should report all specs valid"
    );
}

#[test]
fn test_lint_no_required_fields_configured() {
    let config_content = r#"---
project:
  name: test-project
---

# Config
"#;

    let harness = TestHarness::with_config(config_content);

    let spec_content = r#"---
type: code
status: pending
---

# Test spec without required fields config

This spec should pass even without required fields since none are configured.
"#;
    harness.create_spec("2026-01-27-003-ghi", spec_content);

    let lint_cmd = harness.run(&["lint"]).expect("Failed to run chant lint");

    let stderr = String::from_utf8_lossy(&lint_cmd.stderr);
    let stdout = String::from_utf8_lossy(&lint_cmd.stdout);

    eprintln!("Lint stdout: {}", stdout);
    eprintln!("Lint stderr: {}", stderr);

    assert!(
        lint_cmd.status.success(),
        "Lint should pass when no required fields are configured"
    );

    let output = format!("{}{}", stdout, stderr);
    assert!(
        output.contains("All 1 specs valid"),
        "Should report all specs valid"
    );
}

#[test]
fn test_env_based_derivation_end_to_end() {
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

    let add_output = Command::new(&harness.chant_binary)
        .args(["add", "Test spec with env derivation"])
        .env("TEAM_NAME", "platform")
        .env("DEPLOY_ENV", "production")
        .current_dir(harness.path())
        .output()
        .expect("Failed to run chant add");

    assert!(
        add_output.status.success(),
        "chant add failed: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    let spec_files: Vec<_> = fs::read_dir(&harness.specs_dir)
        .expect("Failed to read specs directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert!(!spec_files.is_empty(), "No spec file was created");

    let spec_file = spec_files[0].path();
    let spec_content = fs::read_to_string(&spec_file).expect("Failed to read spec file");

    eprintln!("Spec content:\n{}", spec_content);

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
}

#[test]
#[serial]
fn test_no_derivation_when_config_empty() {
    let config_content = r#"---
project:
  name: test-project
---

# Config
"#;

    let harness = TestHarness::with_config(config_content);

    let add_output = harness
        .run(&["add", "Test spec without config"])
        .expect("Failed to run chant add");

    assert!(
        add_output.status.success(),
        "chant add failed: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    let spec_files: Vec<_> = fs::read_dir(&harness.specs_dir)
        .expect("Failed to read specs directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert!(!spec_files.is_empty(), "No spec file was created");

    let spec_file = spec_files[0].path();
    let spec_content = fs::read_to_string(&spec_file).expect("Failed to read spec file");

    eprintln!("Spec content:\n{}", spec_content);

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
}

#[test]
#[serial]
fn test_no_derivation_when_enterprise_derived_empty() {
    let config_content = r#"---
project:
  name: test-project
enterprise:
  derived: {}
  required: []
---

# Config
"#;

    let harness = TestHarness::with_config(config_content);

    let add_output = harness
        .run(&["add", "Test spec with empty derived"])
        .expect("Failed to run chant add");

    assert!(
        add_output.status.success(),
        "chant add failed: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    let spec_files: Vec<_> = fs::read_dir(&harness.specs_dir)
        .expect("Failed to read specs directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert!(!spec_files.is_empty(), "No spec file was created");

    let spec_file = spec_files[0].path();
    let spec_content = fs::read_to_string(&spec_file).expect("Failed to read spec file");

    eprintln!("Spec content:\n{}", spec_content);

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
}

/// Test the `chant derive <SPEC_ID>` command re-derives fields for a single spec
/// This verifies:
/// 1. Creating a spec WITHOUT derived fields (no enterprise config initially)
/// 2. Adding enterprise config AFTER spec creation
/// 3. Running `chant derive <SPEC_ID>` to re-derive fields
/// 4. Verifying the spec file is updated with derived fields

#[test]
#[serial]
fn test_chant_derive_single_spec() {
    let harness = TestHarness::new();

    let init_output = harness
        .run(&["init", "--minimal"])
        .expect("Failed to run chant init");
    assert!(
        init_output.status.success(),
        "chant init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

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
    harness.create_spec(spec_id, spec_content);

    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
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
    fs::write(&harness.config_path, config_content).expect("Failed to write config");

    eprintln!("Config written:\n{}", config_content);

    let derive_output = harness
        .run(&["derive", spec_id])
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

    assert!(
        stdout.contains("updated with") || stdout.contains("derived field"),
        "Output should indicate fields were derived. Got:\n{}",
        stdout
    );

    let updated_content = fs::read_to_string(&spec_path).expect("Failed to read updated spec");

    eprintln!("Updated spec content:\n{}", updated_content);

    assert!(
        updated_content.contains("derived_component="),
        "Spec should contain derived_component in context after derivation. Got:\n{}",
        updated_content
    );

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
}

/// Test that spec status is updated to 'completed' after finalization in parallel mode
/// This validates the fix for the issue where parallel execution didn't update spec status

#[test]
fn test_output_schema_validation_valid_output() {
    use chant::validation;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.json");

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
fn test_output_schema_validation_missing_required_field() {
    use chant::validation;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.json");

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

    let agent_output = r#"{"value": 42}"#;

    let result = validation::validate_agent_output("test-001", &schema_path, agent_output).unwrap();

    assert!(!result.is_valid, "Expected validation to fail");
    assert!(!result.errors.is_empty(), "Expected errors");
    let error_text = result.errors.join(" ");
    assert!(
        error_text.contains("spec_id") || error_text.contains("required"),
        "Error should mention missing required field: {}",
        error_text
    );
}

#[test]
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
fn test_finalize_validates_output_schema() {
    use chant::spec::{Spec, SpecFrontmatter, SpecStatus};

    let config_content = r#"---
project:
  name: test-project

validation:
  strict_output_validation: false
---

# Config
"#;

    let harness = TestHarness::with_config(config_content);

    let schemas_dir = harness.path().join(".chant/schemas");
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
    let spec_path = harness.specs_dir.join(format!("{}.md", spec_id));
    spec.save(&spec_path).expect("Failed to save spec");

    let logs_dir = harness.path().join(".chant/logs");
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

    let test_file = harness.path().join("test_changes.txt");
    std::fs::write(&test_file, "Some changes").expect("Failed to write test file");

    harness
        .git_commit(&format!("chant({}): test commit", spec_id))
        .expect("Failed to create git commit");

    let output = harness
        .run(&["finalize", spec_id])
        .expect("Failed to run chant finalize");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("Output validation passed") || stderr.contains("Output validation passed"),
        "Should show validation passed. stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

#[test]
#[serial]
fn test_config_loading_global_and_project_merge() {
    use chant::config::Config;
    use tempfile::TempDir;

    let tmp_dir = TempDir::new().expect("Failed to create temp dir");

    let global_config_dir = tmp_dir.path().join("global_config");
    fs::create_dir_all(&global_config_dir).expect("Failed to create global config dir");
    let global_config_path = global_config_dir.join("config.md");

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

    let project_dir = tmp_dir.path().join("project");
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

    let config = Config::load_merged_from(Some(&global_config_path), &project_config_path, None)
        .expect("Failed to load merged config");

    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.project.prefix.as_deref(), Some("global-prefix"));
    assert_eq!(config.defaults.prompt, "project-prompt");
    assert_eq!(config.defaults.branch_prefix, "global/");
    assert_eq!(config.defaults.model, Some("claude-opus-4".to_string()));
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "global-agent");
}

#[test]
#[serial]
fn test_config_loading_project_overrides_all_global_fields() {
    use chant::config::Config;
    use tempfile::TempDir;

    let tmp_dir = TempDir::new().expect("Failed to create temp dir");

    let global_config_dir = tmp_dir.path().join("global_config");
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

    let project_dir = tmp_dir.path().join("project");
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
}

#[test]
#[serial]
fn test_config_loading_no_global_uses_project_only() {
    use chant::config::Config;
    use tempfile::TempDir;

    let tmp_dir = TempDir::new().expect("Failed to create temp dir");

    let project_dir = tmp_dir.path().join("project");
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

    let config =
        Config::load_merged_from(None, &project_config_path, None).expect("Failed to load config");

    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.project.prefix.as_deref(), Some("project-prefix"));
    assert_eq!(config.defaults.prompt, "project-prompt");
    assert_eq!(config.defaults.model, Some("claude-sonnet-4".to_string()));
}

#[test]
fn test_missing_env_var_graceful_failure() {
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
fn test_partial_env_vars_available() {
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
