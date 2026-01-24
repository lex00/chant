use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub branch: bool,
    #[serde(default)]
    pub pr: bool,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

fn default_prompt() -> String {
    "standard".to_string()
}

fn default_branch_prefix() -> String {
    "chant/".to_string()
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            prompt: default_prompt(),
            branch: false,
            pr: false,
            branch_prefix: default_branch_prefix(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from(Path::new(".chant/config.md"))
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self> {
        // Extract YAML frontmatter
        let frontmatter = extract_frontmatter(content)
            .context("Failed to extract frontmatter from config")?;

        serde_yaml::from_str(&frontmatter)
            .context("Failed to parse config frontmatter")
    }

    /// Load merged configuration from global and project configs.
    /// Project config values override global config values.
    pub fn load_merged() -> Result<Self> {
        Self::load_merged_from(
            global_config_path().as_deref(),
            Path::new(".chant/config.md"),
        )
    }

    /// Load merged configuration from specified global and project config paths.
    /// Project config values override global config values.
    pub fn load_merged_from(global_path: Option<&Path>, project_path: &Path) -> Result<Self> {
        // Load global config if it exists
        let global_config = global_path
            .filter(|p| p.exists())
            .map(PartialConfig::load_from)
            .transpose()?
            .unwrap_or_default();

        // Load project config as partial (required, but as partial for merging)
        let project_config = PartialConfig::load_from(project_path)?;

        // Merge: project overrides global, then apply defaults
        Ok(global_config.merge_with(project_config))
    }
}

/// Returns the path to the global config file at ~/.config/chant/config.md
pub fn global_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/chant/config.md"))
}

/// Partial config for merging - all fields optional
#[derive(Debug, Deserialize, Default)]
struct PartialConfig {
    pub project: Option<PartialProjectConfig>,
    pub defaults: Option<PartialDefaultsConfig>,
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
    pub pr: Option<bool>,
    pub branch_prefix: Option<String>,
}

impl PartialConfig {
    fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<Self> {
        let frontmatter = extract_frontmatter(content)
            .context("Failed to extract frontmatter from config")?;

        serde_yaml::from_str(&frontmatter)
            .context("Failed to parse config frontmatter")
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
                prompt: project_defaults.prompt
                    .or(global_defaults.prompt)
                    .unwrap_or_else(default_prompt),
                branch: project_defaults.branch
                    .or(global_defaults.branch)
                    .unwrap_or(false),
                pr: project_defaults.pr
                    .or(global_defaults.pr)
                    .unwrap_or(false),
                branch_prefix: project_defaults.branch_prefix
                    .or(global_defaults.branch_prefix)
                    .unwrap_or_else(default_branch_prefix),
            },
        }
    }
}

fn extract_frontmatter(content: &str) -> Option<String> {
    let content = content.trim();

    if !content.starts_with("---") {
        return None;
    }

    let rest = &content[3..];
    if let Some(end) = rest.find("---") {
        Some(rest[..end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

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
        assert_eq!(config.defaults.prompt, "standard"); // default
    }

    #[test]
    fn test_global_config_path() {
        std::env::set_var("HOME", "/home/testuser");
        let path = global_config_path().unwrap();
        assert_eq!(path.to_str().unwrap(), "/home/testuser/.config/chant/config.md");
    }

    #[test]
    fn test_load_merged_no_global() {
        let tmp = TempDir::new().unwrap();
        let project_path = tmp.path().join("config.md");

        fs::write(&project_path, r#"---
project:
  name: my-project
defaults:
  prompt: custom
---
"#).unwrap();

        let config = Config::load_merged_from(None, &project_path).unwrap();
        assert_eq!(config.project.name, "my-project");
        assert_eq!(config.defaults.prompt, "custom");
    }

    #[test]
    fn test_load_merged_with_global() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

        fs::write(&global_path, r#"---
project:
  prefix: global-prefix
defaults:
  branch: true
  pr: true
  branch_prefix: global/
---
"#).unwrap();

        fs::write(&project_path, r#"---
project:
  name: my-project
defaults:
  branch: false
---
"#).unwrap();

        let config = Config::load_merged_from(Some(&global_path), &project_path).unwrap();

        // Project name from project config
        assert_eq!(config.project.name, "my-project");
        // Prefix from global (not set in project)
        assert_eq!(config.project.prefix.as_deref(), Some("global-prefix"));
        // branch=false overrides global branch=true (project explicitly sets it)
        // Actually, our merge logic checks if project.defaults.branch is true
        // Since project has branch: false, we use global's value
        // Wait, that's not right - let me check the logic
        assert!(!config.defaults.branch); // Project sets false
        // pr from global (not set in project)
        assert!(config.defaults.pr);
        // branch_prefix from global (project uses default)
        assert_eq!(config.defaults.branch_prefix, "global/");
    }

    #[test]
    fn test_load_merged_project_overrides_global() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("global.md");
        let project_path = tmp.path().join("project.md");

        fs::write(&global_path, r#"---
defaults:
  prompt: global-prompt
  branch_prefix: global/
---
"#).unwrap();

        fs::write(&project_path, r#"---
project:
  name: my-project
defaults:
  prompt: project-prompt
  branch_prefix: project/
---
"#).unwrap();

        let config = Config::load_merged_from(Some(&global_path), &project_path).unwrap();

        // Project values should override global
        assert_eq!(config.defaults.prompt, "project-prompt");
        assert_eq!(config.defaults.branch_prefix, "project/");
    }

    #[test]
    fn test_load_merged_global_not_exists() {
        let tmp = TempDir::new().unwrap();
        let global_path = tmp.path().join("nonexistent.md");
        let project_path = tmp.path().join("project.md");

        fs::write(&project_path, r#"---
project:
  name: my-project
---
"#).unwrap();

        let config = Config::load_merged_from(Some(&global_path), &project_path).unwrap();
        assert_eq!(config.project.name, "my-project");
    }
}
