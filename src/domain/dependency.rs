//! Pure dependency graph functions for topological sorting and cycle detection.

use crate::spec::Spec;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Detects cycles in the dependency graph.
///
/// Returns a list of cycles, where each cycle is represented as a list of spec IDs.
/// If no cycles exist, returns an empty vector.
///
/// # Arguments
///
/// * `specs` - Slice of specs to analyze
///
/// # Returns
///
/// Vector of cycles, each represented as a vector of spec IDs forming the cycle
pub fn detect_cycles(specs: &[Spec]) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    // Build adjacency map
    let adj_map = build_adjacency_map(specs);

    for spec in specs {
        if !visited.contains(&spec.id) {
            if let Some(cycle) =
                detect_cycle_dfs(&spec.id, &adj_map, &mut visited, &mut rec_stack, &mut path)
            {
                cycles.push(cycle);
            }
        }
    }

    cycles
}

/// Performs depth-first search to detect cycles.
fn detect_cycle_dfs(
    node: &str,
    adj_map: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(deps) = adj_map.get(node) {
        for dep in deps {
            if !visited.contains(dep) {
                if let Some(cycle) = detect_cycle_dfs(dep, adj_map, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(dep) {
                // Found a cycle - extract it from path
                let cycle_start = path.iter().position(|id| id == dep).unwrap();
                let cycle = path[cycle_start..].to_vec();
                return Some(cycle);
            }
        }
    }

    rec_stack.remove(node);
    path.pop();
    None
}

/// Builds an adjacency map from specs.
fn build_adjacency_map(specs: &[Spec]) -> HashMap<String, Vec<String>> {
    let mut adj_map = HashMap::new();

    for spec in specs {
        let deps = spec.frontmatter.depends_on.clone().unwrap_or_default();
        adj_map.insert(spec.id.clone(), deps);
    }

    adj_map
}

/// Performs a topological sort on the specs based on their dependencies.
///
/// Returns a vector of spec IDs in topologically sorted order (dependencies before dependents).
/// Returns an error if cycles are detected.
///
/// # Arguments
///
/// * `specs` - Slice of specs to sort
///
/// # Returns
///
/// Result containing either:
/// - Ok: Vector of spec IDs in topologically sorted order
/// - Err: Error describing the cycle found
pub fn topological_sort(specs: &[Spec]) -> Result<Vec<String>> {
    // First check for cycles
    let cycles = detect_cycles(specs);
    if !cycles.is_empty() {
        let cycle_str = cycles[0].join(" -> ");
        return Err(anyhow!("Circular dependency detected: {}", cycle_str));
    }

    let spec_ids: HashSet<String> = specs.iter().map(|s| s.id.clone()).collect();

    // Build reverse adjacency map: for each spec, who depends on it?
    // If A depends_on B, then B -> A (B must come before A)
    let mut dependents_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize
    for spec in specs {
        dependents_of.entry(spec.id.clone()).or_default();
        in_degree.entry(spec.id.clone()).or_insert(0);
    }

    // Build the reverse adjacency and in-degree maps
    for spec in specs {
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep in deps {
                // Only count dependencies that exist in our spec set
                if spec_ids.contains(dep) {
                    // dep -> spec.id (dep must come before spec)
                    dependents_of
                        .entry(dep.clone())
                        .or_default()
                        .push(spec.id.clone());
                    // spec has one more incoming edge (dependency)
                    *in_degree.entry(spec.id.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Kahn's algorithm for topological sort
    let mut queue = VecDeque::new();
    let mut result = Vec::new();

    // Start with nodes that have no dependencies (in_degree = 0)
    for (id, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(id.clone());
        }
    }

    while let Some(node) = queue.pop_front() {
        result.push(node.clone());

        // For each spec that depends on this node, decrement their in-degree
        if let Some(dependents) = dependents_of.get(&node) {
            for dependent in dependents {
                if let Some(degree) = in_degree.get_mut(dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }

    // If we processed all nodes, we have a valid topological order
    if result.len() == specs.len() {
        Ok(result)
    } else {
        Err(anyhow!(
            "Failed to produce topological sort (this shouldn't happen after cycle check)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{SpecFrontmatter, SpecStatus};

    fn make_spec(id: &str, depends_on: Option<Vec<String>>) -> Spec {
        Spec {
            id: id.to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on,
                ..Default::default()
            },
            title: Some(format!("Test {}", id)),
            body: format!("# Test {}\n\nBody.", id),
        }
    }

    #[test]
    fn test_detect_cycles_no_cycles() {
        let specs = vec![
            make_spec("001", None),
            make_spec("002", Some(vec!["001".to_string()])),
            make_spec("003", Some(vec!["002".to_string()])),
        ];

        let cycles = detect_cycles(&specs);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_detect_cycles_simple_cycle() {
        let specs = vec![
            make_spec("001", Some(vec!["002".to_string()])),
            make_spec("002", Some(vec!["001".to_string()])),
        ];

        let cycles = detect_cycles(&specs);
        assert_eq!(cycles.len(), 1);
        assert!(cycles[0].contains(&"001".to_string()));
        assert!(cycles[0].contains(&"002".to_string()));
    }

    #[test]
    fn test_detect_cycles_self_cycle() {
        let specs = vec![make_spec("001", Some(vec!["001".to_string()]))];

        let cycles = detect_cycles(&specs);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], vec!["001"]);
    }

    #[test]
    fn test_detect_cycles_linear_chain() {
        let specs = vec![
            make_spec("A", Some(vec!["B".to_string()])),
            make_spec("B", Some(vec!["C".to_string()])),
            make_spec("C", None),
        ];

        let cycles = detect_cycles(&specs);
        assert!(
            cycles.is_empty(),
            "Linear chain A->B->C should have no cycles"
        );
    }

    #[test]
    fn test_detect_cycles_simple_cycle_abc() {
        let specs = vec![
            make_spec("A", Some(vec!["B".to_string()])),
            make_spec("B", Some(vec!["A".to_string()])),
        ];

        let cycles = detect_cycles(&specs);
        assert_eq!(cycles.len(), 1, "Should detect one cycle");
        assert!(
            cycles[0].contains(&"A".to_string()),
            "Cycle should contain A"
        );
        assert!(
            cycles[0].contains(&"B".to_string()),
            "Cycle should contain B"
        );
    }

    #[test]
    fn test_detect_cycles_three_node() {
        let specs = vec![
            make_spec("A", Some(vec!["B".to_string()])),
            make_spec("B", Some(vec!["C".to_string()])),
            make_spec("C", Some(vec!["A".to_string()])),
        ];

        let cycles = detect_cycles(&specs);
        assert_eq!(cycles.len(), 1, "Should detect one cycle");
        assert!(
            cycles[0].contains(&"A".to_string()),
            "Cycle should contain A"
        );
        assert!(
            cycles[0].contains(&"B".to_string()),
            "Cycle should contain B"
        );
        assert!(
            cycles[0].contains(&"C".to_string()),
            "Cycle should contain C"
        );
    }

    #[test]
    fn test_detect_cycles_self_reference() {
        let specs = vec![make_spec("A", Some(vec!["A".to_string()]))];

        let cycles = detect_cycles(&specs);
        assert_eq!(cycles.len(), 1, "Should detect one cycle");
        assert_eq!(
            cycles[0],
            vec!["A"],
            "Cycle should be just A referencing itself"
        );
    }

    #[test]
    fn test_topological_sort_linear() {
        let specs = vec![
            make_spec("001", None),
            make_spec("002", Some(vec!["001".to_string()])),
            make_spec("003", Some(vec!["002".to_string()])),
        ];

        let result = topological_sort(&specs).unwrap();
        assert_eq!(result.len(), 3);

        let pos_001 = result.iter().position(|id| id == "001").unwrap();
        let pos_002 = result.iter().position(|id| id == "002").unwrap();
        let pos_003 = result.iter().position(|id| id == "003").unwrap();

        assert!(pos_001 < pos_002);
        assert!(pos_002 < pos_003);
    }

    #[test]
    fn test_topological_sort_diamond_numeric() {
        let specs = vec![
            make_spec("001", None),
            make_spec("002", Some(vec!["001".to_string()])),
            make_spec("003", Some(vec!["001".to_string()])),
            make_spec("004", Some(vec!["002".to_string(), "003".to_string()])),
        ];

        let result = topological_sort(&specs).unwrap();
        assert_eq!(result.len(), 4);

        // 001 should come before all others
        let pos_001 = result.iter().position(|id| id == "001").unwrap();
        let pos_002 = result.iter().position(|id| id == "002").unwrap();
        let pos_003 = result.iter().position(|id| id == "003").unwrap();
        let pos_004 = result.iter().position(|id| id == "004").unwrap();

        assert!(pos_001 < pos_002);
        assert!(pos_001 < pos_003);
        assert!(pos_002 < pos_004);
        assert!(pos_003 < pos_004);
    }

    #[test]
    fn test_topological_sort_with_cycle() {
        let specs = vec![
            make_spec("001", Some(vec!["002".to_string()])),
            make_spec("002", Some(vec!["001".to_string()])),
        ];

        let result = topological_sort(&specs);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_topological_sort_no_dependencies() {
        let specs = vec![
            make_spec("001", None),
            make_spec("002", None),
            make_spec("003", None),
        ];

        let result = topological_sort(&specs).unwrap();
        assert_eq!(result.len(), 3);
        // All specs should be included, order doesn't matter when no dependencies
    }

    #[test]
    fn test_topological_sort_empty() {
        let specs: Vec<Spec> = vec![];
        let result = topological_sort(&specs).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_topological_sort_single() {
        let specs = vec![make_spec("A", None)];
        let result = topological_sort(&specs).unwrap();
        assert_eq!(result, vec!["A"]);
    }

    #[test]
    fn test_topological_sort_linear_chain() {
        let specs = vec![
            make_spec("A", Some(vec!["B".to_string()])),
            make_spec("B", Some(vec!["C".to_string()])),
            make_spec("C", None),
        ];

        let result = topological_sort(&specs).unwrap();
        assert_eq!(result.len(), 3);

        let pos_a = result.iter().position(|id| id == "A").unwrap();
        let pos_b = result.iter().position(|id| id == "B").unwrap();
        let pos_c = result.iter().position(|id| id == "C").unwrap();

        assert!(pos_c < pos_b, "C should come before B (dependencies first)");
        assert!(pos_b < pos_a, "B should come before A (dependencies first)");
    }

    #[test]
    fn test_topological_sort_diamond() {
        let specs = vec![
            make_spec("A", Some(vec!["B".to_string(), "C".to_string()])),
            make_spec("B", Some(vec!["D".to_string()])),
            make_spec("C", Some(vec!["D".to_string()])),
            make_spec("D", None),
        ];

        let result = topological_sort(&specs).unwrap();
        assert_eq!(result.len(), 4);

        let pos_a = result.iter().position(|id| id == "A").unwrap();
        let pos_b = result.iter().position(|id| id == "B").unwrap();
        let pos_c = result.iter().position(|id| id == "C").unwrap();
        let pos_d = result.iter().position(|id| id == "D").unwrap();

        assert!(pos_d < pos_b, "D should come before B");
        assert!(pos_d < pos_c, "D should come before C");
        assert!(pos_b < pos_a, "B should come before A");
        assert!(pos_c < pos_a, "C should come before A");
    }

    #[test]
    fn test_topological_sort_cycle_error() {
        let specs = vec![
            make_spec("A", Some(vec!["B".to_string()])),
            make_spec("B", Some(vec!["A".to_string()])),
        ];

        let result = topological_sort(&specs);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Circular dependency"),
            "Error should mention circular dependency"
        );
    }
}
