//! Worktree module - git worktree operations and agent status communication
//!
//! This module provides low-level git worktree operations and the agent status
//! file format for communication between agents and the watch process.

pub mod git_ops;
pub mod status;

// Re-export git operations for backward compatibility
pub use git_ops::*;
