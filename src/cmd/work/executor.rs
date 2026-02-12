//! Shared execution functions for all work modes
//!
//! Provides unified validation, agent invocation, and finalization logic
//! that is used by single, chain, and parallel execution modes.

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use chant::config::Config;
use chant::operations::{
    get_commits_for_spec, get_commits_for_spec_allow_no_commits, get_commits_for_spec_with_branch,
    get_commits_for_spec_with_branch_allow_no_commits,
};
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, Spec, SpecStatus};
use chant::validation;

use crate::cmd;
use crate::cmd::finalize::{append_agent_output, finalize_spec};
use crate::cmd::git_ops::commit_transcript;

/// Validation options
pub struct ValidationOptions {
    pub skip_deps: bool,
    pub skip_criteria: bool,
    pub skip_approval: bool,
    pub skip_quality: bool,
}

/// Run full validation pipeline: lint + approval + dependency + quality
pub fn validate_spec(
    spec: &Spec,
    specs_dir: &Path,
    config: &Config,
    opts: &ValidationOptions,
) -> Result<()> {
    // Lint validation
    eprintln!("{} Validating spec {}...", "→".cyan(), spec.id);
    let lint_result =
        crate::cmd::spec::lint_specific_specs(specs_dir, std::slice::from_ref(&spec.id))?;

    if lint_result.failed > 0 {
        anyhow::bail!(
            "Spec validation failed with {} error(s). Fix the issues before running 'chant work'.\n\
             Run 'chant lint {}' to see details.",
            lint_result.failed,
            spec.id
        );
    }

    if lint_result.warned > 0 {
        eprintln!("{} Quality advisory for {}:", "⚠".yellow(), spec.id);
        for warning in &lint_result.diagnostics {
            eprintln!("  • {}", warning);
        }
        eprintln!(
            "{} Proceeding with execution (warnings are advisory)",
            "✓".green()
        );
    }

    // Validate work preconditions
    if !(opts.skip_deps || opts.skip_criteria) {
        validate_preconditions(spec)?;
    }

    // Approval check
    if !opts.skip_approval {
        validate_approval(spec)?;
    }

    // Dependency check
    if !opts.skip_deps {
        validate_dependencies(spec, specs_dir)?;
    } else {
        let all_specs = spec::load_all_specs(specs_dir)?;
        if !spec.is_ready(&all_specs) {
            let blockers = spec.get_blocking_dependencies(&all_specs, specs_dir);
            let blocker_ids: Vec<&str> = blockers.iter().map(|s| s.spec_id.as_str()).collect();
            eprintln!(
                "{} Skipping dependencies for spec '{}': {}",
                "⚠".yellow(),
                spec.id,
                blocker_ids.join(", ")
            );
        }
    }

    // Quality check (optional in chain/parallel)
    if !opts.skip_quality && !opts.skip_criteria {
        validate_quality(spec, specs_dir, config)?;
    }

    Ok(())
}

fn validate_preconditions(spec: &Spec) -> Result<()> {
    match spec.frontmatter.status {
        SpecStatus::Cancelled => {
            anyhow::bail!(
                "Cannot work on cancelled spec '{}'. Cancelled specs are not eligible for execution.",
                spec.id
            );
        }
        SpecStatus::Completed => {
            anyhow::bail!(
                "Cannot work on completed spec '{}'. Use --skip-deps or --skip-criteria to bypass.",
                spec.id
            );
        }
        SpecStatus::InProgress => {
            anyhow::bail!("Spec '{}' is already in progress.", spec.id);
        }
        SpecStatus::Pending
        | SpecStatus::Failed
        | SpecStatus::Blocked
        | SpecStatus::NeedsAttention
        | SpecStatus::Paused
        | SpecStatus::Ready => Ok(()),
    }
}

