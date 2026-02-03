//! Cross-repository dependency resolution for specs.
//!
//! Handles resolution of dependencies across multiple repositories using the
//! `repo:spec-id` syntax in the `depends_on` field.
//!
//! # Doc Audit
//! - audited: 2026-01-27
//! - docs: concepts/dependencies.md
//! - ignore: false

use crate::config::RepoConfig;
use crate::id::SpecId;
use crate::spec::{Spec, SpecStatus};
use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolves a spec dependency ID (local or cross-repo) and returns the spec.
///
/// Handles both local dependencies (without repo prefix) and cross-repo dependencies
/// with the `repo:spec-id` format.
///
/// # Arguments
///
/// * `dep_id` - The dependency ID string (e.g., "2026-01-27-001-abc" or "backend:2026-01-27-001-abc")
/// * `current_repo_specs_dir` - Path to the current repo's .chant/specs directory
/// * `repos` - List of configured repositories
///
/// # Returns
///
/// The resolved spec, or an error if resolution fails
pub fn resolve_dependency(
    dep_id: &str,
    current_repo_specs_dir: &Path,
    repos: &[RepoConfig],
) -> Result<Spec> {
    let parsed_id = SpecId::parse(dep_id)?;

    if let Some(repo_name) = &parsed_id.repo {
        // Cross-repo dependency
        resolve_cross_repo_dependency(repo_name, dep_id, repos)
    } else {
        // Local dependency
        resolve_local_dependency(dep_id, current_repo_specs_dir)
    }
}

/// Resolves a local dependency in the current repository.
fn resolve_local_dependency(dep_id: &str, specs_dir: &Path) -> Result<Spec> {
    // Try to load the spec from the current repo
    let spec_path = specs_dir.join(format!("{}.md", dep_id));
    if spec_path.exists() {
        return Spec::load(&spec_path);
    }

    // Try to find in archive
    let archive_dir = specs_dir
        .parent()
        .ok_or_else(|| anyhow!("Cannot determine archive directory"))?
        .join("archive");

    if archive_dir.exists() {
        // Search for the spec in archived directories
        for entry in std::fs::read_dir(&archive_dir).context("Failed to read archive directory")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let spec_path = path.join(format!("{}.md", dep_id));
                if spec_path.exists() {
                    return Spec::load(&spec_path);
                }
            }
        }
    }

    Err(anyhow!(
        "Spec not found: {} in {}",
        dep_id,
        specs_dir.display()
    ))
}

/// Resolves a cross-repo dependency.
fn resolve_cross_repo_dependency(
    repo_name: &str,
    spec_id: &str,
    repos: &[RepoConfig],
) -> Result<Spec> {
    // Find the repo configuration
    let repo = repos
        .iter()
        .find(|r| r.name == repo_name)
        .ok_or_else(|| {
            anyhow!(
                "Repository '{}' not found in config. Add it to ~/.config/chant/config.md:\n\nrepos:\n  - name: {}\n    path: /path/to/{}",
                repo_name, repo_name, repo_name
            )
        })?;

    let repo_path = PathBuf::from(shellexpand::tilde(&repo.path).to_string());

    if !repo_path.exists() {
        return Err(anyhow!(
            "Repository path '{}' does not exist for repo '{}'",
            repo_path.display(),
            repo_name
        ));
    }

    let specs_dir = repo_path.join(".chant/specs");

    if !specs_dir.exists() {
        return Err(anyhow!(
            "Specs directory '{}' does not exist for repo '{}'",
            specs_dir.display(),
            repo_name
        ));
    }

    // Extract just the base ID without repo prefix for file lookup
    let parsed_id =
        SpecId::parse(spec_id).context(format!("Failed to parse spec ID: {}", spec_id))?;
    let base_spec_id = parsed_id.to_string();
    let spec_path = specs_dir.join(format!("{}.md", base_spec_id));

    if !spec_path.exists() {
        return Err(anyhow!(
            "Spec '{}' not found in repository '{}' at {}",
            base_spec_id,
            repo_name,
            specs_dir.display()
        ));
    }

    Spec::load(&spec_path)
}

/// Checks for circular dependencies across repos.
///
/// Returns an error if a circular dependency is detected.
pub fn check_circular_dependencies(
    spec_id: &str,
    all_specs: &[Spec],
    current_repo_specs_dir: &Path,
    repos: &[RepoConfig],
) -> Result<()> {
    let mut visited = HashSet::new();
    check_circular_deps_recursive(
        spec_id,
        &mut visited,
        all_specs,
        current_repo_specs_dir,
        repos,
    )
}

fn check_circular_deps_recursive(
    spec_id: &str,
    visited: &mut HashSet<String>,
    all_specs: &[Spec],
    current_repo_specs_dir: &Path,
    repos: &[RepoConfig],
) -> Result<()> {
    if visited.contains(spec_id) {
        return Err(anyhow!(
            "Circular dependency detected involving spec '{}'",
            spec_id
        ));
    }

    visited.insert(spec_id.to_string());

    // Find the spec
    let spec = match find_spec_by_id(spec_id, all_specs, current_repo_specs_dir, repos) {
        Ok(s) => s,
        Err(_) => {
            // If we can't find the spec, we can't check its dependencies
            // This is handled elsewhere, so just return Ok here
            return Ok(());
        }
    };

    // Check all dependencies of this spec
    if let Some(deps) = &spec.frontmatter.depends_on {
        for dep_id in deps {
            check_circular_deps_recursive(
                dep_id,
                visited,
                all_specs,
                current_repo_specs_dir,
                repos,
            )?;
        }
    }

    visited.remove(spec_id);
    Ok(())
}

