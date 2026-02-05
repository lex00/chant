//! Spec quality scoring system
//!
//! Multi-dimensional analysis of spec quality including complexity, confidence,
//! splittability, isolation, and acceptance criteria quality.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Macro to generate Display implementations for letter grade enums (A, B, C, D)
macro_rules! impl_letter_grade_display {
    ($type:ty) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::A => write!(f, "A"),
                    Self::B => write!(f, "B"),
                    Self::C => write!(f, "C"),
                    Self::D => write!(f, "D"),
                }
            }
        }
    };
}

/// Overall score for a spec across all dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecScore {
    /// Complexity grade (size/effort)
    pub complexity: ComplexityGrade,
    /// Confidence grade (structure/clarity)
    pub confidence: ConfidenceGrade,
    /// Splittability grade (decomposability)
    pub splittability: SplittabilityGrade,
    /// Isolation grade (group/split quality) - only for groups with members
    pub isolation: Option<IsolationGrade>,
    /// Acceptance criteria quality grade
    pub ac_quality: ACQualityGrade,
    /// Overall traffic light status
    pub traffic_light: TrafficLight,
}

impl Default for SpecScore {
    fn default() -> Self {
        Self {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Ready,
        }
    }
}

/// Complexity grade based on criteria count, target files, and word count
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityGrade {
    /// 1-3 criteria, 1-2 files, <200 words
    A,
    /// 4-5 criteria, 3 files, 200-400 words
    B,
    /// 6-7 criteria, 4 files, 400-600 words
    C,
    /// 8+ criteria, 5+ files, 600+ words
    D,
}

impl_letter_grade_display!(ComplexityGrade);

/// Confidence grade based on structure and clarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceGrade {
    /// Excellent structure, clear requirements
    A,
    /// Good structure, mostly clear
    B,
    /// Some structure issues or vague language
    C,
    /// Poor structure, vague requirements
    D,
}

impl_letter_grade_display!(ConfidenceGrade);

/// Splittability grade - can this spec be effectively split
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplittabilityGrade {
    /// Clear subsections, independent tasks, multiple target files
    A,
    /// Some structure, could be split with effort
    B,
    /// Monolithic, single concern, splitting would fragment
    C,
    /// Tightly coupled, splitting would create circular deps
    D,
}

impl_letter_grade_display!(SplittabilityGrade);

/// Isolation grade - for groups with members, measures independence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationGrade {
    /// Excellent isolation, minimal cross-references
    A,
    /// Good isolation, some cross-references
    B,
    /// Some coupling, multiple cross-references
    C,
    /// Tightly coupled, many cross-references
    D,
}

impl_letter_grade_display!(IsolationGrade);

/// Acceptance criteria quality grade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ACQualityGrade {
    /// Excellent AC: imperative, valuable, testable
    A,
    /// Good AC: mostly well-phrased
    B,
    /// Some AC issues
    C,
    /// Poor AC quality
    D,
}

impl_letter_grade_display!(ACQualityGrade);

/// Overall traffic light status combining all dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficLight {
    /// Ready - All dimensions pass (Complexity ≤ B AND Confidence ≥ B AND AC Quality ≥ B)
    Ready,
    /// Review - Some dimensions need attention (Any dimension is C)
    Review,
    /// Refine - Significant issues (Any dimension is D OR Confidence is D)
    Refine,
}

impl fmt::Display for TrafficLight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => write!(f, "🟢 Ready"),
            Self::Review => write!(f, "🟡 Review"),
            Self::Refine => write!(f, "🔴 Refine"),
        }
    }
}

/// Calculate complexity grade based on criteria count, target files, and word count
///
/// Grading rules:
/// - Grade A: 1-3 criteria, 1-2 files, <200 words
/// - Grade B: 4-5 criteria, 3 files, 200-400 words
/// - Grade C: 6-7 criteria, 4 files, 400-600 words
/// - Grade D: 8+ criteria OR 5+ files OR 600+ words
///
/// If any single metric triggers D, overall grade is D.
pub fn calculate_complexity(spec: &crate::spec::Spec) -> ComplexityGrade {
    // Count acceptance criteria
    let criteria_count = spec.count_total_checkboxes();

    // Count target files (default to 0 if None)
    let file_count = spec
        .frontmatter
        .target_files
        .as_ref()
        .map(|files| files.len())
        .unwrap_or(0);

    // Count words in body (split by whitespace, filter empty)
    let word_count = spec.body.split_whitespace().count();

    // Determine grade based on all three metrics
    // If any single metric triggers D, overall is D
    if criteria_count >= 8 || file_count >= 5 || word_count >= 600 {
        return ComplexityGrade::D;
    }

    // Check for Grade C thresholds
    if criteria_count >= 6 || file_count >= 4 || word_count >= 400 {
        return ComplexityGrade::C;
    }

    // Check for Grade B thresholds
    if criteria_count >= 4 || file_count >= 3 || word_count >= 200 {
        return ComplexityGrade::B;
    }

    // Otherwise Grade A
    ComplexityGrade::A
}

