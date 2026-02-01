//! Spec quality scoring system
//!
//! Multi-dimensional analysis of spec quality including complexity, confidence,
//! splittability, isolation, and acceptance criteria quality.

use serde::{Deserialize, Serialize};
use std::fmt;

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

impl fmt::Display for ComplexityGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
        }
    }
}

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

impl fmt::Display for ConfidenceGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
        }
    }
}

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

impl fmt::Display for SplittabilityGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
        }
    }
}

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

impl fmt::Display for IsolationGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
        }
    }
}

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

impl fmt::Display for ACQualityGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_score_creation_with_all_a() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: Some(IsolationGrade::A),
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Ready,
        };

        assert_eq!(score.complexity, ComplexityGrade::A);
        assert_eq!(score.confidence, ConfidenceGrade::A);
        assert_eq!(score.splittability, SplittabilityGrade::A);
        assert_eq!(score.isolation, Some(IsolationGrade::A));
        assert_eq!(score.ac_quality, ACQualityGrade::A);
        assert_eq!(score.traffic_light, TrafficLight::Ready);
    }

    #[test]
    fn test_complexity_grade_display() {
        assert_eq!(ComplexityGrade::B.to_string(), "B");
        assert_eq!(ComplexityGrade::A.to_string(), "A");
        assert_eq!(ComplexityGrade::C.to_string(), "C");
        assert_eq!(ComplexityGrade::D.to_string(), "D");
    }

    #[test]
    fn test_confidence_grade_display() {
        assert_eq!(ConfidenceGrade::B.to_string(), "B");
    }

    #[test]
    fn test_splittability_grade_display() {
        assert_eq!(SplittabilityGrade::B.to_string(), "B");
    }

    #[test]
    fn test_isolation_grade_display() {
        assert_eq!(IsolationGrade::B.to_string(), "B");
    }

    #[test]
    fn test_ac_quality_grade_display() {
        assert_eq!(ACQualityGrade::B.to_string(), "B");
    }

    #[test]
    fn test_traffic_light_display() {
        assert_eq!(TrafficLight::Ready.to_string(), "🟢 Ready");
        assert_eq!(TrafficLight::Review.to_string(), "🟡 Review");
        assert_eq!(TrafficLight::Refine.to_string(), "🔴 Refine");
    }

    #[test]
    fn test_spec_score_serialization() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::B,
            splittability: SplittabilityGrade::A,
            isolation: None,
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Ready,
        };

        let json = serde_json::to_string(&score).unwrap();
        let deserialized: SpecScore = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.complexity, ComplexityGrade::A);
        assert_eq!(deserialized.confidence, ConfidenceGrade::B);
        assert_eq!(deserialized.splittability, SplittabilityGrade::A);
        assert_eq!(deserialized.isolation, None);
        assert_eq!(deserialized.ac_quality, ACQualityGrade::A);
        assert_eq!(deserialized.traffic_light, TrafficLight::Ready);
    }

    #[test]
    fn test_isolation_is_optional() {
        let score = SpecScore {
            complexity: ComplexityGrade::A,
            confidence: ConfidenceGrade::A,
            splittability: SplittabilityGrade::A,
            isolation: None, // Should work fine without isolation
            ac_quality: ACQualityGrade::A,
            traffic_light: TrafficLight::Ready,
        };

        assert_eq!(score.isolation, None);
    }

    #[test]
    fn test_default_spec_score() {
        let score = SpecScore::default();
        assert_eq!(score.complexity, ComplexityGrade::A);
        assert_eq!(score.confidence, ConfidenceGrade::A);
        assert_eq!(score.splittability, SplittabilityGrade::A);
        assert_eq!(score.isolation, None);
        assert_eq!(score.ac_quality, ACQualityGrade::A);
        assert_eq!(score.traffic_light, TrafficLight::Ready);
    }

    #[test]
    fn test_calculate_complexity_grade_a() {
        use crate::spec::{Spec, SpecFrontmatter};

        // 2 criteria, 1 file, 150 words → Grade A
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

        // 5 criteria, 3 files, 300 words → Grade B
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
    fn test_calculate_complexity_grade_c() {
        use crate::spec::{Spec, SpecFrontmatter};

        // 6 criteria, 4 files, 500 words → Grade C
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec![
                    "file1.rs".to_string(),
                    "file2.rs".to_string(),
                    "file3.rs".to_string(),
                    "file4.rs".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: format!(
                "## Acceptance Criteria\n- [ ] First\n- [ ] Second\n- [ ] Third\n- [ ] Fourth\n- [ ] Fifth\n- [ ] Sixth\n\n{}",
                "word ".repeat(500)
            ),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::C);
    }

    #[test]
    fn test_calculate_complexity_grade_d_criteria() {
        use crate::spec::{Spec, SpecFrontmatter};

        // 10 criteria, 2 files, 100 words → Grade D (criteria exceeds threshold)
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
    fn test_calculate_complexity_grade_d_files() {
        use crate::spec::{Spec, SpecFrontmatter};

        // 2 criteria, 5 files, 100 words → Grade D (files exceeds threshold)
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
            body: format!(
                "## Acceptance Criteria\n- [ ] First\n- [ ] Second\n\n{}",
                "word ".repeat(100)
            ),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::D);
    }

    #[test]
    fn test_calculate_complexity_grade_d_words() {
        use crate::spec::{Spec, SpecFrontmatter};

        // 2 criteria, 1 file, 700 words → Grade D (words exceeds threshold)
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: format!(
                "## Acceptance Criteria\n- [ ] First\n- [ ] Second\n\n{}",
                "word ".repeat(700)
            ),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::D);
    }

    #[test]
    fn test_calculate_complexity_no_target_files() {
        use crate::spec::{Spec, SpecFrontmatter};

        // Specs with no target_files should default to 0 files
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

    #[test]
    fn test_calculate_complexity_empty_body() {
        use crate::spec::{Spec, SpecFrontmatter};

        // Empty body should have word count of 0
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Test".to_string()),
            body: String::new(),
        };

        assert_eq!(calculate_complexity(&spec), ComplexityGrade::A);
    }
}
