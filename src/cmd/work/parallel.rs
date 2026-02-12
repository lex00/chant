//! Parallel spec execution for chant CLI
//!
//! Handles concurrent execution of multiple specs using agent rotation
//! with thread pool management, worktree isolation, and cleanup handling.

use anyhow::Result;
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chant::config::Config;
use chant::conflict;
use chant::operations::get_commits_for_spec;
use chant::output::{Output, OutputMode};
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, Spec, SpecStatus};
use chant::worktree;

use super::executor;
use crate::cmd;
use crate::cmd::finalize::finalize_spec;

// ============================================================================
// PARALLEL EXECUTION TYPES
// ============================================================================

/// Result of a single spec execution in parallel mode
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
    #[allow(dead_code)]
    pub no_cleanup: bool,
    /// Force cleanup prompt even on success
    #[allow(dead_code)]
    pub force_cleanup: bool,
    /// Labels to filter specs
    pub labels: &'a [String],
    /// CLI branch prefix override
    pub branch_prefix: Option<&'a str>,
    /// Prompt name override
    pub prompt_name: Option<&'a str>,
    /// Specific spec IDs to run (if empty, runs all ready specs)
    pub specific_ids: &'a [String],
    /// Disable auto-merge after parallel execution
    pub no_merge: bool,
    /// Disable auto-rebase before merge in parallel execution
    pub no_rebase: bool,
}

/// Assignment of a spec to an agent
#[derive(Debug, Clone)]
struct AgentAssignment {
    spec_id: String,
    agent_name: String,
    agent_command: String,
}

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
pub enum PitfallType {
    ApiConcurrencyError,
    MergeConflict,
    PartialFailure,
    #[allow(dead_code)]
    UncommittedChanges,
    StaleWorktree,
    AgentError,
}

// ============================================================================
// MODEL OVERRIDE WARNING
// ============================================================================

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
        // Warn if agents are configured but rotation strategy is "none"
        if !config.parallel.agents.is_empty() && config.defaults.rotation_strategy == "none" {
            eprintln!(
                "{} parallel.agents configured but rotation_strategy is 'none' — agents will not be used in parallel mode. Set rotation_strategy: round-robin to enable.",
                "Warning:".yellow()
            );
        }
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

// ============================================================================
// SPEC PREPARATION
// ============================================================================

/// Prepare a spec for parallel execution: set status, create worktree, assemble prompt
#[allow(clippy::too_many_arguments)]
fn prepare_spec_for_parallel(
    spec: &Spec,
    spec_prompt: &str,
    prompt_path: &Path,
    specs_dir: &Path,
    _prompts_dir: &Path,
    config: &Config,
    execution_state: &Arc<ParallelExecutionState>,
    branch_prefix: &str,
) -> Result<(Spec, Option<PathBuf>, String, String)> {
    // Update spec status
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    let mut spec_clone = spec.clone();
    spec_clone.set_status(SpecStatus::InProgress)?;
    spec_clone.save(&spec_path)?;

    // Create lock file to signal agent is running
    let lock_path = PathBuf::from(chant::paths::LOCKS_DIR).join(format!("{}.lock", spec.id));
    std::fs::create_dir_all(chant::paths::LOCKS_DIR)?;
    std::fs::write(&lock_path, format!("{}", std::process::id()))?;

    // Create log file and agent status
    cmd::agent::create_log_file_if_not_exists(&spec.id, spec_prompt)?;
    let status_path = specs_dir.join(format!(".chant-status-{}.json", spec.id));
    let agent_status = chant::worktree::status::AgentStatus {
        spec_id: spec.id.clone(),
        status: chant::worktree::status::AgentStatusState::Working,
        updated_at: chrono::Utc::now().to_rfc3339(),
        error: None,
        commits: vec![],
    };
    chant::worktree::status::write_status(&status_path, &agent_status)?;

    // Determine branch name and create worktree
    let branch_name = format!("{}{}", branch_prefix, spec.id);
    let project_name = Some(config.project.name.as_str()).filter(|n| !n.is_empty());

    // Create worktree with error logging
    let worktree_path = match worktree::create_worktree(&spec.id, &branch_name, project_name) {
        Ok(path) => path,
        Err(e) => {
            // Log the error to the spec log file
            let log_msg = format!(
                "[{}] ERROR: Failed to create worktree: {}\n",
                chrono::Utc::now().to_rfc3339(),
                e
            );
            if let Err(log_err) = cmd::agent::append_to_log(&spec.id, &log_msg) {
                eprintln!(
                    "⚠️  [{}] Failed to write error to log: {}",
                    spec.id, log_err
                );
            }
            return Err(e);
        }
    };

    execution_state.register_worktree(&spec.id, worktree_path.clone());
    worktree::copy_spec_to_worktree(&spec.id, &worktree_path)?;
    worktree::isolate_worktree_specs(&spec.id, &worktree_path)?;

    // Assemble prompt with worktree context
    let worktree_ctx = chant::prompt::WorktreeContext {
        worktree_path: Some(worktree_path.clone()),
        branch_name: Some(branch_name.clone()),
        is_isolated: true,
    };
    let message =
        chant::prompt::assemble_with_context(&spec_clone, prompt_path, config, &worktree_ctx)?;

    Ok((spec_clone, Some(worktree_path), branch_name, message))
}

