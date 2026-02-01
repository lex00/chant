//! Status data aggregation for specs
//!
//! Provides functionality to aggregate status information across all specs,
//! including counts by status, today's activity, attention items, and ready queue.

use anyhow::Result;
use chrono::{DateTime, Duration, Local};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::spec::{self, SpecStatus};

/// Activity data for today (last 24 hours)
#[derive(Debug, Clone, Default)]
pub struct TodayActivity {
    /// Number of specs completed today
    pub completed: usize,
    /// Number of specs started (moved to in_progress) today
    pub started: usize,
    /// Number of specs created today
    pub created: usize,
}

/// A spec requiring attention (failed or blocked)
#[derive(Debug, Clone)]
pub struct AttentionItem {
    /// Spec ID
    pub id: String,
    /// Spec title
    pub title: Option<String>,
    /// Current status
    pub status: SpecStatus,
    /// How long ago the status changed (e.g., "2h ago", "3d ago")
    pub ago: String,
}

/// A spec currently in progress
#[derive(Debug, Clone)]
pub struct InProgressItem {
    /// Spec ID
    pub id: String,
    /// Spec title
    pub title: Option<String>,
    /// Elapsed time in minutes since status changed to in_progress
    pub elapsed_minutes: i64,
}

/// A spec in the ready queue
#[derive(Debug, Clone)]
pub struct ReadyItem {
    /// Spec ID
    pub id: String,
    /// Spec title
    pub title: Option<String>,
}

/// Aggregated status data for all specs
#[derive(Debug, Clone, Default)]
pub struct StatusData {
    /// Counts by status
    pub counts: HashMap<String, usize>,
    /// Today's activity
    pub today: TodayActivity,
    /// Items requiring attention
    pub attention: Vec<AttentionItem>,
    /// In-progress items
    pub in_progress: Vec<InProgressItem>,
    /// Ready queue items (first 5)
    pub ready: Vec<ReadyItem>,
    /// Total count of ready items
    pub ready_count: usize,
}

/// Format a duration as a relative time string (e.g., "2h ago", "3d ago")
fn format_ago(datetime: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(datetime);

    let time_str = if duration.num_minutes() < 1 {
        "now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d", duration.num_days())
    } else if duration.num_weeks() < 4 {
        format!("{}w", duration.num_weeks())
    } else {
        format!("{}mo", duration.num_days() / 30)
    };

    format!("{} ago", time_str)
}

/// Parse an ISO 8601 timestamp string to Local datetime
fn parse_timestamp(timestamp: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
}

/// Get the last modification time of a file
fn get_file_modified_time(path: &Path) -> Option<DateTime<Local>> {
    fs::metadata(path).ok().and_then(|metadata| {
        metadata
            .modified()
            .ok()
            .map(DateTime::<Local>::from)
    })
}

