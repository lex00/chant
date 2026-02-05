use anyhow::Result;

/// A trait for git repository operations.
pub trait GitRepository {
    /// Get the current branch name.
    fn get_current_branch(&self) -> Result<String>;

    /// Check if a branch exists.
    fn branch_exists(&self, name: &str) -> Result<bool>;
}

/// Command-based implementation of GitRepository.
pub struct CommandGitRepository;

impl CommandGitRepository {
    /// Create a new CommandGitRepository.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommandGitRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl GitRepository for CommandGitRepository {
    fn get_current_branch(&self) -> Result<String> {
        crate::git::get_current_branch()
    }

    fn branch_exists(&self, name: &str) -> Result<bool> {
        crate::git::branch_exists(name)
    }
}