// ============================================================================
// AGENT DISTRIBUTION
// ============================================================================

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

// ============================================================================
// PARALLEL EXECUTION CLEANUP STATE
// ============================================================================

/// Tracks active worktrees during parallel execution for cleanup on interrupt.
struct ParallelExecutionState {
    /// Worktrees created this run, keyed by spec_id
    active_worktrees: Arc<Mutex<HashMap<String, PathBuf>>>,
    /// Specs that completed agent work (preserve their branches)
    completed_specs: Arc<Mutex<HashSet<String>>>,
    /// Branch prefix for cleanup (e.g. "chant/")
    branch_prefix: String,
}

impl ParallelExecutionState {
    fn new(branch_prefix: &str) -> Self {
        Self {
            active_worktrees: Arc::new(Mutex::new(HashMap::new())),
            completed_specs: Arc::new(Mutex::new(HashSet::new())),
            branch_prefix: branch_prefix.to_string(),
        }
    }

    fn register_worktree(&self, spec_id: &str, path: PathBuf) {
        if let Ok(mut worktrees) = self.active_worktrees.lock() {
            worktrees.insert(spec_id.to_string(), path);
        }
    }

    fn mark_completed(&self, spec_id: &str) {
        if let Ok(mut completed) = self.completed_specs.lock() {
            completed.insert(spec_id.to_string());
        }
    }

    fn cleanup_incomplete(&self) {
        let active = match self.active_worktrees.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let completed = match self.completed_specs.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        for (spec_id, path) in active.iter() {
            if !completed.contains(spec_id) {
                eprintln!(
                    "\n{} Cleaning up incomplete worktree for spec {}: {}",
                    "→".yellow(),
                    spec_id.cyan(),
                    path.display()
                );

                // Remove worktree
                if let Err(e) = worktree::remove_worktree(path) {
                    eprintln!("{} Failed to remove worktree: {}", "⚠".yellow(), e);
                }

                // Delete branch since work didn't complete
                let branch = format!("{}{}", self.branch_prefix, spec_id);
                if let Err(e) = chant::git::delete_branch(&branch, false) {
                    eprintln!("{} Failed to delete branch {}: {}", "⚠".yellow(), branch, e);
                }
            }
        }
    }
}

/// Set up SIGINT handler for parallel execution cleanup.
fn setup_parallel_cleanup_handlers(state: Arc<ParallelExecutionState>) {
    // SIGINT handler
    let state_clone = state.clone();
    let _ = ctrlc::set_handler(move || {
        eprintln!(
            "\n{} Interrupt received, cleaning up incomplete worktrees...",
            "→".yellow()
        );
        state_clone.cleanup_incomplete();
        eprintln!("{} Cleanup complete, exiting", "✓".green());
        std::process::exit(130);
    });

    // Panic hook for crashes
    let state_clone = state.clone();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        state_clone.cleanup_incomplete();
        default_hook(info);
    }));
}

// ============================================================================
// MERGE PHASE HANDLERS
// ============================================================================

/// Handle direct mode merges
fn handle_direct_mode_merges(
    results: &[ParallelResult],
    specs_dir: &Path,
    config: &Config,
    no_rebase: bool,
) -> Result<(usize, Vec<(String, bool)>)> {
    let mut merged_count = 0;
    let mut merge_failed = Vec::new();

    for result in results {
        if let Some(ref branch) = result.branch_name {
            println!("[{}] Merging to main...", result.spec_id.cyan());
            let merge_result =
                worktree::merge_and_cleanup(branch, &config.defaults.main_branch, no_rebase);

            if merge_result.success {
                merged_count += 1;
                println!("[{}] {} Merged to main", result.spec_id.cyan(), "✓".green());
                if let Some(ref path) = result.worktree_path {
                    let _ = worktree::remove_worktree(path);
                }
            } else {
                merge_failed.push((result.spec_id.clone(), merge_result.has_conflict));
                let spec_path = specs_dir.join(format!("{}.md", result.spec_id));
                if let Ok(mut spec) = spec::resolve_spec(specs_dir, &result.spec_id) {
                    let _ = spec::TransitionBuilder::new(&mut spec)
                        .force()
                        .to(SpecStatus::NeedsAttention);
                    let _ = spec.save(&spec_path);
                }

                let error_msg = merge_result
                    .error
                    .as_deref()
                    .unwrap_or("Unknown merge error");
                println!(
                    "[{}] {} Merge failed (branch preserved):\n  {}\n  Next Steps:\n    1. Auto-resolve: chant merge {} --rebase --auto\n    2. Merge manually: chant merge {}\n    3. Inspect: git log {} --oneline -3",
                    result.spec_id.cyan(),
                    "⚠".yellow(),
                    error_msg,
                    result.spec_id,
                    result.spec_id,
                    branch
                );

                if merge_result.has_conflict {
                    handle_merge_conflict(specs_dir, &result.spec_id, branch)?;
                }
            }
        }
    }

    Ok((merged_count, merge_failed))
}

