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
