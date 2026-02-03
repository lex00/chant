//! Spec deletion and cancellation functionality
//!
//! Provides the `cmd_delete`, `cmd_cancel`, and `cmd_export` command functions.

use anyhow::{Context, Result};
use atty;
use colored::Colorize;
use std::path::PathBuf;

use chant::config::Config;
use chant::git;
use chant::paths::{ARCHIVE_DIR, LOGS_DIR};
use chant::pid;
use chant::spec::{self, SpecStatus};
use chant::worktree;

// ============================================================================
// EXPORT COMMAND (wrapper)
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn cmd_export(
    format: Option<&str>,
    statuses: &[String],
    type_: Option<&str>,
    labels: &[String],
    ready: bool,
    from: Option<&str>,
    to: Option<&str>,
    fields: Option<&str>,
    output: Option<&str>,
) -> Result<()> {
    crate::cmd::export::cmd_export(
        format, statuses, type_, labels, ready, from, to, fields, output,
    )
}

// ============================================================================
// DELETE COMMAND
// ============================================================================

/// Delete a spec permanently, removing the spec file and optionally related artifacts.
///
/// Note: Deletion is allowed regardless of approval status. A spec that requires
/// approval but hasn't been approved can still be deleted - this is intentional
/// since deletion is a cleanup operation that doesn't need approval.
pub fn cmd_delete(
    id: &str,
    force: bool,
    cascade: bool,
    delete_branch: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let logs_dir = PathBuf::from(LOGS_DIR);

    // Load config for branch prefix
    let config = Config::load()?;
    let branch_prefix = &config.defaults.branch_prefix;

    // Load all specs (both active and archived)
    let mut all_specs = spec::load_all_specs(&specs_dir)?;
    let archive_dir = PathBuf::from(ARCHIVE_DIR);
    if archive_dir.exists() {
        let archived_specs = spec::load_all_specs(&archive_dir)?;
        all_specs.extend(archived_specs);
    }

    // Resolve the spec ID
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_id = &spec.id;

    // Check if this is a member spec
    if let Some(driver_id) = spec::extract_driver_id(spec_id) {
        if !cascade {
            anyhow::bail!(
                "Cannot delete member spec '{}' directly. Delete the driver spec '{}' instead, or use --cascade.",
                spec_id,
                driver_id
            );
        }
    }

    // Check if we should collect members for cascade delete
    let members = spec::get_members(spec_id, &all_specs);
    let specs_to_delete: Vec<spec::Spec> = if cascade && !members.is_empty() {
        // Include all members plus the driver
        let mut to_delete: Vec<spec::Spec> = members.iter().map(|s| (*s).clone()).collect();
        to_delete.push(spec.clone());
        to_delete
    } else {
        // Just delete the single spec
        vec![spec.clone()]
    };

    // Check safety constraints
    if !force {
        for spec_to_delete in &specs_to_delete {
            match spec_to_delete.frontmatter.status {
                SpecStatus::InProgress | SpecStatus::Failed | SpecStatus::NeedsAttention => {
                    anyhow::bail!(
                        "Spec '{}' is {}. Use --force to delete anyway.",
                        spec_to_delete.id,
                        match spec_to_delete.frontmatter.status {
                            SpecStatus::InProgress => "in progress",
                            SpecStatus::Failed => "failed",
                            SpecStatus::NeedsAttention => "needs attention",
                            _ => unreachable!(),
                        }
                    );
                }
                _ => {}
            }
        }
    }

    // Check if this spec is a dependency for others
    let mut dependents = Vec::new();
    for other_spec in &all_specs {
        if let Some(deps) = &other_spec.frontmatter.depends_on {
            for dep_id in deps {
                if dep_id == spec_id {
                    dependents.push(other_spec.id.clone());
                }
            }
        }
    }

    if !dependents.is_empty() && !force {
        eprintln!(
            "{} Spec '{}' is a dependency for: {}",
            "⚠".yellow(),
            spec_id,
            dependents.join(", ")
        );
        anyhow::bail!("Use --force to delete this spec and its dependents.");
    }

    // Display what will be deleted
    println!("{} Deleting spec:", "→".cyan());
    for spec_to_delete in &specs_to_delete {
        if spec::extract_driver_id(&spec_to_delete.id).is_some() {
            println!("  {} {} (member)", "→".cyan(), spec_to_delete.id);
        } else if cascade && !members.is_empty() {
            println!(
                "  {} {} (driver with {} member{})",
                "→".cyan(),
                spec_to_delete.id,
                members.len(),
                if members.len() == 1 { "" } else { "s" }
            );
        } else {
            println!("  {} {}", "→".cyan(), spec_to_delete.id);
        }
    }

    // Check for associated artifacts
    let mut artifacts = Vec::new();
    for spec_to_delete in &specs_to_delete {
        let log_path = logs_dir.join(format!("{}.log", spec_to_delete.id));
        if log_path.exists() {
            artifacts.push(format!("log file ({})", log_path.display()));
        }

        let full_spec_path_active = specs_dir.join(format!("{}.md", spec_to_delete.id));
        if full_spec_path_active.exists() {
            artifacts.push(format!("spec file ({})", full_spec_path_active.display()));
        }

        let branch_name = format!("{}{}", branch_prefix, spec_to_delete.id);
        if git::branch_exists(&branch_name).unwrap_or_default() {
            artifacts.push(format!("git branch ({})", branch_name));
        }

        let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_to_delete.id));
        if worktree_path.exists() {
            artifacts.push(format!("worktree ({})", worktree_path.display()));
        }
    }

    if !artifacts.is_empty() {
        println!("{} Artifacts to be removed:", "→".cyan());
        for artifact in &artifacts {
            println!("  {} {}", "→".cyan(), artifact);
        }
    }

    if delete_branch && !members.is_empty() {
        println!("{} (will also delete associated branch)", "→".cyan());
    }

    if dry_run {
        println!("{} {}", "→".cyan(), "(dry run, no changes made)".dimmed());
        return Ok(());
    }

    // Ask for confirmation unless --yes
    if !yes {
        // Detect non-TTY contexts (e.g., when running in worktrees or piped input)
        if !atty::is(atty::Stream::Stdin) {
            eprintln!("ℹ Non-interactive mode detected, proceeding without confirmation");
        } else {
            eprint!(
                "{} Are you sure you want to delete {}? [y/N] ",
                "❓".cyan(),
                spec_id
            );
            std::io::Write::flush(&mut std::io::stderr())?;

            let mut response = String::new();
            std::io::stdin().read_line(&mut response)?;
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("{} Delete cancelled.", "✗".red());
                return Ok(());
            }
        }
    }

    // Perform deletions
    for spec_to_delete in &specs_to_delete {
        // Delete spec file (could be in active or archived)
        let full_spec_path_active = specs_dir.join(format!("{}.md", spec_to_delete.id));
        if full_spec_path_active.exists() {
            std::fs::remove_file(&full_spec_path_active).context("Failed to delete spec file")?;
            println!("  {} {} (deleted)", "✓".green(), spec_to_delete.id);
        }

        // Delete log file if it exists
        let log_path = logs_dir.join(format!("{}.log", spec_to_delete.id));
        if log_path.exists() {
            std::fs::remove_file(&log_path).context("Failed to delete log file")?;
        }

        // Delete worktree if it exists
        let worktree_path = PathBuf::from(format!("/tmp/chant-{}", spec_to_delete.id));
        if worktree_path.exists() {
            worktree::remove_worktree(&worktree_path).context("Failed to clean up worktree")?;
        }
    }

    // Delete branch if requested
    if delete_branch {
        for spec_to_delete in &specs_to_delete {
            let branch_name = format!("{}{}", branch_prefix, spec_to_delete.id);
            if git::branch_exists(&branch_name).unwrap_or_default() {
                git::delete_branch(&branch_name, false).context("Failed to delete branch")?;
            }
        }
    }

    if specs_to_delete.len() == 1 {
        println!("{} Deleted spec: {}", "✓".green(), specs_to_delete[0].id);
    } else {
        println!("{} Deleted {} spec(s)", "✓".green(), specs_to_delete.len());
    }

    Ok(())
}

