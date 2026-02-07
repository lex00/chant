//! Approval workflow functionality
//!
//! Provides the `cmd_approve` and `cmd_reject` command functions
//! along with helper functions for the approval workflow.

use anyhow::{Context, Result};
use atty;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use chant::config::{Config, RejectionAction};
use chant::id;
use chant::spec::{self, ApprovalStatus, Spec, SpecStatus};

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

/// Get list of git committers from the repository history
fn get_git_committers() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["shortlog", "-sn", "--all"])
        .output()
        .context("Failed to run git shortlog")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let committers: Vec<String> = output_str
        .lines()
        .filter_map(|line| {
            // Format is "   123\tName" - extract the name part
            let parts: Vec<&str> = line.trim().splitn(2, '\t').collect();
            parts.get(1).map(|s| s.to_string())
        })
        .collect();

    Ok(committers)
}

/// Validate that a name exists in the git committers list
fn validate_committer(name: &str) -> Result<bool> {
    let committers = get_git_committers()?;

    // Check for exact match or partial match (case-insensitive)
    let name_lower = name.to_lowercase();
    let is_valid = committers
        .iter()
        .any(|c| c.to_lowercase() == name_lower || c.to_lowercase().contains(&name_lower));

    Ok(is_valid)
}

// ============================================================================
// DISCUSSION HELPERS
// ============================================================================

/// Append a message to the Approval Discussion section in the spec body.
/// Creates the section if it doesn't exist.
fn append_to_approval_discussion(spec: &mut Spec, message: &str) {
    let discussion_header = "## Approval Discussion";

    if spec.body.contains(discussion_header) {
        // Find the section and append to it
        if let Some(pos) = spec.body.find(discussion_header) {
            let insert_pos = pos + discussion_header.len();
            // Find the next section heading or end of body
            let rest = &spec.body[insert_pos..];
            let next_section = rest.find("\n## ").unwrap_or(rest.len());
            let insert_at = insert_pos + next_section;

            // Insert the message before the next section (or at end)
            let new_body = format!(
                "{}\n\n{}{}",
                &spec.body[..insert_at].trim_end(),
                message,
                &spec.body[insert_at..]
            );
            spec.body = new_body;
        }
    } else {
        // Add the section at the end of the body
        spec.body = format!(
            "{}\n\n{}\n\n{}",
            spec.body.trim_end(),
            discussion_header,
            message
        );
    }
}

// ============================================================================
// REJECTION HANDLERS
// ============================================================================

/// Handle rejection in dependency mode: create a fix spec and link it as a dependency.
fn handle_rejection_dependency(
    specs_dir: &Path,
    spec: &mut Spec,
    spec_path: &Path,
    reason: &str,
) -> Result<()> {
    // Check if stdin is a TTY for interactive prompt
    let should_create = if atty::is(atty::Stream::Stdin) {
        // Interactive: prompt user
        eprint!("{} Create fix spec? (Y/n): ", "?".cyan());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();
        trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
    } else {
        // Non-interactive: automatically create fix spec
        true
    };

    if !should_create {
        println!(
            "{} Skipping fix spec creation. Spec remains rejected.",
            "ℹ".cyan()
        );
        return Ok(());
    }

    // Generate a new spec ID for the fix spec
    let fix_id = id::generate_id(specs_dir)?;
    let fix_filename = format!("{}.md", fix_id);
    let fix_path = specs_dir.join(&fix_filename);

    // Create the fix spec content
    let fix_description = format!("Fix rejection issues for {}", spec.id);
    let fix_content = format!(
        r#"---
type: code
status: pending
---

# {}

## Context

Original spec {} was rejected with reason:
> {}

## Acceptance Criteria

- [ ] Address rejection feedback
- [ ] Changes ready for re-review
"#,
        fix_description, spec.id, reason
    );

    std::fs::write(&fix_path, fix_content)?;

    // Update original spec: add depends_on and set status to blocked
    let depends_on = spec.frontmatter.depends_on.get_or_insert_with(Vec::new);
    depends_on.push(fix_id.clone());
    spec.set_status(SpecStatus::Blocked)
        .map_err(|e| anyhow::anyhow!("Failed to block spec: {}", e))?;
    spec.save(spec_path)?;

    // Git add and commit both files
    let output = Command::new("git")
        .args([
            "add",
            &fix_path.to_string_lossy(),
            &spec_path.to_string_lossy(),
        ])
        .output()
        .context("Failed to run git add for fix spec")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage fix spec files: {}", stderr);
    }

    let commit_message = format!(
        "chant({}): create fix spec {} (dependency mode)",
        spec.id, fix_id
    );
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to run git commit for fix spec")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
            anyhow::bail!("Failed to commit fix spec: {}", stderr);
        }
    }

    println!("{} Created fix spec {}", "✓".green(), fix_id.cyan());
    println!(
        "{} Spec {} is now blocked, waiting for fix spec {}",
        "ℹ".cyan(),
        spec.id.cyan(),
        fix_id.cyan()
    );

    Ok(())
}

