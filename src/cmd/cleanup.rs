//! Cleanup command to remove orphan worktrees and stale artifacts.
//!
//! Provides functionality to identify and remove chant worktrees from /tmp
//! and other stale artifacts, with options for dry-run and confirmation.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

/// Information about a worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Directory name (e.g., "chant-2026-01-25-01g-v2e")
    pub name: String,
    /// Full path to the worktree
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Age in seconds
    pub age_secs: u64,
    /// Whether it's a valid git worktree
    pub is_valid: bool,
}

impl WorktreeInfo {
    /// Format the size as human-readable string
    fn format_size(&self) -> String {
        format_bytes(self.size)
    }

    /// Format the age as human-readable string
    fn format_age(&self) -> String {
        format_age_secs(self.age_secs)
    }
}

/// Format bytes into human-readable size (B, KB, MB, GB, TB)
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

/// Format age in seconds as human-readable string
pub fn format_age_secs(secs: u64) -> String {
    const SECONDS_PER_MINUTE: u64 = 60;
    const SECONDS_PER_HOUR: u64 = 60 * 60;
    const SECONDS_PER_DAY: u64 = 60 * 60 * 24;

    if secs < SECONDS_PER_MINUTE {
        format!("{} seconds", secs)
    } else if secs < SECONDS_PER_HOUR {
        let minutes = secs / SECONDS_PER_MINUTE;
        format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
    } else if secs < SECONDS_PER_DAY {
        let hours = secs / SECONDS_PER_HOUR;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else {
        let days = secs / SECONDS_PER_DAY;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    }
}

/// Calculate the size of a file or directory recursively
fn dir_size(path: &PathBuf) -> u64 {
    if !path.exists() {
        return 0;
    }

    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        Ok(metadata) if metadata.is_dir() => fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| dir_size(&entry.path()))
            .sum(),
        _ => 0,
    }
}

/// Get the age of a directory in seconds
fn dir_age_secs(path: &PathBuf) -> u64 {
    match fs::metadata(path) {
        Ok(metadata) => match metadata.modified() {
            Ok(modified_time) => match SystemTime::now().duration_since(modified_time) {
                Ok(duration) => duration.as_secs(),
                Err(_) => 0,
            },
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Information about a prunable worktree from git
#[derive(Debug, Clone)]
struct PrunableWorktree {
    path: PathBuf,
    /// Some prunable worktrees may not exist on disk
    exists: bool,
}

/// Get prunable worktrees from git worktree list
fn get_prunable_worktrees() -> Result<Vec<PrunableWorktree>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        // If git command fails, return empty vec
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut prunable_worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut is_prunable = false;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            // Save previous if it was prunable
            if is_prunable {
                if let Some(path) = current_path.take() {
                    // Only include chant worktrees
                    if path
                        .file_name()
                        .map(|n| n.to_string_lossy().starts_with("chant-"))
                        .unwrap_or(false)
                    {
                        let exists = path.exists();
                        prunable_worktrees.push(PrunableWorktree { path, exists });
                    }
                }
            }
            // Start new entry
            let path = line.strip_prefix("worktree ").unwrap_or("");
            current_path = Some(PathBuf::from(path));
            is_prunable = false;
        } else if line.starts_with("prunable") {
            is_prunable = true;
        }
    }

    // Don't forget the last entry
    if is_prunable {
        if let Some(path) = current_path {
            // Only include chant worktrees
            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with("chant-"))
                .unwrap_or(false)
            {
                let exists = path.exists();
                prunable_worktrees.push(PrunableWorktree { path, exists });
            }
        }
    }

    Ok(prunable_worktrees)
}

/// Check if a path is a valid git worktree (not prunable and exists)
fn is_valid_worktree(path: &std::path::Path, prunable_worktrees: &[PrunableWorktree]) -> bool {
    // If git marks it as prunable, it's not valid
    if prunable_worktrees.iter().any(|pw| pw.path == path) {
        return false;
    }

    // A valid worktree should have a .git file or directory
    let git_path = path.join(".git");
    git_path.exists()
}

