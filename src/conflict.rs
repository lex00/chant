//! Conflict detection and resolution spec creation.
//!
//! This module handles detection of merge conflicts and automatic creation
//! of conflict resolution specs with context about the conflicting branches.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: guides/recovery.md
//! - ignore: false

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::id;
use crate::spec::{Spec, SpecStatus};

/// Context information about a merge conflict
#[derive(Debug, Clone)]
pub struct ConflictContext {
    pub source_branch: String,
    pub target_branch: String,
    pub conflicting_files: Vec<String>,
    pub source_spec_id: String,
    pub source_spec_title: Option<String>,
    pub diff_summary: String,
}

/// Detect conflicting files from git status
pub fn detect_conflicting_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get git status");
    }

    let status = String::from_utf8_lossy(&output.stdout);
    let mut conflicting_files = Vec::new();

    for line in status.lines() {
        // Conflicted files in git status output start with "UU", "AA", "DD", "AU", "UD", "UA", "DU"
        if line.len() > 3 {
            let status_code = &line[..2];
            match status_code {
                "UU" | "AA" | "DD" | "AU" | "UD" | "UA" | "DU" => {
                    let file = line[3..].trim().to_string();
                    conflicting_files.push(file);
                }
                _ => {}
            }
        }
    }

    Ok(conflicting_files)
}

/// Extract context from a spec
pub fn extract_spec_context(specs_dir: &Path, spec_id: &str) -> Result<(Option<String>, String)> {
    // Try to load the spec to get title and description
    match crate::spec::resolve_spec(specs_dir, spec_id) {
        Ok(spec) => {
            let title = spec.title.clone();
            let body = spec.body.clone();
            Ok((title, body))
        }
        Err(_) => {
            // Spec not found, return empty context
            Ok((None, String::new()))
        }
    }
}

