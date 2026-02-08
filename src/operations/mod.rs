//! Shared business logic layer for spec operations.
//!
//! This module provides the canonical implementation of spec operations,
//! used by both CLI commands and MCP handlers to ensure consistency.

pub mod commits;
pub mod create;
pub mod finalize;
pub mod model;
pub mod reset;
pub mod update;

pub use commits::{
    detect_agent_in_commit, get_commits_for_spec, get_commits_for_spec_allow_no_commits,
    get_commits_for_spec_with_branch, get_commits_for_spec_with_branch_allow_no_commits,
    AgentDetectionResult, CommitError,
};
pub use create::create_spec;
pub use finalize::{finalize_spec, FinalizeOptions};
pub use model::{get_model_name, get_model_name_with_default};
pub use reset::{reset_spec, ResetOptions};
pub use update::{update_spec, UpdateOptions};
