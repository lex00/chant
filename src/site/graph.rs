//! ASCII dependency graph generation for specs.
//!
//! This module generates ASCII art representations of spec dependencies
//! using box-drawing characters.

use std::collections::{HashMap, HashSet};

use crate::config::GraphDetailLevel;
use crate::spec::Spec;

/// Build an ASCII dependency graph from specs.
///
/// Returns:
/// - The ASCII graph as a string
/// - List of root spec IDs (no dependencies)
/// - List of leaf spec IDs (nothing depends on them)
pub fn build_dependency_graph(
    specs: &[&Spec],
    detail_level: GraphDetailLevel,
) -> (String, Vec<String>, Vec<String>) {
    // Build adjacency lists
    let mut depends_on: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut depended_by: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_ids: HashSet<&str> = HashSet::new();

    for spec in specs {
        all_ids.insert(&spec.id);
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep in deps {
                depends_on.entry(&spec.id).or_default().push(dep.as_str());
                depended_by.entry(dep.as_str()).or_default().push(&spec.id);
            }
        }
    }

    // Find roots (specs with no dependencies)
    let roots: Vec<String> = specs
        .iter()
        .filter(|s| {
            s.frontmatter
                .depends_on
                .as_ref()
                .map(|d| d.is_empty())
                .unwrap_or(true)
        })
        .map(|s| s.id.clone())
        .collect();

    // Find leaves (specs that nothing depends on)
    let leaves: Vec<String> = specs
        .iter()
        .filter(|s| !depended_by.contains_key(s.id.as_str()))
        .map(|s| s.id.clone())
        .collect();

    // Build the ASCII graph
    let graph = build_ascii_graph(specs, &depends_on, detail_level);

    (graph, roots, leaves)
}

/// Build the ASCII representation of the graph
fn build_ascii_graph(
    specs: &[&Spec],
    depends_on: &HashMap<&str, Vec<&str>>,
    detail_level: GraphDetailLevel,
) -> String {
    if specs.is_empty() {
        return "(No specs to display)".to_string();
    }

    let mut output = String::new();

    // Group specs by their dependency depth
    let depths = calculate_depths(specs, depends_on);

    // Find specs at each depth level
    let mut depth_groups: HashMap<usize, Vec<&Spec>> = HashMap::new();
    for spec in specs {
        let depth = *depths.get(spec.id.as_str()).unwrap_or(&0);
        depth_groups.entry(depth).or_default().push(*spec);
    }

    // Get max depth
    let max_depth = depth_groups.keys().max().copied().unwrap_or(0);

    // Render each depth level
    for depth in 0..=max_depth {
        if let Some(level_specs) = depth_groups.get(&depth) {
            // Render boxes for this level
            let boxes = render_spec_boxes(level_specs, detail_level);
            output.push_str(&boxes);
            output.push('\n');

            // Render connections to next level if not last
            if depth < max_depth {
                if let Some(next_specs) = depth_groups.get(&(depth + 1)) {
                    let connections = render_connections(level_specs, next_specs, depends_on);
                    output.push_str(&connections);
                    output.push('\n');
                }
            }
        }
    }

    output
}

/// Calculate the depth (distance from root) of each spec
fn calculate_depths<'a>(
    specs: &[&'a Spec],
    depends_on: &HashMap<&str, Vec<&str>>,
) -> HashMap<&'a str, usize> {
    let mut depths: HashMap<&str, usize> = HashMap::new();

    // Initialize roots at depth 0
    for spec in specs {
        let has_deps = depends_on
            .get(spec.id.as_str())
            .map(|d| !d.is_empty())
            .unwrap_or(false);

        if !has_deps {
            depths.insert(&spec.id, 0);
        }
    }

    // Calculate depths iteratively
    let mut changed = true;
    while changed {
        changed = false;
        for spec in specs {
            if let Some(deps) = depends_on.get(spec.id.as_str()) {
                // Find max depth of dependencies
                let max_dep_depth = deps
                    .iter()
                    .filter_map(|d| depths.get(d))
                    .max()
                    .copied()
                    .unwrap_or(0);

                let new_depth = max_dep_depth + 1;
                let current = depths.get(spec.id.as_str()).copied();

                if current.map(|c| new_depth > c).unwrap_or(true) {
                    depths.insert(&spec.id, new_depth);
                    changed = true;
                }
            }
        }
    }

    // Handle any remaining specs without computed depths
    for spec in specs {
        depths.entry(&spec.id).or_insert(0);
    }

    depths
}

/// Render spec boxes at the same level
fn render_spec_boxes(specs: &[&Spec], detail_level: GraphDetailLevel) -> String {
    if specs.is_empty() {
        return String::new();
    }

    let boxes: Vec<String> = specs.iter().map(|s| render_box(s, detail_level)).collect();

    // Find the height of the tallest box
    let max_height = boxes.iter().map(|b| b.lines().count()).max().unwrap_or(0);

    // Pad all boxes to the same height
    let padded_boxes: Vec<Vec<String>> = boxes
        .iter()
        .map(|b| {
            let lines: Vec<String> = b.lines().map(|l| l.to_string()).collect();
            let width = lines.first().map(|l| l.chars().count()).unwrap_or(0);
            let mut padded = lines;
            while padded.len() < max_height {
                padded.push(" ".repeat(width));
            }
            padded
        })
        .collect();

    // Combine horizontally
    let mut result = String::new();
    for row in 0..max_height {
        for (i, box_lines) in padded_boxes.iter().enumerate() {
            if i > 0 {
                result.push_str("     "); // Space between boxes
            }
            if row < box_lines.len() {
                result.push_str(&box_lines[row]);
            }
        }
        result.push('\n');
    }

    result
}

