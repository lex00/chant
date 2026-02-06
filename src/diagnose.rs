//! Diagnostic utilities for checking spec execution status.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: guides/recovery.md, reference/cli.md
//! - ignore: false

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::paths::SPECS_DIR;
use crate::spec::{Spec, SpecStatus};
use crate::worktree::get_active_worktree;

/// A single diagnostic check result.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub details: Option<String>,
}

impl CheckResult {
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            details: None,
        }
    }

    pub fn pass_with_details(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            details: Some(details.into()),
        }
    }

    pub fn fail(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            details: Some(details.into()),
        }
    }
}

/// Full diagnostic report for a spec.
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub spec_id: String,
    pub status: SpecStatus,
    pub checks: Vec<CheckResult>,
    pub diagnosis: String,
    pub suggestion: Option<String>,
    pub location: String,
}

impl DiagnosticReport {
    /// Returns true if all diagnostic checks passed. Used in tests.
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns the list of failed checks. Used in tests.
    pub fn failed_checks(&self) -> Vec<&CheckResult> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }
}

/// Check if a spec file exists and is valid.
fn check_spec_file(spec_file: &Path) -> CheckResult {
    if !spec_file.exists() {
        return CheckResult::fail("Spec file", "Does not exist");
    }

    match fs::read_to_string(spec_file) {
        Ok(content) => {
            // Try to parse as a spec to validate YAML
            match Spec::parse(
                spec_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown"),
                &content,
            ) {
                Ok(_) => CheckResult::pass_with_details("Spec file", "Valid YAML"),
                Err(e) => CheckResult::fail("Spec file", format!("Invalid YAML: {}", e)),
            }
        }
        Err(e) => CheckResult::fail("Spec file", format!("Cannot read: {}", e)),
    }
}

/// Check if a log file exists and get its age.
fn check_log_file(spec_id: &str, base_path: &Path) -> CheckResult {
    let log_file = base_path
        .join(".chant/logs")
        .join(format!("{}.log", spec_id));

    let location_hint = if base_path.to_string_lossy().contains("/tmp/chant-") {
        "worktree"
    } else {
        "main"
    };

    if !log_file.exists() {
        return CheckResult::fail(
            "Log file",
            format!("Does not exist (checked in {})", location_hint),
        );
    }

    match fs::metadata(&log_file) {
        Ok(metadata) => {
            let size = metadata.len();

            let age_str = match metadata.modified() {
                Ok(modified_time) => match modified_time.elapsed() {
                    Ok(elapsed) => {
                        let secs: u64 = elapsed.as_secs();
                        if secs < 60 {
                            "just now".to_string()
                        } else if secs < 3600 {
                            format!("{} minutes ago", secs / 60)
                        } else if secs < 86400 {
                            format!("{} hours ago", secs / 3600)
                        } else {
                            format!("{} days ago", secs / 86400)
                        }
                    }
                    Err(_) => "unknown age".to_string(),
                },
                Err(_) => "unknown age".to_string(),
            };

            CheckResult::pass_with_details(
                "Log file",
                format!(
                    "Exists ({} bytes), last modified: {} (checked in {})",
                    size, age_str, location_hint
                ),
            )
        }
        Err(e) => CheckResult::fail("Log file", format!("Cannot read metadata: {}", e)),
    }
}

/// Check if there's a lock file.
fn check_lock_file(spec_id: &str, base_path: &Path) -> CheckResult {
    let lock_file = base_path
        .join(".chant/.locks")
        .join(format!("{}.lock", spec_id));

    let location_hint = if base_path.to_string_lossy().contains("/tmp/chant-") {
        "worktree"
    } else {
        "main"
    };

    if lock_file.exists() {
        CheckResult::pass_with_details(
            "Lock file",
            format!(
                "Present (spec may be running) (checked in {})",
                location_hint
            ),
        )
    } else {
        CheckResult::pass_with_details(
            "Lock file",
            format!("Not present (checked in {})", location_hint),
        )
    }
}

/// Check if a git commit exists for this spec.
fn check_git_commit(spec_id: &str) -> CheckResult {
    let output = Command::new("git")
        .args(["log", "--grep", &format!("chant({})", spec_id), "--oneline"])
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let commit_line = stdout.lines().next();

            if let Some(line) = commit_line {
                // Extract commit hash (first 7 chars)
                let commit_hash = line.split_whitespace().next().unwrap_or("unknown");
                CheckResult::pass_with_details("Git commit", format!("Found: {}", commit_hash))
            } else {
                CheckResult::fail("Git commit", "No matching commit found")
            }
        }
        Err(_) => CheckResult::fail("Git commit", "Cannot run git log"),
    }
}

