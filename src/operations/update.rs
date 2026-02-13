//! Spec update operation.
//!
//! Canonical implementation for updating spec fields with validation.

use anyhow::Result;
use std::path::Path;

use crate::domain::dependency;
use crate::spec::{load_all_specs, Spec, SpecStatus, TransitionBuilder};

/// Options for spec update
#[derive(Debug, Clone, Default)]
pub struct UpdateOptions {
    /// New status (validated via state machine)
    pub status: Option<SpecStatus>,
    /// Dependencies to set
    pub depends_on: Option<Vec<String>>,
    /// Labels to set
    pub labels: Option<Vec<String>>,
    /// Target files to set
    pub target_files: Option<Vec<String>>,
    /// Model to set
    pub model: Option<String>,
    /// Output text to append to body
    pub output: Option<String>,
    /// Replace body content instead of appending (default: false)
    pub replace_body: bool,
    /// Force status transition (bypass validation and agent log gate)
    pub force: bool,
}

/// Update spec fields with validation.
///
/// This is the canonical update logic used by both CLI and MCP.
/// Status transitions are validated via the state machine.
pub fn update_spec(spec: &mut Spec, spec_path: &Path, options: UpdateOptions) -> Result<()> {
    let mut updated = false;

    // Update status if provided (use TransitionBuilder with optional force)
    if let Some(new_status) = options.status {
        // Guard: reject status=completed without agent log (unless force is true)
        if new_status == SpecStatus::Completed && !options.force && !has_agent_log(&spec.id) {
            anyhow::bail!(
                "Cannot mark spec as completed: no agent execution log found. \
                 Use force parameter to override."
            );
        }

        let mut builder = TransitionBuilder::new(spec);
        if options.force {
            builder = builder.force();
        }
        builder.to(new_status)?;
        updated = true;
    }

    // Update depends_on if provided
    if let Some(depends_on) = options.depends_on {
        // Check for cycles before applying the dependency change
        let specs_dir = spec_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid spec path"))?;
        let mut all_specs = load_all_specs(specs_dir)?;

        // Create a temporary spec with the new dependencies for cycle detection
        let mut temp_spec = spec.clone();
        temp_spec.frontmatter.depends_on = Some(depends_on.clone());

        // Replace the old spec with the temporary one in the list
        if let Some(idx) = all_specs.iter().position(|s| s.id == spec.id) {
            all_specs[idx] = temp_spec;
        } else {
            // New spec, add it to the list
            all_specs.push(temp_spec);
        }

        // Detect cycles with the updated dependencies
        let cycles = dependency::detect_cycles(&all_specs);
        if !cycles.is_empty() {
            let cycle_str = cycles[0].join(" -> ");
            anyhow::bail!("Circular dependency detected: {}", cycle_str);
        }

        spec.frontmatter.depends_on = Some(depends_on);
        updated = true;
    }

    // Update labels if provided
    if let Some(labels) = options.labels {
        spec.frontmatter.labels = Some(labels);
        updated = true;
    }

    // Update target_files if provided
    if let Some(target_files) = options.target_files {
        spec.frontmatter.target_files = Some(target_files);
        updated = true;
    }

    // Update model if provided
    if let Some(model) = options.model {
        spec.frontmatter.model = Some(model);
        updated = true;
    }

    // Append or replace output if provided
    if let Some(output) = options.output {
        if !output.is_empty() {
            if options.replace_body {
                // Replace body content, preserving title heading if not in new output
                let has_title_in_output = output.lines().any(|l| l.trim().starts_with("# "));
                if !has_title_in_output {
                    if let Some(ref title) = spec.title {
                        spec.body = format!("# {}\n\n{}", title, output);
                    } else {
                        spec.body = output.clone();
                    }
                } else {
                    spec.body = output.clone();
                }
                if !spec.body.ends_with('\n') {
                    spec.body.push('\n');
                }
            } else {
                // Append output (backward-compatible default)
                if !spec.body.ends_with('\n') && !spec.body.is_empty() {
                    spec.body.push('\n');
                }
                spec.body.push_str("\n## Output\n\n");
                spec.body.push_str(&output);
                spec.body.push('\n');
            }
            updated = true;
        }
    }

    if !updated {
        anyhow::bail!("No updates specified");
    }

    // Save the spec
    spec.save(spec_path)?;

    Ok(())
}

/// Check if agent log exists for a spec
fn has_agent_log(spec_id: &str) -> bool {
    use crate::paths::LOGS_DIR;
    use std::path::PathBuf;

    let logs_dir = PathBuf::from(LOGS_DIR);

    // Check for current-generation log file (spec_id.log)
    let log_path = logs_dir.join(format!("{}.log", spec_id));
    if log_path.exists() {
        return true;
    }

    // Check for versioned log files (spec_id.N.log)
    if let Ok(entries) = std::fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            // Match pattern: spec_id.N.log where N is a number
            if filename_str.starts_with(&format!("{}.", spec_id)) && filename_str.ends_with(".log")
            {
                // Extract middle part to check if it's a number
                let middle = &filename_str[spec_id.len() + 1..filename_str.len() - 4];
                if middle.parse::<u32>().is_ok() {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Spec;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_replace_body_preserves_title() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-spec.md");

        // Create a spec with a title (matching what chant_add creates)
        let initial_content = r#"---
type: code
status: pending
---

# Some title
"#;
        fs::write(&spec_path, initial_content).unwrap();

        // Load the spec
        let mut spec = Spec::load(&spec_path).unwrap();
        assert_eq!(spec.title, Some("Some title".to_string()));

        // Update with replace_body but no title in output (exact repro from spec)
        let options = UpdateOptions {
            output: Some(
                "\n\n## Details\n\nBody text\n\n## Acceptance Criteria\n\n- [ ] test".to_string(),
            ),
            replace_body: true,
            ..Default::default()
        };

        update_spec(&mut spec, &spec_path, options).unwrap();

        // Reload the spec from disk
        let reloaded_spec = Spec::load(&spec_path).unwrap();

        // Verify title is preserved
        assert_eq!(
            reloaded_spec.title,
            Some("Some title".to_string()),
            "Title should be preserved after replace_body"
        );
        assert!(
            reloaded_spec.body.contains("# Some title"),
            "Body should contain title heading"
        );
    }

    #[test]
    fn test_replace_body_when_spec_has_no_title_initially() {
        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-spec-no-title.md");

        // Create a spec WITHOUT a title in the body
        let initial_content = r#"---
type: code
status: pending
---

Some body content without a heading
"#;
        fs::write(&spec_path, initial_content).unwrap();

        // Load the spec
        let mut spec = Spec::load(&spec_path).unwrap();
        assert_eq!(spec.title, None, "Spec should have no title");

        // Update with replace_body
        let options = UpdateOptions {
            output: Some(
                "\n\n## Details\n\nBody text\n\n## Acceptance Criteria\n\n- [ ] test".to_string(),
            ),
            replace_body: true,
            ..Default::default()
        };

        update_spec(&mut spec, &spec_path, options).unwrap();

        // Reload the spec from disk
        let reloaded_spec = Spec::load(&spec_path).unwrap();

        // In this case, there's no title to preserve, so it should still be None
        assert_eq!(reloaded_spec.title, None, "Spec should still have no title");
    }
}
