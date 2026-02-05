use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parse_config() {
    let content = r#"---
project:
  name: test-project

defaults:
  prompt: standard
---

# Config
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.project.name, "test-project");
    assert_eq!(config.defaults.prompt, "standard");
}

#[test]
fn test_parse_minimal_config() {
    let content = r#"---
project:
  name: minimal
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.project.name, "minimal");
    assert_eq!(config.defaults.prompt, "bootstrap"); // default
}

#[test]
fn test_global_config_path() {
    std::env::set_var("HOME", "/home/testuser");
    let path = global_config_path().unwrap();
    // Use PathBuf for cross-platform comparison
    let expected = std::path::PathBuf::from("/home/testuser")
        .join(".config")
        .join("chant")
        .join("config.md");
    assert_eq!(path, expected);
}

#[test]
fn test_load_merged_no_global() {
    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().join("config.md");

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
defaults:
  prompt: custom
---
"#,
    )
    .unwrap();

    let config = Config::load_merged_from(None, &project_path, None).unwrap();
    assert_eq!(config.project.name, "my-project");
    assert_eq!(config.defaults.prompt, "custom");
}

#[test]
fn test_load_merged_with_global() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    fs::write(
        &global_path,
        r#"---
project:
  prefix: global-prefix
defaults:
  branch_prefix: global/
---
"#,
    )
    .unwrap();

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();

    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();

    // Project name from project config
    assert_eq!(config.project.name, "my-project");
    // Prefix from global (not set in project)
    assert_eq!(config.project.prefix.as_deref(), Some("global-prefix"));
    // branch_prefix from global (project uses default)
    assert_eq!(config.defaults.branch_prefix, "global/");
}

#[test]
fn test_load_merged_project_overrides_global() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    fs::write(
        &global_path,
        r#"---
defaults:
  prompt: global-prompt
  branch_prefix: global/
---
"#,
    )
    .unwrap();

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
defaults:
  prompt: project-prompt
  branch_prefix: project/
---
"#,
    )
    .unwrap();

    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();

    // Project values should override global
    assert_eq!(config.defaults.prompt, "project-prompt");
    assert_eq!(config.defaults.branch_prefix, "project/");
}

#[test]
fn test_load_merged_global_not_exists() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("nonexistent.md");
    let project_path = tmp.path().join("project.md");

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();

    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.project.name, "my-project");
}

#[test]
fn test_parse_defaults_model() {
    let content = r#"---
project:
  name: test-project
defaults:
  model: claude-sonnet-4
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.defaults.model, Some("claude-sonnet-4".to_string()));
}

#[test]
fn test_defaults_model_none_when_not_specified() {
    let content = r#"---
project:
  name: test-project
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.defaults.model, None);
}

#[test]
fn test_config_merge_priority() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    // Test case 1: defaults.model - global used when project doesn't specify
    fs::write(
        &global_path,
        r#"---
defaults:
  model: claude-opus-4
---
"#,
    )
    .unwrap();
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.defaults.model, Some("claude-opus-4".to_string()));

    // Test case 2: defaults.model - project overrides global
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
defaults:
  model: claude-sonnet-4
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.defaults.model, Some("claude-sonnet-4".to_string()));
}

// =========================================================================
// PARALLEL CONFIG TESTS
// =========================================================================

#[test]
fn test_parse_parallel_config() {
    let content = r#"---
project:
  name: test-project
parallel:
  agents:
    - name: main
      command: claude
      max_concurrent: 2
    - name: alt1
      command: claude-alt1
      max_concurrent: 3
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.parallel.agents.len(), 2);
    assert_eq!(config.parallel.agents[0].name, "main");
    assert_eq!(config.parallel.agents[0].command, "claude");
    assert_eq!(config.parallel.agents[0].max_concurrent, 2);
    assert_eq!(config.parallel.agents[1].name, "alt1");
    assert_eq!(config.parallel.agents[1].command, "claude-alt1");
    assert_eq!(config.parallel.agents[1].max_concurrent, 3);
    assert_eq!(config.parallel.total_capacity(), 5); // 2 + 3
}

