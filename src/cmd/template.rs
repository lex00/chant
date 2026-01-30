//! Template command handlers for chant CLI
//!
//! Handles template operations including:
//! - Listing available templates
//! - Showing template details
//! - Creating specs from templates

use anyhow::{Context, Result};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use chant::config::Config;
use chant::derivation::{DerivationContext, DerivationEngine};
use chant::git;
use chant::id;
use chant::spec_template::{find_template, load_all_templates, parse_var_args, TemplateSource};

// ============================================================================
// TEMPLATE COMMANDS
// ============================================================================

/// List all available templates
pub fn cmd_template_list() -> Result<()> {
    let templates = load_all_templates();

    if templates.is_empty() {
        println!("{}", "No templates found.".yellow());
        println!();
        println!("Templates can be stored in:");
        println!("  • {} (project)", ".chant/templates/".cyan());
        if let Some(global_dir) = chant::spec_template::global_templates_dir() {
            println!("  • {} (global)", global_dir.display().to_string().cyan());
        }
        return Ok(());
    }

    println!("{}", "Available templates:".bold());
    println!();

    for template in &templates {
        let source_indicator = match template.source {
            TemplateSource::Project => "(project)".green(),
            TemplateSource::Global => "(global)".blue(),
        };

        print!("  {} ", template.name.cyan().bold());
        print!("{}", source_indicator);
        println!();

        if !template.frontmatter.description.is_empty() {
            println!("    {}", template.frontmatter.description.dimmed());
        }

        // Show variables summary
        let required_count = template.required_variables().len();
        let total_count = template.frontmatter.variables.len();
        if total_count > 0 {
            let var_summary = if required_count > 0 {
                format!("{} variables ({} required)", total_count, required_count)
            } else {
                format!("{} variables (all optional)", total_count)
            };
            println!("    {}", var_summary.dimmed());
        }
        println!();
    }

    Ok(())
}

/// Show details of a specific template
pub fn cmd_template_show(name: &str) -> Result<()> {
    let template = find_template(name)?;

    // Header
    println!("{} {}", "Template:".bold(), template.name.cyan().bold());
    println!(
        "{} {}",
        "Source:".bold(),
        match template.source {
            TemplateSource::Project => "project".green(),
            TemplateSource::Global => "global".blue(),
        }
    );
    println!("{} {}", "Path:".bold(), template.path.display());
    println!();

    if !template.frontmatter.description.is_empty() {
        println!("{}", template.frontmatter.description);
        println!();
    }

    // Variables
    if !template.frontmatter.variables.is_empty() {
        println!("{}", "Variables:".bold());
        for var in &template.frontmatter.variables {
            let required_tag = if var.required && var.default.is_none() {
                " (required)".red().to_string()
            } else {
                String::new()
            };

            print!("  • {}{}", var.name.cyan(), required_tag);
            if let Some(ref default) = var.default {
                print!(" = {}", default.dimmed());
            }
            println!();

            if !var.description.is_empty() {
                println!("    {}", var.description.dimmed());
            }
        }
        println!();
    }

    // Spec defaults
    println!("{}", "Spec Defaults:".bold());
    if let Some(ref spec_type) = template.frontmatter.r#type {
        println!("  type: {}", spec_type);
    }
    if let Some(ref labels) = template.frontmatter.labels {
        if !labels.is_empty() {
            println!("  labels: {}", labels.join(", "));
        }
    }
    if let Some(ref prompt) = template.frontmatter.prompt {
        println!("  prompt: {}", prompt);
    }
    println!();

    // Body preview
    println!("{}", "Template Body:".bold());
    println!("{}", "─".repeat(40).dimmed());
    // Show first 20 lines of body
    let lines: Vec<&str> = template.body.lines().collect();
    let preview_lines = if lines.len() > 20 {
        &lines[..20]
    } else {
        &lines[..]
    };
    for line in preview_lines {
        println!("{}", line);
    }
    if lines.len() > 20 {
        println!(
            "{}",
            format!("... ({} more lines)", lines.len() - 20).dimmed()
        );
    }
    println!("{}", "─".repeat(40).dimmed());

    Ok(())
}

