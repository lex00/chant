//! Confidence scoring based on spec structure, bullet quality, and vague language.
//!
//! Analyzes spec quality by examining:
//! - Bullet-to-prose ratio (structured vs unstructured content)
//! - Imperative verb usage in bullets (clear actionable items)
//! - Vague language patterns (unclear requirements)

use crate::config::Config;
use crate::scoring::ConfidenceGrade;
use crate::spec::Spec;

/// List of imperative verbs that indicate clear, actionable bullets
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

/// Calculate confidence grade based on spec structure, bullet quality, and vague language.
///
/// Grading rules:
/// - Grade A: High bullet ratio (>80%), verbs in >80% bullets, no vague patterns
/// - Grade B: Medium bullet ratio (>50%), verbs in >50% bullets, <3 vague patterns
/// - Grade C: Low bullet ratio (>20%), verbs in >30% bullets, 3-5 vague patterns
/// - Grade D: Very low bullet ratio (<20%) OR >5 vague patterns
///
/// Edge cases:
/// - Specs with no body text default to Grade D
/// - Empty bullets don't count toward bullet ratio
///
/// # Arguments
///
/// * `spec` - The spec to analyze
/// * `config` - Configuration (for potential future customization of vague patterns)
///
/// # Returns
///
/// A `ConfidenceGrade` based on the spec's structure and clarity
pub fn calculate_confidence(spec: &Spec, _config: &Config) -> ConfidenceGrade {
    // Edge case: empty body defaults to Grade D
    if spec.body.trim().is_empty() {
        return ConfidenceGrade::D;
    }

    // Count bullet lines and paragraph lines
    let (bullet_lines, paragraph_lines) = count_bullets_and_paragraphs(&spec.body);

    // Calculate bullet-to-prose ratio
    let bullet_ratio = if bullet_lines + paragraph_lines == 0 {
        0.0
    } else {
        bullet_lines as f64 / (bullet_lines + paragraph_lines) as f64
    };

    // Count bullets with imperative verbs
    let bullets_with_verbs = count_bullets_with_imperative_verbs(&spec.body);
    let verb_ratio = if bullet_lines == 0 {
        0.0
    } else {
        bullets_with_verbs as f64 / bullet_lines as f64
    };

    // Count all instances of vague patterns (not deduplicated)
    let vague_count = count_all_vague_instances(&spec.body);

    // Apply grading logic (check from best to worst grade)
    // Grade D: Very low bullet ratio (<20%) OR >5 vague patterns
    if bullet_ratio < 0.20 || vague_count > 5 {
        return ConfidenceGrade::D;
    }

    // Grade A: High bullet ratio (>80%), verbs in >80% bullets, no vague patterns
    if bullet_ratio > 0.80 && verb_ratio > 0.80 && vague_count == 0 {
        return ConfidenceGrade::A;
    }

    // Grade B: Medium bullet ratio (>50%), verbs in >50% bullets, <3 vague patterns
    if bullet_ratio > 0.50 && verb_ratio > 0.50 && vague_count < 3 {
        return ConfidenceGrade::B;
    }

    // Grade C: Low bullet ratio (>20%), verbs in >30% bullets, 3-5 vague patterns
    if bullet_ratio > 0.20 && verb_ratio > 0.30 && (3..=5).contains(&vague_count) {
        return ConfidenceGrade::C;
    }

    // Default to C for specs that don't fit clear patterns
    ConfidenceGrade::C
}

