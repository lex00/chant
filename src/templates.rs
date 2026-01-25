//! Agent configuration templates for AI providers.
//!
//! # Doc Audit
//! - ignore: internal implementation detail

use anyhow::{anyhow, Result};
use std::fmt;

/// Supported AI provider types for agent configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentProvider {
    Claude,
    Cursor,
    AmazonQ,
    Generic,
}

impl AgentProvider {
    /// Get the string identifier for this provider
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::AmazonQ => "amazonq",
            Self::Generic => "generic",
        }
    }

    /// Get the config file name for this provider
    pub fn config_filename(&self) -> &'static str {
        match self {
            Self::Claude => "CLAUDE.md",
            Self::Cursor => ".cursorrules",
            Self::AmazonQ => ".amazonq/rules.md",
            Self::Generic => ".ai-instructions",
        }
    }
}

impl fmt::Display for AgentProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Template metadata and content
pub struct AgentTemplate {
    pub provider: AgentProvider,
    pub content: &'static str,
}

/// Embedded templates for each provider
const CLAUDE_TEMPLATE: &str = include_str!("../templates/agent-claude.md");
const CURSOR_TEMPLATE: &str = include_str!("../templates/agent-cursor.md");
const AMAZONQ_TEMPLATE: &str = include_str!("../templates/agent-amazonq.md");
const GENERIC_TEMPLATE: &str = include_str!("../templates/agent-generic.md");

/// Get a template by provider name (case-insensitive)
pub fn get_template(provider_str: &str) -> Result<AgentTemplate> {
    let provider = match provider_str.to_lowercase().as_str() {
        "claude" => AgentProvider::Claude,
        "cursor" => AgentProvider::Cursor,
        "amazonq" => AgentProvider::AmazonQ,
        "generic" => AgentProvider::Generic,
        _ => {
            return Err(anyhow!(
                "Unknown agent provider '{}'. Valid providers: claude, cursor, amazonq, generic",
                provider_str
            ))
        }
    };
    Ok(get_template_for_provider(provider))
}

/// Get a template for a specific provider
fn get_template_for_provider(provider: AgentProvider) -> AgentTemplate {
    let content = match provider {
        AgentProvider::Claude => CLAUDE_TEMPLATE,
        AgentProvider::Cursor => CURSOR_TEMPLATE,
        AgentProvider::AmazonQ => AMAZONQ_TEMPLATE,
        AgentProvider::Generic => GENERIC_TEMPLATE,
    };
    AgentTemplate { provider, content }
}

/// Parse a list of agent provider specifications into a deduplicated, validated list
///
/// Supports:
/// - Single providers: "claude", "cursor", etc.
/// - Special keyword "all" which expands to all providers
/// - Case-insensitive matching
///
/// Returns a deduplicated sorted list of providers.
pub fn parse_agent_providers(agent_specs: &[String]) -> Result<Vec<AgentProvider>> {
    if agent_specs.is_empty() {
        return Ok(vec![]);
    }

    let mut providers = std::collections::HashSet::new();

    for spec in agent_specs {
        let lower = spec.to_lowercase();

        if lower == "all" {
            // Add all providers
            providers.insert(AgentProvider::Claude);
            providers.insert(AgentProvider::Cursor);
            providers.insert(AgentProvider::AmazonQ);
            providers.insert(AgentProvider::Generic);
        } else {
            // Validate and add single provider
            let template = get_template(&lower)?;
            providers.insert(template.provider);
        }
    }

    // Convert to sorted vec for consistent ordering
    let mut result: Vec<_> = providers.into_iter().collect();
    result.sort_by_key(|p| p.as_str());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_provider_as_str() {
        assert_eq!(AgentProvider::Claude.as_str(), "claude");
        assert_eq!(AgentProvider::Cursor.as_str(), "cursor");
        assert_eq!(AgentProvider::AmazonQ.as_str(), "amazonq");
        assert_eq!(AgentProvider::Generic.as_str(), "generic");
    }

    #[test]
    fn test_agent_provider_config_filename() {
        assert_eq!(AgentProvider::Claude.config_filename(), "CLAUDE.md");
        assert_eq!(AgentProvider::Cursor.config_filename(), ".cursorrules");
        assert_eq!(
            AgentProvider::AmazonQ.config_filename(),
            ".amazonq/rules.md"
        );
        assert_eq!(AgentProvider::Generic.config_filename(), ".ai-instructions");
    }

    #[test]
    fn test_get_template_case_insensitive() {
        assert!(get_template("claude").is_ok());
        assert!(get_template("CLAUDE").is_ok());
        assert!(get_template("Claude").is_ok());
        assert!(get_template("cursor").is_ok());
        assert!(get_template("AMAZONQ").is_ok());
        assert!(get_template("generic").is_ok());
    }

    #[test]
    fn test_get_template_invalid() {
        assert!(get_template("invalid-provider").is_err());
        assert!(get_template("python").is_err());
    }

    #[test]
    fn test_parse_agent_providers_single() {
        let specs = vec!["claude".to_string()];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0], AgentProvider::Claude);
    }

    #[test]
    fn test_parse_agent_providers_multiple() {
        let specs = vec!["claude".to_string(), "cursor".to_string()];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&AgentProvider::Claude));
        assert!(providers.contains(&AgentProvider::Cursor));
    }

    #[test]
    fn test_parse_agent_providers_all() {
        let specs = vec!["all".to_string()];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 4);
        assert!(providers.contains(&AgentProvider::Claude));
        assert!(providers.contains(&AgentProvider::Cursor));
        assert!(providers.contains(&AgentProvider::AmazonQ));
        assert!(providers.contains(&AgentProvider::Generic));
    }

    #[test]
    fn test_parse_agent_providers_deduplication() {
        let specs = vec![
            "claude".to_string(),
            "claude".to_string(),
            "cursor".to_string(),
        ];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 2);
    }

    #[test]
    fn test_parse_agent_providers_case_insensitive() {
        let specs = vec!["CLAUDE".to_string(), "Cursor".to_string()];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&AgentProvider::Claude));
        assert!(providers.contains(&AgentProvider::Cursor));
    }

    #[test]
    fn test_parse_agent_providers_empty() {
        let specs = vec![];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 0);
    }

    #[test]
    fn test_parse_agent_providers_invalid() {
        let specs = vec!["invalid".to_string()];
        assert!(parse_agent_providers(&specs).is_err());
    }

    #[test]
    fn test_parse_agent_providers_all_with_individual() {
        let specs = vec!["all".to_string(), "claude".to_string()];
        let providers = parse_agent_providers(&specs).unwrap();
        assert_eq!(providers.len(), 4); // "all" expands to all 4, claude is already included
    }
}