/// Create a spec from a template
pub fn cmd_add_from_template(
    template_name: &str,
    var_args: &[String],
    prompt_override: Option<&str>,
    needs_approval: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let template = find_template(template_name)?;

    // Parse provided variables
    let mut variables = parse_var_args(var_args)?;

    // Check if we need to prompt for missing required variables
    let missing_required: Vec<_> = template
        .required_variables()
        .iter()
        .filter(|v| !variables.contains_key(&v.name))
        .cloned()
        .collect();

    if !missing_required.is_empty() {
        // Check if we're in an interactive terminal
        if atty::is(atty::Stream::Stdin) {
            println!(
                "{} Template '{}' requires the following variables:",
                "ℹ".cyan(),
                template_name
            );
            println!();

            for var in &missing_required {
                let prompt_text = if !var.description.is_empty() {
                    format!("{} ({})", var.name, var.description)
                } else {
                    var.name.clone()
                };

                print!("  {}: ", prompt_text.cyan());
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let value = input.trim().to_string();

                if value.is_empty() {
                    anyhow::bail!("Variable '{}' is required and cannot be empty", var.name);
                }

                variables.insert(var.name.clone(), value);
            }
            println!();
        } else {
            // Non-interactive mode - error out with helpful message
            let var_names: Vec<_> = missing_required.iter().map(|v| v.name.as_str()).collect();
            anyhow::bail!(
                "Missing required variable(s): {}\n\n\
                 Provide them using --var flags:\n  \
                 chant add --template {} {}",
                var_names.join(", "),
                template_name,
                var_names
                    .iter()
                    .map(|n| format!("--var {}=<value>", n))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }

    // Render the spec content
    let mut content = template.render(&variables)?;

    // Add approval if requested
    if needs_approval {
        // Insert approval section into frontmatter
        content = add_approval_to_frontmatter(&content)?;
    }

    // Override prompt if specified
    if let Some(prompt) = prompt_override {
        content = set_prompt_in_frontmatter(&content, prompt)?;
    }

    // Generate ID and write file
    let id = id::generate_id(&specs_dir)?;
    let filename = format!("{}.md", id);
    let filepath = specs_dir.join(&filename);

    std::fs::write(&filepath, &content)?;

    // Apply derived fields if enterprise config is present
    let config = Config::load()?;
    if !config.enterprise.derived.is_empty() {
        let mut spec = chant::spec::Spec::load(&filepath)?;
        let context = build_derivation_context(&id, &specs_dir)?;
        let engine = DerivationEngine::new(config.enterprise.clone());
        let derived_fields = engine.derive_fields(&context);
        spec.add_derived_fields(derived_fields);
        spec.save(&filepath)?;
    }

    // Git commit
    let output = Command::new("git")
        .args(["add", &filepath.to_string_lossy()])
        .output()
        .context("Failed to run git add for spec file")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("ignored") {
            anyhow::bail!("Failed to stage spec file {}: {}", id, stderr);
        }
    } else {
        let commit_message = format!("chant: Add spec {} (from template {})", id, template_name);
        let output = Command::new("git")
            .args(["commit", "-m", &commit_message])
            .output()
            .context("Failed to run git commit for spec file")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
                anyhow::bail!("Failed to commit spec file {}: {}", id, stderr);
            }
        }
    }

    println!(
        "{} {} (from template {})",
        "Created".green(),
        id.cyan(),
        template_name.cyan()
    );
    if needs_approval {
        println!("{} Requires approval before work can begin", "ℹ".cyan());
    }
    println!("Edit: {}", filepath.display());

    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Build derivation context for a newly created spec
fn build_derivation_context(spec_id: &str, specs_dir: &Path) -> Result<DerivationContext> {
    let mut context = chant::derivation::DerivationContext::new();

    if let Ok(branch) = git::get_current_branch() {
        context.branch_name = Some(branch);
    }

    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    context.spec_path = Some(spec_path);
    context.env_vars = std::env::vars().collect();

    // Get git user info
    let (name, email) = git::get_git_user_info();
    context.git_user_name = name;
    context.git_user_email = email;

    Ok(context)
}

/// Add approval section to spec frontmatter
fn add_approval_to_frontmatter(content: &str) -> Result<String> {
    // Find the closing --- of frontmatter
    let content = content.trim_start();
    if !content.starts_with("---") {
        anyhow::bail!("Invalid spec content: missing frontmatter");
    }

    let after_first = &content[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let frontmatter = &after_first[..end_pos];
        let rest = &after_first[end_pos + 4..];

        // Add approval section
        let new_frontmatter = format!(
            "{}{}approval:\n  required: true\n  status: pending\n",
            frontmatter,
            if frontmatter.ends_with('\n') {
                ""
            } else {
                "\n"
            }
        );

        Ok(format!("---\n{}---{}", new_frontmatter, rest))
    } else {
        anyhow::bail!("Invalid spec content: unclosed frontmatter");
    }
}

/// Set or replace prompt in spec frontmatter
fn set_prompt_in_frontmatter(content: &str, prompt: &str) -> Result<String> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        anyhow::bail!("Invalid spec content: missing frontmatter");
    }

    let after_first = &content[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let frontmatter = &after_first[..end_pos];
        let rest = &after_first[end_pos + 4..];

        // Check if prompt already exists
        let new_frontmatter = if frontmatter.contains("\nprompt:") {
            // Replace existing prompt
            let lines: Vec<&str> = frontmatter.lines().collect();
            let updated: Vec<String> = lines
                .iter()
                .map(|line| {
                    if line.starts_with("prompt:") {
                        format!("prompt: {}", prompt)
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            updated.join("\n")
        } else {
            // Add new prompt line
            format!(
                "{}{}prompt: {}",
                frontmatter,
                if frontmatter.ends_with('\n') {
                    ""
                } else {
                    "\n"
                },
                prompt
            )
        };

        Ok(format!("---\n{}\n---{}", new_frontmatter.trim(), rest))
    } else {
        anyhow::bail!("Invalid spec content: unclosed frontmatter");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_approval_to_frontmatter() {
        let content = "---\ntype: code\nstatus: pending\n---\n\n# Title\n";
        let result = add_approval_to_frontmatter(content).unwrap();
        assert!(result.contains("approval:"));
        assert!(result.contains("required: true"));
        assert!(result.contains("status: pending"));
    }

    #[test]
    fn test_set_prompt_in_frontmatter_new() {
        let content = "---\ntype: code\nstatus: pending\n---\n\n# Title\n";
        let result = set_prompt_in_frontmatter(content, "custom").unwrap();
        assert!(result.contains("prompt: custom"));
    }

    #[test]
    fn test_set_prompt_in_frontmatter_replace() {
        let content = "---\ntype: code\nprompt: old\nstatus: pending\n---\n\n# Title\n";
        let result = set_prompt_in_frontmatter(content, "new").unwrap();
        assert!(result.contains("prompt: new"));
        assert!(!result.contains("prompt: old"));
    }
}