/// Render a single spec box
fn render_box(spec: &Spec, detail_level: GraphDetailLevel) -> String {
    let short_id = spec.id.split('-').skip(3).collect::<Vec<_>>().join("-");

    let short_id = if short_id.is_empty() {
        &spec.id
    } else {
        &short_id
    };

    match detail_level {
        GraphDetailLevel::Minimal => {
            let width = short_id.len().max(5) + 4;
            let top = format!("┌{}┐", "─".repeat(width - 2));
            let content = format!("│ {:^width$} │", short_id, width = width - 4);
            let bottom = format!("└{}┘", "─".repeat(width - 2));
            format!("{}\n{}\n{}", top, content, bottom)
        }
        GraphDetailLevel::Titles => {
            let title = spec
                .title
                .as_ref()
                .map(|t| truncate(t, 15))
                .unwrap_or_else(|| "Untitled".to_string());

            let width = short_id.len().max(title.len()).max(10) + 4;
            let top = format!("┌{}┐", "─".repeat(width - 2));
            let id_line = format!("│ {:^width$} │", short_id, width = width - 4);
            let title_line = format!("│ {:^width$} │", title, width = width - 4);
            let bottom = format!("└{}┘", "─".repeat(width - 2));
            format!("{}\n{}\n{}\n{}", top, id_line, title_line, bottom)
        }
        GraphDetailLevel::Full => {
            let title = spec
                .title
                .as_ref()
                .map(|t| truncate(t, 15))
                .unwrap_or_else(|| "Untitled".to_string());

            let status = format!("{:?}", spec.frontmatter.status);
            let status = truncate(&status, 12);

            let labels = spec
                .frontmatter
                .labels
                .as_ref()
                .map(|l| l.join(", "))
                .map(|l| truncate(&l, 12))
                .unwrap_or_default();

            let width = short_id
                .len()
                .max(title.len())
                .max(status.len())
                .max(labels.len())
                .max(10)
                + 4;

            let top = format!("┌{}┐", "─".repeat(width - 2));
            let id_line = format!("│ {:^width$} │", short_id, width = width - 4);
            let title_line = format!("│ {:^width$} │", title, width = width - 4);
            let status_line = format!("│ {:^width$} │", status, width = width - 4);
            let bottom = format!("└{}┘", "─".repeat(width - 2));

            if labels.is_empty() {
                format!(
                    "{}\n{}\n{}\n{}\n{}",
                    top, id_line, title_line, status_line, bottom
                )
            } else {
                let labels_line = format!("│ {:^width$} │", labels, width = width - 4);
                format!(
                    "{}\n{}\n{}\n{}\n{}\n{}",
                    top, id_line, title_line, status_line, labels_line, bottom
                )
            }
        }
    }
}

/// Render connections between levels
fn render_connections(
    from_specs: &[&Spec],
    to_specs: &[&Spec],
    depends_on: &HashMap<&str, Vec<&str>>,
) -> String {
    // Simple connection rendering
    let mut has_connections = false;

    for to_spec in to_specs {
        if let Some(deps) = depends_on.get(to_spec.id.as_str()) {
            for dep in deps {
                if from_specs.iter().any(|s| s.id == *dep) {
                    has_connections = true;
                    break;
                }
            }
        }
        if has_connections {
            break;
        }
    }

    if has_connections {
        let width = from_specs.len() * 20; // Approximate width
        let padding = " ".repeat(width / 4);
        format!("{}│\n{}▼", padding, padding)
    } else {
        String::new()
    }
}

/// Truncate a string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 2).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{SpecFrontmatter, SpecStatus};

    fn make_spec(id: &str, title: &str, deps: Option<Vec<&str>>) -> Spec {
        Spec {
            id: id.to_string(),
            title: Some(title.to_string()),
            body: String::new(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: deps.map(|d| d.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_build_dependency_graph_empty() {
        let specs: Vec<&Spec> = vec![];
        let (graph, roots, leaves) = build_dependency_graph(&specs, GraphDetailLevel::Minimal);
        assert!(graph.contains("No specs"));
        assert!(roots.is_empty());
        assert!(leaves.is_empty());
    }

    #[test]
    fn test_build_dependency_graph_single() {
        let spec = make_spec("2026-01-30-00a-xyz", "Test Spec", None);
        let specs = vec![&spec];
        let (graph, roots, leaves) = build_dependency_graph(&specs, GraphDetailLevel::Minimal);
        assert!(graph.contains("00a-xyz"));
        assert_eq!(roots.len(), 1);
        assert_eq!(leaves.len(), 1);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long string", 10), "a very l…");
    }

    #[test]
    fn test_render_box_minimal() {
        let spec = make_spec("2026-01-30-00a-xyz", "Test", None);
        let box_str = render_box(&spec, GraphDetailLevel::Minimal);
        assert!(box_str.contains("00a-xyz"));
        assert!(box_str.contains("┌"));
        assert!(box_str.contains("└"));
    }
}
