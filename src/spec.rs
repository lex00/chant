use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecFrontmatter {
    #[serde(default = "default_type")]
    pub r#type: String,
    #[serde(default)]
    pub status: SpecStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

fn default_type() -> String {
    "code".to_string()
}

impl Default for SpecFrontmatter {
    fn default() -> Self {
        Self {
            r#type: default_type(),
            status: SpecStatus::Pending,
            depends_on: None,
            labels: None,
            target_files: None,
            context: None,
            prompt: None,
            branch: None,
            commits: None,
            pr: None,
            completed_at: None,
            model: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub frontmatter: SpecFrontmatter,
    pub title: Option<String>,
    pub body: String,
}

impl Spec {
    /// Count unchecked checkboxes (`- [ ]`) in the Acceptance Criteria section only.
    /// Returns the count of unchecked items in that section, skipping code fences.
    /// Uses the LAST `## Acceptance Criteria` heading outside code fences.
    pub fn count_unchecked_checkboxes(&self) -> usize {
        let acceptance_criteria_marker = "## Acceptance Criteria";

        // First pass: find the line number of the LAST AC heading outside code fences
        let mut in_code_fence = false;
        let mut last_ac_line: Option<usize> = None;

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                last_ac_line = Some(line_num);
            }
        }

        let Some(ac_start) = last_ac_line else {
            return 0;
        };

        // Second pass: count checkboxes from the AC section until next ## heading
        let mut in_code_fence = false;
        let mut in_ac_section = false;
        let mut count = 0;

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if in_code_fence {
                continue;
            }

            // Start counting at the last AC heading we found
            if line_num == ac_start {
                in_ac_section = true;
                continue;
            }

            // Stop at the next ## heading after our AC section
            if in_ac_section && trimmed.starts_with("## ") {
                break;
            }

            if in_ac_section && line.contains("- [ ]") {
                count += line.matches("- [ ]").count();
            }
        }

        count
    }

    /// Parse a spec from file content.
    pub fn parse(id: &str, content: &str) -> Result<Self> {
        let (frontmatter_str, body) = split_frontmatter(content);

        let frontmatter: SpecFrontmatter = if let Some(fm) = frontmatter_str {
            serde_yaml::from_str(&fm).context("Failed to parse spec frontmatter")?
        } else {
            SpecFrontmatter::default()
        };

        // Extract title from first heading
        let title = extract_title(body);

        Ok(Self {
            id: id.to_string(),
            frontmatter,
            title,
            body: body.to_string(),
        })
    }

    /// Load a spec from a file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read spec from {}", path.display()))?;

        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid spec filename"))?;

        Self::parse(id, &content)
    }

    /// Save the spec to a file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let frontmatter = serde_yaml::to_string(&self.frontmatter)?;
        let content = format!("---\n{}---\n{}", frontmatter, self.body);
        fs::write(path, content)?;
        Ok(())
    }

    /// Check if this spec is ready to execute.
    pub fn is_ready(&self, all_specs: &[Spec]) -> bool {
        // Must be pending
        if self.frontmatter.status != SpecStatus::Pending {
            return false;
        }

        // Check dependencies are completed
        if let Some(deps) = &self.frontmatter.depends_on {
            for dep_id in deps {
                let dep = all_specs.iter().find(|s| s.id == *dep_id);
                match dep {
                    Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                    _ => return false,
                }
            }
        }

        // Check group members are completed (if this is a driver)
        let members: Vec<_> = all_specs
            .iter()
            .filter(|s| is_member_of(&s.id, &self.id))
            .collect();

        if !members.is_empty() {
            for member in members {
                if member.frontmatter.status != SpecStatus::Completed {
                    return false;
                }
            }
        }

        true
    }
}

/// Check if `member_id` is a group member of `driver_id`.
fn is_member_of(member_id: &str, driver_id: &str) -> bool {
    // Member IDs have format: DRIVER_ID.N or DRIVER_ID.N.M
    if !member_id.starts_with(driver_id) {
        return false;
    }

    let suffix = &member_id[driver_id.len()..];
    suffix.starts_with('.') && suffix.len() > 1
}

/// Get all member specs of a driver spec.
pub fn get_members<'a>(driver_id: &str, specs: &'a [Spec]) -> Vec<&'a Spec> {
    specs
        .iter()
        .filter(|s| is_member_of(&s.id, driver_id))
        .collect()
}

/// Check if all members of a driver spec are completed or archived.
pub fn all_members_completed(driver_id: &str, specs: &[Spec]) -> bool {
    let members = get_members(driver_id, specs);
    if members.is_empty() {
        return true; // No members, so all are "completed"
    }
    members
        .iter()
        .all(|m| m.frontmatter.status == SpecStatus::Completed)
}

