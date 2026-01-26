//! Work command execution for chant CLI
//!
//! Handles spec execution including:
//! - Single spec execution with agent invocation
//! - Parallel spec execution with thread pools
//! - Spec finalization and status management
//! - Branch and PR creation
//! - Worktree management

use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::conflict;
use chant::git;
use chant::paths::PROMPTS_DIR;
use chant::prompt;
use chant::spec::{self, Spec, SpecStatus};
use chant::worktree;

use crate::cmd;
use crate::cmd::commits::get_commits_for_spec;
use crate::cmd::finalize::{
    append_agent_output, confirm_re_finalize, finalize_spec, re_finalize_spec,
};
use crate::cmd::git_ops::{commit_transcript, create_or_switch_branch, push_branch};
use crate::cmd::model::get_model_name_with_default;

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
    let specs_dir = crate::cmd::ensure_initialized()?;
    let prompts_dir = PathBuf::from(PROMPTS_DIR);
    let config = Config::load()?;

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

        // If this is a member spec, check if driver should be auto-completed
        let all_specs = spec::load_all_specs(&specs_dir)?;
        if spec::auto_complete_driver_if_ready(&spec.id, &all_specs, &specs_dir)? {
            println!(
                "\n{} Auto-completed driver spec: {}",
                "✓".green(),
                spec::extract_driver_id(&spec.id).unwrap()
            );
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
        // IMPORTANT: Parallel execution forces branch mode internally for isolation
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
            // Parallel execution forces branch mode even if config.defaults.branch is false
            // This prevents merge race conditions during parallel work
            (false, "spec/".to_string())
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

        let handle = thread::spawn(move || {
            let result = cmd::agent::invoke_agent_with_prefix(
                &message,
                &spec_id,
                &prompt_name_clone,
                config_model.as_deref(),
                worktree_path_clone.as_deref(),
            );
            let (success, commits, error, agent_completed) = match result {
                Ok(_) => {
                    // Agent work succeeded - get commits
                    let commits = get_commits_for_spec(&spec_id).ok();

                    // For branch mode: just remove worktree (don't merge yet)
                    // For direct mode: also don't merge yet - defer to serialized phase
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

                    // Finalize spec with completed status (merge status separate)
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
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

                        // Check if driver with incomplete members
                        let incomplete_members = spec::get_incomplete_members(&spec_id, &all_specs);
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
                            (
                                false,
                                commits,
                                Some("Incomplete members".to_string()),
                                false,
                            )
                        } else {
                            spec.frontmatter.status = SpecStatus::Completed;
                            spec.frontmatter.commits =
                                commits.clone().filter(|c: &Vec<String>| !c.is_empty());
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
                            (true, commits, None, true)
                        }
                    } else {
                        (true, commits, None, true)
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
                println!(
                    "[{}] {} Merge failed (branch preserved): {}",
                    result.spec_id.cyan(),
                    "⚠".yellow(),
                    error_msg
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
    }

    // Show next steps for merge failures
    if !merge_failed.is_empty() {
        println!("\n{} Next steps for merge-pending branches:", "→".cyan());
        println!("  - Review each branch for conflicts");
        println!(
            "  - Resolve conflicts manually or run {} to merge sequentially",
            "chant merge".bold()
        );
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
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