/// Scan /tmp for orphan chant worktrees
pub fn find_orphan_worktrees() -> Result<Vec<WorktreeInfo>> {
    // Get prunable worktrees from git
    let prunable_worktrees = get_prunable_worktrees()?;

    let mut worktrees = Vec::new();
    let mut seen_paths = HashSet::new();

    // First, add all prunable worktrees from git (even if they don't exist on disk)
    for prunable in &prunable_worktrees {
        let name = prunable
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| prunable.path.display().to_string());

        let size = if prunable.exists {
            dir_size(&prunable.path)
        } else {
            0
        };

        let age_secs = if prunable.exists {
            dir_age_secs(&prunable.path)
        } else {
            0
        };

        worktrees.push(WorktreeInfo {
            name,
            path: prunable.path.clone(),
            size,
            age_secs,
            is_valid: false, // Prunable worktrees are not valid
        });

        seen_paths.insert(prunable.path.clone());
    }

    // Then scan /tmp for any other chant directories
    let tmp_path = PathBuf::from("/tmp");
    if tmp_path.exists() {
        for entry in fs::read_dir(&tmp_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            // Only consider directories starting with "chant-"
            if !name_str.starts_with("chant-") {
                continue;
            }

            let path = entry.path();

            // Skip if we already added this from prunable list
            if seen_paths.contains(&path) {
                continue;
            }

            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }

            let is_valid = is_valid_worktree(&path, &prunable_worktrees);

            // We want to show both valid and orphan worktrees, but filter to orphans if requested
            let size = dir_size(&path);
            let age_secs = dir_age_secs(&path);

            worktrees.push(WorktreeInfo {
                name: name_str.to_string(),
                path,
                size,
                age_secs,
                is_valid,
            });
        }
    }

    // Sort by age (oldest first)
    worktrees.sort_by_key(|wt| std::cmp::Reverse(wt.age_secs));

    Ok(worktrees)
}

/// Filter to only orphan worktrees
pub fn filter_orphans(worktrees: &[WorktreeInfo]) -> Vec<&WorktreeInfo> {
    worktrees.iter().filter(|wt| !wt.is_valid).collect()
}

/// Run git worktree prune to clean up stale entries
fn run_git_prune() -> Result<()> {
    let output = Command::new("git").args(["worktree", "prune"]).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree prune failed: {}", stderr);
    }

    Ok(())
}

