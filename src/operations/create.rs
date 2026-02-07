//! Spec creation operation.
//!
//! Canonical implementation for creating new specs with derivation and git commit.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::derivation::{self, DerivationEngine};
use crate::id;
use crate::spec::Spec;

/// Options for spec creation
#[derive(Debug, Clone)]
pub struct CreateOptions {
    /// Optional prompt template name
    pub prompt: Option<String>,
    /// Whether spec requires approval before work can begin
    pub needs_approval: bool,
    /// Whether to auto-commit to git (default: true)
    pub auto_commit: bool,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            prompt: None,
            needs_approval: false,
            auto_commit: true,
        }
    }
}

/// Create a new spec with derivation and optional git commit.
///
/// This is the canonical spec creation logic used by both CLI and MCP.
///
/// # Returns
///
/// Returns the created spec and its file path.
pub fn create_spec(
    description: &str,
    specs_dir: &Path,
    config: &Config,
    options: CreateOptions,
) -> Result<(Spec, PathBuf)> {
    // Generate ID
    let id = id::generate_id(specs_dir)?;
    let filename = format!("{}.md", id);
    let filepath = specs_dir.join(&filename);

    // Create spec content
    let prompt_line = match &options.prompt {
        Some(p) => format!("prompt: {}\n", p),
        None => String::new(),
    };

    let approval_line = if options.needs_approval {
        "approval:\n  required: true\n  status: pending\n"
    } else {
        ""
    };

    // Split description if it's longer than ~80 chars
    let (title, body) = if description.len() > 80 {
        // Find first sentence boundary (period followed by space or end, or newline)
        let mut split_pos = None;

        // Check for newline first
        if let Some(newline_pos) = description.find('\n') {
            split_pos = Some(newline_pos);
        } else {
            // Look for period followed by space or end of string
            for (i, c) in description.char_indices() {
                if c == '.' {
                    let next_pos = i + c.len_utf8();
                    if next_pos >= description.len() {
                        // Period at end
                        split_pos = Some(next_pos);
                        break;
                    } else if description[next_pos..].starts_with(' ') {
                        // Period followed by space
                        split_pos = Some(next_pos);
                        break;
                    }
                }
            }
        }

        if let Some(pos) = split_pos {
            let title_part = description[..pos].trim();
            let body_part = description[pos..].trim();
            if !body_part.is_empty() {
                (title_part.to_string(), format!("\n{}", body_part))
            } else {
                (description.to_string(), String::new())
            }
        } else {
            // No sentence boundary found, use whole description as title
            (description.to_string(), String::new())
        }
    } else {
        // Short description, use as-is
        (description.to_string(), String::new())
    };

    let content = format!(
        r#"---
type: code
status: pending
{}{}---

# {}{}
"#,
        prompt_line, approval_line, title, body
    );

    std::fs::write(&filepath, content)?;

    // Parse the spec to add derived fields if enterprise config is present
    if !config.enterprise.derived.is_empty() {
        // Load the spec we just created
        let mut spec = Spec::load(&filepath)?;

        // Build derivation context
        let context = derivation::build_context(&id, specs_dir);

        // Derive fields using the engine
        let engine = DerivationEngine::new(config.enterprise.clone());
        let derived_fields = engine.derive_fields(&context);

        // Add derived fields to spec frontmatter
        spec.add_derived_fields(derived_fields);

        // Write the spec with derived fields
        spec.save(&filepath)?;
    }

    // Auto-commit the spec file to git (skip if .chant/ is gitignored or if disabled)
    if options.auto_commit {
        let output = Command::new("git")
            .args(["add", &filepath.to_string_lossy()])
            .output()
            .context("Failed to run git add for spec file")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If the path is ignored (silent mode), skip git commit silently
            if !stderr.contains("ignored") {
                anyhow::bail!("Failed to stage spec file {}: {}", id, stderr);
            }
        } else {
            let commit_message = format!("chant: Add spec {}", id);
            let output = Command::new("git")
                .args(["commit", "-m", &commit_message])
                .output()
                .context("Failed to run git commit for spec file")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // It's ok if there's nothing to commit (shouldn't happen but be safe)
                if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
                    anyhow::bail!("Failed to commit spec file {}: {}", id, stderr);
                }
            }
        }
    }

    // Load and return the final spec
    let spec = Spec::load(&filepath)?;
    Ok((spec, filepath))
}
