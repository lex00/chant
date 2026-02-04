//! Chain execution mode for specs
//!
//! This module handles sequential execution of specs, processing them one at a time
//! and stopping on the first failure. It supports:
//! - Graceful interruption with Ctrl+C
//! - Filtering by labels
//! - Execution limits
//! - Both specific spec IDs and all ready specs modes

use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use chant::config::Config;
use chant::spec::{self, Spec, SpecStatus};
use chant::spec_group;

use crate::cmd;
use crate::cmd::finalize::{append_agent_output, finalize_spec};
use crate::cmd::git_ops::{commit_transcript, create_or_switch_branch};

// ============================================================================
// CHAIN INTERRUPTION HANDLING
// ============================================================================

static CHAIN_INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set up SIGINT handler for graceful chain interruption
fn setup_chain_signal_handler() {
    CHAIN_INTERRUPTED.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = ctrlc::set_handler(move || {
        if CHAIN_INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst) {
            // Already interrupted once, force exit
            eprintln!("\n{} Force exit", "✗".red());
            std::process::exit(130);
        }
        eprintln!(
            "\n{} Interrupt received - finishing current spec before stopping...",
            "→".yellow()
        );
        eprintln!("  {} Press Ctrl+C again to force exit", "→".dimmed());
        CHAIN_INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst);
    });
}

/// Check if chain execution was interrupted
fn is_chain_interrupted() -> bool {
    CHAIN_INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst)
}

// ============================================================================
// SPEC DISCOVERY AND FILTERING
// ============================================================================

