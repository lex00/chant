//! Chain execution mode for specs

use anyhow::{bail, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use chant::config::Config;
use chant::spec::{self, Spec, SpecStatus};
use chant::spec_group;

use super::executor;

// CHAIN INTERRUPTION HANDLING

static CHAIN_INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set up SIGINT handler for graceful chain interruption
fn setup_chain_signal_handler() {
    CHAIN_INTERRUPTED.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = ctrlc::set_handler(move || {
        if CHAIN_INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst) {
            // Already interrupted once, force exit
            eprintln!("\n{} Force exit", "✗".red());
            std::process::exit(130);
        }
        eprintln!(
            "\n{} Interrupt received - finishing current spec before stopping...",
            "→".yellow()
        );
        eprintln!("  {} Press Ctrl+C again to force exit", "→".dimmed());
        CHAIN_INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst);
    });
}

/// Check if chain execution was interrupted
fn is_chain_interrupted() -> bool {
    CHAIN_INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst)
}

// SPEC DISCOVERY AND FILTERING

/// Check if a spec is a driver/group spec
fn is_driver_or_group_spec(spec: &Spec, all_specs: &[Spec]) -> bool {
    // Check if spec has type "group" or "driver"
    if spec.frontmatter.r#type == "group" || spec.frontmatter.r#type == "driver" {
        return true;
    }
    // Check if spec has members (i.e., other specs that are children of this spec)
    !spec_group::get_members(&spec.id, all_specs).is_empty()
}

// TOPOLOGICAL SORTING FOR GROUPS

/// Topologically sort groups by their driver `depends_on` fields.
/// Returns groups in execution order (IDs of driver specs).
/// Uses Kahn's algorithm for topological sort with cycle detection.
fn topological_sort_groups(all_specs: &[Spec]) -> Result<Vec<String>> {
    // Identify all driver/group specs
    let driver_specs: Vec<&Spec> = all_specs
        .iter()
        .filter(|s| is_driver_or_group_spec(s, all_specs))
        .collect();

    // Build dependency graph: driver_id -> set of drivers it depends on
    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut all_drivers: HashSet<String> = HashSet::new();

    for driver in &driver_specs {
        all_drivers.insert(driver.id.clone());
        graph.entry(driver.id.clone()).or_default();
        in_degree.entry(driver.id.clone()).or_insert(0);

        if let Some(deps) = &driver.frontmatter.depends_on {
            for dep in deps {
                // Only consider dependencies on other drivers (not member specs)
                if all_drivers.contains(dep) || driver_specs.iter().any(|d| &d.id == dep) {
                    graph
                        .entry(dep.clone())
                        .or_default()
                        .insert(driver.id.clone());
                    *in_degree.entry(driver.id.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Kahn's algorithm: start with nodes that have no incoming edges
    let queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &degree)| degree == 0)
        .map(|(id, _)| id.clone())
        .collect();

    // Sort initial queue by spec ID for consistent ordering
    let mut queue_vec: Vec<String> = queue.into_iter().collect();
    queue_vec.sort_by(|a, b| spec_group::compare_spec_ids(a, b));
    let mut queue: VecDeque<String> = queue_vec.into();

    let mut sorted: Vec<String> = Vec::new();

    while let Some(node) = queue.pop_front() {
        sorted.push(node.clone());

        // Reduce in-degree for all dependent nodes
        if let Some(dependents) = graph.get(&node) {
            let mut ready_nodes = Vec::new();
            for dependent in dependents {
                if let Some(degree) = in_degree.get_mut(dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready_nodes.push(dependent.clone());
                    }
                }
            }
            // Sort ready nodes by ID before adding to queue
            ready_nodes.sort_by(|a, b| spec_group::compare_spec_ids(a, b));
            queue.extend(ready_nodes);
        }
    }

    // Check for cycles: if sorted length < all drivers, there's a cycle
    if sorted.len() < all_drivers.len() {
        let remaining: Vec<String> = all_drivers
            .iter()
            .filter(|id| !sorted.contains(id))
            .cloned()
            .collect();
        bail!(
            "Circular dependency detected in driver specs: {}",
            remaining.join(", ")
        );
    }

    Ok(sorted)
}

/// Find the next ready spec respecting filters, group boundaries, and topological order
fn find_next_ready_spec(
    specs_dir: &Path,
    labels: &[String],
    skip_spec_id: Option<&str>,
    active_group: Option<&str>,
    group_order: &[String],
) -> Result<Option<Spec>> {
    let all_specs = spec::load_all_specs(specs_dir)?;

    // Filter to ready specs
    let mut ready_specs: Vec<Spec> = all_specs
        .iter()
        .filter(|s| {
            // Exclude cancelled specs
            s.frontmatter.status != SpecStatus::Cancelled
                // Must be ready (dependencies satisfied)
                && s.is_ready(&all_specs)
                // Skip the specified spec (if any - used when a specific starting spec was provided)
                && skip_spec_id.is_none_or(|id| s.id != id)
                // Skip driver/group specs - they should not be executed directly
                && !is_driver_or_group_spec(s, &all_specs)
        })
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

    // If there's an active group, prefer members of that group
    if let Some(driver_id) = active_group {
        // Find ready members of the active group and sort by ID
        let mut group_members: Vec<Spec> = ready_specs
            .iter()
            .filter(|s| spec_group::extract_driver_id(&s.id).as_deref() == Some(driver_id))
            .cloned()
            .collect();

        if !group_members.is_empty() {
            group_members.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));
            return Ok(Some(group_members.into_iter().next().unwrap()));
        }
    }

    // Sort ready specs by topological group order, then by spec ID within each group
    ready_specs.sort_by(|a, b| {
        let a_driver = spec_group::extract_driver_id(&a.id);
        let b_driver = spec_group::extract_driver_id(&b.id);

        match (a_driver, b_driver) {
            (Some(ref a_grp), Some(ref b_grp)) => {
                // Both are members: compare by group position in topological order
                let a_pos = group_order.iter().position(|g| g == a_grp);
                let b_pos = group_order.iter().position(|g| g == b_grp);
                match (a_pos, b_pos) {
                    (Some(ap), Some(bp)) => {
                        if ap != bp {
                            ap.cmp(&bp)
                        } else {
                            // Same group: use spec ID order
                            spec_group::compare_spec_ids(&a.id, &b.id)
                        }
                    }
                    (Some(_), None) => std::cmp::Ordering::Greater, // a in topo order, b not
                    (None, Some(_)) => std::cmp::Ordering::Less,    // b in topo order, a not
                    (None, None) => spec_group::compare_spec_ids(&a.id, &b.id),
                }
            }
            (Some(ref a_grp), None) => {
                // a is a member, b is standalone
                // Compare a's group position with b's position in standalone specs
                let a_pos = group_order.iter().position(|g| g == a_grp);
                match a_pos {
                    Some(_) => {
                        // Both exist: compare group order position vs spec's own dependency order
                        // For simplicity, interleave by using spec ID as tiebreaker
                        spec_group::compare_spec_ids(&a.id, &b.id)
                    }
                    None => spec_group::compare_spec_ids(&a.id, &b.id),
                }
            }
            (None, Some(ref b_grp)) => {
                // a is standalone, b is a member
                let b_pos = group_order.iter().position(|g| g == b_grp);
                match b_pos {
                    Some(_) => spec_group::compare_spec_ids(&a.id, &b.id),
                    None => spec_group::compare_spec_ids(&a.id, &b.id),
                }
            }
            (None, None) => {
                // Both are standalone: use spec ID order
                spec_group::compare_spec_ids(&a.id, &b.id)
            }
        }
    });

    // Return the first ready spec
    Ok(ready_specs.into_iter().next())
}

