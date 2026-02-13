//! DAG (dependency graph) visualization command

use anyhow::Result;
use colored::Colorize;

use chant::config::GraphDetailLevel;
use chant::site::graph::build_dependency_graph;
use chant::spec::{self, SpecStatus};

pub fn cmd_dag(
    detail: &str,
    status_filter: Option<&str>,
    labels: &[String],
    type_filter: Option<&str>,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Parse detail level
    let detail_level = match detail.to_lowercase().as_str() {
        "minimal" => GraphDetailLevel::Minimal,
        "titles" => GraphDetailLevel::Titles,
        "full" => GraphDetailLevel::Full,
        _ => {
            anyhow::bail!(
                "Invalid detail level '{}'. Valid options: minimal, titles, full",
                detail
            );
        }
    };

    // Load all specs
    let mut specs = spec::load_all_specs(&specs_dir)?;

    // Apply filters

    // Exclude cancelled specs
    specs.retain(|s| s.frontmatter.status != SpecStatus::Cancelled);

    // Filter by status if specified
    if let Some(status_val) = status_filter {
        let status_lower = status_val.to_lowercase();
        match status_lower.as_str() {
            "blocked" => {
                let all_specs_clone = specs.clone();
                specs.retain(|s| {
                    s.frontmatter.status == SpecStatus::Blocked
                        || (s.frontmatter.status == SpecStatus::Pending
                            && s.is_blocked(&all_specs_clone))
                });
            }
            "ready" => {
                let all_specs_clone = specs.clone();
                specs.retain(|s| s.is_ready(&all_specs_clone));
            }
            _ => {
                let target_status = match status_lower.as_str() {
                    "pending" => SpecStatus::Pending,
                    "in_progress" | "inprogress" => SpecStatus::InProgress,
                    "completed" => SpecStatus::Completed,
                    "failed" => SpecStatus::Failed,
                    "needs_attention" | "needsattention" => SpecStatus::NeedsAttention,
                    _ => {
                        anyhow::bail!("Invalid status filter: {}. Valid options: pending, in_progress, completed, failed, blocked, ready, needs_attention", status_val);
                    }
                };
                specs.retain(|s| s.frontmatter.status == target_status);
            }
        }
    }

    // Filter by type if specified
    if let Some(type_val) = type_filter {
        specs.retain(|s| s.frontmatter.r#type.to_string() == type_val);
    }

    // Filter by labels if specified (OR logic)
    if !labels.is_empty() {
        specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                spec_labels.iter().any(|l| labels.contains(l))
            } else {
                false
            }
        });
    }

    if specs.is_empty() {
        println!("{}", "(No specs to display)".dimmed());
        return Ok(());
    }

    // Build and display the graph
    let spec_refs: Vec<&_> = specs.iter().collect();
    let (graph, roots, leaves) = build_dependency_graph(&spec_refs, detail_level);

    println!("{}", graph);

    // Show summary
    println!();
    println!("{}", "Summary:".bold());
    println!("  Total specs: {}", specs.len());
    println!("  Root specs (no dependencies): {}", roots.len());
    println!("  Leaf specs (nothing depends on): {}", leaves.len());

    Ok(())
}
