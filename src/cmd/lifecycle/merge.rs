//! Spec merging functionality - merges completed spec branches back to main

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::git;
use chant::merge;
use chant::merge_errors;
use chant::paths::PROMPTS_DIR;
use chant::spec::{self, Spec, SpecFrontmatter, SpecStatus};

/// Resolve merge conflicts using an agent
fn resolve_conflicts_with_agent(
    branch_name: &str,
    onto_branch: &str,
    conflicting_files: &[String],
    config: &Config,
) -> Result<()> {
    use crate::cmd::agent;

    // Get the merge-conflict prompt if it exists, otherwise use a default message
    let prompts_dir = PathBuf::from(PROMPTS_DIR);
    let conflict_prompt_path = prompts_dir.join("merge-conflict.md");

    let message = if conflict_prompt_path.exists() {
        // Load and assemble the conflict prompt
        let prompt_content = std::fs::read_to_string(&conflict_prompt_path)
            .context("Failed to read merge-conflict prompt")?;

        // Get diff for conflicting files
        let conflict_diff = get_conflict_diff(conflicting_files)?;

        // Simple template substitution
        prompt_content
            .replace("{{branch_name}}", branch_name)
            .replace("{{target_branch}}", onto_branch)
            .replace("{{conflicting_files}}", &conflicting_files.join(", "))
            .replace("{{conflict_diff}}", &conflict_diff)
    } else {
        // Default inline prompt
        let conflict_diff = get_conflict_diff(conflicting_files)?;
        format!(
            r#"# Resolve Merge Conflict

You are resolving a git conflict during rebase.

## Context
- Branch being rebased: {}
- Rebasing onto: {}
- Conflicting files: {}

## Current Diff
{}

## Instructions
1. Read each conflicting file to see the conflict markers (<<<<<<< ======= >>>>>>>)
2. Edit the files to resolve the conflicts (usually include both changes for additive conflicts)
3. After editing, stage each resolved file with a shell command: git add <file>
4. When all conflicts are resolved, run: git rebase --continue

IMPORTANT: Do NOT use git commit. Just resolve conflicts, stage files, and run git rebase --continue.
"#,
            branch_name,
            onto_branch,
            conflicting_files.join(", "),
            conflict_diff
        )
    };

    // Create a minimal spec for the agent invocation
    let conflict_spec = Spec {
        id: format!("conflict-{}", branch_name.replace('/', "-")),
        frontmatter: SpecFrontmatter::default(),
        title: Some(format!(
            "Resolve conflict: {} → {}",
            branch_name, onto_branch
        )),
        body: message.clone(),
    };

    // Invoke agent to resolve conflicts
    agent::invoke_agent(&message, &conflict_spec, "merge-conflict", config)?;

    // Check if conflicts were resolved
    let remaining_conflicts = git::get_conflicting_files()?;
    if !remaining_conflicts.is_empty() {
        anyhow::bail!(
            "Agent did not resolve all conflicts. Remaining: {}",
            remaining_conflicts.join(", ")
        );
    }

    Ok(())
}

/// Get diff output for conflicting files
fn get_conflict_diff(files: &[String]) -> Result<String> {
    use std::process::Command;

    let mut diff_output = String::new();

    for file in files {
        let output = Command::new("git")
            .args(["diff", file])
            .output()
            .context("Failed to run git diff")?;

        if output.status.success() {
            let diff = String::from_utf8_lossy(&output.stdout);
            diff_output.push_str(&format!("### {}\n```diff\n{}\n```\n\n", file, diff));
        }
    }

    Ok(diff_output)
}

// ============================================================================
// MERGE WIZARD
// ============================================================================

