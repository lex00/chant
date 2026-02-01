//! Lifecycle command handlers for chant CLI
//!
//! Handles lower-volume but logically related lifecycle operations:
//! - Spec merging and archiving
//! - Spec splitting into member specs
//! - Diagnostic information for spec execution issues
//! - Log file retrieval and display
//!
//! Note: Core spec operations (add, list, show) are in cmd::spec module

use anyhow::{Context, Result};
use chrono::Local;
use colored::Colorize;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::diagnose;
use chant::git;
use chant::merge;
use chant::merge_errors;
use chant::paths::{ARCHIVE_DIR, PROMPTS_DIR};
use chant::prompt;
use chant::replay::ReplayContext;
use chant::score::isolation::calculate_isolation;
use chant::score::splittability::calculate_splittability;
use chant::score::traffic_light::{determine_status, generate_suggestions};
use chant::scoring::{SpecScore, SplittabilityGrade};
use chant::spec::{self, Spec, SpecFrontmatter, SpecStatus};

use crate::cmd;

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/// Display detailed diagnostic information for a spec
pub fn cmd_diagnose(id: &str) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve spec ID
    let spec = spec::resolve_spec(&specs_dir, id)?;

    // Run diagnostics
    let report = diagnose::diagnose_spec(&spec.id)?;

    // Display report
    println!("\n{}", format!("Spec: {}", report.spec_id).cyan().bold());
    let status_str = match report.status {
        SpecStatus::Pending => "pending".white(),
        SpecStatus::InProgress => "in_progress".yellow(),
        SpecStatus::Completed => "completed".green(),
        SpecStatus::Failed => "failed".red(),
        SpecStatus::NeedsAttention => "needs_attention".yellow(),
        SpecStatus::Ready => "ready".cyan(),
        SpecStatus::Blocked => "blocked".red(),
        SpecStatus::Cancelled => "cancelled".dimmed(),
    };
    println!("Status: {}", status_str);

    println!("\n{}:", "Checks".bold());
    for check in &report.checks {
        let icon = if check.passed {
            "✓".green()
        } else {
            "✗".red()
        };
        print!("  {} {}", icon, check.name);
        if let Some(details) = &check.details {
            println!(" ({})", details.bright_black());
        } else {
            println!();
        }
    }

    println!("\n{}:", "Diagnosis".bold());
    println!("  {}", report.diagnosis);

    if let Some(suggestion) = &report.suggestion {
        println!("\n{}:", "Suggestion".bold());
        println!("  {}", suggestion);
    }

    Ok(())
}

// ============================================================================
// LOGGING
// ============================================================================

/// Show log for a spec (uses default .chant directory)
pub fn cmd_log(id: &str, lines: usize, follow: bool) -> Result<()> {
    cmd_log_at(&PathBuf::from(".chant"), id, lines, follow)
}

/// Show log for a spec with custom base path (useful for testing)
pub fn cmd_log_at(base_path: &std::path::Path, id: &str, lines: usize, follow: bool) -> Result<()> {
    let specs_dir = base_path.join("specs");
    let logs_dir = base_path.join("logs");

    // Note: For custom base paths, we check specs_dir directly instead of using ensure_initialized()
    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Resolve spec ID to get the full ID
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if !log_path.exists() {
        println!(
            "{} No log file found for spec '{}'.",
            "⚠".yellow(),
            spec.id.cyan()
        );
        println!("\nLogs are created when a spec is executed with `chant work`.");
        println!("Log path: {}", log_path.display());
        return Ok(());
    }

    // Use tail command to show/follow the log
    let mut args = vec!["-n".to_string(), lines.to_string()];

    if follow {
        args.push("-f".to_string());
    }

    args.push(log_path.to_string_lossy().to_string());

    let status = std::process::Command::new("tail")
        .args(&args)
        .status()
        .context("Failed to run tail command")?;

    if !status.success() {
        anyhow::bail!("tail command exited with status: {}", status);
    }

    Ok(())
}

// ============================================================================
// SPLITTING
// ============================================================================

/// Show complexity analysis for a spec before splitting
fn show_complexity_analysis(spec: &Spec) {
    // Thresholds for complexity
    const CRITERIA_THRESHOLD: usize = 5;
    const FILES_THRESHOLD: usize = 5;
    const WORDS_THRESHOLD: usize = 500;

    // Complexity thresholds for "simple" specs (haiku-friendly)
    const HAIKU_CRITERIA_TARGET: usize = 5;
    const HAIKU_FILES_TARGET: usize = 5;
    const HAIKU_WORDS_TARGET: usize = 200;

    let criteria_count = spec.count_total_checkboxes();
    let files_count = spec
        .frontmatter
        .target_files
        .as_ref()
        .map(|f| f.len())
        .unwrap_or(0);
    let word_count = spec.body.split_whitespace().count();

    // Check if complex (exceeds thresholds)
    let is_too_complex = criteria_count > CRITERIA_THRESHOLD
        || files_count > FILES_THRESHOLD
        || word_count > WORDS_THRESHOLD;

    if is_too_complex {
        println!("\n{} Analyzing spec complexity...", "→".cyan());
        println!(
            "  Current: {} criteria, {} files, {} words (too complex for haiku)\n",
            criteria_count, files_count, word_count
        );
        println!("{} Splitting into haiku-friendly specs...", "→".cyan());
        println!(
            "  Target per member: ≤{} criteria, ≤{} files, ≤{} words\n",
            HAIKU_CRITERIA_TARGET, HAIKU_FILES_TARGET, HAIKU_WORDS_TARGET
        );
    }
}

/// Member spec extracted from split analysis
#[derive(Debug, Clone)]
struct MemberSpec {
    title: String,
    description: String,
    target_files: Option<Vec<String>>,
    dependencies: Vec<usize>, // Member numbers this depends on (1-indexed)
}

/// Result of dependency analysis from split prompt
#[derive(Debug, Clone)]
struct DependencyAnalysis {
    /// The dependency graph as text (for display)
    graph_text: String,
    /// Dependency edges: (from_member_num, to_member_num)
    #[allow(dead_code)]
    edges: Vec<(usize, usize)>,
}

/// Split a pending spec into member specs
pub fn cmd_split(
    id: &str,
    override_model: Option<&str>,
    force: bool,
    recursive: bool,
    max_depth: usize,
) -> Result<()> {
    cmd_split_impl(id, override_model, force, recursive, max_depth, 0)
}

