//! Archive operation for specs.
//!
//! Provides the canonical implementation for archiving completed specs.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::paths::ARCHIVE_DIR;
use crate::spec::SpecStatus;

/// Options for the archive operation.
#[derive(Debug, Clone, Default)]
pub struct ArchiveOptions {
    /// Whether to skip git staging (use fs::rename instead of git mv).
    pub no_stage: bool,
    /// Whether to allow archiving non-completed specs.
    pub allow_non_completed: bool,
}

/// Check if we're in a git repository.
fn is_git_repo() -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Move a file using git mv, falling back to fs::rename if not in a git repo or if no_stage is true.
pub fn move_spec_file(src: &PathBuf, dst: &PathBuf, no_stage: bool) -> Result<()> {
    let use_git = !no_stage && is_git_repo();

    if use_git {
        // Try git mv to stage the move
        let status = std::process::Command::new("git")
            .args(["mv", &src.to_string_lossy(), &dst.to_string_lossy()])
            .status()
            .context("Failed to run git mv")?;

        if !status.success() {
            // git mv failed (likely untracked file) - fall back to filesystem rename
            eprintln!(
                "Warning: git mv failed for {} (file may be untracked), using filesystem move",
                src.display()
            );
            std::fs::rename(src, dst).context(format!(
                "Failed to move file from {} to {}",
                src.display(),
                dst.display()
            ))?;
        }
    } else {
        // Fall back to filesystem rename
        std::fs::rename(src, dst).context(format!(
            "Failed to move file from {} to {}",
            src.display(),
            dst.display()
        ))?;
    }

    Ok(())
}

/// Archive a completed spec by moving it to the archive directory.
///
/// This operation:
/// - Verifies the spec is completed
/// - Creates date-based subdirectory in archive (YYYY-MM-DD)
/// - Moves the spec file using git mv (or fs::rename if no_stage is true)
///
/// # Arguments
/// * `specs_dir` - Path to the specs directory
/// * `spec_id` - ID of the spec to archive
/// * `options` - Archive operation options
///
/// # Returns
/// * `Ok(PathBuf)` with the destination path if the spec was successfully archived
/// * `Err(_)` if the spec doesn't exist, is not completed, or can't be moved
pub fn archive_spec(specs_dir: &Path, spec_id: &str, options: &ArchiveOptions) -> Result<PathBuf> {
    use crate::spec;

    // Resolve and load the spec
    let spec = spec::resolve_spec(specs_dir, spec_id)?;

    // Check if completed (unless allow_non_completed is set)
    if spec.frontmatter.status != SpecStatus::Completed && !options.allow_non_completed {
        anyhow::bail!(
            "Spec '{}' must be completed to archive (current: {:?})",
            spec.id,
            spec.frontmatter.status
        );
    }

    let archive_dir = PathBuf::from(ARCHIVE_DIR);

    // Extract date from spec ID (format: YYYY-MM-DD-XXX-abc)
    let date_part = &spec.id[..10]; // First 10 chars: YYYY-MM-DD
    let date_dir = archive_dir.join(date_part);

    // Create date-based subdirectory if it doesn't exist
    if !date_dir.exists() {
        std::fs::create_dir_all(&date_dir)?;
    }

    let source_path = specs_dir.join(format!("{}.md", spec.id));
    let dest_path = date_dir.join(format!("{}.md", spec.id));

    // Move the spec file
    move_spec_file(&source_path, &dest_path, options.no_stage)?;

    Ok(dest_path)
}