/// Handle rejection in group mode: convert spec to driver with numbered member specs.
fn handle_rejection_group(
    specs_dir: &Path,
    spec: &mut Spec,
    spec_path: &Path,
    reason: &str,
) -> Result<()> {
    println!(
        "{} Converting rejected spec to driver with member specs...",
        "→".cyan()
    );

    // Extract acceptance criteria from the spec body to distribute across members
    let criteria = extract_acceptance_criteria(&spec.body);

    let driver_id = spec.id.clone();
    let mut member_ids = Vec::new();

    if criteria.is_empty() {
        // No acceptance criteria to distribute - create a single fix member
        let member_id = format!("{}.1", driver_id);
        let member_path = specs_dir.join(format!("{}.md", member_id));

        let member_content = format!(
            r#"---
type: code
status: pending
---

# Fix rejection issues for {}

## Context

Original spec was rejected with reason:
> {}

## Acceptance Criteria

- [ ] Address rejection feedback
- [ ] Changes ready for re-review
"#,
            driver_id, reason
        );

        std::fs::write(&member_path, member_content)?;
        member_ids.push(member_id);
    } else {
        // Distribute criteria across member specs
        for (index, criterion) in criteria.iter().enumerate() {
            let member_number = index + 1;
            let member_id = format!("{}.{}", driver_id, member_number);
            let member_path = specs_dir.join(format!("{}.md", member_id));

            // Build depends_on for sequential ordering (each depends on previous)
            let depends_on_line = if index > 0 {
                format!(
                    "depends_on:\n  - {}.{}\n",
                    driver_id,
                    index // previous member number
                )
            } else {
                String::new()
            };

            let member_content = format!(
                r#"---
type: code
status: pending
{}---

# {}

## Acceptance Criteria

- [ ] {}
"#,
                depends_on_line,
                criterion
                    .trim_start_matches("- [ ] ")
                    .trim_start_matches("- [x] ")
                    .trim_start_matches("- [X] "),
                criterion
                    .trim_start_matches("- [ ] ")
                    .trim_start_matches("- [x] ")
                    .trim_start_matches("- [X] ")
            );

            std::fs::write(&member_path, member_content)?;
            member_ids.push(member_id);
        }
    }

    // Update driver spec: change type to driver, add members list
    spec.frontmatter.r#type = "driver".to_string();
    spec.frontmatter.members = Some(member_ids.clone());
    spec.save(spec_path)?;

    // Git add all files
    let mut git_args: Vec<String> = vec!["add".to_string()];
    git_args.push(spec_path.to_string_lossy().to_string());
    for member_id in &member_ids {
        git_args.push(
            specs_dir
                .join(format!("{}.md", member_id))
                .to_string_lossy()
                .to_string(),
        );
    }

    let output = Command::new("git")
        .args(&git_args)
        .output()
        .context("Failed to run git add for group mode files")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage group mode files: {}", stderr);
    }

    let commit_message = format!(
        "chant({}): convert to driver with {} member specs (group mode)",
        spec.id,
        member_ids.len()
    );
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to run git commit for group mode")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
            anyhow::bail!("Failed to commit group mode changes: {}", stderr);
        }
    }

    println!(
        "{} Spec {} converted to driver",
        "✓".green(),
        spec.id.cyan()
    );
    println!("Created members:");
    for member_id in &member_ids {
        println!("  {} {}", "•".cyan(), member_id);
    }

    Ok(())
}