/// Confirm cleanup with the user
fn confirm_cleanup() -> Result<bool> {
    use std::io::{self, Write};

    print!("? Clean up these worktrees? [Y/n] ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    Ok(response.is_empty() || response == "y" || response == "yes")
}

/// Cleanup only worktrees (used with --worktrees flag)
fn cleanup_worktrees_only(orphans: &[&WorktreeInfo], dry_run: bool, yes: bool) -> Result<()> {
    if orphans.is_empty() {
        println!("{}", "No orphan worktrees found.".green());
        return Ok(());
    }

    // Display what would be cleaned
    println!("{}", "Scanning for orphan worktrees...".cyan());
    println!();
    println!(
        "Found {} orphan worktree{}:",
        orphans.len().to_string().yellow(),
        if orphans.len() == 1 { "" } else { "s" }
    );

    for worktree in orphans {
        println!(
            "  {} ({}, {})",
            worktree.name.bold(),
            worktree.format_size().yellow(),
            worktree.format_age().dimmed()
        );
    }

    let total: u64 = orphans.iter().map(|wt| wt.size).sum();
    println!();
    println!("Total: {}", format_bytes(total).bold().yellow());
    println!();

    // Handle dry-run
    if dry_run {
        println!("{}", "(dry-run - no changes made)".dimmed());
        return Ok(());
    }

    // Confirm unless --yes is specified
    if !yes && !confirm_cleanup()? {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    }

    println!();

    // Remove each worktree
    let mut removed = 0;
    for worktree in orphans {
        print!("Removing {}... ", worktree.name);
        std::io::stdout().flush()?;

        let mut success = false;

        // Try to remove the git worktree entry first
        let _ = Command::new("git")
            .args(["worktree", "remove", &worktree.path.to_string_lossy()])
            .output();

        // Force remove the directory if it exists
        if worktree.path.exists() {
            match fs::remove_dir_all(&worktree.path) {
                Ok(_) => {
                    success = true;
                }
                Err(e) => {
                    println!("{}", "failed".red());
                    eprintln!("Error removing {}: {}", worktree.name, e);
                    continue;
                }
            }
        } else {
            // Directory doesn't exist, but we still count it as cleaned
            // since git worktree prune will remove the stale reference
            success = true;
        }

        if success {
            println!("{}", "done".green());
            removed += 1;
        }
    }

    // Run git worktree prune
    println!("Running git worktree prune... ");
    match run_git_prune() {
        Ok(_) => println!("{}", "done".green()),
        Err(e) => {
            eprintln!("Warning: git worktree prune failed: {}", e);
        }
    }

    println!();
    println!(
        "Cleaned up {} worktree{}, {} reclaimed",
        removed.to_string().green(),
        if removed == 1 { "" } else { "s" },
        format_bytes(total).green()
    );

    Ok(())
}

/// Execute the cleanup command
pub fn cmd_cleanup(dry_run: bool, yes: bool, worktrees_only: bool) -> Result<()> {
    // Find all worktrees
    let all_worktrees = find_orphan_worktrees()?;

    // Filter to only orphans
    let orphans = filter_orphans(&all_worktrees);

    // If --worktrees flag is specified, skip other cleanup operations
    if worktrees_only {
        return cleanup_worktrees_only(&orphans, dry_run, yes);
    }

    if orphans.is_empty() {
        println!("{}", "No orphan worktrees found.".green());
        return Ok(());
    }

    // Display what would be cleaned
    println!("{}", "Scanning for orphan worktrees...".cyan());
    println!();
    println!(
        "Found {} orphan worktree{}:",
        orphans.len().to_string().yellow(),
        if orphans.len() == 1 { "" } else { "s" }
    );

    for worktree in &orphans {
        println!(
            "  {} ({}, {})",
            worktree.name.bold(),
            worktree.format_size().yellow(),
            worktree.format_age().dimmed()
        );
    }

    let total: u64 = orphans.iter().map(|wt| wt.size).sum();
    println!();
    println!("Total: {}", format_bytes(total).bold().yellow());
    println!();

    // Handle dry-run
    if dry_run {
        println!("{}", "(dry-run - no changes made)".dimmed());
        return Ok(());
    }

    // Confirm unless --yes is specified
    if !yes && !confirm_cleanup()? {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    }

    println!();

    // Remove each worktree
    let mut removed = 0;
    for worktree in &orphans {
        print!("Removing {}... ", worktree.name);
        std::io::stdout().flush()?;

        let mut success = false;

        // Try to remove the git worktree entry first
        let _ = Command::new("git")
            .args(["worktree", "remove", &worktree.path.to_string_lossy()])
            .output();

        // Force remove the directory if it exists
        if worktree.path.exists() {
            match fs::remove_dir_all(&worktree.path) {
                Ok(_) => {
                    success = true;
                }
                Err(e) => {
                    println!("{}", "failed".red());
                    eprintln!("Error removing {}: {}", worktree.name, e);
                    continue;
                }
            }
        } else {
            // Directory doesn't exist, but we still count it as cleaned
            // since git worktree prune will remove the stale reference
            success = true;
        }

        if success {
            println!("{}", "done".green());
            removed += 1;
        }
    }

    // Run git worktree prune
    println!("Running git worktree prune... ");
    match run_git_prune() {
        Ok(_) => println!("{}", "done".green()),
        Err(e) => {
            eprintln!("Warning: git worktree prune failed: {}", e);
        }
    }

    println!();
    println!(
        "Cleaned up {} worktree{}, {} reclaimed",
        removed.to_string().green(),
        if removed == 1 { "" } else { "s" },
        format_bytes(total).green()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(2560), "2.5 KB");
    }

    #[test]
    fn test_format_age_secs() {
        assert_eq!(format_age_secs(30), "30 seconds");
        assert_eq!(format_age_secs(60), "1 minute");
        assert_eq!(format_age_secs(120), "2 minutes");
        assert_eq!(format_age_secs(3600), "1 hour");
        assert_eq!(format_age_secs(7200), "2 hours");
        assert_eq!(format_age_secs(86400), "1 day");
        assert_eq!(format_age_secs(172800), "2 days");
    }

    #[test]
    fn test_filter_orphans() {
        let worktrees = vec![
            WorktreeInfo {
                name: "chant-valid".to_string(),
                path: PathBuf::from("/tmp/chant-valid"),
                size: 1024,
                age_secs: 3600,
                is_valid: true,
            },
            WorktreeInfo {
                name: "chant-orphan".to_string(),
                path: PathBuf::from("/tmp/chant-orphan"),
                size: 2048,
                age_secs: 7200,
                is_valid: false,
            },
        ];

        let orphans = filter_orphans(&worktrees);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].name, "chant-orphan");
    }

    #[test]
    fn test_get_prunable_worktrees_parsing() {
        // Simulate git worktree list output with prunable worktree
        let output = r#"worktree /Users/test/project
HEAD abc123def456
branch refs/heads/main

worktree /tmp/chant-2026-01-27-001-abc
HEAD def456abc789
branch refs/heads/chant/2026-01-27-001-abc
prunable gitdir file points to non-existent location
"#;

        let mut prunable_worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut is_prunable = false;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                if is_prunable {
                    if let Some(path) = current_path.take() {
                        if path
                            .file_name()
                            .map(|n| n.to_string_lossy().starts_with("chant-"))
                            .unwrap_or(false)
                        {
                            prunable_worktrees.push(path);
                        }
                    }
                }
                let path = line.strip_prefix("worktree ").unwrap_or("");
                current_path = Some(PathBuf::from(path));
                is_prunable = false;
            } else if line.starts_with("prunable") {
                is_prunable = true;
            }
        }

        if is_prunable {
            if let Some(path) = current_path {
                if path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with("chant-"))
                    .unwrap_or(false)
                {
                    prunable_worktrees.push(path);
                }
            }
        }

        assert_eq!(prunable_worktrees.len(), 1);
        assert_eq!(
            prunable_worktrees[0],
            PathBuf::from("/tmp/chant-2026-01-27-001-abc")
        );
    }
}
