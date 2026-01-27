//! # Bundled Prompt Management
//!
//! This module manages the standard prompts bundled into the Chant binary.
//! All prompts are embedded at compile time using `include_str!` and can be
//! written to the `.chant/prompts/` directory during project initialization.

/// Bootstrap prompt - minimal prompt that defers to prep command
pub const BOOTSTRAP: &str = include_str!("../prompts/bootstrap.md");

/// Standard execution prompt - default prompt for spec execution
pub const STANDARD: &str = include_str!("../prompts/standard.md");

/// Split prompt - for splitting driver specs into member specs
pub const SPLIT: &str = include_str!("../prompts/split.md");

/// Verify prompt - for verifying acceptance criteria are met
pub const VERIFY: &str = include_str!("../prompts/verify.md");

/// Documentation prompt - for generating documentation from source code
pub const DOCUMENTATION: &str = include_str!("../prompts/documentation.md");

/// Research analysis prompt - for analyzing data and extracting findings
pub const RESEARCH_ANALYSIS: &str = include_str!("../prompts/research-analysis.md");

/// Research synthesis prompt - for synthesizing multiple sources
pub const RESEARCH_SYNTHESIS: &str = include_str!("../prompts/research-synthesis.md");

/// Documentation audit prompt - for auditing Rust code against mdbook documentation
pub const DOC_AUDIT: &str = include_str!("../prompts/doc-audit.md");

/// Merge conflict prompt - for resolving git merge conflicts
pub const MERGE_CONFLICT: &str = include_str!("../prompts/merge-conflict.md");

/// Parallel cleanup prompt - for analyzing parallel execution results
pub const PARALLEL_CLEANUP: &str = include_str!("../prompts/parallel-cleanup.md");

/// Ollama prompt - optimized prompt for local LLM execution
pub const OLLAMA: &str = include_str!("../prompts/ollama.md");

/// Metadata about a bundled prompt
#[derive(Debug, Clone)]
pub struct PromptMetadata {
    /// The name of the prompt (used as filename without .md extension)
    pub name: &'static str,
    /// The purpose/description of the prompt
    pub purpose: &'static str,
    /// The content of the prompt
    pub content: &'static str,
}

/// Returns all bundled prompts with their metadata
pub fn all_bundled_prompts() -> Vec<PromptMetadata> {
    vec![
        PromptMetadata {
            name: "bootstrap",
            purpose: "Minimal bootstrap prompt that defers to prep command",
            content: BOOTSTRAP,
        },
        PromptMetadata {
            name: "standard",
            purpose: "Default execution prompt",
            content: STANDARD,
        },
        PromptMetadata {
            name: "split",
            purpose: "Split a driver spec into members with detailed acceptance criteria",
            content: SPLIT,
        },
        PromptMetadata {
            name: "verify",
            purpose: "Verify that acceptance criteria are met",
            content: VERIFY,
        },
        PromptMetadata {
            name: "documentation",
            purpose: "Generate documentation from tracked source files",
            content: DOCUMENTATION,
        },
        PromptMetadata {
            name: "research-analysis",
            purpose: "Analyze data or code and extract structured findings",
            content: RESEARCH_ANALYSIS,
        },
        PromptMetadata {
            name: "research-synthesis",
            purpose: "Synthesize multiple sources into coherent findings and recommendations",
            content: RESEARCH_SYNTHESIS,
        },
        PromptMetadata {
            name: "doc-audit",
            purpose: "Audit Rust code against mdbook documentation",
            content: DOC_AUDIT,
        },
        PromptMetadata {
            name: "merge-conflict",
            purpose: "Resolve git merge conflicts during rebase operations",
            content: MERGE_CONFLICT,
        },
        PromptMetadata {
            name: "parallel-cleanup",
            purpose: "Analyze parallel execution results and help resolve issues",
            content: PARALLEL_CLEANUP,
        },
        PromptMetadata {
            name: "ollama",
            purpose: "Optimized prompt for local LLM execution",
            content: OLLAMA,
        },
    ]
}

/// Get a prompt by name
pub fn get_prompt(name: &str) -> Option<PromptMetadata> {
    all_bundled_prompts().into_iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_bundled_prompts_not_empty() {
        let prompts = all_bundled_prompts();
        assert!(!prompts.is_empty());
    }

    #[test]
    fn test_all_prompts_have_content() {
        let prompts = all_bundled_prompts();
        for prompt in prompts {
            assert!(
                !prompt.content.is_empty(),
                "Prompt {} has no content",
                prompt.name
            );
        }
    }

    #[test]
    fn test_get_prompt_bootstrap() {
        let prompt = get_prompt("bootstrap");
        assert!(prompt.is_some());
        let p = prompt.unwrap();
        assert_eq!(p.name, "bootstrap");
        assert!(p.content.contains("chant prep"));
    }

    #[test]
    fn test_get_prompt_nonexistent() {
        let prompt = get_prompt("nonexistent");
        assert!(prompt.is_none());
    }
}
