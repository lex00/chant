//! Lifecycle command handlers for chant CLI
//!
//! Handles lower-volume but logically related lifecycle operations:
//! - Spec merging and archiving
//! - Spec splitting into member specs
//! - Diagnostic information for spec execution issues
//! - Log file retrieval and display
//! - Drift detection for documentation specs
//! - Reset functionality for failed specs
//!
//! Note: Core spec operations (add, list, show) are in cmd::spec module

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

use chant::config::Config;
use chant::diagnose;
use chant::git;
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, SpecStatus};

// Submodules
pub mod archive;
pub mod drift;
pub mod merge;
pub mod reset;
pub mod split;

// Re-export public command functions
pub use archive::cmd_archive;
pub use drift::cmd_drift;
pub use merge::cmd_merge;
pub use reset::cmd_reset;
pub use split::cmd_split;

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/// Display detailed diagnostic information for a spec
pub fn cmd_diagnose(id: &str) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve spec ID
    let spec = spec::resolve_spec(&specs_dir, id)?;

    // Run diagnostics
    let report = diagnose::diagnose_spec(&spec.id)?;

    // Display report
    println!("\n{}", format!("Spec: {}", report.spec_id).cyan().bold());
    let status_str = match report.status {
        SpecStatus::Pending => "pending".white(),
        SpecStatus::InProgress => "in_progress".yellow(),
        SpecStatus::Paused => "paused".cyan(),
        SpecStatus::Completed => "completed".green(),
        SpecStatus::Failed => "failed".red(),
        SpecStatus::NeedsAttention => "needs_attention".yellow(),
        SpecStatus::Ready => "ready".cyan(),
        SpecStatus::Blocked => "blocked".red(),
        SpecStatus::Cancelled => "cancelled".dimmed(),
    };
    println!("Status: {}", status_str);
    println!("Location: {}", report.location.bright_black());

    // Show branch if in progress (fix C: show branch in diagnose)
    if let Some(ref branch) = spec.frontmatter.branch {
        println!("Branch: {}", branch.bright_black());
    }

    // Show log file mtime if it exists (fix C: show log mtime in diagnose)
    let log_path = PathBuf::from(".chant/logs").join(format!("{}.log", spec.id));
    if log_path.exists() {
        if let Ok(metadata) = std::fs::metadata(&log_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    let secs = elapsed.as_secs();
                    let age_str = if secs < 60 {
                        "just now".to_string()
                    } else if secs < 3600 {
                        format!("{} minutes ago", secs / 60)
                    } else if secs < 86400 {
                        format!("{} hours ago", secs / 3600)
                    } else {
                        format!("{} days ago", secs / 86400)
                    };
                    println!("Log modified: {}", age_str.bright_black());
                }
            }
        }
    }

    println!("\n{}:", "Checks".bold());
    for check in &report.checks {
        let icon = if check.passed {
            "✓".green()
        } else {
            "✗".red()
        };
        print!("  {} {}", icon, check.name);
        if let Some(details) = &check.details {
            println!(" ({})", details.bright_black());
        } else {
            println!();
        }
    }

    println!("\n{}:", "Diagnosis".bold());
    println!("  {}", report.diagnosis);

    if let Some(suggestion) = &report.suggestion {
        println!("\n{}:", "Suggestion".bold());
        println!("  {}", suggestion);
    }

    Ok(())
}

// ============================================================================
// LOGGING
// ============================================================================

/// Show log for a spec (uses default .chant directory)
pub fn cmd_log(id: &str, lines: usize, follow: bool, run: Option<&str>) -> Result<()> {
    cmd_log_at(&PathBuf::from(".chant"), id, lines, follow, run)
}

