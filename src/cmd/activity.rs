//! Activity command - git-based activity feed for spec operations.
//!
//! Parses git history to show a chronological feed of spec-related activities
//! including creation, approval, rejection, work, and completion.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::process::Command;

/// Activity types detected from git history
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityType {
    /// Spec was created (new spec file added)
    Created,
    /// Spec was approved (approval.status changed to approved)
    Approved,
    /// Spec was rejected (approval.status changed to rejected)
    Rejected,
    /// Spec was worked on (commit message contains `chant(SPEC-ID):`)
    Worked,
    /// Spec was completed (status changed to completed)
    Completed,
}

impl ActivityType {
    fn as_str(&self) -> &'static str {
        match self {
            ActivityType::Created => "CREATED",
            ActivityType::Approved => "APPROVED",
            ActivityType::Rejected => "REJECTED",
            ActivityType::Worked => "WORKED",
            ActivityType::Completed => "COMPLETED",
        }
    }

    fn colorize(&self, s: &str) -> colored::ColoredString {
        match self {
            ActivityType::Created => s.cyan(),
            ActivityType::Approved => s.green(),
            ActivityType::Rejected => s.red(),
            ActivityType::Worked => s.yellow(),
            ActivityType::Completed => s.green().bold(),
        }
    }
}

/// A single activity entry
#[derive(Debug, Clone)]
pub struct Activity {
    /// Commit hash, used for deduplication in activity tracking
    #[allow(dead_code)] // Field used for deduplication, kept for future use/debugging
    pub commit: String,
    /// Author name
    pub author: String,
    /// Unix timestamp
    pub timestamp: i64,
    /// Activity type
    pub activity_type: ActivityType,
    /// Spec ID (short form like "001-abc")
    pub spec_id: String,
    /// Description or spec title
    pub description: String,
}

/// Parse a duration string like "2h", "1d", "1w" into seconds
fn parse_duration(duration: &str) -> Result<i64> {
    let duration = duration.trim();
    if duration.is_empty() {
        anyhow::bail!("Empty duration string");
    }

    let (num_str, unit) = if let Some(stripped) = duration.strip_suffix('h') {
        (stripped, 'h')
    } else if let Some(stripped) = duration.strip_suffix('d') {
        (stripped, 'd')
    } else if let Some(stripped) = duration.strip_suffix('w') {
        (stripped, 'w')
    } else if let Some(stripped) = duration.strip_suffix('m') {
        (stripped, 'm')
    } else {
        anyhow::bail!(
            "Invalid duration format '{}'. Use format like '2h', '1d', '1w', '1m'",
            duration
        );
    };

    let num: i64 = num_str
        .parse()
        .with_context(|| format!("Invalid number in duration: {}", num_str))?;

    let seconds = match unit {
        'h' => num * 3600,
        'd' => num * 86400,
        'w' => num * 604800,
        'm' => num * 2592000, // 30 days
        _ => unreachable!(),
    };

    Ok(seconds)
}

/// Get git log for spec-related commits
fn get_spec_commits() -> Result<Vec<(String, String, i64, String)>> {
    // Get commits affecting .chant/specs/
    let output = Command::new("git")
        .args([
            "log",
            "--all",
            "--format=%H|%an|%at|%s",
            "--",
            ".chant/specs/",
        ])
        .output()
        .context("Failed to execute git log")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() != 4 {
            continue;
        }

        let hash = parts[0].to_string();
        let author = parts[1].to_string();
        let timestamp: i64 = parts[2].parse().unwrap_or(0);
        let subject = parts[3].to_string();

        commits.push((hash, author, timestamp, subject));
    }

    Ok(commits)
}

/// Get files changed in a commit
fn get_commit_files(commit: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-status", "-r", commit])
        .output()
        .context("Failed to get commit files")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            // parts[0] is status (A, M, D), parts[1] is filename
            files.push(format!("{}:{}", parts[0], parts[1]));
        }
    }

    Ok(files)
}