fn validate_approval(spec: &Spec) -> Result<()> {
    if spec.requires_approval() {
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
    Ok(())
}

fn validate_dependencies(spec: &Spec, specs_dir: &Path) -> Result<()> {
    let all_specs = spec::load_all_specs(specs_dir)?;
    // Check actual dependency state instead of using is_ready() as a proxy
    let blockers = spec.get_blocking_dependencies(&all_specs, specs_dir);
    if !blockers.is_empty() {
        let blocking_ids: Vec<String> = blockers.iter().map(|b| b.spec_id.clone()).collect();
        anyhow::bail!(
            "Spec '{}' is blocked by dependencies: {}",
            spec.id,
            blocking_ids.join(", ")
        );
    }
    Ok(())
}

fn validate_quality(spec: &Spec, specs_dir: &Path, config: &Config) -> Result<()> {
    use chant::score::traffic_light;
    use chant::scoring::TrafficLight;

    let all_specs = spec::load_all_specs(specs_dir)?;
    let quality_score = chant::scoring::calculate_spec_score(spec, &all_specs, config);

    match quality_score.traffic_light {
        TrafficLight::Refine => {
            eprintln!(
                "\n{} Spec {} has quality issues that may cause problems",
                "Warning:".red().bold(),
                spec.id.cyan()
            );
            let suggestions = traffic_light::generate_suggestions(&quality_score);
            if !suggestions.is_empty() {
                eprintln!("\nSuggestions:");
                for suggestion in &suggestions {
                    eprintln!("  • {}", suggestion);
                }
            }
        }
        TrafficLight::Review => {
            eprintln!(
                "{} Spec quality: {} - Some dimensions need attention",
                "ℹ".yellow(),
                "Review".yellow()
            );
        }
        TrafficLight::Ready => {}
    }

    Ok(())
}

/// Select and invoke agent for a spec
pub fn invoke_agent_for_spec(
    spec: &Spec,
    prompt_name: &str,
    prompts_dir: &Path,
    config: &Config,
    worktree_path: Option<&Path>,
) -> Result<String> {
    let prompt_path = prompts_dir.join(format!("{}.md", prompt_name));

    eprintln!(
        "{} {} with prompt '{}'",
        "Working".cyan(),
        spec.id,
        prompt_name
    );

    // Assemble prompt
    let message = chant::prompt::assemble(spec, &prompt_path, config)?;

    // Select agent based on rotation strategy
    let agent_command = if config.defaults.rotation_strategy != "none"
        && !config.parallel.agents.is_empty()
    {
        match cmd::agent_rotation::select_agent_for_work(
            &config.defaults.rotation_strategy,
            &config.parallel,
        ) {
            Ok(cmd) => Some(cmd),
            Err(e) => {
                eprintln!("{} Failed to select agent: {}", "⚠".yellow(), e);
                None
            }
        }
    } else {
        // Warn if agents are configured but rotation strategy is "none"
        if !config.parallel.agents.is_empty() && config.defaults.rotation_strategy == "none" {
            eprintln!(
                    "{} parallel.agents configured but rotation_strategy is 'none' — using default claude command. Set rotation_strategy: round-robin to enable agent rotation.",
                    "Warning:".yellow()
                );
        }
        None
    };

    // Invoke agent
    if let Some(agent_cmd) = agent_command {
        cmd::agent::invoke_agent_with_command_override(
            &message,
            spec,
            prompt_name,
            config,
            Some(&agent_cmd),
            worktree_path,
        )
    } else {
        cmd::agent::invoke_agent_with_model(
            &message,
            spec,
            prompt_name,
            config,
            None,
            worktree_path,
        )
    }
}

/// Collect commits for a spec
pub fn collect_commits_for_spec(spec: &Spec, allow_no_commits: bool) -> Result<Vec<String>> {
    let spec_branch = spec.frontmatter.branch.as_deref();
    let commits = if allow_no_commits {
        get_commits_for_spec_with_branch_allow_no_commits(&spec.id, spec_branch)
    } else {
        get_commits_for_spec_with_branch(&spec.id, spec_branch)
    }?;

    if commits.is_empty() && !allow_no_commits {
        anyhow::bail!("No commits found - agent did not make any changes");
    }

    Ok(commits)
}

/// Handle unchecked acceptance criteria based on policy
pub fn handle_acceptance_criteria(
    spec: &mut Spec,
    spec_path: &Path,
    skip_criteria: bool,
) -> Result<()> {
    let unchecked_count = spec.count_unchecked_checkboxes();
    if unchecked_count == 0 {
        return Ok(());
    }

    if skip_criteria {
        // Policy for chain/parallel: fail if unchecked
        anyhow::bail!("Spec has {} unchecked acceptance criteria", unchecked_count);
    } else {
        // Policy for single: auto-check
        eprintln!(
            "\n{} Found {} unchecked acceptance {}. Auto-checking...",
            "→".cyan(),
            unchecked_count,
            if unchecked_count == 1 {
                "criterion"
            } else {
                "criteria"
            }
        );

        let modified = spec.auto_check_acceptance_criteria();
        if modified {
            spec.save(spec_path)?;
            eprintln!(
                "{} Auto-checked {} acceptance {}",
                "✓".green(),
                unchecked_count,
                if unchecked_count == 1 {
                    "criterion"
                } else {
                    "criteria"
                }
            );
        }
    }

    Ok(())
}

/// Finalize a spec after successful agent execution
pub fn finalize_completed_spec(
    spec: &mut Spec,
    specs_dir: &Path,
    config: &Config,
    commits: Vec<String>,
    allow_no_commits: bool,
) -> Result<()> {
    eprintln!("\n{} Finalizing spec {}...", "→".cyan(), spec.id);

    let all_specs = spec::load_all_specs(specs_dir)?;
    let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
    let commits_to_pass = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };

    finalize_spec(
        spec,
        &spec_repo,
        config,
        &all_specs,
        allow_no_commits,
        commits_to_pass,
    )?;

    // Auto-complete driver if ready
    let all_specs = spec::load_all_specs(specs_dir)?;
    if spec::auto_complete_driver_if_ready(&spec.id, &all_specs, specs_dir)? {
        eprintln!(
            "\n{} Auto-completed driver spec: {}",
            "✓".green(),
            spec::extract_driver_id(&spec.id).unwrap()
        );
    }

    Ok(())
}

