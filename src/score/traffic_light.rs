//! Traffic light status determination and suggestion generation.
//!
//! Combines dimension grades into an overall traffic light status and generates
//! actionable suggestions for improving spec quality.

use crate::scoring::{
    ACQualityGrade, ComplexityGrade, ConfidenceGrade, IsolationGrade, SpecScore,
    SplittabilityGrade, TrafficLight,
};

/// Determine overall traffic light status based on dimension grades
///
/// Traffic light logic:
/// - Ready (green): Complexity ≤ B AND Confidence ≥ B AND AC Quality ≥ B
/// - Refine (red): Any dimension is D OR Confidence is D
/// - Review (yellow): All other cases (any dimension is C)
///
/// Note: Isolation grade (optional) does not affect traffic light status
pub fn determine_status(score: &SpecScore) -> TrafficLight {
    // Check for Refine conditions: any dimension is D
    if matches!(score.complexity, ComplexityGrade::D)
        || matches!(score.confidence, ConfidenceGrade::D)
        || matches!(score.ac_quality, ACQualityGrade::D)
        || matches!(score.splittability, SplittabilityGrade::D)
        || score
            .isolation
            .is_some_and(|iso| matches!(iso, IsolationGrade::D))
    {
        return TrafficLight::Refine;
    }

    // Check for Ready conditions: Complexity ≤ B AND Confidence ≥ B AND AC Quality ≥ B
    let complexity_ok = matches!(score.complexity, ComplexityGrade::A | ComplexityGrade::B);
    let confidence_ok = matches!(score.confidence, ConfidenceGrade::A | ConfidenceGrade::B);
    let ac_quality_ok = matches!(score.ac_quality, ACQualityGrade::A | ACQualityGrade::B);

    if complexity_ok && confidence_ok && ac_quality_ok {
        return TrafficLight::Ready;
    }

    // All other cases: Review (any dimension is C)
    TrafficLight::Review
}

/// Generate actionable suggestions based on failing dimensions
///
/// Suggestions are specific to each dimension that needs improvement.
/// Multiple failing dimensions will generate multiple suggestions.
/// Suggestions are deduplicated to avoid repetition.
pub fn generate_suggestions(score: &SpecScore) -> Vec<String> {
    let mut suggestions = Vec::new();

    // Complexity suggestions
    match score.complexity {
        ComplexityGrade::D => {
            suggestions.push("Reduce criteria count or split spec into smaller pieces".to_string());
        }
        ComplexityGrade::C => {
            suggestions.push("Consider reducing scope or splitting into subtasks".to_string());
        }
        _ => {}
    }

    // Confidence suggestions
    match score.confidence {
        ConfidenceGrade::D => {
            suggestions.push("Improve spec structure and clarify vague requirements".to_string());
        }
        ConfidenceGrade::C => {
            suggestions.push("Add more specific details and improve organization".to_string());
        }
        _ => {}
    }

    // AC Quality suggestions
    match score.ac_quality {
        ACQualityGrade::D => {
            suggestions.push(
                "Rewrite acceptance criteria to be imperative, valuable, and testable".to_string(),
            );
        }
        ACQualityGrade::C => {
            suggestions.push("Improve acceptance criteria phrasing and specificity".to_string());
        }
        _ => {}
    }

    // Splittability suggestions
    match score.splittability {
        SplittabilityGrade::D => {
            suggestions
                .push("Refactor to reduce tight coupling and circular dependencies".to_string());
        }
        SplittabilityGrade::C => {
            suggestions.push("Consider breaking into more independent subsections".to_string());
        }
        _ => {}
    }

    // Isolation suggestions (optional field)
    if let Some(isolation) = score.isolation {
        match isolation {
            IsolationGrade::D => {
                suggestions.push(
                    "Reduce cross-references between group members to improve isolation"
                        .to_string(),
                );
            }
            IsolationGrade::C => {
                suggestions.push("Consider reducing coupling between group members".to_string());
            }
            _ => {}
        }
    }

    // Deduplicate suggestions (though our specific suggestions shouldn't duplicate)
    suggestions.sort();
    suggestions.dedup();

    suggestions
}