/// Extract the driver ID from a member ID.
/// For example: "2026-01-24-01e-o0l.1" -> "2026-01-24-01e-o0l"
/// Returns Some(driver_id) if this is a member spec, None otherwise.
pub fn extract_driver_id(member_id: &str) -> Option<String> {
    // Member IDs have format: DRIVER_ID.N or DRIVER_ID.N.M
    if let Some(pos) = member_id.find('.') {
        let (prefix, suffix) = member_id.split_at(pos);
        // Check that what follows the dot is numeric (at least up to the first non-digit)
        if suffix.len() > 1
            && suffix[1..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        {
            return Some(prefix.to_string());
        }
    }
    None
}

/// Mark the driver spec as in_progress if the current spec is a member and driver exists and is pending.
pub fn mark_driver_in_progress(specs_dir: &Path, member_id: &str) -> Result<()> {
    if let Some(driver_id) = extract_driver_id(member_id) {
        // Try to load the driver spec
        let driver_path = specs_dir.join(format!("{}.md", driver_id));
        if driver_path.exists() {
            let mut driver = Spec::load(&driver_path)?;
            if driver.frontmatter.status == SpecStatus::Pending {
                driver.frontmatter.status = SpecStatus::InProgress;
                driver.save(&driver_path)?;
            }
        }
    }
    Ok(())
}

fn split_frontmatter(content: &str) -> (Option<String>, &str) {
    let content = content.trim();

    if !content.starts_with("---") {
        return (None, content);
    }

    let rest = &content[3..];
    if let Some(end) = rest.find("---") {
        let frontmatter = rest[..end].to_string();
        let body = rest[end + 3..].trim_start();
        (Some(frontmatter), body)
    } else {
        (None, content)
    }
}

fn extract_title(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.to_string());
        }
    }
    None
}

