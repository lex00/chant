//! Centralized UI formatting and color utilities
//!
//! This module provides a unified interface for status colors, icons, and
//! formatting patterns used throughout the chant CLI.

use colored::{ColoredString, Colorize};

use crate::spec::SpecStatus;

/// Check if quiet mode is enabled via environment variable or --quiet flag
pub fn is_quiet() -> bool {
    std::env::var("CHANT_QUIET")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Returns a colored status icon for the given spec status.
///
/// Icons:
/// - Pending: ○ (white)
/// - InProgress: ◐ (yellow)
/// - Completed: ● (green)
/// - Failed: ✗ (red)
/// - NeedsAttention: ⚠ (yellow)
/// - Ready: ◕ (cyan)
/// - Blocked: ⊗ (red)
/// - Cancelled: ✓ (dimmed)
pub fn status_icon(status: &SpecStatus) -> ColoredString {
    match status {
        SpecStatus::Pending => "○".white(),
        SpecStatus::InProgress => "◐".yellow(),
        SpecStatus::Completed => "●".green(),
        SpecStatus::Failed => "✗".red(),
        SpecStatus::NeedsAttention => "⚠".yellow(),
        SpecStatus::Ready => "◕".cyan(),
        SpecStatus::Blocked => "⊗".red(),
        SpecStatus::Cancelled => "✓".dimmed(),
    }
}

/// Returns a colored status symbol for attention items (failed/blocked).
///
/// Symbols:
/// - Failed/NeedsAttention: ✗ (red)
/// - Blocked: ◌ (yellow)
/// - Other: ? (normal)
pub fn attention_symbol(status: &SpecStatus) -> ColoredString {
    match status {
        SpecStatus::Failed | SpecStatus::NeedsAttention => "✗".red(),
        SpecStatus::Blocked => "◌".yellow(),
        _ => "?".normal(),
    }
}

/// Color scheme for status-related text output
pub mod colors {
    use colored::{Color, ColoredString, Colorize};

    /// Green for success/completion
    pub fn success(text: &str) -> ColoredString {
        text.green()
    }

    /// Yellow for in-progress/warnings
    pub fn warning(text: &str) -> ColoredString {
        text.yellow()
    }

    /// Red for errors/failures
    pub fn error(text: &str) -> ColoredString {
        text.red()
    }

    /// Cyan for identifiers (spec IDs, etc.)
    pub fn identifier(text: &str) -> ColoredString {
        text.cyan()
    }

    /// Blue for informational text
    pub fn info(text: &str) -> ColoredString {
        text.blue()
    }

    /// Dimmed for secondary text
    pub fn secondary(text: &str) -> ColoredString {
        text.dimmed()
    }

    /// Bold for headings
    pub fn heading(text: &str) -> ColoredString {
        text.bold()
    }

    /// Color for markdown heading levels
    pub fn markdown_heading(text: &str, level: usize) -> ColoredString {
        match level {
            1 => text.bold(),
            2 => text.bold().cyan(),
            3 => text.bold().blue(),
            4 => text.bold().magenta(),
            _ => text.bold(),
        }
    }

    /// Generic colored text
    pub fn colored(text: &str, color: Color) -> ColoredString {
        text.color(color)
    }
}

/// Common text formatting patterns
pub mod format {
    /// Truncate a title to fit terminal width
    pub fn truncate_title(title: &str, max_len: usize) -> String {
        if title.len() <= max_len {
            title.to_string()
        } else {
            format!("{}...", &title[..max_len.saturating_sub(3)])
        }
    }

    /// Format elapsed time in minutes to human-readable string
    pub fn elapsed_minutes(minutes: i64) -> String {
        if minutes < 1 {
            "just now".to_string()
        } else if minutes < 60 {
            format!("{}m", minutes)
        } else if minutes < 1440 {
            // Less than 24 hours
            let hours = minutes / 60;
            let mins = minutes % 60;
            if mins == 0 {
                format!("{}h", hours)
            } else {
                format!("{}h {}m", hours, mins)
            }
        } else {
            // 24 hours or more
            let days = minutes / 1440;
            format!("{}d", days)
        }
    }

    /// Format a separator line for sections
    pub fn separator(width: usize) -> String {
        "─".repeat(width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_icon_all_statuses() {
        status_icon(&SpecStatus::Pending);
        status_icon(&SpecStatus::InProgress);
        status_icon(&SpecStatus::Completed);
        status_icon(&SpecStatus::Failed);
        status_icon(&SpecStatus::NeedsAttention);
        status_icon(&SpecStatus::Ready);
        status_icon(&SpecStatus::Blocked);
        status_icon(&SpecStatus::Cancelled);
    }

    #[test]
    fn test_attention_symbol() {
        attention_symbol(&SpecStatus::Failed);
        attention_symbol(&SpecStatus::NeedsAttention);
        attention_symbol(&SpecStatus::Blocked);
        attention_symbol(&SpecStatus::Pending);
    }

    #[test]
    fn test_truncate_title() {
        assert_eq!(format::truncate_title("short", 10), "short");
        assert_eq!(format::truncate_title("exactly ten", 11), "exactly ten");
        assert_eq!(
            format::truncate_title("this is a very long title", 10),
            "this is..."
        );
    }

    #[test]
    fn test_elapsed_minutes() {
        assert_eq!(format::elapsed_minutes(0), "just now");
        assert_eq!(format::elapsed_minutes(30), "30m");
        assert_eq!(format::elapsed_minutes(60), "1h");
        assert_eq!(format::elapsed_minutes(90), "1h 30m");
        assert_eq!(format::elapsed_minutes(120), "2h");
        assert_eq!(format::elapsed_minutes(1440), "1d");
        assert_eq!(format::elapsed_minutes(2880), "2d");
    }

    #[test]
    fn test_separator() {
        assert_eq!(format::separator(5), "─────");
        assert_eq!(format::separator(10), "──────────");
    }
}
