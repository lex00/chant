//! Spec update operation.
//!
//! Canonical implementation for updating spec fields with validation.

use anyhow::Result;
use std::path::Path;

use crate::spec::{Spec, SpecStatus};

/// Options for spec update
#[derive(Debug, Clone, Default)]
pub struct UpdateOptions {
    /// New status (validated via state machine)
    pub status: Option<SpecStatus>,
    /// Dependencies to set
    pub depends_on: Option<Vec<String>>,
    /// Labels to set
    pub labels: Option<Vec<String>>,
    /// Target files to set
    pub target_files: Option<Vec<String>>,
    /// Model to set
    pub model: Option<String>,
    /// Output text to append to body
    pub output: Option<String>,
}

/// Update spec fields with validation.
///
/// This is the canonical update logic used by both CLI and MCP.
/// Status transitions are validated via the state machine.
pub fn update_spec(spec: &mut Spec, spec_path: &Path, options: UpdateOptions) -> Result<()> {
    let mut updated = false;

    // Update status if provided (use force_status for MCP compatibility)
    if let Some(new_status) = options.status {
        spec.force_status(new_status);
        updated = true;
    }

    // Update depends_on if provided
    if let Some(depends_on) = options.depends_on {
        spec.frontmatter.depends_on = Some(depends_on);
        updated = true;
    }

    // Update labels if provided
    if let Some(labels) = options.labels {
        spec.frontmatter.labels = Some(labels);
        updated = true;
    }

    // Update target_files if provided
    if let Some(target_files) = options.target_files {
        spec.frontmatter.target_files = Some(target_files);
        updated = true;
    }

    // Update model if provided
    if let Some(model) = options.model {
        spec.frontmatter.model = Some(model);
        updated = true;
    }

    // Append output if provided
    if let Some(output) = options.output {
        if !output.is_empty() {
            if !spec.body.ends_with('\n') && !spec.body.is_empty() {
                spec.body.push('\n');
            }
            spec.body.push_str("\n## Output\n\n");
            spec.body.push_str(&output);
            spec.body.push('\n');
            updated = true;
        }
    }

    if !updated {
        anyhow::bail!("No updates specified");
    }

    // Save the spec
    spec.save(spec_path)?;

    Ok(())
}