/// Count bullet lines and paragraph lines in the spec body.
///
/// Bullet lines start with `-` or `*` (after trimming).
/// Paragraph lines are non-empty lines that aren't bullets, headings, or code fences.
/// Empty bullets (just `-` or `*` with no content) don't count.
fn count_bullets_and_paragraphs(body: &str) -> (usize, usize) {
    let mut bullet_count = 0;
    let mut paragraph_count = 0;
    let mut in_code_fence = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // Track code fences
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        // Skip empty lines, code blocks, and headings
        if trimmed.is_empty() || in_code_fence || trimmed.starts_with('#') {
            continue;
        }

        // Skip lone bullet markers (just "-" or "*" without space/content)
        if trimmed == "-" || trimmed == "*" {
            continue;
        }

        // Check if it's a bullet
        if let Some(content) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            // Only count non-empty bullets
            if !content.trim().is_empty() {
                bullet_count += 1;
            }
        } else {
            // It's a paragraph line
            paragraph_count += 1;
        }
    }

    (bullet_count, paragraph_count)
}

/// Count all instances of vague patterns in the spec body.
///
/// Unlike `detect_vague_patterns` which deduplicates, this counts every
/// occurrence of every vague pattern. Multiple patterns in one line count
/// separately.
fn count_all_vague_instances(body: &str) -> usize {
    let body_lower = body.to_lowercase();
    let mut count = 0;

    for pattern in super::vague::DEFAULT_VAGUE_PATTERNS {
        let pattern_lower = pattern.to_lowercase();

        // Count all occurrences of this pattern
        let mut start = 0;
        while let Some(pos) = body_lower[start..].find(&pattern_lower) {
            count += 1;
            start += pos + pattern_lower.len();
        }
    }

    count
}

