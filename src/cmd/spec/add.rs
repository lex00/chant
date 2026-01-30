//! Spec creation functionality
//!
//! Provides the `cmd_add` command function for creating new specs.

use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

use chant::config::Config;
use chant::derivation::{self, DerivationEngine};
use chant::id;
use chant::spec;

pub fn cmd_add(description: &str, prompt: Option<&str>, needs_approval: bool) -> Result<()> {
    let config = Config::load()?;
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Generate ID
    let id = id::generate_id(&specs_dir)?;
    let filename = format!("{}.md", id);
    let filepath = specs_dir.join(&filename);

    // Create spec content
    let prompt_line = match prompt {
        Some(p) => format!("prompt: {}\n", p),
        None => String::new(),
    };

    let approval_line = if needs_approval {
        "approval:\n  required: true\n  status: pending\n"
    } else {
        ""
    };

    let content = format!(
        r#"---
type: code
status: pending
{}{}---

# {}
"#,
        prompt_line, approval_line, description
    );

    std::fs::write(&filepath, content)?;

    // Parse the spec to add derived fields if enterprise config is present
    if !config.enterprise.derived.is_empty() {
        // Load the spec we just created
        let mut spec = spec::Spec::load(&filepath)?;

        // Build derivation context
        let context = derivation::build_context(&id, &specs_dir);

        // Derive fields using the engine
        let engine = DerivationEngine::new(config.enterprise.clone());
        let derived_fields = engine.derive_fields(&context);

        // Add derived fields to spec frontmatter
        spec.add_derived_fields(derived_fields);

        // Write the spec with derived fields
        spec.save(&filepath)?;
    }

    // Auto-commit the spec file to git (skip if .chant/ is gitignored, e.g. silent mode)
    let output = Command::new("git")
        .args(["add", &filepath.to_string_lossy()])
        .output()
        .context("Failed to run git add for spec file")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If the path is ignored (silent mode), skip git commit silently
        if stderr.contains("ignored") {
            // .chant/ is gitignored (silent mode) - skip git commit
        } else {
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

    println!("{} {}", "Created".green(), id.cyan());
    if needs_approval {
        println!("{} Requires approval before work can begin", "ℹ".cyan());
    }
    println!("Edit: {}", filepath.display());

    Ok(())
}
