//! Worktree status command for debugging worktree state.
//!
//! Provides a status view of all chant-related git worktrees,
//! showing their associated specs, branches, and health status.

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use crate::cmd::cleanup::{format_age_secs, format_bytes};
use chant::paths::SPECS_DIR;
use chant::spec::{Spec, SpecStatus};

/// Information about a git worktree from `git worktree list --porcelain`
#[derive(Debug, Clone)]
struct GitWorktreeEntry {
    /// Path to the worktree
    path: PathBuf,
    /// HEAD commit hash
    head: String,
    /// Branch name (without refs/heads/ prefix)
    branch: Option<String>,
    /// Whether the worktree is prunable
    prunable: bool,
    /// Reason for being prunable
    prunable_reason: Option<String>,
}

/// Parse the output of `git worktree list --porcelain`
fn parse_worktree_list(output: &str) -> Vec<GitWorktreeEntry> {
    let mut entries = Vec::new();
    let mut current: Option<GitWorktreeEntry> = None;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            // Save previous entry if exists
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            // Start new entry
            let path = line.strip_prefix("worktree ").unwrap_or("");
            current = Some(GitWorktreeEntry {
                path: PathBuf::from(path),
                head: String::new(),
                branch: None,
                prunable: false,
                prunable_reason: None,
            });
        } else if let Some(ref mut entry) = current {
            if line.starts_with("HEAD ") {
                entry.head = line.strip_prefix("HEAD ").unwrap_or("").to_string();
            } else if line.starts_with("branch ") {
                let branch = line.strip_prefix("branch ").unwrap_or("");
                // Strip refs/heads/ prefix if present
                let branch = branch.strip_prefix("refs/heads/").unwrap_or(branch);
                entry.branch = Some(branch.to_string());
            } else if line.starts_with("prunable ") {
                entry.prunable = true;
                entry.prunable_reason =
                    Some(line.strip_prefix("prunable ").unwrap_or("").to_string());
            } else if line == "prunable" {
                entry.prunable = true;
            }
        }
    }

    // Don't forget the last entry
    if let Some(entry) = current {
        entries.push(entry);
    }

    entries
}

/// Extract spec ID from a worktree path or branch name
fn extract_spec_id(entry: &GitWorktreeEntry, branch_prefix: &str) -> Option<String> {
    // Try to extract from branch name (e.g. chant/SPEC-ID or chant/frontend/SPEC-ID)
    if let Some(ref branch) = entry.branch {
        if let Some(spec_id) = branch.strip_prefix(branch_prefix) {
            return Some(spec_id.to_string());
        }
    }

    // Try to extract from path (/tmp/chant-SPEC-ID or /tmp/chant-project-SPEC-ID)
    if let Some(dir_name) = entry.path.file_name() {
        let name = dir_name.to_string_lossy();
        if let Some(spec_id) = name.strip_prefix("chant-") {
            return Some(spec_id.to_string());
        }
    }

    None
}

/// Information about a spec's current state
#[derive(Debug)]
struct SpecInfo {
    title: String,
    status: SpecStatus,
}

/// Look up spec information from the specs directory
fn lookup_spec(spec_id: &str) -> Option<SpecInfo> {
    let specs_dir = PathBuf::from(SPECS_DIR);
    if !specs_dir.exists() {
        return None;
    }

    // Try to find the spec file
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    if !spec_path.exists() {
        // Check archive
        let archive_path = specs_dir
            .parent()?
            .join("archive")
            .join(format!("{}.md", spec_id));
        if archive_path.exists() {
            if let Ok(spec) = Spec::load(&archive_path) {
                return Some(SpecInfo {
                    title: spec.title.unwrap_or_else(|| "(untitled)".to_string()),
                    status: spec.frontmatter.status,
                });
            }
        }
        return None;
    }

    if let Ok(spec) = Spec::load(&spec_path) {
        Some(SpecInfo {
            title: spec.title.unwrap_or_else(|| "(untitled)".to_string()),
            status: spec.frontmatter.status,
        })
    } else {
        None
    }
}

