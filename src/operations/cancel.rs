//! Cancel operation for specs.
//!
//! Provides the canonical implementation for canceling specs.

use anyhow::Result;
use std::path::Path;

use crate::spec::{Spec, SpecStatus};

/// Options for the cancel operation.
#[derive(Debug, Clone, Default)]
pub struct CancelOptions {
    /// Whether to proceed even if the spec is already cancelled.
    pub force: bool,
}

/// Cancel a spec by setting its status to Cancelled.
///
/// This operation:
/// - Sets the spec status to Cancelled using the state machine
/// - Saves the updated spec to disk
///
/// # Arguments
/// * `specs_dir` - Path to the specs directory
/// * `spec_id` - ID of the spec to cancel
/// * `options` - Cancel operation options
///
/// # Returns
/// * `Ok(())` if the spec was successfully cancelled
/// * `Err(_)` if the spec doesn't exist, is already cancelled (without force), or can't be transitioned
pub fn cancel_spec(specs_dir: &Path, spec_id: &str, options: &CancelOptions) -> Result<Spec> {
    use crate::spec;

    // Resolve and load the spec
    let mut spec = spec::resolve_spec(specs_dir, spec_id)?;

    // Check if already cancelled
    if spec.frontmatter.status == SpecStatus::Cancelled && !options.force {
        anyhow::bail!("Spec '{}' is already cancelled", spec.id);
    }

    // Set status to cancelled using state machine
    spec.set_status(SpecStatus::Cancelled)
        .map_err(|e| anyhow::anyhow!("Failed to transition spec to Cancelled: {}", e))?;

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path)?;

    Ok(spec)
}
