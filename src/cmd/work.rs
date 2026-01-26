//! Work command execution for chant CLI
//!
//! Handles spec execution including:
//! - Single spec execution with agent invocation
//! - Parallel spec execution with thread pools
//! - Spec finalization and status management
//! - Branch and PR creation
//! - Worktree management

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::conflict;
use chant::git;
use chant::prompt;
use chant::spec::{self, Spec, SpecStatus};
use chant::worktree;

use crate::cmd;

// ============================================================================
// CONSTANTS
// ============================================================================

pub(crate) const MAX_AGENT_OUTPUT_CHARS: usize = 5000;

// ============================================================================
// EXECUTION FUNCTIONS
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn cmd_work(
    id: Option<&str>,
    prompt_name: Option<&str>,
    cli_branch: Option<String>,
    cli_pr: bool,
    force: bool,
    parallel: bool,
    labels: &[String],
    finalize: bool,
    allow_no_commits: bool,
) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");
    let prompts_dir = PathBuf::from(".chant/prompts");
    let config = Config::load()?;

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Check for silent mode conflicts
    let in_silent_mode = is_silent_mode();
    if in_silent_mode && cli_pr {
        anyhow::bail!(
            "Cannot create pull request in silent mode - would reveal chant usage to the team. \
             Remove --pr or disable silent mode with `chant init --force` (non-silent)."
        );
    }
    if in_silent_mode && cli_branch.is_some() {
        println!(
            "{} Warning: Creating branches in silent mode will still be visible to the team",
            "⚠".yellow()
        );
    }

    // Handle parallel execution mode
    if parallel && id.is_none() {
        return cmd_work_parallel(
            &specs_dir,
            &prompts_dir,
            &config,
            prompt_name,
            labels,
            cli_branch.as_deref(),
        );
    }

    // If no ID and not parallel, require an ID
    let id = id.ok_or_else(|| anyhow::anyhow!("Spec ID required (or use --parallel)"))?;

    // Resolve spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Handle re-finalization mode
    if finalize {
        // Re-finalize flag requires the spec to be in_progress or completed
        if spec.frontmatter.status != SpecStatus::InProgress
            && spec.frontmatter.status != SpecStatus::Completed
        {
            anyhow::bail!(
                "Cannot re-finalize spec '{}' with status '{:?}'. Must be in_progress or completed.",
                spec.id,
                spec.frontmatter.status
            );
        }

        // Ask for confirmation (unless --force is used)
        if !confirm_re_finalize(&spec.id, force)? {
            println!("Re-finalization cancelled.");
            return Ok(());
        }

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

        return Ok(());
    }

    // Check if already completed
    if spec.frontmatter.status == SpecStatus::Completed && !force {
        println!("{} Spec already completed.", "⚠".yellow());
        println!("Use {} to replay.", "--force".cyan());
        return Ok(());
    }

    // Check if in progress
    if spec.frontmatter.status == SpecStatus::InProgress {
        println!("{} Spec already in progress.", "⚠".yellow());
        return Ok(());
    }

    // Check if dependencies are satisfied
    let all_specs = spec::load_all_specs(&specs_dir)?;
    if !spec.is_ready(&all_specs) && !force {
        // Find which dependencies are blocking
        let mut blocking: Vec<String> = Vec::new();

        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                let dep = all_specs.iter().find(|s| s.id == *dep_id);
                match dep {
                    Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                    Some(d) => blocking
                        .push(format!("{} ({:?})", dep_id, d.frontmatter.status).to_lowercase()),
                    None => blocking.push(format!("{} (not found)", dep_id)),
                }
            }
        }

        // Check for prior siblings
        if let Some(driver_id) = spec::extract_driver_id(&spec.id) {
            if let Some(member_num) = spec::extract_member_number(&spec.id) {
                for i in 1..member_num {
                    let sibling_id = format!("{}.{}", driver_id, i);
                    let sibling = all_specs.iter().find(|s| s.id == sibling_id);
                    if let Some(s) = sibling {
                        if s.frontmatter.status != SpecStatus::Completed {
                            blocking.push(
                                format!("{} ({:?})", sibling_id, s.frontmatter.status)
                                    .to_lowercase(),
                            );
                        }
                    } else {
                        blocking.push(format!("{} (not found)", sibling_id));
                    }
                }
            }
        }

        if !blocking.is_empty() {
            println!("{} Spec has unsatisfied dependencies.", "✗".red());
            println!("Blocked by: {}", blocking.join(", "));
            println!("Use {} to bypass dependency checks.", "--force".cyan());
            anyhow::bail!("Cannot execute spec with unsatisfied dependencies");
        }
    }

    // CLI flags override config defaults
    let create_pr = cli_pr || config.defaults.pr;
    let use_branch_prefix = cli_branch
        .as_deref()
        .unwrap_or(&config.defaults.branch_prefix);
    let create_branch = cli_branch.is_some() || config.defaults.branch || create_pr;

    // Handle branch creation/switching if requested
    let branch_name = if create_branch {
        let branch_name = format!("{}{}", use_branch_prefix, spec.id);
        create_or_switch_branch(&branch_name)?;
        spec.frontmatter.branch = Some(branch_name.clone());
        println!("{} Branch: {}", "→".cyan(), branch_name);
        Some(branch_name)
    } else {
        None
    };

    // Resolve prompt
    let prompt_name = prompt_name
        .or(spec.frontmatter.prompt.as_deref())
        .unwrap_or(&config.defaults.prompt);

    let prompt_path = prompts_dir.join(format!("{}.md", prompt_name));
    if !prompt_path.exists() {
        anyhow::bail!("Prompt not found: {}", prompt_name);
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

    // Invoke agent
    let result = cmd::agent::invoke_agent(&message, &spec, prompt_name, &config);

    match result {
        Ok(agent_output) => {
            // Reload spec (it may have been modified by the agent)
            let mut spec = spec::resolve_spec(&specs_dir, &spec.id)?;

            // Check for unchecked acceptance criteria
            let unchecked_count = spec.count_unchecked_checkboxes();
            if unchecked_count > 0 && !force {
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
                println!("Use {} to skip this validation.", "--force".cyan());
                // Mark as failed since we can't complete with unchecked items
                spec.frontmatter.status = SpecStatus::Failed;
                spec.save(&spec_path)?;
                anyhow::bail!(
                    "Cannot complete spec with {} unchecked acceptance criteria",
                    unchecked_count
                );
            }

            // Finalize the spec (set status, commits, completed_at, model)
            let all_specs = spec::load_all_specs(&specs_dir)?;
            finalize_spec(&mut spec, &spec_path, &config, &all_specs, allow_no_commits)?;

            // If this is a member spec, check if driver should be auto-completed
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

            // Create PR if requested (after finalization so PR URL can be saved)
            if create_pr {
                let branch_name = branch_name
                    .as_ref()
                    .expect("branch_name should exist when create_pr is true");
                println!("\n{} Pushing branch to remote...", "→".cyan());
                match push_branch(branch_name) {
                    Ok(()) => {
                        let provider = git::get_provider(config.git.provider);
                        println!(
                            "{} Creating pull request via {}...",
                            "→".cyan(),
                            provider.name()
                        );
                        let pr_title = spec.title.clone().unwrap_or_else(|| spec.id.clone());
                        let pr_body = spec.body.clone();
                        match provider.create_pr(&pr_title, &pr_body) {
                            Ok(pr_url) => {
                                spec.frontmatter.pr = Some(pr_url.clone());
                                println!("{} PR created: {}", "✓".green(), pr_url);
                            }
                            Err(e) => {
                                // PR creation failed, but spec is still finalized
                                println!("{} Failed to create PR: {}", "⚠".yellow(), e);
                            }
                        }
                    }
                    Err(e) => {
                        // Push failed, but spec is still finalized
                        println!("{} Failed to push branch: {}", "⚠".yellow(), e);
                    }
                }
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
struct ParallelResult {
    spec_id: String,
    success: bool,
    commits: Option<Vec<String>>,
    error: Option<String>,
    worktree_path: Option<PathBuf>,
    branch_name: Option<String>,
    is_direct_mode: bool,
}

pub fn cmd_work_parallel(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    prompt_name: Option<&str>,
    labels: &[String],
    cli_branch_prefix: Option<&str>,
) -> Result<()> {
    use std::sync::mpsc;
    use std::thread;

    // Load all specs and filter to ready ones
    let all_specs = spec::load_all_specs(specs_dir)?;
    let mut ready_specs: Vec<Spec> = all_specs
        .iter()
        .filter(|s| s.is_ready(&all_specs))
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

    if ready_specs.is_empty() {
        if !labels.is_empty() {
            println!("No ready specs with specified labels.");
        } else {
            println!("No ready specs to execute.");
        }
        return Ok(());
    }

    println!(
        "{} Starting {} specs in parallel...\n",
        "→".cyan(),
        ready_specs.len()
    );

    // Resolve prompt name for all specs
    let default_prompt = &config.defaults.prompt;

    // Create channels for collecting results
    let (tx, rx) = mpsc::channel::<ParallelResult>();

    // Spawn threads for each spec
    let mut handles = Vec::new();

    for spec in ready_specs.iter() {
        // Determine prompt for this spec
        let spec_prompt = prompt_name
            .or(spec.frontmatter.prompt.as_deref())
            .unwrap_or(default_prompt);

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

        // Update spec status to in_progress
        let spec_path = specs_dir.join(format!("{}.md", spec.id));
        let mut spec_clone = spec.clone();
        spec_clone.frontmatter.status = SpecStatus::InProgress;
        if let Err(e) = spec_clone.save(&spec_path) {
            println!("{} [{}] Failed to update status: {}", "✗".red(), spec.id, e);
            continue;
        }

        println!("[{}] Working with prompt '{}'", spec.id.cyan(), spec_prompt);

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
        let (is_direct_mode, branch_prefix) = if let Some(cli_prefix) = cli_branch_prefix {
            // CLI --branch specified with explicit prefix
            (false, cli_prefix.to_string())
        } else if let Some(spec_branch) = &spec.frontmatter.branch {
            // Spec has explicit branch prefix
            (false, spec_branch.clone())
        } else if config.defaults.branch {
            // Config enables branch mode - use config's branch_prefix
            (false, config.defaults.branch_prefix.clone())
        } else {
            // Direct mode (no branching, merge immediately)
            (true, String::new())
        };

        // Determine branch name based on mode
        let branch_name = if is_direct_mode {
            format!("spec/{}", spec.id)
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

        let handle = thread::spawn(move || {
            let result = cmd::agent::invoke_agent_with_prefix(
                &message,
                &spec_id,
                &prompt_name_clone,
                config_model.as_deref(),
                worktree_path_clone.as_deref(),
            );
            let (success, commits, error, _final_status) = match result {
                Ok(_) => {
                    // Get the commits
                    let commits = get_commits_for_spec(&spec_id).ok();

                    // Handle cleanup based on mode
                    let (cleanup_error, has_merge_conflict) = if is_direct_mode_clone {
                        // Direct mode: merge and cleanup
                        if let Some(ref branch) = branch_for_cleanup_clone {
                            let merge_result = worktree::merge_and_cleanup(branch);
                            let error = merge_result.error.as_ref().map(|e| e.to_string());
                            (error, merge_result.has_conflict)
                        } else {
                            (None, false)
                        }
                    } else {
                        // Branch mode: just remove worktree
                        if let Some(ref path) = worktree_path_clone {
                            match worktree::remove_worktree(path) {
                                Ok(_) => (None, false),
                                Err(e) => (Some(e.to_string()), false),
                            }
                        } else {
                            (None, false)
                        }
                    };

                    // Handle merge conflicts by creating a conflict spec
                    if has_merge_conflict {
                        // Detect conflicting files
                        if let Ok(conflicting_files) = conflict::detect_conflicting_files() {
                            // Get all specs to identify blocked specs
                            let all_specs =
                                spec::load_all_specs(&specs_dir_clone).unwrap_or_default();
                            let blocked_specs =
                                conflict::get_blocked_specs(&conflicting_files, &all_specs);

                            // Build context for conflict spec
                            let source_branch = if is_direct_mode_clone {
                                format!("spec/{}", spec_id)
                            } else {
                                branch_for_cleanup_clone.clone().unwrap_or_default()
                            };

                            let (spec_title, _) =
                                conflict::extract_spec_context(&specs_dir_clone, &spec_id)
                                    .unwrap_or((None, String::new()));
                            let diff_summary = conflict::get_diff_summary(&source_branch, "main")
                                .unwrap_or_default();

                            let context = conflict::ConflictContext {
                                source_branch: source_branch.clone(),
                                target_branch: "main".to_string(),
                                conflicting_files,
                                source_spec_id: spec_id.clone(),
                                source_spec_title: spec_title,
                                diff_summary,
                            };

                            // Create conflict spec
                            if let Ok(conflict_spec_id) = conflict::create_conflict_spec(
                                &specs_dir_clone,
                                &context,
                                blocked_specs,
                            ) {
                                eprintln!(
                                    "{} [{}] Conflict detected. Created resolution spec: {}",
                                    "⚡".yellow(),
                                    spec_id,
                                    conflict_spec_id
                                );
                            }
                        }
                    }

                    // Update spec status based on cleanup result
                    let mut success_final = cleanup_error.is_none();
                    let mut status_final = if cleanup_error.is_some() {
                        SpecStatus::NeedsAttention
                    } else {
                        SpecStatus::Completed
                    };

                    // Update spec to completed or needs attention
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        // Check if spec is a driver with incomplete members before marking completed
                        if status_final == SpecStatus::Completed {
                            let all_specs = match spec::load_all_specs(&specs_dir_clone) {
                                Ok(specs) => specs,
                                Err(e) => {
                                    eprintln!(
                                        "{} [{}] Warning: Failed to load all specs for validation: {}",
                                        "⚠".yellow(),
                                        spec_id,
                                        e
                                    );
                                    vec![]
                                }
                            };
                            let incomplete_members =
                                spec::get_incomplete_members(&spec_id, &all_specs);
                            if !incomplete_members.is_empty() {
                                eprintln!(
                                    "{} [{}] Cannot complete driver spec with {} incomplete member(s): {}",
                                    "⚠".yellow(),
                                    spec_id,
                                    incomplete_members.len(),
                                    incomplete_members.join(", ")
                                );
                                spec.frontmatter.status = SpecStatus::NeedsAttention;
                                let _ = spec.save(&spec_path);
                                success_final = false;
                                status_final = SpecStatus::NeedsAttention;
                            } else {
                                spec.frontmatter.status = status_final.clone();
                                spec.frontmatter.commits =
                                    commits.clone().filter(|c| !c.is_empty());
                                spec.frontmatter.completed_at = Some(
                                    chrono::Local::now()
                                        .format("%Y-%m-%dT%H:%M:%SZ")
                                        .to_string(),
                                );
                                spec.frontmatter.model =
                                    get_model_name_with_default(config_model.as_deref());
                                if let Err(e) = spec.save(&spec_path) {
                                    eprintln!(
                                        "{} [{}] Warning: Failed to finalize spec: {}",
                                        "⚠".yellow(),
                                        spec_id,
                                        e
                                    );
                                }
                            }
                        } else {
                            spec.frontmatter.status = status_final.clone();
                            spec.frontmatter.commits = commits.clone().filter(|c| !c.is_empty());
                            spec.frontmatter.completed_at = Some(
                                chrono::Local::now()
                                    .format("%Y-%m-%dT%H:%M:%SZ")
                                    .to_string(),
                            );
                            spec.frontmatter.model =
                                get_model_name_with_default(config_model.as_deref());
                            if let Err(e) = spec.save(&spec_path) {
                                eprintln!(
                                    "{} [{}] Warning: Failed to finalize spec: {}",
                                    "⚠".yellow(),
                                    spec_id,
                                    e
                                );
                            }
                        }
                    }

                    (success_final, commits, cleanup_error, status_final)
                }
                Err(e) => {
                    // Agent failed - still need to cleanup worktree
                    let _cleanup_error = if is_direct_mode_clone {
                        // Direct mode: try to merge and cleanup anyway
                        if let Some(ref branch) = branch_for_cleanup_clone {
                            let merge_result = worktree::merge_and_cleanup(branch);
                            merge_result.error.clone()
                        } else {
                            Some(e.to_string())
                        }
                    } else {
                        // Branch mode: try to remove worktree
                        if let Some(ref path) = worktree_path_clone {
                            worktree::remove_worktree(path).err().map(|e| e.to_string())
                        } else {
                            Some(e.to_string())
                        }
                    };

                    // Update spec to failed
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        spec.frontmatter.status = SpecStatus::Failed;
                        if let Err(save_err) = spec.save(&spec_path) {
                            eprintln!(
                                "{} [{}] Warning: Failed to mark spec as failed: {}",
                                "⚠".yellow(),
                                spec_id,
                                save_err
                            );
                        }
                    }

                    (false, None, Some(e.to_string()), SpecStatus::Failed)
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
            });
        });

        handles.push(handle);
    }

    // Drop the original sender so the receiver knows when all threads are done
    drop(tx);

    // Collect results
    let mut completed = 0;
    let mut failed = 0;
    let mut all_results = Vec::new();
    let mut branch_mode_branches = Vec::new();

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

            // Collect branch info for branch mode
            if !result.is_direct_mode {
                if let Some(ref branch) = result.branch_name {
                    branch_mode_branches.push((result.spec_id.clone(), branch.clone()));
                }
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
    println!(
        "\n{}: {} completed, {} failed",
        "Summary".bold(),
        completed,
        failed
    );

    // Show branch mode information
    if !branch_mode_branches.is_empty() {
        println!(
            "\n{} Branch mode branches created for reconciliation:",
            "→".cyan()
        );
        for (_spec_id, branch) in branch_mode_branches {
            println!("  {} {}", "•".yellow(), branch);
        }
        println!(
            "\nUse {} to reconcile branches later.",
            "chant reconcile".bold()
        );
    } else if cli_branch_prefix.is_some() || config.defaults.branch {
        println!("\n{} Direct mode: All changes merged to main.", "→".cyan());
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// ============================================================================
// SPEC FINALIZATION FUNCTIONS
// ============================================================================

/// Enum to distinguish between different commit retrieval scenarios
#[derive(Debug)]
pub(crate) enum CommitError {
    /// Git command failed (e.g., not in a git repository)
    GitCommandFailed(String),
    /// Git log succeeded but found no matching commits
    NoMatchingCommits,
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitError::GitCommandFailed(err) => write!(f, "Git command failed: {}", err),
            CommitError::NoMatchingCommits => write!(f, "No matching commits found"),
        }
    }
}

impl std::error::Error for CommitError {}

pub(crate) fn get_commits_for_spec(spec_id: &str) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, false)
}

pub(crate) fn get_commits_for_spec_allow_no_commits(spec_id: &str) -> Result<Vec<String>> {
    get_commits_for_spec_internal(spec_id, true)
}

fn get_commits_for_spec_internal(spec_id: &str, allow_no_commits: bool) -> Result<Vec<String>> {
    use std::process::Command;

    // Look for all commits with the chant(spec_id) pattern
    let pattern = format!("chant({})", spec_id);

    let output = Command::new("git")
        .args(["log", "--oneline", "--grep", &pattern, "--reverse"])
        .output()
        .context("Failed to execute git log command")?;

    // Check if git command itself failed
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!(
            "git log command failed for pattern 'chant({})': {}",
            spec_id, stderr
        );
        eprintln!("{} {}", "✗".red(), error_msg);
        return Err(anyhow::anyhow!(CommitError::GitCommandFailed(error_msg)));
    }

    // Parse commits from successful output
    let mut commits = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(hash) = line.split_whitespace().next() {
            if !hash.is_empty() {
                commits.push(hash.to_string());
            }
        }
    }

    // If no matching commits found, decide what to do based on flag
    if commits.is_empty() {
        if allow_no_commits {
            // Fallback behavior: use HEAD with warning
            eprintln!(
                "{} No commits found with pattern 'chant({})'. Attempting to use HEAD as fallback.",
                "⚠".yellow(),
                spec_id
            );

            let head_output = Command::new("git")
                .args(["rev-parse", "--short=7", "HEAD"])
                .output()
                .context("Failed to execute git rev-parse command")?;

            if head_output.status.success() {
                let head_hash = String::from_utf8_lossy(&head_output.stdout)
                    .trim()
                    .to_string();
                if !head_hash.is_empty() {
                    eprintln!("{} Using HEAD commit: {}", "⚠".yellow(), head_hash);
                    commits.push(head_hash);
                }
            } else {
                let stderr = String::from_utf8_lossy(&head_output.stderr);
                let error_msg = format!(
                    "Could not find any commit for spec '{}' and HEAD fallback failed: {}",
                    spec_id, stderr
                );
                eprintln!("{} {}", "✗".red(), error_msg);
                return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
            }
        } else {
            // Default behavior: fail loudly
            let error_msg = format!(
                "No commits found matching 'chant({})' pattern. Did the agent forget to commit?\n\
                 Commits must follow the pattern: 'chant({}): <description>'\n\
                 Use --allow-no-commits to use HEAD as fallback (for special cases only).",
                spec_id, spec_id
            );
            eprintln!("{} {}", "✗".red(), error_msg);
            return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
        }
    }

    Ok(commits)
}

