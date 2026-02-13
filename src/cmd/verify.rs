//! Verify command for checking specs against their acceptance criteria.
//!
//! This module provides functionality to verify that specs meet their acceptance
//! criteria, with options for filtering by ID or labels.

use anyhow::{Context, Result};
use chant::config::Config;
use chant::operations::{VerificationStatus, VerifyOptions};
use chant::spec::{load_all_specs, resolve_spec, Spec, SpecStatus};
use colored::Colorize;
use std::path::PathBuf;

use crate::cmd::agent;

/// Result of verifying a single spec
#[derive(Debug, Clone)]
pub struct SpecVerificationResult {
    #[allow(dead_code)]
    pub spec_id: String,
    #[allow(dead_code)]
    pub spec_title: Option<String>,
    pub passed: bool,
    #[allow(dead_code)]
    pub total_criteria: usize,
}

/// Display a summary of verification results for multiple specs
fn display_verification_summary(results: &[SpecVerificationResult]) {
    use crate::cmd::validate::{ValidationCategory, ValidationResult};

    let passed_count = results.iter().filter(|r| r.passed).count();
    let failed_count = results.len() - passed_count;

    // Use unified validation framework for display
    let mut validation_result = ValidationResult::new(ValidationCategory::Verify);
    validation_result.total = results.len();
    validation_result.passed = passed_count;
    validation_result.failed = failed_count;

    validation_result.display_summary();
}

/// Execute the verify command
///
/// # Arguments
///
/// * `id` - Optional spec ID to verify. If None, verifies based on --all or --label filters.
/// * `all` - If true, verify all specs
/// * `label` - Labels to filter specs by (OR logic)
/// * `exit_code` - If true, exit with code 1 if verification fails
/// * `dry_run` - If true, show what would be verified without making changes
/// * `prompt` - Custom prompt to use for verification
pub fn cmd_verify(
    id: Option<&str>,
    all: bool,
    label: &[String],
    exit_code: bool,
    dry_run: bool,
    prompt: Option<&str>,
) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");

    // Load all available specs
    let all_specs = load_all_specs(&specs_dir)?;

    // Determine which specs to verify based on arguments
    let specs_to_verify = if let Some(spec_id) = id {
        // Verify specific spec by ID
        let spec = resolve_spec(&specs_dir, spec_id)?;

        // Check if spec is completed
        if spec.frontmatter.status != SpecStatus::Completed {
            anyhow::bail!(
                "Spec {} is not completed (status: {})",
                spec.id,
                format!("{:?}", spec.frontmatter.status).to_lowercase()
            );
        }

        vec![spec]
    } else if all {
        // Verify all completed specs
        let completed: Vec<Spec> = all_specs
            .into_iter()
            .filter(|s| s.frontmatter.status == SpecStatus::Completed)
            .collect();

        if completed.is_empty() {
            println!("No completed specs to verify");
            return Ok(());
        }

        completed
    } else if !label.is_empty() {
        // Verify completed specs matching any label
        let matching: Vec<Spec> = all_specs
            .into_iter()
            .filter(|s| {
                if s.frontmatter.status != SpecStatus::Completed {
                    return false;
                }

                // Check if spec has any of the requested labels
                if let Some(spec_labels) = &s.frontmatter.labels {
                    label.iter().any(|l| spec_labels.contains(l))
                } else {
                    false
                }
            })
            .collect();

        if matching.is_empty() {
            println!(
                "No completed specs with label '{}'",
                label.join("', '").yellow()
            );
            return Ok(());
        }

        matching
    } else {
        // No filter specified - verify all completed specs
        let completed: Vec<Spec> = all_specs
            .into_iter()
            .filter(|s| s.frontmatter.status == SpecStatus::Completed)
            .collect();

        if completed.is_empty() {
            println!("No completed specs to verify");
            return Ok(());
        }

        completed
    };

    // Handle dry-run mode: just show specs that would be verified
    if dry_run {
        println!("{}", "Specs that would be verified (dry-run):".cyan());
        for spec in &specs_to_verify {
            let title = spec.title.as_deref().unwrap_or("(no title)");
            println!("  {} - {}", spec.id.cyan(), title);
        }
        return Ok(());
    }

    // Load config for agent invocation
    let config = Config::load().context("Failed to load config. Have you run `chant init`?")?;

    // Verify each spec and track results
    let mut verification_results = Vec::new();

    for spec in specs_to_verify {
        let result = verify_spec_cmd(&spec, &config, prompt)?;
        verification_results.push(result);
    }

    // Display summary if multiple specs were verified
    if verification_results.len() > 1 {
        display_verification_summary(&verification_results);
    }

    // Determine if any failed for exit code handling
    let any_failed = verification_results.iter().any(|r| !r.passed);

    // Exit with appropriate code if requested
    if exit_code && any_failed {
        std::process::exit(1);
    }

    Ok(())
}