/// Show branch status for all completed specs
fn show_branch_status(all_specs: &[Spec], branch_prefix: &str, main_branch: &str) -> Result<()> {
    use merge::{BranchInfo, BranchStatus};

    let branch_infos = merge::get_branch_info_for_specs(all_specs, branch_prefix, main_branch)?;

    if branch_infos.is_empty() {
        println!("No completed specs with branches found.");
        return Ok(());
    }

    // Separate into ready and not ready
    let ready: Vec<&BranchInfo> = branch_infos
        .iter()
        .filter(|info| info.status == BranchStatus::Ready)
        .collect();

    let not_ready: Vec<&BranchInfo> = branch_infos
        .iter()
        .filter(|info| info.status != BranchStatus::Ready)
        .collect();

    // Display ready branches
    if !ready.is_empty() {
        println!("{}", "Ready to merge:".green().bold());
        for info in ready {
            let title = info.spec_title.as_deref().unwrap_or("(no title)");
            let commits_str = if info.commit_count == 1 {
                "1 commit".to_string()
            } else {
                format!("{} commits", info.commit_count)
            };
            println!(
                "  {} {}  ({}, all criteria met)",
                branch_prefix.dimmed(),
                info.spec_id.cyan(),
                commits_str.dimmed()
            );
            println!("    {}", title.dimmed());
        }
        println!();
    }

    // Display not ready branches
    if !not_ready.is_empty() {
        println!("{}", "Not ready:".yellow().bold());
        for info in not_ready {
            let title = info.spec_title.as_deref().unwrap_or("(no title)");
            let reason = match &info.status {
                BranchStatus::NeedsRebase => "behind main, needs rebase".to_string(),
                BranchStatus::HasConflicts => "has conflicts with main".to_string(),
                BranchStatus::Incomplete => format!(
                    "{}/{} criteria checked",
                    info.criteria_checked, info.criteria_total
                ),
                BranchStatus::NoCommits => "no commits".to_string(),
                _ => "unknown".to_string(),
            };
            println!(
                "  {} {}  ({})",
                branch_prefix.dimmed(),
                info.spec_id.yellow(),
                reason.dimmed()
            );
            println!("    {}", title.dimmed());
        }
    }

    Ok(())
}

/// Merge all ready branches (can fast-forward, all criteria met)
#[allow(clippy::too_many_arguments)]
fn merge_ready_branches(
    all_specs: &[Spec],
    branch_prefix: &str,
    main_branch: &str,
    dry_run: bool,
    delete_branch: bool,
    continue_on_error: bool,
    yes: bool,
    auto_resolve: bool,
    finalize: bool,
    config: &Config,
    specs_dir: &Path,
) -> Result<()> {
    use merge::{BranchInfo, BranchStatus};

    let branch_infos = merge::get_branch_info_for_specs(all_specs, branch_prefix, main_branch)?;

    let ready: Vec<&BranchInfo> = branch_infos
        .iter()
        .filter(|info| info.status == BranchStatus::Ready)
        .collect();

    if ready.is_empty() {
        println!("No ready branches found.");
        return Ok(());
    }

    println!("{} Found {} ready branch(es):", "→".cyan(), ready.len());
    for info in &ready {
        let title = info.spec_title.as_deref().unwrap_or("(no title)");
        println!("  {} {} {}", "·".cyan(), info.spec_id, title.dimmed());
    }
    println!();

    let spec_ids: Vec<String> = ready.iter().map(|info| info.spec_id.clone()).collect();

    execute_merge(
        &spec_ids,
        false, // not --all mode
        dry_run,
        delete_branch,
        continue_on_error,
        yes,
        false, // no rebase needed for ready branches
        auto_resolve,
        finalize,
        all_specs,
        config,
        branch_prefix,
        main_branch,
        specs_dir,
    )
}

