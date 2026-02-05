//! # Bundled Prompt Management
//!
//! This module manages the standard prompts bundled into the Chant binary.
//! All prompts are embedded at compile time using `include_str!` and can be
//! written to the `.chant/prompts/` directory during project initialization.

/// Standard execution prompt - default prompt for spec execution
pub const STANDARD: &str = include_str!("../prompts/standard.md");

/// Split prompt - for splitting driver specs into member specs
pub const SPLIT: &str = include_str!("../prompts/split.md");

/// Verify prompt - for verifying acceptance criteria are met
pub const VERIFY: &str = include_str!("../prompts/verify.md");

/// Merge conflict prompt - for resolving git merge conflicts
pub const MERGE_CONFLICT: &str = include_str!("../prompts/merge-conflict.md");

/// Ollama prompt - optimized prompt for local LLM execution
pub const OLLAMA: &str = include_str!("../prompts/ollama.md");

// Dev-only prompts (not included in distribution)
#[cfg(debug_assertions)]
mod dev {
    /// Bootstrap prompt - minimal prompt that defers to prep command
    pub const BOOTSTRAP: &str = include_str!("../prompts-dev/bootstrap.md");

    /// Documentation prompt - for generating documentation from source code
    pub const DOCUMENTATION: &str = include_str!("../prompts-dev/documentation.md");

    /// Documentation audit prompt - for auditing Rust code against mdbook documentation
    pub const DOC_AUDIT: &str = include_str!("../prompts-dev/doc-audit.md");

    /// Research analysis prompt - for chant-specific research analysis
    pub const RESEARCH_ANALYSIS: &str = include_str!("../prompts-dev/research-analysis.md");

    /// Research synthesis prompt - for chant-specific research synthesis
    pub const RESEARCH_SYNTHESIS: &str = include_str!("../prompts-dev/research-synthesis.md");
}

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
    let prompts = vec![
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
            name: "merge-conflict",
            purpose: "Resolve git merge conflicts during rebase operations",
            content: MERGE_CONFLICT,
        },
        PromptMetadata {
            name: "ollama",
            purpose: "Optimized prompt for local LLM execution",
            content: OLLAMA,
        },
    ];

    // Include dev-only prompts when running in debug mode
    #[cfg(debug_assertions)]
    let prompts = {
        let mut prompts = prompts;
        prompts.extend(vec![
            PromptMetadata {
                name: "bootstrap",
                purpose: "Minimal bootstrap prompt that defers to prep command",
                content: dev::BOOTSTRAP,
            },
            PromptMetadata {
                name: "documentation",
                purpose: "Generate documentation from tracked source files",
                content: dev::DOCUMENTATION,
            },
            PromptMetadata {
                name: "doc-audit",
                purpose: "Audit Rust code against mdbook documentation",
                content: dev::DOC_AUDIT,
            },
            PromptMetadata {
                name: "research-analysis",
                purpose: "Chant-specific research analysis",
                content: dev::RESEARCH_ANALYSIS,
            },
            PromptMetadata {
                name: "research-synthesis",
                purpose: "Chant-specific research synthesis",
                content: dev::RESEARCH_SYNTHESIS,
            },
        ]);
        prompts
    };

    prompts
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
    #[cfg(debug_assertions)]
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
