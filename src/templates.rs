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
    Kiro,
    Generic,
}

impl AgentProvider {
    /// Get the string identifier for this provider
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Kiro => "kiro",
            Self::Generic => "generic",
        }
    }

    /// Get the config file name for this provider
    pub fn config_filename(&self) -> &'static str {
        match self {
            Self::Claude => "CLAUDE.md",
            Self::Cursor => ".cursorrules",
            Self::Kiro => ".kiro/rules.md",
            Self::Generic => ".ai-instructions",
        }
    }

    /// Returns true if this provider supports MCP and should generate config.
    /// Used in tests to verify MCP support; production code uses mcp_config_filename() directly.
    #[allow(dead_code)]
    pub fn supports_mcp(&self) -> bool {
        match self {
            Self::Claude => true,
            Self::Cursor => true,
            Self::Kiro => false,
            Self::Generic => false,
        }
    }

    /// Get MCP config filename if provider supports MCP
    pub fn mcp_config_filename(&self) -> Option<&'static str> {
        match self {
            Self::Claude => Some(".mcp.json"),
            Self::Cursor => Some(".cursor/mcp.json"),
            _ => None,
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
const KIRO_TEMPLATE: &str = include_str!("../templates/agent-kiro.md");
const GENERIC_TEMPLATE: &str = include_str!("../templates/agent-generic.md");

/// Compact chant section for injection into existing CLAUDE.md files
const CHANT_SECTION: &str = include_str!("../templates/chant-section.md");
/// Even more compact section when MCP is available (agent discovers commands via tools)
const CHANT_SECTION_MCP: &str = include_str!("../templates/chant-section-mcp.md");

/// Markers for the chant section in CLAUDE.md
pub const CHANT_SECTION_BEGIN: &str = "<!-- chant:begin -->";
pub const CHANT_SECTION_END: &str = "<!-- chant:end -->";

/// Get a template by provider name (case-insensitive)
pub fn get_template(provider_str: &str) -> Result<AgentTemplate> {
    let provider = match provider_str.to_lowercase().as_str() {
        "claude" => AgentProvider::Claude,
        "cursor" => AgentProvider::Cursor,
        "kiro" => AgentProvider::Kiro,
        "generic" => AgentProvider::Generic,
        _ => {
            return Err(anyhow!(
                "Unknown agent provider '{}'. Valid providers: claude, cursor, kiro, generic",
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
        AgentProvider::Kiro => KIRO_TEMPLATE,
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
            providers.insert(AgentProvider::Kiro);
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

/// Get the compact chant section for injection into existing files
///
/// # Arguments
/// * `has_mcp` - If true, returns the even more compact MCP version
pub fn get_chant_section(has_mcp: bool) -> &'static str {
    if has_mcp {
        CHANT_SECTION_MCP
    } else {
        CHANT_SECTION
    }
}

/// Result of injecting the chant section into a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionResult {
    /// Created new file with just the chant section
    Created(String),
    /// Appended section to existing file
    Appended(String),
    /// Replaced existing section between markers
    Replaced(String),
    /// File already has up-to-date section
    Unchanged,
}

/// Inject or update the chant section in an existing CLAUDE.md file
///
/// # Behavior
/// - If `existing_content` is None: Returns just the chant section (for new files)
/// - If content has no markers: Appends chant section at end
/// - If content has markers: Replaces content between markers
///
/// # Arguments
/// * `existing_content` - The current file content, or None if file doesn't exist
/// * `has_mcp` - If true, uses the compact MCP-aware template
pub fn inject_chant_section(existing_content: Option<&str>, has_mcp: bool) -> InjectionResult {
    let section = get_chant_section(has_mcp);

    match existing_content {
        None => {
            // No existing file - create with just the chant section
            InjectionResult::Created(section.to_string())
        }
        Some(content) => {
            // Check if markers exist
            let begin_pos = content.find(CHANT_SECTION_BEGIN);
            let end_pos = content.find(CHANT_SECTION_END);

            match (begin_pos, end_pos) {
                (Some(begin), Some(end)) if begin < end => {
                    // Both markers found in correct order - replace between them
                    let end_marker_end = end + CHANT_SECTION_END.len();

                    // Include trailing newline in the section boundary if present
                    // Handle both Unix (\n) and Windows (\r\n) line endings
                    let section_end = if content[end_marker_end..].starts_with("\r\n") {
                        end_marker_end + 2
                    } else if content[end_marker_end..].starts_with('\n') {
                        end_marker_end + 1
                    } else {
                        end_marker_end
                    };

                    let before = &content[..begin];
                    let after = &content[section_end..];

                    // Normalize line endings for comparison (handle CRLF in both template and content)
                    let existing_section = &content[begin..section_end];
                    let existing_normalized = existing_section.replace("\r\n", "\n");
                    let section_normalized = section.replace("\r\n", "\n");
                    if existing_normalized == section_normalized {
                        return InjectionResult::Unchanged;
                    }

                    let new_content = format!("{}{}{}", before, section, after);
                    InjectionResult::Replaced(new_content)
                }
                _ => {
                    // No markers or invalid order - append at end
                    let new_content = if content.ends_with('\n') {
                        format!("{}\n{}", content, section)
                    } else {
                        format!("{}\n\n{}", content, section)
                    };
                    InjectionResult::Appended(new_content)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_template_case_insensitive() {
        assert!(get_template("claude").is_ok());
        assert!(get_template("CLAUDE").is_ok());
        assert!(get_template("Claude").is_ok());
        assert!(get_template("cursor").is_ok());
        assert!(get_template("KIRO").is_ok());
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
        assert!(providers.contains(&AgentProvider::Kiro));
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

    #[test]
    fn test_get_chant_section_standard() {
        let section = get_chant_section(false);
        assert!(section.starts_with(CHANT_SECTION_BEGIN));
        // Section ends with end marker followed by newline
        assert!(section.trim_end().ends_with(CHANT_SECTION_END));
        assert!(section.contains("orchestrator"));
        assert!(section.contains("chant work"));
    }

    #[test]
    fn test_get_chant_section_mcp() {
        let section = get_chant_section(true);
        assert!(section.starts_with(CHANT_SECTION_BEGIN));
        // Section ends with end marker followed by newline
        assert!(section.trim_end().ends_with(CHANT_SECTION_END));
        assert!(section.contains("orchestrator"));
        assert!(section.contains("MCP")); // MCP-specific content
    }

    #[test]
    fn test_inject_chant_section_no_existing_file() {
        let result = inject_chant_section(None, false);
        match result {
            InjectionResult::Created(content) => {
                assert!(content.starts_with(CHANT_SECTION_BEGIN));
                // Content ends with end marker followed by newline
                assert!(content.trim_end().ends_with(CHANT_SECTION_END));
            }
            _ => panic!("Expected Created result"),
        }
    }

    #[test]
    fn test_inject_chant_section_existing_no_markers() {
        let existing = "# My Project\n\nSome existing content.\n";
        let result = inject_chant_section(Some(existing), false);
        match result {
            InjectionResult::Appended(content) => {
                // Should preserve original content
                assert!(content.starts_with("# My Project"));
                // Should have markers at end
                assert!(content.contains(CHANT_SECTION_BEGIN));
                // Content ends with end marker followed by newline
                assert!(content.trim_end().ends_with(CHANT_SECTION_END));
            }
            _ => panic!("Expected Appended result"),
        }
    }

    #[test]
    fn test_inject_chant_section_existing_with_markers() {
        let existing = format!(
            "# My Project\n\n{}\nOld chant content\n{}\n\n## Other section",
            CHANT_SECTION_BEGIN, CHANT_SECTION_END
        );
        let result = inject_chant_section(Some(&existing), false);
        match result {
            InjectionResult::Replaced(content) => {
                // Should preserve content before and after
                assert!(content.starts_with("# My Project"));
                assert!(content.contains("## Other section"));
                // Should NOT have old content
                assert!(!content.contains("Old chant content"));
                // Should have new markers
                assert!(content.contains(CHANT_SECTION_BEGIN));
                assert!(content.contains(CHANT_SECTION_END));
            }
            _ => panic!("Expected Replaced result"),
        }
    }

    #[test]
    fn test_inject_chant_section_idempotent() {
        // First injection
        let result1 = inject_chant_section(None, false);
        let content1 = match result1 {
            InjectionResult::Created(c) => c,
            _ => panic!("Expected Created"),
        };

        // Second injection on same content
        let result2 = inject_chant_section(Some(&content1), false);
        assert_eq!(result2, InjectionResult::Unchanged);
    }

    #[test]
    fn test_inject_chant_section_preserves_surrounding_content() {
        let existing = "# Header\n\nIntro paragraph.\n\n## Code Style\n\nUse TypeScript.\n";
        let result = inject_chant_section(Some(existing), false);
        match result {
            InjectionResult::Appended(content) => {
                // All original content should be preserved
                assert!(content.contains("# Header"));
                assert!(content.contains("Intro paragraph."));
                assert!(content.contains("## Code Style"));
                assert!(content.contains("Use TypeScript."));
            }
            _ => panic!("Expected Appended result"),
        }
    }

    #[test]
    fn test_inject_chant_section_mcp_variant() {
        let result = inject_chant_section(None, true);
        match result {
            InjectionResult::Created(content) => {
                // MCP version should mention MCP tools
                assert!(content.contains("MCP"));
                assert!(content.contains("chant_"));
            }
            _ => panic!("Expected Created result"),
        }
    }
}
