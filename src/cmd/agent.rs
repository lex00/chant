//! Agent invocation module for chant CLI
//!
//! Handles all LLM agent interactions including model selection, provider management,
//! and streaming output capture.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use chant::config::Config;
use chant::paths::SPECS_DIR;
use chant::provider;
use chant::spec::Spec;

/// Invoke an agent with a message and return captured output
pub fn invoke_agent(
    message: &str,
    spec: &Spec,
    prompt_name: &str,
    config: &Config,
) -> Result<String> {
    invoke_agent_with_model(message, spec, prompt_name, config, None, None)
}

/// Invoke an agent with optional agent command override and return captured output
pub fn invoke_agent_with_command_override(
    message: &str,
    spec: &Spec,
    prompt_name: &str,
    config: &Config,
    agent_command: Option<&str>,
    cwd: Option<&Path>,
) -> Result<String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Use specified command or default to "claude"
    let command = agent_command.unwrap_or("claude");

    // Create streaming log writer before spawning agent
    let mut log_writer = match StreamingLogWriter::new(&spec.id, prompt_name) {
        Ok(writer) => Some(writer),
        Err(e) => {
            eprintln!("{} Failed to create agent log: {}", "⚠".yellow(), e);
            None
        }
    };

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!(".chant/specs/{}.md", spec.id))?;

    // Get the model to use
    let model = get_model_for_invocation(config.defaults.model.as_deref());

    // Set CHANT_SPEC_ID and CHANT_SPEC_FILE env vars
    std::env::set_var("CHANT_SPEC_ID", &spec.id);
    std::env::set_var("CHANT_SPEC_FILE", &spec_file);

    let mut cmd = Command::new(command);
    cmd.arg("--print")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--model")
        .arg(&model)
        .arg("--dangerously-skip-permissions")
        .arg(message)
        .env("CHANT_SPEC_ID", &spec.id)
        .env("CHANT_SPEC_FILE", &spec_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set working directory if provided
    if let Some(path) = cwd {
        cmd.current_dir(path);
    }

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to invoke agent '{}'. Is it installed and in PATH?",
            command
        )
    })?;

    // Write PID file for tracking
    if let Err(e) = chant::pid::write_pid_file(&spec.id, child.id()) {
        eprintln!("{} Failed to write PID file: {}", "⚠".yellow(), e);
    }

    // Collect output while streaming to terminal and log
    let mut captured_output = String::new();
    let mut stderr_output = String::new();

    // Stream stdout to both terminal and log file
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            for text in extract_text_from_stream_json(&line) {
                for text_line in text.lines() {
                    println!("{}", text_line);
                    captured_output.push_str(text_line);
                    captured_output.push('\n');
                    if let Some(ref mut writer) = log_writer {
                        if let Err(e) = writer.write_line(text_line) {
                            eprintln!("{} Failed to write to agent log: {}", "⚠".yellow(), e);
                        }
                    }
                }
            }
        }
    }

    // Capture stderr for diagnostics
    if let Some(stderr) = child.stderr.take() {
        use std::io::Read;
        let mut stderr_reader = BufReader::new(stderr);
        let _ = stderr_reader.read_to_string(&mut stderr_output);
    }

    let status = child.wait()?;

    // Clean up PID file
    if let Err(e) = chant::pid::remove_pid_file(&spec.id) {
        eprintln!("{} Failed to write PID file: {}", "⚠".yellow(), e);
    }

    if !status.success() {
        // Include stderr output in error message for diagnostics
        let mut error_msg = format!("Agent exited with status: {}", status);
        if !stderr_output.is_empty() {
            error_msg.push_str(&format!("\n\nStderr:\n{}", stderr_output.trim()));
        }
        if captured_output.is_empty() && stderr_output.is_empty() {
            error_msg.push_str("\n\nNo output captured - agent may have died immediately after starting. Check working directory and CLAUDE.md configuration.");
        }
        anyhow::bail!(error_msg);
    }

    Ok(captured_output)
}