/// Finalize a spec after successful completion
/// Sets status, commits, completed_at, and model
/// This function is idempotent and can be called multiple times safely
pub fn finalize_spec(
    spec: &mut Spec,
    spec_path: &Path,
    config: &Config,
    all_specs: &[Spec],
    allow_no_commits: bool,
) -> Result<()> {
    // Check if this is a driver spec with incomplete members
    let incomplete_members = spec::get_incomplete_members(&spec.id, all_specs);
    if !incomplete_members.is_empty() {
        anyhow::bail!(
            "Cannot complete driver spec '{}' while {} member spec(s) are incomplete: {}",
            spec.id,
            incomplete_members.len(),
            incomplete_members.join(", ")
        );
    }

    // Get the commits for this spec
    let commits = if allow_no_commits {
        get_commits_for_spec_allow_no_commits(&spec.id)?
    } else {
        get_commits_for_spec(&spec.id)?
    };

    // Update spec to completed
    spec.frontmatter.status = SpecStatus::Completed;
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );
    spec.frontmatter.model = get_model_name(Some(config));

    // Save the spec - this must not fail silently
    spec.save(spec_path)
        .context("Failed to save finalized spec")?;

    // Validation 1: Verify that status was actually changed to Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after finalization"
    );

    // Validation 2: Verify that completed_at timestamp is set and in valid ISO format
    let completed_at = spec
        .frontmatter
        .completed_at
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("completed_at timestamp was not set"))?;

    // Validate ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
    if !completed_at.ends_with('Z') {
        anyhow::bail!(
            "completed_at must end with 'Z' (UTC format), got: {}",
            completed_at
        );
    }
    if !completed_at.contains('T') {
        anyhow::bail!(
            "completed_at must contain 'T' separator (ISO format), got: {}",
            completed_at
        );
    }

    // Validation 3: Verify that spec was actually saved (reload and check)
    let saved_spec =
        Spec::load(spec_path).context("Failed to reload spec from disk to verify persistence")?;

    anyhow::ensure!(
        saved_spec.frontmatter.status == SpecStatus::Completed,
        "Persisted spec status is not Completed - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.completed_at.is_some(),
        "Persisted spec is missing completed_at - save may have failed"
    );

    // Model may be None if no model was detected, but commits should match memory
    match (&spec.frontmatter.commits, &saved_spec.frontmatter.commits) {
        (Some(mem_commits), Some(saved_commits)) => {
            anyhow::ensure!(
                mem_commits == saved_commits,
                "Persisted commits don't match memory - save may have failed"
            );
        }
        (None, None) => {
            // Both None is correct
        }
        _ => {
            anyhow::bail!("Persisted commits don't match memory - save may have failed");
        }
    }

    Ok(())
}

