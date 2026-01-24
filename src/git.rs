//! Git provider abstraction for PR creation.
//!
//! Supports multiple git hosting providers (GitHub, GitLab, Bitbucket).

use anyhow::{Context, Result};
use std::process::Command;

use crate::config::GitProvider;

/// Trait for git hosting providers that can create pull/merge requests.
pub trait PrProvider {
    /// Create a pull/merge request with the given title and body.
    /// Returns the URL of the created PR/MR.
    fn create_pr(&self, title: &str, body: &str) -> Result<String>;

    /// Returns the CLI tool name used by this provider.
    #[allow(dead_code)]
    fn cli_tool(&self) -> &'static str;

    /// Returns a human-readable name for this provider.
    fn name(&self) -> &'static str;
}

/// GitHub provider using the `gh` CLI.
pub struct GitHubProvider;

impl PrProvider for GitHubProvider {
    fn create_pr(&self, title: &str, body: &str) -> Result<String> {
        let output = Command::new("gh")
            .args(["pr", "create", "--title", title, "--body", body])
            .output()
            .context("Failed to run gh pr create. Is gh CLI installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create pull request: {}", stderr);
        }

        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(pr_url)
    }

    fn cli_tool(&self) -> &'static str {
        "gh"
    }

    fn name(&self) -> &'static str {
        "GitHub"
    }
}

/// GitLab provider using the `glab` CLI.
pub struct GitLabProvider;

impl PrProvider for GitLabProvider {
    fn create_pr(&self, title: &str, body: &str) -> Result<String> {
        let output = Command::new("glab")
            .args([
                "mr",
                "create",
                "--title",
                title,
                "--description",
                body,
                "--yes",
            ])
            .output()
            .context("Failed to run glab mr create. Is glab CLI installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create merge request: {}", stderr);
        }

        let mr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(mr_url)
    }

    fn cli_tool(&self) -> &'static str {
        "glab"
    }

    fn name(&self) -> &'static str {
        "GitLab"
    }
}

/// Bitbucket provider using the `bb` CLI.
pub struct BitbucketProvider;

impl PrProvider for BitbucketProvider {
    fn create_pr(&self, title: &str, body: &str) -> Result<String> {
        let output = Command::new("bb")
            .args(["pr", "create", "--title", title, "--body", body])
            .output()
            .context("Failed to run bb pr create. Is Bitbucket CLI installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create pull request: {}", stderr);
        }

        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(pr_url)
    }

    fn cli_tool(&self) -> &'static str {
        "bb"
    }

    fn name(&self) -> &'static str {
        "Bitbucket"
    }
}

/// Get the appropriate PR provider for the given config.
pub fn get_provider(provider: GitProvider) -> Box<dyn PrProvider> {
    match provider {
        GitProvider::Github => Box::new(GitHubProvider),
        GitProvider::Gitlab => Box::new(GitLabProvider),
        GitProvider::Bitbucket => Box::new(BitbucketProvider),
    }
}

/// Create a pull/merge request using the configured provider.
#[allow(dead_code)]
pub fn create_pull_request(provider: GitProvider, title: &str, body: &str) -> Result<String> {
    let pr_provider = get_provider(provider);
    pr_provider.create_pr(title, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_names() {
        assert_eq!(GitHubProvider.name(), "GitHub");
        assert_eq!(GitLabProvider.name(), "GitLab");
        assert_eq!(BitbucketProvider.name(), "Bitbucket");
    }

    #[test]
    fn test_provider_cli_tools() {
        assert_eq!(GitHubProvider.cli_tool(), "gh");
        assert_eq!(GitLabProvider.cli_tool(), "glab");
        assert_eq!(BitbucketProvider.cli_tool(), "bb");
    }

    #[test]
    fn test_get_provider() {
        let github = get_provider(GitProvider::Github);
        assert_eq!(github.name(), "GitHub");

        let gitlab = get_provider(GitProvider::Gitlab);
        assert_eq!(gitlab.name(), "GitLab");

        let bitbucket = get_provider(GitProvider::Bitbucket);
        assert_eq!(bitbucket.name(), "Bitbucket");
    }
}
