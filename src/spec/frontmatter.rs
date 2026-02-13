//! Frontmatter types and defaults for specs.

use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

/// Deserialize depends_on as either a string or array of strings
fn deserialize_depends_on<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    let value = Option::<StringOrVec>::deserialize(deserializer)?;
    Ok(value.map(|v| match v {
        StringOrVec::String(s) => vec![s],
        StringOrVec::Vec(v) => v,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    #[default]
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
    NeedsAttention,
    Ready,
    Blocked,
    Cancelled,
}

impl FromStr for SpecStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "needs_attention" => Ok(Self::NeedsAttention),
            "ready" => Ok(Self::Ready),
            "blocked" => Ok(Self::Blocked),
            "cancelled" => Ok(Self::Cancelled),
            _ => anyhow::bail!("Invalid status: {}. Must be one of: pending, in_progress, paused, completed, failed, needs_attention, ready, blocked, cancelled", s),
        }
    }
}

/// Spec type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpecType {
    #[default]
    Code,
    Task,
    Driver,
    Documentation,
    Research,
    Group,
}

pub(crate) fn default_type_enum() -> SpecType {
    SpecType::Code
}

/// Verification status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    Partial,
}

impl FromStr for VerificationStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "partial" => Ok(Self::Partial),
            _ => anyhow::bail!(
                "Invalid verification status: {}. Must be one of: passed, failed, partial",
                s
            ),
        }
    }
}

/// Approval status for a spec
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
}

/// Approval information for a spec
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Approval {
    /// Whether approval is required for this spec
    #[serde(default)]
    pub required: bool,
    /// Current approval status
    #[serde(default)]
    pub status: ApprovalStatus,
    /// Name of the person who approved/rejected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Timestamp of approval/rejection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// Represents a dependency that is blocking a spec from being ready.
#[derive(Debug, Clone)]
pub struct BlockingDependency {
    /// The spec ID of the blocking dependency.
    pub spec_id: String,
    /// The title of the blocking dependency, if available.
    pub title: Option<String>,
    /// The current status of the blocking dependency.
    pub status: SpecStatus,
    /// When the dependency was completed, if applicable.
    pub completed_at: Option<String>,
    /// Whether this is a sibling dependency (from group ordering).
    pub is_sibling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecFrontmatter {
    #[serde(default = "default_type_enum")]
    pub r#type: SpecType,
    #[serde(default)]
    pub status: SpecStatus,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_depends_on"
    )]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    // Documentation-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<String>>,
    // Research-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub informed_by: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    // Conflict-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicting_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_specs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_spec: Option<String>,
    // Verification-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<VerificationStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_failures: Option<Vec<String>>,
    // Replay tracking fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replayed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_completed_at: Option<String>,
    // Derivation tracking - which fields were automatically derived
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_fields: Option<Vec<String>>,
    // Approval workflow fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
    // Driver/group member tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
    // Output schema validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<String>,
    // Site generation control - set to false to exclude from site
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    // Retry state for failed specs (watch mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_state: Option<crate::retry::RetryState>,
}

impl Default for SpecFrontmatter {
    fn default() -> Self {
        Self {
            r#type: default_type_enum(),
            status: SpecStatus::Pending,
            depends_on: None,
            labels: None,
            target_files: None,
            context: None,
            prompt: None,
            branch: None,
            commits: None,
            completed_at: None,
            model: None,
            tracks: None,
            informed_by: None,
            origin: None,
            schedule: None,
            source_branch: None,
            target_branch: None,
            conflicting_files: None,
            blocked_specs: None,
            original_spec: None,
            last_verified: None,
            verification_status: None,
            verification_failures: None,
            replayed_at: None,
            replay_count: None,
            original_completed_at: None,
            derived_fields: None,
            approval: None,
            members: None,
            output_schema: None,
            public: None,
            retry_state: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depends_on_string_format() {
        let yaml = r#"
type: code
status: pending
depends_on: "spec-id"
"#;
        let fm: SpecFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(fm.depends_on, Some(vec!["spec-id".to_string()]));
    }

    #[test]
    fn test_depends_on_array_format() {
        let yaml = r#"
type: code
status: pending
depends_on: ["a", "b"]
"#;
        let fm: SpecFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(fm.depends_on, Some(vec!["a".to_string(), "b".to_string()]));
    }
}
