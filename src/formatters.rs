//! Output formatters for status data
//!
//! Provides formatters that transform StatusData into different output formats.

use crate::status::{AttentionItem, InProgressItem, ReadyItem, StatusData, TodayActivity};
use crate::ui;

/// Format StatusData as regular multi-section text output
pub fn format_regular_status(data: &StatusData) -> String {
    let mut output = vec![
        ui::colors::heading("Chant Status").to_string(),
        ui::format::separator(12),
        String::new(),
        format_counts(&data.counts),
        String::new(),
    ];

    // Today section
    if data.today.completed > 0 || data.today.started > 0 || data.today.created > 0 {
        output.push(ui::colors::heading("Today").to_string());
        output.push(ui::format::separator(5));
        output.push(format_today(&data.today));
        output.push(String::new());
    }

    // Attention section (only if there are items)
    if !data.attention.is_empty() {
        output.push(ui::colors::heading("Attention").to_string());
        output.push(ui::format::separator(9));
        for item in &data.attention {
            output.push(format_attention_item(item));
        }
        output.push(String::new());
    }

    // In Progress section (only if there are items)
    if !data.in_progress.is_empty() {
        output.push(ui::colors::heading("In Progress").to_string());
        output.push(ui::format::separator(11));
        for item in &data.in_progress {
            output.push(format_in_progress_item(item));
        }
        output.push(String::new());
    }

    // Ready section
    output.push(ui::colors::heading(&format!("Ready ({})", data.ready_count)).to_string());
    output.push(ui::format::separator(6));
    if data.ready_count == 0 {
        output.push(ui::colors::secondary("  (no specs ready)").to_string());
    } else {
        for item in &data.ready {
            output.push(format_ready_item(item));
        }
        if data.ready_count > 5 {
            let remaining = data.ready_count - 5;
            output
                .push(ui::colors::secondary(&format!("  ... and {} more", remaining)).to_string());
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
        parts.push(ui::colors::success(&format!("+{} completed", today.completed)).to_string());
    }
    if today.started > 0 {
        parts.push(ui::colors::warning(&format!("+{} started", today.started)).to_string());
    }
    if today.created > 0 {
        parts.push(ui::colors::info(&format!("+{} created", today.created)).to_string());
    }

    if parts.is_empty() {
        ui::colors::secondary("  (no activity today)").to_string()
    } else {
        format!("  {}", parts.join(", "))
    }
}

/// Format an attention item (failed or blocked)
fn format_attention_item(item: &AttentionItem) -> String {
    let symbol = ui::attention_symbol(&item.status);

    let title = item.title.as_deref().unwrap_or("(untitled)");
    let truncated_title = ui::format::truncate_title(title, 60);

    format!(
        "  {} {}  {} ({})",
        symbol,
        ui::colors::identifier(&item.id),
        truncated_title,
        ui::colors::secondary(&item.ago)
    )
}

/// Format an in-progress item
fn format_in_progress_item(item: &InProgressItem) -> String {
    let title = item.title.as_deref().unwrap_or("(untitled)");
    let truncated_title = ui::format::truncate_title(title, 60);

    let elapsed_str = ui::format::elapsed_minutes(item.elapsed_minutes);

    format!(
        "  {} {}  ({})",
        ui::colors::identifier(&item.id),
        truncated_title,
        ui::colors::secondary(&elapsed_str)
    )
}

/// Format a ready item
fn format_ready_item(item: &ReadyItem) -> String {
    let title = item.title.as_deref().unwrap_or("(untitled)");
    let truncated_title = ui::format::truncate_title(title, 60);

    format!("  {} {}", ui::colors::identifier(&item.id), truncated_title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
        // Should not contain Attention section header
        assert!(!result.contains("Attention\n─────────"));
        // Should contain In Progress in counts but not as a section header with underline
        assert!(result.contains("In Progress:"));
        assert!(!result.contains("In Progress\n───────────"));
    }
}
