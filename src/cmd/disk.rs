//! Disk usage command to show size of chant artifacts.

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use chant::paths::{ARCHIVE_DIR, LOCKS_DIR, LOGS_DIR, PROMPTS_DIR, SPECS_DIR, STORE_DIR};

/// Calculate disk usage of a directory using du command
fn dir_size(path: &PathBuf) -> u64 {
    if !path.exists() {
        return 0;
    }

    // Try with -sb first (Linux), fall back to -s (macOS) with block conversion
    let output = Command::new("du").arg("-s").arg(path).output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(|blocks| blocks * 512) // Convert 512-byte blocks to bytes
                .unwrap_or(0)
        }
        Err(_) => 0,
    }
}

/// Format bytes into human-readable size (B, KB, MB, GB)
fn format_size(bytes: u64) -> String {
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

/// Calculate size of worktrees in /tmp (chant-<spec-id> directories and files)
fn worktree_size() -> u64 {
    let tmp_path = PathBuf::from("/tmp");
    if !tmp_path.exists() {
        return 0;
    }

    std::fs::read_dir(&tmp_path)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("chant-"))
                .unwrap_or(false)
        })
        .map(|entry| {
            let path = entry.path();
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    dir_size(&path)
                } else {
                    metadata.len()
                }
            } else {
                0
            }
        })
        .sum()
}

/// Count worktrees in /tmp
fn count_worktrees() -> usize {
    let tmp_path = PathBuf::from("/tmp");
    if !tmp_path.exists() {
        return 0;
    }

    std::fs::read_dir(&tmp_path)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("chant-"))
                .unwrap_or(false)
        })
        .count()
}

/// Show disk usage of chant artifacts
pub fn cmd_disk() -> Result<()> {
    // Ensure chant is initialized
    let _specs_dir = crate::cmd::ensure_initialized()?;

    // Calculate sizes for each directory
    let specs_size = dir_size(&PathBuf::from(SPECS_DIR));
    let prompts_size = dir_size(&PathBuf::from(PROMPTS_DIR));
    let logs_size = dir_size(&PathBuf::from(LOGS_DIR));
    let archive_size = dir_size(&PathBuf::from(ARCHIVE_DIR));
    let locks_size = dir_size(&PathBuf::from(LOCKS_DIR));
    let store_size = dir_size(&PathBuf::from(STORE_DIR));
    let worktree_usage = worktree_size();
    let worktree_count = count_worktrees();

    // Calculate totals
    let chant_dir_total =
        specs_size + prompts_size + logs_size + archive_size + locks_size + store_size;
    let grand_total = chant_dir_total + worktree_usage;

    // Display results
    println!("{}", "Chant Disk Usage".bold().cyan());
    println!();

    println!("{}", ".chant/ directory breakdown:".bold());
    println!("  {:<20} {}", "Specs:", format_size(specs_size).yellow());
    println!(
        "  {:<20} {}",
        "Prompts:",
        format_size(prompts_size).yellow()
    );
    println!("  {:<20} {}", "Logs:", format_size(logs_size).yellow());
    println!(
        "  {:<20} {}",
        "Archive:",
        format_size(archive_size).yellow()
    );
    println!("  {:<20} {}", "Locks:", format_size(locks_size).yellow());
    println!("  {:<20} {}", "Store:", format_size(store_size).yellow());
    println!(
        "  {:<20} {}",
        ".chant/ Total:",
        format_size(chant_dir_total).cyan().bold()
    );
    println!();

    println!("{}", "Worktrees in /tmp:".bold());
    println!(
        "  {:<20} {} worktrees",
        "Count:",
        worktree_count.to_string().yellow()
    );
    println!(
        "  {:<20} {}",
        "Total Size:",
        format_size(worktree_usage).yellow()
    );
    println!();

    println!("{}", "Grand Total:".bold());
    println!("  {}", format_size(grand_total).green().bold());

    Ok(())
}
