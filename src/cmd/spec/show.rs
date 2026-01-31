//! Spec display functionality
//!
//! Provides the `cmd_show` command function for displaying spec details.

use anyhow::Result;
use atty;
use colored::Colorize;
use std::path::PathBuf;

use chant::config::Config;
use chant::spec;

use crate::render;

// ============================================================================
// DISPLAY HELPERS
// ============================================================================

/// Format a YAML value with semantic colors based on key and value type.
/// - status: green (completed), yellow (in_progress/pending), red (failed)
/// - commit: cyan
/// - type: blue
/// - lists: magenta
/// - bools: green (true), red (false)
pub(crate) fn format_yaml_value(key: &str, value: &serde_yaml::Value) -> String {
    use serde_yaml::Value;

    match value {
        Value::Null => "~".dimmed().to_string(),
        Value::Bool(b) => {
            if *b {
                "true".green().to_string()
            } else {
                "false".red().to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            // Apply semantic coloring based on key
            match key {
                "status" => match s.as_str() {
                    "completed" => s.green().to_string(),
                    "failed" => s.red().to_string(),
                    _ => s.yellow().to_string(), // pending, in_progress
                },
                "commit" => s.cyan().to_string(),
                "type" => s.blue().to_string(),
                _ => s.to_string(),
            }
        }
        Value::Sequence(seq) => {
            let items: Vec<String> = seq
                .iter()
                .map(|v| match v {
                    Value::String(s) => {
                        // Color commits like commit hashes
                        if key == "commits" {
                            s.cyan().to_string()
                        } else {
                            s.magenta().to_string()
                        }
                    }
                    _ => format_yaml_value("", v),
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        Value::Mapping(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        Value::String(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    format!("{}: {}", key_str, format_yaml_value(&key_str, v))
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::Tagged(tagged) => format_yaml_value(key, &tagged.value),
    }
}

/// Convert a snake_case key to Title Case for display.
pub(crate) fn key_to_title_case(key: &str) -> String {
    key.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// SHOW COMMAND
// ============================================================================

pub fn cmd_show(id: &str, show_body: bool, no_render: bool) -> Result<()> {
    let spec = if id.contains(':') {
        // Cross-repo spec ID format: "repo:spec-id"
        let parts: Vec<&str> = id.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid spec ID format. Use 'repo:spec-id' for cross-repo specs");
        }

        let repo_name = parts[0];
        let spec_id = parts[1];

        // Load from global config repos
        let config = Config::load_merged()?;
        if !config.repos.iter().any(|r| r.name == repo_name) {
            anyhow::bail!(
                "Repository '{}' not found in global config. Available repos: {}",
                repo_name,
                config
                    .repos
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        let repo_config = config.repos.iter().find(|r| r.name == repo_name).unwrap();
        let repo_path = shellexpand::tilde(&repo_config.path).to_string();
        let repo_path = PathBuf::from(repo_path);
        let specs_dir = repo_path.join(".chant/specs");

        let mut resolved = spec::resolve_spec(&specs_dir, spec_id)?;
        // Keep the full cross-repo ID format
        resolved.id = format!("{}:{}", repo_name, resolved.id);
        resolved
    } else {
        // Local spec ID
        let specs_dir = crate::cmd::ensure_initialized()?;
        spec::resolve_spec(&specs_dir, id)?
    };

    // Print branch resolution header for in_progress specs
    if spec.frontmatter.status == spec::SpecStatus::InProgress {
        let default_branch = format!("chant/{}", spec.id);
        let display_branch = spec.frontmatter.branch.as_ref().unwrap_or(&default_branch);
        println!(
            "Spec: {} (showing state from branch {})",
            spec.id.cyan(),
            display_branch.dimmed()
        );
        println!();
    }

    // Print ID (not from frontmatter)
    println!("{}: {}", "ID".bold(), spec.id.cyan());

    // Print title if available (extracted from body, not frontmatter)
    if let Some(title) = &spec.title {
        println!("{}: {}", "Title".bold(), title);
    }

    // Get list of derived fields for marking
    let derived_fields = spec
        .frontmatter
        .derived_fields
        .as_deref()
        .unwrap_or_default();

    // Convert frontmatter to YAML value and iterate over fields
    let frontmatter_value = serde_yaml::to_value(&spec.frontmatter)?;
    if let serde_yaml::Value::Mapping(map) = frontmatter_value {
        for (key, value) in map {
            // Skip null values and the derived_fields field itself
            if value.is_null() {
                continue;
            }

            let key_str = match &key {
                serde_yaml::Value::String(s) => s.clone(),
                _ => continue,
            };

            // Skip displaying the derived_fields field itself (internal tracking)
            if key_str == "derived_fields" {
                continue;
            }

            let display_key = key_to_title_case(&key_str);
            let formatted_value = format_yaml_value(&key_str, &value);

            // Add [derived] indicator if this field was auto-derived
            let indicator = if derived_fields.contains(&key_str) {
                " [derived]".dimmed()
            } else {
                "".normal()
            };

            println!("{}{}: {}", display_key.bold(), indicator, formatted_value);
        }
    }

    // Only show body if --body flag is passed
    if show_body {
        println!("\n{}", "--- Body ---".dimmed());

        // Check if we should render markdown
        let should_render =
            !no_render && atty::is(atty::Stream::Stdout) && std::env::var("NO_COLOR").is_err();

        if should_render {
            render::render_markdown(&spec.body);
        } else {
            println!("{}", spec.body);
        }
    }

    Ok(())
}