#[test]
fn test_parallel_config_defaults() {
    let content = r#"---
project:
  name: test-project
---
"#;
    let config = Config::parse(content).unwrap();

    // Should have default values
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "main");
    assert_eq!(config.parallel.agents[0].command, "claude");
    assert_eq!(config.parallel.agents[0].max_concurrent, 2);
    assert_eq!(config.parallel.total_capacity(), 2); // Single agent with default max_concurrent
}

#[test]
fn test_parallel_config_partial_agent() {
    let content = r#"---
project:
  name: test-project
parallel:
  agents:
    - name: custom-agent
---
"#;
    let config = Config::parse(content).unwrap();

    // Agent with only name should get default command, max_concurrent, and weight
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "custom-agent");
    assert_eq!(config.parallel.agents[0].command, "claude");
    assert_eq!(config.parallel.agents[0].max_concurrent, 2);
    assert_eq!(config.parallel.agents[0].weight, 1);
}

#[test]
fn test_parse_agent_weight() {
    let content = r#"---
project:
  name: test-project
parallel:
  agents:
    - name: main
      command: claude
      weight: 2
    - name: alt1
      command: claude-alt1
      weight: 1
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.parallel.agents[0].weight, 2);
    assert_eq!(config.parallel.agents[1].weight, 1);
}

#[test]
fn test_parse_rotation_strategy() {
    let content = r#"---
project:
  name: test-project
defaults:
  rotation_strategy: round-robin
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.defaults.rotation_strategy, "round-robin");
}

#[test]
fn test_rotation_strategy_defaults_to_none() {
    let content = r#"---
project:
  name: test-project
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.defaults.rotation_strategy, "none");
}

// =========================================================================
// ENTERPRISE CONFIG TESTS
// =========================================================================

#[test]
fn test_parse_enterprise_config() {
    let content = r#"---
project:
  name: test-project
enterprise:
  derived:
    environment:
      from: branch
      pattern: "^(dev|staging|prod)$"
      validate:
        type: enum
        values:
          - dev
          - staging
          - prod
    team:
      from: env
      pattern: "TEAM_NAME"
  required:
    - environment
    - team
---
"#;
    let config = Config::parse(content).unwrap();

    // Check enterprise config exists
    assert!(!config.enterprise.derived.is_empty());
    assert_eq!(config.enterprise.derived.len(), 2);
    assert!(!config.enterprise.required.is_empty());
    assert_eq!(config.enterprise.required.len(), 2);

    // Check environment field derivation
    let env_field = config.enterprise.derived.get("environment").unwrap();
    assert!(matches!(env_field.from, DerivationSource::Branch));
    assert_eq!(env_field.pattern, "^(dev|staging|prod)$");
    assert!(env_field.validate.is_some());

    // Check team field derivation
    let team_field = config.enterprise.derived.get("team").unwrap();
    assert!(matches!(team_field.from, DerivationSource::Env));
    assert_eq!(team_field.pattern, "TEAM_NAME");
    assert!(team_field.validate.is_none());

    // Check required fields
    assert!(config
        .enterprise
        .required
        .contains(&"environment".to_string()));
    assert!(config.enterprise.required.contains(&"team".to_string()));
}

#[test]
fn test_parse_enterprise_config_with_path_source() {
    let content = r#"---
project:
  name: test-project
enterprise:
  derived:
    project_code:
      from: path
      pattern: "^([a-z]{3})-"
---
"#;
    let config = Config::parse(content).unwrap();

    let field = config.enterprise.derived.get("project_code").unwrap();
    assert!(matches!(field.from, DerivationSource::Path));
    assert_eq!(field.pattern, "^([a-z]{3})-");
}

#[test]
fn test_parse_enterprise_config_with_git_user_source() {
    let content = r#"---
project:
  name: test-project
enterprise:
  derived:
    author:
      from: git_user
      pattern: "name"
---
"#;
    let config = Config::parse(content).unwrap();

    let field = config.enterprise.derived.get("author").unwrap();
    assert!(matches!(field.from, DerivationSource::GitUser));
    assert_eq!(field.pattern, "name");
}

