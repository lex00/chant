//! Validation logic for configuration and derived fields.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::defaults::{FailureConfig, WatchConfig};

impl WatchConfig {
    /// Validate watch configuration
    pub fn validate(&self) -> Result<()> {
        if self.poll_interval_ms == 0 {
            anyhow::bail!("watch.poll_interval_ms must be greater than 0");
        }

        self.failure.validate()
    }
}

impl FailureConfig {
    /// Validate failure configuration
    pub fn validate(&self) -> Result<()> {
        if self.backoff_multiplier < 1.0 {
            anyhow::bail!(
                "watch.failure.backoff_multiplier must be >= 1.0, got {}",
                self.backoff_multiplier
            );
        }

        Ok(())
    }
}

/// Rejection action mode for approval workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RejectionAction {
    /// Leave rejected, user handles it manually
    #[default]
    Manual,
    /// Prompt to create fix spec, original becomes blocked with depends_on
    Dependency,
    /// Convert to driver with numbered member specs
    Group,
}

impl fmt::Display for RejectionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectionAction::Manual => write!(f, "manual"),
            RejectionAction::Dependency => write!(f, "dependency"),
            RejectionAction::Group => write!(f, "group"),
        }
    }
}

/// Approval workflow configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ApprovalConfig {
    /// Action to take when a spec is rejected
    #[serde(default)]
    pub rejection_action: RejectionAction,
    /// Require approval for specs worked by agents (auto-detected via Co-Authored-By)
    #[serde(default)]
    pub require_approval_for_agent_work: bool,
}

/// Output validation configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct OutputValidationConfig {
    /// If true, fail spec when output doesn't match schema; if false, warn only
    #[serde(default)]
    pub strict_output_validation: bool,
}

/// Enterprise configuration for derived frontmatter and validation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseConfig {
    /// Field derivation rules (which fields to derive, from what source, using what pattern)
    #[serde(default)]
    pub derived: HashMap<String, DerivedFieldConfig>,
    /// List of required field names to validate
    #[serde(default)]
    pub required: Vec<String>,
}

/// Configuration for a single derived field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedFieldConfig {
    /// Source of the derived value
    pub from: DerivationSource,
    /// Pattern for extracting/formatting the value
    pub pattern: String,
    /// Optional validation rule for the derived value
    #[serde(default)]
    pub validate: Option<ValidationRule>,
}

/// Source of a derived field value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationSource {
    /// Derive from git branch name
    Branch,
    /// Derive from file path
    Path,
    /// Derive from environment variable
    Env,
    /// Derive from git user information
    GitUser,
}

/// Validation rule for derived fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationRule {
    /// Enum validation: value must be one of the specified values
    Enum {
        /// List of allowed values
        values: Vec<String>,
    },
}
