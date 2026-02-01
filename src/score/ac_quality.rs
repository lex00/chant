//! Acceptance criteria quality scoring.
//!
//! Analyzes individual acceptance criteria based on:
//! - Phrasing: Uses imperative verbs (implement, add, create, etc.)
//! - Value: Addresses real requirements (not meta/admin tasks)
//! - Testability: Concrete and measurable (can verify completion)

use crate::scoring::ACQualityGrade;

/// List of imperative verbs that indicate clear, actionable criteria
const IMPERATIVE_VERBS: &[&str] = &[
    "implement",
    "add",
    "create",
    "update",
    "fix",
    "remove",
    "delete",
    "refactor",
    "test",
    "verify",
    "ensure",
    "validate",
    "configure",
    "setup",
    "install",
    "deploy",
    "build",
    "run",
    "execute",
    "check",
    "document",
    "write",
    "read",
    "parse",
    "handle",
    "process",
    "calculate",
    "compute",
    "convert",
    "transform",
    "migrate",
    "upgrade",
    "downgrade",
];

/// List of meta/admin phrases that indicate low-value criteria
const META_PHRASES: &[&str] = &[
    "update spec",
    "add comment",
    "update comment",
    "add documentation",
    "update documentation",
    "add todo",
    "update readme",
];

/// List of vague verbs that indicate untestable criteria
const VAGUE_VERBS: &[&str] = &[
    "understand",
    "consider",
    "improve",
    "enhance",
    "optimize",
    "investigate",
    "explore",
    "research",
    "think about",
    "look into",
];

/// Calculate acceptance criteria quality grade.
///
/// Scores each criterion on three dimensions:
/// - Phrasing: Starts with an imperative verb
/// - Value: Not a meta/admin task
/// - Testability: Concrete and measurable (no vague verbs)
///
/// A criterion passes if it meets all three checks.
///
/// Grading rules:
/// - Grade A: >90% criteria pass all three checks
/// - Grade B: >70% criteria pass all three checks
/// - Grade C: >50% criteria pass all three checks
/// - Grade D: ≤50% criteria pass
///
/// Edge cases:
/// - Empty criteria list returns Grade D
/// - Single criterion that passes returns Grade A
///
/// # Arguments
///
/// * `criteria` - Slice of acceptance criteria strings to analyze
///
/// # Returns
///
/// An `ACQualityGrade` based on the percentage of criteria that pass all checks
pub fn calculate_ac_quality(criteria: &[String]) -> ACQualityGrade {
    // Edge case: empty criteria list
    if criteria.is_empty() {
        return ACQualityGrade::D;
    }

    // Count criteria that pass all three checks
    let passing_count = criteria
        .iter()
        .filter(|criterion| {
            let has_imperative = has_imperative_verb(criterion);
            let has_value = !is_meta_task(criterion);
            let is_testable = !has_vague_verb(criterion);

            has_imperative && has_value && is_testable
        })
        .count();

    // Calculate pass ratio
    let pass_ratio = passing_count as f64 / criteria.len() as f64;

    // Apply grading rules
    if pass_ratio > 0.90 {
        ACQualityGrade::A
    } else if pass_ratio > 0.70 {
        ACQualityGrade::B
    } else if pass_ratio > 0.50 {
        ACQualityGrade::C
    } else {
        ACQualityGrade::D
    }
}

/// Check if a criterion starts with an imperative verb.
///
/// Extracts the first word and checks if it matches a known imperative verb.
fn has_imperative_verb(criterion: &str) -> bool {
    let first_word = criterion.split_whitespace().next().unwrap_or("");

    IMPERATIVE_VERBS.contains(&first_word.to_lowercase().as_str())
}

/// Check if a criterion is a meta/admin task (low value).
///
/// Looks for common meta phrases that indicate the criterion is about
/// maintaining the spec itself rather than delivering actual functionality.
fn is_meta_task(criterion: &str) -> bool {
    let criterion_lower = criterion.to_lowercase();

    META_PHRASES
        .iter()
        .any(|phrase| criterion_lower.contains(phrase))
}

