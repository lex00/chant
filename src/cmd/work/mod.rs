//! Work command execution for chant CLI
//!
//! Handles spec execution including:
//! - Single spec execution with agent invocation
//! - Chain execution for sequential spec processing
//! - Parallel spec execution with thread pools
//! - Interactive wizard for spec selection
//! - Spec finalization and status management
//! - Branch and PR creation
//! - Worktree management

use anyhow::{Context, Result};
use std::path::Path;

use chant::spec::Spec;

// Submodules
pub mod chain;
pub mod executor;
pub mod parallel;
pub mod single;
pub mod ui;
pub mod wizard;

// Re-export public types from submodules
pub use chain::{cmd_work_chain, ChainOptions};
pub use parallel::{cmd_work_parallel, ParallelOptions};
pub use single::cmd_work;
pub use wizard::{run_wizard, WizardSelection};

// ============================================================================
// SHARED HELPER FUNCTIONS
// ============================================================================

/// Start watch in background if not already running
pub(crate) fn ensure_watch_running() -> Result<()> {
    use crate::cmd::watch::is_watch_running;
    use std::process::Command;

    if is_watch_running() {
        return Ok(());
    }

    // Get the current executable path to spawn watch
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // Spawn watch in background
    Command::new(current_exe)
        .args(["watch"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn watch process")?;

    // Give it a moment to start and write PID
    std::thread::sleep(std::time::Duration::from_millis(100));

    Ok(())
}

/// Load all ready specs from the specs directory
pub(crate) fn load_ready_specs(specs_dir: &Path) -> Result<Vec<Spec>> {
    let all_specs = chant::spec::load_all_specs(specs_dir)?;
    let ready_specs: Vec<Spec> = all_specs
        .iter()
        .filter(|s| s.is_ready(&all_specs))
        .cloned()
        .collect();
    Ok(ready_specs)
}

/// List all available prompts from the prompts directory
pub(crate) fn list_available_prompts(prompts_dir: &Path) -> Result<Vec<String>> {
    let mut prompts = Vec::new();
    if prompts_dir.exists() && prompts_dir.is_dir() {
        for entry in std::fs::read_dir(prompts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                if let Some(stem) = path.file_stem() {
                    prompts.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }
    prompts.sort();
    Ok(prompts)
}