/// Count bullets that start with imperative verbs.
///
/// A bullet is considered to have an imperative verb if the first word
/// (after the bullet marker and checkbox if present) matches one of the
/// known imperative verbs.
fn count_bullets_with_imperative_verbs(body: &str) -> usize {
    let mut count = 0;
    let mut in_code_fence = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // Track code fences
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }

        // Skip non-bullets
        if in_code_fence {
            continue;
        }

        // Extract content after bullet marker
        let content = if let Some(c) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            c
        } else {
            continue;
        };

        // Skip checkbox if present ([ ] or [x])
        let content = if content.trim_start().starts_with("[") {
            if let Some(pos) = content.find(']') {
                &content[pos + 1..]
            } else {
                content
            }
        } else {
            content
        };

        // Get first word
        let first_word = content.split_whitespace().next();

        // Check if first word is an imperative verb
        if let Some(word) = first_word {
            let word_lower = word.to_lowercase();
            if IMPERATIVE_VERBS.contains(&word_lower.as_str()) {
                count += 1;
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Spec, SpecFrontmatter};

    fn make_config() -> Config {
        // Create a minimal config for testing
        Config {
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

    #[test]
    fn test_empty_body_returns_grade_d() {
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: String::new(),
        };

        let config = make_config();
        assert_eq!(calculate_confidence(&spec, &config), ConfidenceGrade::D);
    }

    #[test]
    fn test_grade_a_high_bullet_ratio_no_vague() {
        // 10 bullets, 2 paragraphs = 83% bullet ratio
        // All bullets have imperative verbs
        // No vague language
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: r#"
## Acceptance Criteria

- [ ] Implement feature A
- [ ] Add functionality B
- [ ] Create component C
- [ ] Update module D
- [ ] Fix bug E
- [ ] Remove deprecated code
- [ ] Test the implementation
- [ ] Verify the results
- [ ] Document the changes
- [ ] Deploy to production

Some paragraph here.
Another paragraph here.
"#
            .to_string(),
        };

        let config = make_config();
        assert_eq!(calculate_confidence(&spec, &config), ConfidenceGrade::A);
    }

    #[test]
    fn test_grade_b_medium_bullet_ratio_few_vague() {
        // 5 bullets, 5 paragraphs = 50% bullet ratio (need >50%)
        // Actually 6 bullets, 4 paragraphs = 60%
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: r#"
## Acceptance Criteria

- [ ] Implement feature A
- [ ] Add functionality B
- [ ] Create component C
- [ ] Update module D
- [ ] Fix bug E
- [ ] Deploy to production

Some paragraph here.
Another paragraph here.
Third paragraph.
Fourth paragraph with improve here.
"#
            .to_string(),
        };

        let config = make_config();
        assert_eq!(calculate_confidence(&spec, &config), ConfidenceGrade::B);
    }

    #[test]
    fn test_grade_d_low_bullet_ratio() {
        // 1 bullet, 10 paragraphs = ~9% bullet ratio (< 20%)
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: r#"
This is a wall of prose.
It has many paragraphs.
But very few bullets.
This makes it hard to understand.
Requirements should be clear.
Bullets help with clarity.
Paragraphs can be ambiguous.
We need more structure.
This spec is poorly written.
It will get a low grade.

- [ ] Implement something
"#
            .to_string(),
        };

        let config = make_config();
        assert_eq!(calculate_confidence(&spec, &config), ConfidenceGrade::D);
    }

    #[test]
    fn test_grade_d_many_vague_patterns() {
        // Even with good structure, >5 vague patterns → Grade D
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: r#"
## Acceptance Criteria

- [ ] Improve performance as needed
- [ ] Add features and related functionality
- [ ] Create tests etc
- [ ] Update components as needed
- [ ] Fix bugs and related issues
- [ ] Similar improvements needed
"#
            .to_string(),
        };

        let config = make_config();
        assert_eq!(calculate_confidence(&spec, &config), ConfidenceGrade::D);
    }

    #[test]
    fn test_count_bullets_and_paragraphs() {
        let body = r#"
This is a paragraph.

- [ ] This is a bullet
- [ ] Another bullet

Another paragraph here.

- This is also a bullet

# This is a heading (not counted)

Final paragraph.
"#;

        let (bullets, paragraphs) = count_bullets_and_paragraphs(body);
        assert_eq!(bullets, 3);
        assert_eq!(paragraphs, 3); // Three paragraph lines
    }

    #[test]
    fn test_empty_bullets_not_counted() {
        let body = r#"
- [ ] Valid bullet
-
- [ ] Another valid bullet
"#;

        let (bullets, paragraphs) = count_bullets_and_paragraphs(body);
        assert_eq!(bullets, 2); // Empty bullet not counted
        assert_eq!(paragraphs, 0);
    }

    #[test]
    fn test_count_bullets_with_imperative_verbs() {
        let body = r#"
- [ ] Implement feature A
- [ ] Add functionality B
- [ ] This does not start with a verb
- [ ] Create component C
- Update something without checkbox
"#;

        let count = count_bullets_with_imperative_verbs(body);
        assert_eq!(count, 4); // implement, add, create, update
    }

    #[test]
    fn test_code_blocks_ignored() {
        let body = r#"
- [ ] Implement feature

```rust
// This is code, not a bullet
- This looks like a bullet but it's in a code block
```

- [ ] Add another feature
"#;

        let (bullets, _) = count_bullets_and_paragraphs(body);
        assert_eq!(bullets, 2); // Only the two outside code blocks
    }

    #[test]
    fn test_case_insensitive_verb_matching() {
        let body = r#"
- [ ] IMPLEMENT feature
- [ ] Add functionality
- [ ] CrEaTe component
"#;

        let count = count_bullets_with_imperative_verbs(body);
        assert_eq!(count, 3); // All should match case-insensitively
    }

    #[test]
    fn test_grade_c_with_some_vague_patterns() {
        // Low-medium bullet ratio, some verbs, 3-5 vague patterns
        let spec = Spec {
            id: "test".to_string(),
            frontmatter: SpecFrontmatter::default(),
            title: Some("Test".to_string()),
            body: r#"
## Acceptance Criteria

- [ ] Implement feature A
- [ ] Add functionality B as needed
- [ ] Create tests etc

Some paragraph here.
Another paragraph with improve mentioned.
Third paragraph and related stuff.
"#
            .to_string(),
        };

        let config = make_config();
        assert_eq!(calculate_confidence(&spec, &config), ConfidenceGrade::C);
    }
}
