//! Frontmatter types and defaults for specs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    NeedsAttention,
    Ready,
    Blocked,
    Cancelled,
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
    #[serde(default = "default_type")]
    pub r#type: String,
    #[serde(default)]
    pub status: SpecStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub verification_status: Option<String>,
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

pub(crate) fn default_type() -> String {
    "code".to_string()
}

impl Default for SpecFrontmatter {
    fn default() -> Self {
        Self {
            r#type: default_type(),
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
