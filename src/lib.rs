//! # Chant - Intent Driven Development
//!
//! Chant is a specification-driven development tool that enables reproducible,
//! auditable AI-assisted development workflows.
//!
//! ## Overview
//!
//! Specs define work intentions as markdown files with YAML frontmatter. The chant
//! CLI executes them in isolated git worktrees, ensuring reproducibility and
//! auditability of all changes.
//!
//! ## Core Concepts
//!
//! - **Specs**: Markdown files describing work to be done, with acceptance criteria
//! - **Worktrees**: Isolated git worktrees for spec execution
//! - **Providers**: Pluggable AI model backends (Claude, Ollama, OpenAI)
//!
//! ## Modules
//!
//! - [`spec`] - Spec parsing, frontmatter handling, and lifecycle management
//! - [`spec_group`] - Spec group/driver orchestration logic
//! - [`config`] - Configuration management for chant projects
//! - [`git`] - Git provider abstraction for PR creation
//! - [`provider`] - AI model provider abstraction
//! - [`worktree`] - Isolated git worktree operations
//! - [`id`] - Spec ID generation with date-based sequencing
//! - [`prompt`] - Prompt template management
//! - [`merge`] - Spec merge logic and utilities
//!
//! ## Example
//!
//! ```no_run
//! use std::path::Path;
//! use chant::spec::{Spec, SpecStatus};
//! use chant::config::Config;
//!
//! // Load project configuration
//! let config = Config::load().expect("Failed to load config");
//!
//! // Load a spec from a file
//! let spec = Spec::load(Path::new(".chant/specs/2026-01-24-01m-q7e.md"))
//!     .expect("Failed to load spec");
//!
//! // Check spec status
//! match spec.frontmatter.status {
//!     SpecStatus::Pending => println!("Spec is pending"),
//!     SpecStatus::Completed => println!("Spec is complete"),
//!     _ => {}
//! }
//! ```

// Re-export all public modules
pub mod agent;
pub mod config;
pub mod conflict;
pub mod deps;
pub mod derivation;
pub mod diagnose;
pub mod domain;
pub mod git;
pub mod git_ops;
pub mod id;
pub mod lock;
pub mod mcp;
pub mod merge;
pub mod merge_driver;
pub mod merge_errors;
pub mod operations;
pub mod pid;
pub mod prompt;
pub mod prompts;
pub mod provider;
pub mod repository;
pub mod retry;
pub mod score;
pub mod scoring;
pub mod site;
pub mod spec;
pub mod spec_group;
pub mod spec_template;
pub mod status;
pub mod takeover;
pub mod tools;
pub mod validation;
pub mod worktree;

/// Default path constants for chant directory structure.
pub mod paths {
    /// Directory containing spec files: `.chant/specs`
    pub const SPECS_DIR: &str = ".chant/specs";
    /// Directory containing prompt templates: `.chant/prompts`
    pub const PROMPTS_DIR: &str = ".chant/prompts";
    /// Directory containing execution logs: `.chant/logs`
    pub const LOGS_DIR: &str = ".chant/logs";
    /// Directory containing archived specs: `.chant/archive`
    pub const ARCHIVE_DIR: &str = ".chant/archive";
    /// Directory containing lock files: `.chant/.locks`
    pub const LOCKS_DIR: &str = ".chant/.locks";
    /// Directory containing internal store: `.chant/.store`
    pub const STORE_DIR: &str = ".chant/.store";
}

/// Generate a UTC timestamp in ISO 8601 format: `YYYY-MM-DDTHH:MM:SSZ`
///
/// This function uses `chrono::Utc::now()` to ensure the timestamp is truly in UTC,
/// not local time with a misleading `Z` suffix.
pub fn utc_now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