/// Invoke an agent with a custom command and prefix output with spec ID
/// Used for parallel execution with multiple Claude accounts
pub fn invoke_agent_with_command(
    message: &str,
    spec_id: &str,
    prompt_name: &str,
    config_model: Option<&str>,
    cwd: Option<&Path>,
    agent_command: &str,
    branch_name: Option<&str>,
) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Create streaming log writer before spawning agent (writes header immediately)
    let mut log_writer = match StreamingLogWriter::new(spec_id, prompt_name) {
        Ok(writer) => Some(writer),
        Err(e) => {
            eprintln!(
                "{} [{}] Failed to create agent log: {}",
                "⚠".yellow(),
                spec_id,
                e
            );
            None
        }
    };

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!("{}/{}.md", SPECS_DIR, spec_id))?;

    // Get the model to use
    let model = get_model_for_invocation(config_model);

    let mut cmd = Command::new(agent_command);
    cmd.arg("--print")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--model")
        .arg(&model)
        .arg("--dangerously-skip-permissions")
        .arg(message)
        .env("CHANT_SPEC_ID", spec_id)
        .env("CHANT_SPEC_FILE", &spec_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Set working directory and worktree environment variables if provided
    if let Some(path) = cwd {
        cmd.current_dir(path);
        // Set worktree environment variables
        cmd.env("CHANT_WORKTREE", "1");
        cmd.env("CHANT_WORKTREE_PATH", path);
        if let Some(branch) = branch_name {
            cmd.env("CHANT_BRANCH", branch);
        }
    }

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to invoke agent '{}'. Is it installed and in PATH?",
            agent_command
        )
    })?;

    // Write PID file for tracking
    if let Err(e) = chant::pid::write_pid_file(spec_id, child.id()) {
        eprintln!(
            "{} [{}] Failed to write PID file: {}",
            "⚠".yellow(),
            spec_id,
            e
        );
    }

    // Stream stdout with prefix to both terminal and log file
    let mut stderr_output = String::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let prefix = format!("[{}]", spec_id);
        for line in reader.lines().map_while(Result::ok) {
            for text in extract_text_from_stream_json(&line) {
                for text_line in text.lines() {
                    println!("{} {}", prefix.cyan(), text_line);
                    if let Some(ref mut writer) = log_writer {
                        if let Err(e) = writer.write_line(text_line) {
                            eprintln!(
                                "{} [{}] Failed to write to agent log: {}",
                                "⚠".yellow(),
                                spec_id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    // Capture stderr for diagnostics
    if let Some(stderr) = child.stderr.take() {
        use std::io::Read;
        let mut stderr_reader = BufReader::new(stderr);
        let _ = stderr_reader.read_to_string(&mut stderr_output);
    }

    let status = child.wait()?;

    // Clean up PID file
    if let Err(e) = chant::pid::remove_pid_file(spec_id) {
        eprintln!(
            "{} [{}] Failed to remove PID file: {}",
            "⚠".yellow(),
            spec_id,
            e
        );
    }

    if !status.success() {
        let mut error_msg = format!("[{}] Agent exited with status: {}", spec_id, status);
        if !stderr_output.is_empty() {
            error_msg.push_str(&format!("\n\nStderr:\n{}", stderr_output.trim()));
        }
        anyhow::bail!(error_msg);
    }

    Ok(())
}

/// Invoke an agent with optional model override and working directory
pub fn invoke_agent_with_model(
    message: &str,
    spec: &Spec,
    prompt_name: &str,
    config: &Config,
    override_model: Option<&str>,
    cwd: Option<&Path>,
) -> Result<String> {
    // Create streaming log writer before spawning agent (writes header immediately)
    let mut log_writer = match StreamingLogWriter::new(&spec.id, prompt_name) {
        Ok(writer) => Some(writer),
        Err(e) => {
            eprintln!("{} Failed to create agent log: {}", "⚠".yellow(), e);
            None
        }
    };

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!(".chant/specs/{}.md", spec.id))?;

    // Get the model to use - allow override
    let model = if let Some(override_m) = override_model {
        override_m.to_string()
    } else {
        get_model_for_invocation(config.defaults.model.as_deref())
    };

    // Get the appropriate provider
    let provider_type = config.defaults.provider;
    let model_provider = get_model_provider(provider_type, config)?;

    // Set CHANT_SPEC_ID and CHANT_SPEC_FILE env vars
    std::env::set_var("CHANT_SPEC_ID", &spec.id);
    std::env::set_var("CHANT_SPEC_FILE", &spec_file);

    // Change to working directory if provided
    let original_cwd = std::env::current_dir().ok();
    if let Some(path) = cwd {
        std::env::set_current_dir(path)?;
    }

    // Invoke the model provider with streaming callback
    let captured_output = model_provider.invoke(message, &model, &mut |text_line: &str| {
        println!("{}", text_line);
        if let Some(ref mut writer) = log_writer {
            if let Err(e) = writer.write_line(text_line) {
                eprintln!("{} Failed to write to agent log: {}", "⚠".yellow(), e);
            }
        }
        Ok(())
    })?;

    // Restore original working directory
    if let Some(original_cwd) = original_cwd {
        std::env::set_current_dir(original_cwd)?;
    }

    Ok(captured_output)
}

/// Get the appropriate model provider based on configuration
fn get_model_provider(
    provider_type: provider::ProviderType,
    config: &Config,
) -> Result<Box<dyn provider::ModelProvider>> {
    match provider_type {
        provider::ProviderType::Claude => Ok(Box::new(provider::ClaudeCliProvider)),
        provider::ProviderType::Kirocli => Ok(Box::new(provider::KiroCliProvider)),
        provider::ProviderType::Ollama => {
            let ollama_config = config.providers.ollama.as_ref();
            let endpoint = ollama_config
                .map(|c| c.endpoint.clone())
                .unwrap_or_else(|| "http://localhost:11434/v1".to_string());
            let max_retries = ollama_config.map(|c| c.max_retries).unwrap_or(3);
            let retry_delay_ms = ollama_config.map(|c| c.retry_delay_ms).unwrap_or(1000);
            Ok(Box::new(provider::OllamaProvider {
                endpoint,
                max_retries,
                retry_delay_ms,
            }))
        }
        provider::ProviderType::Openai => {
            let openai_config = config.providers.openai.as_ref();
            let endpoint = openai_config
                .map(|c| c.endpoint.clone())
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let api_key = std::env::var("OPENAI_API_KEY").ok();
            let max_retries = openai_config.map(|c| c.max_retries).unwrap_or(3);
            let retry_delay_ms = openai_config.map(|c| c.retry_delay_ms).unwrap_or(1000);
            Ok(Box::new(provider::OpenaiProvider {
                endpoint,
                api_key,
                max_retries,
                retry_delay_ms,
            }))
        }
    }
}

/// Extract text content from a Claude CLI stream-json line.
/// Returns Vec of text strings from assistant message content blocks.
pub fn extract_text_from_stream_json(line: &str) -> Vec<String> {
    let mut texts = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some("assistant") = json.get("type").and_then(|t| t.as_str()) {
            if let Some(content) = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            {
                for item in content {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        texts.push(text.to_string());
                    }
                }
            }
        }
    }

    texts
}

/// Get the model to use for agent invocation.
/// Priority:
/// 1. CHANT_MODEL env var
/// 2. ANTHROPIC_MODEL env var
/// 3. defaults.model in config
/// 4. "haiku" as hardcoded fallback
pub fn get_model_for_invocation(config_model: Option<&str>) -> String {
    // 1. CHANT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 2. ANTHROPIC_MODEL env var
    if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 3. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 4. Hardcoded fallback
    const DEFAULT_MODEL: &str = "haiku";
    DEFAULT_MODEL.to_string()
}

/// A streaming log writer that writes to a log file in real-time
pub struct StreamingLogWriter {
    file: std::fs::File,
}

impl StreamingLogWriter {
    /// Create a new streaming log writer that opens the log file and writes the header
    pub fn new(spec_id: &str, prompt_name: &str) -> Result<Self> {
        Self::new_at(&PathBuf::from(".chant"), spec_id, prompt_name)
    }

    /// Create a new streaming log writer at the given base path
    pub fn new_at(base_path: &Path, spec_id: &str, prompt_name: &str) -> Result<Self> {
        use std::fs::OpenOptions;
        use std::io::Write;

        ensure_logs_dir_at(base_path)?;

        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        let file_exists = log_path.exists();

        // If file exists, append to it. Otherwise create it.
        let mut file = if file_exists {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?
        } else {
            let timestamp = chant::utc_now_iso();

            let mut file = std::fs::File::create(&log_path)?;

            // Write header immediately for new files
            writeln!(file, "# Agent Log: {}", spec_id)?;
            writeln!(file, "# Started: {}", timestamp)?;
            writeln!(file, "# Prompt: {}", prompt_name)?;
            writeln!(file)?;
            file.flush()?;

            file
        };

        // For existing files, add a visual run separator
        if file_exists {
            let timestamp = chant::utc_now_iso();
            writeln!(file)?;
            writeln!(file, "{}", "=".repeat(80))?;
            writeln!(file, "# New Run: {}", timestamp)?;
            writeln!(file, "# Prompt: {}", prompt_name)?;
            writeln!(file, "{}", "=".repeat(80))?;
            writeln!(file)?;
            file.flush()?;
        }

        Ok(Self { file })
    }

    /// Write a line to the log file and flush immediately for real-time visibility
    pub fn write_line(&mut self, line: &str) -> Result<()> {
        use std::io::Write;

        writeln!(self.file, "{}", line)?;
        self.file.flush()?;
        Ok(())
    }
}

/// Append a message to the spec log file
///
/// Creates the log file if it doesn't exist. Used for logging errors
/// that occur before the agent starts.
pub fn append_to_log(spec_id: &str, message: &str) -> Result<()> {
    use std::io::Write;

    let base_path = PathBuf::from(".chant");
    ensure_logs_dir_at(&base_path)?;

    let log_path = base_path.join("logs").join(format!("{}.log", spec_id));

    // Open in append mode, create if doesn't exist
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .context("Failed to open log file for writing")?;

    write!(file, "{}", message)?;
    file.flush()?;

    Ok(())
}

/// Create log file for a spec if it doesn't already exist
/// This is called when work starts to ensure log file exists immediately
pub fn create_log_file_if_not_exists(spec_id: &str, prompt_name: &str) -> Result<()> {
    create_log_file_if_not_exists_at(&PathBuf::from(".chant"), spec_id, prompt_name)
}

/// Create log file for a spec at the given base path if it doesn't already exist
pub fn create_log_file_if_not_exists_at(
    base_path: &Path,
    spec_id: &str,
    prompt_name: &str,
) -> Result<()> {
    use std::io::Write;

    ensure_logs_dir_at(base_path)?;

    let log_path = base_path.join("logs").join(format!("{}.log", spec_id));

    // Only create if it doesn't exist
    if !log_path.exists() {
        let timestamp = chant::utc_now_iso();

        let mut file = std::fs::File::create(&log_path)?;

        // Write header
        writeln!(file, "# Agent Log: {}", spec_id)?;
        writeln!(file, "# Started: {}", timestamp)?;
        writeln!(file, "# Prompt: {}", prompt_name)?;
        writeln!(file)?;
        file.flush()?;
    }

    Ok(())
}

/// Ensure the logs directory exists and is in .gitignore at the given base path
pub fn ensure_logs_dir_at(base_path: &Path) -> Result<()> {
    let logs_dir = base_path.join("logs");
    let gitignore_path = base_path.join(".gitignore");

    // Create logs directory if it doesn't exist
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)?;
    }

    // Add logs/ to .gitignore if not already present
    let gitignore_content = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    if !gitignore_content.lines().any(|line| line.trim() == "logs/") {
        let new_content = if gitignore_content.is_empty() {
            "logs/\n".to_string()
        } else if gitignore_content.ends_with('\n') {
            format!("{}logs/\n", gitignore_content)
        } else {
            format!("{}\nlogs/\n", gitignore_content)
        };
        std::fs::write(&gitignore_path, new_content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Worktree environment variables to be set for agent invocation.
    /// These are computed separately to make the logic testable.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct WorktreeEnvVars {
        /// CHANT_WORKTREE is set to "1" when running in a worktree
        chant_worktree: Option<String>,
        /// CHANT_WORKTREE_PATH is set to the worktree path
        chant_worktree_path: Option<String>,
        /// CHANT_BRANCH is set to the branch name
        chant_branch: Option<String>,
    }

    impl WorktreeEnvVars {
        /// Compute the environment variables to set for worktree execution.
        ///
        /// When `worktree_path` is `Some`, we're running in a worktree and should set:
        /// - CHANT_WORKTREE=1
        /// - CHANT_WORKTREE_PATH=<path>
        /// - CHANT_BRANCH=<branch> (if branch_name is provided)
        ///
        /// When `worktree_path` is `None`, no worktree env vars are set.
        fn compute(worktree_path: Option<&Path>, branch_name: Option<&str>) -> Self {
            if let Some(path) = worktree_path {
                WorktreeEnvVars {
                    chant_worktree: Some("1".to_string()),
                    chant_worktree_path: Some(path.display().to_string()),
                    chant_branch: branch_name.map(|s| s.to_string()),
                }
            } else {
                WorktreeEnvVars {
                    chant_worktree: None,
                    chant_worktree_path: None,
                    chant_branch: None,
                }
            }
        }

        /// Check if we're in worktree mode
        fn is_worktree_mode(&self) -> bool {
            self.chant_worktree.is_some()
        }
    }

    #[test]
    fn test_worktree_env_vars_with_worktree_and_branch() {
        let path = Path::new("/tmp/chant-test-spec");
        let branch = "chant/test-spec";

        let env_vars = WorktreeEnvVars::compute(Some(path), Some(branch));

        assert!(env_vars.is_worktree_mode());
        assert_eq!(env_vars.chant_worktree, Some("1".to_string()));
        assert_eq!(
            env_vars.chant_worktree_path,
            Some("/tmp/chant-test-spec".to_string())
        );
        assert_eq!(env_vars.chant_branch, Some("chant/test-spec".to_string()));
    }

    #[test]
    fn test_worktree_env_vars_with_worktree_no_branch() {
        let path = Path::new("/tmp/chant-test-spec");

        let env_vars = WorktreeEnvVars::compute(Some(path), None);

        assert!(env_vars.is_worktree_mode());
        assert_eq!(env_vars.chant_worktree, Some("1".to_string()));
        assert_eq!(
            env_vars.chant_worktree_path,
            Some("/tmp/chant-test-spec".to_string())
        );
        assert_eq!(env_vars.chant_branch, None);
    }

    #[test]
    fn test_worktree_env_vars_no_worktree() {
        let env_vars = WorktreeEnvVars::compute(None, None);

        assert!(!env_vars.is_worktree_mode());
        assert_eq!(env_vars.chant_worktree, None);
        assert_eq!(env_vars.chant_worktree_path, None);
        assert_eq!(env_vars.chant_branch, None);
    }

    #[test]
    fn test_worktree_env_vars_no_worktree_with_branch_ignored() {
        // Even if branch is provided, without worktree path, no env vars are set
        let env_vars = WorktreeEnvVars::compute(None, Some("some-branch"));

        assert!(!env_vars.is_worktree_mode());
        assert_eq!(env_vars.chant_worktree, None);
        assert_eq!(env_vars.chant_worktree_path, None);
        assert_eq!(env_vars.chant_branch, None);
    }

    #[test]
    fn test_worktree_env_vars_different_paths() {
        // Test with various path formats
        let paths = [
            "/tmp/chant-2026-01-30-001-abc",
            "/home/user/worktrees/chant-spec",
            "/var/tmp/test",
        ];

        for path_str in &paths {
            let path = Path::new(path_str);
            let env_vars = WorktreeEnvVars::compute(Some(path), Some("test-branch"));

            assert!(env_vars.is_worktree_mode());
            assert_eq!(env_vars.chant_worktree, Some("1".to_string()));
            assert_eq!(env_vars.chant_worktree_path, Some(path_str.to_string()));
            assert_eq!(env_vars.chant_branch, Some("test-branch".to_string()));
        }
    }

    #[test]
    fn test_extract_text_from_stream_json_assistant_message() {
        let json_line = r#"{"type":"assistant","message":{"content":[{"text":"Hello, world!"}]}}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["Hello, world!"]);
    }

    #[test]
    fn test_extract_text_from_stream_json_multiple_content() {
        let json_line =
            r#"{"type":"assistant","message":{"content":[{"text":"Line 1"},{"text":"Line 2"}]}}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["Line 1", "Line 2"]);
    }

    #[test]
    fn test_extract_text_from_stream_json_non_assistant() {
        let json_line = r#"{"type":"user","message":{"content":[{"text":"Should be ignored"}]}}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_invalid_json() {
        let json_line = "not valid json";
        let texts = extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_get_model_for_invocation_config_model() {
        // Clear env vars for test isolation
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let model = get_model_for_invocation(Some("claude-3-opus"));
        assert_eq!(model, "claude-3-opus");
    }

    #[test]
    fn test_get_model_for_invocation_fallback() {
        // Clear env vars for test isolation
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let model = get_model_for_invocation(None);
        assert_eq!(model, "haiku");
    }
}
