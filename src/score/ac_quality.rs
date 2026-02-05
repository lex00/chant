//! Acceptance criteria quality scoring.
//!
//! Simple scoring based on whether the spec has acceptance criteria checkboxes.
//! The presence of checkboxes indicates testable requirements.

use crate::scoring::ACQualityGrade;

/// Calculate acceptance criteria quality grade.
///
/// Simple grading based on number of criteria:
/// - Grade A: 4+ criteria (well-defined, multiple verification points)
/// - Grade B: 2-3 criteria (adequate coverage)
/// - Grade C: 1 criterion (minimal but present)
/// - Grade D: 0 criteria (no acceptance criteria)
///
/// # Arguments
///
/// * `criteria` - Slice of acceptance criteria strings
///
/// # Returns
///
/// An `ACQualityGrade` based on criteria count
pub fn calculate_ac_quality(criteria: &[String]) -> ACQualityGrade {
    match criteria.len() {
        0 => ACQualityGrade::D,
        1 => ACQualityGrade::C,
        2 | 3 => ACQualityGrade::B,
        _ => ACQualityGrade::A,
    }
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
    fn test_single_criterion_returns_grade_c() {
        let criteria = vec!["File exists".to_string()];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::C);
    }

    #[test]
    fn test_two_criteria_returns_grade_b() {
        let criteria = vec![
            "File exists".to_string(),
            "Output matches expected".to_string(),
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::B);
    }

    #[test]
    fn test_three_criteria_returns_grade_b() {
        let criteria = vec![
            "File exists".to_string(),
            "Output matches expected".to_string(),
            "No errors in log".to_string(),
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::B);
    }

    #[test]
    fn test_four_or_more_criteria_returns_grade_a() {
        let criteria = vec![
            "File exists".to_string(),
            "Output matches expected".to_string(),
            "No errors in log".to_string(),
            "Performance under 100ms".to_string(),
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::A);
    }

    #[test]
    fn test_many_criteria_returns_grade_a() {
        let criteria = vec![
            "One".to_string(),
            "Two".to_string(),
            "Three".to_string(),
            "Four".to_string(),
            "Five".to_string(),
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::A);
    }

    #[test]
    fn test_any_content_counts() {
        // Content doesn't matter, only count
        let criteria = vec![
            "hello.sh file exists".to_string(), // declarative - should work
        ];
        assert_eq!(calculate_ac_quality(&criteria), ACQualityGrade::C);
    }
}
