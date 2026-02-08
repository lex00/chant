//! Integration tests runner

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

#[path = "integration/watch_recovery_test.rs"]
mod watch_recovery_test;