/// Internal implementation of split with depth tracking
fn cmd_split_impl(
    id: &str,
    override_model: Option<&str>,
    force: bool,
    recursive: bool,
    max_depth: usize,
    current_depth: usize,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let prompts_dir = PathBuf::from(PROMPTS_DIR);
    let config = Config::load()?;

    // Resolve the spec to split
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Check spec status before splitting
    if !force {
        match spec.frontmatter.status {
            SpecStatus::InProgress => {
                anyhow::bail!("Cannot split spec that is in progress");
            }
            SpecStatus::Completed => {
                anyhow::bail!("Cannot split completed spec");
            }
            SpecStatus::Failed => {
                anyhow::bail!("Cannot split failed spec");
            }
            SpecStatus::NeedsAttention => {
                anyhow::bail!("Cannot split spec that needs attention");
            }
            SpecStatus::Blocked => {
                anyhow::bail!("Cannot split blocked spec");
            }
            SpecStatus::Cancelled => {
                anyhow::bail!("Cannot split cancelled spec");
            }
            SpecStatus::Pending | SpecStatus::Ready => {
                // Allowed to split
            }
        }
    }

    // Check if already a group
    if spec.frontmatter.r#type == "group" {
        anyhow::bail!("Spec is already split");
    }

    // Calculate splittability grade before splitting
    let splittability_grade = calculate_splittability(&spec);

    // Warn if Grade C or D and allow user to proceed with confirmation
    if matches!(
        splittability_grade,
        SplittabilityGrade::C | SplittabilityGrade::D
    ) {
        let warning_level = if matches!(splittability_grade, SplittabilityGrade::D) {
            "🔴 STRONG WARNING"
        } else {
            "🟡 WARNING"
        };

        eprintln!(
            "\n{}: Splittability Grade {}",
            warning_level, splittability_grade
        );

        let suggestion = match splittability_grade {
            SplittabilityGrade::D => "This spec has tight coupling or circular dependencies. Splitting may not be effective.",
            SplittabilityGrade::C => "This spec may not split well. Consider if splitting is appropriate.",
            _ => unreachable!(),
        };

        eprintln!("  {}", suggestion);

        // Prompt user to confirm they want to proceed
        if atty::is(atty::Stream::Stdin) {
            let should_proceed = dialoguer::Confirm::new()
                .with_prompt("Do you want to proceed with splitting anyway?")
                .default(false)
                .interact()?;

            if !should_proceed {
                println!("\nSplit cancelled.");
                return Ok(());
            }
        } else {
            // Non-interactive mode: bail on Grade D, proceed on Grade C
            if matches!(splittability_grade, SplittabilityGrade::D) {
                anyhow::bail!("Cannot split: Splittability Grade D (tightly coupled). Use --force to override.");
            }
        }
    }

    // Show complexity analysis
    show_complexity_analysis(&spec);

    println!("{} Analyzing spec {} for splitting...", "→".cyan(), spec.id);

    // Load prompt from file
    let split_prompt_path = prompts_dir.join("split.md");
    if !split_prompt_path.exists() {
        anyhow::bail!("Split prompt not found: split.md");
    }

    // Assemble prompt for split analysis
    let split_prompt = prompt::assemble(&spec, &split_prompt_path, &config)?;

    // Get the model to use for split
    let model = get_model_for_split(
        override_model,
        config.defaults.model.as_deref(),
        config.defaults.split_model.as_deref(),
    );

    // Invoke agent to propose split
    let agent_output = cmd::agent::invoke_agent_with_model(
        &split_prompt,
        &spec,
        "split",
        &config,
        Some(&model),
        None,
    )?;

    // Parse member specs and dependency analysis from agent output
    let (dep_analysis, members) = parse_split_output(&agent_output)?;

    if members.is_empty() {
        anyhow::bail!("Agent did not propose any member specs. Check the agent output in the log.");
    }

    // Display dependency analysis if present
    if let Some(ref analysis) = dep_analysis {
        println!("\n{} Dependency Analysis:", "→".cyan());
        println!("{}", analysis.graph_text);
        println!();
    }

    println!(
        "{} Creating {} member specs for spec {}",
        "→".cyan(),
        members.len(),
        spec.id
    );

    // Validate members meet complexity thresholds and collect quality metrics
    const HAIKU_CRITERIA_TARGET: usize = 5;
    const HAIKU_FILES_TARGET: usize = 5;
    const HAIKU_WORDS_TARGET: usize = 200;

    let mut over_complex_count = 0;
    let mut quality_issues = Vec::new();

    for (index, member) in members.iter().enumerate() {
        let member_number = index + 1;
        let criteria_count = member.description.matches("- [ ]").count()
            + member.description.matches("- [x]").count()
            + member.description.matches("- [X]").count();
        let files_count = member.target_files.as_ref().map(|f| f.len()).unwrap_or(0);
        let word_count = member.description.split_whitespace().count();

        // Log warnings if member exceeds targets
        let is_over_complex = criteria_count > HAIKU_CRITERIA_TARGET
            || files_count > HAIKU_FILES_TARGET
            || word_count > HAIKU_WORDS_TARGET;

        if is_over_complex {
            over_complex_count += 1;
            eprintln!(
                "  {} Member {}: {} criteria, {} files, {} words (exceeds targets)",
                "⚠".yellow(),
                member_number,
                criteria_count,
                files_count,
                word_count
            );
        }
    }

    // Warn if ALL members exceed complexity thresholds
    if over_complex_count == members.len() && members.len() > 1 {
        eprintln!(
            "\n  {} WARNING: All {} members exceed complexity thresholds!",
            "⚠".yellow(),
            members.len()
        );
        eprintln!("  Consider re-splitting with --recursive flag (future feature)");
        quality_issues.push("All members over-complex".to_string());
    }

    // Create member spec files with DAG-based dependencies
    let driver_id = spec.id.clone();
    for (index, member) in members.iter().enumerate() {
        let member_number = index + 1;
        let member_id = format!("{}.{}", driver_id, member_number);
        let member_filename = format!("{}.md", member_id);
        let member_path = specs_dir.join(&member_filename);

        // Use dependencies from member spec (from Dependencies: field or extracted from DAG)
        // Always set depends_on (even if empty) to indicate explicit DAG ordering
        // This prevents fallback to sequential member ordering in is_ready()
        let depends_on = Some(
            member
                .dependencies
                .iter()
                .map(|dep_num| format!("{}.{}", driver_id, dep_num))
                .collect(),
        );

        let member_frontmatter = SpecFrontmatter {
            r#type: "code".to_string(),
            status: SpecStatus::Pending,
            depends_on,
            target_files: member.target_files.clone(),
            ..Default::default()
        };

        // Build body with title and description
        // If description already contains ### Acceptance Criteria, don't append generic ones
        let body = if member.description.contains("### Acceptance Criteria") {
            format!("# {}\n\n{}", member.title, member.description)
        } else {
            // No acceptance criteria found, append generic section
            format!(
                "# {}\n\n{}\n\n## Acceptance Criteria\n\n- [ ] Implement as described\n- [ ] All tests pass",
                member.title,
                member.description
            )
        };

        let member_spec = Spec {
            id: member_id.clone(),
            frontmatter: member_frontmatter,
            title: Some(member.title.clone()),
            body,
        };

        member_spec.save(&member_path)?;
        println!("  {} {}", "✓".green(), member_id);
    }

    // Update driver spec to type: group with depends_on all members
    spec.frontmatter.r#type = "group".to_string();
    let member_ids: Vec<String> = (1..=members.len())
        .map(|i| format!("{}.{}", driver_id, i))
        .collect();
    spec.frontmatter.depends_on = Some(member_ids.clone());
    spec.save(&spec_path)?;

    println!(
        "\n{} Split complete! Driver spec {} is now type: group",
        "✓".green(),
        spec.id
    );
    println!("Members:");
    for i in 1..=members.len() {
        println!("  • {}.{}", spec.id, i);
    }

    // Detect infrastructure ordering issues
    detect_infrastructure_issues(&members, &mut quality_issues);

    // Auto-lint member specs to validate they pass complexity checks
    println!("\n{} Running lint on member specs...", "→".cyan());

    let member_ids: Vec<String> = (1..=members.len())
        .map(|i| format!("{}.{}", spec.id, i))
        .collect();

    let lint_result = cmd::spec::lint_specific_specs(&specs_dir, &member_ids)?;

    let total_members = member_ids.len();
    let summary = if lint_result.failed > 0 {
        format!(
            "All {} members checked. {} passed, {} warned, {} failed.",
            total_members, lint_result.passed, lint_result.warned, lint_result.failed
        )
    } else if lint_result.warned > 0 {
        format!(
            "All {} members checked. {} passed, {} warned.",
            total_members, lint_result.passed, lint_result.warned
        )
    } else {
        format!("All {} members checked. All passed ✓", total_members)
    };

    println!("{} {}", "→".cyan(), summary);

    // Calculate isolation score for the resulting group
    // Reload all specs to include the newly created member specs
    let all_specs = spec::load_all_specs(&specs_dir)?;
    let driver_spec = all_specs
        .iter()
        .find(|s| s.id == spec.id)
        .context("Driver spec not found after split")?;

    if let Some(isolation_grade) = calculate_isolation(driver_spec, &all_specs) {
        // Calculate isolation percentage for display
        let member_specs: Vec<&Spec> = all_specs
            .iter()
            .filter(|s| {
                s.id.starts_with(&format!("{}.", spec.id)) && s.id.matches('.').count() == 1
            })
            .collect();

        let isolation_percentage = calculate_isolation_percentage(&member_specs);
        let shared_file_count = count_shared_files(&member_specs);
        let total_files = count_total_unique_files(&member_specs);

        // Determine traffic light and suggestions
        let mock_score = SpecScore {
            isolation: Some(isolation_grade),
            ..Default::default()
        };
        let traffic_light = determine_status(&mock_score);
        let suggestions = generate_suggestions(&mock_score);

        // Display isolation scoring
        println!("\n{} Split quality: {}", "→".cyan(), traffic_light);

        let isolation_detail = if shared_file_count > 0 {
            format!(
                "Member isolation: {:.0}% ({} of {} members share {} file{})",
                isolation_percentage,
                member_specs.len()
                    - (isolation_percentage / 100.0 * member_specs.len() as f64).round() as usize,
                member_specs.len(),
                shared_file_count,
                if shared_file_count == 1 { "" } else { "s" }
            )
        } else if total_files > 0 {
            format!(
                "Member isolation: {:.0}% (No shared files)",
                isolation_percentage
            )
        } else {
            format!(
                "Member isolation: {:.0}% (No files specified)",
                isolation_percentage
            )
        };

        println!("  {}", isolation_detail);

        if !suggestions.is_empty() {
            println!("\nSuggestion: {}", suggestions.join("; "));
        }
    }

    // Display split quality report
    if dep_analysis.is_some() || !quality_issues.is_empty() {
        display_split_quality_report(&members, &dep_analysis, &quality_issues);
    }

    // Handle recursive split if requested and members are over-complex
    if recursive && over_complex_count == members.len() && members.len() > 1 {
        if current_depth >= max_depth {
            eprintln!(
                "\n{} Max recursion depth {} reached. Not splitting further.",
                "⚠".yellow(),
                max_depth
            );
        } else {
            println!(
                "\n{} All members exceed complexity thresholds. Recursively splitting...",
                "→".cyan()
            );

            // Recursively split each over-complex member
            for i in 1..=members.len() {
                let member_id = format!("{}.{}", spec.id, i);
                println!("\n{} Splitting member {}", "→".cyan(), member_id);

                // Recursively split this member
                if let Err(e) = cmd_split_impl(
                    &member_id,
                    override_model,
                    true, // force split even if pending
                    recursive,
                    max_depth,
                    current_depth + 1,
                ) {
                    eprintln!(
                        "  {} Failed to split member {}: {}",
                        "⚠".yellow(),
                        member_id,
                        e
                    );
                }
            }

            println!(
                "\n{} Recursive split complete at depth {}",
                "✓".green(),
                current_depth + 1
            );
        }
    }

    Ok(())
}

/// Get the model to use for split operations.
/// Resolution order:
/// 1. --model flag (if provided)
/// 2. CHANT_SPLIT_MODEL env var
/// 3. defaults.split_model from config
/// 4. CHANT_MODEL env var (fallback to general model)
/// 5. defaults.model from config
/// 6. Hardcoded default: "sonnet"
fn get_model_for_split(
    flag_model: Option<&str>,
    config_model: Option<&str>,
    config_split_model: Option<&str>,
) -> String {
    // 1. --model flag
    if let Some(model) = flag_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 2. CHANT_SPLIT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_SPLIT_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 3. defaults.split_model from config
    if let Some(model) = config_split_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 4. CHANT_MODEL env var (fallback to general model)
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 5. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 6. Hardcoded default
    "sonnet".to_string()
}

/// Parse split analysis output (new format with dependency analysis)
fn parse_split_output(output: &str) -> Result<(Option<DependencyAnalysis>, Vec<MemberSpec>)> {
    // Try to extract dependency analysis section
    let dep_analysis = extract_dependency_analysis(output);

    // Parse member specs
    let members = parse_member_specs_from_output(output)?;

    Ok((dep_analysis, members))
}

/// Extract dependency analysis from output
fn extract_dependency_analysis(output: &str) -> Option<DependencyAnalysis> {
    // Look for "# Dependency Analysis" section
    let mut in_dep_section = false;
    let mut dep_text = String::new();
    let mut in_graph = false;
    let mut graph_text = String::new();

    for line in output.lines() {
        if line.starts_with("# Dependency Analysis") {
            in_dep_section = true;
            continue;
        }

        if in_dep_section {
            // Stop when we hit member specs
            if line.starts_with("## Member ") {
                break;
            }

            // Capture the dependency graph
            if line.contains("## Dependency Graph") {
                in_graph = true;
                continue;
            }

            if in_graph {
                if line.starts_with("```") {
                    // Toggle code block
                    if !graph_text.is_empty() {
                        // End of graph
                        break;
                    }
                    continue;
                }
                if line.starts_with("##") && !line.contains("Dependency Graph") {
                    // New section, end of graph
                    in_graph = false;
                }
                if in_graph && !line.trim().is_empty() && !line.starts_with("**") {
                    graph_text.push_str(line);
                    graph_text.push('\n');
                }
            }

            dep_text.push_str(line);
            dep_text.push('\n');
        }
    }

    if graph_text.is_empty() {
        return None;
    }

    // Parse edges from dependency text (simple heuristic)
    let edges = extract_dependency_edges(&dep_text);

    Some(DependencyAnalysis {
        graph_text: graph_text.trim().to_string(),
        edges,
    })
}

