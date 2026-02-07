//! Git merge driver for spec files.
//!
//! This module implements a custom merge driver for `.chant/specs/*.md` files
//! that intelligently resolves frontmatter conflicts while preserving body content.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::spec::{split_frontmatter, SpecFrontmatter, SpecStatus};

/// Merge strategy for a frontmatter field
#[derive(Debug, Clone, PartialEq)]
pub enum MergeRule {
    /// Prefer the more advanced value (for status)
    AdvancedStatus,
    /// Take from either side, preferring theirs (for completion metadata)
    PreferTheirs,
    /// Take from either side, preferring ours (for working branch metadata)
    PreferOurs,
    /// Merge both lists, deduplicate (for arrays)
    Union,
}

/// Declarative merge rules for frontmatter fields
const FIELD_RULES: &[(&str, MergeRule)] = &[
    ("status", MergeRule::AdvancedStatus),
    ("completed_at", MergeRule::PreferTheirs),
    ("model", MergeRule::PreferTheirs),
    ("commits", MergeRule::Union),
    ("branch", MergeRule::PreferOurs),
    ("labels", MergeRule::Union),
    ("target_files", MergeRule::Union),
    ("context", MergeRule::Union),
    ("members", MergeRule::Union),
    ("last_verified", MergeRule::PreferTheirs),
    ("verification_status", MergeRule::PreferTheirs),
    ("verification_failures", MergeRule::PreferTheirs),
    ("replayed_at", MergeRule::PreferOurs),
    ("replay_count", MergeRule::PreferOurs),
    ("original_completed_at", MergeRule::PreferOurs),
];

/// Get merge rule for a field (default to PreferOurs if not specified)
pub fn get_merge_rule(field: &str) -> MergeRule {
    FIELD_RULES
        .iter()
        .find(|(name, _)| *name == field)
        .map(|(_, rule)| rule.clone())
        .unwrap_or(MergeRule::PreferOurs)
}

/// Result of parsing a spec file into frontmatter and body
#[derive(Debug, Clone)]
pub struct ParsedSpec {
    pub frontmatter_yaml: String,
    pub frontmatter: SpecFrontmatter,
    pub body: String,
}

/// Parse a spec file into frontmatter and body components
pub fn parse_spec_file(content: &str) -> Result<ParsedSpec> {
    let (frontmatter_opt, body) = split_frontmatter(content);

    let frontmatter_yaml = frontmatter_opt.unwrap_or_default();
    let frontmatter: SpecFrontmatter = if !frontmatter_yaml.is_empty() {
        serde_yaml::from_str(&frontmatter_yaml).context("Failed to parse frontmatter")?
    } else {
        SpecFrontmatter::default()
    };

    Ok(ParsedSpec {
        frontmatter_yaml,
        frontmatter,
        body: body.to_string(),
    })
}

/// Merge frontmatter from base, ours, and theirs using declarative rules
pub fn merge_frontmatter(
    base: &SpecFrontmatter,
    ours: &SpecFrontmatter,
    theirs: &SpecFrontmatter,
) -> SpecFrontmatter {
    let mut result = ours.clone();

    result.status = merge_status(&base.status, &ours.status, &theirs.status);

    if result.completed_at.is_none() && theirs.completed_at.is_some() {
        result.completed_at = theirs.completed_at.clone();
    }

    if result.model.is_none() && theirs.model.is_some() {
        result.model = theirs.model.clone();
    }

    result.commits = merge_lists(&base.commits, &ours.commits, &theirs.commits);

    if result.branch.is_none() && theirs.branch.is_some() {
        result.branch = theirs.branch.clone();
    }

    result.labels = merge_lists(&base.labels, &ours.labels, &theirs.labels);
    result.target_files = merge_lists(&base.target_files, &ours.target_files, &theirs.target_files);
    result.context = merge_lists(&base.context, &ours.context, &theirs.context);
    result.members = merge_lists(&base.members, &ours.members, &theirs.members);

    if result.last_verified.is_none() && theirs.last_verified.is_some() {
        result.last_verified = theirs.last_verified.clone();
    }
    if result.verification_status.is_none() && theirs.verification_status.is_some() {
        result.verification_status = theirs.verification_status.clone();
    }
    if result.verification_failures.is_none() && theirs.verification_failures.is_some() {
        result.verification_failures = theirs.verification_failures.clone();
    }

    if result.replayed_at.is_none() && theirs.replayed_at.is_some() {
        result.replayed_at = theirs.replayed_at.clone();
    }
    if result.replay_count.is_none() && theirs.replay_count.is_some() {
        result.replay_count = theirs.replay_count;
    }
    if result.original_completed_at.is_none() && theirs.original_completed_at.is_some() {
        result.original_completed_at = theirs.original_completed_at.clone();
    }

    result
}

