use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::config::Config;
use crate::spec::Spec;

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
    let body = extract_body(&prompt_content);

    // Substitute template variables
    let message = substitute(body, spec, config);

    Ok(message)
}

fn extract_body(content: &str) -> &str {
    let content = content.trim();

    if !content.starts_with("---") {
        return content;
    }

    let rest = &content[3..];
    if let Some(end) = rest.find("---") {
        rest[end + 3..].trim_start()
    } else {
        content
    }
}

fn substitute(template: &str, spec: &Spec, config: &Config) -> String {
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

        let result = substitute(template, &spec, &config);

        assert!(result.contains("Project: test-project"));
        assert!(result.contains("Spec: 2026-01-22-001-x7m"));
        assert!(result.contains("Title: Fix the bug"));
    }

    #[test]
    fn test_extract_body() {
        let content = r#"---
name: test
---

Body content here."#;

        let body = extract_body(content);
        assert_eq!(body, "Body content here.");
    }
}