/// Check if a criterion uses vague verbs (untestable).
///
/// Looks for verbs that indicate unclear or unmeasurable requirements.
fn has_vague_verb(criterion: &str) -> bool {
    let criterion_lower = criterion.to_lowercase();

    VAGUE_VERBS.iter().any(|verb| {
        // Check if the vague verb appears as the first word or within the criterion
        let words: Vec<&str> = criterion_lower.split_whitespace().collect();
        (words.first() == Some(verb))
            || criterion_lower.contains(&format!(" {} ", verb))
            || criterion_lower.starts_with(&format!("{} ", verb))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_criteria_returns_grade_d() {
        let criteria: Vec<String> = vec![];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::D);
    }

    #[test]
    fn test_single_passing_criterion_returns_grade_a() {
        let criteria = vec!["Implement function X".to_string()];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::A);
    }

    #[test]
    fn test_all_passing_criteria_grade_a() {
        let criteria = vec![
            "Implement function X".to_string(),
            "Add test for Y".to_string(),
            "Verify Z works".to_string(),
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::A);
    }

    #[test]
    fn test_all_failing_criteria_grade_d() {
        let criteria = vec![
            "Update spec".to_string(),
            "Consider edge cases".to_string(),
            "Improve code".to_string(),
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::D);
    }

    #[test]
    fn test_mixed_criteria_grade_b() {
        // 1 passes, 1 is meta → 50% → Grade C
        // Actually need >70% for B, so let's have 2 pass out of 3
        let criteria = vec![
            "Create endpoint /api/foo".to_string(),
            "Add test coverage".to_string(),
            "Update README".to_string(), // meta task
        ];
        // 2/3 = 66.7% → Grade C (need >70% for B)
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::C);
    }

    #[test]
    fn test_grade_b_boundary() {
        // Need >70%, so 8 out of 10 = 80%
        let criteria = vec![
            "Implement feature A".to_string(),
            "Add feature B".to_string(),
            "Create feature C".to_string(),
            "Build feature D".to_string(),
            "Deploy feature E".to_string(),
            "Test feature F".to_string(),
            "Verify feature G".to_string(),
            "Handle feature H".to_string(),
            "Update spec".to_string(),       // meta
            "Improve something".to_string(), // vague
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::B);
    }

    #[test]
    fn test_has_imperative_verb() {
        assert!(has_imperative_verb("Implement feature X"));
        assert!(has_imperative_verb("add functionality"));
        assert!(has_imperative_verb("CREATE component"));
        assert!(!has_imperative_verb("This does not start with verb"));
        assert!(!has_imperative_verb("Something else"));
    }

    #[test]
    fn test_is_meta_task() {
        assert!(is_meta_task("Update spec with new info"));
        assert!(is_meta_task("Add comment to file"));
        assert!(is_meta_task("Update README"));
        assert!(!is_meta_task("Implement feature X"));
        assert!(!is_meta_task("Create new component"));
    }

    #[test]
    fn test_has_vague_verb() {
        assert!(has_vague_verb("Understand the codebase"));
        assert!(has_vague_verb("Consider edge cases"));
        assert!(has_vague_verb("Improve performance"));
        assert!(has_vague_verb("We should consider this"));
        assert!(!has_vague_verb("Implement feature X"));
        assert!(!has_vague_verb("Add test for Y"));
    }

    #[test]
    fn test_vague_verbs_fail_phrasing() {
        // "Improve" and "enhance" are vague but not imperative verbs
        let criteria = vec![
            "Improve code quality".to_string(),
            "Enhance performance".to_string(),
        ];
        // These fail testability (vague verbs) even though "improve" isn't in imperative list
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::D);
    }

    #[test]
    fn test_case_insensitive_matching() {
        assert!(has_imperative_verb("IMPLEMENT feature"));
        assert!(has_imperative_verb("Add Feature"));
        assert!(has_imperative_verb("CrEaTe component"));
    }

    #[test]
    fn test_grade_c_boundary() {
        // >50% but ≤70% → Grade C
        // 6 out of 10 = 60%
        let criteria = vec![
            "Implement feature A".to_string(),
            "Add feature B".to_string(),
            "Create feature C".to_string(),
            "Build feature D".to_string(),
            "Deploy feature E".to_string(),
            "Test feature F".to_string(),
            "Update spec".to_string(),          // meta
            "Consider this".to_string(),        // vague
            "Improve that".to_string(),         // vague
            "Update documentation".to_string(), // meta
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::C);
    }

    #[test]
    fn test_exactly_50_percent_is_grade_d() {
        // Exactly 50% should be Grade D (need >50% for C)
        let criteria = vec![
            "Implement feature A".to_string(),
            "Add feature B".to_string(),
            "Update spec".to_string(),  // meta
            "Improve code".to_string(), // vague
        ];
        // 2/4 = 50% → Grade D
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::D);
    }

    #[test]
    fn test_single_failing_criterion_returns_grade_d() {
        let criteria = vec!["Update spec".to_string()];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::D);
    }
}
