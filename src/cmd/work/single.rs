//! Single-spec execution for the work command
//!
//! Handles the execution of individual specs with features including:
//! - Approval requirement checking
//! - Re-finalization mode support
//! - Quality score assessment
//! - Agent invocation and output validation
//! - Auto-finalization with acceptance criteria verification

use anyhow::{Context, Result};
use atty;
use colored::Colorize;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::paths::PROMPTS_DIR;
use chant::prompt;
use chant::spec::{self, BlockingDependency, Spec, SpecStatus};
use chant::validation;
use chant::worktree;

use crate::cmd;
use crate::cmd::finalize::{
    append_agent_output, confirm_re_finalize, finalize_spec, re_finalize_spec,
};
use crate::cmd::git_ops::commit_transcript;
use crate::cmd::spec as spec_cmd;

/// Print detailed error message for blocked spec dependencies.
///
/// Shows each blocking dependency with status indicator, title, and status details.
/// Includes actionable next steps and warnings for potentially stale blocked status.
fn print_blocking_dependencies_error(spec_id: &str, blockers: &[BlockingDependency]) {
    eprintln!(
        "\n{} Spec {} is blocked by dependencies\n",
        "Error:".red().bold(),
        spec_id.cyan()
    );
    eprintln!("Blocking dependencies:");

    for blocker in blockers {
        // Status indicator
        let status_indicator = match blocker.status {
            SpecStatus::Completed => "●".green(),
            SpecStatus::InProgress => "◐".yellow(),
            SpecStatus::Failed => "✗".red(),
            SpecStatus::Blocked => "◌".magenta(),
            _ => "○".white(),
        };

        // Title display
        let title_display = blocker.title.as_deref().unwrap_or("");
        let sibling_marker = if blocker.is_sibling { " (sibling)" } else { "" };

        eprintln!(
            "  {} {} {}{}",
            status_indicator,
            blocker.spec_id.cyan(),
            title_display,
            sibling_marker.dimmed()
        );
        eprintln!(
            "    Status: {}",
            format!("{:?}", blocker.status).to_lowercase()
        );

        // Show completed_at if available and warn about potential stale blocking
        if let Some(ref completed_at) = blocker.completed_at {
            eprintln!("    Completed at: {}", completed_at);
            if blocker.status == SpecStatus::Completed {
                eprintln!(
                    "    {} This dependency is complete but spec still shows as blocked - this may be a bug",
                    "⚠️".yellow()
                );
            }
        }
    }

    eprintln!("\nNext steps:");
    eprintln!(
        "  1. Run '{}' to update dependency status",
        "chant refresh".cyan()
    );
    eprintln!(
        "  2. Use '{}' to override dependency checks",
        format!("chant work {} --skip-deps", spec_id).cyan()
    );
    eprintln!(
        "  3. Check dependency details with '{}'",
        "chant show <dep-id>".cyan()
    );

    // Check if any dependencies are marked complete but still blocking
    let has_complete_blockers = blockers.iter().any(|b| b.status == SpecStatus::Completed);
    if has_complete_blockers {
        eprintln!(
            "\n{} If the dependency is truly complete, this is likely a dependency resolution bug",
            "Tip:".yellow().bold()
        );
    }
    eprintln!();
}