/// Aggregate status data from all specs in the specs directory
pub fn aggregate_status(specs_dir: &Path) -> Result<StatusData> {
    let mut data = StatusData::default();

    // Initialize all status counts to 0
    data.counts.insert("pending".to_string(), 0);
    data.counts.insert("in_progress".to_string(), 0);
    data.counts.insert("completed".to_string(), 0);
    data.counts.insert("failed".to_string(), 0);
    data.counts.insert("blocked".to_string(), 0);
    data.counts.insert("ready".to_string(), 0);

    // Empty specs directory - return early
    if !specs_dir.exists() {
        return Ok(data);
    }

    // Load all specs
    let specs = match spec::load_all_specs(specs_dir) {
        Ok(specs) => specs,
        Err(e) => {
            eprintln!("Warning: Failed to load specs: {}", e);
            return Ok(data);
        }
    };

    // Calculate today's cutoff (24 hours ago)
    let today_cutoff = Local::now() - Duration::hours(24);

    // Track which specs are ready (for ready queue)
    let mut ready_specs = Vec::new();

    for spec in &specs {
        // Skip cancelled specs
        if spec.frontmatter.status == SpecStatus::Cancelled {
            continue;
        }

        // Count by status
        let status_key = match spec.frontmatter.status {
            SpecStatus::Pending => "pending",
            SpecStatus::InProgress => "in_progress",
            SpecStatus::Completed => "completed",
            SpecStatus::Failed | SpecStatus::NeedsAttention => "failed",
            SpecStatus::Blocked => "blocked",
            SpecStatus::Ready => "ready",
            SpecStatus::Cancelled => continue, // Already filtered above
        };

        if let Some(count) = data.counts.get_mut(status_key) {
            *count += 1;
        }

        // Track ready specs
        if spec.is_ready(&specs) {
            ready_specs.push(spec);
        }

        // Today's activity - completed specs
        if spec.frontmatter.status == SpecStatus::Completed {
            if let Some(ref completed_at) = spec.frontmatter.completed_at {
                if let Some(completed_time) = parse_timestamp(completed_at) {
                    if completed_time >= today_cutoff {
                        data.today.completed += 1;
                    }
                }
            }
        }

        // Today's activity - started specs (moved to in_progress)
        // We approximate this using file modification time since we don't track status change history
        if spec.frontmatter.status == SpecStatus::InProgress {
            let spec_path = specs_dir.join(format!("{}.md", spec.id));
            if let Some(modified_time) = get_file_modified_time(&spec_path) {
                if modified_time >= today_cutoff {
                    data.today.started += 1;
                }
            }
        }

        // Today's activity - created specs
        // Use file creation time as proxy for spec creation
        let spec_path = specs_dir.join(format!("{}.md", spec.id));
        if let Some(created_time) = get_file_modified_time(&spec_path) {
            if created_time >= today_cutoff {
                data.today.created += 1;
            }
        }

        // Attention items (failed or blocked)
        if matches!(
            spec.frontmatter.status,
            SpecStatus::Failed | SpecStatus::NeedsAttention | SpecStatus::Blocked
        ) {
            let spec_path = specs_dir.join(format!("{}.md", spec.id));
            if let Some(modified_time) = get_file_modified_time(&spec_path) {
                data.attention.push(AttentionItem {
                    id: spec.id.clone(),
                    title: spec.title.clone(),
                    status: spec.frontmatter.status.clone(),
                    ago: format_ago(modified_time),
                });
            }
        }

        // In-progress items
        if spec.frontmatter.status == SpecStatus::InProgress {
            let spec_path = specs_dir.join(format!("{}.md", spec.id));
            if let Some(modified_time) = get_file_modified_time(&spec_path) {
                let elapsed = Local::now()
                    .signed_duration_since(modified_time)
                    .num_minutes();
                data.in_progress.push(InProgressItem {
                    id: spec.id.clone(),
                    title: spec.title.clone(),
                    elapsed_minutes: elapsed,
                });
            }
        }
    }

    // Update ready count
    data.ready_count = ready_specs.len();
    *data.counts.get_mut("ready").unwrap() = ready_specs.len();

    // Ready queue (first 5)
    data.ready = ready_specs
        .iter()
        .take(5)
        .map(|spec| ReadyItem {
            id: spec.id.clone(),
            title: spec.title.clone(),
        })
        .collect();

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ago() {
        let now = Local::now();

        // Less than a minute
        let recent = now - Duration::seconds(30);
        assert_eq!(format_ago(recent), "now ago");

        // Minutes
        let mins = now - Duration::minutes(45);
        assert_eq!(format_ago(mins), "45m ago");

        // Hours
        let hours = now - Duration::hours(5);
        assert_eq!(format_ago(hours), "5h ago");

        // Days
        let days = now - Duration::days(3);
        assert_eq!(format_ago(days), "3d ago");
    }

    #[test]
    fn test_empty_specs_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let specs_dir = temp_dir.path().join("nonexistent");

        let result = aggregate_status(&specs_dir).unwrap();

        assert_eq!(*result.counts.get("pending").unwrap(), 0);
        assert_eq!(*result.counts.get("in_progress").unwrap(), 0);
        assert_eq!(*result.counts.get("completed").unwrap(), 0);
        assert_eq!(*result.counts.get("failed").unwrap(), 0);
        assert_eq!(result.today.completed, 0);
        assert_eq!(result.today.started, 0);
        assert_eq!(result.today.created, 0);
        assert!(result.attention.is_empty());
        assert!(result.in_progress.is_empty());
        assert!(result.ready.is_empty());
        assert_eq!(result.ready_count, 0);
    }
}
