use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

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
}
