//! Git merge driver for spec files.
//!
//! This module implements a custom merge driver for `.chant/specs/*.md` files
//! that intelligently resolves frontmatter conflicts while preserving body content.
//!
//! ## Problem
//!
//! When merging spec branches back to main, frontmatter conflicts occur because:
//! - Main has `status: completed` (from finalize)
//! - Feature branch has `status: in_progress`
//! - Main may have `completed_at` and `model` fields
//! - Feature branch may not have these fields yet
//!
//! ## Solution
//!
//! This merge driver:
//! 1. Parses frontmatter from base, ours, and theirs versions
//! 2. Intelligently merges frontmatter fields
//! 3. Uses standard 3-way merge for body content
//! 4. Produces a clean merge result or marks conflicts
//!
//! ## Git Configuration
//!
//! To use this merge driver, add to `.gitattributes`:
//! ```
//! .chant/specs/*.md merge=chant-spec
//! ```
//!
//! Then configure git:
//! ```
//! git config merge.chant-spec.name "Chant spec merge driver"
//! git config merge.chant-spec.driver "chant merge-driver %O %A %B"
//! ```
//!
//! # Doc Audit
//! - audited: 2026-01-27
//! - docs: guides/recovery.md
//! - ignore: false

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::spec::{split_frontmatter, SpecFrontmatter, SpecStatus};