/// Show log for a spec with custom base path (useful for testing)
pub fn cmd_log_at(
    base_path: &std::path::Path,
    id: &str,
    lines: usize,
    follow: bool,
    run: Option<&str>,
) -> Result<()> {
    let specs_dir = base_path.join("specs");
    let logs_dir = base_path.join("logs");

    // Note: For custom base paths, we check specs_dir directly instead of using ensure_initialized()
    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Resolve spec ID to get the full ID
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if !log_path.exists() {
        println!(
            "{} No log file found for spec '{}'.",
            "⚠".yellow(),
            spec.id.cyan()
        );
        println!("\nLogs are created when a spec is executed with `chant work`.");
        println!("Log path: {}", log_path.display());
        return Ok(());
    }

    // If --run flag is specified, extract that run's content
    if let Some(run_filter) = run {
        if run_filter == "latest" {
            let content = std::fs::read_to_string(&log_path).context("Failed to read log file")?;

            // Find the last run separator or start of file
            let separator = "=".repeat(80);
            let runs: Vec<&str> = content.split(&separator).collect();

            // The latest run is the last segment
            if let Some(latest_run) = runs.last() {
                // Print the latest run content
                print!("{}", latest_run.trim_start_matches('\n'));
                return Ok(());
            } else {
                // No separator found, show entire file
                print!("{}", content);
                return Ok(());
            }
        } else {
            anyhow::bail!(
                "Invalid run filter '{}'. Currently only 'latest' is supported.",
                run_filter
            );
        }
    }

    // Use tail command to show/follow the log
    let mut args = vec!["-n".to_string(), lines.to_string()];

    if follow {
        args.push("-f".to_string());
    }

    args.push(log_path.to_string_lossy().to_string());

    let status = std::process::Command::new("tail")
        .args(&args)
        .status()
        .context("Failed to run tail command")?;

    if !status.success() {
        anyhow::bail!("tail command exited with status: {}", status);
    }

    Ok(())
}

// ============================================================================
// LIFECYCLE OPERATIONS (for watch mode)
// ============================================================================