/// Check if acceptance criteria are all satisfied.
fn check_acceptance_criteria(spec: &Spec) -> CheckResult {
    let unchecked = spec.count_unchecked_checkboxes();

    if unchecked == 0 {
        CheckResult::pass_with_details("Acceptance criteria", "All satisfied")
    } else {
        CheckResult::fail(
            "Acceptance criteria",
            format!("{} unchecked items remaining", unchecked),
        )
    }
}

/// Check for common status mismatches.
fn check_status_consistency(spec: &Spec, commit_exists: bool, unchecked: usize) -> CheckResult {
    match spec.frontmatter.status {
        SpecStatus::InProgress => {
            if commit_exists {
                CheckResult::fail(
                    "Status consistency",
                    "Status is in_progress but commit exists (should be completed?)",
                )
            } else {
                CheckResult::pass("Status consistency")
            }
        }
        SpecStatus::Paused => CheckResult::pass_with_details(
            "Status consistency",
            "Paused (work stopped mid-execution)",
        ),
        SpecStatus::Completed => {
            if !commit_exists {
                CheckResult::fail(
                    "Status consistency",
                    "Status is completed but no commit found",
                )
            } else if unchecked > 0 {
                CheckResult::fail(
                    "Status consistency",
                    "Status is completed but acceptance criteria unchecked",
                )
            } else {
                CheckResult::pass("Status consistency")
            }
        }
        SpecStatus::Pending => {
            if commit_exists {
                CheckResult::fail("Status consistency", "Status is pending but commit exists")
            } else {
                CheckResult::pass("Status consistency")
            }
        }
        SpecStatus::Failed => {
            CheckResult::pass_with_details("Status consistency", "Marked as failed")
        }
        SpecStatus::NeedsAttention => {
            CheckResult::pass_with_details("Status consistency", "Marked as needs attention")
        }
        SpecStatus::Ready => {
            if commit_exists {
                CheckResult::fail("Status consistency", "Status is ready but commit exists")
            } else {
                CheckResult::pass("Status consistency")
            }
        }
        SpecStatus::Blocked => {
            CheckResult::pass_with_details("Status consistency", "Blocked by unmet dependencies")
        }
        SpecStatus::Cancelled => CheckResult::pass_with_details(
            "Status consistency",
            "Marked as cancelled (preserved but excluded from list and work)",
        ),
    }
}

/// Run all diagnostic checks on a spec.
pub fn diagnose_spec(spec_id: &str) -> Result<DiagnosticReport> {
    // Check if spec has an active worktree
    let (base_path, location) = if let Some(worktree) = get_active_worktree(spec_id) {
        (
            worktree.clone(),
            format!("worktree: {}", worktree.display()),
        )
    } else {
        (PathBuf::from("."), "main repository".to_string())
    };

    let specs_dir = base_path.join(SPECS_DIR);
    let spec_file = specs_dir.join(format!("{}.md", spec_id));

    // Load the spec
    let spec = Spec::load(&spec_file).context("Failed to load spec")?;

    // Run checks
    let mut checks = Vec::new();

    // 1. Spec file check
    checks.push(check_spec_file(&spec_file));

    // 2. Log file check
    checks.push(check_log_file(spec_id, &base_path));

    // 3. Lock file check
    checks.push(check_lock_file(spec_id, &base_path));

    // 4. Git commit check
    let commit_result = check_git_commit(spec_id);
    let commit_exists = commit_result.passed;
    checks.push(commit_result);

    // 5. Acceptance criteria check
    let criteria_result = check_acceptance_criteria(&spec);
    let unchecked = spec.count_unchecked_checkboxes();
    checks.push(criteria_result);

    // 6. Status consistency check
    checks.push(check_status_consistency(&spec, commit_exists, unchecked));

    // Determine diagnosis and suggestion
    let (diagnosis, suggestion) = diagnose_issues(&spec, &checks);

    Ok(DiagnosticReport {
        spec_id: spec_id.to_string(),
        status: spec.frontmatter.status.clone(),
        checks,
        diagnosis,
        suggestion,
        location,
    })
}

