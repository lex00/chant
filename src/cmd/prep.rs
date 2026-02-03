//! Prep command for retrieving and cleaning spec content for agents.
//!
//! This module implements the `chant prep` subcommand, which loads a specification
//! and outputs its content in a clean format suitable for agents to read. It handles
//! preprocessing of spec content to remove artifacts from previous executions.
//!
//! Key responsibilities:
//! - Loads spec files from the specs directory
//! - Strips agent conversation sections that may have been added during previous runs
//! - Outputs the cleaned spec content to stdout
//! - Preserves section hierarchy while removing level 2 headers tagged as agent output

use anyhow::{Context, Result};
use std::path::Path;

use chant::spec;

/// Strip agent conversation sections from spec body.
/// Removes markdown sections like "## Agent Conversation" or similar that may have been added during previous runs.
/// Only removes level 2 headers (##) and their direct content, preserving level 3+ headers as regular content.
pub fn strip_agent_conversation(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut result = Vec::new();
    let mut skip_section = false;

    for line in lines {
        // Check if this is a level 2 header (## but not ###)
        if line.starts_with("##") && !line.starts_with("###") {
            let header_lower = line.to_lowercase();
            if header_lower.contains("agent conversation")
                || header_lower.contains("agent output")
                || header_lower.contains("execution result")
            {
                skip_section = true;
                continue; // Skip this header line
            } else {
                skip_section = false;
            }
        }

        // Include line if we're not in a skipped section
        if !skip_section {
            result.push(line);
        }
    }

    result.join("\n").trim_end().to_string()
}

/// Output cleaned spec content for the agent to read.
pub fn cmd_prep(spec_id: &str, clean: bool, specs_dir: &Path) -> Result<()> {
    // Load the spec
    let all_specs = spec::load_all_specs(specs_dir)?;
    let spec = all_specs
        .iter()
        .find(|s| s.id == spec_id || s.id.ends_with(spec_id))
        .cloned()
        .with_context(|| format!("Spec not found: {}", spec_id))?;

    // Get the body content
    let body = if clean {
        strip_agent_conversation(&spec.body)
    } else {
        spec.body.clone()
    };

    // Output the spec content
    println!("{}", body);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_agent_conversation() {
        let body = r#"# My Spec

Some content here.

## Agent Conversation

This should be removed.
More agent output.

## Other Section

This should stay."#;

        let result = strip_agent_conversation(body);
        assert!(!result.contains("Agent Conversation"));
        assert!(!result.contains("This should be removed"));
        assert!(result.contains("My Spec"));
        assert!(result.contains("Some content here"));
        assert!(result.contains("Other Section"));
    }

    #[test]
    fn test_strip_execution_result() {
        let body = r#"# Spec Title

Content.

## Execution Result

Agent output here.

## Acceptance Criteria

- [ ] Item"#;

        let result = strip_agent_conversation(body);
        assert!(!result.contains("Execution Result"));
        assert!(!result.contains("Agent output"));
        assert!(result.contains("Spec Title"));
        assert!(result.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_preserve_level_3_headers() {
        let body = r#"# Spec

## Instructions

### Subsection

This is level 3, not a section header.

## Agent Conversation

Should remove."#;

        let result = strip_agent_conversation(body);
        assert!(!result.contains("Agent Conversation"));
        // Level 3 headers should be preserved as they're part of content
        assert!(result.contains("### Subsection"));
        assert!(result.contains("This is level 3"));
    }

    #[test]
    fn test_no_agent_sections() {
        let body = r#"# Spec

## Instructions

Do this work.

## Acceptance Criteria

- [ ] Item"#;

        let result = strip_agent_conversation(body);
        assert_eq!(result, body);
    }
}
