//! Tool definitions for ollama-rs function calling integration.
//!
//! These tools provide the agent with the ability to read files, write files,
//! run commands, and list files during spec execution.

use anyhow::Result;
use std::fs;
use std::process::Command;

/// Read the contents of a file at the given path.
/// Use this to understand existing code before making changes.
pub fn read_file(path: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    Ok(fs::read_to_string(&path)?)
}

/// Write content to a file at the given path.
/// Creates the file if it doesn't exist, overwrites if it does.
pub fn write_file(
    path: String,
    content: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    fs::write(&path, &content)?;
    Ok(format!("Wrote {} bytes to {}", content.len(), path))
}

/// Run a shell command and return its output.
/// Use for: git operations, cargo build/test, file operations.
pub fn run_command(command: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("sh").arg("-c").arg(&command).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Ok(format!(
            "Command failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        ))
    }
}

/// List files matching a glob pattern.
/// Example: list_files("src/**/*.rs") returns all Rust files in src/
pub fn list_files(pattern: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use glob::glob;

    let paths: Vec<_> = glob(&pattern)?
        .filter_map(Result::ok)
        .map(|p| p.display().to_string())
        .collect();
    Ok(paths.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_write_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let path_str = file_path.to_string_lossy().to_string();

        let result = write_file(path_str.clone(), "test content".to_string()).unwrap();
        assert!(result.contains("12 bytes"));

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_read_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let path_str = file_path.to_string_lossy().to_string();
        let content = read_file(path_str).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_run_command() {
        let result = run_command("echo 'test output'".to_string()).unwrap();
        assert!(result.contains("test output"));
    }

    #[test]
    fn test_list_files() {
        let dir = tempdir().unwrap();
        let _file1 = fs::File::create(dir.path().join("file1.txt")).unwrap();
        let _file2 = fs::File::create(dir.path().join("file2.txt")).unwrap();

        let pattern = format!("{}/*.txt", dir.path().display());
        let result = list_files(pattern).unwrap();

        assert!(result.contains("file1.txt"));
        assert!(result.contains("file2.txt"));
    }
}
