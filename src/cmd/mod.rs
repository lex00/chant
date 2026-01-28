//! Command module structure for chant CLI

use anyhow::Result;
use std::path::PathBuf;

use chant::paths::SPECS_DIR;

pub mod agent;
pub mod agent_rotation;
pub mod cleanup;
pub mod commits;
pub mod config;
pub mod derive;
pub mod disk;
pub mod export;
pub mod finalize;
pub mod git_ops;
pub mod lifecycle;
pub mod model;
pub mod prep;
pub mod refresh;
pub mod search;
pub mod spec;
pub mod verify;
pub mod work;

/// Ensure chant is initialized and return the specs directory path.
///
/// This checks for the existence of `.chant/specs` and returns an error
/// if chant has not been initialized.
pub fn ensure_initialized() -> Result<PathBuf> {
    let specs_dir = PathBuf::from(SPECS_DIR);
    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }
    Ok(specs_dir)
}
