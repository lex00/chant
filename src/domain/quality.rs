//! Quality assessment for specs.
//!
//! Pure functions for scoring spec quality across multiple dimensions.

use serde::{Deserialize, Serialize};

use crate::scoring::{ACQualityGrade, ComplexityGrade, ConfidenceGrade, SplittabilityGrade};

/// Quality assessment result for a spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// Complexity grade (size/effort)
    pub complexity: ComplexityGrade,
    /// Confidence grade (structure/clarity)
    pub confidence: ConfidenceGrade,
    /// Splittability grade (decomposability)
    pub splittability: SplittabilityGrade,
    /// Acceptance criteria quality grade
    pub ac_quality: ACQualityGrade,
}

/// Assess the quality of a spec
///
/// This is a pure function that computes quality metrics without any I/O.
pub fn assess_quality(spec: &crate::spec::Spec) -> QualityAssessment {
    use crate::score::{ac_quality, confidence, splittability};
    use crate::scoring::calculate_complexity;

    // Create a minimal config for confidence calculation
    let config = make_minimal_config();

    // Calculate each dimension
    let complexity = calculate_complexity(spec);
    let confidence_grade = confidence::calculate_confidence(spec, &config);
    let splittability_grade = splittability::calculate_splittability(spec);

    // Calculate AC quality from the spec's acceptance criteria
    let criteria = extract_acceptance_criteria(spec);
    let ac_quality_grade = ac_quality::calculate_ac_quality(&criteria);

    QualityAssessment {
        complexity,
        confidence: confidence_grade,
        splittability: splittability_grade,
        ac_quality: ac_quality_grade,
    }
}

fn make_minimal_config() -> crate::config::Config {
    crate::config::Config {
        project: crate::config::ProjectConfig {
            name: "test".to_string(),
            prefix: None,
            silent: false,
        },
        defaults: crate::config::DefaultsConfig::default(),
        providers: crate::provider::ProviderConfig::default(),
        parallel: crate::config::ParallelConfig::default(),
        repos: vec![],
        enterprise: crate::config::EnterpriseConfig::default(),
        approval: crate::config::ApprovalConfig::default(),
        validation: crate::config::OutputValidationConfig::default(),
        site: crate::config::SiteConfig::default(),
        lint: crate::config::LintConfig::default(),
        watch: crate::config::WatchConfig::default(),
    }
}

/// Extract acceptance criteria from a spec's body
fn extract_acceptance_criteria(spec: &crate::spec::Spec) -> Vec<String> {
    let acceptance_criteria_marker = "## Acceptance Criteria";
    let mut criteria = Vec::new();
    let mut in_code_fence = false;
    let mut in_ac_section = false;

    for line in spec.body.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
            in_ac_section = true;
            continue;
        }

        // Stop if we hit another ## heading
        if in_ac_section && !in_code_fence && trimmed.starts_with("## ") {
            break;
        }

        // Extract checkbox items
        if in_ac_section
            && !in_code_fence
            && (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]"))
        {
            // Extract text after checkbox
            let text = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim()
                .to_string();
            criteria.push(text);
        }
    }

    criteria
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Spec, SpecFrontmatter};

    #[test]
    fn test_assess_quality_simple_spec() {
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter {
                target_files: Some(vec!["file1.rs".to_string()]),
                ..Default::default()
            },
            title: Some("Simple test spec".to_string()),
            body: r#"## Problem

This is a simple test spec.

## Solution

Do something simple.

## Acceptance Criteria

- [ ] Create a new file
- [ ] Add a function
- [ ] Write a test

Simple implementation."#
                .to_string(),
        };

        let assessment = assess_quality(&spec);

        // Simple spec should score well on complexity (few criteria, few files, short)
        assert_eq!(assessment.complexity, ComplexityGrade::A);

        // Should have reasonable AC quality
        assert!(matches!(
            assessment.ac_quality,
            ACQualityGrade::A | ACQualityGrade::B | ACQualityGrade::C
        ));
    }
}