/// Count total ready specs matching filters
fn count_ready_specs(specs_dir: &Path, labels: &[String]) -> Result<usize> {
    let all_specs = spec::load_all_specs(specs_dir)?;

    let mut ready_specs: Vec<&Spec> = all_specs
        .iter()
        .filter(|s| s.frontmatter.status != SpecStatus::Cancelled && s.is_ready(&all_specs))
        .collect();

    if !labels.is_empty() {
        ready_specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    Ok(ready_specs.len())
}

// CHAIN EXECUTION

/// Execute a single spec in chain mode
#[allow(clippy::too_many_arguments)]
fn execute_single_spec_in_chain(
    spec_id: &str,
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    prompt_name: Option<&str>,
    skip_deps: bool,
    skip_criteria: bool,
    allow_no_commits: bool,
    skip_approval: bool,
) -> Result<()> {
    let mut spec = spec::resolve_spec(specs_dir, spec_id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    let validation_opts = executor::ValidationOptions {
        skip_deps,
        skip_criteria,
        skip_approval,
        skip_quality: true,
    };

    let resolved_prompt_name = executor::prepare_spec_for_execution(
        &mut spec,
        &spec_path,
        specs_dir,
        prompts_dir,
        config,
        prompt_name,
        &validation_opts,
    )?;

    let result =
        executor::invoke_agent_for_spec(&spec, &resolved_prompt_name, prompts_dir, config, None);

    match result {
        Ok(agent_output) => {
            executor::write_agent_status_done(specs_dir, &spec.id, allow_no_commits)?;
            let mut spec = spec::resolve_spec(specs_dir, &spec.id)?;
            let commits = executor::collect_commits_for_spec(&spec, allow_no_commits)?;
            executor::handle_acceptance_criteria(&mut spec, &spec_path, skip_criteria)?;
            executor::finalize_completed_spec(
                &mut spec,
                specs_dir,
                config,
                commits,
                allow_no_commits,
            )?;
            executor::cleanup_completed_spec(&mut spec, &spec_path, &agent_output)?;
            Ok(())
        }
        Err(e) => {
            executor::handle_spec_failure(&spec.id, specs_dir, &e)?;
            Err(e)
        }
    }
}

/// Chain execution mode: loop through ready specs until none remain or failure
pub fn cmd_work_chain(
    specs_dir: &Path,
    prompts_dir: &Path,
    config: &Config,
    options: ChainOptions,
) -> Result<()> {
    use std::time::Instant;

    setup_chain_signal_handler();

    // Prepare spec iterator
    let resolved_specs = if !options.specific_ids.is_empty() {
        // Validate specific IDs upfront
        options
            .specific_ids
            .iter()
            .map(|id| spec::resolve_spec(specs_dir, id))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    let total = if !resolved_specs.is_empty() {
        resolved_specs.len()
    } else {
        count_ready_specs(specs_dir, options.labels)?
    };

    if total == 0 {
        println!("No ready specs to execute.");
        return Ok(());
    }

    println!(
        "\n{} Starting chain execution ({} ready specs)...\n",
        "→".cyan(),
        total
    );

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut completed = 0;
    let mut skipped = 0;
    let mut failed_specs: Vec<(String, String)> = Vec::new();
    let mut failed_groups: std::collections::HashSet<String> = std::collections::HashSet::new();
    let start_time = Instant::now();
    let mut all_specs = spec::load_all_specs(specs_dir)?;
    let mut active_group: Option<String> = None;

    // Compute topological order of groups at startup
    let group_order = topological_sort_groups(&all_specs)?;

    loop {
        if is_chain_interrupted() {
            break;
        }

        if options.max_specs > 0 && completed >= options.max_specs {
            println!(
                "\n{} Reached maximum chain limit ({})",
                "✓".green(),
                options.max_specs
            );
            break;
        }

        // Get next spec: from list or find next ready
        let spec = if !resolved_specs.is_empty() {
            let idx = completed + skipped;
            if idx >= resolved_specs.len() {
                break;
            }
            let spec_id = &resolved_specs[idx].id;
            all_specs
                .iter()
                .find(|s| &s.id == spec_id)
                .cloned()
                .unwrap_or_else(|| resolved_specs[idx].clone())
        } else {
            match find_next_ready_spec(
                specs_dir,
                options.labels,
                None,
                active_group.as_deref(),
                &group_order,
            )? {
                Some(s) => s,
                None => break,
            }
        };

        // Skip if not ready, already completed, or blocked by failure
        if should_skip_spec(&spec, &all_specs, &options, &failed_groups) {
            print_skip_reason(&spec, &all_specs, &options, &failed_groups);
            skipped += 1;
            continue;
        }

        // Track group membership for group-aware sequencing
        if let Some(driver_id) = spec_group::extract_driver_id(&spec.id) {
            // This is a member spec - set or maintain the active group
            active_group = Some(driver_id);
        } else {
            // This is a standalone spec - clear the active group
            active_group = None;
        }

        pb.set_message(format!(
            "{}: {}",
            spec.id,
            spec.title.as_deref().unwrap_or("")
        ));

        let spec_start = Instant::now();
        match execute_single_spec_in_chain(
            &spec.id,
            specs_dir,
            prompts_dir,
            config,
            options.prompt_name,
            options.skip_deps,
            options.skip_criteria,
            options.allow_no_commits,
            options.skip_approval,
        ) {
            Ok(()) => {
                let elapsed = spec_start.elapsed();
                pb.inc(1);
                pb.println(format!(
                    "{} Completed {} in {:.1}s",
                    "✓".green(),
                    spec.id,
                    elapsed.as_secs_f64()
                ));
                completed += 1;
                all_specs = spec::load_all_specs(specs_dir)?;

                // Check if the active group is now complete
                if let Some(ref driver_id) = active_group {
                    if spec_group::all_members_completed(driver_id, &all_specs) {
                        // All members of this group are done - clear the active group
                        active_group = None;
                    }
                }
            }
            Err(e) => {
                pb.println(format!("{} Failed {}: {}", "✗".red(), spec.id, e));
                failed_specs.push((spec.id.clone(), e.to_string()));
                // Mark the group as failed so dependent members/groups are skipped
                if let Some(driver_id) = spec_group::extract_driver_id(&spec.id) {
                    failed_groups.insert(driver_id);
                    active_group = None;
                }
                all_specs = spec::load_all_specs(specs_dir)?;
            }
        }
    }

    pb.finish_and_clear();
    print_chain_summary(
        completed,
        skipped,
        &failed_specs,
        &failed_groups,
        start_time.elapsed(),
    );

    if !failed_specs.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

/// Check if a spec should be skipped
fn should_skip_spec(
    spec: &Spec,
    all_specs: &[Spec],
    options: &ChainOptions,
    failed_groups: &std::collections::HashSet<String>,
) -> bool {
    if spec.frontmatter.status == SpecStatus::Cancelled {
        return true;
    }
    if spec.frontmatter.status == SpecStatus::Completed
        && !(options.skip_deps || options.skip_criteria)
    {
        return true;
    }
    if !spec.is_ready(all_specs) && !options.skip_deps {
        return true;
    }
    // Skip driver/group specs - they should not be executed directly
    if is_driver_or_group_spec(spec, all_specs) {
        return true;
    }
    // Skip specs whose group has failed
    if let Some(driver_id) = spec_group::extract_driver_id(&spec.id) {
        if failed_groups.contains(&driver_id) {
            return true;
        }
    }
    // Skip specs that depend on a failed group's driver
    if let Some(deps) = &spec.frontmatter.depends_on {
        for dep in deps {
            if failed_groups.contains(dep) {
                return true;
            }
        }
    }
    false
}

/// Print reason for skipping a spec
fn print_skip_reason(
    spec: &Spec,
    all_specs: &[Spec],
    options: &ChainOptions,
    failed_groups: &std::collections::HashSet<String>,
) {
    if spec.frontmatter.status == SpecStatus::Cancelled {
        println!("{} Skipping {}: cancelled", "⚠".yellow(), spec.id);
    } else if spec.frontmatter.status == SpecStatus::Completed {
        println!("{} Skipping {}: already completed", "⚠".yellow(), spec.id);
    } else if let Some(driver_id) = spec_group::extract_driver_id(&spec.id) {
        if failed_groups.contains(&driver_id) {
            println!(
                "{} Skipping {}: group {} has failures",
                "⚠".yellow(),
                spec.id,
                driver_id
            );
        }
    } else if !spec.is_ready(all_specs) && !options.skip_deps {
        println!(
            "{} Skipping {}: not ready (dependencies not satisfied)",
            "⚠".yellow(),
            spec.id
        );
    } else if is_driver_or_group_spec(spec, all_specs) {
        println!(
            "{} Skipping {}: driver/group spec (execute members instead)",
            "⚠".yellow(),
            spec.id
        );
    }
}

/// Print chain execution summary
fn print_chain_summary(
    completed: usize,
    skipped: usize,
    failed_specs: &[(String, String)],
    failed_groups: &std::collections::HashSet<String>,
    elapsed: std::time::Duration,
) {
    println!("{}", "═".repeat(60).dimmed());
    println!("{}", "Chain execution complete:".bold());
    println!(
        "  {} Completed {} spec(s) in {:.1}s",
        "✓".green(),
        completed,
        elapsed.as_secs_f64()
    );

    if skipped > 0 {
        println!("  {} Skipped {} spec(s)", "→".yellow(), skipped);
    }

    if !failed_specs.is_empty() {
        println!("  {} Failed {} spec(s):", "✗".red(), failed_specs.len());
        for (spec_id, error) in failed_specs {
            println!("    {} {}: {}", "✗".red(), spec_id, error);
        }
    }

    if !failed_groups.is_empty() {
        println!(
            "  {} Affected groups: {}",
            "⚠".yellow(),
            failed_groups.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    if is_chain_interrupted() {
        println!("  {} Interrupted by user", "→".yellow());
    }

    println!("{}", "═".repeat(60).dimmed());
}

// CHAIN OPTIONS STRUCT

/// Options for chain execution mode
pub struct ChainOptions<'a> {
    /// Maximum number of specs to chain (0 = unlimited)
    pub max_specs: usize,
    /// Labels to filter specs (ignored when specific_ids is not empty)
    pub labels: &'a [String],
    /// Prompt name override
    pub prompt_name: Option<&'a str>,
    /// Skip dependency checks
    pub skip_deps: bool,
    /// Skip acceptance criteria validation
    pub skip_criteria: bool,
    /// Allow spec completion without matching commits
    pub allow_no_commits: bool,
    /// Skip approval check
    pub skip_approval: bool,
    /// Specific spec IDs to chain through (if empty, chains through all ready specs)
    pub specific_ids: &'a [String],
}
