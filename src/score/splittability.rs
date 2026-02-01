//! Splittability scoring based on spec structure and decomposability.
//!
//! Analyzes whether a spec can be effectively decomposed by examining:
//! - Number of markdown headers (subsections)
//! - Number of target files
//! - Number of acceptance criteria
//! - Presence of coupling keywords indicating tight dependencies

use crate::scoring::SplittabilityGrade;
use crate::spec::Spec;

/// Coupling keywords that indicate tightly coupled components
const COUPLING_KEYWORDS: &[&str] = &["shared", "depends on each other", "tightly coupled"];

/// Calculate splittability grade based on spec structure and decomposability.
///
/// Grading rules:
/// - Grade A: Clear subsections (3+ headers), multiple target files (3+), independent tasks
/// - Grade B: Some structure (1-2 headers), 2 target files
/// - Grade C: Single concern, 1 target file, minimal structure
/// - Grade D: Tightly coupled indicators (many cross-references, shared state mentioned)
///
/// Edge cases:
/// - Specs already part of a group (has parent_id) should be Grade C (already split)
/// - Specs with 1 criterion should be Grade C (atomic)
/// - Detection of coupling keywords: "shared", "depends on each other", "tightly coupled"
///
/// # Arguments
///
/// * `spec` - The spec to analyze
///
/// # Returns
///
/// A `SplittabilityGrade` based on the spec's decomposability
pub fn calculate_splittability(spec: &Spec) -> SplittabilityGrade {
    // Edge case: Check for coupling keywords first (Grade D)
    if has_coupling_keywords(&spec.body) {
        return SplittabilityGrade::D;
    }

    // Edge case: Specs already part of a group (already split) → Grade C
    if is_part_of_group(&spec.id) {
        return SplittabilityGrade::C;
    }

    // Edge case: Specs with 1 criterion are atomic → Grade C
    let criteria_count = spec.count_total_checkboxes();
    if criteria_count == 1 {
        return SplittabilityGrade::C;
    }

    // Count structural elements
    let header_count = count_markdown_headers(&spec.body);
    let file_count = count_target_files(spec);

    // Grade A: 3+ headers, 3+ files, independent tasks
    if header_count >= 3 && file_count >= 3 {
        return SplittabilityGrade::A;
    }

    // Grade B: 1-2 headers, 2 files
    if (1..=2).contains(&header_count) && file_count == 2 {
        return SplittabilityGrade::B;
    }

    // Grade C: Single concern, 1 target file, minimal structure
    // This is the default for specs that don't fit A or B criteria
    SplittabilityGrade::C
}

/// Count markdown headers (##, ###, etc.) in the spec body.
///
/// Only counts headers outside of code fences.
/// Does not count the top-level title (single #).
fn count_markdown_headers(body: &str) -> usize {
    let mut count = 0;
    let mut in_code_fence = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // Track code fences
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        // Skip lines inside code blocks
        if in_code_fence {
            continue;
        }

        // Count headers (## or more, not single #)
        if trimmed.starts_with("##") {
            count += 1;
        }
    }

    count
}

/// Count the number of target files in the spec.
fn count_target_files(spec: &Spec) -> usize {
    spec.frontmatter
        .target_files
        .as_ref()
        .map(|files| files.len())
        .unwrap_or(0)
}

/// Check if the spec body contains coupling keywords.
///
/// Returns true if any coupling keyword is found (case-insensitive).
fn has_coupling_keywords(body: &str) -> bool {
    let body_lower = body.to_lowercase();

    for keyword in COUPLING_KEYWORDS {
        if body_lower.contains(&keyword.to_lowercase()) {
            return true;
        }
    }

    false
}

