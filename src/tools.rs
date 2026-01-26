//! Tool definitions for ollama-rs function calling integration.
//!
//! These tools provide the agent with the ability to read files, write files,
//! run commands, and list files during spec execution.

use anyhow::Result;
use serde_json::{json, Value};
use std::fs;
use std::process::Command;

/// Get JSON schema definitions for all available tools.
/// These schemas are passed to the ollama model to enable function calling.
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "Function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file at the given path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "Function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file, creating or overwriting it",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to write to"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "Function",
            "function": {
                "name": "run_command",
                "description": "Run a shell command and return its output",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "Function",
            "function": {
                "name": "list_files",
                "description": "List files matching a glob pattern",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern like 'src/**/*.rs' or '*.txt'"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "Function",
            "function": {
                "name": "task_complete",
                "description": "Signal that the task has been completed successfully",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "summary": {
                            "type": "string",
                            "description": "Brief summary of what was accomplished"
                        }
                    },
                    "required": ["summary"]
                }
            }
        }),
    ]
}

/// Execute a tool by name with the given arguments.
/// Returns the result as a string to be sent back to the model.
pub fn execute_tool(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| "missing or invalid 'path' parameter".to_string())?;
            read_file(path.to_string()).map_err(|e| format!("read_file failed: {}", e))
        }
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| "missing or invalid 'path' parameter".to_string())?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| "missing or invalid 'content' parameter".to_string())?;
            write_file(path.to_string(), content.to_string())
                .map_err(|e| format!("write_file failed: {}", e))
        }
        "run_command" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| "missing or invalid 'command' parameter".to_string())?;
            run_command(command.to_string()).map_err(|e| format!("run_command failed: {}", e))
        }
        "list_files" => {
            let pattern = args["pattern"]
                .as_str()
                .ok_or_else(|| "missing or invalid 'pattern' parameter".to_string())?;
            list_files(pattern.to_string()).map_err(|e| format!("list_files failed: {}", e))
        }
        "task_complete" => {
            let summary = args["summary"].as_str().unwrap_or("Task completed");
            Ok(format!("TASK_COMPLETE: {}", summary))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

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