/// Merge status fields, preferring the more "advanced" status
fn merge_status(_base: &SpecStatus, ours: &SpecStatus, theirs: &SpecStatus) -> SpecStatus {
    let priority = |s: &SpecStatus| -> u8 {
        match s {
            SpecStatus::Cancelled => 0,
            SpecStatus::Failed => 1,
            SpecStatus::NeedsAttention => 2,
            SpecStatus::Blocked => 3,
            SpecStatus::Pending => 4,
            SpecStatus::Ready => 5,
            SpecStatus::Paused => 6,
            SpecStatus::InProgress => 7,
            SpecStatus::Completed => 8,
        }
    };

    if priority(ours) >= priority(theirs) {
        ours.clone()
    } else {
        theirs.clone()
    }
}

/// Merge lists using union strategy (deduplicate)
fn merge_lists(
    _base: &Option<Vec<String>>,
    ours: &Option<Vec<String>>,
    theirs: &Option<Vec<String>>,
) -> Option<Vec<String>> {
    match (ours, theirs) {
        (Some(o), Some(t)) => {
            let mut result: Vec<String> = o.clone();
            for item in t {
                if !result.contains(item) {
                    result.push(item.clone());
                }
            }
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        }
        (Some(o), None) => Some(o.clone()),
        (None, Some(t)) => Some(t.clone()),
        (None, None) => None,
    }
}

