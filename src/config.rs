//! Configuration management for chant projects.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/config.md
//! - ignore: false

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::provider::{ProviderConfig, ProviderType};
use crate::spec::split_frontmatter;

/// Rejection action mode for approval workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RejectionAction {
    /// Leave rejected, user handles it manually
    #[default]
    Manual,
    /// Prompt to create fix spec, original becomes blocked with depends_on
    Dependency,
    /// Convert to driver with numbered member specs
    Group,
}

impl fmt::Display for RejectionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectionAction::Manual => write!(f, "manual"),
            RejectionAction::Dependency => write!(f, "dependency"),
            RejectionAction::Group => write!(f, "group"),
        }
    }
}

/// Approval workflow configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ApprovalConfig {
    /// Action to take when a spec is rejected
    #[serde(default)]
    pub rejection_action: RejectionAction,
}

/// Enterprise configuration for derived frontmatter and validation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseConfig {
    /// Field derivation rules (which fields to derive, from what source, using what pattern)
    #[serde(default)]
    pub derived: HashMap<String, DerivedFieldConfig>,
    /// List of required field names to validate
    #[serde(default)]
    pub required: Vec<String>,
}

/// Configuration for a single derived field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedFieldConfig {
    /// Source of the derived value
    pub from: DerivationSource,
    /// Pattern for extracting/formatting the value
    pub pattern: String,
    /// Optional validation rule for the derived value
    #[serde(default)]
    pub validate: Option<ValidationRule>,
}

/// Source of a derived field value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationSource {
    /// Derive from git branch name
    Branch,
    /// Derive from file path
    Path,
    /// Derive from environment variable
    Env,
    /// Derive from git user information
    GitUser,
}

/// Validation rule for derived fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationRule {
    /// Enum validation: value must be one of the specified values
    Enum {
        /// List of allowed values
        values: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub providers: ProviderConfig,
    #[serde(default)]
    pub parallel: ParallelConfig,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
    #[serde(default)]
    pub enterprise: EnterpriseConfig,
    #[serde(default)]
    pub approval: ApprovalConfig,
}

/// Configuration for a single repository in cross-repo dependency resolution
#[derive(Debug, Clone, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub path: String,
}

/// Configuration for parallel execution with multiple agents
#[derive(Debug, Deserialize, Clone)]
pub struct ParallelConfig {
    /// List of available agents (Claude accounts/commands)
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    /// Cleanup configuration
    #[serde(default)]
    pub cleanup: CleanupConfig,
    /// Delay in milliseconds between spawning each agent to avoid API rate limiting
    #[serde(default = "default_stagger_delay_ms")]
    pub stagger_delay_ms: u64,
    /// Jitter in milliseconds for spawn delays (default: 20% of stagger_delay_ms)
    #[serde(default = "default_stagger_jitter_ms")]
    pub stagger_jitter_ms: u64,
}

impl ParallelConfig {
    /// Calculate total capacity as sum of all agent max_concurrent values
    pub fn total_capacity(&self) -> usize {
        self.agents.iter().map(|a| a.max_concurrent).sum()
    }
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            agents: vec![AgentConfig::default()],
            cleanup: CleanupConfig::default(),
            stagger_delay_ms: default_stagger_delay_ms(),
            stagger_jitter_ms: default_stagger_jitter_ms(),
        }
    }
}

/// Configuration for a single agent (Claude account/command)
#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    /// Name of the agent (for display and attribution)
    #[serde(default = "default_agent_name")]
    pub name: String,
    /// Shell command to invoke this agent (e.g., "claude", "claude-alt1")
    #[serde(default = "default_agent_command")]
    pub command: String,
    /// Maximum concurrent instances for this agent
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    /// Weight for agent rotation selection (higher = more likely to be selected)
    #[serde(default = "default_agent_weight")]
    pub weight: usize,
}

fn default_agent_weight() -> usize {
    1
}

fn default_agent_name() -> String {
    "main".to_string()
}

fn default_agent_command() -> String {
    "claude".to_string()
}

fn default_max_concurrent() -> usize {
    2
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: default_agent_name(),
            command: default_agent_command(),
            max_concurrent: default_max_concurrent(),
            weight: default_agent_weight(),
        }
    }
}

/// Configuration for post-parallel cleanup
#[derive(Debug, Deserialize, Clone)]
pub struct CleanupConfig {
    /// Whether cleanup is enabled
    #[serde(default = "default_cleanup_enabled")]
    pub enabled: bool,
    /// Prompt to use for cleanup agent
    #[serde(default = "default_cleanup_prompt")]
    pub prompt: String,
    /// Whether to automatically run cleanup without confirmation
    #[serde(default)]
    pub auto_run: bool,
}

fn default_stagger_delay_ms() -> u64 {
    1000 // Default 1 second between agent spawns
}

fn default_stagger_jitter_ms() -> u64 {
    200 // Default 20% of stagger_delay_ms (200ms is 20% of 1000ms)
}

fn default_cleanup_enabled() -> bool {
    true
}

