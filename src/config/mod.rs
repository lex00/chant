//! Configuration management for chant projects.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/config.md
//! - ignore: false

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::provider::ProviderConfig;
use crate::spec::split_frontmatter;

pub mod defaults;
pub mod providers;
pub mod validation;

pub use defaults::*;
pub use providers::*;
pub use validation::*;

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
    #[serde(default)]
    pub validation: OutputValidationConfig,
    #[serde(default)]
    pub site: SiteConfig,
    #[serde(default)]
    pub lint: LintConfig,
    #[serde(default)]
    pub watch: WatchConfig,
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

        let config: Config =
            serde_yaml::from_str(&frontmatter).context("Failed to parse config frontmatter")?;

        // Validate watch config
        config.watch.validate()?;

        Ok(config)
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
    pub validation: Option<OutputValidationConfig>,
    pub site: Option<SiteConfig>,
    pub lint: Option<LintConfig>,
    pub watch: Option<WatchConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialProjectConfig {
    pub name: Option<String>,
    pub prefix: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialDefaultsConfig {
    pub prompt: Option<String>,
    pub branch_prefix: Option<String>,
    pub model: Option<String>,
    pub split_model: Option<String>,
    pub main_branch: Option<String>,
    pub provider: Option<crate::provider::ProviderType>,
    pub rotation_strategy: Option<String>,
    pub prompt_extensions: Option<Vec<String>>,
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
                    .unwrap_or_else(defaults::default_prompt),
                branch_prefix: project_defaults
                    .branch_prefix
                    .or(global_defaults.branch_prefix)
                    .unwrap_or_else(defaults::default_branch_prefix),
                model: project_defaults.model.or(global_defaults.model),
                split_model: project_defaults.split_model.or(global_defaults.split_model),
                main_branch: project_defaults
                    .main_branch
                    .or(global_defaults.main_branch)
                    .unwrap_or_else(defaults::default_main_branch),
                provider: project_defaults
                    .provider
                    .or(global_defaults.provider)
                    .unwrap_or_default(),
                rotation_strategy: project_defaults
                    .rotation_strategy
                    .or(global_defaults.rotation_strategy)
                    .unwrap_or_else(defaults::default_rotation_strategy),
                prompt_extensions: project_defaults
                    .prompt_extensions
                    .or(global_defaults.prompt_extensions)
                    .unwrap_or_default(),
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
            // Validation config: project overrides global, or use default
            validation: project.validation.or(self.validation).unwrap_or_default(),
            // Site config: project overrides global, or use default
            site: project.site.or(self.site).unwrap_or_default(),
            // Lint config: project overrides global, or use default
            lint: project.lint.or(self.lint).unwrap_or_default(),
            // Watch config: project overrides global, or use default
            watch: project.watch.or(self.watch).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests;
