//! Reset functionality - resets failed specs to pending for retry

use anyhow::Result;
use colored::Colorize;

use chant::spec;

use crate::cmd;

/// Reset a failed spec by resetting it to pending status
pub fn cmd_reset(id: &str, work: bool, prompt: Option<&str>, branch: Option<String>) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    let spec_id = spec.id.clone();

    println!("{} Resetting spec {}", "→".cyan(), spec_id.cyan());

    // Use operations module for reset
    let options = chant::operations::reset::ResetOptions {
        re_execute: work,
        prompt: prompt.map(String::from),
        branch,
    };

    chant::operations::reset::reset_spec(&mut spec, &spec_path, options)?;

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