/// Re-finalize a spec that was left in an incomplete state
/// This can be called on in_progress or completed specs to update commits and timestamp
/// Idempotent: safe to call multiple times
pub(crate) fn re_finalize_spec(
    spec: &mut Spec,
    spec_path: &Path,
    config: &Config,
    allow_no_commits: bool,
) -> Result<()> {
    // Re-finalization only works on specs that have been started (in_progress or completed)
    // A pending spec has never been started and should use normal work flow
    match spec.frontmatter.status {
        SpecStatus::InProgress | SpecStatus::Completed => {
            // These are valid for re-finalization
        }
        _ => {
            anyhow::bail!(
                "Cannot re-finalize spec '{}' with status '{:?}'. Must be in_progress or completed.",
                spec.id,
                spec.frontmatter.status
            );
        }
    }

    // Get the commits for this spec (may have new ones since last finalization)
    let commits = if allow_no_commits {
        get_commits_for_spec_allow_no_commits(&spec.id)?
    } else {
        get_commits_for_spec(&spec.id)?
    };

    // Update spec with new commit info
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };

    // Update the timestamp to now
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );

    // Update model name
    spec.frontmatter.model = get_model_name(Some(config));

    // Ensure spec is marked as completed
    spec.frontmatter.status = SpecStatus::Completed;

    // Save the spec
    spec.save(spec_path)
        .context("Failed to save re-finalized spec")?;

    // Validation 1: Verify that status is Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after re-finalization"
    );

    // Validation 2: Verify completed_at timestamp is set and valid
    let completed_at = spec
        .frontmatter
        .completed_at
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("completed_at timestamp was not set"))?;

    if !completed_at.ends_with('Z') {
        anyhow::bail!(
            "completed_at must end with 'Z' (UTC format), got: {}",
            completed_at
        );
    }
    if !completed_at.contains('T') {
        anyhow::bail!(
            "completed_at must contain 'T' separator (ISO format), got: {}",
            completed_at
        );
    }

    // Validation 3: Verify spec was saved (reload and check)
    let saved_spec =
        Spec::load(spec_path).context("Failed to reload spec from disk to verify persistence")?;

    anyhow::ensure!(
        saved_spec.frontmatter.status == SpecStatus::Completed,
        "Persisted spec status is not Completed - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.completed_at.is_some(),
        "Persisted spec is missing completed_at - save may have failed"
    );

    Ok(())
}

