//! Spec finalization logic.
//!
//! Handles marking specs as complete, updating frontmatter with commits,
//! timestamps, and model information.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use chant::config::Config;
use chant::spec::{self, Spec, SpecStatus};

use crate::cmd::commits::{get_commits_for_spec, get_commits_for_spec_allow_no_commits};
use crate::cmd::model::get_model_name;

/// Maximum characters to store in agent output section
pub const MAX_AGENT_OUTPUT_CHARS: usize = 5000;

/// Finalize a spec after successful completion
/// Sets status, commits, completed_at, and model
/// This function is idempotent and can be called multiple times safely
pub fn finalize_spec(
    spec: &mut Spec,
    spec_path: &Path,
    config: &Config,
    all_specs: &[Spec],
    allow_no_commits: bool,
) -> Result<()> {
    // Check if this is a driver spec with incomplete members
    let incomplete_members = spec::get_incomplete_members(&spec.id, all_specs);
    if !incomplete_members.is_empty() {
        anyhow::bail!(
            "Cannot complete driver spec '{}' while {} member spec(s) are incomplete: {}",
            spec.id,
            incomplete_members.len(),
            incomplete_members.join(", ")
        );
    }

    // Get the commits for this spec
    let commits = if allow_no_commits {
        get_commits_for_spec_allow_no_commits(&spec.id)?
    } else {
        get_commits_for_spec(&spec.id)?
    };

    // Update spec to completed
    spec.frontmatter.status = SpecStatus::Completed;
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );
    spec.frontmatter.model = get_model_name(Some(config));

    // Save the spec - this must not fail silently
    spec.save(spec_path)
        .context("Failed to save finalized spec")?;

    // Validation 1: Verify that status was actually changed to Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after finalization"
    );

    // Validation 2: Verify that completed_at timestamp is set and in valid ISO format
    let completed_at = spec
        .frontmatter
        .completed_at
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("completed_at timestamp was not set"))?;

    // Validate ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
    if !completed_at.ends_with('Z') {
        anyhow::bail!(
            "completed_at must end with 'Z' (UTC format), got: {}",
            completed_at
        );
    }
    if !completed_at.contains('T') {
        anyhow::bail!(
            "completed_at must contain 'T' separator (ISO format), got: {}",
            completed_at
        );
    }

    // Validation 3: Verify that spec was actually saved (reload and check)
    let saved_spec =
        Spec::load(spec_path).context("Failed to reload spec from disk to verify persistence")?;

    anyhow::ensure!(
        saved_spec.frontmatter.status == SpecStatus::Completed,
        "Persisted spec status is not Completed - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.completed_at.is_some(),
        "Persisted spec is missing completed_at - save may have failed"
    );

    // Model may be None if no model was detected, but commits should match memory
    match (&spec.frontmatter.commits, &saved_spec.frontmatter.commits) {
        (Some(mem_commits), Some(saved_commits)) => {
            anyhow::ensure!(
                mem_commits == saved_commits,
                "Persisted commits don't match memory - save may have failed"
            );
        }
        (None, None) => {
            // Both None is correct
        }
        _ => {
            anyhow::bail!("Persisted commits don't match memory - save may have failed");
        }
    }

    Ok(())
}

/// Re-finalize a spec that was left in an incomplete state
/// This can be called on in_progress or completed specs to update commits and timestamp
/// Idempotent: safe to call multiple times
pub fn re_finalize_spec(
    spec: &mut Spec,
    spec_path: &Path,
    config: &Config,
    allow_no_commits: bool,
) -> Result<()> {
    // Re-finalization only works on specs that have been started (in_progress or completed)
    // A pending spec has never been started and should use normal work flow
    match spec.frontmatter.status {
        SpecStatus::InProgress | SpecStatus::Completed => {
            // These are valid for re-finalization
        }
        _ => {
            anyhow::bail!(
                "Cannot re-finalize spec '{}' with status '{:?}'. Must be in_progress or completed.",
                spec.id,
                spec.frontmatter.status
            );
        }
    }

    // Get the commits for this spec (may have new ones since last finalization)
    let commits = if allow_no_commits {
        get_commits_for_spec_allow_no_commits(&spec.id)?
    } else {
        get_commits_for_spec(&spec.id)?
    };

    // Update spec with new commit info
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };

    // Update the timestamp to now
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );

    // Update model name
    spec.frontmatter.model = get_model_name(Some(config));

    // Ensure spec is marked as completed
    spec.frontmatter.status = SpecStatus::Completed;

    // Save the spec
    spec.save(spec_path)
        .context("Failed to save re-finalized spec")?;

    // Validation 1: Verify that status is Completed
    anyhow::ensure!(
        spec.frontmatter.status == SpecStatus::Completed,
        "Status was not set to Completed after re-finalization"
    );

    // Validation 2: Verify completed_at timestamp is set and valid
    let completed_at = spec
        .frontmatter
        .completed_at
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("completed_at timestamp was not set"))?;

    if !completed_at.ends_with('Z') {
        anyhow::bail!(
            "completed_at must end with 'Z' (UTC format), got: {}",
            completed_at
        );
    }
    if !completed_at.contains('T') {
        anyhow::bail!(
            "completed_at must contain 'T' separator (ISO format), got: {}",
            completed_at
        );
    }

    // Validation 3: Verify spec was saved (reload and check)
    let saved_spec =
        Spec::load(spec_path).context("Failed to reload spec from disk to verify persistence")?;

    anyhow::ensure!(
        saved_spec.frontmatter.status == SpecStatus::Completed,
        "Persisted spec status is not Completed - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.completed_at.is_some(),
        "Persisted spec is missing completed_at - save may have failed"
    );

    Ok(())
}