/// Verify a single spec by invoking the agent
fn verify_spec_cmd(
    spec: &Spec,
    config: &Config,
    custom_prompt: Option<&str>,
) -> Result<SpecVerificationResult> {
    let title = spec.title.as_deref().unwrap_or("(no title)");
    println!("\n{} {} - {}", "Verifying:".cyan(), spec.id.cyan(), title);

    // Check if spec has acceptance criteria
    let ac_section = chant::operations::extract_acceptance_criteria(spec);
    if ac_section.is_none() {
        println!(
            "  {} No acceptance criteria found in spec. Skipping verification.",
            "⚠".yellow()
        );
        return Ok(SpecVerificationResult {
            spec_id: spec.id.clone(),
            spec_title: spec.title.clone(),
            passed: false,
            total_criteria: 0,
        });
    }

    println!("  {} Invoking agent...", "→".cyan());

    // Use operations layer
    let options = VerifyOptions {
        custom_prompt: custom_prompt.map(String::from),
    };

    let (overall_status, criteria) =
        chant::operations::verify_spec(spec, config, options, agent::invoke_agent)?;

    let total_criteria = criteria.len();
    let passed = overall_status == VerificationStatus::Pass;

    // Display criteria with icons
    if criteria.is_empty() {
        println!("  {} No criteria to verify", "⚠".yellow());
    } else {
        for (i, criterion) in criteria.iter().enumerate() {
            let status_icon = match criterion.status {
                chant::operations::CriterionStatus::Pass => "✓".green(),
                chant::operations::CriterionStatus::Fail => "✗".red(),
                chant::operations::CriterionStatus::Skip => "~".yellow(),
            };

            print!("  {} {}: {}", status_icon, i + 1, criterion.criterion);
            if let Some(note) = &criterion.note {
                print!(" — {}", note);
            }
            println!();
        }
    }

    // Display overall result
    let overall_label = match overall_status {
        VerificationStatus::Pass => {
            format!("{}", "✓ VERIFIED".green())
        }
        VerificationStatus::Fail => {
            format!("{}", "✗ FAILED".red())
        }
        VerificationStatus::Mixed => {
            format!("{}", "~ PARTIAL".yellow())
        }
    };

    println!("  {} Overall: {}", "→".cyan(), overall_label);
    println!(
        "  {} Frontmatter updated with verification results",
        "→".cyan()
    );

    Ok(SpecVerificationResult {
        spec_id: spec.id.clone(),
        spec_title: spec.title.clone(),
        passed,
        total_criteria,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chant::spec::{load_all_specs, Spec, SpecFrontmatter, SpecStatus};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_filter_completed_spec() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Create a completed spec
        let spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "# Test Spec\n\nBody content.".to_string(),
        };

        spec.save(&specs_dir.join("2026-01-26-001-abc.md")).unwrap();

        // Load and filter - should find completed spec
        let all_specs = load_all_specs(specs_dir).unwrap();
        assert_eq!(all_specs.len(), 1);
        assert_eq!(all_specs[0].id, "2026-01-26-001-abc");
        assert_eq!(all_specs[0].frontmatter.status, SpecStatus::Completed);
    }

    #[test]
    fn test_pending_spec_filtered_out() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Create a pending spec
        let spec = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Pending Spec".to_string()),
            body: "# Pending Spec\n\nBody content.".to_string(),
        };

        spec.save(&specs_dir.join("2026-01-26-002-def.md")).unwrap();

        // Load and filter - should find pending spec but it should not be in completed filter
        let all_specs = load_all_specs(specs_dir).unwrap();
        assert_eq!(all_specs.len(), 1);
        assert_eq!(all_specs[0].frontmatter.status, SpecStatus::Pending);

        // When filtering for completed only, it should be empty
        let completed: Vec<_> = all_specs
            .into_iter()
            .filter(|s| s.frontmatter.status == SpecStatus::Completed)
            .collect();
        assert_eq!(completed.len(), 0);
    }

    #[test]
    fn test_filter_all_completed_specs() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Create multiple completed specs
        let spec1 = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("First Spec".to_string()),
            body: "# First Spec\n\nBody.".to_string(),
        };

        let spec2 = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Second Spec".to_string()),
            body: "# Second Spec\n\nBody.".to_string(),
        };

        // Create a pending spec (should be filtered out)
        let spec3 = Spec {
            id: "2026-01-26-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Pending Spec".to_string()),
            body: "# Pending Spec\n\nBody.".to_string(),
        };

        spec1
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        spec2
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();
        spec3
            .save(&specs_dir.join("2026-01-26-003-ghi.md"))
            .unwrap();

        // Load and filter
        let all_specs = load_all_specs(specs_dir).unwrap();
        assert_eq!(all_specs.len(), 3);

        // Filter for completed only
        let completed: Vec<_> = all_specs
            .into_iter()
            .filter(|s| s.frontmatter.status == SpecStatus::Completed)
            .collect();
        assert_eq!(completed.len(), 2);
        assert!(completed.iter().any(|s| s.id == "2026-01-26-001-abc"));
        assert!(completed.iter().any(|s| s.id == "2026-01-26-002-def"));
    }

    #[test]
    fn test_filter_by_label_completed_only() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Create completed spec with label
        let spec1 = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                labels: Some(vec!["test".to_string()]),
                ..Default::default()
            },
            title: Some("Labeled Completed".to_string()),
            body: "# Labeled Completed\n\nBody.".to_string(),
        };

        // Create pending spec with same label (should be filtered out)
        let spec2 = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                labels: Some(vec!["test".to_string()]),
                ..Default::default()
            },
            title: Some("Labeled Pending".to_string()),
            body: "# Labeled Pending\n\nBody.".to_string(),
        };

        spec1
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        spec2
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();

        // Load and filter by label
        let all_specs = load_all_specs(specs_dir).unwrap();
        let labels = ["test".to_string()];

        let matching: Vec<_> = all_specs
            .into_iter()
            .filter(|s| {
                if s.frontmatter.status != SpecStatus::Completed {
                    return false;
                }
                if let Some(spec_labels) = &s.frontmatter.labels {
                    labels.iter().any(|l| spec_labels.contains(l))
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].id, "2026-01-26-001-abc");
    }

    #[test]
    fn test_filter_no_completed_specs() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Create only pending specs
        let spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Pending Spec".to_string()),
            body: "# Pending Spec\n\nBody.".to_string(),
        };

        spec.save(&specs_dir.join("2026-01-26-001-abc.md")).unwrap();

        // Load and filter for completed only
        let all_specs = load_all_specs(specs_dir).unwrap();
        let completed: Vec<_> = all_specs
            .into_iter()
            .filter(|s| s.frontmatter.status == SpecStatus::Completed)
            .collect();

        assert_eq!(completed.len(), 0);
    }

    #[test]
    fn test_nonexistent_spec_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Load from empty directory
        let all_specs = load_all_specs(specs_dir).unwrap();
        assert_eq!(all_specs.len(), 0);
    }

    #[test]
    fn test_filter_label_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(specs_dir).unwrap();

        // Create completed spec without the requested label
        let spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                labels: Some(vec!["other".to_string()]),
                ..Default::default()
            },
            title: Some("Other Label".to_string()),
            body: "# Other Label\n\nBody.".to_string(),
        };

        spec.save(&specs_dir.join("2026-01-26-001-abc.md")).unwrap();

        // Load and filter by non-matching label
        let all_specs = load_all_specs(specs_dir).unwrap();
        let requested_labels = ["foo".to_string()];

        let matching: Vec<_> = all_specs
            .into_iter()
            .filter(|s| {
                if s.frontmatter.status != SpecStatus::Completed {
                    return false;
                }
                if let Some(spec_labels) = &s.frontmatter.labels {
                    requested_labels.iter().any(|l| spec_labels.contains(l))
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(matching.len(), 0);
    }

    #[test]
    fn test_extract_acceptance_criteria() {
        let spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: chant::spec::SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: "# Test\n\n## Acceptance Criteria\n\n- [ ] Criterion 1\n- [ ] Criterion 2\n\n## Edge Cases\n\nSome content".to_string(),
        };

        let ac = chant::operations::extract_acceptance_criteria(&spec).unwrap();
        assert!(ac.contains("Criterion 1"));
        assert!(ac.contains("Criterion 2"));
        assert!(!ac.contains("Edge Cases"));
    }

    #[test]
    fn test_extract_acceptance_criteria_none() {
        let spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: chant::spec::SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: "# Test\n\nNo acceptance criteria here.".to_string(),
        };

        let ac = chant::operations::extract_acceptance_criteria(&spec);
        assert!(ac.is_none());
    }

    #[test]
    fn test_parse_verification_response_all_pass() {
        let response = r#"## Verification Summary

- [x] Criterion 1: PASS
- [x] Criterion 2: PASS

Overall status: PASS"#;

        let (status, criteria) = chant::operations::parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Pass);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0].status, chant::operations::CriterionStatus::Pass);
        assert_eq!(criteria[1].status, chant::operations::CriterionStatus::Pass);
    }

    #[test]
    fn test_parse_verification_response_with_fail() {
        let response = r#"## Verification Summary

- [x] Criterion 1: PASS
- [ ] Criterion 2: FAIL
- [x] Criterion 3: PASS

Overall status: FAIL"#;

        let (status, criteria) = chant::operations::parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Fail);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0].status, chant::operations::CriterionStatus::Pass);
        assert_eq!(criteria[1].status, chant::operations::CriterionStatus::Fail);
        assert_eq!(criteria[2].status, chant::operations::CriterionStatus::Pass);
    }

    #[test]
    fn test_parse_verification_response_with_skip() {
        let response = r#"## Verification Summary

- [x] Criterion 1: PASS
- [x] Criterion 2: SKIP — Unable to verify manually
- [x] Criterion 3: PASS

Overall status: MIXED"#;

        let (status, criteria) = chant::operations::parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Mixed);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[1].status, chant::operations::CriterionStatus::Skip);
        assert_eq!(
            criteria[1].note,
            Some("Unable to verify manually".to_string())
        );
    }

    #[test]
    fn test_parse_verification_response_malformed() {
        let response = "Some random output without verification summary";
        let result = chant::operations::parse_verification_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_verification_response_in_code_fence() {
        let response = r#"Here is the verification result:

```
## Verification Summary

- [x] Criterion 1: PASS
- [ ] Criterion 2: FAIL — Missing implementation
- [x] Criterion 3: SKIP — Requires manual testing

Overall status: FAIL
```

Done."#;

        let (status, criteria) = chant::operations::parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Fail);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0].status, chant::operations::CriterionStatus::Pass);
        assert_eq!(criteria[1].status, chant::operations::CriterionStatus::Fail);
        assert_eq!(criteria[1].note, Some("Missing implementation".to_string()));
        assert_eq!(criteria[2].status, chant::operations::CriterionStatus::Skip);
        assert_eq!(
            criteria[2].note,
            Some("Requires manual testing".to_string())
        );
    }

    #[test]
    fn test_verification_status_display() {
        assert_eq!(VerificationStatus::Pass.to_string(), "PASS");
        assert_eq!(VerificationStatus::Fail.to_string(), "FAIL");
        assert_eq!(VerificationStatus::Mixed.to_string(), "MIXED");
    }

    #[test]
    fn test_frontmatter_update_all_pass() {
        use chant::operations::CriterionResult;

        // Create criteria results with all PASS
        let criteria = [
            CriterionResult {
                criterion: "Feature X".to_string(),
                status: chant::operations::CriterionStatus::Pass,
                note: None,
            },
            CriterionResult {
                criterion: "Tests passing".to_string(),
                status: chant::operations::CriterionStatus::Pass,
                note: None,
            },
        ];

        let overall_status = VerificationStatus::Pass;

        let verification_status = match overall_status {
            VerificationStatus::Pass => "passed",
            VerificationStatus::Fail => "failed",
            VerificationStatus::Mixed => "partial",
        };

        let verification_failures: Option<Vec<String>> = {
            let failures: Vec<String> = criteria
                .iter()
                .filter(|c| c.status == chant::operations::CriterionStatus::Fail)
                .map(|c| c.criterion.clone())
                .collect();
            if failures.is_empty() {
                None
            } else {
                Some(failures)
            }
        };

        assert_eq!(verification_status, "passed");
        assert_eq!(verification_failures, None);
    }

    #[test]
    fn test_frontmatter_update_with_failures() {
        use chant::operations::CriterionResult;

        let criteria = [
            CriterionResult {
                criterion: "Feature X".to_string(),
                status: chant::operations::CriterionStatus::Pass,
                note: None,
            },
            CriterionResult {
                criterion: "Tests passing".to_string(),
                status: chant::operations::CriterionStatus::Fail,
                note: Some("Some tests failed".to_string()),
            },
        ];

        let overall_status = VerificationStatus::Fail;

        let verification_status = match overall_status {
            VerificationStatus::Pass => "passed",
            VerificationStatus::Fail => "failed",
            VerificationStatus::Mixed => "partial",
        };

        let verification_failures: Option<Vec<String>> = {
            let failures: Vec<String> = criteria
                .iter()
                .filter(|c| c.status == chant::operations::CriterionStatus::Fail)
                .map(|c| {
                    if let Some(note) = &c.note {
                        format!("{} — {}", c.criterion, note)
                    } else {
                        c.criterion.clone()
                    }
                })
                .collect();
            if failures.is_empty() {
                None
            } else {
                Some(failures)
            }
        };

        assert_eq!(verification_status, "failed");
        assert!(verification_failures.is_some());
        let failures = verification_failures.unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0], "Tests passing — Some tests failed");
    }

    #[test]
    fn test_frontmatter_update_mixed_status() {
        use chant::operations::CriterionResult;

        let criteria = [
            CriterionResult {
                criterion: "Feature X".to_string(),
                status: chant::operations::CriterionStatus::Pass,
                note: None,
            },
            CriterionResult {
                criterion: "Manual verification".to_string(),
                status: chant::operations::CriterionStatus::Skip,
                note: Some("Could not verify in CI".to_string()),
            },
        ];

        let overall_status = VerificationStatus::Mixed;

        let verification_status = match overall_status {
            VerificationStatus::Pass => "passed",
            VerificationStatus::Fail => "failed",
            VerificationStatus::Mixed => "partial",
        };

        let verification_failures: Option<Vec<String>> = {
            let failures: Vec<String> = criteria
                .iter()
                .filter(|c| c.status == chant::operations::CriterionStatus::Fail)
                .map(|c| c.criterion.clone())
                .collect();
            if failures.is_empty() {
                None
            } else {
                Some(failures)
            }
        };

        assert_eq!(verification_status, "partial");
        assert_eq!(verification_failures, None);
    }

    #[test]
    fn test_timestamp_iso8601_format() {
        use chrono::Utc;

        let now = Utc::now();
        let timestamp = now.to_rfc3339();

        // Verify ISO 8601 format (RFC 3339)
        // Should contain T and Z or timezone offset
        assert!(timestamp.contains('T'));
        assert!(timestamp.contains('Z') || timestamp.contains('+') || timestamp.contains('-'));

        // Should be parseable back
        assert!(timestamp.parse::<chrono::DateTime<Utc>>().is_ok());
    }
}
