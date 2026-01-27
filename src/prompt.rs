//! Prompt template management and variable substitution.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: concepts/prompts.md
//! - ignore: false

use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::config::Config;
use crate::paths::SPECS_DIR;
use crate::spec::{split_frontmatter, Spec};

/// Ask user for confirmation with a yes/no prompt.
/// Returns true if user confirms (y/yes), false if user declines (n/no).
/// Repeats until user provides valid input.
pub fn confirm(message: &str) -> Result<bool> {
    loop {
        print!("{} (y/n): ", message);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                println!("Please enter 'y' or 'n'.");
            }
        }
    }
}

/// Assemble a prompt by substituting template variables.
pub fn assemble(spec: &Spec, prompt_path: &Path, config: &Config) -> Result<String> {
    let prompt_content = fs::read_to_string(prompt_path)
        .with_context(|| format!("Failed to read prompt from {}", prompt_path.display()))?;

    // Extract body (skip frontmatter)
    let (_frontmatter, body) = split_frontmatter(&prompt_content);

    // Check if this is a split prompt (don't inject commit instruction for analysis prompts)
    let is_split_prompt = prompt_path
        .file_stem()
        .map(|s| s.to_string_lossy() == "split")
        .unwrap_or(false);

    // Substitute template variables and inject commit instruction (except for split)
    let message = substitute(body, spec, config, !is_split_prompt);

    Ok(message)
}

fn substitute(template: &str, spec: &Spec, config: &Config, inject_commit: bool) -> String {
    let mut result = template.to_string();

    // Project variables
    result = result.replace("{{project.name}}", &config.project.name);

    // Spec variables
    result = result.replace("{{spec.id}}", &spec.id);
    result = result.replace(
        "{{spec.title}}",
        spec.title.as_deref().unwrap_or("(untitled)"),
    );
    result = result.replace("{{spec.description}}", &spec.body);

    // Spec path (constructed from id)
    let spec_path = format!("{}/{}.md", SPECS_DIR, spec.id);
    result = result.replace("{{spec.path}}", &spec_path);

    // The full spec content
    result = result.replace("{{spec}}", &format_spec_for_prompt(spec));

    // Target files
    if let Some(files) = &spec.frontmatter.target_files {
        result = result.replace("{{spec.target_files}}", &files.join("\n"));
    } else {
        result = result.replace("{{spec.target_files}}", "");
    }

    // Context files - read and include content
    if let Some(context_paths) = &spec.frontmatter.context {
        let mut context_content = String::new();
        for path in context_paths {
            if let Ok(content) = fs::read_to_string(path) {
                context_content.push_str(&format!("\n--- {} ---\n{}\n", path, content));
            }
        }
        result = result.replace("{{spec.context}}", &context_content);
    } else {
        result = result.replace("{{spec.context}}", "");
    }

    // Inject commit instruction if not already present (and if enabled)
    if inject_commit && !result.to_lowercase().contains("commit your work") {
        let commit_instruction = "\n\n## Required: Commit Your Work\n\n\
             When you have completed the work, commit your changes with:\n\n\
             ```\n\
             git commit -m \"chant(";
        result.push_str(commit_instruction);
        result.push_str(&spec.id);
        result.push_str(
            "): <brief description of changes>\"\n\
             ```\n\n\
             This commit message pattern is required for chant to track your work.",
        );
    }

    result
}

fn format_spec_for_prompt(spec: &Spec) -> String {
    let mut output = String::new();

    // ID
    output.push_str(&format!("Spec ID: {}\n\n", spec.id));

    // Title and body
    output.push_str(&spec.body);

    // Target files if any
    if let Some(files) = &spec.frontmatter.target_files {
        output.push_str("\n\n## Target Files\n\n");
        for file in files {
            output.push_str(&format!("- {}\n", file));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::SpecFrontmatter;

    fn make_test_config() -> Config {
        Config {
            project: crate::config::ProjectConfig {
                name: "test-project".to_string(),
                prefix: None,
            },
            defaults: crate::config::DefaultsConfig::default(),
            git: crate::config::GitConfig::default(),
            providers: crate::provider::ProviderConfig::default(),
            parallel: crate::config::ParallelConfig::default(),
        }
    }

    fn make_test_spec() -> Spec {
        Spec {
            id: "2026-01-22-001-x7m".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Fix the bug".to_string()),
            body: "# Fix the bug\n\nDescription here.".to_string(),
        }
    }

    #[test]
    fn test_substitute() {
        let template = "Project: {{project.name}}\nSpec: {{spec.id}}\nTitle: {{spec.title}}";
        let spec = make_test_spec();
        let config = make_test_config();

        let result = substitute(template, &spec, &config, true);

        assert!(result.contains("Project: test-project"));
        assert!(result.contains("Spec: 2026-01-22-001-x7m"));
        assert!(result.contains("Title: Fix the bug"));
    }

    #[test]
    fn test_spec_path_substitution() {
        let template = "Edit {{spec.path}} to check off criteria";
        let spec = make_test_spec();
        let config = make_test_config();

        let result = substitute(template, &spec, &config, true);

        assert!(result.contains(".chant/specs/2026-01-22-001-x7m.md"));
    }

    #[test]
    fn test_split_frontmatter_extracts_body() {
        let content = r#"---
name: test
---

Body content here."#;

        let (_frontmatter, body) = split_frontmatter(content);
        assert_eq!(body, "Body content here.");
    }

    #[test]
    fn test_commit_instruction_is_injected() {
        let template = "# Do some work\n\nThis is a test prompt.";
        let spec = make_test_spec();
        let config = make_test_config();

        let result = substitute(template, &spec, &config, true);

        // Should contain commit instruction
        assert!(result.contains("## Required: Commit Your Work"));
        assert!(result.contains("git commit -m \"chant(2026-01-22-001-x7m):"));
    }

    #[test]
    fn test_commit_instruction_not_duplicated() {
        let template =
            "# Do some work\n\n## Required: Commit Your Work\n\nAlready has instruction.";
        let spec = make_test_spec();
        let config = make_test_config();

        let result = substitute(template, &spec, &config, true);

        // Count occurrences of the section header
        let count = result.matches("## Required: Commit Your Work").count();
        assert_eq!(count, 1, "Commit instruction should not be duplicated");
    }

    #[test]
    fn test_commit_instruction_skipped_when_disabled() {
        let template = "# Analyze something\n\nJust output text.";
        let spec = make_test_spec();
        let config = make_test_config();

        let result = substitute(template, &spec, &config, false);

        // Should NOT contain commit instruction
        assert!(
            !result.contains("## Required: Commit Your Work"),
            "Commit instruction should not be injected when disabled"
        );
    }
}