/// Check if a spec ID indicates it's part of a group.
///
/// Group members have IDs in the format: DRIVER_ID.N or DRIVER_ID.N.M
/// where N and M are numbers.
///
/// Examples:
/// - "2026-01-25-00y-abc.1" → true (member of group)
/// - "2026-01-25-00y-abc.1.2" → true (nested member)
/// - "2026-01-25-00y-abc" → false (driver, not member)
fn is_part_of_group(spec_id: &str) -> bool {
    // A spec is part of a group if its ID contains a dot followed by a number
    // We need to check if there's a pattern like ".N" where N is a digit

    // Split by dots and check if there's at least one numeric segment after the base ID
    let parts: Vec<&str> = spec_id.split('.').collect();

    // If there's more than one part and any part after the first contains only digits,
    // this is a group member
    if parts.len() > 1 {
        for part in &parts[1..] {
            if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::SpecFrontmatter;

    #[test]
    fn test_grade_a_multiple_headers_and_files() {
        // 4 headers, 5 files, 8 criteria → Grade A
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                    "file4.rs".to_string(),
                    "file5.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Section 1
- [ ] Criterion 1
- [ ] Criterion 2

## Section 2
- [ ] Criterion 3
- [ ] Criterion 4

## Section 3
- [ ] Criterion 5
- [ ] Criterion 6

## Section 4
- [ ] Criterion 7
- [ ] Criterion 8
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::A);
    }

    #[test]
    fn test_grade_b_some_structure() {
        // 1 header, 2 files, 3 criteria → Grade B
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string(), "file2.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::B);
    }

    #[test]
    fn test_grade_c_single_concern() {
        // 0 headers, 1 file, 1 criterion → Grade C
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
- [ ] Single criterion
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::C);
    }

    #[test]
    fn test_grade_d_coupling_keywords() {
        // Spec mentioning "tightly coupled components" → Grade D
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Section 1
- [ ] Criterion 1

## Section 2
- [ ] Criterion 2

These components are tightly coupled and cannot be separated.
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::D);
    }

    #[test]
    fn test_edge_case_group_member() {
        // Spec with ID indicating group membership → Grade C
        let spec = Spec {
            id: "2026-01-25-00y-abc.1".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Section 1
- [ ] Criterion 1
- [ ] Criterion 2

## Section 2
- [ ] Criterion 3
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::C);
    }

    #[test]
    fn test_edge_case_single_criterion_atomic() {
        // Spec with only 1 criterion → Grade C (atomic)
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Section 1
- [ ] Single criterion
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::C);
    }

    #[test]
    fn test_count_markdown_headers() {
        let body = r#"
# Title (not counted)

## Section 1
Some content

## Section 2
More content

### Subsection
Even more

```rust
// ## This header in code is not counted
## Neither is this
```

## Section 3
Final section
"#;

        assert_eq!(count_markdown_headers(body), 4); // Sections 1, 2, Subsection, Section 3
    }

    #[test]
    fn test_count_target_files() {
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: String::new(),
        };

        assert_eq!(count_target_files(&spec), 3);
    }

    #[test]
    fn test_count_target_files_none() {
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: None,
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: String::new(),
        };

        assert_eq!(count_target_files(&spec), 0);
    }

    #[test]
    fn test_has_coupling_keywords_shared() {
        let body = "This code uses shared state between components.";
        assert!(has_coupling_keywords(body));
    }

    #[test]
    fn test_has_coupling_keywords_depends_on_each_other() {
        let body = "These modules depends on each other heavily.";
        assert!(has_coupling_keywords(body));
    }

    #[test]
    fn test_has_coupling_keywords_tightly_coupled() {
        let body = "The components are TIGHTLY COUPLED.";
        assert!(has_coupling_keywords(body));
    }

    #[test]
    fn test_has_coupling_keywords_none() {
        let body = "This is a simple independent module.";
        assert!(!has_coupling_keywords(body));
    }

    #[test]
    fn test_is_part_of_group_member() {
        assert!(is_part_of_group("2026-01-25-00y-abc.1"));
        assert!(is_part_of_group("2026-01-25-00y-abc.2"));
        assert!(is_part_of_group("2026-01-25-00y-abc.1.2"));
    }

    #[test]
    fn test_is_part_of_group_driver() {
        assert!(!is_part_of_group("2026-01-25-00y-abc"));
    }

    #[test]
    fn test_is_part_of_group_edge_cases() {
        // Edge case: dot but not numeric
        assert!(!is_part_of_group("2026-01-25-00y-abc.md"));

        // Edge case: multiple dots with numbers
        assert!(is_part_of_group("2026-01-25-00y-abc.1.2.3"));
    }

    #[test]
    fn test_grade_b_two_headers() {
        // 2 headers, 2 files → Grade B
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string(), "file2.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Section 1
- [ ] Criterion 1

## Section 2
- [ ] Criterion 2
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::B);
    }

    #[test]
    fn test_grade_c_no_structure() {
        // 0 headers, 2 files, 3 criteria → Grade C (default)
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string(), "file2.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::C);
    }

    #[test]
    fn test_coupling_overrides_good_structure() {
        // Even with 4 headers and 5 files, coupling keywords force Grade D
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                    "file4.rs".to_string(),
                    "file5.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: r#"
## Section 1
- [ ] Criterion 1

## Section 2
- [ ] Criterion 2

## Section 3
- [ ] Criterion 3

## Section 4
- [ ] Criterion 4

Note: These components have shared state.
"#
            .to_string(),
        };

        assert_eq!(calculate_splittability(&spec), SplittabilityGrade::D);
    }
}