/// Generate detailed actionable guidance with examples for failing dimensions
///
/// Provides comprehensive, example-driven guidance on how to fix quality issues.
/// Returns a multi-line string with "Why This Matters" and "How to Fix" sections.
pub fn generate_detailed_guidance(score: &SpecScore) -> String {
    let mut output = String::new();

    // Only generate guidance if there are issues
    if matches!(score.traffic_light, TrafficLight::Ready) {
        return output;
    }

    output.push_str("\nWhy This Matters:\n");
    output.push_str(
        "  Agents perform best with ISOLATED tasks that have TESTABLE acceptance criteria.\n",
    );
    output.push_str("  Vague specs lead to scope creep, wrong assumptions, and wasted tokens.\n");
    output.push_str("\nHow to Fix:\n");

    // Confidence guidance
    if matches!(score.confidence, ConfidenceGrade::C | ConfidenceGrade::D) {
        let grade_letter = match score.confidence {
            ConfidenceGrade::D => "D",
            ConfidenceGrade::C => "C",
            _ => "",
        };
        output.push_str(&format!("\n  Confidence ({} → A):\n", grade_letter));
        output.push_str("    ✗ \"Update the API\"\n");
        output.push_str("    ✓ \"In src/api/users.rs, add `get_user_by_email()` method\"\n");
        output.push_str("    → Add specific file paths, function names, or line numbers\n");
    }

    // Splittability guidance
    if matches!(
        score.splittability,
        SplittabilityGrade::C | SplittabilityGrade::D
    ) {
        let grade_letter = match score.splittability {
            SplittabilityGrade::D => "D",
            SplittabilityGrade::C => "C",
            _ => "",
        };
        output.push_str(&format!("\n  Splittability ({} → A):\n", grade_letter));
        output.push_str("    ✗ \"Add auth and update docs and fix tests\"\n");
        output.push_str(
            "    ✓ Split into 3 specs: auth, docs, tests (use depends_on for ordering)\n",
        );
        output.push_str("    → Each spec should do ONE thing\n");
    }

    // AC Quality guidance
    if matches!(score.ac_quality, ACQualityGrade::C | ACQualityGrade::D) {
        let grade_letter = match score.ac_quality {
            ACQualityGrade::D => "D",
            ACQualityGrade::C => "C",
            _ => "",
        };
        output.push_str(&format!("\n  AC Quality ({} → A):\n", grade_letter));
        output.push_str("    ✗ \"- [ ] Code works correctly\"\n");
        output.push_str("    ✗ \"- [ ] Tests pass\"\n");
        output
            .push_str("    ✓ \"- [ ] Add `validate_email()` fn in src/utils.rs returning bool\"\n");
        output.push_str("    ✓ \"- [ ] `cargo test test_validate_email` passes\"\n");
        output.push_str(
            "    → Criteria must be: imperative verb + specific location + verifiable outcome\n",
        );
    }

    // Complexity guidance
    if matches!(score.complexity, ComplexityGrade::C | ComplexityGrade::D) {
        let grade_letter = match score.complexity {
            ComplexityGrade::D => "D",
            ComplexityGrade::C => "C",
            _ => "",
        };
        output.push_str(&format!("\n  Complexity ({} → B):\n", grade_letter));
        output.push_str("    → Split large specs into smaller, focused tasks\n");
        output.push_str("    → Aim for 1-5 acceptance criteria per spec\n");
        output.push_str("    → Use `chant split <spec-id>` to break into subtasks\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_status_all_a_ready() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: Some(IsolationGrade::A),
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Ready,
        };

        assert_eq!(determine_status(&score), TrafficLight::Ready);
    }

    #[test]
    fn test_determine_status_b_grades_ready() {
        let score = SpecScore {
            complexity: ComplexityGrade::B,
            confidence: ConfidenceGrade::B,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::B,
            traffic_light: TrafficLight::Ready,
        };

        assert_eq!(determine_status(&score), TrafficLight::Ready);
    }

    #[test]
    fn test_determine_status_complexity_c_review() {
        let score = SpecScore {
            complexity: ComplexityGrade::C,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Review,
        };

        assert_eq!(determine_status(&score), TrafficLight::Review);
    }

    #[test]
    fn test_determine_status_confidence_c_review() {
        let score = SpecScore {
            complexity: ComplexityGrade::B,
            confidence: ConfidenceGrade::C,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Review,
        };

        assert_eq!(determine_status(&score), TrafficLight::Review);
    }

    #[test]
    fn test_determine_status_ac_quality_c_review() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::B,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::C,
            traffic_light: TrafficLight::Review,
        };

        assert_eq!(determine_status(&score), TrafficLight::Review);
    }

    #[test]
    fn test_determine_status_complexity_d_refine() {
        let score = SpecScore {
            complexity: ComplexityGrade::D,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Refine,
        };

        assert_eq!(determine_status(&score), TrafficLight::Refine);
    }

    #[test]
    fn test_determine_status_confidence_d_refine() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::D,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Refine,
        };

        assert_eq!(determine_status(&score), TrafficLight::Refine);
    }

    #[test]
    fn test_determine_status_isolation_d_refine() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: Some(IsolationGrade::D),
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Refine,
        };

        assert_eq!(determine_status(&score), TrafficLight::Refine);
    }

    #[test]
    fn test_generate_suggestions_all_a_no_suggestions() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: Some(IsolationGrade::A),
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Ready,
        };

        let suggestions = generate_suggestions(&score);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_generate_suggestions_complexity_d() {
        let score = SpecScore {
            complexity: ComplexityGrade::D,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Refine,
        };

        let suggestions = generate_suggestions(&score);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].contains("Reduce criteria count"));
    }

    #[test]
    fn test_generate_suggestions_confidence_c() {
        let score = SpecScore {
            complexity: ComplexityGrade::B,
            confidence: ConfidenceGrade::C,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Review,
        };

        let suggestions = generate_suggestions(&score);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].contains("Add more specific details"));
    }

    #[test]
    fn test_generate_suggestions_multiple_dimensions() {
        let score = SpecScore {
            complexity: ComplexityGrade::D,
            confidence: ConfidenceGrade::C,
            splittability: SplittabilityGrade::C,
            isolation: Some(IsolationGrade::C),
            ac_quality: ACQualityGrade::D,
            traffic_light: TrafficLight::Refine,
        };

        let suggestions = generate_suggestions(&score);
        assert_eq!(suggestions.len(), 5);
        // Verify each dimension has a suggestion
        assert!(suggestions
            .iter()
            .any(|s| s.contains("Reduce criteria count")));
        assert!(suggestions
            .iter()
            .any(|s| s.contains("Add more specific details")));
        assert!(suggestions
            .iter()
            .any(|s| s.contains("breaking into more independent")));
        assert!(suggestions
            .iter()
            .any(|s| s.contains("reducing coupling between group")));
        assert!(suggestions.iter().any(|s| s.contains("imperative")));
    }

    #[test]
    fn test_generate_suggestions_no_duplicates() {
        let score = SpecScore {
            complexity: ComplexityGrade::C,
            confidence: ConfidenceGrade::C,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Review,
        };

        let suggestions = generate_suggestions(&score);
        // Check for uniqueness
        let unique_count = suggestions.len();
        let mut sorted = suggestions.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(unique_count, sorted.len());
    }

    #[test]
    fn test_generate_suggestions_isolation_none() {
        let score = SpecScore {
            complexity: ComplexityGrade::D,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Refine,
        };

        let suggestions = generate_suggestions(&score);
        // Should only have complexity suggestion, no isolation suggestion
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].contains("Reduce criteria count"));
    }
}
