//! Verify command for checking specs against their acceptance criteria.
//!
//! This module provides functionality to verify that specs meet their acceptance
//! criteria, with options for filtering by ID or labels.

use anyhow::{Context, Result};
use chant::config::Config;
use chant::prompt;
use chant::spec::{load_all_specs, resolve_spec, Spec, SpecStatus};
use colored::Colorize;
use std::path::PathBuf;

use crate::cmd::agent;

/// Verification status for a spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    Pass,
    Fail,
    Mixed,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail => write!(f, "FAIL"),
            Self::Mixed => write!(f, "MIXED"),
        }
    }
}

/// Result of verifying an individual criterion
#[derive(Debug, Clone)]
pub struct CriterionResult {
    pub criterion: String,
    pub status: String, // "PASS", "FAIL", or "SKIP"
    pub note: Option<String>,
}

/// Parse verification response from the agent
fn parse_verification_response(
    response: &str,
) -> Result<(VerificationStatus, Vec<CriterionResult>)> {
    let mut criteria_results = Vec::new();
    let mut overall_status = VerificationStatus::Pass;
    let mut in_verification_section = false;

    for line in response.lines() {
        let trimmed = line.trim();

        // Look for the Verification Summary section
        if trimmed.contains("Verification Summary") {
            in_verification_section = true;
            continue;
        }

        // Stop at next section (marked by ## heading)
        if in_verification_section
            && trimmed.starts_with("##")
            && !trimmed.contains("Verification Summary")
        {
            break;
        }

        if !in_verification_section {
            continue;
        }

        // Parse criterion lines: "- [x] Criterion: STATUS — optional note"
        if trimmed.starts_with("- [") {
            // Extract the status and criterion
            if let Some(rest) = trimmed.strip_prefix("- [") {
                if let Some(criterion_part) = rest.split_once(']') {
                    let criterion_line = criterion_part.1.trim();

                    // Parse criterion and status
                    if let Some(colon_pos) = criterion_line.find(':') {
                        let criterion_text = criterion_line[..colon_pos].trim().to_string();
                        let status_part = criterion_line[colon_pos + 1..].trim();

                        // Extract status and optional note
                        let (status, note) = if let Some(dash_idx) = status_part.find(" — ") {
                            let status_text = status_part[..dash_idx].trim().to_uppercase();
                            let note_text = status_part[dash_idx + " — ".len()..].trim();
                            (status_text, Some(note_text.to_string()))
                        } else {
                            (status_part.to_uppercase(), None)
                        };

                        // Validate status
                        if !["PASS", "FAIL", "SKIP"].iter().any(|s| status.contains(s)) {
                            continue;
                        }

                        // Update overall status based on individual results
                        if status.contains("FAIL") {
                            overall_status = VerificationStatus::Fail;
                        } else if status.contains("SKIP")
                            && overall_status == VerificationStatus::Pass
                        {
                            overall_status = VerificationStatus::Mixed;
                        }

                        criteria_results.push(CriterionResult {
                            criterion: criterion_text,
                            status: if status.contains("PASS") {
                                "PASS".to_string()
                            } else if status.contains("FAIL") {
                                "FAIL".to_string()
                            } else {
                                "SKIP".to_string()
                            },
                            note,
                        });
                    }
                }
            }
        }

        // Also look for "Overall status: X" line
        if trimmed.starts_with("Overall status:") {
            if let Some(status_text) = trimmed.split(':').nth(1) {
                let status_upper = status_text.trim().to_uppercase();
                overall_status = if status_upper.contains("FAIL") {
                    VerificationStatus::Fail
                } else if status_upper.contains("PASS") {
                    VerificationStatus::Pass
                } else {
                    VerificationStatus::Mixed
                };
            }
        }
    }

    // If we didn't find any criteria, it's an error
    if criteria_results.is_empty() {
        anyhow::bail!("Could not parse verification response from agent. Expected format with 'Verification Summary' section.");
    }

    Ok((overall_status, criteria_results))
}

/// Extract acceptance criteria section from spec body
fn extract_acceptance_criteria(spec: &Spec) -> Option<String> {
    let acceptance_criteria_marker = "## Acceptance Criteria";
    let mut in_ac_section = false;
    let mut ac_content = String::new();
    let mut in_code_fence = false;

    for line in spec.body.lines() {
        let trimmed = line.trim_start();

        // Track code fences
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
        }

        // Look for AC section heading outside code fences
        if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
            in_ac_section = true;
            continue;
        }

        // Stop at next heading
        if in_ac_section
            && trimmed.starts_with("## ")
            && !trimmed.starts_with(acceptance_criteria_marker)
        {
            break;
        }

        if in_ac_section {
            ac_content.push_str(line);
            ac_content.push('\n');
        }
    }

    if ac_content.is_empty() {
        None
    } else {
        Some(ac_content)
    }
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

    // Verify each spec
    let mut any_failed = false;

    for spec in specs_to_verify {
        verify_spec(&spec, &config, prompt)?;

        // Check verification status in frontmatter
        if let Some(status) = &spec.frontmatter.verification_status {
            if status == "FAIL" {
                any_failed = true;
            }
        }
    }

    // Exit with appropriate code if requested
    if exit_code && any_failed {
        std::process::exit(1);
    }

    Ok(())
}