/// Generate a diagnosis message and suggestion based on check results.
fn diagnose_issues(spec: &Spec, checks: &[CheckResult]) -> (String, Option<String>) {
    let failed = checks.iter().filter(|c| !c.passed).collect::<Vec<_>>();

    if failed.is_empty() {
        return ("All checks passed. Spec appears healthy.".to_string(), None);
    }

    // Look for specific patterns
    if spec.frontmatter.status == SpecStatus::InProgress {
        // Check for the common "stuck in progress" pattern
        let has_commit = checks.iter().any(|c| c.name == "Git commit" && c.passed);
        let all_criteria_met = checks
            .iter()
            .find(|c| c.name == "Acceptance criteria")
            .map(|c| c.passed)
            .unwrap_or(false);

        if has_commit && all_criteria_met {
            return (
                "Spec appears complete but wasn't finalized.".to_string(),
                Some(format!(
                    "Run `just chant work {} --finalize` to fix.",
                    &spec.id
                )),
            );
        }

        if has_commit && !all_criteria_met {
            return (
                "Spec has a commit but acceptance criteria not all satisfied.".to_string(),
                Some("Complete the unchecked acceptance criteria and re-run the spec.".to_string()),
            );
        }

        return (
            "Spec is in progress but has issues.".to_string(),
            Some(format!("Check the log: `just chant log {}`", &spec.id)),
        );
    }

    // Generic diagnosis for failed checks
    let check_names = failed
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    (
        format!("Spec has issues: {}", check_names),
        Some(format!("Run `just chant log {}` for details", &spec.id)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_creation() {
        let pass = CheckResult::pass("Test");
        assert!(pass.passed);
        assert_eq!(pass.name, "Test");
        assert_eq!(pass.details, None);

        let fail = CheckResult::fail("Test", "Failed");
        assert!(!fail.passed);
        assert_eq!(fail.details, Some("Failed".to_string()));
    }

    #[test]
    fn test_spec_file_check_missing() {
        let result = check_spec_file(Path::new("nonexistent.md"));
        assert!(!result.passed);
        assert!(result.details.is_some());
    }

    #[test]
    fn test_diagnostic_report_all_passed() {
        let report = DiagnosticReport {
            spec_id: "test".to_string(),
            status: SpecStatus::Completed,
            checks: vec![CheckResult::pass("Check 1"), CheckResult::pass("Check 2")],
            diagnosis: "All good".to_string(),
            suggestion: None,
            location: "main repository".to_string(),
        };

        assert!(report.all_passed());
        assert_eq!(report.failed_checks().len(), 0);
    }

    #[test]
    fn test_diagnostic_report_some_failed() {
        let report = DiagnosticReport {
            spec_id: "test".to_string(),
            status: SpecStatus::InProgress,
            checks: vec![
                CheckResult::pass("Check 1"),
                CheckResult::fail("Check 2", "Bad"),
            ],
            diagnosis: "Some issues".to_string(),
            suggestion: None,
            location: "main repository".to_string(),
        };

        assert!(!report.all_passed());
        assert_eq!(report.failed_checks().len(), 1);
    }

    #[test]
    fn test_status_consistency_in_progress_with_commit() {
        use crate::spec::SpecFrontmatter;

        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: None,
            body: String::new(),
        };

        let result = check_status_consistency(&spec, true, 0);
        assert!(!result.passed);
        assert!(result
            .details
            .unwrap()
            .contains("in_progress but commit exists"));
    }

    #[test]
    fn test_status_consistency_completed_no_commit() {
        use crate::spec::SpecFrontmatter;

        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: None,
            body: String::new(),
        };

        let result = check_status_consistency(&spec, false, 0);
        assert!(!result.passed);
        assert!(result
            .details
            .unwrap()
            .contains("completed but no commit found"));
    }

    #[test]
    fn test_acceptance_criteria_all_satisfied() {
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: Default::default(),
            title: None,
            body: "## Acceptance Criteria\n\n- [x] Item 1\n- [x] Item 2".to_string(),
        };

        let result = check_acceptance_criteria(&spec);
        assert!(result.passed);
    }

    #[test]
    fn test_acceptance_criteria_some_unchecked() {
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: Default::default(),
            title: None,
            body: "## Acceptance Criteria\n\n- [ ] Item 1\n- [x] Item 2".to_string(),
        };

        let result = check_acceptance_criteria(&spec);
        assert!(!result.passed);
        assert!(result
            .details
            .unwrap()
            .contains("1 unchecked items remaining"));
    }
}