/// Extract acceptance criteria from a spec's body
///
/// Looks for checkboxes anywhere in the spec body, not just under
/// a specific header. This matches the behavior of count_total_checkboxes().
fn extract_acceptance_criteria(spec: &crate::spec::Spec) -> Vec<String> {
    let mut criteria = Vec::new();
    let mut in_code_fence = false;

    for line in spec.body.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        // Skip content in code fences
        if in_code_fence {
            continue;
        }

        // Extract checkbox items (case insensitive for [x]/[X])
        if trimmed.starts_with("- [ ]")
            || trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]")
        {
            // Extract text after checkbox
            let text = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim_start_matches("- [X]")
                .trim()
                .to_string();
            if !text.is_empty() {
                criteria.push(text);
            }
        }
    }

    criteria
}

/// Calculate the overall SpecScore for a given spec
///
/// This function computes all scoring dimensions and determines the traffic light status.
pub fn calculate_spec_score(
    spec: &crate::spec::Spec,
    all_specs: &[crate::spec::Spec],
    config: &crate::config::Config,
) -> SpecScore {
    use crate::score::{ac_quality, confidence, isolation, splittability, traffic_light};

    // Calculate each dimension
    let complexity = calculate_complexity(spec);
    let confidence_grade = confidence::calculate_confidence(spec, config);
    let splittability_grade = splittability::calculate_splittability(spec);
    let isolation_grade = isolation::calculate_isolation(spec, all_specs);

    // Calculate AC quality from the spec's acceptance criteria
    let criteria = extract_acceptance_criteria(spec);
    let ac_quality_grade = ac_quality::calculate_ac_quality(&criteria);

    // Create the score struct
    let mut score = SpecScore {
        complexity,
        confidence: confidence_grade,
        splittability: splittability_grade,
        isolation: isolation_grade,
        ac_quality: ac_quality_grade,
        traffic_light: TrafficLight::Ready, // Temporary, will be recalculated
    };

    // Determine the traffic light status
    score.traffic_light = traffic_light::determine_status(&score);

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_light_display() {
        assert_eq!(TrafficLight::Ready.to_string(), "🟢 Ready");
        assert_eq!(TrafficLight::Review.to_string(), "🟡 Review");
        assert_eq!(TrafficLight::Refine.to_string(), "🔴 Refine");
    }

    #[test]
    fn test_calculate_complexity_grade_a() {
        use crate::spec::{Spec, SpecFrontmatter};

        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: format!(
                "## Acceptance Criteria\n- [ ] First\n- [ ] Second\n\n{}",
                "word ".repeat(150)
            ),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::A);
    }

    #[test]
    fn test_calculate_complexity_grade_b() {
        use crate::spec::{Spec, SpecFrontmatter};

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
            body: format!(
                "## Acceptance Criteria\n- [ ] First\n- [ ] Second\n- [ ] Third\n- [ ] Fourth\n- [ ] Fifth\n\n{}",
                "word ".repeat(300)
            ),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::B);
    }

    #[test]
    fn test_calculate_complexity_grade_d_criteria() {
        use crate::spec::{Spec, SpecFrontmatter};

        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string(), "file2.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: format!(
                "## Acceptance Criteria\n{}\n\n{}",
                (1..=10)
                    .map(|i| format!("- [ ] Item {}", i))
                    .collect::<Vec<_>>()
                    .join("\n"),
                "word ".repeat(100)
            ),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::D);
    }

    #[test]
    fn test_calculate_complexity_no_target_files() {
        use crate::spec::{Spec, SpecFrontmatter};

        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: None,
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body:
                "## Acceptance Criteria\n- [ ] First\n- [ ] Second\n\nSome content here with words."
                    .to_string(),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::A);
    }
}
