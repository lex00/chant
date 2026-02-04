//! Agent status file format for worktree communication
//!
//! Provides the data structure and I/O operations for agents to communicate
//! their status to the watch process via `.chant-status.json` files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Status enum for agent execution state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatusState {
    /// Agent is currently working on the spec
    Working,
    /// Agent has successfully completed the spec
    Done,
    /// Agent has failed to complete the spec
    Failed,
}

/// Status information written by agent to communicate with watch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// The spec ID this status refers to
    pub spec_id: String,
    /// Current status of agent execution
    pub status: AgentStatusState,
    /// When this status was last updated (ISO 8601 timestamp)
    pub updated_at: String,
    /// Error message if status is Failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Commit hashes produced by the agent
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commits: Vec<String>,
}

/// Write agent status to a file atomically
///
/// Uses a temporary file + rename strategy to ensure atomic writes
/// and prevent corruption from concurrent access.
///
/// # Arguments
///
/// * `path` - Path where the status file should be written
/// * `status` - The status data to write
///
/// # Returns
///
/// Ok(()) if write succeeds, Err otherwise
pub fn write_status(path: &Path, status: &AgentStatus) -> Result<()> {
    // Serialize to JSON
    let json =
        serde_json::to_string_pretty(status).context("Failed to serialize AgentStatus to JSON")?;

    // Write to temporary file
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, json).context(format!(
        "Failed to write status to temporary file: {:?}",
        temp_path
    ))?;

    // Atomically rename temp file to final destination
    fs::rename(&temp_path, path).context(format!(
        "Failed to rename temporary status file to: {:?}",
        path
    ))?;

    Ok(())
}

/// Read agent status from a file
///
/// # Arguments
///
/// * `path` - Path to the status file
///
/// # Returns
///
/// Ok(AgentStatus) if read and parse succeed, Err otherwise
///
/// # Errors
///
/// Returns distinct errors for:
/// - Missing file (file not found)
/// - Corrupt JSON (parse error)
/// - I/O errors
pub fn read_status(path: &Path) -> Result<AgentStatus> {
    // Check if file exists first to provide a clear error
    if !path.exists() {
        anyhow::bail!("Status file not found at {:?}", path);
    }

    // Read file contents
    let contents =
        fs::read_to_string(path).context(format!("Failed to read status file: {:?}", path))?;

    // Parse JSON
    let status: AgentStatus = serde_json::from_str(&contents)
        .context(format!("Failed to parse status file as JSON: {:?}", path))?;

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_serialization_round_trip() {
        let status = AgentStatus {
            spec_id: "2026-02-03-test".to_string(),
            status: AgentStatusState::Done,
            updated_at: "2026-02-03T10:00:00Z".to_string(),
            error: None,
            commits: vec!["abc123".to_string()],
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: AgentStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.spec_id, "2026-02-03-test");
        assert_eq!(deserialized.status, AgentStatusState::Done);
        assert_eq!(deserialized.updated_at, "2026-02-03T10:00:00Z");
        assert_eq!(deserialized.error, None);
        assert_eq!(deserialized.commits, vec!["abc123"]);
    }

    #[test]
    fn test_write_status_atomic() {
        let temp_dir = TempDir::new().unwrap();
        let status_path = temp_dir.path().join(".chant-status.json");

        let status = AgentStatus {
            spec_id: "2026-02-03-test".to_string(),
            status: AgentStatusState::Working,
            updated_at: "2026-02-03T10:00:00Z".to_string(),
            error: None,
            commits: vec![],
        };

        write_status(&status_path, &status).unwrap();

        // Verify final file exists
        assert!(status_path.exists());

        // Verify temp file was cleaned up
        let temp_path = status_path.with_extension("tmp");
        assert!(!temp_path.exists());

        // Verify contents are correct
        let read_back = read_status(&status_path).unwrap();
        assert_eq!(read_back.spec_id, status.spec_id);
        assert_eq!(read_back.status, status.status);
    }

    #[test]
    fn test_read_status_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let status_path = temp_dir.path().join("nonexistent.json");

        let result = read_status(&status_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found"));
    }

    #[test]
    fn test_read_status_corrupt_json() {
        let temp_dir = TempDir::new().unwrap();
        let status_path = temp_dir.path().join("corrupt.json");

        // Write invalid JSON
        fs::write(&status_path, "{ invalid json }").unwrap();

        let result = read_status(&status_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("parse"));
    }

    #[test]
    fn test_status_with_error() {
        let status = AgentStatus {
            spec_id: "2026-02-03-test".to_string(),
            status: AgentStatusState::Failed,
            updated_at: "2026-02-03T10:00:00Z".to_string(),
            error: Some("Build failed".to_string()),
            commits: vec![],
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: AgentStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.status, AgentStatusState::Failed);
        assert_eq!(deserialized.error, Some("Build failed".to_string()));
    }

    #[test]
    fn test_status_multiple_commits() {
        let status = AgentStatus {
            spec_id: "2026-02-03-test".to_string(),
            status: AgentStatusState::Done,
            updated_at: "2026-02-03T10:00:00Z".to_string(),
            error: None,
            commits: vec![
                "abc123".to_string(),
                "def456".to_string(),
                "ghi789".to_string(),
            ],
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: AgentStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.commits.len(), 3);
        assert_eq!(deserialized.commits[0], "abc123");
        assert_eq!(deserialized.commits[2], "ghi789");
    }
}