/// Get size of a directory recursively
fn dir_size(path: &PathBuf) -> u64 {
    if !path.exists() {
        return 0;
    }

    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        Ok(metadata) if metadata.is_dir() => std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| dir_size(&entry.path()))
            .sum(),
        _ => 0,
    }
}

/// Get age of a path in seconds
fn path_age_secs(path: &PathBuf) -> u64 {
    match std::fs::metadata(path) {
        Ok(metadata) => match metadata.modified() {
            Ok(modified_time) => match std::time::SystemTime::now().duration_since(modified_time) {
                Ok(duration) => duration.as_secs(),
                Err(_) => 0,
            },
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Format spec status with color
fn format_status(status: &SpecStatus) -> String {
    match status {
        SpecStatus::Pending => "pending".dimmed().to_string(),
        SpecStatus::Ready => "ready".cyan().to_string(),
        SpecStatus::InProgress => "in_progress".yellow().to_string(),
        SpecStatus::Paused => "paused".cyan().to_string(),
        SpecStatus::Completed => "completed".green().to_string(),
        SpecStatus::Failed => "failed".red().to_string(),
        SpecStatus::NeedsAttention => "needs_attention".red().bold().to_string(),
        SpecStatus::Blocked => "blocked".magenta().to_string(),
        SpecStatus::Cancelled => "cancelled".dimmed().to_string(),
    }
}

/// Execute the worktree status command
pub fn cmd_worktree_status() -> Result<()> {
    // Get worktree list from git
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list worktrees: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let all_worktrees = parse_worktree_list(&stdout);

    let branch_prefix = chant::config::Config::load()
        .map(|c| c.defaults.branch_prefix.clone())
        .unwrap_or_else(|_| "chant/".to_string());

    // Filter to chant-related worktrees (skip main worktree)
    let chant_worktrees: Vec<_> = all_worktrees
        .into_iter()
        .filter(|wt| {
            // Include if path contains "chant-" or branch starts with configured prefix
            let path_match = wt
                .path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with("chant-"))
                .unwrap_or(false);
            let branch_match = wt
                .branch
                .as_ref()
                .map(|b| b.starts_with(branch_prefix.as_str()))
                .unwrap_or(false);
            path_match || branch_match
        })
        .collect();

    if chant_worktrees.is_empty() {
        println!("{}", "No chant worktrees found.".dimmed());
        return Ok(());
    }

    println!(
        "{} {} chant worktree{}:\n",
        "Found".cyan(),
        chant_worktrees.len(),
        if chant_worktrees.len() == 1 { "" } else { "s" }
    );

    for wt in &chant_worktrees {
        let spec_id = extract_spec_id(wt, &branch_prefix);
        let spec_info = spec_id.as_ref().and_then(|id| lookup_spec(id));

        // Print worktree path
        print!("  {}", wt.path.display().to_string().bold());

        // Print prunable warning
        if wt.prunable {
            print!(" {}", "[prunable]".red());
        }
        println!();

        // Print branch
        if let Some(ref branch) = wt.branch {
            println!("    Branch: {}", branch.cyan());
        }

        // Print HEAD (shortened)
        let short_head = if wt.head.len() >= 7 {
            &wt.head[..7]
        } else {
            &wt.head
        };
        println!("    HEAD:   {}", short_head.dimmed());

        // Print spec info if available
        if let Some(ref id) = spec_id {
            println!("    Spec:   {}", id.yellow());
            if let Some(ref info) = spec_info {
                println!("    Title:  {}", info.title);
                println!("    Status: {}", format_status(&info.status));
            } else {
                println!("    Status: {} (spec not found)", "unknown".red());
            }
        }

        // Print size and age
        let size = dir_size(&wt.path);
        let age = path_age_secs(&wt.path);
        println!(
            "    Size:   {}  Age: {}",
            format_bytes(size).yellow(),
            format_age_secs(age).dimmed()
        );

        // Print prunable reason if available
        if let Some(ref reason) = wt.prunable_reason {
            println!("    Reason: {}", reason.red());
        }

        println!();
    }

    // Summary
    let prunable_count = chant_worktrees.iter().filter(|wt| wt.prunable).count();
    if prunable_count > 0 {
        println!(
            "{} {} prunable worktree{} (run {} to clean up)",
            "⚠".yellow(),
            prunable_count,
            if prunable_count == 1 { "" } else { "s" },
            "chant cleanup".cyan()
        );
    }

    let total_size: u64 = chant_worktrees.iter().map(|wt| dir_size(&wt.path)).sum();
    println!(
        "Total disk usage: {}",
        format_bytes(total_size).bold().yellow()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_worktree_list_basic() {
        let output = r#"worktree /Users/test/project
HEAD abc123def456
branch refs/heads/main

worktree /tmp/chant-2026-01-27-001-abc
HEAD def456abc789
branch refs/heads/chant/2026-01-27-001-abc
"#;
        let entries = parse_worktree_list(output);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].path, PathBuf::from("/Users/test/project"));
        assert_eq!(entries[0].head, "abc123def456");
        assert_eq!(entries[0].branch, Some("main".to_string()));
        assert!(!entries[0].prunable);

        assert_eq!(
            entries[1].path,
            PathBuf::from("/tmp/chant-2026-01-27-001-abc")
        );
        assert_eq!(entries[1].head, "def456abc789");
        assert_eq!(
            entries[1].branch,
            Some("chant/2026-01-27-001-abc".to_string())
        );
    }

    #[test]
    fn test_parse_worktree_list_prunable() {
        let output = r#"worktree /tmp/chant-2026-01-27-001-abc
HEAD def456abc789
branch refs/heads/chant/2026-01-27-001-abc
prunable gitdir file points to non-existent location
"#;
        let entries = parse_worktree_list(output);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].prunable);
        assert_eq!(
            entries[0].prunable_reason,
            Some("gitdir file points to non-existent location".to_string())
        );
    }

    #[test]
    fn test_extract_spec_id_from_branch() {
        let entry = GitWorktreeEntry {
            path: PathBuf::from("/tmp/chant-2026-01-27-001-abc"),
            head: "abc123".to_string(),
            branch: Some("chant/2026-01-27-001-abc".to_string()),
            prunable: false,
            prunable_reason: None,
        };
        assert_eq!(
            extract_spec_id(&entry, "chant/"),
            Some("2026-01-27-001-abc".to_string())
        );
    }

    #[test]
    fn test_extract_spec_id_from_branch_custom_prefix() {
        let entry = GitWorktreeEntry {
            path: PathBuf::from("/tmp/chant-frontend-2026-01-27-001-abc"),
            head: "abc123".to_string(),
            branch: Some("chant/frontend/2026-01-27-001-abc".to_string()),
            prunable: false,
            prunable_reason: None,
        };
        assert_eq!(
            extract_spec_id(&entry, "chant/frontend/"),
            Some("2026-01-27-001-abc".to_string())
        );
    }

    #[test]
    fn test_extract_spec_id_from_path() {
        let entry = GitWorktreeEntry {
            path: PathBuf::from("/tmp/chant-2026-01-27-001-abc"),
            head: "abc123".to_string(),
            branch: None,
            prunable: false,
            prunable_reason: None,
        };
        assert_eq!(
            extract_spec_id(&entry, "chant/"),
            Some("2026-01-27-001-abc".to_string())
        );
    }

    #[test]
    fn test_extract_spec_id_no_match() {
        let entry = GitWorktreeEntry {
            path: PathBuf::from("/Users/test/project"),
            head: "abc123".to_string(),
            branch: Some("main".to_string()),
            prunable: false,
            prunable_reason: None,
        };
        assert_eq!(extract_spec_id(&entry, "chant/"), None);
    }
}
