//! Shared business logic layer for spec operations.
//!
//! This module provides the canonical implementation of spec operations,
//! used by both CLI commands and MCP handlers to ensure consistency.

pub mod archive;
pub mod cancel;
pub mod commits;
pub mod create;
pub mod finalize;
pub mod model;
pub mod pause;
pub mod reset;
pub mod update;
pub mod verify;

pub use archive::{archive_spec, ArchiveOptions};
pub use cancel::{cancel_spec, CancelOptions};
pub use commits::{
    detect_agent_in_commit, get_commits_for_spec, get_commits_for_spec_allow_no_commits,
    get_commits_for_spec_with_branch, get_commits_for_spec_with_branch_allow_no_commits,
    AgentDetectionResult, CommitError,
};
pub use create::create_spec;
pub use finalize::{finalize_spec, FinalizeOptions};
pub use model::{get_model_name, get_model_name_with_default};
pub use pause::{pause_spec, PauseOptions};
pub use reset::{reset_spec, ResetOptions};
pub use update::{update_spec, UpdateOptions};
pub use verify::{
    extract_acceptance_criteria, parse_verification_response,
    update_spec_with_verification_results, verify_spec, CriterionResult, VerificationStatus,
    VerifyOptions,
};
