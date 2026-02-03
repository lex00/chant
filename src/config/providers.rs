//! Provider configuration management.

use serde::Deserialize;

/// Configuration for a single repository in cross-repo dependency resolution
#[derive(Debug, Clone, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub prefix: Option<String>,
}
