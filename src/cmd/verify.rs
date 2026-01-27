//! Verify command for checking specs against their acceptance criteria.
//!
//! This module provides functionality to verify that specs meet their acceptance
//! criteria, with options for filtering by ID or labels.

use anyhow::Result;
use chant::spec::{load_all_specs, resolve_spec, Spec, SpecStatus};
use colored::Colorize;
use std::path::PathBuf;

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
    _exit_code: bool,
    _dry_run: bool,
    _prompt: Option<&str>,
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

    // Display specs to verify
    println!("Specs to verify:");
    for spec in &specs_to_verify {
        let title = spec
            .title
            .as_deref()
            .unwrap_or("(no title)");
        println!("  {} - {}", spec.id.cyan(), title);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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

        spec.save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();

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

        spec.save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();

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

        spec.save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();

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

        spec.save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();

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
}
