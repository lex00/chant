// Integration test modules
#[allow(dead_code)]
mod support {
    pub use crate::support::*;
}

mod config_test;
mod dependency_test;
mod spec_lifecycle_test;
mod workflow_test;
mod worktree_test;