/// Prompt for user confirmation
/// Returns true if user confirms, false otherwise
/// force_flag bypasses the confirmation
fn confirm_re_finalize(spec_id: &str, force_flag: bool) -> Result<bool> {
    if force_flag {
        return Ok(true);
    }

    println!(
        "{} Are you sure you want to re-finalize spec '{}'?",
        "?".cyan(),
        spec_id
    );
    println!("This will update commits and completion timestamp to now.");
    println!("Use {} to skip this confirmation.", "--force".cyan());

    use std::io::{self, Write};
    print!("Continue? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

// ============================================================================
// GIT OPERATIONS
// ============================================================================

fn create_or_switch_branch(branch_name: &str) -> Result<()> {
    use std::process::Command;

    // Try to create a new branch
    let create_output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .output()
        .context("Failed to run git checkout")?;

    if create_output.status.success() {
        return Ok(());
    }

    // Branch might already exist, try to switch to it
    let switch_output = Command::new("git")
        .args(["checkout", branch_name])
        .output()
        .context("Failed to run git checkout")?;

    if switch_output.status.success() {
        return Ok(());
    }

    // Both failed, return error
    let stderr = String::from_utf8_lossy(&switch_output.stderr);
    anyhow::bail!(
        "Failed to create or switch to branch '{}': {}",
        branch_name,
        stderr
    )
}

fn push_branch(branch_name: &str) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["push", "-u", "origin", branch_name])
        .output()
        .context("Failed to run git push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to push branch '{}': {}", branch_name, stderr);
    }

    Ok(())
}

