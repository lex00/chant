//! Spec editing functionality
//!
//! Provides the `cmd_edit` command function for opening specs in $EDITOR.

use anyhow::{Context, Result};
use std::process::Command;

use chant::spec;

/// Opens a spec file in $EDITOR for editing.
///
/// Resolves the spec ID to its file path and launches the editor specified
/// in the EDITOR environment variable.
pub fn cmd_edit(id: &str) -> Result<()> {
    // Resolve spec ID to file path
    let specs_dir = crate::cmd::ensure_initialized()?;
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Get editor from environment
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        // Fall back to common editors
        if cfg!(target_os = "windows") {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });

    // Launch editor
    let status = Command::new(&editor)
        .arg(&spec_path)
        .status()
        .context(format!("Failed to launch editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    Ok(())
}