/// Extract acceptance criteria items from a spec body.
/// Returns a list of criterion text strings (including the checkbox prefix).
fn extract_acceptance_criteria(body: &str) -> Vec<String> {
    let acceptance_criteria_marker = "## Acceptance Criteria";
    let mut criteria = Vec::new();
    let mut in_ac_section = false;
    let mut in_code_fence = false;

    for line in body.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        if in_code_fence {
            continue;
        }

        if trimmed.starts_with(acceptance_criteria_marker) {
            in_ac_section = true;
            continue;
        }

        if in_ac_section && trimmed.starts_with("## ") {
            break;
        }

        if in_ac_section {
            let checkbox_line = trimmed;
            if checkbox_line.starts_with("- [ ] ")
                || checkbox_line.starts_with("- [x] ")
                || checkbox_line.starts_with("- [X] ")
            {
                // Only include top-level criteria (not indented sub-items)
                if line.starts_with("- ") || line.starts_with("  - ") {
                    // Skip deeply nested items (more than one level of indentation)
                    let indent = line.len() - line.trim_start().len();
                    if indent <= 2 {
                        criteria.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    criteria
}

// ============================================================================
// APPROVE COMMAND
// ============================================================================

pub fn cmd_approve(id: &str, by: &str) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Validate the approver name against git committers
    match validate_committer(by) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "{} Warning: '{}' is not a known git committer in this repository",
                "⚠".yellow(),
                by
            );
        }
        Err(e) => {
            eprintln!(
                "{} Warning: Could not validate committer name: {}",
                "⚠".yellow(),
                e
            );
        }
    }

    // Check if spec has approval section
    let approval = spec
        .frontmatter
        .approval
        .get_or_insert_with(Default::default);

    // Check if already approved
    if approval.status == ApprovalStatus::Approved {
        println!(
            "{} Spec {} is already approved{}",
            "ℹ".cyan(),
            spec.id,
            approval
                .by
                .as_ref()
                .map(|b| format!(" by {}", b))
                .unwrap_or_default()
        );
        return Ok(());
    }

    // Update approval status
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    approval.status = ApprovalStatus::Approved;
    approval.by = Some(by.to_string());
    approval.at = Some(timestamp.clone());

    // Add discussion entry
    let discussion_entry = format!(
        "**{}** - {} - APPROVED",
        by,
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    append_to_approval_discussion(&mut spec, &discussion_entry);

    // Save the spec
    spec.save(&spec_path)?;

    // Commit the change
    let output = Command::new("git")
        .args(["add", &spec_path.to_string_lossy()])
        .output()
        .context("Failed to run git add for spec file")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage spec file: {}", stderr);
    }

    let commit_message = format!("chant({}): approve spec", spec.id);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to run git commit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
            anyhow::bail!("Failed to commit spec file: {}", stderr);
        }
    }

    println!("{} Spec {} approved by {}", "✓".green(), spec.id.cyan(), by);

    Ok(())
}

// ============================================================================
// REJECT COMMAND
// ============================================================================

pub fn cmd_reject(id: &str, by: &str, reason: &str) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let config = Config::load()?;

    // Resolve spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Validate the rejector name against git committers
    match validate_committer(by) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "{} Warning: '{}' is not a known git committer in this repository",
                "⚠".yellow(),
                by
            );
        }
        Err(e) => {
            eprintln!(
                "{} Warning: Could not validate committer name: {}",
                "⚠".yellow(),
                e
            );
        }
    }

    // Check if spec has approval section
    let approval = spec
        .frontmatter
        .approval
        .get_or_insert_with(Default::default);

    // Update approval status
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    approval.status = ApprovalStatus::Rejected;
    approval.by = Some(by.to_string());
    approval.at = Some(timestamp.clone());

    // Add discussion entry with reason
    let discussion_entry = format!(
        "**{}** - {} - REJECTED\n{}",
        by,
        chrono::Local::now().format("%Y-%m-%d %H:%M"),
        reason
    );
    append_to_approval_discussion(&mut spec, &discussion_entry);

    // Save the spec
    spec.save(&spec_path)?;

    // Commit the change
    let output = Command::new("git")
        .args(["add", &spec_path.to_string_lossy()])
        .output()
        .context("Failed to run git add for spec file")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage spec file: {}", stderr);
    }

    let commit_message = format!("chant({}): reject spec", spec.id);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .context("Failed to run git commit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
            anyhow::bail!("Failed to commit spec file: {}", stderr);
        }
    }

    println!(
        "{} Spec {} rejected by {}: {}",
        "✗".red(),
        spec.id.cyan(),
        by,
        reason
    );

    // Apply rejection action based on config
    let rejection_action = config.approval.rejection_action;
    match rejection_action {
        RejectionAction::Manual => {
            // No automatic action - user handles it manually
        }
        RejectionAction::Dependency => {
            handle_rejection_dependency(&specs_dir, &mut spec, &spec_path, reason)?;
        }
        RejectionAction::Group => {
            handle_rejection_group(&specs_dir, &mut spec, &spec_path, reason)?;
        }
    }

    Ok(())
}