// ============================================================================
// MODEL SELECTION
// ============================================================================

/// Get the model name using the following priority:
/// 1. CHANT_MODEL env var (explicit override)
/// 2. ANTHROPIC_MODEL env var (Claude CLI default)
/// 3. defaults.model in config
/// 4. Parse from `claude --version` output (last resort)
pub(crate) fn get_model_name(config: Option<&Config>) -> Option<String> {
    get_model_name_with_default(config.and_then(|c| c.defaults.model.as_deref()))
}

/// Get the model name with an optional default from config.
/// Used by parallel execution where full Config isn't available.
pub(crate) fn get_model_name_with_default(config_model: Option<&str>) -> Option<String> {
    // 1. CHANT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return Some(model);
        }
    }

    // 2. ANTHROPIC_MODEL env var
    if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
        if !model.is_empty() {
            return Some(model);
        }
    }

    // 3. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return Some(model.to_string());
        }
    }

    // 4. Parse from claude --version output
    parse_model_from_claude_version()
}

/// Parse model name from `claude --version` output.
/// Expected format: "X.Y.Z (model-name)" or similar patterns.
fn parse_model_from_claude_version() -> Option<String> {
    use std::process::Command;

    let output = Command::new("claude").arg("--version").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);

    // Try to extract model from parentheses, e.g., "1.0.0 (claude-sonnet-4)"
    if let Some(start) = version_str.find('(') {
        if let Some(end) = version_str.find(')') {
            if start < end {
                let model = version_str[start + 1..end].trim();
                // Check if it looks like a model name (contains "claude" or common model patterns)
                if model.contains("claude")
                    || model.contains("sonnet")
                    || model.contains("opus")
                    || model.contains("haiku")
                {
                    return Some(model.to_string());
                }
            }
        }
    }

    None
}