/// Find a spec by ID in the all_specs list, or resolve cross-repo dependency.
pub fn find_spec_by_id(
    spec_id: &str,
    all_specs: &[Spec],
    current_repo_specs_dir: &Path,
    repos: &[RepoConfig],
) -> Result<Spec> {
    // First, try to find in local specs
    if let Some(spec) = all_specs.iter().find(|s| s.id == spec_id) {
        return Ok(spec.clone());
    }

    // Otherwise, try to resolve as a cross-repo dependency
    resolve_dependency(spec_id, current_repo_specs_dir, repos)
}

/// Check if a spec ID exists in the archive directory.
fn is_spec_archived(spec_id: &str, specs_dir: &Path) -> bool {
    let archive_dir = match specs_dir.parent() {
        Some(parent) => parent.join("archive"),
        None => return false,
    };

    if !archive_dir.exists() {
        return false;
    }

    // Search for the spec in archived directories
    if let Ok(entries) = std::fs::read_dir(&archive_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let spec_path = path.join(format!("{}.md", spec_id));
                if spec_path.exists() {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if a spec is blocked by unmet dependencies, including cross-repo deps.
pub fn is_blocked_by_dependencies(
    spec: &Spec,
    all_specs: &[Spec],
    current_repo_specs_dir: &Path,
    repos: &[RepoConfig],
) -> bool {
    if let Some(deps) = &spec.frontmatter.depends_on {
        for dep_id in deps {
            match find_spec_by_id(dep_id, all_specs, current_repo_specs_dir, repos) {
                Ok(dep_spec) => {
                    // Check if dependency is completed
                    if dep_spec.frontmatter.status == SpecStatus::Completed {
                        continue;
                    }
                    // Check if the spec is archived (archived specs are treated as completed)
                    if is_spec_archived(dep_id, current_repo_specs_dir) {
                        continue;
                    }
                    return true; // Unmet dependency (not completed and not archived)
                }
                _ => return true, // Unmet dependency (not found)
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_local_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        fs::create_dir_all(&specs_dir).unwrap();

        // Create a spec
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: Default::default(),
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };
        spec.save(&specs_dir.join("2026-01-27-001-abc.md")).unwrap();

        // Resolve it
        let resolved = resolve_local_dependency("2026-01-27-001-abc", &specs_dir).unwrap();
        assert_eq!(resolved.id, "2026-01-27-001-abc");
    }

    #[test]
    fn test_resolve_nonexistent_local_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        fs::create_dir_all(&specs_dir).unwrap();

        let result = resolve_local_dependency("2026-01-27-001-xyz", &specs_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_repo_dependency_repo_not_configured() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        fs::create_dir_all(&specs_dir).unwrap();

        let result = resolve_cross_repo_dependency("backend", "backend:2026-01-27-001-abc", &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found in config"));
    }

    #[test]
    fn test_cross_repo_dependency_path_not_exists() {
        let repos = vec![RepoConfig {
            name: "backend".to_string(),
            path: "/nonexistent/path".to_string(),
        }];

        let result = resolve_cross_repo_dependency("backend", "backend:2026-01-27-001-abc", &repos);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_check_circular_dependencies_simple() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        fs::create_dir_all(&specs_dir).unwrap();

        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                depends_on: Some(vec!["2026-01-27-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };
        spec.save(&specs_dir.join("2026-01-27-001-abc.md")).unwrap();

        let result = check_circular_dependencies("2026-01-27-001-abc", &[spec], &specs_dir, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_is_blocked_by_dependencies_unmet() {
        let spec_with_dep = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                depends_on: Some(vec!["2026-01-27-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        let dependency = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Dep".to_string()),
            body: "# Dep\n\nBody.".to_string(),
        };

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        fs::create_dir_all(&specs_dir).unwrap();

        let is_blocked = is_blocked_by_dependencies(&spec_with_dep, &[dependency], &specs_dir, &[]);
        assert!(is_blocked);
    }

    #[test]
    fn test_is_blocked_by_dependencies_met() {
        let spec_with_dep = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                depends_on: Some(vec!["2026-01-27-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        let dependency = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Dep".to_string()),
            body: "# Dep\n\nBody.".to_string(),
        };

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        fs::create_dir_all(&specs_dir).unwrap();

        let is_blocked = is_blocked_by_dependencies(&spec_with_dep, &[dependency], &specs_dir, &[]);
        assert!(!is_blocked);
    }

    #[test]
    fn test_is_blocked_by_dependencies_archived() {
        let spec_with_dep = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                depends_on: Some(vec!["2026-01-27-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        // Create an archived dependency that is NOT marked as completed
        let archived_dependency = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::InProgress, // Not completed
                ..Default::default()
            },
            title: Some("Archived Dep".to_string()),
            body: "# Archived Dep\n\nBody.".to_string(),
        };

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        let archive_dir = temp_dir.path().join("archive").join("2026-01-27");
        fs::create_dir_all(&specs_dir).unwrap();
        fs::create_dir_all(&archive_dir).unwrap();

        // Save the archived dependency to the archive directory
        archived_dependency
            .save(&archive_dir.join("2026-01-27-002-def.md"))
            .unwrap();

        // The spec should NOT be blocked because the dependency is in the archive
        let is_blocked = is_blocked_by_dependencies(&spec_with_dep, &[], &specs_dir, &[]);
        assert!(!is_blocked);
    }
}
