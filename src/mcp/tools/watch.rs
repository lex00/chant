//! MCP tools for watch visibility and control

use anyhow::Result;
use serde_json::{json, Value};

use crate::worktree::status::read_status;

/// Information about an active worktree
#[derive(Debug, Clone)]
struct WorktreeInfo {
    path: std::path::PathBuf,
    spec_id: String,
}

/// Find all active worktrees with chant/* branches
fn find_active_worktrees() -> Result<Vec<WorktreeInfo>> {
    use std::path::PathBuf;
    use std::process::Command;

    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree list failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                if branch.starts_with("chant/") {
                    if let Some(spec_id) = branch.strip_prefix("chant/") {
                        worktrees.push(WorktreeInfo {
                            path,
                            spec_id: spec_id.to_string(),
                        });
                    }
                }
            }
            let path = line.strip_prefix("worktree ").unwrap_or("");
            current_path = Some(PathBuf::from(path));
            current_branch = None;
        } else if line.starts_with("branch ") {
            let branch = line.strip_prefix("branch ").unwrap_or("");
            let branch = branch.strip_prefix("refs/heads/").unwrap_or(branch);
            current_branch = Some(branch.to_string());
        }
    }

    if let (Some(path), Some(branch)) = (current_path, current_branch) {
        if branch.starts_with("chant/") {
            if let Some(spec_id) = branch.strip_prefix("chant/") {
                worktrees.push(WorktreeInfo {
                    path,
                    spec_id: spec_id.to_string(),
                });
            }
        }
    }

    Ok(worktrees)
}

/// Check if watch is currently running
fn is_watch_running() -> bool {
    use std::fs;
    use std::path::PathBuf;

    let pid_path = PathBuf::from(".chant/watch.pid");
    let pid = match fs::read_to_string(&pid_path) {
        Ok(content) => match content.trim().parse::<u32>() {
            Ok(p) => p,
            Err(_) => return false,
        },
        Err(_) => return false,
    };

    // Check if process is alive
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        kill(Pid::from_raw(pid as i32), None).is_ok()
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Some(stdout.contains(&pid.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
}

/// Handle chant_watch_status tool call
pub fn tool_chant_watch_status(_arguments: Option<&Value>) -> Result<Value> {
    // Check if watch is running
    let is_running = is_watch_running();

    // Find active worktrees
    let worktrees = match find_active_worktrees() {
        Ok(wts) => wts,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Failed to list worktrees: {}", e)
                    }
                ],
                "isError": true
            }));
        }
    };

    // Build worktree status list
    let mut worktree_statuses = Vec::new();
    for worktree in &worktrees {
        let status_file = worktree.path.join(".chant-status.json");

        let status_info = match read_status(&status_file) {
            Ok(status) => json!({
                "spec_id": worktree.spec_id,
                "path": worktree.path.display().to_string(),
                "status": format!("{:?}", status.status).to_lowercase(),
                "updated_at": status.updated_at,
                "error": status.error,
                "commits": status.commits
            }),
            Err(_) => json!({
                "spec_id": worktree.spec_id,
                "path": worktree.path.display().to_string(),
                "status": "unknown",
                "error": "No status file found"
            }),
        };

        worktree_statuses.push(status_info);
    }

    let response = json!({
        "watch_running": is_running,
        "worktrees": worktree_statuses,
        "worktree_count": worktrees.len()
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)?
            }
        ]
    }))
}

/// Handle chant_watch_start tool call
pub fn tool_chant_watch_start(_arguments: Option<&Value>) -> Result<Value> {
    use std::process::{Command, Stdio};

    // Check if already running
    if is_watch_running() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "Watch is already running"
                }
            ],
            "isError": true
        }));
    }

    // Spawn watch in background
    let child = Command::new("chant")
        .arg("watch")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match child {
        Ok(child) => {
            let pid = child.id();
            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Started watch process (PID: {})", pid)
                    }
                ]
            }))
        }
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Failed to start watch: {}", e)
                }
            ],
            "isError": true
        })),
    }
}

/// Handle chant_watch_stop tool call
pub fn tool_chant_watch_stop(_arguments: Option<&Value>) -> Result<Value> {
    use std::fs;
    use std::path::PathBuf;

    // Check if watch is running
    if !is_watch_running() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "Watch is not running"
                }
            ],
            "isError": true
        }));
    }

    // Read PID
    let pid_path = PathBuf::from(".chant/watch.pid");
    let pid_str = match fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Failed to read PID file: {}", e)
                    }
                ],
                "isError": true
            }));
        }
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Invalid PID in file: {}", e)
                    }
                ],
                "isError": true
            }));
        }
    };

    // Send SIGTERM to watch process
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
            Ok(()) => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Sent stop signal to watch process (PID: {})", pid)
                    }
                ]
            })),
            Err(e) => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Failed to stop watch: {}", e)
                    }
                ],
                "isError": true
            })),
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        match Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
        {
            Ok(output) if output.status.success() => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Stopped watch process (PID: {})", pid)
                    }
                ]
            })),
            Ok(_) => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": "Failed to stop watch process"
                    }
                ],
                "isError": true
            })),
            Err(e) => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Failed to stop watch: {}", e)
                    }
                ],
                "isError": true
            })),
        }
    }
}
