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
use chant::output::{Output, OutputMode};
use chant::paths::PROMPTS_DIR;
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, Spec, SpecStatus};
use chant::spec_group;
use chant::worktree;

use super::{executor, ui};
use crate::cmd;
use crate::cmd::finalize::{confirm_re_finalize, re_finalize_spec};

#[allow(clippy::too_many_arguments)]
pub fn cmd_work(
    ids: &[String],
    prompt_name: Option<&str>,
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
    no_watch: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let prompts_dir = PathBuf::from(PROMPTS_DIR);
    let config = Config::load()?;

    // Auto-start watch if not disabled
    if !no_watch {
        super::ensure_watch_running()?;
    }

    // Handle parallel execution mode (with specific IDs or all ready specs)
    if parallel {
        let options = super::ParallelOptions {
            max_override: max_parallel,
            no_cleanup,
            force_cleanup,
            labels,
            branch_prefix: None,
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
    let (final_id, final_prompt, _final_branch) = if ids.is_empty() {
        // If not a TTY, print usage hint instead of launching wizard
        if !atty::is(atty::Stream::Stdin) {
            ui::print_work_usage_hint();
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
                    branch_prefix: None,
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

    // Create output handler (Human mode for interactive CLI)
    let out = Output::new(OutputMode::Human);

    // Resolve spec
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Check if this is a driver/group spec - if so, execute members as a chain
    let all_specs = spec::load_all_specs(&specs_dir)?;
    if is_driver_or_group_spec(&spec, &all_specs) {
        return execute_driver_as_chain(
            &spec,
            &specs_dir,
            &prompts_dir,
            &config,
            prompt_name,
            skip_deps,
            skip_criteria,
            allow_no_commits,
            skip_approval,
        );
    }

    // Not a driver - continue with normal single spec execution
    let mut spec = spec;

    // Run validation through executor
    let validation_opts = executor::ValidationOptions {
        skip_deps,
        skip_criteria,
        skip_approval,
        skip_quality: false,
    };

    if let Err(e) = executor::validate_spec(&spec, &specs_dir, &config, &validation_opts) {
        show_friendly_validation_errors(&spec, &specs_dir, skip_approval, skip_deps)?;
        return Err(e);
    }

    // Handle re-finalization mode
    if finalize {
        return handle_refinalize_mode(
            &spec,
            &specs_dir,
            &config,
            &out,
            skip_criteria,
            allow_no_commits,
        );
    }

    // Interactive quality score display (single mode only)
    if !skip_criteria {
        check_quality_interactive(&spec, &specs_dir, &config)?;
    }

    // Worktree mode is always enabled
    // Create worktree for this spec
    let branch_name = format!("{}{}", config.defaults.branch_prefix, spec.id);
    let project_name = Some(config.project.name.as_str()).filter(|n| !n.is_empty());

    // Update status to in_progress BEFORE creating worktree
    // so copy_spec_to_worktree picks up the correct status.
    spec.set_status(SpecStatus::InProgress)
        .map_err(|e| anyhow::anyhow!("Failed to transition spec to InProgress: {}", e))?;
    spec.frontmatter.branch = Some(branch_name.clone());
    spec.save(&spec_path)?;

    let worktree_path = worktree::create_worktree(&spec.id, &branch_name, project_name)?;
    worktree::copy_spec_to_worktree(&spec.id, &worktree_path)?;
    worktree::isolate_worktree_specs(&spec.id, &worktree_path)?;
    out.step(&format!("Worktree: {}", worktree_path.display()));

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
    let final_prompt_name = resolved_prompt_name.as_str();

    // Create log file immediately
    cmd::agent::create_log_file_if_not_exists(&spec.id, final_prompt_name)?;

    // Write agent status file: working
    let status_path = specs_dir.join(format!(".chant-status-{}.json", spec.id));
    let agent_status = chant::worktree::status::AgentStatus {
        spec_id: spec.id.clone(),
        status: chant::worktree::status::AgentStatusState::Working,
        updated_at: chrono::Utc::now().to_rfc3339(),
        error: None,
        commits: vec![],
    };
    chant::worktree::status::write_status(&status_path, &agent_status)?;

    // If this is a member spec, mark the driver spec as in_progress if it's pending
    spec::mark_driver_in_progress(&specs_dir, &spec.id)?;

    // Invoke agent through executor
    let result = executor::invoke_agent_for_spec(
        &spec,
        final_prompt_name,
        &prompts_dir,
        &config,
        Some(&worktree_path),
    );

    match result {
        Ok(agent_output) => {
            executor::write_agent_status_done(&specs_dir, &spec.id, allow_no_commits)?;

            let worktree_spec_path = worktree_path
                .join(".chant/specs")
                .join(format!("{}.md", spec.id));
            let mut spec = if worktree_spec_path.exists() {
                spec::Spec::load(&worktree_spec_path)?
            } else {
                spec::resolve_spec(&specs_dir, &spec.id)?
            };

            let commits = executor::collect_commits_for_spec(&spec, allow_no_commits)?;
            executor::validate_output_schema(&spec, &agent_output, &config, &spec_path)?;
            executor::handle_acceptance_criteria(&mut spec, &spec_path, skip_criteria)?;
            executor::finalize_completed_spec(
                &mut spec,
                &specs_dir,
                &config,
                commits,
                allow_no_commits,
            )?;

            out.success("\nSpec completed!");
            if let Some(commits) = &spec.frontmatter.commits {
                for commit in commits {
                    out.info(&format!("Commit: {}", commit));
                }
            }
            if let Some(model) = &spec.frontmatter.model {
                out.info(&format!("Model: {}", model));
            }

            // Cleanup: append output and create transcript
            executor::cleanup_completed_spec(&mut spec, &spec_path, &agent_output)?;
        }
        Err(e) => {
            executor::handle_spec_failure(&spec.id, &specs_dir, &e)?;
            let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);
            println!("\n{} Spec failed: {}", "✗".red(), e);
            return Err(e);
        }
    }

    // Ensure main repo is back on main branch
    let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

    Ok(())
}

/// Handle re-finalization mode
fn handle_refinalize_mode(
    spec: &Spec,
    specs_dir: &Path,
    config: &Config,
    out: &Output,
    skip_criteria: bool,
    allow_no_commits: bool,
) -> Result<()> {
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

    if !confirm_re_finalize(&spec.id, skip_criteria)? {
        out.info("Re-finalization cancelled.");
        return Ok(());
    }

    if let Some(worktree_path) = worktree::get_active_worktree(&spec.id, None) {
        out.step(&format!("Re-finalizing spec {} in worktree...", spec.id));
        let worktree_spec_path = worktree_path
            .join(".chant/specs")
            .join(format!("{}.md", spec.id));
        let mut worktree_spec =
            spec::Spec::load(&worktree_spec_path).context("Failed to load spec from worktree")?;
        let worktree_specs_dir = worktree_path.join(".chant/specs");
        let spec_repo = FileSpecRepository::new(worktree_specs_dir);
        re_finalize_spec(&mut worktree_spec, &spec_repo, config, allow_no_commits)?;
        let commit_message = format!("chant({}): finalize spec", spec.id);
        worktree::commit_in_worktree(&worktree_path, &commit_message)?;
        out.success("Spec re-finalized in worktree!");
        print_finalization_info(&worktree_spec, Some(&worktree_path));
    } else {
        out.step(&format!("Re-finalizing spec {}...", spec.id));
        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
        let mut spec = spec.clone();
        re_finalize_spec(&mut spec, &spec_repo, config, allow_no_commits)?;
        out.success("Spec re-finalized!");
        print_finalization_info(&spec, None);
        let all_specs = spec::load_all_specs(specs_dir)?;
        if spec::auto_complete_driver_if_ready(&spec.id, &all_specs, specs_dir)? {
            out.success(&format!(
                "\nAuto-completed driver spec: {}",
                spec::extract_driver_id(&spec.id).unwrap()
            ));
        }
    }

    Ok(())
}

fn print_finalization_info(spec: &Spec, worktree_path: Option<&Path>) {
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
    if let Some(path) = worktree_path {
        println!("Worktree: {}", path.display());
    }
}

/// Show friendly validation errors for single mode
fn show_friendly_validation_errors(
    spec: &Spec,
    specs_dir: &Path,
    skip_approval: bool,
    skip_deps: bool,
) -> Result<()> {
    if !skip_approval && spec.requires_approval() {
        if let Some(approval) = &spec.frontmatter.approval {
            if approval.status == spec::ApprovalStatus::Pending {
                ui::print_approval_error(&spec.id);
            }
        }
    }

    if !skip_deps {
        let all_specs = spec::load_all_specs(specs_dir)?;
        if !spec.is_ready(&all_specs) {
            let blockers = spec.get_blocking_dependencies(&all_specs, specs_dir);
            if !blockers.is_empty() {
                ui::print_blocking_dependencies_error(&spec.id, &blockers);
            }
        }
    }

    Ok(())
}

/// Interactive quality score check for single mode
fn check_quality_interactive(spec: &Spec, specs_dir: &Path, config: &Config) -> Result<()> {
    use chant::scoring::TrafficLight;

    let all_specs = spec::load_all_specs(specs_dir)?;
    let quality_score = chant::scoring::calculate_spec_score(spec, &all_specs, config);

    match quality_score.traffic_light {
        TrafficLight::Refine => {
            eprintln!(
                "\n{} Spec {} has quality issues that may cause problems\n",
                "Warning:".red().bold(),
                spec.id.cyan()
            );
            ui::print_quality_assessment(&quality_score);
            ui::print_quality_suggestions_and_guidance(&quality_score);
            eprintln!();

            if atty::is(atty::Stream::Stdin) {
                if !ui::confirm_continue_with_quality_issues()? {
                    println!("Work cancelled.");
                    anyhow::bail!("User cancelled work due to quality issues");
                }
            } else {
                eprintln!(
                    "\n{} Cannot proceed in non-interactive mode with quality issues.",
                    "Error:".red().bold()
                );
                eprintln!("Use {} to bypass quality checks.", "--skip-criteria".cyan());
                anyhow::bail!("Spec quality check failed");
            }
        }
        TrafficLight::Review => {
            println!(
                "{} Spec quality: {} - Some dimensions need attention",
                "ℹ".yellow(),
                "Review".yellow()
            );
        }
        TrafficLight::Ready => {}
    }

    Ok(())
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

/// Check if a spec is a driver/group spec
fn is_driver_or_group_spec(spec: &Spec, all_specs: &[Spec]) -> bool {
    // Check if spec has type "group" or "driver"
    if spec.frontmatter.r#type == "group" || spec.frontmatter.r#type == "driver" {
        return true;
    }
    // Check if spec has members (i.e., other specs that are children of this spec)
    !spec_group::get_members(&spec.id, all_specs).is_empty()
}

/// Execute a driver spec by chaining through its member specs
#[allow(clippy::too_many_arguments)]
fn execute_driver_as_chain(
    driver: &Spec,
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    prompt_name: Option<&str>,
    skip_deps: bool,
    skip_criteria: bool,
    allow_no_commits: bool,
    skip_approval: bool,
) -> Result<()> {
    // Collect member specs sorted by sequence number (.1, .2, .3, ...)
    let all_specs = spec::load_all_specs(specs_dir)?;
    let mut members = spec_group::get_members(&driver.id, &all_specs);

    if members.is_empty() {
        anyhow::bail!(
            "Driver spec '{}' has no member specs. Cannot execute an empty driver.",
            driver.id
        );
    }

    // Sort members by spec ID to ensure sequence order
    members.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Check if all members are already completed
    if members
        .iter()
        .all(|m| m.frontmatter.status == SpecStatus::Completed)
    {
        println!(
            "{} Driver spec '{}' - all {} member(s) already completed",
            "✓".green(),
            driver.id,
            members.len()
        );
        return Ok(());
    }

    // Print message about chaining through members
    println!(
        "\n{} Executing driver spec '{}' by chaining through {} member spec(s)...\n",
        "→".cyan(),
        driver.id,
        members.len()
    );

    // Collect member IDs
    let member_ids: Vec<String> = members.iter().map(|m| m.id.clone()).collect();

    // Execute members as a chain
    let chain_options = super::ChainOptions {
        max_specs: 0, // No limit
        labels: &[],
        prompt_name,
        skip_deps,
        skip_criteria,
        allow_no_commits,
        skip_approval,
        specific_ids: &member_ids,
    };

    super::cmd_work_chain(specs_dir, prompts_dir, config, chain_options)
}
