//! Command dispatch trait for reducing main.rs boilerplate
//!
//! This module defines the `Execute` trait that Commands enum implements,
//! moving the dispatch logic out of a massive match statement in main.rs.

use anyhow::Result;

/// Trait for executing CLI commands
///
/// Each command variant should implement this trait to handle its execution logic.
/// This allows the main `run()` function to simply call `command.execute()` instead
/// of maintaining a large match statement with destructuring and forwarding logic.
pub trait Execute {
    /// Execute the command and return a result
    fn execute(self) -> Result<()>;
}
