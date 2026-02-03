//! Parallel spec execution for chant CLI
//!
//! Handles concurrent execution of multiple specs using agent rotation
//! with thread pool management, worktree isolation, and cleanup handling.

use anyhow::Result;
use colored::Colorize;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chant::config::Config;
use chant::conflict;
use chant::spec::{self, Spec, SpecStatus};
use chant::worktree;

use crate::cmd;
use crate::cmd::commits::get_commits_for_spec;
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
}

impl ParallelExecutionState {
    fn new() -> Self {
        Self {
            active_worktrees: Arc::new(Mutex::new(HashMap::new())),
            completed_specs: Arc::new(Mutex::new(HashSet::new())),
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
                let branch = format!("chant/{}", spec_id);
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
// PUBLIC API - MAIN PARALLEL EXECUTION FUNCTION
// ============================================================================

/// Helper function to auto-select prompt based on spec type
fn auto_select_prompt_for_type(_spec: &Spec, _prompts_dir: &Path) -> Option<String> {
    // This function is defined in the original work.rs
    // For now, we'll return None to use default
    None
}

pub fn cmd_work_parallel(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: ParallelOptions,
) -> Result<()> {
    // Initialize parallel execution state for cleanup on interrupt
    let execution_state = Arc::new(ParallelExecutionState::new());
    setup_parallel_cleanup_handlers(execution_state.clone());

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
    let mut agent_counts: HashMap<&str, usize> = HashMap::new();
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
    let spec_map: HashMap<&str, &Spec> = ready_specs.iter().map(|s| (s.id.as_str(), s)).collect();

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

        // Determine branch mode
        // Priority: CLI --branch flag > spec frontmatter.branch
        // IMPORTANT: Parallel execution forces branch mode internally for isolation
        let (is_direct_mode, branch_prefix) = if let Some(cli_prefix) = options.branch_prefix {
            // CLI --branch specified with explicit prefix
            (false, cli_prefix.to_string())
        } else if let Some(spec_branch) = &spec.frontmatter.branch {
            // Spec has explicit branch prefix
            (false, spec_branch.clone())
        } else {
            // Parallel execution forces branch mode
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
            Ok(path) => {
                // Register worktree for cleanup on interrupt
                execution_state.register_worktree(&spec.id, path.clone());

                // Copy the updated spec file to the worktree
                if let Err(e) = worktree::copy_spec_to_worktree(&spec.id, &path) {
                    println!(
                        "{} [{}] Failed to copy spec to worktree: {}",
                        "✗".red(),
                        spec.id,
                        e
                    );
                    // Clean up worktree since we failed
                    let _ = worktree::remove_worktree(&path);
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

                (Some(path), Some(branch_name.clone()))
            }
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

        // Assemble the prompt message with worktree context
        // Now that we know the worktree path and branch, we can provide this context to the agent
        let worktree_ctx = chant::prompt::WorktreeContext {
            worktree_path: worktree_path.clone(),
            branch_name: Some(branch_name.clone()),
            is_isolated: true, // Parallel execution always uses isolated worktrees
        };
        let message = match chant::prompt::assemble_with_context(
            &spec_clone,
            &prompt_path,
            config,
            &worktree_ctx,
        ) {
            Ok(m) => m,
            Err(e) => {
                println!(
                    "{} [{}] Failed to assemble prompt: {}",
                    "✗".red(),
                    spec.id,
                    e
                );
                // Clean up worktree since we failed
                if let Some(ref path) = worktree_path {
                    let _ = worktree::remove_worktree(path);
                }
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
        let branch_name_clone = branch_for_cleanup.clone();
        let execution_state_clone = execution_state.clone();

        let handle = thread::spawn(move || {
            let result = cmd::agent::invoke_agent_with_command(
                &message,
                &spec_id,
                &prompt_name_clone,
                config_model.as_deref(),
                worktree_path_clone.as_deref(),
                &agent_command,
                branch_name_clone.as_deref(),
            );
            let (success, commits, error, agent_completed) = match result {
                Ok(_) => {
                    // Agent work succeeded - get commits
                    let commits = get_commits_for_spec(&spec_id).ok();

                    // Mark spec as completed for cleanup tracking
                    execution_state_clone.mark_completed(&spec_id);

                    // In branch mode: DON'T finalize in worktree - defer to post-merge phase
                    // This prevents the race condition where feature branch shows Completed
                    // but main doesn't have the finalization yet.
                    //
                    // In direct mode (no worktree): finalize on main branch directly
                    if !is_direct_mode_clone {
                        // Branch mode: skip finalization here, it will happen after merge
                        eprintln!(
                            "{} [{}] Agent work completed, deferring finalization to post-merge",
                            "→".cyan(),
                            spec_id
                        );

                        // Remove worktree - the branch is preserved for merging
                        if let Some(ref path) = worktree_path_clone {
                            if let Err(e) = worktree::remove_worktree(path) {
                                eprintln!(
                                    "{} [{}] Warning: Failed to remove worktree: {}",
                                    "⚠".yellow(),
                                    spec_id,
                                    e
                                );
                            }
                        }

                        // Return success with agent_completed=true but the spec is NOT finalized yet
                        // Finalization will happen in the merge phase on main branch
                        (true, commits, None, true)
                    } else {
                        // Direct mode (no worktree) - finalize on main branch directly
                        eprintln!(
                            "{} [{}] Finalizing spec on main branch (direct mode)",
                            "→".cyan(),
                            spec_id
                        );

                        let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                        let finalize_result =
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
                            };

                        match finalize_result {
                            Ok(()) => {
                                eprintln!("{} [{}] ✓ Finalization complete", "✓".green(), spec_id);
                                (true, commits, None, true)
                            }
                            Err(e) => {
                                eprintln!(
                                    "{} [{}] ✗ Cannot finalize spec: {}",
                                    "✗".red(),
                                    spec_id,
                                    e
                                );
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
            let merge_result = worktree::merge_and_cleanup(branch, options.no_rebase);

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
    // BRANCH MODE MERGE PHASE - Auto-merge branch mode branches unless --no-merge
    // =========================================================================

    let mut branch_mode_merged = 0;
    let mut branch_mode_failed: Vec<(String, bool)> = Vec::new();
    let mut branch_mode_skipped: Vec<(String, String)> = Vec::new();

    if !options.no_merge && !branch_mode_branches.is_empty() {
        println!(
            "\n{} Auto-merging {} branch mode branch(es)...",
            "→".cyan(),
            branch_mode_branches.len()
        );

        for (spec_id, branch) in &branch_mode_branches {
            println!("[{}] Merging to main...", spec_id.cyan());
            let merge_result = worktree::merge_and_cleanup(branch, options.no_rebase);

            if merge_result.success {
                // Merge succeeded - NOW finalize on main branch
                // This is the fix for the race condition: finalization happens AFTER merge
                println!(
                    "[{}] Merge succeeded, finalizing on main...",
                    spec_id.cyan()
                );

                let spec_path = specs_dir.join(format!("{}.md", spec_id));
                let finalize_result = if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
                    let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
                    // Get commits for the spec (now on main after merge)
                    let commits = get_commits_for_spec(spec_id).ok();
                    finalize_spec(&mut spec, &spec_path, config, &all_specs, false, commits)
                } else {
                    Err(anyhow::anyhow!("Failed to load spec for finalization"))
                };

                match finalize_result {
                    Ok(()) => {
                        branch_mode_merged += 1;
                        println!("[{}] {} Merged and finalized", spec_id.cyan(), "✓".green());
                    }
                    Err(e) => {
                        // Finalization failed AFTER successful merge
                        // The work is merged but not marked complete
                        eprintln!(
                            "[{}] {} Merged but finalization failed: {}",
                            spec_id.cyan(),
                            "⚠".yellow(),
                            e
                        );

                        // Mark as NeedsAttention with clear error context
                        let spec_path = specs_dir.join(format!("{}.md", spec_id));
                        if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
                            spec.frontmatter.status = SpecStatus::NeedsAttention;
                            let _ = spec.save(&spec_path);
                        }

                        // Track as failed for reporting
                        branch_mode_failed.push((spec_id.clone(), false));
                    }
                }
            } else {
                // Merge failed - preserve branch, spec stays in_progress
                branch_mode_failed.push((spec_id.clone(), merge_result.has_conflict));

                // DON'T mark as NeedsAttention here - keep spec in_progress
                // The spec status is still in_progress from when the agent started work
                // This is intentional: the work completed but merge failed
                // User needs to resolve merge conflict and then re-run finalization

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

                // Check for actual conflicts that need resolution spec
                if merge_result.has_conflict {
                    if let Ok(conflicting_files) = conflict::detect_conflicting_files() {
                        let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
                        let blocked_specs =
                            conflict::get_blocked_specs(&conflicting_files, &all_specs);

                        let source_branch = branch.to_string();
                        let (spec_title, _) = conflict::extract_spec_context(specs_dir, spec_id)
                            .unwrap_or((None, String::new()));
                        let diff_summary =
                            conflict::get_diff_summary(&source_branch, "main").unwrap_or_default();

                        let context = conflict::ConflictContext {
                            source_branch,
                            target_branch: "main".to_string(),
                            conflicting_files,
                            source_spec_id: spec_id.clone(),
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
                }
            }
        }
    } else if options.no_merge && !branch_mode_branches.is_empty() {
        // --no-merge specified, skip auto-merge
        branch_mode_skipped = branch_mode_branches.clone();
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

    // Report direct mode merges (if any)
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

    // Report branch mode merges
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
        for (spec_id, has_conflict) in &branch_mode_failed {
            let indicator = if *has_conflict { "⚡" } else { "→" };
            println!("    {} {}", indicator.yellow(), spec_id);
        }
    }

    // Show branches that were skipped due to --no-merge
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

    // Show branch mode information (only if --no-merge was used)
    if !branch_mode_skipped.is_empty() {
        println!(
            "\n{} Branch mode branches preserved for manual merging:",
            "→".cyan()
        );
        for (_spec_id, branch) in &branch_mode_skipped {
            println!("  {} {}", "•".yellow(), branch);
        }
        println!("\nUse {} to merge branches later.", "chant merge".bold());
    }

    // Show next steps for merge failures (direct mode or branch mode)
    let all_merge_failed = !merge_failed.is_empty() || !branch_mode_failed.is_empty();
    if all_merge_failed {
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

    // Ensure main repo is back on main branch after merge phase
    let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

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
