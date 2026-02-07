//! Shared execution functions for all work modes
//!
//! Provides unified validation, agent invocation, and finalization logic
//! that is used by single, chain, and parallel execution modes.

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use chant::config::Config;
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, Spec, SpecStatus};

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
    let lint_result = crate::cmd::spec::lint_specific_specs(specs_dir, &[spec.id.clone()])?;

    if lint_result.failed > 0 {
        anyhow::bail!(
            "Spec validation failed with {} error(s). Fix the issues before running 'chant work'.\n\
             Run 'chant lint {}' to see details.",
            lint_result.failed,
            spec.id
        );
    }

    if lint_result.warned > 0 {
        eprintln!(
            "{} Spec {} has {} warning(s) but is valid for execution",
            "⚠".yellow(),
            spec.id,
            lint_result.warned
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
    if !spec.is_ready(&all_specs) {
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
        cmd::agent::invoke_agent_with_model(&message, spec, prompt_name, config, None, worktree_path)
    }
}

/// Collect commits for a spec
pub fn collect_commits_for_spec(
    spec: &Spec,
    allow_no_commits: bool,
) -> Result<Vec<String>> {
    let spec_branch = spec.frontmatter.branch.as_deref();
    let commits = if allow_no_commits {
        cmd::commits::get_commits_for_spec_with_branch_allow_no_commits(&spec.id, spec_branch)
    } else {
        cmd::commits::get_commits_for_spec_with_branch(&spec.id, spec_branch)
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
pub fn cleanup_completed_spec(
    spec: &mut Spec,
    spec_path: &Path,
    agent_output: &str,
) -> Result<()> {
    append_agent_output(spec, agent_output);
    spec.save(spec_path)?;
    commit_transcript(&spec.id, spec_path)?;
    Ok(())
}

/// Handle spec failure: write agent status and set Failed status
pub fn handle_spec_failure(
    spec_id: &str,
    specs_dir: &Path,
    error: &anyhow::Error,
) -> Result<()> {
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
        spec.force_status(SpecStatus::Failed);
    }
    spec.save(&spec_path)?;

    Ok(())
}
