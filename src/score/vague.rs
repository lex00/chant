//! Vague language detection utility for spec text analysis.
//!
//! Detects vague language patterns that may indicate unclear requirements
//! or underspecified acceptance criteria.

use std::collections::HashSet;

/// Default vague language patterns to detect
pub const DEFAULT_VAGUE_PATTERNS: &[&str] =
    &["improve", "as needed", "etc", "and related", "similar"];

/// Detects vague language patterns in text.
///
/// Performs case-insensitive matching and returns a list of matched patterns.
/// Each pattern is reported at most once, even if it appears multiple times.
///
/// # Arguments
///
/// * `text` - The text to analyze for vague patterns
/// * `patterns` - The patterns to search for
///
/// # Returns
///
/// A vector of matched pattern strings (deduplicated, in order of first match)
///
/// # Examples
///
/// ```
/// use chant::score::vague::detect_vague_patterns;
///
/// let text = "improve performance";
/// let patterns = vec!["improve".to_string()];
/// let matches = detect_vague_patterns(text, &patterns);
/// assert_eq!(matches, vec!["improve"]);
/// ```
///
/// ```
/// use chant::score::vague::detect_vague_patterns;
///
/// let text = "Add feature and related tests";
/// let patterns = vec!["and related".to_string()];
/// let matches = detect_vague_patterns(text, &patterns);
/// assert_eq!(matches, vec!["and related"]);
/// ```
///
/// ```
/// use chant::score::vague::detect_vague_patterns;
///
/// let text = "Clean code";
/// let patterns = vec!["improve".to_string()];
/// let matches = detect_vague_patterns(text, &patterns);
/// assert_eq!(matches, Vec::<String>::new());
/// ```
pub fn detect_vague_patterns(text: &str, patterns: &[String]) -> Vec<String> {
    // Handle edge cases
    if text.is_empty() || patterns.is_empty() {
        return Vec::new();
    }

    let text_lower = text.to_lowercase();
    let mut found_patterns = Vec::new();
    let mut seen = HashSet::new();

    // Check each pattern
    for pattern in patterns {
        let pattern_lower = pattern.to_lowercase();

        // Only add if we haven't seen this pattern yet and it's in the text
        if !seen.contains(&pattern_lower) && text_lower.contains(&pattern_lower) {
            found_patterns.push(pattern.clone());
            seen.insert(pattern_lower);
        }
    }

    found_patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_single_pattern() {
        let text = "improve performance";
        let patterns = vec!["improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, vec!["improve"]);
    }

    #[test]
    fn test_detect_pattern_in_phrase() {
        let text = "Add feature and related tests";
        let patterns = vec!["and related".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, vec!["and related"]);
    }

    #[test]
    fn test_no_match() {
        let text = "Clean code";
        let patterns = vec!["improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_empty_text() {
        let text = "";
        let patterns = vec!["improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_empty_patterns() {
        let text = "improve performance";
        let patterns: Vec<String> = vec![];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_case_insensitive() {
        let text = "IMPROVE Performance";
        let patterns = vec!["improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, vec!["improve"]);
    }

    #[test]
    fn test_case_insensitive_pattern() {
        let text = "improve performance";
        let patterns = vec!["IMPROVE".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, vec!["IMPROVE"]);
    }

    #[test]
    fn test_multiple_patterns() {
        let text = "improve performance and related metrics etc";
        let patterns = vec![
            "improve".to_string(),
            "and related".to_string(),
            "etc".to_string(),
        ];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, vec!["improve", "and related", "etc"]);
    }

    #[test]
    fn test_overlapping_patterns_reported_once() {
        let text = "improve improve improve";
        let patterns = vec!["improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        // Should only be reported once
        assert_eq!(result, vec!["improve"]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_duplicate_patterns_in_list() {
        let text = "improve performance";
        let patterns = vec!["improve".to_string(), "improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        // Should only report first occurrence
        assert_eq!(result, vec!["improve"]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_partial_word_match() {
        // "improve" should match "improved" or "improvement"
        let text = "we need improvement here";
        let patterns = vec!["improve".to_string()];
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result, vec!["improve"]);
    }

    #[test]
    fn test_default_patterns() {
        // Test that default patterns are defined
        assert!(DEFAULT_VAGUE_PATTERNS.contains(&"improve"));
        assert!(DEFAULT_VAGUE_PATTERNS.contains(&"as needed"));
        assert!(DEFAULT_VAGUE_PATTERNS.contains(&"etc"));
        assert!(DEFAULT_VAGUE_PATTERNS.contains(&"and related"));
        assert!(DEFAULT_VAGUE_PATTERNS.contains(&"similar"));
    }

    #[test]
    fn test_all_default_patterns() {
        let text = "improve as needed, etc and related similar things";
        let patterns: Vec<String> = DEFAULT_VAGUE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let result = detect_vague_patterns(text, &patterns);
        assert_eq!(result.len(), 5);
        assert!(result.contains(&"improve".to_string()));
        assert!(result.contains(&"as needed".to_string()));
        assert!(result.contains(&"etc".to_string()));
        assert!(result.contains(&"and related".to_string()));
        assert!(result.contains(&"similar".to_string()));
    }
}