/// Prompt for user confirmation
/// Returns true if user confirms, false otherwise
/// force_flag bypasses the confirmation
pub fn confirm_re_finalize(spec_id: &str, force_flag: bool) -> Result<bool> {
    if force_flag {
        return Ok(true);
    }

    println!(
        "{} Are you sure you want to re-finalize spec '{}'?",
        "?".cyan(),
        spec_id
    );
    println!("This will update commits and completion timestamp to now.");
    println!("Use {} to skip this confirmation.", "--force".cyan());

    use std::io::{self, Write};
    print!("Continue? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Finalize a replayed spec with audit trail tracking
/// Preserves original_completed_at from first completion and tracks replay metadata
/// Increments replay_count and sets replayed_at timestamp
#[allow(dead_code)]
pub fn replay_finalize_spec(
    spec: &mut Spec,
    spec_path: &Path,
    config: &Config,
    allow_no_commits: bool,
) -> Result<()> {
    // First, get the commits
    let commits = if allow_no_commits {
        get_commits_for_spec_allow_no_commits(&spec.id)?
    } else {
        get_commits_for_spec(&spec.id)?
    };

    // Store the original completed_at if this is the first replay
    let should_preserve_original = spec.frontmatter.original_completed_at.is_none();
    if should_preserve_original {
        if let Some(completed_at) = &spec.frontmatter.completed_at {
            spec.frontmatter.original_completed_at = Some(completed_at.clone());
        } else {
            // If no completed_at exists, this shouldn't happen if validation works,
            // but handle gracefully by using current time as original
            spec.frontmatter.original_completed_at = Some(
                chrono::Local::now()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            );
        }
    }

    // Update replay tracking
    let current_time = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    spec.frontmatter.replayed_at = Some(current_time);

    // Increment replay_count (starts at 1 for first replay)
    spec.frontmatter.replay_count = Some(spec.frontmatter.replay_count.unwrap_or(0) + 1);

    // Update status, commits, completed_at, and model
    spec.frontmatter.status = SpecStatus::Completed;
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );
    spec.frontmatter.model = get_model_name(Some(config));

    // Save the spec - this must not fail silently
    spec.save(spec_path)
        .context("Failed to save replayed spec")?;

    // Validation 1: Verify that replay fields were set
    anyhow::ensure!(
        spec.frontmatter.replayed_at.is_some(),
        "replayed_at timestamp was not set"
    );

    anyhow::ensure!(
        spec.frontmatter.replay_count.is_some(),
        "replay_count was not set"
    );

    anyhow::ensure!(
        spec.frontmatter.original_completed_at.is_some(),
        "original_completed_at was not preserved"
    );

    // Validation 2: Verify that spec was actually saved (reload and check)
    let saved_spec =
        Spec::load(spec_path).context("Failed to reload spec from disk to verify persistence")?;

    anyhow::ensure!(
        saved_spec.frontmatter.status == SpecStatus::Completed,
        "Persisted spec status is not Completed - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.replayed_at.is_some(),
        "Persisted spec is missing replayed_at - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.replay_count.is_some(),
        "Persisted spec is missing replay_count - save may have failed"
    );

    anyhow::ensure!(
        saved_spec.frontmatter.original_completed_at.is_some(),
        "Persisted spec is missing original_completed_at - save may have failed"
    );

    Ok(())
}

/// Append agent output to the spec body, truncating if too long.
pub fn append_agent_output(spec: &mut Spec, output: &str) {
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let formatted_output = if output.len() > MAX_AGENT_OUTPUT_CHARS {
        let truncated = &output[..MAX_AGENT_OUTPUT_CHARS];
        format!(
            "{}\n\n... (output truncated, {} chars total)",
            truncated,
            output.len()
        )
    } else {
        output.to_string()
    };

    let agent_section = format!(
        "\n\n## Agent Output\n\n{}\n\n```\n{}```\n",
        timestamp,
        formatted_output.trim_end()
    );

    spec.body.push_str(&agent_section);
}