/// Find the next ready spec respecting filters
fn find_next_ready_spec(
    specs_dir: &Path,
    labels: &[String],
    skip_spec_id: Option<&str>,
) -> Result<Option<Spec>> {
    let all_specs = spec::load_all_specs(specs_dir)?;

    // Filter to ready specs
    let mut ready_specs: Vec<Spec> = all_specs
        .iter()
        .filter(|s| {
            // Exclude cancelled specs
            s.frontmatter.status != SpecStatus::Cancelled
                // Must be ready (dependencies satisfied)
                && s.is_ready(&all_specs)
                // Skip the specified spec (if any - used when a specific starting spec was provided)
                && skip_spec_id.is_none_or(|id| s.id != id)
        })
        .cloned()
        .collect();

    // Filter by labels if specified
    if !labels.is_empty() {
        ready_specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    // Sort by spec ID to ensure chronological order (IDs are date-based: YYYY-MM-DD-NNN-xxx)
    ready_specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Return the first (oldest) ready spec
    Ok(ready_specs.into_iter().next())
}

/// Count total ready specs matching filters
fn count_ready_specs(specs_dir: &Path, labels: &[String]) -> Result<usize> {
    let all_specs = spec::load_all_specs(specs_dir)?;

    let mut ready_specs: Vec<&Spec> = all_specs
        .iter()
        .filter(|s| s.frontmatter.status != SpecStatus::Cancelled && s.is_ready(&all_specs))
        .collect();

    if !labels.is_empty() {
        ready_specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    Ok(ready_specs.len())
}

// ============================================================================
// CHAIN EXECUTION
// ============================================================================

/// Execute a single spec in chain mode (simplified version of cmd_work for single spec)
#[allow(clippy::too_many_arguments)]
fn execute_single_spec_in_chain(
    spec_id: &str,
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    prompt_name: Option<&str>,
    cli_branch: Option<&str>,
    skip_deps: bool,
    skip_criteria: bool,
    allow_no_commits: bool,
    skip_approval: bool,
) -> Result<()> {
    // Resolve spec
    let mut spec = spec::resolve_spec(specs_dir, spec_id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Reject cancelled specs
    if spec.frontmatter.status == SpecStatus::Cancelled {
        anyhow::bail!(
            "Cannot work on cancelled spec '{}'. Cancelled specs are not eligible for execution.",
            spec.id
        );
    }

    // Check approval requirements
    if spec.requires_approval() && !skip_approval {
        let approval = spec.frontmatter.approval.as_ref().unwrap();
        if approval.status == spec::ApprovalStatus::Rejected {
            let by_info = approval
                .by
                .as_ref()
                .map(|b| format!(" by {}", b))
                .unwrap_or_default();
            anyhow::bail!(
                "Cannot work on spec '{}' - it has been rejected{}. \
                 Address the feedback and get approval first.",
                spec.id,
                by_info
            );
        } else {
            anyhow::bail!(
                "Spec '{}' requires approval before work can begin. Use --skip-approval to bypass.",
                spec.id
            );
        }
    }

    // Check if already completed
    if spec.frontmatter.status == SpecStatus::Completed && !(skip_deps || skip_criteria) {
        println!(
            "{} Spec {} already completed, skipping.",
            "→".cyan(),
            spec.id
        );
        return Ok(());
    }

    // Check if dependencies are satisfied
    let all_specs = spec::load_all_specs(specs_dir)?;
    if !spec.is_ready(&all_specs) && !skip_deps {
        let blockers = spec.get_blocking_dependencies(&all_specs, specs_dir);
        if !blockers.is_empty() {
            let blocking_ids: Vec<String> = blockers.iter().map(|b| b.spec_id.clone()).collect();
            anyhow::bail!(
                "Spec '{}' is blocked by dependencies: {}",
                spec.id,
                blocking_ids.join(", ")
            );
        }
    }

    // Handle branch creation if requested
    let create_branch = cli_branch.is_some();
    let use_branch_prefix = cli_branch.unwrap_or(&config.defaults.branch_prefix);
    let _branch_name = if create_branch {
        let branch_name = format!("{}{}", use_branch_prefix, spec.id);
        create_or_switch_branch(&branch_name)?;
        spec.frontmatter.branch = Some(branch_name.clone());
        Some(branch_name)
    } else {
        None
    };

    // Resolve prompt
    let resolved_prompt_name = prompt_name
        .map(std::string::ToString::to_string)
        .or_else(|| spec.frontmatter.prompt.clone())
        .or_else(|| super::auto_select_prompt_for_type(&spec, prompts_dir))
        .unwrap_or_else(|| config.defaults.prompt.clone());

    let prompt_path = prompts_dir.join(format!("{}.md", resolved_prompt_name));
    if !prompt_path.exists() {
        anyhow::bail!("Prompt not found: {}", resolved_prompt_name);
    }

    // Update status to in_progress
    spec.frontmatter.status = SpecStatus::InProgress;
    spec.save(&spec_path)?;
    eprintln!("{} [chain] Set {} to InProgress", "→".cyan(), spec.id);

    // Don't mark driver as in_progress in chain mode to keep only 1 spec in_progress at a time
    // The driver will be auto-completed when all members finish
    spec::mark_driver_in_progress_conditional(specs_dir, &spec.id, true)?;

    // Assemble prompt
    let message = chant::prompt::assemble(&spec, &prompt_path, config)?;

    // Select agent for execution
    let agent_command =
        if config.defaults.rotation_strategy != "none" && !config.parallel.agents.is_empty() {
            match cmd::agent_rotation::select_agent_for_work(
                &config.defaults.rotation_strategy,
                &config.parallel,
            ) {
                Ok(cmd) => Some(cmd),
                Err(e) => {
                    println!("{} Failed to select agent: {}", "⚠".yellow(), e);
                    None
                }
            }
        } else {
            None
        };

    // Invoke agent
    let result = if let Some(agent_cmd) = agent_command {
        cmd::agent::invoke_agent_with_command_override(
            &message,
            &spec,
            &resolved_prompt_name,
            config,
            Some(&agent_cmd),
        )
    } else {
        cmd::agent::invoke_agent(&message, &spec, &resolved_prompt_name, config)
    };

    match result {
        Ok(agent_output) => {
            // Reload spec (it may have been modified by the agent)
            let mut spec = spec::resolve_spec(specs_dir, &spec.id)?;

            // Check for commits
            let found_commits = match if allow_no_commits {
                cmd::commits::get_commits_for_spec_allow_no_commits(&spec.id)
            } else {
                cmd::commits::get_commits_for_spec(&spec.id)
            } {
                Ok(commits) => {
                    if commits.is_empty() {
                        spec.frontmatter.status = SpecStatus::Failed;
                        spec.save(&spec_path)?;
                        anyhow::bail!("No commits found - agent did not make any changes");
                    }
                    commits
                }
                Err(e) => {
                    if allow_no_commits {
                        vec![]
                    } else {
                        spec.frontmatter.status = SpecStatus::Failed;
                        spec.save(&spec_path)?;
                        return Err(e);
                    }
                }
            };

            // Check acceptance criteria
            let unchecked_count = spec.count_unchecked_checkboxes();
            if unchecked_count > 0 && !skip_criteria {
                spec.frontmatter.status = SpecStatus::Failed;
                spec.save(&spec_path)?;
                anyhow::bail!("Spec has {} unchecked acceptance criteria", unchecked_count);
            }

            // Auto-finalize the spec
            let all_specs = spec::load_all_specs(specs_dir)?;
            let commits_to_pass = if found_commits.is_empty() {
                None
            } else {
                Some(found_commits)
            };
            finalize_spec(
                &mut spec,
                &spec_path,
                config,
                &all_specs,
                allow_no_commits,
                commits_to_pass,
            )?;

            // Check if driver should be auto-completed
            let all_specs = spec::load_all_specs(specs_dir)?;
            if spec::auto_complete_driver_if_ready(&spec.id, &all_specs, specs_dir)? {
                println!(
                    "  {} Auto-completed driver spec: {}",
                    "✓".green(),
                    spec::extract_driver_id(&spec.id).unwrap()
                );
            }

            // Append agent output to spec body
            append_agent_output(&mut spec, &agent_output);
            spec.save(&spec_path)?;

            // Create transcript commit
            commit_transcript(&spec.id, &spec_path)?;

            Ok(())
        }
        Err(e) => {
            // Update spec to failed
            let mut spec = spec::resolve_spec(specs_dir, &spec.id)?;
            spec.frontmatter.status = SpecStatus::Failed;
            spec.save(&spec_path)?;
            Err(e)
        }
    }
}

/// Chain execution mode: loop through ready specs until none remain or failure
pub fn cmd_work_chain(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: ChainOptions,
) -> Result<()> {
    // Set up signal handler for graceful interruption
    setup_chain_signal_handler();

    // If specific IDs are provided, chain through ONLY those specs in order
    if !options.specific_ids.is_empty() {
        return cmd_work_chain_specific_ids(specs_dir, prompts_dir, config, &options);
    }

    // No specific IDs - chain through all ready specs (existing behavior)
    cmd_work_chain_all_ready(specs_dir, prompts_dir, config, &options)
}

/// Chain through specific spec IDs in order
fn cmd_work_chain_specific_ids(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: &ChainOptions,
) -> Result<()> {
    use std::time::Instant;

    // Validate all IDs upfront and fail fast if any are invalid
    let mut resolved_specs = Vec::new();
    for spec_id in options.specific_ids {
        match spec::resolve_spec(specs_dir, spec_id) {
            Ok(spec) => resolved_specs.push(spec),
            Err(e) => {
                anyhow::bail!("Invalid spec ID '{}': {}", spec_id, e);
            }
        }
    }

    let total = resolved_specs.len();
    println!(
        "\n{} Starting chain execution ({} specified specs)...\n",
        "→".cyan(),
        total
    );

    // Create progress bar
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Note: --label filter is ignored when specific IDs are provided
    if !options.labels.is_empty() {
        println!(
            "{} Note: --label filter ignored when specific spec IDs are provided\n",
            "→".dimmed()
        );
    }

    let mut all_specs = spec::load_all_specs(specs_dir)?;
    let mut completed = 0;
    let mut skipped = 0;
    let mut failed_spec: Option<(String, String)> = None;
    let start_time = Instant::now();

    for spec in resolved_specs.iter() {
        // Check for interrupt
        if is_chain_interrupted() {
            println!("\n{} Chain interrupted by user", "→".yellow());
            break;
        }

        // Check max limit
        if options.max_specs > 0 && completed >= options.max_specs {
            println!(
                "\n{} Reached maximum chain limit ({})",
                "✓".green(),
                options.max_specs
            );
            break;
        }

        // Get fresh spec state from all_specs
        let current_spec = all_specs
            .iter()
            .find(|s| s.id == spec.id)
            .cloned()
            .unwrap_or_else(|| spec.clone());

        // Check if spec is ready
        if !current_spec.is_ready(&all_specs) && !options.skip_deps {
            println!(
                "{} Skipping {}: not ready (dependencies not satisfied)",
                "⚠".yellow(),
                current_spec.id
            );
            skipped += 1;
            continue;
        }

        // Check if spec is already completed
        if current_spec.frontmatter.status == SpecStatus::Completed
            && !(options.skip_deps || options.skip_criteria)
        {
            println!(
                "{} Skipping {}: already completed",
                "⚠".yellow(),
                current_spec.id
            );
            skipped += 1;
            continue;
        }

        // Check if spec is cancelled
        if current_spec.frontmatter.status == SpecStatus::Cancelled {
            println!("{} Skipping {}: cancelled", "⚠".yellow(), current_spec.id);
            skipped += 1;
            continue;
        }

        pb.set_message(format!(
            "{}: {}",
            current_spec.id,
            current_spec.title.as_deref().unwrap_or("")
        ));

        let spec_start = Instant::now();
        match execute_single_spec_in_chain(
            &spec.id,
            specs_dir,
            prompts_dir,
            config,
            options.prompt_name,
            options.cli_branch,
            options.skip_deps,
            options.skip_criteria,
            options.allow_no_commits,
            options.skip_approval,
        ) {
            Ok(()) => {
                let elapsed = spec_start.elapsed();
                pb.inc(1);
                pb.println(format!(
                    "{} Completed {} in {:.1}s",
                    "✓".green(),
                    spec.id,
                    elapsed.as_secs_f64()
                ));
                completed += 1;
                // Reload all specs to get fresh dependency state for next iteration
                all_specs = spec::load_all_specs(specs_dir)?;
            }
            Err(e) => {
                pb.println(format!("{} Failed {}: {}", "✗".red(), spec.id, e));
                failed_spec = Some((spec.id.clone(), e.to_string()));
                break; // Stop chain on first failure
            }
        }
    }

    // Finish progress bar
    pb.finish_and_clear();

    // Print summary
    let total_elapsed = start_time.elapsed();
    println!("{}", "═".repeat(60).dimmed());
    println!("{}", "Chain execution complete:".bold());
    println!(
        "  {} Chained through {} spec(s) in {:.1}s",
        "✓".green(),
        completed,
        total_elapsed.as_secs_f64()
    );

    if skipped > 0 {
        println!("  {} Skipped {} spec(s)", "→".yellow(), skipped);
    }

    if let Some((spec_id, error)) = &failed_spec {
        println!("  {} Stopped due to failure in {}", "✗".red(), spec_id);
        println!("    Error: {}", error);
        println!("{}", "═".repeat(60).dimmed());
        // Exit with error code
        std::process::exit(1);
    }

    if is_chain_interrupted() {
        println!("  {} Interrupted by user", "→".yellow());
    }

    println!("{}", "═".repeat(60).dimmed());

    Ok(())
}

/// Chain through all ready specs (original behavior when no specific IDs provided)
fn cmd_work_chain_all_ready(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: &ChainOptions,
) -> Result<()> {
    use std::time::Instant;

    // Count total ready specs for progress display
    let initial_total = count_ready_specs(specs_dir, options.labels)?;

    if initial_total == 0 {
        if !options.labels.is_empty() {
            println!("No ready specs with specified labels.");
        } else {
            println!("No ready specs to execute.");
        }
        return Ok(());
    }

    println!(
        "\n{} Starting chain execution ({} ready specs)...\n",
        "→".cyan(),
        initial_total
    );

    // Create progress bar (indeterminate since total may change)
    let pb = ProgressBar::new(initial_total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut completed = 0;
    let mut failed_spec: Option<(String, String)> = None;
    let start_time = Instant::now();

    // Chain loop: continue until interrupted, max reached, no more specs, or failure
    while failed_spec.is_none() {
        // Check for interrupt
        if is_chain_interrupted() {
            println!("\n{} Chain interrupted by user", "→".yellow());
            break;
        }

        // Check max limit
        if options.max_specs > 0 && completed >= options.max_specs {
            println!(
                "\n{} Reached maximum chain limit ({})",
                "✓".green(),
                options.max_specs
            );
            break;
        }

        // Debug: Log current in_progress specs before selecting next
        let all_specs_debug = spec::load_all_specs(specs_dir)?;
        let in_progress_count = all_specs_debug
            .iter()
            .filter(|s| s.frontmatter.status == SpecStatus::InProgress)
            .count();
        if in_progress_count > 0 {
            eprintln!(
                "{} [chain] Currently {} spec(s) in_progress before selecting next",
                "→".dimmed(),
                in_progress_count
            );
            for s in all_specs_debug
                .iter()
                .filter(|s| s.frontmatter.status == SpecStatus::InProgress)
            {
                eprintln!("  - {}", s.id);
            }
        }

        // Find next ready spec
        let next_spec = find_next_ready_spec(specs_dir, options.labels, None)?;

        let spec = match next_spec {
            Some(s) => s,
            None => {
                println!("\n{} No more ready specs", "✓".green());
                break;
            }
        };

        // Get current count for progress display
        let current_total = count_ready_specs(specs_dir, options.labels)?;
        let display_total = initial_total.max(completed + current_total);

        // Update progress bar
        pb.set_length(display_total as u64);
        pb.set_message(format!(
            "{}: {}",
            spec.id,
            spec.title.as_deref().unwrap_or("")
        ));

        let spec_start = Instant::now();
        match execute_single_spec_in_chain(
            &spec.id,
            specs_dir,
            prompts_dir,
            config,
            options.prompt_name,
            options.cli_branch,
            options.skip_deps,
            options.skip_criteria,
            options.allow_no_commits,
            options.skip_approval,
        ) {
            Ok(()) => {
                let elapsed = spec_start.elapsed();
                pb.inc(1);
                pb.println(format!(
                    "{} Completed {} in {:.1}s",
                    "✓".green(),
                    spec.id,
                    elapsed.as_secs_f64()
                ));
                completed += 1;
            }
            Err(e) => {
                pb.println(format!("{} Failed {}: {}", "✗".red(), spec.id, e));
                failed_spec = Some((spec.id, e.to_string()));
            }
        }
    }

    // Finish progress bar
    pb.finish_and_clear();

    // Print summary
    let total_elapsed = start_time.elapsed();
    println!("{}", "═".repeat(60).dimmed());
    println!("{}", "Chain execution complete:".bold());
    println!(
        "  {} Chained through {} spec(s) in {:.1}s",
        "✓".green(),
        completed,
        total_elapsed.as_secs_f64()
    );

    if let Some((spec_id, error)) = &failed_spec {
        println!("  {} Stopped due to failure in {}", "✗".red(), spec_id);
        println!("    Error: {}", error);
        println!("{}", "═".repeat(60).dimmed());
        // Exit with error code
        std::process::exit(1);
    }

    if is_chain_interrupted() {
        println!("  {} Interrupted by user", "→".yellow());
    }

    println!("{}", "═".repeat(60).dimmed());

    Ok(())
}

// ============================================================================
// CHAIN OPTIONS STRUCT
// ============================================================================

/// Options for chain execution mode
pub struct ChainOptions<'a> {
    /// Maximum number of specs to chain (0 = unlimited)
    pub max_specs: usize,
    /// Labels to filter specs (ignored when specific_ids is not empty)
    pub labels: &'a [String],
    /// Prompt name override
    pub prompt_name: Option<&'a str>,
    /// CLI branch prefix override
    pub cli_branch: Option<&'a str>,
    /// Skip dependency checks
    pub skip_deps: bool,
    /// Skip acceptance criteria validation
    pub skip_criteria: bool,
    /// Allow spec completion without matching commits
    pub allow_no_commits: bool,
    /// Skip approval check
    pub skip_approval: bool,
    /// Specific spec IDs to chain through (if empty, chains through all ready specs)
    pub specific_ids: &'a [String],
}