/// Result type for branch mode merge operations
type BranchMergeResult = (usize, Vec<(String, bool)>, Vec<(String, String)>);

/// Handle branch mode merges
fn handle_branch_mode_merges(
    branches: &[(String, String)],
    specs_dir: &Path,
    config: &Config,
    no_merge: bool,
    no_rebase: bool,
) -> Result<BranchMergeResult> {
    let mut merged_count = 0;
    let mut failed = Vec::new();
    let skipped = Vec::new();

    if no_merge {
        return Ok((0, vec![], branches.to_vec()));
    }

    if branches.is_empty() {
        return Ok((0, vec![], vec![]));
    }

    println!(
        "\n{} Auto-merging {} branch mode branch(es)...",
        "→".cyan(),
        branches.len()
    );

    for (spec_id, branch) in branches {
        println!("[{}] Merging to main...", spec_id.cyan());
        let merge_result =
            worktree::merge_and_cleanup(branch, &config.defaults.main_branch, no_rebase);

        if merge_result.success {
            println!(
                "[{}] Merge succeeded, finalizing on main...",
                spec_id.cyan()
            );

            let finalize_result = if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
                let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
                let commits = get_commits_for_spec(spec_id).ok();
                let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
                finalize_spec(&mut spec, &spec_repo, config, &all_specs, false, commits)
            } else {
                Err(anyhow::anyhow!("Failed to load spec for finalization"))
            };

            match finalize_result {
                Ok(()) => {
                    merged_count += 1;
                    println!("[{}] {} Merged and finalized", spec_id.cyan(), "✓".green());
                }
                Err(e) => {
                    eprintln!(
                        "[{}] {} Merged but finalization failed: {}",
                        spec_id.cyan(),
                        "⚠".yellow(),
                        e
                    );
                    let spec_path = specs_dir.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
                        let _ = spec::TransitionBuilder::new(&mut spec)
                            .force()
                            .to(SpecStatus::Failed);
                        let _ = spec.save(&spec_path);
                    }
                    failed.push((spec_id.clone(), false));
                }
            }
        } else {
            failed.push((spec_id.clone(), merge_result.has_conflict));
            let error_msg = merge_result
                .error
                .as_deref()
                .unwrap_or("Unknown merge error");
            println!(
                "[{}] {} Merge failed (branch preserved):\n  {}\n  Next Steps:\n    1. Auto-resolve: chant merge {} --rebase --auto\n    2. Merge manually: chant merge {}\n    3. Inspect: git log {} --oneline -3",
                spec_id.cyan(),
                "⚠".yellow(),
                error_msg,
                spec_id,
                spec_id,
                branch
            );

            if merge_result.has_conflict {
                handle_merge_conflict(specs_dir, spec_id, branch)?;
            }
        }
    }

    Ok((merged_count, failed, skipped))
}

/// Handle merge conflict by creating conflict resolution spec
fn handle_merge_conflict(specs_dir: &Path, spec_id: &str, branch: &str) -> Result<()> {
    if let Ok(conflicting_files) = conflict::detect_conflicting_files() {
        let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
        let blocked_specs = conflict::get_blocked_specs(&conflicting_files, &all_specs);
        let (spec_title, _) =
            conflict::extract_spec_context(specs_dir, spec_id).unwrap_or((None, String::new()));
        let diff_summary = conflict::get_diff_summary(branch, "main").unwrap_or_default();

        let context = conflict::ConflictContext {
            source_branch: branch.to_string(),
            target_branch: "main".to_string(),
            conflicting_files,
            source_spec_id: spec_id.to_string(),
            source_spec_title: spec_title,
            diff_summary,
        };

        if let Ok(conflict_spec_id) =
            conflict::create_conflict_spec(specs_dir, &context, blocked_specs)
        {
            println!(
                "[{}] Created conflict resolution spec: {}",
                spec_id.cyan(),
                conflict_spec_id
            );
        }
    }
    Ok(())
}