#[test]
fn test_config_without_enterprise_section() {
    let content = r#"---
project:
  name: test-project
defaults:
  prompt: custom
---
"#;
    let config = Config::parse(content).unwrap();

    // Enterprise should default to empty
    assert!(config.enterprise.derived.is_empty());
    assert!(config.enterprise.required.is_empty());
}

#[test]
fn test_enterprise_config_empty_derived() {
    let content = r#"---
project:
  name: test-project
enterprise:
  required:
    - field1
---
"#;
    let config = Config::parse(content).unwrap();

    // Derived should be empty but required should have values
    assert!(config.enterprise.derived.is_empty());
    assert_eq!(config.enterprise.required.len(), 1);
}

#[test]
fn test_enterprise_config_minimal() {
    let content = r#"---
project:
  name: test-project
enterprise: {}
---
"#;
    let config = Config::parse(content).unwrap();

    // Both should be empty
    assert!(config.enterprise.derived.is_empty());
    assert!(config.enterprise.required.is_empty());
}

#[test]
fn test_validation_rule_enum() {
    let content = r#"---
project:
  name: test-project
enterprise:
  derived:
    region:
      from: env
      pattern: "REGION"
      validate:
        type: enum
        values:
          - us-east-1
          - us-west-2
          - eu-central-1
---
"#;
    let config = Config::parse(content).unwrap();

    let field = config.enterprise.derived.get("region").unwrap();
    assert!(field.validate.is_some());

    if let Some(ValidationRule::Enum { values }) = &field.validate {
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"us-east-1".to_string()));
        assert!(values.contains(&"us-west-2".to_string()));
        assert!(values.contains(&"eu-central-1".to_string()));
    } else {
        panic!("Expected Enum validation rule");
    }
}

#[test]
fn test_config_merge_priority_enterprise() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    // Global enterprise config used when project doesn't specify
    fs::write(
        &global_path,
        r#"---
enterprise:
  derived:
    global_field:
      from: branch
      pattern: "pattern1"
  required:
    - global_field
---
"#,
    )
    .unwrap();
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert!(config.enterprise.derived.contains_key("global_field"));
    assert_eq!(config.enterprise.required.len(), 1);

    // Project enterprise overrides global
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
enterprise:
  derived:
    project_field:
      from: env
      pattern: "pattern2"
  required:
    - project_field
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert!(config.enterprise.derived.contains_key("project_field"));
    assert!(!config.enterprise.derived.contains_key("global_field"));
    assert_eq!(config.enterprise.required.len(), 1);
    assert!(config
        .enterprise
        .required
        .contains(&"project_field".to_string()));
}

// =========================================================================
// APPROVAL CONFIG TESTS
// =========================================================================

#[test]
fn test_parse_approval_config_manual() {
    let content = r#"---
project:
  name: test-project
approval:
  rejection_action: manual
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.approval.rejection_action, RejectionAction::Manual);
}

#[test]
fn test_parse_approval_config_dependency() {
    let content = r#"---
project:
  name: test-project
approval:
  rejection_action: dependency
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(
        config.approval.rejection_action,
        RejectionAction::Dependency
    );
}

#[test]
fn test_parse_approval_config_group() {
    let content = r#"---
project:
  name: test-project
approval:
  rejection_action: group
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.approval.rejection_action, RejectionAction::Group);
}

#[test]
fn test_approval_config_defaults_to_manual() {
    let content = r#"---
project:
  name: test-project
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.approval.rejection_action, RejectionAction::Manual);
    assert!(!config.approval.require_approval_for_agent_work);
}

#[test]
fn test_approval_config_require_approval_for_agent_work() {
    let content = r#"---
project:
  name: test-project
approval:
  require_approval_for_agent_work: true
---
"#;
    let config = Config::parse(content).unwrap();
    assert!(config.approval.require_approval_for_agent_work);
}