/// Verify a single spec by invoking the agent
fn verify_spec(spec: &Spec, config: &Config, custom_prompt: Option<&str>) -> Result<()> {
    println!("\n{} {}", "Verifying:".cyan(), spec.id);

    // Check if spec has acceptance criteria
    let ac_section = extract_acceptance_criteria(spec);
    if ac_section.is_none() {
        println!(
            "  {} No acceptance criteria found in spec. Skipping verification.",
            "⚠".yellow()
        );
        return Ok(());
    }

    // Determine which prompt to use
    let prompt_name = custom_prompt.unwrap_or("verify");
    let prompt_path = PathBuf::from(format!(".chant/prompts/{}.md", prompt_name));

    if !prompt_path.exists() {
        anyhow::bail!(
            "Prompt file not found: {}. Run `chant init` to create default prompts.",
            prompt_path.display()
        );
    }

    // Assemble the prompt with spec context
    let message = prompt::assemble(spec, &prompt_path, config)
        .context("Failed to assemble verification prompt")?;

    // Invoke the agent
    println!("  {} Invoking agent...", "→".cyan());

    let response = match agent::invoke_agent(&message, spec, "verify", config) {
        Ok(output) => output,
        Err(e) => {
            println!("  {} Agent invocation failed: {}", "✗".red(), e);
            return Err(e).context("Failed to invoke agent for verification");
        }
    };

    // Parse the response
    match parse_verification_response(&response) {
        Ok((overall_status, criteria)) => {
            // Display results
            println!("  {} Verification Result: {}", "→".cyan(), overall_status);

            for (i, criterion) in criteria.iter().enumerate() {
                let status_icon = match criterion.status.as_str() {
                    "PASS" => "✓".green(),
                    "FAIL" => "✗".red(),
                    "SKIP" => "⊘".yellow(),
                    _ => "?".bright_yellow(),
                };

                print!("    {} {}: {}", status_icon, i + 1, criterion.criterion);
                if let Some(note) = &criterion.note {
                    print!(" — {}", note);
                }
                println!();
            }

            println!(
                "  {} Overall: {}",
                "→".cyan(),
                match overall_status {
                    VerificationStatus::Pass => "✓ PASS".green(),
                    VerificationStatus::Fail => "✗ FAIL".red(),
                    VerificationStatus::Mixed => "⊘ MIXED".yellow(),
                }
            );

            Ok(())
        }
        Err(e) => {
            println!(
                "  {} Failed to parse verification response: {}",
                "✗".red(),
                e
            );
            Err(e).context("Could not parse agent response")
        }
    }
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
        fs::create_dir_all(&specs_dir).unwrap();

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
        fs::create_dir_all(&specs_dir).unwrap();

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
        fs::create_dir_all(&specs_dir).unwrap();

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
        fs::create_dir_all(&specs_dir).unwrap();

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
        let labels = vec!["test".to_string()];

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
        fs::create_dir_all(&specs_dir).unwrap();

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
        fs::create_dir_all(&specs_dir).unwrap();

        // Load from empty directory
        let all_specs = load_all_specs(specs_dir).unwrap();
        assert_eq!(all_specs.len(), 0);
    }

    #[test]
    fn test_filter_label_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();
        fs::create_dir_all(&specs_dir).unwrap();

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
        let requested_labels = vec!["foo".to_string()];

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
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: "# Test\n\n## Acceptance Criteria\n\n- [ ] Criterion 1\n- [ ] Criterion 2\n\n## Edge Cases\n\nSome content".to_string(),
        };

        let ac = extract_acceptance_criteria(&spec).unwrap();
        assert!(ac.contains("Criterion 1"));
        assert!(ac.contains("Criterion 2"));
        assert!(!ac.contains("Edge Cases"));
    }

    #[test]
    fn test_extract_acceptance_criteria_none() {
        let spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: "# Test\n\nNo acceptance criteria here.".to_string(),
        };

        let ac = extract_acceptance_criteria(&spec);
        assert!(ac.is_none());
    }

    #[test]
    fn test_parse_verification_response_all_pass() {
        let response = r#"## Verification Summary

- [x] Criterion 1: PASS
- [x] Criterion 2: PASS

Overall status: PASS"#;

        let (status, criteria) = parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Pass);
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0].status, "PASS");
        assert_eq!(criteria[1].status, "PASS");
    }

    #[test]
    fn test_parse_verification_response_with_fail() {
        let response = r#"## Verification Summary

- [x] Criterion 1: PASS
- [ ] Criterion 2: FAIL
- [x] Criterion 3: PASS

Overall status: FAIL"#;

        let (status, criteria) = parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Fail);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0].status, "PASS");
        assert_eq!(criteria[1].status, "FAIL");
        assert_eq!(criteria[2].status, "PASS");
    }

    #[test]
    fn test_parse_verification_response_with_skip() {
        let response = r#"## Verification Summary

- [x] Criterion 1: PASS
- [x] Criterion 2: SKIP — Unable to verify manually
- [x] Criterion 3: PASS

Overall status: MIXED"#;

        let (status, criteria) = parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Mixed);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[1].status, "SKIP");
        assert_eq!(
            criteria[1].note,
            Some("Unable to verify manually".to_string())
        );
    }

    #[test]
    fn test_parse_verification_response_malformed() {
        let response = "Some random output without verification summary";
        let result = parse_verification_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_verification_status_display() {
        assert_eq!(VerificationStatus::Pass.to_string(), "PASS");
        assert_eq!(VerificationStatus::Fail.to_string(), "FAIL");
        assert_eq!(VerificationStatus::Mixed.to_string(), "MIXED");
    }
}