/// Extract dependency edges from analysis text
/// Looks for patterns like "Task N depends on Task M" or similar
fn extract_dependency_edges(text: &str) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();

    // Simple pattern matching for "Member N depends on Member M"
    // or "Task N depends on Task M"
    for line in text.lines() {
        if line.contains("depends on") {
            // Try to extract numbers
            let words: Vec<&str> = line.split_whitespace().collect();
            let mut from_num = None;
            let mut to_nums = Vec::new();

            for (i, word) in words.iter().enumerate() {
                if (word.starts_with("Member") || word.starts_with("Task")) && i + 1 < words.len() {
                    if let Ok(num) = words[i + 1].trim_end_matches([',', ':']).parse::<usize>() {
                        if from_num.is_none() {
                            from_num = Some(num);
                        }
                    }
                }

                if *word == "on" && i + 1 < words.len() {
                    // Look ahead for numbers
                    for next_word in words.iter().skip(i + 1) {
                        let next = next_word.trim_end_matches([',', '.', ':', ';', ')']);
                        if let Ok(num) = next.parse::<usize>() {
                            to_nums.push(num);
                        }
                        if next_word.contains("because") || next_word.contains("and") {
                            break;
                        }
                    }
                }
            }

            if let Some(from) = from_num {
                for to in to_nums {
                    edges.push((to, from)); // (dependency, dependent)
                }
            }
        }
    }

    edges
}

/// Parse member specs from agent output (split analysis)
/// Tuple representing a member being parsed: (title, description, files, dependencies, has_requires)
type MemberInProgress = (String, String, Vec<String>, Vec<usize>, bool);

fn parse_member_specs_from_output(output: &str) -> Result<Vec<MemberSpec>> {
    let mut members = Vec::new();
    let mut current_member: Option<MemberInProgress> = None;
    let mut collecting_files = false;
    let mut collecting_dependencies = false;
    let mut collecting_requires = false;
    let mut in_code_block = false;

    for line in output.lines() {
        // Check for member headers (## Member N: ...)
        if line.starts_with("## Member ") && line.contains(':') {
            // Save previous member if any
            if let Some((title, desc, files, deps, _has_requires)) = current_member.take() {
                members.push(MemberSpec {
                    title,
                    description: desc.trim().to_string(),
                    target_files: if files.is_empty() { None } else { Some(files) },
                    dependencies: deps,
                });
            }

            // Extract title from "## Member N: Title Here"
            if let Some(title_part) = line.split(':').nth(1) {
                let title = title_part.trim().to_string();
                current_member = Some((title, String::new(), Vec::new(), Vec::new(), false));
                collecting_files = false;
                collecting_dependencies = false;
                collecting_requires = false;
            }
        } else if current_member.is_some() {
            // Check for code block markers
            if line.trim() == "```" {
                in_code_block = !in_code_block;
                if let Some((_, ref mut desc, _, _, _)) = &mut current_member {
                    desc.push_str(line);
                    desc.push('\n');
                }
                continue;
            }

            // Check for "Affected Files:" header
            if line.contains("**Affected Files:**") || line.contains("Affected Files:") {
                collecting_files = true;
                collecting_dependencies = false;
                collecting_requires = false;
                continue;
            }

            // Check for "Dependencies:" header
            if line.contains("**Dependencies:**")
                || (line.starts_with("Dependencies:") && !line.contains("##"))
            {
                collecting_dependencies = true;
                collecting_files = false;
                collecting_requires = false;
                // Parse dependencies from same line if present, but ONLY if we haven't
                // already collected dependencies from a Requires section
                if let Some(deps_part) = line.split(':').nth(1) {
                    if let Some((_, _, _, ref mut deps, has_requires)) = &mut current_member {
                        if !*has_requires {
                            parse_dependencies_from_text(deps_part, deps);
                        } else {
                            eprintln!("Warning: Ignoring Dependencies line because Requires section was already parsed");
                        }
                    }
                }
                continue;
            }

            // Check for "### Requires" header
            if line.contains("### Requires") {
                collecting_requires = true;
                collecting_files = false;
                collecting_dependencies = false;
                // Mark that we're using Requires section for dependencies
                if let Some((_, _, _, _, ref mut has_requires)) = &mut current_member {
                    *has_requires = true;
                }
                continue;
            }

            // If collecting files, parse them (format: "- filename")
            if collecting_files {
                if let Some(stripped) = line.strip_prefix("- ") {
                    let file = stripped.trim().to_string();
                    if !file.is_empty() {
                        // Strip annotations like "(test module)" from filename
                        let cleaned_file = if let Some(paren_pos) = file.find('(') {
                            file[..paren_pos].trim().to_string()
                        } else {
                            file
                        };
                        if let Some((_, _, ref mut files, _, _)) = current_member {
                            files.push(cleaned_file);
                        }
                    }
                } else if line.starts_with('-') && !line.starts_with("- ") {
                    // Not a bullet list, stop collecting
                    collecting_files = false;
                } else if line.trim().is_empty() {
                    // Empty line might end the files section, depending on context
                } else if line.starts_with("##") {
                    // New section
                    collecting_files = false;
                }
            } else if collecting_dependencies {
                // Parse any additional dependency info
                if line.starts_with("##") || line.trim().is_empty() {
                    collecting_dependencies = false;
                }
            } else if collecting_requires {
                // Parse member references from Requires section
                // Stop if we hit another section
                if line.starts_with("###") || line.starts_with("##") {
                    collecting_requires = false;
                    // Don't continue - let the line be processed normally
                } else if !line.trim().is_empty() {
                    // Parse lines like "- Uses X from Member N" or "- Requires Member N"
                    if let Some((_, _, _, ref mut deps, _)) = &mut current_member {
                        parse_dependencies_from_text(line, deps);
                    }
                    continue;
                }
            } else if !in_code_block {
                // Skip "Provides:" section - don't include in description
                if line.contains("### Provides") {
                    // Skip this section header
                    continue;
                }
                // Preserve ### headers and all content except special sections
                if let Some((_, ref mut desc, _, _, _)) = &mut current_member {
                    desc.push_str(line);
                    desc.push('\n');
                }
            }
        }
    }

    // Save last member
    if let Some((title, desc, files, deps, _has_requires)) = current_member {
        members.push(MemberSpec {
            title,
            description: desc.trim().to_string(),
            target_files: if files.is_empty() { None } else { Some(files) },
            dependencies: deps,
        });
    }

    if members.is_empty() {
        anyhow::bail!("No member specs found in agent output");
    }

    // Remove self-references and validate no cycles
    validate_and_clean_dependencies(&mut members)?;

    Ok(members)
}

/// Validate dependencies and remove self-references and cycles
fn validate_and_clean_dependencies(members: &mut [MemberSpec]) -> Result<()> {
    // Remove self-references
    for (index, member) in members.iter_mut().enumerate() {
        let member_number = index + 1;
        let original_len = member.dependencies.len();
        member.dependencies.retain(|&dep| dep != member_number);
        if member.dependencies.len() < original_len {
            eprintln!(
                "Warning: Removed self-reference in Member {}: {}",
                member_number, member.title
            );
        }
    }

    // Detect cycles using depth-first search
    fn has_cycle_from(
        node: usize,
        members: &[MemberSpec],
        visited: &mut Vec<bool>,
        rec_stack: &mut Vec<bool>,
    ) -> Option<Vec<usize>> {
        if rec_stack[node] {
            return Some(vec![node]);
        }
        if visited[node] {
            return None;
        }

        visited[node] = true;
        rec_stack[node] = true;

        if let Some(member) = members.get(node) {
            for &dep in &member.dependencies {
                if dep > 0 && dep <= members.len() {
                    let dep_idx = dep - 1;
                    if let Some(mut cycle) = has_cycle_from(dep_idx, members, visited, rec_stack) {
                        cycle.push(node);
                        return Some(cycle);
                    }
                }
            }
        }

        rec_stack[node] = false;
        None
    }

    let n = members.len();
    let mut visited = vec![false; n];
    let mut rec_stack = vec![false; n];

    for i in 0..n {
        if !visited[i] {
            if let Some(cycle) = has_cycle_from(i, members, &mut visited, &mut rec_stack) {
                let cycle_members: Vec<usize> = cycle.iter().rev().map(|&i| i + 1).collect();
                eprintln!(
                    "Warning: Detected dependency cycle: {} -> {}",
                    cycle_members
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> "),
                    cycle_members[0]
                );

                // Remove the edge that creates the cycle (last dependency in cycle)
                if cycle.len() >= 2 {
                    let from_idx = cycle[1];
                    let to_member = cycle[0] + 1;
                    if let Some(member) = members.get_mut(from_idx) {
                        member.dependencies.retain(|&dep| dep != to_member);
                        eprintln!(
                            "  Removed dependency: Member {} -> Member {}",
                            from_idx + 1,
                            to_member
                        );
                    }
                }

                // Reset for next iteration
                visited = vec![false; n];
                rec_stack = vec![false; n];
            }
        }
    }

    Ok(())
}

/// Parse dependencies from text like "Member 2, Member 3" or "None"
fn parse_dependencies_from_text(text: &str, deps: &mut Vec<usize>) {
    if text.trim().to_lowercase() == "none" {
        return;
    }

    // Extract numbers from text
    for word in text.split(&[',', ' ', ';'][..]) {
        let trimmed = word.trim();
        if let Ok(num) = trimmed.parse::<usize>() {
            if !deps.contains(&num) {
                deps.push(num);
            }
        } else if trimmed.starts_with("Member ") || trimmed.starts_with("Task ") {
            // Try to extract number after "Member " or "Task "
            let num_part = trimmed.split_whitespace().nth(1).unwrap_or("");
            if let Ok(num) = num_part.trim_end_matches([',', '.']).parse::<usize>() {
                if !deps.contains(&num) {
                    deps.push(num);
                }
            }
        }
    }
}

/// Detect if a member is infrastructure based on title/description keywords
fn is_infrastructure_member(member: &MemberSpec) -> bool {
    let text = format!("{} {}", member.title, member.description).to_lowercase();

    // Infrastructure keywords
    let infra_keywords = [
        "logging",
        "logger",
        "config",
        "configuration",
        "error handling",
        "error type",
        "utility",
        "helper",
        "common type",
        "shared type",
        "base type",
        "interface",
        "trait",
        "constant",
    ];

    infra_keywords.iter().any(|keyword| text.contains(keyword))
}

/// Calculate isolation percentage for a group of member specs.
///
/// Returns the percentage of members that are isolated (no cross-references to other members).
fn calculate_isolation_percentage(members: &[&Spec]) -> f64 {
    if members.is_empty() {
        return 100.0;
    }

    // Use regex to detect "Member N" patterns
    let member_pattern = regex::Regex::new(r"(?i)\bmember\s+\d+\b").unwrap();

    let isolated_count = members
        .iter()
        .filter(|member| !member_pattern.is_match(&member.body))
        .count();

    (isolated_count as f64 / members.len() as f64) * 100.0
}