/// Get file content at a specific commit
fn get_file_at_commit(commit: &str, file: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit, file)])
        .output()
        .context("Failed to get file at commit")?;

    if !output.status.success() {
        return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get file content at parent commit
fn get_file_at_parent(commit: &str, file: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["show", &format!("{}^:{}", commit, file)])
        .output()
        .context("Failed to get file at parent")?;

    if !output.status.success() {
        return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract spec ID from a filename like ".chant/specs/2026-01-28-001-abc.md"
fn extract_spec_id(filename: &str) -> Option<String> {
    let path = std::path::Path::new(filename);
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Extract short spec ID (e.g., "001-abc" from "2026-01-28-001-abc")
fn short_spec_id(spec_id: &str) -> String {
    // Try to extract just the sequence and random parts
    let parts: Vec<&str> = spec_id.split('-').collect();
    if parts.len() >= 5 {
        // Format: 2026-01-28-001-abc -> 001-abc
        format!("{}-{}", parts[3], parts[4])
    } else {
        spec_id.to_string()
    }
}

/// Extract title from spec content
fn extract_title(content: &str) -> Option<String> {
    // Skip frontmatter
    let body = if let Some(stripped) = content.strip_prefix("---") {
        if let Some(end_idx) = stripped.find("---") {
            &stripped[end_idx + 3..]
        } else {
            content
        }
    } else {
        content
    };

    // Find first # heading
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }

    None
}

/// Check if content has approval.status: approved
fn has_approval_status(content: &str, status: &str) -> bool {
    // Check for approval block in frontmatter
    if !content.starts_with("---") {
        return false;
    }

    if let Some(end_idx) = content[3..].find("---") {
        let frontmatter = &content[3..end_idx + 3];
        // Look for approval section with status
        let mut in_approval = false;
        for line in frontmatter.lines() {
            let trimmed = line.trim();
            // Check if we're entering the approval block
            if trimmed == "approval:" || trimmed.starts_with("approval:") {
                in_approval = true;
                continue;
            }
            // Check if we're exiting the approval block (line doesn't start with space and isn't empty)
            if in_approval
                && !line.starts_with(' ')
                && !line.starts_with('\t')
                && !trimmed.is_empty()
            {
                in_approval = false;
            }
            // Check for status within approval block
            if in_approval && trimmed.starts_with("status:") {
                let value = trimmed.trim_start_matches("status:").trim();
                return value == status;
            }
        }
    }

    false
}

/// Check if content has status: completed in frontmatter
fn has_status_completed(content: &str) -> bool {
    if !content.starts_with("---") {
        return false;
    }

    if let Some(end_idx) = content[3..].find("---") {
        let frontmatter = &content[3..end_idx + 3];
        for line in frontmatter.lines() {
            let trimmed = line.trim();
            if trimmed == "status: completed" {
                return true;
            }
        }
    }

    false
}

/// Detect activities from a single commit
fn detect_activities(
    commit: &str,
    author: &str,
    timestamp: i64,
    subject: &str,
) -> Result<Vec<Activity>> {
    let mut activities = Vec::new();

    // Check for "chant(SPEC-ID):" pattern in commit message
    if let Some(start) = subject.find("chant(") {
        if let Some(end) = subject[start..].find("):") {
            let spec_id = &subject[start + 6..start + end];
            let description = subject[start + end + 2..].trim();

            // Check if it's a finalize commit
            if description.starts_with("finalize") {
                activities.push(Activity {
                    commit: commit.to_string(),
                    author: author.to_string(),
                    timestamp,
                    activity_type: ActivityType::Completed,
                    spec_id: spec_id.to_string(),
                    description: description.to_string(),
                });
                return Ok(activities);
            }

            activities.push(Activity {
                commit: commit.to_string(),
                author: author.to_string(),
                timestamp,
                activity_type: ActivityType::Worked,
                spec_id: spec_id.to_string(),
                description: description.to_string(),
            });
        }
    }

    // Check for special commit message patterns
    if subject.starts_with("chant: Add spec ") {
        // Spec creation via chant add
        if let Some(spec_id) = subject.strip_prefix("chant: Add spec ") {
            let files = get_commit_files(commit)?;
            let mut title = String::new();

            // Try to get title from the spec file
            for file_entry in &files {
                if file_entry.starts_with("A:") && file_entry.contains(".chant/specs/") {
                    let filename = &file_entry[2..];
                    if let Ok(content) = get_file_at_commit(commit, filename) {
                        if let Some(t) = extract_title(&content) {
                            title = t;
                            break;
                        }
                    }
                }
            }

            activities.push(Activity {
                commit: commit.to_string(),
                author: author.to_string(),
                timestamp,
                activity_type: ActivityType::Created,
                spec_id: spec_id.to_string(),
                description: title,
            });
            return Ok(activities);
        }
    }

    // Get changed files to detect other activity types
    let files = get_commit_files(commit)?;

    for file_entry in files {
        let parts: Vec<&str> = file_entry.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }

        let status = parts[0];
        let filename = parts[1];

        // Only process spec files
        if !filename.starts_with(".chant/specs/") || !filename.ends_with(".md") {
            continue;
        }

        let spec_id = match extract_spec_id(filename) {
            Some(id) => id,
            None => continue,
        };

        match status {
            "A" => {
                // New file added - spec created
                let content = get_file_at_commit(commit, filename)?;
                let title = extract_title(&content).unwrap_or_default();

                // Skip if we already added this spec from commit message
                if !activities
                    .iter()
                    .any(|a| a.spec_id == spec_id && a.activity_type == ActivityType::Created)
                {
                    activities.push(Activity {
                        commit: commit.to_string(),
                        author: author.to_string(),
                        timestamp,
                        activity_type: ActivityType::Created,
                        spec_id,
                        description: title,
                    });
                }
            }
            "M" => {
                // File modified - check for approval/rejection/completion
                let current = get_file_at_commit(commit, filename)?;
                let previous = get_file_at_parent(commit, filename)?;

                let title = extract_title(&current).unwrap_or_default();

                // Check for approval status change
                if has_approval_status(&current, "approved")
                    && !has_approval_status(&previous, "approved")
                {
                    activities.push(Activity {
                        commit: commit.to_string(),
                        author: author.to_string(),
                        timestamp,
                        activity_type: ActivityType::Approved,
                        spec_id: spec_id.clone(),
                        description: title.clone(),
                    });
                }

                if has_approval_status(&current, "rejected")
                    && !has_approval_status(&previous, "rejected")
                {
                    activities.push(Activity {
                        commit: commit.to_string(),
                        author: author.to_string(),
                        timestamp,
                        activity_type: ActivityType::Rejected,
                        spec_id: spec_id.clone(),
                        description: title.clone(),
                    });
                }

                // Check for status change to completed
                if has_status_completed(&current) && !has_status_completed(&previous) {
                    // Skip if we already have a completion activity for this spec from commit message
                    if !activities
                        .iter()
                        .any(|a| a.spec_id == spec_id && a.activity_type == ActivityType::Completed)
                    {
                        activities.push(Activity {
                            commit: commit.to_string(),
                            author: author.to_string(),
                            timestamp,
                            activity_type: ActivityType::Completed,
                            spec_id,
                            description: title,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(activities)
}

/// Filter activities by criteria
fn filter_activities(
    activities: Vec<Activity>,
    by: Option<&str>,
    since: Option<&str>,
    spec: Option<&str>,
) -> Result<Vec<Activity>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let since_timestamp = if let Some(duration) = since {
        let seconds = parse_duration(duration)?;
        now - seconds
    } else {
        0
    };

    Ok(activities
        .into_iter()
        .filter(|a| {
            // Filter by author
            if let Some(by_filter) = by {
                if !a.author.to_lowercase().contains(&by_filter.to_lowercase()) {
                    return false;
                }
            }

            // Filter by since
            if a.timestamp < since_timestamp {
                return false;
            }

            // Filter by spec
            if let Some(spec_filter) = spec {
                if !a.spec_id.contains(spec_filter) {
                    return false;
                }
            }

            true
        })
        .collect())
}

/// Main activity command implementation
pub fn cmd_activity(by: Option<&str>, since: Option<&str>, spec: Option<&str>) -> Result<()> {
    // Ensure chant is initialized
    let _specs_dir = crate::cmd::ensure_initialized()?;

    // Get all spec-related commits
    let commits = get_spec_commits()?;

    if commits.is_empty() {
        println!("{}", "No activity found.".yellow());
        return Ok(());
    }

    // Collect all activities
    let mut all_activities = Vec::new();
    let mut seen: HashMap<(String, String), bool> = HashMap::new(); // (spec_id, activity_type) -> seen

    for (hash, author, timestamp, subject) in commits {
        let activities = detect_activities(&hash, &author, timestamp, &subject)?;

        for activity in activities {
            // Deduplicate: only keep first occurrence of each (spec_id, activity_type) pair
            let key = (
                activity.spec_id.clone(),
                activity.activity_type.as_str().to_string(),
            );
            if seen.contains_key(&key) {
                continue;
            }
            seen.insert(key, true);
            all_activities.push(activity);
        }
    }

    // Apply filters
    all_activities = filter_activities(all_activities, by, since, spec)?;

    // Sort by timestamp (most recent first)
    all_activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    if all_activities.is_empty() {
        println!("{}", "No activity found matching filters.".yellow());
        return Ok(());
    }

    // Print activities
    for activity in all_activities {
        let datetime =
            chrono::DateTime::from_timestamp(activity.timestamp, 0).unwrap_or_else(|| {
                chrono::DateTime::from_timestamp(0, 0).expect("epoch should be valid")
            });
        let date_str = datetime.format("%Y-%m-%d %H:%M").to_string();

        let action_str = activity.activity_type.as_str();
        let action_colored = activity.activity_type.colorize(action_str);

        let short_id = short_spec_id(&activity.spec_id);

        // Truncate description if too long
        let desc = if activity.description.len() > 40 {
            format!("{}...", &activity.description[..37])
        } else {
            activity.description.clone()
        };

        println!(
            "{}  {:<12} {:<12} {:<10} {}",
            date_str.dimmed(),
            activity.author,
            action_colored,
            short_id.cyan(),
            desc
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("2h").unwrap(), 7200);
        assert_eq!(parse_duration("24h").unwrap(), 86400);
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("1d").unwrap(), 86400);
        assert_eq!(parse_duration("7d").unwrap(), 604800);
    }

    #[test]
    fn test_parse_duration_weeks() {
        assert_eq!(parse_duration("1w").unwrap(), 604800);
        assert_eq!(parse_duration("2w").unwrap(), 1209600);
    }

    #[test]
    fn test_parse_duration_months() {
        assert_eq!(parse_duration("1m").unwrap(), 2592000);
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("2x").is_err());
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn test_short_spec_id() {
        assert_eq!(short_spec_id("2026-01-28-001-abc"), "001-abc");
        assert_eq!(short_spec_id("2026-01-28-123-xyz"), "123-xyz");
        assert_eq!(short_spec_id("short"), "short");
    }

    #[test]
    fn test_extract_spec_id() {
        assert_eq!(
            extract_spec_id(".chant/specs/2026-01-28-001-abc.md"),
            Some("2026-01-28-001-abc".to_string())
        );
        assert_eq!(extract_spec_id("other/path.txt"), Some("path".to_string()));
    }

    #[test]
    fn test_extract_title() {
        let content = "---\nstatus: pending\n---\n\n# My Title\n\nBody text";
        assert_eq!(extract_title(content), Some("My Title".to_string()));

        let no_title = "---\nstatus: pending\n---\n\nBody without title";
        assert_eq!(extract_title(no_title), None);
    }

    #[test]
    fn test_has_approval_status() {
        let approved = "---\napproval:\n  status: approved\n  by: alice\n---\n# Title";
        assert!(has_approval_status(approved, "approved"));
        assert!(!has_approval_status(approved, "rejected"));

        let rejected = "---\napproval:\n  status: rejected\n  by: bob\n---\n# Title";
        assert!(has_approval_status(rejected, "rejected"));
        assert!(!has_approval_status(rejected, "approved"));

        let no_approval = "---\nstatus: pending\n---\n# Title";
        assert!(!has_approval_status(no_approval, "approved"));
    }

    #[test]
    fn test_has_status_completed() {
        let completed = "---\nstatus: completed\nmodel: test\n---\n# Title";
        assert!(has_status_completed(completed));

        let pending = "---\nstatus: pending\n---\n# Title";
        assert!(!has_status_completed(pending));

        let in_progress = "---\nstatus: in_progress\n---\n# Title";
        assert!(!has_status_completed(in_progress));
    }

    #[test]
    fn test_activity_type_display() {
        assert_eq!(ActivityType::Created.as_str(), "CREATED");
        assert_eq!(ActivityType::Approved.as_str(), "APPROVED");
        assert_eq!(ActivityType::Rejected.as_str(), "REJECTED");
        assert_eq!(ActivityType::Worked.as_str(), "WORKED");
        assert_eq!(ActivityType::Completed.as_str(), "COMPLETED");
    }
}