// ============================================================================
// CANCEL COMMAND
// ============================================================================

/// Cancel a spec (soft-delete) by setting its status to cancelled.
/// Preserves the spec file and git history.
///
/// Note: Cancellation is allowed regardless of approval status. A spec that requires
/// approval but hasn't been approved can still be cancelled - this is intentional
/// since cancelling doesn't need approval (it's just removing the spec from active work).
pub fn cmd_cancel(id: &str, force: bool, dry_run: bool, yes: bool) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec ID
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_id = &spec.id.clone();

    // Check if this is a member spec - cancel is not allowed for members
    if let Some(driver_id) = spec::extract_driver_id(spec_id) {
        anyhow::bail!(
            "Cannot cancel member spec '{}'. Cancel the driver spec '{}' instead.",
            spec_id,
            driver_id
        );
    }

    // Stop any running process
    let pid = pid::read_pid_file(spec_id)?;
    if let Some(pid) = pid {
        if pid::is_process_running(pid) {
            println!("  {} Stopping running process (PID: {})", "•".cyan(), pid);
            pid::stop_process(pid)?;
            pid::remove_pid_file(spec_id)?;
            println!("  {} Process stopped", "✓".green());
        } else {
            pid::remove_pid_file(spec_id)?;
        }
    }

    // Check safety constraints
    if !force {
        match spec.frontmatter.status {
            SpecStatus::Cancelled => {
                anyhow::bail!("Spec '{}' is already cancelled.", spec_id);
            }
            SpecStatus::InProgress | SpecStatus::Failed | SpecStatus::NeedsAttention => {
                anyhow::bail!(
                    "Spec '{}' is {}. Use --force to cancel anyway.",
                    spec_id,
                    match spec.frontmatter.status {
                        SpecStatus::InProgress => "in progress",
                        SpecStatus::Failed => "failed",
                        SpecStatus::NeedsAttention => "needs attention",
                        _ => unreachable!(),
                    }
                );
            }
            _ => {}
        }
    }

    // Check if this spec is a dependency for others
    let all_specs = spec::load_all_specs(&specs_dir)?;
    let mut dependents = Vec::new();
    for other_spec in &all_specs {
        if let Some(deps) = &other_spec.frontmatter.depends_on {
            for dep_id in deps {
                if dep_id == spec_id {
                    dependents.push(other_spec.id.clone());
                }
            }
        }
    }

    if !dependents.is_empty() && !force {
        eprintln!(
            "{} Spec '{}' is a dependency for: {}",
            "⚠".yellow(),
            spec_id,
            dependents.join(", ")
        );
        anyhow::bail!("Use --force to cancel this spec and its dependents.");
    }

    // Display what will be cancelled
    println!("{} Cancelling spec:", "→".cyan());
    println!("  {} {}", "→".cyan(), spec_id);

    if !dependents.is_empty() {
        println!("{} Dependents will be blocked:", "⚠".yellow());
        for dep in &dependents {
            println!("  {} {}", "⚠".yellow(), dep);
        }
    }

    if dry_run {
        println!("{} {}", "→".cyan(), "(dry run, no changes made)".dimmed());
        return Ok(());
    }

    // Ask for confirmation unless --yes
    if !yes {
        // Detect non-TTY contexts (e.g., when running in worktrees or piped input)
        if !atty::is(atty::Stream::Stdin) {
            eprintln!("ℹ Non-interactive mode detected, proceeding without confirmation");
        } else {
            eprint!(
                "{} Are you sure you want to cancel {}? [y/N] ",
                "❓".cyan(),
                spec_id
            );
            std::io::Write::flush(&mut std::io::stderr())?;

            let mut response = String::new();
            std::io::stdin().read_line(&mut response)?;
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("{} Cancel cancelled.", "✗".red());
                return Ok(());
            }
        }
    }

    // Update the spec status to Cancelled
    spec.frontmatter.status = SpecStatus::Cancelled;

    // Save the spec file with the new status
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    spec.save(&spec_path)?;

    println!("{} Cancelled spec: {}", "✓".green(), spec_id);

    Ok(())
}