/// Count the number of files that appear in multiple members' target_files.
fn count_shared_files(members: &[&Spec]) -> usize {
    use std::collections::HashMap;

    let mut file_counts: HashMap<String, usize> = HashMap::new();

    for member in members {
        if let Some(target_files) = &member.frontmatter.target_files {
            let unique_files: std::collections::HashSet<_> = target_files.iter().collect();
            for file in unique_files {
                *file_counts.entry(file.clone()).or_insert(0) += 1;
            }
        }
    }

    file_counts.values().filter(|&&count| count > 1).count()
}

/// Count total unique files across all members.
fn count_total_unique_files(members: &[&Spec]) -> usize {
    use std::collections::HashSet;

    let mut all_files = HashSet::new();

    for member in members {
        if let Some(target_files) = &member.frontmatter.target_files {
            for file in target_files {
                all_files.insert(file.clone());
            }
        }
    }

    all_files.len()
}

/// Detect infrastructure ordering issues and add to quality issues
fn detect_infrastructure_issues(members: &[MemberSpec], quality_issues: &mut Vec<String>) {
    for (index, member) in members.iter().enumerate() {
        let member_number = index + 1;

        if is_infrastructure_member(member) {
            // Check if infrastructure depends on non-infrastructure
            for dep in &member.dependencies {
                let dep_index = dep - 1;
                if dep_index < members.len() {
                    let dep_member = &members[dep_index];
                    if !is_infrastructure_member(dep_member) {
                        eprintln!(
                            "  {} Member {} (infrastructure) depends on Member {} (feature) - may be incorrect",
                            "⚠".yellow(),
                            member_number,
                            dep
                        );
                        quality_issues.push(format!(
                            "Infrastructure Member {} depends on feature Member {}",
                            member_number, dep
                        ));
                    }
                }
            }

            // Warn if infrastructure appears late in sequence
            if member_number > members.len() / 2 {
                eprintln!(
                    "  {} Member {} (infrastructure) appears late in sequence - consider reordering",
                    "⚠".yellow(),
                    member_number
                );
            }
        }
    }
}

/// Display split quality report
fn display_split_quality_report(
    members: &[MemberSpec],
    dep_analysis: &Option<DependencyAnalysis>,
    quality_issues: &[String],
) {
    println!("\n{} Split Quality Report", "→".cyan());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Show dependency structure
    if let Some(analysis) = dep_analysis {
        println!("\n{} Dependency Graph:", "📊".cyan());
        println!("{}", analysis.graph_text);

        // Analyze parallelism
        let mut parallel_groups: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, member) in members.iter().enumerate() {
            let deps_key = if member.dependencies.is_empty() {
                "none".to_string()
            } else {
                let mut deps = member.dependencies.clone();
                deps.sort();
                format!("{:?}", deps)
            };
            parallel_groups.entry(deps_key).or_default().push(i + 1);
        }

        let parallel_count = parallel_groups.values().filter(|v| v.len() > 1).count();
        if parallel_count > 0 {
            println!("\n{} Parallelism Detected:", "✓".green());
            for (deps, group) in parallel_groups.iter() {
                if group.len() > 1 {
                    println!(
                        "  Members {:?} can run in parallel (depend on: {})",
                        group,
                        if deps == "none" {
                            "nothing".to_string()
                        } else {
                            deps.clone()
                        }
                    );
                }
            }
        }
    }

    // Show complexity metrics
    println!("\n{} Complexity Metrics:", "📏".cyan());
    for (index, member) in members.iter().enumerate() {
        let member_number = index + 1;
        let criteria_count = member.description.matches("- [ ]").count()
            + member.description.matches("- [x]").count()
            + member.description.matches("- [X]").count();
        let files_count = member.target_files.as_ref().map(|f| f.len()).unwrap_or(0);
        let word_count = member.description.split_whitespace().count();

        println!(
            "  Member {}: {} criteria, {} files, {} words",
            member_number, criteria_count, files_count, word_count
        );
    }

    // Show quality issues
    if !quality_issues.is_empty() {
        println!("\n{} Quality Issues:", "⚠".yellow());
        for issue in quality_issues {
            println!("  • {}", issue);
        }
    } else {
        println!("\n{} No quality issues detected", "✓".green());
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

// ============================================================================
// ARCHIVING
// ============================================================================

/// Check if we're in a git repository
fn is_git_repo() -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Result of verifying target files have changes
#[derive(Debug)]
pub struct TargetFilesVerification {
    /// Files that have changes in spec commits
    pub files_with_changes: Vec<String>,
    /// Files listed in target_files but without changes
    pub files_without_changes: Vec<String>,
    /// Commits found for the spec
    pub commits: Vec<String>,
    /// All files that were actually changed (file path, net additions)
    pub actual_files_changed: Vec<(String, i64)>,
}

/// Get commits associated with a spec by searching git log
fn get_spec_commits(spec_id: &str) -> Result<Vec<String>> {
    // Look for commits with the chant(spec_id): pattern
    let pattern = format!("chant({}):", spec_id);

    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "--grep", &pattern, "--reverse"])
        .output()
        .context("Failed to execute git log command")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let mut commits = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(hash) = line.split_whitespace().next() {
            if !hash.is_empty() {
                commits.push(hash.to_string());
            }
        }
    }

    Ok(commits)
}

/// Get file stats (insertions, deletions) for a commit
/// Returns a map of file path -> (insertions, deletions)
fn get_commit_file_stats(commit: &str) -> Result<std::collections::HashMap<String, (i64, i64)>> {
    use std::collections::HashMap;

    let output = std::process::Command::new("git")
        .args(["show", "--numstat", "--format=", commit])
        .output()
        .context("Failed to execute git show command")?;

    if !output.status.success() {
        return Ok(HashMap::new());
    }

    let mut stats = HashMap::new();
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            // Format: insertions\tdeletions\tfile_path
            // Binary files show "-" for insertions/deletions
            let insertions: i64 = parts[0].parse().unwrap_or(0);
            let deletions: i64 = parts[1].parse().unwrap_or(0);
            let file_path = parts[2].to_string();

            // Accumulate stats for files that appear in multiple hunks
            let entry = stats.entry(file_path).or_insert((0i64, 0i64));
            entry.0 += insertions;
            entry.1 += deletions;
        }
    }

    Ok(stats)
}

/// Verify that target files listed in a spec have actual changes from spec commits
fn verify_target_files(spec: &Spec) -> Result<TargetFilesVerification> {
    use std::collections::HashSet;

    // Get target files from frontmatter
    let target_files = match &spec.frontmatter.target_files {
        Some(files) if !files.is_empty() => files.clone(),
        _ => {
            // No target_files specified - nothing to verify
            return Ok(TargetFilesVerification {
                files_with_changes: Vec::new(),
                files_without_changes: Vec::new(),
                commits: Vec::new(),
                actual_files_changed: Vec::new(),
            });
        }
    };

    // Get commits for this spec
    let commits = get_spec_commits(&spec.id)?;

    if commits.is_empty() {
        // No commits found - all target files are without changes
        return Ok(TargetFilesVerification {
            files_with_changes: Vec::new(),
            files_without_changes: target_files,
            commits: Vec::new(),
            actual_files_changed: Vec::new(),
        });
    }

    // Collect all file changes across all commits
    let mut all_file_stats: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();

    for commit in &commits {
        let commit_stats = get_commit_file_stats(commit)?;
        for (file, (ins, del)) in commit_stats {
            let entry = all_file_stats.entry(file).or_insert((0, 0));
            entry.0 += ins;
            entry.1 += del;
        }
    }

    // Build set of files that were modified
    let modified_files: HashSet<String> = all_file_stats.keys().cloned().collect();

    // Check each target file
    let mut files_with_changes = Vec::new();
    let mut files_without_changes = Vec::new();

    for target_file in &target_files {
        if modified_files.contains(target_file) {
            files_with_changes.push(target_file.clone());
        } else {
            files_without_changes.push(target_file.clone());
        }
    }

    // Collect all actual files changed with their net additions
    let mut actual_files_changed: Vec<(String, i64)> = all_file_stats
        .iter()
        .map(|(file, (ins, del))| (file.clone(), ins - del))
        .collect();
    // Sort by file path for consistent output
    actual_files_changed.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(TargetFilesVerification {
        files_with_changes,
        files_without_changes,
        commits,
        actual_files_changed,
    })
}