#[test]
fn test_approval_config_require_approval_for_agent_work_false() {
    let content = r#"---
project:
  name: test-project
approval:
  require_approval_for_agent_work: false
---
"#;
    let config = Config::parse(content).unwrap();
    assert!(!config.approval.require_approval_for_agent_work);
}

#[test]
fn test_approval_config_empty_section() {
    let content = r#"---
project:
  name: test-project
approval: {}
---
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.approval.rejection_action, RejectionAction::Manual);
}

#[test]
fn test_rejection_action_display() {
    assert_eq!(format!("{}", RejectionAction::Manual), "manual");
    assert_eq!(format!("{}", RejectionAction::Dependency), "dependency");
    assert_eq!(format!("{}", RejectionAction::Group), "group");
}

#[test]
fn test_config_merge_priority_approval() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    // Global approval config used when project doesn't specify
    fs::write(
        &global_path,
        r#"---
approval:
  rejection_action: dependency
---
"#,
    )
    .unwrap();
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(
        config.approval.rejection_action,
        RejectionAction::Dependency
    );

    // Project approval overrides global
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
approval:
  rejection_action: group
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.approval.rejection_action, RejectionAction::Group);
}

// =========================================================================
// AGENTS CONFIG TESTS
// =========================================================================

#[test]
fn test_agents_config_overrides_project() {
    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().join("config.md");
    let agents_path = tmp.path().join("agents.md");

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
parallel:
  agents:
    - name: project-agent
      command: claude
      max_concurrent: 1
---
"#,
    )
    .unwrap();

    fs::write(
        &agents_path,
        r#"---
parallel:
  agents:
    - name: override-agent
      command: claude-override
      max_concurrent: 5
---
"#,
    )
    .unwrap();

    let config = Config::load_merged_from(None, &project_path, Some(&agents_path)).unwrap();

    // Agents file should override project agents
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "override-agent");
    assert_eq!(config.parallel.agents[0].command, "claude-override");
    assert_eq!(config.parallel.agents[0].max_concurrent, 5);
}

#[test]
fn test_agents_config_overrides_global() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("config.md");
    let agents_path = tmp.path().join("agents.md");

    fs::write(
        &global_path,
        r#"---
parallel:
  agents:
    - name: global-agent
      command: claude-global
      max_concurrent: 2
---
"#,
    )
    .unwrap();

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();

    fs::write(
        &agents_path,
        r#"---
parallel:
  agents:
    - name: local-agent
      command: claude-local
      max_concurrent: 3
---
"#,
    )
    .unwrap();

    let config =
        Config::load_merged_from(Some(&global_path), &project_path, Some(&agents_path)).unwrap();

    // Agents file should override global agents
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "local-agent");
    assert_eq!(config.parallel.agents[0].command, "claude-local");
    assert_eq!(config.parallel.agents[0].max_concurrent, 3);
}

#[test]
fn test_agents_config_not_exists_uses_global() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("config.md");
    let agents_path = tmp.path().join("nonexistent.md");

    fs::write(
        &global_path,
        r#"---
parallel:
  agents:
    - name: global-agent
      command: claude-global
      max_concurrent: 2
---
"#,
    )
    .unwrap();

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();

    let config =
        Config::load_merged_from(Some(&global_path), &project_path, Some(&agents_path)).unwrap();

    // Should use global agents when agents file doesn't exist
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "global-agent");
}

#[test]
fn test_agents_config_empty_agents_uses_defaults() {
    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().join("config.md");
    let agents_path = tmp.path().join("agents.md");

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
parallel:
  agents:
    - name: project-agent
      command: claude-project
---
"#,
    )
    .unwrap();

    // Empty agents list should not override
    fs::write(
        &agents_path,
        r#"---
parallel:
  agents: []
---
"#,
    )
    .unwrap();

    let config = Config::load_merged_from(None, &project_path, Some(&agents_path)).unwrap();

    // Empty agents list should not override, use project agents
    assert_eq!(config.parallel.agents.len(), 1);
    assert_eq!(config.parallel.agents[0].name, "project-agent");
}

