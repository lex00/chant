//! Replay commit message formatting and utilities.
//!
//! Handles special commit message formatting for replay executions,
//! distinguishing them from original work commits.

/// Represents the context and metadata for a replay execution.
#[derive(Debug, Clone)]
pub struct ReplayContext {
    /// The spec ID being replayed
    pub spec_id: String,
    /// The spec title
    pub spec_title: Option<String>,
    /// Original completion timestamp (ISO 8601 format)
    pub original_completion: String,
    /// Reason for replay (default: "manual")
    pub replay_reason: String,
}

impl ReplayContext {
    /// Create a new replay context
    pub fn new(
        spec_id: String,
        spec_title: Option<String>,
        original_completion: String,
        replay_reason: Option<String>,
    ) -> Self {
        Self {
            spec_id,
            spec_title,
            original_completion,
            replay_reason: replay_reason.unwrap_or_else(|| "manual".to_string()),
        }
    }

    /// Format the commit message for a replay execution.
    ///
    /// Returns a tuple of (first_line, body) where:
    /// - first_line: "chant(<id>): replay - <title>"
    /// - body: Multi-line body with original completion date and replay reason
    ///
    /// The title is truncated to ensure reasonable commit message length.
    pub fn format_commit_message(&self) -> (String, String) {
        // Build the first line
        let title_part = self
            .spec_title
            .as_ref()
            .map(|t| format!("replay - {}", truncate_title(t)))
            .unwrap_or_else(|| "replay - Replay execution".to_string());

        let first_line = format!("chant({}): {}", self.spec_id, title_part);

        // Build the body with metadata
        let body = format!(
            "Original completion: {}\nReplay reason: {}",
            self.original_completion, self.replay_reason
        );

        (first_line, body)
    }
}

/// Truncate a spec title to a reasonable length for commit messages.
/// Git conventionally limits the first line to ~72 characters.
/// We reserve ~25 characters for "chant(<id>): replay - "
/// leaving ~47 characters for the title.
fn truncate_title(title: &str) -> String {
    const MAX_TITLE_LENGTH: usize = 47;

    if title.len() <= MAX_TITLE_LENGTH {
        title.to_string()
    } else {
        format!("{}...", &title[..MAX_TITLE_LENGTH - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_context_new() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            Some("Add feature X".to_string()),
            "2026-01-15T15:00:00Z".to_string(),
            None,
        );

        assert_eq!(ctx.spec_id, "2026-01-26-00f-xhl");
        assert_eq!(ctx.spec_title, Some("Add feature X".to_string()));
        assert_eq!(ctx.original_completion, "2026-01-15T15:00:00Z");
        assert_eq!(ctx.replay_reason, "manual");
    }

    #[test]
    fn test_replay_context_custom_reason() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            Some("Add feature X".to_string()),
            "2026-01-15T15:00:00Z".to_string(),
            Some("regression fix".to_string()),
        );

        assert_eq!(ctx.replay_reason, "regression fix");
    }

    #[test]
    fn test_format_commit_message_basic() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            Some("Add feature X".to_string()),
            "2026-01-15T15:00:00Z".to_string(),
            None,
        );

        let (first_line, body) = ctx.format_commit_message();

        assert_eq!(
            first_line,
            "chant(2026-01-26-00f-xhl): replay - Add feature X"
        );
        assert_eq!(
            body,
            "Original completion: 2026-01-15T15:00:00Z\nReplay reason: manual"
        );
    }

    #[test]
    fn test_format_commit_message_no_title() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            None,
            "2026-01-15T15:00:00Z".to_string(),
            None,
        );

        let (first_line, body) = ctx.format_commit_message();

        assert_eq!(
            first_line,
            "chant(2026-01-26-00f-xhl): replay - Replay execution"
        );
        assert_eq!(
            body,
            "Original completion: 2026-01-15T15:00:00Z\nReplay reason: manual"
        );
    }

    #[test]
    fn test_format_commit_message_with_custom_reason() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            Some("Add feature X".to_string()),
            "2026-01-15T15:00:00Z".to_string(),
            Some("regression fix".to_string()),
        );

        let (first_line, body) = ctx.format_commit_message();

        assert_eq!(
            body,
            "Original completion: 2026-01-15T15:00:00Z\nReplay reason: regression fix"
        );
    }

    #[test]
    fn test_truncate_title_short() {
        let short_title = "Add feature X";
        let truncated = truncate_title(short_title);
        assert_eq!(truncated, "Add feature X");
    }

    #[test]
    fn test_truncate_title_long() {
        let long_title = "This is a very long title that definitely exceeds the maximum length we want for commit messages";
        let truncated = truncate_title(long_title);

        // Should be 47 chars or less (MAX_TITLE_LENGTH = 47)
        assert!(truncated.len() <= 47);
        // Should end with "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_title_boundary() {
        let title = "a".repeat(47);
        let truncated = truncate_title(&title);
        // Exactly at boundary, should not truncate
        assert_eq!(truncated, title);

        let title_over = "a".repeat(48);
        let truncated_over = truncate_title(&title_over);
        // Over boundary, should truncate
        assert!(truncated_over.len() <= 47);
        assert!(truncated_over.ends_with("..."));
    }

    #[test]
    fn test_commit_message_first_line_length() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            Some("a".repeat(100).to_string()),
            "2026-01-15T15:00:00Z".to_string(),
            None,
        );

        let (first_line, _body) = ctx.format_commit_message();

        // First line should not be excessively long even with very long title
        // Git convention is ~72 chars, but we allow up to ~85 chars for reasonable spec IDs
        assert!(
            first_line.len() < 100,
            "First line too long: {}",
            first_line
        );
        // Title should be truncated (shown by "..." at end)
        assert!(first_line.ends_with("..."), "Long title should be truncated");
    }

    #[test]
    fn test_replay_context_example_from_spec() {
        let ctx = ReplayContext::new(
            "2026-01-26-00f-xhl".to_string(),
            Some("Add feature X".to_string()),
            "2026-01-15T15:00:00Z".to_string(),
            None,
        );

        let (first_line, body) = ctx.format_commit_message();

        // Check exact format from acceptance criteria
        assert_eq!(
            first_line,
            "chant(2026-01-26-00f-xhl): replay - Add feature X"
        );
        assert!(body.contains("Original completion: 2026-01-15T15:00:00Z"));
        assert!(body.contains("Replay reason: manual"));
    }
}
