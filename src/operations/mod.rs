//! Shared business logic layer for spec operations.
//!
//! This module provides the canonical implementation of spec operations,
//! used by both CLI commands and MCP handlers to ensure consistency.

pub mod create;
pub mod finalize;
pub mod reset;
pub mod update;

pub use create::create_spec;
pub use finalize::{finalize_spec, FinalizeOptions};
pub use reset::{reset_spec, ResetOptions};
pub use update::{update_spec, UpdateOptions};
