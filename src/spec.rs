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
}