/// Merge body content using git's 3-way merge
pub fn merge_body(base: &str, ours: &str, theirs: &str) -> Result<String> {
    if base.trim() == ours.trim() {
        return Ok(theirs.to_string());
    }
    if base.trim() == theirs.trim() {
        return Ok(ours.to_string());
    }
    if ours.trim() == theirs.trim() {
        return Ok(ours.to_string());
    }

    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let base_path = temp_dir.path().join("base");
    let ours_path = temp_dir.path().join("ours");
    let theirs_path = temp_dir.path().join("theirs");

    fs::write(&base_path, base).context("Failed to write base file")?;
    fs::write(&ours_path, ours).context("Failed to write ours file")?;
    fs::write(&theirs_path, theirs).context("Failed to write theirs file")?;

    let output = Command::new("git")
        .args([
            "merge-file",
            "-p",
            ours_path.to_str().unwrap(),
            base_path.to_str().unwrap(),
            theirs_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to run git merge-file")?;

    let merged = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(merged)
}

/// Serialize frontmatter back to YAML string
pub fn serialize_frontmatter(frontmatter: &SpecFrontmatter) -> Result<String> {
    serde_yaml::to_string(frontmatter).context("Failed to serialize frontmatter")
}

/// Assemble a spec file from frontmatter and body
pub fn assemble_spec(frontmatter: &SpecFrontmatter, body: &str) -> Result<String> {
    let frontmatter_yaml = serialize_frontmatter(frontmatter)?;
    Ok(format!("---\n{}---\n{}", frontmatter_yaml, body))
}

/// Run the merge driver
pub fn run_merge_driver(base_path: &Path, ours_path: &Path, theirs_path: &Path) -> Result<bool> {
    let base_content = fs::read_to_string(base_path)
        .with_context(|| format!("Failed to read base file: {}", base_path.display()))?;
    let ours_content = fs::read_to_string(ours_path)
        .with_context(|| format!("Failed to read ours file: {}", ours_path.display()))?;
    let theirs_content = fs::read_to_string(theirs_path)
        .with_context(|| format!("Failed to read theirs file: {}", theirs_path.display()))?;

    let base = parse_spec_file(&base_content)?;
    let ours = parse_spec_file(&ours_content)?;
    let theirs = parse_spec_file(&theirs_content)?;

    let merged_frontmatter =
        merge_frontmatter(&base.frontmatter, &ours.frontmatter, &theirs.frontmatter);

    let merged_body = merge_body(&base.body, &ours.body, &theirs.body)?;

    let has_conflicts = merged_body.contains("<<<<<<<")
        || merged_body.contains("=======")
        || merged_body.contains(">>>>>>>");

    let result = assemble_spec(&merged_frontmatter, &merged_body)?;

    fs::write(ours_path, result)
        .with_context(|| format!("Failed to write result to: {}", ours_path.display()))?;

    Ok(!has_conflicts)
}

/// Generate git configuration instructions for the merge driver
pub fn get_setup_instructions() -> String {
    r#"# Chant Spec Merge Driver Setup

## Step 1: Add .gitattributes entry

Add to your `.gitattributes` file (or create one):

```
.chant/specs/*.md merge=chant-spec
```

## Step 2: Configure git merge driver

Run these commands in your repository:

```bash
git config merge.chant-spec.name "Chant spec merge driver"
git config merge.chant-spec.driver "chant merge-driver %O %A %B"
```

## How it works

The merge driver intelligently handles spec file merges by:

1. **Frontmatter conflicts**: Automatically resolved using declarative rules
   - `status`: Prefers more "advanced" status (completed > in_progress > pending)
   - `completed_at`, `model`: Takes values from theirs (finalize metadata)
   - `commits`, `labels`, `target_files`: Merges both lists, deduplicates

2. **Body conflicts**: Uses standard 3-way merge
   - Shows conflict markers if both sides changed same section
"#
    .to_string()
}

/// Result of setting up the merge driver
#[derive(Debug, Clone)]
pub struct MergeDriverSetupResult {
    pub git_config_set: bool,
    pub gitattributes_updated: bool,
    pub warning: Option<String>,
}

/// Set up the merge driver for the current repository
pub fn setup_merge_driver() -> Result<MergeDriverSetupResult> {
    let mut result = MergeDriverSetupResult {
        git_config_set: false,
        gitattributes_updated: false,
        warning: None,
    };

    let in_git_repo = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if in_git_repo {
        let name_result = Command::new("git")
            .args(["config", "merge.chant-spec.name", "Chant spec merge driver"])
            .output();

        let driver_result = Command::new("git")
            .args([
                "config",
                "merge.chant-spec.driver",
                "chant merge-driver %O %A %B",
            ])
            .output();

        match (name_result, driver_result) {
            (Ok(name_out), Ok(driver_out))
                if name_out.status.success() && driver_out.status.success() =>
            {
                result.git_config_set = true;
            }
            _ => {
                result.warning = Some("Failed to configure git merge driver".to_string());
            }
        }
    } else {
        result.warning = Some("Not in a git repository - merge driver config skipped".to_string());
    }

    let gitattributes_path = std::path::Path::new(".gitattributes");
    let merge_pattern = ".chant/specs/*.md merge=chant-spec";

    if gitattributes_path.exists() {
        let content =
            fs::read_to_string(gitattributes_path).context("Failed to read .gitattributes")?;

        if !content.contains(merge_pattern) {
            let mut new_content = content;
            if !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str("\n# Chant spec files use a custom merge driver for intelligent conflict resolution\n");
            new_content.push_str(merge_pattern);
            new_content.push('\n');
            fs::write(gitattributes_path, new_content)
                .context("Failed to update .gitattributes")?;
            result.gitattributes_updated = true;
        }
    } else {
        let content = format!(
            "# Chant spec files use a custom merge driver for intelligent conflict resolution\n# This driver automatically resolves frontmatter conflicts while preserving implementation content\n#\n# The merge driver is configured automatically by `chant init`\n{}\n",
            merge_pattern
        );
        fs::write(gitattributes_path, content).context("Failed to create .gitattributes")?;
        result.gitattributes_updated = true;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spec_file_basic() {
        let content = r#"---
type: code
status: pending
---
# Test Spec

Body content here.
"#;
        let result = parse_spec_file(content).unwrap();
        assert_eq!(result.frontmatter.status, SpecStatus::Pending);
        assert!(result.body.contains("# Test Spec"));
        assert!(result.body.contains("Body content here."));
    }

    #[test]
    fn test_merge_status_prefers_completed() {
        let base = SpecStatus::Pending;
        let ours = SpecStatus::InProgress;
        let theirs = SpecStatus::Completed;

        let result = merge_status(&base, &ours, &theirs);
        assert_eq!(result, SpecStatus::Completed);
    }

    #[test]
    fn test_merge_lists_deduplicates() {
        let base = Some(vec!["abc".to_string()]);
        let ours = Some(vec!["abc".to_string(), "def".to_string()]);
        let theirs = Some(vec!["abc".to_string(), "ghi".to_string()]);

        let result = merge_lists(&base, &ours, &theirs);
        let result = result.unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"abc".to_string()));
        assert!(result.contains(&"def".to_string()));
        assert!(result.contains(&"ghi".to_string()));
    }

    #[test]
    fn test_merge_frontmatter_takes_completed_at_from_theirs() {
        let base = SpecFrontmatter::default();
        let ours = SpecFrontmatter {
            status: SpecStatus::InProgress,
            ..Default::default()
        };
        let theirs = SpecFrontmatter {
            status: SpecStatus::Completed,
            completed_at: Some("2026-01-27T10:00:00Z".to_string()),
            model: Some("claude-opus-4-5".to_string()),
            ..Default::default()
        };

        let result = merge_frontmatter(&base, &ours, &theirs);
        assert_eq!(result.status, SpecStatus::Completed);
        assert_eq!(
            result.completed_at,
            Some("2026-01-27T10:00:00Z".to_string())
        );
        assert_eq!(result.model, Some("claude-opus-4-5".to_string()));
    }

    #[test]
    fn test_merge_body_takes_ours_when_theirs_unchanged() {
        let base = "Original content";
        let ours = "Modified content";
        let theirs = "Original content";

        let result = merge_body(base, ours, theirs).unwrap();
        assert_eq!(result, "Modified content");
    }

    #[test]
    fn test_get_merge_rule() {
        assert_eq!(get_merge_rule("status"), MergeRule::AdvancedStatus);
        assert_eq!(get_merge_rule("completed_at"), MergeRule::PreferTheirs);
        assert_eq!(get_merge_rule("branch"), MergeRule::PreferOurs);
        assert_eq!(get_merge_rule("labels"), MergeRule::Union);
        assert_eq!(get_merge_rule("unknown_field"), MergeRule::PreferOurs);
    }

    #[test]
    fn test_real_world_scenario() {
        let base_content = r#"---
type: code
status: pending
---
# Implement feature X

## Problem

Description of the problem.

## Acceptance Criteria

- [ ] Feature X implemented
- [ ] Tests passing
"#;

        let ours_content = r#"---
type: code
status: in_progress
commits:
  - abc123
---
# Implement feature X

## Problem

Description of the problem.

## Solution

Here's how we solved it...

## Acceptance Criteria

- [x] Feature X implemented
- [x] Tests passing
"#;

        let theirs_content = r#"---
type: code
status: completed
completed_at: 2026-01-27T15:00:00Z
model: claude-opus-4-5
commits:
  - def456
---
# Implement feature X

## Problem

Description of the problem.

## Acceptance Criteria

- [ ] Feature X implemented
- [ ] Tests passing
"#;

        let base = parse_spec_file(base_content).unwrap();
        let ours = parse_spec_file(ours_content).unwrap();
        let theirs = parse_spec_file(theirs_content).unwrap();

        let merged_fm =
            merge_frontmatter(&base.frontmatter, &ours.frontmatter, &theirs.frontmatter);

        assert_eq!(merged_fm.status, SpecStatus::Completed);
        assert_eq!(
            merged_fm.completed_at,
            Some("2026-01-27T15:00:00Z".to_string())
        );
        assert_eq!(merged_fm.model, Some("claude-opus-4-5".to_string()));
        let commits = merged_fm.commits.unwrap();
        assert!(commits.contains(&"abc123".to_string()));
        assert!(commits.contains(&"def456".to_string()));

        let merged_body = merge_body(&base.body, &ours.body, &theirs.body).unwrap();
        assert!(
            merged_body.contains("## Solution") || merged_body.contains("Here's how we solved it")
        );
    }
}
