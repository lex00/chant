//! ASCII timeline generation for specs.
//!
//! This module generates ASCII art timelines showing spec activity
//! grouped by date, week, or month.

use std::collections::HashMap;

use chrono::Datelike;

use crate::config::TimelineGroupBy;
use crate::spec::{Spec, SpecStatus};

/// A group of specs for a timeline entry
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineGroup {
    /// The date/period label for this group
    pub date: String,
    /// ASCII tree representation of specs in this group
    pub ascii_tree: String,
}

/// Build timeline groups from specs
pub fn build_timeline_groups(
    specs: &[&Spec],
    group_by: TimelineGroupBy,
    include_pending: bool,
) -> Vec<TimelineGroup> {
    // Filter specs based on include_pending
    let filtered_specs: Vec<_> = specs
        .iter()
        .filter(|s| {
            if !include_pending && s.frontmatter.status == SpecStatus::Pending {
                return false;
            }
            true
        })
        .collect();

    // Group specs by date
    let mut groups: HashMap<String, Vec<&&Spec>> = HashMap::new();

    for spec in &filtered_specs {
        let date_key = extract_date_key(spec, group_by);
        groups.entry(date_key).or_default().push(spec);
    }

    // Sort groups by date descending
    let mut sorted_keys: Vec<_> = groups.keys().cloned().collect();
    sorted_keys.sort_by(|a, b| b.cmp(a));

    // Build timeline groups
    sorted_keys
        .into_iter()
        .map(|date| {
            let specs_in_group = groups.get(&date).unwrap();
            let ascii_tree = render_tree(specs_in_group);
            let display_date = format_date_display(&date, group_by);
            TimelineGroup {
                date: display_date,
                ascii_tree,
            }
        })
        .collect()
}

/// Extract the date key for grouping
fn extract_date_key(spec: &Spec, group_by: TimelineGroupBy) -> String {
    // Try to get date from completed_at or from spec ID
    let date_str = spec
        .frontmatter
        .completed_at
        .as_ref()
        .and_then(|s| s.split('T').next())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Extract date from ID (e.g., 2026-01-30-00a-xyz -> 2026-01-30)
            let parts: Vec<_> = spec.id.split('-').collect();
            if parts.len() >= 3 {
                format!("{}-{}-{}", parts[0], parts[1], parts[2])
            } else {
                "unknown".to_string()
            }
        });

    match group_by {
        TimelineGroupBy::Day => date_str,
        TimelineGroupBy::Week => {
            // Get the week start (Monday)
            if let Some(date) = parse_date(&date_str) {
                let weekday = date.weekday();
                let days_since_monday = weekday.num_days_from_monday();
                let monday = date - chrono::Duration::days(days_since_monday as i64);
                monday.format("%Y-%m-%d").to_string()
            } else {
                date_str
            }
        }
        TimelineGroupBy::Month => {
            // Get YYYY-MM
            let parts: Vec<_> = date_str.split('-').collect();
            if parts.len() >= 2 {
                format!("{}-{}", parts[0], parts[1])
            } else {
                date_str
            }
        }
    }
}

/// Parse a date string to a chrono NaiveDate
fn parse_date(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Format the date for display
fn format_date_display(date_key: &str, group_by: TimelineGroupBy) -> String {
    match group_by {
        TimelineGroupBy::Day => date_key.to_string(),
        TimelineGroupBy::Week => format!("Week of {}", date_key),
        TimelineGroupBy::Month => {
            if let Some(date) = parse_month(date_key) {
                date.format("%B %Y").to_string()
            } else {
                date_key.to_string()
            }
        }
    }
}

/// Parse a month string (YYYY-MM) to a date
fn parse_month(s: &str) -> Option<chrono::NaiveDate> {
    let full = format!("{}-01", s);
    chrono::NaiveDate::parse_from_str(&full, "%Y-%m-%d").ok()
}

/// Render specs as an ASCII tree
fn render_tree(specs: &[&&Spec]) -> String {
    if specs.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();

    for (i, spec) in specs.iter().enumerate() {
        let is_last = i == specs.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };

        let status_icon = match spec.frontmatter.status {
            SpecStatus::Completed => "✓",
            SpecStatus::InProgress => "◐",
            SpecStatus::Pending => "○",
            SpecStatus::Failed => "✗",
            SpecStatus::NeedsAttention => "⚠",
            _ => "•",
        };

        let short_id = spec.id.split('-').skip(3).collect::<Vec<_>>().join("-");

        let short_id = if short_id.is_empty() {
            &spec.id
        } else {
            &short_id
        };

        let title = spec
            .title
            .as_ref()
            .map(|t| truncate(t, 30))
            .unwrap_or_else(|| "Untitled".to_string());

        let status_display = match spec.frontmatter.status {
            SpecStatus::Completed => "completed",
            SpecStatus::InProgress => "in_progress",
            SpecStatus::Pending => "pending",
            SpecStatus::Failed => "failed",
            _ => "other",
        };

        lines.push(format!(
            "{}{} {}  {} ({})",
            prefix, status_icon, short_id, title, status_display
        ));
    }

    lines.join("\n")
}

/// Truncate a string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::SpecFrontmatter;

    fn make_spec(id: &str, status: SpecStatus, completed_at: Option<&str>) -> Spec {
        Spec {
            id: id.to_string(),
            title: Some("Test Spec".to_string()),
            body: String::new(),
            frontmatter: SpecFrontmatter {
                status,
                completed_at: completed_at.map(|s| s.to_string()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_build_timeline_groups_empty() {
        let specs: Vec<&Spec> = vec![];
        let groups = build_timeline_groups(&specs, TimelineGroupBy::Day, false);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_extract_date_key_from_id() {
        let spec = make_spec("2026-01-30-00a-xyz", SpecStatus::Pending, None);
        let key = extract_date_key(&spec, TimelineGroupBy::Day);
        assert_eq!(key, "2026-01-30");
    }

    #[test]
    fn test_extract_date_key_from_completed_at() {
        let spec = make_spec(
            "2026-01-30-00a-xyz",
            SpecStatus::Completed,
            Some("2026-01-29T12:00:00Z"),
        );
        let key = extract_date_key(&spec, TimelineGroupBy::Day);
        assert_eq!(key, "2026-01-29");
    }

    #[test]
    fn test_render_tree() {
        let spec1 = make_spec("2026-01-30-00a-xyz", SpecStatus::Completed, None);
        let spec2 = make_spec("2026-01-30-00b-abc", SpecStatus::InProgress, None);
        let specs: Vec<&Spec> = vec![&spec1, &spec2];
        let refs: Vec<&&Spec> = specs.iter().collect();
        let tree = render_tree(&refs);

        assert!(tree.contains("✓"));
        assert!(tree.contains("◐"));
        assert!(tree.contains("├──"));
        assert!(tree.contains("└──"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long title here", 10), "a very lo…");
    }

    #[test]
    fn test_format_date_display() {
        assert_eq!(
            format_date_display("2026-01-30", TimelineGroupBy::Day),
            "2026-01-30"
        );
        assert_eq!(
            format_date_display("2026-01-27", TimelineGroupBy::Week),
            "Week of 2026-01-27"
        );
        assert_eq!(
            format_date_display("2026-01", TimelineGroupBy::Month),
            "January 2026"
        );
    }
}
