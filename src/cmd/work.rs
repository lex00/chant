//! Work command execution for chant CLI
//!
//! Handles spec execution including:
//! - Single spec execution with agent invocation
//! - Parallel spec execution with thread pools
//! - Spec finalization and status management
//! - Branch and PR creation
//! - Worktree management

use anyhow::{Context, Result};
use atty;
use colored::Colorize;
use rand::Rng;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::conflict;
use chant::paths::PROMPTS_DIR;
use chant::prompt;
use chant::spec::{self, BlockingDependency, Spec, SpecStatus};
use chant::worktree;
use dialoguer::Select;

/// Print usage hint for work command in non-TTY contexts
fn print_work_usage_hint() {
    println!("Usage: chant work <SPEC_ID>\n");
    println!("Examples:");
    println!("  chant work 2026-01-27-001-abc");
    println!("  chant work 001-abc");
    println!("  chant work --parallel\n");
    println!("Run 'chant work --help' for all options.");
}

use crate::cmd;
use crate::cmd::commits::get_commits_for_spec;
use crate::cmd::finalize::{
    append_agent_output, confirm_re_finalize, finalize_spec, re_finalize_spec,
};
use crate::cmd::git_ops::{commit_transcript, create_or_switch_branch};
use crate::cmd::spec as spec_cmd;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
        format!("chant work {} --force", spec_id).cyan()
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

/// Load all ready specs from the specs directory
fn load_ready_specs(specs_dir: &Path) -> Result<Vec<Spec>> {
    let all_specs = spec::load_all_specs(specs_dir)?;
    let ready_specs: Vec<Spec> = all_specs
        .iter()
        .filter(|s| s.is_ready(&all_specs))
        .cloned()
        .collect();
    Ok(ready_specs)
}

