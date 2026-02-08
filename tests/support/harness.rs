use chant::spec::Spec;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// TestHarness provides isolated test environments with full chant project structure.
/// Each harness creates a temporary directory with .chant/specs/, .chant/config.md,
/// and initializes a git repository.
pub struct TestHarness {
    pub dir: TempDir,
    pub specs_dir: PathBuf,
    #[allow(dead_code)]
    pub config_path: PathBuf,
    #[allow(dead_code)]
    pub chant_binary: PathBuf,
}

impl TestHarness {
    /// Creates a new test harness with default configuration.
    /// Sets up:
    /// - Temporary directory (auto-cleaned on drop)
    /// - .chant/specs/ directory
    /// - .chant/prompts/ directory with standard.md
    /// - .chant/config.md with default config
    /// - Git repository with initial commit
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        let specs_dir = base_path.join(".chant/specs");
        let prompts_dir = base_path.join(".chant/prompts");
        let config_path = base_path.join(".chant/config.md");

        fs::create_dir_all(&specs_dir).expect("Failed to create specs dir");
        fs::create_dir_all(&prompts_dir).expect("Failed to create prompts dir");

        let default_config = r#"---
model: sonnet
silent: false
---

# Project Config
"#;
        fs::write(&config_path, default_config).expect("Failed to write config");

        let prompt_content = "You are implementing a task for chant.";
        fs::write(prompts_dir.join("standard.md"), prompt_content).expect("Failed to write prompt");
        fs::write(prompts_dir.join("bootstrap.md"), prompt_content).expect("Failed to write bootstrap prompt");

        // Initialize git repo
        Self::init_git_repo(base_path);

        TestHarness {
            dir: temp_dir,
            specs_dir,
            config_path,
            chant_binary: PathBuf::from(env!("CARGO_BIN_EXE_chant")),
        }
    }

    /// Creates a test harness with custom config content.
    #[allow(dead_code)]
    pub fn with_config(config_content: &str) -> Self {
        let harness = Self::new();
        fs::write(&harness.config_path, config_content).expect("Failed to write custom config");
        harness
    }

    /// Returns the base directory path (the TempDir path).
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Executes the chant binary with the given arguments in the harness directory.
    #[allow(dead_code)]
    pub fn run(&self, args: &[&str]) -> std::io::Result<std::process::Output> {
        Command::new(&self.chant_binary)
            .args(args)
            .current_dir(self.path())
            .output()
    }

    /// Creates a spec file with the given ID and content.
    /// The content should be the full spec markdown including frontmatter.
    pub fn create_spec(&self, id: &str, content: &str) {
        let spec_path = self.specs_dir.join(format!("{}.md", id));
        fs::write(&spec_path, content).expect("Failed to write spec file");
    }

    /// Loads a spec from the specs directory by ID.
    #[allow(dead_code)]
    pub fn load_spec(&self, id: &str) -> Spec {
        let spec_path = self.specs_dir.join(format!("{}.md", id));
        let content = fs::read_to_string(&spec_path).expect("Failed to read spec file");
        let mut spec = Spec::parse(id, &content).expect("Failed to parse spec");
        spec.id = id.to_string();
        spec
    }

    /// Creates a git commit with the given message.
    #[allow(dead_code)]
    pub fn git_commit(&self, msg: &str) -> std::io::Result<()> {
        Command::new("git")
            .args(["add", "."])
            .current_dir(self.path())
            .output()?;

        Command::new("git")
            .args(["commit", "-m", msg])
            .current_dir(self.path())
            .output()?;

        Ok(())
    }

    /// Checks if a git branch exists.
    pub fn branch_exists(&self, branch_name: &str) -> bool {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", branch_name])
            .current_dir(self.path())
            .output()
            .expect("Failed to check branch");
        output.status.success()
    }

    /// Gets all git branches in the repo.
    pub fn get_branches(&self) -> Vec<String> {
        let output = Command::new("git")
            .args(["branch", "-a"])
            .current_dir(self.path())
            .output()
            .expect("Failed to list branches");

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// Gets the commit count for a given branch.
    pub fn get_commit_count(&self, branch: &str) -> usize {
        let output = Command::new("git")
            .args(["rev-list", "--count", branch])
            .current_dir(self.path())
            .output()
            .expect("Failed to count commits");

        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0)
    }

    fn init_git_repo(repo_dir: &Path) {
        let output = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(repo_dir)
            .output()
            .expect("Failed to init git repo");
        assert!(output.status.success(), "git init failed");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_dir)
            .output()
            .expect("Failed to set git email");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_dir)
            .output()
            .expect("Failed to set git name");

        // Create initial commit
        fs::write(repo_dir.join("README.md"), "# Test Repo").expect("Failed to write README");

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_dir)
            .output()
            .expect("Failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_dir)
            .output()
            .expect("Failed to create initial commit");
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}
