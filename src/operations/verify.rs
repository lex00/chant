//! Spec verification operation.
//!
//! Canonical implementation for verifying specs against acceptance criteria.

use anyhow::{Context, Result};
use chrono::Utc;

use crate::config::Config;
use crate::prompt;
use crate::spec::Spec;
use std::path::PathBuf;

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

/// Options for verification
#[derive(Debug, Clone, Default)]
pub struct VerifyOptions {
    /// Custom prompt to use for verification
    pub custom_prompt: Option<String>,
}

/// Extract acceptance criteria section from spec body
pub fn extract_acceptance_criteria(spec: &Spec) -> Option<String> {
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

/// Parse verification response from the agent
pub fn parse_verification_response(
    response: &str,
) -> Result<(VerificationStatus, Vec<CriterionResult>)> {
    let mut criteria_results = Vec::new();
    let mut overall_status = VerificationStatus::Pass;
    let mut in_verification_section = false;
    let mut in_code_fence = false;

    for line in response.lines() {
        let trimmed = line.trim();

        // Track code fence boundaries
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        // Look for the Verification Summary section (can be anywhere, including inside code fences)
        if trimmed.contains("Verification Summary") {
            in_verification_section = true;
            continue;
        }

        // Stop at next section (marked by ## heading), but only if we're not in a code fence
        if in_verification_section
            && !in_code_fence
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

/// Update spec frontmatter with verification results
pub fn update_spec_with_verification_results(
    spec: &Spec,
    overall_status: VerificationStatus,
    criteria: &[CriterionResult],
) -> Result<()> {
    // Get current UTC timestamp in ISO 8601 format
    let now = Utc::now();
    let timestamp = now.to_rfc3339();

    // Determine verification status string
    let verification_status = match overall_status {
        VerificationStatus::Pass => "passed".to_string(),
        VerificationStatus::Fail => "failed".to_string(),
        VerificationStatus::Mixed => "partial".to_string(),
    };

    // Extract failure reasons from FAIL criteria
    let verification_failures: Option<Vec<String>> = {
        let failures: Vec<String> = criteria
            .iter()
            .filter(|c| c.status == "FAIL")
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

    // Create updated spec with new frontmatter
    let mut updated_spec = spec.clone();
    updated_spec.frontmatter.last_verified = Some(timestamp);
    updated_spec.frontmatter.verification_status = Some(verification_status);
    updated_spec.frontmatter.verification_failures = verification_failures;

    // Save the updated spec to disk
    let spec_path = PathBuf::from(format!(".chant/specs/{}.md", spec.id));
    updated_spec.save(&spec_path).context(format!(
        "Failed to write updated spec to {}",
        spec_path.display()
    ))?;

    Ok(())
}

/// Verify a spec by invoking the agent.
///
/// This is the canonical verification logic:
/// - Checks for acceptance criteria
/// - Assembles verification prompt
/// - Invokes the agent
/// - Parses verification response
/// - Updates spec frontmatter
///
/// Returns (overall_status, criteria_results).
pub fn verify_spec(
    spec: &Spec,
    config: &Config,
    options: VerifyOptions,
    invoke_agent_fn: impl Fn(&str, &Spec, &str, &Config) -> Result<String>,
) -> Result<(VerificationStatus, Vec<CriterionResult>)> {
    // Check if spec has acceptance criteria
    let ac_section = extract_acceptance_criteria(spec);
    if ac_section.is_none() {
        anyhow::bail!("No acceptance criteria found in spec");
    }

    // Determine which prompt to use
    let prompt_name = options.custom_prompt.as_deref().unwrap_or("verify");
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
    let response = invoke_agent_fn(&message, spec, "verify", config)
        .context("Failed to invoke agent for verification")?;

    // Parse the response
    let (overall_status, criteria) = parse_verification_response(&response)?;

    // Update spec frontmatter with verification results
    update_spec_with_verification_results(spec, overall_status, &criteria)?;

    Ok((overall_status, criteria))
}
