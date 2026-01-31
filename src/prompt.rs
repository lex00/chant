//! Prompt template management and variable substitution.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: concepts/prompts.md
//! - ignore: false

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::paths::SPECS_DIR;
use crate::spec::{split_frontmatter, Spec};
use crate::validation;

/// Frontmatter for prompt templates
#[derive(Debug, Deserialize, Default)]
pub struct PromptFrontmatter {
    /// Name of the prompt
    pub name: Option<String>,
    /// Purpose/description of the prompt
    pub purpose: Option<String>,
    /// Parent prompt name to extend from
    pub extends: Option<String>,
}

/// Context about the execution environment (worktree, branch, isolation).
///
/// This information is passed to prompt assembly so agents can be aware
/// of their execution context - whether they're running in an isolated
/// worktree, what branch they're on, etc.
#[derive(Debug, Clone, Default)]
pub struct WorktreeContext {
    /// Path to the worktree directory (e.g., `/tmp/chant-{spec-id}`)
    pub worktree_path: Option<PathBuf>,
    /// Branch name the agent is working on
    pub branch_name: Option<String>,
    /// Whether execution is isolated (in a worktree vs main repo)
    pub is_isolated: bool,
}

/// Ask user for confirmation with a yes/no prompt.
/// Returns true if user confirms (y/yes), false if user declines (n/no).
/// Repeats until user provides valid input.
///
/// In non-interactive (non-TTY) contexts, automatically proceeds without prompting
/// and logs a message indicating confirmation was skipped.
pub fn confirm(message: &str) -> Result<bool> {
    // Detect non-TTY contexts (e.g., when running in worktrees or piped input)
    if !atty::is(atty::Stream::Stdin) {
        eprintln!("ℹ Non-interactive mode detected, proceeding without confirmation");
        return Ok(true);
    }

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
///
/// This version uses default (empty) worktree context. For parallel execution
/// in isolated worktrees, use `assemble_with_context` instead.
pub fn assemble(spec: &Spec, prompt_path: &Path, config: &Config) -> Result<String> {
    assemble_with_context(spec, prompt_path, config, &WorktreeContext::default())
}

/// Assemble a prompt with explicit worktree context.
///
/// Use this when the agent will run in an isolated worktree and should be
/// aware of its execution environment (worktree path, branch, isolation status).
pub fn assemble_with_context(
    spec: &Spec,
    prompt_path: &Path,
    config: &Config,
    worktree_ctx: &WorktreeContext,
) -> Result<String> {
    // Resolve prompt with inheritance
    let mut visited = HashSet::new();
    let resolved_body = resolve_prompt_inheritance(prompt_path, &mut visited)?;

    // Check if this is a split prompt (don't inject commit instruction for analysis prompts)
    let is_split_prompt = prompt_path
        .file_stem()
        .map(|s| s.to_string_lossy() == "split")
        .unwrap_or(false);

    // Substitute template variables and inject commit instruction (except for split)
    let mut message = substitute(&resolved_body, spec, config, !is_split_prompt, worktree_ctx);

    // Append prompt extensions from config
    for extension_name in &config.defaults.prompt_extensions {
        let extension_content = load_extension(extension_name)?;
        message.push_str("\n\n");
        message.push_str(&extension_content);
    }

    Ok(message)
}

/// Resolve prompt inheritance by loading parent prompts recursively.
/// Returns the fully resolved prompt body with {{> parent}} markers replaced.
fn resolve_prompt_inheritance(
    prompt_path: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<String> {
    // Check for circular dependencies
    if visited.contains(prompt_path) {
        anyhow::bail!(
            "Circular prompt inheritance detected: {}",
            prompt_path.display()
        );
    }
    visited.insert(prompt_path.to_path_buf());

    let prompt_content = fs::read_to_string(prompt_path)
        .with_context(|| format!("Failed to read prompt from {}", prompt_path.display()))?;

    // Parse frontmatter
    let (frontmatter_str, body) = split_frontmatter(&prompt_content);

    // Check if this prompt extends another
    if let Some(frontmatter_str) = frontmatter_str {
        let frontmatter: PromptFrontmatter =
            serde_yaml::from_str(&frontmatter_str).with_context(|| {
                format!(
                    "Failed to parse prompt frontmatter from {}",
                    prompt_path.display()
                )
            })?;

        if let Some(parent_name) = frontmatter.extends {
            // Construct parent prompt path
            let prompt_dir = prompt_path.parent().unwrap_or(Path::new(".chant/prompts"));
            let parent_path = prompt_dir.join(format!("{}.md", parent_name));

            // Recursively resolve parent
            let parent_body = resolve_prompt_inheritance(&parent_path, visited)?;

            // Replace {{> parent}} marker with parent content
            let resolved = body.replace("{{> parent}}", &parent_body);
            return Ok(resolved);
        }
    }

    // No parent, return body as-is
    Ok(body.to_string())
}

/// Load a prompt extension from .chant/prompts/extensions/
fn load_extension(extension_name: &str) -> Result<String> {
    let extension_path =
        Path::new(".chant/prompts/extensions").join(format!("{}.md", extension_name));

    let content = fs::read_to_string(&extension_path)
        .with_context(|| format!("Failed to read extension from {}", extension_path.display()))?;

    // Extract body (skip frontmatter if present)
    let (_frontmatter, body) = split_frontmatter(&content);

    Ok(body.to_string())
}

fn substitute(
    template: &str,
    spec: &Spec,
    config: &Config,
    inject_commit: bool,
    worktree_ctx: &WorktreeContext,
) -> String {
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

    // Worktree context variables
    result = result.replace(
        "{{worktree.path}}",
        worktree_ctx
            .worktree_path
            .as_ref()
            .map(|p| p.display().to_string())
            .as_deref()
            .unwrap_or(""),
    );
    result = result.replace(
        "{{worktree.branch}}",
        worktree_ctx.branch_name.as_deref().unwrap_or(""),
    );
    result = result.replace(
        "{{worktree.isolated}}",
        if worktree_ctx.is_isolated {
            "true"
        } else {
            "false"
        },
    );

    // Inject execution environment section if running in a worktree
    // This gives agents awareness of their isolated context
    if worktree_ctx.is_isolated {
        let env_section = format!(
            "\n\n## Execution Environment\n\n\
             You are running in an **isolated worktree**:\n\
             - **Working directory:** `{}`\n\
             - **Branch:** `{}`\n\
             - **Isolation:** Changes are isolated from the main repository until merged\n\n\
             This means your changes will not affect the main branch until explicitly merged.\n",
            worktree_ctx
                .worktree_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            worktree_ctx.branch_name.as_deref().unwrap_or("unknown"),
        );
        result.push_str(&env_section);
    }

    // Inject output schema section if present
    if let Some(ref schema_path) = spec.frontmatter.output_schema {
        let schema_path = Path::new(schema_path);
        if schema_path.exists() {
            match validation::generate_schema_prompt_section(schema_path) {
                Ok(schema_section) => {
                    result.push_str(&schema_section);
                }
                Err(e) => {
                    // Log warning but don't fail prompt assembly
                    eprintln!("Warning: Failed to generate schema prompt section: {}", e);
                }
            }
        } else {
            eprintln!(
                "Warning: Output schema file not found: {}",
                schema_path.display()
            );
        }
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
            providers: crate::provider::ProviderConfig::default(),
            parallel: crate::config::ParallelConfig::default(),
            repos: vec![],
            enterprise: crate::config::EnterpriseConfig::default(),
            approval: crate::config::ApprovalConfig::default(),
            validation: crate::config::OutputValidationConfig::default(),
            site: crate::config::SiteConfig::default(),
            lint: crate::config::LintConfig::default(),
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
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, true, &worktree_ctx);

        assert!(result.contains("Project: test-project"));
        assert!(result.contains("Spec: 2026-01-22-001-x7m"));
        assert!(result.contains("Title: Fix the bug"));
    }

    #[test]
    fn test_spec_path_substitution() {
        let template = "Edit {{spec.path}} to check off criteria";
        let spec = make_test_spec();
        let config = make_test_config();
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, true, &worktree_ctx);

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
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, true, &worktree_ctx);

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
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, true, &worktree_ctx);

        // Count occurrences of the section header
        let count = result.matches("## Required: Commit Your Work").count();
        assert_eq!(count, 1, "Commit instruction should not be duplicated");
    }

    #[test]
    fn test_commit_instruction_skipped_when_disabled() {
        let template = "# Analyze something\n\nJust output text.";
        let spec = make_test_spec();
        let config = make_test_config();
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, false, &worktree_ctx);

        // Should NOT contain commit instruction
        assert!(
            !result.contains("## Required: Commit Your Work"),
            "Commit instruction should not be injected when disabled"
        );
    }

    #[test]
    fn test_worktree_context_substitution() {
        let template =
            "Path: {{worktree.path}}\nBranch: {{worktree.branch}}\nIsolated: {{worktree.isolated}}";
        let spec = make_test_spec();
        let config = make_test_config();
        let worktree_ctx = WorktreeContext {
            worktree_path: Some(PathBuf::from("/tmp/chant-test-spec")),
            branch_name: Some("chant/test-spec".to_string()),
            is_isolated: true,
        };

        let result = substitute(template, &spec, &config, false, &worktree_ctx);

        assert!(result.contains("Path: /tmp/chant-test-spec"));
        assert!(result.contains("Branch: chant/test-spec"));
        assert!(result.contains("Isolated: true"));
    }

    #[test]
    fn test_worktree_context_empty_when_not_isolated() {
        let template = "Path: '{{worktree.path}}'\nBranch: '{{worktree.branch}}'\nIsolated: {{worktree.isolated}}";
        let spec = make_test_spec();
        let config = make_test_config();
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, false, &worktree_ctx);

        assert!(result.contains("Path: ''"));
        assert!(result.contains("Branch: ''"));
        assert!(result.contains("Isolated: false"));
    }

    #[test]
    fn test_execution_environment_section_injected_when_isolated() {
        let template = "# Do some work";
        let spec = make_test_spec();
        let config = make_test_config();
        let worktree_ctx = WorktreeContext {
            worktree_path: Some(PathBuf::from("/tmp/chant-test-spec")),
            branch_name: Some("chant/test-spec".to_string()),
            is_isolated: true,
        };

        let result = substitute(template, &spec, &config, false, &worktree_ctx);

        assert!(result.contains("## Execution Environment"));
        assert!(result.contains("isolated worktree"));
        assert!(result.contains("/tmp/chant-test-spec"));
        assert!(result.contains("chant/test-spec"));
    }

    #[test]
    fn test_execution_environment_section_not_injected_when_not_isolated() {
        let template = "# Do some work";
        let spec = make_test_spec();
        let config = make_test_config();
        let worktree_ctx = WorktreeContext::default();

        let result = substitute(template, &spec, &config, false, &worktree_ctx);

        assert!(!result.contains("## Execution Environment"));
    }

    // =========================================================================
    // PROMPT INHERITANCE TESTS
    // =========================================================================

    #[test]
    fn test_resolve_prompt_no_inheritance() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let prompt_path = tmp.path().join("simple.md");

        fs::write(
            &prompt_path,
            r#"---
name: simple
---

Simple prompt body."#,
        )
        .unwrap();

        let mut visited = HashSet::new();
        let result = resolve_prompt_inheritance(&prompt_path, &mut visited).unwrap();

        assert_eq!(result, "Simple prompt body.");
    }

    #[test]
    fn test_resolve_prompt_with_parent() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let parent_path = tmp.path().join("parent.md");
        let child_path = tmp.path().join("child.md");

        fs::write(
            &parent_path,
            r#"---
name: parent
---

Parent content here."#,
        )
        .unwrap();

        fs::write(
            &child_path,
            r#"---
name: child
extends: parent
---

{{> parent}}

Additional child content."#,
        )
        .unwrap();

        let mut visited = HashSet::new();
        let result = resolve_prompt_inheritance(&child_path, &mut visited).unwrap();

        assert!(result.contains("Parent content here."));
        assert!(result.contains("Additional child content."));
        assert!(!result.contains("{{> parent}}"));
    }

    #[test]
    fn test_circular_inheritance_detection() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let prompt_a = tmp.path().join("a.md");
        let prompt_b = tmp.path().join("b.md");

        fs::write(
            &prompt_a,
            r#"---
name: a
extends: b
---

{{> parent}}"#,
        )
        .unwrap();

        fs::write(
            &prompt_b,
            r#"---
name: b
extends: a
---

{{> parent}}"#,
        )
        .unwrap();

        let mut visited = HashSet::new();
        let result = resolve_prompt_inheritance(&prompt_a, &mut visited);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular prompt inheritance"));
    }

    #[test]
    fn test_load_extension() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let extensions_dir = tmp.path().join(".chant/prompts/extensions");
        fs::create_dir_all(&extensions_dir).unwrap();

        let extension_path = extensions_dir.join("test-ext.md");
        fs::write(
            &extension_path,
            r#"---
name: test-ext
---

Extension content here."#,
        )
        .unwrap();

        // Change to temp directory for test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        let result = load_extension("test-ext").unwrap();
        assert_eq!(result, "Extension content here.");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_prompt_extensions_in_config() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let extensions_dir = tmp.path().join(".chant/prompts/extensions");
        fs::create_dir_all(&extensions_dir).unwrap();

        let extension_path = extensions_dir.join("concise.md");
        fs::write(&extension_path, "Keep output concise.").unwrap();

        let prompt_path = tmp.path().join("main.md");
        fs::write(&prompt_path, "Main prompt.").unwrap();

        let mut config = make_test_config();
        config.defaults.prompt_extensions = vec!["concise".to_string()];

        let spec = make_test_spec();
        let worktree_ctx = WorktreeContext::default();

        // Change to temp directory for test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        let result = assemble_with_context(&spec, &prompt_path, &config, &worktree_ctx).unwrap();

        assert!(result.contains("Main prompt."));
        assert!(result.contains("Keep output concise."));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}