// =========================================================================
// LINT CONFIG TESTS
// =========================================================================

#[test]
fn test_parse_lint_config_with_thresholds() {
    let content = r#"---
project:
  name: test-project
lint:
  thresholds:
    complexity_criteria: 15
    complexity_files: 8
    complexity_words: 75
    simple_criteria: 2
    simple_files: 2
    simple_words: 5
  disable:
    - rule1
    - rule2
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.lint.thresholds.complexity_criteria, 15);
    assert_eq!(config.lint.thresholds.complexity_files, 8);
    assert_eq!(config.lint.thresholds.complexity_words, 75);
    assert_eq!(config.lint.thresholds.simple_criteria, 2);
    assert_eq!(config.lint.thresholds.simple_files, 2);
    assert_eq!(config.lint.thresholds.simple_words, 5);
    assert_eq!(config.lint.disable.len(), 2);
    assert!(config.lint.disable.contains(&"rule1".to_string()));
    assert!(config.lint.disable.contains(&"rule2".to_string()));
}

#[test]
fn test_lint_config_defaults() {
    let content = r#"---
project:
  name: test-project
---
"#;
    let config = Config::parse(content).unwrap();

    // Should have default threshold values
    assert_eq!(config.lint.thresholds.complexity_criteria, 10);
    assert_eq!(config.lint.thresholds.complexity_files, 5);
    assert_eq!(config.lint.thresholds.complexity_words, 150);
    assert_eq!(config.lint.thresholds.simple_criteria, 1);
    assert_eq!(config.lint.thresholds.simple_files, 1);
    assert_eq!(config.lint.thresholds.simple_words, 3);
    assert!(config.lint.disable.is_empty());
}

#[test]
fn test_lint_config_partial_thresholds() {
    let content = r#"---
project:
  name: test-project
lint:
  thresholds:
    complexity_criteria: 20
---
"#;
    let config = Config::parse(content).unwrap();

    // Only complexity_criteria should be overridden, others use defaults
    assert_eq!(config.lint.thresholds.complexity_criteria, 20);
    assert_eq!(config.lint.thresholds.complexity_files, 5);
    assert_eq!(config.lint.thresholds.complexity_words, 150);
    assert_eq!(config.lint.thresholds.simple_criteria, 1);
    assert_eq!(config.lint.thresholds.simple_files, 1);
    assert_eq!(config.lint.thresholds.simple_words, 3);
}

#[test]
fn test_lint_config_disable_only() {
    let content = r#"---
project:
  name: test-project
lint:
  disable:
    - no-empty-title
    - complexity-check
---
"#;
    let config = Config::parse(content).unwrap();

    // Thresholds should use defaults
    assert_eq!(config.lint.thresholds.complexity_criteria, 10);
    // Disable list should be populated
    assert_eq!(config.lint.disable.len(), 2);
    assert!(config.lint.disable.contains(&"no-empty-title".to_string()));
    assert!(config
        .lint
        .disable
        .contains(&"complexity-check".to_string()));
}

#[test]
fn test_lint_config_empty_section() {
    let content = r#"---
project:
  name: test-project
lint: {}
---
"#;
    let config = Config::parse(content).unwrap();

    // Should use all defaults
    assert_eq!(config.lint.thresholds.complexity_criteria, 10);
    assert!(config.lint.disable.is_empty());
}

#[test]
fn test_config_merge_priority_lint() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    // Global lint config used when project doesn't specify
    fs::write(
        &global_path,
        r#"---
lint:
  thresholds:
    complexity_criteria: 15
  disable:
    - global-rule
---
"#,
    )
    .unwrap();
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.lint.thresholds.complexity_criteria, 15);
    assert!(config.lint.disable.contains(&"global-rule".to_string()));

    // Project lint config overrides global
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
lint:
  thresholds:
    complexity_criteria: 25
  disable:
    - project-rule
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.lint.thresholds.complexity_criteria, 25);
    assert!(config.lint.disable.contains(&"project-rule".to_string()));
    assert!(!config.lint.disable.contains(&"global-rule".to_string()));
}

