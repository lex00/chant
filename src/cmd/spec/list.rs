//! Spec listing and filtering functionality
//!
//! Provides the `cmd_list` and `cmd_status` command functions
//! along with helper functions for filtering, sorting, and displaying specs.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use chant::config::Config;
use chant::id;
use chant::spec::{self, ApprovalStatus, Spec, SpecStatus};
use chant::spec_group;

use crate::render;

// ============================================================================
// MULTI-REPO HELPERS
// ============================================================================

/// Load specs from all configured repos (or a specific repo if specified)
pub(crate) fn load_specs_from_repos(repo_filter: Option<&str>) -> Result<Vec<Spec>> {
    // Load global config
    let config = Config::load_merged()?;

    if config.repos.is_empty() {
        anyhow::bail!(
            "No repos configured in global config. \
             Please add repos to ~/.config/chant/config.md or use local mode without --global/--repo"
        );
    }

    // If repo_filter is specified, validate it exists
    if let Some(repo_name) = repo_filter {
        if !config.repos.iter().any(|r| r.name == repo_name) {
            anyhow::bail!(
                "Repository '{}' not found in global config. Available repos: {}",
                repo_name,
                config
                    .repos
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    let mut all_specs = Vec::new();

    for repo_config in &config.repos {
        // Skip if filtering by repo and this isn't it
        if let Some(filter) = repo_filter {
            if repo_config.name != filter {
                continue;
            }
        }

        // Expand path (handle ~ and environment variables)
        let repo_path = shellexpand::tilde(&repo_config.path).to_string();
        let repo_path = PathBuf::from(repo_path);

        let specs_dir = repo_path.join(".chant/specs");

        // Gracefully skip if repo doesn't exist or has no specs dir
        if !specs_dir.exists() {
            eprintln!(
                "{} Warning: Specs directory not found for repo '{}' at {}",
                "⚠".yellow(),
                repo_config.name,
                specs_dir.display()
            );
            continue;
        }

        // Load specs from this repo
        match spec::load_all_specs(&specs_dir) {
            Ok(mut repo_specs) => {
                // Add repo prefix to each spec ID
                for spec in &mut repo_specs {
                    spec.id = format!("{}:{}", repo_config.name, spec.id);
                }
                all_specs.extend(repo_specs);
            }
            Err(e) => {
                eprintln!(
                    "{} Failed to load specs from repo '{}': {}",
                    "⚠".yellow(),
                    repo_config.name,
                    e
                );
            }
        }
    }

    if all_specs.is_empty() && repo_filter.is_none() {
        eprintln!(
            "{} No specs found in any configured repositories",
            "⚠".yellow()
        );
    }

    Ok(all_specs)
}

// ============================================================================
// GIT METADATA HELPERS
// ============================================================================

/// Git metadata for a spec file, loaded in batch for performance
#[derive(Debug, Clone)]
struct SpecGitMetadata {
    /// Author who first created the spec file
    creator: Option<String>,
    /// Last modification time from git log
    last_modified: Option<chrono::DateTime<chrono::Local>>,
}

/// Batch load git metadata (creator and last_modified) for all spec files.
/// This uses targeted git commands to minimize history traversal.
///
/// Performance optimization:
/// - last_modified: Uses limited history (recent 200 commits) for fast lookup
/// - creator: Only loaded when `include_creator` is true, as it requires full history scan
fn batch_load_spec_git_metadata(
    specs_dir: &Path,
    include_creator: bool,
) -> HashMap<String, SpecGitMetadata> {
    let mut metadata: HashMap<String, SpecGitMetadata> = HashMap::new();

    // Get last_modified: Use limited history for fast lookup
    // We check the last 200 commits which should cover all active specs
    let output = Command::new("git")
        .args([
            "log",
            "-200", // Limit to recent commits for speed
            "--name-only",
            "--format=COMMIT|%an|%aI",
            "--",
        ])
        .arg(specs_dir)
        .output();

    if let Ok(ref output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_author: Option<String> = None;
            let mut current_timestamp: Option<chrono::DateTime<chrono::Local>> = None;

            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if let Some(commit_data) = line.strip_prefix("COMMIT|") {
                    let parts: Vec<&str> = commit_data.splitn(2, '|').collect();
                    if parts.len() == 2 {
                        current_author = Some(parts[0].to_string());
                        current_timestamp = chrono::DateTime::parse_from_rfc3339(parts[1])
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Local));
                    }
                } else if line.starts_with(".chant/specs/") && line.ends_with(".md") {
                    let spec_id = Path::new(line)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string());

                    if let Some(spec_id) = spec_id {
                        let entry = metadata.entry(spec_id).or_insert(SpecGitMetadata {
                            creator: None,
                            last_modified: None,
                        });

                        // First occurrence = most recent commit = last_modified
                        if entry.last_modified.is_none() {
                            entry.last_modified = current_timestamp;
                            // Also use this as tentative creator (will be overwritten if include_creator)
                            if !include_creator {
                                entry.creator = current_author.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    // Get creator: Only when needed (requires scanning full history for file additions)
    if include_creator {
        let output = Command::new("git")
            .args([
                "log",
                "--name-only",
                "--format=COMMIT|%an|%aI",
                "--diff-filter=A", // Only file additions
                "--",
            ])
            .arg(specs_dir)
            .output();

        if let Ok(ref output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut current_author: Option<String> = None;

                for line in stdout.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Some(commit_data) = line.strip_prefix("COMMIT|") {
                        let parts: Vec<&str> = commit_data.splitn(2, '|').collect();
                        if !parts.is_empty() {
                            current_author = Some(parts[0].to_string());
                        }
                    } else if line.starts_with(".chant/specs/") && line.ends_with(".md") {
                        let spec_id = Path::new(line)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string());

                        if let Some(spec_id) = spec_id {
                            if let Some(entry) = metadata.get_mut(&spec_id) {
                                // This is the commit that added the file = creator
                                if let Some(ref author) = current_author {
                                    entry.creator = Some(author.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    metadata
}

/// Format a duration as a relative time string (e.g., "2h", "3d", "1w")
fn format_relative_time(datetime: chrono::DateTime<chrono::Local>) -> String {
    let now = chrono::Local::now();
    let duration = now.signed_duration_since(datetime);

    if duration.num_minutes() < 1 {
        "now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d", duration.num_days())
    } else if duration.num_weeks() < 4 {
        format!("{}w", duration.num_weeks())
    } else {
        format!("{}mo", duration.num_days() / 30)
    }
}

/// Parse a duration string like "2h", "1d", "1w" into a chrono::Duration
fn parse_duration(s: &str) -> Option<chrono::Duration> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    let (num_str, unit) = if s.ends_with("mo") {
        (&s[..s.len() - 2], "mo")
    } else {
        let unit_char = s.chars().last()?;
        (
            &s[..s.len() - 1],
            match unit_char {
                'm' => "m",
                'h' => "h",
                'd' => "d",
                'w' => "w",
                _ => return None,
            },
        )
    };

    let num: i64 = num_str.parse().ok()?;
    match unit {
        "m" => Some(chrono::Duration::minutes(num)),
        "h" => Some(chrono::Duration::hours(num)),
        "d" => Some(chrono::Duration::days(num)),
        "w" => Some(chrono::Duration::weeks(num)),
        "mo" => Some(chrono::Duration::days(num * 30)),
        _ => None,
    }
}

/// Count comments in the approval discussion section
fn count_approval_comments(body: &str) -> usize {
    let discussion_header = "## Approval Discussion";

    if let Some(pos) = body.find(discussion_header) {
        let section_start = pos + discussion_header.len();
        let rest = &body[section_start..];
        // Find the next section heading or end of body
        let section_end = rest.find("\n## ").unwrap_or(rest.len());
        let section = &rest[..section_end];

        // Count entries that start with **name** pattern (bold names indicate comments)
        section
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("**") && trimmed.contains("** -")
            })
            .count()
    } else {
        0
    }
}

/// Check if a name is mentioned in the approval discussion section
fn mentions_person(body: &str, name: &str) -> bool {
    let discussion_header = "## Approval Discussion";
    let name_lower = name.to_lowercase();

    if let Some(pos) = body.find(discussion_header) {
        let section_start = pos + discussion_header.len();
        let rest = &body[section_start..];
        // Find the next section heading or end of body
        let section_end = rest.find("\n## ").unwrap_or(rest.len());
        let section = &rest[..section_end].to_lowercase();

        section.contains(&name_lower)
    } else {
        false
    }
}

/// Check if silent mode is enabled via environment variable.
fn is_silent_mode() -> bool {
    std::env::var("CHANT_SILENT_MODE")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or_default()
}

// ============================================================================
// LIST COMMAND
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn cmd_list(
    ready_only: bool,
    labels: &[String],
    type_filter: Option<&str>,
    status_filter: Option<&str>,
    global: bool,
    repo: Option<&str>,
    project: Option<&str>,
    approval_filter: Option<&str>,
    created_by_filter: Option<&str>,
    activity_since_filter: Option<&str>,
    mentions_filter: Option<&str>,
    count_only: bool,
    main_only: bool,
) -> Result<()> {
    let is_multi_repo = global || repo.is_some();

    // Get specs_dir for file path resolution
    let specs_dir = if is_multi_repo {
        None
    } else {
        Some(crate::cmd::ensure_initialized()?)
    };

    let mut specs = if is_multi_repo {
        // Load specs from multiple repos
        load_specs_from_repos(repo)?
    } else {
        // Load specs from current repo only (existing behavior)
        let use_branch_resolution = !main_only;
        spec::load_all_specs_with_options(specs_dir.as_ref().unwrap(), use_branch_resolution)?
    };

    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Batch load git metadata for all specs (single git call instead of 2 per spec)
    // Only load creator info when filtering by creator (requires full history scan)
    let need_creator = created_by_filter.is_some();
    let git_metadata = if let Some(ref dir) = specs_dir {
        batch_load_spec_git_metadata(dir, need_creator)
    } else {
        HashMap::new()
    };

    // Exclude cancelled specs
    specs.retain(|s| s.frontmatter.status != SpecStatus::Cancelled);

    if ready_only {
        let all_specs = specs.clone();
        specs.retain(|s| s.is_ready(&all_specs));
        // Filter out group specs - they are containers, not actionable work
        specs.retain(|s| s.frontmatter.r#type != "group");
    }

    // Filter by type if specified
    if let Some(type_val) = type_filter {
        specs.retain(|s| s.frontmatter.r#type == type_val);
    }

    // Filter by status if specified
    if let Some(status_val) = status_filter {
        let status_lower = status_val.to_lowercase();
        match status_lower.as_str() {
            "blocked" => {
                // "blocked" is a computed state: pending specs with incomplete dependencies
                // or specs explicitly marked with status: blocked
                let all_specs_clone = specs.clone();
                specs.retain(|s| {
                    s.frontmatter.status == SpecStatus::Blocked
                        || (s.frontmatter.status == SpecStatus::Pending
                            && s.is_blocked(&all_specs_clone))
                });
            }
            "ready" => {
                // "ready" is a computed state: pending with all dependencies met
                let all_specs_clone = specs.clone();
                specs.retain(|s| s.is_ready(&all_specs_clone));
            }
            _ => {
                let target_status = match status_lower.as_str() {
                    "pending" => SpecStatus::Pending,
                    "in_progress" | "inprogress" => SpecStatus::InProgress,
                    "completed" => SpecStatus::Completed,
                    "failed" => SpecStatus::Failed,
                    "needs_attention" | "needsattention" => SpecStatus::NeedsAttention,
                    "cancelled" => SpecStatus::Cancelled,
                    _ => {
                        anyhow::bail!("Invalid status filter: {}. Valid options: pending, in_progress, completed, failed, blocked, cancelled, ready, needs_attention", status_val);
                    }
                };
                specs.retain(|s| s.frontmatter.status == target_status);
            }
        }
    }

    // Filter by labels if specified (OR logic - show specs with any matching label)
    if !labels.is_empty() {
        specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    // Filter by project if specified
    if let Some(proj_val) = project {
        specs.retain(|s| {
            // Parse the spec ID to check if it matches the project
            if let Ok(parsed_id) = id::SpecId::parse(&s.id) {
                parsed_id.project.as_deref() == Some(proj_val)
            } else {
                false
            }
        });
    }

    // Filter by approval status if specified
    if let Some(approval_val) = approval_filter {
        let approval_lower = approval_val.to_lowercase();
        specs.retain(|s| {
            if let Some(ref approval) = s.frontmatter.approval {
                match approval_lower.as_str() {
                    "pending" => approval.status == ApprovalStatus::Pending,
                    "approved" => approval.status == ApprovalStatus::Approved,
                    "rejected" => approval.status == ApprovalStatus::Rejected,
                    _ => false,
                }
            } else {
                // Specs without approval section are not in the approval workflow
                false
            }
        });
    }

    // Filter by creator if specified (uses batch-loaded metadata)
    if let Some(creator) = created_by_filter {
        let creator_lower = creator.to_lowercase();
        specs.retain(|s| {
            if let Some(meta) = git_metadata.get(&s.id) {
                if let Some(ref file_creator) = meta.creator {
                    file_creator.to_lowercase().contains(&creator_lower)
                } else {
                    false
                }
            } else {
                false // No metadata available
            }
        });
    }

    // Filter by activity since if specified (uses batch-loaded metadata)
    if let Some(duration_str) = activity_since_filter {
        if let Some(duration) = parse_duration(duration_str) {
            let cutoff = chrono::Local::now() - duration;
            specs.retain(|s| {
                if let Some(meta) = git_metadata.get(&s.id) {
                    if let Some(last_modified) = meta.last_modified {
                        last_modified >= cutoff
                    } else {
                        false
                    }
                } else {
                    false // No metadata available
                }
            });
        } else {
            anyhow::bail!(
                "Invalid duration format: '{}'. Valid formats: 2h, 1d, 1w, 2mo",
                duration_str
            );
        }
    }

    // Filter by mentions in approval discussion if specified
    if let Some(name) = mentions_filter {
        specs.retain(|s| mentions_person(&s.body, name));
    }

    // Handle count-only mode
    if count_only {
        println!("{}", specs.len());
        return Ok(());
    }

    if specs.is_empty() {
        if !chant::ui::is_quiet() {
            if ready_only && !labels.is_empty() {
                println!("No ready specs with specified labels.");
            } else if ready_only {
                println!("No ready specs.");
            } else if !labels.is_empty() {
                println!("No specs with specified labels.");
            } else {
                println!("No specs. Create one with `chant add \"description\"`");
            }
        }
        return Ok(());
    }

    for spec in &specs {
        let icon = if spec.frontmatter.r#type == "conflict" {
            "⚡".yellow()
        } else {
            render::status_icon(&spec.frontmatter.status)
        };

        // Build approval status marker
        let approval_marker = if let Some(ref approval) = spec.frontmatter.approval {
            match approval.status {
                ApprovalStatus::Pending if approval.required => {
                    format!(" {}", "[needs approval]".yellow())
                }
                ApprovalStatus::Rejected => {
                    format!(" {}", "[rejected]".red())
                }
                ApprovalStatus::Approved => {
                    format!(" {}", "[approved]".green())
                }
                _ => String::new(),
            }
        } else {
            String::new()
        };

        // Build visual indicators (using batch-loaded metadata for performance)
        let mut indicators: Vec<String> = Vec::new();

        // Created by and last activity indicators (from batch-loaded git metadata)
        if let Some(meta) = git_metadata.get(&spec.id) {
            if let Some(ref creator) = meta.creator {
                indicators.push(format!("👤 {}", creator.dimmed()));
            }

            if let Some(last_modified) = meta.last_modified {
                let relative = format_relative_time(last_modified);
                indicators.push(format!("↩ {}", relative.dimmed()));
            }
        }

        // Comment count in approval discussion
        let comment_count = count_approval_comments(&spec.body);
        if comment_count > 0 {
            indicators.push(format!("💬 {}", comment_count.to_string().dimmed()));
        }

        // Approved by indicator (from frontmatter)
        if let Some(ref approval) = spec.frontmatter.approval {
            if approval.status == ApprovalStatus::Approved {
                if let Some(ref by) = approval.by {
                    indicators.push(format!("✓ {}", by.green()));
                }
            }
        }

        // Branch indicator for in_progress specs
        if spec.frontmatter.status == spec::SpecStatus::InProgress {
            indicators.push("[branch]".dimmed().to_string());
        }

        let indicators_str = if indicators.is_empty() {
            String::new()
        } else {
            format!(" {}", indicators.join(" "))
        };

        if !chant::ui::is_quiet() {
            println!(
                "{} {}{} {}{}",
                icon,
                spec.id.cyan(),
                approval_marker,
                spec.title.as_deref().unwrap_or("(no title)"),
                indicators_str
            );
        }
    }

    Ok(())
}

// ============================================================================
// STATUS COMMAND
// ============================================================================

pub fn cmd_status(
    global: bool,
    repo_filter: Option<&str>,
    watch: bool,
    brief: bool,
    json: bool,
) -> Result<()> {
    // Validate mutually exclusive flags
    if brief && json {
        anyhow::bail!("Error: --brief and --json are mutually exclusive. Use one or the other.\n\nUsage: chant status [--brief | --json] [--watch]");
    }

    if watch {
        cmd_status_watch(global, repo_filter, brief, json)
    } else {
        cmd_status_once(global, repo_filter, brief, json)
    }
}

fn cmd_status_once(global: bool, repo_filter: Option<&str>, brief: bool, json: bool) -> Result<()> {
    if global || repo_filter.is_some() {
        // Multi-repo status output
        let specs = load_specs_from_repos(repo_filter)?;
        let mut per_repo_stats: HashMap<String, (usize, usize, usize, usize)> = HashMap::new();

        for spec in &specs {
            // Extract repo prefix from spec ID (format: "repo:spec-id")
            let repo_name = if let Some(idx) = spec.id.find(':') {
                spec.id[..idx].to_string()
            } else {
                "local".to_string()
            };

            let entry = per_repo_stats.entry(repo_name).or_insert((0, 0, 0, 0));
            match spec.frontmatter.status {
                SpecStatus::Pending | SpecStatus::Ready | SpecStatus::Blocked => entry.0 += 1,
                SpecStatus::InProgress => entry.1 += 1,
                SpecStatus::Completed => entry.2 += 1,
                SpecStatus::Failed | SpecStatus::NeedsAttention => entry.3 += 1,
                SpecStatus::Cancelled => {
                    // Cancelled specs are not counted in the summary
                }
            }
        }

        if !chant::ui::is_quiet() {
            println!("{}", "Chant Status (Global)".bold());
            println!("====================");
        }

        // Sort repos by name for consistent output
        let mut repos: Vec<_> = per_repo_stats.into_iter().collect();
        repos.sort_by(|a, b| a.0.cmp(&b.0));

        let mut total_pending = 0;
        let mut total_in_progress = 0;
        let mut total_completed = 0;
        let mut total_failed = 0;

        for (repo_name, (pending, in_progress, completed, failed)) in repos {
            if !chant::ui::is_quiet() {
                println!("\n{}: {}", "Repository".bold(), repo_name.cyan());
                println!(
                    "  {:<18} {} | {:<18} {} | {:<18} {} | {:<18} {}",
                    "Pending",
                    pending,
                    "In Progress",
                    in_progress,
                    "Completed",
                    completed,
                    "Failed",
                    failed
                );
            }

            total_pending += pending;
            total_in_progress += in_progress;
            total_completed += completed;
            total_failed += failed;
        }

        let total = total_pending + total_in_progress + total_completed + total_failed;
        if !chant::ui::is_quiet() {
            println!("\n{}", "Total".bold());
            println!("─────");
            println!(
                "  {:<18} {} | {:<18} {} | {:<18} {} | {:<18} {}",
                "Pending",
                total_pending,
                "In Progress",
                total_in_progress,
                "Completed",
                total_completed,
                "Failed",
                total_failed
            );
            println!("  {:<18} {}", "Overall Total:", total);
        }
    } else {
        // Single repo status output - use formatter based on flags
        let specs_dir = crate::cmd::ensure_initialized()?;
        let status_data = chant::status::aggregate_status(&specs_dir)?;

        if !chant::ui::is_quiet() {
            if json {
                let output = chant::status::format_status_as_json(&status_data)?;
                println!("{}", output);
            } else if brief {
                let output = status_data.format_brief();
                println!("{}", output);
            } else {
                let output = chant::formatters::format_regular_status(&status_data);
                println!("{}", output);

                // Show silent mode indicator if enabled
                if is_silent_mode() {
                    println!(
                        "\n{} Silent mode enabled - specs are local-only",
                        "ℹ".cyan()
                    );
                }
            }
        }
    }

    Ok(())
}

fn cmd_status_watch(
    global: bool,
    repo_filter: Option<&str>,
    brief: bool,
    json: bool,
) -> Result<()> {
    use std::time::Duration;

    loop {
        // Clear the screen using ANSI escape codes
        // Try ANSI codes first, fall back to separator line if that fails
        if clear_screen().is_err() {
            // Fallback: print separator line
            println!("\n{}\n", "=".repeat(80));
        }

        // Display status once
        if let Err(e) = cmd_status_once(global, repo_filter, brief, json) {
            eprintln!("Error refreshing status: {}", e);
            // Continue watching even if there's an error
        }

        // Flush stdout to ensure output is visible immediately
        io::stdout().flush()?;

        // Sleep for 5 seconds
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Clear the screen using ANSI escape codes
fn clear_screen() -> io::Result<()> {
    // ANSI escape code to clear screen and move cursor to top-left
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush()
}