fn default_cleanup_prompt() -> String {
    "parallel-cleanup".to_string()
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            enabled: default_cleanup_enabled(),
            prompt: default_cleanup_prompt(),
            auto_run: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[allow(dead_code)]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub branch: bool,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
    /// Default model name to use when env vars are not set
    #[serde(default)]
    pub model: Option<String>,
    /// Default model name for split operations (defaults to sonnet)
    #[serde(default)]
    pub split_model: Option<String>,
    /// Default main branch name for merges (defaults to "main")
    #[allow(dead_code)]
    #[serde(default = "default_main_branch")]
    pub main_branch: String,
    /// Default provider (claude, ollama, openai)
    #[serde(default)]
    pub provider: ProviderType,
    /// Agent rotation strategy for single spec execution (none, random, round-robin)
    #[serde(default = "default_rotation_strategy")]
    pub rotation_strategy: String,
}

fn default_rotation_strategy() -> String {
    "none".to_string()
}

fn default_prompt() -> String {
    "bootstrap".to_string()
}

fn default_branch_prefix() -> String {
    "chant/".to_string()
}

fn default_main_branch() -> String {
    "main".to_string()
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            prompt: default_prompt(),
            branch: false,
            branch_prefix: default_branch_prefix(),
            model: None,
            split_model: None,
            main_branch: default_main_branch(),
            provider: ProviderType::Claude,
            rotation_strategy: default_rotation_strategy(),
        }
    }
}

impl Config {
    /// Load configuration with full merge semantics.
    /// Merge order (later overrides earlier):
    /// 1. Global config (~/.config/chant/config.md)
    /// 2. Project config (.chant/config.md)
    /// 3. Project agents config (.chant/agents.md) - only for parallel.agents
    pub fn load() -> Result<Self> {
        Self::load_merged_from(
            global_config_path().as_deref(),
            Path::new(".chant/config.md"),
            Some(Path::new(".chant/agents.md")),
        )
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self> {
        // Extract YAML frontmatter using shared function
        let (frontmatter, _body) = split_frontmatter(content);
        let frontmatter = frontmatter.context("Failed to extract frontmatter from config")?;

        serde_yaml::from_str(&frontmatter).context("Failed to parse config frontmatter")
    }

    /// Load merged configuration from global and project configs.
    /// Project config values override global config values.
    pub fn load_merged() -> Result<Self> {
        Self::load_merged_from(
            global_config_path().as_deref(),
            Path::new(".chant/config.md"),
            Some(Path::new(".chant/agents.md")),
        )
    }

    /// Load merged configuration from specified global, project, and agents config paths.
    /// Merge order (later overrides earlier):
    /// 1. Global config
    /// 2. Project config
    /// 3. Agents config (only for parallel.agents section)
    pub fn load_merged_from(
        global_path: Option<&Path>,
        project_path: &Path,
        agents_path: Option<&Path>,
    ) -> Result<Self> {
        // Load global config if it exists
        let global_config = global_path
            .filter(|p| p.exists())
            .map(PartialConfig::load_from)
            .transpose()?
            .unwrap_or_default();

        // Load project config as partial (required, but as partial for merging)
        let project_config = PartialConfig::load_from(project_path)?;

        // Load agents config if it exists (optional, gitignored)
        let agents_config = agents_path
            .filter(|p| p.exists())
            .map(AgentsConfig::load_from)
            .transpose()?;

        // Merge: global < project < agents (for parallel.agents only)
        let mut config = global_config.merge_with(project_config);

        // Apply agents override if present
        if let Some(agents) = agents_config {
            if let Some(parallel) = agents.parallel {
                if !parallel.agents.is_empty() {
                    config.parallel.agents = parallel.agents;
                }
            }
        }

        Ok(config)
    }
}

/// Returns the path to the global config file at ~/.config/chant/config.md
pub fn global_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/chant/config.md"))
}

/// Agents-only config for project-specific agent overrides (.chant/agents.md)
/// This file is gitignored and contains only the parallel.agents section
#[derive(Debug, Deserialize, Default)]
struct AgentsConfig {
    pub parallel: Option<AgentsParallelConfig>,
}

/// Parallel config subset for agents.md - only contains agents list
#[derive(Debug, Deserialize, Default)]
struct AgentsParallelConfig {
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
}

impl AgentsConfig {
    fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read agents config from {}", path.display()))?;

        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<Self> {
        let (frontmatter, _body) = split_frontmatter(content);
        let frontmatter =
            frontmatter.context("Failed to extract frontmatter from agents config")?;

        serde_yaml::from_str(&frontmatter).context("Failed to parse agents config frontmatter")
    }
}

