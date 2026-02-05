//! Spec splitting functionality - breaks large specs into smaller member specs

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

use chant::config::Config;
use chant::paths::PROMPTS_DIR;
use chant::prompt;
use chant::score::isolation::calculate_isolation;
use chant::score::splittability::calculate_splittability;
use chant::score::traffic_light::{determine_status, generate_suggestions};
use chant::scoring::{SpecScore, SplittabilityGrade};
use chant::spec::{self, Spec, SpecFrontmatter, SpecStatus};

use crate::cmd;

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
pub(crate) struct MemberSpec {
    pub title: String,
    pub description: String,
    pub target_files: Option<Vec<String>>,
    pub dependencies: Vec<usize>, // Member numbers this depends on (1-indexed)
}

/// Result of dependency analysis from split prompt
#[derive(Debug, Clone)]
pub(crate) struct DependencyAnalysis {
    /// The dependency graph as text (for display)
    pub graph_text: String,
    /// Dependency edges: (from_member_num, to_member_num)
    #[allow(dead_code)]
    pub edges: Vec<(usize, usize)>,
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
            SpecStatus::Paused => {
                anyhow::bail!("Cannot split paused spec");
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

    // Build the split prompt
    let split_prompt = build_split_prompt(&spec, &config, &prompts_dir)?;

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

/// Build the split prompt by loading the template and assembling it with the spec
fn build_split_prompt(spec: &Spec, config: &Config, prompts_dir: &std::path::Path) -> Result<String> {
    // Load prompt from file
    let split_prompt_path = prompts_dir.join("split.md");
    if !split_prompt_path.exists() {
        anyhow::bail!("Split prompt not found: split.md");
    }

    // Assemble prompt for split analysis
    prompt::assemble(spec, &split_prompt_path, config)
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

#[cfg(test)]
mod tests {
    use super::*;
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