/// Load all specs from a directory.
pub fn load_all_specs(specs_dir: &Path) -> Result<Vec<Spec>> {
    let mut specs = Vec::new();

    if !specs_dir.exists() {
        return Ok(specs);
    }

    for entry in fs::read_dir(specs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            match Spec::load(&path) {
                Ok(spec) => specs.push(spec),
                Err(e) => {
                    eprintln!("Warning: Failed to load spec {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(specs)
}

/// Resolve a partial spec ID to a full spec.
pub fn resolve_spec(specs_dir: &Path, partial_id: &str) -> Result<Spec> {
    let specs = load_all_specs(specs_dir)?;

    // Exact match
    if let Some(spec) = specs.iter().find(|s| s.id == partial_id) {
        return Ok(spec.clone());
    }

    // Suffix match (random suffix)
    let suffix_matches: Vec<_> = specs
        .iter()
        .filter(|s| s.id.ends_with(partial_id))
        .collect();
    if suffix_matches.len() == 1 {
        return Ok(suffix_matches[0].clone());
    }

    // Sequence match for today (e.g., "001")
    if partial_id.len() == 3 {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_pattern = format!("{}-{}-", today, partial_id);
        let today_matches: Vec<_> = specs
            .iter()
            .filter(|s| s.id.starts_with(&today_pattern))
            .collect();
        if today_matches.len() == 1 {
            return Ok(today_matches[0].clone());
        }
    }

    // Partial date match (e.g., "22-001" or "01-22-001")
    let partial_matches: Vec<_> = specs.iter().filter(|s| s.id.contains(partial_id)).collect();
    if partial_matches.len() == 1 {
        return Ok(partial_matches[0].clone());
    }

    if partial_matches.len() > 1 {
        anyhow::bail!(
            "Ambiguous spec ID '{}'. Matches: {}",
            partial_id,
            partial_matches
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    anyhow::bail!("Spec not found: {}", partial_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spec() {
        let content = r#"---
type: code
status: pending
---

# Fix the bug

Description here.
"#;
        let spec = Spec::parse("2026-01-22-001-x7m", content).unwrap();
        assert_eq!(spec.id, "2026-01-22-001-x7m");
        assert_eq!(spec.frontmatter.status, SpecStatus::Pending);
        assert_eq!(spec.title, Some("Fix the bug".to_string()));
    }

    #[test]
    fn test_is_member_of() {
        assert!(is_member_of("2026-01-22-001-x7m.1", "2026-01-22-001-x7m"));
        assert!(is_member_of("2026-01-22-001-x7m.2.1", "2026-01-22-001-x7m"));
        assert!(!is_member_of("2026-01-22-001-x7m", "2026-01-22-001-x7m"));
        assert!(!is_member_of("2026-01-22-002-y8n", "2026-01-22-001-x7m"));
    }

    #[test]
    fn test_extract_driver_id() {
        assert_eq!(
            extract_driver_id("2026-01-22-001-x7m.1"),
            Some("2026-01-22-001-x7m".to_string())
        );
        assert_eq!(
            extract_driver_id("2026-01-22-001-x7m.2.1"),
            Some("2026-01-22-001-x7m".to_string())
        );
        assert_eq!(extract_driver_id("2026-01-22-001-x7m"), None);
        assert_eq!(extract_driver_id("2026-01-22-001-x7m.abc"), None);
    }

    #[test]
    fn test_spec_is_ready() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        assert!(spec.is_ready(&[]));
    }

    #[test]
    fn test_spec_not_ready_if_in_progress() {
        let spec = Spec::parse(
            "001",
            r#"---
status: in_progress
---
# Test
"#,
        )
        .unwrap();

        assert!(!spec.is_ready(&[]));
    }

    #[test]
    fn test_count_unchecked_checkboxes() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Acceptance Criteria

- [ ] First unchecked item
- [x] Checked item
- [ ] Second unchecked item
- [X] Another checked item
"#,
        )
        .unwrap();

        assert_eq!(spec.count_unchecked_checkboxes(), 2);
    }

    #[test]
    fn test_count_unchecked_checkboxes_none() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Acceptance Criteria

- [x] All checked
- [X] Also checked
"#,
        )
        .unwrap();

        assert_eq!(spec.count_unchecked_checkboxes(), 0);
    }

    #[test]
    fn test_count_unchecked_checkboxes_skip_code_blocks() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Expected Format

```markdown
## Acceptance Criteria

- [ ] Example checkbox in code block
```

## Acceptance Criteria

- [x] Real checkbox
- [ ] Another real unchecked
"#,
        )
        .unwrap();

        // Should only count the 1 unchecked in the real Acceptance Criteria section
        // The one in the code block should be ignored
        assert_eq!(spec.count_unchecked_checkboxes(), 1);
    }

    #[test]
    fn test_count_unchecked_checkboxes_nested_code_blocks() {
        // This tests the pattern from spec 01j where there are multiple code fences
        // and the first ## Acceptance Criteria appears inside a code block example
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Example

```markdown
## Expected Format

```markdown
## Acceptance Criteria

- [ ] Example checkbox in code block
```

## Acceptance Criteria

- [x] Real checkbox
```

## Acceptance Criteria

- [x] First real checked
- [ ] Second real unchecked
"#,
        )
        .unwrap();

        // The real Acceptance Criteria section at the end has 1 unchecked checkbox
        // The ones inside the code block examples should all be ignored
        assert_eq!(spec.count_unchecked_checkboxes(), 1);
    }

    #[test]
    fn test_parse_spec_with_model() {
        let content = r#"---
type: code
status: completed
commits:
  - abc1234
completed_at: 2026-01-24T15:30:00Z
model: claude-opus-4-5
---

# Test spec with model

Description here.
"#;
        let spec = Spec::parse("2026-01-24-001-abc", content).unwrap();
        assert_eq!(spec.frontmatter.model, Some("claude-opus-4-5".to_string()));
        assert_eq!(spec.frontmatter.status, SpecStatus::Completed);
        assert_eq!(spec.frontmatter.commits, Some(vec!["abc1234".to_string()]));
    }

    #[test]
    fn test_parse_spec_without_model() {
        let content = r#"---
type: code
status: pending
---

# Test spec without model
"#;
        let spec = Spec::parse("2026-01-24-002-def", content).unwrap();
        assert_eq!(spec.frontmatter.model, None);
    }

    #[test]
    fn test_spec_save_includes_model() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-spec.md");

        let spec = Spec {
            id: "2026-01-24-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                model: Some("claude-opus-4-5".to_string()),
                commits: Some(vec!["abc1234".to_string()]),
                ..Default::default()
            },
            title: Some("Test spec".to_string()),
            body: "# Test spec\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(saved_content.contains("model: claude-opus-4-5"));
        assert!(saved_content.contains("commits:"));
    }

    #[test]
    fn test_mark_driver_in_progress_when_member_starts() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is pending
        let driver_spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Driver spec".to_string()),
            body: "# Driver spec\n\nBody content.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-001-abc.md");
        driver_spec.save(&driver_path).unwrap();

        // Mark driver as in_progress when member starts
        mark_driver_in_progress(specs_dir, "2026-01-24-001-abc.1").unwrap();

        // Verify driver status was updated to in_progress
        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::InProgress);
    }

    #[test]
    fn test_mark_driver_in_progress_skips_if_already_in_progress() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is already in_progress
        let driver_spec = Spec {
            id: "2026-01-24-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Driver spec".to_string()),
            body: "# Driver spec\n\nBody content.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-002-def.md");
        driver_spec.save(&driver_path).unwrap();

        // Try to mark driver as in_progress
        mark_driver_in_progress(specs_dir, "2026-01-24-002-def.1").unwrap();

        // Verify driver status is still in_progress (not changed)
        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::InProgress);
    }

    #[test]
    fn test_mark_driver_in_progress_nonexistent_driver() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Try to mark driver as in_progress when driver doesn't exist
        // Should not error, just skip
        mark_driver_in_progress(specs_dir, "2026-01-24-003-ghi.1").unwrap();
    }
}
