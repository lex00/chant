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
use std::path::PathBuf;

use chant::config::Config;
use chant::diagnose;
use chant::git;
use chant::merge;
use chant::paths::{ARCHIVE_DIR, PROMPTS_DIR};
use chant::prompt;
use chant::replay::ReplayContext;
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
}

/// Split a pending spec into member specs
pub fn cmd_split(id: &str, override_model: Option<&str>, force: bool) -> Result<()> {
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

    // Parse member specs from agent output
    let members = parse_member_specs_from_output(&agent_output)?;

    if members.is_empty() {
        anyhow::bail!("Agent did not propose any member specs. Check the agent output in the log.");
    }

    println!(
        "{} Creating {} member specs for spec {}",
        "→".cyan(),
        members.len(),
        spec.id
    );

    // Validate members meet complexity thresholds
    const HAIKU_CRITERIA_TARGET: usize = 5;
    const HAIKU_FILES_TARGET: usize = 5;
    const HAIKU_WORDS_TARGET: usize = 200;

    for (index, member) in members.iter().enumerate() {
        let member_number = index + 1;
        let criteria_count = member.description.matches("- [ ]").count()
            + member.description.matches("- [x]").count()
            + member.description.matches("- [X]").count();
        let files_count = member.target_files.as_ref().map(|f| f.len()).unwrap_or(0);
        let word_count = member.description.split_whitespace().count();

        // Log warnings if member exceeds targets
        if criteria_count > HAIKU_CRITERIA_TARGET
            || files_count > HAIKU_FILES_TARGET
            || word_count > HAIKU_WORDS_TARGET
        {
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

    // Create member spec files
    let driver_id = spec.id.clone();
    for (index, member) in members.iter().enumerate() {
        let member_number = index + 1;
        let member_id = format!("{}.{}", driver_id, member_number);
        let member_filename = format!("{}.md", member_id);
        let member_path = specs_dir.join(&member_filename);

        // Create frontmatter with dependencies
        let depends_on = if index > 0 {
            Some(vec![format!("{}.{}", driver_id, index)])
        } else {
            None
        };

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

    // Update driver spec to type: group
    spec.frontmatter.r#type = "group".to_string();
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

/// Parse member specs from agent output (split analysis)
fn parse_member_specs_from_output(output: &str) -> Result<Vec<MemberSpec>> {
    let mut members = Vec::new();
    let mut current_member: Option<(String, String, Vec<String>)> = None;
    let mut collecting_files = false;
    let mut in_code_block = false;

    for line in output.lines() {
        // Check for member headers (## Member N: ...)
        if line.starts_with("## Member ") && line.contains(':') {
            // Save previous member if any
            if let Some((title, desc, files)) = current_member.take() {
                members.push(MemberSpec {
                    title,
                    description: desc.trim().to_string(),
                    target_files: if files.is_empty() { None } else { Some(files) },
                });
            }

            // Extract title from "## Member N: Title Here"
            if let Some(title_part) = line.split(':').nth(1) {
                let title = title_part.trim().to_string();
                current_member = Some((title, String::new(), Vec::new()));
                collecting_files = false;
            }
        } else if current_member.is_some() {
            // Check for code block markers
            if line.trim() == "```" {
                in_code_block = !in_code_block;
                if let Some((_, ref mut desc, _)) = &mut current_member {
                    desc.push_str(line);
                    desc.push('\n');
                }
                continue;
            }

            // Check for "Affected Files:" header
            if line.contains("**Affected Files:**") || line.contains("Affected Files:") {
                collecting_files = true;
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
                        if let Some((_, _, ref mut files)) = current_member {
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
            } else if !in_code_block {
                // Preserve ### headers and all content except "Affected Files" section
                if let Some((_, ref mut desc, _)) = &mut current_member {
                    desc.push_str(line);
                    desc.push('\n');
                }
            }
        }
    }

    // Save last member
    if let Some((title, desc, files)) = current_member {
        members.push(MemberSpec {
            title,
            description: desc.trim().to_string(),
            target_files: if files.is_empty() { None } else { Some(files) },
        });
    }

    if members.is_empty() {
        anyhow::bail!("No member specs found in agent output");
    }

    Ok(members)
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

/// Run the interactive wizard for selecting specs to merge
/// Returns (selected_spec_ids, delete_branch, rebase)
fn run_merge_wizard(
    all_specs: &[Spec],
    branch_prefix: &str,
    delete_branch: bool,
    rebase: bool,
) -> Result<(Vec<String>, bool, bool)> {
    use dialoguer::{Confirm, MultiSelect};

    // Get completed specs that have branches
    let mergeable_specs: Vec<(String, &Spec)> = all_specs
        .iter()
        .filter(|spec| spec.frontmatter.status == SpecStatus::Completed)
        .filter_map(|spec| {
            let branch_name = format!("{}{}", branch_prefix, spec.id);
            if git::branch_exists(&branch_name).unwrap_or(false) {
                Some((spec.id.clone(), spec))
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

/// Merge completed spec branches back to main
#[allow(clippy::too_many_arguments)]
pub fn cmd_merge(
    ids: &[String],
    all: bool,
    dry_run: bool,
    delete_branch: bool,
    continue_on_error: bool,
    yes: bool,
    rebase: bool,
    auto_resolve: bool,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load config
    let config = Config::load()?;
    let branch_prefix = &config.defaults.branch_prefix;
    let main_branch = merge::load_main_branch(&config);

    // Load all specs first (needed for wizard and validation)
    let all_specs = spec::load_all_specs(&specs_dir)?;

    // Handle wizard mode when no arguments provided
    let (final_ids, final_delete_branch, final_rebase) = if !all && ids.is_empty() {
        run_merge_wizard(&all_specs, branch_prefix, delete_branch, rebase)?
    } else {
        (ids.to_vec(), delete_branch, rebase)
    };

    // Validate arguments after wizard
    if !all && final_ids.is_empty() {
        anyhow::bail!(
            "Please specify one or more spec IDs, or use --all to merge all completed specs"
        );
    }

    // Get specs to merge using the merge module function
    let mut specs_to_merge = merge::get_specs_to_merge(&final_ids, all, &all_specs)?;

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
    let mut skipped_conflicts: Vec<(String, Vec<String>)> = Vec::new();

    println!(
        "{} Executing merges{}...",
        "→".cyan(),
        if rebase { " with rebase" } else { "" }
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

            match git::rebase_branch(&branch_name, &main_branch) {
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
                                &main_branch,
                                &rebase_result.conflicting_files,
                                &config,
                            ) {
                                Ok(()) => {
                                    println!("    {} Conflicts resolved", "✓".green());
                                }
                                Err(e) => {
                                    let error_msg = format!("Auto-resolve failed: {}", e);
                                    errors.push((spec_id.clone(), error_msg.clone()));
                                    skipped_conflicts
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

                            let error_msg = format!(
                                "Rebase conflict in: {}",
                                rebase_result.conflicting_files.join(", ")
                            );
                            errors.push((spec_id.clone(), error_msg.clone()));
                            skipped_conflicts
                                .push((spec_id.clone(), rebase_result.conflicting_files));
                            println!("    {} {} (use --auto to resolve)", "✗".red(), error_msg);
                            if !continue_on_error {
                                anyhow::bail!("Merge stopped at spec {}. Use --auto to auto-resolve conflicts.", spec_id);
                            }
                            continue;
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Rebase failed: {}", e);
                    errors.push((spec_id.clone(), error_msg.clone()));
                    println!("    {} {}", "✗".red(), error_msg);
                    if !continue_on_error {
                        anyhow::bail!("Merge stopped at spec {}.", spec_id);
                    }
                    continue;
                }
            }
        }

        // Check if this is a driver spec
        let is_driver = merge::is_driver_spec(spec, &all_specs);

        let merge_op_result = if is_driver {
            // Merge driver and its members
            merge::merge_driver_spec(
                spec,
                &all_specs,
                branch_prefix,
                &main_branch,
                final_delete_branch,
                false,
            )
        } else {
            // Merge single spec
            match git::merge_single_spec(
                spec_id,
                &branch_name,
                &main_branch,
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
                    anyhow::bail!(
                        "Merge stopped at spec {}. Use --continue-on-error to continue.",
                        spec_id
                    );
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

    // Display summary
    println!("\n{} Summary", "→".cyan());
    println!("{}", "─".repeat(60));
    println!("  {} Specs merged: {}", "✓".green(), merge_results.len());
    if !errors.is_empty() {
        println!("  {} Specs failed: {}", "✗".red(), errors.len());
        for (spec_id, error_msg) in &errors {
            println!("    - {}: {}", spec_id, error_msg);
        }
    }
    if final_delete_branch {
        let deleted_count = merge_results.iter().filter(|r| r.branch_deleted).count();
        println!("  {} Branches deleted: {}", "✓".green(), deleted_count);
    }

    if !errors.is_empty() {
        println!("\n{}", "Some merges failed.".yellow());
        return Ok(());
    }

    println!("\n{} All specs merged successfully.", "✓".green());
    Ok(())
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
            false, // pr
            false, // force
            false, // parallel
            &[],   // label
            false, // finalize
            false, // allow_no_commits
            None,  // max_parallel
            false, // no_cleanup
            false, // cleanup
        )?;
    }

    Ok(())
}

// ============================================================================
// REPLAY
// ============================================================================

/// Replay a completed spec by executing it again with the same or updated options
pub fn cmd_replay(
    id: &str,
    prompt: Option<&str>,
    branch: Option<String>,
    pr: bool,
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
    if pr {
        println!("    {} Create pull request", "∘".cyan());
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
    if branch.is_none() && !pr && !force && prompt.is_none() {
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
        pr,
        true,  // force=true to bypass guards in cmd_work for replay
        false, // parallel
        &[],   // label
        false, // finalize
        false, // allow_no_commits
        None,  // max_parallel
        false, // no_cleanup
        false, // cleanup
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
/// and adds model information to frontmatter
pub fn cmd_finalize(id: &str, specs_dir: &std::path::Path) -> Result<()> {
    use crate::cmd::finalize;
    use chant::spec;

    // Resolve the spec
    let spec = spec::resolve_spec(specs_dir, id)?;
    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

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

    // Load the config for model information
    let config = Config::load()?;

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
}
