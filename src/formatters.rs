//! Output formatters for status data
//!
//! Provides formatters that transform StatusData into different output formats.

use colored::Colorize;

use crate::spec::SpecStatus;
use crate::status::{AttentionItem, InProgressItem, ReadyItem, StatusData, TodayActivity};

/// Format StatusData as regular multi-section text output
pub fn format_regular_status(data: &StatusData) -> String {
    let mut output = vec![
        "Chant Status".bold().to_string(),
        "============".to_string(),
        String::new(),
        format_counts(&data.counts),
        String::new(),
    ];

    // Today section
    if data.today.completed > 0 || data.today.started > 0 || data.today.created > 0 {
        output.push("Today".bold().to_string());
        output.push("─────".to_string());
        output.push(format_today(&data.today));
        output.push(String::new());
    }

    // Attention section (only if there are items)
    if !data.attention.is_empty() {
        output.push("Attention".bold().to_string());
        output.push("─────────".to_string());
        for item in &data.attention {
            output.push(format_attention_item(item));
        }
        output.push(String::new());
    }

    // In Progress section (only if there are items)
    if !data.in_progress.is_empty() {
        output.push("In Progress".bold().to_string());
        output.push("───────────".to_string());
        for item in &data.in_progress {
            output.push(format_in_progress_item(item));
        }
        output.push(String::new());
    }

    // Ready section
    output.push(format!("Ready ({})", data.ready_count).bold().to_string());
    output.push("──────".to_string());
    if data.ready_count == 0 {
        output.push("  (no specs ready)".dimmed().to_string());
    } else {
        for item in &data.ready {
            output.push(format_ready_item(item));
        }
        if data.ready_count > 5 {
            let remaining = data.ready_count - 5;
            output.push(format!("  ... and {} more", remaining).dimmed().to_string());
        }
    }

    output.join("\n")
}

/// Format counts section with aligned numbers
fn format_counts(counts: &std::collections::HashMap<String, usize>) -> String {
    let pending = counts.get("pending").copied().unwrap_or(0);
    let in_progress = counts.get("in_progress").copied().unwrap_or(0);
    let completed = counts.get("completed").copied().unwrap_or(0);
    let failed = counts.get("failed").copied().unwrap_or(0);
    let blocked = counts.get("blocked").copied().unwrap_or(0);
    let ready = counts.get("ready").copied().unwrap_or(0);

    format!(
        "  {:<12} {}\n  {:<12} {}\n  {:<12} {}\n  {:<12} {}\n  {:<12} {}\n  {:<12} {}",
        "Pending:",
        pending,
        "Ready:",
        ready,
        "In Progress:",
        in_progress,
        "Completed:",
        completed,
        "Failed:",
        failed,
        "Blocked:",
        blocked,
    )
}

/// Format today's activity
fn format_today(today: &TodayActivity) -> String {
    let mut parts = Vec::new();

    if today.completed > 0 {
        parts.push(
            format!("+{} completed", today.completed)
                .green()
                .to_string(),
        );
    }
    if today.started > 0 {
        parts.push(format!("+{} started", today.started).yellow().to_string());
    }
    if today.created > 0 {
        parts.push(format!("+{} created", today.created).blue().to_string());
    }

    if parts.is_empty() {
        "  (no activity today)".dimmed().to_string()
    } else {
        format!("  {}", parts.join(", "))
    }
}

/// Format an attention item (failed or blocked)
fn format_attention_item(item: &AttentionItem) -> String {
    let symbol = match item.status {
        SpecStatus::Failed | SpecStatus::NeedsAttention => "✗".red(),
        SpecStatus::Blocked => "◌".yellow(),
        _ => "?".normal(),
    };

    let title = item.title.as_deref().unwrap_or("(untitled)");
    let truncated_title = truncate_title(title, 60);

    format!(
        "  {} {}  {} ({})",
        symbol,
        item.id.cyan(),
        truncated_title,
        item.ago.dimmed()
    )
}

/// Format an in-progress item
fn format_in_progress_item(item: &InProgressItem) -> String {
    let title = item.title.as_deref().unwrap_or("(untitled)");
    let truncated_title = truncate_title(title, 60);

    let elapsed_str = format_elapsed_minutes(item.elapsed_minutes);

    format!(
        "  {} {}  ({})",
        item.id.cyan(),
        truncated_title,
        elapsed_str.dimmed()
    )
}

/// Format a ready item
fn format_ready_item(item: &ReadyItem) -> String {
    let title = item.title.as_deref().unwrap_or("(untitled)");
    let truncated_title = truncate_title(title, 60);

    format!("  {} {}", item.id.cyan(), truncated_title)
}

/// Truncate a title to fit terminal width
fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else {
        format!("{}...", &title[..max_len.saturating_sub(3)])
    }
}

/// Format elapsed time in minutes to human-readable string
fn format_elapsed_minutes(minutes: i64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_truncate_title() {
        assert_eq!(truncate_title("short", 10), "short");
        assert_eq!(truncate_title("exactly ten", 11), "exactly ten");
        assert_eq!(
            truncate_title("this is a very long title", 10),
            "this is..."
        );
    }

    #[test]
    fn test_format_elapsed_minutes() {
        assert_eq!(format_elapsed_minutes(0), "just now");
        assert_eq!(format_elapsed_minutes(30), "30m");
        assert_eq!(format_elapsed_minutes(60), "1h");
        assert_eq!(format_elapsed_minutes(90), "1h 30m");
        assert_eq!(format_elapsed_minutes(120), "2h");
        assert_eq!(format_elapsed_minutes(1440), "1d");
        assert_eq!(format_elapsed_minutes(2880), "2d");
    }

    #[test]
    fn test_format_counts() {
        let mut counts = HashMap::new();
        counts.insert("pending".to_string(), 5);
        counts.insert("in_progress".to_string(), 2);
        counts.insert("completed".to_string(), 10);
        counts.insert("failed".to_string(), 1);
        counts.insert("blocked".to_string(), 0);
        counts.insert("ready".to_string(), 3);

        let result = format_counts(&counts);
        assert!(result.contains("Pending:"));
        assert!(result.contains("5"));
        assert!(result.contains("Ready:"));
        assert!(result.contains("3"));
    }

    #[test]
    fn test_format_today_all_activity() {
        let today = TodayActivity {
            completed: 2,
            started: 1,
            created: 3,
        };

        let result = format_today(&today);
        assert!(result.contains("+2 completed"));
        assert!(result.contains("+1 started"));
        assert!(result.contains("+3 created"));
    }

    #[test]
    fn test_format_today_no_activity() {
        let today = TodayActivity {
            completed: 0,
            started: 0,
            created: 0,
        };

        let result = format_today(&today);
        assert!(result.contains("no activity"));
    }

    #[test]
    fn test_format_regular_status_empty() {
        let data = StatusData::default();
        let result = format_regular_status(&data);

        assert!(result.contains("Chant Status"));
        assert!(result.contains("Ready (0)"));
        assert!(result.contains("no specs ready"));
        // Should not contain Attention or In Progress sections
        assert!(!result.contains("Attention"));
        assert!(!result.contains("In Progress"));
    }
}
