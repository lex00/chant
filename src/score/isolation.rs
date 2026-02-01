//! Isolation scoring for group specs to measure member independence.
//!
//! Analyzes whether a group's members are well-isolated from each other by examining:
//! - Cross-references between members in body text
//! - Shared files across multiple members' target_files
//!
//! Only applies to group specs with members.

use crate::scoring::IsolationGrade;
use crate::spec::Spec;
use crate::spec_group::get_members;
use regex::Regex;
use std::collections::HashSet;

/// Calculate isolation grade for group specs to measure member independence.
///
/// Grading rules:
/// - Grade A: >90% isolation, minimal shared files (<20% overlap)
/// - Grade B: >70% isolation
/// - Grade C: >50% isolation
/// - Grade D: ≤50% isolation OR >50% file overlap
///
/// Edge cases:
/// - Returns None for non-group specs (specs without members)
/// - Groups with 1 member return Grade A (trivially isolated)
/// - Cross-references detected by "Member N" patterns in member body text
/// - File overlap calculated as files appearing in multiple members' target_files
///
/// # Arguments
///
/// * `spec` - The group spec to analyze
/// * `all_specs` - All available specs (to look up members)
///
/// # Returns
///
/// * `Some(IsolationGrade)` - For group specs with members
/// * `None` - For non-group specs or groups without members
///
/// # Examples
///
/// ```ignore
/// // Group with 5 members, 0 cross-references, no shared files → Grade A
/// let grade = calculate_isolation(&driver_spec, &all_specs);
/// assert_eq!(grade, Some(IsolationGrade::A));
///
/// // Group with 6 members, 2 with cross-refs, 1 shared file → Grade B (67% isolation)
/// let grade = calculate_isolation(&driver_spec, &all_specs);
/// assert_eq!(grade, Some(IsolationGrade::B));
///
/// // Group with 4 members, 3 with cross-refs → Grade D (25% isolation)
/// let grade = calculate_isolation(&driver_spec, &all_specs);
/// assert_eq!(grade, Some(IsolationGrade::D));
/// ```
pub fn calculate_isolation(spec: &Spec, all_specs: &[Spec]) -> Option<IsolationGrade> {
    // Get all members of this spec
    let members = get_members(&spec.id, all_specs);

    // Return None if this is not a group spec or has no members
    if members.is_empty() {
        return None;
    }

    // Edge case: Groups with 1 member are trivially isolated
    if members.len() == 1 {
        return Some(IsolationGrade::A);
    }

    // Count members with cross-references
    let members_with_cross_refs = count_members_with_cross_references(&members);

    // Calculate isolation percentage
    let isolation_percentage =
        ((members.len() - members_with_cross_refs) as f64 / members.len() as f64) * 100.0;

    // Calculate file overlap percentage
    let file_overlap_percentage = calculate_file_overlap_percentage(&members);

    // Apply grading logic
    // Grade D: ≤50% isolation OR >50% file overlap
    if isolation_percentage <= 50.0 || file_overlap_percentage > 50.0 {
        return Some(IsolationGrade::D);
    }

    // Grade A: >90% isolation, minimal shared files (<20% overlap)
    if isolation_percentage > 90.0 && file_overlap_percentage < 20.0 {
        return Some(IsolationGrade::A);
    }

    // Grade B: >70% isolation
    if isolation_percentage > 70.0 {
        return Some(IsolationGrade::B);
    }

    // Grade C: >50% isolation (default for remaining cases)
    Some(IsolationGrade::C)
}

/// Count how many members have cross-references to other members in their body text.
///
/// Detects patterns like "Member N", "Member 1", "member 2", etc. in the body text.
fn count_members_with_cross_references(members: &[&Spec]) -> usize {
    // Regex to match "Member N" patterns (case-insensitive)
    let member_pattern = Regex::new(r"(?i)\bmember\s+\d+\b").unwrap();

    members
        .iter()
        .filter(|member| member_pattern.is_match(&member.body))
        .count()
}