/// Append agent output and create transcript commit
pub fn cleanup_completed_spec(spec: &mut Spec, spec_path: &Path, agent_output: &str) -> Result<()> {
    append_agent_output(spec, agent_output);
    spec.save(spec_path)?;
    commit_transcript(&spec.id, spec_path)?;
    Ok(())
}

/// Handle spec failure: write agent status and set Failed status
pub fn handle_spec_failure(spec_id: &str, specs_dir: &Path, error: &anyhow::Error) -> Result<()> {
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    // Write agent status file: failed
    let status_path = specs_dir.join(format!(".chant-status-{}.json", spec_id));
    let agent_status = chant::worktree::status::AgentStatus {
        spec_id: spec_id.to_string(),
        status: chant::worktree::status::AgentStatusState::Failed,
        updated_at: chrono::Utc::now().to_rfc3339(),
        error: Some(error.to_string()),
        commits: vec![],
    };
    chant::worktree::status::write_status(&status_path, &agent_status)?;

    // Update spec to Failed
    let mut spec = spec::resolve_spec(specs_dir, spec_id)?;
    if let Err(transition_err) = spec.set_status(SpecStatus::Failed) {
        eprintln!(
            "{} Failed to transition spec to failed: {}",
            "⚠".yellow(),
            transition_err
        );
        let _ = spec::TransitionBuilder::new(&mut spec)
            .force()
            .to(SpecStatus::Failed);
    }
    spec.save(&spec_path)?;

    Ok(())
}

/// Write agent status file for completed work
pub fn write_agent_status_done(
    specs_dir: &Path,
    spec_id: &str,
    allow_no_commits: bool,
) -> Result<()> {
    let status_path = specs_dir.join(format!(".chant-status-{}.json", spec_id));
    let found_commits = (if allow_no_commits {
        get_commits_for_spec_allow_no_commits(spec_id)
    } else {
        get_commits_for_spec(spec_id)
    })
    .unwrap_or_default();

    let agent_status = chant::worktree::status::AgentStatus {
        spec_id: spec_id.to_string(),
        status: chant::worktree::status::AgentStatusState::Done,
        updated_at: chrono::Utc::now().to_rfc3339(),
        error: None,
        commits: found_commits,
    };
    chant::worktree::status::write_status(&status_path, &agent_status)?;
    Ok(())
}

/// Prepare spec for execution: validate, resolve prompt, update status
pub fn prepare_spec_for_execution(
    spec: &mut Spec,
    spec_path: &Path,
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    prompt_name: Option<&str>,
    validation_opts: &ValidationOptions,
) -> Result<String> {
    validate_spec(spec, specs_dir, config, validation_opts)?;

    // Resolve prompt
    let resolved_prompt_name = prompt_name
        .map(std::string::ToString::to_string)
        .or_else(|| spec.frontmatter.prompt.clone())
        .unwrap_or_else(|| config.defaults.prompt.clone());

    let prompt_path = prompts_dir.join(format!("{}.md", resolved_prompt_name));
    if !prompt_path.exists() {
        anyhow::bail!("Prompt not found: {}", resolved_prompt_name);
    }

    // Update status to in_progress
    spec.set_status(SpecStatus::InProgress)
        .map_err(|e| anyhow::anyhow!("Failed to transition spec to InProgress: {}", e))?;
    spec.save(spec_path)?;
    eprintln!("{} Set {} to InProgress", "→".cyan(), spec.id);

    // Create log file
    cmd::agent::create_log_file_if_not_exists(&spec.id, &resolved_prompt_name)?;

    // Write agent status: working
    let status_path = specs_dir.join(format!(".chant-status-{}.json", spec.id));
    let agent_status = chant::worktree::status::AgentStatus {
        spec_id: spec.id.clone(),
        status: chant::worktree::status::AgentStatusState::Working,
        updated_at: chrono::Utc::now().to_rfc3339(),
        error: None,
        commits: vec![],
    };
    chant::worktree::status::write_status(&status_path, &agent_status)?;

    // Mark driver as in_progress if needed (conditional for chain mode)
    spec::mark_driver_in_progress_conditional(specs_dir, &spec.id, true)?;

    Ok(resolved_prompt_name)
}

/// Validate agent output against schema if defined
pub fn validate_output_schema(
    spec: &Spec,
    agent_output: &str,
    config: &Config,
    spec_path: &Path,
) -> Result<()> {
    if let Some(ref schema_path_str) = spec.frontmatter.output_schema {
        let schema_path = Path::new(schema_path_str);
        if schema_path.exists() {
            match validation::validate_agent_output(&spec.id, schema_path, agent_output) {
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
                        if config.validation.strict_output_validation {
                            let mut spec = spec.clone();
                            let _ = spec::TransitionBuilder::new(&mut spec)
                                .force()
                                .to(SpecStatus::NeedsAttention);
                            spec.save(spec_path)?;
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
                        let mut spec = spec.clone();
                        let _ = spec::TransitionBuilder::new(&mut spec)
                            .force()
                            .to(SpecStatus::NeedsAttention);
                        spec.save(spec_path)?;
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
    Ok(())
}