// ============================================================================
// WORKER THREAD SPAWNING
// ============================================================================

/// Spawn worker threads for all spec assignments
#[allow(clippy::type_complexity)]
fn spawn_worker_threads(
    assignments: &[AgentAssignment],
    ready_specs: &[Spec],
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: &ParallelOptions,
    execution_state: &Arc<ParallelExecutionState>,
) -> Result<(
    mpsc::Sender<ParallelResult>,
    mpsc::Receiver<ParallelResult>,
    Vec<thread::JoinHandle<()>>,
    ProgressBar,
)> {
    let multi_progress = Arc::new(MultiProgress::new());
    let main_pb = multi_progress.add(ProgressBar::new(assignments.len() as u64));
    main_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} specs completed")
            .unwrap()
            .progress_chars("=>-"),
    );

    let default_prompt = &config.defaults.prompt;
    let (tx, rx) = mpsc::channel::<ParallelResult>();
    let mut handles = Vec::new();
    let spec_map: HashMap<&str, &Spec> = ready_specs.iter().map(|s| (s.id.as_str(), s)).collect();

    for assignment in assignments.iter() {
        let spec = match spec_map.get(assignment.spec_id.as_str()) {
            Some(s) => *s,
            None => continue,
        };

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

        let (is_direct_mode, branch_prefix) = if let Some(cli_prefix) = options.branch_prefix {
            (false, cli_prefix.to_string())
        } else if let Some(spec_branch) = &spec.frontmatter.branch {
            (false, spec_branch.clone())
        } else {
            (false, config.defaults.branch_prefix.clone())
        };

        let (_spec_clone, worktree_path, branch_name, message) = match prepare_spec_for_parallel(
            spec,
            spec_prompt,
            &prompt_path,
            specs_dir,
            prompts_dir,
            config,
            execution_state,
            &branch_prefix,
        ) {
            Ok(result) => result,
            Err(e) => {
                println!("{} [{}] Failed to prepare spec: {}", "✗".red(), spec.id, e);

                // Mark spec as failed in the filesystem
                let spec_path = specs_dir.join(format!("{}.md", spec.id));
                if let Ok(failed_spec) = spec::resolve_spec(specs_dir, &spec.id) {
                    let mut failed_spec = failed_spec;
                    let _ = spec::TransitionBuilder::new(&mut failed_spec)
                        .force()
                        .to(SpecStatus::Failed);
                    let _ = failed_spec.save(&spec_path);
                }

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

        println!(
            "[{}] Working with prompt '{}' via {}",
            spec.id.cyan(),
            spec_prompt,
            assignment.agent_name.dimmed()
        );

        let handle = thread::spawn({
            let tx = tx.clone();
            let spec_id = spec.id.clone();
            let specs_dir = specs_dir.to_path_buf();
            let prompt_name = spec_prompt.to_string();
            let config_model = config.defaults.model.clone();
            let agent_command = assignment.agent_command.clone();
            let config = config.clone();
            let execution_state = execution_state.clone();

            move || {
                execute_spec_in_thread(
                    spec_id,
                    message,
                    prompt_name,
                    config_model,
                    worktree_path,
                    Some(branch_name),
                    is_direct_mode,
                    agent_command,
                    specs_dir,
                    config,
                    execution_state,
                    tx,
                );
            }
        });

        handles.push(handle);

        // Stagger thread spawning
        if config.parallel.stagger_delay_ms > 0 {
            let mut rng = rand::thread_rng();
            let jitter = if config.parallel.stagger_jitter_ms > 0 {
                rng.gen_range(
                    -(config.parallel.stagger_jitter_ms as i64)
                        ..=(config.parallel.stagger_jitter_ms as i64),
                )
            } else {
                0
            };
            let delay_ms = (config.parallel.stagger_delay_ms as i64 + jitter).max(0) as u64;
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    Ok((tx, rx, handles, main_pb))
}

// ============================================================================
// PUBLIC API - MAIN PARALLEL EXECUTION FUNCTION
// ============================================================================

/// Helper function to auto-select prompt based on spec type
fn auto_select_prompt_for_type(_spec: &Spec, _prompts_dir: &Path) -> Option<String> {
    // This function is defined in the original work.rs
    // For now, we'll return None to use default
    None
}

/// Check if a spec is a driver/group spec
fn is_driver_spec(spec: &Spec, all_specs: &[Spec]) -> bool {
    spec.frontmatter.r#type == "group"
        || spec.frontmatter.r#type == "driver"
        || !chant::spec_group::get_members(&spec.id, all_specs).is_empty()
}

/// Check if a spec's group (if any) has all upstream driver dependencies satisfied
fn group_upstream_deps_satisfied(spec: &Spec, all_specs: &[Spec]) -> bool {
    // Standalone specs have no group dependencies
    let Some(driver_id) = chant::spec_group::extract_driver_id(&spec.id) else {
        return true;
    };

    // Find the driver spec for this member
    let Some(driver) = all_specs.iter().find(|s| s.id == driver_id) else {
        return true; // No driver found, allow execution
    };

    // Check if all of the driver's dependencies are completed
    let Some(deps) = &driver.frontmatter.depends_on else {
        return true; // No dependencies, always satisfied
    };

    deps.iter().all(|dep_id| {
        all_specs
            .iter()
            .find(|s| &s.id == dep_id)
            .map(|dep| dep.frontmatter.status == SpecStatus::Completed)
            .unwrap_or(true) // If dependency doesn't exist, don't block
    })
}

/// Get ready specs for parallel execution, excluding drivers and filtering by group readiness
fn get_ready_specs_for_parallel(
    specs_dir: &Path,
    specific_ids: &[String],
    labels: &[String],
) -> Result<Vec<Spec>> {
    let all_specs = spec::load_all_specs(specs_dir)?;

    let ready_specs: Vec<Spec> = if !specific_ids.is_empty() {
        // Resolve specific IDs and validate
        let mut specs = Vec::new();
        for id in specific_ids {
            match spec::resolve_spec(specs_dir, id) {
                Ok(s) => {
                    if s.frontmatter.status != SpecStatus::Pending {
                        println!(
                            "{} Spec '{}' has status {:?}, expected pending",
                            "✗".red(),
                            s.id,
                            s.frontmatter.status
                        );
                        return Err(anyhow::anyhow!("Spec '{}' is not in pending status", s.id));
                    }
                    if !s.is_ready(&all_specs) {
                        println!(
                            "{} Spec '{}' is not ready (has unmet dependencies)",
                            "✗".red(),
                            s.id
                        );
                        return Err(anyhow::anyhow!("Spec '{}' has unmet dependencies", s.id));
                    }
                    // Exclude driver specs
                    if is_driver_spec(&s, &all_specs) {
                        println!(
                            "{} Spec '{}' is a driver spec (will execute members instead)",
                            "→".yellow(),
                            s.id
                        );
                        continue;
                    }
                    specs.push(s);
                }
                Err(e) => {
                    println!("{} Failed to resolve spec '{}': {}", "✗".red(), id, e);
                    return Err(e);
                }
            }
        }
        specs
    } else {
        // Load all ready specs, excluding drivers, and checking group dependencies
        let mut specs: Vec<Spec> = all_specs
            .iter()
            .filter(|s| {
                s.is_ready(&all_specs)
                    && !is_driver_spec(s, &all_specs)
                    && group_upstream_deps_satisfied(s, &all_specs)
            })
            .cloned()
            .collect();

        // Filter by labels if specified
        if !labels.is_empty() {
            specs.retain(|s| {
                if let Some(spec_labels) = &s.frontmatter.labels {
                    labels.iter().any(|l| spec_labels.contains(l))
                } else {
                    false
                }
            });
        }

        specs
    };

    Ok(ready_specs)
}

pub fn cmd_work_parallel(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: ParallelOptions,
) -> Result<()> {
    // Initialize parallel execution state for cleanup on interrupt
    let execution_state = Arc::new(ParallelExecutionState::new(&config.defaults.branch_prefix));
    setup_parallel_cleanup_handlers(execution_state.clone());

    // Create output handler
    let out = Output::new(OutputMode::Human);

    // Load ready specs (group-aware, excluding drivers)
    let ready_specs =
        get_ready_specs_for_parallel(specs_dir, options.specific_ids, options.labels)?;

    if ready_specs.is_empty() {
        if !options.specific_ids.is_empty() {
            out.info("No specs resolved from provided IDs.");
        } else if !options.labels.is_empty() {
            out.info("No ready specs with specified labels.");
        } else {
            out.info("No ready specs to execute.");
        }
        return Ok(());
    }

    // Run validation on all specs before starting parallel work - fail fast if any have issues
    out.step(&format!("Validating {} spec(s)...", ready_specs.len()));

    let validation_opts = executor::ValidationOptions {
        skip_deps: false,
        skip_criteria: false,
        skip_approval: false,
        skip_quality: true, // Skip quality checks in parallel mode
    };

    for spec in &ready_specs {
        if let Err(e) = executor::validate_spec(spec, specs_dir, config, &validation_opts) {
            anyhow::bail!(
                "Spec {} validation failed: {}. Fix the issues before running 'chant work --parallel'.",
                spec.id,
                e
            );
        }
    }

    // Distribute specs across configured agents
    let assignments = distribute_specs_to_agents(&ready_specs, config, options.max_override);

    if assignments.len() < ready_specs.len() {
        out.warn(&format!(
            "Only {} of {} ready specs will be executed (capacity limit)",
            assignments.len(),
            ready_specs.len()
        ));
    }

    // Warn if user has set model preferences that will be ignored by agent CLI profiles
    warn_model_override_in_parallel(config, options.prompt_name);

    // Show agent distribution
    out.step(&format!(
        "Starting {} specs in parallel...\n",
        assignments.len()
    ));

    // Group assignments by agent for display
    let mut agent_counts: HashMap<&str, usize> = HashMap::new();
    for assignment in &assignments {
        *agent_counts.entry(&assignment.agent_name).or_insert(0) += 1;
    }
    for (agent_name, count) in &agent_counts {
        println!("  {} {}: {} specs", "•".dimmed(), agent_name, count);
    }
    println!();

    // Spawn worker threads for all assignments
    let (tx, rx, handles, main_pb) = spawn_worker_threads(
        &assignments,
        &ready_specs,
        specs_dir,
        prompts_dir,
        config,
        &options,
        &execution_state,
    )?;
    drop(tx); // Signal completion

    // Collect results from threads with group-aware progress reporting
    let mut completed = 0;
    let mut failed = 0;
    let mut all_results = Vec::new();
    let mut branch_mode_branches = Vec::new();
    let mut direct_mode_results = Vec::new();

    // Track group progress for reporting
    let mut group_stats: HashMap<String, (usize, usize, usize)> = HashMap::new(); // driver_id -> (completed, failed, total)

    // Pre-compute group memberships and totals
    let current_specs = spec::load_all_specs(specs_dir)?;
    let mut group_totals: HashMap<String, usize> = HashMap::new();
    for spec in &ready_specs {
        if let Some(driver_id) = chant::spec_group::extract_driver_id(&spec.id) {
            *group_totals.entry(driver_id.clone()).or_insert(0) += 1;
        }
    }

    // Initialize group stats
    for (driver_id, total) in &group_totals {
        group_stats.insert(driver_id.clone(), (0, 0, *total));
    }

    // Compute group order for indexing
    let group_order: Vec<String> = {
        let driver_specs: Vec<&Spec> = current_specs
            .iter()
            .filter(|s| {
                s.frontmatter.r#type == "group"
                    || s.frontmatter.r#type == "driver"
                    || !chant::spec_group::get_members(&s.id, &current_specs).is_empty()
            })
            .collect();
        driver_specs.iter().map(|s| s.id.clone()).collect()
    };

    for result in rx {
        main_pb.inc(1);

        // Update group stats if this is a member spec
        let group_context =
            if let Some(driver_id) = chant::spec_group::extract_driver_id(&result.spec_id) {
                if let Some(stats) = group_stats.get_mut(&driver_id) {
                    if result.success {
                        stats.0 += 1; // completed
                    } else {
                        stats.1 += 1; // failed
                    }

                    let group_index = group_order
                        .iter()
                        .position(|g| g == &driver_id)
                        .unwrap_or(0)
                        + 1;
                    let total_groups = group_order.len();
                    let completed_in_group = stats.0;
                    let failed_in_group = stats.1;
                    let total_in_group = stats.2;

                    Some((
                        group_index,
                        total_groups,
                        completed_in_group + failed_in_group,
                        total_in_group,
                    ))
                } else {
                    None
                }
            } else {
                None
            };

        if result.success {
            completed += 1;

            // Print with group context
            let prefix =
                if let Some((group_idx, total_groups, member_idx, total_members)) = group_context {
                    format!(
                        "[group {}/{}] [member {}/{}]",
                        group_idx, total_groups, member_idx, total_members
                    )
                } else {
                    "[standalone]".to_string()
                };

            if let Some(ref commits) = result.commits {
                let commits_str = commits.join(", ");
                main_pb.println(format!(
                    "{} [{}] {} Completed (commits: {})",
                    prefix,
                    result.spec_id.cyan(),
                    "✓".green(),
                    commits_str
                ));
            } else {
                main_pb.println(format!(
                    "{} [{}] {} Completed",
                    prefix,
                    result.spec_id.cyan(),
                    "✓".green()
                ));
            }

            // Print running tally
            main_pb.println(format!(
                "  {} [{}/{}] completed, {} failed, {} remaining",
                "→".dimmed(),
                completed,
                assignments.len(),
                failed,
                assignments.len().saturating_sub(completed + failed)
            ));

            // Collect branch info
            if result.is_direct_mode {
                direct_mode_results.push(result.clone());
            } else if let Some(ref branch) = result.branch_name {
                branch_mode_branches.push((result.spec_id.clone(), branch.clone()));
            }
        } else {
            failed += 1;
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");

            let prefix =
                if let Some((group_idx, total_groups, member_idx, total_members)) = group_context {
                    format!(
                        "[group {}/{}] [member {}/{}]",
                        group_idx, total_groups, member_idx, total_members
                    )
                } else {
                    "[standalone]".to_string()
                };

            main_pb.println(format!(
                "{} [{}] {} Failed: {}",
                prefix,
                result.spec_id.cyan(),
                "✗".red(),
                error_msg
            ));
        }
        all_results.push(result);
    }

    // Print group completion summaries
    for (driver_id, (completed_members, failed_members, total_members)) in &group_stats {
        if *completed_members > 0 || *failed_members > 0 {
            if *failed_members == 0 && completed_members == total_members {
                main_pb.println(format!(
                    "{} Group {} completed ({}/{} members)",
                    "✓".green(),
                    driver_id,
                    completed_members,
                    total_members
                ));
            } else if *failed_members > 0 {
                let skipped_members = total_members - completed_members - failed_members;
                main_pb.println(format!(
                    "{} Group {} partially failed: {} completed, {} failed, {} skipped",
                    "⚠".yellow(),
                    driver_id,
                    completed_members,
                    failed_members,
                    skipped_members
                ));
            }
        }
    }

    // Finish progress bar
    main_pb.finish_and_clear();

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }

    // =========================================================================
    // SERIALIZED MERGE PHASE - Handle all direct mode merges sequentially
    // =========================================================================

    let (merged_count, merge_failed) =
        handle_direct_mode_merges(&direct_mode_results, specs_dir, config, options.no_rebase)?;

    // =========================================================================
    // BRANCH MODE MERGE PHASE - Auto-merge branch mode branches unless --no-merge
    // =========================================================================

    let (branch_mode_merged, branch_mode_failed, branch_mode_skipped) = handle_branch_mode_merges(
        &branch_mode_branches,
        specs_dir,
        config,
        options.no_merge,
        options.no_rebase,
    )?;

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

    // Detect issues and print summary
    let pitfalls = detect_parallel_pitfalls(&all_results, specs_dir);
    print_parallel_summary(
        completed,
        failed,
        &direct_mode_results,
        merged_count,
        &merge_failed,
        branch_mode_merged,
        &branch_mode_failed,
        &branch_mode_skipped,
        &pitfalls,
    );

    if failed > 0 {
        std::process::exit(1);
    }

    // Ensure main repo is back on main branch after merge phase
    let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

    Ok(())
}

// ============================================================================
// WORKER THREAD EXECUTION
// ============================================================================

/// Execute a single spec in a worker thread
#[allow(clippy::too_many_arguments)]
fn execute_spec_in_thread(
    spec_id: String,
    message: String,
    prompt_name: String,
    config_model: Option<String>,
    worktree_path: Option<PathBuf>,
    branch_name: Option<String>,
    is_direct_mode: bool,
    agent_command: String,
    specs_dir: PathBuf,
    config: Config,
    execution_state: Arc<ParallelExecutionState>,
    tx: mpsc::Sender<ParallelResult>,
) {
    let result = cmd::agent::invoke_agent_with_command(
        &message,
        &spec_id,
        &prompt_name,
        config_model.as_deref(),
        worktree_path.as_deref(),
        &agent_command,
        branch_name.as_deref(),
    );

    // Remove lock file after agent completes (both success and failure)
    let lock_path = PathBuf::from(chant::paths::LOCKS_DIR).join(format!("{}.lock", spec_id));
    let _ = std::fs::remove_file(&lock_path);

    let (success, commits, error, agent_completed) = match result {
        Ok(_) => {
            // Write status and get commits
            if let Err(e) = executor::write_agent_status_done(&specs_dir, &spec_id, false) {
                eprintln!(
                    "{} [{}] Failed to write agent status: {}",
                    "⚠".yellow(),
                    spec_id,
                    e
                );
            }

            let commits = get_commits_for_spec(&spec_id).ok();
            execution_state.mark_completed(&spec_id);

            if !is_direct_mode {
                // Branch mode: defer finalization to post-merge
                eprintln!(
                    "{} [{}] Agent work completed, deferring finalization to post-merge",
                    "→".cyan(),
                    spec_id
                );

                if let Some(ref path) = worktree_path {
                    if let Err(e) = worktree::remove_worktree(path) {
                        eprintln!(
                            "{} [{}] Warning: Failed to remove worktree: {}",
                            "⚠".yellow(),
                            spec_id,
                            e
                        );
                    }
                }

                (true, commits, None, true)
            } else {
                // Direct mode: finalize immediately
                eprintln!(
                    "{} [{}] Finalizing spec on main branch (direct mode)",
                    "→".cyan(),
                    spec_id
                );

                let finalize_result = if let Ok(mut spec) = spec::resolve_spec(&specs_dir, &spec_id)
                {
                    let all_specs = spec::load_all_specs(&specs_dir).unwrap_or_default();
                    let spec_repo = FileSpecRepository::new(specs_dir.clone());
                    finalize_spec(
                        &mut spec,
                        &spec_repo,
                        &config,
                        &all_specs,
                        false,
                        commits.clone(),
                    )
                } else {
                    Err(anyhow::anyhow!("Failed to load spec for finalization"))
                };

                match finalize_result {
                    Ok(()) => {
                        eprintln!("{} [{}] ✓ Finalization complete", "✓".green(), spec_id);
                        (true, commits, None, true)
                    }
                    Err(e) => {
                        eprintln!("{} [{}] ✗ Cannot finalize spec: {}", "✗".red(), spec_id, e);
                        if let Ok(mut spec) = spec::resolve_spec(&specs_dir, &spec_id) {
                            let _ = spec::TransitionBuilder::new(&mut spec)
                                .force()
                                .to(SpecStatus::Failed);
                            let _ = spec.save(&specs_dir.join(format!("{}.md", spec_id)));
                        }
                        (false, commits, Some(e.to_string()), false)
                    }
                }
            }
        }
        Err(e) => {
            let _ = executor::handle_spec_failure(&spec_id, &specs_dir, &e);
            if let Some(ref path) = worktree_path {
                if !is_direct_mode {
                    let _ = worktree::remove_worktree(path);
                }
            }
            (false, None, Some(e.to_string()), false)
        }
    };

    let _ = tx.send(ParallelResult {
        spec_id,
        success,
        commits,
        error,
        worktree_path,
        branch_name,
        is_direct_mode,
        agent_completed,
    });
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
            "\nManually cleanup orphan worktrees with: {}",
            "git worktree prune".bold()
        );
    }
}

