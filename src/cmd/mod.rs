//! Command module structure for chant CLI

use anyhow::Result;
use std::path::PathBuf;

use chant::paths::SPECS_DIR;

pub mod activity;
pub mod agent;
pub mod agent_rotation;
pub mod cleanup;
pub mod commits;
pub mod config;
pub mod derive;
pub mod disk;
pub mod dispatch;
pub mod export;
pub mod finalize;
pub mod git_ops;
pub mod lifecycle;
pub mod model;
pub mod prep;
pub mod refresh;
pub mod search;
pub mod silent;
pub mod site;
pub mod spec;
pub mod stop;
pub mod takeover;
pub mod template;
pub mod validate;
pub mod verify;
pub mod watch;
pub mod work;
pub mod worktree;

/// Ensure chant is initialized and return the specs directory path.
///
/// This checks for the existence of `.chant/specs` and returns an error
/// if chant has not been initialized.
pub fn ensure_initialized() -> Result<PathBuf> {
    let specs_dir = PathBuf::from(SPECS_DIR);
    if !specs_dir.exists() {
        anyhow::bail!(
            "Not a chant project (no .chant/ directory found)\n\n\
             To initialize chant, run:\n    \
             chant init\n\n\
             This starts an interactive wizard to configure your project."
        );
    }
    Ok(specs_dir)
}