/// Format a warning message when target files don't match actual changes
fn format_target_files_warning(spec_id: &str, verification: &TargetFilesVerification) -> String {
    // Combine all predicted files (both with and without changes)
    let mut all_predicted = verification.files_without_changes.clone();
    all_predicted.extend(verification.files_with_changes.clone());
    let predicted = all_predicted.join(", ");

    // Format actual files list
    let actual = if verification.actual_files_changed.is_empty() {
        "(none)".to_string()
    } else {
        verification
            .actual_files_changed
            .iter()
            .map(|(f, _)| f.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "Note: Spec {} predicted [{}] but changed [{}]\n      (Prediction mismatch - implementation is fine)\n",
        spec_id, predicted, actual
    )
}

/// Move a file using git mv, falling back to fs::rename if not in a git repo or if no_stage is true
fn move_spec_file(src: &PathBuf, dst: &PathBuf, no_stage: bool) -> Result<()> {
    let use_git = !no_stage && is_git_repo();

    if use_git {
        // Use git mv to stage the move
        let status = std::process::Command::new("git")
            .args(["mv", &src.to_string_lossy(), &dst.to_string_lossy()])
            .status()
            .context("Failed to run git mv")?;

        if !status.success() {
            anyhow::bail!("git mv failed for {}", src.display());
        }
    } else {
        // Fall back to filesystem rename
        std::fs::rename(src, dst).context(format!(
            "Failed to move file from {} to {}",
            src.display(),
            dst.display()
        ))?;
    }

    Ok(())
}

/// Archive completed specs (move from specs to archive directory)
pub fn cmd_archive(
    spec_id: Option<&str>,
    dry_run: bool,
    older_than: Option<u64>,
    force: bool,
    commit: bool,
    no_stage: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let archive_dir = PathBuf::from(ARCHIVE_DIR);

    // Load all specs
    let specs = spec::load_all_specs(&specs_dir)?;

    // Filter specs to archive
    let mut to_archive = Vec::new();

    if let Some(id) = spec_id {
        // Archive specific spec
        if let Some(spec) = specs.iter().find(|s| s.id.starts_with(id)) {
            // Check if this is a member spec
            if spec::extract_driver_id(&spec.id).is_some() {
                // This is a member spec - always allow archiving members directly
                to_archive.push(spec.clone());
            } else {
                // This is a driver spec or standalone spec
                let members = spec::get_members(&spec.id, &specs);
                if !members.is_empty() {
                    // This is a driver spec with members
                    if !spec::all_members_completed(&spec.id, &specs) {
                        eprintln!(
                            "{} Skipping driver spec {} - not all members are completed",
                            "⚠ ".yellow(),
                            spec.id
                        );
                        return Ok(());
                    }

                    // All members are completed, automatically add them first (sorted by member number)
                    let mut sorted_members = members.clone();
                    sorted_members
                        .sort_by_key(|m| spec::extract_member_number(&m.id).unwrap_or(u32::MAX));
                    for member in sorted_members {
                        to_archive.push(member.clone());
                    }
                    // Then add the driver
                    to_archive.push(spec.clone());
                } else {
                    // Standalone spec or driver with no members
                    to_archive.push(spec.clone());
                }
            }
        } else {
            anyhow::bail!("Spec {} not found", id);
        }
    } else {
        // Archive by criteria
        let now = chrono::Local::now();

        for spec in &specs {
            // Skip if not completed (unless force)
            if spec.frontmatter.status != SpecStatus::Completed && !force {
                continue;
            }

            // Check older_than filter
            if let Some(days) = older_than {
                if let Some(completed_at_str) = &spec.frontmatter.completed_at {
                    if let Ok(completed_at) = chrono::DateTime::parse_from_rfc3339(completed_at_str)
                    {
                        let completed_at_local =
                            chrono::DateTime::<chrono::Local>::from(completed_at);
                        let age = now.signed_duration_since(completed_at_local);
                        if age.num_days() < days as i64 {
                            continue;
                        }
                    }
                } else {
                    // No completion date, skip
                    continue;
                }
            }

            // Check group constraints
            if let Some(driver_id) = spec::extract_driver_id(&spec.id) {
                // This is a member spec - skip unless driver is already archived
                let driver_exists = specs.iter().any(|s| s.id == driver_id);
                if driver_exists {
                    continue; // Driver still exists, skip this member
                }
            } else {
                // This is a driver spec or standalone spec
                let members = spec::get_members(&spec.id, &specs);
                if !members.is_empty() {
                    // This is a driver spec with members - check if all are completed
                    if !spec::all_members_completed(&spec.id, &specs) {
                        continue; // Not all members completed, skip this driver
                    }
                    // Add members first (sorted by member number)
                    let mut sorted_members = members.clone();
                    sorted_members
                        .sort_by_key(|m| spec::extract_member_number(&m.id).unwrap_or(u32::MAX));
                    for member in sorted_members {
                        to_archive.push(member.clone());
                    }
                }
            }

            to_archive.push(spec.clone());
        }
    }

    if to_archive.is_empty() {
        println!("No specs to archive.");
        return Ok(());
    }

    // Verify target files have changes (unless --force is set)
    if !force && is_git_repo() {
        let mut specs_with_missing_changes = Vec::new();

        for spec in &to_archive {
            // Only verify specs with target_files
            if spec.frontmatter.target_files.is_some() {
                let verification = verify_target_files(spec)?;

                // Check if there are target files without changes
                if !verification.files_without_changes.is_empty() {
                    specs_with_missing_changes.push((spec.clone(), verification));
                }
            }
        }

        // If any specs have missing changes, warn the user
        if !specs_with_missing_changes.is_empty() {
            println!(
                "\n{} {} spec(s) have target_files without changes:\n",
                "⚠".yellow(),
                specs_with_missing_changes.len()
            );

            for (spec, verification) in &specs_with_missing_changes {
                println!("{}", format_target_files_warning(&spec.id, verification));
                if !verification.commits.is_empty() {
                    println!("Commits found: {}\n", verification.commits.join(", "));
                } else {
                    println!("No commits found with pattern 'chant({}):'.\n", spec.id);
                }
            }

            // Prompt for confirmation
            let confirmed = prompt::confirm("Archive anyway?")?;
            if !confirmed {
                println!("{} Archive cancelled.", "✗".yellow());
                return Ok(());
            }
        }
    }

    // Count drivers and members for summary
    let mut driver_count = 0;
    let mut member_count = 0;
    for spec in &to_archive {
        if spec::extract_driver_id(&spec.id).is_some() {
            member_count += 1;
        } else {
            driver_count += 1;
        }
    }

    if dry_run {
        println!("{} Would archive {} spec(s):", "→".cyan(), to_archive.len());
        for spec in &to_archive {
            if spec::extract_driver_id(&spec.id).is_some() {
                println!("  {} {} (member)", "→".cyan(), spec.id);
            } else {
                println!("  {} {} (driver)", "→".cyan(), spec.id);
            }
        }
        let summary = if driver_count > 0 && member_count > 0 {
            format!(
                "Archived {} spec(s) ({} driver + {} member{})",
                to_archive.len(),
                driver_count,
                member_count,
                if member_count == 1 { "" } else { "s" }
            )
        } else {
            format!("Archived {} spec(s)", to_archive.len())
        };
        println!("{} {}", "→".cyan(), summary);
        return Ok(());
    }

    // Create archive directory if it doesn't exist
    if !archive_dir.exists() {
        std::fs::create_dir_all(&archive_dir)?;
        println!("{} Created archive directory", "✓".green());
    }

    // Migrate existing flat archive files to date subfolders (if any)
    migrate_flat_archive(&archive_dir)?;

    // Move specs to archive
    let count = to_archive.len();
    for spec in to_archive {
        let src = specs_dir.join(format!("{}.md", spec.id));

        // Extract date from spec ID (format: YYYY-MM-DD-XXX-abc)
        let date_part = &spec.id[..10]; // First 10 chars: YYYY-MM-DD
        let date_dir = archive_dir.join(date_part);

        // Create date-based subdirectory if it doesn't exist
        if !date_dir.exists() {
            std::fs::create_dir_all(&date_dir)?;
        }

        let dst = date_dir.join(format!("{}.md", spec.id));

        move_spec_file(&src, &dst, no_stage)?;
        if spec::extract_driver_id(&spec.id).is_some() {
            println!("  {} {} (archived)", "→".cyan(), spec.id);
        } else {
            println!("  {} {} (driver, archived)", "→".cyan(), spec.id);
        }
    }

    // Print summary
    let summary = if driver_count > 0 && member_count > 0 {
        format!(
            "Archived {} spec(s) ({} driver + {} member{})",
            count,
            driver_count,
            member_count,
            if member_count == 1 { "" } else { "s" }
        )
    } else {
        format!("Archived {} spec(s)", count)
    };
    println!("{} {}", "✓".green(), summary);

    // Create commit if requested (and in a git repo)
    if commit && is_git_repo() {
        let status = std::process::Command::new("git")
            .args(["commit", "-m", "Archive completed specs"])
            .status()
            .context("Failed to create commit")?;

        if !status.success() {
            anyhow::bail!("git commit failed");
        }
        println!("{} Created commit: Archive completed specs", "✓".green());
    }

    Ok(())
}

/// Migrate existing flat archive files to date-based subfolders.
/// This handles the transition from `.chant/archive/*.md` to `.chant/archive/YYYY-MM-DD/*.md`
fn migrate_flat_archive(archive_dir: &std::path::PathBuf) -> anyhow::Result<()> {
    use std::fs;

    if !archive_dir.exists() {
        return Ok(());
    }

    let mut flat_files = Vec::new();

    // Find all flat .md files in the archive directory (not in subdirectories)
    for entry in fs::read_dir(archive_dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        // Only process .md files directly in archive_dir, not subdirectories
        if !metadata.is_dir() && path.extension().map(|e| e == "md").unwrap_or(false) {
            flat_files.push(path);
        }
    }

    // Migrate each flat file to its date subfolder
    for file_path in flat_files {
        if let Some(file_name) = file_path.file_name() {
            if let Some(file_name_str) = file_name.to_str() {
                // Extract spec ID from filename (e.g., "2026-01-24-001-abc.md" -> "2026-01-24-001-abc")
                if let Some(spec_id) = file_name_str.strip_suffix(".md") {
                    // Extract date from spec ID (format: YYYY-MM-DD-XXX-abc)
                    if spec_id.len() >= 10 {
                        let date_part = &spec_id[..10]; // First 10 chars: YYYY-MM-DD
                        let date_dir = archive_dir.join(date_part);

                        // Create date-based subdirectory if it doesn't exist
                        if !date_dir.exists() {
                            fs::create_dir_all(&date_dir)?;
                        }

                        let dst = date_dir.join(file_name);

                        // Move the file to the date subdirectory using git mv when possible
                        if let Err(e) = move_spec_file(&file_path, &dst, false) {
                            eprintln!(
                                "Warning: Failed to migrate archive file {:?}: {}",
                                file_path, e
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// MERGING
// ============================================================================

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
                "chant/".dimmed(),
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
                "chant/".dimmed(),
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
        let confirmed = prompt::confirm(&format!(
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
                        spec.frontmatter.status = SpecStatus::Completed;

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

// ============================================================================
// DRIFT DETECTION
// ============================================================================

/// Check if documentation and research specs have stale inputs
pub fn cmd_drift(id: Option<&str>) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    let specs = spec::load_all_specs(&specs_dir)?;

    // If a specific ID is provided, filter to that spec
    let specs_to_check: Vec<&Spec> = if let Some(filter_id) = id {
        specs.iter().filter(|s| s.id.contains(filter_id)).collect()
    } else {
        specs.iter().collect()
    };

    if specs_to_check.is_empty() {
        if let Some(filter_id) = id {
            anyhow::bail!("No specs found matching: {}", filter_id);
        } else {
            println!("No specs to check for drift.");
            return Ok(());
        }
    }

    let mut drifted_specs = Vec::new();
    let mut up_to_date_specs = Vec::new();

    for spec in specs_to_check {
        // Only check completed specs
        if spec.frontmatter.status != SpecStatus::Completed {
            continue;
        }

        // Get completion time
        let completed_at = match &spec.frontmatter.completed_at {
            Some(timestamp) => timestamp.clone(),
            None => {
                // If completed but no timestamp, skip
                continue;
            }
        };

        // Parse timestamp - format is ISO 8601 UTC (e.g., "2026-01-24T15:30:00Z")
        let completed_time = match chrono::DateTime::parse_from_rfc3339(&completed_at) {
            Ok(dt) => dt,
            Err(_) => {
                // If timestamp format is invalid, skip
                continue;
            }
        };

        // Check for drifts
        let mut drift_report = DriftReport {
            spec_id: spec.id.clone(),
            spec_type: spec.frontmatter.r#type.clone(),
            completed_at: completed_at.clone(),
            drifted_files: Vec::new(),
        };

        // Check tracked files (documentation specs)
        if let Some(tracked) = &spec.frontmatter.tracks {
            for file_pattern in tracked {
                check_files_for_changes(file_pattern, &completed_time, &mut drift_report)?;
            }
        }

        // Check origin files (research specs)
        if let Some(origin) = &spec.frontmatter.origin {
            for file_pattern in origin {
                check_files_for_changes(file_pattern, &completed_time, &mut drift_report)?;
            }
        }

        // Check informed_by files (research specs)
        if let Some(informed_by) = &spec.frontmatter.informed_by {
            for file_pattern in informed_by {
                check_files_for_changes(file_pattern, &completed_time, &mut drift_report)?;
            }
        }

        if drift_report.drifted_files.is_empty() {
            up_to_date_specs.push(drift_report);
        } else {
            drifted_specs.push(drift_report);
        }
    }

    // Display results
    if drifted_specs.is_empty() && up_to_date_specs.is_empty() {
        println!("No completed specs with tracked/origin/informed_by fields to check.");
        return Ok(());
    }

    if !drifted_specs.is_empty() {
        println!(
            "\n{} Drifted Specs (inputs changed after completion)",
            "⚠".yellow()
        );
        println!("{}", "─".repeat(70));

        for report in &drifted_specs {
            println!(
                "\n{} Spec: {} ({})",
                "●".red(),
                report.spec_id,
                report.spec_type
            );
            println!("  Completed: {}", report.completed_at.bright_black());
            for drifted_file in &report.drifted_files {
                println!(
                    "    {} {} (modified {})",
                    "→".bright_black(),
                    drifted_file.path,
                    drifted_file.modified_at.bright_black()
                );
            }
            println!(
                "  {}",
                "Recommendation: Re-run spec to update analysis/documentation".yellow()
            );
        }
    }

    if !up_to_date_specs.is_empty() && !drifted_specs.is_empty() {
        println!();
    }

    if !up_to_date_specs.is_empty() {
        println!("\n{} Up-to-date Specs (no input changes)", "✓".green());
        println!("{}", "─".repeat(70));

        for report in &up_to_date_specs {
            println!("{} {} ({})", "●".green(), report.spec_id, report.spec_type);
        }
    }

    // Return success if checking specific spec even if it drifted
    Ok(())
}

#[derive(Debug)]
struct DriftReport {
    spec_id: String,
    spec_type: String,
    completed_at: String,
    drifted_files: Vec<DriftedFile>,
}

#[derive(Debug)]
struct DriftedFile {
    path: String,
    modified_at: String,
}

/// Check if any files matching a pattern have been modified after a certain time
fn check_files_for_changes(
    pattern: &str,
    completed_time: &chrono::DateTime<chrono::FixedOffset>,
    drift_report: &mut DriftReport,
) -> Result<()> {
    // Expand glob pattern to actual files
    let mut expanded_files = Vec::new();

    // Check if pattern is a glob
    if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
        // Use glob to expand
        use glob::glob as glob_fn;
        for entry in glob_fn(pattern)
            .context(format!("Invalid glob pattern: {}", pattern))?
            .flatten()
        {
            if entry.is_file() {
                expanded_files.push(entry);
            }
        }
    } else {
        // Literal path
        let path = std::path::PathBuf::from(pattern);
        if path.exists() && path.is_file() {
            expanded_files.push(path);
        }
    }

    // For each file, check if it was modified after completed_at
    for file_path in expanded_files {
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                let file_modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
                let completed_utc = completed_time.with_timezone(&chrono::Utc);

                if file_modified_time > completed_utc {
                    let relative_path = file_path.to_string_lossy().to_string();
                    drift_report.drifted_files.push(DriftedFile {
                        path: relative_path,
                        modified_at: file_modified_time.format("%Y-%m-%d").to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// RESUME
// ============================================================================

/// Resume a failed spec by resetting it to pending status
pub fn cmd_resume(
    id: &str,
    work: bool,
    prompt: Option<&str>,
    branch: Option<String>,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    let spec_id = spec.id.clone();

    // Check if spec is in failed or in_progress state
    if spec.frontmatter.status != SpecStatus::Failed
        && spec.frontmatter.status != SpecStatus::InProgress
    {
        anyhow::bail!(
            "Spec {} is not in failed or in_progress state (current status: {:?}). \
             Only failed or in_progress specs can be resumed.",
            spec_id,
            spec.frontmatter.status
        );
    }

    println!("{} Resuming spec {}", "→".cyan(), spec_id.cyan());

    // Reset to pending
    spec.frontmatter.status = SpecStatus::Pending;
    spec.save(&spec_path)?;

    println!("{} Spec {} reset to pending", "✓".green(), spec_id);

    // If --work flag specified, execute the spec
    if work {
        println!("{} Re-executing spec...", "→".cyan());

        // Use cmd_work to execute the spec
        cmd::work::cmd_work(
            std::slice::from_ref(&spec_id),
            prompt,
            branch,
            false, // force
            false, // parallel
            &[],   // label
            false, // finalize
            false, // allow_no_commits
            None,  // max_parallel
            false, // no_cleanup
            false, // cleanup
            false, // skip_approval
            false, // chain
            0,     // chain_max
            false, // no_merge
            false, // no_rebase
        )?;
    }

    Ok(())
}

// ============================================================================
// LIFECYCLE OPERATIONS (for watch mode)
// ============================================================================

/// Handle a completed spec: finalize it, then merge if on a branch.
///
/// This orchestrates the completion workflow:
/// 1. Run `chant finalize <spec_id>` to validate and mark complete
/// 2. If spec is on a `chant/<spec-id>` branch, run `chant merge <spec_id>`
/// 3. If spec is on main branch, skip merge step
///
/// # Arguments
/// * `spec_id` - The spec ID to complete
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(_)` if finalize or merge subprocess fails
///
/// # Edge Cases
/// * Finalize fails: Return error, do not proceed to merge
/// * Merge fails: Return error with conflict details
/// * Spec on main branch: Skip merge step
pub fn handle_completed(spec_id: &str) -> Result<()> {
    use std::process::Command;

    let _specs_dir = crate::cmd::ensure_initialized()?;

    // Step 1: Finalize the spec
    println!("{} Finalizing spec {}", "→".cyan(), spec_id.cyan());

    let status = Command::new(std::env::current_exe()?)
        .args(["finalize", spec_id])
        .status()
        .context("Failed to run chant finalize")?;

    if !status.success() {
        anyhow::bail!("Finalize failed for spec {}", spec_id);
    }

    println!("{} Finalized spec {}", "✓".green(), spec_id);

    // Step 2: Check if spec is on a branch
    let branch_name = format!("chant/{}", spec_id);
    let on_branch = is_spec_on_branch(spec_id, &branch_name)?;

    if !on_branch {
        println!(
            "{} Spec {} is on main branch, skipping merge",
            "→".cyan(),
            spec_id
        );
        return Ok(());
    }

    // Step 3: Check if branch exists and hasn't been merged already
    let config = Config::load()?;
    let main_branch = merge::load_main_branch(&config);

    if git::branch_exists(&branch_name)? {
        // Check if the branch has already been merged
        if git::is_branch_merged(&branch_name, &main_branch)? {
            println!(
                "{} Branch {} already merged to {}, auto-deleting",
                "→".cyan(),
                branch_name.cyan(),
                main_branch
            );

            // Auto-delete the merged branch
            if let Err(e) = git::delete_branch(&branch_name, false) {
                println!(
                    "{} Warning: Could not delete branch {}: {}",
                    "⚠".yellow(),
                    branch_name,
                    e
                );
            } else {
                println!("{} Deleted merged branch {}", "✓".green(), branch_name);
            }

            return Ok(());
        }
    } else {
        println!(
            "{} Branch {} does not exist, skipping merge",
            "→".cyan(),
            branch_name
        );
        return Ok(());
    }

    // Step 4: Merge the branch
    println!("{} Merging branch {}", "→".cyan(), branch_name.cyan());

    let status = Command::new(std::env::current_exe()?)
        .args(["merge", spec_id])
        .status()
        .context("Failed to run chant merge")?;

    if !status.success() {
        anyhow::bail!(
            "Merge failed for spec {} (branch: {})",
            spec_id,
            branch_name
        );
    }

    println!("{} Merged spec {}", "✓".green(), spec_id);

    Ok(())
}

/// Handle a failed spec: decide whether to retry or mark permanent failure.
///
/// This orchestrates the failure handling workflow:
/// 1. Load spec and retry state
/// 2. Read error log
/// 3. Use retry logic to decide: retry or permanent failure
/// 4. If retry: schedule resume with exponential backoff
/// 5. If permanent: log and mark for manual intervention
///
/// # Arguments
/// * `spec_id` - The spec ID that failed
/// * `config` - Failure configuration with retry settings
///
/// # Returns
/// * `Ok(())` on success (retry scheduled or marked failed)
/// * `Err(_)` on subprocess failure or configuration error
///
/// # Edge Cases
/// * Resume fails: Treat as permanent failure
/// * Empty error log: Permanent failure
/// * Max retries exceeded: Permanent failure
pub fn handle_failed(spec_id: &str, config: &chant::config::FailureConfig) -> Result<()> {
    use chant::retry::{decide_retry, RetryDecision};

    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load the spec
    let mut spec = spec::resolve_spec(&specs_dir, spec_id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    // Get retry state (or create new one)
    let mut retry_state = spec.frontmatter.retry_state.clone().unwrap_or_default();

    // Read error log
    let log_path = specs_dir
        .parent()
        .unwrap_or(&specs_dir)
        .join("logs")
        .join(format!("{}.log", spec_id));

    let error_log = if log_path.exists() {
        std::fs::read_to_string(&log_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Decide whether to retry
    let decision = decide_retry(&retry_state, &error_log, config);

    match decision {
        RetryDecision::Retry(delay) => {
            // Schedule retry with exponential backoff
            let delay_ms = delay.as_millis() as u64;
            retry_state.record_attempt(delay_ms);

            let next_retry_time = retry_state.next_retry_time;

            println!(
                "{} Scheduling retry for spec {} (attempt {}/{}, delay: {}ms)",
                "→".cyan(),
                spec_id,
                retry_state.attempts,
                config.max_retries,
                delay_ms
            );

            // Update spec with new retry state
            spec.frontmatter.retry_state = Some(retry_state);
            spec.save(&spec_path)?;

            // Note: The watch loop will check next_retry_time and call resume when ready
            println!(
                "{} Retry will be attempted at timestamp {}",
                "→".cyan(),
                next_retry_time
            );

            Ok(())
        }
        RetryDecision::PermanentFailure(reason) => {
            // Mark as permanent failure
            println!(
                "{} Permanent failure for spec {}: {}",
                "✗".red(),
                spec_id,
                reason
            );

            // Update spec status to failed (it should already be failed, but ensure it)
            spec.frontmatter.status = chant::spec::SpecStatus::Failed;
            spec.save(&spec_path)?;

            println!(
                "{} Spec {} marked for manual intervention",
                "→".cyan(),
                spec_id
            );

            Ok(())
        }
    }
}

/// Check if a spec's worktree is on the specified branch.
///
/// # Arguments
/// * `spec_id` - The spec ID
/// * `branch_name` - The branch name to check (e.g., "chant/spec-id")
///
/// # Returns
/// * `Ok(true)` if worktree exists and is on the specified branch
/// * `Ok(false)` if worktree doesn't exist or is on a different branch
/// * `Err(_)` if git operations fail
fn is_spec_on_branch(spec_id: &str, branch_name: &str) -> Result<bool> {
    use std::process::Command;

    // Get worktree path
    let worktree_path = chant::worktree::worktree_path_for_spec(spec_id);

    // Check if worktree exists
    if !worktree_path.exists() {
        return Ok(false);
    }

    // Get current branch in worktree
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&worktree_path)
        .output()
        .context("Failed to get current branch in worktree")?;

    if !output.status.success() {
        return Ok(false);
    }

    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(current_branch == branch_name)
}

// ============================================================================
// REPLAY
// ============================================================================

/// Replay a completed spec by executing it again with the same or updated options
pub fn cmd_replay(
    id: &str,
    prompt: Option<&str>,
    branch: Option<String>,
    force: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_id = spec.id.clone();

    // Validate that spec exists and is completed
    if spec.frontmatter.status != SpecStatus::Completed {
        anyhow::bail!(
            "Only completed specs can be replayed. Spec {} has status: {:?}",
            spec_id,
            spec.frontmatter.status
        );
    }

    // Extract date from spec ID (format: YYYY-MM-DD-...)
    let completion_date = spec_id.split('-').take(3).collect::<Vec<_>>().join("-");
    let current_date = Local::now().format("%Y-%m-%d").to_string();

    // Display what will be replayed
    println!(
        "{} {} replay spec {}",
        "→".cyan(),
        if dry_run { "Would" } else { "Will" },
        spec_id.cyan()
    );
    if let Some(title) = &spec.title {
        println!("  {} {}", "•".cyan(), title.dimmed());
    }
    println!(
        "  {} Original completion: {}",
        "•".cyan(),
        completion_date.dimmed()
    );
    println!("  {} Current date: {}", "•".cyan(), current_date.dimmed());

    if let Some(completed_at) = &spec.frontmatter.completed_at {
        println!("  {} Completed at: {}", "•".cyan(), completed_at.dimmed());
    }
    if let Some(model) = &spec.frontmatter.model {
        println!("  {} Model: {}", "•".cyan(), model.dimmed());
    }

    // Show options that will be applied
    println!("  {} Options:", "•".cyan());
    if branch.is_some() {
        println!(
            "    {} Create feature branch{}",
            "∘".cyan(),
            branch
                .as_ref()
                .map(|b| format!(" with prefix: {}", b))
                .unwrap_or_default()
        );
    }
    if force {
        println!(
            "    {} Skip validation of unchecked acceptance criteria",
            "∘".cyan()
        );
    }
    if prompt.is_some() {
        println!(
            "    {} Use custom prompt: {}",
            "∘".cyan(),
            prompt.unwrap_or("standard").cyan()
        );
    }
    if branch.is_none() && !force && prompt.is_none() {
        println!("    {} {}", "∘".cyan(), "(no additional options)".dimmed());
    }

    // If dry-run, show what would happen and exit
    if dry_run {
        println!("{} Dry-run mode: no changes made.", "ℹ".blue());
        return Ok(());
    }

    // Ask for confirmation unless --yes
    if !yes {
        let confirmed = prompt::confirm(&format!("Proceed with replaying spec {}?", spec_id))?;
        if !confirmed {
            println!("{} Replay cancelled.", "✗".yellow());
            return Ok(());
        }
    }

    println!("{} Replaying spec {}", "→".cyan(), spec_id.cyan());

    // Reset spec status to in_progress before execution
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    let mut spec = spec::resolve_spec(&specs_dir, &spec_id)?;

    // Capture original completion info for the replay context
    let original_completion = spec.frontmatter.completed_at.clone();
    let spec_title = spec.title.clone();

    spec.frontmatter.status = SpecStatus::InProgress;
    spec.save(&spec_path)?;

    // Execute the spec using cmd_work
    // Pass force=true to ensure cmd_work proceeds (it will see the InProgress status
    // and still execute because force bypasses various guards)
    let work_result = cmd::work::cmd_work(
        std::slice::from_ref(&spec_id),
        prompt,
        branch,
        true,  // force=true to bypass guards in cmd_work for replay
        false, // parallel
        &[],   // label
        false, // finalize
        false, // allow_no_commits
        None,  // max_parallel
        false, // no_cleanup
        false, // cleanup
        true,  // skip_approval - replays should skip approval check
        false, // chain
        0,     // chain_max
        false, // no_merge
        false, // no_rebase
    );

    // Handle result: cmd_work will have set the status to completed or failed
    if work_result.is_ok() {
        // Replay completed successfully, create a replay transcript commit if we have the original completion date
        if let Some(original_completed_at) = original_completion {
            let replay_context = ReplayContext::new(
                spec_id.clone(),
                spec_title,
                original_completed_at,
                None, // Use default "manual" reason
            );

            // Create the replay transcript commit
            if let Err(e) = cmd::git_ops::commit_replay(&spec_path, &replay_context) {
                eprintln!(
                    "{} Warning: Failed to create replay transcript commit: {}",
                    "⚠".yellow(),
                    e
                );
                // Don't fail the entire replay if the transcript commit fails
                // The important thing is that the spec was replayed
            }
        }
    }

    work_result
}

/// Finalize a completed or in_progress spec
/// Validates all acceptance criteria are checked, updates status to completed,
/// and adds model information to frontmatter.
///
/// If the spec has an active worktree, finalization happens in the worktree
/// to prevent merge conflicts when the feature branch is later merged to main.
pub fn cmd_finalize(id: &str, specs_dir: &std::path::Path) -> Result<()> {
    use crate::cmd::finalize;
    use chant::spec;
    use chant::validation;
    use chant::worktree;

    // Resolve the spec
    let spec = spec::resolve_spec(specs_dir, id)?;
    let spec_id = spec.id.clone();

    // Check if spec is in a valid state for finalization
    // Allow failed too - agents often leave specs in failed state when they actually completed the work
    match spec.frontmatter.status {
        SpecStatus::Completed | SpecStatus::InProgress | SpecStatus::Failed => {
            // These are valid for finalization
        }
        _ => {
            anyhow::bail!(
                "Spec '{}' must be in_progress, completed, or failed to finalize. Current status: {:?}",
                spec_id,
                spec.frontmatter.status
            );
        }
    }

    // Check for unchecked acceptance criteria
    let unchecked = spec.count_unchecked_checkboxes();
    if unchecked > 0 {
        anyhow::bail!(
            "Spec '{}' has {} unchecked acceptance criteria. All criteria must be checked before finalization.",
            spec_id,
            unchecked
        );
    }

    // Load the config for model information and validation settings
    let config = Config::load()?;

    // Validate output against schema if output_schema is defined
    if let Some(ref schema_path_str) = spec.frontmatter.output_schema {
        let schema_path = std::path::Path::new(schema_path_str);
        if schema_path.exists() {
            // Read agent output from log file
            let log_path = specs_dir
                .parent()
                .unwrap_or(specs_dir)
                .join("logs")
                .join(format!("{}.log", spec_id));

            if log_path.exists() {
                let agent_output = std::fs::read_to_string(&log_path)
                    .with_context(|| format!("Failed to read agent log: {}", log_path.display()))?;

                match validation::validate_agent_output(&spec_id, schema_path, &agent_output) {
                    Ok(result) => {
                        if result.is_valid {
                            println!(
                                "{} Output validation passed (schema: {})",
                                "✓".green(),
                                schema_path_str
                            );
                        } else {
                            println!(
                                "{} Output validation failed (schema: {})",
                                "✗".red(),
                                schema_path_str
                            );
                            for error in &result.errors {
                                println!("  - {}", error);
                            }
                            println!("  → Review .chant/logs/{}.log for details", spec_id);

                            // Check if strict validation is enabled
                            if config.validation.strict_output_validation {
                                anyhow::bail!(
                                    "Cannot finalize: output validation failed ({} error(s), strict mode enabled)",
                                    result.errors.len()
                                );
                            } else {
                                println!(
                                    "  {} Proceeding with finalization (strict_output_validation=false)",
                                    "→".cyan()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} Failed to validate output: {}", "⚠".yellow(), e);
                        if config.validation.strict_output_validation {
                            anyhow::bail!(
                                "Cannot finalize: output validation error (strict mode enabled)"
                            );
                        } else {
                            println!(
                                "  {} Proceeding with finalization (strict_output_validation=false)",
                                "→".cyan()
                            );
                        }
                    }
                }
            } else {
                println!(
                    "{} No log file found at {}, skipping output validation",
                    "⚠".yellow(),
                    log_path.display()
                );
            }
        } else {
            println!(
                "{} Output schema file not found: {}, skipping validation",
                "⚠".yellow(),
                schema_path.display()
            );
        }
    }

    // Check if this spec has an active worktree - if so, finalize there
    if let Some(worktree_path) = worktree::get_active_worktree(&spec_id) {
        println!(
            "{} Finalizing spec {} in worktree",
            "→".cyan(),
            spec_id.cyan()
        );

        // Get the spec path in the worktree
        let worktree_spec_path = worktree_path
            .join(".chant/specs")
            .join(format!("{}.md", spec_id));

        // Load the spec from the worktree
        let mut worktree_spec =
            spec::Spec::load(&worktree_spec_path).context("Failed to load spec from worktree")?;

        // Get all specs from worktree for validation
        let worktree_specs_dir = worktree_path.join(".chant/specs");
        let all_specs = spec::load_all_specs(&worktree_specs_dir).unwrap_or_default();

        // Finalize in worktree
        finalize::finalize_spec(
            &mut worktree_spec,
            &worktree_spec_path,
            &config,
            &all_specs,
            false,
            None,
        )?;

        // Commit the finalization changes in the worktree
        let commit_message = format!("chant({}): finalize spec", spec_id);
        worktree::commit_in_worktree(&worktree_path, &commit_message)?;

        println!(
            "{} Spec {} finalized in worktree and committed",
            "✓".green(),
            spec_id.green()
        );
        if let Some(model) = &worktree_spec.frontmatter.model {
            println!("  {} Model: {}", "•".cyan(), model);
        }
        if let Some(completed_at) = &worktree_spec.frontmatter.completed_at {
            println!("  {} Completed at: {}", "•".cyan(), completed_at);
        }
        if let Some(commits) = &worktree_spec.frontmatter.commits {
            println!(
                "  {} {} commit{}",
                "•".cyan(),
                commits.len(),
                if commits.len() == 1 { "" } else { "s" }
            );
        }
        println!("  {} Worktree: {}", "•".cyan(), worktree_path.display());
    } else {
        // No active worktree - finalize on current branch (main)
        let spec_path = specs_dir.join(format!("{}.md", spec_id));

        // Perform finalization
        let mut mut_spec = spec.clone();
        finalize::re_finalize_spec(&mut mut_spec, &spec_path, &config, false)?;

        println!("{} Spec {} finalized", "✓".green(), spec_id.green());
        if let Some(model) = &mut_spec.frontmatter.model {
            println!("  {} Model: {}", "•".cyan(), model);
        }
        if let Some(completed_at) = &mut_spec.frontmatter.completed_at {
            println!("  {} Completed at: {}", "•".cyan(), completed_at);
        }
        if let Some(commits) = &mut_spec.frontmatter.commits {
            println!(
                "  {} {} commit{}",
                "•".cyan(),
                commits.len(),
                if commits.len() == 1 { "" } else { "s" }
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_logs_dir_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Logs dir shouldn't exist yet
        assert!(!base_path.join("logs").exists());

        // Call ensure_logs_dir_at
        cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

        // Logs dir should now exist
        assert!(base_path.join("logs").exists());
        assert!(base_path.join("logs").is_dir());
    }

    #[test]
    fn test_ensure_logs_dir_updates_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create base dir without .gitignore
        // (tempdir already exists, no need to create)

        // Call ensure_logs_dir_at
        cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should now exist and contain "logs/"
        let gitignore_path = base_path.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("logs/"));
    }

    #[test]
    fn test_verify_target_files_no_target_files() {
        // Spec without target_files should return empty verification
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                target_files: None,
                ..Default::default()
            },
            title: Some("Test spec".to_string()),
            body: "# Test\n\nBody".to_string(),
        };

        let verification = verify_target_files(&spec).unwrap();
        assert!(verification.files_with_changes.is_empty());
        assert!(verification.files_without_changes.is_empty());
        assert!(verification.commits.is_empty());
        assert!(verification.actual_files_changed.is_empty());
    }

    #[test]
    fn test_verify_target_files_empty_target_files() {
        // Spec with empty target_files should return empty verification
        let spec = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                target_files: Some(vec![]),
                ..Default::default()
            },
            title: Some("Test spec".to_string()),
            body: "# Test\n\nBody".to_string(),
        };

        let verification = verify_target_files(&spec).unwrap();
        assert!(verification.files_with_changes.is_empty());
        assert!(verification.files_without_changes.is_empty());
        assert!(verification.commits.is_empty());
        assert!(verification.actual_files_changed.is_empty());
    }

    #[test]
    fn test_format_target_files_warning() {
        let verification = TargetFilesVerification {
            files_with_changes: vec![],
            files_without_changes: vec!["src/test.rs".to_string(), "src/main.rs".to_string()],
            commits: vec![],
            actual_files_changed: vec![],
        };

        let warning = format_target_files_warning("2026-01-27-001-abc", &verification);

        assert!(warning.contains("2026-01-27-001-abc"));
        assert!(warning.contains("predicted"));
        assert!(warning.contains("src/test.rs"));
        assert!(warning.contains("src/main.rs"));
        assert!(warning.contains("Prediction mismatch"));
    }

    #[test]
    fn test_target_files_verification_struct() {
        let verification = TargetFilesVerification {
            files_with_changes: vec!["src/lib.rs".to_string()],
            files_without_changes: vec!["src/test.rs".to_string()],
            commits: vec!["abc1234".to_string(), "def5678".to_string()],
            actual_files_changed: vec![("src/lib.rs".to_string(), 50)],
        };

        assert_eq!(verification.files_with_changes.len(), 1);
        assert_eq!(verification.files_without_changes.len(), 1);
        assert_eq!(verification.commits.len(), 2);
        assert_eq!(verification.actual_files_changed.len(), 1);
    }

    #[test]
    fn test_format_target_files_warning_with_mismatch() {
        // Test case where target_files exist but changes were made to different files
        let verification = TargetFilesVerification {
            files_with_changes: vec![],
            files_without_changes: vec!["src/cmd/finalize.rs".to_string()],
            commits: vec!["abc1234".to_string()],
            actual_files_changed: vec![
                ("src/commands/finalize.rs".to_string(), 128),
                ("tests/finalize_test.rs".to_string(), -10),
            ],
        };

        let warning = format_target_files_warning("2026-01-29-00a-qza", &verification);

        // Check spec ID is present
        assert!(warning.contains("2026-01-29-00a-qza"));

        // Check predicted file is shown
        assert!(warning.contains("src/cmd/finalize.rs"));

        // Check actual files changed are shown
        assert!(warning.contains("src/commands/finalize.rs"));
        assert!(warning.contains("tests/finalize_test.rs"));

        // Check reassuring message
        assert!(warning.contains("Prediction mismatch"));
    }

    #[test]
    fn test_parse_member_specs_requires_section() {
        let output = r#"
## Member 1: Base Configuration

This member implements the base configuration system.

### Provides
- Config type
- Default settings

**Affected Files:**
- src/config.rs

## Member 2: Feature A

This member implements feature A.

### Requires
- Uses `Config` from Member 1

**Affected Files:**
- src/feature_a.rs

## Member 3: Feature B

This member implements feature B.

### Requires
- Requires Member 1 for configuration
- Uses types from Member 2

**Affected Files:**
- src/feature_b.rs

## Member 4: Integration

This member integrates all features.

### Requires
- Uses Member 2 and Member 3

**Affected Files:**
- src/integration.rs
"#;

        let result = parse_member_specs_from_output(output);
        assert!(result.is_ok());

        let members = result.unwrap();
        assert_eq!(members.len(), 4);

        // Member 1 should have no dependencies
        assert_eq!(members[0].dependencies.len(), 0);

        // Member 2 should depend on Member 1
        assert_eq!(members[1].dependencies, vec![1]);

        // Member 3 should depend on Members 1 and 2
        let mut deps = members[2].dependencies.clone();
        deps.sort();
        assert_eq!(deps, vec![1, 2]);

        // Member 4 should depend on Members 2 and 3
        let mut deps = members[3].dependencies.clone();
        deps.sort();
        assert_eq!(deps, vec![2, 3]);
    }

    #[test]
    fn test_parse_member_specs_preserves_dependencies_fallback() {
        let output = r#"
## Member 1: Base

Base implementation.

**Dependencies:** None

**Affected Files:**
- src/base.rs

## Member 2: Feature

Feature implementation.

**Dependencies:** Member 1

**Affected Files:**
- src/feature.rs
"#;

        let result = parse_member_specs_from_output(output);
        assert!(result.is_ok());

        let members = result.unwrap();
        assert_eq!(members.len(), 2);

        // Member 1 should have no dependencies
        assert_eq!(members[0].dependencies.len(), 0);

        // Member 2 should depend on Member 1 (from **Dependencies:** section)
        assert_eq!(members[1].dependencies, vec![1]);
    }

    #[test]
    fn test_requires_preferred_over_dependencies() {
        // Test that Requires section takes precedence over Dependencies line
        let output = r#"
## Member 1: Base

Base implementation.

**Affected Files:**
- src/base.rs

## Member 2: Feature

Feature implementation.

### Requires
- Uses `WatchConfig` from Member 1

**Dependencies:** Member 5

**Affected Files:**
- src/feature.rs
"#;

        let result = parse_member_specs_from_output(output);
        assert!(result.is_ok());

        let members = result.unwrap();
        assert_eq!(members.len(), 2);

        // Member 2 should depend ONLY on Member 1 (from Requires), not Member 5 (from Dependencies)
        assert_eq!(members[1].dependencies, vec![1]);
    }

    #[test]
    fn test_cycle_detection_and_removal() {
        // Test that cycles are detected and broken
        let output = r#"
## Member 1: Base

Base implementation.

**Affected Files:**
- src/base.rs

## Member 2: Feature A

Feature A.

### Requires
- Uses Member 3

**Affected Files:**
- src/feature_a.rs

## Member 3: Feature B

Feature B.

### Requires
- Uses Member 2

**Affected Files:**
- src/feature_b.rs
"#;

        let result = parse_member_specs_from_output(output);
        assert!(result.is_ok());

        let members = result.unwrap();
        assert_eq!(members.len(), 3);

        // After cycle removal, at least one of the circular edges should be removed
        // Member 2 depends on Member 3, Member 3 depends on Member 2 (cycle)
        let member2_deps = &members[1].dependencies;
        let member3_deps = &members[2].dependencies;

        // At least one of these should be empty to break the cycle
        assert!(
            member2_deps.is_empty()
                || member3_deps.is_empty()
                || !member2_deps.contains(&3)
                || !member3_deps.contains(&2),
            "Cycle should be broken"
        );
    }

    #[test]
    fn test_self_reference_removal() {
        // Test that self-references are removed
        let output = r#"
## Member 1: Feature

Feature implementation.

### Requires
- Uses Member 1

**Affected Files:**
- src/feature.rs
"#;

        let result = parse_member_specs_from_output(output);
        assert!(result.is_ok());

        let members = result.unwrap();
        assert_eq!(members.len(), 1);

        // Self-reference should be removed
        assert_eq!(members[0].dependencies.len(), 0);
    }

    #[test]
    fn test_both_requires_and_dependencies_prefer_requires() {
        // Test with both sections present - should use only Requires
        let output = r#"
## Member 1: Base

Base config.

**Affected Files:**
- src/config.rs

## Member 2: Logger

Logger module.

**Affected Files:**
- src/logger.rs

## Member 3: Feature

Feature implementation.

### Requires
- Uses `Config` from Member 1
- Uses `Logger` from Member 2

**Dependencies:** None

**Affected Files:**
- src/feature.rs
"#;

        let result = parse_member_specs_from_output(output);
        assert!(result.is_ok());

        let members = result.unwrap();
        assert_eq!(members.len(), 3);

        // Member 3 should depend on Members 1 and 2 (from Requires), not "None" (from Dependencies)
        let mut deps = members[2].dependencies.clone();
        deps.sort();
        assert_eq!(deps, vec![1, 2]);
    }
}