/// Handle a completed spec: finalize it, then merge if on a branch.
///
/// This orchestrates the completion workflow:
/// 1. Run `chant finalize <spec_id>` to validate and mark complete
/// 2. If spec is on a `chant/<spec-id>` branch, run `chant merge <spec_id>`
/// 3. If spec is on main branch, skip merge step
///
/// # Arguments
/// * `spec_id` - The spec ID to complete
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(_)` if finalize or merge subprocess fails
///
/// # Edge Cases
/// * Finalize fails: Return error, do not proceed to merge
/// * Merge fails: Return error with conflict details
/// * Spec on main branch: Skip merge step
pub fn handle_completed(spec_id: &str) -> Result<()> {
    use std::process::Command;

    let _specs_dir = crate::cmd::ensure_initialized()?;

    // Step 1: Finalize the spec
    println!("{} Finalizing spec {}", "→".cyan(), spec_id.cyan());

    let output = Command::new(std::env::current_exe()?)
        .args(["finalize", spec_id])
        .output()
        .context("Failed to run chant finalize")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "Finalize failed for spec {}\nStdout: {}\nStderr: {}",
            spec_id,
            stdout.trim(),
            stderr.trim()
        );
    }

    println!("{} Finalized spec {}", "✓".green(), spec_id);

    // Step 2: Check if spec is on a branch
    let branch_name = format!("chant/{}", spec_id);
    let on_branch = is_spec_on_branch(spec_id, &branch_name)?;

    if !on_branch {
        println!(
            "{} Spec {} is on main branch, skipping merge",
            "→".cyan(),
            spec_id
        );
        return Ok(());
    }

    // Step 3: Check if branch exists and hasn't been merged already
    let config = Config::load()?;
    let main_branch = chant::merge::load_main_branch(&config);

    if git::branch_exists(&branch_name)? {
        // Check if the branch has already been merged
        if git::is_branch_merged(&branch_name, &main_branch)? {
            println!(
                "{} Branch {} already merged to {}, auto-deleting",
                "→".cyan(),
                branch_name.cyan(),
                main_branch
            );

            // Auto-delete the merged branch
            if let Err(e) = git::delete_branch(&branch_name, false) {
                println!(
                    "{} Warning: Could not delete branch {}: {}",
                    "⚠".yellow(),
                    branch_name,
                    e
                );
            } else {
                println!("{} Deleted merged branch {}", "✓".green(), branch_name);
            }

            return Ok(());
        }
    } else {
        println!(
            "{} Branch {} does not exist, skipping merge",
            "→".cyan(),
            branch_name
        );
        return Ok(());
    }

    // Step 4: Merge the branch
    println!("{} Merging branch {}", "→".cyan(), branch_name.cyan());

    let output = Command::new(std::env::current_exe()?)
        .args(["merge", spec_id])
        .output()
        .context("Failed to run chant merge")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "Merge failed for spec {} (branch: {})\nStdout: {}\nStderr: {}",
            spec_id,
            branch_name,
            stdout.trim(),
            stderr.trim()
        );
    }

    println!("{} Merged spec {}", "✓".green(), spec_id);

    // Step 5: Verify spec status after merge
    // The finalization commit should have been merged to main
    // Reload the spec from main and verify it has status=completed
    let specs_dir = crate::cmd::ensure_initialized()?;
    let spec = spec::resolve_spec(&specs_dir, spec_id)?;

    if spec.frontmatter.status != spec::SpecStatus::Completed {
        // Merge succeeded but spec status wasn't updated - this indicates
        // the finalization commit didn't make it to main
        anyhow::bail!(
            "Merge succeeded but spec {} status is {:?} instead of Completed. \
             This may indicate the finalization commit was not included in the merge.",
            spec_id,
            spec.frontmatter.status
        );
    }

    println!(
        "{} Verified spec {} has status=completed on main",
        "✓".green(),
        spec_id
    );

    Ok(())
}

/// Handle a failed spec: decide whether to retry or mark permanent failure.
///
/// This orchestrates the failure handling workflow:
/// 1. Load spec and retry state
/// 2. Read error log
/// 3. Use retry logic to decide: retry or permanent failure
/// 4. If retry: schedule resume with exponential backoff
/// 5. If permanent: log and mark for manual intervention
///
/// # Arguments
/// * `spec_id` - The spec ID that failed
/// * `config` - Failure configuration with retry settings
///
/// # Returns
/// * `Ok(())` on success (retry scheduled or marked failed)
/// * `Err(_)` on subprocess failure or configuration error
///
/// # Edge Cases
/// * Resume fails: Treat as permanent failure
/// * Empty error log: Permanent failure
/// * Max retries exceeded: Permanent failure
pub fn handle_failed(spec_id: &str, config: &chant::config::FailureConfig) -> Result<()> {
    use chant::retry::{decide_retry, RetryDecision};

    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load the spec
    let mut spec = spec::resolve_spec(&specs_dir, spec_id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    // Get retry state (or create new one)
    let mut retry_state = spec.frontmatter.retry_state.clone().unwrap_or_default();

    // Read error log
    let log_path = specs_dir
        .parent()
        .unwrap_or(&specs_dir)
        .join("logs")
        .join(format!("{}.log", spec_id));

    let error_log = if log_path.exists() {
        std::fs::read_to_string(&log_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Decide whether to retry
    let decision = decide_retry(&retry_state, &error_log, config);

    match decision {
        RetryDecision::Retry(delay) => {
            // Schedule retry with exponential backoff
            let delay_ms = delay.as_millis() as u64;
            retry_state.record_attempt(delay_ms);

            let next_retry_time = retry_state.next_retry_time;

            println!(
                "{} Scheduling retry for spec {} (attempt {}/{}, delay: {}ms)",
                "→".cyan(),
                spec_id,
                retry_state.attempts,
                config.max_retries,
                delay_ms
            );

            // Update spec with new retry state
            spec.frontmatter.retry_state = Some(retry_state);
            spec.save(&spec_path)?;

            // Note: The watch loop will check next_retry_time and call resume when ready
            println!(
                "{} Retry will be attempted at timestamp {}",
                "→".cyan(),
                next_retry_time
            );

            Ok(())
        }
        RetryDecision::PermanentFailure(reason) => {
            // Mark as permanent failure
            println!(
                "{} Permanent failure for spec {}: {}",
                "✗".red(),
                spec_id,
                reason
            );

            // Update spec status to failed (it should already be failed, but ensure it)
            spec.frontmatter.status = chant::spec::SpecStatus::Failed;
            spec.save(&spec_path)?;

            println!(
                "{} Spec {} marked for manual intervention",
                "→".cyan(),
                spec_id
            );

            Ok(())
        }
    }
}

/// Check if a spec's worktree is on the specified branch.
///
/// # Arguments
/// * `spec_id` - The spec ID
/// * `branch_name` - The branch name to check (e.g., "chant/spec-id")
///
/// # Returns
/// * `Ok(true)` if worktree exists and is on the specified branch
/// * `Ok(false)` if worktree doesn't exist or is on a different branch
/// * `Err(_)` if git operations fail
fn is_spec_on_branch(spec_id: &str, branch_name: &str) -> Result<bool> {
    use std::process::Command;

    // Get worktree path
    let worktree_path = chant::worktree::worktree_path_for_spec(spec_id, None);

    // Check if worktree exists
    if !worktree_path.exists() {
        return Ok(false);
    }

    // Get current branch in worktree
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&worktree_path)
        .output()
        .context("Failed to get current branch in worktree")?;

    if !output.status.success() {
        return Ok(false);
    }

    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(current_branch == branch_name)
}

/// Finalize a completed spec by verifying all criteria are checked
pub fn cmd_finalize(id: &str, specs_dir: &std::path::Path) -> Result<()> {
    use crate::cmd::finalize;
    use chant::spec;
    use chant::validation;
    use chant::worktree;

    // Resolve the spec
    let spec = spec::resolve_spec(specs_dir, id)?;
    let spec_id = spec.id.clone();

    // Check if spec is in a valid state for finalization
    // Allow failed too - agents often leave specs in failed state when they actually completed the work
    match spec.frontmatter.status {
        SpecStatus::Completed | SpecStatus::InProgress | SpecStatus::Failed => {
            // These are valid for finalization
        }
        _ => {
            anyhow::bail!(
                "Spec '{}' must be in_progress, completed, or failed to finalize. Current status: {:?}",
                spec_id,
                spec.frontmatter.status
            );
        }
    }

    // Check for unchecked acceptance criteria
    let unchecked = spec.count_unchecked_checkboxes();
    if unchecked > 0 {
        anyhow::bail!(
            "Spec '{}' has {} unchecked acceptance criteria. All criteria must be checked before finalization.",
            spec_id,
            unchecked
        );
    }

    // Load the config for model information and validation settings
    let config = Config::load()?;

    // Validate output against schema if output_schema is defined
    if let Some(ref schema_path_str) = spec.frontmatter.output_schema {
        let schema_path = std::path::Path::new(schema_path_str);
        if schema_path.exists() {
            // Read agent output from log file
            let log_path = specs_dir
                .parent()
                .unwrap_or(specs_dir)
                .join("logs")
                .join(format!("{}.log", spec_id));

            if log_path.exists() {
                let agent_output = std::fs::read_to_string(&log_path)
                    .with_context(|| format!("Failed to read agent log: {}", log_path.display()))?;

                match validation::validate_agent_output(&spec_id, schema_path, &agent_output) {
                    Ok(result) => {
                        if result.is_valid {
                            println!(
                                "{} Output validation passed (schema: {})",
                                "✓".green(),
                                schema_path_str
                            );
                        } else {
                            println!(
                                "{} Output validation failed (schema: {})",
                                "✗".red(),
                                schema_path_str
                            );
                            for error in &result.errors {
                                println!("  - {}", error);
                            }
                            println!("  → Review .chant/logs/{}.log for details", spec_id);

                            // Check if strict validation is enabled
                            if config.validation.strict_output_validation {
                                anyhow::bail!(
                                    "Cannot finalize: output validation failed ({} error(s), strict mode enabled)",
                                    result.errors.len()
                                );
                            } else {
                                println!(
                                    "  {} Proceeding with finalization (strict_output_validation=false)",
                                    "→".cyan()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} Failed to validate output: {}", "⚠".yellow(), e);
                        if config.validation.strict_output_validation {
                            anyhow::bail!(
                                "Cannot finalize: output validation error (strict mode enabled)"
                            );
                        } else {
                            println!(
                                "  {} Proceeding with finalization (strict_output_validation=false)",
                                "→".cyan()
                            );
                        }
                    }
                }
            } else {
                println!(
                    "{} No log file found at {}, skipping output validation",
                    "⚠".yellow(),
                    log_path.display()
                );
            }
        } else {
            println!(
                "{} Output schema file not found: {}, skipping validation",
                "⚠".yellow(),
                schema_path.display()
            );
        }
    }

    // Check if this spec has an active worktree - if so, finalize there
    if let Some(worktree_path) = worktree::get_active_worktree(&spec_id, None) {
        println!(
            "{} Finalizing spec {} in worktree",
            "→".cyan(),
            spec_id.cyan()
        );

        // Get the spec path in the worktree
        let worktree_spec_path = worktree_path
            .join(".chant/specs")
            .join(format!("{}.md", spec_id));

        // Load the spec from the worktree
        let mut worktree_spec =
            spec::Spec::load(&worktree_spec_path).context("Failed to load spec from worktree")?;

        // Get all specs from worktree for validation
        let worktree_specs_dir = worktree_path.join(".chant/specs");
        let all_specs = spec::load_all_specs(&worktree_specs_dir).unwrap_or_default();

        // Create repository for worktree
        let spec_repo = FileSpecRepository::new(worktree_specs_dir.clone());

        // Finalize in worktree
        finalize::finalize_spec(
            &mut worktree_spec,
            &spec_repo,
            &config,
            &all_specs,
            false,
            None,
        )?;

        // Commit the finalization changes in the worktree
        let commit_message = format!("chant({}): finalize spec", spec_id);
        worktree::commit_in_worktree(&worktree_path, &commit_message)?;

        println!(
            "{} Spec {} finalized in worktree and committed",
            "✓".green(),
            spec_id.green()
        );
        if let Some(model) = &worktree_spec.frontmatter.model {
            println!("  {} Model: {}", "•".cyan(), model);
        }
        if let Some(completed_at) = &worktree_spec.frontmatter.completed_at {
            println!("  {} Completed at: {}", "•".cyan(), completed_at);
        }
        if let Some(commits) = &worktree_spec.frontmatter.commits {
            println!(
                "  {} {} commit{}",
                "•".cyan(),
                commits.len(),
                if commits.len() == 1 { "" } else { "s" }
            );
        }
        println!("  {} Worktree: {}", "•".cyan(), worktree_path.display());
    } else {
        // No active worktree - finalize on current branch (main)
        // Create repository for main branch
        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());

        // Perform finalization
        let mut mut_spec = spec.clone();
        finalize::re_finalize_spec(&mut mut_spec, &spec_repo, &config, false)?;

        println!("{} Spec {} finalized", "✓".green(), spec_id.green());
        if let Some(model) = &mut_spec.frontmatter.model {
            println!("  {} Model: {}", "•".cyan(), model);
        }
        if let Some(completed_at) = &mut_spec.frontmatter.completed_at {
            println!("  {} Completed at: {}", "•".cyan(), completed_at);
        }
        if let Some(commits) = &mut_spec.frontmatter.commits {
            println!(
                "  {} {} commit{}",
                "•".cyan(),
                commits.len(),
                if commits.len() == 1 { "" } else { "s" }
            );
        }
    }

    Ok(())
}