// =========================================================================
// WATCH CONFIG TESTS
// =========================================================================

#[test]
fn test_parse_watch_config_all_fields() {
    let content = r#"---
project:
  name: test-project
watch:
  poll_interval_ms: 10000
  failure:
    max_retries: 5
    retry_delay_ms: 30000
    backoff_multiplier: 1.5
    retryable_patterns:
      - "network timeout"
      - "connection refused"
    on_permanent_failure: stop
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.watch.poll_interval_ms, 10000);
    assert_eq!(config.watch.failure.max_retries, 5);
    assert_eq!(config.watch.failure.retry_delay_ms, 30000);
    assert_eq!(config.watch.failure.backoff_multiplier, 1.5);
    assert_eq!(config.watch.failure.retryable_patterns.len(), 2);
    assert!(config
        .watch
        .failure
        .retryable_patterns
        .contains(&"network timeout".to_string()));
    assert_eq!(
        config.watch.failure.on_permanent_failure,
        OnPermanentFailure::Stop
    );
}

#[test]
fn test_watch_config_defaults() {
    let content = r#"---
project:
  name: test-project
---
"#;
    let config = Config::parse(content).unwrap();

    // Should have default values
    assert_eq!(config.watch.poll_interval_ms, 5000);
    assert_eq!(config.watch.failure.max_retries, 3);
    assert_eq!(config.watch.failure.retry_delay_ms, 60000);
    assert_eq!(config.watch.failure.backoff_multiplier, 2.0);
    assert!(config.watch.failure.retryable_patterns.is_empty());
    assert_eq!(
        config.watch.failure.on_permanent_failure,
        OnPermanentFailure::Skip
    );
}

#[test]
fn test_parse_watch_config_missing_fields() {
    let content = r#"---
project:
  name: test-project
watch:
  poll_interval_ms: 8000
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(config.watch.poll_interval_ms, 8000);
    // Failure config should use defaults
    assert_eq!(config.watch.failure.max_retries, 3);
    assert_eq!(config.watch.failure.retry_delay_ms, 60000);
    assert_eq!(config.watch.failure.backoff_multiplier, 2.0);
}

#[test]
fn test_parse_watch_config_on_permanent_failure_skip() {
    let content = r#"---
project:
  name: test-project
watch:
  failure:
    on_permanent_failure: skip
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(
        config.watch.failure.on_permanent_failure,
        OnPermanentFailure::Skip
    );
}

#[test]
fn test_parse_watch_config_on_permanent_failure_stop() {
    let content = r#"---
project:
  name: test-project
watch:
  failure:
    on_permanent_failure: stop
---
"#;
    let config = Config::parse(content).unwrap();

    assert_eq!(
        config.watch.failure.on_permanent_failure,
        OnPermanentFailure::Stop
    );
}

#[test]
fn test_parse_watch_config_invalid_on_permanent_failure() {
    let content = r#"---
project:
  name: test-project
watch:
  failure:
    on_permanent_failure: invalid_value
---
"#;
    let result = Config::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse config frontmatter"));
}

#[test]
fn test_watch_config_validation_negative_interval() {
    let content = r#"---
project:
  name: test-project
watch:
  poll_interval_ms: 0
---
"#;
    let result = Config::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("poll_interval_ms must be greater than 0"));
}

#[test]
fn test_watch_config_validation_invalid_backoff_multiplier() {
    let content = r#"---
project:
  name: test-project
watch:
  failure:
    backoff_multiplier: 0.5
---
"#;
    let result = Config::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("backoff_multiplier must be >= 1.0"));
}

#[test]
fn test_watch_config_empty_retryable_patterns() {
    let content = r#"---
project:
  name: test-project
watch:
  failure:
    retryable_patterns: []
---
"#;
    let config = Config::parse(content).unwrap();

    // Empty patterns list should be valid
    assert!(config.watch.failure.retryable_patterns.is_empty());
}

