//! Spec archiving functionality - moves completed specs to archive directory

use anyhow::{Context, Result};
use chrono::Local;
use colored::Colorize;
use std::path::PathBuf;

use chant::paths::ARCHIVE_DIR;
use chant::prompt;
use chant::spec::{self, Spec, SpecStatus};

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

/// Check if we're in a git repository
pub(crate) fn is_git_repo() -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if there are uncommitted changes other than the archived spec files.
///
/// This function checks git status to see if there are any staged or unstaged changes
/// that aren't related to the archived specs. This is used to prevent auto-committing
/// when the working directory has unrelated changes.
///
/// # Arguments
/// * `archived_spec_ids` - List of spec IDs that were just archived
///
/// # Returns
/// * `Ok(true)` if there are other uncommitted changes
/// * `Ok(false)` if only archived spec files have changes (or no changes)
/// * `Err(_)` if git status command fails
pub(crate) fn has_other_uncommitted_changes(archived_spec_ids: &[String]) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    if !output.status.success() {
        anyhow::bail!("git status failed");
    }

    let status_output = String::from_utf8_lossy(&output.stdout);

    // Parse status output to check for changes
    for line in status_output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Extract file path from status line (format: "XY filename")
        if line.len() < 3 {
            continue;
        }
        let file_path = &line[3..];

        // Check if this file is one of the archived spec files
        let is_archived_spec = archived_spec_ids.iter().any(|spec_id| {
            // Check for source location (.chant/specs/{spec_id}.md)
            let src_pattern = format!(".chant/specs/{}.md", spec_id);
            // Check for destination location (.chant/archive/YYYY-MM-DD/{spec_id}.md)
            let date_part = &spec_id[..10]; // First 10 chars: YYYY-MM-DD
            let dst_pattern = format!(".chant/archive/{}/{}.md", date_part, spec_id);

            file_path == src_pattern || file_path == dst_pattern
        });

        // If we find a change that's not an archived spec file, return true
        if !is_archived_spec {
            return Ok(true);
        }
    }

    // All changes are related to archived spec files
    Ok(false)
}

/// Get commits associated with a spec by searching git log
pub(crate) fn get_spec_commits(spec_id: &str) -> Result<Vec<String>> {
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
pub(crate) fn get_commit_file_stats(
    commit: &str,
) -> Result<std::collections::HashMap<String, (i64, i64)>> {
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
pub(crate) fn verify_target_files(spec: &Spec) -> Result<TargetFilesVerification> {
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
pub(crate) fn format_target_files_warning(
    spec_id: &str,
    verification: &TargetFilesVerification,
) -> String {
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

/// Print condensed warnings when there are many repeated warning types
pub(crate) fn print_condensed_warnings(
    specs_with_missing_changes: &[(spec::Spec, TargetFilesVerification)],
) {
    use std::collections::HashMap;

    // Group specs by warning type signature
    // Warning type is determined by whether there are predicted files and actual files
    let mut warning_groups: HashMap<String, Vec<&str>> = HashMap::new();

    for (spec, verification) in specs_with_missing_changes {
        // Create a warning type key based on the pattern
        let has_predicted = !verification.files_without_changes.is_empty()
            || !verification.files_with_changes.is_empty();
        let has_actual = !verification.actual_files_changed.is_empty();

        let warning_type = match (has_predicted, has_actual) {
            (true, true) => "target_files_mismatch",
            (true, false) => "target_files_no_changes",
            (false, true) => "no_target_files_with_changes",
            (false, false) => "no_prediction_no_changes",
        };

        warning_groups
            .entry(warning_type.to_string())
            .or_default()
            .push(spec.id.as_str());
    }

    // Print condensed or individual warnings based on count
    for (warning_type, spec_ids) in &warning_groups {
        if spec_ids.len() > 3 {
            // Condense when count > 3
            let message = match warning_type.as_str() {
                "target_files_mismatch" => {
                    "Prediction mismatch (target_files) - implementation is fine"
                }
                "target_files_no_changes" => "target_files specified but no changes found",
                "no_target_files_with_changes" => "Changes made but no target_files specified",
                "no_prediction_no_changes" => "No prediction and no changes",
                _ => "Unknown warning type",
            };
            println!("{} {}: {} specs", "⚠".yellow(), message, spec_ids.len());
        } else {
            // Show individual warnings when count ≤ 3
            for spec_id in spec_ids {
                if let Some((spec, verification)) = specs_with_missing_changes
                    .iter()
                    .find(|(s, _)| s.id == *spec_id)
                {
                    println!("{}", format_target_files_warning(&spec.id, verification));
                    if !verification.commits.is_empty() {
                        println!("Commits found: {}\n", verification.commits.join(", "));
                    } else {
                        println!("No commits found with pattern 'chant({}):'.\n", spec.id);
                    }
                }
            }
        }
    }
}

/// Move a file using git mv, falling back to fs::rename if not in a git repo or if no_stage is true
pub(crate) fn move_spec_file(src: &PathBuf, dst: &PathBuf, no_stage: bool) -> Result<()> {
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
        let now = Local::now();

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

            // Condense repeated warnings (count > 3)
            print_condensed_warnings(&specs_with_missing_changes);

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
    let mut archived_spec_ids = Vec::new();
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
        archived_spec_ids.push(spec.id.clone());
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
        // Check if there are other uncommitted changes besides the archived spec files
        if has_other_uncommitted_changes(&archived_spec_ids)? {
            eprintln!(
                "{} Working directory has other uncommitted changes. Skipping auto-commit.",
                "⚠".yellow()
            );
            eprintln!("  Please commit or stash other changes, then run 'git commit' manually.");
        } else {
            // Create commit message with all archived spec IDs
            let commit_msg = if archived_spec_ids.len() == 1 {
                format!("chant: Archive {}", archived_spec_ids[0])
            } else {
                let spec_list = archived_spec_ids.join(", ");
                format!("chant: Archive {}", spec_list)
            };

            let status = std::process::Command::new("git")
                .args(["commit", "-m", &commit_msg])
                .status()
                .context("Failed to create commit")?;

            if !status.success() {
                anyhow::bail!("git commit failed");
            }
            println!("{} Created commit: {}", "✓".green(), commit_msg);
        }
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
#[cfg(test)]
mod tests {
    use super::*;
    use chant::spec::{Spec, SpecFrontmatter, SpecStatus};
    use tempfile::TempDir;

    #[test]
    fn test_ensure_logs_dir_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Logs dir shouldn't exist yet
        assert!(!base_path.join("logs").exists());

        // Call ensure_logs_dir_at
        crate::cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

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
        crate::cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

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
}