// ============================================================================
// RESULT REPORTING
// ============================================================================

/// Print final summary of parallel execution
#[allow(clippy::too_many_arguments)]
fn print_parallel_summary(
    completed: usize,
    failed: usize,
    direct_mode_results: &[ParallelResult],
    merged_count: usize,
    merge_failed: &[(String, bool)],
    branch_mode_merged: usize,
    branch_mode_failed: &[(String, bool)],
    branch_mode_skipped: &[(String, String)],
    pitfalls: &[Pitfall],
) {
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
            for (spec_id, has_conflict) in merge_failed {
                let indicator = if *has_conflict { "⚡" } else { "→" };
                println!("    {} {}", indicator.yellow(), spec_id);
            }
        }
    }

    if branch_mode_merged > 0 {
        println!(
            "  {} {} branch mode specs merged to main",
            "✓".green(),
            branch_mode_merged
        );
    }

    if !branch_mode_failed.is_empty() {
        println!(
            "  {} {} branch mode specs need attention (merge failed)",
            "⚠".yellow(),
            branch_mode_failed.len()
        );
        for (spec_id, has_conflict) in branch_mode_failed {
            let indicator = if *has_conflict { "⚡" } else { "→" };
            println!("    {} {}", indicator.yellow(), spec_id);
        }
    }

    if !branch_mode_skipped.is_empty() {
        println!(
            "  {} {} branches preserved (--no-merge)",
            "→".cyan(),
            branch_mode_skipped.len()
        );
    }

    if failed > 0 {
        println!("  {} {} specs failed", "✗".red(), failed);
    }
    println!("{}", "═".repeat(60).dimmed());

    if !branch_mode_skipped.is_empty() {
        println!(
            "\n{} Branch mode branches preserved for manual merging:",
            "→".cyan()
        );
        for (_spec_id, branch) in branch_mode_skipped {
            println!("  {} {}", "•".yellow(), branch);
        }
        println!("\nUse {} to merge branches later.", "chant merge".bold());
    }

    let all_merge_failed = !merge_failed.is_empty() || !branch_mode_failed.is_empty();
    if all_merge_failed {
        println!("\n{} Next steps for merge-pending branches:", "→".cyan());
        println!("  1. Review each branch:  git log <branch> --oneline -5");
        println!("  2. Auto-resolve conflicts:  chant merge --all --rebase --auto");
        println!("  3. Or merge sequentially:  chant merge <spec-id>");
        println!("  4. List worktrees:  git worktree list");
        println!("\n  Documentation: See 'chant merge --help' for more options");
    }

    if !pitfalls.is_empty() {
        println!("\n{} Issues detected:", "→".yellow());
        for pitfall in pitfalls {
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
    }
}

// ============================================================================
// PARALLEL PITFALL DETECTION
// ============================================================================

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