/// Interactive mode to select which branches to merge
#[allow(clippy::too_many_arguments)]
fn merge_interactive(
    all_specs: &[Spec],
    branch_prefix: &str,
    main_branch: &str,
    dry_run: bool,
    delete_branch: bool,
    continue_on_error: bool,
    yes: bool,
    rebase: bool,
    auto_resolve: bool,
    finalize: bool,
    config: &Config,
    specs_dir: &Path,
) -> Result<()> {
    use dialoguer::MultiSelect;
    use merge::BranchStatus;

    let branch_infos = merge::get_branch_info_for_specs(all_specs, branch_prefix, main_branch)?;

    if branch_infos.is_empty() {
        println!("No completed specs with branches found.");
        return Ok(());
    }

    // Build display items with status indicators
    let display_items: Vec<String> = branch_infos
        .iter()
        .map(|info| {
            let title = info.spec_title.as_deref().unwrap_or("(no title)");
            let status_str = match &info.status {
                BranchStatus::Ready => "(ready)".green().to_string(),
                BranchStatus::NeedsRebase => "(needs rebase)".yellow().to_string(),
                BranchStatus::HasConflicts => "(has conflicts)".red().to_string(),
                BranchStatus::Incomplete => format!(
                    "(incomplete: {}/{})",
                    info.criteria_checked, info.criteria_total
                )
                .yellow()
                .to_string(),
                BranchStatus::NoCommits => "(no commits)".dimmed().to_string(),
                _ => "".to_string(),
            };
            format!("{} - {} {}", info.spec_id, title, status_str)
        })
        .collect();

    // Pre-select ready branches
    let defaults: Vec<bool> = branch_infos
        .iter()
        .map(|info| info.status == BranchStatus::Ready)
        .collect();

    // Show multi-select prompt
    let selection = MultiSelect::new()
        .with_prompt("Select branches to merge")
        .items(&display_items)
        .defaults(&defaults)
        .interact()?;

    if selection.is_empty() {
        println!("No branches selected");
        return Ok(());
    }

    let spec_ids: Vec<String> = selection
        .iter()
        .map(|&idx| branch_infos[idx].spec_id.clone())
        .collect();

    println!(
        "\n{} Merge {} selected branch(es)? (y/n)",
        "?".cyan(),
        spec_ids.len()
    );

    if !yes {
        use dialoguer::Confirm;
        if !Confirm::new().interact()? {
            println!("Cancelled");
            return Ok(());
        }
    }

    execute_merge(
        &spec_ids,
        false,
        dry_run,
        delete_branch,
        continue_on_error,
        yes,
        rebase,
        auto_resolve,
        finalize,
        all_specs,
        config,
        branch_prefix,
        main_branch,
        specs_dir,
    )
}

/// Run the interactive wizard for selecting specs to merge
/// Returns (selected_spec_ids, delete_branch, rebase)
fn run_merge_wizard(
    all_specs: &[Spec],
    branch_prefix: &str,
    main_branch: &str,
    delete_branch: bool,
    rebase: bool,
) -> Result<(Vec<String>, bool, bool)> {
    use dialoguer::{Confirm, MultiSelect};

    // Get completed specs that have branches and haven't been merged yet
    let mergeable_specs: Vec<(String, &Spec)> = all_specs
        .iter()
        .filter(|spec| spec.frontmatter.status == SpecStatus::Completed)
        .filter_map(|spec| {
            let branch_name = format!("{}{}", branch_prefix, spec.id);
            if git::branch_exists(&branch_name).unwrap_or(false) {
                // Check if branch has already been merged
                if git::is_branch_merged(&branch_name, main_branch).unwrap_or(false) {
                    // Skip already-merged branches
                    None
                } else {
                    Some((spec.id.clone(), spec))
                }
            } else {
                None
            }
        })
        .collect();

    // If no mergeable specs, show message and return early
    if mergeable_specs.is_empty() {
        println!("No specs to merge");
        return Ok((Vec::new(), delete_branch, rebase));
    }

    // Build display items with ID, title, and branch name
    let display_items: Vec<String> = mergeable_specs
        .iter()
        .map(|(spec_id, spec)| {
            let title = spec.title.as_deref().unwrap_or("(no title)");
            let branch_name = format!("{}{}", branch_prefix, spec_id);
            format!("{}  {}  ({})", spec_id, title, branch_name)
        })
        .collect();

    // Add "Select all" option at the end
    let mut all_items = display_items.clone();
    all_items.push("[Select all]".to_string());

    // Show multi-select prompt
    let selection = MultiSelect::new()
        .with_prompt("Select specs to merge")
        .items(&all_items)
        .interact()?;

    // Determine which specs were selected
    let selected_spec_ids: Vec<String> =
        if selection.len() == 1 && selection[0] == all_items.len() - 1 {
            // "Select all" was the only selection
            mergeable_specs.iter().map(|(id, _)| id.clone()).collect()
        } else if selection.contains(&(all_items.len() - 1)) {
            // "Select all" was selected along with other specs - treat as select all
            mergeable_specs.iter().map(|(id, _)| id.clone()).collect()
        } else {
            // Regular selections
            selection
                .iter()
                .map(|&idx| mergeable_specs[idx].0.clone())
                .collect()
        };

    if selected_spec_ids.is_empty() {
        println!("No specs selected");
        return Ok((Vec::new(), delete_branch, rebase));
    }

    // Ask about rebase strategy
    let use_rebase = Confirm::new()
        .with_prompt("Use rebase strategy")
        .default(false)
        .interact()?;

    // Ask about delete branches
    let delete_after_merge = Confirm::new()
        .with_prompt("Delete branches after merge")
        .default(true)
        .interact()?;

    Ok((selected_spec_ids, delete_after_merge, use_rebase))
}

