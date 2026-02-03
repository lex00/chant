//! Refresh command to update dependency status for all specs.
//!
//! This command reloads all specs from disk and recalculates their dependency
//! status, showing what changed (newly ready vs blocked).

use anyhow::Result;
use colored::Colorize;

use chant::spec::{load_all_specs, Spec, SpecStatus};

/// Get blocking dependencies for a spec.
fn get_blocking_dependencies(spec: &Spec, all_specs: &[Spec]) -> Vec<String> {
    let mut blockers = Vec::new();

    if let Some(deps) = &spec.frontmatter.depends_on {
        for dep_id in deps {
            let dep = all_specs.iter().find(|s| s.id == *dep_id);
            match dep {
                Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                Some(d) => blockers.push(format!(
                    "{} ({})",
                    dep_id,
                    status_label(&d.frontmatter.status)
                )),
                None => blockers.push(format!("{} (not found)", dep_id)),
            }
        }
    }

    blockers
}

/// Get a short label for a status.
fn status_label(status: &SpecStatus) -> &'static str {
    match status {
        SpecStatus::Pending => "pending",
        SpecStatus::InProgress => "in_progress",
        SpecStatus::Paused => "paused",
        SpecStatus::Completed => "completed",
        SpecStatus::Failed => "failed",
        SpecStatus::NeedsAttention => "needs_attention",
        SpecStatus::Ready => "ready",
        SpecStatus::Blocked => "blocked",
        SpecStatus::Cancelled => "cancelled",
    }
}

/// Execute the refresh command.
///
/// Reloads all specs from disk, recalculates dependency status, and reports
/// what changed.
pub fn cmd_refresh(verbose: bool) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    println!("Checking dependency status...");

    // Load all specs fresh from disk (this applies blocked status automatically)
    let specs = load_all_specs(&specs_dir)?;

    let total = specs.len();
    let completed = specs
        .iter()
        .filter(|s| s.frontmatter.status == SpecStatus::Completed)
        .count();
    let ready: Vec<_> = specs.iter().filter(|s| s.is_ready(&specs)).collect();
    let blocked: Vec<_> = specs
        .iter()
        .filter(|s| {
            s.frontmatter.status == SpecStatus::Blocked
                || (s.frontmatter.status == SpecStatus::Pending && s.is_blocked(&specs))
        })
        .collect();
    let in_progress = specs
        .iter()
        .filter(|s| s.frontmatter.status == SpecStatus::InProgress)
        .count();
    let pending_not_blocked = specs
        .iter()
        .filter(|s| s.frontmatter.status == SpecStatus::Pending && !s.is_blocked(&specs))
        .count();

    println!("{} Refreshed {} specs", "✓".green(), total);
    println!("  {}: {}", "Completed".green(), completed);
    println!("  {}: {}", "Ready".cyan(), ready.len());
    println!("  {}: {}", "In Progress".yellow(), in_progress);
    println!("  {}: {}", "Pending".white(), pending_not_blocked);
    println!("  {}: {}", "Blocked".red(), blocked.len());

    if verbose {
        if !ready.is_empty() {
            println!("\n{}", "Ready specs:".bold());
            for spec in &ready {
                if let Some(title) = &spec.title {
                    println!("  {} {} {}", "○".green(), spec.id, title.dimmed());
                } else {
                    println!("  {} {}", "○".green(), spec.id);
                }
            }
        }

        if !blocked.is_empty() {
            println!("\n{}", "Blocked specs:".bold());
            for spec in &blocked {
                let blockers = get_blocking_dependencies(spec, &specs);
                let title = spec.title.as_deref().unwrap_or("");
                if blockers.is_empty() {
                    // Blocked by prior sibling or other reason
                    println!("  {} {} {}", "⊗".red(), spec.id, title.dimmed());
                } else {
                    println!(
                        "  {} {} {} (blocked by: {})",
                        "⊗".red(),
                        spec.id,
                        title.dimmed(),
                        blockers.join(", ")
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chant::spec::SpecFrontmatter;

    #[test]
    fn test_get_blocking_dependencies_none() {
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        let blockers = get_blocking_dependencies(&spec, &[]);
        assert!(blockers.is_empty());
    }

    #[test]
    fn test_get_blocking_dependencies_unmet() {
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        let dependency = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Dependency".to_string()),
            body: "# Dependency\n\nBody.".to_string(),
        };

        let blockers = get_blocking_dependencies(&spec, &[dependency]);
        assert_eq!(blockers.len(), 1);
        assert!(blockers[0].contains("2026-01-27-002-def"));
        assert!(blockers[0].contains("pending"));
    }

    #[test]
    fn test_get_blocking_dependencies_met() {
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        let dependency = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Dependency".to_string()),
            body: "# Dependency\n\nBody.".to_string(),
        };

        let blockers = get_blocking_dependencies(&spec, &[dependency]);
        assert!(blockers.is_empty());
    }

    #[test]
    fn test_get_blocking_dependencies_not_found() {
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["nonexistent-spec".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: "# Test\n\nBody.".to_string(),
        };

        let blockers = get_blocking_dependencies(&spec, &[]);
        assert_eq!(blockers.len(), 1);
        assert!(blockers[0].contains("nonexistent-spec"));
        assert!(blockers[0].contains("not found"));
    }

    #[test]
    fn test_status_label() {
        assert_eq!(status_label(&SpecStatus::Pending), "pending");
        assert_eq!(status_label(&SpecStatus::Completed), "completed");
        assert_eq!(status_label(&SpecStatus::Blocked), "blocked");
        assert_eq!(status_label(&SpecStatus::InProgress), "in_progress");
    }
}
