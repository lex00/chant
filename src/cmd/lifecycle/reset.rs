//! Reset functionality - resets failed specs to pending for retry

use anyhow::Result;
use colored::Colorize;

use chant::spec::{self, SpecStatus};

use crate::cmd;

/// Reset a failed spec by resetting it to pending status
pub fn cmd_reset(
    id: &str,
    work: bool,
    prompt: Option<&str>,
    _branch: Option<String>,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    let spec_id = spec.id.clone();

    // Check if spec is in failed or in_progress state
    if spec.frontmatter.status != SpecStatus::Failed
        && spec.frontmatter.status != SpecStatus::InProgress
    {
        anyhow::bail!(
            "Spec {} is not in failed or in_progress state (current status: {:?}). \
             Only failed or in_progress specs can be reset.",
            spec_id,
            spec.frontmatter.status
        );
    }

    println!("{} Resetting spec {}", "→".cyan(), spec_id.cyan());

    // Reset to pending
    spec.frontmatter.status = SpecStatus::Pending;
    spec.save(&spec_path)?;

    println!("{} Spec {} reset to pending", "✓".green(), spec_id);

    // If --work flag specified, execute the spec
    if work {
        println!("{} Re-executing spec...", "→".cyan());

        // Use cmd_work to execute the spec
        cmd::work::cmd_work(
            std::slice::from_ref(&spec_id),
            prompt,
            false, // skip_deps
            false, // skip_criteria
            false, // parallel
            &[],   // label
            false, // finalize
            false, // allow_no_commits
            None,  // max_parallel
            false, // no_cleanup
            false, // cleanup
            false, // skip_approval
            false, // chain
            0,     // chain_max
            false, // no_merge
            false, // no_rebase
            false, // no_watch (allow auto-start)
        )?;
    }

    Ok(())
}