/// Find all completed specs that have corresponding branches.
/// Used by --all-completed to find specs to merge after parallel execution.
fn find_completed_specs_with_branches(
    all_specs: &[Spec],
    branch_prefix: &str,
    main_branch: &str,
) -> Result<Vec<String>> {
    let mut completed_with_branches = Vec::new();

    for spec in all_specs {
        // Only consider completed specs
        if spec.frontmatter.status != SpecStatus::Completed {
            continue;
        }

        // Check if the branch exists
        let branch_name = format!("{}{}", branch_prefix, spec.id);
        if git::branch_exists(&branch_name).unwrap_or(false) {
            // Skip already-merged branches
            if !git::is_branch_merged(&branch_name, main_branch).unwrap_or(false) {
                completed_with_branches.push(spec.id.clone());
            }
        }
    }

    Ok(completed_with_branches)
}

/// Execute the merge operation for a list of spec IDs.
/// This is the core merge logic shared between different entry points.
#[allow(clippy::too_many_arguments)]
fn execute_merge(
    final_ids: &[String],
    all: bool,
    dry_run: bool,
    final_delete_branch: bool,
    continue_on_error: bool,
    yes: bool,
    final_rebase: bool,
    auto_resolve: bool,
    finalize: bool,
    all_specs: &[Spec],
    config: &Config,
    branch_prefix: &str,
    main_branch: &str,
    specs_dir: &Path,
) -> Result<()> {
    // Get specs to merge using the merge module function
    let mut specs_to_merge = merge::get_specs_to_merge(final_ids, all, all_specs)?;

    // Filter to only those with branches that exist (unless dry-run)
    if !dry_run {
        specs_to_merge.retain(|(spec_id, _spec)| {
            git::branch_exists(&format!("{}{}", branch_prefix, spec_id)).unwrap_or_default()
        });
    }

    if specs_to_merge.is_empty() {
        println!("No completed specs with branches to merge.");
        return Ok(());
    }

    // Check for specs requiring approval before merge
    let mut unapproved_specs: Vec<(String, String)> = Vec::new();
    for (spec_id, spec) in &specs_to_merge {
        if spec.requires_approval() {
            let title = spec.title.as_deref().unwrap_or("(no title)").to_string();
            unapproved_specs.push((spec_id.clone(), title));
        }
    }

    if !unapproved_specs.is_empty() {
        println!(
            "{} {} spec(s) require approval before merge:",
            "✗".red(),
            unapproved_specs.len()
        );
        for (spec_id, title) in &unapproved_specs {
            println!("  {} {} {}", "·".red(), spec_id, title.dimmed());
        }
        println!();
        println!(
            "Run {} to approve specs before merging.",
            "chant approve <spec-id> --by <approver>".cyan()
        );
        anyhow::bail!(
            "Cannot merge: {} spec(s) require approval",
            unapproved_specs.len()
        );
    }

    // Display what would be merged
    println!(
        "{} {} merge {} spec(s){}:",
        "→".cyan(),
        if dry_run { "Would" } else { "Will" },
        specs_to_merge.len(),
        if all { " (all completed)" } else { "" }
    );
    for (spec_id, spec) in &specs_to_merge {
        let title = spec.title.as_deref().unwrap_or("(no title)");
        let branch_name = format!("{}{}", branch_prefix, spec_id);
        println!(
            "  {} {} → {} {}",
            "·".cyan(),
            branch_name,
            main_branch,
            title.dimmed()
        );
    }
    println!();

    // If dry-run, show what would happen and exit
    if dry_run {
        println!("{} Dry-run mode: no changes made.", "ℹ".blue());
        return Ok(());
    }

    // Show confirmation prompt unless --yes or --dry-run
    if !yes {
        let confirmed = chant::prompt::confirm(&format!(
            "Proceed with merging {} spec(s)?",
            specs_to_merge.len()
        ))?;
        if !confirmed {
            println!("{} Merge cancelled.", "✗".yellow());
            return Ok(());
        }
    }

    // Sort specs to merge members before drivers
    // This ensures driver specs are merged after all their members
    let mut sorted_specs: Vec<(String, Spec)> = specs_to_merge.clone();
    sorted_specs.sort_by(|(id_a, _), (id_b, _)| {
        // Count dots in IDs - members have more dots, sort them first
        let dots_a = id_a.matches('.').count();
        let dots_b = id_b.matches('.').count();
        dots_b.cmp(&dots_a) // Reverse order: members (more dots) before drivers (fewer dots)
    });

    // Execute merges
    let mut merge_results: Vec<git::MergeResult> = Vec::new();
    let mut errors: Vec<(String, String)> = Vec::new();
    let mut _skipped_conflicts: Vec<(String, Vec<String>)> = Vec::new();

    println!(
        "{} Executing merges{}...",
        "→".cyan(),
        if final_rebase { " with rebase" } else { "" }
    );

    for (spec_id, spec) in &sorted_specs {
        let branch_name = format!("{}{}", branch_prefix, spec_id);

        // If rebase mode, rebase branch onto main first
        if final_rebase {
            println!(
                "  {} Rebasing {} onto {}...",
                "→".cyan(),
                branch_name,
                main_branch
            );

            match git::rebase_branch(&branch_name, main_branch) {
                Ok(rebase_result) => {
                    if !rebase_result.success {
                        // Rebase had conflicts
                        if auto_resolve {
                            // Try to resolve conflicts with agent
                            println!(
                                "    {} Conflict in: {}",
                                "⚠".yellow(),
                                rebase_result.conflicting_files.join(", ")
                            );
                            println!("    {} Invoking agent to resolve...", "→".cyan());

                            match resolve_conflicts_with_agent(
                                &branch_name,
                                main_branch,
                                &rebase_result.conflicting_files,
                                config,
                            ) {
                                Ok(()) => {
                                    println!("    {} Conflicts resolved", "✓".green());
                                }
                                Err(e) => {
                                    let error_msg = format!("Auto-resolve failed: {}", e);
                                    errors.push((spec_id.clone(), error_msg.clone()));
                                    _skipped_conflicts
                                        .push((spec_id.clone(), rebase_result.conflicting_files));
                                    println!("    {} {}", "✗".red(), error_msg);
                                    if !continue_on_error {
                                        anyhow::bail!("Merge stopped at spec {}.", spec_id);
                                    }
                                    continue;
                                }
                            }
                        } else {
                            // No auto-resolve, abort rebase and skip this branch
                            git::rebase_abort()?;

                            let error_msg = merge_errors::rebase_conflict(
                                spec_id,
                                &branch_name,
                                &rebase_result.conflicting_files,
                            );
                            errors.push((spec_id.clone(), error_msg.clone()));
                            _skipped_conflicts
                                .push((spec_id.clone(), rebase_result.conflicting_files));
                            println!("    {} {}", "✗".red(), error_msg);
                            if !continue_on_error {
                                anyhow::bail!("{}", merge_errors::rebase_stopped(spec_id));
                            }
                            continue;
                        }
                    }
                }
                Err(e) => {
                    let error_msg = merge_errors::generic_merge_failed(
                        spec_id,
                        &branch_name,
                        main_branch,
                        &format!("Rebase failed: {}", e),
                    );
                    errors.push((spec_id.clone(), error_msg.clone()));
                    println!("    {} {}", "✗".red(), error_msg);
                    if !continue_on_error {
                        anyhow::bail!("{}", merge_errors::merge_stopped(spec_id));
                    }
                    continue;
                }
            }
        }

        // Check if this is a driver spec
        let is_driver = merge::is_driver_spec(spec, all_specs);

        let merge_op_result = if is_driver {
            // Merge driver and its members
            merge::merge_driver_spec(
                spec,
                all_specs,
                branch_prefix,
                main_branch,
                final_delete_branch,
                false,
            )
        } else {
            // Merge single spec
            match git::merge_single_spec(
                spec_id,
                &branch_name,
                main_branch,
                final_delete_branch,
                false,
            ) {
                Ok(result) => Ok(vec![result]),
                Err(e) => Err(e),
            }
        };

        match merge_op_result {
            Ok(results) => {
                merge_results.extend(results);
            }
            Err(e) => {
                let error_msg = e.to_string();
                errors.push((spec_id.clone(), error_msg.clone()));
                println!("  {} {} failed: {}", "✗".red(), spec_id, error_msg);

                if !continue_on_error {
                    anyhow::bail!("{}", merge_errors::merge_stopped(spec_id));
                }
            }
        }
    }

    // Display results
    println!("\n{} Merge Results", "→".cyan());
    println!("{}", "─".repeat(60));

    for result in &merge_results {
        println!("{}", git::format_merge_summary(result));
    }

    // Finalize specs if --finalize flag is set
    let mut finalized_count = 0;
    let mut finalize_errors: Vec<(String, String)> = Vec::new();

    if finalize && !dry_run {
        println!("\n{} Finalizing merged specs...", "→".cyan());
        for result in &merge_results {
            if result.success {
                // Reload the spec from disk (it may have changed during merge)
                match spec::resolve_spec(specs_dir, &result.spec_id) {
                    Ok(mut spec) => {
                        // Update spec status to completed
                        spec.force_status(SpecStatus::Completed);

                        // Add completed_at if not present
                        if spec.frontmatter.completed_at.is_none() {
                            spec.frontmatter.completed_at =
                                Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
                        }

                        // Save the spec
                        let spec_path = specs_dir.join(format!("{}.md", result.spec_id));
                        match spec.save(&spec_path) {
                            Ok(_) => {
                                finalized_count += 1;
                                println!("  {} {} finalized", "✓".green(), result.spec_id);
                            }
                            Err(e) => {
                                finalize_errors.push((
                                    result.spec_id.clone(),
                                    format!("Failed to save: {}", e),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        finalize_errors.push((
                            result.spec_id.clone(),
                            format!("Failed to load spec: {}", e),
                        ));
                    }
                }
            }
        }
    }

    // Display summary
    println!("\n{} Summary", "→".cyan());
    println!("{}", "─".repeat(60));
    println!("  {} Specs merged: {}", "✓".green(), merge_results.len());
    if finalize && finalized_count > 0 {
        println!("  {} Specs finalized: {}", "✓".green(), finalized_count);
    }
    if !errors.is_empty() {
        println!("  {} Specs failed: {}", "✗".red(), errors.len());
        for (spec_id, error_msg) in &errors {
            println!("    - {}: {}", spec_id, error_msg);
        }
    }
    if !finalize_errors.is_empty() {
        println!(
            "  {} Specs failed to finalize: {}",
            "⚠".yellow(),
            finalize_errors.len()
        );
        for (spec_id, error_msg) in &finalize_errors {
            println!("    - {}: {}", spec_id, error_msg);
        }
    }
    if final_delete_branch {
        let deleted_count = merge_results.iter().filter(|r| r.branch_deleted).count();
        println!("  {} Branches deleted: {}", "✓".green(), deleted_count);
    }

    if !errors.is_empty() {
        println!("\n{}", "Some merges failed.".yellow());
        println!("\nNext steps:");
        println!("  1. Review failed specs with:  chant show <spec-id>");
        println!("  2. Retry with rebase:  chant merge --all --rebase");
        println!("  3. Auto-resolve conflicts:  chant merge --all --rebase --auto");
        println!("  4. Or merge individually:  chant merge <spec-id>");
        println!("\nDocumentation: See 'chant merge --help' for more options");
        return Ok(());
    }

    if finalize {
        if finalize_errors.is_empty() {
            println!(
                "\n{} All specs merged and finalized successfully.",
                "✓".green()
            );
        } else {
            println!("\n{}", "Some specs failed to finalize.".yellow());
            println!(
                "Run {} for failed specs.",
                "chant finalize <spec-id>".bold()
            );
        }
    } else {
        println!("\n{} All specs merged successfully.", "✓".green());
    }
    Ok(())
}

/// Merge completed spec branches back to main
#[allow(clippy::too_many_arguments)]
pub fn cmd_merge(
    ids: &[String],
    all: bool,
    all_completed: bool,
    list: bool,
    ready: bool,
    interactive: bool,
    dry_run: bool,
    delete_branch: bool,
    continue_on_error: bool,
    yes: bool,
    rebase: bool,
    auto_resolve: bool,
    finalize: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load config
    let config = Config::load()?;
    let branch_prefix = &config.defaults.branch_prefix;
    let main_branch = merge::load_main_branch(&config);

    // Ensure main repo starts on main branch
    let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

    // Load all specs first (needed for wizard and validation)
    let all_specs = spec::load_all_specs(&specs_dir)?;

    // Handle --list flag: show branch status
    if list {
        return show_branch_status(&all_specs, branch_prefix, &main_branch);
    }

    // Validate --all-completed is not used with explicit spec IDs
    if all_completed && !ids.is_empty() {
        anyhow::bail!(
            "Cannot use --all-completed with explicit spec IDs. Use either --all-completed or provide spec IDs."
        );
    }

    // Handle --ready flag: merge all ready branches
    if ready {
        return merge_ready_branches(
            &all_specs,
            branch_prefix,
            &main_branch,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            auto_resolve,
            finalize,
            &config,
            &specs_dir,
        );
    }

    // Handle -i/--interactive flag: interactive selection
    if interactive {
        return merge_interactive(
            &all_specs,
            branch_prefix,
            &main_branch,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            rebase,
            auto_resolve,
            finalize,
            &config,
            &specs_dir,
        );
    }

    // Handle --all-completed flag: find all completed specs with branches
    if all_completed {
        let completed_with_branches =
            find_completed_specs_with_branches(&all_specs, branch_prefix, &main_branch)?;

        if completed_with_branches.is_empty() {
            println!("No completed specs with branches found.");
            return Ok(());
        }

        // Print which specs will be merged
        println!(
            "{} Found {} completed spec(s) with branches:",
            "→".cyan(),
            completed_with_branches.len()
        );
        for spec_id in &completed_with_branches {
            let spec = all_specs.iter().find(|s| &s.id == spec_id);
            let title = spec
                .and_then(|s| s.title.as_deref())
                .unwrap_or("(no title)");
            println!("  {} {} {}", "·".cyan(), spec_id, title.dimmed());
        }
        println!();

        // Proceed with merging using the found spec IDs
        let final_ids = completed_with_branches;
        let final_delete_branch = delete_branch;
        let final_rebase = rebase;

        let result = execute_merge(
            &final_ids,
            false, // not --all mode
            dry_run,
            final_delete_branch,
            continue_on_error,
            yes,
            final_rebase,
            auto_resolve,
            finalize,
            &all_specs,
            &config,
            branch_prefix,
            &main_branch,
            &specs_dir,
        );

        // Ensure main repo ends on main branch
        let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

        return result;
    }

    // Handle wizard mode when no arguments provided
    let (final_ids, final_delete_branch, final_rebase) = if !all && ids.is_empty() {
        run_merge_wizard(
            &all_specs,
            branch_prefix,
            &main_branch,
            delete_branch,
            rebase,
        )?
    } else {
        (ids.to_vec(), delete_branch, rebase)
    };

    // Validate arguments after wizard
    if !all && final_ids.is_empty() {
        anyhow::bail!(
            "Please specify one or more spec IDs, or use --all to merge all completed specs"
        );
    }

    // Execute merge using shared helper
    let result = execute_merge(
        &final_ids,
        all,
        dry_run,
        final_delete_branch,
        continue_on_error,
        yes,
        final_rebase,
        auto_resolve,
        finalize,
        &all_specs,
        &config,
        branch_prefix,
        &main_branch,
        &specs_dir,
    );

    // Ensure main repo ends on main branch
    let _ = chant::git::ensure_on_main_branch(&config.defaults.main_branch);

    result
}