/// List all available prompts from the prompts directory
fn list_available_prompts(prompts_dir: &Path) -> Result<Vec<String>> {
    let mut prompts = Vec::new();
    if prompts_dir.exists() && prompts_dir.is_dir() {
        for entry in std::fs::read_dir(prompts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                if let Some(stem) = path.file_stem() {
                    prompts.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }
    prompts.sort();
    Ok(prompts)
}

/// Run the interactive wizard for selecting a spec
fn run_wizard(specs_dir: &Path, prompts_dir: &Path) -> Result<Option<WizardSelection>> {
    // Load ready specs
    let ready_specs = load_ready_specs(specs_dir)?;

    if ready_specs.is_empty() {
        println!("No ready specs to execute.");
        return Ok(None);
    }

    // Build spec selection items
    let spec_items: Vec<String> = ready_specs
        .iter()
        .map(|s| {
            if let Some(title) = &s.title {
                format!("{}  {}", s.id, title)
            } else {
                s.id.clone()
            }
        })
        .collect();

    // Add parallel option at the end
    let mut all_items = spec_items.clone();
    all_items.push("[Run all ready specs in parallel]".to_string());

    // Show spec selection
    let selection = Select::new()
        .with_prompt("? Select spec to work")
        .items(&all_items)
        .default(0)
        .interact()?;

    // Check if parallel mode was selected
    if selection == all_items.len() - 1 {
        return Ok(Some(WizardSelection::Parallel));
    }

    let selected_spec = ready_specs[selection].clone();

    // Show prompt selection
    let available_prompts = list_available_prompts(prompts_dir)?;

    if available_prompts.is_empty() {
        anyhow::bail!("No prompts found in {}", prompts_dir.display());
    }

    let prompt_selection = Select::new()
        .with_prompt("? Select prompt")
        .items(&available_prompts)
        .default(0)
        .interact()?;

    let selected_prompt = available_prompts[prompt_selection].clone();

    // Show branch confirmation
    let create_branch = dialoguer::Confirm::new()
        .with_prompt("Create feature branch")
        .default(false)
        .interact()?;

    Ok(Some(WizardSelection::SingleSpec {
        spec_id: selected_spec.id,
        prompt: selected_prompt,
        create_branch,
    }))
}

/// Result of the wizard selection
enum WizardSelection {
    /// Run a single spec
    SingleSpec {
        spec_id: String,
        prompt: String,
        create_branch: bool,
    },
    /// Run all ready specs in parallel
    Parallel,
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

// ============================================================================
// EXECUTION FUNCTIONS
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn cmd_work(
    ids: &[String],
    prompt_name: Option<&str>,
    cli_branch: Option<String>,
    force: bool,
    parallel: bool,
    labels: &[String],
    finalize: bool,
    allow_no_commits: bool,
    max_parallel: Option<usize>,
    no_cleanup: bool,
    force_cleanup: bool,
    skip_approval: bool,
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
        let options = ParallelOptions {
            max_override: max_parallel,
            no_cleanup,
            force_cleanup,
            labels,
            branch_prefix: cli_branch.as_deref(),
            prompt_name,
            specific_ids: ids,
        };
        return cmd_work_parallel(&specs_dir, &prompts_dir, &config, options);
    }

    // If no ID and not parallel, check for TTY
    let (final_id, final_prompt, final_branch) = if ids.is_empty() {
        // If not a TTY, print usage hint instead of launching wizard
        if !atty::is(atty::Stream::Stdin) {
            print_work_usage_hint();
            return Ok(());
        }
        match run_wizard(&specs_dir, &prompts_dir)? {
            Some(WizardSelection::SingleSpec {
                spec_id,
                prompt,
                create_branch,
            }) => (spec_id, Some(prompt), create_branch),
            Some(WizardSelection::Parallel) => {
                // User selected parallel mode via wizard
                let options = ParallelOptions {
                    max_override: max_parallel,
                    no_cleanup,
                    force_cleanup,
                    labels,
                    branch_prefix: cli_branch.as_deref(),
                    prompt_name,
                    specific_ids: &[],
                };
                return cmd_work_parallel(&specs_dir, &prompts_dir, &config, options);
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

        // Ask for confirmation (unless --force is used)
        if !confirm_re_finalize(&spec.id, force)? {
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
    if spec.frontmatter.status == SpecStatus::Completed && !force {
        println!("{} Spec already completed.", "⚠".yellow());
        println!("Use {} to replay.", "--force".cyan());
        return Ok(());
    }

    // Check if in progress
    if spec.frontmatter.status == SpecStatus::InProgress && !force {
        println!("{} Spec already in progress.", "⚠".yellow());
        return Ok(());
    }

    // Check if dependencies are satisfied
    let all_specs = spec::load_all_specs(&specs_dir)?;
    if !spec.is_ready(&all_specs) {
        // Get detailed blocking dependency information
        let blockers = spec.get_blocking_dependencies(&all_specs, &specs_dir);

        if !blockers.is_empty() {
            if force {
                // Print warning when forcing past dependency checks
                eprintln!(
                    "{} Warning: Forcing work on spec (skipping dependency checks)",
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
        create_or_switch_branch(&branch_name)?;
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
                println!("Use {} to skip this validation.", "--force".cyan());

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

    Ok(())
}

/// Result of a single spec execution in parallel mode
#[allow(dead_code)]
#[derive(Clone)]
struct ParallelResult {
    spec_id: String,
    success: bool,
    commits: Option<Vec<String>>,
    error: Option<String>,
    worktree_path: Option<PathBuf>,
    branch_name: Option<String>,
    is_direct_mode: bool,
    agent_completed: bool, // Whether agent work completed (separate from merge status)
}

/// Options for parallel execution
#[derive(Default)]
pub struct ParallelOptions<'a> {
    /// Override maximum total concurrent agents
    pub max_override: Option<usize>,
    /// Skip cleanup prompt after execution
    pub no_cleanup: bool,
    /// Force cleanup prompt even on success
    pub force_cleanup: bool,
    /// Labels to filter specs
    pub labels: &'a [String],
    /// CLI branch prefix override
    pub branch_prefix: Option<&'a str>,
    /// Prompt name override
    pub prompt_name: Option<&'a str>,
    /// Specific spec IDs to run (if empty, runs all ready specs)
    pub specific_ids: &'a [String],
}

/// Assignment of a spec to an agent
#[derive(Debug, Clone)]
struct AgentAssignment {
    spec_id: String,
    agent_name: String,
    agent_command: String,
}

/// Check if a warning should be shown when parallel mode uses agent rotation
/// with user-specified model preferences that will be ignored.
///
/// Returns true if a warning should be shown, false otherwise.
fn should_warn_model_override_in_parallel(
    config: &Config,
    prompt_name: Option<&str>,
    chant_model_set: bool,
    anthropic_model_set: bool,
) -> bool {
    // Check if using agent rotation (multiple agents or non-"none" rotation strategy)
    let uses_agent_rotation =
        config.parallel.agents.len() > 1 || config.defaults.rotation_strategy != "none";

    // If not using agent rotation, no warning needed
    if !uses_agent_rotation {
        return false;
    }

    // Check if user has set model preferences
    let config_model_set = config.defaults.model.is_some();

    // Check if user specified a non-default prompt
    let user_specified_prompt = prompt_name
        .map(|p| p != "standard" && p != config.defaults.prompt.as_str())
        .unwrap_or(false);

    // If any model preference is set or custom prompt is specified, warn
    chant_model_set || anthropic_model_set || config_model_set || user_specified_prompt
}

/// Warn when parallel execution uses agent rotation and user has set model preferences
/// that will be ignored because each agent has its own CLI profile with its own model.
fn warn_model_override_in_parallel(config: &Config, prompt_name: Option<&str>) {
    let chant_model_set = std::env::var("CHANT_MODEL").is_ok();
    let anthropic_model_set = std::env::var("ANTHROPIC_MODEL").is_ok();

    if !should_warn_model_override_in_parallel(
        config,
        prompt_name,
        chant_model_set,
        anthropic_model_set,
    ) {
        return;
    }

    // Print warning
    eprintln!(
        "{} Note: Parallel mode uses agent CLI profile models, not config/prompt settings",
        "⚠️ ".yellow()
    );
    eprintln!("   The prompt instructions are used, but model selection comes from:");

    // List agents and their config sources
    let agents = if config.parallel.agents.is_empty() {
        vec![chant::config::AgentConfig::default()]
    } else {
        config.parallel.agents.clone()
    };

    for agent in &agents {
        eprintln!(
            "   - {} → model from `{} config show`",
            agent.name, agent.command
        );
    }
    eprintln!();
    eprintln!("   To change which model is used:");
    for agent in &agents {
        eprintln!(
            "   $ {} config set model <opus|sonnet|haiku>",
            agent.command
        );
    }
    eprintln!();
}

/// Distribute specs across agents respecting per-agent and total limits
fn distribute_specs_to_agents(
    specs: &[Spec],
    config: &Config,
    max_override: Option<usize>,
) -> Vec<AgentAssignment> {
    use chant::config::AgentConfig;

    let agents = if config.parallel.agents.is_empty() {
        vec![AgentConfig::default()]
    } else {
        config.parallel.agents.clone()
    };

    let total_max = max_override.unwrap_or_else(|| config.parallel.total_capacity());

    // Track current allocation per agent
    let mut agent_allocations: Vec<usize> = vec![0; agents.len()];
    let mut assignments = Vec::new();

    for spec in specs {
        if assignments.len() >= total_max {
            break;
        }

        // Find agent with most remaining capacity (least-loaded-first strategy)
        let mut best_agent_idx = None;
        let mut best_remaining_capacity = 0;

        for (idx, agent) in agents.iter().enumerate() {
            let remaining = agent.max_concurrent.saturating_sub(agent_allocations[idx]);
            if remaining > best_remaining_capacity {
                best_remaining_capacity = remaining;
                best_agent_idx = Some(idx);
            }
        }

        if let Some(idx) = best_agent_idx {
            agent_allocations[idx] += 1;
            assignments.push(AgentAssignment {
                spec_id: spec.id.clone(),
                agent_name: agents[idx].name.clone(),
                agent_command: agents[idx].command.clone(),
            });
        }
    }

    assignments
}

pub fn cmd_work_parallel(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: ParallelOptions,
) -> Result<()> {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    // Load specs: either specific IDs or all ready specs
    let ready_specs: Vec<Spec> = if !options.specific_ids.is_empty() {
        // Resolve specific IDs
        let mut specs = Vec::new();
        for id in options.specific_ids {
            match spec::resolve_spec(specs_dir, id) {
                Ok(s) => specs.push(s),
                Err(e) => {
                    println!("{} Failed to resolve spec '{}': {}", "✗".red(), id, e);
                    return Err(e);
                }
            }
        }
        specs
    } else {
        // Load all ready specs
        let all_specs = spec::load_all_specs(specs_dir)?;
        let mut specs: Vec<Spec> = all_specs
            .iter()
            .filter(|s| s.frontmatter.status != SpecStatus::Cancelled && s.is_ready(&all_specs))
            .cloned()
            .collect();

        // Filter by labels if specified
        if !options.labels.is_empty() {
            specs.retain(|s| {
                if let Some(spec_labels) = &s.frontmatter.labels {
                    options.labels.iter().any(|l| spec_labels.contains(l))
                } else {
                    false
                }
            });
        }
        specs
    };

    if ready_specs.is_empty() {
        if !options.specific_ids.is_empty() {
            println!("No specs resolved from provided IDs.");
        } else if !options.labels.is_empty() {
            println!("No ready specs with specified labels.");
        } else {
            println!("No ready specs to execute.");
        }
        return Ok(());
    }

    // Distribute specs across configured agents
    let assignments = distribute_specs_to_agents(&ready_specs, config, options.max_override);

    if assignments.len() < ready_specs.len() {
        println!(
            "{} Warning: Only {} of {} ready specs will be executed (capacity limit)",
            "⚠".yellow(),
            assignments.len(),
            ready_specs.len()
        );
    }

    // Warn if user has set model preferences that will be ignored by agent CLI profiles
    warn_model_override_in_parallel(config, options.prompt_name);

    // Show agent distribution
    println!(
        "{} Starting {} specs in parallel...\n",
        "→".cyan(),
        assignments.len()
    );

    // Group assignments by agent for display
    let mut agent_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for assignment in &assignments {
        *agent_counts.entry(&assignment.agent_name).or_insert(0) += 1;
    }
    for (agent_name, count) in &agent_counts {
        println!("  {} {}: {} specs", "•".dimmed(), agent_name, count);
    }
    println!();

    // Resolve prompt name for all specs
    let default_prompt = &config.defaults.prompt;

    // Create channels for collecting results
    let (tx, rx) = mpsc::channel::<ParallelResult>();

    // Spawn threads for each assignment
    let mut handles = Vec::new();

    // Create a map of spec_id to spec for quick lookup
    let spec_map: std::collections::HashMap<&str, &Spec> =
        ready_specs.iter().map(|s| (s.id.as_str(), s)).collect();

    for assignment in assignments.iter() {
        let spec = match spec_map.get(assignment.spec_id.as_str()) {
            Some(s) => *s,
            None => continue,
        };

        // Determine prompt for this spec: explicit > frontmatter > auto-select by type > default
        let spec_prompt = options
            .prompt_name
            .map(|s| s.to_string())
            .or_else(|| spec.frontmatter.prompt.clone())
            .or_else(|| auto_select_prompt_for_type(spec, prompts_dir))
            .unwrap_or_else(|| default_prompt.to_string());

        let prompt_path = prompts_dir.join(format!("{}.md", spec_prompt));
        if !prompt_path.exists() {
            println!(
                "{} [{}] Prompt not found: {}",
                "✗".red(),
                spec.id,
                spec_prompt
            );
            continue;
        }
        let spec_prompt = spec_prompt.as_str();

        // Update spec status to in_progress
        let spec_path = specs_dir.join(format!("{}.md", spec.id));
        let mut spec_clone = spec.clone();
        spec_clone.frontmatter.status = SpecStatus::InProgress;
        if let Err(e) = spec_clone.save(&spec_path) {
            println!("{} [{}] Failed to update status: {}", "✗".red(), spec.id, e);
            continue;
        }

        println!(
            "[{}] Working with prompt '{}' via {}",
            spec.id.cyan(),
            spec_prompt,
            assignment.agent_name.dimmed()
        );

        // Assemble the prompt message
        let message = match prompt::assemble(&spec_clone, &prompt_path, config) {
            Ok(m) => m,
            Err(e) => {
                println!(
                    "{} [{}] Failed to assemble prompt: {}",
                    "✗".red(),
                    spec.id,
                    e
                );
                continue;
            }
        };

        // Determine branch mode
        // Priority: CLI --branch flag > spec frontmatter.branch > config defaults.branch
        // IMPORTANT: Parallel execution forces branch mode internally for isolation
        let (is_direct_mode, branch_prefix) = if let Some(cli_prefix) = options.branch_prefix {
            // CLI --branch specified with explicit prefix
            (false, cli_prefix.to_string())
        } else if let Some(spec_branch) = &spec.frontmatter.branch {
            // Spec has explicit branch prefix
            (false, spec_branch.clone())
        } else if config.defaults.branch {
            // Config enables branch mode - use config's branch_prefix
            (false, config.defaults.branch_prefix.clone())
        } else {
            // Parallel execution forces branch mode even if config.defaults.branch is false
            // This prevents merge race conditions during parallel work
            // Use config's branch_prefix to stay consistent with merge command expectations
            (false, config.defaults.branch_prefix.clone())
        };

        // Determine branch name based on mode
        let branch_name = if is_direct_mode {
            // Direct mode uses config prefix (this branch is currently unused)
            format!("{}{}", config.defaults.branch_prefix, spec.id)
        } else {
            format!("{}{}", branch_prefix, spec.id)
        };

        // Create worktree
        let worktree_result = worktree::create_worktree(&spec.id, &branch_name);
        let (worktree_path, branch_for_cleanup) = match worktree_result {
            Ok(path) => (Some(path), Some(branch_name.clone())),
            Err(e) => {
                println!(
                    "{} [{}] Failed to create worktree: {}",
                    "✗".red(),
                    spec.id,
                    e
                );
                // Update spec to failed
                let spec_path = specs_dir.join(format!("{}.md", spec.id));
                if let Ok(mut failed_spec) = spec::resolve_spec(specs_dir, &spec.id) {
                    failed_spec.frontmatter.status = SpecStatus::Failed;
                    let _ = failed_spec.save(&spec_path);
                }
                // Send failed result without spawning thread
                let _ = tx.send(ParallelResult {
                    spec_id: spec.id.clone(),
                    success: false,
                    commits: None,
                    error: Some(e.to_string()),
                    worktree_path: None,
                    branch_name: None,
                    is_direct_mode,
                    agent_completed: false,
                });
                continue;
            }
        };

        // Clone data for the thread
        let tx_clone = tx.clone();
        let spec_id = spec.id.clone();
        let specs_dir_clone = specs_dir.to_path_buf();
        let prompt_name_clone = spec_prompt.to_string();
        let config_model = config.defaults.model.clone();
        let worktree_path_clone = worktree_path.clone();
        let branch_for_cleanup_clone = branch_for_cleanup.clone();
        let is_direct_mode_clone = is_direct_mode;
        let agent_command = assignment.agent_command.clone();
        let config_clone = config.clone();

        let handle = thread::spawn(move || {
            let result = cmd::agent::invoke_agent_with_command(
                &message,
                &spec_id,
                &prompt_name_clone,
                config_model.as_deref(),
                worktree_path_clone.as_deref(),
                &agent_command,
            );
            let (success, commits, error, agent_completed) = match result {
                Ok(_) => {
                    // Agent work succeeded - get commits
                    let commits = get_commits_for_spec(&spec_id).ok();

                    // Finalize IN the worktree before removing it
                    // This prevents merge conflicts when feature branch is merged to main
                    eprintln!(
                        "{} [{}] Attempting to finalize spec in worktree",
                        "→".cyan(),
                        spec_id
                    );

                    let finalize_result = if let Some(ref worktree_path) = worktree_path_clone {
                        // Get the spec path in the worktree
                        let worktree_specs_dir = worktree_path.join(".chant/specs");
                        let worktree_spec_path = worktree_specs_dir.join(format!("{}.md", spec_id));

                        // Load the spec from the worktree
                        match spec::Spec::load(&worktree_spec_path) {
                            Ok(mut worktree_spec) => {
                                eprintln!(
                                    "{} [{}] Loaded spec from worktree (current status: {:?})",
                                    "→".cyan(),
                                    spec_id,
                                    worktree_spec.frontmatter.status
                                );

                                // Load all specs from worktree for finalization validation
                                let all_specs = match spec::load_all_specs(&worktree_specs_dir) {
                                    Ok(specs) => {
                                        eprintln!(
                                            "{} [{}] Loaded {} total specs for validation",
                                            "→".cyan(),
                                            spec_id,
                                            specs.len()
                                        );
                                        specs
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "{} [{}] Warning: Failed to load all specs for finalization: {}",
                                            "⚠".yellow(),
                                            spec_id,
                                            e
                                        );
                                        vec![]
                                    }
                                };

                                // Use proper finalize_spec function with extracted commits
                                let commits_to_finalize = commits.clone();
                                eprintln!(
                                    "{} [{}] Calling finalize_spec in worktree with {} commits",
                                    "→".cyan(),
                                    spec_id,
                                    commits_to_finalize.as_ref().map(|c| c.len()).unwrap_or(0)
                                );

                                match finalize_spec(
                                    &mut worktree_spec,
                                    &worktree_spec_path,
                                    &config_clone,
                                    &all_specs,
                                    false,
                                    commits_to_finalize,
                                ) {
                                    Ok(()) => {
                                        eprintln!(
                                            "{} [{}] ✓ Finalization succeeded in worktree",
                                            "✓".green(),
                                            spec_id
                                        );

                                        // Commit the finalization changes in the worktree
                                        let commit_message =
                                            format!("chant({}): finalize spec", spec_id);
                                        if let Err(e) = worktree::commit_in_worktree(
                                            worktree_path,
                                            &commit_message,
                                        ) {
                                            eprintln!(
                                                "{} [{}] Warning: Failed to commit finalization: {}",
                                                "⚠".yellow(),
                                                spec_id,
                                                e
                                            );
                                        }

                                        Ok(())
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "{} [{}] Failed to load spec from worktree: {}",
                                    "⚠".yellow(),
                                    spec_id,
                                    e
                                );
                                Err(e)
                            }
                        }
                    } else {
                        // No worktree - finalize on main branch (fallback)
                        let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                        if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                            let all_specs =
                                spec::load_all_specs(&specs_dir_clone).unwrap_or_default();
                            let commits_to_finalize = commits.clone();
                            finalize_spec(
                                &mut spec,
                                &spec_path,
                                &config_clone,
                                &all_specs,
                                false,
                                commits_to_finalize,
                            )
                        } else {
                            Err(anyhow::anyhow!("Failed to load spec for finalization"))
                        }
                    };

                    // Now remove the worktree after finalization is committed
                    if let Some(ref path) = worktree_path_clone {
                        if !is_direct_mode_clone {
                            // Branch mode: remove worktree now
                            if let Err(e) = worktree::remove_worktree(path) {
                                eprintln!(
                                    "{} [{}] Warning: Failed to remove worktree: {}",
                                    "⚠".yellow(),
                                    spec_id,
                                    e
                                );
                            }
                        }
                        // Direct mode: leave worktree for now, merge later
                    }

                    match finalize_result {
                        Ok(()) => {
                            eprintln!("{} [{}] ✓ Finalization complete", "✓".green(), spec_id);
                            (true, commits, None, true)
                        }
                        Err(e) => {
                            eprintln!("{} [{}] ✗ Cannot finalize spec: {}", "✗".red(), spec_id, e);
                            // Mark as needs attention instead of completed
                            let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                            if let Ok(mut failed_spec) =
                                spec::resolve_spec(&specs_dir_clone, &spec_id)
                            {
                                eprintln!(
                                    "{} [{}] Marking spec as NeedsAttention due to finalization error",
                                    "→".yellow(),
                                    spec_id
                                );
                                failed_spec.frontmatter.status = SpecStatus::NeedsAttention;
                                let _ = failed_spec.save(&spec_path);
                            }
                            (false, commits, Some(e.to_string()), false)
                        }
                    }
                }
                Err(e) => {
                    // Agent failed - cleanup worktree
                    if let Some(ref path) = worktree_path_clone {
                        if !is_direct_mode_clone {
                            let _ = worktree::remove_worktree(path);
                        }
                    }

                    // Update spec to failed
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        spec.frontmatter.status = SpecStatus::Failed;
                        let _ = spec.save(&spec_path);
                    }

                    (false, None, Some(e.to_string()), false)
                }
            };

            let _ = tx_clone.send(ParallelResult {
                spec_id,
                success,
                commits,
                error,
                worktree_path: worktree_path_clone,
                branch_name: branch_for_cleanup_clone,
                is_direct_mode: is_direct_mode_clone,
                agent_completed,
            });
        });

        handles.push(handle);

        // Apply stagger delay with jitter between spawning agents to avoid API rate limiting
        if config.parallel.stagger_delay_ms > 0 {
            let mut rng = rand::thread_rng();
            let jitter = if config.parallel.stagger_jitter_ms > 0 {
                // Generate random jitter from -jitter to +jitter
                rng.gen_range(
                    -(config.parallel.stagger_jitter_ms as i64)
                        ..=(config.parallel.stagger_jitter_ms as i64),
                )
            } else {
                0
            };

            // Calculate actual delay: base_delay + jitter, but ensure it's non-negative
            let delay_ms = (config.parallel.stagger_delay_ms as i64 + jitter).max(0) as u64;
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    // Drop the original sender so the receiver knows when all threads are done
    drop(tx);

    // Collect results from threads
    let mut completed = 0;
    let mut failed = 0;
    let mut all_results = Vec::new();
    let mut branch_mode_branches = Vec::new();
    let mut direct_mode_results = Vec::new();

    println!();

    for result in rx {
        if result.success {
            completed += 1;
            if let Some(ref commits) = result.commits {
                let commits_str = commits.join(", ");
                println!(
                    "[{}] {} Completed (commits: {})",
                    result.spec_id.cyan(),
                    "✓".green(),
                    commits_str
                );
            } else {
                println!("[{}] {} Completed", result.spec_id.cyan(), "✓".green());
            }

            // Collect branch info
            if result.is_direct_mode {
                direct_mode_results.push(result.clone());
            } else if let Some(ref branch) = result.branch_name {
                branch_mode_branches.push((result.spec_id.clone(), branch.clone()));
            }
        } else {
            failed += 1;
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");
            println!(
                "[{}] {} Failed: {}",
                result.spec_id.cyan(),
                "✗".red(),
                error_msg
            );
        }
        all_results.push(result);
    }

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }

    // =========================================================================
    // SERIALIZED MERGE PHASE - Handle all direct mode merges sequentially
    // =========================================================================

    let mut merged_count = 0;
    let mut merge_failed = Vec::new();

    for result in &direct_mode_results {
        if let Some(ref branch) = result.branch_name {
            println!("[{}] Merging to main...", result.spec_id.cyan());
            let merge_result = worktree::merge_and_cleanup(branch);

            if merge_result.success {
                merged_count += 1;
                println!("[{}] {} Merged to main", result.spec_id.cyan(), "✓".green());

                // Cleanup worktree after successful merge
                if let Some(ref path) = result.worktree_path {
                    let _ = worktree::remove_worktree(path);
                }
            } else {
                // Merge failed - preserve branch and worktree
                merge_failed.push((result.spec_id.clone(), merge_result.has_conflict));

                // Update spec status to indicate merge pending
                let spec_path = specs_dir.join(format!("{}.md", result.spec_id));
                if let Ok(mut spec) = spec::resolve_spec(specs_dir, &result.spec_id) {
                    spec.frontmatter.status = SpecStatus::NeedsAttention;
                    let _ = spec.save(&spec_path);
                }

                // Don't cleanup worktree - needed for manual merge

                let error_msg = merge_result
                    .error
                    .as_deref()
                    .unwrap_or("Unknown merge error");
                let branch_name = branch.as_str();
                println!(
                    "[{}] {} Merge failed (branch preserved):\n  {}\n  Next Steps:\n    1. Auto-resolve: chant merge {} --rebase --auto\n    2. Merge manually: chant merge {}\n    3. Inspect: git log {} --oneline -3",
                    result.spec_id.cyan(),
                    "⚠".yellow(),
                    error_msg,
                    result.spec_id,
                    result.spec_id,
                    branch_name
                );

                // Check for actual conflicts that need resolution spec
                if merge_result.has_conflict {
                    if let Ok(conflicting_files) = conflict::detect_conflicting_files() {
                        let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
                        let blocked_specs =
                            conflict::get_blocked_specs(&conflicting_files, &all_specs);

                        let source_branch = branch.to_string();
                        let (spec_title, _) =
                            conflict::extract_spec_context(specs_dir, &result.spec_id)
                                .unwrap_or((None, String::new()));
                        let diff_summary =
                            conflict::get_diff_summary(&source_branch, "main").unwrap_or_default();

                        let context = conflict::ConflictContext {
                            source_branch,
                            target_branch: "main".to_string(),
                            conflicting_files,
                            source_spec_id: result.spec_id.clone(),
                            source_spec_title: spec_title,
                            diff_summary,
                        };

                        if let Ok(conflict_spec_id) =
                            conflict::create_conflict_spec(specs_dir, &context, blocked_specs)
                        {
                            println!(
                                "[{}] Created conflict resolution spec: {}",
                                result.spec_id.cyan(),
                                conflict_spec_id
                            );
                        }
                    }
                }
            }
        }
    }

    // =========================================================================
    // CLEANUP PHASE - Remove worktrees for successful specs
    // =========================================================================

    cleanup_successful_worktrees(&all_results);

    // Auto-complete drivers if all their members completed
    let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();

    for result in &all_results {
        if result.success {
            // Check if this completed spec triggers driver auto-completion
            if let Ok(true) =
                spec::auto_complete_driver_if_ready(&result.spec_id, &all_specs, specs_dir)
            {
                if let Some(driver_id) = spec::extract_driver_id(&result.spec_id) {
                    println!(
                        "[{}] {} Auto-completed driver spec: {}",
                        result.spec_id.cyan(),
                        "✓".green(),
                        driver_id
                    );
                }
            }
        }
    }

    // Print summary
    println!("\n{}", "═".repeat(60).dimmed());
    println!("{}", "Parallel execution complete:".bold());
    println!("  {} {} specs completed work", "✓".green(), completed);

    if !direct_mode_results.is_empty() {
        println!("  {} {} branches merged to main", "✓".green(), merged_count);

        if !merge_failed.is_empty() {
            println!(
                "  {} {} branches preserved (merge pending)",
                "→".yellow(),
                merge_failed.len()
            );
            for (spec_id, has_conflict) in &merge_failed {
                let indicator = if *has_conflict { "⚡" } else { "→" };
                println!("    {} {}", indicator.yellow(), spec_id);
            }
        }
    }

    if failed > 0 {
        println!("  {} {} specs failed", "✗".red(), failed);
    }
    println!("{}", "═".repeat(60).dimmed());

    // Show branch mode information
    if !branch_mode_branches.is_empty() {
        println!("\n{} Branch mode branches created for merging:", "→".cyan());
        for (_spec_id, branch) in branch_mode_branches {
            println!("  {} {}", "•".yellow(), branch);
        }
        println!("\nUse {} to merge branches later.", "chant merge".bold());
    }

    // Show next steps for merge failures
    if !merge_failed.is_empty() {
        println!("\n{} Next steps for merge-pending branches:", "→".cyan());
        println!("  1. Review each branch:  git log <branch> --oneline -5");
        println!("  2. Auto-resolve conflicts:  chant merge --all --rebase --auto");
        println!("  3. Or merge sequentially:  chant merge <spec-id>");
        println!("  4. List worktrees:  git worktree list");
        println!("\n  Documentation: See 'chant merge --help' for more options");
    }

    // Detect parallel pitfalls
    let pitfalls = detect_parallel_pitfalls(&all_results, specs_dir);

    // Offer cleanup if issues found (and cleanup is enabled)
    let should_offer_cleanup = if options.force_cleanup {
        true
    } else if options.no_cleanup {
        false
    } else {
        config.parallel.cleanup.enabled && !pitfalls.is_empty()
    };

    if should_offer_cleanup && !pitfalls.is_empty() {
        println!("\n{} Issues detected:", "→".yellow());
        for pitfall in &pitfalls {
            let severity_icon = match pitfall.severity {
                PitfallSeverity::High => "✗".red(),
                PitfallSeverity::Medium => "⚠".yellow(),
                PitfallSeverity::Low => "→".dimmed(),
            };
            if let Some(ref spec_id) = pitfall.spec_id {
                println!("  {} [{}] {}", severity_icon, spec_id, pitfall.message);
            } else {
                println!("  {} {}", severity_icon, pitfall.message);
            }
        }

        if config.parallel.cleanup.auto_run {
            println!("\n{} Running cleanup agent...", "→".cyan());
            // Auto-run cleanup would be implemented here
        } else {
            println!(
                "\n{} Run {} to analyze and resolve issues.",
                "→".cyan(),
                "chant cleanup".bold()
            );
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// ============================================================================
// WORKTREE CLEANUP
// ============================================================================

/// Clean up worktrees for successfully completed specs
fn cleanup_successful_worktrees(results: &[ParallelResult]) {
    let mut cleaned_count = 0;
    let mut failed_cleanup = Vec::new();

    for result in results {
        // Only cleanup worktrees for successful specs that aren't direct mode merge-pending
        if result.success && result.agent_completed {
            if let Some(ref path) = result.worktree_path {
                if path.exists() {
                    match worktree::remove_worktree(path) {
                        Ok(()) => {
                            cleaned_count += 1;
                            eprintln!(
                                "{} [{}] Cleaned up worktree: {}",
                                "✓".green(),
                                result.spec_id,
                                path.display()
                            );
                        }
                        Err(e) => {
                            failed_cleanup.push((result.spec_id.clone(), e.to_string()));
                            eprintln!(
                                "{} [{}] Failed to cleanup worktree: {}",
                                "⚠".yellow(),
                                result.spec_id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    if cleaned_count > 0 {
        eprintln!(
            "{} Cleaned up {} worktree{}",
            "✓".green(),
            cleaned_count,
            if cleaned_count == 1 { "" } else { "s" }
        );
    }

    if !failed_cleanup.is_empty() {
        eprintln!(
            "\n{} Failed to cleanup {} worktree{}:",
            "⚠".yellow(),
            failed_cleanup.len(),
            if failed_cleanup.len() == 1 { "" } else { "s" }
        );
        for (spec_id, error) in failed_cleanup {
            eprintln!("  {} [{}]: {}", "→".yellow(), spec_id, error);
        }
        eprintln!(
            "\nRun {} to manually cleanup orphan worktrees.",
            "chant cleanup --worktrees".bold()
        );
    }
}

// ============================================================================
// PARALLEL PITFALL DETECTION
// ============================================================================

/// Severity level for parallel execution pitfalls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitfallSeverity {
    High,
    Medium,
    Low,
}

/// A detected pitfall from parallel execution
#[derive(Debug, Clone)]
pub struct Pitfall {
    pub spec_id: Option<String>,
    pub message: String,
    pub severity: PitfallSeverity,
    #[allow(dead_code)]
    pub pitfall_type: PitfallType,
}

/// Types of parallel execution pitfalls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PitfallType {
    ApiConcurrencyError,
    MergeConflict,
    PartialFailure,
    UncommittedChanges,
    StaleWorktree,
    AgentError,
}

/// Detect pitfalls from parallel execution results
fn detect_parallel_pitfalls(results: &[ParallelResult], specs_dir: &Path) -> Vec<Pitfall> {
    let mut pitfalls = Vec::new();

    // Check for failures
    for result in results {
        if !result.success {
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");

            // Check for API concurrency errors
            if error_msg.contains("429")
                || error_msg.contains("concurrency")
                || error_msg.contains("rate limit")
            {
                pitfalls.push(Pitfall {
                    spec_id: Some(result.spec_id.clone()),
                    message: format!("API concurrency error (retryable): {}", error_msg),
                    severity: PitfallSeverity::High,
                    pitfall_type: PitfallType::ApiConcurrencyError,
                });
            } else {
                pitfalls.push(Pitfall {
                    spec_id: Some(result.spec_id.clone()),
                    message: format!("Agent error: {}", error_msg),
                    severity: PitfallSeverity::High,
                    pitfall_type: PitfallType::AgentError,
                });
            }
        }

        // Check for worktrees that weren't cleaned up
        if let Some(ref path) = result.worktree_path {
            if path.exists() {
                pitfalls.push(Pitfall {
                    spec_id: Some(result.spec_id.clone()),
                    message: format!("Worktree not cleaned up: {}", path.display()),
                    severity: PitfallSeverity::Low,
                    pitfall_type: PitfallType::StaleWorktree,
                });
            }
        }
    }

    // Check for merge conflict indicators in specs
    if let Ok(all_specs) = spec::load_all_specs(specs_dir) {
        for spec in &all_specs {
            if spec.frontmatter.status == SpecStatus::NeedsAttention {
                // Check if it's a conflict resolution spec
                let title_lower = spec
                    .title
                    .as_ref()
                    .map(|t| t.to_lowercase())
                    .unwrap_or_default();
                if title_lower.contains("conflict") || title_lower.contains("merge") {
                    pitfalls.push(Pitfall {
                        spec_id: Some(spec.id.clone()),
                        message: "Merge conflict requires resolution".to_string(),
                        severity: PitfallSeverity::High,
                        pitfall_type: PitfallType::MergeConflict,
                    });
                }
            }
        }
    }

    // Check for partial failure (some succeeded, some failed)
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed_count = results.iter().filter(|r| !r.success).count();
    if succeeded > 0 && failed_count > 0 {
        pitfalls.push(Pitfall {
            spec_id: None,
            message: format!(
                "Partial failure: {} succeeded, {} failed",
                succeeded, failed_count
            ),
            severity: PitfallSeverity::Medium,
            pitfall_type: PitfallType::PartialFailure,
        });
    }

    pitfalls
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn is_silent_mode() -> bool {
    std::env::var("CHANT_SILENT_MODE")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or_default()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::commits::{get_commits_for_spec_allow_no_commits, CommitError};
    use chant::spec::SpecFrontmatter;
    use serial_test::serial;
    use tempfile::TempDir;

    /// Creates a temporary git repository with an initial commit.
    /// Returns the TempDir (must be kept alive) and the original working directory.
    /// The current directory is changed to the temp repo.
    fn setup_temp_git_repo() -> (TempDir, std::path::PathBuf) {
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        Command::new("git")
            .args(["init"])
            .output()
            .expect("Failed to init git repo");
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .output()
            .expect("Failed to set git email");
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .output()
            .expect("Failed to set git name");

        // Create an initial commit so HEAD exists
        std::fs::write(temp_dir.path().join("README.md"), "init").unwrap();
        Command::new("git")
            .args(["add", "."])
            .output()
            .expect("Failed to git add");
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .output()
            .expect("Failed to create initial commit");

        (temp_dir, original_dir)
    }

    #[test]
    fn test_commit_error_display() {
        // Test GitCommandFailed variant
        let err = CommitError::GitCommandFailed("test error".to_string());
        assert_eq!(
            err.to_string(),
            "Git command failed: test error",
            "GitCommandFailed should format correctly"
        );

        // Test NoMatchingCommits variant
        let err = CommitError::NoMatchingCommits;
        assert_eq!(
            err.to_string(),
            "No matching commits found",
            "NoMatchingCommits should format correctly"
        );
    }

    #[test]
    #[serial]
    fn test_get_commits_for_spec_error_behavior() {
        // This test verifies that when the spec ID doesn't have matching commits,
        // get_commits_for_spec returns an error (default behavior)
        let (_temp_dir, original_dir) = setup_temp_git_repo();

        let spec_id = "nonexistent-spec-xyz-abc-999";
        let result = get_commits_for_spec(spec_id);

        // Should return an error since there are no matching commits
        assert!(
            result.is_err(),
            "get_commits_for_spec should fail when no commits match the pattern"
        );

        // Check that the error is about missing commits
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("No matching commits found")
                    || error_msg.contains("Did the agent forget to commit"),
                "Error message should mention missing commits or agent error. Got: {}",
                error_msg
            );
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_get_commits_for_spec_allow_no_commits_behavior() {
        // This test verifies that when allow_no_commits is true,
        // the function returns HEAD as a fallback
        let (_temp_dir, original_dir) = setup_temp_git_repo();

        let spec_id = "nonexistent-spec-fallback-test";
        let result = get_commits_for_spec_allow_no_commits(spec_id);

        // Should succeed and return at least one commit (HEAD)
        assert!(
            result.is_ok(),
            "get_commits_for_spec_allow_no_commits should succeed with HEAD fallback"
        );

        if let Ok(commits) = result {
            assert!(
                !commits.is_empty(),
                "Should have at least one commit (HEAD)"
            );
            // HEAD should be a short hash (7 chars)
            assert!(commits[0].len() >= 7, "First commit should be HEAD hash");
        }

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_auto_select_prompt_documentation() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();

        // Create the documentation prompt file
        std::fs::write(prompts_dir.join("documentation.md"), "# Test prompt").unwrap();

        let spec = Spec {
            id: "test-spec".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "documentation".to_string(),
                tracks: Some(vec!["src/**/*.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test".to_string(),
        };

        let result = auto_select_prompt_for_type(&spec, prompts_dir);
        assert_eq!(result, Some("documentation".to_string()));
    }

    #[test]
    fn test_auto_select_prompt_code_type_returns_none() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();

        let spec = Spec {
            id: "test-spec".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test".to_string(),
        };

        let result = auto_select_prompt_for_type(&spec, prompts_dir);
        assert_eq!(result, None);
    }

    #[test]
    fn test_auto_select_prompt_task_type_returns_none() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();

        let spec = Spec {
            id: "test-spec".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "task".to_string(),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test".to_string(),
        };

        let result = auto_select_prompt_for_type(&spec, prompts_dir);
        assert_eq!(result, None);
    }

    #[test]
    fn test_auto_select_prompt_returns_none_when_prompt_file_missing() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();
        // Don't create any prompt files

        let spec = Spec {
            id: "test-spec".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "documentation".to_string(),
                tracks: Some(vec!["src/**/*.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test".to_string(),
        };

        // Should return None because documentation.md doesn't exist
        let result = auto_select_prompt_for_type(&spec, prompts_dir);
        assert_eq!(result, None);
    }

    // =========================================================================
    // DISTRIBUTION LOGIC TESTS
    // =========================================================================

    fn make_test_config_with_agents(agents: Vec<chant::config::AgentConfig>) -> Config {
        Config {
            project: chant::config::ProjectConfig {
                name: "test".to_string(),
                prefix: None,
            },
            defaults: chant::config::DefaultsConfig::default(),
            providers: chant::provider::ProviderConfig::default(),
            parallel: chant::config::ParallelConfig {
                agents,
                cleanup: chant::config::CleanupConfig::default(),
                stagger_delay_ms: 1000,
                stagger_jitter_ms: 200,
            },
            repos: vec![],
            enterprise: chant::config::EnterpriseConfig::default(),
            approval: chant::config::ApprovalConfig::default(),
        }
    }

    fn make_test_spec_for_parallel(id: &str) -> Spec {
        Spec {
            id: id.to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Ready,
                ..Default::default()
            },
            title: Some(format!("Spec {}", id)),
            body: "# Test".to_string(),
        }
    }

    #[test]
    fn test_distribute_specs_single_agent() {
        let agents = vec![chant::config::AgentConfig {
            name: "main".to_string(),
            command: "claude".to_string(),
            max_concurrent: 3,
            weight: 1,
        }];
        let config = make_test_config_with_agents(agents);

        let specs: Vec<Spec> = (1..=5)
            .map(|i| make_test_spec_for_parallel(&format!("spec-{}", i)))
            .collect();

        let assignments = distribute_specs_to_agents(&specs, &config, None);

        // Should assign 3 specs (agent max) to "main"
        assert_eq!(assignments.len(), 3);
        for assignment in &assignments {
            assert_eq!(assignment.agent_name, "main");
            assert_eq!(assignment.agent_command, "claude");
        }
    }

    #[test]
    fn test_distribute_specs_multiple_agents() {
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 3,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let specs: Vec<Spec> = (1..=5)
            .map(|i| make_test_spec_for_parallel(&format!("spec-{}", i)))
            .collect();

        let assignments = distribute_specs_to_agents(&specs, &config, None);

        // Should assign all 5 specs (2 + 3 = 5)
        assert_eq!(assignments.len(), 5);

        // Check distribution
        let main_count = assignments
            .iter()
            .filter(|a| a.agent_name == "main")
            .count();
        let alt1_count = assignments
            .iter()
            .filter(|a| a.agent_name == "alt1")
            .count();

        assert_eq!(main_count, 2);
        assert_eq!(alt1_count, 3);
    }

    #[test]
    fn test_distribute_specs_respects_total_max() {
        // Total capacity is sum of agent max_concurrent values (5 + 5 = 10)
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 5,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 5,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let specs: Vec<Spec> = (1..=15)
            .map(|i| make_test_spec_for_parallel(&format!("spec-{}", i)))
            .collect();

        let assignments = distribute_specs_to_agents(&specs, &config, None);

        // Should assign only 10 specs (5 + 5 total capacity)
        assert_eq!(assignments.len(), 10);
    }

    #[test]
    fn test_distribute_specs_with_max_override() {
        // Agent has capacity for 10, but we override with --max 3
        let agents = vec![chant::config::AgentConfig {
            name: "main".to_string(),
            command: "claude".to_string(),
            max_concurrent: 10,
            weight: 1,
        }];
        let config = make_test_config_with_agents(agents);

        let specs: Vec<Spec> = (1..=10)
            .map(|i| make_test_spec_for_parallel(&format!("spec-{}", i)))
            .collect();

        // Override with --max 3
        let assignments = distribute_specs_to_agents(&specs, &config, Some(3));

        assert_eq!(assignments.len(), 3);
    }

    #[test]
    fn test_distribute_specs_least_loaded_first() {
        // Agents with different capacities - should use least-loaded-first
        let agents = vec![
            chant::config::AgentConfig {
                name: "small".to_string(),
                command: "claude-small".to_string(),
                max_concurrent: 1,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "large".to_string(),
                command: "claude-large".to_string(),
                max_concurrent: 4,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let specs: Vec<Spec> = (1..=3)
            .map(|i| make_test_spec_for_parallel(&format!("spec-{}", i)))
            .collect();

        let assignments = distribute_specs_to_agents(&specs, &config, None);

        assert_eq!(assignments.len(), 3);

        // First spec should go to "large" (more capacity)
        assert_eq!(assignments[0].agent_name, "large");
    }

    // =========================================================================
    // PITFALL DETECTION TESTS
    // =========================================================================

    fn make_parallel_result(spec_id: &str, success: bool, error: Option<&str>) -> ParallelResult {
        ParallelResult {
            spec_id: spec_id.to_string(),
            success,
            commits: None,
            error: error.map(|s| s.to_string()),
            worktree_path: None,
            branch_name: None,
            is_direct_mode: false,
            agent_completed: success,
        }
    }

    #[test]
    fn test_detect_pitfalls_api_concurrency_error() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        let results = vec![make_parallel_result(
            "spec-001",
            false,
            Some("Error 429: Rate limit exceeded"),
        )];

        let pitfalls = detect_parallel_pitfalls(&results, specs_dir);

        assert_eq!(pitfalls.len(), 1);
        assert_eq!(pitfalls[0].pitfall_type, PitfallType::ApiConcurrencyError);
        assert_eq!(pitfalls[0].severity, PitfallSeverity::High);
    }

    #[test]
    fn test_detect_pitfalls_partial_failure() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        let results = vec![
            make_parallel_result("spec-001", true, None),
            make_parallel_result("spec-002", false, Some("Agent error")),
            make_parallel_result("spec-003", true, None),
        ];

        let pitfalls = detect_parallel_pitfalls(&results, specs_dir);

        // Should have 2 pitfalls: 1 AgentError + 1 PartialFailure
        assert!(pitfalls.len() >= 2);
        assert!(pitfalls
            .iter()
            .any(|p| p.pitfall_type == PitfallType::PartialFailure));
        assert!(pitfalls
            .iter()
            .any(|p| p.pitfall_type == PitfallType::AgentError));
    }

    #[test]
    fn test_detect_pitfalls_stale_worktree() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a worktree path that exists
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir(&worktree_path).unwrap();

        let mut result = make_parallel_result("spec-001", true, None);
        result.worktree_path = Some(worktree_path);

        let results = vec![result];

        let pitfalls = detect_parallel_pitfalls(&results, specs_dir);

        assert!(pitfalls
            .iter()
            .any(|p| p.pitfall_type == PitfallType::StaleWorktree));
    }

    #[test]
    fn test_detect_pitfalls_no_issues_on_success() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        let results = vec![
            make_parallel_result("spec-001", true, None),
            make_parallel_result("spec-002", true, None),
        ];

        let pitfalls = detect_parallel_pitfalls(&results, specs_dir);

        // No failures, no stale worktrees = no pitfalls
        assert!(pitfalls.is_empty());
    }

    // =========================================================================
    // WIZARD TESTS
    // =========================================================================

    #[test]
    fn test_load_ready_specs_empty() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Empty specs directory - should succeed but return no specs
        let result = load_ready_specs(specs_dir);

        assert!(result.is_ok());
        let specs = result.unwrap();
        assert_eq!(specs.len(), 0);
    }

    #[test]
    fn test_list_available_prompts_empty() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();

        let result = list_available_prompts(prompts_dir).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_list_available_prompts_finds_md_files() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();

        // Create some prompt files
        std::fs::write(prompts_dir.join("standard.md"), "# Standard").unwrap();
        std::fs::write(prompts_dir.join("tdd.md"), "# TDD").unwrap();
        std::fs::write(prompts_dir.join("minimal.md"), "# Minimal").unwrap();
        // Also create a non-markdown file to ensure it's ignored
        std::fs::write(prompts_dir.join("readme.txt"), "# Not a prompt").unwrap();

        let result = list_available_prompts(prompts_dir).unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.contains(&"standard".to_string()));
        assert!(result.contains(&"tdd".to_string()));
        assert!(result.contains(&"minimal".to_string()));
        // Should be sorted
        assert_eq!(result[0], "minimal");
        assert_eq!(result[1], "standard");
        assert_eq!(result[2], "tdd");
    }

    #[test]
    fn test_list_available_prompts_sorted() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path();

        // Create prompt files in non-alphabetical order
        std::fs::write(prompts_dir.join("zebra.md"), "# Z").unwrap();
        std::fs::write(prompts_dir.join("alpha.md"), "# A").unwrap();
        std::fs::write(prompts_dir.join("beta.md"), "# B").unwrap();

        let result = list_available_prompts(prompts_dir).unwrap();

        assert_eq!(result.len(), 3);
        // Should be alphabetically sorted
        assert_eq!(result[0], "alpha");
        assert_eq!(result[1], "beta");
        assert_eq!(result[2], "zebra");
    }

    // =========================================================================
    // CLEANUP TESTS
    // =========================================================================

    #[test]
    fn test_cleanup_successful_worktrees_cleans_up_successful_specs() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path().join("test-worktree");
        std::fs::create_dir(&worktree_path).unwrap();

        let results = vec![ParallelResult {
            spec_id: "spec-001".to_string(),
            success: true,
            commits: Some(vec!["abc123".to_string()]),
            error: None,
            worktree_path: Some(worktree_path.clone()),
            branch_name: Some("chant/spec-001".to_string()),
            is_direct_mode: false,
            agent_completed: true,
        }];

        // Should not panic and should attempt cleanup
        cleanup_successful_worktrees(&results);
        // Note: actual cleanup depends on worktree.rs::remove_worktree implementation
    }

    #[test]
    fn test_cleanup_successful_worktrees_skips_failed_specs() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path().join("test-worktree");
        std::fs::create_dir(&worktree_path).unwrap();

        let results = vec![ParallelResult {
            spec_id: "spec-001".to_string(),
            success: false,
            commits: None,
            error: Some("Test failure".to_string()),
            worktree_path: Some(worktree_path),
            branch_name: Some("chant/spec-001".to_string()),
            is_direct_mode: false,
            agent_completed: false,
        }];

        // Should not panic and should skip cleanup for failed specs
        cleanup_successful_worktrees(&results);
    }

    #[test]
    fn test_cleanup_successful_worktrees_with_no_worktree_path() {
        let results = vec![ParallelResult {
            spec_id: "spec-001".to_string(),
            success: true,
            commits: Some(vec!["abc123".to_string()]),
            error: None,
            worktree_path: None,
            branch_name: Some("chant/spec-001".to_string()),
            is_direct_mode: false,
            agent_completed: true,
        }];

        // Should not panic even without worktree_path
        cleanup_successful_worktrees(&results);
    }

    #[test]
    fn test_cleanup_successful_worktrees_mixed_results() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let worktree_path1 = temp_dir.path().join("test-worktree-1");
        std::fs::create_dir(&worktree_path1).unwrap();
        let worktree_path2 = temp_dir.path().join("test-worktree-2");
        std::fs::create_dir(&worktree_path2).unwrap();

        let results = vec![
            ParallelResult {
                spec_id: "spec-001".to_string(),
                success: true,
                commits: Some(vec!["abc123".to_string()]),
                error: None,
                worktree_path: Some(worktree_path1),
                branch_name: Some("chant/spec-001".to_string()),
                is_direct_mode: false,
                agent_completed: true,
            },
            ParallelResult {
                spec_id: "spec-002".to_string(),
                success: false,
                commits: None,
                error: Some("Test failure".to_string()),
                worktree_path: Some(worktree_path2),
                branch_name: Some("chant/spec-002".to_string()),
                is_direct_mode: false,
                agent_completed: false,
            },
        ];

        // Should clean up only successful spec, skip failed
        cleanup_successful_worktrees(&results);
    }

    // =========================================================================
    // MODEL OVERRIDE WARNING TESTS
    // =========================================================================

    #[test]
    fn test_warn_model_override_with_chant_model_set() {
        // Multiple agents + CHANT_MODEL set → should warn
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let should_warn = should_warn_model_override_in_parallel(
            &config, None,  // no custom prompt
            true,  // CHANT_MODEL is set
            false, // ANTHROPIC_MODEL not set
        );

        assert!(
            should_warn,
            "Should warn when multiple agents and CHANT_MODEL set"
        );
    }

    #[test]
    fn test_warn_model_override_with_anthropic_model_set() {
        // Multiple agents + ANTHROPIC_MODEL set → should warn
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let should_warn = should_warn_model_override_in_parallel(
            &config, None,  // no custom prompt
            false, // CHANT_MODEL not set
            true,  // ANTHROPIC_MODEL is set
        );

        assert!(
            should_warn,
            "Should warn when multiple agents and ANTHROPIC_MODEL set"
        );
    }

    #[test]
    fn test_warn_model_override_with_config_model_set() {
        // Multiple agents + config.defaults.model set → should warn
        let mut config = make_test_config_with_agents(vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
        ]);
        config.defaults.model = Some("claude-opus-4".to_string());

        let should_warn = should_warn_model_override_in_parallel(
            &config, None,  // no custom prompt
            false, // CHANT_MODEL not set
            false, // ANTHROPIC_MODEL not set
        );

        assert!(
            should_warn,
            "Should warn when multiple agents and config.model set"
        );
    }

    #[test]
    fn test_warn_model_override_with_custom_prompt() {
        // Multiple agents + custom prompt → should warn
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let should_warn = should_warn_model_override_in_parallel(
            &config,
            Some("research-analysis"), // custom prompt
            false,                     // CHANT_MODEL not set
            false,                     // ANTHROPIC_MODEL not set
        );

        assert!(
            should_warn,
            "Should warn when multiple agents and custom prompt"
        );
    }

    #[test]
    fn test_no_warn_with_rotation_strategy_none_single_agent() {
        // Single agent + rotation_strategy: none → should NOT warn
        let agents = vec![chant::config::AgentConfig {
            name: "main".to_string(),
            command: "claude".to_string(),
            max_concurrent: 2,
            weight: 1,
        }];
        let mut config = make_test_config_with_agents(agents);
        config.defaults.rotation_strategy = "none".to_string();

        let should_warn = should_warn_model_override_in_parallel(
            &config, None,  // no custom prompt
            true,  // CHANT_MODEL is set
            false, // ANTHROPIC_MODEL not set
        );

        assert!(
            !should_warn,
            "Should NOT warn when single agent and rotation_strategy is none"
        );
    }

    #[test]
    fn test_warn_with_rotation_strategy_random_single_agent() {
        // Single agent + rotation_strategy: random → SHOULD warn (rotation enabled)
        let agents = vec![chant::config::AgentConfig {
            name: "main".to_string(),
            command: "claude".to_string(),
            max_concurrent: 2,
            weight: 1,
        }];
        let mut config = make_test_config_with_agents(agents);
        config.defaults.rotation_strategy = "random".to_string();

        let should_warn = should_warn_model_override_in_parallel(
            &config, None,  // no custom prompt
            true,  // CHANT_MODEL is set
            false, // ANTHROPIC_MODEL not set
        );

        assert!(
            should_warn,
            "Should warn when rotation_strategy is not 'none' even with single agent"
        );
    }

    #[test]
    fn test_no_warn_without_model_preferences() {
        // Multiple agents but no model preferences → should NOT warn
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        let should_warn = should_warn_model_override_in_parallel(
            &config, None,  // no custom prompt (or default prompt)
            false, // CHANT_MODEL not set
            false, // ANTHROPIC_MODEL not set
        );

        assert!(
            !should_warn,
            "Should NOT warn when no model preferences are set"
        );
    }

    #[test]
    fn test_no_warn_with_default_prompt_name() {
        // Multiple agents + default prompt name → should NOT warn
        let agents = vec![
            chant::config::AgentConfig {
                name: "main".to_string(),
                command: "claude".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
            chant::config::AgentConfig {
                name: "alt1".to_string(),
                command: "claude-alt1".to_string(),
                max_concurrent: 2,
                weight: 1,
            },
        ];
        let config = make_test_config_with_agents(agents);

        // Using "standard" (the hardcoded default) or config.defaults.prompt
        let should_warn = should_warn_model_override_in_parallel(
            &config,
            Some("standard"), // default prompt
            false,            // CHANT_MODEL not set
            false,            // ANTHROPIC_MODEL not set
        );

        assert!(!should_warn, "Should NOT warn when using default prompt");
    }
}