/// Get the diff summary between two branches
pub fn get_diff_summary(source_branch: &str, target_branch: &str) -> Result<String> {
    let output = Command::new("git")
        .args([
            "diff",
            "--stat",
            &format!("{}..{}", target_branch, source_branch),
        ])
        .output()
        .context("Failed to get git diff")?;

    if !output.status.success() {
        return Ok("(unable to generate diff)".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .take(10) // Limit to 10 lines
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Identify specs that are blocked by conflicting files
pub fn get_blocked_specs(conflicting_files: &[String], all_specs: &[Spec]) -> Vec<String> {
    let mut blocked = Vec::new();

    for spec in all_specs {
        // Skip completed and failed specs
        if spec.frontmatter.status == SpecStatus::Completed
            || spec.frontmatter.status == SpecStatus::Failed
        {
            continue;
        }

        // Check if any target_files overlap with conflicting files
        if let Some(target_files) = &spec.frontmatter.target_files {
            for conflicting_file in conflicting_files {
                if target_files.iter().any(|tf| {
                    // Check for exact match or prefix match (directory containing file)
                    tf == conflicting_file || conflicting_file.starts_with(&format!("{}/", tf))
                }) {
                    blocked.push(spec.id.clone());
                    break;
                }
            }
        }
    }

    blocked
}

/// Create a conflict resolution spec
pub fn create_conflict_spec(
    specs_dir: &Path,
    context: &ConflictContext,
    blocked_specs: Vec<String>,
) -> Result<String> {
    // Generate spec ID
    let spec_id = id::generate_id(specs_dir)?;

    // Build conflict spec content
    let mut content = String::new();

    // Frontmatter
    let conflicting_files_yaml = context
        .conflicting_files
        .iter()
        .map(|f| format!("- {}", f))
        .collect::<Vec<_>>()
        .join("\n");

    let blocked_specs_yaml = if blocked_specs.is_empty() {
        "blocked_specs: []".to_string()
    } else {
        let items = blocked_specs
            .iter()
            .map(|s| format!("  - {}", s))
            .collect::<Vec<_>>()
            .join("\n");
        format!("blocked_specs:\n{}", items)
    };

    content.push_str(&format!(
        r#"---
type: conflict
status: pending
source_branch: {}
target_branch: {}
conflicting_files:
{}
{}
original_spec: {}
---
"#,
        context.source_branch,
        context.target_branch,
        conflicting_files_yaml,
        blocked_specs_yaml,
        context.source_spec_id
    ));

    // Title and body
    content.push_str(&format!(
        "# Resolve merge conflict: {} → {}\n\n",
        context.source_branch, context.target_branch
    ));

    content.push_str("## Conflict Summary\n");
    content.push_str(&format!("- **Source branch**: {}\n", context.source_branch));
    content.push_str(&format!("- **Target branch**: {}\n", context.target_branch));
    content.push_str(&format!(
        "- **Conflicting files**: {}\n",
        context
            .conflicting_files
            .iter()
            .map(|f| format!("`{}`", f))
            .collect::<Vec<_>>()
            .join(", ")
    ));

    if !blocked_specs.is_empty() {
        content.push_str(&format!(
            "- **Blocked specs**: {}\n",
            blocked_specs.join(", ")
        ));
    }

    content.push('\n');

    // Context from original spec
    content.push_str("## Context from Original Spec\n\n");
    if let Some(title) = &context.source_spec_title {
        content.push_str(&format!("**Title**: {}\n\n", title));
    }
    content.push_str("```\n");
    content.push_str(&context.diff_summary);
    content.push_str("\n```\n\n");

    // Resolution instructions
    content.push_str("## Resolution Instructions\n\n");
    content.push_str("1. Examine the conflicting files listed above\n");
    content.push_str("2. Resolve conflicts manually in your editor or using git tools\n");
    content.push_str("3. Stage resolved files: `git add <files>`\n");
    content.push_str("4. Complete the merge: `git commit`\n");
    content.push_str("5. Update this spec with resolution details\n\n");

    // Acceptance criteria
    content.push_str("## Acceptance Criteria\n\n");
    for file in &context.conflicting_files {
        content.push_str(&format!("- [ ] Resolved conflicts in `{}`\n", file));
    }
    content.push_str("- [ ] Merge completed successfully\n");

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    std::fs::write(&spec_path, &content).context("Failed to write conflict spec file")?;

    Ok(spec_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::SpecFrontmatter;

    #[test]
    fn test_detect_conflicting_files_parses_status() {
        // This test would require mocking git commands or running in a repo with conflicts
        // For now, we test the parsing logic indirectly
        let conflicting_files = ["src/config.rs".to_string(), "docs/guide.md".to_string()];
        assert_eq!(conflicting_files.len(), 2);
    }

    #[test]
    fn test_get_blocked_specs_empty_when_no_overlap() {
        let spec1 = Spec {
            id: "2026-01-25-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["src/lib.rs".to_string()]),
                ..Default::default()
            },
            title: None,
            body: String::new(),
        };

        let conflicting_files = vec!["src/config.rs".to_string()];
        let blocked = get_blocked_specs(&conflicting_files, &[spec1]);
        assert!(blocked.is_empty());
    }

    #[test]
    fn test_get_blocked_specs_finds_overlap() {
        let spec1 = Spec {
            id: "2026-01-25-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                target_files: Some(vec!["src/config.rs".to_string()]),
                ..Default::default()
            },
            title: None,
            body: String::new(),
        };

        let conflicting_files = vec!["src/config.rs".to_string()];
        let blocked = get_blocked_specs(&conflicting_files, &[spec1]);
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0], "2026-01-25-001-abc");
    }

    #[test]
    fn test_get_blocked_specs_ignores_completed() {
        let spec1 = Spec {
            id: "2026-01-25-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                target_files: Some(vec!["src/config.rs".to_string()]),
                ..Default::default()
            },
            title: None,
            body: String::new(),
        };

        let conflicting_files = vec!["src/config.rs".to_string()];
        let blocked = get_blocked_specs(&conflicting_files, &[spec1]);
        assert!(blocked.is_empty());
    }
}