/// Partial config for merging - all fields optional
#[derive(Debug, Deserialize, Default)]
struct PartialConfig {
    pub project: Option<PartialProjectConfig>,
    pub defaults: Option<PartialDefaultsConfig>,
    pub parallel: Option<ParallelConfig>,
    pub repos: Option<Vec<RepoConfig>>,
    pub enterprise: Option<EnterpriseConfig>,
    pub approval: Option<ApprovalConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialProjectConfig {
    pub name: Option<String>,
    pub prefix: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialDefaultsConfig {
    pub prompt: Option<String>,
    pub branch: Option<bool>,
    pub branch_prefix: Option<String>,
    pub model: Option<String>,
    pub split_model: Option<String>,
    pub main_branch: Option<String>,
    pub provider: Option<ProviderType>,
    pub rotation_strategy: Option<String>,
}

impl PartialConfig {
    fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<Self> {
        let (frontmatter, _body) = split_frontmatter(content);
        let frontmatter = frontmatter.context("Failed to extract frontmatter from config")?;

        serde_yaml::from_str(&frontmatter).context("Failed to parse config frontmatter")
    }

    /// Merge this global config with a project config, returning the merged result.
    /// Values from the project config take precedence over global.
    fn merge_with(self, project: PartialConfig) -> Config {
        let global_project = self.project.unwrap_or_default();
        let global_defaults = self.defaults.unwrap_or_default();
        let project_project = project.project.unwrap_or_default();
        let project_defaults = project.defaults.unwrap_or_default();

        Config {
            project: ProjectConfig {
                // Project name is required in project config
                name: project_project.name.unwrap_or_default(),
                // Project prefix overrides global prefix
                prefix: project_project.prefix.or(global_project.prefix),
            },
            defaults: DefaultsConfig {
                // Project value > global value > default
                prompt: project_defaults
                    .prompt
                    .or(global_defaults.prompt)
                    .unwrap_or_else(default_prompt),
                branch: project_defaults
                    .branch
                    .or(global_defaults.branch)
                    .unwrap_or(false),
                branch_prefix: project_defaults
                    .branch_prefix
                    .or(global_defaults.branch_prefix)
                    .unwrap_or_else(default_branch_prefix),
                model: project_defaults.model.or(global_defaults.model),
                split_model: project_defaults.split_model.or(global_defaults.split_model),
                main_branch: project_defaults
                    .main_branch
                    .or(global_defaults.main_branch)
                    .unwrap_or_else(default_main_branch),
                provider: project_defaults
                    .provider
                    .or(global_defaults.provider)
                    .unwrap_or_default(),
                rotation_strategy: project_defaults
                    .rotation_strategy
                    .or(global_defaults.rotation_strategy)
                    .unwrap_or_else(default_rotation_strategy),
            },
            providers: Default::default(),
            // Parallel config: project overrides global, or use default
            parallel: project.parallel.or(self.parallel).unwrap_or_default(),
            // Repos: project overrides global, or use default
            repos: project
                .repos
                .unwrap_or_else(|| self.repos.unwrap_or_default()),
            // Enterprise config: project overrides global, or use default
            enterprise: project.enterprise.or(self.enterprise).unwrap_or_default(),
            // Approval config: project overrides global, or use default
            approval: project.approval.or(self.approval).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
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
  branch: false
---

# Config
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.defaults.prompt, "standard");
        assert!(!config.defaults.branch);
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
  branch: true
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
  branch: false
---
"#,
        )
        .unwrap();

        let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();

        // Project name from project config
        assert_eq!(config.project.name, "my-project");
        // Prefix from global (not set in project)
        assert_eq!(config.project.prefix.as_deref(), Some("global-prefix"));
        // branch=false overrides global branch=true (project explicitly sets it)
        // Actually, our merge logic checks if project.defaults.branch is true
        // Since project has branch: false, we use global's value
        // Wait, that's not right - let me check the logic
        assert!(!config.defaults.branch); // Project sets false
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
    fn test_load_merged_defaults_model() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

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
        // Global model is used when project doesn't specify
        assert_eq!(config.defaults.model, Some("claude-opus-4".to_string()));
    }

    #[test]
    fn test_load_merged_defaults_model_project_overrides() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

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
defaults:
  model: claude-sonnet-4
---
"#,
        )
        .unwrap();

        let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
        // Project model overrides global
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
  cleanup:
    enabled: true
    prompt: custom-cleanup
    auto_run: true
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
        assert!(config.parallel.cleanup.enabled);
        assert_eq!(config.parallel.cleanup.prompt, "custom-cleanup");
        assert!(config.parallel.cleanup.auto_run);
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
        assert!(config.parallel.cleanup.enabled);
        assert_eq!(config.parallel.cleanup.prompt, "parallel-cleanup");
        assert!(!config.parallel.cleanup.auto_run);
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
    fn test_load_merged_enterprise_config() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

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

        // Project enterprise overrides global
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
    fn test_load_merged_approval_config() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

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
        // Global sets dependency, project doesn't override
        assert_eq!(
            config.approval.rejection_action,
            RejectionAction::Dependency
        );
    }

    #[test]
    fn test_load_merged_approval_config_project_overrides() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

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
approval:
  rejection_action: group
---
"#,
        )
        .unwrap();

        let config = Config::load_merged_from(Some(&global_path), &project_path, None).unwrap();
        // Project overrides global
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
            Config::load_merged_from(Some(&global_path), &project_path, Some(&agents_path))
                .unwrap();

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
            Config::load_merged_from(Some(&global_path), &project_path, Some(&agents_path))
                .unwrap();

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
}
