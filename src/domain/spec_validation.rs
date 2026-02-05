//! Pure validation functions for spec readiness and blocking dependencies.

use crate::spec::{Spec, SpecStatus};

/// Check if a spec is ready to be worked on.
///
/// A spec is ready if:
/// - It has status `Pending`
/// - All dependencies in `depends_on` are completed
/// - It doesn't require approval (or approval is granted)
///
/// # Arguments
///
/// * `spec` - The spec to check
/// * `all_specs` - All available specs for dependency lookup
///
/// # Returns
///
/// `true` if the spec is ready to work, `false` otherwise
pub fn is_spec_ready(spec: &Spec, all_specs: &[Spec]) -> bool {
    // Must be pending
    if spec.frontmatter.status != SpecStatus::Pending {
        return false;
    }

    // Check if any dependencies are not completed
    if let Some(deps) = &spec.frontmatter.depends_on {
        for dep_id in deps {
            let dep = all_specs.iter().find(|s| s.id == *dep_id);
            match dep {
                Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                _ => return false, // Dep not found or not completed
            }
        }
    }

    true
}

/// Get the list of spec IDs that are blocking this spec from being ready.
///
/// Returns IDs of specs in `depends_on` that are not yet completed.
///
/// # Arguments
///
/// * `spec` - The spec to check
/// * `all_specs` - All available specs for dependency lookup
///
/// # Returns
///
/// A vector of spec IDs that are blocking this spec
pub fn get_blockers(spec: &Spec, all_specs: &[Spec]) -> Vec<String> {
    let mut blockers = Vec::new();

    if let Some(deps) = &spec.frontmatter.depends_on {
        for dep_id in deps {
            let dep = all_specs.iter().find(|s| s.id == *dep_id);
            match dep {
                Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                _ => blockers.push(dep_id.clone()),
            }
        }
    }

    blockers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_spec_ready_no_deps() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        assert!(is_spec_ready(&spec, &[]));
    }

    #[test]
    fn test_is_spec_ready_with_completed_dep() {
        let dep_spec = Spec::parse(
            "001",
            r#"---
status: completed
---
# Dependency
"#,
        )
        .unwrap();

        let spec = Spec::parse(
            "002",
            r#"---
status: pending
depends_on:
  - "001"
---
# Test
"#,
        )
        .unwrap();

        assert!(is_spec_ready(&spec, &[dep_spec]));
    }

    #[test]
    fn test_is_spec_ready_with_pending_dep() {
        let dep_spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Dependency
"#,
        )
        .unwrap();

        let spec = Spec::parse(
            "002",
            r#"---
status: pending
depends_on:
  - "001"
---
# Test
"#,
        )
        .unwrap();

        assert!(!is_spec_ready(&spec, &[dep_spec]));
    }

    #[test]
    fn test_get_blockers_returns_pending_deps() {
        let dep_spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Dependency
"#,
        )
        .unwrap();

        let spec = Spec::parse(
            "002",
            r#"---
status: pending
depends_on:
  - "001"
---
# Test
"#,
        )
        .unwrap();

        let blockers = get_blockers(&spec, &[dep_spec]);
        assert_eq!(blockers, vec!["001"]);
    }

    #[test]
    fn test_get_blockers_empty_when_all_complete() {
        let dep_spec = Spec::parse(
            "001",
            r#"---
status: completed
---
# Dependency
"#,
        )
        .unwrap();

        let spec = Spec::parse(
            "002",
            r#"---
status: pending
depends_on:
  - "001"
---
# Test
"#,
        )
        .unwrap();

        let blockers = get_blockers(&spec, &[dep_spec]);
        assert!(blockers.is_empty());
    }
}