/// Result of parsing a spec file into frontmatter and body
#[derive(Debug, Clone)]
pub struct ParsedSpec {
    /// Raw frontmatter YAML string (without ---\n markers)
    pub frontmatter_yaml: String,
    /// Parsed frontmatter structure
    pub frontmatter: SpecFrontmatter,
    /// Body content after frontmatter
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

/// Merge frontmatter from base, ours, and theirs
///
/// Strategy:
/// - `status`: Prefer the more "advanced" status (completed > in_progress > pending)
/// - `completed_at`, `model`: Take from whichever side has them (prefer ours)
/// - `commits`: Merge both lists, deduplicate
/// - Other fields: Prefer ours (feature branch) as it's fresher for work-in-progress
pub fn merge_frontmatter(
    base: &SpecFrontmatter,
    ours: &SpecFrontmatter,
    theirs: &SpecFrontmatter,
) -> SpecFrontmatter {
    let mut result = ours.clone();

    // Status: prefer the more "advanced" status
    result.status = merge_status(&base.status, &ours.status, &theirs.status);

    // completed_at: take from whichever side has it (prefer theirs as it's from finalize)
    if result.completed_at.is_none() && theirs.completed_at.is_some() {
        result.completed_at = theirs.completed_at.clone();
    }

    // model: take from whichever side has it (prefer theirs as it's from finalize)
    if result.model.is_none() && theirs.model.is_some() {
        result.model = theirs.model.clone();
    }

    // commits: merge both lists, deduplicate
    result.commits = merge_commits(&base.commits, &ours.commits, &theirs.commits);

    // branch: prefer ours (feature branch has the actual branch info)
    if result.branch.is_none() && theirs.branch.is_some() {
        result.branch = theirs.branch.clone();
    }

    // pr: prefer ours, fallback to theirs
    if result.pr.is_none() && theirs.pr.is_some() {
        result.pr = theirs.pr.clone();
    }

    // labels: merge both lists, deduplicate
    result.labels = merge_string_lists(&base.labels, &ours.labels, &theirs.labels);

    // target_files: merge both lists, deduplicate
    result.target_files =
        merge_string_lists(&base.target_files, &ours.target_files, &theirs.target_files);

    // context: merge both lists, deduplicate
    result.context = merge_string_lists(&base.context, &ours.context, &theirs.context);

    // Verification fields: prefer theirs (from finalize) if present
    if result.last_verified.is_none() && theirs.last_verified.is_some() {
        result.last_verified = theirs.last_verified.clone();
    }
    if result.verification_status.is_none() && theirs.verification_status.is_some() {
        result.verification_status = theirs.verification_status.clone();
    }
    if result.verification_failures.is_none() && theirs.verification_failures.is_some() {
        result.verification_failures = theirs.verification_failures.clone();
    }

    // Replay fields: prefer ours
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
    // Status priority (higher is more "advanced"):
    // Cancelled < Failed < NeedsAttention < Blocked < Pending < Ready < InProgress < Completed
    let priority = |s: &SpecStatus| -> u8 {
        match s {
            SpecStatus::Cancelled => 0,
            SpecStatus::Failed => 1,
            SpecStatus::NeedsAttention => 2,
            SpecStatus::Blocked => 3,
            SpecStatus::Pending => 4,
            SpecStatus::Ready => 5,
            SpecStatus::InProgress => 6,
            SpecStatus::Completed => 7,
        }
    };

    let ours_priority = priority(ours);
    let theirs_priority = priority(theirs);

    // If both changed from base, prefer the higher priority
    if ours_priority >= theirs_priority {
        ours.clone()
    } else {
        theirs.clone()
    }
}

/// Merge commit lists, deduplicating entries
fn merge_commits(
    _base: &Option<Vec<String>>,
    ours: &Option<Vec<String>>,
    theirs: &Option<Vec<String>>,
) -> Option<Vec<String>> {
    match (ours, theirs) {
        (Some(o), Some(t)) => {
            let mut result: Vec<String> = o.clone();
            for commit in t {
                if !result.contains(commit) {
                    result.push(commit.clone());
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

/// Merge string lists, deduplicating entries
fn merge_string_lists(
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
///
/// Returns Ok(merged_body) if merge succeeded, or Err with conflict markers
pub fn merge_body(base: &str, ours: &str, theirs: &str) -> Result<String> {
    // If base and ours are the same, take theirs
    if base.trim() == ours.trim() {
        return Ok(theirs.to_string());
    }
    // If base and theirs are the same, take ours
    if base.trim() == theirs.trim() {
        return Ok(ours.to_string());
    }
    // If ours and theirs are the same, take ours
    if ours.trim() == theirs.trim() {
        return Ok(ours.to_string());
    }

    // Write to temporary files and use git merge-file
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let base_path = temp_dir.path().join("base");
    let ours_path = temp_dir.path().join("ours");
    let theirs_path = temp_dir.path().join("theirs");

    fs::write(&base_path, base).context("Failed to write base file")?;
    fs::write(&ours_path, ours).context("Failed to write ours file")?;
    fs::write(&theirs_path, theirs).context("Failed to write theirs file")?;

    // Run git merge-file
    let output = Command::new("git")
        .args([
            "merge-file",
            "-p", // Write to stdout instead of overwriting
            ours_path.to_str().unwrap(),
            base_path.to_str().unwrap(),
            theirs_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to run git merge-file")?;

    let merged = String::from_utf8_lossy(&output.stdout).to_string();

    // Exit code 0 = clean merge, >0 = conflicts (but content is still usable)
    // We return the merged content either way, as it contains conflict markers if needed
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
///
/// This is the main entry point called by git.
/// Arguments:
/// - base_path: Path to the common ancestor version (%O)
/// - ours_path: Path to our version (%A) - this is also where we write the result
/// - theirs_path: Path to their version (%B)
///
/// Returns:
/// - 0 (Ok) if merge succeeded
/// - 1 (Err) if there are conflicts
pub fn run_merge_driver(base_path: &Path, ours_path: &Path, theirs_path: &Path) -> Result<bool> {
    // Read all three versions
    let base_content = fs::read_to_string(base_path)
        .with_context(|| format!("Failed to read base file: {}", base_path.display()))?;
    let ours_content = fs::read_to_string(ours_path)
        .with_context(|| format!("Failed to read ours file: {}", ours_path.display()))?;
    let theirs_content = fs::read_to_string(theirs_path)
        .with_context(|| format!("Failed to read theirs file: {}", theirs_path.display()))?;

    // Parse all three
    let base = parse_spec_file(&base_content)?;
    let ours = parse_spec_file(&ours_content)?;
    let theirs = parse_spec_file(&theirs_content)?;

    // Merge frontmatter
    let merged_frontmatter =
        merge_frontmatter(&base.frontmatter, &ours.frontmatter, &theirs.frontmatter);

    // Merge body
    let merged_body = merge_body(&base.body, &ours.body, &theirs.body)?;

    // Check for conflict markers in body
    let has_conflicts = merged_body.contains("<<<<<<<")
        || merged_body.contains("=======")
        || merged_body.contains(">>>>>>>");

    // Assemble result
    let result = assemble_spec(&merged_frontmatter, &merged_body)?;

    // Write result to ours_path (git expects us to modify this file)
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
# Configure the merge driver
git config merge.chant-spec.name "Chant spec merge driver"
git config merge.chant-spec.driver "chant merge-driver %O %A %B"
```

Or add to your `.git/config`:

```ini
[merge "chant-spec"]
    name = Chant spec merge driver
    driver = chant merge-driver %O %A %B
```

## How it works

The merge driver intelligently handles spec file merges by:

1. **Frontmatter conflicts**: Automatically resolved
   - `status`: Prefers more "advanced" status (completed > in_progress > pending)
   - `completed_at`, `model`: Takes values from either side
   - `commits`: Merges both lists, deduplicates

2. **Body conflicts**: Uses standard 3-way merge
   - Shows conflict markers if both sides changed same section

This prevents the common issue where `git checkout --theirs` discards
implementation code while keeping wrong metadata.
"#
    .to_string()
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
    fn test_parse_spec_file_with_all_fields() {
        let content = r#"---
type: code
status: completed
commits:
  - abc123
  - def456
completed_at: 2026-01-27T10:00:00Z
model: claude-opus-4-5
---
# Completed Spec

Implementation details.
"#;
        let result = parse_spec_file(content).unwrap();
        assert_eq!(result.frontmatter.status, SpecStatus::Completed);
        assert_eq!(
            result.frontmatter.model,
            Some("claude-opus-4-5".to_string())
        );
        assert_eq!(
            result.frontmatter.commits,
            Some(vec!["abc123".to_string(), "def456".to_string()])
        );
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
    fn test_merge_status_prefers_in_progress_over_pending() {
        let base = SpecStatus::Pending;
        let ours = SpecStatus::InProgress;
        let theirs = SpecStatus::Pending;

        let result = merge_status(&base, &ours, &theirs);
        assert_eq!(result, SpecStatus::InProgress);
    }

    #[test]
    fn test_merge_commits_deduplicates() {
        let base = Some(vec!["abc".to_string()]);
        let ours = Some(vec!["abc".to_string(), "def".to_string()]);
        let theirs = Some(vec!["abc".to_string(), "ghi".to_string()]);

        let result = merge_commits(&base, &ours, &theirs);
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
    fn test_merge_body_takes_theirs_when_ours_unchanged() {
        let base = "Original content";
        let ours = "Original content";
        let theirs = "Modified content";

        let result = merge_body(base, ours, theirs).unwrap();
        assert_eq!(result, "Modified content");
    }

    #[test]
    fn test_assemble_spec() {
        let frontmatter = SpecFrontmatter {
            status: SpecStatus::Completed,
            ..Default::default()
        };
        let body = "# Test\n\nContent here.";

        let result = assemble_spec(&frontmatter, body).unwrap();
        assert!(result.starts_with("---\n"));
        assert!(result.contains("status: completed"));
        assert!(result.contains("---\n# Test"));
        assert!(result.contains("Content here."));
    }

    #[test]
    fn test_merge_string_lists_deduplicates() {
        let base: Option<Vec<String>> = None;
        let ours = Some(vec!["a".to_string(), "b".to_string()]);
        let theirs = Some(vec!["b".to_string(), "c".to_string()]);

        let result = merge_string_lists(&base, &ours, &theirs).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"a".to_string()));
        assert!(result.contains(&"b".to_string()));
        assert!(result.contains(&"c".to_string()));
    }

    #[test]
    fn test_real_world_scenario() {
        // Simulate the exact conflict scenario from the spec:
        // - Main has status: completed (from finalize)
        // - Feature branch has status: in_progress
        // - Main has completed_at and model fields

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

        // Merge frontmatter
        let merged_fm =
            merge_frontmatter(&base.frontmatter, &ours.frontmatter, &theirs.frontmatter);

        // Should get completed status (higher priority)
        assert_eq!(merged_fm.status, SpecStatus::Completed);
        // Should get completed_at from theirs
        assert_eq!(
            merged_fm.completed_at,
            Some("2026-01-27T15:00:00Z".to_string())
        );
        // Should get model from theirs
        assert_eq!(merged_fm.model, Some("claude-opus-4-5".to_string()));
        // Should have both commits merged
        let commits = merged_fm.commits.unwrap();
        assert!(commits.contains(&"abc123".to_string()));
        assert!(commits.contains(&"def456".to_string()));

        // Merge body - ours has the implementation, so it should be preserved
        let merged_body = merge_body(&base.body, &ours.body, &theirs.body).unwrap();

        // The merged body should have our solution section
        assert!(
            merged_body.contains("## Solution") || merged_body.contains("Here's how we solved it")
        );
        // And our checked checkboxes (or at least not revert to unchecked without conflict)
    }
}
