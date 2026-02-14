//! MCP tool handlers and method implementations.

use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::paths::SPECS_DIR;

use super::protocol::{PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION};
use super::response::mcp_error_response;
use super::tools::{
    tool_chant_add, tool_chant_archive, tool_chant_cancel, tool_chant_diagnose,
    tool_chant_finalize, tool_chant_lint, tool_chant_log, tool_chant_pause, tool_chant_ready,
    tool_chant_reset, tool_chant_search, tool_chant_spec_get, tool_chant_spec_list,
    tool_chant_spec_update, tool_chant_split, tool_chant_status, tool_chant_takeover,
    tool_chant_verify, tool_chant_watch_start, tool_chant_watch_status, tool_chant_watch_stop,
    tool_chant_work_list, tool_chant_work_start,
};

pub fn handle_method(method: &str, params: Option<&Value>) -> Result<Value> {
    match method {
        "initialize" => handle_initialize(params),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params),
        _ => anyhow::bail!("Method not found: {}", method),
    }
}

pub fn handle_notification(method: &str, _params: Option<&Value>) {
    // Handle notifications that don't require a response
    match method {
        "notifications/initialized" => {
            // Client is ready
        }
        _ => {
            // Unknown notification, ignore
        }
    }
}

fn handle_initialize(_params: Option<&Value>) -> Result<Value> {
    Ok(json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION
        }
    }))
}

fn handle_tools_list() -> Result<Value> {
    Ok(json!({
        "tools": [
            // Query tools (read-only)
            {
                "name": "chant_spec_list",
                "description": "List all chant specs in the current project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Filter by status (pending, in_progress, completed, failed, ready, blocked)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of specs to return (default: 50)"
                        }
                    }
                }
            },
            {
                "name": "chant_spec_get",
                "description": "Get details of a chant spec including full body content",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_ready",
                "description": "List all specs that are ready to be worked (no unmet dependencies)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of specs to return (default: 50)"
                        }
                    }
                }
            },
            {
                "name": "chant_status",
                "description": "Get project status summary with spec counts by status",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "brief": {
                            "type": "boolean",
                            "description": "Return brief single-line output (e.g., '3 pending | 2 in_progress | 15 completed')"
                        },
                        "include_activity": {
                            "type": "boolean",
                            "description": "Include activity info for in_progress specs (last modified time, log activity)"
                        }
                    }
                }
            },
            {
                "name": "chant_log",
                "description": "Read execution log for a spec",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "lines": {
                            "type": "integer",
                            "description": "Number of lines to return (default: 100)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Start from byte offset (for incremental reads)"
                        },
                        "since": {
                            "type": "string",
                            "description": "ISO timestamp - only lines after this time"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_search",
                "description": "Search specs by title and body content",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query (case-insensitive substring match)"
                        },
                        "status": {
                            "type": "string",
                            "description": "Filter by status"
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "chant_diagnose",
                "description": "Diagnose issues with a spec (check file, log, locks, commits, criteria)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_lint",
                "description": "Lint specs to check for quality issues (complexity, missing criteria, etc.)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID to lint (optional, lints all if not provided)"
                        }
                    }
                }
            },
            // Mutating tools
            {
                "name": "chant_spec_update",
                "description": "Update a chant spec status or add output",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "status": {
                            "type": "string",
                            "description": "New status (pending, in_progress, completed, failed)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Output text to append to spec body (or replace if replace_body is true)"
                        },
                        "replace_body": {
                            "type": "boolean",
                            "description": "Replace spec body with output instead of appending (default: false)"
                        },
                        "depends_on": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "List of spec IDs this spec depends on"
                        },
                        "labels": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Labels to tag the spec with"
                        },
                        "target_files": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "List of target files this spec affects"
                        },
                        "model": {
                            "type": "string",
                            "description": "Model to use for this spec (e.g., 'sonnet', 'opus', 'haiku')"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force update (bypass agent log gate for status=completed, default: false)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_add",
                "description": "Create a new spec with description",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Description of work to be done (becomes spec title)"
                        },
                        "prompt": {
                            "type": "string",
                            "description": "Optional prompt template name to use"
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "chant_finalize",
                "description": "Mark a spec as completed (validates all criteria are checked)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force finalization (bypass agent log gate, default: false)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_reset",
                "description": "Reset a failed spec to pending status so it can be reworked",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_cancel",
                "description": "Cancel a spec (sets status to cancelled)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_archive",
                "description": "Move a completed spec to the archive directory",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_verify",
                "description": "Verify a spec meets its acceptance criteria",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_work_start",
                "description": "Start working on a spec asynchronously (returns immediately)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "chain": {
                            "type": "boolean",
                            "description": "Continue to next ready spec after completion"
                        },
                        "parallel": {
                            "type": "integer",
                            "description": "Number of parallel workers (requires multiple ready specs)"
                        },
                        "skip_criteria": {
                            "type": "boolean",
                            "description": "Skip acceptance criteria validation"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_work_list",
                "description": "List running work processes",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "process_id": {
                            "type": "string",
                            "description": "Filter to specific process"
                        },
                        "include_completed": {
                            "type": "boolean",
                            "description": "Include recently completed processes"
                        }
                    }
                }
            },
            {
                "name": "chant_pause",
                "description": "Pause a running work process for a spec",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_takeover",
                "description": "Take over a running spec, stopping the agent and analyzing progress",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Force takeover even if spec is not running"
                        }
                    },
                    "required": ["id"]
                }
            },
            // Watch tools
            {
                "name": "chant_watch_status",
                "description": "Get status of watch process and active worktrees",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "chant_watch_start",
                "description": "Start the watch process",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "chant_watch_stop",
                "description": "Stop the watch process",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "chant_split",
                "description": "Split a complex spec into smaller member specs using AI analysis",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Skip confirmation prompts"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Recursively split member specs that are still too complex"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum recursion depth (default: 3)"
                        }
                    },
                    "required": ["id"]
                }
            }
        ]
    }))
}