#[test]
fn test_config_merge_priority_watch() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    // Global watch config used when project doesn't specify
    fs::write(
        &global_path,
        r#"---
watch:
  poll_interval_ms: 15000
  failure:
    max_retries: 10
---
"#,
    )
    .unwrap();
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.watch.poll_interval_ms, 15000);
    assert_eq!(config.watch.failure.max_retries, 10);

    // Project watch config overrides global
    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
watch:
  poll_interval_ms: 3000
  failure:
    max_retries: 2
    on_permanent_failure: stop
---
"#,
    )
    .unwrap();
    let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
    assert_eq!(config.watch.poll_interval_ms, 3000);
    assert_eq!(config.watch.failure.max_retries, 2);
    assert_eq!(
        config.watch.failure.on_permanent_failure,
        OnPermanentFailure::Stop
    );
}

// =========================================================================
// ERROR HANDLING TESTS FOR MALFORMED CONFIGS
// =========================================================================

#[test]
fn test_parse_config_missing_frontmatter() {
    let content = r#"# Config

Just a markdown file with no frontmatter
"#;
    let result = Config::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to extract frontmatter"));
}

#[test]
fn test_parse_config_invalid_yaml() {
    let content = r#"---
project:
  name: test
  invalid: [unclosed bracket
---
"#;
    let result = Config::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse config frontmatter"));
}

#[test]
fn test_parse_config_missing_project_section() {
    let content = r#"---
defaults:
  prompt: custom
---
"#;
    let result = Config::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse config frontmatter"));
}

#[test]
fn test_load_from_nonexistent_file() {
    let result = Config::load_from(Path::new("/nonexistent/path/config.md"));
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to read config from"));
}

#[test]
fn test_partial_config_load_from_nonexistent_file() {
    let result = PartialConfig::load_from(Path::new("/nonexistent/path/config.md"));
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to read config from"));
}

#[test]
fn test_agents_config_load_from_nonexistent_file() {
    let result = AgentsConfig::load_from(Path::new("/nonexistent/path/agents.md"));
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to read agents config from"));
}

#[test]
fn test_agents_config_missing_frontmatter() {
    let content = r#"# Agents Config

No frontmatter here
"#;
    let result = AgentsConfig::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to extract frontmatter"));
}

#[test]
fn test_agents_config_invalid_yaml() {
    let content = r#"---
parallel:
  agents: [
    name: broken
---
"#;
    let result = AgentsConfig::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse agents config frontmatter"));
}

#[test]
fn test_partial_config_missing_frontmatter() {
    let content = "Just markdown, no frontmatter";
    let result = PartialConfig::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to extract frontmatter"));
}

#[test]
fn test_partial_config_invalid_yaml() {
    let content = r#"---
project: {
  name: "unclosed
---
"#;
    let result = PartialConfig::parse(content);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse config frontmatter"));
}

#[test]
fn test_load_merged_project_config_missing() {
    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().join("nonexistent.md");

    let result = Config::load_merged_from(None, &project_path, None);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to read config from"));
}

#[test]
fn test_load_merged_malformed_global_config() {
    let tmp = TempDir::new().unwrap();
    let global_path = tmp.path().join("global.md");
    let project_path = tmp.path().join("project.md");

    // Write invalid global config
    fs::write(
        &global_path,
        r#"---
invalid: yaml: syntax:
---
"#,
    )
    .unwrap();

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();

    let result = Config::load_merged_from(Some(&global_path), &project_path, None);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse config frontmatter"));
}

#[test]
fn test_load_merged_malformed_agents_config() {
    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().join("project.md");
    let agents_path = tmp.path().join("agents.md");

    fs::write(
        &project_path,
        r#"---
project:
  name: my-project
---
"#,
    )
    .unwrap();

    // Write invalid agents config
    fs::write(
        &agents_path,
        r#"---
parallel: not a valid structure
---
"#,
    )
    .unwrap();

    let result = Config::load_merged_from(None, &project_path, Some(&agents_path));
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Failed to parse agents config frontmatter"));
}