/// Format a grade enum for display with color coding
fn format_grade<T: std::fmt::Display>(grade: &T) -> colored::ColoredString {
    let grade_str = format!("{}", grade);
    match grade_str.as_str() {
        "A" => grade_str.green(),
        "B" => grade_str.green(),
        "C" => grade_str.yellow(),
        "D" => grade_str.red(),
        _ => grade_str.white(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_work(
    ids: &[String],
    prompt_name: Option<&str>,
    cli_branch: Option<String>,
    skip_deps: bool,
    skip_criteria: bool,
    parallel: bool,
    labels: &[String],
    finalize: bool,
    allow_no_commits: bool,
    max_parallel: Option<usize>,
    no_cleanup: bool,
    force_cleanup: bool,
    skip_approval: bool,
    chain: bool,
    chain_max: usize,
    no_merge: bool,
    no_rebase: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let prompts_dir = PathBuf::from(PROMPTS_DIR);
    let config = Config::load()?;

    // Check for silent mode conflicts
    let in_silent_mode = is_silent_mode();
    if in_silent_mode && cli_branch.is_some() {
        println!(
            "{} Warning: Creating branches in silent mode will still be visible to the team",
            "⚠".yellow()
        );
    }

    // Handle parallel execution mode (with specific IDs or all ready specs)
    if parallel {
        let options = super::ParallelOptions {
            max_override: max_parallel,
            no_cleanup,
            force_cleanup,
            labels,
            branch_prefix: cli_branch.as_deref(),
            prompt_name,
            specific_ids: ids,
            no_merge,
            no_rebase,
        };
        return super::cmd_work_parallel(&specs_dir, &prompts_dir, &config, options);
    }

    // Handle chain mode: loop through ready specs until none remain or failure
    if chain {
        let chain_options = super::ChainOptions {
            max_specs: chain_max,
            labels,
            prompt_name,
            cli_branch: cli_branch.as_deref(),
            skip_deps,
            skip_criteria,
            allow_no_commits,
            skip_approval,
            specific_ids: ids,
        };
        return super::cmd_work_chain(&specs_dir, &prompts_dir, &config, chain_options);
    }

    // Reject multiple IDs without --chain or --parallel
    if ids.len() > 1 {
        anyhow::bail!(
            "Multiple spec IDs provided without --chain or --parallel.\n\
             Use --chain to execute specs sequentially: chant work --chain {} {}\n\
             Use --parallel to execute specs concurrently: chant work --parallel {} {}",
            ids[0],
            ids[1],
            ids[0],
            ids[1]
        );
    }

    // If no ID and not parallel, check for TTY
    let (final_id, final_prompt, final_branch) = if ids.is_empty() {
        // If not a TTY, print usage hint instead of launching wizard
        if !atty::is(atty::Stream::Stdin) {
            print_work_usage_hint();
            return Ok(());
        }
        match super::run_wizard(&specs_dir, &prompts_dir)? {
            Some(super::WizardSelection::SingleSpec {
                spec_id,
                prompt,
                create_branch,
            }) => (spec_id, Some(prompt), create_branch),
            Some(super::WizardSelection::Parallel) => {
                // User selected parallel mode via wizard
                let options = super::ParallelOptions {
                    max_override: max_parallel,
                    no_cleanup,
                    force_cleanup,
                    labels,
                    branch_prefix: cli_branch.as_deref(),
                    prompt_name,
                    specific_ids: &[],
                    no_merge,
                    no_rebase,
                };
                return super::cmd_work_parallel(&specs_dir, &prompts_dir, &config, options);
            }
            None => return Ok(()),
        }
    } else {
        (ids[0].clone(), None, false)
    };

    let id = &final_id;

    // Resolve spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
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
            // Status is Pending
            eprintln!(
                "\n{} Spec {} requires approval before work can begin\n",
                "Error:".red().bold(),
                spec.id.cyan()
            );
            eprintln!("This spec has 'approval.required: true' but has not been approved yet.");
            eprintln!("\nNext steps:");
            eprintln!(
                "  1. Get approval: {}",
                format!("chant approve {} --by <name>", spec.id).cyan()
            );
            eprintln!(
                "  2. Or bypass with: {}",
                format!("chant work {} --skip-approval", spec.id).cyan()
            );
            eprintln!();
            anyhow::bail!("Spec requires approval");
        }
    }

    // Handle re-finalization mode
    if finalize {
        // Re-finalize flag requires the spec to be in_progress, completed, or failed
        // Allow failed too - agents often leave specs in failed state when they actually completed the work
        if spec.frontmatter.status != SpecStatus::InProgress
            && spec.frontmatter.status != SpecStatus::Completed
            && spec.frontmatter.status != SpecStatus::Failed
        {
            anyhow::bail!(
                "Cannot re-finalize spec '{}' with status '{:?}'. Must be in_progress, completed, or failed.",
                spec.id,
                spec.frontmatter.status
            );
        }

        // Ask for confirmation (unless --skip-criteria is used)
        if !confirm_re_finalize(&spec.id, skip_criteria)? {
            println!("Re-finalization cancelled.");
            return Ok(());
        }

        // Check if this spec has an active worktree - if so, finalize there
        if let Some(worktree_path) = worktree::get_active_worktree(&spec.id) {
            println!(
                "{} Re-finalizing spec {} in worktree...",
                "→".cyan(),
                spec.id
            );

            // Get the spec path in the worktree
            let worktree_spec_path = worktree_path
                .join(".chant/specs")
                .join(format!("{}.md", spec.id));

            // Load the spec from the worktree
            let mut worktree_spec = spec::Spec::load(&worktree_spec_path)
                .context("Failed to load spec from worktree")?;

            // Re-finalize in the worktree
            re_finalize_spec(
                &mut worktree_spec,
                &worktree_spec_path,
                &config,
                allow_no_commits,
            )?;

            // Commit the finalization changes in the worktree
            let commit_message = format!("chant({}): finalize spec", spec.id);
            worktree::commit_in_worktree(&worktree_path, &commit_message)?;

            println!("{} Spec re-finalized in worktree!", "✓".green());

            if let Some(commits) = &worktree_spec.frontmatter.commits {
                for commit in commits {
                    println!("Commit: {}", commit);
                }
            }
            if let Some(completed_at) = &worktree_spec.frontmatter.completed_at {
                println!("Completed at: {}", completed_at);
            }
            if let Some(model) = &worktree_spec.frontmatter.model {
                println!("Model: {}", model);
            }
            println!("Worktree: {}", worktree_path.display());
        } else {
            // No active worktree - finalize on current branch
            println!("{} Re-finalizing spec {}...", "→".cyan(), spec.id);
            re_finalize_spec(&mut spec, &spec_path, &config, allow_no_commits)?;
            println!("{} Spec re-finalized!", "✓".green());

            if let Some(commits) = &spec.frontmatter.commits {
                for commit in commits {
                    println!("Commit: {}", commit);
                }
            }
            if let Some(completed_at) = &spec.frontmatter.completed_at {
                println!("Completed at: {}", completed_at);
            }
            if let Some(model) = &spec.frontmatter.model {
                println!("Model: {}", model);
            }

            // If this is a member spec, check if driver should be auto-completed
            let all_specs = spec::load_all_specs(&specs_dir)?;
            if spec::auto_complete_driver_if_ready(&spec.id, &all_specs, &specs_dir)? {
                println!(
                    "\n{} Auto-completed driver spec: {}",
                    "✓".green(),
                    spec::extract_driver_id(&spec.id).unwrap()
                );
            }
        }

        return Ok(());
    }

    // Check if already completed
    if spec.frontmatter.status == SpecStatus::Completed && !(skip_deps || skip_criteria) {
        println!("{} Spec already completed.", "⚠".yellow());
        println!(
            "Use {} or {} to bypass.",
            "--skip-deps".cyan(),
            "--skip-criteria".cyan()
        );
        return Ok(());
    }

    // Check if in progress
    if spec.frontmatter.status == SpecStatus::InProgress && !(skip_deps || skip_criteria) {
        println!("{} Spec already in progress.", "⚠".yellow());
        return Ok(());
    }

    // Check if dependencies are satisfied
    let all_specs = spec::load_all_specs(&specs_dir)?;
    if !spec.is_ready(&all_specs) {
        // Get detailed blocking dependency information
        let blockers = spec.get_blocking_dependencies(&all_specs, &specs_dir);

        if !blockers.is_empty() {
            if skip_deps {
                // Print warning when skipping dependency checks
                eprintln!(
                    "{} Warning: Skipping dependency checks for spec",
                    "⚠".yellow()
                );
                let blocking_ids: Vec<String> = blockers
                    .iter()
                    .map(|b| format!("{} ({:?})", b.spec_id, b.status).to_lowercase())
                    .collect();
                eprintln!("  Skipping dependencies: {}", blocking_ids.join(", "));
            } else {
                // Print detailed error message
                print_blocking_dependencies_error(&spec.id, &blockers);
                anyhow::bail!("Spec blocked by dependencies");
            }
        }
    }

    // CLI flags override config defaults
    // Wizard selection overrides both config and CLI (when ids were empty)
    let use_branch_prefix = cli_branch
        .as_deref()
        .unwrap_or(&config.defaults.branch_prefix);
    let create_branch = if ids.is_empty() {
        // Wizard mode: use wizard's branch selection
        final_branch || cli_branch.is_some() || config.defaults.branch
    } else {
        // Direct mode: use CLI flags and config
        cli_branch.is_some() || config.defaults.branch
    };

    // Handle branch creation/switching if requested
    let _branch_name = if create_branch {
        let branch_name = format!("{}{}", use_branch_prefix, spec.id);
        crate::cmd::git_ops::create_or_switch_branch(&branch_name)?;
        spec.frontmatter.branch = Some(branch_name.clone());
        println!("{} Branch: {}", "→".cyan(), branch_name);
        Some(branch_name)
    } else {
        None
    };

    // Resolve prompt: CLI > wizard > frontmatter > auto-select by type > default
    let resolved_prompt_name = prompt_name
        .map(std::string::ToString::to_string)
        .or(final_prompt)
        .or_else(|| spec.frontmatter.prompt.clone())
        .or_else(|| auto_select_prompt_for_type(&spec, &prompts_dir))
        .unwrap_or_else(|| config.defaults.prompt.clone());

    let prompt_path = prompts_dir.join(format!("{}.md", resolved_prompt_name));
    if !prompt_path.exists() {
        anyhow::bail!("Prompt not found: {}", resolved_prompt_name);
    }
    let prompt_name = resolved_prompt_name.as_str();

    // Calculate quality score before starting work (unless --skip-criteria is used)
    if !skip_criteria {
        use chant::score::traffic_light;
        use chant::scoring::TrafficLight;

        let quality_score = chant::scoring::calculate_spec_score(&spec, &all_specs, &config);

        match quality_score.traffic_light {
            TrafficLight::Refine => {
                // Red status: Show warning and require user confirmation
                eprintln!(
                    "\n{} Spec {} has quality issues that may cause problems\n",
                    "Warning:".red().bold(),
                    spec.id.cyan()
                );

                // Show dimension grades
                eprintln!("Quality Assessment:");
                eprintln!(
                    "  Complexity:    {}",
                    format_grade(&quality_score.complexity)
                );
                eprintln!(
                    "  Confidence:    {}",
                    format_grade(&quality_score.confidence)
                );
                eprintln!(
                    "  Splittability: {}",
                    format_grade(&quality_score.splittability)
                );
                eprintln!(
                    "  AC Quality:    {}",
                    format_grade(&quality_score.ac_quality)
                );
                if let Some(iso) = quality_score.isolation {
                    eprintln!("  Isolation:     {}", format_grade(&iso));
                }

                // Show suggestions
                let suggestions = traffic_light::generate_suggestions(&quality_score);
                if !suggestions.is_empty() {
                    eprintln!("\nSuggestions:");
                    for suggestion in &suggestions {
                        eprintln!("  • {}", suggestion);
                    }
                }

                eprintln!();

                // Prompt user for confirmation (unless non-interactive)
                if atty::is(atty::Stream::Stdin) {
                    use std::io::{self, Write};

                    print!("Continue anyway? [y/N] ");
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let input = input.trim().to_lowercase();

                    if input != "y" && input != "yes" {
                        println!("Work cancelled.");
                        return Ok(());
                    }
                } else {
                    // Non-interactive mode: abort (user should use --skip-criteria to bypass)
                    eprintln!(
                        "\n{} Cannot proceed in non-interactive mode with quality issues.",
                        "Error:".red().bold()
                    );
                    eprintln!("Use {} to bypass quality checks.", "--skip-criteria".cyan());
                    anyhow::bail!("Spec quality check failed");
                }
            }
            TrafficLight::Review => {
                // Yellow status: Show info message but proceed automatically
                println!(
                    "{} Spec quality: {} - Some dimensions need attention",
                    "ℹ".yellow(),
                    "Review".yellow()
                );

                // Show which dimensions are at C level
                let mut review_dims = Vec::new();
                if matches!(quality_score.complexity, chant::scoring::ComplexityGrade::C) {
                    review_dims.push(format!(
                        "Complexity: {}",
                        format_grade(&quality_score.complexity)
                    ));
                }
                if matches!(quality_score.confidence, chant::scoring::ConfidenceGrade::C) {
                    review_dims.push(format!(
                        "Confidence: {}",
                        format_grade(&quality_score.confidence)
                    ));
                }
                if matches!(quality_score.ac_quality, chant::scoring::ACQualityGrade::C) {
                    review_dims.push(format!(
                        "AC Quality: {}",
                        format_grade(&quality_score.ac_quality)
                    ));
                }
                if matches!(
                    quality_score.splittability,
                    chant::scoring::SplittabilityGrade::C
                ) {
                    review_dims.push(format!(
                        "Splittability: {}",
                        format_grade(&quality_score.splittability)
                    ));
                }

                if !review_dims.is_empty() {
                    for dim in review_dims {
                        println!("  • {}", dim);
                    }
                }
                println!();
            }
            TrafficLight::Ready => {
                // Green status: Proceed silently (no message)
            }
        }
    }

    // Update status to in_progress
    spec.frontmatter.status = SpecStatus::InProgress;
    spec.save(&spec_path)?;

    // If this is a member spec, mark the driver spec as in_progress if it's pending
    spec::mark_driver_in_progress(&specs_dir, &spec.id)?;

    println!(
        "{} {} with prompt '{}'",
        "Working".cyan(),
        spec.id,
        prompt_name
    );

    // Assemble prompt
    let message = prompt::assemble(&spec, &prompt_path, &config)?;

    // Select agent for single spec execution based on rotation strategy
    let agent_command =
        if config.defaults.rotation_strategy != "none" && !config.parallel.agents.is_empty() {
            // Use rotation to select an agent
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
            prompt_name,
            &config,
            Some(&agent_cmd),
        )
    } else {
        cmd::agent::invoke_agent(&message, &spec, prompt_name, &config)
    };

    match result {
        Ok(agent_output) => {
            // Reload spec (it may have been modified by the agent)
            let mut spec = spec::resolve_spec(&specs_dir, &spec.id)?;

            // Auto-finalize logic after agent exits:
            // 1. Check if agent made a commit (indicates work was done)
            // 2. Run lint checks on the spec
            // 3. If all criteria checked, auto-finalize
            // 4. If criteria unchecked, fail with clear message

            // Check for commits and store them for finalization
            let found_commits = match if allow_no_commits {
                cmd::commits::get_commits_for_spec_allow_no_commits(&spec.id)
            } else {
                cmd::commits::get_commits_for_spec(&spec.id)
            } {
                Ok(commits) => {
                    if commits.is_empty() {
                        println!(
                            "\n{} No commits found - agent did not make any changes.",
                            "⚠".yellow()
                        );
                        // Mark as failed since no work was done
                        spec.frontmatter.status = SpecStatus::Failed;
                        spec.save(&spec_path)?;
                        anyhow::bail!("Cannot complete spec without commits - did the agent make any changes?");
                    }
                    commits
                }
                Err(e) => {
                    if allow_no_commits {
                        println!(
                            "\n{} No matching commits found, using HEAD as fallback.",
                            "→".cyan()
                        );
                        // Will use HEAD fallback in finalize
                        vec![]
                    } else {
                        println!("\n{} {}", "⚠".yellow(), e);
                        // Mark as failed since we need commits
                        spec.frontmatter.status = SpecStatus::Failed;
                        spec.save(&spec_path)?;
                        return Err(e);
                    }
                }
            };

            // Run lint on the spec to check acceptance criteria and get warnings
            let lint_result = spec_cmd::lint_specific_specs(&specs_dir, &[spec.id.clone()])?;

            // Check if all acceptance criteria are checked
            let unchecked_count = spec.count_unchecked_checkboxes();
            if unchecked_count > 0 {
                println!(
                    "\n{} Found {} unchecked acceptance {}.",
                    "⚠".yellow(),
                    unchecked_count,
                    if unchecked_count == 1 {
                        "criterion"
                    } else {
                        "criteria"
                    }
                );

                // Show which criteria are unchecked
                println!("Please check off all acceptance criteria before completing.");
                println!("Use {} to skip this validation.", "--skip-criteria".cyan());

                // Mark as failed since we can't complete with unchecked items
                spec.frontmatter.status = SpecStatus::Failed;
                spec.save(&spec_path)?;
                anyhow::bail!(
                    "Cannot auto-finalize spec with {} unchecked acceptance criteria",
                    unchecked_count
                );
            }

            // Show lint warnings if any (but allow finalization if criteria are checked)
            if lint_result.warned > 0 {
                println!(
                    "\n{} Lint check found {} warning(s), but criteria are all checked - proceeding with finalization.",
                    "→".cyan(),
                    lint_result.warned
                );
            }

            // Validate output against schema if output_schema is defined
            if let Some(ref schema_path_str) = spec.frontmatter.output_schema {
                let schema_path = Path::new(schema_path_str);
                if schema_path.exists() {
                    match validation::validate_agent_output(&spec.id, schema_path, &agent_output) {
                        Ok(result) => {
                            if result.is_valid {
                                println!(
                                    "\n{} Output validation passed (schema: {})",
                                    "✓".green(),
                                    schema_path_str
                                );
                            } else {
                                println!(
                                    "\n{} Output validation failed (schema: {})",
                                    "✗".red(),
                                    schema_path_str
                                );
                                for error in &result.errors {
                                    println!("  - {}", error);
                                }
                                println!("  → Review .chant/logs/{}.log for details", spec.id);

                                // Check if strict validation is enabled
                                if config.validation.strict_output_validation {
                                    spec.frontmatter.status = SpecStatus::NeedsAttention;
                                    spec.save(&spec_path)?;
                                    anyhow::bail!(
                                        "Output validation failed: {} error(s)",
                                        result.errors.len()
                                    );
                                } else {
                                    println!(
                                        "  {} Proceeding anyway (strict_output_validation=false)",
                                        "→".cyan()
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            println!("\n{} Failed to validate output: {}", "⚠".yellow(), e);
                            if config.validation.strict_output_validation {
                                spec.frontmatter.status = SpecStatus::NeedsAttention;
                                spec.save(&spec_path)?;
                                return Err(e);
                            }
                        }
                    }
                } else {
                    println!(
                        "\n{} Output schema file not found: {}",
                        "⚠".yellow(),
                        schema_path_str
                    );
                }
            }

            // All criteria are checked, auto-finalize the spec
            println!(
                "\n{} Auto-finalizing spec (all acceptance criteria checked)...",
                "→".cyan()
            );
            let all_specs = spec::load_all_specs(&specs_dir)?;
            // Pass the commits we already retrieved to avoid fetching twice
            let commits_to_pass = if found_commits.is_empty() {
                None // Let finalize fetch with fallback
            } else {
                Some(found_commits)
            };
            finalize_spec(
                &mut spec,
                &spec_path,
                &config,
                &all_specs,
                allow_no_commits,
                commits_to_pass,
            )?;

            // If this is a member spec, check if driver should be auto-completed
            // Reload specs to get the freshly-saved completed status
            let all_specs = spec::load_all_specs(&specs_dir)?;
            if spec::auto_complete_driver_if_ready(&spec.id, &all_specs, &specs_dir)? {
                println!(
                    "\n{} Auto-completed driver spec: {}",
                    "✓".green(),
                    spec::extract_driver_id(&spec.id).unwrap()
                );
            }

            println!("\n{} Spec completed!", "✓".green());
            if let Some(commits) = &spec.frontmatter.commits {
                for commit in commits {
                    println!("Commit: {}", commit);
                }
            }
            if let Some(model) = &spec.frontmatter.model {
                println!("Model: {}", model);
            }

            // Append agent output to spec body (after finalization so finalized spec is the base)
            append_agent_output(&mut spec, &agent_output);

            spec.save(&spec_path)?;

            // Create a follow-up commit for the transcript
            commit_transcript(&spec.id, &spec_path)?;
        }
        Err(e) => {
            // Update spec to failed
            let mut spec = spec::resolve_spec(&specs_dir, &spec.id)?;
            spec.frontmatter.status = SpecStatus::Failed;
            spec.save(&spec_path)?;

            println!("\n{} Spec failed: {}", "✗".red(), e);
            return Err(e);
        }
    }

    // Ensure main repo is back on main branch
    let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

    Ok(())
}

/// Print usage hint for work command in non-TTY contexts
fn print_work_usage_hint() {
    println!("Usage: chant work <SPEC_ID>\n");
    println!("Examples:");
    println!("  chant work 2026-01-27-001-abc");
    println!("  chant work 001-abc");
    println!("  chant work --parallel\n");
    println!("Run 'chant work --help' for all options.");
}

/// Auto-select a prompt based on spec type if the prompt file exists.
/// Returns None if no auto-selected prompt is appropriate or available.
fn auto_select_prompt_for_type(spec: &Spec, prompts_dir: &Path) -> Option<String> {
    let auto_prompt = match spec.frontmatter.r#type.as_str() {
        "documentation" => Some("documentation"),
        _ => None,
    };

    // Check if the auto-selected prompt actually exists
    if let Some(prompt_name) = auto_prompt {
        let prompt_path = prompts_dir.join(format!("{}.md", prompt_name));
        if prompt_path.exists() {
            return Some(prompt_name.to_string());
        }
    }

    None
}

/// Check if the system is in silent mode
fn is_silent_mode() -> bool {
    std::env::var("CHANT_SILENT").is_ok() || std::env::var("CI").is_ok()
}