fn handle_tools_call(params: Option<&Value>) -> Result<Value> {
    let params = params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;

    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

    let arguments = params.get("arguments");

    let mut result = match name {
        // Query tools (read-only)
        "chant_spec_list" => tool_chant_spec_list(arguments),
        "chant_spec_get" => tool_chant_spec_get(arguments),
        "chant_ready" => tool_chant_ready(arguments),
        "chant_status" => tool_chant_status(arguments),
        "chant_log" => tool_chant_log(arguments),
        "chant_search" => tool_chant_search(arguments),
        "chant_diagnose" => tool_chant_diagnose(arguments),
        "chant_lint" => tool_chant_lint(arguments),
        // Mutating tools
        "chant_spec_update" => tool_chant_spec_update(arguments),
        "chant_add" => tool_chant_add(arguments),
        "chant_finalize" => tool_chant_finalize(arguments),
        "chant_reset" => tool_chant_reset(arguments),
        "chant_cancel" => tool_chant_cancel(arguments),
        "chant_archive" => tool_chant_archive(arguments),
        "chant_verify" => tool_chant_verify(arguments),
        "chant_work_start" => tool_chant_work_start(arguments),
        "chant_work_list" => tool_chant_work_list(arguments),
        "chant_pause" => tool_chant_pause(arguments),
        "chant_takeover" => tool_chant_takeover(arguments),
        // Watch tools
        "chant_watch_status" => tool_chant_watch_status(arguments),
        "chant_watch_start" => tool_chant_watch_start(arguments),
        "chant_watch_stop" => tool_chant_watch_stop(arguments),
        // AI-powered tools
        "chant_split" => tool_chant_split(arguments),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }?;

    // Add branded banner for action tools
    if matches!(
        name,
        "chant_ready" | "chant_work_start" | "chant_verify" | "chant_finalize"
    ) {
        wrap_in_banner(&mut result);
    }

    Ok(result)
}

/// Wrap response text in branded banner
fn wrap_in_banner(result: &mut Value) {
    if let Some(content_array) = result.get_mut("content").and_then(|v| v.as_array_mut()) {
        if let Some(last_item) = content_array.last_mut() {
            if let Some(text) = last_item.get_mut("text").and_then(|v| v.as_str()) {
                let banner_top = "── chant ─────────────────────────";
                let banner_bottom = "──────────────────────────────────";
                let new_text = format!("{}\n{}\n{}", banner_top, text, banner_bottom);
                last_item["text"] = json!(new_text);
            }
        }
    }
}

/// Check for running single or chain work processes.
///
/// Returns:
/// - `Ok(Some((spec_id, pid)))` if a running non-parallel process is found
/// - `Ok(None)` if no running processes
/// - `Err(...)` if unable to check (should fail open)
pub fn check_for_running_work_processes() -> Result<Option<(String, u32)>> {
    let active_pids = crate::pid::list_active_pids()?;

    // Filter to only running processes (ignore stale PIDs)
    for (spec_id, pid, is_running) in active_pids {
        if is_running {
            return Ok(Some((spec_id, pid)));
        }
    }

    Ok(None)
}

/// Find the project root by walking up from cwd looking for `.chant/` directory.
///
/// # Returns
///
/// - `Some(path)`: The directory containing `.chant/`
/// - `None`: No `.chant/` directory found in any parent
pub fn find_project_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        if current.join(".chant").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Check if chant is initialized and return specs_dir, or an MCP error response.
///
/// # Validation
///
/// Checks that the `.chant/specs` directory exists, indicating that `chant init` has been run.
///
/// # Returns
///
/// - `Ok(specs_dir)`: If the specs directory exists
/// - `Err(response)`: MCP response object with `isError: true` if not initialized
///
/// # Tool-Level Error Format
///
/// When returning an error, the response format differs from JSON-RPC protocol errors:
/// - Not a JSON-RPC error (no `error` field)
/// - Instead uses `isError: true` flag in the result
/// - Error message in `content[].text`
/// - This allows tools to return meaningful errors while maintaining valid JSON-RPC responses
pub fn mcp_ensure_initialized() -> Result<PathBuf, Value> {
    let project_root = find_project_root().ok_or_else(|| {
        mcp_error_response("Not in a chant project directory. Run `chant init` first or navigate to a directory containing `.chant/`.")
    })?;

    let specs_dir = project_root.join(SPECS_DIR);
    if !specs_dir.exists() {
        return Err(mcp_error_response(
            "Chant not initialized. Run `chant init` first.",
        ));
    }
    Ok(specs_dir)
}
