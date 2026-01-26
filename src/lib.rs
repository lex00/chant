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
pub mod diagnose;
pub mod git;
pub mod id;
pub mod merge;
pub mod prompt;
pub mod provider;
pub mod spec;
pub mod spec_group;
pub mod tools;
pub mod worktree;
