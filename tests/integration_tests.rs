//! Integration tests runner

#[path = "common.rs"]
mod common;

#[path = "support/mod.rs"]
mod support;

#[path = "integration/config_test.rs"]
mod config_test;

#[path = "integration/dependency_test.rs"]
mod dependency_test;

#[path = "integration/spec_lifecycle_test.rs"]
mod spec_lifecycle_test;

#[path = "integration/worktree_test.rs"]
mod worktree_test;