/// Calculate the percentage of files that appear in multiple members' target_files.
///
/// Returns a percentage from 0.0 to 100.0.
/// If there are no target files, returns 0.0 (no overlap).
fn calculate_file_overlap_percentage(members: &[&Spec]) -> f64 {
    // Collect all files and count how many members reference each file
    let mut file_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for member in members {
        if let Some(target_files) = &member.frontmatter.target_files {
            // Use a HashSet to avoid counting the same file twice in one member
            let unique_files: HashSet<_> = target_files.iter().collect();
            for file in unique_files {
                *file_counts.entry(file.clone()).or_insert(0) += 1;
            }
        }
    }

    // If no files at all, no overlap
    if file_counts.is_empty() {
        return 0.0;
    }

    // Count how many files appear in more than one member
    let shared_files = file_counts.values().filter(|&&count| count > 1).count();

    // Calculate percentage
    (shared_files as f64 / file_counts.len() as f64) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::SpecFrontmatter;

    fn make_spec(id: &str, body: &str, target_files: Option<Vec<String>>) -> Spec {
        Spec {
            id: id.to_string(),
            frontmatter: SpecFrontmatter {
                target_files,
                ..Default::default()
            },
            title: Some(format!("Test spec {}", id)),
            body: body.to_string(),
        }
    }

    #[test]
    fn test_non_group_returns_none() {
        // A spec without members should return None
        let driver = make_spec("2026-01-30-abc", "Driver spec body", None);
        let all_specs = vec![driver.clone()];

        assert_eq!(calculate_isolation(&driver, &all_specs), None);
    }

    #[test]
    fn test_single_member_returns_grade_a() {
        // Groups with 1 member are trivially isolated
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec("2026-01-30-abc.1", "Member 1 body", None);
        let all_specs = vec![driver.clone(), member1];

        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::A)
        );
    }

    #[test]
    fn test_grade_a_perfect_isolation() {
        // 5 members, 0 cross-references, no shared files → Grade A
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec(
            "2026-01-30-abc.1",
            "Implement feature A",
            Some(vec!["file1.rs".to_string()]),
        );
        let member2 = make_spec(
            "2026-01-30-abc.2",
            "Implement feature B",
            Some(vec!["file2.rs".to_string()]),
        );
        let member3 = make_spec(
            "2026-01-30-abc.3",
            "Implement feature C",
            Some(vec!["file3.rs".to_string()]),
        );
        let member4 = make_spec(
            "2026-01-30-abc.4",
            "Implement feature D",
            Some(vec!["file4.rs".to_string()]),
        );
        let member5 = make_spec(
            "2026-01-30-abc.5",
            "Implement feature E",
            Some(vec!["file5.rs".to_string()]),
        );

        let all_specs = vec![driver.clone(), member1, member2, member3, member4, member5];

        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::A)
        );
    }

    #[test]
    fn test_grade_b_good_isolation() {
        // 6 members, 1 with cross-refs, 1 shared file → Grade B (83% isolation)
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec(
            "2026-01-30-abc.1",
            "Implement feature A. See Member 2 for details.",
            Some(vec!["file1.rs".to_string()]),
        );
        let member2 = make_spec(
            "2026-01-30-abc.2",
            "Implement feature B independently.",
            Some(vec!["file2.rs".to_string()]),
        );
        let member3 = make_spec(
            "2026-01-30-abc.3",
            "Implement feature C",
            Some(vec!["file3.rs".to_string()]),
        );
        let member4 = make_spec(
            "2026-01-30-abc.4",
            "Implement feature D",
            Some(vec!["file4.rs".to_string()]),
        );
        let member5 = make_spec(
            "2026-01-30-abc.5",
            "Implement feature E",
            Some(vec!["file5.rs".to_string()]),
        );
        let member6 = make_spec(
            "2026-01-30-abc.6",
            "Implement feature F",
            Some(vec!["file1.rs".to_string()]), // Shared with member1
        );

        let all_specs = vec![
            driver.clone(),
            member1,
            member2,
            member3,
            member4,
            member5,
            member6,
        ];

        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::B)
        );
    }

    #[test]
    fn test_grade_d_low_isolation() {
        // 4 members, 3 with cross-refs → Grade D (25% isolation)
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec(
            "2026-01-30-abc.1",
            "Implement feature A. See Member 2.",
            Some(vec!["file1.rs".to_string()]),
        );
        let member2 = make_spec(
            "2026-01-30-abc.2",
            "Implement feature B. Depends on Member 1 and Member 3.",
            Some(vec!["file2.rs".to_string()]),
        );
        let member3 = make_spec(
            "2026-01-30-abc.3",
            "Implement feature C. Uses Member 2.",
            Some(vec!["file3.rs".to_string()]),
        );
        let member4 = make_spec(
            "2026-01-30-abc.4",
            "Implement feature D",
            Some(vec!["file4.rs".to_string()]),
        );

        let all_specs = vec![driver.clone(), member1, member2, member3, member4];

        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::D)
        );
    }

    #[test]
    fn test_grade_d_high_file_overlap() {
        // 3 members, 0 cross-refs, but >50% file overlap → Grade D
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec(
            "2026-01-30-abc.1",
            "Implement feature A",
            Some(vec!["shared1.rs".to_string(), "shared2.rs".to_string()]),
        );
        let member2 = make_spec(
            "2026-01-30-abc.2",
            "Implement feature B",
            Some(vec!["shared1.rs".to_string(), "shared2.rs".to_string()]),
        );
        let member3 = make_spec(
            "2026-01-30-abc.3",
            "Implement feature C",
            Some(vec!["file3.rs".to_string()]),
        );

        let all_specs = vec![driver.clone(), member1, member2, member3];

        // 2 out of 3 files are shared = 66% overlap → Grade D
        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::D)
        );
    }

    #[test]
    fn test_cross_reference_detection_case_insensitive() {
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec(
            "2026-01-30-abc.1",
            "See MEMBER 2 for details. Also check member 3.",
            None,
        );
        let member2 = make_spec("2026-01-30-abc.2", "Independent work", None);
        let member3 = make_spec("2026-01-30-abc.3", "Independent work", None);

        let all_specs = vec![driver.clone(), member1, member2, member3];

        // Member 1 has cross-references, 2 and 3 don't
        // 2 out of 3 isolated = 67% → Grade C
        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::C)
        );
    }

    #[test]
    fn test_no_target_files_no_overlap() {
        // Members without target_files should have 0% overlap
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec("2026-01-30-abc.1", "Feature A", None);
        let member2 = make_spec("2026-01-30-abc.2", "Feature B", None);
        let member3 = make_spec("2026-01-30-abc.3", "Feature C", None);
        let member4 = make_spec("2026-01-30-abc.4", "Feature D", None);
        let member5 = make_spec("2026-01-30-abc.5", "Feature E", None);

        let all_specs = vec![driver.clone(), member1, member2, member3, member4, member5];

        // 5 members, 0 cross-refs, no files = >90% isolation + <20% overlap → Grade A
        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::A)
        );
    }

    #[test]
    fn test_grade_c_medium_isolation() {
        // 5 members, 2 with cross-refs → 60% isolation → Grade C
        let driver = make_spec("2026-01-30-abc", "Driver spec", None);
        let member1 = make_spec(
            "2026-01-30-abc.1",
            "See Member 2",
            Some(vec!["file1.rs".to_string()]),
        );
        let member2 = make_spec(
            "2026-01-30-abc.2",
            "Depends on Member 1",
            Some(vec!["file2.rs".to_string()]),
        );
        let member3 = make_spec(
            "2026-01-30-abc.3",
            "Independent",
            Some(vec!["file3.rs".to_string()]),
        );
        let member4 = make_spec(
            "2026-01-30-abc.4",
            "Independent",
            Some(vec!["file4.rs".to_string()]),
        );
        let member5 = make_spec(
            "2026-01-30-abc.5",
            "Independent",
            Some(vec!["file5.rs".to_string()]),
        );

        let all_specs = vec![driver.clone(), member1, member2, member3, member4, member5];

        assert_eq!(
            calculate_isolation(&driver, &all_specs),
            Some(IsolationGrade::C)
        );
    }
}