// ============================================================================
// TRANSCRIPT HANDLING
// ============================================================================

fn commit_transcript(spec_id: &str, spec_path: &Path) -> Result<()> {
    use std::process::Command;

    // Stage the spec file
    let output = Command::new("git")
        .args(["add", &spec_path.to_string_lossy()])
        .output()
        .context("Failed to run git add for transcript commit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to stage spec file for transcript commit: {}",
            stderr
        );
    }

    // Create commit for transcript
    let commit_message = format!("chant: Record agent transcript for {}", spec_id);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to run git commit for transcript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // It's ok if there's nothing to commit (no changes after finalization)
        if stderr.contains("nothing to commit") || stderr.contains("no changes added") {
            return Ok(());
        }
        anyhow::bail!("Failed to commit transcript: {}", stderr);
    }

    Ok(())
}

pub(crate) fn append_agent_output(spec: &mut Spec, output: &str) {
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let formatted_output = if output.len() > MAX_AGENT_OUTPUT_CHARS {
        let truncated = &output[..MAX_AGENT_OUTPUT_CHARS];
        format!(
            "{}\n\n... (output truncated, {} chars total)",
            truncated,
            output.len()
        )
    } else {
        output.to_string()
    };

    let agent_section = format!(
        "\n\n## Agent Output\n\n{}\n\n```\n{}```\n",
        timestamp,
        formatted_output.trim_end()
    );

    spec.body.push_str(&agent_section);
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
    fn test_get_commits_for_spec_error_behavior() {
        // This test verifies that when the spec ID doesn't have matching commits,
        // get_commits_for_spec returns an error (default behavior)
        // Note: This test assumes we're in a git repo with no commits matching "chant(nonexistent-spec-xyz-abc)"

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
    }

    #[test]
    fn test_get_commits_for_spec_allow_no_commits_behavior() {
        // This test verifies that when allow_no_commits is true,
        // the function returns HEAD as a fallback

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
    }
}
